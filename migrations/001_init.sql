-- ROOM schema. SQLite, WAL, foreign keys on.

CREATE TABLE IF NOT EXISTS users (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  name          TEXT    NOT NULL,
  handle        TEXT    NOT NULL UNIQUE,
  avatar        TEXT    NOT NULL DEFAULT '',
  color         TEXT    NOT NULL DEFAULT '#5333f4',
  role          TEXT    NOT NULL DEFAULT 'member',      -- owner | admin | member
  state         TEXT    NOT NULL DEFAULT 'active',     -- active | suspended
  bio           TEXT    NOT NULL DEFAULT '',
  created_at    INTEGER NOT NULL,
  last_seen_at  INTEGER NOT NULL DEFAULT 0,
  presence      TEXT    NOT NULL DEFAULT 'offline'     -- online | away | offline
);

CREATE TABLE IF NOT EXISTS devices (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name          TEXT    NOT NULL,
  platform      TEXT    NOT NULL DEFAULT '',
  token_hash    TEXT    NOT NULL UNIQUE,
  fingerprint   TEXT    NOT NULL DEFAULT '',
  ip            TEXT    NOT NULL DEFAULT '',
  approved      INTEGER NOT NULL DEFAULT 0,
  revoked       INTEGER NOT NULL DEFAULT 0,
  requested_at  INTEGER NOT NULL,
  approved_at   INTEGER NOT NULL DEFAULT 0,
  last_seen_at  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user_id);

CREATE TABLE IF NOT EXISTS conversations (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  kind            TEXT    NOT NULL DEFAULT 'direct',    -- direct | group
  name            TEXT    NOT NULL DEFAULT '',
  topic           TEXT    NOT NULL DEFAULT '',
  color           TEXT    NOT NULL DEFAULT '#5333f4',
  created_by      INTEGER REFERENCES users(id) ON DELETE SET NULL,
  created_at      INTEGER NOT NULL,
  last_message_at INTEGER NOT NULL DEFAULT 0,
  last_preview    TEXT    NOT NULL DEFAULT '',
  last_sender     TEXT    NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_conv_last ON conversations(last_message_at DESC);

CREATE TABLE IF NOT EXISTS conversation_members (
  conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role            TEXT    NOT NULL DEFAULT 'member',
  joined_at       INTEGER NOT NULL,
  last_read_id    INTEGER NOT NULL DEFAULT 0,
  muted           INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (conversation_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_members_user ON conversation_members(user_id);

CREATE TABLE IF NOT EXISTS messages (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  sender_id       INTEGER REFERENCES users(id) ON DELETE SET NULL,
  body            TEXT    NOT NULL DEFAULT '',
  reply_to_id     INTEGER NOT NULL DEFAULT 0,
  edited_at       INTEGER NOT NULL DEFAULT 0,
  deleted_at      INTEGER NOT NULL DEFAULT 0,
  created_at      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, id);
CREATE INDEX IF NOT EXISTS idx_messages_time ON messages(created_at);

CREATE TABLE IF NOT EXISTS reactions (
  message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  emoji      TEXT    NOT NULL,
  PRIMARY KEY (message_id, user_id, emoji)
);
CREATE INDEX IF NOT EXISTS idx_reactions_msg ON reactions(message_id);

CREATE TABLE IF NOT EXISTS files (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_id   INTEGER NOT NULL DEFAULT 0,              -- 0 = root of the drive
  owner_id    INTEGER REFERENCES users(id) ON DELETE SET NULL,
  name        TEXT    NOT NULL,
  mime        TEXT    NOT NULL DEFAULT 'application/octet-stream',
  size        INTEGER NOT NULL DEFAULT 0,
  storage_key TEXT    NOT NULL,
  kind        TEXT    NOT NULL DEFAULT 'file',         -- file | folder
  trashed     INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_files_parent ON files(parent_id, kind, name);
CREATE INDEX IF NOT EXISTS idx_files_owner ON files(owner_id);

CREATE TABLE IF NOT EXISTS file_shares (
  file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  permission  TEXT    NOT NULL DEFAULT 'read',         -- read | write
  PRIMARY KEY (file_id, user_id)
);

CREATE TABLE IF NOT EXISTS calls (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  kind            TEXT    NOT NULL DEFAULT 'video',    -- audio | video
  conversation_id INTEGER REFERENCES conversations(id) ON DELETE SET NULL,
  started_by      INTEGER REFERENCES users(id) ON DELETE SET NULL,
  status          TEXT    NOT NULL DEFAULT 'active',   -- active | ended
  started_at      INTEGER NOT NULL,
  ended_at        INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS call_participants (
  call_id  INTEGER NOT NULL REFERENCES calls(id) ON DELETE CASCADE,
  user_id  INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  joined_at INTEGER NOT NULL,
  left_at   INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (call_id, user_id)
);

CREATE TABLE IF NOT EXISTS audit_log (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  actor_id   INTEGER NOT NULL DEFAULT 0,
  actor_name TEXT    NOT NULL DEFAULT '',
  action     TEXT    NOT NULL,
  entity     TEXT    NOT NULL DEFAULT '',
  entity_id  INTEGER NOT NULL DEFAULT 0,
  detail     TEXT    NOT NULL DEFAULT '',
  ip         TEXT    NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_log(created_at DESC);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS backups (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT    NOT NULL,
  size       INTEGER NOT NULL DEFAULT 0,
  path       TEXT    NOT NULL,
  created_at INTEGER NOT NULL
);
