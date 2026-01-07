use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use arachnid::api::{serve, AppState};
use arachnid::capabilities::{
    search::SearchCapability, synthesizer::SynthesizerCapability, Capability, Providers,
};
use arachnid::engine::coordination::CoordinationEngine;
use arachnid::providers::embedding::{EmbeddingProvider, OpenAIEmbeddingProvider};
use arachnid::providers::llm::{AnthropicProvider, LLMProvider, OpenAIProvider};
use arachnid::providers::search::{BraveSearchProvider, SearchProvider};
use arachnid::storage::memory::{InMemoryStore, WebStore};
use arachnid::storage::postgres::PostgresStorage;
use arachnid::storage::Storage;
use arachnid::types::{Agent, CapabilityType, Signal, SignalDirection, Web, WebConfig, WebState};
use arachnid::Config;

#[derive(Parser)]
#[command(name = "arachnid")]
#[command(author, version, about = "Autonomous agent coordination runtime", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a task and return results
    Run {
        /// The task to execute
        task: String,

        /// Watch mode: show live progress
        #[arg(long)]
        watch: bool,

        /// Output format
        #[arg(long, default_value = "text", value_enum)]
        output: OutputFormat,

        /// Timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
    },

    /// Start the HTTP API server
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },

    /// Show status of current/recent webs
    Status {
        /// Show detailed status
        #[arg(long)]
        detailed: bool,

        /// Filter by state
        #[arg(long)]
        state: Option<String>,

        /// Maximum number of webs to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Inspect a web
    Web {
        /// Web ID
        id: Uuid,

        #[command(subcommand)]
        action: Option<WebAction>,
    },

    /// Inspect an agent
    Agent {
        /// Agent ID
        id: Uuid,

        /// Show accumulated context
        #[arg(long)]
        context: bool,

        /// Show emitted signals
        #[arg(long)]
        signals: bool,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Database migrations
    Migrate {
        /// Show migration status only
        #[arg(long)]
        status: bool,

        /// Rollback last migration
        #[arg(long)]
        rollback: bool,
    },

    /// Validate configuration
    ValidateConfig,

    /// Show version information
    Version {
        /// Show detailed version info
        #[arg(long)]
        detailed: bool,
    },
}

#[derive(Subcommand)]
enum WebAction {
    /// Get web results
    Results,
    /// List agents in web
    Agents,
    /// List signals in web
    Signals,
    /// Terminate the web
    Terminate,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Quiet,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            task,
            watch,
            output,
            timeout,
        } => run_task(&task, watch, output, timeout, cli.verbose).await?,
        Commands::Serve { port, host } => run_serve(port, &host).await?,
        Commands::Status {
            detailed,
            state,
            limit,
        } => run_status(detailed, state, limit).await?,
        Commands::Web { id, action } => run_web(id, action).await?,
        Commands::Agent {
            id,
            context,
            signals,
        } => run_agent(id, context, signals).await?,
        Commands::Config { action } => run_config(action)?,
        Commands::Migrate { status, rollback } => run_migrate(status, rollback).await?,
        Commands::ValidateConfig => run_validate_config()?,
        Commands::Version { detailed } => run_version(detailed)?,
    }

    Ok(())
}

