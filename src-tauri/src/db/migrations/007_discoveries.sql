-- "Découvertes de la semaine": the tracks proposed for a week, frozen until the next
-- one. `week_start` is the unix timestamp of the week's start (Monday 00:00 UTC);
-- `position` keeps the order. Only the current and the previous week are kept.
CREATE TABLE discoveries (
    week_start INTEGER NOT NULL,
    position   INTEGER NOT NULL,
    youtube_id TEXT    NOT NULL,
    title      TEXT    NOT NULL,
    artist     TEXT,
    duration   REAL    NOT NULL DEFAULT 0,
    PRIMARY KEY (week_start, position)
);
