-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Webs table
CREATE TABLE webs (
    id UUID PRIMARY KEY,
    task TEXT NOT NULL,
    state VARCHAR(20) NOT NULL,
    root_agent_id UUID,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Agents table
CREATE TABLE agents (
    id UUID PRIMARY KEY,
    web_id UUID NOT NULL REFERENCES webs(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES agents(id),
    purpose TEXT NOT NULL,
    tuning vector(1536),
    capability VARCHAR(50) NOT NULL,
    state VARCHAR(20) NOT NULL,
    health REAL NOT NULL DEFAULT 1.0,
    activation_threshold REAL NOT NULL,
    context JSONB NOT NULL DEFAULT '{}',
    probation_remaining INTEGER NOT NULL DEFAULT 5,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    dormant_since TIMESTAMPTZ,
    CONSTRAINT fk_web FOREIGN KEY (web_id) REFERENCES webs(id)
);

-- Signals table
CREATE TABLE signals (
    id UUID PRIMARY KEY,
    web_id UUID NOT NULL REFERENCES webs(id) ON DELETE CASCADE,
    origin_agent_id UUID NOT NULL REFERENCES agents(id),
    frequency vector(1536),
    content TEXT NOT NULL,
    amplitude REAL NOT NULL,
    direction VARCHAR(10) NOT NULL,
    hop_count INTEGER NOT NULL DEFAULT 0,
    payload JSONB,
    suspect BOOLEAN NOT NULL DEFAULT FALSE,
    processed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Web memory (failure patterns)
CREATE TABLE web_memory (
    id UUID PRIMARY KEY,
    web_id UUID NOT NULL REFERENCES webs(id) ON DELETE CASCADE,
    pattern_type VARCHAR(50) NOT NULL,
    pattern_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_agents_web_id ON agents(web_id);
CREATE INDEX idx_agents_parent_id ON agents(parent_id);
CREATE INDEX idx_agents_state ON agents(state);
CREATE INDEX idx_signals_web_id ON signals(web_id);
CREATE INDEX idx_signals_processed ON signals(processed) WHERE NOT processed;
CREATE INDEX idx_web_memory_web_id ON web_memory(web_id);

-- Vector similarity index (for resonance lookups)
CREATE INDEX idx_agents_tuning ON agents USING ivfflat (tuning vector_cosine_ops) WITH (lists = 100);
