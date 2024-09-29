/// create a screenshot of chunks
///
///
///
use crate::types::*;

struct Screenshot {
    pub chunks: Vec<Vec<Chunk>>,
}

impl Screenshot {
    pub fn new(chunks: Vec<Vec<Chunk>>) -> Self {
        Self { chunks }
    }

    pub fn save(&self) {
        // todo
        // each
    }
}
