use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::AppState;

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, room_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, room_id: String) {
    let (mut sender, mut receiver) = socket.split();

    let room_uuid = match Uuid::parse_str(&room_id) {
        Ok(id) => id,
        Err(_) => return,
    };

    // TODO: Verify user is in room via token in query string
    // For now, accept any connection - add auth in production

    let mut rx = {
        let mut rooms = state.rooms.write().await;
        let tx = rooms
            .entry(room_uuid)
            .or_insert_with(|| broadcast::channel(100).0)
            .subscribe();
        tx
    };

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Broadcast to room - in production, validate and store in DB
            if let Some(tx) = state.rooms.read().await.get(&room_uuid) {
                let _ = tx.send(text);
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
