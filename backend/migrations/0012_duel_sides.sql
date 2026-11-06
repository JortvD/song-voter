-- Remember which side was A and which was B when the duel was served.
-- Existing rows fall back to the normalized low/high order.
ALTER TABLE seen_pairs
    ADD COLUMN served_flip BOOLEAN NOT NULL DEFAULT FALSE;
