CREATE TABLE tracks (
    id         INTEGER PRIMARY KEY,
    path       TEXT    NOT NULL UNIQUE,
    title      TEXT    NOT NULL,
    artist     TEXT,
    album      TEXT,
    duration   REAL    NOT NULL DEFAULT 0,
    youtube_id TEXT,
    added_at   INTEGER NOT NULL
);
CREATE INDEX tracks_youtube_id ON tracks (youtube_id);

CREATE TABLE playlists (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE playlist_tracks (
    playlist_id INTEGER NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, track_id)
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
