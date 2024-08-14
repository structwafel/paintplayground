use std::sync::{
    atomic::{AtomicI64, AtomicU64},
    Arc,
};

use crate::{
    chunk_db::ChunkLoaderSaver,
    chunk_manager::{ChunkManager, ChunkUpdate, HandlerData},
    types::*,
};

pub enum Error {
    TooManyChunksLoaded,
}

#[derive(Debug)]
pub enum ChunkRequest {
    Storage,
    Live,
}

pub enum BoardManagerMessage {
    GetChunk(
        ChunkCoordinates,
        ChunkRequest,
        oneshot::Sender<Option<Chunk>>,
    ),
    GetHandler(
        ChunkCoordinates,
        oneshot::Sender<Result<HandlerData, Error>>,
    ),
}
#[derive(Debug, Clone)]
pub struct BoardManagerCommunicator {
    board_manager_tx: tokio::sync::mpsc::Sender<BoardManagerMessage>,
}

impl BoardManagerCommunicator {
    pub async fn get_chunk(
        &self,
        coordinates: ChunkCoordinates,
        request_type: ChunkRequest,
    ) -> Option<Chunk> {
        let (sender, receiver) = oneshot::channel();
        self.board_manager_tx
            .send(BoardManagerMessage::GetChunk(
                coordinates,
                request_type,
                sender,
            ))
            .await
            .unwrap();
        receiver.await.unwrap()
    }

    pub async fn get_handler(&self, coordinates: ChunkCoordinates) -> Result<HandlerData, Error> {
        debug!("BMC- get_handler");
        let (sender, receiver) = oneshot::channel();
        self.board_manager_tx
            .send(BoardManagerMessage::GetHandler(coordinates, sender))
            .await
            .unwrap();
        receiver.await.unwrap()
    }
}

#[derive(Debug)]
pub struct BoardManager<T>
where
    T: ChunkLoaderSaver + 'static,
{
    /// the chunks currently managed my the BoardManager
    chunks: dashmap::DashMap<ChunkCoordinates, HandlerData>,

    /// The manager for updating the chunks, this is given to each chunk manager
    chunks_loader_saver: Arc<T>,

    // limit how many chunks are loaded at the same time
    chunks_loaded: AtomicU64,

    /// The chunk manager will tell the BoardManager when it needs to be removed from chunks.
    chunk_m_updates_rx: mpsc::Receiver<ChunkUpdate>,
    /// Pass to ChunkManager when creating to be able to receive updates
    chunk_m_updates_tx: mpsc::Sender<ChunkUpdate>,

    /// The board manager receives messages from BoardManagerCommunicator
    board_manager_rx: tokio::sync::mpsc::Receiver<BoardManagerMessage>,
}

impl<T> BoardManager<T>
where
    T: ChunkLoaderSaver + 'static,
{
    pub fn start(chunks_loader_saver: T) -> BoardManagerCommunicator {
        let (board_manager_tx, board_manager_rx) = tokio::sync::mpsc::channel(100);
        let (chunk_updates_tx, chunk_updates_rx) = mpsc::channel(100);

        let board_manager = Self {
            chunks: dashmap::DashMap::new(),
            chunks_loaded: 0.into(),
            chunks_loader_saver: Arc::new(chunks_loader_saver),
            chunk_m_updates_rx: chunk_updates_rx,
            board_manager_rx,
            chunk_m_updates_tx: chunk_updates_tx,
        };

        // start the board manager
        tokio::spawn(async move {
            board_manager.run().await;
        });

        // the communicator is how the Appstate talks to the BoardManager
        let board_manager_communicator = BoardManagerCommunicator {
            board_manager_tx: board_manager_tx.clone(),
        };

        board_manager_communicator
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                // The BoardManagerCommunicator wants to talk to you
                message=self.board_manager_rx.recv()=>{
                    match message {
                        Some(BoardManagerMessage::GetChunk(coordinates, request_type, sender)) => {
                            debug!("BM - GetChunk request {:?}:{:?}", coordinates, request_type);
                            let chunk = self.read_chunk(coordinates, request_type).await;
                            let _ = sender.send(chunk);
                        }
                        Some(BoardManagerMessage::GetHandler(coordinates, sender)) => {
                            debug!("BM - GetHandler request {:?}", coordinates);
                            let handler = self.get_chunk_handler(coordinates);
                            let _ = sender.send(handler);
                        }
                        None => {
                            panic!("Board manager is closed")
                        }
                    }
                }
                // The chunk wants to talk to you
                msg = self.chunk_m_updates_rx.recv()=>{
                    match msg {
                        Some(f) => match f {
                            ChunkUpdate::Clear(coords) => {
                                // remove this ChunkManager from the Map of active chunks.
                                // ! it needs to save itself before sending this message
                                let _ = self.chunks.remove(&coords);
                            }
                            ChunkUpdate::Save => {
                                // ? Is probably better if Chunks just save themselves instead of the BoardManager
                            }
                        },
                        None => panic!("Board manager is holding a sender, yet all senders are dropped?"),
                    }
                }

            }
        }
        debug!("BM - Stopping");
    }

    pub async fn read_chunk(
        &self,
        coordinates: ChunkCoordinates,
        request_type: ChunkRequest,
    ) -> Option<Chunk> {
        match request_type {
            ChunkRequest::Storage => self
                .chunks_loader_saver
                .load_chunk(coordinates)
                .map_err(|err| {
                    error!("loading error setting default: {:?}", err);
                })
                .ok(),

            ChunkRequest::Live => {
                let handler = self
                    .chunks
                    .get(&coordinates)
                    .map(|entry| entry.value().clone())?;
                Some(handler.fetch_chunk().await)
            }
        }
    }

    // get the data neccesary for a handler to start
    pub fn get_chunk_handler(&self, coordinates: ChunkCoordinates) -> Result<HandlerData, Error> {
        let handler = self
            .chunks
            .entry(coordinates)
            .or_try_insert_with(|| {
                if self.chunks_loaded() < 100 {
                    debug!("Creating new ChunkManager");

                    self.chunks_loaded
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    Ok(ChunkManager::new(
                        coordinates,
                        self.chunks_loader_saver.clone(),
                        self.chunk_m_updates_tx.clone(),
                    ))
                } else {
                    debug!("Too many chunks loaded");
                    // return Error::TooManyChunksLoaded;
                    Err(Error::TooManyChunksLoaded)
                }
            })
            .map(|handler| handler.value().clone())?;

        Ok(handler)
    }

    fn chunks_loaded(&self) -> u64 {
        self.chunks_loaded.load(std::sync::atomic::Ordering::SeqCst)
    }
}
