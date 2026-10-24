use crate::{sdk::TawksSdk, types::*};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic;
use std::time::{Duration, Instant};
use crate::types::SpeakInLobbyArgs;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::debug;
use tracing::error;
use url::Url;
use workflow::CreateWorkflowDefinition;
use workflow::service::ProcessWorkflowActionArgs;
use workflow::service::StartWorkflowFromInstanceArgs;
use workflow::service::StartWorkflowOnBehalfArgs;
use workflow::service::WorkflowResource;
use workflow::service::WorkflowResourceWithContext;


const WORKFLOW_UPDATED: &str = "workflow_updated";
const WORKFLOW_CREATED: &str = "workflow_created";

#[derive(Deserialize)]
struct Peek {
    #[serde(rename = "type")]
    kind: String,
}
type ServerActionResult = crate::workflow::server_action::ServerActionResult;
pub type ServerActionHandler = Arc<
    Box<
        dyn Fn(
                WorkflowResourceWithContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = ServerActionHandlerResult> + Send>,
            > + Send
            + Sync,
    >,
>;

#[derive(Clone)]
pub struct TawksBot {
    pub sdk: Arc<TawksSdk>,
    shutdown_notify: Arc<Notify>,
    running: Arc<atomic::AtomicBool>,
    server_actions: Arc<RwLock<HashMap<String, ServerActionHandler>>>,
    registered_action_ids: Arc<RwLock<HashSet<String>>>,
    registered_workflow_ids: Arc<RwLock<Vec<String>>>,
    recent_server_action_tokens: Arc<RwLock<HashMap<String, Instant>>>,
    registrar_id: String,
    events_ws_url: String,
}

impl std::fmt::Debug for TawksBot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TawksBot")
            .field("shutdown_notify", &self.shutdown_notify)
            .field("running", &self.running)
            .finish()
    }
}

pub type ServerActionHandlerResult = anyhow::Result<ServerActionResult>;

#[derive(Debug, serde::Deserialize)]
struct WorkflowAvailability {
    workflow_id: String,
    available_now: bool,
    #[serde(default)]
    unavailable_reason: Option<String>,
}

impl TawksBot {
    pub async fn new(opts: BotOptions) -> anyhow::Result<Self> {
        let sdk = Arc::new(TawksSdk::new(
            &opts.rspc_url,
            &format!(
                "{} {}",
                opts.token_type.clone().unwrap_or("Bearer".into()),
                opts.token
            ),
        )?);

        Ok(Self {
            sdk,
            shutdown_notify: Arc::new(Notify::new()),
            running: Arc::new(atomic::AtomicBool::new(true)),
            server_actions: Arc::new(RwLock::new(HashMap::new())),
            registered_action_ids: Arc::new(RwLock::new(HashSet::new())),
            registered_workflow_ids: Arc::new(RwLock::new(Vec::new())),
            recent_server_action_tokens: Arc::new(RwLock::new(HashMap::new())),
            registrar_id: format!("rs-bot-{}", ulid::Ulid::new().to_string().to_lowercase()),
            events_ws_url: opts.events_ws_url.clone(),
        })
    }

    pub async fn register_workflow(
        &self,
        workflow_definition: CreateWorkflowDefinition,
    ) -> anyhow::Result<String> {
        tracing::debug!("Attempting to register {}", workflow_definition.id);
        let final_id = self
            .sdk
            .web_client
            .mutate::<_, String>("workflow_register_workflow", &workflow_definition)
            .await?;
        tracing::info!("Registered workflow {final_id}");
        self.registered_workflow_ids
            .write()
            .await
            .push(final_id.clone());
        Ok(final_id)
    }

    pub async fn register_server_action(
        &self,
        action_id: impl Into<String>,
        handler: ServerActionHandler,
    ) -> anyhow::Result<String> {
        let action_id = action_id.into();
        tracing::debug!("Attempting to register {action_id}");
        let final_id = self
            .sdk
            .server_actions
            .register_with_registrar(&action_id, Some(&self.registrar_id))
            .await?;
        tracing::info!("Registered server action {final_id}");
        self.server_actions
            .write()
            .await
            .insert(final_id.clone(), handler);
        self.registered_action_ids.write().await.insert(action_id);
        Ok(final_id)
    }

    pub async fn start<F, Fut>(&self, on_event: F) -> anyhow::Result<()>
    where
        F: FnMut(Event, &TawksSdk) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        crate::web_client::require_tls(&self.events_ws_url)?;
        let mut ws_url = Url::parse(&self.events_ws_url)?;
        let token = self.sdk().web_client.fetch_access_token().await?;
        let mut params: Vec<(String, String)> = ws_url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        params.push(("token".to_string(), token));
        let mut qp = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in params {
            qp.append_pair(&k, &v);
        }
        ws_url.set_query(Some(&qp.finish()));

