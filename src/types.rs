pub use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;
pub use std::sync::Arc;

pub use tokio::sync::broadcast;
pub use tokio::sync::mpsc;
pub use tokio::sync::oneshot;

// re-export common used types
pub use crate::board_manager::ChunkRequest;

// pub type BoardRequester = mpsc::Sender<oneshot::Sender<Arc<RwLock<Chunk>>>>;

pub const CHUNK_LENGTH: usize = 100;
pub const CHUNK_SIZE: usize = CHUNK_LENGTH * CHUNK_LENGTH;
pub const CHUNK_BYTE_SIZE: usize = CHUNK_SIZE / 2;
// pub const CHUNKS_IN_DIRECTION: usize = 1_000;
// pub const CHUNKS: usize = CHUNKS_IN_DIRECTION * CHUNKS_IN_DIRECTION;

pub const MB: u64 = 1024 * 1024;
pub const CACHE_SIZE: u64 = 100 * MB;

/// Represents the possible colors of a cell.
pub enum Color {
    Grey,
    Red,
    Green,
    Blue,
    Yellow,
    Purple,
    Orange,
    Pink,
    Brown,
    Black,
}

impl Color {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Color::Grey,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Blue,
            4 => Color::Yellow,
            5 => Color::Purple,
            6 => Color::Orange,
            7 => Color::Pink,
            8 => Color::Brown,
            9 => Color::Black,
            _ => Color::Grey,
        }
    }

    fn to_u8(&self) -> u8 {
        match self {
            Color::Grey => 0,
            Color::Red => 1,
            Color::Green => 2,
            Color::Blue => 3,
            Color::Yellow => 4,
            Color::Purple => 5,
            Color::Orange => 6,
            Color::Pink => 7,
            Color::Brown => 8,
            Color::Black => 9,
        }
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

/// a u8 keeps two colors, as each color is 4 bits.
///
/// This is done to reduce the memory footprint of the board.
#[derive(Debug, Copy, Clone)]
pub struct ChunkColor(u8);

impl Default for ChunkColor {
    fn default() -> Self {
        Self::new(Color::Grey, Color::Grey)
    }
}

impl From<u8> for ChunkColor {
    fn from(value: u8) -> Self {
        Self(Color::from_u8(value).to_u8())
    }
}

impl Into<u8> for ChunkColor {
    fn into(self) -> u8 {
        self.0
    }
}

impl ChunkColor {
    pub fn new(color1: Color, color2: Color) -> Self {
        ChunkColor((color1.to_u8() << 4) | color2.to_u8())
    }

    // get the left color of the packed u8
    //
    // xxxx----
    pub fn left(&self) -> u8 {
        self.0 >> 4
    }

    // get the right color of the packed u8
    //
    // ----xxxx
    pub fn right(&self) -> u8 {
        self.0 & 0b1111
    }

    pub fn set_left(&mut self, color: Color) {
        self.0 = (color.to_u8() << 4) | (self.0 & 0b1111)
    }

    pub fn set_right(&mut self, color: Color) {
        self.0 = (self.0 & 0b11110000) | color.to_u8()
    }
}

type ChunkArray = [ChunkColor; CHUNK_SIZE / 2];

#[derive(Debug, Clone)]
pub struct Chunk(Arc<ChunkArray>);

impl Deref for Chunk {
    type Target = ChunkArray;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Chunk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self(Arc::new([ChunkColor::default(); CHUNK_SIZE / 2]))
    }
}

impl From<Vec<u8>> for Chunk {
    fn from(value: Vec<u8>) -> Self {
        if value.len() != CHUNK_BYTE_SIZE {
            return Self::default();
        }
        // Convert the vector into an array of ChunkColor
        let mut array = [ChunkColor::default(); CHUNK_SIZE / 2];
        for (i, byte) in value.into_iter().enumerate() {
            array[i] = byte.into();
        }

        Self(Arc::new(array))
    }
}

impl Chunk {
    pub fn new(coordinates: ChunkCoordinates) -> Self {
        Self::default()
    }

    pub fn new_with(data: ChunkArray) -> Self {
        Chunk(Arc::new(data))
    }

    pub fn to_vec(self) -> Vec<ChunkColor> {
        self.0.deref().into()
    }

