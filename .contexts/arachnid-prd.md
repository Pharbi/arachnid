# Arachnid: Product Requirements Document

**Version:** 0.1  
**Last Updated:** January 2026  
**Author:** Kwesi Yankson

---

## 1. Overview

### 1.1 Problem Statement

Current multi-agent AI systems require explicit orchestration. Developers must either:

1. **Design workflows upfront** — Define agents, connections, and handoffs before execution. Breaks when problems don't match the predetermined structure.

2. **Use LLM-as-orchestrator** — A manager LLM decides routing at runtime. Every coordination decision costs tokens and latency. The orchestrator becomes a bottleneck and often misses cross-cutting concerns.

Both approaches separate structure from work. Coordination requires constant explicit decisions.

### 1.2 Solution

Arachnid is an autonomous coordination runtime where agents self-organize through resonance. Inspired by how spider webs transmit vibrations, Arachnid enables:

- **Implicit coordination** — Agents activate based on signal relevance, not explicit calls
- **Emergent structure** — The web topology grows from the work itself
- **Automatic cross-cutting** — Concerns like security and testing surface through resonance
- **Self-maintenance** — Health-based lifecycle management without manual intervention

### 1.3 Key Differentiators

| Aspect                 | Traditional Approaches | Arachnid                      |
| ---------------------- | ---------------------- | ----------------------------- |
| Coordination           | Explicit orchestration | Resonance-based activation    |
| Routing decisions      | Developer or LLM       | Signal-tuning similarity      |
| Structure              | Designed upfront       | Emerges from work             |
| Cross-cutting concerns | Manual wiring          | Automatic through resonance   |
| Agent lifecycle        | Manual management      | Health-based self-maintenance |

---

## 2. Goals and Non-Goals

### 2.1 Goals

1. **Autonomous coordination loop** — Arachnid runs the spawn → work → signal → resonate → activate loop without external orchestration

2. **Emergent agent organization** — Web structure reflects problem structure without upfront design

3. **Cross-cutting concern detection** — Related agents activate automatically through resonance

4. **Self-maintaining system** — Health tracking, validation, and lifecycle management are automatic

5. **Provider agnostic** — Users bring their own LLM, embedding, and search provider keys

6. **Simple interfaces** — CLI for direct use, HTTP API for integration

7. **Open source** — MIT licensed, community can extend and contribute

### 2.2 Non-Goals (v1)

1. **Inter-web communication** — Multiple webs communicating with each other (future)

2. **Visual UI** — No graphical interface for v1 (CLI and API only)

3. **Multi-tenancy** — Single-user runtime for v1

4. **Hosted service** — Users run Arachnid themselves

5. **Custom capability plugins** — Predefined capabilities only for v1

6. **Real-time collaboration** — No multiplayer features

---

## 3. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Arachnid Runtime                            │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Coordination Engine                       │ │
│  │                                                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │   Signal    │  │  Resonance  │  │     Lifecycle       │ │ │
│  │  │ Propagation │  │  Matching   │  │    Management       │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  │                                                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │    Web      │  │   Health    │  │     Validation      │ │ │
│  │  │  (DAG)      │  │  Tracking   │  │      Service        │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  │                                                             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Agent Execution                           │ │
│  │                                                             │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      │ │
│  │  │  Search  │ │   Code   │ │  Review  │ │  Analyze │      │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘      │ │
│  │                              │                              │ │
│  │                              ▼                              │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │              Provider Adapters                        │  │ │
│  │  │  LLM: Anthropic | OpenAI | Ollama | Azure | Bedrock  │  │ │
│  │  │  Embeddings: OpenAI | Cohere | Ollama                │  │ │
│  │  │  Search: Brave | Tavily | SerpAPI                    │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                      Storage                                │ │
│  │           PostgreSQL + pgvector (or in-memory)             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
              ▼                ▼                ▼
         ┌─────────┐     ┌─────────┐     ┌─────────────┐
         │   CLI   │     │HTTP API │     │ (Future)    │
         │         │     │         │     │  Bindings   │
         └─────────┘     └─────────┘     └─────────────┘
```

---

## 4. Core Concepts

### 4.1 Web

A Web is a directed acyclic graph (DAG) of agents created for a specific task. The web grows through spawning and represents both the task decomposition and the communication topology.

```rust
struct Web {
    id: Uuid,
    root_agent: AgentId,
    task: String,
    state: WebState,          // Running, Converged, Failed
    created_at: DateTime,
    config: WebConfig,
}

