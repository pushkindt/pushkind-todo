-- Your SQL goes here
CREATE TABLE clients (
    id INTEGER NOT NULL PRIMARY KEY,
    public_id TEXT NOT NULL,
    hub_id INTEGER NOT NULL,
    name VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
ALTER TABLE tasks ADD COLUMN client_id INTEGER
  REFERENCES clients(id) ON DELETE SET NULL;
CREATE INDEX idx_clients_public_id_hub_id ON clients(public_id, hub_id);
CREATE INDEX idx_clients_id_hub_id ON clients(id, hub_id);
