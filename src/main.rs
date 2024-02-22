use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Form, Router,
};
use chrono::NaiveDateTime;
use rand::Rng;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{
    postgres::PgPoolOptions, query_builder::QueryBuilder, Error as SqlxError, Pool, Postgres,
};
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc};
use tower_http::services::ServeDir;

// db model

#[derive(serde::Serialize)]
struct User {
    id: i32,
    name: String,
    email: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(serde::Deserialize)]
struct UserForm {
    name: Option<String>,
    email: Option<String>,
}

#[derive(serde::Serialize)]
struct Deck {
    id: i32,
    user_id: i32,
    from_language: String,
    to_language_primary: String,
    to_language_secondary: Option<String>,
    design_key: Option<String>,
    seen_at: NaiveDateTime,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(serde::Deserialize)]
struct DeckForm {
    from_language: Option<String>,
    to_language_primary: Option<String>,
    to_language_secondary: Option<String>,
    design_key: Option<String>,
    seen_at: Option<NaiveDateTime>,
}

#[derive(Clone, serde::Serialize)]
struct Card {
    id: i32,
    deck_id: i32,
    related_card_ids: Vec<i32>,
    from_text: String,
    to_text_primary: String,
    to_text_secondary: Option<String>,
    example_text: Option<String>,
    audio_url: Option<String>,
    seen_at: NaiveDateTime,
    seen_for: Option<i32>,
    rating: i32,
    prev_rating: i32,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(serde::Deserialize)]
struct CardForm {
    related_card_ids: Option<Vec<i32>>,
    from_text: Option<String>,
    to_text_primary: Option<String>,
    to_text_secondary: Option<String>,
    example_text: Option<String>,
    audio_url: Option<String>,
    seen_at: Option<NaiveDateTime>,
    seen_for: Option<i32>,
    rating: Option<i32>,
}

// json response model

// TODO: make mutually exclusive enum

#[derive(serde::Serialize)]
struct ApiResponse<T: Serialize> {
    data: Option<T>,
    error: Option<ApiResponseError>,
}

#[derive(serde::Serialize)]
struct ApiResponseError {
    message: String,
}

#[derive(serde::Serialize)]
struct DatabaseQueryResult {
    rows_affected: u64,
}

// html response model

struct HtmlResponse<T>(T);

impl<T> IntoResponse for HtmlResponse<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

// askama templates

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    decks: Vec<Deck>,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    message: String,
}

#[derive(Template)]
#[template(path = "action.html")]
struct ActionTemplate {
    card: Card,
    num_cards: i32,
    deck_id: i32,
    index: usize,
    side: String,
    random: String,
    uuid: String,
}

#[derive(Template)]
#[template(path = "add_card.html")]
struct AddCardTemplate {
    deck_id: i32,
    card_index: i32,
    uuid: String,
}

// global state

struct AppState {
    pool: Pool<Postgres>,
    user: Option<User>,
    uuid: String,
}

// main

#[tokio::main]
async fn main() -> Result<(), SqlxError> {
    // env

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let uuid = env::var("UUID").expect("UUID must be set");

    // db

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // sever

    let app_state = Arc::new(AppState {
        pool,
        user: Some(User {
            id: 1i32,
            name: String::from("glencoden"),
            email: String::from("glen@coden.io"),
            created_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap(),
            updated_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap(),
        }),
        uuid,
    });

    let root_path = env::current_dir().unwrap();

    let api_router = Router::new()
        .route("/users", get(get_users).post(post_user))
        .route(
            "/users/:user_id",
            get(get_user).put(put_user).delete(delete_user),
        )
        .route("/decks", get(get_decks).post(post_deck))
        .route(
            "/decks/:deck_id",
            get(get_deck).put(put_deck).delete(delete_deck),
        )
        .route("/cards/:deck_id", get(get_cards).post(post_card))
        .route(
            "/cards/:deck_id/:card_id",
            get(get_card).put(put_card).delete(delete_card),
        );

    let app = Router::new()
        .nest("/api", api_router)
        .route("/", get(page_home))
        .route("/action/:deck_id/:card_index/:card_side", get(page_action))
        .route("/add_card/:deck_id/:card_index", get(page_add_card))
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", root_path.to_str().unwrap())),
        )
        .with_state(app_state);

    let port = 3000_u16;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// website route handlers

async fn page_home(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let uuid = query.get("uuid");
    // TODO: add error template
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        let template = HomeTemplate { decks: Vec::new() };

        return HtmlResponse(template);
    }

    let result = read_decks_query(&app_state.pool, app_state.user.as_ref().unwrap().id).await;

    if let Ok(decks) = result {
        let template = HomeTemplate { decks };

        HtmlResponse(template)
    } else {
        let template = HomeTemplate { decks: Vec::new() };

        HtmlResponse(template)
    }
}

