use crate::{Card, CardForm, Deck, DeckForm, User, UserForm};
use sqlx::{query_builder::QueryBuilder, Error, Pool, Postgres};

#[derive(serde::Serialize)]
pub struct DatabaseQueryResult {
    rows_affected: u64,
}

// database queries

pub async fn read_users_query(pool: &Pool<Postgres>) -> Result<Vec<User>, Error> {
    sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(pool)
        .await
}

pub async fn read_user(pool: &Pool<Postgres>, user_id: i32) -> Result<Vec<User>, Error> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_all(pool)
        .await
}

pub async fn create_user_query(
    pool: &Pool<Postgres>,
    user_form: UserForm,
) -> Result<DatabaseQueryResult, Error> {
    if let None = user_form.name {
        return Err(Error::RowNotFound);
    }

    if let None = user_form.email {
        return Err(Error::RowNotFound);
    }

    let result = sqlx::query!(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        user_form.name,
        user_form.email,
    )
    .execute(pool)
    .await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

pub async fn update_user_query(
    pool: &Pool<Postgres>,
    user_id: i32,
    user_form: UserForm,
) -> Result<DatabaseQueryResult, Error> {
    let mut query = QueryBuilder::new("UPDATE users SET");

    let mut num_updates = 0;

    // TODO: iterate over fields and extract as helper function

    if let Some(name) = user_form.name {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" name =");
        query.push_bind(name);
        num_updates += 1;
    }

    if let Some(email) = user_form.email {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" email =");
        query.push_bind(email);
        num_updates += 1;
    }

    if num_updates == 0 {
        return Err(Error::RowNotFound);
    }

    query.push(" WHERE id =");
    query.push_bind(user_id);

    let result = query.build().execute(pool).await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

// TODO: delete all related decks and cards or implement soft delete
pub async fn delete_user_query(
    pool: &Pool<Postgres>,
    user_id: i32,
) -> Result<DatabaseQueryResult, Error> {
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

pub async fn read_decks_query(pool: &Pool<Postgres>, user_id: i32) -> Result<Vec<Deck>, Error> {
    sqlx::query_as!(Deck, "SELECT * FROM decks WHERE user_id = $1", user_id)
        .fetch_all(pool)
        .await
}

pub async fn read_deck(
    pool: &Pool<Postgres>,
    deck_id: i32,
    user_id: i32,
) -> Result<Vec<Deck>, Error> {
    sqlx::query_as!(
        Deck,
        "SELECT * FROM decks WHERE id = $1 AND user_id = $2",
        deck_id,
        user_id
    )
    .fetch_all(pool)
    .await
}

pub async fn create_deck_query(
    pool: &Pool<Postgres>,
    deck_form: DeckForm,
    user_id: i32,
) -> Result<DatabaseQueryResult, Error> {
    if let None = deck_form.from_language {
        return Err(Error::RowNotFound);
    }

    if let None = deck_form.to_language_primary {
        return Err(Error::RowNotFound);
    }

    let result = sqlx::query!(
        "INSERT INTO decks (user_id, from_language, to_language_primary, to_language_secondary, design_key) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        deck_form.from_language,
        deck_form.to_language_primary,
        deck_form.to_language_secondary,
        deck_form.design_key,
    )
        .execute(pool)
        .await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

pub async fn update_deck_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    deck_form: DeckForm,
    user_id: i32,
) -> Result<DatabaseQueryResult, Error> {
    let mut query = QueryBuilder::new("UPDATE decks SET");

    let mut num_updates = 0;

    if let Some(from_language) = deck_form.from_language {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" from_language =");
        query.push_bind(from_language);
        num_updates += 1;
    }

    if let Some(to_language_primary) = deck_form.to_language_primary {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" to_language_primary =");
        query.push_bind(to_language_primary);
        num_updates += 1;
    }

    if let Some(to_language_secondary) = deck_form.to_language_secondary {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" to_language_secondary =");
        query.push_bind(to_language_secondary);
        num_updates += 1;
    }

    if let Some(design_key) = deck_form.design_key {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" design_key =");
        query.push_bind(design_key);
        num_updates += 1;
    }

    if let Some(seen_at) = deck_form.seen_at {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" seen_at =");
        query.push_bind(seen_at);
        num_updates += 1;
    }

    if num_updates == 0 {
        return Err(Error::RowNotFound);
    }

    query.push(" WHERE id =");
    query.push_bind(deck_id);

    query.push(" AND user_id =");
    query.push_bind(user_id);

    let result = query.build().execute(pool).await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

