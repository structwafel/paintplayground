use std::{io::Write, os::unix::fs::MetadataExt};

use paintplayground::types::{CHUNK_BYTE_SIZE, CHUNK_LENGTH, CHUNK_SIZE, Chunk};

const CHUNKS_IN_DIRECTION: usize = 20000;
const CHUNKS_IN_MAP: usize = CHUNKS_IN_DIRECTION * CHUNKS_IN_DIRECTION;

// TODO:
// small chunks, such that you can increase cache rate and lower data being discovered
// - you can put limits on the quantity of chunks per minute you can move to.
// - you can limit squares changed
// empty chunks which only have less than 10 values after a week

#[test]
fn calculate_stuff() {
    println!(
        "playable area: {}x{}",
        CHUNKS_IN_DIRECTION * CHUNK_SIZE,
        CHUNKS_IN_DIRECTION * CHUNK_SIZE
    );
    let bytes = CHUNKS_IN_MAP * CHUNK_BYTE_SIZE;
    println!("bytes for a map: {}", bytes);
    println!(
        "terrabits for a map: {:.4}",
        bytes as f64 / 1024.0 / 1024.0 / 1024.0 / 1024.0
    );
    println!();
    // size of each chunk
    println!("chunk size: {}", CHUNK_SIZE);
    println!("data size: {}", CHUNK_BYTE_SIZE);
    println!(
        "data size MB: {:.4}",
        (CHUNK_BYTE_SIZE) as f64 / 1024.0 / 1024.0
    );

    println!();
    let mut chunk = Chunk::new();
    let mut one_pixel_chunk = Chunk::new();
    chunk.set(0, 0, 1);

    // data at set indexes
    for i in 0..CHUNK_SIZE {
        for j in 0..CHUNK_SIZE {
            // random number between 0 and 15
            let value = rand::random::<u8>() % 16;

            chunk.set(i, j, value);
        }
    }

    // length of data
    println!("data length: {}", chunk.data.len());

    // save chunk to a file
    let mut file = std::fs::File::create("chunk.bin").unwrap();
    file.write_all(&chunk.data).unwrap();

    let file_bytes = std::fs::metadata("chunk.bin").unwrap().size();
    // print file size
    println!("file size: {}", file_bytes);
    println!("file size MB: {}", file_bytes as f64 / 1024.0 / 1024.0);

    // actual size for an entire map
    println!(
        "actual size for a map: {}",
        file_bytes as usize * CHUNKS_IN_MAP
    );
    println!(
        "actual size for a map TB: {}",
        (file_bytes as usize * CHUNKS_IN_MAP) as f64 / 1024.0 / 1024.0 / 1024.0 / 1024.0
    );

    // now compress the data and save it to a file using LZ4 or Zstd
    let compressed = lz4_flex::compress(&chunk.data);
    let mut file = std::fs::File::create("chunk.lz4").unwrap();
    file.write_all(&compressed).unwrap();

    println!();

    let file_bytes = std::fs::metadata("chunk.lz4").unwrap().size();
    // print file size
    println!("file size compress: {}", file_bytes);
    println!(
        "file size compress MB: {}",
        file_bytes as f64 / 1024.0 / 1024.0
    );

    // actual size for an entire map
    println!(
        "compress size for a map: {}",
        file_bytes as usize * CHUNKS_IN_MAP
    );
    println!(
        "compress size for a map TB: {}",
        (file_bytes as usize * CHUNKS_IN_MAP) as f64 / 1024.0 / 1024.0 / 1024.0 / 1024.0
    );
}
