use std::{
    net::SocketAddr,
    sync::{atomic::AtomicUsize, Arc},
};

use mimalloc::MiMalloc;

// When alot of connections are made at the same time, default allocator doesn't release the memory at all.
// https://github.com/hyperium/hyper/issues/1790#issuecomment-2170928353
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// use jemallocator::Jemalloc;

// #[global_allocator]
// static GLOBAL: Jemalloc = Jemalloc;

use tracing::debug;

mod manager;
mod router;
#[cfg(test)]
mod tests;
mod types;
mod ws;

use types::*;

const CLEAR_BUFFER_INTERVAL: u64 = 1;

#[derive(Debug, Clone)]
struct AppState {
    board: Board,
    connections: Arc<AtomicUsize>,
}

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .with_target(false)
        .init();

    let (broadcast_sender, boardcast_receiver) = tokio::sync::broadcast::channel(1_000);
    let (manager_sender, manager_receiver) = tokio::sync::mpsc::channel::<PackedCell>(1_000);

    // manages the canvas and message passing to websockets
    let canvas_manager = manager::CanvasManager::new(broadcast_sender);

    // state of the application
    let state = AppState {
        board: canvas_manager.board.clone(),
        connections: Arc::new(AtomicUsize::new(0)),
    };

    // spawn a task to flush the buffer every second
    // to send the updates to the clients
    tokio::spawn(async move {
        canvas_manager.run(manager_receiver).await;
    });

    let app = router::all_routes(state, boardcast_receiver, manager_sender);

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    debug!("listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
