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

async fn handle_connection(stream: TcpStream) {
    let addr = stream.peer_addr().expect("Can't get perr addr");
    info!(%addr, "handle connection");
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("failed to accept stream");
    let (mut write, mut read) = ws_stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(1000));
    let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(5000)); // Send PING every 5 secs
    let mut liveness_timer = tokio::time::interval(Duration::from_millis(15_000)); // Close connection after 15 secs of not responding to ping.

    liveness_timer.reset(); //reset the timer, so we won't hit it at the beginning
    loop {
        tokio::select! {
            msg = read.next() => {
                if let Some(msg) = msg {
                    let msg = msg.expect("failed to parse message?");
                    match msg {
                        Message::Text(txt) => {
                            info!(%addr, txt = txt, "text received");
                            write.send(Message::Text(txt.clone())).await.expect("failed to send txt");
                        }
                        Message::Binary(_bin) => {
                            info!(%addr, "bin info")
                        }
                        Message::Close(_) => {
                            info!(%addr, "close message received");
                            break;
                        }
                        Message::Pong(_) => {
                            info!(%addr, "pong received, response timer reset");
                            liveness_timer.reset();
                        }
                        _ => {
                            info!("other message received {:?}", msg)
                        }
                    }
                }
            }
            _ = interval.tick() => {
            }
            _ = heartbeat_interval.tick() => {
                write.send(Message::Ping(vec![])).await.expect("ping fail");
            }
            _ = liveness_timer.tick() => {
                info!(%addr, "response timer hit, dropping dead connection");
                break;
            }
        }
    }
}
