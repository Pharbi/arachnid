use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::sync::Arc;

use arachnid::capabilities::{
    search::SearchCapability, synthesizer::SynthesizerCapability, Capability, Providers,
};
use arachnid::engine::coordination::CoordinationEngine;
use arachnid::providers::embedding::{EmbeddingProvider, OpenAIEmbeddingProvider};
use arachnid::providers::llm::{AnthropicProvider, LLMProvider, OpenAIProvider};
use arachnid::providers::search::{BraveSearchProvider, SearchProvider};
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
        if let Some(api_key) = config.openai_api_key.clone() {
            Some(Box::new(OpenAIEmbeddingProvider::new(api_key)))
        } else {
            None
        };

    let llm_provider: Option<Box<dyn LLMProvider>> = if let Some(api_key) = config.anthropic_api_key.clone() {
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

    if providers.llm.is_none() {
        println!("Warning: No LLM provider configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY");
    }
    if providers.search.is_none() {
        println!("Warning: No search provider configured. Set BRAVE_API_KEY");
    }

    let engine = CoordinationEngine::new(store.clone(), capabilities, providers);
    engine.run_coordination_loop(&web.id).await?;

    let final_web = store.get_web(&web.id)?.expect("Web not found");
    println!("\nWeb completed with state: {:?}", final_web.state);

    let agents = store.get_agents_by_web(&web.id)?;
    println!("Total agents created: {}", agents.len());

    for agent in &agents {
        println!(
            "  - {} ({:?}, {:?})",
            agent.purpose, agent.capability, agent.state
        );
    }

    let all_signals = store.get_pending_signals(&web.id)?;
    println!("\nPending signals: {}", all_signals.len());

    if let Some(root) = agents.iter().find(|a| a.is_root()) {
        if !root.context.accumulated_knowledge.is_empty() {
            println!("\nAccumulated knowledge at root:");
            for (i, item) in root.context.accumulated_knowledge.iter().enumerate() {
                println!("  {}. {}", i + 1, item.content);
            }
        }
    }

    Ok(())
}
