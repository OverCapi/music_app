# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Desktop music player (Spotify-like clone) for **local files**: Tauri 2 + SvelteKit 3 (Svelte 5 runes, SPA via adapter-static, `ssr = false`) with a Rust backend. It searches/downloads from YouTube via the external `yt-dlp` binary, and has synchronized "listening groups" through a separate Rust server. UI strings and user-facing errors are **French**; code and comments are English.

One Cargo workspace (root `Cargo.toml`, edition 2024, shared `[workspace.dependencies]` and `[workspace.lints]`, single `Cargo.lock`/`target/`):
- `app/src-tauri/` — the app backend (crate `music_app`, lib `music_app_lib`)
- `crates/server/` — listening-group server (crate `music-group-server`)
- `crates/protocol/` — crate `group-protocol`, WebSocket/REST types shared by both (feature `specta` derives TS types for the app)

## Model choice

When delegating work to subagents, pick the model by difficulty: **Haiku** for very simple tasks (a SQL file, a thin command wrapper, regenerating bindings), **Sonnet** by default, **Opus** only when the task needs a lot of reasoning (concurrency, engine/state-machine edge cases). Split large tasks into small ones so each can use the right model.

## Commands

```sh
cd app                          # pnpm commands run from app/
pnpm install
pnpm tauri dev                 # run the app (Vite on 127.0.0.1:1420, strictPort)
pnpm check                     # svelte-check (expected: 0 errors, 0 warnings)
pnpm lint                      # eslint + prettier --check (pnpm format to fix)
pnpm build                     # frontend production build

cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace         # also regenerates app/src/lib/bindings.ts
cargo test -p music-group-server --test api <name>   # single server integration test
cargo run -p music-group-server   # env: MUSIC_SERVER_ADDR (127.0.0.1:8787), MUSIC_SERVER_DB (./server.db), MUSIC_SERVER_READY_TIMEOUT_MS (30000)
```

`tauri::generate_context!` needs the `build/` folder to exist (`mkdir -p build`) for cargo commands on a fresh checkout.

