use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use workflow_model::condition::eval_condition;
use workflow_model::schedule::job::{ScheduledAction, ScheduledJob};
use workflow_model::server_action::ServerActionResult;
use workflow_model::service::{WorkflowResource, WorkflowResourceWithContext};
use workflow_model::transform::{apply_input_map_into_responses, apply_transform_ops};
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
            avatar_url: format!("default/default-{}.png", (n - 1) % 5 + 1),
            db_score: 0,
            is_bot: false,
        }
    }
}

/// Server actions tawks itself answers, which devhost only notes.
const BUILT_IN_ACTIONS: [&str; 1] = ["db_score.added"];

/// The niche every devhost lobby belongs to.
pub const DEV_NICHE: &str = "dev-niche";

#[derive(Clone, Debug, Serialize)]
pub struct Instance {
    pub id: String,
    /// The visitor whose lobby this is. Locally there is one, `local`.
    pub session: String,
    pub workflow_id: String,
    pub user_id: String,
    pub responses: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub completed: bool,
    pub complete_message: Option<String>,
    /// For a workflow a game started inside a player's page: that page's
    /// instance, and where in it the new one shows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject: Option<Value>,
}

/// What the page hears about: instance changes and a running log.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PageEvent {
    Instance { instance: Instance },
    Log { level: &'static str, text: String, session: Option<String> },
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
    /// The registrar that took this request, when copies of a bot share a token.
    claimed_by: Option<String>,
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
    /// Each session's current lobby, and when the session was last used.
    lobbies: HashMap<String, String>,
    touched: HashMap<String, Instant>,
    /// Jobs games scheduled with a ScheduleJobs result, by when they are due.
    jobs: Vec<(Instant, String, ScheduledJob)>,
}

