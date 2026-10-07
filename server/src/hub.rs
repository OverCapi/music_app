//! Routes messages between connections and groups. Everything happens under one
//! short-lived lock, and outgoing messages go through per-connection channels so
//! the lock is never held across I/O.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use group_protocol::{parse_invite, ClientMessage, ServerMessage, CODE_LEN};
use tokio::sync::mpsc::UnboundedSender;

use crate::group::{ConnId, Group};

const ERR_NO_GROUP: &str = "Tu n'es dans aucun groupe";
const ERR_UNKNOWN_GROUP: &str = "Groupe introuvable";
const CODE_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Serialized JSON frames waiting to be written to a socket.
pub type Outbox = UnboundedSender<String>;

struct Conn {
    username: String,
    outbox: Outbox,
    group: Option<String>,
}

#[derive(Default)]
struct Inner {
    conns: HashMap<ConnId, Conn>,
    groups: HashMap<String, Group>,
    next_conn: ConnId,
    /// Distinguishes groups that reuse a deleted group's code, for stale timers.
    next_group_seq: u64,
    group_seqs: HashMap<String, u64>,
}

pub struct Hub {
    inner: Mutex<Inner>,
    ready_timeout: Duration,
}

impl Hub {
    pub fn new(ready_timeout: Duration) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner::default()),
            ready_timeout,
        })
    }

    pub fn connect(&self, username: String, outbox: Outbox) -> ConnId {
        let mut inner = self.lock();
        inner.next_conn += 1;
        let id = inner.next_conn;
        inner.conns.insert(
            id,
            Conn {
                username,
                outbox,
                group: None,
            },
        );
        id
    }

    /// A disconnect counts as leaving the group.
    pub fn disconnect(&self, conn: ConnId) {
        let mut inner = self.lock();
        leave_group(&mut inner, conn, now_ms());
        inner.conns.remove(&conn);
    }

    /// Replies to a frame that is not a valid [`ClientMessage`].
    pub fn invalid_message(&self, conn: ConnId) {
        send_error(&self.lock(), conn, "Message invalide");
    }

    pub fn handle(self: &Arc<Self>, conn: ConnId, msg: ClientMessage) {
        let mut inner = self.lock();
        let inner = &mut *inner;
        let now = now_ms();
        match msg {
            ClientMessage::Ping { client_ms } => send(
                inner,
                conn,
                &ServerMessage::Pong {
                    client_ms,
                    server_ms: now,
                },
            ),
            ClientMessage::Create => {
                leave_group(inner, conn, now);
                let code = unique_code(&inner.groups);
                inner.next_group_seq += 1;
                inner.group_seqs.insert(code.clone(), inner.next_group_seq);
                let group = Group::new(code.clone(), self.ready_timeout.as_millis() as u64);
                inner.groups.insert(code.clone(), group);
                self.enter_group(inner, conn, code);
            }
            ClientMessage::Join { code } => {
                let Some(code) = parse_invite(&code).filter(|c| inner.groups.contains_key(c))
                else {
                    return send_error(inner, conn, ERR_UNKNOWN_GROUP);
                };
                let current = inner.conns.get(&conn).and_then(|c| c.group.clone());
                if current.as_deref() == Some(code.as_str()) {
                    // Already there: just resend the state (keeps the ready set).
                    let state = inner.groups[&code].state();
                    return send(inner, conn, &ServerMessage::State { state });
                }
                leave_group(inner, conn, now);
                self.enter_group(inner, conn, code);
            }
            ClientMessage::Leave => {
                if !leave_group(inner, conn, now) {
                    return send_error(inner, conn, ERR_NO_GROUP);
                }
                send(inner, conn, &ServerMessage::Left);
            }
            msg => {
                let Some(code) = inner.conns.get(&conn).and_then(|c| c.group.clone()) else {
                    return send_error(inner, conn, ERR_NO_GROUP);
                };
                let Some(group) = inner.groups.get_mut(&code) else {
                    return send_error(inner, conn, ERR_NO_GROUP);
                };
                match group.apply(conn, msg, now) {
                    Ok(true) => {
                        self.arm_timer(inner, &code);
                        broadcast(inner, &code);
                    }
                    Ok(false) => {}
                    Err(message) => send_error(inner, conn, &message),
                }
            }
        }
    }

    fn enter_group(self: &Arc<Self>, inner: &mut Inner, conn: ConnId, code: String) {
        let Some(c) = inner.conns.get_mut(&conn) else {
            return;
        };
        c.group = Some(code.clone());
        let username = c.username.clone();
        if let Some(group) = inner.groups.get_mut(&code) {
            group.add_member(conn, username);
        }
        broadcast(inner, &code);
    }

    /// Schedules the end of `Preparing` if the group just selected a new entry.
    fn arm_timer(self: &Arc<Self>, inner: &mut Inner, code: &str) {
        let Some(request) = inner.groups.get_mut(code).and_then(Group::take_timer) else {
            return;
        };
        let seq = inner.group_seqs[code];
        let hub = Arc::downgrade(self);
        let code = code.to_owned();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(request.delay_ms)).await;
            let Some(hub) = hub.upgrade() else { return };
            let mut inner = hub.lock();
            if inner.group_seqs.get(&code) != Some(&seq) {
                return;
            }
            let started = inner
                .groups
                .get_mut(&code)
                .is_some_and(|g| g.on_deadline(request.generation, now_ms()));
            if started {
                broadcast(&inner, &code);
            }
        });
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // Group operations never leave the maps half-updated, so keep serving.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Removes `conn` from its group (deleting the group if empty). Returns whether it was in one.
fn leave_group(inner: &mut Inner, conn: ConnId, now: u64) -> bool {
    let Some(code) = inner.conns.get_mut(&conn).and_then(|c| c.group.take()) else {
        return false;
    };
    let Some(group) = inner.groups.get_mut(&code) else {
        return true;
    };
    group.remove_member(conn, now);
    if group.is_empty() {
        inner.groups.remove(&code);
        inner.group_seqs.remove(&code);
    } else {
        broadcast(inner, &code);
    }
    true
}

