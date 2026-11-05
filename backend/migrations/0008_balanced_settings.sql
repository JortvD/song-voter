-- Phase 9: tuning constants for the volume-adjusted ("balanced") ranking.
-- The table is a singleton: one row controls every playlist.

CREATE TABLE balanced_settings (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    influence_exponent DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    confidence_prior DOUBLE PRECISION NOT NULL DEFAULT 10.0,
    neutral_prior DOUBLE PRECISION NOT NULL DEFAULT 20.0,
    min_picks INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (id IS TRUE),
    CHECK (influence_exponent >= 0.0 AND influence_exponent <= 2.0),
    CHECK (confidence_prior >= 0.0 AND confidence_prior <= 500.0),
    CHECK (neutral_prior >= 0.0 AND neutral_prior <= 500.0),
    CHECK (min_picks >= 0 AND min_picks <= 10000)
);

INSERT INTO balanced_settings (id) VALUES (TRUE) ON CONFLICT (id) DO NOTHING;

COMMENT ON TABLE balanced_settings IS
    'Tuning constants for the balanced ranking: square-root influence, confidence shrinkage and a neutral prior.';