struct WebConfig {
    attenuation_factor: f32,      // Default: 0.8
    min_amplitude: f32,           // Default: 0.1
    default_threshold: f32,       // Default: 0.6
    max_agents: usize,            // Default: 100
    max_depth: usize,             // Default: 10
    idle_timeout_secs: u64,       // Default: 30
    dormant_ttl_secs: u64,        // Default: 600
}
```

### 4.2 Agent

An agent is a node in the web that can do work and emit signals. Each agent has a single capability and a tuning vector that determines what signals it resonates with.

```rust
struct Agent {
    id: AgentId,
    web_id: WebId,
    parent_id: Option<AgentId>,

    // Identity
    purpose: String,              // Human-readable description
    tuning: Vec<f32>,             // Embedding vector
    capability: CapabilityType,

    // State
    state: AgentState,
    health: f32,                  // 0.0 to 1.0
    activation_threshold: f32,    // Per-agent threshold

    // Lifecycle
    created_at: DateTime,
    last_active_at: DateTime,
    probation_remaining: u32,     // Reduced penalties during probation

    // Context
    context: AgentContext,        // Accumulated knowledge
}

enum AgentState {
    Active,       // Currently working
    Listening,    // Waiting for signals
    Dormant,      // Idle, can reactivate
    Quarantine,   // Low health, signals marked suspect
    Isolated,     // Very low health, signals dampened
    WindingDown,  // Terminal, transferring state
    Terminated,   // Gone
}

struct AgentContext {
    purpose: String,
    accumulated_knowledge: Vec<ContextItem>,
    failure_warnings: Vec<String>,    // From similar failed agents
}
```

### 4.3 Signal

A signal is a semantic vibration that propagates through the web. Signals carry meaning (frequency), urgency (amplitude), and optional data (payload).

```rust
struct Signal {
    id: Uuid,
    origin: AgentId,

    // Semantic content
    frequency: Vec<f32>,          // Embedding of signal content
    content: String,              // Human-readable content

    // Propagation
    amplitude: f32,               // Starts at 1.0, decays
    direction: SignalDirection,
    hop_count: u32,

    // Optional data
    payload: Option<serde_json::Value>,

    created_at: DateTime,
}

enum SignalDirection {
    Upward,     // Results flowing toward root
    Downward,   // Needs flowing toward leaves
}
```

### 4.4 Resonance

Resonance determines whether an agent activates in response to a signal.

```rust
struct ResonanceResult {
    agent_id: AgentId,
    signal_id: SignalId,
    similarity: f32,              // Cosine similarity
    effective_strength: f32,      // similarity × amplitude
    activated: bool,              // effective_strength > threshold
}

fn compute_resonance(agent: &Agent, signal: &Signal) -> ResonanceResult {
    let similarity = cosine_similarity(&agent.tuning, &signal.frequency);
    let effective_strength = similarity * signal.amplitude;
    let activated = effective_strength > agent.activation_threshold;

    ResonanceResult {
        agent_id: agent.id,
        signal_id: signal.id,
        similarity,
        effective_strength,
        activated,
    }
}
```

### 4.5 Capability

A capability defines what an agent can do. Each agent has exactly one capability.

```rust
trait Capability: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    async fn execute(
        &self,
        context: &AgentContext,
        trigger: Option<&Signal>,
        providers: &Providers,
    ) -> Result<ExecutionResult>;
}

struct ExecutionResult {
    status: ExecutionStatus,
    output: serde_json::Value,
    artifacts: Vec<Artifact>,
    signals_to_emit: Vec<SignalDraft>,
    needs: Vec<Need>,             // May trigger spawning
}

enum ExecutionStatus {
    Complete,
    NeedsMore(String),            // Requires additional work
    Failed(String),
}

