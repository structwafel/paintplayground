use std::{
    fs::File,
    io::{Read, Write},
};

use crate::types::*;
use s3::{creds::Credentials, error::S3Error};

#[trait_variant::make(ChunkLoaderSaver: Send)]
pub trait LocalChunkLoaderSaver: Send + Sync + Debug {
    async fn save_chunk(
        &self,
        chunk: Chunk,
        coordinates: ChunkCoordinates,
    ) -> Result<(), ChunkLoaderSaverError>;

    async fn load_chunk(
        &self,
        coordinates: ChunkCoordinates,
        create_new: bool,
    ) -> Result<Chunk, ChunkLoaderSaverError>;
}

#[derive(Debug)]
pub enum ChunkLoaderSaverError {
    ChunkLoadError(String),
    ChunkSaveError(String),
    CompressionError(String),
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
        format!("canvas/{}", coordinates.object_name())
    }
}

/// Saves in canvas dir
impl ChunkLoaderSaver for SimpleToFileSaver {
    async fn save_chunk(
        &self,
        chunk: Chunk,
        coordinates: ChunkCoordinates,
    ) -> Result<(), ChunkLoaderSaverError> {
        debug!("Saving chunk at {:?}", coordinates);
        let mut file = File::create(self.file_path(coordinates)).unwrap();

        file.write_all(&chunk.to_storage_bytes(USED_COMPRESSION))
            .unwrap();

        Ok(())
    }

    async fn load_chunk(
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
                )));
            }
        };

        Ok(match buf {
            Some(data) => Chunk::from_raw_data(&data)
                .map_err(|err| ChunkLoaderSaverError::CompressionError(err.to_string()))?,
            None => Chunk::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CFR2ChunkSaver {
    client: Box<s3::Bucket>,
}
impl CFR2ChunkSaver {
    pub fn new(
        access_key_id: &str,
        secret_access_key: &str,
        account_id: &str,
        bucket: &str,
    ) -> Self {
        let credentials = Credentials::new(
            Some(access_key_id),
            Some(secret_access_key),
            None,
            None,
            None,
        )
        .unwrap();

        let client = s3::Bucket::new(
            bucket,
            s3::Region::R2 {
                account_id: account_id.to_string(),
            },
            credentials,
        )
        .unwrap();
        CFR2ChunkSaver { client: client }
    }

    pub fn new_from_env() -> Self {
        let access_key_id = std::env::var("S3ACCESSKEY").unwrap();
        let secret_access_key = std::env::var("S3SECRETACCESSKEY").unwrap();
        let account_id = std::env::var("S3ACCOUNTID").unwrap();
        let bucket = std::env::var("S3BUCKETNAME").unwrap();

        CFR2ChunkSaver::new(&access_key_id, &secret_access_key, &account_id, &bucket)
    }

    fn object_path(coordinates: ChunkCoordinates) -> String {
        format!("chunks/{}", coordinates.object_name())
    }
}

impl ChunkLoaderSaver for CFR2ChunkSaver {
    async fn save_chunk(
        &self,
        chunk: Chunk,
        coordinates: ChunkCoordinates,
    ) -> Result<(), ChunkLoaderSaverError> {
        self.client
            .put_object(
                Self::object_path(coordinates),
                &chunk.to_storage_bytes(USED_COMPRESSION),
            )
            .await
            .map_err(|err| ChunkLoaderSaverError::ChunkSaveError(err.to_string()))?;

        Ok(())
    }

    async fn load_chunk(
        &self,
        coordinates: ChunkCoordinates,
        create_new: bool,
    ) -> Result<Chunk, ChunkLoaderSaverError> {
        match self.client.get_object(Self::object_path(coordinates)).await {
            Ok(result) => {
                // return the chunk
                Ok(Chunk::from_raw_data(result.as_slice())
                    .map_err(|err| ChunkLoaderSaverError::CompressionError(err.to_string()))?)
            }
            Err(S3Error::HttpFailWithBody(404, _)) => {
                if create_new {
                    Ok(Chunk::new())
                } else {
                    Err(ChunkLoaderSaverError::ChunkLoadError(
                        "chunk not found".into(),
                    ))
                }
            }
            Err(err) => Err(ChunkLoaderSaverError::ChunkLoadError(format!(
                "Error loading chunk from R2 at {:?}: {:?}",
                coordinates, err
            ))),
        }
    }
}

#[cfg(test)]
mod testing {

    use crate::chunk_db;

    use super::*;
    use crate::types::SmallChunkArray;

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
    #[tokio::test]
    async fn chunk_loading_saving() {
        let mut chunk = Chunk::default();
        let coordinates = ChunkCoordinates::new(0, 0).unwrap();

        // edit some values in the chunk
        chunk[0].set_left(Color::Ten);
        chunk[CHUNK_BYTE_SIZE - 1].set_right(Color::Eight);
        chunk[CHUNK_BYTE_SIZE / 2].set_left(Color::One);

        let saver = SimpleToFileSaver::new();
        let _ = chunk_db::ChunkLoaderSaver::save_chunk(&saver, chunk.clone(), coordinates).await;
        // saver.save_chunk(chunk.clone(), coordinates).await;

        let loaded_chunk = chunk_db::ChunkLoaderSaver::load_chunk(&saver, coordinates, true)
            .await
            .unwrap();

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

    // test to see if r2 works
    #[tokio::test]
    async fn test_r2_bucket() {
        dotenvy::dotenv().unwrap();

        let loader = CFR2ChunkSaver::new_from_env();

        let chunk = chunk_db::ChunkLoaderSaver::load_chunk(
            &loader,
            ChunkCoordinates::new(10, 10).unwrap(), // 10,10 doesn't exist
            false,
        )
        .await;

        assert!(chunk.is_err());

        let mut new_chunk = Chunk::new();
        new_chunk[0].set_left(Color::One);

        let _ = chunk_db::ChunkLoaderSaver::save_chunk(
            &loader,
            new_chunk.clone(),
            ChunkCoordinates::new(10, 0).unwrap(),
        )
        .await;

        // let's try and get it again
        let chunk = chunk_db::ChunkLoaderSaver::load_chunk(
            &loader,
            ChunkCoordinates::new(10, 0).unwrap(),
            false,
        )
        .await;

        assert!(chunk.is_ok());
        let chunk = chunk.unwrap();

        assert_eq!(new_chunk[0], chunk[0])
    }
}
