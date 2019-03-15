DROP TABLE songs;

CREATE TABLE songs (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    track_id VARCHAR NOT NULL,
    added_at TIMESTAMP NOT NULL,
    user VARCHAR
);

CREATE INDEX songs_deleted_added_at ON songs (deleted, added_at);