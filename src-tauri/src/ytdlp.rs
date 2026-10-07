//! The only place that runs the external `yt-dlp` binary (never through a shell).
//!
//! The binary is looked up in PATH, then `~/Applications/yt-dlp/yt-dlp`. If `node`
//! is in PATH it is passed to yt-dlp as JS runtime. Used by `search` and `downloads`.
//!
//! URLs are always built here from a validated YouTube id
//! ([`group_protocol::is_valid_youtube_id`]), never taken from user input.

// TODO(agent C): port from master `src-tauri/src/youtube.rs` (`ytdlp_path`, `find_in_path`,
// `run_ytdlp`) and `downloads.rs` (spawn + progress parsing).
