use crate::types::*;

// todo, these should probably return errors
pub trait ChunkLoaderSaver: Send + Sync + Debug {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates);
    fn load_chunk(&self, coordinates: ChunkCoordinates) -> Option<Chunk>;
}

#[derive(Debug)]
pub enum ChunkLoaderSavers {
    SimpleToFileSaver(SimpleToFileSaver),
    SimpleInMemoryLoader(SimpleInMemoryLoader),
}

impl ChunkLoaderSaver for ChunkLoaderSavers {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates) {
        match self {
            ChunkLoaderSavers::SimpleToFileSaver(s) => s.save_chunk(chunk, coordinates),
            ChunkLoaderSavers::SimpleInMemoryLoader(s) => s.save_chunk(chunk, coordinates),
        }
    }

    fn load_chunk(&self, coordinates: ChunkCoordinates) -> Option<Chunk> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct SimpleToFileSaver {}

impl ChunkLoaderSaver for SimpleToFileSaver {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates) {}

    fn load_chunk(&self, coordinates: ChunkCoordinates) -> Option<Chunk> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct SimpleInMemoryLoader {}
impl ChunkLoaderSaver for SimpleInMemoryLoader {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates) {}

    fn load_chunk(&self, coordinates: ChunkCoordinates) -> Option<Chunk> {
        todo!()
    }
}
