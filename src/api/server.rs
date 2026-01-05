use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::api::handlers;
use crate::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/webs", post(handlers::create_web))
        .route("/webs", get(handlers::list_webs))
        .layer(CorsLayer::permissive())
        .with_state(state.storage)
}

pub async fn serve(state: AppState, port: u16) -> Result<()> {
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    println!("Arachnid API server listening on port {}", port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::InMemoryStore;

    #[tokio::test]
    async fn test_create_router() {
        let storage = Arc::new(InMemoryStore::new());
        let state = AppState {
            storage: storage as Arc<dyn Storage>,
        };
        let _router = create_router(state);
    }
}
