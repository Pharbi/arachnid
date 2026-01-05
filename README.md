# Arachnid

An autonomous agent coordination runtime where agents self-organize through semantic resonance. Agents activate based on signal similarity rather than explicit routing, like vibrations propagating through a spider web.

## Status: Phase 2 Complete

Phase 2 adds real LLM/search integration with autonomous agent spawning and research capabilities.

## Features

**Core Infrastructure (Phase 1):**
- Signal propagation along agent DAG edges with configurable attenuation
- Resonance-based activation (agents activate when signal resonance exceeds threshold)
- Coordination loop managing agent lifecycle and signal processing
- In-memory HashMap-based storage for webs, agents, and signals

**Agent Capabilities (Phase 2):**
- **SearchCapability**: Web search using Brave Search API
- **SynthesizerCapability**: LLM-powered analysis and synthesis
- **Autonomous Spawning**: Agents spawn child agents for unmet needs
- **Lineage Check**: Before spawning, checks if existing lineage agent can handle the need
- **Context Accumulation**: Parent agents accumulate knowledge from child results

**Provider Integrations:**
- **LLM**: Anthropic Claude 3.5 Sonnet or OpenAI GPT-4o
- **Embeddings**: OpenAI text-embedding-3-small
- **Search**: Brave Search API

## Building

```bash
cargo build --release
```

## Running

```bash
# With all providers configured (recommended for research tasks)
export ANTHROPIC_API_KEY=sk-ant-...  # or OPENAI_API_KEY
export BRAVE_API_KEY=BSA...
export OPENAI_API_KEY=sk-...  # for embeddings
cargo run -- run "Research quantum computing error correction"

# Minimal (no external APIs)
cargo run -- run "test task"

# Version info
cargo run -- --version
```

## Testing

```bash
cargo test
```

## Architecture

```
arachnid/
├── src/
│   ├── types/          # Core data structures (Agent, Signal, Web)
│   ├── engine/         # Coordination loop, propagation, resonance, spawning
│   ├── storage/        # WebStore trait and in-memory implementation
│   ├── providers/      # External service adapters
│   │   ├── embedding.rs  # OpenAI embeddings
│   │   ├── llm.rs        # Anthropic/OpenAI LLM
│   │   └── search.rs     # Brave Search
│   └── capabilities/   # Agent capabilities
│       ├── search.rs      # SearchCapability
│       └── synthesizer.rs # SynthesizerCapability
```

## Configuration

Environment variables:
- `ANTHROPIC_API_KEY`: Anthropic API key for LLM (or use OPENAI_API_KEY)
- `OPENAI_API_KEY`: OpenAI API key for LLM/embeddings
- `BRAVE_API_KEY`: Brave Search API key

## How It Works

1. **Task Initiation**: Root agent (Synthesizer) receives the task
2. **Need Identification**: Synthesizer analyzes task and identifies research needs
3. **Spawning Decision**:
   - First checks lineage (ancestors + descendants) for agents that resonate with the need
   - If found, sends signal to existing agent
   - If not found, spawns new child agent with Search capability
4. **Research Execution**: Search agents query Brave API and emit result signals upward
5. **Context Accumulation**: Parent agents accumulate knowledge from child signals
6. **Synthesis**: Root synthesizer combines gathered knowledge into coherent summary
7. **Convergence**: Web converges when no active agents and no pending signals

## Example Flow

```
Research "quantum error correction"
  └─> Root Synthesizer identifies needs:
       ├─> "quantum error correction techniques"
       ├─> "surface codes and stabilizer codes"
       └─> "practical implementations"
            └─> Each spawns Search agent
                 └─> Search results flow up as signals
                      └─> Root synthesizes final answer
```

## Phase Implementation Details

### Phase 1
- Core types: Agent, Signal, Web with supporting enums
- Cosine similarity for vector comparison
- Signal attenuation (default: 0.8 per hop)
- Minimum amplitude threshold (default: 0.1)
- Agent activation threshold (default: 0.6)
- Maximum depth limit (default: 10)
- Maximum agents per web (default: 100)

### Phase 2
- LLMProvider trait with Anthropic (Claude 3.5 Sonnet) and OpenAI (GPT-4o) adapters
- SearchProvider trait with Brave Search API adapter
- Capability trait for pluggable agent behaviors
- ExecutionResult with signals and needs
- Need handling with lineage resonance check
- Context accumulation (max 10 items per agent)
- Providers struct bundling all external services
- HashMap-based capability registry

## Test Coverage

29 unit tests covering:
- Cosine similarity edge cases
- Resonance computation
- Signal propagation and attenuation
- Storage operations (ancestors, descendants, lineage queries)
- Provider initialization
- Capability interfaces

## Future Enhancements

- TOML configuration files
- Integration tests with mocked APIs
- Additional capabilities (code generation, data analysis)
- Persistent storage backends
- Web UI for monitoring agent activity
- Performance optimizations for large webs
