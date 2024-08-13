use std::time::Duration;

use crate::{chunk_db::ChunkLoaderSaver, types::*};

pub enum ChunkUpdate {
    /// The chunk manager doesn't have any clients, and can be removed
    Clear,
    /// Enough updates have been made to the chunk, and it should be saved
    Save,
}

#[derive(Debug)]
pub struct ChunkManager {
    /// for which coordinates this chunk is
    coordinates: ChunkCoordinates,

    /// data of the chunk
    chunk: Chunk,
    /// chunk saver
    chunk_saver: Arc<dyn ChunkLoaderSaver>,

    /// broadcast updates to all websockets connections
    broadcaster_tx: broadcast::Sender<Vec<PackedCell>>,
    /// receive updates from the websockets
    update_rx: mpsc::Receiver<PackedCell>,

    /// Websockets can make requests to the manager
    chunk_requester_rx: mpsc::Receiver<oneshot::Sender<Chunk>>,
    /// Pinging to know if the ChunkManager is still alive
    ping_chunk_requester_rx: mpsc::Receiver<oneshot::Sender<()>>,
}

impl ChunkManager {
    /// Create a new chunk manager
    ///
    /// And start it in a new thread, where it will handle updates to/from the chunk
    pub fn new(
        coordinates: ChunkCoordinates,
        chunk_saver: Arc<dyn ChunkLoaderSaver>,
    ) -> HandlerData {
        let (update_tx, update_rx) = mpsc::channel(100);
        let (broadcaster_tx, broadcast_rx) = broadcast::channel(10);

        let (chunk_requester_tx, chunk_requester_rx) = mpsc::channel(10);
        let (ping_chunk_requester_tx, ping_chunk_requester_rx) = mpsc::channel(10);

        let handler_data = HandlerData {
            broadcast_rx,
            update_tx,
            chunk_requester_tx,
            ping_chunk_requester_tx,
        };

        let chunk_manager = Self {
            chunk_saver,
            chunk: Chunk::new(coordinates),
            coordinates,
            broadcaster_tx,
            update_rx,
            chunk_requester_rx,
            ping_chunk_requester_rx,
            // handler_data: HandlerData {
            // broadcast_rx,
            // update_tx,
            // },
        };

        tokio::spawn(async move { chunk_manager.run().await });

        handler_data
    }

    // pub fn handler_data(&self) -> HandlerData {
    //     self.handler_data.clone()
    // }

    pub fn coordinates(&self) -> ChunkCoordinates {
        self.coordinates
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    pub async fn run(mut self) {
        // recieve updates, and buffer them
        println!("Starting canvas manager");

        let mut changed;
        loop {
            let mut smaller_buffer = Vec::new();
            changed = false;
            let timeout = tokio::time::sleep(Duration::from_secs(crate::CLEAR_BUFFER_INTERVAL));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    // handle updates from the websockets
                    Some(change) = self.update_rx.recv() => {
                        smaller_buffer.push(change);
                        // buffer[change.index()] = change.value();
                        changed = true;
                    }
                    // handle requests from the websockets
                    Some(request) = self.chunk_requester_rx.recv() => {
                        request.send(self.chunk.clone()).unwrap();
                    }
                    // handle pings
                    Some(ping) = self.ping_chunk_requester_rx.recv() => {
                        ping.send(()).unwrap();
                    }
                    _ = &mut timeout => {
                        // breaking so we need to empty the smaller_buffer
                        break;
                    }
                }
            }

            if !changed {
                continue;
            }
            println!("Size of smaller buffer {}", smaller_buffer.len());

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
                for changes in &last_changes {
                    let packed_index = changes.index();

                    let byte_index = packed_index / 2;
                    let is_left = packed_index % 2 == 0;

                    if is_left {
                        self.chunk[byte_index].set_left(changes.value().into());
                    } else {
                        self.chunk[byte_index].set_right(changes.value().into());
                    }
                }
            }

            // broadcast the changes made to all the clients
            self.broadcast(last_changes);
        }
    }

    fn broadcast(&mut self, messages: Vec<PackedCell>) {
        self.broadcaster_tx.send(messages).unwrap();
    }
}

#[derive(Debug)]
pub struct HandlerData {
    pub broadcast_rx: broadcast::Receiver<Vec<PackedCell>>,
    pub update_tx: mpsc::Sender<PackedCell>,

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

    pub async fn is_alive(&self) -> bool {
        let (oneshot_ping_tx, mut oneshot_ping_rx) = oneshot::channel();
        self.ping_chunk_requester_tx
            .send(oneshot_ping_tx)
            .await
            .unwrap();

        oneshot_ping_rx.try_recv().is_ok()
    }
}
