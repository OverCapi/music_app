-- Cache of the tracks YouTube suggests around a seed track (its "radio"), so yt-dlp
-- is not run again for a few days. One row per suggested track, `position` keeps
-- YouTube's order; every row of a seed shares the same `fetched_at`.
CREATE TABLE radio_cache (
    seed_id    TEXT    NOT NULL,
    position   INTEGER NOT NULL,
    youtube_id TEXT    NOT NULL,
    title      TEXT    NOT NULL,
    artist     TEXT,
    duration   REAL    NOT NULL DEFAULT 0,
    fetched_at INTEGER NOT NULL,
    PRIMARY KEY (seed_id, position)
);
CREATE INDEX radio_cache_fetched_at ON radio_cache (fetched_at);
