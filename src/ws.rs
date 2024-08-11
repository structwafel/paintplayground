use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    Extension,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tracing::{debug, error, info};

use crate::types::*;
use crate::{AppState, CellChangeSender, NotifyCellChangeReceiver};

#[axum::debug_handler]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(crate::Receiver(reciever)): Extension<crate::Receiver>,
    Extension(crate::UpdateTransmitter(update_tx)): Extension<crate::UpdateTransmitter>,
    // Extension(board_request_tx): Extension<BoardRequester>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // upgrade the request to a websocket
    ws.on_upgrade(move |socket| handle_socket(socket, reciever, update_tx, state))
}

async fn handle_socket(
    socket: WebSocket,
    mut state_receiver: NotifyCellChangeReceiver,
    update_tx: CellChangeSender,
    // board_request_tx: BoardRequester,
    state: AppState,
) {
    state
        .connections
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // handle the websocket
    let (mut sender, mut receiver) = socket.split();

    // ! problematic_code
    // ! sending the message through websocket keeps memory allocated.
    // ! afaik, sender is not doing things properly.
    // ! the Vec::with_capacity(BOARD_SIZE + 1) is not released,
    {
        // let board = state.board.read().await.clone();

        // let mut board_message = Vec::with_capacity(BOARD_SIZE + 1);
        // board_message.push(0x00); // 0x00 indicates full board
        // board_message.extend_from_slice(&board[..BOARD_SIZE / 10].to_vec());

        // sender.send(Message::Binary(board_message)).await.unwrap();
    }
    // todo, send only the chunk requested. Is annoying to keep track of which chunks the client is looking at.
    // todo, non dymanic chunks, ui shows you navigation to go to the next chunk.
    // todo, ^ this still needs the update vec to include for which chunk it was meant.

    let mut handler_receiver = tokio::spawn(async move {
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
                // we can ignore ping, handled by axum
                axum::extract::ws::Message::Ping(_) => {
                    continue;
                }
                axum::extract::ws::Message::Text(_) => todo!(),
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
                axum::extract::ws::Message::Pong(_) => todo!(),
                axum::extract::ws::Message::Close(_) => {
                    info!("client closed the connection");
                    break;
                }
            }
        }
    });

    // Receive messages from the CanvasManager and send them to the client
    //
    // The messages will be the buffered changes
    let mut handler_sender = tokio::spawn(async move {
        loop {
            match state_receiver.recv().await {
                Ok(packed_cells) => {
                    // Serialize the Vec<CellChangeMessage> into a binary format
                    let mut buffer = Vec::with_capacity(packed_cells.len() * 8);
                    // buffer.push(0x01); // 0x01 indicates chunk updates
                    for packed_cell in packed_cells {
                        buffer.extend_from_slice(&packed_cell.to_binary());
                    }

                    sender.send(Message::Binary(buffer)).await.unwrap();
                    // match send_chunk_update(packed_cells, &mut sender).await {
                    //     Ok(_) => {}
                    //     Err(e) => {
                    //         // try to reconnect?
                    //         error!("error sending message: {:?}", e);
                    //         return;
                    //     }
                    // }
                }
                Err(e) => {
                    error!("error receiving message: {:?}", e);
                    return;
                }
            }
        }

        // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    });

    // If any one of the tasks exit, abort the other.

    tokio::select! {
        _ = &mut handler_receiver => {
            debug!("receiver task exited");
            handler_sender.abort();
        }
        _ = &mut handler_sender => {
            debug!("sender task exited");
            handler_receiver.abort();
        }
    }

    info!("socket closed");
    // decrement the connections amount in appstate
    state
        .connections
        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
}
