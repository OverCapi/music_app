-- Local playlist filled from a playlist of another service (YouTube Music for now).
-- `external_id` is the service's playlist id ('LM' for YouTube Music liked songs).
-- Deleting the local playlist forgets the link: the next sync recreates it.
CREATE TABLE sync_playlists (
    service     TEXT    NOT NULL,
    external_id TEXT    NOT NULL,
    playlist_id INTEGER NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    PRIMARY KEY (service, external_id)
);
