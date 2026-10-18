-- Favorite tracks keep their audio file when the storage optimizer runs.
ALTER TABLE tracks ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0;
