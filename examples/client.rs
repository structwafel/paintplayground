#[tokio::main]
async fn main() {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::new(env_filter)
                .add_directive("hyper=error".parse().unwrap())
                .add_directive("tokio=error".parse().unwrap()),
        )
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let quantity = if args.len() == 2 {
        args[1].parse::<usize>().unwrap()
    } else if args.len() > 2 {
        error!("what, too many things given {:?}", args);
        return;
    } else {
        // default
        1000
    };

    // Call the client function with args
    spawn_clients(quantity).await
}

use std::{borrow::Cow, ops::ControlFlow};

use futures::{SinkExt, StreamExt};

use tokio_tungstenite::connect_async;
use tungstenite::{
    Message,
    protocol::{CloseFrame, frame::coding::CloseCode},
};

use paintplayground::types::*;

const SERVER: &str = "ws://localhost:3001/ws/0/0";
const TIMEOUT: u64 = 10;

pub async fn spawn_clients(n: usize) {
    let mut clients = Vec::new();
    for i in 0..n {
        clients.push(spawn_client(i));
    }

    // await all clients to finish
    futures::future::join_all(clients).await;
}

//creates a client. quietly exits on failure.
async fn spawn_client(who: usize) {
    let ws_stream = match connect_async(SERVER).await {
        Ok((stream, _response)) => {
            // println!("Handshake for client {who} has been completed");
            // This will be the HTTP response, same as with server this is the last moment we
            // can still access HTTP stuff.
            // println!("Server response was {response:?}");
            stream
        }
        Err(e) => {
            println!("WebSocket handshake for client {who} failed with {e}!");
            return;
        }
    };

    let (mut sender, mut receiver) = ws_stream.split();

    //spawn an async sender to push some more messages into the server
    let mut send_task = tokio::spawn(async move {
        loop {
            let random_index = rand::random_range(0..CHUNK_SIZE);
            let random_color = rand::random_range(0..=15);

            let packed_cell = PackedCell::new(random_index, random_color).unwrap();

            // In any websocket error, break loop.

            if sender
                .send(Message::Binary(packed_cell.to_vec()))
                .await
                .is_err()
            {
                //just as with server, if send fails there is nothing we can do but exit.
                return;
            }

            tokio::time::sleep(std::time::Duration::from_millis(TIMEOUT)).await;
        }

        // When we are done we may want our client to close connection cleanly.
        println!("Sending close to {who}...");
        if let Err(e) = sender
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: Cow::from("Goodbye"),
            })))
            .await
        {
            println!("Could not send Close due to {e:?}, probably it is ok?");
        };
    });

    //receiver just prints whatever it gets
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // let number = rand::random::<u8>();
            // if number < 10 {
            //     // request the entire board,
            //     let response = reqwest::get("http://localhost:3000/board").await.unwrap();
            //     let board = response.bytes().await.unwrap();

            //     // check if the frist 5 entries in the byte array are 0.
            //     for entry in board.to_vec().iter().take(10) {
            //         if entry == &0x20 {
            //             println!("entry is 20")
            //         }
            //     }
            //     drop(board);
            // }

            // print message and break if instructed to do so
            if process_message(msg, who).is_break() {
                break;
            }
        }
    });

    //wait for either task to finish and kill the other task
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
}

fn process_message(msg: Message, who: usize) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(_t) => {
            // println!(">>> {who} got str: {t:?}");
        }
        Message::Binary(_d) => {
            // println!(">>> {} got {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} got close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow got close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(_v) => {
            // println!(">>> {who} got pong with {v:?}");
        }
        Message::Ping(v) => {
            println!(">>> {who} got ping with {v:?}");
        }

        Message::Frame(_) => {
            unreachable!("This is never supposed to happen")
        }
    }
    ControlFlow::Continue(())
}
