mod room;

use std::time::Duration;
use std::{env, error::Error};

use crate::room::Room;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tracing::info;
use tracing_subscriber::filter::EnvFilter;

fn setup_logging() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_logging();
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    let mut default_room = Room::new(listener);
    info!("Listening on: {addr}");
    default_room.run().await?;
    Ok(())
}
