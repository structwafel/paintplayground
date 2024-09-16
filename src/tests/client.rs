use std::{borrow::Cow, ops::ControlFlow, time::Duration};

use futures::{SinkExt, StreamExt};

use tokio_tungstenite::connect_async;
use tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Message,
};

use crate::types::*;

const SERVER: &str = "ws://127.0.0.1:3001/ws";
const TIMEOUT: u64 = 1000;

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
        Ok((stream, response)) => {
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

    //     // request the entire board,
    // let response = reqwest::get("http://localhost:3001/board").await.unwrap();
    // let board = response.bytes().await.unwrap();

    // // check if the frist 5 entries in the byte array are 0.
    // for entry in board.to_vec().iter().take(10) {
    //     if entry == &0x20 {
    //         println!("entry is 20")
    //     }
    // }
    // drop(board);

    // //we can ping the server for start
    // sender
    //     .send(Message::Ping("Hello, Server!".into()))
    //     .await
    //     .expect("Can not send!");

    //spawn an async sender to push some more messages into the server
    let mut send_task = tokio::spawn(async move {
        loop {
            let random_index = rand::random::<usize>() % 100;
            let random_color = rand::random::<u8>() % 9;

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
        Message::Text(t) => {
            // println!(">>> {who} got str: {t:?}");
        }
        Message::Binary(d) => {
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

        Message::Pong(v) => {
            // println!(">>> {who} got pong with {v:?}");
        }
        // Just as with axum server, the underlying tungstenite websocket library
        // will handle Ping for you automagically by replying with Pong and copying the
        // v according to spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {who} got ping with {v:?}");
        }

        Message::Frame(_) => {
            unreachable!("This is never supposed to happen")
        }
    }
    ControlFlow::Continue(())
}
