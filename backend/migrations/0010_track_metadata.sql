-- Track metadata enrichment: release dates, album details, artist links and
-- broad genre tags. The official ranking fields are untouched.

ALTER TABLE tracks
    ADD COLUMN release_date TEXT,
    ADD COLUMN release_date_precision TEXT,
    ADD COLUMN release_year INTEGER,
    ADD COLUMN decade INTEGER,
    ADD COLUMN album_id TEXT,
    ADD COLUMN album_type TEXT,
    ADD COLUMN total_tracks INTEGER,
    ADD COLUMN disc_number INTEGER,
    ADD COLUMN track_number INTEGER,
    ADD COLUMN primary_genre TEXT;

CREATE INDEX tracks_playlist_decade_idx ON tracks (playlist_id, decade);
CREATE INDEX tracks_playlist_genre_idx ON tracks (playlist_id, primary_genre);
CREATE INDEX tracks_playlist_album_idx ON tracks (playlist_id, album_id);

CREATE TABLE artists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    spotify_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    genres TEXT[] NOT NULL DEFAULT '{}',
    primary_genre TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE track_artists (
    track_id UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    artist_id UUID NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    PRIMARY KEY (track_id, artist_id)
);

CREATE INDEX track_artists_artist_idx ON track_artists (artist_id);

CREATE TABLE metadata_backfill_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'completed', 'failed')),
    total_tracks INTEGER NOT NULL DEFAULT 0,
    processed_tracks INTEGER NOT NULL DEFAULT 0,
    total_artists INTEGER NOT NULL DEFAULT 0,
    processed_artists INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    error TEXT
);

CREATE INDEX metadata_backfill_runs_playlist_started_idx
    ON metadata_backfill_runs (playlist_id, started_at DESC);

COMMENT ON TABLE metadata_backfill_runs IS
    'Progress for the Spotify metadata/artist-genre backfill.';
