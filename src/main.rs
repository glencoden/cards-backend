mod api;
mod pages;
mod queries;

use crate::api::{
    delete_card, delete_deck, delete_user, get_card, get_cards, get_deck, get_decks, get_user,
    get_users, post_card, post_deck, post_user, put_card, put_deck, put_user,
};
use crate::pages::{page_action, page_add_card, page_edit_card, page_home};
use axum::{routing::get, Router};
use chrono::NaiveDateTime;
use sqlx::{postgres::PgPoolOptions, Error, Pool, Postgres};
use std::sync::RwLock;
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

#[derive(Clone, serde::Serialize)]
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

// global state

struct AppState {
    pool: Pool<Postgres>,
    user: Option<User>,
    uuid: String,
    active_decks: RwLock<HashMap<i32, Vec<Card>>>,
}

// main

#[tokio::main]
async fn main() -> Result<(), Error> {
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
        active_decks: RwLock::new(HashMap::new()),
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
        .route(
            "/edit_card/:deck_id/:card_id/:card_index",
            get(page_edit_card),
        )
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
