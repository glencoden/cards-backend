use crate::queries::{
    create_card_query, create_deck_query, create_user_query, delete_card_query, delete_deck_query,
    delete_user_query, read_card_query, read_cards_query, read_deck, read_decks_query, read_user,
    read_users_query, update_card_query, update_deck_query, update_user_query,
};
use crate::{AppState, CardForm, DeckForm, UserForm};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Form, Json};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::Error;
use std::collections::HashMap;
use std::sync::Arc;

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

// helpers

fn db_result_to_json_response<T: Serialize>(result: Result<T, Error>) -> Json<Value> {
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

// api route handlers

pub async fn get_users(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = read_users_query(&app_state.pool).await;

    Ok(db_result_to_json_response(result))
}

pub async fn get_user(
    State(app_state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = read_user(&app_state.pool, user_id).await;

    Ok(db_result_to_json_response(result))
}

pub async fn post_user(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>,
    Form(user_form): Form<UserForm>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = create_user_query(&app_state.pool, user_form).await;

    Ok(db_result_to_json_response(result))
}

pub async fn put_user(
    State(app_state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
    Form(user_form): Form<UserForm>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = update_user_query(&app_state.pool, user_id, user_form).await;

    Ok(db_result_to_json_response(result))
}

pub async fn delete_user(
    State(app_state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let uuid = query.get("uuid");
    if app_state.user.is_none() || uuid.is_none() || uuid.unwrap() != &app_state.uuid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let result = delete_user_query(&app_state.pool, user_id).await;

    Ok(db_result_to_json_response(result))
}

pub async fn get_decks(
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

pub async fn get_deck(
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

pub async fn post_deck(
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

pub async fn put_deck(
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

pub async fn delete_deck(
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

pub async fn get_cards(
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

pub async fn get_card(
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

pub async fn post_card(
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

pub async fn put_card(
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

pub async fn delete_card(
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