async fn run_task(
    task: &str,
    watch: bool,
    output: OutputFormat,
    timeout_secs: u64,
    verbose: bool,
) -> Result<()> {
    let config = Config::from_env();
    let store = Arc::new(InMemoryStore::new());

    let embedding_provider: Option<Box<dyn EmbeddingProvider>> =
        if let Some(api_key) = config.openai_api_key.clone() {
            Some(Box::new(OpenAIEmbeddingProvider::new(api_key)))
        } else {
            None
        };

    let llm_provider: Option<Box<dyn LLMProvider>> =
        if let Some(api_key) = config.anthropic_api_key.clone() {
            Some(Box::new(AnthropicProvider::new(api_key)))
        } else if let Some(api_key) = config.openai_api_key.clone() {
            Some(Box::new(OpenAIProvider::new(api_key)))
        } else {
            None
        };

    let search_provider: Option<Box<dyn SearchProvider>> =
        if let Some(api_key) = config.brave_api_key.clone() {
            Some(Box::new(BraveSearchProvider::new(api_key)))
        } else {
            None
        };

    let providers = Providers {
        embedding: embedding_provider,
        llm: llm_provider,
        search: search_provider,
    };

    let mut capabilities: HashMap<CapabilityType, Box<dyn Capability>> = HashMap::new();
    capabilities.insert(CapabilityType::Search, Box::new(SearchCapability::new()));
    capabilities.insert(
        CapabilityType::Synthesizer,
        Box::new(SynthesizerCapability::new()),
    );

    let task_embedding = if let Some(provider) = &providers.embedding {
        provider.embed(task).await?
    } else {
        vec![1.0; 1536]
    };

    let web_id = uuid::Uuid::new_v4();
    let root_agent = Agent::new(
        web_id,
        None,
        task.to_string(),
        task_embedding.clone(),
        CapabilityType::Synthesizer,
        0.6,
    );

    let web = Web {
        id: web_id,
        root_agent: root_agent.id,
        task: task.to_string(),
        state: arachnid::types::WebState::Running,
        config: WebConfig::default(),
    };

    WebStore::create_web(&*store, web.clone())?;
    WebStore::add_agent(&*store, root_agent.clone())?;

    let initial_signal = Signal::new(
        root_agent.id,
        task_embedding,
        task.to_string(),
        SignalDirection::Downward,
    );
    WebStore::add_signal(&*store, initial_signal)?;

    match output {
        OutputFormat::Text => {
            println!("Starting web {} for task: {}", web.id, task);
            println!("Root agent: {}", root_agent.id);
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "event": "started",
                    "web_id": web.id,
                    "root_agent_id": root_agent.id,
                    "task": task
                })
            );
        }
        OutputFormat::Quiet => {}
    }

    if providers.llm.is_none() {
        print_warning(
            &output,
            "No LLM provider configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY",
        );
    }
    if providers.search.is_none() {
        print_warning(&output, "No search provider configured. Set BRAVE_API_KEY");
    }

    let engine = CoordinationEngine::new(store.clone(), capabilities, providers);

    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    // Run coordination loop with timeout
    let coordination_result = tokio::time::timeout(timeout, async {
        if watch {
            run_with_watch(&engine, &web.id, &store, &output, verbose).await
        } else {
            engine.run_coordination_loop(&web.id).await
        }
    })
    .await;

    let elapsed = start.elapsed();

    match coordination_result {
        Ok(Ok(())) => {
            let final_web = WebStore::get_web(&*store, &web.id)?.expect("Web not found");
            let agents = WebStore::get_agents_by_web(&*store, &web.id)?;

            match output {
                OutputFormat::Text => {
                    println!("\nCompleted in {:.1}s", elapsed.as_secs_f32());
                    println!("Web state: {:?}", final_web.state);
                    println!("Total agents created: {}", agents.len());

                    for agent in &agents {
                        println!(
                            "  - {} ({:?}, {:?})",
                            truncate(&agent.purpose, 50),
                            agent.capability,
                            agent.state
                        );
                    }

                    if let Some(root) = agents.iter().find(|a| a.is_root()) {
                        if !root.context.accumulated_knowledge.is_empty() {
                            println!("\nAccumulated knowledge at root:");
                            for (i, item) in root.context.accumulated_knowledge.iter().enumerate() {
                                println!("  {}. {}", i + 1, item.content);
                            }
                        }
                    }
                }
                OutputFormat::Json => {
                    let root_knowledge: Vec<String> = agents
                        .iter()
                        .find(|a| a.is_root())
                        .map(|r| {
                            r.context
                                .accumulated_knowledge
                                .iter()
                                .map(|k| k.content.clone())
                                .collect()
                        })
                        .unwrap_or_default();

                    println!(
                        "{}",
                        serde_json::json!({
                            "event": "completed",
                            "web_id": web.id,
                            "state": format!("{:?}", final_web.state),
                            "duration_secs": elapsed.as_secs_f32(),
                            "agent_count": agents.len(),
                            "output": root_knowledge
                        })
                    );
                }
                OutputFormat::Quiet => {
                    if let Some(root) = agents.iter().find(|a| a.is_root()) {
                        for item in &root.context.accumulated_knowledge {
                            println!("{}", item.content);
                        }
                    }
                }
            }
        }
        Ok(Err(e)) => {
            return Err(e).context("Coordination loop failed");
        }
        Err(_) => {
            match output {
                OutputFormat::Text => {
                    println!("\nTimeout after {}s", timeout_secs);
                }
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::json!({
                            "event": "timeout",
                            "web_id": web.id,
                            "timeout_secs": timeout_secs
                        })
                    );
                }
                OutputFormat::Quiet => {}
            }
            return Err(anyhow::anyhow!("Task timed out after {}s", timeout_secs));
        }
    }

    Ok(())
}

