# Arachnid

Autonomous agent coordination through semantic resonance.

## Overview

Arachnid is a runtime where AI agents self-organize to accomplish tasks. Instead of explicit orchestration, agents activate based on signal relevance, like vibrations propagating through a spider web.

## Status: v1.0 Stable

Complete autonomous agent coordination system with lifecycle management, PostgreSQL persistence, validation service, HTTP API, and comprehensive capabilities.

**Current version:** 1.0.0 (Phases 1-8 complete)

## Features

- **Resonance-based coordination**: Agents activate when signals resonate with their tuning
- **Lifecycle management**: Health tracking, probation periods, state machines, graceful degradation
- **Signal propagation**: Upward/downward signal flow with attenuation and hop counting
- **Validation service**: LLM-based output quality assurance with risk prioritization
- **Multiple capabilities**: Search, Synthesizer, Code Writer, Code Reviewer, Analyst
- **PostgreSQL + pgvector**: Persistent storage with vector similarity search
- **HTTP API**: RESTful endpoints with Server-Sent Events for real-time updates
- **Local LLM support**: Ollama provider for running models locally

## Quick Start

### Installation

```bash
cargo install arachnid
```

### Configuration

Set your API keys:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
export OPENAI_API_KEY=sk-...
export BRAVE_API_KEY=BSA...
export DATABASE_URL=postgres://user:pass@localhost:5432/arachnid
```

### Usage

```bash
# Run a research task
arachnid run "What are the main approaches to quantum error correction?"

# Watch progress in real-time
arachnid run --watch "Analyze the competitive landscape for AI coding tools"

# Start API server
arachnid serve --port 8080

# Check status of recent webs
arachnid status

# View agent details
arachnid agent <agent-id> --context
```

## Architecture

Arachnid uses a web-based coordination model:

1. **Web**: A task execution environment with a root agent
2. **Agents**: Specialized workers with capabilities and health tracking
3. **Signals**: Messages with semantic embeddings propagating through the web
4. **Resonance**: Cosine similarity between agent tuning and signal frequency

### Agent Lifecycle

Agents transition through states based on health and activity:
- **Active**: Currently executing
- **Listening**: Waiting for resonant signals
- **Dormant**: Idle, can be reactivated
- **Quarantine**: Low health (< 0.6), signals marked suspect
- **Isolated**: Very low health (< 0.4), signals dampened
- **WindingDown**: Terminal, transferring state
- **Terminated**: Removed from web

### Capabilities

- **Search**: Web search using Brave API
- **Synthesizer**: Multi-source information synthesis
- **CodeWriter**: Code generation with LLM
- **CodeReviewer**: Security and quality review
- **Analyst**: Data analysis and insight extraction

## HTTP API

```bash
# Create a web
curl -X POST http://localhost:8080/webs \\
  -H "Content-Type: application/json" \\
  -d '{"task": "Research quantum computing"}'

# Stream events
curl http://localhost:8080/webs/{id}/events

# Get results
curl http://localhost:8080/webs/{id}/results
```

Endpoint handlers are defined in `src/api/handlers.rs`.

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Check lints
cargo clippy --all-features -- -D warnings

# Run database migrations
cargo run -- migrate

# Validate configuration
cargo run -- validate-config
```

## Database Setup

```bash
# Install PostgreSQL with pgvector
# macOS
brew install postgresql pgvector

# Create database
createdb arachnid

# Run migrations
cargo run -- migrate
```

## Provider Configuration

### Anthropic (Claude)
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

### OpenAI (GPT-4, Embeddings)
```bash
export OPENAI_API_KEY=sk-...
```

### Ollama (Local LLM)
```bash
export OLLAMA_URL=http://localhost:11434
export OLLAMA_MODEL=llama3.1
```

### Brave Search
```bash
export BRAVE_API_KEY=BSA...
```

## Test Coverage

146 tests across three targets: 112 unit tests, 12 definition-architecture integration
tests (`tests/phase9_integration.rs`), and 22 tool integration tests (`tests/tools/`).

