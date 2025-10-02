-- Create table to store members managed by the service.
CREATE TABLE users (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Enforce unique email per hub to avoid duplicates.
CREATE UNIQUE INDEX users_email_per_hub_idx
    ON users (hub_id, email);

-- Maintain quick lookups by hub.
CREATE INDEX users_hub_idx
    ON users (hub_id);
