use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AgentId, SignalDirection, SignalId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: SignalId,
    pub origin: AgentId,
    pub frequency: Vec<f32>,
    pub content: String,
    pub amplitude: f32,
    pub direction: SignalDirection,
    pub hop_count: u32,
    pub payload: Option<Value>,
}

impl Signal {
    pub fn new(
        origin: AgentId,
        frequency: Vec<f32>,
        content: String,
        direction: SignalDirection,
    ) -> Self {
        Self {
            id: SignalId::new_v4(),
            origin,
            frequency,
            content,
            amplitude: 1.0,
            direction,
            hop_count: 0,
            payload: None,
        }
    }

    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn attenuate(&mut self, factor: f32) {
        self.amplitude *= factor;
        self.hop_count += 1;
    }

    pub fn is_alive(&self, min_amplitude: f32) -> bool {
        self.amplitude >= min_amplitude
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalDraft {
    pub frequency: Vec<f32>,
    pub content: String,
    pub direction: SignalDirection,
    pub payload: Option<Value>,
}

impl SignalDraft {
    pub fn into_signal(self, origin: AgentId) -> Signal {
        Signal {
            id: SignalId::new_v4(),
            origin,
            frequency: self.frequency,
            content: self.content,
            amplitude: 1.0,
            direction: self.direction,
            hop_count: 0,
            payload: self.payload,
        }
    }
}
