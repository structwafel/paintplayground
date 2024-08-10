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