fn broadcast(inner: &Inner, code: &str) {
    let Some(group) = inner.groups.get(code) else {
        return;
    };
    let text = to_json(&ServerMessage::State {
        state: group.state(),
    });
    for conn in group.member_conns() {
        if let Some(c) = inner.conns.get(&conn) {
            let _ = c.outbox.send(text.clone());
        }
    }
}

fn send(inner: &Inner, conn: ConnId, msg: &ServerMessage) {
    if let Some(c) = inner.conns.get(&conn) {
        let _ = c.outbox.send(to_json(msg));
    }
}

fn send_error(inner: &Inner, conn: ConnId, message: &str) {
    send(
        inner,
        conn,
        &ServerMessage::Error {
            message: message.to_owned(),
        },
    );
}

fn to_json(msg: &ServerMessage) -> String {
    serde_json::to_string(msg).expect("server messages always serialize")
}

fn unique_code(groups: &HashMap<String, Group>) -> String {
    loop {
        let mut bytes = [0u8; CODE_LEN];
        // Codes are not secrets worth failing over: fall back to the clock if the OS RNG fails.
        if getrandom::fill(&mut bytes).is_err() {
            let seed = now_ms().to_le_bytes();
            for (i, b) in bytes.iter_mut().enumerate() {
                *b = seed[i % seed.len()].wrapping_add(i as u8 * 31);
            }
        }
        let code: String = bytes
            .iter()
            .map(|b| CODE_ALPHABET[*b as usize % CODE_ALPHABET.len()] as char)
            .collect();
        if !groups.contains_key(&code) {
            return code;
        }
    }
}

/// Server clock: milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
