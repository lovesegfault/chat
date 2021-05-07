mod common;

use std::sync::Arc;

use ahash::AHashSet;
use anyhow::Error;
use chat::client::Client; // we use the real client here to avoid timeouts
use common::TestServer as Server;
use futures::{
    future,
    stream::{self, StreamExt},
};
use tokio::{sync::Mutex, task::JoinHandle};

const CONCURRENCY_LIMIT: usize = 2000;

#[tokio::test]
async fn test_many_simultaneous_connections() -> Result<(), Error> {
    let server = Server::new().await?;
    let socket = server.socket;

    let user_table: Arc<Mutex<AHashSet<String>>> = Arc::new(Mutex::new(
        (0..CONCURRENCY_LIMIT)
            .into_iter()
            .map(|i| format!("user_{} has joined", i))
            .collect(),
    ));

    let mut observer = Client::new(&socket).await?;
    observer.send("JOIN test observer").await?;
    observer.recv().await?;

    let _users =
        stream::iter(0..CONCURRENCY_LIMIT)
            .then(|i| async move {
                tokio::spawn(async move {
                    let mut client = Client::new(&socket).await?;
                    client.send(&format!("JOIN test user_{}", i)).await?;
                    Ok(client)
                })
            })
            .then(|handle: JoinHandle<Result<Client, Error>>| async move {
                handle.await.unwrap().unwrap()
            })
            .collect::<Vec<Client>>()
            .await;

    observer
        .into_inner()
        .by_ref()
        .take(CONCURRENCY_LIMIT)
        .filter_map(|f| future::ready(f.ok()))
        .map(|msg| (msg, user_table.clone()))
        .for_each_concurrent(None, |(msg, table)| async move {
            match table.lock().await.remove(&msg) {
                true => (),
                false => panic!("Message already popped from table: `{}`", msg),
            }
        })
        .await;

    Ok(())
}
