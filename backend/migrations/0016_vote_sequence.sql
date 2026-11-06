-- Monotonic application-order sequence for vote replay.
--
-- `created_at` is stamped at transaction start, before the track locks, so it
-- can disagree with the order in which concurrent votes are actually applied.
-- `seq` is assigned at INSERT time, after the locks, and is used by undo and
-- every admin rebuild so the ledger and the recomputed standings agree.

ALTER TABLE votes ADD COLUMN seq BIGINT;

WITH ordered AS (
    SELECT id, ROW_NUMBER() OVER (ORDER BY created_at ASC, id ASC) AS row_number
    FROM votes
)
UPDATE votes
SET seq = ordered.row_number
FROM ordered
WHERE votes.id = ordered.id;

CREATE SEQUENCE votes_seq_seq;
SELECT setval('votes_seq_seq', COALESCE((SELECT MAX(seq) FROM votes), 0) + 1, false);

ALTER TABLE votes ALTER COLUMN seq SET DEFAULT nextval('votes_seq_seq');
ALTER TABLE votes ALTER COLUMN seq SET NOT NULL;
ALTER SEQUENCE votes_seq_seq OWNED BY votes.seq;

CREATE UNIQUE INDEX votes_seq_key ON votes (seq);
CREATE INDEX votes_playlist_seq_idx ON votes (playlist_id, seq);
