use std::env;
pub use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;
pub use std::sync::Arc;
use std::sync::LazyLock;

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
pub static CHUNKS_IN_DIRECTION: LazyLock<i64> = LazyLock::new(|| {
    let number = env::var("CHUNKS_IN_DIRECTION")
        .unwrap_or({
            info!("CHUNKS_IN_DIRECTION not set, using 10");
            "10".to_string()
        })
        // .expect("CHUNKS_IN_DIRECTION environment variable not set")
        .parse()
        .expect("CHUNKS_IN_DIRECTION is not a unsigned number");
    if number < 0 {
        panic!("CHUNKS_IN_DIRECTION cannot be negative")
    }
    number
});
// pub const CHUNKS: usize = CHUNKS_IN_DIRECTION * CHUNKS_IN_DIRECTION;

pub const MB: u64 = 1024 * 1024;
pub const CACHE_SIZE: u64 = 100 * MB;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
/// Represents the possible colors of a cell.
///
/// The colors are taken from the 4-bit RGB palette from Lospec.
/// https://lospec.com/palette-list/woodspark
pub enum Color {
    ///  #e0d3c8
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

    pub fn u8(self) -> u8 {
        self as u8
    }

    /// this color as the rgb value from the pallet
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Zero => (224, 211, 200),
            Color::One => (245, 238, 176),
            Color::Two => (250, 191, 97),
            Color::Three => (224, 141, 81),
            Color::Four => (138, 88, 101),
            Color::Five => (69, 43, 63),
            Color::Six => (44, 94, 59),
            Color::Seven => (96, 156, 79),
            Color::Eight => (198, 204, 84),
            Color::Nine => (120, 194, 214),
            Color::Ten => (84, 121, 176),
            Color::Eleven => (86, 84, 110),
            Color::Twelve => (131, 158, 166),
            Color::Thirteen => (240, 91, 91),
            Color::Fourteen => (143, 50, 95),
            Color::Fifteen => (235, 108, 152),
        }
    }

    pub fn all_colors_rgb() -> [(u8, u8, u8); 16] {
        [
            (224, 211, 200),
            (245, 238, 176),
            (250, 191, 97),
            (224, 141, 81),
            (138, 88, 101),
            (69, 43, 63),
            (44, 94, 59),
            (96, 156, 79),
            (198, 204, 84),
            (120, 194, 214),
            (84, 121, 176),
            (86, 84, 110),
            (131, 158, 166),
            (240, 91, 91),
            (143, 50, 95),
            (235, 108, 152),
        ]
    }

    pub fn to_index(&self) -> u8 {
        let (r, g, b) = self.to_rgb();
        Self::rgb_to_index(r, g, b)
    }

    // we use all colours to get the correct index for the mapped colours
    pub fn rgb_to_index(r: u8, g: u8, b: u8) -> u8 {
        let colors = Color::all_colors_rgb();
        for (idx, &(cr, cg, cb)) in colors.iter().enumerate() {
            if r == cr && g == cg && b == cb {
                return idx as u8;
            }
        }
        // Default to first color if no match (shouldn't happen)
        0
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

    pub fn left_color(&self) -> Color {
        Color::new(self.left()).unwrap()
    }

    // get the right color of the packed u8
    //
    // ----xxxx
    pub fn right(&self) -> u8 {
        self.0 & 0b1111
    }

    pub fn right_color(&self) -> Color {
        Color::new(self.right()).unwrap()
    }

    pub fn set_left(&mut self, color: Color) {
        self.0 = (color.u8() << 4) | (self.0 & 0b1111)
    }

    pub fn set_right(&mut self, color: Color) {
        self.0 = (self.0 & 0b11110000) | color.u8()
    }
}

pub const USED_COMPRESSION: CompressionType = CompressionType::Zstd;

pub enum CompressionType {
    None,
    Zstd,
    Lz4,
    // Gzip, // it is very compact, but very slow
}

type ChunkArray<const N: usize> = [ChunkColor; N];
// type ChunkArray = [ChunkColor; CHUNK_SIZE / 2];

