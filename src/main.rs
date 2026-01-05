use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;

use arachnid::engine::coordination::CoordinationEngine;
use arachnid::providers::embedding::{EmbeddingProvider, OpenAIEmbeddingProvider};
use arachnid::storage::memory::{InMemoryStore, WebStore};
use arachnid::types::{Agent, CapabilityType, Signal, SignalDirection, Web, WebConfig};
use arachnid::Config;

#[derive(Parser)]
#[command(name = "arachnid")]
#[command(about = "Autonomous agent coordination runtime", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(help = "Task description")]
        task: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { task } => run_task(&task).await?,
    }

    Ok(())
}

async fn run_task(task: &str) -> Result<()> {
    let config = Config::from_env();
    let store = Arc::new(InMemoryStore::new());

    let embedding_provider: Option<Box<dyn EmbeddingProvider>> =
        if let Some(api_key) = config.openai_api_key {
            Some(Box::new(OpenAIEmbeddingProvider::new(api_key)))
        } else {
            None
        };

    let task_embedding = if let Some(provider) = &embedding_provider {
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

    store.create_web(web.clone())?;
    store.add_agent(root_agent.clone())?;

    let initial_signal = Signal::new(
        root_agent.id,
        task_embedding,
        task.to_string(),
        SignalDirection::Downward,
    );
    store.add_signal(initial_signal)?;

    println!("Starting web {} for task: {}", web.id, task);
    println!("Root agent: {}", root_agent.id);

    let engine = CoordinationEngine::new(store.clone());
    engine.run_coordination_loop(&web.id).await?;

    let final_web = store.get_web(&web.id)?.expect("Web not found");
    println!("\nWeb completed with state: {:?}", final_web.state);

    let agents = store.get_agents_by_web(&web.id)?;
    println!("Total agents created: {}", agents.len());

    let all_signals = store.get_pending_signals(&web.id)?;
    println!("Pending signals: {}", all_signals.len());

    Ok(())
}