Coverage includes:
- Core coordination and resonance
- Signal propagation with attenuation
- Lifecycle management and state transitions
- Health tracking with probation
- Tuning drift and adaptation
- Storage operations (memory and PostgreSQL)
- All capabilities
- Validation service
- HTTP API endpoints
- Tool runtime: schema exposure, permission filtering, file operations, fetch, and
  coordination tools

## Documentation

Reference documentation has not been written yet. Until it is, the Architecture and
Runtime Hardening sections of this file are the current design record, and the source is
the reference: `src/engine/` for coordination and resonance, `src/definitions/` for the
agent definition format, `src/tools/` for the tool runtime, and `src/api/` for HTTP
endpoints and SSE events.

## Performance

Target benchmarks:
- Signal propagation: < 10ms per hop
- Resonance computation: < 5ms per agent
- Agent spawn time: < 100ms

## Security

- API keys stored in environment variables
- Each agent works in its own sandbox directory (`<sandbox_root>/<web_id>/<agent_id>`), so
  agents cannot read or overwrite each other's files. File tools reject paths that escape
  it, and reject a context whose sandbox falls outside the configured root
- Code execution runs on a remote host over SSH rather than in-process
- Tool access is restricted per agent by the definition's tool allowlist
- Input validation on all endpoints
- No authentication by default (use reverse proxy for production)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Write tests for your changes
4. Ensure all tests pass
5. Submit a pull request

## Roadmap

- [ ] Concurrent signal processing (see Runtime Hardening)
- [ ] Artifact references in accumulated context (see Runtime Hardening)
- [ ] Durable coordination state with resume (see Runtime Hardening)
- [ ] Structured definition output contracts (see Runtime Hardening)
- [ ] Tiered model routing
- [ ] Web UI for monitoring
- [ ] Additional embedding providers
- [ ] Local sandboxed code execution (currently dispatches to a remote host over SSH)
- [ ] `query_database` tool
- [ ] Wire the definition/tool path into the coordination engine (Phase 11)
- [ ] Streaming LLM responses
- [ ] Agent definition templates
- [ ] Performance optimizations

## Runtime Hardening (Planned)

Arachnid coordinates agents through emergent topology: no execution graph is authored
in advance, and the active structure condenses at runtime from resonance between signal
frequencies and agent tuning. That design choice is deliberate and is not up for
revision. The items below are the orthogonal concerns - concurrency, state durability,
context transport, and output typing - that any coordination runtime needs regardless of
whether its topology is authored or emergent. Each one is a gap between what the runtime
currently does and what the resonance model already assumes.

### 1. Concurrent signal processing

The coordination loop currently drains pending signals sequentially
(`src/engine/coordination.rs`), awaiting each `process_signal` call before starting the
next. Agents that resonate with independent signals therefore execute one at a time. The
resonance model already establishes that these activations are independent - the loop
just does not exploit it.

Planned: process a signal batch concurrently with bounded fan-out, tolerating individual
activation failures without aborting the batch. Blocked on `WebStore`, which is a
synchronous trait over `Arc<RwLock<HashMap<..>>>`: it holds blocking locks inside async
tasks, and its read-modify-write sequences (`get_agent` then `update_agent`) are not
atomic, so concurrent activations touching the same agent or parent context would drop
writes.

### 2. Artifact references in accumulated context

`accumulate_context_from_signal` clones full signal content into the parent's
`accumulated_knowledge`. Content is copied again at each hop, so context grows with tree
depth and upward traffic, and a deep web can exhaust the root agent's window with retold
material.

Planned: persist signal payloads once and propagate an artifact reference plus a short
structured summary. The `Artifact` enum in `src/tools/mod.rs` already models the
reference type; it needs a storage path and a resolution step so a receiving agent can
read the original rather than a compression of it.

### 3. Durable coordination state with resume

Web execution state lives in the in-memory `WebStore` that `CoordinationEngine` is
generic over, while definitions and agent records use the separate Postgres-backed
`Storage` trait. Live coordination state is not on the durable path, so an interrupted
web cannot be resumed and long-running work is lost on restart.

Planned: converge the two storage abstractions, checkpoint iteration state and signal
queues after expensive operations, and add a resume path that reconstructs a web from its
last checkpoint. Tool side effects need idempotency keys so replay after a checkpoint does
not duplicate writes.

