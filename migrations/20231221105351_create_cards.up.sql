-- up.sql
CREATE TABLE cards (
    id                SERIAL PRIMARY KEY,
    deck_id           INTEGER REFERENCES decks (id) NOT NULL,
    related_card_ids  INT[] NOT NULL DEFAULT ARRAY[]:: INT [],
    from_text         VARCHAR(100)                  NOT NULL,
    to_text_primary   VARCHAR(100)                  NOT NULL,
    to_text_secondary VARCHAR(100),
    example_text      VARCHAR(255),
    audio_url         VARCHAR(255),
    seen_at           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    seen_for          INTEGER,
    rating            INTEGER                       NOT NULL DEFAULT 0,
    prev_rating       INTEGER                       NOT NULL DEFAULT 0,
    created_at        TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at        TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE OR REPLACE FUNCTION update_cards_modified_column()
RETURNS TRIGGER AS $$
BEGIN
   NEW.updated_at = CURRENT_TIMESTAMP;
RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER update_cards_modtime
    BEFORE UPDATE
    ON cards
    FOR EACH ROW
    EXECUTE FUNCTION update_cards_modified_column();

CREATE OR REPLACE FUNCTION update_prev_rating_column()
RETURNS TRIGGER AS $$
BEGIN
   IF OLD.rating IS DISTINCT FROM NEW.rating THEN
       NEW.prev_rating = OLD.rating;
END IF;
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_cards_rating
    BEFORE UPDATE
    ON cards
    FOR EACH ROW
    WHEN (OLD.rating IS DISTINCT FROM NEW.rating)
    EXECUTE FUNCTION update_prev_rating_column();