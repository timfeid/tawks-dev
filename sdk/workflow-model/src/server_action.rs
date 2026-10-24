use std::{collections::HashMap, sync::Arc};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use specta::Type;

use crate::{
    WorkflowPredicate,
    error::{AppResult, WorkflowError},
    transform::TransformOp,
    schedule::ScheduledJob,
    service::InjectWorkflowPlace,
};

#[derive(Debug, Clone)]
pub struct ServerActionContext {
    pub workflow_id: String,
    pub action_id: String,
    pub instance_id: String,
    pub user_id: String,
    pub owner_id: String,
    pub inputs: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
}

impl ServerActionContext {
    pub fn get_root_input_as<T: DeserializeOwned>(&self) -> Result<T, ServerActionErr> {
        let obj = serde_json::to_value(&self.inputs).map_err(|e| {
            ServerActionErr::from(format!("could not convert inputs to value: {e}"))
        })?;
        serde_json::from_value(obj)
            .map_err(|e| ServerActionErr::from(format!("invalid root input: {e}")))
    }

    pub fn get_input_as<T: DeserializeOwned>(&self, key: &str) -> Result<T, ServerActionErr> {
        let value = self
            .get_input(key)
            .ok_or(ServerActionErr::from(format!("no input")))?;
        serde_json::from_value(value.clone())
            .map_err(|e| ServerActionErr::from(format!("invalid input '{}': {e}", key)))
    }

    pub fn get_required_nested_value_as_str<'a>(
        map: &'a HashMap<String, serde_json::Value>,
        dotted_path: &str,
    ) -> AppResult<&'a str> {
        let value = Self::get_nested_value(map, dotted_path);

        return value
            .and_then(|x| x.as_str())
            .ok_or(WorkflowError::ServerActionFailed(format!(
                "{dotted_path} not found in {:?}",
                map
            )));
    }

    pub fn get_nested_value<'a>(
        map: &'a HashMap<String, serde_json::Value>,
        dotted_path: &str,
    ) -> Option<&'a serde_json::Value> {
        let mut current = map.get(dotted_path.split('.').next()?)?;

        for key in dotted_path.split('.').skip(1) {
            match current {
                serde_json::Value::Object(obj) => {
                    current = obj.get(key)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }
    pub fn get_input(&self, path: &str) -> Option<&Value> {
        Self::get_nested_value(&self.inputs, path)
    }

    pub fn get_required_input_as_bool(&self, key: &str) -> AppResult<bool> {
        self.get_input(key)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| {
                WorkflowError::ServerActionFailed(format!("Missing or invalid input: {key}"))
            })
    }

    pub fn get_required_input_as_str(&self, key: &str) -> AppResult<&str> {
        self.get_input(key).and_then(|v| v.as_str()).ok_or_else(|| {
            WorkflowError::ServerActionFailed(format!("Missing or invalid input: {key}"))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum ServerActionResult {
    ReplaceWithNew {
        workflow_definition_id: String,
        context: HashMap<String, Value>,
        inputs: HashMap<String, Value>,
    },
    None,
    Reset {
        context: HashMap<String, Value>,
        inputs: HashMap<String, Value>,
    },
    Error(String),
    ExternalServerActionStarted {
        action_id: String,
        token: String,
    },
    NextPage {
        page_id: String,
    },
    UpdateResponses(HashMap<String, Value>),
    UpdateContext(HashMap<String, Value>),
    ScheduleJobs {
        context: HashMap<String, Value>,
        inputs: HashMap<String, Value>,
        jobs: Vec<ScheduledJob>,
        reconcile_node: bool,
    },
    UpdateContextAndResponses {
        context: HashMap<String, Value>,
        responses: HashMap<String, Value>,
    },
    CompleteWorkflow {
        responses: HashMap<String, Value>,
        close_active_modal: bool,
        message: String,
    },
    StartNewWorkflow {
        inject: Option<InjectWorkflowPlace>,
        workflow_id: String,
        responses: HashMap<String, Value>,
        attach_id: String,
        inputs: HashMap<String, Value>,
        context: HashMap<String, Value>,
    },
    CancelWorkflow,
    StartAndWaitWorkflow {
        attach_id: String,
        responses: HashMap<String, Value>,
        context: HashMap<String, Value>,
        inputs: HashMap<String, Value>,
        definition_id: String,
        inject: Option<InjectWorkflowPlace>,
        inject_response_as: Option<String>,
        on_complete: Vec<TransformOp>,
        reconcile_node: bool,
    },
    WaitForPredicate {
        responses: HashMap<String, Value>,
        predicate: WorkflowPredicate,
        inject_response_as: Option<String>,
        on_complete: Vec<TransformOp>,
        reconcile_node: bool,
    },
    ReplaceWithInstance(String),
}

pub type ServerActionErr = Box<dyn std::error::Error + Send + Sync>;
pub type ServerActionResponse = BoxFuture<'static, Result<ServerActionResult, ServerActionErr>>;

pub type ServerActionHandler =
    Arc<Box<dyn Fn(ServerActionContext) -> ServerActionResponse + Send + Sync>>;

pub fn bail_server_action_error<E: ToString>(e: E) -> Result<ServerActionResult, ServerActionErr> {
    Ok(ServerActionResult::CompleteWorkflow {
        close_active_modal: false,
        responses: {
            let mut responses = HashMap::new();
            responses.insert("error".to_string(), json!(e.to_string()));
            responses
        },
        message: "".into(),
    })
}

pub fn bail_server_workflow_str(
    msg: impl Into<String>,
) -> Result<ServerActionResult, ServerActionErr> {
    Ok(ServerActionResult::CompleteWorkflow {
        close_active_modal: false,
        responses: {
            let mut responses = HashMap::new();
            responses.insert("error".to_string(), json!(msg.into()));
            responses
        },
        message: "".into(),
    })
}
