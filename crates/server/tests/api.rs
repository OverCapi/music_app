//! End-to-end tests against a real server on 127.0.0.1:0 with a temporary database.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use group_protocol::{
    AuthResponse, ClientMessage, ErrorResponse, GroupState, Playback, RESUME_LEAD_MS,
    START_LEAD_MS, ServerMessage,
};
use music_group_server::{Config, serve};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Long enough that it never fires by accident in tests that don't wait for it.
const LONG_TIMEOUT: Duration = Duration::from_secs(30);
const SHORT_TIMEOUT: Duration = Duration::from_millis(300);
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

struct Server {
    addr: String,
    dir: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn start(ready_timeout: Duration) -> Server {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "music-group-server-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let config = Config {
        addr: listener.local_addr().unwrap(),
        db_path: dir.join("test.db"),
        ready_timeout,
    };
    tokio::spawn(async move { serve(listener, config).await.unwrap() });
    Server { addr, dir }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

async fn post(server: &Server, path: &str, username: &str, password: &str) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("http://{}{path}", server.addr))
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .unwrap()
}

async fn register(server: &Server, username: &str) -> String {
    let res = post(server, "/register", username, "secret123").await;
    assert_eq!(res.status(), 200);
    res.json::<AuthResponse>().await.unwrap().token
}

struct Client {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Client {
    async fn connect(server: &Server, username: &str) -> Self {
        let token = register(server, username).await;
        let url = format!("ws://{}/ws?token={token}", server.addr);
        let (ws, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        Self { ws }
    }

    async fn send(&mut self, msg: ClientMessage) {
        let text = serde_json::to_string(&msg).unwrap();
        self.ws.send(Message::text(text)).await.unwrap();
    }

    async fn recv(&mut self) -> ServerMessage {
        loop {
            let frame = tokio::time::timeout(RECV_TIMEOUT, self.ws.next())
                .await
                .expect("timed out waiting for a server message")
                .expect("connection closed")
                .unwrap();
            if let Message::Text(text) = frame {
                return serde_json::from_str(&text).unwrap();
            }
        }
    }

    async fn state(&mut self) -> GroupState {
        match self.recv().await {
            ServerMessage::State { state } => state,
            other => panic!("expected state, got {other:?}"),
        }
    }

    async fn error(&mut self) -> String {
        match self.recv().await {
            ServerMessage::Error { message } => message,
            other => panic!("expected error, got {other:?}"),
        }
    }

    /// Asserts that nothing arrives for a short while.
    async fn silent(&mut self) {
        let wait = async {
            loop {
                match self.ws.next().await {
                    Some(Ok(Message::Text(text))) => return text,
                    Some(Ok(_)) => continue,
                    other => panic!("connection ended: {other:?}"),
                }
            }
        };
        let res = tokio::time::timeout(Duration::from_millis(150), wait).await;
        assert!(res.is_err(), "unexpected message: {res:?}");
    }

    async fn add(&mut self, youtube_id: &str, title: &str) {
        self.send(ClientMessage::Add {
            youtube_id: youtube_id.into(),
            title: title.into(),
            artist: Some("Artist".into()),
            duration: Some(200.0),
        })
        .await;
    }

    async fn ready(&mut self, youtube_id: &str) {
        self.send(ClientMessage::Ready {
            youtube_id: youtube_id.into(),
        })
        .await;
    }
}

/// A creates a group and B joins it; both have consumed the join broadcasts.
async fn pair(server: &Server) -> (Client, Client, String) {
    let mut a = Client::connect(server, "alice").await;
    let mut b = Client::connect(server, "bob").await;
    a.send(ClientMessage::Create).await;
    let code = a.state().await.code;
    b.send(ClientMessage::Join {
        code: code.to_lowercase(),
    })
    .await;
    assert_eq!(b.state().await.members.len(), 2);
    assert_eq!(a.state().await.members.len(), 2);
    (a, b, code)
}

fn assert_near(actual: u64, expected: u64) {
    assert!(
        actual.abs_diff(expected) < 300,
        "{actual} is not close to {expected}"
    );
}

const V1: &str = "aaaaaaaaaa1";
const V2: &str = "bbbbbbbbbb2";
const V3: &str = "ccccccccc_3";

#[tokio::test]
async fn accounts() {
    let server = start(LONG_TIMEOUT).await;
    let token = register(&server, "Alice").await;
    assert_eq!(token.len(), 64);

    let res = post(&server, "/login", "alice", "secret123").await;
    assert_eq!(res.status(), 200);
    let auth: AuthResponse = res.json().await.unwrap();
    assert_eq!(auth.username, "Alice");
    assert_ne!(auth.token, token, "each login issues a new token");

    let res = post(&server, "/login", "Alice", "wrong-password").await;
    assert_eq!(res.status(), 401);
    let err: ErrorResponse = res.json().await.unwrap();
    assert_eq!(err.error, "Pseudo ou mot de passe incorrect");
    assert_eq!(
        post(&server, "/login", "nobody", "secret123")
            .await
            .status(),
        401
    );

    let res = post(&server, "/register", "ALICE", "secret123").await;
    assert_eq!(res.status(), 409);
    let err: ErrorResponse = res.json().await.unwrap();
    assert_eq!(err.error, "Ce pseudo est déjà pris");

    for (username, password) in [
        ("ab", "secret123"),
        ("bad name", "secret123"),
        (&"x".repeat(33), "secret123"),
        ("valid_name", "short"),
        ("valid_name", &"p".repeat(129)),
    ] {
        let res = post(&server, "/register", username, password).await;
        assert_eq!(res.status(), 400, "{username:?} / {password:?}");
        res.json::<ErrorResponse>().await.unwrap();
    }
    let res = reqwest::Client::new()
        .post(format!("http://{}/register", server.addr))
        .body("not json")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);

