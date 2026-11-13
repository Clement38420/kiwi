use crate::channels::{Base, Communication};
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
use shared::messages::{ControlMode, GcsToRpi, RpiToGcs, RpiToMcu};
use shared::rpc::RpcResponse;
use tokio::sync::broadcast;

const GCS_PORT: &str = "8080";

#[derive(RustEmbed)]
#[folder = "../../gcs/dist/"]
struct GcsAssets;

#[derive(Clone)]
struct WsState {
    comm: Communication,
    base: Base,
}

pub async fn gcs_task(comm: Communication, base: Base) {
    let app = Router::new()
        .route("/ws", any(ws_handler))
        .fallback(static_handler)
        .with_state(WsState { comm, base });

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

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.comm, state.base))
}

async fn handle_socket(mut socket: WebSocket, comm: Communication, base: Base) {
    println!("GCS: Client connected via WebSocket");
    let mut gcs_rx = comm.gcs.subscribe();

    loop {
        tokio::select! {
            result = gcs_rx.recv() => match result {
                Ok(message) => {
                    if ws_send(&mut socket, &message).await.is_err() {
                        break; // Client disconnected
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    eprintln!("GCS TX: sending lagged, skipped {skipped} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            message = socket.recv() => match message {
                Some(Ok(WsMessage::Binary(bytes))) => {
                    match rmp_serde::from_slice::<GcsToRpi>(&bytes) {
                        Ok(message) => {
                            match &message {
                                GcsToRpi::Arm(_) => {}
                                GcsToRpi::GetStateReq(req) => {
                                    let res = RpiToGcs::GetStateRes(RpcResponse::new(
                                        req,
                                        *base.state.borrow(),
                                    ));
                                    if ws_send(&mut socket, &res).await.is_err() {
                                        break; // Client disconnected
                                    }
                                }
                                GcsToRpi::Reset => {}
                                GcsToRpi::Twist(t) => {
                                    let _ = comm.mcu.send(RpiToMcu::WheelSpeedSetpoints(t.to_wheel_speeds()));
                                }
                            }

                            // If the message can be converted to RpiToMcu, forward it
                            if let Ok(message) = RpiToMcu::try_from(message) {
                                let _ = comm.mcu.send(message);
                            }
                        }
                        Err(e) => {
                            eprintln!("GCS RX: Failed to decode message: {e}");
                        }
                    }
                }
                Some(Ok(WsMessage::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => {} // Ignore Text, Ping, Pong (automatically handled by Axum)
            },
        }
    }
    println!("GCS: Client disconnected");
}

#[derive(thiserror::Error, Debug)]
enum WsError {
    #[error("Failed to encode message: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("WebSocket error: {0}")]
    Socket(#[from] axum::Error),
}

async fn ws_send(socket: &mut WebSocket, message: &RpiToGcs) -> Result<(), WsError> {
    let bytes = rmp_serde::to_vec_named(message)?;
    socket.send(WsMessage::Binary(bytes.into())).await?;
    Ok(())
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
