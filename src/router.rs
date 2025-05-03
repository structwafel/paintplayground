use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use tower_http::{
    compression::CompressionLayer,
    services::ServeDir,
};

use crate::AppState;
use crate::{board_manager::ChunkRequest, screenshot};
use paintplayground::types::*;

pub fn all_routes(state: AppState) -> Router {
    let compression_layer = CompressionLayer::new().gzip(true);

    Router::new()
        .route_service("/", ServeDir::new("public"))
        .nest_service("/js", ServeDir::new("js"))
        .route("/ws/{x}/{y}", get(crate::ws::ws_handler))
        .route("/chunk/{x}/{y}", get(get_chunk))
        .route("/connections", get(get_connections))
        .route("/screenshot", get(screenshot_handler))
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
    let Ok(coordinates) = ChunkCoordinates::new(x, y) else {
        return Err(axum::http::StatusCode::NOT_FOUND);
    };
    // todo perhaps have  a _raw version that gets the Vec<u8> directly

    let Some(chunk) = state
        .board_communicator
        .get_chunk(coordinates, ChunkRequest::Storage)
        .await
    else {
        return Err(axum::http::StatusCode::NOT_FOUND);
    };

    // return the chunk in binary
    return Ok(chunk.into());
}

#[derive(Deserialize)]
struct ScreenshotQuery {
    x: i64,
    y: i64,
    x2: Option<i64>,
    y2: Option<i64>,
    q: Option<u8>,
}

#[axum::debug_handler]
async fn screenshot_handler(
    Query(params): Query<ScreenshotQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let ScreenshotQuery { x, y, x2, y2, q } = params;
    let (x2, y2) = (x2.unwrap_or(x), y2.unwrap_or(y));
    let q = q.unwrap_or(4).min(8); // max quality

    if x > x2 || y < y2 {
        debug!("Invalid coordinates: x={} y={} x2={} y2={}", x, y, x2, y2);
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let Ok(top_left) = ChunkCoordinates::new(x, y) else {
        debug!("top_left not found");
        return Err(axum::http::StatusCode::NOT_FOUND);
    };

    let Ok(bottom_right) = ChunkCoordinates::new(x2, y2) else {
        debug!("bottom_right not found");
        return Err(axum::http::StatusCode::NOT_FOUND);
    };

    let chunks = state
        .board_communicator
        .get_screenshot_chunks(top_left, bottom_right)
        .await
        .map_err(|err| {
            error!("fetching screenshot chunks failed: {:?}", err);
            &axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let screenshot = screenshot::Screenshot::from_chunks(chunks);

    let png_buffer = screenshot.create_png(q);

    Ok(axum::response::Response::builder()
        .header("Content-Type", "image/png")
        .header("Content-Length", png_buffer.len().to_string())
        .header("Cache-Control", "no-cache")
        .body(Body::from(png_buffer))
        .unwrap())
}
