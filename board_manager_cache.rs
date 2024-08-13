use std::{
    sync::{atomic::AtomicI64, Arc},
    thread::sleep,
    time::Duration,
};

use moka::future::Cache;
use tokio::sync::{broadcast, mpsc};

use crate::types::*;

#[derive(Debug)]
struct ChunkManager {
    coordinates: ChunkCoordinates,

    chunk: Chunk,

    broadcaster_rx: broadcast::Sender<Vec<PackedCell>>,
    update_rx: mpsc::Receiver<PackedCell>,

    // stuff to send to the clients
    handler_data: HandlerData,
}

impl ChunkManager {
    fn new(coordinates: ChunkCoordinates) -> Self {
        let (update_tx, update_rx) = mpsc::channel(100);
        let (broadcaster_rx, broadcast_rx) = broadcast::channel(100);

        Self {
            chunk: Chunk::new(coordinates),
            coordinates,
            broadcaster_rx,
            update_rx,
            handler_data: HandlerData {
                broadcast_rx,
                update_tx,
            },
        }
    }
}

#[derive(Debug)]
struct HandlerData {
    broadcast_rx: broadcast::Receiver<Vec<PackedCell>>,
    update_tx: mpsc::Sender<PackedCell>,
}

impl Clone for HandlerData {
    fn clone(&self) -> Self {
        Self {
            broadcast_rx: self.broadcast_rx.resubscribe(),
            update_tx: self.update_tx.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct BoardManager {
    chunks: Cache<ChunkCoordinates, Arc<ChunkManager>>,

    #[cfg(test)]
    id: Arc<AtomicI64>,
}

impl BoardManager {
    fn new() -> Self {
        Self {
            chunks: Cache::builder()
                .max_capacity(100) // Set the max capacity based on your RAM size
                .build(),

            #[cfg(test)]
            id: Arc::new(AtomicI64::new(0)),
        }
    }

    async fn get_chunk_values(&self, coordinates: ChunkCoordinates) -> HandlerData {
        let chunk = self
            .chunks
            .entry(coordinates)
            .or_insert_with(|| Arc::new(ChunkManager::new(coordinates)));

        chunk.value().handler_data.clone()
    }

    #[cfg(test)]
    fn get_or_create_chunkmanager(
        &self,
        coordinates: ChunkCoordinates,
        id: i64,
    ) -> ChunkCoordinates {
        let mut inserted_id = None;
        let chunk = self.chunks.entry(coordinates).or_insert_with(|| {
            println!("creating new chunk manager for {:?}", id);
            inserted_id = Some(id);
            Arc::new(ChunkManager::new(ChunkCoordinates::new(id, id)))
        });

        if let Some(id) = inserted_id {
            self.id.store(id, std::sync::atomic::Ordering::Relaxed);
        }

        chunk.value().coordinates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(unhandled_panic = "shutdown_runtime")]
    async fn test_get_or_create_chunk_manager() {
        let manager = Arc::new(BoardManager {
            chunks: dashmap::DashMap::new(),
            id: Arc::new(0.into()),
        });

        let coordinates = ChunkCoordinates::new(0, 0);

        let mut stuff = Vec::new();

        // spawn 2000 threads to get or create the chunk manager
        for i in 5..2000 {
            let m_clone = manager.clone();
            stuff.push(tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(rand::random::<u64>() % 10))
                    .await;

                let chunk = m_clone.get_or_create_chunkmanager(coordinates, i);

                let id = m_clone.id.load(std::sync::atomic::Ordering::Relaxed);

                assert_eq!(chunk.x(), id);

                tokio::time::sleep(std::time::Duration::from_millis(rand::random::<u64>() % 10))
                    .await;
                let chunk = m_clone.get_or_create_chunkmanager(coordinates, i);
                let id = m_clone.id.load(std::sync::atomic::Ordering::Relaxed);

                assert_eq!(chunk.x(), id);

                // assert_eq!(5, 20);
            }));
        }

        futures::future::join_all(stuff).await;
    }
}
