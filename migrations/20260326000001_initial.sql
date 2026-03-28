CREATE TABLE users (
    id            TEXT PRIMARY KEY NOT NULL,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE rooms (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    created_by  TEXT NOT NULL REFERENCES users(id),
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE messages (
    id          TEXT PRIMARY KEY NOT NULL,
    room_id     TEXT NOT NULL REFERENCES rooms(id),
    user_id     TEXT NOT NULL REFERENCES users(id),
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_messages_room_id ON messages(room_id);
CREATE INDEX idx_messages_created_at ON messages(created_at);
