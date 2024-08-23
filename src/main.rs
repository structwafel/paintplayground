use std::{net::SocketAddr, sync::atomic::AtomicUsize};

use chunk_db::SimpleToFileSaver;
use mimalloc::MiMalloc;

// When alot of connections are made at the same time, default allocator doesn't release the memory at all.
// https://github.com/hyperium/hyper/issues/1790#issuecomment-2170928353
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// use jemallocator::Jemalloc;

// #[global_allocator]
// static GLOBAL: Jemalloc = Jemalloc;

use tracing::debug;

mod board_manager;
mod chunk_db;
mod chunk_manager;
mod router;
#[cfg(test)]
mod tests;
mod types;
mod ws;

use types::*;

const CLEAR_BUFFER_INTERVAL: u64 = 1;

#[derive(Debug, Clone)]
struct AppState {
    pub board_communicator: board_manager::BoardManagerCommunicator,
    connections: Arc<AtomicUsize>,
}

impl AppState {
    pub fn new(board_communicator: board_manager::BoardManagerCommunicator) -> Self {
        Self {
            board_communicator,
            connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn add_connection(&self) {
        debug!("Adding connection");
        self.connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!(
            "Connections {}",
            self.connections.load(std::sync::atomic::Ordering::Relaxed)
        );
    }

    pub fn remove_connection(&self) {
        debug!("Removing connection");
        self.connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug!(
            "Connections {}",
            self.connections.load(std::sync::atomic::Ordering::Relaxed)
        );
    }
}

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::new(env_filter)
                .add_directive("hyper=error".parse().unwrap())
                .add_directive("tokio=error".parse().unwrap()),
        )
        .with_target(false)
        .init();

    startup_things().await;

    let chunk_saver = SimpleToFileSaver {};

    // start THE BoardManager
    let board_manager_communicator = board_manager::BoardManager::start(chunk_saver);

    // state of the application
    let state = AppState::new(board_manager_communicator);

    let app = router::all_routes(state);

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn startup_things() {
    // create the canvas dir if it doesn't exist
    let canvas_dir = "canvas";
    if !std::path::Path::new(canvas_dir).exists() {
        std::fs::create_dir(canvas_dir).unwrap();
    }
}
