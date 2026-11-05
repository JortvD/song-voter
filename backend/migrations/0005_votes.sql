-- Phase 4: votes, per-vote Elo deltas and the full audit trail.
-- Ratios are stored as deltas so an undo removes exactly this vote's effect
-- without clobbering changes made by other users in the meantime.

CREATE TABLE votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_low UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    track_high UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    winner UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    rating_low_before DOUBLE PRECISION NOT NULL,
    rating_high_before DOUBLE PRECISION NOT NULL,
    delta_low DOUBLE PRECISION NOT NULL,
    delta_high DOUBLE PRECISION NOT NULL,
    k_factor DOUBLE PRECISION NOT NULL,
    comparisons_low_before INTEGER NOT NULL,
    comparisons_high_before INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (track_low < track_high),
    CHECK (winner = track_low OR winner = track_high),
    UNIQUE (user_id, playlist_id, track_low, track_high)
);

CREATE INDEX votes_user_created_idx ON votes (user_id, created_at DESC);
CREATE INDEX votes_playlist_created_idx ON votes (playlist_id, created_at DESC);
