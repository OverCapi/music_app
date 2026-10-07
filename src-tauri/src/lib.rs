mod account;
mod audio;
mod downloads;
mod group;
mod library;
mod opus;
mod playlists;
mod storage;
mod youtube;

use std::path::PathBuf;
use std::sync::Arc;

use account::{Account, AccountStore};
use audio::{AudioHandle, PlayerState, QueueState};
use downloads::Downloads;
use group::{GroupClient, GroupView, Intent};
use group_protocol::{is_valid_youtube_id, parse_invite, ClientMessage};
use library::{Library, Track};
use playlists::{Playlist, Playlists};
use tauri::{AppHandle, Manager, State};
use youtube::{SearchResponse, SearchSource, YoutubeSearch};

#[tauri::command]
fn play_queue(
    audio: State<AudioHandle>,
    tracks: Vec<String>,
    start: usize,
) -> Result<QueueState, String> {
    audio.play_queue(tracks, start)
}

#[tauri::command]
fn enqueue(audio: State<AudioHandle>, tracks: Vec<String>) -> Result<QueueState, String> {
    audio.enqueue(tracks)
}

#[tauri::command]
fn play_index(audio: State<AudioHandle>, index: usize) -> Result<(), String> {
    audio.play_index(index)
}

#[tauri::command]
fn remove_from_queue(audio: State<AudioHandle>, index: usize) -> Result<QueueState, String> {
    audio.remove_from_queue(index)
}

#[tauri::command]
fn clear_queue(audio: State<AudioHandle>) -> Result<QueueState, String> {
    audio.clear_queue()
}

#[tauri::command]
fn get_queue(audio: State<AudioHandle>) -> QueueState {
    audio.queue()
}

#[tauri::command]
fn next(audio: State<AudioHandle>) -> Result<(), String> {
    audio.next()
}

#[tauri::command]
fn previous(audio: State<AudioHandle>) -> Result<(), String> {
    audio.previous()
}

#[tauri::command]
fn play(audio: State<AudioHandle>) -> Result<(), String> {
    audio.play()
}

#[tauri::command]
fn pause(audio: State<AudioHandle>) -> Result<(), String> {
    audio.pause()
}

#[tauri::command]
fn stop(audio: State<AudioHandle>) -> Result<(), String> {
    audio.stop()
}

#[tauri::command]
fn seek(audio: State<AudioHandle>, secs: f64) -> Result<(), String> {
    audio.seek(secs)
}

#[tauri::command]
fn set_volume(audio: State<AudioHandle>, volume: f32) -> Result<(), String> {
    audio.set_volume(volume)
}

#[tauri::command]
fn get_state(audio: State<AudioHandle>) -> PlayerState {
    audio.state()
}

#[tauri::command]
fn get_library(library: State<Library>) -> Vec<Track> {
    library.tracks()
}

// Async so reading tags of many files doesn't block the main thread.
#[tauri::command]
async fn add_tracks(library: State<'_, Library>, paths: Vec<String>) -> Result<Vec<Track>, String> {
    library.add(&paths)
}

#[tauri::command]
fn remove_track(
    library: State<Library>,
    playlists: State<Playlists>,
    downloads: State<Downloads>,
    path: String,
) -> Result<Vec<Track>, String> {
    // Trash the file first: if that fails, the track stays in the library.
    downloads.trash_if_owned(&path)?;
    let tracks = library.remove(&path)?;
    playlists.forget_track(&path)?;
    Ok(tracks)
}

#[tauri::command]
fn get_playlists(playlists: State<Playlists>) -> Vec<Playlist> {
    playlists.all()
}

#[derive(serde::Serialize)]
struct CreatedPlaylist {
    playlist: Playlist,
    playlists: Vec<Playlist>,
}

#[tauri::command]
fn create_playlist(playlists: State<Playlists>, name: String) -> Result<CreatedPlaylist, String> {
    let (playlist, playlists) = playlists.create(&name)?;
    Ok(CreatedPlaylist {
        playlist,
        playlists,
    })
}

#[tauri::command]
fn rename_playlist(
    playlists: State<Playlists>,
    id: String,
    name: String,
) -> Result<Vec<Playlist>, String> {
    playlists.rename(&id, &name)
}

#[tauri::command]
fn delete_playlist(playlists: State<Playlists>, id: String) -> Result<Vec<Playlist>, String> {
    playlists.delete(&id)
}

#[tauri::command]
fn add_to_playlist(
    playlists: State<Playlists>,
    id: String,
    paths: Vec<String>,
) -> Result<Vec<Playlist>, String> {
    playlists.add_tracks(&id, paths)
}

#[tauri::command]
fn remove_from_playlist(
    playlists: State<Playlists>,
    id: String,
    path: String,
) -> Result<Vec<Playlist>, String> {
    playlists.remove_track(&id, &path)
}

#[tauri::command]
async fn search_youtube(
    app: AppHandle,
    search: State<'_, YoutubeSearch>,
    query: String,
    source: SearchSource,
    offset: u32,
    search_id: Option<u64>,
) -> Result<SearchResponse, String> {
    search.search(app, &query, source, offset, search_id).await
}

#[tauri::command]
fn download_track(
    app: AppHandle,
    downloads: State<Downloads>,
    id: String,
    url: String,
) -> Result<(), String> {
    downloads.start(app, id, url)
}

