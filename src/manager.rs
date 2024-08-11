use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::types::*;

use crate::NotifyCellChangeSender;

#[derive(Debug, Clone)]
pub struct CanvasManager {
    // the game board
    pub board: Board,

    // the broadcast sender to send messages to all clients
    broadcast_cellchanges: NotifyCellChangeSender,
}

impl CanvasManager {
    pub fn new(sender: NotifyCellChangeSender) -> Self {
        let board = load_map_from_disk();

        Self {
            board: Arc::new(RwLock::new(board)),
            broadcast_cellchanges: sender,
        }
    }

    pub async fn run(mut self, mut recieve_cell_changes: CellChangeReceiver) {
        // recieve updates, and buffer them
        println!("Starting canvas manager");

        let mut changed;
        loop {
            let mut smaller_buffer = Vec::new();
            changed = false;
            let timeout = tokio::time::sleep(Duration::from_secs(crate::CLEAR_BUFFER_INTERVAL));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Some(change) = recieve_cell_changes.recv() => {
                        smaller_buffer.push(change);
                        // buffer[change.index()] = change.value();
                        changed = true;
                    }
                    _ = &mut timeout => {
                        // breaking so we need to empty the smaller_buffer
                        break;
                    }
                }
            }

            if !changed {
                continue;
            }
            println!("Size of smaller buffer {}", smaller_buffer.len());

            // buffer and board are chunks, only the non-zero buffer values need to be set in the board
            // only take the last of each unique indes
            let mut last_changes: Vec<PackedCell> = Vec::with_capacity(smaller_buffer.len());
            {
                for change in smaller_buffer {
                    if let Some(last_change) = last_changes
                        .iter_mut()
                        .find(|c| c.index() == change.index())
                    {
                        *last_change = change;
                    } else {
                        last_changes.push(change);
                    }
                }
            }

            // apply the changes to the board
            {
                // let mut board = self.board.write().await;
                let mut board = self.board.write().await;

                for change in last_changes.iter() {
                    board[change.index()] = change.value();
                }
            }

            // save the changes to disk
            // todo do something else
            {
                println!("Saving board to disk");
                let board = self.board.read().await;
                let board = board.to_vec();
                tokio::spawn(async move {
                    save_map_to_disk(board);
                });
            }

            // broadcast the changes made to all the clients
            self.broadcast(last_changes);
        }
    }

    fn broadcast(&mut self, messages: Vec<PackedCell>) {
        self.broadcast_cellchanges.send(messages).unwrap();
    }
}

fn save_map_to_disk(map: Vec<u8>) {
    // create the canvas directory if it doesn't exist
    println!("creating dir if no exist");
    std::fs::create_dir_all("canvas").unwrap();

    println!("actual saving to file");
    let mut file = File::create("canvas/0-0-chunk.bin").unwrap();
    file.write_all(&map).unwrap();
}

fn load_map_from_disk() -> Chunk {
    let file = match File::open("canvas/0-0-chunk.bin") {
        Ok(f) => f,
        Err(_) => return new_chunk(),
    };

    let mut reader = std::io::BufReader::new(file);
    let mut buffer = Vec::with_capacity(crate::BOARD_SIZE);
    reader.read_to_end(&mut buffer).unwrap();

    // Ensure the buffer has the correct length
    if buffer.len() != crate::BOARD_SIZE {
        return new_chunk();
    }

    // Convert the Vec to an array
    // let array: Box<[u8; BOARD_SIZE]> = buffer.try_into().expect("Buffer length mismatch");

    buffer
}
