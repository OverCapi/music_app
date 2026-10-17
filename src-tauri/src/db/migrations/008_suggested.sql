-- Suggested tracks: recommended by the app, not kept by the user. They are rows of
-- `tracks` (the play queue works on track ids) hidden from the library; liking one
-- clears the flag, otherwise the row is deleted later (plays keep their copy of the
-- title and artist).
ALTER TABLE tracks ADD COLUMN suggested INTEGER NOT NULL DEFAULT 0;
CREATE INDEX tracks_suggested ON tracks (suggested) WHERE suggested = 1;
