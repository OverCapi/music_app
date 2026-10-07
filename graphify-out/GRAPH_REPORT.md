# Graph Report - music_app  (2026-10-07)

## Corpus Check
- Corpus is ~37,737 words - fits in a single context window. You may not need a graph.

## Summary
- 1305 nodes · 2716 edges · 54 communities (43 shown, 11 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 48 edges (avg confidence: 0.85)
- Token cost: 56,583 input · 0 output

## Community Hubs (Navigation)
- AudioHandle Commands API
- Account & Group Commands
- Protocol Client Messages
- Frontend Component Imports
- Server Group State Machine
- DB & Legacy Import
- Audio Engine Thread
- Downloads & yt-dlp
- Protocol Shared Types
- Opus Decoder
- Server Auth DB
- Server HTTP App
- Server Integration Tests
- App Setup & Reconcile
- Generated TS Bindings
- Layout & Download UI
- Search & Download Commands
- Playlists Module
- Account Store (Rust)
- Library & Player Stores
- Player Bar UI
- Group WebSocket Connection
- Lint & Package Config
- Group Session UI
- Group Lobby & Login UI
- Frontend Dev Dependencies
- Tauri Config
- Sidebar & Add To Group
- Add To Playlist UI
- AppError Type
- Typed Contract Rules
- Playback Architecture Docs
- Project Structure Docs
- NPM Scripts
- Server Docs & Roadmap
- Group Client Docs
- TS Config
- Protocol & yt-dlp Docs
- Account Store (TS)
- Vite Config
- Search Store
- Tauri Capabilities
- Data & Download Docs
- CI Pipeline
- Runtime JS Dependencies
- Workspace Crates
- Import Alias
- Invite Deep Link
- Run Script
- SPA Layout Config
- Group Code UIDs (TODO)

## God Nodes (most connected - your core abstractions)
1. `State` - 52 edges
2. `AudioHandle` - 47 edges
3. `attempt()` - 46 edges
4. `Group` - 31 edges
5. `ClientMessage` - 25 edges
6. `group` - 25 edges
7. `GroupClient` - 24 edges
8. `Engine` - 22 edges
9. `player` - 22 edges
10. `Queue` - 21 edges

## Surprising Connections (you probably didn't know these)
- `group_add()` --calls--> `is_valid_youtube_id()`  [INFERRED]
  src-tauri/src/group/commands.rs → crates/protocol/src/lib.rs
- `video_url()` --calls--> `is_valid_youtube_id()`  [INFERRED]
  src-tauri/src/ytdlp.rs → crates/protocol/src/lib.rs
- `SvelteKit app.html shell (lang=fr)` --conceptually_related_to--> `SvelteKit frontend (src/)`  [INFERRED]
  src/app.html → CLAUDE.md
- `group_join()` --calls--> `parse_invite()`  [INFERRED]
  src-tauri/src/group/commands.rs → crates/protocol/src/lib.rs
- `frame()` --references--> `ClientMessage`  [EXTRACTED]
  src-tauri/src/group/connection.rs → crates/protocol/src/lib.rs

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Listening group synchronized playback flow** — claude_groupstate, claude_playback_state, claude_clock, claude_reconcile, claude_connection, claude_server_hub, claude_audio_engine [INFERRED 0.85]
- **Typed backend-frontend contract** — claude_tauri_specta, claude_specta_builder, claude_bindings_ts, claude_apperror, claude_attempt [EXTRACTED 1.00]
- **yt-dlp search/download pipeline** — claude_ytdlp, claude_search, claude_downloads, claude_validated_youtube_ids [INFERRED 0.85]

## Communities (54 total, 11 thin omitted)

### Community 0 - "AudioHandle Commands API"
Cohesion: 0.06
Nodes (19): AudioHandle, clamp_position(), PlayerChanged, PlayerState, QueueChanged, QueueItem, QueueState, RESTART_THRESHOLD_SECS (+11 more)

### Community 1 - "Account & Group Commands"
Cohesion: 0.07
Nodes (44): parse_invite(), State, get_account(), login(), logout(), register(), set_server_url(), clear_queue() (+36 more)

### Community 2 - "Protocol Client Messages"
Cohesion: 0.05
Nodes (35): ClientMessage, Add, Create, Ended, Join, Leave, Pause, Ping (+27 more)

### Community 3 - "Frontend Component Imports"
Cohesion: 0.05
Nodes (35): QueueItem, playing, queue, { view }, { download }, progress, { trackId }, current (+27 more)

### Community 4 - "Server Group State Machine"
Cohesion: 0.09
Nodes (28): A, add(), add_on_idle_prepares_and_requests_timer(), B, deadline_starts_and_stale_deadlines_are_ignored(), ended_only_counts_for_current_entry(), entry_id(), ERR_NOT_FOUND (+20 more)

### Community 5 - "DB & Legacy Import"
Cohesion: 0.08
Nodes (22): imports_and_renames_legacy_files(), LegacyAccount, LegacyPlaylist, LegacyTrack, read(), run(), CLEANUP_DELAY, remove_dir() (+14 more)

### Community 6 - "Audio Engine Thread"
Cohesion: 0.07
Nodes (24): AudioCommand, EnterGroup, GroupLoad, GroupStop, LeaveGroup, Pause, Play, PlayIndex (+16 more)

### Community 7 - "Downloads & yt-dlp"
Cohesion: 0.06
Nodes (28): ATTEMPTS, download(), DownloadProgress, Downloads, DownloadStatus, Cancelled, Converting, Done (+20 more)

### Community 8 - "Protocol Shared Types"
Cohesion: 0.07
Nodes (34): AuthResponse, CODE_LEN, Credentials, ErrorResponse, GroupState, LINK_PREFIX, MAX_POSITION_SECS, Member (+26 more)

### Community 9 - "Opus Decoder"
Cohesion: 0.06
Nodes (11): OPUS_SAMPLE_RATE, OpusSource, Clock, falls_back_to_wall_clock(), keeps_the_latest_samples(), MAX_SAMPLES, MAX_SECS, median() (+3 more)

### Community 10 - "Server Auth DB"
Cohesion: 0.06
Nodes (18): AuthError, BadCredentials, Join, PasswordHash, Random, Sqlite, UsernameTaken, blocking() (+10 more)

### Community 11 - "Server HTTP App"
Cohesion: 0.07
Nodes (22): AppState, auth_error(), Config, credentials(), env(), Error, Database, Env (+14 more)

### Community 12 - "Server Integration Tests"
Cohesion: 0.14
Nodes (24): accounts(), add_and_ready(), assert_near(), Client, create_and_join(), ended_advances_once(), leave_and_disconnect(), LONG_TIMEOUT (+16 more)

### Community 13 - "App Setup & Reconcile"
Cohesion: 0.09
Nodes (20): Actions, current_entry(), engine_recommanded_only_on_change(), entry(), files(), plan(), plan_of(), PREFETCH (+12 more)

### Community 14 - "Generated TS Bindings"
Cohesion: 0.09
Nodes (28): @tauri-apps/api, AppError, commands, DownloadStatus, EventEmit, events, GroupChanged, GroupError (+20 more)

### Community 15 - "Layout & Download UI"
Cohesion: 0.08
Nodes (14): svelte, DownloadProgress, download, { id }, inLibrary, progress, { progress, stop = false }, Download (+6 more)

### Community 16 - "Search & Download Commands"
Cohesion: 0.09
Nodes (15): cancel_download(), download_track(), Details, fetch_details(), FlatEntry, MAX_PARALLEL_DETAILS, PAGE_SIZE, Search (+7 more)

### Community 17 - "Playlists Module"
Cohesion: 0.18
Nodes (11): clean_name(), crud_and_ordering(), ensure_exists(), gone(), MAX_NAME_LEN, names_are_validated(), Playlist, Playlists (+3 more)

### Community 18 - "Account Store (Rust)"
Cohesion: 0.17
Nodes (12): Account, AccountStore, clear_credentials(), DEFAULT_SERVER_URL, get(), HTTP_TIMEOUT, normalize_server_url(), server_url() (+4 more)

### Community 19 - "Library & Player Stores"
Cohesion: 0.12
Nodes (4): library, player, playlists, attempt()

### Community 20 - "Player Bar UI"
Cohesion: 0.08
Nodes (20): canSeek, duration, entry, groupLoaded, groupMode, groupStatus, livePosition, now (+12 more)

### Community 21 - "Group WebSocket Connection"
Cohesion: 0.12
Nodes (16): BACKOFF_SECS, CONNECT_TIMEOUT, frame(), INITIAL_PING_INTERVAL, INITIAL_PINGS, Outgoing, Close, Msg (+8 more)

### Community 22 - "Lint & Package Config"
Cohesion: 0.11
Nodes (19): description, license, name, type, version, eslint, eslint-config-prettier, @eslint/js (+11 more)

### Community 23 - "Group Session UI"
Cohesion: 0.12
Nodes (6): GroupView, copied, hasCurrent, status, { view }, group

### Community 24 - "Group Lobby & Login UI"
Cohesion: 0.17
Nodes (8): invite, onsubmit(), password, serverUrl, submit(), username, valid, account

### Community 25 - "Frontend Dev Dependencies"
Cohesion: 0.11
Nodes (18): devDependencies, eslint, eslint-config-prettier, @eslint/js, eslint-plugin-svelte, globals, prettier, prettier-plugin-svelte (+10 more)

### Community 26 - "Tauri Config"
Cohesion: 0.11
Nodes (17): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+9 more)

