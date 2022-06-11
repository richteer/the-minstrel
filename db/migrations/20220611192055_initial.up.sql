PRAGMA foreign_keys = 1;

-- TODO: probably NOT NULL the columns?
CREATE TABLE IF NOT EXISTS user (
    id INTEGER PRIMARY KEY,
    displayname TEXT,
    icon TEXT
);

CREATE TABLE IF NOT EXISTS user_auth (
    id INTEGER PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    user_id INTEGER NOT NULL REFERENCES user(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS discord_user (
    id INTEGER PRIMARY KEY,
    discord_id INTEGER UNIQUE NOT NULL, -- technically u64
    user_id INTEGER NOT NULL REFERENCES user(id) ON DELETE CASCADE
);

-- Playlist, directory, etc.
-- Probably can work as a single song?
CREATE TABLE IF NOT EXISTS source (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    active INTEGER NOT NULL,      -- boolean, is the source active in the song pooling.
    source_type INTEGER NOT NULL, -- enum, youtube playlist, m3u, etc
    user_id INTEGER NOT NULL REFERENCES user(id) ON DELETE CASCADE
);
