-- This file should undo anything in `up.sql`
DROP INDEX idx_clients_public_id_hub_id;
DROP INDEX idx_clients_id_hub_id;
ALTER TABLE tasks DROP COLUMN client_id;
DROP TABLE clients;