pub async fn delete_deck_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    user_id: i32,
) -> Result<DatabaseQueryResult, Error> {
    let result = sqlx::query!(
        "DELETE FROM decks WHERE id = $1 AND user_id = $2",
        deck_id,
        user_id
    )
    .execute(pool)
    .await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

pub async fn read_cards_query(pool: &Pool<Postgres>, deck_id: i32) -> Result<Vec<Card>, Error> {
    sqlx::query_as!(Card, "SELECT * FROM cards WHERE deck_id = $1", deck_id)
        .fetch_all(pool)
        .await
}

pub async fn read_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_id: i32,
) -> Result<Vec<Card>, Error> {
    sqlx::query_as!(
        Card,
        "SELECT * FROM cards WHERE id = $1 AND deck_id = $2",
        card_id,
        deck_id
    )
    .fetch_all(pool)
    .await
}

pub async fn create_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_form: CardForm,
) -> Result<DatabaseQueryResult, Error> {
    if let None = card_form.from_text {
        return Err(Error::RowNotFound);
    }

    if let None = card_form.to_text_primary {
        return Err(Error::RowNotFound);
    }

    let result = sqlx::query!(
        "INSERT INTO cards (deck_id, from_text, to_text_primary, to_text_secondary, example_text, audio_url) VALUES ($1, $2, $3, $4, $5, $6)",
        deck_id,
        card_form.from_text,
        card_form.to_text_primary,
        card_form.to_text_secondary,
        card_form.example_text,
        card_form.audio_url,
    )
        .execute(pool)
        .await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

pub async fn update_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_id: i32,
    card_form: CardForm,
) -> Result<DatabaseQueryResult, Error> {
    let mut query = QueryBuilder::new("UPDATE cards SET");

    let mut num_updates = 0;

    if let Some(related_card_ids) = card_form.related_card_ids {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" related_card_ids =");
        query.push_bind(related_card_ids);
        num_updates += 1;
    }

    if let Some(from_text) = card_form.from_text {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" from_text =");
        query.push_bind(from_text);
        num_updates += 1;
    }

    if let Some(to_text_primary) = card_form.to_text_primary {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" to_text_primary =");
        query.push_bind(to_text_primary);
        num_updates += 1;
    }

    if let Some(to_text_secondary) = card_form.to_text_secondary {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" to_text_secondary =");
        query.push_bind(to_text_secondary);
        num_updates += 1;
    }

    if let Some(example_text) = card_form.example_text {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" example_text =");
        query.push_bind(example_text);
        num_updates += 1;
    }

    if let Some(audio_url) = card_form.audio_url {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" audio_url =");
        query.push_bind(audio_url);
        num_updates += 1;
    }

    if let Some(seen_at) = card_form.seen_at {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" seen_at =");
        query.push_bind(seen_at);
        num_updates += 1;
    }

    if let Some(seen_for) = card_form.seen_for {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" seen_for =");
        query.push_bind(seen_for);
        num_updates += 1;
    }

    if let Some(rating) = card_form.rating {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" rating =");
        query.push_bind(rating);
        num_updates += 1;
    }

    if num_updates == 0 {
        return Err(Error::RowNotFound);
    }

    query.push(" WHERE id =");
    query.push_bind(card_id);

    query.push(" AND deck_id =");
    query.push_bind(deck_id);

    let result = query.build().execute(pool).await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}

pub async fn delete_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_id: i32,
) -> Result<DatabaseQueryResult, Error> {
    let result = sqlx::query!(
        "DELETE FROM cards WHERE id = $1 AND deck_id = $2",
        card_id,
        deck_id
    )
    .execute(pool)
    .await;

    match result {
        Ok(pg_query_result) => Ok(DatabaseQueryResult {
            rows_affected: pg_query_result.rows_affected(),
        }),
        Err(err) => Err(err),
    }
}