    // ? let's pray that this gets optimized out
    pub fn to_u8vec(self) -> Vec<u8> {
        // self.0.iter().map(|&color| color.0).collect()

        // or we just do unsafe, how scary.
        // ? this will fail when ChunkColor is not u8 anymore tho
        let slice: &[u8] =
            unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.0.len()) };
        slice.to_vec()
    }
}

impl Into<Vec<u8>> for Chunk {
    fn into(self) -> Vec<u8> {
        self.to_u8vec()
    }
}

// pub type Chunk = Box<[Color; CHUNK_SIZE]>; // was not faster with Box<[u8; BOARD_SIZE]>, vec is more convenient.
// pub type Board = Arc<RwLock<Chunk>>;

/// Alway valid coordinates of a chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoordinates {
    x: i64,
    y: i64,
}

impl Default for ChunkCoordinates {
    fn default() -> Self {
        Self {
            x: Default::default(),
            y: Default::default(),
        }
    }
}

impl ChunkCoordinates {
    pub fn new(x: i64, y: i64) -> Self {
        // check if the values are valid
        // if x > CHUNKS_IN_DIRECTION - 1 || y > CHUNKS_IN_DIRECTION - 1 {
        // return None;
        // }

        Self { x, y }
    }
    pub fn x(&self) -> i64 {
        self.x
    }

    pub fn y(&self) -> i64 {
        self.y
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CellChangeMessage {
    pub index: usize,
    pub value: u8,
}

/// packed cell is an index and value packed into a u64
///
/// * this could be less thas u64
/// ! change the case 2 of index.html when changeing the size
#[derive(Debug, Clone)]
pub struct PackedCell(u64);

impl PackedCell {
    pub fn new(index: usize, value: u8) -> Self {
        PackedCell(((index as u64) << 4) | (value as u64))
    }

    pub fn index(&self) -> usize {
        (self.0 >> 4) as usize
    }

    pub fn value(&self) -> u8 {
        (self.0 & 0xF) as u8
    }

    pub fn to_binary(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.to_binary().to_vec()
    }
}

pub enum WsMessages {
    EntireChunk,
    ChunkUpdate,
    ChunkNotFound,
    TooManyChunksLoaded,
}

pub enum WsByte {
    EntireChunk,
    ChunkUpdate,
    ChunkNotFound,
    TooManyChunksLoaded,
}

impl Into<u8> for WsByte {
    fn into(self) -> u8 {
        match self {
            WsByte::EntireChunk => 1,
            WsByte::ChunkUpdate => 2,
            WsByte::ChunkNotFound => 3,
            WsByte::TooManyChunksLoaded => 4,
        }
    }
}

impl WsMessages {
    pub fn too_many_chunks_buffer() -> Vec<u8> {
        vec![WsByte::TooManyChunksLoaded.into()]
    }

    pub fn chunk_not_found_buffer() -> Vec<u8> {
        vec![WsByte::ChunkNotFound.into()]
    }

    pub fn chunk_update_buffer(updates: Vec<PackedCell>) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(updates.len() * 8 + 1);
        buffer.push(WsByte::ChunkUpdate.into());
        for update in updates {
            buffer.extend_from_slice(&update.to_binary());
        }
        buffer
    }
    pub fn entire_chunk_buffer(chunk: Chunk) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(CHUNK_BYTE_SIZE + 1);
        buffer.push(WsByte::EntireChunk.into());
        buffer.extend_from_slice(&chunk.to_u8vec());
        buffer
    }
}

#[cfg(test)]
mod testing {
    use super::*;

    #[test]
    fn chunk_color_packed_values() {
        let mut chunk_color = ChunkColor::default();

        // check that it is 00000000
        assert_eq!(chunk_color.0, 0b00000000);

        chunk_color.set_left(Color::Brown);

        assert_eq!(chunk_color.left(), Color::Brown.to_u8());
        // right should be untouched
        assert_eq!(chunk_color.right(), Color::Grey.to_u8());

        chunk_color.set_right(Color::Blue);
        assert_eq!(chunk_color.right(), Color::Blue.to_u8());
        // left should be untouchedm
        assert_eq!(chunk_color.left(), Color::Brown.to_u8());
    }
}
