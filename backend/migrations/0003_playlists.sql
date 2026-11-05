-- Phase 2: playlists imported from Spotify and their tracks.
-- Rating columns already exist so phase 3 can start voting without another schema change.

CREATE TABLE playlists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    spotify_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    image_url TEXT,
    owner_name TEXT,
    imported_by UUID REFERENCES users(id) ON DELETE SET NULL,
    imported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reveal_state TEXT NOT NULL DEFAULT 'hidden'
        CHECK (reveal_state IN ('hidden', 'revealed')),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- At most one active playlist at a time.
CREATE UNIQUE INDEX playlists_one_active_idx ON playlists (is_active) WHERE is_active;

CREATE TABLE tracks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    spotify_id TEXT NOT NULL,
    title TEXT NOT NULL,
    artists TEXT NOT NULL,
    album TEXT,
    album_image_url TEXT,
    duration_ms INTEGER,
    position INTEGER NOT NULL DEFAULT 0,
    rating DOUBLE PRECISION NOT NULL DEFAULT 1000,
    comparisons INTEGER NOT NULL DEFAULT 0,
    wins INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (playlist_id, spotify_id)
);

CREATE INDEX tracks_playlist_position_idx ON tracks (playlist_id, position);
CREATE INDEX tracks_playlist_rating_idx ON tracks (playlist_id, rating DESC);
