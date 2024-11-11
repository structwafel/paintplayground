/// create a screenshot of chunks
///
///
///
use crate::types::*;

struct Screenshot {
    /// the chunks in top left to bottom right order
    pub chunks: Vec<Vec<Option<Chunk>>>,
}

impl Screenshot {
    pub fn new(chunks: Vec<Vec<Option<Chunk>>>) -> Self {
        Self { chunks }
    }

    pub fn save(&self) {
        // todo
        // we need to iterate from CHUNKS_IN_DIRECTION to CHUNKS_IN_DIRECTION

        for i in 0..*CHUNKS_IN_DIRECTION {}
    }
}

#[cfg(test)]
mod tests {
    use crate::chunk_db::{ChunkLoaderSaver, SimpleToFileSaver};
    use crate::types::*;

    use super::ChunkCoordinates;

    #[test]
    fn all_canvasases() {
        let loader = SimpleToFileSaver::new();

        let mut chunks: Vec<Vec<Option<Chunk>>> = vec![];

        // first left-top and right top

        for y in (-*CHUNKS_IN_DIRECTION..=*CHUNKS_IN_DIRECTION).rev() {
            let mut row: Vec<Option<Chunk>> = vec![];
            for x in -*CHUNKS_IN_DIRECTION..=*CHUNKS_IN_DIRECTION {
                // println!("{};{}", x, y);
                let coordinate = ChunkCoordinates::new(x, y).unwrap();

                // todo, use a "direct loader orso", or a method to check if chunk exist.
                let chunk = loader.load_chunk(coordinate);
                match &chunk {
                    Ok(_) => (),
                    Err(_) => println!("errored"),
                }
                row.push(chunk.ok());
            }
            chunks.push(row);
        }

        let mut s = String::new();
        for row in &chunks {
            for c in row {
                match c {
                    Some(_) => s.push('C'),
                    None => s.push('.'),
                }
            }
            s.push_str("\n");
        }

        println!("{}", s)
    }
}
