use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use dotenv::dotenv;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, Error as SqlxError, Pool, Postgres};
use std::sync::Arc;
use std::{env, net::SocketAddr};

// db models

#[derive(serde::Serialize)]
struct User {
    id: i32,
    name: String,
    first: String,
    last: String,
    email: String,
}

// response models

// #[derive(serde::Serialize)]
// enum ApiResponse<'a, T: Serialize> {
//     Ok(ApiResponseVariantOk<T>),
//     Err(&'a ApiResponseVariantErr),
// }

#[derive(serde::Serialize)]
struct ApiResponseVariantOk<T: Serialize> {
    data: T,
    error: (),
}

// #[derive(serde::Serialize)]
// struct ApiResponseVariantErr {
//     data: (),
//     error: ApiResponseError,
// }
//
// #[derive(serde::Serialize)]
// struct ApiResponseError {
//     message: str,
// }

// global state

struct AppState {
    pool: Pool<Postgres>,
}

#[tokio::main]
async fn main() -> Result<(), SqlxError> {
    // db

    dotenv().ok();

    let db_url = env::var("DATABASE_URL").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // state

    let shared_state = Arc::new(AppState { pool });

    // sever

    let app = Router::new()
        .route("/users", get(get_users))
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// route handlers

async fn get_users(State(app_state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let users_result = sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(&app_state.pool)
        .await;

    if let Ok(users) = users_result {
        Ok(to_json_response(users))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// util

fn to_json_response<T: Serialize>(data: T) -> Json<Value> {
    let result = ApiResponseVariantOk { data, error: () };

    Json(json!(result))
}
