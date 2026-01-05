-- Agent definitions table
CREATE TABLE agent_definitions (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    tuning_keywords TEXT[] NOT NULL,
    tuning_embedding vector(1536),
    system_prompt TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.4,
    tools TEXT[] NOT NULL,
    source VARCHAR(20) NOT NULL,
    health_score REAL NOT NULL DEFAULT 1.0,
    use_count INTEGER NOT NULL DEFAULT 0,
    version VARCHAR(20),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_source CHECK (source IN ('built_in', 'user_custom', 'generated'))
);

-- Add definition_id to agents table
ALTER TABLE agents ADD COLUMN definition_id UUID REFERENCES agent_definitions(id);

-- Indexes
CREATE INDEX idx_definitions_source ON agent_definitions(source);
CREATE INDEX idx_definitions_name ON agent_definitions(name);
CREATE INDEX idx_definitions_tuning ON agent_definitions
    USING ivfflat (tuning_embedding vector_cosine_ops) WITH (lists = 50);
