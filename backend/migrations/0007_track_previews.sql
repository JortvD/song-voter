-- Spotify's 30-second preview MP3 for each track, when available.
-- The embed player shows a "Use the Spotify app" prompt we cannot control;
-- native previews give a clean in-page listening experience instead.

ALTER TABLE tracks ADD COLUMN preview_url TEXT;
