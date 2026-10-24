//! A local stand-in for tawks. Point a bot at it (`RSPC_URL` and
//! `EVENTS_WS_URL`), open the page, and play your game as several fake
//! players at once. It fakes only what bots and activity frames use.

mod host;

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::extract::Request;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, AUTHORIZATION, HOST,
    ORIGIN, VARY,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{SinkExt, Stream, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::BroadcastStream;
use workflow_model::CreateWorkflowDefinition;
use workflow_model::server_action::ServerActionResult;

use host::Host;

type Shared = Arc<Host>;

fn ok(data: Value) -> Json<Value> {
    Json(json!({ "jsonrpc": "2.0", "id": null, "result": { "type": "response", "data": data } }))
}

fn fail(message: impl Into<String>) -> Json<Value> {
    Json(json!({ "jsonrpc": "2.0", "id": null, "result": { "type": "error", "data": { "code": 400, "message": message.into() } } }))
}

fn parse<T: serde::de::DeserializeOwned>(input: Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|e| format!("bad input: {e}"))
}

#[derive(Deserialize)]
struct RespondArgs {
    token: String,
    result: ServerActionResult,
}

#[derive(Deserialize)]
struct ReconcileArgs {
    instance_id: String,
    #[serde(default)]
    inputs: HashMap<String, Value>,
    #[serde(default)]
    context: HashMap<String, Value>,
    #[serde(default)]
    complete_message: Option<String>,
}

#[derive(Deserialize)]
struct ProcessArgs {
    instance_id: String,
    action_id: String,
    #[serde(default)]
    inputs: HashMap<String, Value>,
}

#[derive(Deserialize, Default)]
struct BroadcastFilter {
    user_id: Option<String>,
    response_equals: Option<(String, Value)>,
    #[serde(default)]
    include_completed: bool,
}

#[derive(Deserialize)]
struct BroadcastArgs {
    workflow_definition_id: String,
    #[serde(default)]
    filter: BroadcastFilter,
    #[serde(default)]
    updates: HashMap<String, Value>,
}

#[derive(Deserialize)]
struct StartOnBehalfArgs {
    workflow_definition_id: String,
    #[serde(default)]
    input: HashMap<String, Value>,
    #[serde(default)]
    context: HashMap<String, Value>,
    user_id: String,
}

#[derive(Deserialize)]
struct StartFromInstanceArgs {
    instance_id: String,
    workflow_definition_id: String,
    #[serde(default)]
    context: HashMap<String, Value>,
    #[serde(default)]
    inputs: HashMap<String, Value>,
}

/// The bot's own token stands in for its identity and its access token.
fn bot_of(headers: &HeaderMap) -> String {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_whitespace().last())
        .unwrap_or("dev")
        .to_string()
}

