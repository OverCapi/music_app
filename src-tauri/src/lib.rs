//! App backend. Each feature lives in its own module (`mod.rs` = logic, testable
//! without Tauri; `commands.rs` = thin Tauri commands). To add a feature: create the
//! module, manage its state in `setup`, list its commands/events in [`specta_builder`].
//!
//! TypeScript bindings (`src/lib/bindings.ts`) are generated from the commands and
//! events below: at startup in debug builds, or with `cargo test -p music_app export_bindings`.

mod account;
mod audio;
mod db;
mod discoveries;
mod downloads;
mod error;
mod feedback;
mod group;
mod history;
mod library;
mod mixes;
mod playlists;
mod radio;
mod search;
mod sync;
mod taste;
mod ytdlp;

use std::path::PathBuf;

use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{Builder, ErrorHandlingMode, collect_commands, collect_events};

use account::AccountStore;
use audio::AudioHandle;
use db::Db;
use downloads::Downloads;
use feedback::Feedback;
use group::GroupClient;
use history::History;
use library::Library;
use playlists::Playlists;
use search::Search;
use sync::LibrarySync;

const BINDINGS_PATH: &str = "../src/lib/bindings.ts";

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .error_handling(ErrorHandlingMode::Throw)
        // Ids and timestamps stay far below 2^53.
        .dangerously_cast_bigints_to_number()
        .commands(collect_commands![
            audio::commands::play_queue,
            audio::commands::enqueue,
            audio::commands::play_index,
            audio::commands::remove_from_queue,
            audio::commands::clear_queue,
            audio::commands::get_queue,
            audio::commands::get_player_state,
            audio::commands::play,
            audio::commands::pause,
            audio::commands::stop,
            audio::commands::next,
            audio::commands::previous,
            audio::commands::seek,
            audio::commands::set_volume,
            audio::commands::set_shuffle,
            audio::commands::set_repeat,
            library::commands::get_library,
            library::commands::add_tracks,
            library::commands::remove_track,
            playlists::commands::get_playlists,
            playlists::commands::create_playlist,
            playlists::commands::rename_playlist,
            playlists::commands::delete_playlist,
            playlists::commands::add_to_playlist,
            playlists::commands::remove_from_playlist,
            search::commands::search_youtube,
            downloads::commands::download_track,
            downloads::commands::cancel_download,
            account::commands::get_account,
            account::commands::set_server_url,
            account::commands::register,
            account::commands::login,
            account::commands::logout,
            group::commands::get_group,
            group::commands::group_create,
            group::commands::group_join,
            group::commands::group_leave,
            group::commands::group_add,
            group::commands::group_remove,
            group::commands::group_play_entry,
            group::commands::group_play,
            group::commands::group_pause,
            group::commands::group_seek,
            group::commands::group_skip,
            history::commands::get_recent_plays,
            feedback::commands::get_feedback,
            feedback::commands::set_feedback,
            feedback::commands::clear_feedback,
            taste::commands::get_taste_profile,
            radio::commands::get_radio,
            mixes::commands::get_mixes,
            discoveries::commands::get_discoveries,
            discoveries::commands::refresh_discoveries,
            sync::commands::get_sync_settings,
            sync::commands::set_ytmusic_browser,
            sync::commands::sync_youtube_music,
        ])
        .events(collect_events![
            audio::PlayerChanged,
            audio::QueueChanged,
            library::LibraryChanged,
            playlists::PlaylistsChanged,
            search::SearchDetails,
            downloads::DownloadProgress,
            group::GroupChanged,
            group::GroupError,
            feedback::FeedbackChanged,
            sync::SyncProgress,
        ])
        .typ::<error::AppError>()
}

fn export_bindings(builder: &Builder<tauri::Wry>) {
    builder
        .export(Typescript::default(), BINDINGS_PATH)
        .expect("failed to export TypeScript bindings");
}

