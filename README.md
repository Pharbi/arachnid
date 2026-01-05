# Arachnid

An autonomous agent coordination runtime where agents self-organize through semantic resonance. Agents activate based on signal similarity rather than explicit routing, like vibrations propagating through a spider web.

## Status: Phase 1 Complete

Phase 1 implements the foundation of the system with working signal propagation, resonance computation, and coordination loop.

## Features

- **Signal Propagation**: Signals propagate along agent DAG edges with configurable attenuation
- **Resonance-Based Activation**: Agents activate when signal resonance exceeds their threshold
- **Coordination Loop**: Manages agent lifecycle and signal processing
- **In-Memory Storage**: HashMap-based storage for webs, agents, and signals
- **Embedding Integration**: OpenAI text-embedding-3-small support

## Building

```bash
cargo build --release
```

## Running

```bash
# Basic usage
cargo run -- run "your task here"

# With OpenAI embeddings (requires OPENAI_API_KEY)
export OPENAI_API_KEY=sk-...
cargo run -- run "your task here"

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
│   ├── engine/         # Coordination loop, propagation, resonance
│   ├── storage/        # WebStore trait and in-memory implementation
│   ├── providers/      # External service adapters (OpenAI)
│   └── capabilities/   # Agent capabilities (Phase 2)
```

## Configuration

Environment variables:
- `OPENAI_API_KEY`: OpenAI API key for embeddings (optional, uses dummy embeddings if not set)

## Phase 1 Implementation Details

- Core types: Agent, Signal, Web with supporting enums
- Cosine similarity for vector comparison
- Signal attenuation based on hop count (default: 0.8 per hop)
- Minimum amplitude threshold (default: 0.1)
- Agent activation threshold (default: 0.6)
- Maximum depth limit (default: 10)
- Convergence detection when no active agents remain

## Next Steps (Phase 2)

- LLM provider adapters (Anthropic, OpenAI)
- Search capability (Brave API)
- Synthesizer capability
- Agent spawning for unmet needs
- Context accumulation
- Configuration system (TOML + env vars)
