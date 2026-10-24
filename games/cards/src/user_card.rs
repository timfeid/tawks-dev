use crate::types::{AuditEventKind, VisualSpec};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserCardInstance {
    pub instance_id: String,
    pub card_id: String,
    pub owner_id: String,
    pub visual_spec: VisualSpec,
    pub is_tradeable: bool,
    pub is_bound: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardAuditEntry {
    pub log_id: String,
    pub instance_id: String,
    pub event_kind: AuditEventKind,
    pub actor_id: String,
    pub previous_owner_id: Option<String>,
    pub new_owner_id: Option<String>,
    pub previous_visual_spec: Option<VisualSpec>,
    pub new_visual_spec: Option<VisualSpec>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
