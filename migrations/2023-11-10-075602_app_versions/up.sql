-- Your SQL goes here
CREATE TABLE app_versions (
    hash TEXT PRIMARY KEY NOT NULL,
    app_id BIGINT NOT NULL,


    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