pub type Chunk = InnerChunk<{ CHUNK_SIZE / 2 }>;

pub type SmallChunkArray = InnerChunk<5>;

impl Chunk {
    pub fn row_of_colors(&self, x: usize) -> Vec<Color> {
        let start = x * (CHUNK_LENGTH / 2);
        let end = start + (CHUNK_LENGTH / 2);

        // Collect both left and right colors for the entire row
        self.0[start..end]
            .iter()
            .flat_map(|chunk_color| vec![chunk_color.left_color(), chunk_color.right_color()])
            .collect()
    }

    pub fn from_raw_data(data: &[u8]) -> Result<Self, String> {
        if data.is_empty() {
            return Err("empty data".into());
        }

        // if we are reading exactly byte size, we have an old uncompressed format
        if data.len() == CHUNK_BYTE_SIZE {
            return Ok(data.to_vec().into());
        }

        let format = data[0];
        let content = &data[1..];

        match format {
            0 => {
                if content.len() != CHUNK_BYTE_SIZE {
                    return Err("Invalid chunk size with uncompressed data".into());
                }
                Ok(content.to_vec().into())
            }
            // ZSTD compressed
            1 => {
                let uncompressed =
                    ZstdCompression::decompress(content, CHUNK_BYTE_SIZE).map_err(|err| {
                        let text = format!("zstd decompression failed: {}", err);
                        error!(text);
                        text
                    })?;
                Ok(uncompressed.into())
            }
            // Lz4 compressed
            2 => {
                let uncompressed =
                    LZ4Compression::decompress(content, CHUNK_BYTE_SIZE).map_err(|err| {
                        let text = format!("lz4 decompression failed: {}", err);
                        error!(text);
                        text
                    })?;
                Ok(uncompressed.into())
            }
            _ => Err("Unknown compression format".into()),
        }
    }

    pub fn to_storage_bytes(self, compression: CompressionType) -> Vec<u8> {
        let mut result = Vec::with_capacity(CHUNK_BYTE_SIZE + 1);
        let raw_data = self.to_u8vec();

        match compression {
            CompressionType::None => {
                result.push(0); // no compression
                result.extend_from_slice(&raw_data);
            }
            CompressionType::Zstd => {
                result.push(1); // zstd compression
                let compressed =
                    ZstdCompression::compress(&raw_data).expect("failed to compress with zstd");
                result.extend_from_slice(&compressed);
            }
            CompressionType::Lz4 => {
                result.push(2);
                let compressed =
                    LZ4Compression::compress(&raw_data).expect("failed to compress with zstd");
                result.extend_from_slice(&compressed);
            }
        }

        result
    }
}

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

    /// Set a pixel color at a packed index (0 to CHUNK_SIZE-1)
    pub fn set_pixel(&mut self, packed_index: usize, color: Color) {
        if packed_index >= CHUNK_SIZE {
            return;
        }

        let byte_index = packed_index / 2;
        let is_left = packed_index & 1 == 0;

        if byte_index >= N {
            return;
        }

        let chunk_array = Arc::make_mut(&mut self.0);
        if is_left {
            chunk_array[byte_index].set_left(color);
        } else {
            chunk_array[byte_index].set_right(color);
        }
    }

    /// Apply a PackedCell update to this chunk
    pub fn apply_packed_cell(&mut self, packed_cell: &PackedCell) {
        self.set_pixel(packed_cell.index(), packed_cell.color());
    }

    pub fn data(&self) -> Vec<u8> {
        self.clone().to_u8vec()
    }
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

    /// The chunk name of this coordinate
    pub fn object_name(&self) -> String {
        format!("{}_{}.chunk", self.x, self.y)
    }
}

use serde::{Deserialize, Serialize};

use crate::compression::Compression;
use crate::compression::LZ4Compression;
use crate::compression::ZstdCompression;

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

    pub fn new_from_u64(packed_value: u64) -> Option<Self> {
        let index = (packed_value >> 4) as usize;
        // last 58 bits are the color
        let value = (packed_value & 0xF) as u8;

        Self::new(index, value)
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
