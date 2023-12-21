-- down.sql
DROP TABLE users;

DROP TRIGGER IF EXISTS update_users_modtime ON users;

DROP FUNCTION IF EXISTS update_users_modified_column;