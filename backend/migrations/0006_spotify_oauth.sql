-- Phase 2 follow-up: Spotify user authorization.
--
-- Since February 2026, Spotify Development Mode apps can only read the items
-- of playlists owned by (or collaborated on by) the authenticated user. The
-- admin therefore connects a Spotify account once via OAuth.

CREATE TABLE spotify_credentials (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    spotify_user_id TEXT,
    display_name TEXT,
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    scope TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    connected_by UUID REFERENCES users(id) ON DELETE SET NULL,
    connected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Single-use, short-lived OAuth state values for CSRF protection.
CREATE TABLE spotify_oauth_states (
    state TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX spotify_oauth_states_created_at_idx
    ON spotify_oauth_states (created_at);
