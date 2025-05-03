use std::io::{Read, Write};

use crate::types::{CHUNK_BYTE_SIZE, InnerChunk};

pub trait Compression {
    fn compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error>;
    fn decompress(data: &[u8], expected_size: usize) -> Result<Vec<u8>, std::io::Error>;
}

pub struct GzipCompression;
pub struct LZ4Compression;
pub struct ZstdCompression;

impl Compression for GzipCompression {
    fn compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        encoder.finish()
    }

    fn decompress(compressed: &[u8], _expected_size: usize) -> Result<Vec<u8>, std::io::Error> {
        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(compressed);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}

impl Compression for LZ4Compression {
    fn compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        Ok(lz4_flex::block::compress(data))
    }

    fn decompress(data: &[u8], _expected_size: usize) -> Result<Vec<u8>, std::io::Error> {
        let mut output = [0u8; CHUNK_BYTE_SIZE];
        lz4_flex::block::decompress_into(data, &mut output).unwrap();
        Ok(output.into())
    }
}

impl Compression for ZstdCompression {
    fn compress(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        zstd::bulk::compress(data, 1)
    }

    fn decompress(data: &[u8], _expected_size: usize) -> Result<Vec<u8>, std::io::Error> {
        zstd::bulk::decompress(data, CHUNK_BYTE_SIZE)
    }
}
pub trait ChunkCompression {
    fn compress_with<C: Compression>(self) -> Result<Vec<u8>, std::io::Error>;
    fn decompress_with<C: Compression>(data: &[u8]) -> Result<Self, std::io::Error>
    where
        Self: Sized;
}

impl<const N: usize> ChunkCompression for InnerChunk<N> {
    fn compress_with<C: Compression>(self) -> Result<Vec<u8>, std::io::Error> {
        let data = self.to_u8vec();
        C::compress(&data)
    }

    fn decompress_with<C: Compression>(data: &[u8]) -> Result<Self, std::io::Error> {
        let decompressed = C::decompress(data, N)?;
        Ok(Self::from(decompressed))
    }
}
