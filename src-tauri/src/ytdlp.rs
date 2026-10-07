//! The only place that runs the external `yt-dlp` binary (never through a shell).
//!
//! The binary is looked up in PATH, then `~/Applications/yt-dlp/yt-dlp`. If `node`
//! is in PATH it is passed to yt-dlp as JS runtime. Used by `search` and `downloads`.
//!
//! URLs are always built here from a validated YouTube id
//! ([`group_protocol::is_valid_youtube_id`]), never taken from user input.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Stdio;

use group_protocol::is_valid_youtube_id;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use crate::error::{AppError, AppResult};

/// `https://music.youtube.com/watch?v=<id>`, only for a valid id. The YouTube Music
/// URL works for any video and gives yt-dlp the artist/album metadata when it exists.
pub fn video_url(youtube_id: &str) -> AppResult<String> {
    if !is_valid_youtube_id(youtube_id) {
        return Err(AppError::invalid("Identifiant YouTube invalide"));
    }
    Ok(format!("https://music.youtube.com/watch?v={youtube_id}"))
}

/// Runs yt-dlp and parses every stdout line that is a JSON `T` (others are skipped).
pub async fn run_json<T: DeserializeOwned>(args: &[&str]) -> AppResult<Vec<T>> {
    let output = command(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(spawn_error)?;
    if !output.status.success() {
        return Err(failure(&String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect())
}

/// What a download reports while it runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Progress {
    /// Fraction 0.0–1.0, when the size is known.
    Downloading(Option<f64>),
    /// Download finished; extracting/tagging (ffmpeg) follows.
    Converting,
}

/// Runs a yt-dlp download (`args` = URL and format options) and returns the final
/// file path. Progress comes from a custom `--progress-template` on stdout.
pub async fn download(
    args: &[&OsStr],
    mut on_progress: impl FnMut(Progress) + Send,
) -> AppResult<PathBuf> {
    let mut cmd = command(args);
    cmd.args(["--progress", "--newline"]).args([
        "--progress-template",
        "download:PROGRESS %(progress.downloaded_bytes)s %(progress.total_bytes,progress.total_bytes_estimate)s %(progress.status)s",
        "--print",
        "after_move:FILE %(filepath)s",
    ]);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let stdout = child.stdout.take().expect("stdout is piped");

    let read_stdout = async {
        let mut lines = BufReader::new(stdout).lines();
        let mut file = None;
        let mut last_emitted = -1.0;
        while let Ok(Some(line)) = lines.next_line().await {
            match parse_line(&line) {
                Some(Line::File(path)) => file = Some(PathBuf::from(path)),
                Some(Line::Progress(Progress::Downloading(fraction))) => {
                    // Throttle: one report per percent at most.
                    if fraction.is_none_or(|f| f - last_emitted >= 0.01) {
                        last_emitted = fraction.unwrap_or(last_emitted);
                        on_progress(Progress::Downloading(fraction));
                    }
                }
                Some(Line::Progress(p)) => on_progress(p),
                None => {}
            }
        }
        file
    };
    let read_stderr = async {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text).await;
        text
    };
    let (file, stderr) = tokio::join!(read_stdout, read_stderr);

    let status = child.wait().await?;
    if !status.success() {
        return Err(failure(&stderr));
    }
    file.ok_or_else(|| AppError::Tool("yt-dlp n'a pas indiqué le fichier téléchargé".into()))
}

#[derive(Debug, PartialEq)]
enum Line<'a> {
    File(&'a str),
    Progress(Progress),
}

/// Parses a line printed by our `--progress-template` / `--print` options.
fn parse_line(line: &str) -> Option<Line<'_>> {
    if let Some(path) = line.strip_prefix("FILE ") {
        return Some(Line::File(path));
    }
    let mut parts = line.strip_prefix("PROGRESS ")?.split_whitespace();
    let done = parts.next().and_then(|v| v.parse::<f64>().ok());
    let total = parts.next().and_then(|v| v.parse::<f64>().ok());
    if parts.next() == Some("finished") {
        return Some(Line::Progress(Progress::Converting));
    }
    let fraction = done
        .zip(total.filter(|t| *t > 0.0))
        .map(|(d, t)| (d / t).clamp(0.0, 1.0));
    Some(Line::Progress(Progress::Downloading(fraction)))
}

/// yt-dlp with the common options; killed when dropped (aborting a task stops it).
fn command<S: AsRef<OsStr>>(args: &[S]) -> Command {
    let mut cmd = Command::new(binary());
    cmd.args(args).arg("--no-warnings");
    // Recent YouTube extraction wants a JS runtime; yt-dlp only looks for deno by default.
    if let Some(node) = find_in_path("node") {
        cmd.arg("--js-runtimes")
            .arg(format!("node:{}", node.display()));
    }
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    cmd
}

/// `yt-dlp` from PATH, else the standalone binary in ~/Applications
/// (apps started from the desktop don't always have ~/.local/bin in PATH).
fn binary() -> PathBuf {
    find_in_path("yt-dlp")
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join("Applications/yt-dlp/yt-dlp"))
        })
        .unwrap_or_else(|| PathBuf::from("yt-dlp"))
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

fn spawn_error(e: std::io::Error) -> AppError {
    AppError::Tool(match e.kind() {
        std::io::ErrorKind::NotFound => {
            "yt-dlp est introuvable (ni dans le PATH, ni dans ~/Applications/yt-dlp)".into()
        }
        _ => format!("Impossible de lancer yt-dlp : {e}"),
    })
}

/// The last non-empty stderr line is yt-dlp's actual error.
fn failure(stderr: &str) -> AppError {
    let reason = stderr.lines().rev().find(|l| !l.trim().is_empty());
    AppError::Tool(format!(
        "yt-dlp a échoué : {}",
        reason.unwrap_or("erreur inconnue")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_only_from_valid_ids() {
        assert_eq!(
            video_url("d7woYjGWs6k").unwrap(),
            "https://music.youtube.com/watch?v=d7woYjGWs6k"
        );
        assert!(video_url("d7woYjGWs6k&list=x").is_err());
        assert!(video_url("https://evil.example").is_err());
    }

    #[test]
    fn progress_lines() {
        assert_eq!(
            parse_line("FILE /m/Song [abc].opus"),
            Some(Line::File("/m/Song [abc].opus"))
        );
        assert_eq!(
            parse_line("PROGRESS 50 200 downloading"),
            Some(Line::Progress(Progress::Downloading(Some(0.25))))
        );
        assert_eq!(
            parse_line("PROGRESS 50 NA downloading"),
            Some(Line::Progress(Progress::Downloading(None)))
        );
        assert_eq!(
            parse_line("PROGRESS 300 200 downloading"),
            Some(Line::Progress(Progress::Downloading(Some(1.0))))
        );
        assert_eq!(
            parse_line("PROGRESS 200 200 finished"),
            Some(Line::Progress(Progress::Converting))
        );
        assert_eq!(parse_line("[youtube] Extracting URL"), None);
    }
}
