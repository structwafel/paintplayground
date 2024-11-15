use std::{
    fs::File,
    io::{Read, Write},
};

use crate::types::*;

// todo, these should probably return errors
pub trait ChunkLoaderSaver: Send + Sync + Debug {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates);
    fn load_chunk(
        &self,
        coordinates: ChunkCoordinates,
        create_new: bool,
    ) -> Result<Chunk, ChunkLoaderSaverError>;
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
        // if there is no canvas dir, create it
        std::fs::create_dir_all("canvas").unwrap();

        Self {}
    }

    fn file_path(&self, coordinates: ChunkCoordinates) -> String {
        format!("canvas/{}_{}.chunk", coordinates.x(), coordinates.y())
    }
}

// todo use compression for smaller saved files
/// Saves in canvas dir
impl ChunkLoaderSaver for SimpleToFileSaver {
    fn save_chunk(&self, chunk: Chunk, coordinates: ChunkCoordinates) {
        // save the chunk to the file system
        debug!("Saving chunk at {:?}", coordinates);
        let mut file = File::create(self.file_path(coordinates)).unwrap();
        // use best compression before saving
        // let compressed = chunk.to_compressed();
        file.write_all(&chunk.to_u8vec()).unwrap();
    }

    fn load_chunk(
        &self,
        coordinates: ChunkCoordinates,
        create_new: bool,
    ) -> Result<Chunk, ChunkLoaderSaverError> {
        // load a chunk from the file system, if it doesn't exist create a new one
        debug!("Loading chunk at {:?}", coordinates);

        let path = self.file_path(coordinates);
        info!("Loading chunk from {:?}", path);

        let mut file = match File::open(self.file_path(coordinates)) {
            Ok(f) => {
                info!("Chunk found at {:?}", coordinates);
                f
            }
            Err(err) => match err {
                err if err.kind() == std::io::ErrorKind::NotFound => {
                    if !create_new {
                        return Err(ChunkLoaderSaverError::ChunkLoadError(format!(
                            "No chunk found"
                        )));
                    }
                    debug!("Chunk not found, creating new chunk at {:?}", coordinates);
                    return Ok(Chunk::new());
                }
                _ => {
                    return Err(ChunkLoaderSaverError::ChunkLoadError(format!(
                        "Error loading chunk at {:?}: {:?}",
                        coordinates, err
                    )))
                }
            },
        };

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

    fn load_chunk(
        &self,
        coordinates: ChunkCoordinates,
        create_new: bool,
    ) -> Result<Chunk, ChunkLoaderSaverError> {
        todo!()
    }
}
