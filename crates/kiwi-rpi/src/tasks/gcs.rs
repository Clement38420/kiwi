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
use shared::messages::{LidarPoint, RpiToGcs};
use std::sync::Arc;
use tokio::sync::watch;

const GCS_PORT: &str = "8080";

type LidarWatchRx = watch::Receiver<Arc<Vec<LidarPoint>>>;

#[derive(RustEmbed)]
#[folder = "../../gcs/dist/"]
struct GcsAssets;

pub async fn gcs_task(lidar_rx: LidarWatchRx) {
    let app = Router::new()
        .route("/ws", any(ws_handler))
        .fallback(static_handler)
        .with_state(lidar_rx);

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
    State(lidar_rx): State<LidarWatchRx>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, lidar_rx))
}

async fn handle_socket(mut socket: WebSocket, mut lidar_rx: LidarWatchRx) {
    println!("GCS: Client connected via WebSocket");
    loop {
        tokio::select! {
            Ok(()) = lidar_rx.changed() => {
                let points = lidar_rx.borrow_and_update().clone();
                let msg = RpiToGcs::LidarScan(points);

                if let Ok(bytes) = rmp_serde::to_vec_named(&msg)
                    && socket.send(WsMessage::Binary(bytes.into())).await.is_err() // Client disconnected
                {
                    break;
                }
            }
            else => break,
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
