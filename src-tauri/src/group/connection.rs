//! The WebSocket task: connect, reconnect with re-Join and re-sent `Ready`, ping loop.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use group_protocol::{ClientMessage, ServerMessage};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{self, Message};

use super::Inner;
use crate::error::AppError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Reconnection delays; the last one repeats.
const BACKOFF_SECS: [u64; 4] = [1, 2, 5, 10];
/// Clock sync: a quick burst at connection, then a periodic refresh.
const INITIAL_PINGS: u32 = 5;
const INITIAL_PING_INTERVAL: Duration = Duration::from_millis(200);
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// Nothing received for this long (not even a pong) → the connection is dead.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(75);

pub enum Outgoing {
    Msg(ClientMessage),
    /// Leave the group and close the connection.
    Close,
}

enum SocketEnd {
    /// Leave requested (or the membership is over): stop for good.
    Closed,
    /// Unexpected disconnection: reconnect.
    Lost,
}

/// Owns the WebSocket of membership `id`, reconnecting with backoff until the
/// group is left.
pub async fn run(
    inner: Arc<Inner>,
    id: u64,
    url: String,
    mut rx: mpsc::UnboundedReceiver<Outgoing>,
) {
    let mut failures = 0usize;
    loop {
        let connect = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(&url));
        let result = tokio::select! {
            result = connect => result,
            () = wait_for_close(&mut rx) => return,
        };
        let error = match result {
            Ok(Ok((socket, _))) => {
                failures = 0;
                match run_socket(&inner, id, socket, &mut rx).await {
                    SocketEnd::Closed => return,
                    SocketEnd::Lost => "connexion perdue".to_owned(),
                }
            }
            Ok(Err(tungstenite::Error::Http(response))) if response.status() == 401 => {
                // Bad or expired token: retrying would never work.
                inner.fail(id, AppError::invalid("Session invalide, reconnecte-toi"));
                return;
            }
            Ok(Err(e)) => e.to_string(),
            Err(_) => "délai dépassé".to_owned(),
        };
        inner.on_disconnected(id);
        let unreachable = AppError::Network(format!("Serveur injoignable : {error}"));
        if inner.fail_if_joining(id, unreachable) {
            return;
        }
        tracing::info!("group: disconnected ({error}), reconnecting");
        let delay = BACKOFF_SECS[failures.min(BACKOFF_SECS.len() - 1)];
        failures += 1;
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(delay)) => {}
            () = wait_for_close(&mut rx) => return,
        }
    }
}

/// While offline: drops intents (commands already refuse them) until `Close`.
async fn wait_for_close(rx: &mut mpsc::UnboundedReceiver<Outgoing>) {
    while let Some(Outgoing::Msg(_)) = rx.recv().await {}
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

fn frame(msg: &ClientMessage) -> Message {
    Message::text(serde_json::to_string(msg).expect("client messages serialize"))
}

async fn run_socket(
    inner: &Arc<Inner>,
    id: u64,
    socket: Socket,
    rx: &mut mpsc::UnboundedReceiver<Outgoing>,
) -> SocketEnd {
    let (mut sink, mut stream) = socket.split();

    let Some(hello) = inner.on_connected(id) else {
        let _ = sink.close().await;
        return SocketEnd::Closed;
    };
    if sink.send(frame(&hello)).await.is_err() {
        return SocketEnd::Lost;
    }
    let mut awaiting_join = true;
    let mut pings = 0;
    let mut next_ping = tokio::time::Instant::now();
    let mut last_received = tokio::time::Instant::now();

    loop {
        tokio::select! {
            incoming = stream.next() => {
                last_received = tokio::time::Instant::now();
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ServerMessage>(text.as_str()) {
                            Ok(msg) => {
                                if !inner.on_message(id, msg, &mut awaiting_join) {
                                    let _ = sink.close().await;
                                    return SocketEnd::Closed;
                                }
                            }
                            Err(e) => tracing::warn!("group: ignoring unknown message: {e}"),
                        }
                    }
                    Some(Ok(Message::Close(_)) | Err(_)) | None => return SocketEnd::Lost,
                    Some(Ok(_)) => {}
                }
            }
            out = rx.recv() => match out {
                Some(Outgoing::Msg(msg)) => {
                    if sink.send(frame(&msg)).await.is_err() {
                        return SocketEnd::Lost;
                    }
                }
                Some(Outgoing::Close) | None => {
                    let _ = sink.send(frame(&ClientMessage::Leave)).await;
                    let _ = sink.close().await;
                    return SocketEnd::Closed;
                }
            },
            () = tokio::time::sleep_until(next_ping) => {
                if last_received.elapsed() > SILENCE_TIMEOUT {
                    return SocketEnd::Lost;
                }
                let Some(ping) = inner.ping(id) else {
                    let _ = sink.close().await;
                    return SocketEnd::Closed;
                };
                if sink.send(frame(&ping)).await.is_err() {
                    return SocketEnd::Lost;
                }
                pings += 1;
                next_ping += if pings < INITIAL_PINGS {
                    INITIAL_PING_INTERVAL
                } else {
                    PING_INTERVAL
                };
            }
        }
    }
}
