use anyhow::Result;
use axum::{
    routing::{delete, get, post},
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
        .route("/config", get(handlers::get_config))
        .route("/webs", post(handlers::create_web))
        .route("/webs", get(handlers::list_webs))
        .route("/webs/:id", get(handlers::get_web))
        .route("/webs/:id", delete(handlers::terminate_web))
        .route("/webs/:id/results", get(handlers::get_web_results))
        .route("/webs/:id/agents", get(handlers::get_web_agents))
        .route("/webs/:id/signals", get(handlers::get_web_signals))
        .route("/webs/:id/events", get(handlers::stream_web_events))
        .route("/agents/:id", get(handlers::get_agent))
        .route("/agents/:id/context", get(handlers::get_agent_context))
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
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::storage::memory::InMemoryStore;
    use crate::types::{Agent, CapabilityType, Web, WebConfig};

    fn create_test_app() -> (Router, Arc<InMemoryStore>) {
        let storage = Arc::new(InMemoryStore::new());
        let state = AppState {
            storage: storage.clone() as Arc<dyn Storage>,
        };
        (create_router(state), storage)
    }

    #[tokio::test]
    async fn test_create_router() {
        let storage = Arc::new(InMemoryStore::new());
        let state = AppState {
            storage: storage as Arc<dyn Storage>,
        };
        let _router = create_router(state);
    }

    #[tokio::test]
    async fn test_health_check() {
        let (app, _) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "healthy");
    }

    #[tokio::test]
    async fn test_get_config() {
        let (app, _) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json["version"].is_string());
        assert!(json["default_activation_threshold"].is_number());
    }

    #[tokio::test]
    async fn test_create_web() {
        let (app, _) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"task": "Test task"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["task"], "Test task");
        assert_eq!(json["state"], "Running");
    }

    #[tokio::test]
    async fn test_list_webs_empty() {
        let (app, _) = create_test_app();

        let response = app
            .oneshot(Request::builder().uri("/webs").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_web_not_found() {
        let (app, _) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/webs/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_web_success() {
        let (app, storage) = create_test_app();

        let root_agent_id = uuid::Uuid::new_v4();
        let web = Web::new(root_agent_id, "Test task".to_string(), WebConfig::default());
        let web_id = web.id;
        storage.create_web(&web).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/webs/{}", web_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["task"], "Test task");
    }

    #[tokio::test]
    async fn test_terminate_web() {
        let (app, storage) = create_test_app();

        let root_agent_id = uuid::Uuid::new_v4();
        let web = Web::new(root_agent_id, "Test task".to_string(), WebConfig::default());
        let web_id = web.id;
        storage.create_web(&web).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/webs/{}", web_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["state"], "Failed");
    }

    #[tokio::test]
    async fn test_get_web_agents() {
        let (app, storage) = create_test_app();

        let web = Web::new(
            uuid::Uuid::new_v4(),
            "Test task".to_string(),
            WebConfig::default(),
        );
        let web_id = web.id;
        storage.create_web(&web).await.unwrap();

        let agent = Agent::new(
            web_id,
            None,
            "Test agent".to_string(),
            vec![1.0; 1536],
            CapabilityType::Search,
            0.6,
        );
        storage.create_agent(&agent).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/webs/{}/agents", web_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["purpose"], "Test agent");
    }

    #[tokio::test]
    async fn test_get_agent_not_found() {
        let (app, _) = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/agents/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_agent_success() {
        let (app, storage) = create_test_app();

        let web = Web::new(
            uuid::Uuid::new_v4(),
            "Test task".to_string(),
            WebConfig::default(),
        );
        storage.create_web(&web).await.unwrap();

        let agent = Agent::new(
            web.id,
            None,
            "Test agent".to_string(),
            vec![1.0; 1536],
            CapabilityType::Synthesizer,
            0.6,
        );
        let agent_id = agent.id;
        storage.create_agent(&agent).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/agents/{}", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["purpose"], "Test agent");
        assert_eq!(json["capability"], "Synthesizer");
    }

    #[tokio::test]
    async fn test_get_agent_context() {
        let (app, storage) = create_test_app();

        let web = Web::new(
            uuid::Uuid::new_v4(),
            "Test task".to_string(),
            WebConfig::default(),
        );
        storage.create_web(&web).await.unwrap();

        let agent = Agent::new(
            web.id,
            None,
            "Test agent".to_string(),
            vec![1.0; 1536],
            CapabilityType::Search,
            0.6,
        );
        let agent_id = agent.id;
        storage.create_agent(&agent).await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/agents/{}/context", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["purpose"], "Test agent");
        assert!(json["accumulated_knowledge"].as_array().unwrap().is_empty());
    }
}
