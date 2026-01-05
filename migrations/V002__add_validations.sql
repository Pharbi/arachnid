-- Validations table
CREATE TABLE validations (
    id UUID PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agents(id),
    web_id UUID NOT NULL REFERENCES webs(id),
    output_hash TEXT NOT NULL,
    judgment VARCHAR(20) NOT NULL,
    confidence REAL,
    reason TEXT,
    raw_response TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_validations_agent_id ON validations(agent_id);
CREATE INDEX idx_validations_web_id ON validations(web_id);
