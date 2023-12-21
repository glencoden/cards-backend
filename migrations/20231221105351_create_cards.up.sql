-- up.sql
CREATE TABLE cards (
    id               SERIAL PRIMARY KEY,
    deck_id          INTEGER REFERENCES decks (id),
    related_card_ids INT[],
    example_text     VARCHAR(255),
    audio_url        VARCHAR(255),
    seen_at          TIMESTAMP WITHOUT TIME ZONE,
    seen_for         INTEGER,
    rating           INTEGER,
    prev_rating      INTEGER,
    created_at       TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
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
