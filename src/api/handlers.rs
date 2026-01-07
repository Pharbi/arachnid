use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Duration};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::storage::Storage;
use crate::types::{Agent, AgentContext, Signal, Web, WebConfig, WebState};

#[derive(Deserialize)]
pub struct CreateWebRequest {
    pub task: String,
}

#[derive(Serialize)]
pub struct WebResponse {
    pub id: String,
    pub task: String,
    pub state: String,
    pub root_agent_id: String,
}

impl From<Web> for WebResponse {
    fn from(web: Web) -> Self {
        Self {
            id: web.id.to_string(),
            task: web.task,
            state: format!("{:?}", web.state),
            root_agent_id: web.root_agent.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct WebDetailResponse {
    pub id: String,
    pub task: String,
    pub state: String,
    pub root_agent_id: String,
    pub agent_count: usize,
    pub pending_signal_count: usize,
}

#[derive(Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub web_id: String,
    pub parent_id: Option<String>,
    pub purpose: String,
    pub capability: String,
    pub state: String,
    pub health: f32,
    pub activation_threshold: f32,
}

impl From<Agent> for AgentResponse {
    fn from(agent: Agent) -> Self {
        Self {
            id: agent.id.to_string(),
            web_id: agent.web_id.to_string(),
            parent_id: agent.parent_id.map(|id| id.to_string()),
            purpose: agent.purpose,
            capability: format!("{:?}", agent.capability),
            state: format!("{:?}", agent.state),
            health: agent.health,
            activation_threshold: agent.activation_threshold,
        }
    }
}

#[derive(Serialize)]
pub struct AgentDetailResponse {
    pub id: String,
    pub web_id: String,
    pub parent_id: Option<String>,
    pub purpose: String,
    pub capability: String,
    pub state: String,
    pub health: f32,
    pub activation_threshold: f32,
    pub probation_remaining: u32,
    pub created_at: String,
    pub last_active_at: String,
    pub children_count: usize,
}

#[derive(Serialize)]
pub struct SignalResponse {
    pub id: String,
    pub origin: String,
    pub content: String,
    pub direction: String,
    pub amplitude: f32,
    pub hop_count: u32,
}

impl From<Signal> for SignalResponse {
    fn from(signal: Signal) -> Self {
        Self {
            id: signal.id.to_string(),
            origin: signal.origin.to_string(),
            content: signal.content,
            direction: format!("{:?}", signal.direction),
            amplitude: signal.amplitude,
            hop_count: signal.hop_count,
        }
    }
}

#[derive(Serialize)]
pub struct WebResultsResponse {
    pub web_id: String,
    pub state: String,
    pub accumulated_knowledge: Vec<KnowledgeItem>,
}

#[derive(Serialize)]
pub struct KnowledgeItem {
    pub source_agent: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ContextResponse {
    pub purpose: String,
    pub accumulated_knowledge: Vec<KnowledgeItem>,
}

impl From<AgentContext> for ContextResponse {
    fn from(ctx: AgentContext) -> Self {
        Self {
            purpose: ctx.purpose,
            accumulated_knowledge: ctx
                .accumulated_knowledge
                .into_iter()
                .map(|item| KnowledgeItem {
                    source_agent: item.source_agent.to_string(),
                    content: item.content,
                })
                .collect(),
        }
    }
}

#[derive(Deserialize)]
pub struct ListWebsQuery {
    pub state: Option<String>,
    pub limit: Option<usize>,
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
    Query(query): Query<ListWebsQuery>,
) -> Result<Json<Vec<WebResponse>>, ApiError> {
    let state_filter = query
        .state
        .as_ref()
        .and_then(|s| match s.to_lowercase().as_str() {
            "running" => Some(WebState::Running),
            "converged" => Some(WebState::Converged),
            "failed" => Some(WebState::Failed),
            _ => None,
        });

    let webs = storage.list_webs(state_filter).await?;
    let limit = query.limit.unwrap_or(100);

    Ok(Json(
        webs.into_iter()
            .take(limit)
            .map(WebResponse::from)
            .collect(),
    ))
}

pub async fn get_web(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebDetailResponse>, ApiError> {
    let web = storage
        .get_web(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Web {} not found", id)))?;

    let agents = storage.get_web_agents(id).await?;
    let signals = storage.get_pending_signals(id).await?;

    Ok(Json(WebDetailResponse {
        id: web.id.to_string(),
        task: web.task,
        state: format!("{:?}", web.state),
        root_agent_id: web.root_agent.to_string(),
        agent_count: agents.len(),
        pending_signal_count: signals.len(),
    }))
}

pub async fn get_web_results(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebResultsResponse>, ApiError> {
    let web = storage
        .get_web(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Web {} not found", id)))?;

    let root_agent = storage
        .get_agent(web.root_agent)
        .await?
        .ok_or_else(|| ApiError::NotFound("Root agent not found".to_string()))?;

    Ok(Json(WebResultsResponse {
        web_id: web.id.to_string(),
        state: format!("{:?}", web.state),
        accumulated_knowledge: root_agent
            .context
            .accumulated_knowledge
            .into_iter()
            .map(|item| KnowledgeItem {
                source_agent: item.source_agent.to_string(),
                content: item.content,
            })
            .collect(),
    }))
}

pub async fn get_web_agents(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AgentResponse>>, ApiError> {
    let _web = storage
        .get_web(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Web {} not found", id)))?;

    let agents = storage.get_web_agents(id).await?;

    Ok(Json(agents.into_iter().map(AgentResponse::from).collect()))
}

pub async fn get_web_signals(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SignalResponse>>, ApiError> {
    let _web = storage
        .get_web(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Web {} not found", id)))?;

    let signals = storage.get_pending_signals(id).await?;

    Ok(Json(
        signals.into_iter().map(SignalResponse::from).collect(),
    ))
}

pub async fn terminate_web(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebResponse>, ApiError> {
    let mut web = storage
        .get_web(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Web {} not found", id)))?;

    web.state = WebState::Failed;
    storage.update_web(&web).await?;

    Ok(Json(WebResponse::from(web)))
}

pub async fn get_agent(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Json<AgentDetailResponse>, ApiError> {
    let agent = storage
        .get_agent(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", id)))?;

    let children = storage.get_children(id).await?;

    Ok(Json(AgentDetailResponse {
        id: agent.id.to_string(),
        web_id: agent.web_id.to_string(),
        parent_id: agent.parent_id.map(|id| id.to_string()),
        purpose: agent.purpose,
        capability: format!("{:?}", agent.capability),
        state: format!("{:?}", agent.state),
        health: agent.health,
        activation_threshold: agent.activation_threshold,
        probation_remaining: agent.probation_remaining,
        created_at: agent.created_at.to_rfc3339(),
        last_active_at: agent.last_active_at.to_rfc3339(),
        children_count: children.len(),
    }))
}

pub async fn get_agent_context(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ContextResponse>, ApiError> {
    let agent = storage
        .get_agent(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", id)))?;

    Ok(Json(ContextResponse::from(agent.context)))
}

pub async fn stream_web_events(
    State(storage): State<Arc<dyn Storage>>,
    Path(id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let _web = storage
        .get_web(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Web {} not found", id)))?;

    let stream = async_stream::stream! {
        let mut last_agent_count = 0;
        let mut last_signal_count = 0;
        let mut iteration = 0;

        loop {
            iteration += 1;

            if let Ok(Some(web)) = storage.get_web(id).await {
                if let Ok(agents) = storage.get_web_agents(id).await {
                    if agents.len() > last_agent_count {
                        for agent in agents.iter().skip(last_agent_count) {
                            let event_data = serde_json::json!({
                                "type": "agent_spawned",
                                "agent_id": agent.id.to_string(),
                                "purpose": agent.purpose,
                                "capability": format!("{:?}", agent.capability),
                            });
                            yield Ok(Event::default()
                                .event("agent_spawned")
                                .data(event_data.to_string()));
                        }
                        last_agent_count = agents.len();
                    }
                }

                if let Ok(signals) = storage.get_pending_signals(id).await {
                    if signals.len() != last_signal_count {
                        let event_data = serde_json::json!({
                            "type": "signals_updated",
                            "pending_count": signals.len(),
                        });
                        yield Ok(Event::default()
                            .event("signals_updated")
                            .data(event_data.to_string()));
                        last_signal_count = signals.len();
                    }
                }

                if web.state != WebState::Running {
                    let event_data = serde_json::json!({
                        "type": "web_state_changed",
                        "state": format!("{:?}", web.state),
                    });
                    yield Ok(Event::default()
                        .event("web_state_changed")
                        .data(event_data.to_string()));
                    break;
                }
            } else {
                break;
            }

            if iteration > 1000 {
                break;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

pub async fn get_config() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "default_activation_threshold": 0.6,
        "default_attenuation": 0.8,
        "max_agents": 50,
        "max_signal_hops": 10,
    }))
}
