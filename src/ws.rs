use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use tracing::{debug, info};

use crate::AppState;
use crate::{board_manager, types::*};

#[axum::debug_handler]
pub async fn ws_handler(
    Path((x, y)): Path<(i64, i64)>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // todo, check if the user is allowed to connect to this chunk
    let coordinates = ChunkCoordinates::new(x, y);

    // upgrade the request to a websocket
    ws.on_upgrade(move |socket| handle_socket(socket, coordinates, state))
}

struct WebSocketHandler {
    coordinates: ChunkCoordinates,
    broadcast_rx: broadcast::Receiver<Vec<PackedCell>>,
    update_tx: mpsc::Sender<PackedCell>,

    // split websocket
    sender: SplitSink<WebSocket, Message>,
    receiver: SplitStream<WebSocket>,
}

impl WebSocketHandler {
    // possibly returns websocket to be able to do stuff with it
    async fn connect(
        state: &AppState,
        mut socket: WebSocket,
        coordinates: ChunkCoordinates,
    ) -> Result<Self, WebSocket> {
        // try to get the chunk
        let handler_data = match state.board_communicator.get_handler(coordinates).await {
            Err(err) => match err {
                // if there are too many chunks loaded, tell the client
                board_manager::Error::TooManyChunksLoaded => {
                    let message = WsMessages::too_many_chunks_buffer();
                    socket.send(Message::Binary(message)).await.unwrap();
                    return Err(socket);
                }
            },
            Ok(c) => c,
        };

        // request the chunk from the chunk manager
        // you could also request the chunk from the board_manager
        // let chunk = state
        //     .board_communicator
        //     .get_chunk(coordinates, ChunkRequest::Live)
        //     .await;
        let chunk = handler_data.fetch_chunk().await;

        // send the chunk to the client
        let message = WsMessages::entire_chunk_buffer(chunk);
        socket.send(Message::Binary(message)).await.unwrap();

        let (sender, receiver) = socket.split();

        Ok(Self {
            coordinates,
            broadcast_rx: handler_data.broadcast_rx,
            update_tx: handler_data.update_tx,
            sender,
            receiver,
        })
    }

    async fn run(self) {
        let mut receiver_handler = Self::start_receiver(self.receiver, self.update_tx);
        let mut sender_handler = Self::start_sender(self.sender, self.broadcast_rx);

        tokio::select! {
            _ = &mut receiver_handler => {
                debug!("receiver task exited");
                sender_handler.abort();
            }
            _ = &mut sender_handler => {
                debug!("sender task exited");
                receiver_handler.abort();
            }
        }
        return;
    }

    fn start_receiver(
        mut receiver: SplitStream<WebSocket>,
        update_tx: mpsc::Sender<PackedCell>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                info!("received message: {:?}", msg);
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(e) => {
                        info!("error receiving message: {:?}", e);
                        continue;
                    }
                };

                match msg {
                    axum::extract::ws::Message::Binary(data) => {
                        if data.len() == 8 {
                            let packed_value = u64::from_le_bytes(data.try_into().unwrap());
                            let index = (packed_value >> 4) as usize;
                            let color_number = (packed_value & 0xF) as u8;

                            let packed_cell = PackedCell::new(index, color_number);

                            // Send message to the AppState
                            update_tx.send(packed_cell).await.unwrap();
                        } else {
                            info!("invalid binary message length: {:?}", data.len());
                            // todo, pehaps a "resync" with the boardRequester
                        }
                    }
                    // we can ignore ping, handled by axum
                    axum::extract::ws::Message::Ping(_) => {}
                    axum::extract::ws::Message::Text(_) => {}
                    axum::extract::ws::Message::Pong(_) => {}
                    axum::extract::ws::Message::Close(_) => {
                        info!("client closed the connection");
                        break;
                    }
                }
            }
        })
    }

    /// Receive messages from the [`ChunkManager`](crate::chunk_manager::ChunkManager) and send them to the client
    ///
    /// The messages will be the buffered changes
    fn start_sender(
        mut sender: SplitSink<WebSocket, Message>,
        mut broadcast_rx: broadcast::Receiver<Vec<PackedCell>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match broadcast_rx.recv().await {
                    Ok(packed_cells) => {
                        let message = WsMessages::chunk_update_buffer(packed_cells);

                        sender.send(Message::Binary(message)).await.unwrap();
                    }
                    Err(e) => {
                        info!("error receiving message: {:?}", e);
                        break;
                    }
                }
            }
        })
    }
}

async fn handle_socket(socket: WebSocket, coordinates: ChunkCoordinates, state: AppState) {
    state.add_connection();

    let handler = WebSocketHandler::connect(&state, socket, coordinates).await;

    match handler {
        Ok(handler) => {
            handler.run().await;
        }
        Err(mut socket) => {
            socket.send(Message::Close(None)).await.unwrap();
            return;
        }
    }

    info!("socket closed");

    // decrement the connections amount in appstate
    state.remove_connection();
}

// ! problematic_code
// ! sending the message through websocket keeps memory allocated.
// ! afaik, sender is not doing things properly.
// ! the Vec::with_capacity(BOARD_SIZE + 1) is not released,
// {
// let board = state.board.read().await.clone();

// let mut board_message = Vec::with_capacity(BOARD_SIZE + 1);
// board_message.push(0x00); // 0x00 indicates full board
// board_message.extend_from_slice(&board[..BOARD_SIZE / 10].to_vec());

// sender.send(Message::Binary(board_message)).await.unwrap();
// }
// todo, send only the chunk requested. Is annoying to keep track of which chunks the client is looking at.
// todo, non dymanic chunks, ui shows you navigation to go to the next chunk.
// todo, ^ this still needs the update vec to include for which chunk it was meant.
