use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{Context, Error};
use structopt::StructOpt;
use tracing::{info, Level};

use chat::server::Server;
use tracing_subscriber::fmt::time::ChronoUtc;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "chat",
    author = "Bernardo Meurer Costa",
    about = "Simple chat server"
)]
struct Opt {
    #[structopt(default_value = "1234")]
    port: u16,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Error> {
    // Configures a `tracing` subscriber using UTC time.
    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_timer(ChronoUtc::default())
        .with_max_level(Level::DEBUG)
        .init();

    // Parse CLI args
    let opt = Opt::from_args();

    // Construct bind address for server
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), opt.port);

    // Create and bind the server to the address
    let mut server = Server::new(&addr)
        .await
        .with_context(|| "failed to create chat server")?;
    info!("created server at {}", addr);

    // Start listening for clients
    server
        .listen()
        .await
        .with_context(|| "server encountered an error")?;

    Ok(())
}
