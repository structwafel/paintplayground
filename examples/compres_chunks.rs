use std::{fs, path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chunk_dir = PathBuf::from("canvas");
    let mut files = Vec::new();

    // Find all chunk files
    for entry in fs::read_dir(chunk_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().unwrap_or_default() == "chunk" {
            files.push(path);
        }
    }

    // center chunk, which is full random (after running clients on it)
    let center = [PathBuf::from("canvas/0_0.chunk")];

    println!("Found {} chunk files", files.len());

    //  compression methods on all available chunks
    benchmark_compression::<paintplayground::compression::GzipCompression>("Gzip", &files)?;
    benchmark_compression::<paintplayground::compression::LZ4Compression>("LZ4", &files)?;
    benchmark_compression::<paintplayground::compression::ZstdCompression>("Zstd", &files)?;

    // compression methods on center
    benchmark_compression::<paintplayground::compression::GzipCompression>("Gzip-center", &center)?;
    benchmark_compression::<paintplayground::compression::LZ4Compression>("LZ4-center", &center)?;
    benchmark_compression::<paintplayground::compression::ZstdCompression>("Zstd-center", &center)?;

    Ok(())
}

fn benchmark_compression<C: paintplayground::compression::Compression>(
    name: &str,
    files: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTesting {} compression:", name);

    let mut total_original_size = 0;
    let mut total_compressed_size = 0;
    let mut compress_time = std::time::Duration::default();
    let mut decompress_time = std::time::Duration::default();

    for path in files {
        let data = fs::read(path)?;
        total_original_size += data.len();

        // Benchmark compression
        let start = Instant::now();
        let compressed = C::compress(&data)?;
        compress_time += start.elapsed();

        total_compressed_size += compressed.len();

        // Benchmark decompression
        let start = Instant::now();
        let _decompressed = C::decompress(&compressed, data.len())?;
        decompress_time += start.elapsed();
    }

    println!("Total original size: {} bytes", total_original_size);
    println!("Total compressed size: {} bytes", total_compressed_size);
    println!(
        "Compression ratio: {:.2}x",
        total_original_size as f64 / total_compressed_size as f64
    );
    println!("Compression time: {:?}", compress_time);
    println!("Decompression time: {:?}", decompress_time);

    Ok(())
}