async fn rspc(
    State(host): State<Shared>,
    Path(procedure): Path<String>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Json<Value> {
    let input = body.map(|Json(v)| v).unwrap_or(Value::Null);
    match call(&host, &bot_of(&headers), &procedure, input).await {
        Ok(data) => ok(data),
        Err(error) => {
            host.log("warn", format!("{procedure}: {error}"));
            fail(error)
        }
    }
}

async fn call(host: &Shared, bot: &str, procedure: &str, input: Value) -> Result<Value, String> {
    match procedure {
        "auth_access_token" => Ok(json!({ "access_token": bot })),
        "workflow_register_workflow" => {
            let definition: CreateWorkflowDefinition = parse(input)?;
            Ok(json!(host.register_workflow(bot, definition).await))
        }
        "server_action_register" => {
            let action_id = input.get("action_id").and_then(Value::as_str).ok_or("action_id is required")?;
            let registrar = input.get("registrar_id").and_then(Value::as_str).map(String::from);
            Ok(json!(host.register_action(bot, action_id, registrar).await))
        }
        "server_action_renew_leases" => {
            let registrar = input.get("registrar_id").and_then(Value::as_str).unwrap_or_default();
            Ok(json!(host.renew(registrar).await))
        }
        "workflow_definition_availability" => {
            let ids: Vec<String> = input.get("workflow_ids").cloned().map(parse).transpose()?.unwrap_or_default();
            let mut rows = Vec::new();
            for id in ids {
                let available = host.has_workflow(&id).await;
                rows.push(json!({ "workflow_id": id, "available_now": available, "unavailable_reason": null }));
            }
            Ok(json!(rows))
        }
        "activity_register" => {
            host.register_activity(input).await;
            Ok(Value::Null)
        }
        "workflow_respond_server_action" => {
            let args: RespondArgs = parse(input)?;
            host.respond(&args.token, args.result).await?;
            Ok(Value::Null)
        }
        "workflow_reconcile_instance" => {
            let args: ReconcileArgs = parse(input)?;
            host.reconcile(&args.instance_id, args.inputs, args.context, args.complete_message).await?;
            Ok(Value::Null)
        }
        "server_action_process" => {
            let args: ProcessArgs = parse(input)?;
            host.reconcile(&args.instance_id, args.inputs, HashMap::new(), None).await?;
            dispatch_later(host.clone(), args.instance_id, args.action_id);
            Ok(Value::Null)
        }
        "workflow_process_action" => {
            let args: ProcessArgs = parse(input)?;
            host.act(&args.instance_id, &args.action_id, args.inputs).await?;
            Ok(Value::Null)
        }
        "workflow_start_on_behalf" => {
            let args: StartOnBehalfArgs = parse(input)?;
            let n = args.user_id.trim_start_matches("player-").parse().unwrap_or(1);
            let user = host::Player { id: args.user_id.clone(), ..host::player(n) };
            Ok(json!(host.start(&args.workflow_definition_id, &user, args.input, args.context).await?))
        }
        "workflow_start_from_instance" => {
            let args: StartFromInstanceArgs = parse(input)?;
            let user = host.user_of(&args.instance_id).await.ok_or("Workflow instance not found")?;
            Ok(json!(host.start(&args.workflow_definition_id, &user, args.inputs, args.context).await?))
        }
        "workflow_restart" => {
            let id = input.get("instance_id").and_then(Value::as_str).ok_or("instance_id is required")?;
            Ok(json!(host.restart(id).await?))
        }
        "lobby_speak" => {
            let text = input.get("text").and_then(Value::as_str).unwrap_or_default();
            host.log("chat", format!("bot says: {text}"));
            Ok(Value::Null)
        }
        "milestone_event" => {
            host.log("info", format!("milestone {}", input));
            Ok(Value::Null)
        }
        "channel_post_publish" => {
            let channel = input.get("channel_id").and_then(Value::as_str).unwrap_or_default();
            host.log("info", format!("published the post of {channel}"));
            Ok(json!(true))
        }
        "workflow_broadcast_updates" => {
            let args: BroadcastArgs = parse(input)?;
            let count = host
                .broadcast(
                    &args.workflow_definition_id,
                    args.filter.user_id.as_deref(),
                    args.filter.response_equals,
                    args.filter.include_completed,
                    args.updates,
                )
                .await;
            Ok(json!({ "count": count, "instances": [] }))
        }
        "game_result_record" => {
            host.log("info", format!("game result {input}"));
            Ok(Value::Null)
        }
        "game_result_recent" | "game_result_leaderboard" => Ok(json!([])),
        "bot_notifications_register_kinds" | "bot_notifications_send" | "presence_set_activity" | "workflow_cancel_job" => {
            Ok(Value::Null)
        }
        "bot_notifications_subscribers" => Ok(json!([])),
        other => Err(format!("devhost does not fake {other}")),
    }
}

/// Server actions the bot asks for itself run after its request returns, so
/// the bot is free to answer them.
fn dispatch_later(host: Shared, instance_id: String, action_id: String) {
    tokio::spawn(async move {
        if let Err(error) = host.dispatch(&instance_id, &action_id).await {
            host.log("error", error);
        }
    });
}

#[derive(Deserialize)]
struct EventsQuery {
    token: Option<String>,
}

async fn events(
    State(host): State<Shared>,
    Query(query): Query<EventsQuery>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    let token = query.token.unwrap_or_else(|| "dev".into());
    upgrade.on_upgrade(move |socket| bot_socket(host, token, socket))
}

async fn bot_socket(host: Shared, token: String, socket: WebSocket) {
    let (mut write, mut read) = socket.split();
    let (sender, mut outgoing) = mpsc::unbounded_channel::<String>();
    host.connect_bot(&token, sender).await;
    let writer = tokio::spawn(async move {
        while let Some(text) = outgoing.recv().await {
            if write.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });
    while let Some(Ok(message)) = read.next().await {
        if let Message::Close(_) = message {
            break;
        }
    }
    writer.abort();
    host.log("warn", format!("bot {token} disconnected from the events socket"));
}

async fn page() -> Html<&'static str> {
    Html(include_str!("page.html"))
}

async fn dev_state(State(host): State<Shared>) -> Json<Value> {
    let (activities, instances, lobby_id) = host.snapshot().await;
    let mut displays = HashMap::new();
    for activity in &activities {
        if let Some(id) = activity.get("workflow_id").and_then(Value::as_str) {
            displays.insert(id.to_string(), host.activity_display(id).await);
        }
    }
    Json(json!({ "activities": activities, "displays": displays, "instances": instances, "lobby_id": lobby_id }))
}

async fn dev_stream(State(host): State<Shared>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(host.page.subscribe())
        .filter_map(|event| async move { event.ok() })
        .map(|event| Ok(Event::default().json_data(event).unwrap_or_default()));
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Deserialize)]
struct StartGame {
    workflow_id: String,
    players: usize,
}

async fn dev_start(State(host): State<Shared>, Json(args): Json<StartGame>) -> Json<Value> {
    host.reset().await;
    match host.start_players(&args.workflow_id, args.players).await {
        Ok(instances) => Json(json!({ "ok": true, "instances": instances })),
        Err(error) => {
            host.log("error", error.clone());
            Json(json!({ "ok": false, "error": error }))
        }
    }
}

