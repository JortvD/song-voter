-- Phase 3: matchup bookkeeping.
-- A pair is recorded when it is shown to a user, so a user never sees the same
-- pair twice. Phase 4 will reuse track_low/track_high when recording votes.

ALTER TABLE tracks ADD COLUMN last_compared_at TIMESTAMPTZ;

CREATE INDEX tracks_playlist_comparisons_idx
    ON tracks (playlist_id, comparisons, last_compared_at);

CREATE TABLE seen_pairs (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    playlist_id UUID NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_low UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    track_high UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, playlist_id, track_low, track_high),
    CHECK (track_low < track_high)
);