Second app instance for testing groups (uses the first instance's Vite dev server, needs a built debug binary):
`MUSIC_APP_PROFILE=b target/debug/music_app` — the profile gets separate data/cache dirs (`<data>/profiles/b`).

Runtime requirements: `yt-dlp`, `ffmpeg`, `node` (passed to yt-dlp as JS runtime if present), all looked up with `ytdlp::search_path()` (PATH + `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/Applications/yt-dlp`, WinGet links; an app started from the desktop has a minimal PATH). libopus is compiled in (`bundled`, needs `cmake`). Spawn external tools with `ytdlp::tool_command` (PATH + no console window on Windows).

## Architecture

### Backend (`app/src-tauri/src/`)
One module per feature: `mod.rs` holds the logic (testable without Tauri, unit tests use `Db::in_memory()` and `None` for the `AppHandle`), `commands.rs` holds thin `#[tauri::command] #[specta::specta]` wrappers. `lib.rs` builds managed state in `setup` and lists every command and event in `specta_builder()`.

**Adding a feature**: new module (`mod.rs` + `commands.rs`), manage its state in `setup`, add its commands/events to `specta_builder()`, a new numbered SQL file in `db/migrations/` (appended to `MIGRATIONS`, never edit a shipped one) if it stores data, then a store in `app/src/lib/stores/` + components. Run `cargo test -p music_app` to regenerate `app/src/lib/bindings.ts` (committed, never edited by hand).

- **Contract**: tauri-specta generates `app/src/lib/bindings.ts` (`commands.*`, `events.*`, types). Errors are `AppError` (`error.rs`), serialized `{kind, message}` and **thrown** in TS. Rust `f64` exports as `number | null`; ids/timestamps (`i64`/`u64`) are cast to `number`.
- **State flows by events, never polling**: getters (`get_*`) for the initial load, mutations return `()` (or the created item), and the backend pushes the new state: `PlayerChanged`, `QueueChanged`, `LibraryChanged`, `PlaylistsChanged`, `DownloadProgress`, `SearchDetails`, `GroupChanged`, `GroupError`, `SyncProgress`.
- **`db/`** — SQLite (rusqlite, one connection behind a mutex, WAL, foreign keys). Tables `tracks`, `playlists`, `playlist_tracks` (ON DELETE CASCADE: removing a track cleans playlists), `live_channels` (favorite channels of the Lives page), `settings` (key/value: `server_url`, `username`, `token`, `legacy_imported`, `ytmusic_browser`). `import_legacy.rs` imports the old `library.json`/`playlists.json`/`account.json` once.
- **`audio/`** — playback engine. A dedicated thread (`engine.rs`) owns the rodio 0.22 output (`MixerDeviceSink`/`Player`, not `Send`) and receives `AudioCommand`s over an mpsc channel; `AudioHandle` (managed state, `mod.rs`) shares `PlayerState` + the queue via `Arc<Mutex<Shared>>`. `queue.rs` is a pure `Queue` (unit-tested). The queue lives in the engine so tracks advance without the UI; when it runs out the engine appends a random library track (`Library::random_picker`). Only the engine thread emits player/queue events. **Group mode**: the group client drives playback with scheduled starts (`GroupStart`); local queue commands return `AppError::GroupMode`, and the engine loop wakes exactly at the scheduled instant. `live.rs` plays a YouTube live as a radio: `yt-dlp -o -` → `ffmpeg` → raw f32 PCM in a small buffer (`LiveSource`, silence when empty, restarted a few times on failure); a live sits outside the queue and "next" leaves it for the queue (`PlayerState.live`). `opus.rs` is a custom rodio `Source` for `.opus` (symphonia Ogg demuxer + libopus) — rodio has no Opus codec.
- **`library/` / `playlists/`** — tracks have an integer `id` (`TrackId`); downloaded tracks carry `youtube_id`. Removing a library track moves the file to the trash only if it is inside the app music folder.
- **`ytdlp.rs`** — the only place that runs yt-dlp (never through a shell, `kill_on_drop`). URLs are built only from validated ids (`video_url`).
- **`search/`** — YouTube Music flat search only returns id+title, so results are returned immediately and enriched in the background (bounded parallel yt-dlp calls) via `SearchDetails`; a `search_id` discards stale results; `next_offset` drives pagination.
- **`downloads/`** — `yt-dlp -x --audio-format opus --embed-metadata --embed-thumbnail` into `~/Music/Music App`, progress parsed from a custom `--progress-template`. `download()` is reused by the group client.
- **`lives/`** — finding YouTube lives (keyword search, channel search/listing, pasted link) via yt-dlp, plus favorite channels. Only validated video ids / channel handles are kept; URLs are rebuilt (`ytdlp::watch_url`). `proxy.rs` serves a `live://localhost/<id>.m3u8` scheme (video-only HLS playlist, segments relayed with CORS headers, only `https` `*.googlevideo.com`) so hls.js in the webview can show the video muted over the audio-engine playback; the audio is never switched to the webview.
- **`sync/`** — one-way import of other services into the library. YouTube Music: yt-dlp `--flat-playlist` on the liked songs (`LM`) and `/feed/playlists` with `--cookies-from-browser` (browser in `settings.ytmusic_browser`); tracks are added without file (`Library::add_remote`, deduplicated by YouTube id then `match_key`) and downloaded when played (`TrackFetcher` → `Downloads::fetch`). Each service playlist fills a local one linked in `sync_playlists`; a sync replaces its content. Progress via `SyncProgress`.
- **`history/` / `feedback/` / `taste/` / `radio/` / `mixes/` / `discoveries/` / `suggestions/`** — recommendations: plays and likes/dislikes feed a taste profile (`taste`, pure); `radio` caches YouTube radios of seed tracks; `mixes` (Que du connu / Que du nouveau / Mélangé) and the weekly `discoveries` are generated on demand from them; `suggestions` plays `SuggestedTrack`s (YouTube ids, downloaded on play). The home page shows them (`home` store).
- **`account/` / `group/`** — listening-group client: REST login (reqwest, rustls), token never exposed to the frontend. `group/` is split into `clock.rs` (NTP-style median offset, pure), `reconcile.rs` (server state → prefetch/Ready/engine target, pure), `connection.rs` (WebSocket task, reconnect + re-Join + re-sent `Ready`), `files.rs` (`<cache>/groups/<CODE>/`, wiped at startup, deleted on leave/quit). Engine commands are only re-issued when the derived target changes.

### Listening-group protocol (`crates/protocol/src/lib.rs`)
Read its module doc before touching group code. The server is the source of truth and broadcasts the full `GroupState` after every change; clients send intents. Playback is `Idle | Preparing{deadline_ms} | Playing{position, at_ms} | Paused{position}` in **server time**; `Playing` means "at server time `at_ms` the track is at `position`". The server only relays **validated YouTube ids** and positions ≤ `MAX_POSITION_SECS`; clients rebuild download URLs themselves from the id (never use a URL from another member).

### Server (`crates/server/src/`)
axum + WebSocket. `group.rs` is a pure state machine (time injected → unit-testable), `hub.rs` handles connections, broadcasts and deadline timers (generation counter so stale timers are no-ops), `db.rs` is SQLite (rusqlite + rusqlite_migration, `app/src/migrations/`) with argon2 passwords and sha256-hashed tokens. Groups are in memory and deleted when empty; membership does not survive a reconnect. Integration tests (`tests/api.rs`) use `Client::sync()` (Ping/Pong) to make sure the server handled earlier frames before another client acts.

### Frontend (`app/src/`)
SvelteKit 3: config lives in `app/vite.config.js` (`sveltekit({ adapter })`, there is no `svelte.config.js`); import `app/src/lib` as `#lib/...` **with the file extension** (`package.json` `imports`). Routes: `routes/+layout.svelte` (layout `Sidebar | page | optional QueueView`, `PlayerBar` at the bottom; initializes every store in `onMount`), `+page.svelte` (home: mix and discoveries cards, recent plays; data loaded lazily by the `home` store, never at startup), `mix/[id]/` and `discoveries/` (playlist-like pages over the in-memory `home` mixes, not stored, not in the sidebar), `library/` (the library: albums/artists are grouped client-side from track metadata), `search/`, `playlist/[id]/`, `group/`, `lives/` (search, favorite channels, 16:9 video panel that expands to a fullscreen overlay, `components/live/`), `settings/` (appearance, YouTube Music sync, Spotify placeholder). Design is "Halo" (liquid glass): tokens and shared classes (`.glass`, `.lift`, `.row`, `.halo-btn`, `.play-btn`, `.pill`, `.field`) live in `app/src/app.css`; appearance is a mode (`data-mode="dark|light"`) plus an accent hue (`--accent-h`), chosen in `theme.svelte.ts` (localStorage, applied before first paint by `app.html`). Shell: floating Playlists panel (left, reorder/pin by drag), top bar, player dock, queue panel (right, reorder by drag); under 768 px the panels become drawers (`ui.svelte.ts` holds panel/width state). Pages are a `.view` size container (`@container page`); lists are a `TrackList` container (`@container tracks`). Animations are in `lib/motion.ts` (Web Animations API: `liquid` press, `flip`, page enter/exit). Playlist order/pin and queue moves are persisted by the backend (`reorder_playlists`, `set_playlist_pinned`, `move_in_queue`).

State lives in class-based singleton stores in `app/src/lib/stores/*.svelte.ts` (`player`, `library`, `playlists`, `search`, `downloads`, `account`, `group`, `home`, `toasts`, `lives`, `sync`, `feedback`, `theme`): `init()` subscribes to events then loads the initial state. Errors go through `attempt()` (`toasts.svelte.ts`), which shows the `AppError` message as a toast. `player` interpolates the position between `PlayerChanged` events. Reusable primitives in `app/src/lib/components/ui/` (`Panel`, `Card`, `MediaCard`, `MiniCard`, `Cover`, `Button`, `IconButton`, `PlayButton`, `SegmentedPills`, `TrackList` + `TrackRow`, `RowMenu`, `PageHeader`, `Section`, `Toggle`, `Checkbox`, `Chip`, `Avatar`, `Skeleton`, `ProgressBar`, `Equalizer`, `Visualizer`, `Icon`, `Toasts`); track rows for pages are in `components/tracks/`; feature components in `app/src/lib/components/<feature>/`.

`app/vite.config.js` contains a dev-only plugin (`compileSvelteBeforeCss`) that compiles a component before serving its virtual CSS module; without it, a CSS request arriving before the component's JS caches the raw `.svelte` file as CSS.