impl State {
    fn lobby_for(&mut self, session: &str) -> String {
        if let Some(lobby) = self.lobbies.get(session) {
            return lobby.clone();
        }
        self.lobby += 1;
        let lobby = lobby_id(self.lobby);
        self.lobbies.insert(session.to_string(), lobby.clone());
        lobby
    }
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
        self.log_to(None, level, text);
    }

    /// A log line for one session's page, or for every page when `session` is None.
    pub fn log_to(&self, session: Option<&str>, level: &'static str, text: impl Into<String>) {
        let text = text.into();
        match level {
            "error" => tracing::error!("{text}"),
            "warn" => tracing::warn!("{text}"),
            _ => tracing::info!("{text}"),
        }
        let _ = self.page.send(PageEvent::Log { level, text, session: session.map(String::from) });
    }

    pub async fn touch(&self, session: &str) {
        self.state.lock().await.touched.insert(session.to_string(), Instant::now());
    }

    pub async fn session_count(&self) -> usize {
        self.state.lock().await.touched.len()
    }

    pub async fn session_of(&self, instance_id: &str) -> Option<String> {
        self.state.lock().await.instances.get(instance_id).map(|i| i.session.clone())
    }

    /// Forgets sessions nobody has used for `idle`, with their lobbies and instances.
    pub async fn expire(&self, idle: Duration) -> usize {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let stale: Vec<String> =
            state.touched.iter().filter(|(_, at)| now.duration_since(**at) > idle).map(|(s, _)| s.clone()).collect();
        for session in &stale {
            state.touched.remove(session);
            state.lobbies.remove(session);
            state.instances.retain(|_, i| &i.session != session);
        }
        stale.len()
    }

    /// Counts what one bot has registered, for limits on a public sandbox.
    pub async fn registrations_of(&self, bot: &str) -> (usize, usize) {
        let state = self.state.lock().await;
        let suffix = format!("-{bot}");
        let workflows = state.definitions.keys().filter(|id| id.ends_with(&suffix)).count();
        let actions = state.actions.values().filter(|(_, owner)| owner == bot).count();
        (workflows, actions)
    }

    pub async fn connect_bot(&self, token: &str, sender: mpsc::UnboundedSender<String>) {
        self.state.lock().await.bots.insert(token.to_string(), sender);
        self.log("info", format!("bot {token} connected to the events socket"));
    }

    pub async fn snapshot(&self, session: &str) -> (Vec<Value>, Vec<Instance>, String) {
        let mut state = self.state.lock().await;
        let lobby = state.lobby_for(session);
        let instances = state.instances.values().filter(|i| i.session == session).cloned().collect();
        (state.activities.clone(), instances, lobby)
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

    pub async fn has_action(&self, id: &str) -> bool {
        self.state.lock().await.actions.contains_key(id)
    }

    pub async fn renew(&self, registrar: &str) -> u32 {
        let state = self.state.lock().await;
        state.actions.values().filter(|(r, _)| r.as_deref() == Some(registrar)).count() as u32
    }

    /// A bot may name its own workflow by its bare key, as tawks allows.
    pub async fn resolve_workflow(&self, id: &str, bot: &str) -> String {
        let scoped = format!("wf-{id}-{bot}");
        let state = self.state.lock().await;
        if !state.definitions.contains_key(id) && state.definitions.contains_key(&scoped) { scoped } else { id.to_string() }
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
        session: &str,
        workflow_id: &str,
        user: &Player,
        input: HashMap<String, Value>,
        context: HashMap<String, Value>,
    ) -> Result<Instance, String> {
        self.start_within(session, workflow_id, user, input, context, None, None).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_within(
        &self,
        session: &str,
        workflow_id: &str,
        user: &Player,
        input: HashMap<String, Value>,
        mut context: HashMap<String, Value>,
        parent_instance_id: Option<String>,
        inject: Option<Value>,
    ) -> Result<Instance, String> {
        let (starting, lobby) = {
            let mut state = self.state.lock().await;
            let starting = state
                .definitions
                .get(workflow_id)
                .ok_or_else(|| format!("no workflow {workflow_id} is registered"))?
                .starting_server_action
                .clone();
            (starting, state.lobby_for(session))
        };
        context.insert("$user".into(), json!(user));
        context.insert("$anonymous".into(), json!(false));
        let mut responses = input;
        responses.entry("lobby_id".into()).or_insert(json!(lobby));
        // tawks starts lobby games with the lobby's niche too.
        responses.entry("niche_id".into()).or_insert(json!(DEV_NICHE));
        let instance = Instance {
            id: ulid::Ulid::new().to_string(),
            session: session.to_string(),
            workflow_id: workflow_id.to_string(),
            user_id: user.id.clone(),
            responses,
            context,
            completed: false,
            complete_message: None,
            parent_instance_id,
            inject,
        };
        self.state.lock().await.instances.insert(instance.id.clone(), instance.clone());
        self.publish(&instance.id).await;
        if let Some(action) = starting {
            self.dispatch(&instance.id, &action).await?;
        }
        let started = self.state.lock().await.instances.get(&instance.id).cloned();
        Ok(started.unwrap_or(instance))
    }

    pub async fn start_players(&self, session: &str, workflow_id: &str, players: usize) -> Result<Vec<Instance>, String> {
        let mut started = Vec::new();
        for n in 1..=players.clamp(1, 8) {
            let player = Player::numbered(n);
            started.push(self.start(session, workflow_id, &player, HashMap::new(), HashMap::new()).await?);
        }
        Ok(started)
    }

    /// A move from the page: the inputs land in the responses, then the
    /// workflow action runs.
    pub async fn act(&self, instance_id: &str, action_id: &str, inputs: HashMap<String, Value>) -> Result<(), String> {
        let target = {
            let mut state = self.state.lock().await;
            let workflow_id = state.instances.get(instance_id).ok_or("no such instance")?.workflow_id.clone();
            let definition = state.definitions.get(&workflow_id).ok_or("the workflow is gone")?;
            let action = definition
                .actions
                .iter()
                .find(|a| a.id == action_id)
                .ok_or_else(|| format!("the workflow has no action {action_id}"))?
                .clone();
            let instance = state.instances.get_mut(instance_id).ok_or("no such instance")?;
            instance.responses.extend(inputs);
            match (&action.action_type, &action.target) {
                (ActionType::RunServerAction, Some(target)) => Some(target.clone()),
                (ActionType::TransformThenRun { ops, input_map, server_action_id, .. }, _) => {
                    apply_transform_ops(&mut instance.responses, ops, &mut instance.context)?;
                    if let Some(map) = input_map {
                        apply_input_map_into_responses(&mut instance.responses, &instance.context, map);
                    }
                    Some(server_action_id.clone())
                }
                (ActionType::TransformResponses { ops, .. }, _) => {
                    apply_transform_ops(&mut instance.responses, ops, &mut instance.context)?;
                    None
                }
                (ActionType::NextNode | ActionType::PreviousNode | ActionType::Noop, _) => None,
                (other, _) => return Err(format!("devhost does not run {other:?} actions yet")),
            }
        };
        self.publish(instance_id).await;
        match target {
            Some(target) => self.dispatch(instance_id, &target).await,
            None => Ok(()),
        }
    }

    /// Hands a server action to the bot and waits for its answer.
    pub async fn dispatch(&self, instance_id: &str, action_id: &str) -> Result<(), String> {
        // tawks runs some server actions itself, such as awarding score.
        if BUILT_IN_ACTIONS.contains(&action_id) {
            let session = self.session_of(instance_id).await;
            self.log_to(session.as_deref(), "info", format!("{action_id} (tawks would handle this)"));
            return Ok(());
        }
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
            session: "playground".to_string(),
            workflow_id: workflow_id.to_string(),
            user_id: user.id.clone(),
            responses,
            context,
            completed: false,
            complete_message: None,
            parent_instance_id: None,
            inject: None,
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
            state.pending.insert(token.clone(), Pending { instance_id: instance_id.into(), reply, claimed_by: None });
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

    /// The first copy of a bot to claim a request handles it; the rest leave it.
    pub async fn claim(&self, token: &str, registrar: &str) -> bool {
        let mut state = self.state.lock().await;
        let Some(pending) = state.pending.get_mut(token) else { return false };
        match &pending.claimed_by {
            Some(owner) => owner == registrar,
            None => {
                pending.claimed_by = Some(registrar.to_string());
                true
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
                ServerActionResult::ScheduleJobs { inputs, context, jobs, .. } => {
                    instance.responses.extend(inputs);
                    instance.context.extend(context);
                    let now = Instant::now();
                    for job in jobs {
                        let duplicate = state
                            .jobs
                            .iter()
                            .any(|(_, id, queued)| id == instance_id && queued.idempotency_key == job.idempotency_key);
                        if !duplicate {
                            let due = now + Duration::from_millis(job.after_ms.into());
                            state.jobs.push((due, instance_id.to_string(), job));
                        }
                    }
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

    /// Runs the scheduled jobs that are due, as tawks' scheduler would.
    pub async fn run_due_jobs(&self) {
        let due: Vec<(String, ScheduledJob)> = {
            let mut state = self.state.lock().await;
            let now = Instant::now();
            let (ready, waiting): (Vec<_>, Vec<_>) = state.jobs.drain(..).partition(|(at, _, _)| *at <= now);
            state.jobs = waiting;
            ready.into_iter().map(|(_, id, job)| (id, job)).collect()
        };
        for (instance_id, job) in due {
            let runs = {
                let state = self.state.lock().await;
                state.instances.get(&instance_id).is_some_and(|i| {
                    job.guard.as_ref().is_none_or(|guard| eval_condition(guard, &i.responses, &i.context))
                })
            };
            if !runs {
                continue;
            }
            let outcome = match job.action {
                ScheduledAction::RunServerAction { action_id, payload } => {
                    let inputs = payload.as_object().map(|o| o.clone().into_iter().collect()).unwrap_or_default();
                    self.act(&instance_id, &action_id, inputs).await
                }
                ScheduledAction::UpdateResponses { updates, .. } => {
                    if let Some(instance) = self.state.lock().await.instances.get_mut(&instance_id) {
                        instance.responses.extend(updates);
                    }
                    self.publish(&instance_id).await;
                    Ok(())
                }
                ScheduledAction::TransformResponses { ops, .. } => {
                    let applied = match self.state.lock().await.instances.get_mut(&instance_id) {
                        Some(instance) => apply_transform_ops(&mut instance.responses, &ops, &mut instance.context).map(|_| ()),
                        None => Ok(()),
                    };
                    self.publish(&instance_id).await;
                    applied
                }
            };
            if let Err(error) = outcome {
                let session = self.session_of(&instance_id).await;
                self.log_to(session.as_deref(), "error", format!("scheduled job {}: {error}", job.idempotency_key));
            }
        }
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
        self.start(&old.session, &old.workflow_id, &user, HashMap::new(), HashMap::new()).await
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

    /// Clears one session's instances and gives it a new lobby, so its next Start is a fresh game.
    pub async fn reset(&self, session: &str) {
        let mut state = self.state.lock().await;
        let gone: Vec<String> = state.instances.values().filter(|i| i.session == session).map(|i| i.id.clone()).collect();
        state.instances.retain(|_, i| i.session != session);
        state.pending.retain(|_, p| !gone.contains(&p.instance_id));
        state.lobbies.remove(session);
    }

    /// The session a bot means when it starts a workflow for a player: the
    /// newest one that player has an instance in.
    pub async fn session_of_lobby(&self, lobby_id: &str) -> Option<String> {
        let state = self.state.lock().await;
        state.lobbies.iter().find(|(_, lobby)| lobby.as_str() == lobby_id).map(|(session, _)| session.clone())
    }

    pub async fn definition(&self, workflow_id: &str) -> Option<CreateWorkflowDefinition> {
        self.state.lock().await.definitions.get(workflow_id).cloned()
    }

    pub async fn session_of_player(&self, user_id: &str) -> Option<String> {
        let state = self.state.lock().await;
        state.instances.values().filter(|i| i.user_id == user_id).max_by(|a, b| a.id.cmp(&b.id)).map(|i| i.session.clone())
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
