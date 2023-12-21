-- down.sql
DROP TABLE cards;

DROP TRIGGER IF EXISTS update_cards_modtime ON cards;

DROP FUNCTION IF EXISTS update_cards_modified_column;
