use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};

use futures::{sink::SinkExt, stream::StreamExt};
use shared_lib::{CellChangeMessage, BOARD_SIZE};
use tokio::{
    sync::{broadcast, mpsc},
    time,
};
use tower::layer;
use tower_http::{
    services::ServeFile,
    trace::{DefaultMakeSpan, TraceLayer},
};

#[cfg(test)]
mod test;

use tracing::{debug, error, info};

const CLEAR_BUFFER_INTERVAL: u64 = 5;

type NotifyCellChangeReceiver = broadcast::Receiver<Vec<CellChangeMessage>>;
type NotifyCellChangeSender = broadcast::Sender<Vec<CellChangeMessage>>;

type CellChangeReceiver = mpsc::Receiver<shared_lib::PackedCell>;
type CellChangeSender = mpsc::Sender<shared_lib::PackedCell>;

type Color = u8;
type Chunk = Box<[Color; BOARD_SIZE]>;

// TODO hold all conenctions in a vec, loop through and check if connected, then send buffer to all connected clients

#[inline]
fn new_board() -> Chunk {
    [0; BOARD_SIZE].into()
}

#[derive(Debug, Clone)]
struct AppState {
    // the game board
    board: Arc<Mutex<Chunk>>,

    // the broadcast sender to send messages to all clients
    client_sender: NotifyCellChangeSender,
}

impl AppState {
    fn new(sender: NotifyCellChangeSender) -> Self {
        Self {
            board: Arc::new(Mutex::new(new_board())),
            client_sender: sender,
        }
    }

    async fn run(&mut self, mut recieve_cell_changes: CellChangeReceiver) {
        // recieve updates, and buffer them

        let mut buffer = new_board();
        let mut changed;
        loop {
            changed = false;
            let timeout = time::sleep(Duration::from_secs(CLEAR_BUFFER_INTERVAL));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Some(change) = recieve_cell_changes.recv() => {
                        buffer[change.index()] = change.value();
                        changed = true;
                    }
                    _ = &mut timeout => {
                        break;
                    }
                }
            }
            if !changed {
                println!("no changes, skipping");
                continue;
            }

            // buffer and board are chunks, only the non-zero buffer values need to be set in the board
            {
                let mut board = self.board.lock().unwrap();

                board
                    .iter_mut()
                    .zip(buffer.iter())
                    .for_each(|(board_val, buf_val)| {
                        if *buf_val != 0 {
                            *board_val = *buf_val;
                        }
                    });
            }

            // send the changes to all clients, which is just the buffer except for the zero values
            let changes = buffer
                .iter()
                .enumerate()
                .filter_map(|(index, value)| {
                    if *value != 0 {
                        Some(CellChangeMessage {
                            index,
                            value: *value,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            buffer = new_board();

            self.broadcast(changes);
        }
    }

    fn broadcast(&mut self, messages: Vec<CellChangeMessage>) {
        // println!(
        //     "broadcast buffer size: {}",
        //     self.client_sender.receiver_count()
        // );
        self.client_sender.send(messages).unwrap();
    }

    fn get_board(&self) -> Chunk {
        let board = self.board.lock().unwrap();
        board.clone()
    }
}

struct Receiver(NotifyCellChangeReceiver);
impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self(self.0.resubscribe())
    }
}

#[derive(Clone)]
struct UpdateTransmitter(CellChangeSender);

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .with_target(false)
        .init();

    let (sender, receiver) = broadcast::channel(1_000_000);

    let (manager_sender, manager_receiver) = mpsc::channel::<shared_lib::PackedCell>(1_000_000);

    let state = AppState::new(sender);

    // spawn a task to flush the buffer every second
    let mut state_clone = state.clone();
    tokio::spawn(async move {
        state_clone.run(manager_receiver).await;
    });

    // build our application with some routes
    let app = Router::new()
        .route_service("/", ServeFile::new("public/index.html"))
        .route("/ws", get(ws_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(Receiver(receiver)))
        .layer(Extension(UpdateTransmitter(manager_sender)))
        .layer(Extension(state.board.clone()));
    // .with_state(state);

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    debug!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

#[axum::debug_handler]
async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(Receiver(reciever)): Extension<Receiver>,
    Extension(UpdateTransmitter(update_tx)): Extension<UpdateTransmitter>,
    Extension(board): Extension<Arc<Mutex<Chunk>>>,
    // State(state): State<AppState>,
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.

    let board = board.lock().unwrap().clone();

    // ws.on_upgrade(move |socket| handle_socket(socket, reciever, update_tx, state))
    ws.on_upgrade(move |socket| handle_socket(socket, reciever, update_tx, board))
}

async fn handle_socket(
    socket: WebSocket,
    mut state_receiver: NotifyCellChangeReceiver,
    update_tx: CellChangeSender,
    board: Chunk,
    // state: AppState,
) {
    // handle the websocket
    let (mut sender, mut receiver) = socket.split();

    if let Err(e) = sender.send(Message::Binary(board.to_vec())).await {
        info!("error sending board state: {:?}", e);
        return;
    }

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

                axum::extract::ws::Message::Text(msg) => {
                    println!("received text message: {:?}", msg);
                    continue;
                    let message: CellChangeMessage = serde_json::from_str(&msg).unwrap();

                    // send message to the Appstate
                    // update_tx.send(message).await.unwrap();

                    // info!("received message: {:?}", message);

                    // change some cell in the board
                    // state.change_cell(&message);

                    // broadcast the message to all clients
                    // state.broadcast(message);
                }

                axum::extract::ws::Message::Binary(data) => {
                    if data.len() == 8 {
                        let packed_value = u64::from_le_bytes(data.try_into().unwrap());
                        let index = (packed_value >> 4) as usize;
                        let color_number = (packed_value & 0xF) as u8;

                        let packed_cell = shared_lib::PackedCell::new(index, color_number);

                        // Send message to the AppState
                        update_tx.send(packed_cell).await.unwrap();
                    } else {
                        info!("invalid binary message length: {:?}", data.len());
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

    let mut handler_sender = tokio::spawn(async move {
        loop {
            match state_receiver.recv().await {
                Ok(msgs) => {
                    // Serialize the Vec<CellChangeMessage> into a binary format
                    let mut buffer = Vec::with_capacity(msgs.len() * 8);
                    for msg in msgs {
                        let packed_value = ((msg.index as u64) << 4) | (msg.value as u64);
                        buffer.extend_from_slice(&packed_value.to_le_bytes());
                    }

                    // if the buffer is exactly the size of the board, add one
                    if buffer.len() == BOARD_SIZE {
                        buffer.push(0);
                    }

                    match sender.send(Message::Binary(buffer)).await {
                        Ok(_) => {}
                        Err(e) => {
                            // try to reconnect?
                            info!("error sending message: {:?}", e);
                            return;
                        }
                    }
                }
                Err(e) => {
                    info!("error receiving message: {:?}", e);
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
}