async fn run_with_watch<S: WebStore>(
    engine: &CoordinationEngine<S>,
    web_id: &Uuid,
    store: &Arc<InMemoryStore>,
    output: &OutputFormat,
    verbose: bool,
) -> Result<()> {
    let mut reported_agents: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    let mut last_signal_count = 0;

    loop {
        let web =
            WebStore::get_web(&**store, web_id)?.ok_or_else(|| anyhow::anyhow!("Web not found"))?;
        let agents = WebStore::get_agents_by_web(&**store, web_id)?;
        let signals = WebStore::get_pending_signals(&**store, web_id)?;

        // Report new agents
        for agent in &agents {
            if !reported_agents.contains(&agent.id) {
                reported_agents.insert(agent.id);
                match output {
                    OutputFormat::Text => {
                        println!(
                            "  [+] Spawned: {} ({:?})",
                            truncate(&agent.purpose, 40),
                            agent.capability
                        );
                    }
                    OutputFormat::Json => {
                        println!(
                            "{}",
                            serde_json::json!({
                                "event": "agent_spawned",
                                "agent_id": agent.id,
                                "purpose": agent.purpose,
                                "capability": format!("{:?}", agent.capability)
                            })
                        );
                    }
                    OutputFormat::Quiet => {}
                }
            }
        }

        // Report signal activity
        if verbose && signals.len() != last_signal_count {
            match output {
                OutputFormat::Text => {
                    if signals.len() > last_signal_count {
                        for signal in signals.iter().skip(last_signal_count) {
                            println!(
                                "  --> Signal: \"{}\" (amplitude: {:.2})",
                                truncate(&signal.content, 40),
                                signal.amplitude
                            );
                        }
                    }
                }
                OutputFormat::Json => {
                    for signal in signals.iter().skip(last_signal_count) {
                        println!(
                            "{}",
                            serde_json::json!({
                                "event": "signal",
                                "signal_id": signal.id,
                                "content": signal.content,
                                "amplitude": signal.amplitude
                            })
                        );
                    }
                }
                OutputFormat::Quiet => {}
            }
            last_signal_count = signals.len();
        }

        // Check if converged
        if web.state != WebState::Running {
            break;
        }

        // Run one iteration of coordination
        let should_continue = engine.run_single_iteration(web_id).await?;
        if !should_continue {
            break;
        }

        // Small delay to prevent busy loop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

async fn run_serve(port: u16, host: &str) -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").ok();

    let storage: Arc<dyn Storage> = if let Some(url) = database_url {
        println!("Connecting to PostgreSQL...");
        let pg = PostgresStorage::new(&url)
            .await
            .context("Failed to connect to PostgreSQL")?;
        Arc::new(pg)
    } else {
        println!("No DATABASE_URL set, using in-memory storage");
        Arc::new(InMemoryStore::new())
    };

    let state = AppState { storage };

    println!("Starting Arachnid API server on {}:{}", host, port);
    serve(state, port).await
}

async fn run_status(detailed: bool, state_filter: Option<String>, limit: usize) -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").ok();

    let storage: Arc<dyn Storage> = if let Some(url) = database_url {
        Arc::new(
            PostgresStorage::new(&url)
                .await
                .context("Failed to connect to PostgreSQL")?,
        )
    } else {
        println!("Note: No DATABASE_URL set. Showing empty status (no persistent storage).");
        Arc::new(InMemoryStore::new())
    };

    let state = state_filter
        .as_ref()
        .and_then(|s| match s.to_lowercase().as_str() {
            "running" => Some(WebState::Running),
            "converged" => Some(WebState::Converged),
            "failed" => Some(WebState::Failed),
            _ => None,
        });

    let webs = storage.list_webs(state).await?;

    if webs.is_empty() {
        println!("No webs found.");
        return Ok(());
    }

    println!("Recent webs:");
    println!("{:-<80}", "");

    for web in webs.iter().take(limit) {
        let agent_count = storage.get_web_agents(web.id).await?.len();
        let signal_count = storage.get_pending_signals(web.id).await?.len();

        println!("Web: {}", web.id);
        println!("  Task: {}", truncate(&web.task, 60));
        println!("  State: {:?}", web.state);
        println!(
            "  Agents: {}, Pending signals: {}",
            agent_count, signal_count
        );

        if detailed {
            let agents = storage.get_web_agents(web.id).await?;
            for agent in agents.iter().take(5) {
                println!(
                    "    - {} ({:?}, health: {:.2})",
                    truncate(&agent.purpose, 40),
                    agent.state,
                    agent.health
                );
            }
            if agents.len() > 5 {
                println!("    ... and {} more agents", agents.len() - 5);
            }
        }
        println!();
    }

    if webs.len() > limit {
        println!(
            "... and {} more webs (use --limit to show more)",
            webs.len() - limit
        );
    }

    Ok(())
}

