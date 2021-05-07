//! Simple chat client.

use std::{io, net::SocketAddr};

use futures::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::net::TcpStream;

use crate::codec::{ChatCodec, ChatCodecError};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("failed to connect to server at address `{0}`")]
    ConnectToServer(SocketAddr, #[source] io::Error),
    #[error("failed to send message to server")]
    SendMessage(#[source] ChatCodecError),
    #[error("failed to receive message from server")]
    RecvMessage(#[source] ChatCodecError),
    #[error("connection to server closed")]
    ConnectionClosed,
}

/// A basic chat client, made to communicate with [`crate::server::Server`].
///
/// This is mostly used in internal testing, and is a simple wrapper around [`ChatCodec`].
pub struct Client {
    socket: ChatCodec<TcpStream>,
}

impl Client {
    /// Creates a new [`Client`] connected to `server_addr`.
    pub async fn new(server_addr: &SocketAddr) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(server_addr)
            .await
            .map_err(|e| ClientError::ConnectToServer(*server_addr, e))?;
        let chat = ChatCodec::new(stream);
        Ok(Self { socket: chat })
    }

    /// Sends a message to the server.
    pub async fn send(&mut self, msg: &str) -> Result<(), ClientError> {
        self.socket
            .send(msg)
            .await
            .map_err(ClientError::SendMessage)
    }

    /// Receives a message from the server.
    pub async fn recv(&mut self) -> Result<String, ClientError> {
        match self.socket.next().await {
            Some(Ok(msg)) => Ok(msg),
            Some(Err(e)) => Err(ClientError::RecvMessage(e)),
            None => Err(ClientError::ConnectionClosed),
        }
    }

    /// Consumes the client, returning the inner [`ChatCodec`]
    pub fn into_inner(self) -> ChatCodec<TcpStream> {
        self.socket
    }
}
