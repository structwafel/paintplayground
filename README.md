# Paint Playground

A websocket service where everyone can contribute to sections of a complete playground


# Todo
- [x] s3 bucket instead of files
- [ ] make screenshot available on website

# Experiments
```sh
cargo run -r --example compres_chunks
```
Figure out what the best compression algo is. 

Zstd is a good compromise between speed and ration

# Design

The architecture is basically `Actor Model`.

Endpoints request to the `BoardManager` for channels to an `ChunkManager`.
Then through websocket the `ChunkManager` receives updates from all the connected clients on that chunk.

This was done such that each chunk will be processed in parallel with other chunks, making it even possible to make a service of the boardmanager and make the chunkmanagers distributed per region on the board.

```
      Client 
         |
    BoardManager
    /    |    \
  CM1   CM2   C23
```

## Storage

A chunk is currenty 100x100 pixels, meaning 10_000 individual pixels.
One optimization done is to limit the colour choices, such that one pixel is only 4bit. Making it possible to pack 10_000 pixels into 5_000 packed pixels. Which is only 5kb.

Storage is possible to local files, or to S3 bucket

### Compression

We can compress a chunk aswell, due to the expectation that not all chunks will be fully random.
The compression savings are quite good (depending on the chunk ofcourse. but between 2x - 15x).
Multiple compressions are supported, and the used compression is saved as the first byte (Format Header Byte)


Expected storage requirements for a "big" 1000x1000 board:

- 2x compression ratio  =>  2.5TB
- 5x compression ratio  =>    1TB
- 10x compression ratio =>  500GB

My expectation that we will hover around the 5x compression ratio.

But for 1_000_000 chunks there will probably be alot of undiscovered chunks, or non-random chunks.
