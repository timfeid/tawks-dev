use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use specta::Type;
use std::{collections::HashMap, str::FromStr};

use crate::{
    AttachedWorkflowMeta, WorkflowRuntimeStatus, WorkflowVisual,
    error::{AppResult, WorkflowError},
    path::{get_path_any, get_path_from_value},
    schedule::ScheduledJob,
    server_action::ServerActionResult,
};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BroadcastFilter {
    pub user_id: Option<String>,
    pub response_equals: Option<(String, Value)>,
    pub include_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BroadcastInstanceInfo {
    pub instance_id: String,
    pub workflow_id: String,
    pub user_id: String,
    pub status: String,
    pub current_node_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BroadcastUpdatesResult {
    pub count: i32,
    pub instances: Vec<BroadcastInstanceInfo>,
}


#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum InjectOnComplete {
    Detach,
}

impl PartialEq for InjectOnComplete {
    fn eq(&self, _other: &Self) -> bool {
        matches!(
            (self, _other),
            (InjectOnComplete::Detach, InjectOnComplete::Detach)
        )
    }
}
impl Eq for InjectOnComplete {}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum InjectWorkflowPlace {
    This,
    Tile {
        name: String,
        on_complete: Option<InjectOnComplete>,
    },
    Modal {
        title: String,
    },
    Sidebar {
        position: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WorkflowDefinitionWithInput {
    pub definition: String,
    pub input: HashMap<String, serde_json::Value>,
    pub context: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WorkflowResource {
    pub inject: Option<InjectWorkflowPlace>,
    pub instance_id: String,
    pub workflow_id: String,
    pub stale_definition: bool,
    pub name: String,
    pub description: Option<String>,
    pub responses: HashMap<String, serde_json::Value>,
    pub completed: bool,
    pub status: WorkflowRuntimeStatus,
    pub complete_message: Option<String>,
    pub visuals: Vec<WorkflowVisual>,
    pub layout: Option<String>,
    pub user_id: String,
    pub current_node_id: String,
    pub waiting: bool,
    pub attached_workflows: HashMap<String, AttachedWorkflowMeta>,
    pub parent: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WorkflowResourceWithContext {
    pub workflow: WorkflowResource,
    pub context: HashMap<String, Value>,
}


#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WorkflowDefinitionAvailabilityResource {
    pub workflow_id: String,
    pub available_now: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WorkflowDefinitionSummary {
    pub id: String,
    pub workflow_id: String,
    pub owner_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
}

impl From<crate::WorkflowDefinition> for WorkflowDefinitionSummary {
    fn from(definition: crate::WorkflowDefinition) -> Self {
        let workflow_id = format!(
            "wf-{}-{}",
            definition.id,
            definition.owner_id.as_deref().unwrap_or_default()
        );

        Self {
            id: definition.id,
            workflow_id,
            owner_id: definition.owner_id,
            name: definition.name,
            description: definition.description,
        }
    }
}

impl WorkflowResourceWithContext {
    pub fn ctx_get<'a>(&'a self, key: &str) -> Option<&'a Value> {
        self.context.get(key)
    }

    pub fn ctx_get_u64(&self, key: &str, default: u64) -> u64 {
        self.ctx_get(key)
            .and_then(|v| v.as_u64())
            .unwrap_or(default)
    }

    pub fn ctx_get_bool(&self, key: &str, default: bool) -> bool {
        self.ctx_get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    pub fn ctx_get_i64(&self, key: &str, default: i64) -> i64 {
        self.ctx_get(key)
            .and_then(|v| v.as_i64())
            .unwrap_or(default)
    }

    pub fn ctx_get_array(&self, key: &str) -> Vec<Value> {
        self.ctx_get(key)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_root_input_as<T: DeserializeOwned>(&self) -> Result<T, WorkflowError> {
        let obj = serde_json::to_value(&self.workflow.responses).map_err(|e| {
            WorkflowError::ServerActionFailed(format!("could not convert inputs to value: {e}"))
        })?;
        serde_json::from_value(obj)
            .map_err(|e| WorkflowError::ServerActionFailed(format!("invalid root input: {e}")))
    }

    pub fn ctx_required_as_result<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<T, WorkflowError> {
        let value = self.context.get(key).ok_or_else(|| {
            WorkflowError::ServerActionFailed(format!("Missing context key: {key}"))
        })?;
        serde_json::from_value(value.clone())
            .map_err(|e| WorkflowError::ServerActionFailed(format!("Invalid context value: {e}")))
    }

    pub fn get_path_any<'a>(&'a self, arg: &str) -> Option<&'a Value> {
        get_path_any(&self.workflow.responses, &self.context, arg)
    }

    pub fn input_str(&self, key: &str) -> Option<String> {
        self.get_path_any(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn input_required_bool(&self, key: &str) -> AppResult<bool> {
        self.get_path_any(key)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| {
                WorkflowError::ServerActionFailed(format!("Missing input key: {}", key)).into()
            })
    }

    pub fn input_required_str(&self, key: &str) -> AppResult<String> {
        self.input_str(key).ok_or_else(|| {
            WorkflowError::ServerActionFailed(format!("Missing input key: {}", key)).into()
        })
    }

    pub fn ctx_required_str(&self, key: &str) -> AppResult<String> {
        let ctx_root = serde_json::Value::Object(self.context.clone().into_iter().collect());
        get_path_from_value(&ctx_root, key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                WorkflowError::ServerActionFailed(format!("Missing context key: {}", key)).into()
            })
    }

    pub fn ctx_required_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> AppResult<T> {
        let v = self.get_path_any(key).cloned().ok_or_else(|| {
            WorkflowError::ServerActionFailed(format!("Missing context key: {}", key))
        })?;

        Ok(serde_json::from_value(v).map_err(|e| {
            WorkflowError::ServerActionFailed(format!("failed to Deserialize {}: {:?}", key, e))
        })?)
    }

    pub fn get_input_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> AppResult<T> {
        let ctx_root =
            serde_json::Value::Object(self.workflow.responses.clone().into_iter().collect());
        let v = get_path_from_value(&ctx_root, key)
            .cloned()
            .ok_or_else(|| {
                WorkflowError::ServerActionFailed(format!("Missing input key: {}", key)).into()
            })?;

        Ok(serde_json::from_value(v).map_err(|e| {
            WorkflowError::ServerActionFailed(format!("failed to Deserialize {}: {:?}", key, e))
        })?)
    }

    pub fn input_get_array(&self, key: &str) -> Vec<Value> {
        // pub fn ctx_get_array(&self, key: &str) -> Vec<Value> {
        self.get_path_any(key)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        // }
    }
}

impl WorkflowResource {
    pub fn allowed_user_ids(&self) -> Vec<String> {
        vec![self.user_id.clone()]
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    pub fn is_waiting(&self) -> bool {
        matches!(self.status, WorkflowRuntimeStatus::Waiting)
    }

    pub fn is_accessible_by(&self, user_id: &str) -> bool {
        self.user_id == user_id
    }

    pub fn age(&self) -> chrono::Duration {
        let Ok(utc) = NaiveDateTime::from_str(&self.created_at) else {
            return chrono::Duration::zero();
        };
        let datetime = DateTime::<Utc>::from_naive_utc_and_offset(utc, Utc);
        chrono::Utc::now() - datetime
    }
}


#[derive(Type, Deserialize, Serialize, Debug)]
pub struct ScheduleJobsArgs {
    pub instance_id: String,
    pub jobs: Vec<ScheduledJob>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WorkflowRespondServerActionArgs {
    pub token: String,
    pub result: ServerActionResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RegisterExternalServerActionArgs {
    pub action_id: String,
    pub registrar_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RenewRegistrarLeaseArgs {
    pub registrar_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReconcileWorkflowArgs {
    pub instance_id: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub context: HashMap<String, serde_json::Value>,
    pub reconcile_node: bool,
    /// When set, the instance is completed with this message after the update.
    #[serde(default)]
    pub complete_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BroadcastWorkflowUpdatesArgs {
    pub workflow_definition_id: String,
    pub filter: BroadcastFilter,
    pub updates: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProcessWorkflowActionArgs {
    pub instance_id: String,
    pub action_id: String,
    pub inputs: HashMap<String, serde_json::Value>,
}

impl ProcessWorkflowActionArgs {
    pub fn new(
        instance_id: String,
        action_id: String,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            instance_id,
            action_id,
            inputs,
        }
    }
}


#[derive(Type, Clone, Deserialize, Serialize, Debug)]
pub struct StartWorkflowFromInstanceArgs {
    pub instance_id: String,
    pub attach_as: Option<String>,
    pub workflow_definition_id: String,
    pub context: HashMap<String, Value>,
    pub inputs: HashMap<String, Value>,
    pub inject: Option<InjectWorkflowPlace>,
}

#[derive(Type, Clone, Deserialize, Serialize, Debug)]
pub struct StartWorkflowOnBehalfArgs {
    pub workflow_definition_id: String,
    pub input: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub user_id: String,
    pub inject: Option<InjectWorkflowPlace>,
}