async fn page_action(
    State(app_state): State<Arc<AppState>>,
    Path(params): Path<(i32, usize, String)>,
) -> impl IntoResponse {
    let result = read_cards_query(&app_state.pool, params.0).await;

    if let Ok(mut cards) = result {
        cards.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let card = cards.get(params.1).cloned();
        let random_number = rand::thread_rng().gen_range(0..=2);

        let random = if random_number > 0 {
            String::from("from")
        } else {
            String::from("to")
        };

        if let Some(card) = card {
            let template = ActionTemplate {
                card,
                num_cards: cards.len() as i32,
                deck_id: params.0,
                index: params.1,
                side: params.2,
                random,
                uuid: app_state.uuid.clone(),
            };

            return HtmlResponse(template);
        }
    }

    // TODO: add error template

    // let template = ErrorTemplate {
    //     message: String::from("No cards found for action"),
    // };
    //
    // HtmlResponse(template)

    let template = ActionTemplate {
        card: Card {
            id: 0,
            deck_id: 0,
            related_card_ids: Vec::new(),
            from_text: String::from("The End"),
            to_text_primary: String::from("The End"),
            to_text_secondary: None,
            example_text: None,
            audio_url: None,
            seen_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap(),
            seen_for: None,
            rating: 0,
            prev_rating: 0,
            created_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap(),
            updated_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap(),
        },
        num_cards: 0,
        deck_id: params.0,
        index: 0,
        side: String::from("from"),
        random: String::from("from"),
        uuid: app_state.uuid.clone(),
    };

    HtmlResponse(template)
}

async fn page_add_card(
    State(app_state): State<Arc<AppState>>,
    Path(params): Path<(i32, i32)>,
) -> impl IntoResponse {
    let template = AddCardTemplate {
        deck_id: params.0,
        card_index: params.1,
        uuid: app_state.uuid.clone(),
    };

    HtmlResponse(template)
}

// api route handlers

async fn get_users(State(app_state): State<Arc<AppState>>) -> Json<Value> {
    let result = read_users_query(&app_state.pool).await;

    db_result_to_json_response(result)
}

async fn get_user(State(app_state): State<Arc<AppState>>, Path(user_id): Path<i32>) -> Json<Value> {
    let result = read_user(&app_state.pool, user_id).await;

    db_result_to_json_response(result)
}

async fn post_user(
    State(app_state): State<Arc<AppState>>,
    Form(user_form): Form<UserForm>,
) -> Json<Value> {
    let result = create_user_query(&app_state.pool, user_form).await;

    db_result_to_json_response(result)
}

