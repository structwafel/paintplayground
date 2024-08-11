use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};

use futures::{
    sink::SinkExt,
    stream::{SplitSink, StreamExt},
};
use shared_lib::{PackedCell, BOARD_SIZE};
use tokio::{
    sync::{broadcast, mpsc, oneshot, RwLock},
    time,
};
use tower_http::{
    services::ServeFile,
    trace::{DefaultMakeSpan, TraceLayer},
};

#[cfg(test)]
mod test;

use tracing::{debug, info};

const CLEAR_BUFFER_INTERVAL: u64 = 5;

type NotifyCellChangeReceiver = broadcast::Receiver<Vec<PackedCell>>;
type NotifyCellChangeSender = broadcast::Sender<Vec<PackedCell>>;

type CellChangeReceiver = mpsc::Receiver<shared_lib::PackedCell>;
type CellChangeSender = mpsc::Sender<shared_lib::PackedCell>;

type BoardRequester = mpsc::Sender<oneshot::Sender<Arc<Chunk>>>;

type Color = u8;
type Chunk = Box<[Color; BOARD_SIZE]>;

#[inline]
fn new_board() -> Chunk {
    [0; BOARD_SIZE].into()
}

#[derive(Debug, Clone)]
struct AppState {
    // the game board
    board: Arc<RwLock<Chunk>>,

    // the broadcast sender to send messages to all clients
    client_sender: NotifyCellChangeSender,
}

impl AppState {
    fn new(sender: NotifyCellChangeSender) -> Self {
        Self {
            board: Arc::new(RwLock::new(new_board())),
            client_sender: sender,
        }
    }

    async fn run(mut self, mut recieve_cell_changes: CellChangeReceiver) {
        // recieve updates, and buffer them

        let mut changed;
        loop {
            let mut smaller_buffer = Vec::new();
            changed = false;
            let timeout = time::sleep(Duration::from_secs(CLEAR_BUFFER_INTERVAL));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Some(change) = recieve_cell_changes.recv() => {
                        smaller_buffer.push(change);
                        // buffer[change.index()] = change.value();
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
            // only take the last of each unique indes
            let mut last_changes: Vec<PackedCell> = Vec::with_capacity(smaller_buffer.len());
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

            // apply the changes to the board
            {
                let mut board = self.board.write().await;

                for change in last_changes.iter() {
                    board[change.index()] = change.value();
                }
            }

            self.broadcast(last_changes);
        }
    }

    fn broadcast(&mut self, messages: Vec<PackedCell>) {
        self.client_sender.send(messages).unwrap();
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
    let state_clone = state.clone();
    tokio::spawn(async move {
        state_clone.run(manager_receiver).await;
    });

    // Spawn a task to handle board state requests
    let (board_request_tx, mut board_request_rx) =
        mpsc::channel::<oneshot::Sender<Arc<RwLock<Chunk>>>>(2);
    let board_clone = state.board.clone();
    tokio::spawn(async move {
        while let Some(reply_tx) = board_request_rx.recv().await {
            let board = board_clone.clone();
            let _ = reply_tx.send(board);
        }
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
        .layer(Extension(board_request_tx));

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
    Extension(board_request_tx): Extension<BoardRequester>,
    // State(state): State<AppState>,
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.

    // let board = board.lock().unwrap().clone();

    // ws.on_upgrade(move |socket| handle_socket(socket, reciever, update_tx, state))
    ws.on_upgrade(move |socket| handle_socket(socket, reciever, update_tx, board_request_tx))
    // let board = board.lock().unwrap().clone();

    // ws.on_upgrade(move |socket| handle_socket(socket, reciever, update_tx))
}

async fn handle_socket(
    socket: WebSocket,
    mut state_receiver: NotifyCellChangeReceiver,
    update_tx: CellChangeSender,
    board_request_tx: BoardRequester,
    // state: AppState,
) {
    // handle the websocket
    let (mut sender, mut receiver) = socket.split();

    {
        // Request full board state using a oneshot channel
        let (board_tx, board_rx) = oneshot::channel();
        board_request_tx.send(board_tx).await.unwrap();

        // Receive and send the full board state
        let full_board = board_rx.await.unwrap();
        send_full_board(full_board, &mut sender).await;
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
                axum::extract::ws::Message::Text(_) => todo!(),
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

    // Receive messages from the AppState and send them to the client
    //
    // The messages will be the buffered changes
    let mut handler_sender = tokio::spawn(async move {
        loop {
            match state_receiver.recv().await {
                Ok(packed_cells) => {
                    match send_chunk_update(packed_cells, &mut sender).await {
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

async fn send_full_board(board: Arc<Chunk>, sender: &mut SplitSink<WebSocket, Message>) {
    let mut board_message = Vec::with_capacity(BOARD_SIZE + 1);
    board_message.push(0x00); // 0x00 indicates full board
    board_message.extend_from_slice(&board.to_vec());
    sender.send(Message::Binary(board_message)).await.unwrap();
}

async fn send_chunk_update(
    packed_cells: Vec<PackedCell>,
    sender: &mut SplitSink<WebSocket, Message>,
) -> Result<(), axum::Error> {
    // Serialize the Vec<CellChangeMessage> into a binary format
    let mut buffer = Vec::with_capacity(packed_cells.len() * 8 + 1);
    buffer.push(0x01); // 0x01 indicates chunk updates
    for packed_cell in packed_cells {
        buffer.extend_from_slice(&packed_cell.to_binary());
    }

    sender.send(Message::Binary(buffer)).await
}