#[derive(Deserialize)]
struct Act {
    instance_id: String,
    action: String,
    inputs: HashMap<String, Value>,
}

async fn dev_act(State(host): State<Shared>, Json(args): Json<Act>) -> Json<Value> {
    match host.act(&args.instance_id, &args.action, args.inputs).await {
        Ok(()) => Json(json!({ "ok": true })),
        Err(error) => {
            host.log("error", error.clone());
            Json(json!({ "ok": false, "error": error }))
        }
    }
}

async fn dev_workflows(State(host): State<Shared>) -> Json<Value> {
    Json(json!({ "workflows": host.workflows().await }))
}

#[derive(Deserialize)]
struct Run {
    workflow_id: String,
    action_id: String,
    #[serde(default)]
    responses: HashMap<String, Value>,
    #[serde(default)]
    context: HashMap<String, Value>,
}

async fn dev_run(State(host): State<Shared>, Json(args): Json<Run>) -> Json<Value> {
    host.log("info", format!("playground ran {} on {}", args.action_id, args.workflow_id));
    match host.run_for_playground(&args.workflow_id, &args.action_id, args.responses, args.context).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })),
        Err(error) => {
            host.log("error", error.clone());
            Json(json!({ "ok": false, "error": error }))
        }
    }
}

/// Endpoints the hosted playground and API explorer may call from their own origin.
const PLAYGROUND_PATHS: [&str; 2] = ["/dev/workflows", "/dev/run"];

fn for_docs_site(path: &str) -> bool {
    PLAYGROUND_PATHS.contains(&path) || path.starts_with("/rspc/")
}
const PLAYGROUND_ORIGINS: [&str; 3] = ["https://tawks-dev.vercel.app", "http://localhost:5300", "http://127.0.0.1:5300"];

fn playground_origins() -> Vec<String> {
    let mut origins: Vec<String> = PLAYGROUND_ORIGINS.iter().map(|o| o.to_string()).collect();
    if let Ok(extra) = std::env::var("DEVHOST_PLAYGROUND_ORIGINS") {
        origins.extend(extra.split(',').map(|o| o.trim().trim_end_matches('/').to_string()).filter(|o| !o.is_empty()));
    }
    origins
}

/// Only this machine may talk to devhost. Checking Host stops DNS rebinding,
/// and checking Origin stops other sites in the developer's browser from
/// driving the page's endpoints or hijacking a bot's events socket. The
/// playground's origins may reach its endpoints and the API, and nothing else.
async fn local_only(port: u16, playground: Arc<Vec<String>>, request: Request, next: Next) -> Result<Response, StatusCode> {
    let local = [format!("localhost:{port}"), format!("127.0.0.1:{port}")];
    let host_ok = request
        .headers()
        .get(HOST)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| local.iter().any(|l| l == h));
    if !host_ok {
        return Err(StatusCode::FORBIDDEN);
    }
    let Some(origin) = request.headers().get(ORIGIN).and_then(|o| o.to_str().ok()).map(String::from) else {
        return Ok(next.run(request).await);
    };
    if local.iter().any(|l| origin == format!("http://{l}")) {
        return Ok(next.run(request).await);
    }
    let for_playground = for_docs_site(request.uri().path()) && playground.iter().any(|o| *o == origin);
    if !for_playground {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut response = if request.method() == Method::OPTIONS {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(request).await
    };
    let headers = response.headers_mut();
    let origin = HeaderValue::from_str(&origin).map_err(|_| StatusCode::FORBIDDEN)?;
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    headers.insert(ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST"));
    headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("authorization, content-type"));
    headers.insert("access-control-allow-private-network", HeaderValue::from_static("true"));
    headers.insert(VARY, HeaderValue::from_static("origin"));
    Ok(response)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .init();

    let port: u16 = std::env::var("DEVHOST_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787);
    let host: Shared = Arc::new(Host::new());
    let playground = Arc::new(playground_origins());
    let app = Router::new()
        .route("/", get(page))
        .route("/rspc/:procedure", post(rspc))
        .route("/events", get(events))
        .route("/dev/state", get(dev_state))
        .route("/dev/stream", get(dev_stream))
        .route("/dev/start", post(dev_start))
        .route("/dev/act", post(dev_act))
        .route("/dev/workflows", get(dev_workflows))
        .route("/dev/run", post(dev_run))
        .layer(middleware::from_fn(move |request, next| local_only(port, playground.clone(), request, next)))
        .with_state(host);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!("tawks devhost on http://localhost:{port}");
    println!("  RSPC_URL=http://localhost:{port}/rspc");
    println!("  EVENTS_WS_URL=ws://localhost:{port}/events");
    println!("  BOT_TOKEN=<any name, different for each bot>");
    axum::serve(listener, app).await?;
    Ok(())
}
