use axum::{response::Json, routing::get, Router};
use dotenv::dotenv;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr};

#[derive(serde::Serialize)]
struct User {
    id: i32,
    name: String,
    first: String,
    last: String,
    email: String,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // db

    dotenv().ok();

    let db_url = env::var("DATABASE_URL").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    let users = sqlx::query_as!(User, "SELECT * FROM users")
        .fetch_all(&pool)
        .await?;

    // sever

    let app = Router::new().route("/", get(Json(json!(users))));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// TODO: figure out how to use this function in a handler

// async fn json_stringify<T: Serialize>(data: T) -> Json<Value> {
//     Json(json!(data))
// }
