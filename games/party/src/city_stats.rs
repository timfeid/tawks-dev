//! A record of each job, for the balance harness and match summaries.
use serde::Serialize;

/// The furthest a job got before it ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Power,
    Vault,
    Carry,
    Delivered,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct JobRecord {
    pub started: f32,
    pub ended: Option<f32>,
    pub phase: Phase,
    pub bags_picked: u32,
    pub bags_delivered: u32,
    pub bags_recovered: u32,
    pub arrests: u32,
    /// The whole crew ended up in the cell.
    pub wiped: bool,
}

impl JobRecord {
    pub fn starting(at: f32) -> Self {
        JobRecord {
            started: at,
            ..Default::default()
        }
    }
    pub fn reach(&mut self, phase: Phase) {
        self.phase = self.phase.max(phase);
    }
}