struct Need {
    description: String,
    suggested_capability: Option<CapabilityType>,
}
```

---

## 5. Functional Requirements

### 5.1 Signal Propagation

**FR-SIG-01:** Signals propagate along DAG edges only (strict lineage).

**FR-SIG-02:** Signals have a direction:

- Upward: From child toward root (results, completions)
- Downward: From parent toward children (needs, queries)

**FR-SIG-03:** Signal amplitude attenuates by `attenuation_factor` (default 0.8) per hop.

**FR-SIG-04:** Signals with amplitude below `min_amplitude` (default 0.1) stop propagating.

**FR-SIG-05:** Signal propagation is depth-first, processing each branch fully before moving to siblings.

### 5.2 Resonance and Activation

**FR-RES-01:** Resonance is computed as:

```
similarity = cosine_similarity(agent.tuning, signal.frequency)
effective_strength = similarity × signal.amplitude
activated = effective_strength > agent.activation_threshold
```

**FR-RES-02:** Each agent has its own `activation_threshold`, defaulting to web's `default_threshold`.

**FR-RES-03:** When an agent activates:

1. State changes to `Active`
2. Capability executes with signal as trigger
3. Agent may emit new signals
4. Agent may express needs (triggering spawn check)

**FR-RES-04:** If no agent resonates with a signal, it dies when amplitude falls below minimum.

### 5.3 Spawning

**FR-SPN-01:** Spawning occurs when an agent expresses a need and no existing agent in lineage resonates.

**FR-SPN-02:** Before spawning, check lineage (ancestors and descendants) for resonating agents.

**FR-SPN-03:** New agents are created with:

- `parent_id` = spawning agent
- `tuning` = embedding of the need
- `health` = 1.0
- `probation_remaining` = configured probation period
- `context` = derived from parent context + need
- `failure_warnings` = relevant warnings from web memory

**FR-SPN-04:** Spawning creates a DAG edge (parent → child).

**FR-SPN-05:** Web enforces `max_agents` and `max_depth` limits.

### 5.4 Tuning Drift

**FR-DFT-01:** Agent tuning evolves based on signals it successfully responds to:

```
new_tuning = α × old_tuning + (1 - α) × avg(recent_signals)
```

**FR-DFT-02:** α = 0.8 (slow drift, configurable).

**FR-DFT-03:** Drift considers last 15 signals responded to.

**FR-DFT-04:** Drift only occurs on successful execution (not on failed activations).

### 5.5 Agent Lifecycle

**FR-LCY-01:** Agent states and transitions:

```
                    ┌─────────────────────────────────┐
                    │                                 │
                    ▼                                 │
Spawned ──► Active ◄──► Listening ──► Dormant ───────┤
                │                         │          │
                │        health < 0.6     │          │
                └────────────┬────────────┘          │
                             ▼                       │
                        Quarantine                   │
                             │                       │
                │        health < 0.4                │
                             ▼                       │
                         Isolated                    │
                             │                       │
                │        health < 0.2                │
                             ▼                       │
                       WindingDown                   │
                             │                       │
                             ▼                       │
                        Terminated ◄─────────────────┘
                                     (TTL expired while dormant)
