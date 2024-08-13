use std::{
    net::SocketAddr,
    sync::{atomic::AtomicUsize, Arc},
};

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

#[derive(Debug)]
struct AppState {
    pub board_communicator: board_manager::BoardManagerCommunicator,
    connections: AtomicUsize,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            board_communicator: self.board_communicator.clone(),
            connections: AtomicUsize::new(
                self.connections.load(std::sync::atomic::Ordering::SeqCst),
            ),
        }
    }
}

impl AppState {
    pub fn add_connection(&self) {
        self.connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn remove_connection(&self) {
        self.connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("hyper=error".parse().unwrap())
                .add_directive("tokio=error".parse().unwrap()),
        )
        // .with_max_level(tracing::Level::ERROR)
        .with_target(false)
        .init();

    let chunk_saver = Arc::new(SimpleToFileSaver {});

    // start THE BoardManager
    let board_manager_communicator = board_manager::BoardManager::start(chunk_saver);

    // state of the application
    let state = AppState {
        connections: AtomicUsize::new(0),
        board_communicator: board_manager_communicator,
    };

    // spawn a task to flush the buffer every second
    // to send the updates to the clients
    // tokio::spawn(async move {
    // board_manager.run(manager_receiver).await;
    // });

    let app = router::all_routes(state);

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
