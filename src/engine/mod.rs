pub mod coordination;
pub mod executor;
pub mod lifecycle_management;
pub mod propagation;
pub mod resonance;
pub mod spawning;

pub use executor::{AgentExecutionResult, AgentExecutor, ExecutorConfig};
pub use lifecycle_management::{ConvergenceDetector, LifecycleManager};