### Community 27 - "Sidebar & Add To Group"
Cohesion: 0.12
Nodes (10): add(), added, { entry, size = 16 }, label, queued, create(), creating, input (+2 more)

### Community 28 - "Add To Playlist UI"
Cohesion: 0.16
Nodes (13): add(), choices, close(), createAndAdd(), creating, input, menu, newName (+5 more)

### Community 29 - "AppError Type"
Cohesion: 0.24
Nodes (9): AppError, Db, GroupMode, Internal, Invalid, Io, Network, NotFound (+1 more)

### Community 30 - "Typed Contract Rules"
Cohesion: 0.33
Nodes (7): Prettier config (printWidth 100, .prettierignore), AppError {kind, message}, attempt() toast error helper, src/lib/bindings.ts (tauri-specta generated), specta_builder(), Class-based singleton stores (*.svelte.ts), tauri-specta

### Community 31 - "Playback Architecture Docs"
Cohesion: 0.20
Nodes (10): audio engine thread (engine.rs), AudioHandle, Group mode playback (GroupStart), library/ and playlists/ modules, opus.rs custom rodio Source, Queue (queue.rs), Library::random_picker, server hub.rs (broadcasts, deadline timers, generation counter) (+2 more)

### Community 32 - "Project Structure Docs"
Cohesion: 0.22
Nodes (7): Cargo workspace, compileSvelteBeforeCss Vite plugin, SvelteKit frontend (src/), Music App (Tauri 2 + SvelteKit desktop player), src-tauri app backend (music_app), UI primitives (Icon, Slider, ProgressRing, TrackTable, Toasts), SvelteKit app.html shell (lang=fr)

