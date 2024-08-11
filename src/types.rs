use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

pub type NotifyCellChangeReceiver = broadcast::Receiver<Vec<PackedCell>>;
pub type NotifyCellChangeSender = broadcast::Sender<Vec<PackedCell>>;
pub type CellChangeReceiver = mpsc::Receiver<PackedCell>;
pub type CellChangeSender = mpsc::Sender<PackedCell>;
pub type BoardRequester = mpsc::Sender<oneshot::Sender<Arc<RwLock<Chunk>>>>;

pub type Color = u8;
pub type Chunk = Vec<Color>; // was not faster with Box<[u8; BOARD_SIZE]>, vec is more convenient.
pub type Board = Arc<RwLock<Chunk>>;

#[inline]
pub fn new_chunk() -> Chunk {
    [0; BOARD_SIZE].into()
}

pub fn new_board() -> Board {
    Arc::new(RwLock::new(new_chunk()))
}

pub struct Receiver(pub NotifyCellChangeReceiver);
impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self(self.0.resubscribe())
    }
}

#[derive(Clone)]
pub struct UpdateTransmitter(pub CellChangeSender);

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CellChangeMessage {
    pub index: usize,
    pub value: u8,
}

pub const BOARD_SIZE: usize = 1_000_000;

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
