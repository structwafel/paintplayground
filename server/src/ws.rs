use axum::{
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use tracing::{debug, info};

use crate::board_manager;
use crate::{AppState, chunk_manager};

use paintplayground::types::*;

#[axum::debug_handler]
pub async fn ws_handler(
    Path((x, y)): Path<(i64, i64)>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // todo, check if the user is allowed to connect to this chunk
    let Ok(coordinates) = ChunkCoordinates::new(x, y) else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };

    // upgrade the request to a websocket
    ws.on_upgrade(move |socket| handle_socket(socket, coordinates, state))
}

struct WebSocketHandler {
    _coordinates: ChunkCoordinates,
    handler_data: chunk_manager::HandlerData,
    // broadcast_rx: broadcast::Receiver<Vec<PackedCell>>,
    // update_tx: mpsc::Sender<Vec<PackedCell>>,

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
        debug!("WH - getting handler data");
        let handler_data = match state.board_communicator.get_handler(coordinates).await {
            Err(err) => match err {
                board_manager::BoardManagerError::TooManyChunksLoaded => {
                    let message = WsMessage::too_many_chunks_buffer();
                    socket.send(Message::Binary(message.into())).await.unwrap();
                    return Err(socket);
                }
                board_manager::BoardManagerError::_LoadingChunks => {
                    let message = WsMessage::chunk_not_found_buffer();
                    socket.send(Message::Binary(message.into())).await.unwrap();
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
        debug!("sending chunk to client");
        let message = WsMessage::entire_chunk_buffer(chunk);
        socket.send(Message::Binary(message.into())).await.unwrap();

        let (sender, receiver) = socket.split();

        Ok(Self {
            _coordinates: coordinates,
            handler_data,
            // broadcast_rx: handler_data.broadcast_rx,
            // update_tx: handler_data.update_tx,
            sender,
            receiver,
        })
    }

    async fn run(self) {
        let mut receiver_handler = Self::start_receiver(self.receiver, self.handler_data.update_tx);
        let mut sender_handler = Self::start_sender(self.sender, self.handler_data.broadcast_rx);

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
        update_tx: mpsc::Sender<Vec<PackedCell>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(e) => {
                        debug!("error receiving message: {:?}", e);
                        continue;
                    }
                };

                match msg {
                    axum::extract::ws::Message::Binary(data) => {
                        // todo, add first byte for message type.

                        // messages will be an array of index and value (PackedCell)
                        let updates: Vec<PackedCell> = data
                            .chunks_exact(8)
                            .map(|chunk| {
                                let eight_arr: [u8; 8] = chunk.try_into().unwrap();

                                // in 8 bytes, we have the index and the value.
                                match u64::from_le_bytes(eight_arr) {
                                    0 => None,
                                    packed_value => PackedCell::new_from_u64(packed_value),
                                }
                            })
                            .filter_map(|x| x)
                            .collect();

                        debug!("received {} updates", updates.len());
                        update_tx.send(updates).await.unwrap();
                    }
                    // we can ignore ping, handled by axum
                    axum::extract::ws::Message::Ping(_) => {}
                    axum::extract::ws::Message::Text(_) => {}
                    axum::extract::ws::Message::Pong(_) => {}
                    axum::extract::ws::Message::Close(_) => {
                        debug!("client closed the connection");
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
                        debug!("received broadcast");
                        let message = WsMessage::chunk_update_buffer(packed_cells);

                        match sender.send(Message::Binary(message.into())).await {
                            Ok(_) => (), // message got send fine,
                            Err(err) => {
                                // something broke the pipe, most likely the connection was closed in between await operations
                                error!("sender could not send {}", err);
                                break;
                            }
                        };
                    }
                    Err(e) => {
                        debug!("error receiving message: {:?}", e);
                        // The ChunkManager has been dropped, close the connection
                        let _ = sender
                            .send(Message::Close(None))
                            .await
                            .map_err(|err| error!("could not send close message {}", err));

                        break;
                    }
                }
            }
        })
    }
}

async fn handle_socket(socket: WebSocket, coordinates: ChunkCoordinates, state: AppState) {
    state.add_connection();

    debug!("new websocket connection");
    let handler = WebSocketHandler::connect(&state, socket, coordinates).await;

    match handler {
        Ok(handler) => {
            debug!("websocket connected");
            handler.run().await;
        }
        Err(mut socket) => {
            info!("too many chunks loaded, closing connection");
            socket.send(Message::Close(None)).await.unwrap();
            return;
        }
    }

    debug!("socket closed");
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
