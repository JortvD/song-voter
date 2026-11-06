-- Scheduled starts, scheduled reveals and a global voting pause.
--
-- A playlist row may exist before its songs are fetched: `scheduled_start_at`
-- is set, `started_at` stays NULL, and the scheduler loads the tracks when the
-- start time arrives (or when an admin force-starts it).
ALTER TABLE playlists
    ADD COLUMN scheduled_start_at TIMESTAMPTZ,
    ADD COLUMN started_at TIMESTAMPTZ,
    ADD COLUMN voting_paused BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN voting_paused_at TIMESTAMPTZ,
    ADD COLUMN scheduled_reveal_at TIMESTAMPTZ,
    ADD COLUMN pause_voting_on_reveal BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN start_error TEXT,
    ADD COLUMN start_attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN last_start_attempt_at TIMESTAMPTZ,
    ADD COLUMN start_in_progress BOOLEAN NOT NULL DEFAULT FALSE;

-- Existing playlists were imported immediately.
UPDATE playlists
SET started_at = imported_at
WHERE started_at IS NULL;

CREATE INDEX playlists_scheduled_start_idx
    ON playlists (scheduled_start_at)
    WHERE started_at IS NULL;

CREATE INDEX playlists_scheduled_reveal_idx
    ON playlists (scheduled_reveal_at)
    WHERE reveal_state = 'hidden';

COMMENT ON COLUMN playlists.started_at IS
    'When the track list was actually fetched; NULL while scheduled.';
COMMENT ON COLUMN playlists.voting_paused IS
    'Global voting pause; does not affect viewing revealed standings.';
