use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::info;

use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Error)]
pub enum RoomError {
    #[error("User failed to join the room")]
    JoinError(#[from] io::Error),
}

/// Room are multiple users chatting with eachother.
/// Technically it's hodling a websocket connection to each of the users,
/// and broadcasts any message sent within the room.

pub struct Room {
    peer_map: Arc<Mutex<HashMap<SocketAddr, UnboundedSender<Message>>>>,
    listener: TcpListener,
}

impl Room {
    pub fn new(listener: TcpListener) -> Room {
        Room {
            peer_map: Arc::new(Mutex::new(HashMap::new())),
            listener,
        }
    }

    pub async fn run(&mut self) -> Result<(), RoomError> {
        while let (stream, addr) = self.listener.accept().await? {
            info!(%addr, "New user from {addr} incoming");
            let mut peers = self.peer_map.clone();
            tokio::spawn(handle_user(peers, stream, addr));
        }
        Ok(())
    }
}

async fn handle_user(
    mut peer_map: Arc<Mutex<HashMap<SocketAddr, UnboundedSender<Message>>>>,
    stream: TcpStream,
    addr: SocketAddr,
) {
    info!(%addr, "user joins the room");
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("failed to accept stream");

    let (mut tx, rx) = unbounded();
    peer_map.lock().await.insert(addr, tx);

    let (mut outgoing, mut incoming) = ws_stream.split();
    let message_incoming = incoming.try_for_each(|msg| {
        match msg {
            Message::Text(txt) => {
                info!(%addr, msg = ?txt, "message ");
            }
            Message::Binary(_) => {}
            Message::Ping(_) => {}
            Message::Pong(_) => {}
            Message::Close(_) => {}
            Message::Frame(_) => {}
        }
        future::ok(())
    });
    let broadcast_message = rx.map(Ok).forward(outgoing);
    future::select(message_incoming, broadcast_message).await;
    info!(%addr, "user disconnected");
}