        let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
        let (ws_write, mut ws_read) = ws_stream.split();

        let sdk = self.sdk.clone();
        let event_notify = self.shutdown_notify.clone();
        let running = self.running.clone();
        let server_actions = self.server_actions.clone();
        let registered_action_ids = self.registered_action_ids.clone();
        let recent_server_action_tokens = self.recent_server_action_tokens.clone();

        let write_for_actions = Arc::new(tokio::sync::Mutex::new(ws_write));

        // The events server only marks a user present on a `heartbeat`
        // message, the same one the app sends, so a running bot shows online.
        let hb_write = write_for_actions.clone();
        let heartbeat_task = tokio::spawn(async move {
            use tokio::time::{Duration, interval};
            let mut tick = interval(Duration::from_secs(25));
            loop {
                tick.tick().await;
                if let Ok(mut w) = hb_write.try_lock() {
                    let beat = Message::Text(r#"{"type":"heartbeat","niche_id":null,"channel_id":null}"#.into());
                    if let Err(e) = w.send(beat).await {
                        tracing::warn!("heartbeat failed: {e}");
                        break; // connection is broken, let reconnect logic take over
                    }
                }
            }
        });

        let renew_sdk = sdk.clone();
        let renew_running = running.clone();
        let renew_registrar_id = self.registrar_id.clone();
        let renew_registered = registered_action_ids.clone();
        let renew_workflows = self.registered_workflow_ids.clone();
        let renew_notify = event_notify.clone();
        let renewal_task = tokio::spawn(async move {
            use tokio::time::{Duration, interval};
            let mut tick = interval(Duration::from_secs(15));
            loop {
                tick.tick().await;
                if !renew_running.load(atomic::Ordering::SeqCst) {
                    break;
                }

                match renew_sdk
                    .server_actions
                    .renew_leases(&renew_registrar_id)
                    .await
                {
                    Ok(renewed) => {
                        // Fewer leases than we registered means the API lost
                        // its registry (a restart); end this session so the
                        // caller's reconnect loop registers everything again.
                        let expected = renew_registered.read().await.len() as u32;
                        if renewed < expected {
                            tracing::warn!(
                                registrar_id = %renew_registrar_id,
                                renewed,
                                expected,
                                "server action leases lost; reconnecting to re-register"
                            );
                            renew_notify.notify_waiters();
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            registrar_id = %renew_registrar_id,
                            error = %error,
                            "failed to bulk-renew server action leases"
                        );
                        continue;
                    }
                }

                // Workflow definitions registered by bots live in the API's
                // memory, so an API restart drops them until we register again.
                let workflow_ids = renew_workflows.read().await.clone();
                if workflow_ids.is_empty() {
                    continue;
                }
                match renew_sdk
                    .web_client
                    .query::<_, Vec<WorkflowAvailability>>(
                        "workflow_definition_availability",
                        &serde_json::json!({ "workflow_ids": workflow_ids }),
                    )
                    .await
                {
                    Ok(rows) => {
                        if let Some(missing) = rows.iter().find(|r| !r.available_now) {
                            tracing::warn!(
                                workflow_id = %missing.workflow_id,
                                reason = ?missing.unavailable_reason,
                                "workflow definition gone from the API; reconnecting to re-register"
                            );
                            renew_notify.notify_waiters();
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "workflow availability check failed");
                    }
                }
            }
        });

        let read_task = tokio::spawn({
            let mut on_event = on_event;
            let sdk = sdk.clone();
            let event_notify = event_notify.clone();
            let running = running.clone();
            let server_actions = server_actions.clone();
            async move {
                on_event(Event::Connect, &sdk).await;
                loop {
                    tokio::select! {
                                            _ = event_notify.notified() => break,
                                            next = ws_read.next() => {
                                                match next {
                                                    Some(Ok(Message::Text(txt))) => {
                                                        match serde_json::from_str::<Peek>(&txt) {
                                                            Ok(Peek { kind }) if kind == WORKFLOW_UPDATED || kind == WORKFLOW_CREATED => {}
                                                            Ok(_) => match serde_json::from_str::<MessageEvent>(&txt) {
                                                                Ok(MessageEvent::WorkflowRequestServerAction { action_id, id, workflow }) => {
                                                                    let duplicate = {
                                                                        let mut recent = recent_server_action_tokens.write().await;
                                                                        let now = Instant::now();
                                                                        recent.retain(|_, seen_at| {
                                                                            now.duration_since(*seen_at) < Duration::from_secs(600)
                                                                        });
                                                                        if recent.contains_key(&id) {
                                                                            debug!(
                                                                                "Ignoring duplicate workflow server action token {}",
                                                                                id
                                                                            );
                                                                            true
                                                                        } else {
                                                                            recent.insert(id.clone(), now);
                                                                            false
                                                                        }
                                                                    };
                                                                    if !duplicate {
                                                                        let handler = {
                                                                            let lock = server_actions.read().await;
                                                                            lock.get(&action_id).cloned()
                                                                        };
                                                                        match handler {
                                                                            Some(handler) => {
                                                                                let sdk = sdk.clone();
                                                                                tokio::spawn(async move {
                        debug!(
                            "Handling WorkflowRequestServerAction:\naction_id:\n{} id:\n{} inputs:\n{:?}",
                            action_id, id, workflow.workflow.responses
                        );

                        let payload = match handler(workflow).await {
                            Ok(result) => {
                                json!({ "token": id, "result": result })
                            }
                            Err(err) => {
                                json!({ "token": id, "result": ServerActionResult::Error(err.to_string()) })
                            }
                        };

                        if let Err(e) = sdk.web_client.mutate::<_, ()>("workflow_respond_server_action", &payload).await {
                            let err_str = format!("{:?}", e);
                            if err_str.contains("No external action context")
                                || err_str.contains("Workflow instance not found")
                                || err_str.contains("unexpected rspc result kind: error")
                            {
                                // Stale token or server-side error — workflow was cleared/restarted.
                                tracing::debug!(
                                    "Stale token {} for action {} — skipping ({})",
                                    id, action_id, err_str.chars().take(120).collect::<String>()
                                );
                            } else {
                                tracing::warn!(
                                    "Failed to respond to workflow_respond_server_action (token={}, action_id={}): {:?}",
                                    id, action_id, e
                                );
                            }
                        }
                    });
                                                                            }
                                                                            None => {
                                                                                error!("no handler found");
                                                                                let message = MessageEvent::WorkflowRequestServerAction { action_id, id, workflow };
                                                                                on_event(Event::Message { message }, &sdk).await;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Ok(message) => {
                                                                    debug!("Received unhandled message: {:?}", message);
                                                                    on_event(Event::Message { message }, &sdk).await;
                                                                }
                                                                Err(e) => {
                                                                    eprintln!("Failed to parse MessageEvent: {e:?}");
                                                                }
                                                            },
                                                            Err(e) => {
                                                                eprintln!("Failed to parse MessageEvent: {e:?}");
                                                            }
                                                        }
                                                    }
                                                    Some(Ok(Message::Close(_))) | None => {
                                                        on_event(Event::Disconnect, &sdk).await;
                                                        break;
                                                    }
                                                    Some(Ok(_)) => {}
                                                    Some(Err(e)) => {
                                                        on_event(Event::Error { error: format!("ws error: {}", e) }, &sdk).await;
                                                        break;
                                                    }
                                                }
                                                if !running.load(atomic::Ordering::SeqCst) {
                                                    break;
                                                }
                                            }
                                        }
                }
            }
        });

        let shutdown = self.shutdown_notify.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            running.store(false, atomic::Ordering::SeqCst);
            shutdown.notify_waiters();
        });

        read_task.await?;
        heartbeat_task.abort();
        renewal_task.abort();

        Ok(())
    }

    pub async fn disconnect(&self) {
        self.running.store(false, atomic::Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
    }

    pub fn sdk(&self) -> &TawksSdk {
        &self.sdk
    }

    pub async fn start_workflow_from_instance(
        &self,
        args: StartWorkflowFromInstanceArgs,
    ) -> Result<WorkflowResource> {
        self.sdk
            .web_client
            .mutate("workflow_start_from_instance", &args)
            .await
    }

    pub async fn workflow_process_action(
        &self,
        args: ProcessWorkflowActionArgs,
    ) -> Result<WorkflowResource> {
        self.sdk
            .web_client
            .mutate("workflow_process_action", &args)
            .await
    }

    pub async fn speak(&self, lobby_id: String, text: String) -> Result<()> {
        self.sdk
            .web_client
            .mutate("lobby_speak", &SpeakInLobbyArgs { text, lobby_id })
            .await
    }

    pub async fn start_workflow_on_behalf(
        &self,
        args: StartWorkflowOnBehalfArgs,
    ) -> Result<WorkflowResource> {
        self.sdk
            .web_client
            .mutate("workflow_start_on_behalf", &args)
            .await
    }

    pub async fn workflow_restart(
        &self,
        instance_id: &str,
        input: HashMap<String, Value>,
    ) -> Result<WorkflowResource> {
        self.sdk
            .web_client
            .mutate(
                "workflow_restart",
                &json!({
                                "instance_id": instance_id,
                "input": input,
                "context": {},
                        }),
            )
            .await
    }
}
