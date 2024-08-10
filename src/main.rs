use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
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
use serde::{Deserialize, Serialize};
use shared_lib::{CellChangeMessage, BOARD_SIZE};
use tokio::sync::broadcast;
use tower_http::{
    services::ServeFile,
    trace::{DefaultMakeSpan, TraceLayer},
};

mod test;

use tracing::{debug, error, info};

type CellChangeReceiver = broadcast::Receiver<CellChangeMessage>;
type CellChangeSender = broadcast::Sender<CellChangeMessage>;

type Color = u8;
type Chunk = [Color; BOARD_SIZE];

// TODO hold all conenctions in a vec, loop through and check if connected, then send buffer to all connected clients

fn new_board() -> Chunk {
    [0; BOARD_SIZE]
}

#[derive(Debug, Clone)]
struct AppState {
    // the game board
    board: Arc<Mutex<Chunk>>,

    // the broadcast sender to send messages to all clients
    client_sender: CellChangeSender,
}

impl AppState {
    fn new(sender: CellChangeSender) -> Self {
        Self {
            board: Arc::new(Mutex::new(new_board())),
            client_sender: sender,
        }
    }

    fn broadcast(&mut self, message: CellChangeMessage) {
        // println!(
        //     "broadcast buffer size: {}",
        //     self.client_sender.receiver_count()
        // );
        self.client_sender.send(message).unwrap();
    }

    fn change_cell(&mut self, change: &CellChangeMessage) {
        let mut board = self.board.lock().unwrap();

        if change.index >= BOARD_SIZE {
            return;
        }

        board[change.index] = change.value;
    }

    fn get_board(&self) -> Chunk {
        let board = self.board.lock().unwrap();
        *board
    }
}

struct Receiver(CellChangeReceiver);
impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self(self.0.resubscribe())
    }
}

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .with_target(false)
        .init();

    let (sender, receiver) = broadcast::channel(1_000_000);
    let state = AppState::new(sender);

    // build our application with some routes
    let app = Router::new()
        .route_service("/", ServeFile::new("public/index.html"))
        .route("/ws", get(ws_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(Receiver(receiver)))
        .with_state(state);

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
    State(state): State<AppState>,
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.

    ws.on_upgrade(move |socket| handle_socket(socket, reciever, state))
}

async fn handle_socket(
    socket: WebSocket,
    mut state_receiver: CellChangeReceiver,
    mut state: AppState,
) {
    // handle the websocket
    let (mut sender, mut receiver) = socket.split();

    // Send the entire board state to the client upon connection
    let board_state = state.get_board(); // Assuming this method exists

    // board_state is a 2D array, we need to convert it to a 1D array of bytes.
    // first all the x values, then all the y values.
    // let board_state: Vec<u8> = board_state.iter().flatten().copied().collect();

    if let Err(e) = sender.send(Message::Binary(board_state.into())).await {
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
                    let message: CellChangeMessage = serde_json::from_str(&msg).unwrap();

                    // info!("received message: {:?}", message);

                    // change some cell in the board
                    state.change_cell(&message);

                    // broadcast the message to all clients
                    state.broadcast(message);
                }

                axum::extract::ws::Message::Binary(_) => todo!(),
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
                Ok(msg) => {
                    let msg = serde_json::to_string(&msg).unwrap();
                    match sender.send(msg.into()).await {
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
