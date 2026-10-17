-- Explicit feedback on what the app suggests: a like or a "never suggest again" on
-- one track (`youtube_id`, unique in `tracks`) or on one artist (`artist`, normalized:
-- trimmed + lowercase). Exactly one of the two is set. `label` is the title / artist
-- name as displayed, kept so the feedback survives the track leaving the library.
CREATE TABLE feedback (
    id         INTEGER PRIMARY KEY,
    kind       TEXT    NOT NULL CHECK (kind IN ('like', 'dislike')),
    youtube_id TEXT,
    artist     TEXT,
    label      TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    CHECK ((youtube_id IS NULL) <> (artist IS NULL))
);
CREATE UNIQUE INDEX feedback_track ON feedback (youtube_id) WHERE youtube_id IS NOT NULL;
CREATE UNIQUE INDEX feedback_artist ON feedback (artist) WHERE artist IS NOT NULL;
