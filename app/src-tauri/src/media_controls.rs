//! macOS "Now Playing": registers the app with the system media center so AirPods,
//! media keys and the control center treat it as a media app. rodio output alone
//! is not enough. Playback commands are forwarded to [`AudioHandle`].

use std::sync::Mutex;
use std::time::Duration;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use tauri::{AppHandle, Manager};

use crate::audio::{AudioHandle, PlayerState};
use crate::library::{Library, TrackId};

/// Managed state: the controls and the last (track, live id, playing) pushed.
type Key = (Option<TrackId>, Option<String>, bool);
struct NowPlaying(Mutex<(MediaControls, Key)>);

pub fn init(app: &AppHandle) {
    let mut controls = match MediaControls::new(PlatformConfig {
        dbus_name: "music_app",
        display_name: "Music App",
        hwnd: None,
    }) {
        Ok(c) => c,
        Err(e) => return tracing::warn!("media controls unavailable: {e:?}"),
    };
    let handle = app.clone();
    if let Err(e) = controls.attach(move |event| on_event(&handle, event)) {
        return tracing::warn!("media controls attach failed: {e:?}");
    }
    app.manage(NowPlaying(Mutex::new((controls, (None, None, false)))));
}

/// Called by the engine on every player state change; resends the system metadata
/// only when the track, live or play/pause state changed.
pub fn sync(app: &AppHandle, state: &PlayerState) {
    let Some(now) = app.try_state::<NowPlaying>() else {
        return;
    };
    let key = (
        state.track_id,
        state.live.as_ref().map(|l| l.id.clone()),
        state.playing,
    );
    let mut guard = now.0.lock().unwrap();
    if guard.1 != key {
        guard.1 = key;
        update(app, &mut guard.0, state);
    }
}

fn on_event(app: &AppHandle, event: MediaControlEvent) {
    let audio = app.state::<AudioHandle>();
    let playing = audio.state().playing;
    let result = match event {
        MediaControlEvent::Play => audio.play(),
        MediaControlEvent::Pause => audio.pause(),
        MediaControlEvent::Toggle if playing => audio.pause(),
        MediaControlEvent::Toggle => audio.play(),
        MediaControlEvent::Next => audio.next(),
        MediaControlEvent::Previous => audio.previous(),
        MediaControlEvent::SetPosition(MediaPosition(p)) => audio.seek(p.as_secs_f64()),
        _ => Ok(()),
    };
    if let Err(e) = result {
        tracing::debug!("media control ignored: {e}");
    }
}

fn update(app: &AppHandle, controls: &mut MediaControls, state: &PlayerState) {
    let progress = Some(MediaPosition(Duration::from_secs_f64(
        state.position.max(0.0),
    )));
    let playback = if !state.loaded && !state.loading {
        MediaPlayback::Stopped
    } else if state.playing {
        MediaPlayback::Playing { progress }
    } else {
        MediaPlayback::Paused { progress }
    };
    let track = state
        .track_id
        .and_then(|id| app.try_state::<Library>()?.get(id).ok());
    let duration = (state.duration > 0.0).then(|| Duration::from_secs_f64(state.duration));
    let metadata = if let Some(t) = &track {
        MediaMetadata {
            title: Some(&t.title),
            artist: t.artist.as_deref(),
            album: t.album.as_deref(),
            duration,
            cover_url: None,
        }
    } else if let Some(live) = &state.live {
        MediaMetadata {
            title: Some(&live.title),
            artist: live.channel.as_deref(),
            ..Default::default()
        }
    } else {
        MediaMetadata {
            title: Some("Écoute en groupe"),
            ..Default::default()
        }
    };
    if let Err(e) = controls.set_metadata(metadata) {
        tracing::debug!("set_metadata failed: {e:?}");
    }
    if let Err(e) = controls.set_playback(playback) {
        tracing::debug!("set_playback failed: {e:?}");
    }
}
