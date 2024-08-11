use std::{io::Write, os::unix::fs::MetadataExt};

const CHUNKSIZE: usize = 100;
// const CHUNKSIZE: usize = 1_024;
const CHUNKDATA_SIZE: usize = CHUNKSIZE * CHUNKSIZE / 2;
const CHUNKINDEX_SIZE: usize = CHUNKSIZE / 2;

const CHUNKS_IN_DIRECTION: usize = 40000;
const CHUNKS_IN_MAP: usize = CHUNKS_IN_DIRECTION * CHUNKS_IN_DIRECTION;

// TODO:
// small chunks, such that you can increase cache rate and lower data being discovered
// - you can put limits on the quantity of chunks per minute you can move to.
// - you can limit squares changed
// empty chunks which only have less than 10 values after a week

#[test]
fn calculate_stuff() {
    // maximun value of u21
    let num: usize = (1 << 13) - 1;
    println!("u13: {}", num);

    // let CHUNKSIZE = 1_024u128;

    println!("u20: {}", CHUNKS_IN_DIRECTION);
    println!(
        "playable area: {}x{}",
        CHUNKS_IN_DIRECTION * CHUNKSIZE,
        CHUNKS_IN_DIRECTION * CHUNKSIZE
    );
    let bytes = CHUNKS_IN_MAP * CHUNKDATA_SIZE;
    println!("bytes for a map: {}", bytes);
    println!(
        "terrabits for a map: {:.4}",
        bytes as f64 / 1024.0 / 1024.0 / 1024.0 / 1024.0
    );
    println!();
    // size of each chunk
    println!("chunk size: {}", CHUNKSIZE);
    println!("data size: {}", CHUNKDATA_SIZE);
    println!(
        "data size MB: {:.4}",
        (CHUNKDATA_SIZE) as f64 / 1024.0 / 1024.0
    );

    println!();
    let mut chunk = Chunk::new();

    // data at set indexes
    for i in 0..CHUNKSIZE {
        for j in 0..CHUNKSIZE {
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

    // open file, decompress data anc check if it is the same as the original
    let compressed = std::fs::read("chunk.lz4").unwrap();
    let decompressed = lz4_flex::decompress(&compressed, CHUNKDATA_SIZE).unwrap();

    let mut chunk2 = Chunk::from_slice(&decompressed);

    // chunk2.set(11, 1, 1);
    // check if the data is the same
    for i in 0..CHUNKINDEX_SIZE {
        for j in 0..CHUNKINDEX_SIZE {
            let value = chunk.get(i, j);
            let value2 = chunk2.get(i, j);
            // println!("{} {} {} {}", i, j, value, value2);
            if value != value2 {
                println!("error at index: {} {} {} {}", i, j, value, value2);
            }
        }
    }
}

struct Chunk {
    data: [u8; CHUNKDATA_SIZE],
}

impl Chunk {
    fn new() -> Self {
        Chunk {
            data: [0; CHUNKDATA_SIZE],
        }
    }

    fn from_slice(data: &[u8]) -> Self {
        let mut packed = Chunk::new();
        packed.data.copy_from_slice(data);
        packed
    }

    fn get(&self, x: usize, y: usize) -> u8 {
        let index = x + y * CHUNKSIZE;
        let byte_index = index / 2;
        let is_high_nibble = index % 2 == 0;

        if is_high_nibble {
            (self.data[byte_index] & 0xF0) >> 4
        } else {
            self.data[byte_index] & 0x0F
        }
    }

    fn set(&mut self, x: usize, y: usize, value: u8) {
        let index = x + y * CHUNKSIZE;
        let byte_index = index / 2;
        if byte_index >= CHUNKDATA_SIZE {
            panic!("index out of bounds");
        }
        let is_high_nibble = index % 2 == 0;

        // println!(
        //     "Before: data[{}] = {:08b}",
        //     byte_index, self.data[byte_index]
        // );

        if is_high_nibble {
            self.data[byte_index] = (self.data[byte_index] & 0x0F) | (value << 4);
        } else {
            self.data[byte_index] = (self.data[byte_index] & 0xF0) | (value & 0x0F);
        }

        // println!(
        //     "After: data[{}] = {:08b}",
        //     byte_index, self.data[byte_index]
        // );
    }
}

#[test]
fn test_packed_u4_array() {
    let chunk1 = Chunk::new();
    let mut chunk2 = Chunk::new();

    println!("Before: {:08b}", chunk2.data[0]);
    chunk2.set(11, 1, 1);
    println!("After: {:08b}", chunk2.data[0]);

    for i in 0..CHUNKINDEX_SIZE {
        for j in 0..CHUNKINDEX_SIZE {
            let value = chunk1.get(i, j);
            let value2 = chunk2.get(i, j);
            // println!("{} {} {} {}", i, j, value, value2);
            if i == 11 && j == 1 {
                assert_eq!(value2, 1);
            } else {
                assert_eq!(value, value2);
            }
            if value != value2 {
                println!("error at index: {} {} {} {}", i, j, value, value2);
            }
        }
    }
}
