use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, Route},
    Router,
};
use tower_http::{
    compression::CompressionLayer,
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use crate::board_manager::ChunkRequest;
use crate::types::*;
use crate::AppState;

pub fn all_routes(state: AppState) -> Router {
    let compression_layer = CompressionLayer::new().gzip(true);
    Router::new()
        .nest_service("/", ServeDir::new("public"))
        .nest_service("/js", ServeDir::new("js"))
        .route("/ws/:x/:y", get(crate::ws::ws_handler))
        .route("/chunk/:x/:y", get(get_chunk))
        .route("/connections", get(get_connections))
        // .layer(
        //     TraceLayer::new_for_http()
        //         .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        // )
        .layer(compression_layer)
        .with_state(state)
}

async fn get_connections(State(state): State<AppState>) -> String {
    format!(
        "Connections {}",
        state.connections.load(std::sync::atomic::Ordering::Relaxed)
    )
}

#[axum::debug_handler]
async fn get_chunk(
    Path((x, y)): Path<(i64, i64)>,
    State(state): State<AppState>,
) -> Result<Vec<u8>, impl IntoResponse> {
    // check based on the user if they are allowed to get these coordinates
    let coordinates = ChunkCoordinates::new(x, y);

    // todo perhaps have  a _raw version that gets the Vec<u8> directly

    let Some(chunk) = state
        .board_communicator
        .get_chunk(coordinates, ChunkRequest::Storage)
        .await
    else {
        return Err(axum::http::StatusCode::NOT_FOUND);
    };

    // return the chunk in binary
    return Ok(chunk.clone().into());
}
