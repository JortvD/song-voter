-- Faster per-user duel history ordering.
CREATE INDEX seen_pairs_history_idx
    ON seen_pairs (user_id, playlist_id, seen_at DESC, track_low, track_high);
