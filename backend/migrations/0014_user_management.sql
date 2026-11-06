-- User management: disabled accounts and session revocation.
--
-- `session_version` is embedded in issued JWTs. Incrementing it invalidates
-- every existing session for that user without needing a session table.
ALTER TABLE users ADD COLUMN disabled_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN session_version BIGINT NOT NULL DEFAULT 0;

CREATE INDEX users_disabled_at_idx ON users (disabled_at);