async fn put_user(
    State(app_state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    Form(user_form): Form<UserForm>,
) -> Result<Json<Value>, StatusCode> {
    let result = update_user_query(&app_state.pool, user_id, user_form).await;

    Ok(db_result_to_json_response(result))
}

async fn delete_user(
    State(app_state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
) -> Json<Value> {
    let result = delete_user_query(&app_state.pool, user_id).await;

    db_result_to_json_response(result)
}

async fn get_decks(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = read_decks_query(&app_state.pool, app_state.user.as_ref().unwrap().id).await;

    Ok(db_result_to_json_response(result))
}

async fn get_deck(
    State(app_state): State<Arc<AppState>>,
    Path(deck_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = read_deck(
        &app_state.pool,
        deck_id,
        app_state.user.as_ref().unwrap().id,
    )
    .await;

    Ok(db_result_to_json_response(result))
}

async fn post_deck(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>,
    Form(deck_form): Form<DeckForm>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = create_deck_query(
        &app_state.pool,
        deck_form,
        app_state.user.as_ref().unwrap().id,
    )
    .await;

    Ok(db_result_to_json_response(result))
}

async fn put_deck(
    State(app_state): State<Arc<AppState>>,
    Path(deck_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
    Form(deck_form): Form<DeckForm>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = update_deck_query(
        &app_state.pool,
        deck_id,
        deck_form,
        app_state.user.as_ref().unwrap().id,
    )
    .await;

    Ok(db_result_to_json_response(result))
}

async fn delete_deck(
    State(app_state): State<Arc<AppState>>,
    Path(deck_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = delete_deck_query(
        &app_state.pool,
        deck_id,
        app_state.user.as_ref().unwrap().id,
    )
    .await;

    Ok(db_result_to_json_response(result))
}

async fn get_cards(
    State(app_state): State<Arc<AppState>>,
    Path(deck_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = read_cards_query(&app_state.pool, deck_id).await;

    Ok(db_result_to_json_response(result))
}

async fn get_card(
    State(app_state): State<Arc<AppState>>,
    Path(ids): Path<(i32, i32)>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = read_card_query(&app_state.pool, ids.0, ids.1).await;

    Ok(db_result_to_json_response(result))
}

async fn post_card(
    State(app_state): State<Arc<AppState>>,
    Path(deck_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
    Form(card_form): Form<CardForm>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = create_card_query(&app_state.pool, deck_id, card_form).await;

    Ok(db_result_to_json_response(result))
}

async fn put_card(
    State(app_state): State<Arc<AppState>>,
    Path(ids): Path<(i32, i32)>,
    Query(query): Query<HashMap<String, String>>,
    Form(card_form): Form<CardForm>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = update_card_query(&app_state.pool, ids.0, ids.1, card_form).await;

    Ok(db_result_to_json_response(result))
}

async fn delete_card(
    State(app_state): State<Arc<AppState>>,
    Path(ids): Path<(i32, i32)>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = delete_card_query(&app_state.pool, ids.0, ids.1).await;

    Ok(db_result_to_json_response(result))
}

// database queries

async fn read_users_query(pool: &Pool<Postgres>) -> Result<Vec<User>, SqlxError> {
    sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(pool)
        .await
}

async fn read_user(pool: &Pool<Postgres>, user_id: i32) -> Result<Vec<User>, SqlxError> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_all(pool)
        .await
}

async fn create_user_query(
    pool: &Pool<Postgres>,
    user_form: UserForm,
) -> Result<DatabaseQueryResult, SqlxError> {
    if let None = user_form.name {
        return Err(SqlxError::RowNotFound);
    }

    if let None = user_form.email {
        return Err(SqlxError::RowNotFound);
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

async fn update_user_query(
    pool: &Pool<Postgres>,
    user_id: i32,
    user_form: UserForm,
) -> Result<DatabaseQueryResult, SqlxError> {
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
        return Err(SqlxError::RowNotFound);
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
async fn delete_user_query(
    pool: &Pool<Postgres>,
    user_id: i32,
) -> Result<DatabaseQueryResult, SqlxError> {
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

async fn read_decks_query(pool: &Pool<Postgres>, user_id: i32) -> Result<Vec<Deck>, SqlxError> {
    sqlx::query_as!(Deck, "SELECT * FROM decks WHERE user_id = $1", user_id)
        .fetch_all(pool)
        .await
}

async fn read_deck(
    pool: &Pool<Postgres>,
    deck_id: i32,
    user_id: i32,
) -> Result<Vec<Deck>, SqlxError> {
    sqlx::query_as!(
        Deck,
        "SELECT * FROM decks WHERE id = $1 AND user_id = $2",
        deck_id,
        user_id
    )
    .fetch_all(pool)
    .await
}

async fn create_deck_query(
    pool: &Pool<Postgres>,
    deck_form: DeckForm,
    user_id: i32,
) -> Result<DatabaseQueryResult, SqlxError> {
    if let None = deck_form.from_language {
        return Err(SqlxError::RowNotFound);
    }

    if let None = deck_form.to_language_primary {
        return Err(SqlxError::RowNotFound);
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

async fn update_deck_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    deck_form: DeckForm,
    user_id: i32,
) -> Result<DatabaseQueryResult, SqlxError> {
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
        return Err(SqlxError::RowNotFound);
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

async fn delete_deck_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    user_id: i32,
) -> Result<DatabaseQueryResult, SqlxError> {
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

async fn read_cards_query(pool: &Pool<Postgres>, deck_id: i32) -> Result<Vec<Card>, SqlxError> {
    sqlx::query_as!(Card, "SELECT * FROM cards WHERE deck_id = $1", deck_id)
        .fetch_all(pool)
        .await
}

async fn read_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_id: i32,
) -> Result<Vec<Card>, SqlxError> {
    sqlx::query_as!(
        Card,
        "SELECT * FROM cards WHERE id = $1 AND deck_id = $2",
        card_id,
        deck_id
    )
    .fetch_all(pool)
    .await
}

async fn create_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_form: CardForm,
) -> Result<DatabaseQueryResult, SqlxError> {
    if let None = card_form.from_text {
        return Err(SqlxError::RowNotFound);
    }

    if let None = card_form.to_text_primary {
        return Err(SqlxError::RowNotFound);
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

async fn update_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_id: i32,
    card_form: CardForm,
) -> Result<DatabaseQueryResult, SqlxError> {
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
        return Err(SqlxError::RowNotFound);
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

async fn delete_card_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    card_id: i32,
) -> Result<DatabaseQueryResult, SqlxError> {
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

// helpers

fn db_result_to_json_response<T: Serialize>(result: Result<T, SqlxError>) -> Json<Value> {
    let response = match result {
        Ok(data) => ApiResponse {
            data: Some(data),
            error: None,
        },
        Err(err) => ApiResponse {
            data: None,
            error: Some(ApiResponseError {
                message: format!("{}", err),
            }),
        },
    };

    Json(json!(response))
}
