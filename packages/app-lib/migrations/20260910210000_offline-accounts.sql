-- Offline ("cracked") accounts: no Microsoft tokens, usable only on
-- servers running with online-mode=false.
ALTER TABLE minecraft_users ADD COLUMN offline BOOLEAN NOT NULL DEFAULT FALSE;
