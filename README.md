# Arachnid

Autonomous agent coordination through semantic resonance.

## Overview

Arachnid is a runtime where AI agents self-organize to accomplish tasks. Instead of explicit orchestration, agents activate based on signal relevance, like vibrations propagating through a spider web.

## Status: v1.0 Release Candidate

Complete autonomous agent coordination system with lifecycle management, PostgreSQL persistence, validation service, HTTP API, and comprehensive capabilities.

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

See [API Reference](.contexts/api-reference.md) for full documentation.

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

72 unit tests covering:
- Core coordination and resonance
- Signal propagation with attenuation
- Lifecycle management and state transitions
- Health tracking with probation
- Tuning drift and adaptation
- Storage operations (memory and PostgreSQL)
- All capabilities
- Validation service
- HTTP API endpoints

## Documentation

- [Architecture Guide](.contexts/architecture.md) - System design and concepts
- [API Reference](.contexts/api-reference.md) - HTTP endpoints and SSE events
- [Configuration Guide](.contexts/configuration.md) - All configuration options
- [Capability Development](.contexts/capabilities.md) - Creating new capabilities

## Performance

Target benchmarks:
- Signal propagation: < 10ms per hop
- Resonance computation: < 5ms per agent
- Agent spawn time: < 100ms

## Security

- API keys stored in environment variables
- Sandboxed file operations (planned)
- Input validation on all endpoints
- No authentication by default (use reverse proxy for production)

See [Security Guide](.contexts/security.md) for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Write tests for your changes
4. Ensure all tests pass
5. Submit a pull request

## Roadmap

- [ ] Web UI for monitoring
- [ ] Additional embedding providers
- [ ] Code execution sandbox
- [ ] Streaming LLM responses
- [ ] Agent definition templates
- [ ] Performance optimizations

## v2.0 Architecture (In Progress)

Version 2.0 introduces a flexible definition/instance/tool model, moving from hardcoded capabilities to dynamic agent definitions.

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

MIT

## Acknowledgments

Built with:
- Rust async runtime (Tokio)
- PostgreSQL with pgvector extension
- Axum web framework
- Anthropic Claude API
- OpenAI API
- Brave Search API