```

**FR-LCY-02:** Active → Listening: After completing current work.

**FR-LCY-03:** Listening → Dormant: After `idle_timeout_secs` with no activation.

**FR-LCY-04:** Dormant → Terminated: After `dormant_ttl_secs` without reactivation.

**FR-LCY-05:** Any state → Quarantine: When health drops below 0.6.

**FR-LCY-06:** Quarantine → Isolated: When health drops below 0.4.

**FR-LCY-07:** Isolated → WindingDown: When health drops below 0.2.

**FR-LCY-08:** WindingDown process:

1. Notify parent with failure summary
2. Orphan healthy children to grandparent
3. Cascade wind-down for unhealthy children
4. Record failure patterns in web memory
5. Terminate

### 5.6 Health Management

**FR-HLT-01:** Health is a float from 0.0 to 1.0.

**FR-HLT-02:** Health changes:

- Validation confirms output: +0.05
- Validation challenges output: -0.15
- Inconsistent with previous similar outputs: additional -0.05

**FR-HLT-03:** Agents in probation period receive reduced penalties (50%).

**FR-HLT-04:** Health affects signal propagation:

- Quarantine: Signals marked as suspect
- Isolated: Signal amplitude × 0.3

### 5.7 Validation Service

**FR-VAL-01:** Validation is triggered based on risk:

```
priority = impact × (1 - agent_health) × uncertainty
```

**FR-VAL-02:** Not all outputs are validated. Validation samples based on:

- Agent health (lower health = more validation)
- Output impact (code execution > text summary)
- Resource budget

**FR-VAL-03:** Validation produces judgments: Confirm, Challenge, Uncertain.

**FR-VAL-04:** Validation results update agent health per FR-HLT-02.

### 5.8 Web Lifecycle

**FR-WEB-01:** Web states: Initializing, Running, Converged, Failed, Terminated.

**FR-WEB-02:** Convergence detection:

- No active agents
- No pending signals
- Root agent has produced output

**FR-WEB-03:** Failure conditions:

- Root agent health drops below 0.2
- Max agents exceeded with no progress
- Timeout exceeded

**FR-WEB-04:** Web termination cleans up all agents and resources.

---

## 6. Technical Decisions

### 6.1 Confirmed Decisions

| Decision                | Choice                          | Rationale                                     |
| ----------------------- | ------------------------------- | --------------------------------------------- |
| Language                | Rust                            | Performance, safety, single binary deployment |
| Signal propagation      | Strict lineage only             | Simplicity, maintains "structure is meaning"  |
| Activation threshold    | Per-agent                       | Allows agents to be more/less sensitive       |
| Tuning drift            | α=0.8, 15 signals, success only | Conservative drift, proven work only          |
| Spawning                | Check lineage first             | Reuse existing agents when possible           |
| Lifecycle               | Health-based + TTL              | Self-maintaining, no manual cleanup           |
| Validation              | Risk-based sampling             | Balance thoroughness with cost                |
| Capability model        | Single capability per agent     | Simplicity, spawn specialists                 |
| Inter-web communication | Not in v1                       | Defer complexity                              |
| Embeddings              | OpenAI text-embedding-3-small   | Quality, cost, swappable later                |
| Storage (v1)            | PostgreSQL + pgvector           | DAG + vectors in one store                    |
| LLM provider            | Configurable                    | User brings their own keys                    |
| Primary interface       | CLI                             | Direct use, scriptable                        |
| Secondary interface     | HTTP API                        | Integration with other systems                |

### 6.2 Configuration Parameters

| Parameter                  | Default | Description                         |
| -------------------------- | ------- | ----------------------------------- |
| `attenuation_factor`       | 0.8     | Signal amplitude decay per hop      |
| `min_amplitude`            | 0.1     | Threshold for signal death          |
| `default_threshold`        | 0.6     | Default activation threshold        |
| `max_agents`               | 100     | Maximum agents per web              |
| `max_depth`                | 10      | Maximum DAG depth                   |
| `idle_timeout_secs`        | 30      | Time before Listening → Dormant     |
| `dormant_ttl_secs`         | 600     | Time before Dormant → Terminated    |
| `tuning_drift_alpha`       | 0.8     | Tuning drift rate                   |
| `tuning_drift_window`      | 15      | Number of signals for drift         |
| `health_boost_confirm`     | 0.05    | Health gain on validation confirm   |
| `health_penalty_challenge` | 0.15    | Health loss on validation challenge |
| `probation_period`         | 5       | Executions before full penalties    |
| `quarantine_threshold`     | 0.6     | Health threshold for quarantine     |
| `isolation_threshold`      | 0.4     | Health threshold for isolation      |
| `winddown_threshold`       | 0.2     | Health threshold for wind-down      |

---

## 7. Interfaces

### 7.1 CLI Commands

```bash
# Core operations
arachnid run <task>              # Run a task, return results
arachnid run --watch <task>      # Run with live progress output

# Web inspection
arachnid status                  # Show current/recent webs
arachnid web <id>                # Show web details
arachnid web <id> --agents       # List agents in web
arachnid web <id> --signals      # Show signal history

# Agent inspection
arachnid agent <id>              # Show agent details
arachnid agent <id> --context    # Show accumulated context
arachnid agent <id> --signals    # Show emitted signals

# Configuration
arachnid config list             # Show all config
arachnid config get <key>        # Get config value
arachnid config set <key> <val>  # Set config value

# Server mode
arachnid serve                   # Start HTTP API server
arachnid serve --port 8080       # Specify port

# Utilities
arachnid validate-config         # Check configuration
arachnid version                 # Show version
```

### 7.2 CLI Output Formats

```bash
# Default: human-readable
$ arachnid run "Research quantum computing"
Starting web abc123...
  Spawned: root-agent (research coordinator)
  Spawned: searcher-1 (quantum computing basics)
  Signal: "quantum error correction" → searcher-1 resonated
  ...
