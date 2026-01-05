use anyhow::Result;
use async_trait::async_trait;
use pgvector::Vector;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::storage::traits::{FailurePattern, FailurePatternType, Storage};
use crate::types::{
    Agent, AgentContext, AgentId, AgentState, CapabilityType, Signal, SignalDirection, SignalId,
    Web, WebConfig, WebId, WebState,
};

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::query(include_str!("../../migrations/V001__initial_schema.sql"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn create_web(&self, web: &Web) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO webs (id, task, state, root_agent_id, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            "#,
        )
        .bind(web.id)
        .bind(&web.task)
        .bind(web.state.as_str())
        .bind(web.root_agent)
        .bind(serde_json::to_value(&web.config)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_web(&self, id: WebId) -> Result<Option<Web>> {
        let row = sqlx::query(
            r#"
            SELECT id, task, state, root_agent_id, config
            FROM webs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let config: WebConfig = serde_json::from_value(r.get("config"))?;
                let state_str: String = r.get("state");
                let state = match state_str.as_str() {
                    "Running" => WebState::Running,
                    "Converged" => WebState::Converged,
                    "Failed" => WebState::Failed,
                    _ => WebState::Running,
                };

                Ok(Some(Web {
                    id: r.get("id"),
                    task: r.get("task"),
                    state,
                    root_agent: r.get("root_agent_id"),
                    config,
                }))
            }
            None => Ok(None),
        }
    }

    async fn update_web(&self, web: &Web) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webs
            SET task = $2, state = $3, root_agent_id = $4, config = $5, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(web.id)
        .bind(&web.task)
        .bind(web.state.as_str())
        .bind(web.root_agent)
        .bind(serde_json::to_value(&web.config)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_webs(&self, state: Option<WebState>) -> Result<Vec<Web>> {
        let rows = match state {
            Some(s) => {
                sqlx::query(
                    r#"
                    SELECT id, task, state, root_agent_id, config
                    FROM webs
                    WHERE state = $1
                    ORDER BY created_at DESC
                    "#,
                )
                .bind(s.as_str())
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT id, task, state, root_agent_id, config
                    FROM webs
                    ORDER BY created_at DESC
                    "#,
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        rows.iter()
            .map(|r| {
                let config: WebConfig = serde_json::from_value(r.get("config"))?;
                let state_str: String = r.get("state");
                let state = match state_str.as_str() {
                    "Running" => WebState::Running,
                    "Converged" => WebState::Converged,
                    "Failed" => WebState::Failed,
                    _ => WebState::Running,
                };

                Ok(Web {
                    id: r.get("id"),
                    task: r.get("task"),
                    state,
                    root_agent: r.get("root_agent_id"),
                    config,
                })
            })
            .collect()
    }

    async fn create_agent(&self, agent: &Agent) -> Result<()> {
        let tuning_vec = Vector::from(agent.tuning.clone());

        sqlx::query(
            r#"
            INSERT INTO agents (
                id, web_id, parent_id, purpose, tuning, capability, state, health,
                activation_threshold, context, probation_remaining, created_at,
                last_active_at, dormant_since
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(agent.id)
        .bind(agent.web_id)
        .bind(agent.parent_id)
        .bind(&agent.purpose)
        .bind(tuning_vec)
        .bind(capability_to_str(&agent.capability))
        .bind(agent.state.as_str())
        .bind(agent.health)
        .bind(agent.activation_threshold)
        .bind(serde_json::to_value(&agent.context)?)
        .bind(agent.probation_remaining as i32)
        .bind(agent.created_at)
        .bind(agent.last_active_at)
        .bind(agent.dormant_since)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_agent(&self, id: AgentId) -> Result<Option<Agent>> {
        let row = sqlx::query(
            r#"
            SELECT id, web_id, parent_id, purpose, tuning, capability, state, health,
                   activation_threshold, context, probation_remaining, created_at,
                   last_active_at, dormant_since
            FROM agents
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(row_to_agent(&r)?)),
            None => Ok(None),
        }
    }

    async fn update_agent(&self, agent: &Agent) -> Result<()> {
        let tuning_vec = Vector::from(agent.tuning.clone());

        sqlx::query(
            r#"
            UPDATE agents
            SET web_id = $2, parent_id = $3, purpose = $4, tuning = $5, capability = $6,
                state = $7, health = $8, activation_threshold = $9, context = $10,
                probation_remaining = $11, last_active_at = $12, dormant_since = $13
            WHERE id = $1
            "#,
        )
        .bind(agent.id)
        .bind(agent.web_id)
        .bind(agent.parent_id)
        .bind(&agent.purpose)
        .bind(tuning_vec)
        .bind(capability_to_str(&agent.capability))
        .bind(agent.state.as_str())
        .bind(agent.health)
        .bind(agent.activation_threshold)
        .bind(serde_json::to_value(&agent.context)?)
        .bind(agent.probation_remaining as i32)
        .bind(agent.last_active_at)
        .bind(agent.dormant_since)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_children(&self, parent_id: AgentId) -> Result<Vec<Agent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, web_id, parent_id, purpose, tuning, capability, state, health,
                   activation_threshold, context, probation_remaining, created_at,
                   last_active_at, dormant_since
            FROM agents
            WHERE parent_id = $1
            "#,
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_agent).collect()
    }

    async fn get_ancestors(&self, agent_id: AgentId) -> Result<Vec<Agent>> {
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE ancestors AS (
                SELECT a.*, 0 as level
                FROM agents a
                WHERE a.id = $1
                UNION ALL
                SELECT p.*, anc.level + 1
                FROM agents p
                INNER JOIN ancestors anc ON p.id = anc.parent_id
            )
            SELECT id, web_id, parent_id, purpose, tuning, capability, state, health,
                   activation_threshold, context, probation_remaining, created_at,
                   last_active_at, dormant_since
            FROM ancestors
            WHERE level > 0
            ORDER BY level ASC
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_agent).collect()
    }

    async fn get_agents_by_state(&self, web_id: WebId, state: AgentState) -> Result<Vec<Agent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, web_id, parent_id, purpose, tuning, capability, state, health,
                   activation_threshold, context, probation_remaining, created_at,
                   last_active_at, dormant_since
            FROM agents
            WHERE web_id = $1 AND state = $2
            "#,
        )
        .bind(web_id)
        .bind(state.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_agent).collect()
    }

    async fn get_web_agents(&self, web_id: WebId) -> Result<Vec<Agent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, web_id, parent_id, purpose, tuning, capability, state, health,
                   activation_threshold, context, probation_remaining, created_at,
                   last_active_at, dormant_since
            FROM agents
            WHERE web_id = $1
            "#,
        )
        .bind(web_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_agent).collect()
    }

    async fn find_resonating_agents(
        &self,
        web_id: WebId,
        frequency: &[f32],
        threshold: f32,
    ) -> Result<Vec<(Agent, f32)>> {
        let frequency_vec = Vector::from(frequency.to_vec());

        let rows = sqlx::query(
            r#"
            SELECT
                id, web_id, parent_id, purpose, tuning, capability, state, health,
                activation_threshold, context, probation_remaining, created_at,
                last_active_at, dormant_since,
                1 - (tuning <=> $2::vector) as similarity
            FROM agents
            WHERE web_id = $1
              AND state NOT IN ('Terminated', 'WindingDown')
              AND 1 - (tuning <=> $2::vector) > $3
            ORDER BY similarity DESC
            "#,
        )
        .bind(web_id)
        .bind(frequency_vec)
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|r| {
                let agent = row_to_agent(r)?;
                let similarity: f32 = r.get("similarity");
                Ok((agent, similarity))
            })
            .collect()
    }

    async fn create_signal(&self, signal: &Signal) -> Result<()> {
        let frequency_vec = Vector::from(signal.frequency.clone());

        sqlx::query(
            r#"
            INSERT INTO signals (
                id, web_id, origin_agent_id, frequency, content, amplitude,
                direction, hop_count, payload, processed, created_at
            )
            SELECT $1, a.web_id, $2, $3, $4, $5, $6, $7, $8, false, NOW()
            FROM agents a
            WHERE a.id = $2
            "#,
        )
        .bind(signal.id)
        .bind(signal.origin)
        .bind(frequency_vec)
        .bind(&signal.content)
        .bind(signal.amplitude)
        .bind(direction_to_str(&signal.direction))
        .bind(signal.hop_count as i32)
        .bind(&signal.payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_pending_signals(&self, web_id: WebId) -> Result<Vec<Signal>> {
        let rows = sqlx::query(
            r#"
            SELECT id, origin_agent_id, frequency, content, amplitude, direction,
                   hop_count, payload
            FROM signals
            WHERE web_id = $1 AND processed = false
            ORDER BY created_at ASC
            "#,
        )
        .bind(web_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|r| {
                let freq_vec: Vector = r.get("frequency");
                let dir_str: String = r.get("direction");
                let direction = match dir_str.as_str() {
                    "Upward" => SignalDirection::Upward,
                    _ => SignalDirection::Downward,
                };

                Ok(Signal {
                    id: r.get("id"),
                    origin: r.get("origin_agent_id"),
                    frequency: freq_vec.to_vec(),
                    content: r.get("content"),
                    amplitude: r.get("amplitude"),
                    direction,
                    hop_count: r.get::<i32, _>("hop_count") as u32,
                    payload: r.get("payload"),
                })
            })
            .collect()
    }

    async fn mark_signal_processed(&self, id: SignalId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE signals
            SET processed = true
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn record_failure_pattern(&self, web_id: WebId, pattern: &FailurePattern) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO web_memory (id, web_id, pattern_type, pattern_data, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(pattern.id)
        .bind(web_id)
        .bind(pattern.pattern_type.as_str())
        .bind(&pattern.pattern_data)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_failure_patterns(&self, web_id: WebId) -> Result<Vec<FailurePattern>> {
        let rows = sqlx::query(
            r#"
            SELECT id, web_id, pattern_type, pattern_data, created_at
            FROM web_memory
            WHERE web_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(web_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|r| {
                let type_str: String = r.get("pattern_type");
                let pattern_type = match type_str.as_str() {
                    "AgentWindDown" => FailurePatternType::AgentWindDown,
                    "RepeatedValidationFailure" => FailurePatternType::RepeatedValidationFailure,
                    "CyclicSpawning" => FailurePatternType::CyclicSpawning,
                    _ => FailurePatternType::ResourceExhaustion,
                };

                Ok(FailurePattern {
                    id: r.get("id"),
                    web_id: r.get("web_id"),
                    pattern_type,
                    pattern_data: r.get("pattern_data"),
                    created_at: r.get("created_at"),
                })
            })
            .collect()
    }
}

fn row_to_agent(r: &sqlx::postgres::PgRow) -> Result<Agent> {
    let tuning_vec: Vector = r.get("tuning");
    let cap_str: String = r.get("capability");
    let state_str: String = r.get("state");

    let capability = match cap_str.as_str() {
        "Search" => CapabilityType::Search,
        "Synthesizer" => CapabilityType::Synthesizer,
        "CodeWriter" => CapabilityType::CodeWriter,
        "CodeReviewer" => CapabilityType::CodeReviewer,
        "Analyst" => CapabilityType::Analyst,
        s => CapabilityType::Custom(s.to_string()),
    };

    let state = match state_str.as_str() {
        "Active" => AgentState::Active,
        "Listening" => AgentState::Listening,
        "Dormant" => AgentState::Dormant,
        "Quarantine" => AgentState::Quarantine,
        "Isolated" => AgentState::Isolated,
        "WindingDown" => AgentState::WindingDown,
        "Terminated" => AgentState::Terminated,
        _ => AgentState::Listening,
    };

    let context_json: serde_json::Value = r.get("context");
    let context: AgentContext = serde_json::from_value(context_json)?;

    Ok(Agent {
        id: r.get("id"),
        web_id: r.get("web_id"),
        parent_id: r.get("parent_id"),
        purpose: r.get("purpose"),
        tuning: tuning_vec.to_vec(),
        capability,
        state,
        health: r.get("health"),
        activation_threshold: r.get("activation_threshold"),
        context,
        probation_remaining: r.get::<i32, _>("probation_remaining") as u32,
        created_at: r.get("created_at"),
        last_active_at: r.get("last_active_at"),
        dormant_since: r.get("dormant_since"),
    })
}

fn capability_to_str(capability: &CapabilityType) -> String {
    match capability {
        CapabilityType::Search => "Search".to_string(),
        CapabilityType::Synthesizer => "Synthesizer".to_string(),
        CapabilityType::CodeWriter => "CodeWriter".to_string(),
        CapabilityType::CodeReviewer => "CodeReviewer".to_string(),
        CapabilityType::Analyst => "Analyst".to_string(),
        CapabilityType::Custom(s) => s.clone(),
    }
}

fn direction_to_str(direction: &SignalDirection) -> String {
    match direction {
        SignalDirection::Upward => "Upward".to_string(),
        SignalDirection::Downward => "Downward".to_string(),
    }
}

impl WebState {
    pub fn as_str(&self) -> &str {
        match self {
            WebState::Running => "Running",
            WebState::Converged => "Converged",
            WebState::Failed => "Failed",
        }
    }
}
