-- Tracks may exist before their file is downloaded (`path` NULL), a YouTube id
-- identifies at most one track, and `match_key` (normalized artist + title, filled
-- by the app) finds the same song coming from another service.
-- Runs with foreign keys off (see `Db::init`), so dropping `tracks` keeps
-- `playlist_tracks`.

-- Merge tracks sharing a YouTube id onto the oldest one.
CREATE TEMP TABLE duplicate_tracks AS
SELECT t.id, (SELECT MIN(o.id) FROM tracks o WHERE o.youtube_id = t.youtube_id) AS keep
FROM tracks t
WHERE t.youtube_id IS NOT NULL;
DELETE FROM duplicate_tracks WHERE id = keep;

INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
SELECT pt.playlist_id, d.keep, pt.position
FROM playlist_tracks pt JOIN duplicate_tracks d ON d.id = pt.track_id;
DELETE FROM playlist_tracks WHERE track_id IN (SELECT id FROM duplicate_tracks);
UPDATE plays SET track_id = (SELECT keep FROM duplicate_tracks d WHERE d.id = plays.track_id)
WHERE track_id IN (SELECT id FROM duplicate_tracks);

CREATE TABLE tracks_new (
    id         INTEGER PRIMARY KEY,
    path       TEXT    UNIQUE,
    title      TEXT    NOT NULL,
    artist     TEXT,
    album      TEXT,
    duration   REAL    NOT NULL DEFAULT 0,
    youtube_id TEXT    UNIQUE,
    match_key  TEXT,
    added_at   INTEGER NOT NULL
);
INSERT INTO tracks_new (id, path, title, artist, album, duration, youtube_id, added_at)
SELECT id, path, title, artist, album, duration, youtube_id, added_at
FROM tracks
WHERE id NOT IN (SELECT id FROM duplicate_tracks);
DROP TABLE duplicate_tracks;

DROP TABLE tracks;
ALTER TABLE tracks_new RENAME TO tracks;
CREATE INDEX tracks_match_key ON tracks (match_key);
