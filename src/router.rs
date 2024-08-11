use axum::{extract::State, routing::get, Extension, Router};
use tokio::sync::{
    broadcast::{self, Receiver},
    mpsc,
};
use tower_http::{
    services::ServeFile,
    trace::{DefaultMakeSpan, TraceLayer},
};

use crate::types;
use crate::types::*;
use crate::AppState;

pub fn all_routes(
    state: AppState,
    boardcast_receiver: broadcast::Receiver<Vec<PackedCell>>,
    manager_sender: mpsc::Sender<PackedCell>,
) -> Router {
    Router::new()
        .route_service("/", ServeFile::new("public/index.html"))
        .route("/ws", get(crate::ws::ws_handler))
        .route("/board", get(get_board))
        .route("/connections", get(get_connections))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(types::Receiver(boardcast_receiver)))
        .layer(Extension(UpdateTransmitter(manager_sender)))
        .with_state(state)
}

async fn get_connections(State(state): State<AppState>) -> String {
    format!(
        "Connections {}",
        state.connections.load(std::sync::atomic::Ordering::Relaxed)
    )
}

async fn get_board(State(board): State<AppState>) -> Vec<u8> {
    let board = board.board.read().await;

    // return the board in binary
    let board_vec = board.to_vec();

    drop(board);
    return board_vec;
}
