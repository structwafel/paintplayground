use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post, Route},
    Router,
};
use tower_http::{
    compression::CompressionLayer,
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use crate::types::*;
use crate::AppState;
use crate::{board_manager::ChunkRequest, jwt, utils::password};

pub fn all_routes(state: AppState) -> Router {
    let compression_layer = CompressionLayer::new().gzip(true);
    Router::new()
        .nest_service("/", ServeDir::new("public"))
        .nest_service("/js", ServeDir::new("js"))
        .route("/ws/:x/:y", get(crate::ws::ws_handler))
        .route("/chunk/:x/:y", get(get_chunk))
        .route("/connections", get(get_connections))
        .route("/login", post(login))
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
    return Ok(chunk.clone().into());
}

#[derive(serde::Deserialize)]
struct LoginData {
    username: String,
    password: String,
}

#[derive(serde::Serialize)]
struct LoginResponse {
    token: String,
}

async fn login(
    login_data: axum::Json<LoginData>,
    State(state): State<AppState>,
) -> Result<axum::Json<LoginResponse>, impl IntoResponse> {
    let password_data = state
        .db
        .get_user_password_by_user_id(&login_data.username)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or((axum::http::StatusCode::UNAUTHORIZED, "user not found"))?;

    // check if the password is correct
    password::verify_password(&login_data.password, &password_data)
        .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "password incorrect"))?;

    let token = state
        .jwt
        .create_token(&user.id.to_string())
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(axum::Json(LoginResponse { token }))
}
