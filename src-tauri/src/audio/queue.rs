//! The play queue as a pure data structure (no I/O), unit-tested.

use super::{QueueItem, QueueState, RepeatMode};
use crate::error::{AppError, AppResult};
use crate::library::TrackId;

const GONE: &str = "Ce titre n'est plus dans la file";

#[derive(Debug, Default)]
pub struct Queue {
    items: Vec<QueueItem>,
    current: Option<usize>,
}

impl Queue {
    pub fn state(&self) -> QueueState {
        QueueState {
            items: self.items.clone(),
            current: self.current.map(|c| c as u32),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&QueueItem> {
        self.items.get(index)
    }

    pub fn current(&self) -> Option<usize> {
        self.current
    }

    pub fn set_current(&mut self, index: usize) {
        self.current = Some(index);
    }

    /// Replaces the items; `start` becomes the current one.
    pub fn replace(&mut self, items: Vec<QueueItem>, start: usize) -> AppResult<()> {
        if start >= items.len() {
            return Err(AppError::invalid("Index de départ invalide"));
        }
        self.items = items;
        self.current = Some(start);
        Ok(())
    }

    pub fn append(&mut self, items: impl IntoIterator<Item = QueueItem>) {
        self.items.extend(items);
    }

    /// Errors if `index` is out of range.
    pub fn check_index(&self, index: usize) -> AppResult<()> {
        if index < self.items.len() {
            Ok(())
        } else {
            Err(AppError::not_found(GONE))
        }
    }

    /// Index to play after the current one, if the queue has it.
    /// `auto`: the track ended by itself (repeat-one replays it; a manual "next" moves on).
    /// `rand` picks the track when shuffling. `None` = queue exhausted.
    pub fn next_index(
        &self,
        shuffle: bool,
        repeat: RepeatMode,
        auto: bool,
        rand: u64,
    ) -> Option<usize> {
        let len = self.items.len();
        if auto && repeat == RepeatMode::One && self.current.is_some() {
            return self.current;
        }
        if shuffle && len > 1 {
            // ponytail: plain random pick (a track can come back before all were played),
            // a shuffled order would need a play history.
            let mut pick = (rand % (len as u64 - 1)) as usize;
            if self.current.is_some_and(|c| pick >= c) {
                pick += 1;
            }
            return Some(pick);
        }
        let next = self.current.map_or(0, |c| c + 1);
        if next < len {
            Some(next)
        } else {
            (repeat != RepeatMode::Off && len > 0).then_some(0)
        }
    }

    /// The current item cannot be removed.
    pub fn remove(&mut self, index: usize) -> AppResult<()> {
        self.check_index(index)?;
        match self.current {
            Some(c) if c == index => {
                return Err(AppError::invalid("Impossible de retirer le titre en cours"));
            }
            Some(c) if c > index => self.current = Some(c - 1),
            _ => {}
        }
        self.items.remove(index);
        Ok(())
    }

    pub fn clear_except_current(&mut self) {
        match self.current {
            Some(c) => {
                let current = self.items.swap_remove(c);
                self.items = vec![current];
                self.current = Some(0);
            }
            None => self.items.clear(),
        }
    }

    /// Sets the file of every entry of `track_id` (just downloaded). Returns whether
    /// anything changed.
    pub fn set_path(&mut self, track_id: TrackId, path: &str) -> bool {
        let mut changed = false;
        for item in self.items.iter_mut().filter(|i| i.track_id == track_id) {
            if item.path.as_deref() != Some(path) {
                item.path = Some(path.to_owned());
                changed = true;
            }
        }
        changed
    }

    /// Drops every entry of `track_id` except the current one. Returns whether anything changed.
    pub fn forget_track(&mut self, track_id: TrackId) -> bool {
        let before = self.items.len();
        let mut index = 0;
        let mut current = self.current;
        self.items.retain(|item| {
            let i = index;
            index += 1;
            let keep = item.track_id != track_id || Some(i) == self.current;
            if !keep && self.current.is_some_and(|c| i < c) {
                current = current.map(|c| c - 1);
            }
            keep
        });
        self.current = current;
        self.items.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::PlaySource;

    fn item(id: TrackId) -> QueueItem {
        QueueItem {
            track_id: id,
            path: Some(format!("/music/{id}.mp3")),
            youtube_id: None,
            source: PlaySource::Library,
        }
    }

    fn ids(q: &Queue) -> Vec<TrackId> {
        q.items.iter().map(|i| i.track_id).collect()
    }

    fn queue(items: &[TrackId], current: usize) -> Queue {
        let mut q = Queue::default();
        q.replace(items.iter().map(|&id| item(id)).collect(), current)
            .unwrap();
        q
    }

    #[test]
    fn replace_checks_start() {
        let mut q = Queue::default();
        assert!(q.replace(vec![item(1)], 1).is_err());
        assert!(q.replace(vec![], 0).is_err());
        q.replace(vec![item(1), item(2)], 1).unwrap();
        assert_eq!(q.current(), Some(1));
    }

    #[test]
    fn next_index_stops_at_end() {
        use RepeatMode::Off;
        let mut q = Queue::default();
        assert_eq!(q.next_index(false, Off, true, 0), None);
        q.append([item(1)]);
        assert_eq!(q.next_index(false, Off, true, 0), Some(0));
        q.set_current(0);
        assert_eq!(q.next_index(false, Off, true, 0), None);
        q.append([item(2)]);
        assert_eq!(q.next_index(false, Off, true, 0), Some(1));
    }

    #[test]
    fn repeat_modes() {
        use RepeatMode::{All, One};
        let mut q = queue(&[1, 2], 1);
        assert_eq!(q.next_index(false, All, true, 0), Some(0));
        assert_eq!(q.next_index(false, One, true, 0), Some(1));
        // Manual "next" does not replay the track.
        assert_eq!(q.next_index(false, One, false, 0), Some(0));
        q.clear_except_current();
        assert_eq!(q.next_index(false, All, true, 0), Some(0));
    }

    #[test]
    fn shuffle_never_picks_current() {
        let q = queue(&[1, 2, 3], 1);
        for rand in 0..20 {
            let next = q.next_index(true, RepeatMode::Off, true, rand).unwrap();
            assert!(next != 1 && next < 3);
        }
        // Single track: nothing to shuffle.
        let q = queue(&[1], 0);
        assert_eq!(q.next_index(true, RepeatMode::Off, true, 5), None);
        assert_eq!(q.next_index(true, RepeatMode::All, true, 5), Some(0));
    }

    #[test]
    fn remove_shifts_current() {
        let mut q = queue(&[1, 2, 3], 2);
        assert!(q.remove(2).is_err());
        assert!(q.remove(5).is_err());
        q.remove(0).unwrap();
        assert_eq!(ids(&q), [2, 3]);
        assert_eq!(q.current(), Some(1));
        q.remove(0).unwrap();
        assert_eq!(q.current(), Some(0));
    }

    #[test]
    fn clear_keeps_current() {
        let mut q = queue(&[1, 2, 3], 1);
        q.clear_except_current();
        assert_eq!(ids(&q), [2]);
        assert_eq!(q.current(), Some(0));

        let mut q = Queue::default();
        q.append([item(1)]);
        q.clear_except_current();
        assert!(q.is_empty());
    }

    #[test]
    fn forget_track_keeps_current_entry() {
        let mut q = queue(&[7, 1, 7, 2, 7], 2);
        assert!(q.forget_track(7));
        assert_eq!(ids(&q), [1, 7, 2]);
        assert_eq!(q.current(), Some(1));
        assert_eq!(q.get(1).unwrap().track_id, 7);
        assert!(!q.forget_track(9));
    }

    #[test]
    fn state_mirrors_queue() {
        let q = queue(&[1, 2], 1);
        let state = q.state();
        assert_eq!(state.items.len(), 2);
        assert_eq!(state.current, Some(1));
    }

    #[test]
    fn set_path_fills_every_entry_of_the_track() {
        let mut q = queue(&[1, 2, 1], 0);
        for item in &mut q.items {
            item.path = None;
        }
        assert!(q.set_path(1, "/music/1.opus"));
        assert!(!q.set_path(1, "/music/1.opus"));
        let paths: Vec<_> = q.items.iter().map(|i| i.path.as_deref()).collect();
        assert_eq!(paths, [Some("/music/1.opus"), None, Some("/music/1.opus")]);
        assert!(!q.set_path(3, "/x"));
    }
}
