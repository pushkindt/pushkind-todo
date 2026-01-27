-- Your SQL goes here
ALTER TABLE tasks ADD COLUMN public_id BLOB;
UPDATE tasks SET public_id = randomblob(16) WHERE public_id IS NULL;
CREATE UNIQUE INDEX tasks_hub_id_public_id_idx ON tasks (hub_id, public_id);