Results saved to ./arachnid-output/abc123/

# JSON output for scripting
$ arachnid run --output json "Research quantum computing"
{"web_id": "abc123", "status": "converged", "results": {...}}

# Quiet mode
$ arachnid run --quiet "Research quantum computing"
./arachnid-output/abc123/
```

### 7.3 HTTP API Endpoints

```
POST   /webs                    Create and run a new web
GET    /webs                    List webs
GET    /webs/:id                Get web status
GET    /webs/:id/results        Get web results
GET    /webs/:id/agents         List agents in web
GET    /webs/:id/signals        Get signal history
GET    /webs/:id/events         SSE stream of web events
DELETE /webs/:id                Terminate web

GET    /agents/:id              Get agent details
GET    /agents/:id/context      Get agent context

GET    /health                  Runtime health check
GET    /config                  Get current configuration
```

### 7.4 HTTP API Examples

```bash
# Create a web
curl -X POST http://localhost:8080/webs \
  -H "Content-Type: application/json" \
  -d '{"task": "Research quantum error correction"}'

# Response
{
  "id": "web-abc123",
  "task": "Research quantum error correction",
  "state": "running",
  "created_at": "2026-01-04T12:00:00Z"
}

# Stream events
curl -N http://localhost:8080/webs/web-abc123/events

# Events (SSE)
event: agent_spawned
data: {"agent_id": "agent-1", "purpose": "search quantum papers"}

event: signal_emitted
data: {"signal_id": "sig-1", "content": "found surface code papers"}

event: resonance
data: {"agent_id": "agent-2", "signal_id": "sig-1", "strength": 0.74}

