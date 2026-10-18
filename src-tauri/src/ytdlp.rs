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

/// `https://www.youtube.com/watch?v=<id>` (live streams), only for a valid id.
pub fn watch_url(youtube_id: &str) -> AppResult<String> {
    if !is_valid_youtube_id(youtube_id) {
        return Err(AppError::invalid("Identifiant YouTube invalide"));
    }
    Ok(format!("https://www.youtube.com/watch?v={youtube_id}"))
}

/// `https://www.youtube.com/@<handle>/streams`, only for a valid channel handle.
pub fn channel_streams_url(handle: &str) -> AppResult<String> {
    let valid = (3..=30).contains(&handle.len())
        && handle
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !valid {
        return Err(AppError::invalid("Nom de chaîne YouTube invalide"));
    }
    Ok(format!("https://www.youtube.com/@{handle}/streams"))
}

/// A channel id: `UC` + 22 URL-safe base64 characters.
pub fn is_valid_channel_id(id: &str) -> bool {
    id.len() == 24
        && id.starts_with("UC")
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// `https://www.youtube.com/channel/<id>/streams`, only for a valid channel id.
pub fn channel_id_streams_url(channel_id: &str) -> AppResult<String> {
    if !is_valid_channel_id(channel_id) {
        return Err(AppError::invalid("Identifiant de chaîne invalide"));
    }
    Ok(format!(
        "https://www.youtube.com/channel/{channel_id}/streams"
    ))
}

/// Radio URLs of a seed track, only for a valid id: the YouTube Music radio
/// (`RDAMVM<id>`) first, then the classic YouTube Mix (`RD<id>`) as fallback.
pub fn radio_urls(seed_id: &str) -> AppResult<[String; 2]> {
    if !is_valid_youtube_id(seed_id) {
        return Err(AppError::invalid("Identifiant YouTube invalide"));
    }
    Ok([
        format!("https://music.youtube.com/watch?v={seed_id}&list=RDAMVM{seed_id}"),
        format!("https://www.youtube.com/watch?v={seed_id}&list=RD{seed_id}"),
    ])
}

/// `https://www.youtube.com/playlist?list=<id>`, only for a valid playlist id.
pub fn playlist_url(playlist_id: &str) -> AppResult<String> {
    let valid = (2..=64).contains(&playlist_id.len())
        && playlist_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !valid {
        return Err(AppError::invalid("Identifiant de playlist invalide"));
    }
    Ok(format!(
        "https://www.youtube.com/playlist?list={playlist_id}"
    ))
}

/// Browsers yt-dlp can read cookies from (`--cookies-from-browser`).
pub const COOKIE_BROWSERS: &[&str] = &[
    "brave", "chrome", "chromium", "edge", "firefox", "opera", "safari", "vivaldi", "whale",
];

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

/// yt-dlp writing the audio of a live stream to stdout (HLS as MPEG-TS), for
/// `audio::live` to pipe into ffmpeg. Format 91 is the smallest HLS format, which
/// has audio when no audio-only one exists. Synchronous: it runs on its own thread.
pub fn live_audio_command(youtube_id: &str) -> AppResult<std::process::Command> {
    let url = watch_url(youtube_id)?;
    let mut cmd = std::process::Command::new(binary());
    cmd.args(["--no-playlist", "--no-warnings", "-q", "--hls-use-mpegts"])
        .args(["-f", "234/bestaudio/91", "-o", "-"])
        .arg(url);
    if let Some(node) = find_in_path("node") {
        cmd.arg("--js-runtimes")
            .arg(format!("node:{}", node.display()));
    }
    Ok(cmd)
}

/// URL of the HLS playlist of the video-only stream (720p at most) of a live, for the
/// `live://` proxy. It expires after a few hours.
pub async fn live_video_playlist_url(youtube_id: &str) -> AppResult<String> {
    let url = watch_url(youtube_id)?;
    let output = command(&[
        "--no-playlist",
        "-g",
        "-f",
        "232/best[vcodec!=none][acodec=none][height<=720]/best[height<=720]",
        &url,
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .await
    .map_err(spawn_error)?;
    if !output.status.success() {
        return Err(failure(&String::from_utf8_lossy(&output.stderr)));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| AppError::Tool("yt-dlp n'a renvoyé aucune URL vidéo".into()))
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

pub(crate) fn spawn_error(e: std::io::Error) -> AppError {
    AppError::Tool(match e.kind() {
        std::io::ErrorKind::NotFound => {
            "yt-dlp est introuvable (ni dans le PATH, ni dans ~/Applications/yt-dlp)".into()
        }
        _ => format!("Impossible de lancer yt-dlp : {e}"),
    })
}

/// The last non-empty stderr line is yt-dlp's actual error.
fn failure(stderr: &str) -> AppError {
    if stderr.contains("does not pass filter") {
        return AppError::invalid("Impossible de télécharger un direct");
    }
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
        assert_eq!(
            playlist_url("PLabc_-1").unwrap(),
            "https://www.youtube.com/playlist?list=PLabc_-1"
        );
        assert!(playlist_url("PL&x=1").is_err());
        assert!(playlist_url("").is_err());
    }

    #[test]
    fn live_urls_only_from_valid_ids_and_handles() {
        assert_eq!(
            watch_url("jfKfPfyJRdk").unwrap(),
            "https://www.youtube.com/watch?v=jfKfPfyJRdk"
        );
        assert!(watch_url("jfKfPfyJRdk&x=1").is_err());
        assert_eq!(
            channel_streams_url("LofiGirl").unwrap(),
            "https://www.youtube.com/@LofiGirl/streams"
        );
        for bad in ["", "ab", "a/b/c", "x?y=1", "a b c"] {
            assert!(channel_streams_url(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn channel_ids() {
        assert!(is_valid_channel_id("UCSJ4gkVC6NrvII8umztf0Ow"));
        for bad in [
            "",
            "UCshort",
            "XXSJ4gkVC6NrvII8umztf0Ow",
            "UCSJ4gkVC6NrvII8umztf0O/",
        ] {
            assert!(channel_id_streams_url(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn radio_urls_only_from_valid_ids() {
        assert_eq!(
            radio_urls("dQw4w9WgXcQ").unwrap(),
            [
                "https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=RDAMVMdQw4w9WgXcQ",
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=RDdQw4w9WgXcQ",
            ]
        );
        for bad in [
            "",
            "abc",
            "dQw4w9WgXcQ&list=x",
            "dQw4w9/gXcQ",
            "dQw4w9 gXcQ",
        ] {
            assert!(radio_urls(bad).is_err(), "{bad:?}");
        }
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
