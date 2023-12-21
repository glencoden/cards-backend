use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Form, Router,
};
use chrono::NaiveDateTime;
use dotenvy::dotenv;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{
    postgres::PgPoolOptions, query_builder::QueryBuilder, Error as SqlxError, Pool, Postgres,
};
use std::{env, net::SocketAddr, sync::Arc};
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
    to_language: String,
    seen_at: NaiveDateTime,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(serde::Deserialize)]
struct DeckForm {
    from_language: Option<String>,
    to_language: Option<String>,
    seen_at: Option<NaiveDateTime>,
}

#[derive(serde::Serialize)]
struct Card {
    id: i32,
    deck_id: i32,
    related_card_ids: Vec<i32>,
    example_text: String,
    audio_url: String,
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
struct HomeTemplate;

#[derive(Template)]
#[template(path = "users.html")]
struct UsersTemplate {
    users: Vec<User>,
}

// global state

struct AppState {
    pool: Pool<Postgres>,
    user: Option<User>,
}

// main

#[tokio::main]
async fn main() -> Result<(), SqlxError> {
    dotenv().unwrap();

    // db

    let db_url = env::var("DATABASE_URL").unwrap();

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
            email: String::from("simon.der.meyer@gmail.com"),
            created_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap(),
            updated_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                .unwrap()
                .and_hms_opt(9, 10, 11)
                .unwrap(),
        }),
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
        );

    let app = Router::new()
        .nest("/api", api_router)
        .route("/", get(page_home))
        .route("/users", get(users_page))
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", root_path.to_str().unwrap())),
        )
        .with_state(app_state);

    let port = 3000_u16;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// website route handlers

async fn page_home() -> impl IntoResponse {
    let template = HomeTemplate {};

    HtmlResponse(template)
}

async fn users_page(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = read_users_query(&app_state.pool).await;

    if let Ok(users) = result {
        let template = UsersTemplate { users };

        HtmlResponse(template)
    } else {
        // TODO: add error template

        let template = UsersTemplate { users: Vec::new() };

        HtmlResponse(template)
    }
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

async fn get_decks(State(app_state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    if let None = app_state.user {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = read_decks_query(&app_state.pool, app_state.user.as_ref().unwrap().id).await;

    Ok(db_result_to_json_response(result))
}

async fn get_deck(
    State(app_state): State<Arc<AppState>>,
    Path(deck_id): Path<i32>,
) -> Result<Json<Value>, StatusCode> {
    if let None = app_state.user {
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
    Form(deck_form): Form<DeckForm>,
) -> Result<Json<Value>, StatusCode> {
    if let None = app_state.user {
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
    Form(deck_form): Form<DeckForm>,
) -> Result<Json<Value>, StatusCode> {
    if let None = app_state.user {
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
) -> Result<Json<Value>, StatusCode> {
    if let None = app_state.user {
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

    if let None = deck_form.to_language {
        return Err(SqlxError::RowNotFound);
    }

    if let None = deck_form.seen_at {
        return Err(SqlxError::RowNotFound);
    }

    let result = sqlx::query!(
        "INSERT INTO decks (from_language, to_language, seen_at, user_id) VALUES ($1, $2, $3, $4)",
        deck_form.from_language,
        deck_form.to_language,
        deck_form.seen_at,
        user_id,
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

    if let Some(to_language) = deck_form.to_language {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" to_language =");
        query.push_bind(to_language);
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
