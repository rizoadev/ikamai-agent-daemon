//! WebSocket server — real-time event streaming (Phase 3.8).

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use serde::Deserialize;
use std::collections::HashSet;
use tracing::{info, warn};
use crate::AppState;

#[derive(Deserialize)]
pub struct WsQuery {
    pub topics: Option<String>,
    pub token:  Option<String>,
}

pub async fn ws_handler(
    ws:           WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Some(ref expected) = state.api_token {
        let provided = query.token.as_deref().unwrap_or("");
        if provided != expected {
            return axum::http::StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let topic_filter: HashSet<String> = query
        .topics
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let event_bus = state.event_bus.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, event_bus, topic_filter))
}

async fn handle_socket(
    mut socket: WebSocket,
    event_bus:  std::sync::Arc<iad_core::event_bus::EventBus>,
    topic_filter: HashSet<String>,
) {
    info!("WebSocket client connected");
    let mut rx = event_bus.subscribe_all().await;

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                if !topic_filter.is_empty() {
                    let matches = topic_filter.iter().any(|f| {
                        event.topic.starts_with(f.trim_end_matches('*').trim_end_matches('.'))
                    });
                    if !matches { continue; }
                }
                let msg = serde_json::json!({
                    "id":         event.id,
                    "topic":      event.topic,
                    "event_type": event.event_type,
                    "payload":    event.payload,
                    "sequence":   event.sequence,
                    "ts":         event.created_at,
                });
                if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(d))) => { let _ = socket.send(Message::Pong(d)).await; }
                    Some(Ok(Message::Close(_))) | None => { info!("WebSocket client disconnected"); break; }
                    Some(Err(e)) => { warn!("WebSocket error: {e}"); break; }
                    _ => {}
                }
            }
        }
    }
}
