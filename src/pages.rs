use crate::queries::{
    read_card_query, read_cards_query, read_deck, read_decks_query, update_deck_query,
};
use crate::{AppState, Card, Deck, DeckForm};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use rand::Rng;
use sqlx::{Error, Pool, Postgres};
use std::collections::HashMap;
use std::sync::Arc;

// askama templates

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    decks: Vec<Deck>,
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
    deck: Deck,
    card_index: i32,
    uuid: String,
}

#[derive(Template)]
#[template(path = "edit_card.html")]
struct EditCardTemplate {
    deck: Deck,
    card: Card,
    card_index: i32,
    uuid: String,
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

// route handlers

pub async fn page_home(
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

    if let Ok(mut decks) = result {
        decks.sort_by(|a, b| a.id.cmp(&b.id));

        let template = HomeTemplate { decks };

        HtmlResponse(template)
    } else {
        let template = HomeTemplate { decks: Vec::new() };

        HtmlResponse(template)
    }
}

pub async fn read_cards_and_set_deck_timestamp_query(
    pool: &Pool<Postgres>,
    deck_id: i32,
    user_id: i32,
) -> Result<Vec<Card>, Error> {
    update_deck_query(
        pool,
        deck_id,
        DeckForm {
            from_language: None,
            to_language_primary: None,
            to_language_secondary: None,
            design_key: None,
            seen_at: Some(chrono::Utc::now().naive_utc()),
        },
        user_id,
    )
    .await
    .expect("should be defined");

    read_cards_query(pool, deck_id).await
}

pub async fn page_action(
    State(app_state): State<Arc<AppState>>,
    Path(params): Path<(i32, usize, String)>,
) -> impl IntoResponse {
    if params.1 == 0 && params.2 == "from" {
        let deck_result = read_deck(
            &app_state.pool,
            params.0,
            app_state.user.as_ref().unwrap().id,
        )
        .await;

        let cards_result = read_cards_and_set_deck_timestamp_query(
            &app_state.pool,
            params.0,
            app_state.user.as_ref().unwrap().id,
        )
        .await;

        if let Ok(mut cards) = cards_result {
            if let Ok(decks) = deck_result {
                let deck = decks.get(0).cloned().unwrap();

                cards.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

                let mut weights: HashMap<i32, i32> = HashMap::new();

                for card in &cards {
                    // 1. All set to 4 after deck last seen

                    if card.rating == 4 && deck.seen_at < card.updated_at {
                        weights.insert(card.id, 1_000_000);
                    }

                    // 2. All unrated

                    if card.rating == 0 {
                        weights.insert(card.id, 100_000);
                    }
                }

                let span = cards[0].updated_at - cards[cards.len() - 1].updated_at; // youngest - oldest

                for card in &cards {
                    // Continue if weights already include card id

                    if weights.contains_key(&card.id) {
                        continue;
                    }

                    // 3. If num < DAILY_REVIEW_COUNT, fill with youngest

                    if weights.len() < 9 {
                        weights.insert(card.id, 100_000);
                    }

                    // 4. Weight by rating times weight by last seen - ceil((youngest - current) / span * 4)
                    // TODO: Weight by time looked at: max(lower_limit, min(x, upper_limit))

                    let current_age = cards[0].updated_at - card.updated_at;

                    let span_number = span.num_milliseconds() as f32;
                    let current_age_number = current_age.num_milliseconds() as f32;

                    let weight_by_last_seen: f32 =
                        (current_age_number / span_number * 4_f32).ceil();

                    weights.insert(card.id, card.rating + weight_by_last_seen as i32);
                }

                // Sort cards by weight

                cards.sort_by(|a, b| weights.get(&b.id).cmp(&weights.get(&a.id)));

                let mut decks = app_state.active_decks.write().unwrap();

                decks.insert(params.0, cards);
            }
        }
    }

    if let Some(deck) = app_state.active_decks.read().unwrap().get(&params.0) {
        let card = deck.get(params.1).cloned();
        let random_number = rand::thread_rng().gen_range(0..=2);

        let random = if random_number > 0 {
            String::from("from")
        } else {
            String::from("to")
        };

        if let Some(card) = card {
            let template = ActionTemplate {
                card,
                num_cards: deck.len() as i32,
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

pub async fn page_add_card(
    State(app_state): State<Arc<AppState>>,
    Path(params): Path<(i32, i32)>,
) -> impl IntoResponse {
    let result = read_deck(
        &app_state.pool,
        params.0,
        app_state.user.as_ref().unwrap().id,
    )
    .await;

    if let Ok(deck) = result {
        let template = AddCardTemplate {
            deck: deck.get(0).cloned().unwrap(),
            card_index: params.1,
            uuid: app_state.uuid.clone(),
        };

        HtmlResponse(template)
    } else {
        let template = AddCardTemplate {
            deck: Deck {
                id: 0,
                user_id: 0,
                from_language: String::from("Not found"),
                to_language_primary: String::from("Not found"),
                to_language_secondary: None,
                design_key: None,
                seen_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                    .unwrap()
                    .and_hms_opt(9, 10, 11)
                    .unwrap(),
                created_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                    .unwrap()
                    .and_hms_opt(9, 10, 11)
                    .unwrap(),
                updated_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                    .unwrap()
                    .and_hms_opt(9, 10, 11)
                    .unwrap(),
            },
            card_index: params.1,
            uuid: app_state.uuid.clone(),
        };

        HtmlResponse(template)
    }
}

pub async fn page_edit_card(
    State(app_state): State<Arc<AppState>>,
    Path(params): Path<(i32, i32, i32)>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let uuid = query.get("uuid");

    let deck_result = read_deck(
        &app_state.pool,
        params.0,
        app_state.user.as_ref().unwrap().id,
    )
    .await;

    let card_result = read_card_query(&app_state.pool, params.0, params.1).await;

    if app_state.user.is_none()
        || uuid.is_none()
        || uuid.unwrap() != &app_state.uuid
        || deck_result.is_err()
        || card_result.is_err()
    {
        let template = EditCardTemplate {
            deck: Deck {
                id: 0,
                user_id: 0,
                from_language: String::from("Not found"),
                to_language_primary: String::from("Not found"),
                to_language_secondary: None,
                design_key: None,
                seen_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                    .unwrap()
                    .and_hms_opt(9, 10, 11)
                    .unwrap(),
                created_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                    .unwrap()
                    .and_hms_opt(9, 10, 11)
                    .unwrap(),
                updated_at: chrono::NaiveDate::from_ymd_opt(2016, 7, 8)
                    .unwrap()
                    .and_hms_opt(9, 10, 11)
                    .unwrap(),
            },
            card: Card {
                id: 0,
                deck_id: 0,
                related_card_ids: Vec::new(),
                from_text: String::from("Not found"),
                to_text_primary: String::from("Not found"),
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
            card_index: params.2,
            uuid: app_state.uuid.clone(),
        };

        return HtmlResponse(template);
    }

    let template = EditCardTemplate {
        deck: deck_result
            .expect("should be defined")
            .get(0)
            .cloned()
            .unwrap(),
        card: card_result
            .expect("should be defined")
            .get(0)
            .cloned()
            .unwrap(),
        card_index: params.2,
        uuid: app_state.uuid.clone(),
    };

    return HtmlResponse(template);
}
