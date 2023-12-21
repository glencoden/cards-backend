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

// TODO: combine User structs and add Options >> create http test flow

#[derive(serde::Serialize)]
struct User {
    id: i32,
    name: String,
    email: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(serde::Deserialize)]
struct UserPostForm {
    name: String,
    email: String,
}

#[derive(serde::Deserialize)]
struct UserPutForm {
    id: i32,
    name: Option<String>,
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

    let app_state = Arc::new(AppState { pool });

    let root_path = env::current_dir().unwrap();

    let api_router = Router::new()
        .route("/users", get(get_users).post(create_user).put(update_user))
        .route("/users/:user_id", get(read_user).delete(delete_user));

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
    let result = read_users(&app_state).await;

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

// TODO: either extract all db logic or find a way to call api from website route handlers

async fn read_users(app_state: &Arc<AppState>) -> Result<Vec<User>, SqlxError> {
    sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(&app_state.pool)
        .await
}

async fn get_users(State(app_state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let result = read_users(&app_state).await;

    // TODO: set status code and simply pass result to to_json_response

    if let Ok(users) = result {
        Ok(to_json_response(Ok(users)))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// END

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
    Form(user_post_form): Form<UserPostForm>,
) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query!(
        "INSERT INTO users (name, email) VALUES ($1, $2)",
        user_post_form.name,
        user_post_form.email,
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
    Form(user_put_form): Form<UserPutForm>,
) -> Result<Json<Value>, StatusCode> {
    let mut query = QueryBuilder::new("UPDATE users SET");

    let mut num_updates = 0;

    // TODO: iterate over fields and extract as helper function

    if let Some(name) = user_put_form.name {
        if num_updates > 0 {
            query.push(",");
        }
        query.push(" name =");
        query.push_bind(name);
        num_updates += 1;
    }

    if let Some(email) = user_put_form.email {
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
    query.push_bind(user_put_form.id);

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
