-- down.sql
DROP TABLE decks;

DROP TRIGGER IF EXISTS update_decks_modtime ON decks;

DROP FUNCTION IF EXISTS update_decks_modified_column;
