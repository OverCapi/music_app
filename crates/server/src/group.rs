//! One listening group as a pure state machine: no I/O, the caller passes the clock.
//!
//! The networking layer feeds it [`ClientMessage`]s, broadcasts [`Group::state`]
//! when [`Group::apply`] reports a change, and arms a timer for every
//! [`Group::take_timer`] request (calling [`Group::on_deadline`] when it fires).

use std::collections::HashSet;

use group_protocol::{
    ClientMessage, GroupState, Member, Playback, QueueEntry, RESUME_LEAD_MS, START_LEAD_MS,
    is_valid_youtube_id,
};

/// Identifies a WebSocket connection (a member is a connection, not a user).
pub type ConnId = u64;

pub const MAX_QUEUE: usize = 500;
const MAX_TEXT: usize = 200;
/// Bounds the memory a client can make us hold with `Ready` messages.
const MAX_READY_IDS: usize = 1000;

pub const ERR_NOT_FOUND: &str = "Morceau introuvable";

struct GroupMember {
    conn: ConnId,
    username: String,
    /// Videos this member has downloaded. Kept as a set because clients
    /// pre-download upcoming tracks and report them before they become current.
    ready_ids: HashSet<String>,
}

/// A request to call [`Group::on_deadline`] with `generation` after `delay_ms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerRequest {
    pub generation: u64,
    pub delay_ms: u64,
}

pub struct Group {
    code: String,
    members: Vec<GroupMember>,
    queue: Vec<QueueEntry>,
    current: Option<usize>,
    playback: Playback,
    next_entry_id: u64,
    /// Bumped whenever an entry becomes current, so stale deadline timers do nothing.
    generation: u64,
    /// Where playback starts once `Preparing` ends (a seek while preparing moves it).
    start_position: f64,
    ready_timeout_ms: u64,
    timer: Option<TimerRequest>,
}