async fn run_web(id: Uuid, action: Option<WebAction>) -> Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL required for web inspection")?;

    let storage = PostgresStorage::new(&database_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    let web = storage
        .get_web(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Web {} not found", id))?;

    match action {
        None => {
            // Show web details
            let agents = storage.get_web_agents(id).await?;
            let signals = storage.get_pending_signals(id).await?;

            println!("Web: {}", web.id);
            println!("Task: {}", web.task);
            println!("State: {:?}", web.state);
            println!("Root Agent: {}", web.root_agent);
            println!("Agent Count: {}", agents.len());
            println!("Pending Signals: {}", signals.len());
        }
        Some(WebAction::Results) => {
            let root = storage
                .get_agent(web.root_agent)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Root agent not found"))?;

            println!("Results for web {}:", web.id);
            println!("State: {:?}", web.state);
            println!();

            if root.context.accumulated_knowledge.is_empty() {
                println!("No accumulated knowledge yet.");
            } else {
                println!("Accumulated Knowledge:");
                for (i, item) in root.context.accumulated_knowledge.iter().enumerate() {
                    println!("{}. {}", i + 1, item.content);
                    println!();
                }
            }
        }
        Some(WebAction::Agents) => {
            let agents = storage.get_web_agents(id).await?;

            println!("Agents in web {} ({} total):", web.id, agents.len());
            println!("{:-<80}", "");

            for agent in &agents {
                let children = storage.get_children(agent.id).await?;
                println!("Agent: {}", agent.id);
                println!("  Purpose: {}", truncate(&agent.purpose, 60));
                println!("  Capability: {:?}", agent.capability);
                println!("  State: {:?}", agent.state);
                println!("  Health: {:.2}", agent.health);
                println!("  Children: {}", children.len());
                if agent.parent_id.is_some() {
                    println!("  Parent: {}", agent.parent_id.unwrap());
                }
                println!();
            }
        }
        Some(WebAction::Signals) => {
            let signals = storage.get_pending_signals(id).await?;

            println!(
                "Pending signals in web {} ({} total):",
                web.id,
                signals.len()
            );
            println!("{:-<80}", "");

            for signal in &signals {
                println!("Signal: {}", signal.id);
                println!("  Origin: {}", signal.origin);
                println!("  Content: {}", truncate(&signal.content, 60));
                println!("  Direction: {:?}", signal.direction);
                println!("  Amplitude: {:.2}", signal.amplitude);
                println!("  Hop Count: {}", signal.hop_count);
                println!();
            }
        }
        Some(WebAction::Terminate) => {
            // Update web state to Failed (termination)
            let mut updated_web = web.clone();
            updated_web.state = WebState::Failed;
            storage.update_web(&updated_web).await?;
            println!("Web {} terminated.", id);
        }
    }

    Ok(())
}

async fn run_agent(id: Uuid, show_context: bool, show_signals: bool) -> Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL required for agent inspection")?;

    let storage = PostgresStorage::new(&database_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    let agent = storage
        .get_agent(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent {} not found", id))?;

    println!("Agent: {}", agent.id);
    println!("Web: {}", agent.web_id);
    println!("Purpose: {}", agent.purpose);
    println!("Capability: {:?}", agent.capability);
    println!("State: {:?}", agent.state);
    println!("Health: {:.2}", agent.health);
    println!("Activation Threshold: {:.2}", agent.activation_threshold);
    println!("Probation Remaining: {}", agent.probation_remaining);
    println!("Created: {}", agent.created_at);
    println!("Last Active: {}", agent.last_active_at);

    if let Some(parent_id) = agent.parent_id {
        println!("Parent: {}", parent_id);
    } else {
        println!("Parent: None (root agent)");
    }

    let children = storage.get_children(id).await?;
    println!("Children: {}", children.len());

    if show_context {
        println!();
        println!(
            "Accumulated Knowledge ({} items):",
            agent.context.accumulated_knowledge.len()
        );
        for (i, item) in agent.context.accumulated_knowledge.iter().enumerate() {
            println!(
                "  {}. [{}] {}",
                i + 1,
                item.source_agent,
                truncate(&item.content, 60)
            );
        }
    }

    if show_signals {
        println!();
        let signals = storage.get_pending_signals(agent.web_id).await?;
        let agent_signals: Vec<_> = signals.iter().filter(|s| s.origin == id).collect();
        println!("Signals from this agent ({}):", agent_signals.len());
        for signal in agent_signals {
            println!(
                "  - {} ({:?}, amp: {:.2})",
                truncate(&signal.content, 40),
                signal.direction,
                signal.amplitude
            );
        }
    }

    Ok(())
}

fn run_config(action: ConfigAction) -> Result<()> {
    let config = Config::from_env();

    match action {
        ConfigAction::Show => {
            println!("Current Configuration:");
            println!("{:-<40}", "");
            println!(
                "Anthropic API Key: {}",
                if config.anthropic_api_key.is_some() {
                    "[set]"
                } else {
                    "[not set]"
                }
            );
            println!(
                "OpenAI API Key: {}",
                if config.openai_api_key.is_some() {
                    "[set]"
                } else {
                    "[not set]"
                }
            );
            println!(
                "Brave API Key: {}",
                if config.brave_api_key.is_some() {
                    "[set]"
                } else {
                    "[not set]"
                }
            );
            println!(
                "Database URL: {}",
                if std::env::var("DATABASE_URL").is_ok() {
                    "[set]"
                } else {
                    "[not set]"
                }
            );
        }
        ConfigAction::Path => {
            println!("Configuration is loaded from environment variables:");
            println!("  ANTHROPIC_API_KEY");
            println!("  OPENAI_API_KEY");
            println!("  BRAVE_API_KEY");
            println!("  DATABASE_URL");
        }
    }

    Ok(())
}

async fn run_migrate(status_only: bool, rollback: bool) -> Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL required for migrations")?;

    if rollback {
        println!("Rollback is not yet implemented.");
        println!("To rollback, manually run SQL against your database.");
        return Ok(());
    }

    println!("Connecting to database...");
    let storage = PostgresStorage::new(&database_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    if status_only {
        println!("Migration status check...");
        println!("Available migrations:");
        println!("  - V001__initial_schema.sql");
        println!("  - V002__add_validations.sql");
        println!("  - V010__agent_definitions.sql");
        println!();
        println!("Note: Run without --status to apply migrations.");
        return Ok(());
    }

    println!("Running migrations...");
    storage.run_migrations().await?;
    println!("Migrations completed successfully.");

    Ok(())
}

fn run_validate_config() -> Result<()> {
    let config = Config::from_env();
    let mut errors: Vec<String> = vec![];
    let mut warnings: Vec<String> = vec![];

    // Check for at least one LLM provider
    if config.anthropic_api_key.is_none() && config.openai_api_key.is_none() {
        errors.push(
            "No LLM provider configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY.".to_string(),
        );
    }

    // Check for embedding provider
    if config.openai_api_key.is_none() {
        warnings.push(
            "No embedding provider configured (OPENAI_API_KEY). Will use dummy embeddings."
                .to_string(),
        );
    }

    // Check for search provider
    if config.brave_api_key.is_none() {
        warnings.push(
            "No search provider configured (BRAVE_API_KEY). Search capability will not work."
                .to_string(),
        );
    }

    // Check for database
    if std::env::var("DATABASE_URL").is_err() {
        warnings
            .push("No DATABASE_URL set. Will use in-memory storage (not persistent).".to_string());
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("Configuration is valid.");
        return Ok(());
    }

    if !warnings.is_empty() {
        println!("Warnings:");
        for w in &warnings {
            println!("  - {}", w);
        }
    }

    if !errors.is_empty() {
        println!("Errors:");
        for e in &errors {
            println!("  - {}", e);
        }
        return Err(anyhow::anyhow!("Configuration validation failed"));
    }

    println!();
    println!("Configuration is valid (with warnings).");
    Ok(())
}

fn run_version(detailed: bool) -> Result<()> {
    println!("arachnid {}", env!("CARGO_PKG_VERSION"));

    if detailed {
        println!();
        println!("Build Information:");
        println!("  Package: {}", env!("CARGO_PKG_NAME"));
        println!("  Version: {}", env!("CARGO_PKG_VERSION"));
        println!("  Authors: {}", env!("CARGO_PKG_AUTHORS"));
        println!("  License: {}", env!("CARGO_PKG_LICENSE"));
        println!("  Repository: {}", env!("CARGO_PKG_REPOSITORY"));
        println!();
        println!("Runtime Information:");
        println!("  Rust version: {}", rustc_version());
        println!("  Target: {}", std::env::consts::ARCH);
        println!("  OS: {}", std::env::consts::OS);
    }

    Ok(())
}

fn print_warning(output: &OutputFormat, message: &str) {
    match output {
        OutputFormat::Text => println!("Warning: {}", message),
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "event": "warning",
                    "message": message
                })
            );
        }
        OutputFormat::Quiet => {}
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn rustc_version() -> &'static str {
    // This would ideally be set at compile time
    "stable"
}