    // Both tokens work for the WebSocket; a bad one is rejected.
    for t in [&token, &auth.token] {
        let url = format!("ws://{}/ws?token={t}", server.addr);
        tokio_tungstenite::connect_async(url).await.unwrap();
    }
    for url in [
        format!("ws://{}/ws?token=deadbeef", server.addr),
        format!("ws://{}/ws", server.addr),
    ] {
        match tokio_tungstenite::connect_async(url).await {
            Err(tungstenite::Error::Http(res)) => assert_eq!(res.status(), 401),
            other => panic!("expected 401, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn create_and_join() {
    let server = start(LONG_TIMEOUT).await;
    let (mut a, mut b, code) = pair(&server).await;
    assert_eq!(code.len(), 6);
    assert!(
        code.bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    );

    let mut c = Client::connect(&server, "carol").await;
    c.send(ClientMessage::Join {
        code: "ZZZZZZ".into(),
    })
    .await;
    assert_eq!(c.error().await, "Groupe introuvable");
    c.send(ClientMessage::Skip).await;
    assert_eq!(c.error().await, "Tu n'es dans aucun groupe");

    // Invalid JSON keeps the connection open.
    c.ws.send(Message::text("{oops")).await.unwrap();
    c.error().await;
    c.send(ClientMessage::Join {
        code: format!("musicapp://join/{code}"),
    })
    .await;
    let state = c.state().await;
    let names: Vec<_> = state.members.iter().map(|m| m.username.as_str()).collect();
    assert_eq!(names, ["alice", "bob", "carol"]);
    assert_eq!(a.state().await, state);
    assert_eq!(b.state().await, state);
}

#[tokio::test]
async fn add_and_ready() {
    let server = start(LONG_TIMEOUT).await;
    let (mut a, mut b, _) = pair(&server).await;

    a.add("not-an-id", "Bad").await;
    assert_eq!(a.error().await, "Identifiant YouTube invalide");
    b.silent().await;

    a.add(V1, "Song").await;
    for client in [&mut a, &mut b] {
        let state = client.state().await;
        assert_eq!(state.current, Some(0));
        assert_eq!(state.queue[0].youtube_id, V1);
        assert_eq!(state.queue[0].added_by, "alice");
        assert!(matches!(state.playback, Playback::Preparing { .. }));
    }

    a.ready(V1).await;
    for client in [&mut a, &mut b] {
        let state = client.state().await;
        assert!(state.members[0].ready && !state.members[1].ready);
        assert!(matches!(state.playback, Playback::Preparing { .. }));
    }

    b.ready(V1).await;
    let sent = now_ms();
    for client in [&mut a, &mut b] {
        match client.state().await.playback {
            Playback::Playing { position, at_ms } => {
                assert_eq!(position, 0.0);
                assert_near(at_ms, sent + START_LEAD_MS);
            }
            other => panic!("expected playing, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn ready_timeout_starts_anyway() {
    let server = start(SHORT_TIMEOUT).await;
    let (mut a, mut b, _) = pair(&server).await;
    let added = now_ms();
    a.add(V1, "Song").await;
    match a.state().await.playback {
        Playback::Preparing { deadline_ms } => assert_near(deadline_ms, added + 300),
        other => panic!("expected preparing, got {other:?}"),
    }
    b.state().await;
    a.ready(V1).await;
    a.state().await;
    b.state().await;
    let state = b.state().await;
    let fired = now_ms();
    assert!(fired >= added + 250, "started before the deadline");
    match state.playback {
        Playback::Playing { at_ms, .. } => assert_near(at_ms, fired + START_LEAD_MS),
        other => panic!("expected playing, got {other:?}"),
    }
    assert!(!state.members[1].ready);
}

/// A and B start playing V1 (and V2, V3 are queued when `extra`).
async fn playing_pair(server: &Server, extra: bool) -> (Client, Client) {
    let (mut a, mut b, _) = pair(server).await;
    for id in [V1, V2, V3] {
        a.ready(id).await;
        b.ready(id).await;
    }
    a.add(V1, "One").await;
    if extra {
        a.add(V2, "Two").await;
        a.add(V3, "Three").await;
    }
    let count = if extra { 3 } else { 1 };
    for _ in 0..count {
        a.state().await;
        b.state().await;
    }
    (a, b)
}

#[tokio::test]
async fn pause_play_seek() {
    let server = start(LONG_TIMEOUT).await;
    let (mut a, mut b) = playing_pair(&server, false).await;

    // Wait until playback actually runs so the paused position is > 0.
    tokio::time::sleep(Duration::from_millis(START_LEAD_MS + 300)).await;
    a.send(ClientMessage::Pause).await;
    let paused = match b.state().await.playback {
        Playback::Paused { position } => position,
        other => panic!("expected paused, got {other:?}"),
    };
    assert!((0.1..1.5).contains(&paused), "position {paused}");
    a.state().await;

    b.send(ClientMessage::Play).await;
    let sent = now_ms();
    match a.state().await.playback {
        Playback::Playing { position, at_ms } => {
            assert_eq!(position, paused);
            assert_near(at_ms, sent + RESUME_LEAD_MS);
        }
        other => panic!("expected playing, got {other:?}"),
    }
    b.state().await;

    a.send(ClientMessage::Seek { position: 61.5 }).await;
    let sent = now_ms();
    match b.state().await.playback {
        Playback::Playing { position, at_ms } => {
            assert_eq!(position, 61.5);
            assert_near(at_ms, sent + RESUME_LEAD_MS);
        }
        other => panic!("expected playing, got {other:?}"),
    }
    a.state().await;
    a.send(ClientMessage::Seek { position: -3.0 }).await;
    assert_eq!(a.error().await, "Position invalide");
}

#[tokio::test]
async fn ended_advances_once() {
    let server = start(LONG_TIMEOUT).await;
    let (mut a, mut b) = playing_pair(&server, true).await;

    a.send(ClientMessage::Ended { entry_id: 1 }).await;
    let state = a.state().await;
    assert_eq!(state.current, Some(1));
    // Everyone already downloaded V2, so it starts right away.
    assert!(matches!(state.playback, Playback::Playing { .. }));
    b.state().await;

    // B's Ended for the previous entry arrives late: ignored.
    b.send(ClientMessage::Ended { entry_id: 1 }).await;
    a.silent().await;
    b.silent().await;

    b.send(ClientMessage::Skip).await;
    assert_eq!(a.state().await.current, Some(2));
    b.state().await;
    a.send(ClientMessage::Ended { entry_id: 3 }).await;
    let state = b.state().await;
    assert_eq!(state.current, None);
    assert_eq!(state.playback, Playback::Idle);
    assert_eq!(state.queue.len(), 3);
}

#[tokio::test]
async fn remove_and_play_entry() {
    let server = start(LONG_TIMEOUT).await;
    let (mut a, mut b) = playing_pair(&server, true).await;

    a.send(ClientMessage::PlayEntry { entry_id: 2 }).await;
    assert_eq!(b.state().await.current, Some(1));
    a.state().await;

    // Before the current entry: the index shifts.
    a.send(ClientMessage::Remove { entry_id: 1 }).await;
    let state = b.state().await;
    assert_eq!(state.current, Some(0));
    assert_eq!(state.queue[0].entry_id, 2);
    a.state().await;

    // The current entry: the next one takes its place.
    a.send(ClientMessage::Remove { entry_id: 2 }).await;
    let state = b.state().await;
    assert_eq!(state.current, Some(0));
    assert_eq!(state.queue.len(), 1);
    assert_eq!(state.queue[0].entry_id, 3);
    a.state().await;

    a.send(ClientMessage::Remove { entry_id: 42 }).await;
    assert_eq!(a.error().await, "Morceau introuvable");
    a.send(ClientMessage::PlayEntry { entry_id: 42 }).await;
    assert_eq!(a.error().await, "Morceau introuvable");
    b.silent().await;

    // Removing the last current entry goes idle.
    a.send(ClientMessage::Remove { entry_id: 3 }).await;
    let state = b.state().await;
    assert_eq!(state.current, None);
    assert_eq!(state.playback, Playback::Idle);
}

#[tokio::test]
async fn leave_and_disconnect() {
    let server = start(LONG_TIMEOUT).await;
    let (mut a, mut b, code) = pair(&server).await;

    a.send(ClientMessage::Leave).await;
    assert_eq!(a.recv().await, ServerMessage::Left);
    let state = b.state().await;
    assert_eq!(state.members.len(), 1);
    assert_eq!(state.members[0].username, "bob");
    a.send(ClientMessage::Leave).await;
    assert_eq!(a.error().await, "Tu n'es dans aucun groupe");

    // The last member disconnects: the group is gone. The server drops the socket
    // only after removing the member, so wait for the end of B's stream first.
    b.ws.close(None).await.unwrap();
    tokio::time::timeout(RECV_TIMEOUT, async {
        while let Some(Ok(_)) = b.ws.next().await {}
    })
    .await
    .expect("server did not close the socket");
    a.send(ClientMessage::Join { code }).await;
    assert_eq!(a.error().await, "Groupe introuvable");
}

#[tokio::test]
async fn ping_pong() {
    let server = start(LONG_TIMEOUT).await;
    let mut a = Client::connect(&server, "alice").await;
    let before = now_ms();
    a.send(ClientMessage::Ping { client_ms: 1234 }).await;
    match a.recv().await {
        ServerMessage::Pong {
            client_ms,
            server_ms,
        } => {
            assert_eq!(client_ms, 1234);
            assert!(server_ms >= before && server_ms <= now_ms());
        }
        other => panic!("expected pong, got {other:?}"),
    }
}
