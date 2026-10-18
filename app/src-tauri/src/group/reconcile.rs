//! Pure reconciliation: server `GroupState` + local files → playback target,
//! tracks to prefetch (current + next 2), `Ready` messages to send. Unit-tested.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use group_protocol::{GroupState, Playback, QueueEntry};

/// Current entry + the next two.
const PREFETCH: usize = 3;

/// What the audio engine should be doing for a given group state. The engine is
/// only re-commanded when this changes.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// Nothing to play, still preparing, or our file isn't there yet.
    Stopped,
    Play {
        entry_id: u64,
        path: PathBuf,
        position: f64,
        at_ms: u64,
    },
    Pause {
        entry_id: u64,
        path: PathBuf,
        position: f64,
    },
}

#[derive(Debug, PartialEq)]
pub struct Actions {
    /// Downloaded YouTube ids not announced yet on this connection.
    pub ready: Vec<String>,
    /// New engine target, `None` when the applied one is still right.
    pub target: Option<Target>,
}

/// YouTube ids whose files should be on disk: the current entry and the next two.
pub fn prefetch(state: &GroupState) -> Vec<String> {
    let skip = state.current.unwrap_or(state.queue.len());
    state
        .queue
        .iter()
        .skip(skip)
        .take(PREFETCH)
        .map(|e| e.youtube_id.clone())
        .collect()
}

/// `files`: downloaded files by YouTube id; `announced`: ids already sent as
/// `Ready` on this connection; `applied`: the last target given to the engine.
pub fn plan(
    state: &GroupState,
    files: &HashMap<String, PathBuf>,
    announced: &HashSet<String>,
    applied: Option<&Target>,
) -> Actions {
    let mut ready: Vec<String> = files
        .keys()
        .filter(|id| !announced.contains(*id))
        .cloned()
        .collect();
    ready.sort();
    let target = target(state, files);
    Actions {
        ready,
        target: (applied != Some(&target)).then_some(target),
    }
}

pub fn current_entry(state: &GroupState) -> Option<&QueueEntry> {
    state.queue.get(state.current?)
}

fn target(state: &GroupState, files: &HashMap<String, PathBuf>) -> Target {
    let Some(entry) = current_entry(state) else {
        return Target::Stopped;
    };
    let Some(path) = files.get(&entry.youtube_id) else {
        return Target::Stopped;
    };
    match state.playback {
        Playback::Playing { position, at_ms } => Target::Play {
            entry_id: entry.entry_id,
            path: path.clone(),
            position,
            at_ms,
        },
        Playback::Paused { position } => Target::Pause {
            entry_id: entry.entry_id,
            path: path.clone(),
            position,
        },
        Playback::Idle | Playback::Preparing { .. } => Target::Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use group_protocol::Member;

    fn entry(entry_id: u64, youtube_id: &str) -> QueueEntry {
        QueueEntry {
            entry_id,
            youtube_id: youtube_id.to_owned(),
            title: format!("Titre {entry_id}"),
            artist: None,
            duration: Some(200.0),
            added_by: "alice".to_owned(),
        }
    }

    fn state(current: Option<usize>, playback: Playback) -> GroupState {
        GroupState {
            code: "ABC123".to_owned(),
            members: vec![Member {
                username: "alice".to_owned(),
                ready: false,
            }],
            queue: vec![
                entry(1, "aaaaaaaaaaa"),
                entry(2, "bbbbbbbbbbb"),
                entry(3, "ccccccccccc"),
                entry(4, "ddddddddddd"),
            ],
            current,
            playback,
        }
    }

    fn files(ids: &[&str]) -> HashMap<String, PathBuf> {
        ids.iter()
            .map(|id| (id.to_string(), PathBuf::from(format!("/tmp/{id}.opus"))))
            .collect()
    }

    fn plan_of(state: &GroupState, files: &HashMap<String, PathBuf>) -> Actions {
        plan(state, files, &HashSet::new(), None)
    }

    #[test]
    fn prefetches_current_and_next_two() {
        let ids = |s: &GroupState| prefetch(s);
        assert_eq!(
            ids(&state(Some(0), Playback::Idle)),
            ["aaaaaaaaaaa", "bbbbbbbbbbb", "ccccccccccc"]
        );
        assert_eq!(
            ids(&state(Some(2), Playback::Idle)),
            ["ccccccccccc", "ddddddddddd"]
        );
        assert!(ids(&state(None, Playback::Idle)).is_empty());
    }

    #[test]
    fn ready_only_for_unannounced_files() {
        let s = state(Some(0), Playback::Idle);
        let files = files(&["bbbbbbbbbbb", "aaaaaaaaaaa"]);
        assert_eq!(
            plan(&s, &files, &HashSet::new(), None).ready,
            ["aaaaaaaaaaa", "bbbbbbbbbbb"]
        );
        let announced = HashSet::from(["aaaaaaaaaaa".to_owned()]);
        assert_eq!(plan(&s, &files, &announced, None).ready, ["bbbbbbbbbbb"]);
    }

    #[test]
    fn target_follows_playback() {
        let playing = Playback::Playing {
            position: 12.0,
            at_ms: 1000,
        };
        // Not downloaded yet: wait.
        let none = HashMap::new();
        assert_eq!(
            plan_of(&state(Some(1), playing.clone()), &none).target,
            Some(Target::Stopped)
        );
        let files = files(&["bbbbbbbbbbb"]);
        let path = PathBuf::from("/tmp/bbbbbbbbbbb.opus");
        assert_eq!(
            plan_of(&state(Some(1), playing), &files).target,
            Some(Target::Play {
                entry_id: 2,
                path: path.clone(),
                position: 12.0,
                at_ms: 1000,
            })
        );
        assert_eq!(
            plan_of(&state(Some(1), Playback::Paused { position: 3.0 }), &files).target,
            Some(Target::Pause {
                entry_id: 2,
                path,
                position: 3.0,
            })
        );
        let preparing = Playback::Preparing { deadline_ms: 5 };
        assert_eq!(
            plan_of(&state(Some(1), preparing), &files).target,
            Some(Target::Stopped)
        );
        assert_eq!(
            plan_of(&state(None, Playback::Idle), &files).target,
            Some(Target::Stopped)
        );
    }

    #[test]
    fn engine_recommanded_only_on_change() {
        let files = files(&["aaaaaaaaaaa"]);
        let playing = Playback::Playing {
            position: 0.0,
            at_ms: 42,
        };
        let before = state(Some(0), playing);
        let applied = plan_of(&before, &files).target.unwrap();

        // A member becoming ready, the queue changing: nothing to re-command.
        let mut after = before.clone();
        after.members[0].ready = true;
        after.queue.push(entry(5, "eeeeeeeeeee"));
        after.queue.remove(2);
        let none = HashSet::new();
        assert_eq!(plan(&after, &files, &none, Some(&applied)).target, None);

        // A seek (new position/time) does.
        let mut seeked = before.clone();
        seeked.playback = Playback::Playing {
            position: 30.0,
            at_ms: 99,
        };
        assert!(
            plan(&seeked, &files, &none, Some(&applied))
                .target
                .is_some()
        );
    }
}
