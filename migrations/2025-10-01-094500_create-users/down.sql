-- Roll back the user table setup.
DROP INDEX IF EXISTS users_hub_idx;
DROP INDEX IF EXISTS users_email_per_hub_idx;
DROP TABLE IF EXISTS users;
