-- Artists followed on another service (imported from the Spotify export). The app
-- has no artist pages yet: this is only a list, replaced by each import.
CREATE TABLE followed_artists (
    service TEXT NOT NULL,
    name    TEXT NOT NULL,
    PRIMARY KEY (service, name)
);