/// `MUSIC_APP_PROFILE=<name>` gives this instance its own data and cache dirs
/// (database, group temp files), e.g. to run two clients side by side on one
/// machine. The downloaded music folder stays shared.
/// Downloads library tracks that have no file yet when the audio engine needs them.
/// `Downloads` and `Library` are looked up at call time (managed after the engine).
fn track_fetcher(app: tauri::AppHandle) -> audio::TrackFetcher {
    std::sync::Arc::new(move |_track_id, youtube_id, done| {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let fetch = match (app.try_state::<Downloads>(), app.try_state::<Library>()) {
                (Some(downloads), Some(library)) => {
                    downloads.fetch(app.clone(), library.inner().clone(), youtube_id)
                }
                _ => return done(Err("L'application démarre encore".into())),
            };
            done(fetch.await.map_err(|e| e.to_string()));
        });
    })
}

fn profile_dirs(app: &tauri::App) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let cache_dir = app.path().app_cache_dir()?;
    let Some(profile) = std::env::var_os("MUSIC_APP_PROFILE") else {
        return Ok((data_dir, cache_dir));
    };
    let profile = profile.to_string_lossy();
    let valid = !profile.is_empty()
        && profile
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if !valid {
        return Err(format!(
            "MUSIC_APP_PROFILE invalide ({profile:?}) : lettres, chiffres, _ et - uniquement"
        )
        .into());
    }
    Ok((
        data_dir.join("profiles").join(&*profile),
        cache_dir.join("profiles").join(&*profile),
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,music_app_lib=debug".into()),
        )
        .init();

    let builder = specta_builder();
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        // `cover://localhost/<track id>`: embedded cover art of a library track.
        .register_asynchronous_uri_scheme_protocol("cover", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let id = request.uri().path().trim_start_matches('/').parse::<i64>();
            tauri::async_runtime::spawn_blocking(move || {
                let cover = id
                    .ok()
                    .and_then(|id| app.state::<Library>().cover(id).ok().flatten());
                let response = match cover {
                    Some((bytes, mime)) => tauri::http::Response::builder()
                        .header("Content-Type", mime)
                        .header("Cache-Control", "max-age=3600")
                        .body(bytes),
                    None => tauri::http::Response::builder()
                        .status(404)
                        .body(Vec::new()),
                };
                responder.respond(response.expect("static response parts are valid"));
            });
        })
        .setup(move |app| {
            builder.mount_events(app);
            let handle = app.handle().clone();
            let (data_dir, cache_dir) = profile_dirs(app)?;

            let db = Db::open(&data_dir.join("music.db"), &data_dir)?;
            let library = Library::new(db.clone(), Some(handle.clone()));
            let history = History::new(db.clone());
            let feedback = Feedback::new(db.clone(), Some(handle.clone()));
            app.manage(feedback.clone());
            let radio = radio::Radio::new(db.clone());
            app.manage(radio.clone());
            app.manage(AudioHandle::spawn(
                handle.clone(),
                library.random_picker(),
                history.recorder(),
                track_fetcher(handle.clone()),
            ));
            let taste = taste::Taste::new(history.clone(), feedback.clone());
            let mixes = mixes::Mixes::new(
                taste.clone(),
                radio,
                library.clone(),
                feedback,
                history.clone(),
            );
            app.manage(discoveries::Discoveries::new(db.clone(), mixes.clone()));
            app.manage(mixes);
            app.manage(taste);
            app.manage(history);
            let playlists = Playlists::new(db.clone(), Some(handle.clone()));
            app.manage(LibrarySync::new(
                db.clone(),
                library.clone(),
                playlists.clone(),
                Some(handle.clone()),
            ));
            app.manage(playlists);
            app.manage(AccountStore::new(db));
            app.manage(GroupClient::new(handle, cache_dir.join("groups")));
            let music_dir = app
                .path()
                .audio_dir()
                .map(|dir| dir.join("Music App"))
                .or_else(|_| app.path().app_data_dir().map(|dir| dir.join("music")))?;
            app.manage(Downloads::new(music_dir));
            app.manage(Search::default());
            app.manage(library);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Group temp files never outlive the app.
                if let Some(group) = app.try_state::<GroupClient>() {
                    group.shutdown();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    /// Regenerates `src/lib/bindings.ts` without launching the app.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
