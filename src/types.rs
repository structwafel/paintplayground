use std::env;
pub use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;
pub use std::sync::Arc;
use std::sync::LazyLock;

use axum::response::IntoResponse;
pub use tokio::sync::broadcast;
pub use tokio::sync::mpsc;
pub use tokio::sync::oneshot;

pub use tracing::{debug, error, info, warn};

// re-export common used types

// pub type BoardRequester = mpsc::Sender<oneshot::Sender<Arc<RwLock<Chunk>>>>;

pub const CHUNK_LENGTH: usize = 100;
pub const CHUNK_SIZE: usize = CHUNK_LENGTH * CHUNK_LENGTH;
pub const CHUNK_BYTE_SIZE: usize = CHUNK_SIZE / 2;

// get this from env
pub static CHUNKS_IN_DIRECTION: LazyLock<usize> = LazyLock::new(|| {
    env::var("CHUNKS_IN_DIRECTION")
        .unwrap_or({
            info!("CHUNKS_IN_DIRECTION not set, using 10");
            "10".to_string()
        })
        // .expect("CHUNKS_IN_DIRECTION environment variable not set")
        .parse()
        .expect("CHUNKS_IN_DIRECTION is not a unsigned number")
});
// pub const CHUNKS: usize = CHUNKS_IN_DIRECTION * CHUNKS_IN_DIRECTION;

pub const MB: u64 = 1024 * 1024;
pub const CACHE_SIZE: u64 = 100 * MB;

#[repr(u8)]
/// Represents the possible colors of a cell.
///
/// The colors are taken from the 4-bit RGB palette from Lospec.
/// https://lospec.com/palette-list/4-bit-rgb
pub enum Color {
    Zero = 0b0000,
    One = 0b0001,
    Two = 0b0010,
    Three = 0b0011,
    Four = 0b0100,
    Five = 0b0101,
    Six = 0b0110,
    Seven = 0b0111,
    Eight = 0b1000,
    Nine = 0b1001,
    Ten = 0b1010,
    Eleven = 0b1011,
    Twelve = 0b1100,
    Thirteen = 0b1101,
    Fourteen = 0b1110,
    Fifteen = 0b1111,
}

impl Color {
    fn new(value: u8) -> Option<Self> {
        Some(match value {
            0 => Color::Zero,
            1 => Color::One,
            2 => Color::Two,
            3 => Color::Three,
            4 => Color::Four,
            5 => Color::Five,
            6 => Color::Six,
            7 => Color::Seven,
            8 => Color::Eight,
            9 => Color::Nine,
            10 => Color::Ten,
            11 => Color::Eleven,
            12 => Color::Twelve,
            13 => Color::Thirteen,
            14 => Color::Fourteen,
            15 => Color::Fifteen,
            _ => return None,
        })
    }

    fn u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Color {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match Color::new(value) {
            Some(color) => Ok(color),
            None => Err(()),
        }
    }
}

/// a u8 keeps two colors, as each color is 4 bits.
///
/// This is done to reduce the memory footprint of the board.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChunkColor(u8);

impl Default for ChunkColor {
    fn default() -> Self {
        Self::new(Color::Zero, Color::Zero)
    }
}

impl TryFrom<u8> for ChunkColor {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match (Color::new(value >> 4), Color::new(value & 0b1111)) {
            (Some(left), Some(right)) => Ok(Self::new(left, right)),
            _ => Err(()),
        }
    }
}

impl Into<u8> for ChunkColor {
    fn into(self) -> u8 {
        self.0
    }
}

impl ChunkColor {
    pub fn new(color1: Color, color2: Color) -> Self {
        ChunkColor(((color1 as u8) << 4) | color2 as u8)
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
        self.0 = (color.u8() << 4) | (self.0 & 0b1111)
    }

    pub fn set_right(&mut self, color: Color) {
        self.0 = (self.0 & 0b11110000) | color.u8()
    }
}
type ChunkArray<const N: usize> = [ChunkColor; N];
// type ChunkArray = [ChunkColor; CHUNK_SIZE / 2];

pub type Chunk = InnerChunk<{ CHUNK_SIZE / 2 }>;
#[cfg(test)]
type SmallChunkArray = InnerChunk<5>;

#[derive(Debug, Clone)]
pub struct InnerChunk<const N: usize>(Arc<ChunkArray<N>>);

