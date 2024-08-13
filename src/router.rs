use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::{DefaultMakeSpan, TraceLayer},
};

use crate::board_manager::ChunkRequest;
use crate::types::*;
use crate::AppState;

pub fn all_routes(state: AppState) -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("public"))
        .nest_service("/js", ServeDir::new("js"))
        .route("/ws/:x/:y", get(crate::ws::ws_handler))
        .route("/chunk/:x/:y", get(get_chunk))
        .route("/connections", get(get_connections))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(state)
}

async fn get_connections(State(state): State<AppState>) -> String {
    format!(
        "Connections {}",
        state.connections.load(std::sync::atomic::Ordering::Relaxed)
    )
}

#[axum::debug_handler]
async fn get_chunk(Path((x, y)): Path<(i64, i64)>, State(state): State<AppState>) -> Vec<u8> {
    // check based on the user if they are allowed to get these coordinates
    let coordinates = ChunkCoordinates::new(x, y);

    let Some(chunk) = state
        .board_communicator
        .get_chunk(coordinates, ChunkRequest::Storage)
        .await
    else {
        return vec![];
    };

    // return the chunk in binary
    return chunk.clone().into();
}
