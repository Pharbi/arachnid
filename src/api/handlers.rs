use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::error::ApiError;
use crate::storage::Storage;
use crate::types::{Web, WebConfig, WebState};

#[derive(Deserialize)]
pub struct CreateWebRequest {
    pub task: String,
}

#[derive(Serialize)]
pub struct WebResponse {
    pub id: String,
    pub task: String,
    pub state: String,
}

impl From<Web> for WebResponse {
    fn from(web: Web) -> Self {
        Self {
            id: web.id.to_string(),
            task: web.task,
            state: match web.state {
                WebState::Running => "Running".to_string(),
                WebState::Converged => "Converged".to_string(),
                WebState::Failed => "Failed".to_string(),
            },
        }
    }
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn create_web(
    State(storage): State<Arc<dyn Storage>>,
    Json(request): Json<CreateWebRequest>,
) -> Result<Json<WebResponse>, ApiError> {
    let root_agent_id = uuid::Uuid::new_v4();
    let web = Web::new(root_agent_id, request.task, WebConfig::default());

    storage.create_web(&web).await?;

    Ok(Json(WebResponse::from(web)))
}

pub async fn list_webs(
    State(storage): State<Arc<dyn Storage>>,
) -> Result<Json<Vec<WebResponse>>, ApiError> {
    let webs = storage.list_webs(None).await?;
    Ok(Json(webs.into_iter().map(WebResponse::from).collect()))
}
