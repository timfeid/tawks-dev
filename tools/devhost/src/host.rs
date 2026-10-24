use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use workflow_model::server_action::ServerActionResult;
use workflow_model::service::{WorkflowResource, WorkflowResourceWithContext};
use workflow_model::{ActionType, CreateWorkflowDefinition, WorkflowRuntimeStatus};

const ACTION_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Serialize)]
pub struct Player {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub db_score: i32,
    pub is_bot: bool,
}

impl Player {
    fn numbered(n: usize) -> Self {
        Player {
            id: format!("player-{n}"),
            username: format!("player{n}"),
            display_name: format!("Player {n}"),
            avatar_url: "default/1.png".into(),
            db_score: 0,
            is_bot: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Instance {
    pub id: String,
    pub workflow_id: String,
    pub user_id: String,
    pub responses: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub completed: bool,
    pub complete_message: Option<String>,
}

/// What the page hears about: instance changes and a running log.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PageEvent {
    Instance { instance: Instance },
    Log { level: &'static str, text: String },
    Activities { activities: Vec<Value> },
}

/// Where the game's state lives in its responses, and the action each move runs.
#[derive(Clone, Debug, Serialize)]
pub struct ActivityDisplay {
    pub state: String,
    pub action: String,
}

/// Who waits on a server action: the page, which wants the result applied
/// to the instance, or the playground, which applies it itself.
enum Reply {
    Apply(oneshot::Sender<Result<(), String>>),
    Raw(oneshot::Sender<ServerActionResult>),
}

struct Pending {
    instance_id: String,
    reply: Reply,
}

#[derive(Default)]
struct State {
    definitions: HashMap<String, CreateWorkflowDefinition>,
    /// Server action id to the registrar and the bot that registered it.
    actions: HashMap<String, (Option<String>, String)>,
    activities: Vec<Value>,
    instances: HashMap<String, Instance>,
    pending: HashMap<String, Pending>,
    /// Each bot's events socket, by its token.
    bots: HashMap<String, mpsc::UnboundedSender<String>>,
    /// Every Start opens a new lobby, so bots start a fresh game.
    lobby: u32,
}

pub struct Host {
    state: Mutex<State>,
    pub page: broadcast::Sender<PageEvent>,
}

impl Host {
    pub fn new() -> Self {
        Host { state: Mutex::new(State::default()), page: broadcast::channel(512).0 }
    }

    pub fn log(&self, level: &'static str, text: impl Into<String>) {
        let text = text.into();
        match level {
            "error" => tracing::error!("{text}"),
            "warn" => tracing::warn!("{text}"),
            _ => tracing::info!("{text}"),
        }
        let _ = self.page.send(PageEvent::Log { level, text });
    }

    pub async fn connect_bot(&self, token: &str, sender: mpsc::UnboundedSender<String>) {
        self.state.lock().await.bots.insert(token.to_string(), sender);
        self.log("info", format!("bot {token} connected to the events socket"));
    }

    pub async fn snapshot(&self) -> (Vec<Value>, Vec<Instance>, String) {
        let state = self.state.lock().await;
        (state.activities.clone(), state.instances.values().cloned().collect(), lobby_id(state.lobby))
    }

    /// Ids are scoped to the bot the way tawks scopes them, with the bot's
    /// token standing in for its user id.
    pub async fn register_workflow(&self, bot: &str, definition: CreateWorkflowDefinition) -> String {
        let id = format!("wf-{}-{bot}", definition.id);
        self.log("info", format!("registered workflow {id}"));
        self.state.lock().await.definitions.insert(id.clone(), definition);
        id
    }

    pub async fn register_action(&self, bot: &str, action_id: &str, registrar: Option<String>) -> String {
        let id = format!("sa-{action_id}-{bot}");
        self.state.lock().await.actions.insert(id.clone(), (registrar, bot.to_string()));
        id
    }

    pub async fn renew(&self, registrar: &str) -> u32 {
        let state = self.state.lock().await;
        state.actions.values().filter(|(r, _)| r.as_deref() == Some(registrar)).count() as u32
    }

    pub async fn has_workflow(&self, id: &str) -> bool {
        self.state.lock().await.definitions.contains_key(id)
    }

    pub async fn register_activity(&self, activity: Value) {
        let activities = {
            let mut state = self.state.lock().await;
            let id = activity.get("workflow_id").cloned();
            state.activities.retain(|a| a.get("workflow_id") != id.as_ref());
            state.activities.push(activity);
            state.activities.clone()
        };
        self.log("info", "registered an activity");
        let _ = self.page.send(PageEvent::Activities { activities });
    }

    pub async fn activity_display(&self, workflow_id: &str) -> Option<ActivityDisplay> {
        let state = self.state.lock().await;
        let definition = serde_json::to_value(state.definitions.get(workflow_id)?).ok()?;
        find_activity(&definition)
    }

    /// Starts one player's instance of a workflow and runs its starting action.
    pub async fn start(
        &self,
        workflow_id: &str,
        user: &Player,
        input: HashMap<String, Value>,
        mut context: HashMap<String, Value>,
    ) -> Result<Instance, String> {
        let (starting, lobby) = {
            let state = self.state.lock().await;
            let definition = state
                .definitions
                .get(workflow_id)
                .ok_or_else(|| format!("no workflow {workflow_id} is registered"))?;
            (definition.starting_server_action.clone(), lobby_id(state.lobby))
        };
        context.insert("$user".into(), json!(user));
        context.insert("$anonymous".into(), json!(false));
        let mut responses = input;
        responses.entry("lobby_id".into()).or_insert(json!(lobby));
        let instance = Instance {
            id: ulid::Ulid::new().to_string(),
            workflow_id: workflow_id.to_string(),
            user_id: user.id.clone(),
            responses,
            context,
            completed: false,
            complete_message: None,
        };
        self.state.lock().await.instances.insert(instance.id.clone(), instance.clone());
        self.publish(&instance.id).await;
        if let Some(action) = starting {
            self.dispatch(&instance.id, &action).await?;
        }
        let started = self.state.lock().await.instances.get(&instance.id).cloned();
        Ok(started.unwrap_or(instance))
    }

    pub async fn start_players(&self, workflow_id: &str, players: usize) -> Result<Vec<Instance>, String> {
        let mut started = Vec::new();
        for n in 1..=players.clamp(1, 8) {
            let player = Player::numbered(n);
            started.push(self.start(workflow_id, &player, HashMap::new(), HashMap::new()).await?);
        }
        Ok(started)
    }

    /// A move from the page: the inputs land in the responses, then the
    /// workflow action runs.
    pub async fn act(&self, instance_id: &str, action_id: &str, inputs: HashMap<String, Value>) -> Result<(), String> {
        let target = {
            let mut state = self.state.lock().await;
            let instance = state.instances.get_mut(instance_id).ok_or("no such instance")?;
            instance.responses.extend(inputs);
            let workflow_id = instance.workflow_id.clone();
            let definition = state.definitions.get(&workflow_id).ok_or("the workflow is gone")?;
            let action = definition
                .actions
                .iter()
                .find(|a| a.id == action_id)
                .ok_or_else(|| format!("the workflow has no action {action_id}"))?;
            match (&action.action_type, &action.target) {
                (ActionType::RunServerAction, Some(target)) => target.clone(),
                (other, _) => return Err(format!("devhost only runs server actions, not {other:?}")),
            }
        };
        self.publish(instance_id).await;
        self.dispatch(instance_id, &target).await
    }

    /// Hands a server action to the bot and waits for its answer.
    pub async fn dispatch(&self, instance_id: &str, action_id: &str) -> Result<(), String> {
        let (done, wait) = oneshot::channel();
        match self.request(instance_id, action_id, Reply::Apply(done), wait).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err("the action was dropped".into()),
            Err(error) => Err(error),
        }
    }

    /// Lists every registered workflow with its definition.
    pub async fn workflows(&self) -> Vec<Value> {
        let state = self.state.lock().await;
        let mut list: Vec<Value> = state
            .definitions
            .iter()
            .map(|(id, definition)| json!({ "id": id, "definition": definition }))
            .collect();
        list.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        list
    }

    /// Runs one server action for a page the playground drives, on a
    /// throwaway instance holding the playground's responses and context, and
    /// returns the bot's answer without applying it.
    pub async fn run_for_playground(
        &self,
        workflow_id: &str,
        action_id: &str,
        responses: HashMap<String, Value>,
        mut context: HashMap<String, Value>,
    ) -> Result<ServerActionResult, String> {
        if !self.has_workflow(workflow_id).await {
            return Err(format!("no workflow {workflow_id} is registered"));
        }
        let user = context
            .get("$user")
            .and_then(|u| serde_json::from_value::<PlayerIn>(u.clone()).ok())
            .map(PlayerIn::into_player)
            .unwrap_or_else(|| Player::numbered(1));
        context.insert("$user".into(), json!(user));
        context.entry("$anonymous".into()).or_insert(json!(false));
        let instance = Instance {
            id: format!("playground-{}", ulid::Ulid::new()),
            workflow_id: workflow_id.to_string(),
            user_id: user.id.clone(),
            responses,
            context,
            completed: false,
            complete_message: None,
        };
        let id = instance.id.clone();
        self.state.lock().await.instances.insert(id.clone(), instance);
        let (raw, wait) = oneshot::channel();
        let outcome = self.request(&id, action_id, Reply::Raw(raw), wait).await;
        self.state.lock().await.instances.remove(&id);
        match outcome {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err("the action was dropped".into()),
            Err(error) => Err(error),
        }
    }

    async fn request<T>(
        &self,
        instance_id: &str,
        action_id: &str,
        reply: Reply,
        wait: oneshot::Receiver<T>,
    ) -> Result<Result<T, oneshot::error::RecvError>, String> {
        let token = ulid::Ulid::new().to_string();
        let message = {
            let mut state = self.state.lock().await;
            let (_, owner) = state
                .actions
                .get(action_id)
                .ok_or_else(|| format!("no bot registered the server action {action_id}"))?;
            let bot = state
                .bots
                .get(owner)
                .cloned()
                .ok_or_else(|| format!("bot {owner} registered {action_id} but is not connected"))?;
            let instance = state.instances.get(instance_id).ok_or("no such instance")?.clone();
            let name = state.definitions.get(&instance.workflow_id).map(|d| d.name.clone()).unwrap_or_default();
            state.pending.insert(token.clone(), Pending { instance_id: instance_id.into(), reply });
            let message = json!({
                "type": "workflow_request_server_action",
                "id": token,
                "action_id": action_id,
                "workflow": resource(&instance, name),
            })
            .to_string();
            (bot, message)
        };
        message.0.send(message.1).map_err(|_| "the bot disconnected".to_string())?;
        match tokio::time::timeout(ACTION_TIMEOUT, wait).await {
            Ok(received) => Ok(received),
            Err(_) => {
                self.state.lock().await.pending.remove(&token);
                Err(format!("the bot did not answer {action_id} within {}s", ACTION_TIMEOUT.as_secs()))
            }
        }
    }

    pub async fn respond(&self, token: &str, result: ServerActionResult) -> Result<(), String> {
        let pending = self
            .state
            .lock()
            .await
            .pending
            .remove(token)
            .ok_or("No external action context for that token")?;
        match pending.reply {
            Reply::Apply(done) => {
                let outcome = self.apply(&pending.instance_id, result).await;
                let _ = done.send(outcome);
            }
            Reply::Raw(raw) => {
                let _ = raw.send(result);
            }
        }
        Ok(())
    }

    async fn apply(&self, instance_id: &str, result: ServerActionResult) -> Result<(), String> {
        {
            let mut state = self.state.lock().await;
            let instance = state.instances.get_mut(instance_id).ok_or("Workflow instance not found")?;
            match result {
                ServerActionResult::None => {}
                ServerActionResult::Error(error) => return Err(error),
                ServerActionResult::UpdateResponses(responses) => instance.responses.extend(responses),
                ServerActionResult::UpdateContext(context) => instance.context.extend(context),
                ServerActionResult::UpdateContextAndResponses { responses, context } => {
                    instance.responses.extend(responses);
                    instance.context.extend(context);
                }
                ServerActionResult::CompleteWorkflow { responses, message, .. } => {
                    instance.responses.extend(responses);
                    instance.completed = true;
                    instance.complete_message = Some(message);
                }
                other => {
                    let kind = serde_json::to_value(&other)
                        .ok()
                        .and_then(|v| v.as_object().and_then(|o| o.keys().next().cloned()).or(v.as_str().map(String::from)))
                        .unwrap_or_default();
                    return Err(format!("devhost does not fake the {kind} result yet"));
                }
            }
        }
        self.publish(instance_id).await;
        Ok(())
    }

    /// The game framework pushes each player's view this way.
    pub async fn reconcile(
        &self,
        instance_id: &str,
        inputs: HashMap<String, Value>,
        context: HashMap<String, Value>,
        complete_message: Option<String>,
    ) -> Result<(), String> {
        {
            let mut state = self.state.lock().await;
            let instance = state.instances.get_mut(instance_id).ok_or("Workflow instance not found")?;
            instance.responses.extend(inputs);
            instance.context.extend(context);
            if let Some(message) = complete_message {
                instance.completed = true;
                instance.complete_message = Some(message);
            }
        }
        self.publish(instance_id).await;
        Ok(())
    }

    /// Merges `updates` into every matching instance of a workflow.
    pub async fn broadcast(
        &self,
        workflow_id: &str,
        user_id: Option<&str>,
        response_equals: Option<(String, Value)>,
        include_completed: bool,
        updates: HashMap<String, Value>,
    ) -> usize {
        let ids: Vec<String> = {
            let mut state = self.state.lock().await;
            state
                .instances
                .values_mut()
                .filter(|i| i.workflow_id == workflow_id)
                .filter(|i| include_completed || !i.completed)
                .filter(|i| user_id.is_none_or(|u| i.user_id == u))
                .filter(|i| response_equals.as_ref().is_none_or(|(k, v)| i.responses.get(k) == Some(v)))
                .map(|i| {
                    i.responses.extend(updates.clone());
                    i.id.clone()
                })
                .collect()
        };
        for id in &ids {
            self.publish(id).await;
        }
        ids.len()
    }

    pub async fn restart(&self, instance_id: &str) -> Result<Instance, String> {
        let user = self.user_of(instance_id).await;
        let old = self.state.lock().await.instances.remove(instance_id).ok_or("Workflow instance not found")?;
        let user = user.unwrap_or(Player { id: old.user_id.clone(), ..Player::numbered(0) });
        self.start(&old.workflow_id, &user, HashMap::new(), HashMap::new()).await
    }

    pub async fn user_of(&self, instance_id: &str) -> Option<Player> {
        let state = self.state.lock().await;
        let instance = state.instances.get(instance_id)?;
        instance.context.get("$user").and_then(|u| serde_json::from_value::<PlayerIn>(u.clone()).ok()).map(PlayerIn::into_player)
    }

    async fn publish(&self, instance_id: &str) {
        let instance = self.state.lock().await.instances.get(instance_id).cloned();
        if let Some(instance) = instance {
            let _ = self.page.send(PageEvent::Instance { instance });
        }
    }

    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.instances.clear();
        state.pending.clear();
        state.lobby += 1;
    }
}

#[derive(serde::Deserialize)]
struct PlayerIn {
    id: String,
    username: String,
    display_name: String,
}

impl PlayerIn {
    fn into_player(self) -> Player {
        Player { id: self.id, username: self.username, display_name: self.display_name, ..Player::numbered(0) }
    }
}

fn lobby_id(n: u32) -> String {
    format!("dev-lobby-{n}")
}

pub fn player(n: usize) -> Player {
    Player::numbered(n)
}

fn resource(instance: &Instance, name: String) -> WorkflowResourceWithContext {
    let status = if instance.completed { WorkflowRuntimeStatus::Completed } else { WorkflowRuntimeStatus::Active };
    WorkflowResourceWithContext {
        workflow: WorkflowResource {
            inject: None,
            instance_id: instance.id.clone(),
            workflow_id: instance.workflow_id.clone(),
            stale_definition: false,
            name,
            description: None,
            responses: instance.responses.clone(),
            completed: instance.completed,
            status,
            complete_message: instance.complete_message.clone(),
            visuals: Vec::new(),
            layout: None,
            user_id: instance.user_id.clone(),
            current_node_id: String::new(),
            waiting: false,
            attached_workflows: HashMap::new(),
            parent: None,
            created_at: String::new(),
            updated_at: String::new(),
            completed_at: None,
        },
        context: instance.context.clone(),
    }
}

fn find_activity(value: &Value) -> Option<ActivityDisplay> {
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(spec)) = map.get("Activity") {
                if let (Some(state), Some(action)) = (spec.get("state"), spec.get("action")) {
                    return Some(ActivityDisplay {
                        state: state.as_str()?.to_string(),
                        action: action.as_str()?.to_string(),
                    });
                }
            }
            map.values().find_map(find_activity)
        }
        Value::Array(items) => items.iter().find_map(find_activity),
        _ => None,
    }
}
