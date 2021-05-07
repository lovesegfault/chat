#![allow(dead_code)]

use std::{
    future::Future,
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::{anyhow, Error};
use chat::client::Client;
use chat::server::Server;
use tokio::{task::JoinHandle, time::timeout};

pub struct TestServer {
    pub socket: SocketAddr,
    handle: JoinHandle<Result<(), Error>>,
}

impl TestServer {
    pub async fn new() -> Result<Self, Error> {
        let mut server = Server::new(&SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0)).await?;
        let socket = server.local_addr()?;
        let handle = tokio::spawn(async move {
            server.listen().await?;
            Ok(())
        });
        Ok(Self { socket, handle })
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub struct TestClient(Client);

impl TestClient {
    const TIMEOUT: Duration = Duration::from_millis(10);

    async fn timeout_call<T: Future>(f: T) -> Result<T::Output, Error> {
        match timeout(Self::TIMEOUT, f).await {
            Ok(f) => Ok(f),
            Err(_) => Err(anyhow!("Client timed-out")),
        }
    }

    pub async fn new(server_addr: &SocketAddr) -> Result<Self, Error> {
        let client = Self::timeout_call(Client::new(server_addr)).await??;
        Ok(Self(client))
    }

    pub async fn send(&mut self, msg: &str) -> Result<(), Error> {
        Self::timeout_call(self.0.send(msg)).await??;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<String, Error> {
        let msg = Self::timeout_call(self.0.recv()).await??;
        Ok(msg)
    }
}
