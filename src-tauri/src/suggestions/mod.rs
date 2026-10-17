//! Playing recommended tracks (mixes, discoveries) that are not in the library.
//! They become hidden "suggested" library rows (see [`crate::library`]), so the
//! regular queue, fetcher and history work unchanged; the file is downloaded on
//! demand into the suggestion cache ([`crate::downloads::Downloads`]).

pub mod commands;

use serde::Deserialize;

use crate::audio::QueueItem;
use crate::error::AppResult;
use crate::history::PlaySource;
use crate::library::{Library, RemoteTrack};

/// A recommended track as sent by the frontend (a `MixTrack` fits).
#[derive(Clone, Debug, PartialEq, Deserialize, specta::Type)]
pub struct SuggestedTrack {
    pub youtube_id: String,
    pub title: String,
    pub artist: Option<String>,
    /// Seconds, 0 when unknown.
    pub duration: f64,
}

impl From<SuggestedTrack> for RemoteTrack {
    fn from(t: SuggestedTrack) -> Self {
        Self {
            title: t.title,
            artist: t.artist,
            album: None,
            duration: t.duration,
            youtube_id: Some(t.youtube_id),
        }
    }
}

/// Registers `tracks` as suggested library rows (YouTube ids are validated there)
/// and resolves them to queue items from [`PlaySource::Mix`], in order.
pub fn queue_items(library: &Library, tracks: Vec<SuggestedTrack>) -> AppResult<Vec<QueueItem>> {
    let remote: Vec<RemoteTrack> = tracks.into_iter().map(Into::into).collect();
    let ids = library.add_suggested(&remote)?;
    library.queue_items(&ids, PlaySource::Mix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn track(id: &str) -> SuggestedTrack {
        SuggestedTrack {
            youtube_id: id.into(),
            title: format!("Title {id}"),
            artist: Some("Artist".into()),
            duration: 120.0,
        }
    }

    #[test]
    fn builds_mix_queue_items_and_hides_the_rows() {
        let lib = Library::new(Db::in_memory(), None);
        let items = queue_items(&lib, vec![track("aaaaaaaaaaa"), track("bbbbbbbbbbb")]).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].youtube_id.as_deref(), Some("aaaaaaaaaaa"));
        assert!(items.iter().all(|i| i.path.is_none()));
        assert!(items.iter().all(|i| i.source == PlaySource::Mix));
        assert!(lib.tracks().unwrap().is_empty());
        assert!(lib.is_suggested(items[0].track_id).unwrap());
    }

    #[test]
    fn rejects_invalid_youtube_ids() {
        let lib = Library::new(Db::in_memory(), None);
        assert!(queue_items(&lib, vec![track("aaaaaaaaaaa"), track("../etc")]).is_err());
        assert!(queue_items(&lib, vec![track("")]).is_err());
    }
}
