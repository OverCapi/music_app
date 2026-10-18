-- Favorite YouTube channels of the Lives page. `id` is the channel id (`UC…`); the
-- name and picture are kept as displayed when the channel was added.
CREATE TABLE live_channels (
    id        TEXT    PRIMARY KEY,
    name      TEXT    NOT NULL,
    handle    TEXT,
    thumbnail TEXT    NOT NULL,
    followers INTEGER,
    added_at  INTEGER NOT NULL
);
