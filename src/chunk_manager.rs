use std::{error::Error, time::Duration};

use tracing::error;

use paintplayground::{chunk_db::ChunkLoaderSaver, types::*};

pub enum ChunkUpdate {
    /// The chunk manager doesn't have any clients, and can be removed
    Clear(ChunkCoordinates),
    /// Enough updates have been made to the chunk, and it should be saved
    _Save,
}

#[derive(Debug)]
pub struct ChunkManager<T>
where
    T: ChunkLoaderSaver + 'static,
{
    /// for which coordinates this chunk is
    coordinates: ChunkCoordinates,

    /// data of the chunk
    chunk: Chunk,
    /// chunk saver
    chunk_saver: Arc<T>,

    /// broadcast updates to all websockets connections
    broadcaster_tx: broadcast::Sender<Vec<PackedCell>>,
    /// receive updates from the websockets
    update_rx: mpsc::Receiver<Vec<PackedCell>>,

    /// Websockets can make requests to the manager
    chunk_requester_rx: mpsc::Receiver<oneshot::Sender<Chunk>>,
    /// Pinging to know if the ChunkManager is still alive
    ping_chunk_requester_rx: mpsc::Receiver<oneshot::Sender<()>>,

    /// Send a message to the BoardManager, to tell you are ded.
    chunk_m_updates_tx: mpsc::Sender<ChunkUpdate>,

    /// Keep track when was the last change, or if no changes: when it started
    last_change: std::time::Instant,
}

impl<T> ChunkManager<T>
where
    T: ChunkLoaderSaver + 'static,
{
    /// Create a new chunk manager
    ///
    /// And start it in a new thread, where it will handle updates to/from the chunk
    pub fn create(
        coordinates: ChunkCoordinates,
        chunk_saver: Arc<T>,
        chunk_m_updates_tx: mpsc::Sender<ChunkUpdate>,
    ) -> HandlerData {
        let (update_tx, update_rx) = mpsc::channel(1000);
        let (broadcaster_tx, broadcast_rx) = broadcast::channel(1000);

        let (chunk_requester_tx, chunk_requester_rx) = mpsc::channel(100);
        let (ping_chunk_requester_tx, ping_chunk_requester_rx) = mpsc::channel(100);

        let handler_data = HandlerData {
            broadcast_rx,
            update_tx,
            chunk_requester_tx,
            ping_chunk_requester_tx,
        };

        debug!("Starting chunk manager for {:?}", coordinates);
        tokio::spawn(async move {
            let chunk = chunk_saver
                .load_chunk(coordinates, true)
                .await
                .map_err(|err| {
                    error!("loading error setting default: {:?}", err);
                })
                .unwrap_or_default();

            let chunk_manager = Self {
                chunk_saver,
                chunk,
                coordinates,
                broadcaster_tx,
                update_rx,
                chunk_requester_rx,
                ping_chunk_requester_rx,
                chunk_m_updates_tx,
                last_change: std::time::Instant::now(),
            };

            chunk_manager.run().await;
        });

        handler_data
    }

    pub async fn run(mut self) {
        // receive updates, and buffer them
        let mut changed;
        loop {
            let mut smaller_buffer = Vec::new();
            changed = false;

            let timeout = tokio::time::sleep(Duration::from_millis(*crate::CLEAR_BUFFER_INTERVAL));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    // handle updates from the websockets
                    Some(changes) = self.update_rx.recv() => {
                        self.last_change = std::time::Instant::now();
                        debug!("CH - {:?} got an update", self.coordinates);
                        // todo, for contested chunks, use chunk as buffer
                        smaller_buffer.extend(changes);
                        changed = true;
                    }
                    // handle requests from the websockets
                    Some(request) = self.chunk_requester_rx.recv() => {
                        debug!("CH - {:?} got a chunk request, responding...", self.coordinates);
                        request.send(self.chunk.clone()).unwrap();
                    }
                    // handle pings
                    Some(ping) = self.ping_chunk_requester_rx.recv() => {
                        debug!("CM - {:?} got a ping request, responding...", self.coordinates);
                        ping.send(()).unwrap();
                    }
                    _ = &mut timeout => {
                        // breaking so we need to empty the smaller_buffer
                        break;
                    }
                }
            }

            if !changed {
                // check if there are connections
                if self.no_connections() {
                    // check if there have been no changes past 5 minutes
                    if self.last_change.elapsed().as_secs() > 60 * 5 {
                        break;
                    }
                }
                continue;
            }

            // buffer and board are chunks, only the non-zero buffer values need to be set in the board
            // only take the last of each unique indes
            let mut last_changes: Vec<PackedCell> = Vec::with_capacity(smaller_buffer.len());
            {
                for change in smaller_buffer {
                    if let Some(last_change) = last_changes
                        .iter_mut()
                        .find(|c| c.index() == change.index())
                    {
                        *last_change = change;
                    } else {
                        last_changes.push(change);
                    }
                }
            }

            // apply the changes to the board
            {
                // debug!("CH - {:?} applying changes", self.coordinates);
                for changes in &last_changes {
                    self.chunk.apply_packed_cell(changes);
                }
            }

            // broadcast the changes made to all the clients
            self.broadcast(last_changes);

            // todo, only save every some time. and save on exit
            // save the chunk, if it has been changed
            let _ = self
                .chunk_saver
                .save_chunk(self.chunk.clone(), self.coordinates)
                .await;
        }

        // loop is stopped, lets destroy ourselves
        match self.delete_yourself().await {
            Ok(_) => {
                debug!("CM - {:?} is deleted", self.coordinates);
            }
            Err(error) => {
                // what do we do now?
                error!("I want to delete myself, but i errored doing so");
                panic!("{:?}", error);
                // self.run().await;
            }
        }
    }

    fn broadcast(&mut self, messages: Vec<PackedCell>) {
        debug!(
            "CM - {:?} is broadcasting {}",
            self.coordinates,
            messages.len()
        );
        self.broadcaster_tx.send(messages).unwrap();
    }

    async fn delete_yourself(&self) -> Result<(), Box<dyn Error>> {
        debug!("CM - {:?} is trying deleting itself", self.coordinates);
        // check if there are no websockets connected to you
        if self.update_rx.sender_weak_count() > 0 {
            return Err("There are senders".into());
        }

        // send request to BoardManager to remove yourself
        // if errors everything is ded
        let _ = self
            .chunk_m_updates_tx
            .send(ChunkUpdate::Clear(self.coordinates))
            .await?;

        // save the chunk, One LAST TIME
        let _ = self
            .chunk_saver
            .save_chunk(self.chunk.clone(), self.coordinates); // silent error, let's go

        // You can stop now
        return Ok(());
    }

    fn connections_quantity(&self) -> usize {
        self.update_rx.sender_strong_count()
    }
    fn has_connections(&self) -> bool {
        self.connections_quantity() > 0
    }
    fn no_connections(&self) -> bool {
        !self.has_connections()
    }
}

