use axum::{
    Router,
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::any,
};
use rust_embed::RustEmbed;
use shared::messages::RpiToGcs;
use tokio::sync::broadcast;

const GCS_PORT: &str = "8080";

type WsSendingQueueTx = broadcast::Sender<RpiToGcs>;
type WsSendingQueueRx = broadcast::Receiver<RpiToGcs>;

#[derive(RustEmbed)]
#[folder = "../../gcs/dist/"]
struct GcsAssets;

pub async fn gcs_task(ws_sending_queue: WsSendingQueueTx) {
    let app = Router::new()
        .route("/ws", any(ws_handler))
        .fallback(static_handler)
        .with_state(ws_sending_queue);

    let listener = loop {
        if let Ok(listener) = tokio::net::TcpListener::bind(format!("0.0.0.0:{GCS_PORT}")).await {
            break listener;
        }
        eprintln!("GCS: Failed to bind port {GCS_PORT}, retrying in 5 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    };

    println!("GCS: Web interface available at http://0.0.0.0:{GCS_PORT}");

    #[allow(clippy::missing_panics_doc)]
    axum::serve(listener, app).await.unwrap(); // This never panics
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(sending_queue_tx): State<WsSendingQueueTx>,
) -> impl IntoResponse {
    let sending_queue_rx = sending_queue_tx.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, sending_queue_rx))
}

async fn handle_socket(mut socket: WebSocket, mut sending_queue: WsSendingQueueRx) {
    println!("GCS: Client connected via WebSocket");
    loop {
        tokio::select! {
            result = sending_queue.recv() => match result {
                Ok(message) => {
                    if let Ok(bytes) = rmp_serde::to_vec_named(&message)
                        && socket.send(WsMessage::Binary(bytes.into())).await.is_err()
                    {
                        break; // Client disconnected
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    eprintln!("GCS: WebSocket client lagged, skipped {skipped} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    println!("GCS: Client disconnected");
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match GcsAssets::get(path).or_else(|| GcsAssets::get("index.html")) {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, content.metadata.mimetype())
            .body(axum::body::Body::from(content.data))
            .unwrap(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
