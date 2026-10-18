-- One row per listen. Title/artist/youtube_id are copied so the history survives
-- the track being removed from the library (track_id is then set to NULL).
CREATE TABLE plays (
    id         INTEGER PRIMARY KEY,
    track_id   INTEGER REFERENCES tracks (id) ON DELETE SET NULL,
    youtube_id TEXT,
    title      TEXT    NOT NULL,
    artist     TEXT,
    source     TEXT    NOT NULL,
    started_at INTEGER NOT NULL,
    listened   REAL    NOT NULL,
    duration   REAL    NOT NULL DEFAULT 0,
    completed  INTEGER NOT NULL
);
CREATE INDEX plays_started_at ON plays (started_at);
CREATE INDEX plays_track_id ON plays (track_id);
CREATE INDEX plays_youtube_id ON plays (youtube_id);
