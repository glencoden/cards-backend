-- up.sql
CREATE TABLE decks (
    id                    SERIAL PRIMARY KEY,
    user_id               INTEGER REFERENCES users (id) NOT NULL,
    from_language         VARCHAR(100)                  NOT NULL,
    to_language_primary   VARCHAR(100)                  NOT NULL,
    to_language_secondary VARCHAR(100),
    design_key            VARCHAR(100),
    seen_at               TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at            TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
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
