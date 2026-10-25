use axum::{
    Router,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

const GCS_PORT: &str = "8080";

#[derive(RustEmbed)]
#[folder = "../../gcs/dist/"]
struct GcsAssets;

pub async fn gcs_task() {
    let app = Router::new().fallback(static_handler);

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