impl<const N: usize> Deref for InnerChunk<N> {
    type Target = ChunkArray<N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for InnerChunk<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<const N: usize> Default for InnerChunk<N> {
    fn default() -> Self {
        Self(Arc::new([ChunkColor::default(); N]))
    }
}

impl<const N: usize> From<Vec<u8>> for InnerChunk<N> {
    fn from(value: Vec<u8>) -> Self {
        if value.len() != N {
            panic!(
                "Invalid size of the vector, expected {}, got {}",
                N,
                value.len()
            );
        }
        // Convert the vector into an array of ChunkColor
        let mut array = [ChunkColor::default(); N];
        for (i, byte) in value.into_iter().enumerate() {
            if let Ok(color) = byte.try_into() {
                array[i] = color;
            }
        }

        Self(Arc::new(array))
    }
}

impl<const N: usize> InnerChunk<N> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with(data: ChunkArray<N>) -> Self {
        InnerChunk(Arc::new(data))
    }

    pub fn to_vec(self) -> Vec<ChunkColor> {
        self.0.deref().into()
    }

    // ? let's pray that this gets optimized out
    pub fn to_u8vec(self) -> Vec<u8> {
        self.0.iter().map(|&color| color.0).collect()

        // or we just do unsafe, how scary.
        // ? this will fail when ChunkColor is not u8 anymore tho
        // let slice: &[u8] =
        //     unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.0.len()) };
        // slice.to_vec()
    }

    // ? add compression
    // pub(crate) fn to_compressed(&self) -> Vec<u8> {
    // let mut encoder = lz4::EncoderBuilder::new().build(Vec::new()).unwrap();
    // encoder.write_all(&self.to_u8vec()).unwrap();
    // encoder.finish().0
    // unimplemented!()
    // }
}

impl<const N: usize> Into<Vec<u8>> for InnerChunk<N> {
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
    pub fn new(x: i64, y: i64) -> Result<Self, ()> {
        // check if the values are valid, chunks_in_direction is usize.

        let chunks_in_direction = *CHUNKS_IN_DIRECTION as i64;

        if x.abs() > chunks_in_direction || y.abs() > chunks_in_direction {
            debug!(
                "Invalid coordinates, x: {}, y: {}, chunks_in_direction: {}",
                x, y, chunks_in_direction
            );
            return Err(());
        }

        Ok(Self { x, y })
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
/// index is 60 bits, value is 4 bits
///
/// * this could be less thas u64
/// ! change the case 2 of index.html when changeing the size
#[derive(Debug, Clone)]
pub struct PackedCell(u64);

impl PackedCell {
    pub fn new(index: usize, value: u8) -> Option<Self> {
        if index >= CHUNK_SIZE {
            return None;
        }
        if let Some(color) = Color::new(value) {
            Some(PackedCell(((index as u64) << 4) | (color.u8() as u64)))
        } else {
            None
        }
    }

    pub fn index(&self) -> usize {
        (self.0 >> 4) as usize
    }

    pub fn value(&self) -> u8 {
        (self.0 & 0xF) as u8
    }

    // we know that the value is a valid color
    pub fn color(&self) -> Color {
        Color::new(self.value()).unwrap()
    }

    pub fn to_binary(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.to_binary().to_vec()
    }
}

pub enum WsMessage {
    EntireChunk,
    ChunkUpdate,
    ChunkNotFound,
    TooManyChunksLoaded,
}

impl Into<u8> for WsMessage {
    fn into(self) -> u8 {
        match self {
            WsMessage::EntireChunk => 1,
            WsMessage::ChunkUpdate => 2,
            WsMessage::ChunkNotFound => 3,
            WsMessage::TooManyChunksLoaded => 4,
        }
    }
}

impl WsMessage {
    pub fn too_many_chunks_buffer() -> Vec<u8> {
        vec![WsMessage::TooManyChunksLoaded.into()]
    }

    pub fn chunk_not_found_buffer() -> Vec<u8> {
        vec![WsMessage::ChunkNotFound.into()]
    }

    pub fn chunk_update_buffer(updates: Vec<PackedCell>) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(updates.len() * 8 + 1);
        buffer.push(WsMessage::ChunkUpdate.into());
        for update in updates {
            buffer.extend_from_slice(&update.to_binary());
        }
        buffer
    }
    pub fn entire_chunk_buffer(chunk: Chunk) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(CHUNK_BYTE_SIZE + 1);
        buffer.push(WsMessage::EntireChunk.into());
        buffer.extend_from_slice(&chunk.to_u8vec());
        buffer
    }
}

#[cfg(test)]
mod testing {
    use crate::chunk_db::{ChunkLoaderSaver, SimpleToFileSaver};

    use super::*;
    // Initialize tracing subscriber

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn chunk_color_packed_values() {
        let mut chunk_color = ChunkColor::default();

        // check that it is 00000000
        assert_eq!(chunk_color.0, 0b00000000);

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

        let loaded_chunk = saver.load_chunk(coordinates).unwrap();

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
