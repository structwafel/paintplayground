use std::sync::{
    Arc,
    atomic::{AtomicI64, AtomicU64},
};

use crate::{
    chunk_db::ChunkLoaderSaver,
    chunk_manager::{ChunkManager, ChunkUpdate, HandlerData},
};
use paintplayground::types::*;

#[derive(thiserror::Error, Debug)]
pub enum BoardManagerError {
    #[error("too many chunks loaded")]
    TooManyChunksLoaded,
    #[error("failed to load chunk/chunks")]
    LoadingChunks,
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
    GetScreenshotChunks(
        ChunkCoordinates, // top_left
        ChunkCoordinates, // bottom_right
        oneshot::Sender<Result<Vec<Vec<Option<Chunk>>>, BoardManagerError>>,
    ),
    GetHandler(
        ChunkCoordinates,
        oneshot::Sender<Result<HandlerData, BoardManagerError>>,
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

    pub async fn get_handler(
        &self,
        coordinates: ChunkCoordinates,
    ) -> Result<HandlerData, BoardManagerError> {
        debug!("BMC- get_handler");
        let (sender, receiver) = oneshot::channel();
        self.board_manager_tx
            .send(BoardManagerMessage::GetHandler(coordinates, sender))
            .await
            .unwrap();
        receiver.await.unwrap()
    }

    pub async fn get_screenshot_chunks(
        &self,
        top_left: ChunkCoordinates,
        bottom_right: ChunkCoordinates,
    ) -> Result<Vec<Vec<Option<Chunk>>>, BoardManagerError> {
        let (sender, receiver) = oneshot::channel();

        self.board_manager_tx
            .send(BoardManagerMessage::GetScreenshotChunks(
                top_left,
                bottom_right,
                sender,
            ))
            .await
            .unwrap();

        let matrix_chunks = receiver.await.unwrap();
        matrix_chunks
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
    board_manager_rx: mpsc::Receiver<BoardManagerMessage>,
}

impl<T> BoardManager<T>
where
    T: ChunkLoaderSaver + 'static,
{
    pub fn start(chunks_loader_saver: T) -> BoardManagerCommunicator {
        let (board_manager_tx, board_manager_rx) = mpsc::channel(100);
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
                        Some(BoardManagerMessage::GetScreenshotChunks(top_left, bottom_right, sender))=>{
                            debug!("BM - GetScreenshotChunks request from {:?} to {:?}", top_left, bottom_right);

                            let chunks_map = self.chunks.clone();
                            let chunks_loader_saver = self.chunks_loader_saver.clone();

                            tokio::spawn(async move {
                                let chunks = Self::get_screenshot_chunks(
                                    &chunks_map,
                                    &chunks_loader_saver,
                                    top_left,
                                    bottom_right
                                ).await;

                                let _ = sender.send(Ok(chunks));
                            });


                        }
                        Some(BoardManagerMessage::GetChunk(coordinates, request_type, sender)) => {
                            debug!("BM - GetChunk request {:?}:{:?}", coordinates, request_type);
                            let chunk = Self::read_chunk(&self.chunks,&self.chunks_loader_saver, coordinates, request_type).await;
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
    }

    pub async fn read_chunk(
        chunks: &dashmap::DashMap<ChunkCoordinates, HandlerData>,
        chunks_loader_saver: &T,
        coordinates: ChunkCoordinates,
        request_type: ChunkRequest,
    ) -> Option<Chunk> {
        match request_type {
            ChunkRequest::Storage => chunks_loader_saver
                .load_chunk(coordinates, true)
                .await
                .map_err(|err| {
                    error!("loading error setting default: {:?}", err);
                })
                .ok(),

            ChunkRequest::Live => {
                let handler = chunks.get(&coordinates);

                if let Some(handler) = handler {
                    return Some(handler.fetch_chunk().await);
                } else {
                    // get from storage
                    let chunk = chunks_loader_saver
                        .load_chunk(coordinates, true)
                        .await
                        .map_err(|err| {
                            error!("loading error setting default: {:?}", err);
                        })
                        .ok()?;

                    return Some(chunk);
                }
            }
        }
    }

    // get the data neccesary for a handler to start
    pub fn get_chunk_handler(
        &self,
        coordinates: ChunkCoordinates,
    ) -> Result<HandlerData, BoardManagerError> {
        let handler = self
            .chunks
            .entry(coordinates)
            .or_try_insert_with(|| {
                if self.chunks_loaded() < 100 {
                    debug!("Creating new ChunkManager");

                    self.chunks_loaded
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    Ok(ChunkManager::create(
                        coordinates,
                        self.chunks_loader_saver.clone(),
                        self.chunk_m_updates_tx.clone(),
                    ))
                } else {
                    debug!("Too many chunks loaded");
                    // return Error::TooManyChunksLoaded;
                    Err(BoardManagerError::TooManyChunksLoaded)
                }
            })
            .map(|handler| handler.value().clone())?;

        Ok(handler)
    }

    fn chunks_loaded(&self) -> u64 {
        self.chunks_loaded.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn get_screenshot_chunks(
        chunks: &dashmap::DashMap<ChunkCoordinates, HandlerData>,
        chunks_loader_saver: &T,
        top_left: ChunkCoordinates,
        bottom_right: ChunkCoordinates,
    ) -> Vec<Vec<Option<Chunk>>> {
        let min_x = top_left.x().min(bottom_right.x());
        let max_x = top_left.x().max(bottom_right.x());
        let min_y = bottom_right.y().min(top_left.y());
        let max_y = bottom_right.y().max(top_left.y());

        let width = (max_x - min_x + 1) as usize;
        let height = (max_y - min_y + 1) as usize;

        let mut chunks_grid = Vec::with_capacity(height);

        // Collect all coordinates that need to be fetched
        let mut coordinates = Vec::with_capacity(width * height);

        for y in (min_y..=max_y).rev() {
            for x in min_x..=max_x {
                if let Ok(coordinate) = ChunkCoordinates::new(x, y) {
                    coordinates.push(coordinate);
                }
            }
        }

        // Fetch all chunks in parallel (using existing read_chunk function)
        let mut fetched_chunks = futures::future::join_all(coordinates.iter().map(|&coordinate| {
            Self::read_chunk(chunks, chunks_loader_saver, coordinate, ChunkRequest::Live)
        }))
        .await;

        // Organize the chunks into the grid
        for y in (min_y..=max_y).rev() {
            let mut row = Vec::with_capacity(width);
            for x in min_x..=max_x {
                if let Ok(coordinate) = ChunkCoordinates::new(x, y) {
                    // Find this coordinate in our fetched results
                    let position = coordinates.iter().position(|&c| c == coordinate);
                    if let Some(pos) = position {
                        row.push(fetched_chunks.remove(pos));
                        coordinates.remove(pos);
                    } else {
                        row.push(None);
                    }
                } else {
                    row.push(None);
                }
            }
            chunks_grid.push(row);
        }

        chunks_grid
    }
}
