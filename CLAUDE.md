# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Desktop music player (Spotify-like clone) for **local files**: Tauri 2 + SvelteKit (Svelte 5 runes, SPA via adapter-static, `ssr = false`) with a Rust backend. It searches/downloads from YouTube via the external `yt-dlp` binary, and has synchronized "listening groups" through a separate Rust server. UI strings and user-facing errors are **French**; code and comments are English.

Three independent Cargo projects (no workspace, each has its own `Cargo.lock`/`target/`):
- `src-tauri/` — the app backend (crate `music_app`, lib `music_app_lib`)
- `server/` — listening-group server (crate `music-group-server`)
- `protocol/` — crate `group-protocol`, WebSocket/REST types shared by both (path dependency)

## Commands

```sh
pnpm install
pnpm tauri dev                 # run the app (Vite on 127.0.0.1:1420, strictPort)
pnpm check                     # svelte-check / TypeScript (expected: 0 errors, 0 warnings)
pnpm build                     # frontend production build

cd src-tauri && cargo clippy --all-targets && cargo test
cd server    && cargo test     # unit tests (src/group.rs) + integration tests (tests/api.rs)
cd server    && cargo test --test api <name>   # single integration test
cd server    && cargo run      # env: MUSIC_SERVER_ADDR (127.0.0.1:8787), MUSIC_SERVER_DB (./server.db), MUSIC_SERVER_READY_TIMEOUT_MS (30000)
cd protocol  && cargo test
```

Second app instance for testing groups (uses the first instance's Vite dev server, needs a built debug binary):
`MUSIC_APP_PROFILE=b src-tauri/target/debug/music_app` — the profile gets separate data/cache dirs (`<data>/profiles/b`).

Runtime requirements: `yt-dlp` (looked up in PATH, then `~/Applications/yt-dlp/yt-dlp`), `ffmpeg`, system `libopus` (linked dynamically), `node` (passed to yt-dlp as JS runtime if present).

## Architecture

### Backend (`src-tauri/src/`)
`lib.rs` registers all Tauri commands and builds managed state in `setup` (order matters: `Library` is created before `AudioHandle`, which receives `library.random_picker()`).

- **`audio.rs`** — playback engine. A dedicated thread owns the rodio 0.22 output (`MixerDeviceSink`/`Player`, not `Send`) and receives `AudioCommand`s over an mpsc channel; `AudioHandle` (managed state) shares `PlayerState` + the queue via `Arc<Mutex<Shared>>`. The **queue lives in the engine** so tracks advance without the UI; when it runs out the engine appends a random library track. `queue_version` is bumped on every queue change so the UI (which polls `get_state` every 250 ms) knows to refetch `get_queue`. **Group mode**: the group client drives playback with scheduled starts (`GroupStart`); local queue commands then return an error, and the engine loop wakes exactly at the scheduled instant (not on its 100 ms tick).
- **`opus.rs`** — custom rodio `Source` for `.opus` (symphonia Ogg demuxer + libopus via `symphonia-adapter-libopus`), because rodio has no Opus codec and no codec registry hook. Decoder is chosen by file extension in `audio.rs::open_track`.
- **`library.rs` / `playlists.rs`** — JSON persistence in the app data dir through `storage.rs` (`load_json` / atomic `save_json`). Tracks are identified by absolute path; downloaded tracks carry `youtube_id`. Playlists store paths; removing a library track also removes it from playlists and (only if inside the app music folder) moves the file to the trash.
- **`youtube.rs`** — search via yt-dlp subprocesses (never through a shell). YouTube Music flat search only returns id+title, so results are returned immediately and enriched in the background (bounded parallel yt-dlp calls) via `search-details` events; a `search_id` discards stale results.
- **`downloads.rs`** — `yt-dlp -x --audio-format opus --embed-metadata --embed-thumbnail` into `~/Music/Music App`, progress parsed from a custom `--progress-template` on stdout, emitted as `download-progress`; `library-changed` when a file is added. `download()` / `download_with_retry()` are reused by the group client.
- **`account.rs` / `group.rs`** — listening-group client: REST login (reqwest, rustls), one WebSocket task (tokio-tungstenite) with reconnect + automatic re-Join and re-sending `Ready`, NTP-style clock offset (median of Ping/Pong samples), downloads of current + next 2 tracks into `<cache>/groups/<CODE>/` (deleted on leave/quit, wiped at startup). Engine commands are only re-issued when the derived playback target changes. Emits `group-state` (`GroupView | null`) and `group-error`.

### Listening-group protocol (`protocol/src/lib.rs`)
Read its module doc before touching group code. The server is the source of truth and broadcasts the full `GroupState` after every change; clients send intents. Playback is `Idle | Preparing{deadline_ms} | Playing{position, at_ms} | Paused{position}` in **server time**; `Playing` means "at server time `at_ms` the track is at `position`". The server only relays **validated YouTube ids**; clients rebuild download URLs themselves from the id (never use a URL from another member).

### Server (`server/src/`)
axum + WebSocket. `group.rs` is a pure state machine (time injected → unit-testable), `hub.rs` handles connections, broadcasts and deadline timers (generation counter so stale timers are no-ops), `db.rs` is SQLite (rusqlite) with argon2 passwords and sha256-hashed tokens. Groups are in memory and deleted when empty; membership does not survive a reconnect.

### Frontend (`src/`)
Single route `src/routes/+page.svelte` (layout: `Sidebar | view | optional QueueView`, `PlayerBar` at the bottom; global CSS variables defined there). State lives in class-based singleton stores in `src/lib/*.svelte.ts` (`player`, `library`, `playlists`, `search`, `downloads`, `account`, `group`, `nav`) that wrap `invoke` calls and Tauri event listeners (subscribed in `+page.svelte` `onMount`). Views are switched with `nav.view` (no SvelteKit routing). `TrackTable.svelte` is the shared track list; row buttons are passed through its `actions` snippet and use the `row-action` class (styled by `TrackTable` via `:global`).

`vite.config.js` contains a dev-only plugin (`compileSvelteBeforeCss`) that compiles a component before serving its virtual CSS module; without it, a CSS request arriving before the component's JS caches the raw `.svelte` file as CSS.