#[derive(Debug)]
pub struct HandlerData {
    pub broadcast_rx: broadcast::Receiver<Vec<PackedCell>>,
    pub update_tx: mpsc::Sender<Vec<PackedCell>>,

    pub chunk_requester_tx: mpsc::Sender<oneshot::Sender<Chunk>>,
    pub ping_chunk_requester_tx: mpsc::Sender<oneshot::Sender<()>>,
}

impl Clone for HandlerData {
    fn clone(&self) -> Self {
        Self {
            broadcast_rx: self.broadcast_rx.resubscribe(),
            update_tx: self.update_tx.clone(),

            chunk_requester_tx: self.chunk_requester_tx.clone(),
            ping_chunk_requester_tx: self.ping_chunk_requester_tx.clone(),
        }
    }
}

impl HandlerData {
    pub async fn fetch_chunk(&self) -> Chunk {
        // request the chunk from the chunk manager
        let (oneshot_chunk_tx, oneshot_chunk_rx) = oneshot::channel();
        self.chunk_requester_tx
            .send(oneshot_chunk_tx)
            .await
            .unwrap();

        // await on the oneshot the chunk
        oneshot_chunk_rx.await.unwrap()
    }

    pub async fn _is_alive(&self) -> bool {
        let (oneshot_ping_tx, mut oneshot_ping_rx) = oneshot::channel();
        self.ping_chunk_requester_tx
            .send(oneshot_ping_tx)
            .await
            .unwrap();

        oneshot_ping_rx.try_recv().is_ok()
    }
}