impl Group {
    pub fn new(code: String, ready_timeout_ms: u64) -> Self {
        Self {
            code,
            members: Vec::new(),
            queue: Vec::new(),
            current: None,
            playback: Playback::Idle,
            next_entry_id: 1,
            generation: 0,
            start_position: 0.0,
            ready_timeout_ms,
            timer: None,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn member_conns(&self) -> impl Iterator<Item = ConnId> + '_ {
        self.members.iter().map(|m| m.conn)
    }

    pub fn has_member(&self, conn: ConnId) -> bool {
        self.members.iter().any(|m| m.conn == conn)
    }

    pub fn state(&self) -> GroupState {
        let current_id = self.current_youtube_id();
        GroupState {
            code: self.code.clone(),
            members: self
                .members
                .iter()
                .map(|m| Member {
                    username: m.username.clone(),
                    ready: current_id.is_some_and(|id| m.ready_ids.contains(id)),
                })
                .collect(),
            queue: self.queue.clone(),
            current: self.current,
            playback: self.playback.clone(),
        }
    }

    /// Returns the pending timer request, if an entry became current since the last call.
    pub fn take_timer(&mut self) -> Option<TimerRequest> {
        self.timer.take()
    }

    pub fn add_member(&mut self, conn: ConnId, username: String) {
        self.members.push(GroupMember {
            conn,
            username,
            ready_ids: HashSet::new(),
        });
    }

    /// Removes a member; the remaining ones may now all be ready.
    pub fn remove_member(&mut self, conn: ConnId, now: u64) {
        self.members.retain(|m| m.conn != conn);
        self.start_if_all_ready(now);
    }

    /// The deadline of the `Preparing` phase of `generation` passed: start anyway.
    pub fn on_deadline(&mut self, generation: u64, now: u64) -> bool {
        if generation == self.generation && matches!(self.playback, Playback::Preparing { .. }) {
            self.start(now);
            true
        } else {
            false
        }
    }

    /// Applies an in-group message from `conn`. `Ok(true)` means the state changed
    /// and must be broadcast; `Err` is a French message for the sender only.
    /// Create/Join/Leave/Ping are handled by the caller and ignored here.
    pub fn apply(&mut self, conn: ConnId, msg: ClientMessage, now: u64) -> Result<bool, String> {
        match msg {
            ClientMessage::Add {
                youtube_id,
                title,
                artist,
                duration,
            } => self.add(conn, youtube_id, title, artist, duration, now),
            ClientMessage::Remove { entry_id } => self.remove(entry_id, now),
            ClientMessage::PlayEntry { entry_id } => {
                let index = self.index_of(entry_id).ok_or(ERR_NOT_FOUND)?;
                self.make_current(index, now);
                Ok(true)
            }
            ClientMessage::Play => Ok(self.play(now)),
            ClientMessage::Pause => Ok(self.pause(now)),
            ClientMessage::Seek { position } => self.seek(position, now),
            ClientMessage::Skip => Ok(self.advance(now)),
            ClientMessage::Ended { entry_id } => {
                let is_current = self.current.map(|i| self.queue[i].entry_id) == Some(entry_id);
                Ok(is_current && self.advance(now))
            }
            ClientMessage::Ready { youtube_id } => self.ready(conn, youtube_id, now),
            ClientMessage::Create
            | ClientMessage::Join { .. }
            | ClientMessage::Leave
            | ClientMessage::Ping { .. } => Ok(false),
        }
    }

    fn add(
        &mut self,
        conn: ConnId,
        youtube_id: String,
        title: String,
        artist: Option<String>,
        duration: Option<f64>,
        now: u64,
    ) -> Result<bool, String> {
        if !is_valid_youtube_id(&youtube_id) {
            return Err("Identifiant YouTube invalide".into());
        }
        let title = title.trim().to_owned();
        if title.is_empty() || title.chars().count() > MAX_TEXT {
            return Err("Titre invalide".into());
        }
        let artist = artist
            .map(|a| a.trim().to_owned())
            .filter(|a| !a.is_empty());
        if artist
            .as_ref()
            .is_some_and(|a| a.chars().count() > MAX_TEXT)
        {
            return Err("Artiste invalide".into());
        }
        if duration.is_some_and(|d| !d.is_finite() || d < 0.0) {
            return Err("Durée invalide".into());
        }
        if self.queue.len() >= MAX_QUEUE {
            return Err(format!(
                "La file d'attente est pleine ({MAX_QUEUE} morceaux maximum)"
            ));
        }
        let added_by = self
            .members
            .iter()
            .find(|m| m.conn == conn)
            .map(|m| m.username.clone())
            .unwrap_or_default();
        let entry_id = self.next_entry_id;
        self.next_entry_id += 1;
        self.queue.push(QueueEntry {
            entry_id,
            youtube_id,
            title,
            artist,
            duration,
            added_by,
        });
        if self.current.is_none() {
            self.make_current(self.queue.len() - 1, now);
        }
        Ok(true)
    }

    fn remove(&mut self, entry_id: u64, now: u64) -> Result<bool, String> {
        let index = self.index_of(entry_id).ok_or(ERR_NOT_FOUND)?;
        self.queue.remove(index);
        match self.current {
            Some(current) if index < current => self.current = Some(current - 1),
            // The entry that followed the removed one now sits at `index`.
            Some(current) if index == current => {
                if index < self.queue.len() {
                    self.make_current(index, now);
                } else {
                    self.go_idle();
                }
            }
            _ => {}
        }
        Ok(true)
    }

    fn play(&mut self, now: u64) -> bool {
        match self.playback {
            Playback::Paused { position } => {
                self.playback = Playback::Playing {
                    position,
                    at_ms: now + RESUME_LEAD_MS,
                };
                true
            }
            Playback::Idle if !self.queue.is_empty() => {
                self.make_current(0, now);
                true
            }
            _ => false,
        }
    }

    fn pause(&mut self, now: u64) -> bool {
        match self.playback {
            Playback::Playing { position, at_ms } => {
                let elapsed = now.saturating_sub(at_ms) as f64 / 1000.0;
                self.playback = Playback::Paused {
                    position: position + elapsed,
                };
                true
            }
            _ => false,
        }
    }

    fn seek(&mut self, position: f64, now: u64) -> Result<bool, String> {
        if !position.is_finite() || position < 0.0 {
            return Err("Position invalide".into());
        }
        Ok(match self.playback {
            Playback::Playing { .. } => {
                self.playback = Playback::Playing {
                    position,
                    at_ms: now + RESUME_LEAD_MS,
                };
                true
            }
            Playback::Paused { .. } => {
                self.playback = Playback::Paused { position };
                true
            }
            // Not visible in the state until playback starts, so nothing to broadcast.
            Playback::Preparing { .. } => {
                self.start_position = position;
                false
            }
            Playback::Idle => false,
        })
    }

    /// Next entry (`Preparing`), or `Idle` past the end of the queue.
    fn advance(&mut self, now: u64) -> bool {
        let Some(current) = self.current else {
            return false;
        };
        if current + 1 < self.queue.len() {
            self.make_current(current + 1, now);
        } else {
            self.go_idle();
        }
        true
    }

    fn ready(&mut self, conn: ConnId, youtube_id: String, now: u64) -> Result<bool, String> {
        if !is_valid_youtube_id(&youtube_id) {
            return Err("Identifiant YouTube invalide".into());
        }
        let is_current = self.current_youtube_id() == Some(youtube_id.as_str());
        let queue = &self.queue;
        let Some(member) = self.members.iter_mut().find(|m| m.conn == conn) else {
            return Ok(false);
        };
        if member.ready_ids.len() >= MAX_READY_IDS {
            member
                .ready_ids
                .retain(|id| queue.iter().any(|e| &e.youtube_id == id));
        }
        let newly_ready = member.ready_ids.insert(youtube_id);
        if !(newly_ready && is_current) {
            return Ok(false);
        }
        self.start_if_all_ready(now);
        Ok(true)
    }

    fn make_current(&mut self, index: usize, now: u64) {
        self.current = Some(index);
        self.generation += 1;
        self.start_position = 0.0;
        self.playback = Playback::Preparing {
            deadline_ms: now + self.ready_timeout_ms,
        };
        if !self.start_if_all_ready(now) {
            self.timer = Some(TimerRequest {
                generation: self.generation,
                delay_ms: self.ready_timeout_ms,
            });
        }
    }

    fn go_idle(&mut self) {
        self.current = None;
        self.generation += 1;
        self.playback = Playback::Idle;
    }

    /// Starts playback if preparing and every member has the current track.
    fn start_if_all_ready(&mut self, now: u64) -> bool {
        let Some(id) = self.current_youtube_id() else {
            return false;
        };
        let all_ready =
            !self.members.is_empty() && self.members.iter().all(|m| m.ready_ids.contains(id));
        if all_ready && matches!(self.playback, Playback::Preparing { .. }) {
            self.start(now);
            true
        } else {
            false
        }
    }

    fn start(&mut self, now: u64) {
        self.playback = Playback::Playing {
            position: self.start_position,
            at_ms: now + START_LEAD_MS,
        };
    }

    fn current_youtube_id(&self) -> Option<&str> {
        self.current.map(|i| self.queue[i].youtube_id.as_str())
    }

    fn index_of(&self, entry_id: u64) -> Option<usize> {
        self.queue.iter().position(|e| e.entry_id == entry_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: u64 = 30_000;
    const A: ConnId = 1;
    const B: ConnId = 2;

    fn yt(n: u8) -> String {
        format!("video{n:06}")
    }

    fn add(n: u8) -> ClientMessage {
        ClientMessage::Add {
            youtube_id: yt(n),
            title: format!("Track {n}"),
            artist: None,
            duration: Some(180.0),
        }
    }

    fn ready(n: u8) -> ClientMessage {
        ClientMessage::Ready { youtube_id: yt(n) }
    }

    fn two_members() -> Group {
        let mut g = Group::new("ABC123".into(), TIMEOUT);
        g.add_member(A, "alice".into());
        g.add_member(B, "bob".into());
        g
    }

    fn entry_id(g: &Group, index: usize) -> u64 {
        g.state().queue[index].entry_id
    }

    #[test]
    fn add_on_idle_prepares_and_requests_timer() {
        let mut g = two_members();
        assert_eq!(g.apply(A, add(1), 1000), Ok(true));
        let s = g.state();
        assert_eq!(s.current, Some(0));
        assert_eq!(s.queue[0].added_by, "alice");
        assert_eq!(
            s.playback,
            Playback::Preparing {
                deadline_ms: 1000 + TIMEOUT
            }
        );
        assert_eq!(
            g.take_timer(),
            Some(TimerRequest {
                generation: 1,
                delay_ms: TIMEOUT
            })
        );
        assert_eq!(g.take_timer(), None);
        // A second add only queues.
        g.apply(A, add(2), 1100).unwrap();
        assert_eq!(g.state().current, Some(0));
    }

    #[test]
    fn rejects_invalid_add() {
        let mut g = two_members();
        let bad = |youtube_id: &str, title: &str, duration| ClientMessage::Add {
            youtube_id: youtube_id.into(),
            title: title.into(),
            artist: None,
            duration,
        };
        assert!(g.apply(A, bad("nope", "t", None), 0).is_err());
        assert!(g.apply(A, bad(&yt(1), "  ", None), 0).is_err());
        assert!(g.apply(A, bad(&yt(1), &"x".repeat(201), None), 0).is_err());
        assert!(g.apply(A, bad(&yt(1), "t", Some(f64::NAN)), 0).is_err());
        assert!(g.apply(A, bad(&yt(1), "t", Some(-1.0)), 0).is_err());
        assert!(g.state().queue.is_empty());
    }

    #[test]
    fn queue_is_bounded() {
        let mut g = two_members();
        for _ in 0..MAX_QUEUE {
            g.apply(A, add(1), 0).unwrap();
        }
        assert!(g.apply(A, add(1), 0).is_err());
    }

    #[test]
    fn starts_when_all_ready() {
        let mut g = two_members();
        g.apply(A, add(1), 0).unwrap();
        assert_eq!(g.apply(A, ready(1), 10), Ok(true));
        assert!(matches!(g.state().playback, Playback::Preparing { .. }));
        assert!(g.state().members[0].ready);
        assert!(!g.state().members[1].ready);
        // Duplicate ready changes nothing.
        assert_eq!(g.apply(A, ready(1), 15), Ok(false));
        assert_eq!(g.apply(B, ready(1), 20), Ok(true));
        assert_eq!(
            g.state().playback,
            Playback::Playing {
                position: 0.0,
                at_ms: 20 + START_LEAD_MS
            }
        );
    }

    #[test]
    fn pre_downloaded_tracks_start_immediately() {
        let mut g = two_members();
        for m in [A, B] {
            assert_eq!(g.apply(m, ready(1), 0), Ok(false));
        }
        g.apply(A, add(1), 100).unwrap();
        assert_eq!(
            g.state().playback,
            Playback::Playing {
                position: 0.0,
                at_ms: 100 + START_LEAD_MS
            }
        );
        assert_eq!(g.take_timer(), None);
    }

    #[test]
    fn deadline_starts_and_stale_deadlines_are_ignored() {
        let mut g = two_members();
        g.apply(A, add(1), 0).unwrap();
        g.apply(A, add(2), 0).unwrap();
        let first = g.take_timer().unwrap();
        g.apply(A, ClientMessage::Skip, 10).unwrap();
        let second = g.take_timer().unwrap();
        assert!(!g.on_deadline(first.generation, TIMEOUT));
        assert!(matches!(g.state().playback, Playback::Preparing { .. }));
        assert!(g.on_deadline(second.generation, TIMEOUT + 10));
        assert!(matches!(g.state().playback, Playback::Playing { .. }));
        // Already playing: a late duplicate does nothing.
        assert!(!g.on_deadline(second.generation, TIMEOUT + 20));
    }

    #[test]
    fn leaving_member_can_unblock_start() {
        let mut g = two_members();
        g.apply(A, add(1), 0).unwrap();
        g.apply(A, ready(1), 0).unwrap();
        g.remove_member(B, 50);
        assert_eq!(
            g.state().playback,
            Playback::Playing {
                position: 0.0,
                at_ms: 50 + START_LEAD_MS
            }
        );
    }

    #[test]
    fn pause_play_seek() {
        let mut g = two_members();
        g.apply(A, add(1), 0).unwrap();
        g.on_deadline(1, 0);
        // Playing { 0.0, at 1500 }; pausing before at_ms keeps position 0.
        g.apply(A, ClientMessage::Pause, 1000).unwrap();
        assert_eq!(g.state().playback, Playback::Paused { position: 0.0 });
        g.apply(A, ClientMessage::Play, 2000).unwrap();
        assert_eq!(
            g.state().playback,
            Playback::Playing {
                position: 0.0,
                at_ms: 2000 + RESUME_LEAD_MS
            }
        );
        g.apply(A, ClientMessage::Pause, 5000).unwrap();
        assert_eq!(g.state().playback, Playback::Paused { position: 2.5 });
        assert_eq!(g.apply(A, ClientMessage::Pause, 5000), Ok(false));
        g.apply(A, ClientMessage::Seek { position: 42.0 }, 6000)
            .unwrap();
        assert_eq!(g.state().playback, Playback::Paused { position: 42.0 });
        g.apply(A, ClientMessage::Play, 7000).unwrap();
        g.apply(A, ClientMessage::Seek { position: 10.0 }, 8000)
            .unwrap();
        assert_eq!(
            g.state().playback,
            Playback::Playing {
                position: 10.0,
                at_ms: 8000 + RESUME_LEAD_MS
            }
        );
        assert!(
            g.apply(A, ClientMessage::Seek { position: -1.0 }, 0)
                .is_err()
        );
        assert!(
            g.apply(
                A,
                ClientMessage::Seek {
                    position: f64::INFINITY
                },
                0
            )
            .is_err()
        );
    }

    #[test]
    fn seek_while_preparing_sets_start_position() {
        let mut g = two_members();
        g.apply(A, add(1), 0).unwrap();
        assert_eq!(
            g.apply(A, ClientMessage::Seek { position: 30.0 }, 5),
            Ok(false)
        );
        g.on_deadline(1, 100);
        assert_eq!(
            g.state().playback,
            Playback::Playing {
                position: 30.0,
                at_ms: 100 + START_LEAD_MS
            }
        );
    }

    #[test]
    fn play_on_idle_restarts_queue() {
        let mut g = two_members();
        assert_eq!(g.apply(A, ClientMessage::Play, 0), Ok(false));
        g.apply(A, add(1), 0).unwrap();
        g.apply(A, ClientMessage::Skip, 0).unwrap();
        assert_eq!(g.state().playback, Playback::Idle);
        assert_eq!(g.state().current, None);
        assert_eq!(g.apply(A, ClientMessage::Skip, 0), Ok(false));
        g.apply(A, ClientMessage::Play, 10).unwrap();
        assert_eq!(g.state().current, Some(0));
        assert!(matches!(g.state().playback, Playback::Preparing { .. }));
    }

    #[test]
    fn ended_only_counts_for_current_entry() {
        let mut g = two_members();
        g.apply(A, add(1), 0).unwrap();
        g.apply(A, add(2), 0).unwrap();
        let first = entry_id(&g, 0);
        assert_eq!(
            g.apply(A, ClientMessage::Ended { entry_id: first }, 0),
            Ok(true)
        );
        assert_eq!(g.state().current, Some(1));
        assert_eq!(
            g.apply(B, ClientMessage::Ended { entry_id: first }, 0),
            Ok(false)
        );
        assert_eq!(g.state().current, Some(1));
        let second = entry_id(&g, 1);
        g.apply(B, ClientMessage::Ended { entry_id: second }, 0)
            .unwrap();
        assert_eq!(g.state().current, None);
        assert_eq!(g.state().playback, Playback::Idle);
    }

    #[test]
    fn remove_adjusts_current() {
        let mut g = two_members();
        for n in 1..=4 {
            g.apply(A, add(n), 0).unwrap();
        }
        let ids: Vec<u64> = (0..4).map(|i| entry_id(&g, i)).collect();
        g.apply(A, ClientMessage::PlayEntry { entry_id: ids[2] }, 0)
            .unwrap();
        assert_eq!(g.state().current, Some(2));
        // Before current.
        g.apply(A, ClientMessage::Remove { entry_id: ids[0] }, 0)
            .unwrap();
        assert_eq!(g.state().current, Some(1));
        assert_eq!(g.state().queue[1].entry_id, ids[2]);
        // After current.
        g.apply(A, ClientMessage::Remove { entry_id: ids[3] }, 0)
            .unwrap();
        assert_eq!(g.state().current, Some(1));
        // The current (last) one → idle.
        g.apply(A, ClientMessage::Remove { entry_id: ids[2] }, 0)
            .unwrap();
        assert_eq!(g.state().current, None);
        assert_eq!(g.state().playback, Playback::Idle);
        assert_eq!(
            g.apply(A, ClientMessage::Remove { entry_id: 999 }, 0),
            Err(ERR_NOT_FOUND.into())
        );
        assert_eq!(
            g.apply(A, ClientMessage::PlayEntry { entry_id: 999 }, 0),
            Err(ERR_NOT_FOUND.into())
        );
    }

    #[test]
    fn removing_current_moves_to_next() {
        let mut g = two_members();
        g.apply(A, add(1), 0).unwrap();
        g.apply(A, add(2), 0).unwrap();
        let first = entry_id(&g, 0);
        let second = entry_id(&g, 1);
        g.apply(A, ClientMessage::Remove { entry_id: first }, 0)
            .unwrap();
        let s = g.state();
        assert_eq!(s.current, Some(0));
        assert_eq!(s.queue[0].entry_id, second);
        assert!(matches!(s.playback, Playback::Preparing { .. }));
    }
}