event: web_converged
data: {"web_id": "web-abc123", "agent_count": 8}
```

---

## 8. Capabilities (v1)

### 8.1 Search Capability

**Purpose:** Find information from the web.

**Inputs:** Query derived from purpose or triggering signal.

**Outputs:**

- List of relevant URLs with snippets
- Extracted key concepts
- Signals for discovered concepts

**Provider:** Brave Search API (configurable).

### 8.2 Synthesizer Capability

**Purpose:** Combine information from multiple sources into coherent output.

**Inputs:** Accumulated context from child agents.

**Outputs:**

- Synthesized summary
- Key findings
- Source attribution

**Provider:** LLM.

### 8.3 Code Writer Capability

**Purpose:** Write code based on specifications.

**Inputs:** Requirements from context or triggering signal.

**Outputs:**

- Code files
- Signals for concepts (functions, imports, patterns)
- Needs for testing, review

**Provider:** LLM.

### 8.4 Code Reviewer Capability

**Purpose:** Review code for issues.

**Inputs:** Code from triggering signal or context.

**Outputs:**

- Review findings
- Signals for issues found
- Approval or rejection

**Provider:** LLM.

### 8.5 Analyst Capability

**Purpose:** Analyze data or documents.

**Inputs:** Data or documents from context.

**Outputs:**

- Analysis findings
- Patterns discovered
- Signals for key insights

**Provider:** LLM.

---

## 9. Work Breakdown

### Phase 1: Foundation (Weeks 1-2)

**Milestone: Core types and in-memory runtime**

| Task | Description                                 | Estimate |
| ---- | ------------------------------------------- | -------- |
| 1.1  | Core types: Agent, Signal, Web, Resonance   | 2 days   |
| 1.2  | Embedding integration (OpenAI adapter)      | 1 day    |
| 1.3  | Cosine similarity and resonance computation | 0.5 days |
| 1.4  | In-memory web storage                       | 1 day    |
| 1.5  | Signal propagation (strict lineage)         | 2 days   |
| 1.6  | Basic coordination loop                     | 2 days   |
| 1.7  | Simple CLI (run command only)               | 1 day    |
| 1.8  | Unit tests for core logic                   | 1.5 days |

**Deliverable:** Can run `arachnid run "test"` with hardcoded capability, see signals propagate.

---

### Phase 2: Agent Execution (Weeks 3-4)

**Milestone: Agents can do useful work**

| Task | Description                                  | Estimate |
| ---- | -------------------------------------------- | -------- |
| 2.1  | LLM provider adapter (Anthropic)             | 1 day    |
| 2.2  | LLM provider adapter (OpenAI)                | 0.5 days |
| 2.3  | Search capability (Brave)                    | 2 days   |
| 2.4  | Synthesizer capability                       | 1.5 days |
| 2.5  | Capability trait and execution               | 1 day    |
| 2.6  | Agent context accumulation                   | 1 day    |
| 2.7  | Spawning logic (check lineage, create child) | 2 days   |
| 2.8  | Configuration system                         | 1 day    |
| 2.9  | Integration tests                            | 1 day    |

**Deliverable:** Can run research tasks end-to-end, get useful results.

---

### Phase 3: Lifecycle Management (Weeks 5-6)

**Milestone: Self-maintaining system**

| Task | Description                               | Estimate |
| ---- | ----------------------------------------- | -------- |
| 3.1  | Agent state machine                       | 1.5 days |
| 3.2  | Health tracking                           | 1 day    |
| 3.3  | Idle timeout (Listening → Dormant)        | 0.5 days |
| 3.4  | TTL expiration (Dormant → Terminated)     | 0.5 days |
| 3.5  | Health thresholds (Quarantine, Isolation) | 1 day    |
| 3.6  | Wind-down process                         | 1.5 days |
| 3.7  | Web convergence detection                 | 1 day    |
| 3.8  | Tuning drift implementation               | 1.5 days |
| 3.9  | Tests for lifecycle                       | 1.5 days |

**Deliverable:** Agents manage their own lifecycle, unhealthy agents wind down.

---

### Phase 4: Persistence (Weeks 7-8)

**Milestone: State survives restarts**

| Task | Description                                | Estimate |
| ---- | ------------------------------------------ | -------- |
| 4.1  | PostgreSQL schema design                   | 1 day    |
| 4.2  | pgvector setup for embeddings              | 0.5 days |
| 4.3  | Storage trait abstraction                  | 1 day    |
| 4.4  | PostgreSQL adapter (agents, signals, webs) | 2 days   |
| 4.5  | Web memory (failure patterns)              | 1 day    |
| 4.6  | Migration system                           | 0.5 days |
| 4.7  | Recovery on restart                        | 1.5 days |
| 4.8  | Database tests                             | 1.5 days |

**Deliverable:** Can stop and restart runtime, state persists.

---

### Phase 5: Validation Service (Weeks 9-10)

**Milestone: Output quality assurance**

| Task | Description                          | Estimate |
| ---- | ------------------------------------ | -------- |
| 5.1  | Validation service architecture      | 1 day    |
| 5.2  | Risk-based validation prioritization | 1 day    |
| 5.3  | Validation execution (LLM-based)     | 1.5 days |
| 5.4  | Health updates from validation       | 1 day    |
| 5.5  | Probation period logic               | 0.5 days |
| 5.6  | Validation storage and history       | 1 day    |
| 5.7  | Integration with coordination loop   | 1.5 days |
| 5.8  | Validation tests                     | 1.5 days |

**Deliverable:** Outputs are validated, health reflects validation results.

---

### Phase 6: HTTP API (Weeks 11-12)

**Milestone: External integration**

| Task | Description                  | Estimate |
| ---- | ---------------------------- | -------- |
| 6.1  | HTTP server setup (axum)     | 1 day    |
| 6.2  | Web CRUD endpoints           | 1.5 days |
| 6.3  | Agent endpoints              | 1 day    |
| 6.4  | SSE event streaming          | 1.5 days |
| 6.5  | Error handling and responses | 1 day    |
| 6.6  | API documentation            | 1 day    |
| 6.7  | API tests                    | 1.5 days |
| 6.8  | CLI `serve` command          | 0.5 days |

**Deliverable:** Full HTTP API, can integrate with external systems.

---

### Phase 7: Additional Capabilities (Weeks 13-14)

**Milestone: Broader task support**

| Task | Description                    | Estimate |
| ---- | ------------------------------ | -------- |
| 7.1  | Code Writer capability         | 2 days   |
| 7.2  | Code Reviewer capability       | 1.5 days |
| 7.3  | Analyst capability             | 1.5 days |
| 7.4  | Capability selection logic     | 1 day    |
| 7.5  | Ollama adapter (local LLM)     | 1 day    |
| 7.6  | Additional embedding providers | 1 day    |
| 7.7  | Capability tests               | 2 days   |

**Deliverable:** Can handle research, coding, and analysis tasks.

---

### Phase 8: Polish and Release (Weeks 15-16)

**Milestone: v1.0 release**

| Task | Description                       | Estimate |
| ---- | --------------------------------- | -------- |
| 8.1  | CLI polish (help, errors, output) | 2 days   |
| 8.2  | Documentation (README, guides)    | 2 days   |
| 8.3  | Example use cases                 | 1 day    |
| 8.4  | Performance testing               | 1 day    |
| 8.5  | Security review (key handling)    | 1 day    |
| 8.6  | CI/CD setup                       | 1 day    |
| 8.7  | Release packaging (cargo publish) | 0.5 days |
| 8.8  | Launch prep (GitHub, docs site)   | 1.5 days |

**Deliverable:** v1.0 released, documented, usable.

---

## 10. Success Criteria

### 10.1 Functional Success

- [ ] Research task runs end-to-end without intervention
- [ ] Cross-cutting agents activate through resonance (not explicit calls)
- [ ] Unhealthy agents wind down automatically
- [ ] Web converges and produces coherent output
- [ ] Can configure any supported LLM/embedding/search provider

### 10.2 Performance Targets

| Metric                     | Target          |
| -------------------------- | --------------- |
| Signal propagation latency | < 10ms per hop  |
| Resonance check            | < 5ms per agent |
| Agent spawn time           | < 100ms         |
| Memory per dormant agent   | < 10KB          |
| Typical research task      | < 5 minutes     |

### 10.3 Quality Targets

| Metric                    | Target                     |
| ------------------------- | -------------------------- |
| Unit test coverage        | > 80%                      |
| Integration test coverage | Key paths covered          |
| Documentation             | All public APIs documented |
| CLI help                  | All commands documented    |

### 10.4 Adoption Targets (6 months post-launch)

| Metric                   | Target             |
| ------------------------ | ------------------ |
| GitHub stars             | 500+               |
| Unique users (estimated) | 100+               |
| Community contributions  | 10+ PRs merged     |
| Issues resolved          | 80% within 2 weeks |

---

## 11. Risks and Mitigations

| Risk                                               | Impact | Likelihood | Mitigation                                      |
| -------------------------------------------------- | ------ | ---------- | ----------------------------------------------- |
| Resonance model doesn't surface useful connections | High   | Medium     | Validate early with real tasks; tune thresholds |
| LLM costs higher than expected                     | Medium | Medium     | Add cost tracking; optimize prompt efficiency   |
| Performance issues at scale                        | Medium | Low        | Profile early; design for optimization          |
| Rust learning curve deters contributors            | Medium | Medium     | Good docs; consider Python bindings later       |
| Provider API changes break adapters                | Low    | Medium     | Abstraction layer; version pinning              |

---

## 12. Future Considerations (Post v1)

- Inter-web communication (stigmergic artifacts)
- Visual web inspector
- Plugin system for custom capabilities
- Python bindings (PyO3)
- Multi-tenancy / hosted service
- Persistent long-running webs
- Human-in-the-loop intervention points
- Cost tracking and budgets
- Streaming output

---

## Appendix A: Glossary

| Term            | Definition                                                   |
| --------------- | ------------------------------------------------------------ |
| **Web**         | A DAG of agents created for a specific task                  |
| **Agent**       | A node in the web that can work and emit signals             |
| **Signal**      | A semantic vibration that propagates through the web         |
| **Resonance**   | The match between a signal's frequency and an agent's tuning |
| **Tuning**      | An embedding vector representing what an agent cares about   |
| **Capability**  | What an agent can do (search, code, review, etc.)            |
| **Activation**  | When resonance exceeds threshold, agent begins working       |
| **Attenuation** | Signal amplitude decay as it propagates                      |
| **Health**      | Agent reliability score, affects lifecycle                   |
| **Spawning**    | Creating a child agent to handle a need                      |
| **Wind-down**   | Terminal process for unhealthy agents                        |
| **Convergence** | When a web has no more work to do                            |

---

## Appendix B: References

- Spider web vibration research: Mortimer et al., "A Spider's Vibration Landscape" (2019)
- Stigmergy: Grassé, Pierre-Paul (1959)
- TDAG framework: Wang et al., "Dynamic Task Decomposition and Agent Generation" (2024)
- Semantic routing: Aurelio AI Semantic Router
- Emergent coordination: Riedl, "Emergent Coordination in Multi-Agent Language Models" (2025)

