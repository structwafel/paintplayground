use std::{
    fs::File,
    io::{Read, Write},
};

use crate::types::*;

// todo, these should probably return errors
pub trait ChunkLoaderSaver: Send + Sync + Debug {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates);
    fn load_chunk(&self, coordinates: ChunkCoordinates) -> Result<Chunk, ChunkLoaderSaverError>;
}

#[derive(Debug)]
pub enum ChunkLoaderSaverError {
    ChunkLoadError(String),
    ChunkSaveError,
}

#[derive(Debug, Clone)]
pub struct SimpleToFileSaver {}

impl SimpleToFileSaver {
    pub fn new() -> Self {
        Self {}
    }

    fn file_path(&self, coordinates: ChunkCoordinates) -> String {
        format!("canvas/{}-{}.chunk", coordinates.x(), coordinates.y())
    }
}

/// Saves in canvas dir
impl ChunkLoaderSaver for SimpleToFileSaver {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates) {
        // save the chunk to the file system
        debug!("Saving chunk at {:?}", coordinates);
        let mut file = File::create(self.file_path(coordinates)).unwrap();
        file.write_all(&chunk.to_u8vec()).unwrap();
    }

    fn load_chunk(&self, coordinates: ChunkCoordinates) -> Result<Chunk, ChunkLoaderSaverError> {
        // load a chunk from the file system, if it doesn't exist create a new one
        debug!("Loading chunk at {:?}", coordinates);

        let mut file = File::open(self.file_path(coordinates)).map_err(|err| {
            ChunkLoaderSaverError::ChunkLoadError(format!(
                "Error loading chunk at {:?}: {:?}",
                coordinates, err
            ))
        })?;

        // read the file
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|err| {
            ChunkLoaderSaverError::ChunkLoadError(format!(
                "Error reading chunk at {:?}: {:?}",
                coordinates, err
            ))
        })?;
        Ok(buf.into())
    }
}

#[derive(Debug, Clone)]
pub struct SimpleInMemoryLoader {}
impl ChunkLoaderSaver for SimpleInMemoryLoader {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates) {}

    fn load_chunk(&self, coordinates: ChunkCoordinates) -> Result<Chunk, ChunkLoaderSaverError> {
        todo!()
    }
}
