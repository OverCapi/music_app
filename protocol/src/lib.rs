//! Messages exchanged over the listening-group WebSocket, shared by the server and the app.
//!
//! Wire format: one JSON object per WebSocket text frame, tagged by `"type"`
//! (e.g. `{"type":"add","youtube_id":"...","title":"..."}`).
//!
//! Times are server clock milliseconds since the Unix epoch (`*_ms`); positions are
//! seconds into the current track.
//!
//! The server is the source of truth: after every change it sends the full
//! [`GroupState`] to every member. Clients only send intents ([`ClientMessage`]).
//!
//! Playback model ([`Playback`]):
//! - `Preparing`: the current track was just selected; members download it and send
//!   `Ready`. When every member is ready, or at `deadline_ms`, the server switches
//!   to `Playing` with `at_ms` = now + [`START_LEAD_MS`].
//! - `Playing { position, at_ms }`: the track is at `position` at server time `at_ms`,
//!   so at server time `t >= at_ms` it is at `position + (t - at_ms) / 1000`.
//!   Before `at_ms` the client waits (loaded, paused at `position`), then starts.
//!   A member that becomes ready later joins at the computed position.
//! - `Paused { position }`, `Idle` (nothing to play).

use serde::{Deserialize, Serialize};

/// Delay between the server deciding to (re)start playback and the actual start,
/// so every client receives the message in time.
pub const START_LEAD_MS: u64 = 1500;
/// Delay applied to resume/seek (the file is already loaded everywhere).
pub const RESUME_LEAD_MS: u64 = 500;
/// Maximum time the group waits for slow members before starting anyway.
pub const READY_TIMEOUT_MS: u64 = 30_000;
/// Length of a group invite code.
pub const CODE_LEN: usize = 6;
/// Invite links look like `musicapp://join/ABC123`.
pub const LINK_PREFIX: &str = "musicapp://join/";

/// YouTube video ids are 11 chars of `[A-Za-z0-9_-]`. Clients only ever build the
/// download URL from a validated id, never from a URL sent by another member.
pub fn is_valid_youtube_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Accepts a bare code (`abc123`) or an invite link; returns the uppercase code.
pub fn parse_invite(input: &str) -> Option<String> {
    let input = input.trim();
    let code = input.strip_prefix(LINK_PREFIX).unwrap_or(input);
    let code = code.trim_end_matches('/').to_ascii_uppercase();
    (code.len() == CODE_LEN && code.bytes().all(|b| b.is_ascii_alphanumeric())).then_some(code)
}

pub fn invite_link(code: &str) -> String {
    format!("{LINK_PREFIX}{code}")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Unique within the group (the same video can be queued twice).
    pub entry_id: u64,
    pub youtube_id: String,
    pub title: String,
    pub artist: Option<String>,
    /// Seconds, if known.
    pub duration: Option<f64>,
    /// Username of the member who added it.
    pub added_by: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub username: String,
    /// Has downloaded the current track.
    pub ready: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Playback {
    Idle,
    Preparing { deadline_ms: u64 },
    Playing { position: f64, at_ms: u64 },
    Paused { position: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupState {
    pub code: String,
    pub members: Vec<Member>,
    pub queue: Vec<QueueEntry>,
    /// Index in `queue` of the current track.
    pub current: Option<usize>,
    pub playback: Playback,
}

/// Client → server. A connection is in at most one group at a time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Creates a group and joins it.
    Create,
    /// Joins an existing group by code.
    Join {
        code: String,
    },
    Leave,
    /// Appends a track. If nothing is playing, it becomes current (`Preparing`).
    Add {
        youtube_id: String,
        title: String,
        artist: Option<String>,
        duration: Option<f64>,
    },
    /// Removes a queued track. Removing the current one skips to the next.
    Remove {
        entry_id: u64,
    },
    /// Makes this entry current (`Preparing`).
    PlayEntry {
        entry_id: u64,
    },
    /// Resumes from `Paused`.
    Play,
    Pause,
    Seek {
        position: f64,
    },
    /// Goes to the next track (or `Idle` at the end of the queue).
    Skip,
    /// The sender has downloaded this video.
    Ready {
        youtube_id: String,
    },
    /// The current track finished playing for the sender. Ignored unless
    /// `entry_id` is still the current entry (several members send it).
    Ended {
        entry_id: u64,
    },
    /// Clock sync.
    Ping {
        client_ms: u64,
    },
}

/// Server → client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Full group state, after joining and after every change.
    State {
        state: GroupState,
    },
    /// The sender left its group (reply to `Leave`).
    Left,
    Pong {
        client_ms: u64,
        server_ms: u64,
    },
    /// Human-readable (French) error for the sender only.
    Error {
        message: String,
    },
}

/// REST bodies: `POST /register` and `POST /login` take [`Credentials`] and return
/// [`AuthResponse`] (200) or [`ErrorResponse`] (400 invalid input, 401 bad credentials,
/// 409 username taken). The WebSocket is `GET /ws?token=<token>` (401 if invalid).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub username: String,
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_ids() {
        assert!(is_valid_youtube_id("d7woYjGWs6k"));
        assert!(is_valid_youtube_id("-wHnFMf-bm8"));
        assert!(!is_valid_youtube_id("short"));
        assert!(!is_valid_youtube_id("d7woYjGWs6k&list"));
        assert!(!is_valid_youtube_id("https://x.y"));
    }

    #[test]
    fn invites() {
        assert_eq!(parse_invite("abc123"), Some("ABC123".into()));
        assert_eq!(
            parse_invite(" musicapp://join/AbC123/ "),
            Some("ABC123".into())
        );
        assert_eq!(parse_invite("musicapp://join/ABC12"), None);
        assert_eq!(parse_invite("ABC-12"), None);
        assert_eq!(parse_invite(&invite_link("XYZ789")), Some("XYZ789".into()));
    }

    #[test]
    fn wire_format() {
        let msg: ClientMessage =
            serde_json::from_str(r#"{"type":"seek","position":12.5}"#).unwrap();
        assert_eq!(msg, ClientMessage::Seek { position: 12.5 });
        let json = serde_json::to_string(&Playback::Playing {
            position: 1.0,
            at_ms: 5,
        })
        .unwrap();
        assert_eq!(json, r#"{"status":"playing","position":1.0,"at_ms":5}"#);
    }
}
