use super::*;

#[tokio::test]
async fn test_websocket_clients() {
    let n_clients = 10000; // or any number you want to test with
    client::spawn_clients(n_clients).await;
}