#[tauri::command]
fn cancel_download(app: AppHandle, downloads: State<Downloads>, id: String) {
    downloads.cancel(&app, &id);
}

#[tauri::command]
fn get_account(account: State<AccountStore>) -> Account {
    account.account()
}

#[tauri::command]
fn set_server_url(
    account: State<AccountStore>,
    group: State<GroupClient>,
    url: String,
) -> Result<Account, String> {
    if account.set_server_url(&url)? {
        // Logged out of the old server: its group is gone too.
        group.leave();
    }
    Ok(account.account())
}

#[tauri::command]
async fn register(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
    username: String,
    password: String,
) -> Result<Account, String> {
    let result = account.register(&username, &password).await?;
    // The group belonged to the previous account, if any.
    group.leave();
    Ok(result)
}

#[tauri::command]
async fn login(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
    username: String,
    password: String,
) -> Result<Account, String> {
    let result = account.login(&username, &password).await?;
    group.leave();
    Ok(result)
}

#[tauri::command]
fn logout(account: State<AccountStore>, group: State<GroupClient>) -> Result<Account, String> {
    group.leave();
    account.logout()
}

#[tauri::command]
fn get_group(group: State<GroupClient>) -> Option<GroupView> {
    group.view()
}

#[tauri::command]
async fn group_create(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
) -> Result<GroupView, String> {
    let session = account.session().ok_or("Connecte-toi d'abord")?;
    group.start(Intent::Create, session).await
}

#[tauri::command]
async fn group_join(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
    invite: String,
) -> Result<GroupView, String> {
    let code = parse_invite(&invite).ok_or("Code d'invitation invalide")?;
    let session = account.session().ok_or("Connecte-toi d'abord")?;
    group.start(Intent::Join(code), session).await
}

#[tauri::command]
fn group_leave(group: State<GroupClient>) {
    group.leave();
}

#[tauri::command]
fn group_add(
    group: State<GroupClient>,
    youtube_id: String,
    title: String,
    artist: Option<String>,
    duration: Option<f64>,
) -> Result<(), String> {
    if !is_valid_youtube_id(&youtube_id) {
        return Err("Identifiant YouTube invalide".to_owned());
    }
    group.send(ClientMessage::Add {
        youtube_id,
        title,
        artist,
        duration: duration.filter(|d| d.is_finite() && *d > 0.0),
    })
}

#[tauri::command]
fn group_remove(group: State<GroupClient>, entry_id: u64) -> Result<(), String> {
    group.send(ClientMessage::Remove { entry_id })
}

#[tauri::command]
fn group_play_entry(group: State<GroupClient>, entry_id: u64) -> Result<(), String> {
    group.send(ClientMessage::PlayEntry { entry_id })
}

#[tauri::command]
fn group_play(group: State<GroupClient>) -> Result<(), String> {
    group.send(ClientMessage::Play)
}

#[tauri::command]
fn group_pause(group: State<GroupClient>) -> Result<(), String> {
    group.send(ClientMessage::Pause)
}

#[tauri::command]
fn group_seek(group: State<GroupClient>, position: f64) -> Result<(), String> {
    if !position.is_finite() {
        return Err("Position invalide".to_owned());
    }
    group.send(ClientMessage::Seek {
        position: position.max(0.0),
    })
}

#[tauri::command]
fn group_skip(group: State<GroupClient>) -> Result<(), String> {
    group.send(ClientMessage::Skip)
}

/// `MUSIC_APP_PROFILE=<name>` gives this instance its own data and cache dirs
/// (library, playlists, account, group temp files), e.g. to run two clients side
/// by side on one machine. The downloaded music folder stays shared.
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(YoutubeSearch::default())
        .setup(|app| {
            let (data_dir, cache_dir) = profile_dirs(app)?;
            app.manage(AccountStore::load(data_dir.join("account.json")));
            app.manage(GroupClient::new(
                app.handle().clone(),
                cache_dir.join("groups"),
            ));
            app.manage(Playlists::load(data_dir.join("playlists.json")));
            let file = data_dir.join("library.json");
            let library = Library::load(file);
            app.manage(AudioHandle::spawn(Arc::new(library.random_picker())));
            app.manage(library);
            let music_dir = app
                .path()
                .audio_dir()
                .map(|dir| dir.join("Music App"))
                .or_else(|_| app.path().app_data_dir().map(|dir| dir.join("music")))?;
            app.manage(Downloads::new(music_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            play_queue,
            enqueue,
            play_index,
            remove_from_queue,
            clear_queue,
            get_queue,
            next,
            previous,
            play,
            pause,
            stop,
            seek,
            set_volume,
            get_state,
            get_library,
            add_tracks,
            remove_track,
            search_youtube,
            download_track,
            cancel_download,
            get_playlists,
            create_playlist,
            rename_playlist,
            delete_playlist,
            add_to_playlist,
            remove_from_playlist,
            get_account,
            set_server_url,
            register,
            login,
            logout,
            get_group,
            group_create,
            group_join,
            group_leave,
            group_add,
            group_remove,
            group_play_entry,
            group_play,
            group_pause,
            group_seek,
            group_skip
        ])
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
