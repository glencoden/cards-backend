use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use dotenvy::dotenv;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{
    postgres::PgPoolOptions, query_builder::QueryBuilder, Error as SqlxError, Pool, Postgres,
};
use std::sync::Arc;
use std::{env, net::SocketAddr};

// db models

// TODO: combine User structs and add Options

#[derive(serde::Serialize)]
struct User {
    id: i32,
    name: String,
    first: String,
    last: String,
    email: String,
}

#[derive(serde::Deserialize)]
struct UserCreatePayload {
    name: String,
    first: String,
    last: String,
    email: String,
}

#[derive(serde::Deserialize)]
struct UserUpdatePayload {
    id: i32,
    name: Option<String>,
    first: Option<String>,
    last: Option<String>,
    email: Option<String>,
}

// response models

// TODO: decide where to send custom errors in response

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

#[derive(serde::Serialize)]
struct DbInsertResult {
    rows_affected: u64,
}

// global state

struct AppState {
    pool: Pool<Postgres>,
}

#[tokio::main]
async fn main() -> Result<(), SqlxError> {
    dotenv().unwrap();

    // db

    let db_url = env::var("DATABASE_URL").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // state

    let app_state = Arc::new(AppState { pool });

    // sever

    let app = Router::new()
        .route("/users", get(read_users).post(create_user).put(update_user))
        .route("/users/:user_id", get(read_user).delete(delete_user))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// route handlers

async fn read_users(State(app_state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(&app_state.pool)
        .await;

    if let Ok(users) = result {
        Ok(to_json_response(users))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn read_user(
    State(app_state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_all(&app_state.pool)
        .await;

    if let Ok(users) = result {
        Ok(to_json_response(users))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn create_user(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UserCreatePayload>,
) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query!(
        "INSERT INTO users (name, first, last, email) VALUES ($1, $2, $3, $4)",
        payload.name,
        payload.first,
        payload.last,
        payload.email,
    )
    .execute(&app_state.pool)
    .await;

    if let Ok(pg_query_result) = result {
        Ok(to_json_response(DbInsertResult {
            rows_affected: pg_query_result.rows_affected(),
        }))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn update_user(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UserUpdatePayload>,
) -> Result<Json<Value>, StatusCode> {
    let mut query = QueryBuilder::new("UPDATE users SET");

    let mut num_updates = 0;

    // TODO: iterate over fields and extract as helper function

    if let Some(name) = payload.name {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" name =");
        query.push_bind(name);
        num_updates += 1;
    }

    if let Some(first) = payload.first {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" first =");
        query.push_bind(first);
        num_updates += 1;
    }

    if let Some(last) = payload.last {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" last =");
        query.push_bind(last);
        num_updates += 1;
    }

    if let Some(email) = payload.email {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" email =");
        query.push_bind(email);
        num_updates += 1;
    }

    if num_updates == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    query.push(" WHERE id =");
    query.push_bind(payload.id);

    let result = query.build().execute(&app_state.pool).await;

    if let Ok(pg_query_result) = result {
        Ok(to_json_response(DbInsertResult {
            rows_affected: pg_query_result.rows_affected(),
        }))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn delete_user(
    State(app_state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&app_state.pool)
        .await;

    if let Ok(pg_query_result) = result {
        Ok(to_json_response(DbInsertResult {
            rows_affected: pg_query_result.rows_affected(),
        }))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// helpers

fn to_json_response<T: Serialize>(data: T) -> Json<Value> {
    let result = ApiResponseVariantOk { data, error: () };

    Json(json!(result))
}
