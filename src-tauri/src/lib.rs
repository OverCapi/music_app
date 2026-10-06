mod audio;
mod downloads;
mod library;
mod opus;
mod youtube;

use audio::{AudioHandle, PlayerState, QueueState};
use downloads::Downloads;
use library::{Library, Track};
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
fn enqueue(audio: State<AudioHandle>, tracks: Vec<String>) -> QueueState {
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
fn clear_queue(audio: State<AudioHandle>) -> QueueState {
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
fn remove_track(library: State<Library>, path: String) -> Result<Vec<Track>, String> {
    library.remove(&path)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioHandle::spawn())
        .manage(YoutubeSearch::default())
        .setup(|app| {
            let file = app.path().app_data_dir()?.join("library.json");
            app.manage(Library::load(file));
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
            cancel_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