### 4. Structured definition output contracts

`AgentDefinition` specifies a bounded job, a tuning vector, a system prompt, and a tool
allowlist, but says nothing about output shape. `AgentExecutor::execute` returns an
untyped value and recovers signals by parsing free text, so a malformed response
propagates downstream before anything can reject it.

Planned: add an optional output schema and declared failure states to the definition
format, validate executor output against the schema, and route validation failures to the
existing repair and health-degradation paths rather than onward through the web.

### 5. Tiered model routing

Every agent resolves to a single configured LLM provider. Bounded work such as
classification, extraction, and formatting costs the same as decomposition and synthesis.

Planned: let definitions declare a model tier, and select the provider per activation.

### Considered and declined

Authoring the execution graph explicitly - predeclared nodes, static edges, and a routing
table - would make the system inspectable in ways emergent topology is not. It is declined
because it replaces the project's central mechanism. Resonance is the router; making
routes static removes the reason arachnid exists rather than improving it. The
inspectability concern is real and is addressed instead by recording resonance scores and
activation decisions in durable state (item 3), so any route the system took can be
explained after the fact.

## v2.0 Architecture (In Development)

Version 2.0 introduces a flexible definition/instance/tool model, moving from hardcoded
capabilities to dynamic agent definitions.

**Development Status:** Phases 9-10 substantially complete, Phase 11 pending
- Phase 9 (Definition Architecture): schema, builtin definition, generator, factory, and
  definition storage implemented
- Phase 10 (Tool Runtime): 8 of 9 tools implemented - web_search, fetch_url, read_file,
  write_file, execute_code, emit_signal, spawn_agent, search_codebase. `query_database`
  is declared in `ToolType` but not yet implemented. Runtime filters tool schemas by the
  definition's allowlist, so definitions control tool access
- Phase 11 (Integration): pending - CLI and API still drive the v1 capability path, and
  user custom definitions are not yet loaded

The v1 `Capability` implementations in `src/capabilities/` remain the live execution path.
The definition/tool architecture runs alongside it and is not yet wired into the
coordination engine.

### Code Execution

`execute_code` does not sandbox locally. It dispatches to a remote execution host over SSH
via `ImpresarioClient` (`src/tools/impresario_client.rs`), configured with
`IMPRESARIO_HOST`, `IMPRESARIO_PORT`, `IMPRESARIO_USER`, and `IMPRESARIO_KEY`. Isolation
is a property of that remote host, not of the local process.

### Key Changes

**Agent Definitions** - YAML templates that describe agents:
- Tuning keywords for resonance matching
- System prompts and temperature settings
- Available tools (web_search, spawn_agent, emit_signal, etc.)
- Source: built-in, user custom, or LLM-generated

**Agent Instances** - Running agents that reference definitions:
- Maintain individual state and health
- Can drift from base definition over time
- Created by Agent Factory based on needs

**Tool Runtime** - Rust-implemented actions:
- Information: web_search, fetch_url, read_file, search_codebase
- Output: write_file, emit_signal
- Coordination: spawn_agent
- Execution: execute_code (sandboxed)

### Benefits

- **Flexibility**: Agents adapt to any task via generated definitions
- **Reusability**: Generated definitions cached for future use
- **Extensibility**: Users can add custom definitions
- **Efficiency**: Dormant agents reactivate instead of spawning new ones

### Custom Definitions

Users can create custom agent definitions in `~/.arachnid/agents/custom/`:

```yaml
name: security-reviewer
version: 1.0.0

tuning:
  keywords:
    - security vulnerabilities
    - code review
    - SQL injection

llm:
  system_prompt: |
    You are a security expert reviewing code.
    Use emit_signal to report issues found.
  temperature: 0.3

tools:
  - read_file
  - search_codebase
  - emit_signal
```

## License

Apache-2.0

## Acknowledgments

Built with:
- Rust async runtime (Tokio)
- PostgreSQL with pgvector extension
- Axum web framework
- Anthropic Claude API
- OpenAI API
- Brave Search API
