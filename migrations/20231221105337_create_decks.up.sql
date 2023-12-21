-- up.sql
CREATE TABLE decks (
    id            SERIAL PRIMARY KEY,
    user_id       INTEGER REFERENCES users (id) NOT NULL,
    from_language VARCHAR(100) NOT NULL,
    to_language   VARCHAR(100) NOT NULL,
    seen_at       TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    created_at    TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at    TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE OR REPLACE FUNCTION update_decks_modified_column()
RETURNS TRIGGER AS $$
BEGIN
   NEW.updated_at = CURRENT_TIMESTAMP;
RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER update_decks_modtime
    BEFORE UPDATE
    ON decks
    FOR EACH ROW
    EXECUTE FUNCTION update_decks_modified_column();
