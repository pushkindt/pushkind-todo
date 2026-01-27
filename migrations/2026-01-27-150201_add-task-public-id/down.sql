-- This file should undo anything in `up.sql`
DROP INDEX tasks_hub_id_public_id_idx;
ALTER TABLE tasks DROP COLUMN public_id;
