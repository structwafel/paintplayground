use std::{
    fs::File,
    io::{Read, Write},
};

use paintplayground::types::*;

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
        debug!("Loading chunk from {:?}", path);

        let buf = match File::open(&path) {
            Ok(mut file) => {
                debug!("Chunk found at {:?}", coordinates);
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(|err| {
                    ChunkLoaderSaverError::ChunkLoadError(format!(
                        "Error reading chunk at {:?}: {:?}",
                        coordinates, err
                    ))
                })?;
                Some(buf)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if !create_new {
                    return Err(ChunkLoaderSaverError::ChunkLoadError(
                        "No chunk found".to_string(),
                    ));
                }
                debug!("Chunk not found, creating new chunk at {:?}", coordinates);
                None
            }
            Err(err) => {
                return Err(ChunkLoaderSaverError::ChunkLoadError(format!(
                    "Error loading chunk at {:?}: {:?}",
                    coordinates, err
                )))
            }
        };

        Ok(match buf {
            Some(data) => data.into(),
            None => Chunk::new(),
        })
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

#[cfg(test)]
mod testing {

    use super::*;
    use paintplayground::types::SmallChunkArray;

    // Initialize tracing subscriber

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn chunk_color_packed_values() {
        let mut chunk_color = ChunkColor::default();
        assert_eq!(chunk_color.left(), Color::Zero.u8());
        assert_eq!(chunk_color.right(), Color::Zero.u8());

        chunk_color.set_left(Color::Ten);

        assert_eq!(chunk_color.left(), Color::Ten.u8());
        // right should be untouched
        assert_eq!(chunk_color.right(), Color::Zero.u8());

        chunk_color.set_right(Color::Twelve);
        assert_eq!(chunk_color.right(), Color::Twelve.u8());
        // left should be untouchedm
        assert_eq!(chunk_color.left(), Color::Ten.u8());
    }

    // test if loading and saving the chunk gives you the same chunk
    #[test]
    fn chunk_loading_saving() {
        let mut chunk = Chunk::default();
        let coordinates = ChunkCoordinates::new(0, 0).unwrap();

        // edit some values in the chunk
        chunk[0].set_left(Color::Ten);
        chunk[CHUNK_BYTE_SIZE - 1].set_right(Color::Eight);
        chunk[CHUNK_BYTE_SIZE / 2].set_left(Color::One);

        let saver = SimpleToFileSaver::new();
        saver.save_chunk(chunk.clone(), coordinates);

        let loaded_chunk = saver.load_chunk(coordinates, true).unwrap();

        chunk.iter().zip(loaded_chunk.iter()).for_each(|(a, b)| {
            assert_eq!((a.left(), a.right()), (b.left(), b.right()),);
            // assert_eq!(a.right(), b.right(), "right numbers");
        });
    }

    // test to vec etc for chunk
    #[test]
    fn chunk_to_vec() {
        init_tracing();
        let mut chunk = SmallChunkArray::default();
        chunk[0].set_left(Color::Ten);
        chunk[1].set_right(Color::Eight);
        chunk[4].set_left(Color::One);

        let vec = chunk.clone().to_u8vec();
        let chunk2 = SmallChunkArray::from(vec);

        chunk.iter().zip(chunk2.iter()).for_each(|(a, b)| {
            assert_eq!((a.left(), a.right()), (b.left(), b.right()),);
        });
    }
}
