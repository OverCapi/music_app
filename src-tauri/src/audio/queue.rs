//! The play queue as a pure data structure (no I/O), unit-tested.

use super::{QueueItem, QueueState};
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

    /// Index after the current one, if the queue has it.
    pub fn next_index(&self) -> Option<usize> {
        let next = self.current.map_or(0, |c| c + 1);
        (next < self.items.len()).then_some(next)
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

    fn item(id: TrackId) -> QueueItem {
        QueueItem {
            track_id: id,
            path: format!("/music/{id}.mp3"),
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
        let mut q = Queue::default();
        assert_eq!(q.next_index(), None);
        q.append([item(1)]);
        assert_eq!(q.next_index(), Some(0));
        q.set_current(0);
        assert_eq!(q.next_index(), None);
        q.append([item(2)]);
        assert_eq!(q.next_index(), Some(1));
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
}
