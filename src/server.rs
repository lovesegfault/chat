//! Simple chat server

use std::{io, net::SocketAddr};

use futures::{stream::StreamExt, SinkExt};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::broadcast::{
        self,
        error::{RecvError, SendError},
    },
};
use tracing::{debug, error, warn};

use crate::{
    codec::{ChatCodec, ChatCodecError},
    ConcurrentMap,
};

/// Utility alias for the transmission portion of the message channel.
type Tx = broadcast::Sender<String>;

/// Utility alias for the map from channel name to (Vec<Users>, Transmitter).
type Channels = ConcurrentMap<String, (Vec<String>, Tx)>;

/// Error type for `Server` and associated methods.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("failed to bind to address `{0}`")]
    Bind(SocketAddr, #[source] io::Error),
    #[error("never received join command from user at address `{0}`")]
    NoJoin(SocketAddr),
    #[error("invalid join command from user at address `{0}`")]
    InvalidJoin(SocketAddr),
    #[error("failed to broadcast message")]
    BroadcastMessage(#[source] SendError<String>),
    #[error("failed to send message to user at address `{0}`")]
    SendMessage(SocketAddr, #[source] ChatCodecError),
    #[error("failed to add user `{0}` to channel, username in use")]
    UserAlreadyInChannel(String),
    #[error("failed to get local address of the server listener")]
    GetLocalAddress(#[source] io::Error),
}

/// This listens on the specified address for new clients, and then spawns tasks with
/// `Server::handle_client` which deal with the receiving and sending of messages.
pub struct Server {
    listener: TcpListener,
    channels: Channels,
}

impl Server {
    /// Maximum number of messages to hold before we start dropping them from slow clients.
    const MAX_MESSAGES: usize = 1000;

    /// Construct a new [`Server`], binding it to the provided [`SocketAddr`].
    #[tracing::instrument]
    pub async fn new(addr: &SocketAddr) -> Result<Server, ServerError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ServerError::Bind(*addr, e))?;

        let channels = Default::default();

        Ok(Self { listener, channels })
    }

    /// Provide the address the [`Server`] is listening on.
    pub fn local_addr(&self) -> Result<SocketAddr, ServerError> {
        self.listener
            .local_addr()
            .map_err(ServerError::GetLocalAddress)
    }

    /// Start listening for new clients.
    #[tracing::instrument(skip(self))]
    pub async fn listen(&mut self) -> Result<(), ServerError> {
        tracing::info!("server listening");
        loop {
            // wait for a new TcpStream.
            let (socket, addr) = match self.listener.accept().await {
                Ok(x) => x,
                Err(e) => {
                    error!("failed to accept new connection: {}", e);
                    continue;
                }
            };

            // clone the channels map. It's a [`ConcurrentMap`], so the clone is just the (cheap)
            // clone of an [`Arc`].
            let channels = self.channels.clone();

            // Spawn the client handler asynchronously.
            tokio::spawn(async move {
                tracing::debug!("accepted connection");
                if let Err(e) = Self::handle_client(channels, socket, addr).await {
                    warn!("failed to handle client conection: {}", e);
                }
            });
        }
    }

    /// Parses and validates the join command from an user.
    ///
    /// Users are expected to begin their connection to the server with a message specifying the
    /// channel they'd like to join, and their username. To do this, they send a command in the
    /// form `JOIN CHANNEL USERNAME`. If user `bernardo` wanted to join channel `rust`, for
    /// example, he would begin his connection with `JOIN rust bernardo`.
    ///
    /// A number of restrictions exist on this initial string, which this function sets out to
    /// validate, namely they are:
    /// 1. The first term of the string _must_ be "JOIN".
    /// 2. Channel and user names are not allowed any whitespace.
    /// 3. Channel and user names may not be longer than 20 characters.
    /// 4. Only three terms, `JOIN`, `channel_name`, and `username` may be given, and no more.
    pub fn parse_join_command(join_cmd: &str) -> Option<(&str, &str)> {
        let mut cmd_terms = join_cmd
            .split(' ')
            .filter(|term| term.len() <= 20)
            .filter(|term| !term.chars().any(|c| c.is_whitespace()));

        let _header = cmd_terms.next().filter(|&h| h == "JOIN")?;
        let chan_name = cmd_terms.next()?;
        let user_name = cmd_terms.next()?;

        if let Some(_canary) = cmd_terms.next() {
            // if we find a canary, the join command has too many arguments and is invalid
            return None;
        }

        Some((chan_name, user_name))
    }

    /// Handle the connection to a single client.
    ///
    /// This function remains running for as long as the connection to the client is unbroken.
    #[tracing::instrument(skip(channels, stream))]
    pub async fn handle_client<S: AsyncRead + AsyncWrite + Unpin>(
        channels: Channels,
        stream: S,
        addr: SocketAddr,
    ) -> Result<(), ServerError> {
        tracing::debug!("handling client");
        // Wrap the TcpStream in a ChatCodec. This makes it easy for us to write and read lines
        // from the stream.
        let mut chat = ChatCodec::new(stream);

        // A join command must be provided by the user, else we don't know what to do with them.
        let join_cmd = match chat.next().await {
            Some(Ok(line)) => line,
            _ => {
                return Err(ServerError::NoJoin(addr));
            }
        };

        // Validate the join command.
        let (chan_name, user_name) = match Self::parse_join_command(&join_cmd) {
            Some(x) => x,
            None => {
                chat.send("ERROR").await.ok();
                return Err(ServerError::InvalidJoin(addr));
            }
        };

        // We get a reference to the channel the user asked to join, or create a new channel
        // if there is none under that name.
        // Here we also take care to check that the name the user chose is unique, to avoid
        // confusion.
        let channel_tx = {
            let mut channels = channels.lock().await;
            let (users, tx) = channels
                .entry(chan_name.into())
                .or_insert((Vec::new(), broadcast::channel(Self::MAX_MESSAGES).0));
            let user_name = user_name.to_owned();
            if users.contains(&user_name) {
                debug!(
                    "user `{}@{}` attempted to join channel with unavailable username",
                    user_name, addr
                );
                chat.send("ERROR").await.ok();
                Err(ServerError::UserAlreadyInChannel(user_name))
            } else {
                users.push(user_name);
                Ok(tx.clone())
            }
        }?;

        // Create a receiver for the user, this will allow them to read messages from the broadcast
        // channel.
        let mut channel_rx = channel_tx.subscribe();

        // Broadcast to the channel that a new user has joined.
        let join_msg = format!("{} has joined", user_name);
        channel_tx
            .send(join_msg)
            .map_err(ServerError::BroadcastMessage)?;

        // Process incoming messages until we disconnected (or fail.)
        loop {
            tokio::select! {
                // A message was received in our channel, we pass it to the user over TCP.
                result = channel_rx.recv() => match result {
                    Ok(msg) => chat.send(&msg).await.map_err(|e| ServerError::SendMessage(addr, e))?,
                    Err(RecvError::Closed) => {
                        // The channel has no more senders. This should be impossible as a sender
                        // is always kept by the Server, until there are no receivers when it is
                        // dropped.
                        unreachable!()
                    },
                    Err(RecvError::Lagged(num_skipped)) => {
                        // The receiver is lagging, most likely due to this client being too slow.
                        // We report this to the client, but attempt to keep going.
                        chat.send("ERROR".to_owned()).await.map_err(|e| ServerError::SendMessage(addr, e))?;
                        warn!("user `{}@{}` is lagging. {} messages skipped", user_name, addr, num_skipped);
                    },
                },
                // An event on the user's TCP socket has occured
                result = chat.next() => match result {
                    // A message was received, we broadcast it to the channel.
                    Some(Ok(msg)) => {
                        let msg = format!("{}: {}", user_name, msg);
                        // channel.broadcast(&addr, &msg).await;
                        channel_tx.send(msg).map_err(ServerError::BroadcastMessage)?;
                    }
                    // Some form of error occured
                    Some(Err(e)) => {
                        warn!("error while processing message from user `{}@{}` on channel `{}`: {}", user_name, addr, chan_name, e);
                    }
                    // The stream is over, we are done!
                    None => {
                        debug!("user `{}@{}` disconnected", user_name, addr);
                        break;
                    }
                }
            }
        }

        // If this line is reached the client is disconnected, therefore we must notify the channel
        // and drop their receiver.
        let leave_msg = format!("{} has left", user_name);
        channel_tx
            .send(leave_msg)
            .map_err(ServerError::BroadcastMessage)?;

        drop(channel_rx);

        // Finally, if the channel is now empty, we can drop it.
        if channel_tx.receiver_count() == 0 {
            debug!("channel `{}` is now empty and will be deleted.", chan_name);
            channels.lock().await.remove(chan_name);
        }

        Ok(())
    }
}
