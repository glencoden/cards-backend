use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
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

// db model

// TODO: combine User structs and add Options > create http test flow

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

// json response model

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
struct DbInsertResult {
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
#[template(path = "test.html")]
struct TestTemplate;

// global state

struct AppState {
    pool: Pool<Postgres>,
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

    // state

    let app_state = Arc::new(AppState { pool });

    // sever

    let app = Router::new()
        .route("/", get(test))
        .route(
            "/api/users",
            get(read_users).post(create_user).put(update_user),
        )
        .route("/api/users/:user_id", get(read_user).delete(delete_user))
        .with_state(app_state);

    let port = 3000_u16;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// website route handlers

async fn test() -> impl IntoResponse {
    let template = TestTemplate {};

    HtmlResponse(template)
}

// api route handlers

async fn read_users(State(app_state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(&app_state.pool)
        .await;

    // TODO: set status code and simply pass result to to_json_response

    if let Ok(users) = result {
        Ok(to_json_response(Ok(users)))
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
        Ok(to_json_response(Ok(users)))
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
        Ok(to_json_response(Ok(DbInsertResult {
            rows_affected: pg_query_result.rows_affected(),
        })))
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
        Ok(to_json_response(Ok(DbInsertResult {
            rows_affected: pg_query_result.rows_affected(),
        })))
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
        Ok(to_json_response(Ok(DbInsertResult {
            rows_affected: pg_query_result.rows_affected(),
        })))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// helpers

fn to_json_response<T: Serialize>(result: Result<T, ApiResponseError>) -> Json<Value> {
    let response = match result {
        Ok(data) => ApiResponse {
            data: Some(data),
            error: None,
        },
        Err(err) => ApiResponse {
            data: None,
            error: Some(err),
        },
    };

    Json(json!(response))
}