### Community 33 - "NPM Scripts"
Cohesion: 0.20
Nodes (10): scripts, build, check, check:watch, dev, format, lint, prepare (+2 more)

### Community 34 - "Server Docs & Roadmap"
Cohesion: 0.22
Nodes (8): music-group-server crate, server integration tests (tests/api.rs, Client::sync), server db.rs (argon2, sha256 tokens), server group.rs pure state machine, Server env vars (MUSIC_SERVER_ADDR, MUSIC_SERVER_DB, MUSIC_SERVER_READY_TIMEOUT_MS), Docker install script for server, Server login rate limiting, Server deployment (HTTPS/WSS, hosting, backup)

### Community 35 - "Group Client Docs"
Cohesion: 0.22
Nodes (8): account/ and group/ listening-group client, group clock.rs (NTP-style median offset), group connection.rs (WebSocket reconnect), group files.rs (group cache dir), MUSIC_APP_PROFILE second instance, group reconcile.rs, Resync mid-track when clock offset changes, Separate webview storage per profile

### Community 36 - "TS Config"
Cohesion: 0.22
Nodes (8): $app/tsconfig, compilerOptions, allowJs, checkJs, strict, exclude, extends, include

### Community 37 - "Protocol & yt-dlp Docs"
Cohesion: 0.25
Nodes (7): group-protocol crate, GroupState (server source of truth), Playback Idle|Preparing|Playing|Paused in server time, Runtime requirements (yt-dlp, ffmpeg, libopus, node), search/ module (background enrichment, search_id), ytdlp.rs (sole yt-dlp runner), Configurable yt-dlp path

### Community 39 - "Vite Config"
Cohesion: 0.29
Nodes (3): @sveltejs/adapter-static, @sveltejs/kit, vite

### Community 41 - "Tauri Capabilities"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 42 - "Data & Download Docs"
Cohesion: 0.40
Nodes (4): db/ SQLite module, downloads/ module (opus into ~/Music/Music App), import_legacy.rs (legacy JSON import), Music App features (playback, queue, library, playlists, search, download, groups)

### Community 44 - "Runtime JS Dependencies"
Cohesion: 0.67
Nodes (3): dependencies, @tauri-apps/api, @tauri-apps/plugin-dialog

### Community 45 - "Workspace Crates"
Cohesion: 0.67
Nodes (3): group-protocol, music_app, music-group-server

## Knowledge Gaps
- **318 isolated node(s):** `START_LEAD_MS`, `RESUME_LEAD_MS`, `READY_TIMEOUT_MS`, `MAX_POSITION_SECS`, `CODE_LEN` (+313 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 501 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **11 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AudioHandle` connect `AudioHandle Commands API` to `Account & Group Commands`, `Protocol Client Messages`, `Audio Engine Thread`?**
  _High betweenness centrality (0.057) - this node is a cross-community bridge._
- **What connects `START_LEAD_MS`, `RESUME_LEAD_MS`, `READY_TIMEOUT_MS` to the rest of the system?**
  _318 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `AudioHandle Commands API` be split into smaller, more focused modules?**
  _Cohesion score 0.06037000973709834 - nodes in this community are weakly interconnected._
- **Why does `State` connect `Account & Group Commands` to `Protocol Shared Types`, `Search & Download Commands`, `Protocol Client Messages`, `Server HTTP App`?**
  _High betweenness centrality (0.036) - this node is a cross-community bridge._
- **Should `Account & Group Commands` be split into smaller, more focused modules?**
  _Cohesion score 0.06893106893106893 - nodes in this community are weakly interconnected._
- **Should `Protocol Client Messages` be split into smaller, more focused modules?**
  _Cohesion score 0.05251141552511415 - nodes in this community are weakly interconnected._
- **Should `Frontend Component Imports` be split into smaller, more focused modules?**
  _Cohesion score 0.05174825174825175 - nodes in this community are weakly interconnected._