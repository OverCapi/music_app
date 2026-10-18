//! Imports a Spotify liked-songs list (TSV: `n  titre  artiste  durée`) into the
//! library. Songs already in the library are skipped; the others are searched on
//! YouTube (first result with a close duration wins) and added without download.
//! Songs not found are written to `<file stem>_non_trouves.tsv` next to the input.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use group_protocol::is_valid_youtube_id;
use serde::Serialize;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::{FlatEntry, LibrarySync};
use crate::error::{AppError, AppResult};
use crate::library::{RemoteTrack, match_key};
use crate::ytdlp;

const MAX_PARALLEL_SEARCHES: usize = 4;
const CANDIDATES: u32 = 5;
const HEADER: &str = "n\ttitre\tartiste\tdurée";

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct SpotifyImportReport {
    pub total: u32,
    /// Already in the library (not searched).
    pub already_owned: u32,
    /// Found on YouTube and added to the library.
    pub added: u32,
    pub not_found: u32,
    /// File listing the songs not found (empty when none).
    pub not_found_path: Option<String>,
}

struct Row {
    n: String,
    title: String,
    artist: String,
    duration: f64,
}

impl LibrarySync {
    pub async fn import_spotify_tsv(&self, path: &str) -> AppResult<SpotifyImportReport> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| AppError::invalid(format!("Lecture du fichier impossible : {e}")))?;
        let rows = parse_tsv(&text);
        if rows.is_empty() {
            return Err(AppError::invalid("Aucun titre dans le fichier"));
        }
        if self.running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Err(AppError::invalid("Une synchronisation est déjà en cours"));
        }
        let result = self.run_spotify(path, rows).await;
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        result
    }

    async fn run_spotify(&self, path: &str, rows: Vec<Row>) -> AppResult<SpotifyImportReport> {
        let mut report = SpotifyImportReport {
            total: rows.len() as u32,
            ..Default::default()
        };
        let mut todo = Vec::new();
        for row in rows {
            if self
                .library
                .find_similar(Some(&row.artist), &row.title, row.duration)?
                .is_empty()
            {
                todo.push(row);
            } else {
                report.already_owned += 1;
            }
        }

        let permits = Arc::new(Semaphore::new(MAX_PARALLEL_SEARCHES));
        let mut tasks = JoinSet::new();
        for (i, row) in todo.iter().enumerate() {
            let permits = Arc::clone(&permits);
            let query = format!("{} {}", row.artist, row.title);
            let duration = row.duration;
            tasks.spawn(async move {
                let _permit = permits.acquire().await;
                (i, find_on_youtube(&query, duration).await)
            });
        }
        let total = todo.len() as u32;
        let mut ids: Vec<Option<String>> = vec![None; todo.len()];
        let mut done = 0;
        while let Some(res) = tasks.join_next().await {
            if let Ok((i, id)) = res {
                ids[i] = id;
            }
            done += 1;
            self.progress("Spotify : recherche sur YouTube", done, total);
        }

        let mut found = Vec::new();
        let mut missing = Vec::new();
        for (row, id) in todo.into_iter().zip(ids) {
            match id {
                Some(youtube_id) => found.push(RemoteTrack {
                    title: row.title,
                    artist: Some(row.artist).filter(|a| !a.is_empty()),
                    album: None,
                    duration: row.duration,
                    youtube_id: Some(youtube_id),
                }),
                None => missing.push(row),
            }
        }
        self.library.add_remote(&found)?;
        report.added = found.len() as u32;
        report.not_found = missing.len() as u32;
        if !missing.is_empty() {
            let out = not_found_path(path);
            std::fs::write(&out, tsv_of(&missing))
                .map_err(|e| AppError::invalid(format!("Écriture impossible : {e}")))?;
            report.not_found_path = Some(out.to_string_lossy().into_owned());
        }
        Ok(report)
    }
}

/// First YouTube result whose duration is close to `duration` (any when unknown).
async fn find_on_youtube(query: &str, duration: f64) -> Option<String> {
    let target = format!("ytsearch{CANDIDATES}:{query}");
    let entries: Vec<FlatEntry> = ytdlp::run_json(&[&target, "--flat-playlist", "--dump-json"])
        .await
        .ok()?;
    entries
        .into_iter()
        .filter(|e| is_valid_youtube_id(&e.id))
        .find(|e| match_key::same_duration(e.duration.unwrap_or(0.0), duration))
        .map(|e| e.id)
}

fn parse_tsv(text: &str) -> Vec<Row> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut f = line.split('\t');
            let (n, title, artist, dur) = (f.next()?, f.next()?, f.next()?, f.next()?);
            let title = title.trim();
            (!title.is_empty()).then(|| Row {
                n: n.trim().to_owned(),
                title: title.to_owned(),
                artist: artist.trim().to_owned(),
                duration: parse_duration(dur),
            })
        })
        .collect()
}

/// "3:06" or "1:02:03" → seconds (0 when unreadable).
fn parse_duration(s: &str) -> f64 {
    s.trim()
        .split(':')
        .try_fold(0.0, |acc, p| p.parse::<f64>().ok().map(|v| acc * 60.0 + v))
        .unwrap_or(0.0)
}

fn tsv_of(rows: &[Row]) -> String {
    let mut out = format!("{HEADER}\n");
    for r in rows {
        let d = r.duration as u64;
        out.push_str(&format!(
            "{}\t{}\t{}\t{}:{:02}\n",
            r.n,
            r.title,
            r.artist,
            d / 60,
            d % 60
        ));
    }
    out
}

fn not_found_path(input: &str) -> PathBuf {
    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("titres");
    p.with_file_name(format!("{stem}_non_trouves.tsv"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rows_and_durations() {
        let rows = parse_tsv("n\ttitre\tartiste\tdurée\n1\tOn My Own\tZach Diamond\t2:13\nbad\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].duration, 133.0);
        assert_eq!(parse_duration("1:02:03"), 3723.0);
        assert_eq!(parse_duration("x"), 0.0);
        assert!(tsv_of(&rows).contains("1\tOn My Own\tZach Diamond\t2:13"));
    }
}
