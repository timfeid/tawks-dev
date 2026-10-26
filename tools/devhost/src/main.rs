//! A stand-in for tawks. Point a bot at it (`RSPC_URL` and `EVENTS_WS_URL`),
//! open the page, and play your game as several fake players at once. It
//! fakes only what bots and activity frames use.
//!
//! Locally it trusts this machine and nothing else. With DEVHOST_PUBLIC_HOST
//! set it runs as a hosted sandbox: bots need a sandbox token, made by
//! signing in with a tawks account, and every visitor gets lobbies of their own.

mod host;
mod tokens;

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, AUTHORIZATION, HOST,
    ORIGIN, VARY,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{SinkExt, Stream, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::BroadcastStream;
use workflow_model::CreateWorkflowDefinition;
use workflow_model::server_action::ServerActionResult;

use host::{Host, PageEvent};
use tokens::Tokens;

const LOCAL_SESSION: &str = "local";
const MAX_BODY_BYTES: usize = 512 * 1024;
const MAX_DEFINITION_BYTES: usize = 256 * 1024;
const MAX_WORKFLOWS_PER_BOT: usize = 20;
const MAX_ACTIONS_PER_BOT: usize = 100;
const MAX_SESSIONS: usize = 500;
const SESSION_IDLE: Duration = Duration::from_secs(30 * 60);

/// Pages that may call devhost from their own origin: the docs site's
/// playground, API explorer and Play page.
const DOCS_ORIGINS: [&str; 3] = ["https://tawks-dev.vercel.app", "http://localhost:5300", "http://127.0.0.1:5300"];

struct Public {
    /// The sandbox's own host name, such as sandbox.tawks.app.
    host: String,
    tawks_app: String,
    tawks_rspc: String,
    tokens: Tokens,
    http: reqwest::Client,
}

struct App {
    host: Host,
    public: Option<Public>,
    port: u16,
    docs_origins: Vec<String>,
}

type Shared = Arc<App>;

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
struct ClaimArgs {
    token: String,
    registrar_id: String,
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
    #[serde(default)]
    inject: Option<Value>,
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_whitespace().last())
        .map(String::from)
}

/// Which bot is calling. Locally the token is the bot's name; in the sandbox
/// it has to be a token the sandbox made.
async fn bot_for(app: &App, token: Option<String>) -> Option<String> {
    match &app.public {
        None => Some(token.unwrap_or_else(|| "dev".into())),
        Some(public) => public.tokens.bot_for(&token?).await,
    }
}

async fn rspc(
    State(app): State<Shared>,
    Path(procedure): Path<String>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Json<Value> {
    let token = bearer(&headers);
    let Some(bot) = bot_for(&app, token.clone()).await else {
        return fail("Unknown sandbox token. Make one at /sandbox/token.");
    };
    // The events socket takes the same token, so it doubles as the access token.
    if procedure == "auth_access_token" {
        return ok(json!({ "access_token": token.unwrap_or(bot) }));
    }
    let input = body.map(|Json(v)| v).unwrap_or(Value::Null);
    match call(&app, &bot, &procedure, input).await {
        Ok(data) => ok(data),
        Err(error) => {
            app.host.log("warn", format!("{bot} {procedure}: {error}"));
            fail(error)
        }
    }
}

async fn call(app: &Shared, bot: &str, procedure: &str, input: Value) -> Result<Value, String> {
    let host = &app.host;
    let public = app.public.is_some();
    match procedure {
        "workflow_register_workflow" => {
            if public {
                if input.to_string().len() > MAX_DEFINITION_BYTES {
                    return Err(format!("A sandbox workflow can be at most {} KB.", MAX_DEFINITION_BYTES / 1024));
                }
                let id = input.get("id").and_then(Value::as_str).unwrap_or_default();
                let exists = host.has_workflow(&format!("wf-{id}-{bot}")).await;
                if !exists && host.registrations_of(bot).await.0 >= MAX_WORKFLOWS_PER_BOT {
                    return Err(format!("A sandbox bot can register {MAX_WORKFLOWS_PER_BOT} workflows."));
                }
            }
            let definition: CreateWorkflowDefinition = parse(input)?;
            Ok(json!(host.register_workflow(bot, definition).await))
        }
        "server_action_register" => {
            let action_id = input.get("action_id").and_then(Value::as_str).ok_or("action_id is required")?;
            if public
                && !host.has_action(&format!("sa-{action_id}-{bot}")).await
                && host.registrations_of(bot).await.1 >= MAX_ACTIONS_PER_BOT
            {
                return Err(format!("A sandbox bot can register {MAX_ACTIONS_PER_BOT} server actions."));
            }
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
            // Devhost only runs games with a web app; ones drawn by their workflow need tawks.
            if input.get("client_url").and_then(Value::as_str).is_none() {
                return Ok(Value::Null);
            }
            if public {
                let workflow = input.get("workflow_id").and_then(Value::as_str).unwrap_or_default();
                if !workflow.ends_with(&format!("-{bot}")) {
                    return Err("An activity must use one of your own workflows.".into());
                }
                let url = input.get("client_url").and_then(Value::as_str).unwrap_or_default();
                if !url.starts_with("https://") {
                    return Err("A sandbox activity needs an https client_url, so every visitor can load it.".into());
                }
            }
            let mut activity = input;
            activity["bot"] = json!(bot);
            host.register_activity(activity).await;
            Ok(Value::Null)
        }
        "workflow_claim_server_action" => {
            let args: ClaimArgs = parse(input)?;
            Ok(json!(host.claim(&args.token, &args.registrar_id).await))
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
            dispatch_later(app.clone(), args.instance_id, args.action_id);
            Ok(Value::Null)
        }
        "workflow_process_action" => {
            let args: ProcessArgs = parse(input)?;
            host.act(&args.instance_id, &args.action_id, args.inputs).await?;
            Ok(Value::Null)
        }
        "workflow_start_on_behalf" => {
            let args: StartOnBehalfArgs = parse(input)?;
            let session = match host.session_of_player(&args.user_id).await {
                Some(session) => session,
                None if !public => LOCAL_SESSION.to_string(),
                None => return Err(format!("{} is not in any sandbox lobby", args.user_id)),
            };
            let n = args.user_id.trim_start_matches("player-").parse().unwrap_or(1);
            let user = host::Player { id: args.user_id.clone(), ..host::player(n) };
            let workflow_id = host.resolve_workflow(&args.workflow_definition_id, bot).await;
            Ok(json!(host.start(&session, &workflow_id, &user, args.input, args.context).await?))
        }
        "workflow_start_from_instance" => {
            let args: StartFromInstanceArgs = parse(input)?;
            let user = host.user_of(&args.instance_id).await.ok_or("Workflow instance not found")?;
            let session = host.session_of(&args.instance_id).await.ok_or("Workflow instance not found")?;
            let workflow_id = host.resolve_workflow(&args.workflow_definition_id, bot).await;
            let started = host
                .start_within(
                    &session,
                    &workflow_id,
                    &user,
                    args.inputs,
                    args.context,
                    Some(args.instance_id),
                    args.inject,
                )
                .await?;
            Ok(json!(started))
        }
        "workflow_restart" => {
            let id = input.get("instance_id").and_then(Value::as_str).ok_or("instance_id is required")?;
            Ok(json!(host.restart(id).await?))
        }
        "lobby_speak" => {
            let text = input.get("text").and_then(Value::as_str).unwrap_or_default();
            let lobby = input.get("lobby_id").and_then(Value::as_str).unwrap_or_default();
            let session = host.session_of_lobby(lobby).await;
            host.log_to(session.as_deref(), "chat", format!("{bot} says: {text}"));
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
            let lobby = input.get("lobby_id").and_then(Value::as_str).unwrap_or_default();
            let session = host.session_of_lobby(lobby).await;
            host.log_to(session.as_deref(), "info", format!("game result {input}"));
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
fn dispatch_later(app: Shared, instance_id: String, action_id: String) {
    tokio::spawn(async move {
        if let Err(error) = app.host.dispatch(&instance_id, &action_id).await {
            app.host.log("error", error);
        }
    });
}

#[derive(Deserialize)]
struct EventsQuery {
    token: Option<String>,
}

async fn events(State(app): State<Shared>, Query(query): Query<EventsQuery>, upgrade: WebSocketUpgrade) -> Response {
    match bot_for(&app, query.token).await {
        Some(bot) => upgrade.on_upgrade(move |socket| bot_socket(app, bot, socket)).into_response(),
        None => (StatusCode::UNAUTHORIZED, "Unknown sandbox token").into_response(),
    }
}

async fn bot_socket(app: Shared, bot: String, socket: WebSocket) {
    let (mut write, mut read) = socket.split();
    let (sender, mut outgoing) = mpsc::unbounded_channel::<String>();
    app.host.connect_bot(&bot, sender).await;
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
    app.host.log("warn", format!("bot {bot} disconnected from the events socket"));
}

async fn page(State(app): State<Shared>) -> Html<String> {
    match &app.public {
        None => Html(include_str!("page.html").to_string()),
        Some(_) => Html(include_str!("sandbox.html").to_string()),
    }
}

#[derive(Deserialize, Default)]
struct SessionQuery {
    session: Option<String>,
}

/// Which visitor is asking. Locally everyone shares one session; in the
/// sandbox each browser makes up its own id.
fn session_of(app: &App, session: Option<String>) -> Result<String, String> {
    match (&app.public, session) {
        (None, session) => Ok(session.unwrap_or_else(|| LOCAL_SESSION.into())),
        (Some(_), Some(session))
            if (8..=64).contains(&session.len())
                && session.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
        {
            Ok(session)
        }
        (Some(_), _) => Err("A session id of 8 to 64 letters, digits, dashes or underscores is required.".into()),
    }
}

fn bad(error: String) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": error }))).into_response()
}

async fn dev_state(State(app): State<Shared>, Query(query): Query<SessionQuery>) -> Response {
    let session = match session_of(&app, query.session) {
        Ok(session) => session,
        Err(error) => return bad(error),
    };
    app.host.touch(&session).await;
    let (activities, instances, lobby_id) = app.host.snapshot(&session).await;
    let mut displays = HashMap::new();
    let mut definitions = HashMap::new();
    for activity in &activities {
        if let Some(id) = activity.get("workflow_id").and_then(Value::as_str) {
            displays.insert(id.to_string(), app.host.activity_display(id).await);
            definitions.insert(id.to_string(), app.host.definition(id).await);
        }
    }
    Json(json!({
        "activities": activities,
        "displays": displays,
        "definitions": definitions,
        "instances": instances,
        "lobby_id": lobby_id,
        "session": session,
    }))
    .into_response()
}

async fn dev_stream(State(app): State<Shared>, Query(query): Query<SessionQuery>) -> Response {
    let session = match session_of(&app, query.session) {
        Ok(session) => session,
        Err(error) => return bad(error),
    };
    let public = app.public.is_some();
    let stream: std::pin::Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(
        BroadcastStream::new(app.host.page.subscribe())
            .filter_map(move |event| {
                let session = session.clone();
                async move {
                    let event = event.ok()?;
                    let visible = match &event {
                        PageEvent::Instance { instance } => instance.session == session,
                        PageEvent::Log { session: Some(s), .. } => *s == session,
                        PageEvent::Log { session: None, .. } => !public,
                        PageEvent::Activities { .. } => true,
                    };
                    visible.then_some(event)
                }
            })
            .map(|event| Ok(Event::default().json_data(event).unwrap_or_default())),
    );
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

#[derive(Deserialize)]
struct StartGame {
    #[serde(default)]
    session: Option<String>,
    workflow_id: String,
    players: usize,
}

async fn dev_start(State(app): State<Shared>, Json(args): Json<StartGame>) -> Response {
    let session = match session_of(&app, args.session) {
        Ok(session) => session,
        Err(error) => return bad(error),
    };
    if app.public.is_some() && app.host.session_count().await >= MAX_SESSIONS {
        return bad("The sandbox is full right now. Try again in a few minutes.".into());
    }
    app.host.touch(&session).await;
    app.host.reset(&session).await;
    match app.host.start_players(&session, &args.workflow_id, args.players).await {
        Ok(instances) => Json(json!({ "ok": true, "instances": instances })).into_response(),
        Err(error) => {
            app.host.log_to(Some(&session), "error", error.clone());
            Json(json!({ "ok": false, "error": error })).into_response()
        }
    }
}

#[derive(Deserialize)]
struct Act {
    #[serde(default)]
    session: Option<String>,
    instance_id: String,
    action: String,
    inputs: HashMap<String, Value>,
}

async fn dev_act(State(app): State<Shared>, Json(args): Json<Act>) -> Response {
    let session = match session_of(&app, args.session) {
        Ok(session) => session,
        Err(error) => return bad(error),
    };
    if app.host.session_of(&args.instance_id).await.as_deref() != Some(session.as_str()) {
        return bad("That instance is not in your lobby.".into());
    }
    app.host.touch(&session).await;
    match app.host.act(&args.instance_id, &args.action, args.inputs).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(error) => {
            app.host.log_to(Some(&session), "error", error.clone());
            Json(json!({ "ok": false, "error": error })).into_response()
        }
    }
}

async fn dev_workflows(State(app): State<Shared>) -> Json<Value> {
    Json(json!({ "workflows": app.host.workflows().await }))
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

async fn dev_run(State(app): State<Shared>, Json(args): Json<Run>) -> Json<Value> {
    app.host.log("info", format!("playground ran {} on {}", args.action_id, args.workflow_id));
    match app.host.run_for_playground(&args.workflow_id, &args.action_id, args.responses, args.context).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })),
        Err(error) => {
            app.host.log("error", error.clone());
            Json(json!({ "ok": false, "error": error }))
        }
    }
}

async fn token_page(State(app): State<Shared>) -> Response {
    match &app.public {
        Some(public) => {
            Html(include_str!("token.html").replace("{{TAWKS_APP}}", &public.tawks_app)).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Local devhost takes any BOT_TOKEN; no sandbox token needed.").into_response(),
    }
}

struct Account {
    id: String,
    username: String,
}

/// Asks tawks who a signed-in person is, from the access token they brought.
async fn account(public: &Public, headers: &HeaderMap) -> Result<Account, String> {
    let token = bearer(headers).ok_or("Sign in with tawks first.")?;
    let body: Value = public
        .http
        .post(format!("{}/auth_whoami", public.tawks_rspc))
        .bearer_auth(token)
        .header("content-type", "application/json")
        .body("null")
        .send()
        .await
        .map_err(|_| "Could not reach tawks to check your sign-in.".to_string())?
        .json()
        .await
        .map_err(|_| "tawks did not recognise your sign-in.".to_string())?;
    let user = body.pointer("/result/data/user").ok_or("Your tawks sign-in has expired. Sign in again.")?;
    if user.get("is_bot").and_then(Value::as_bool) == Some(true) {
        return Err("Sandbox bots are made by people, not bots.".into());
    }
    let id = user.get("id").and_then(Value::as_str).ok_or("tawks did not say who you are.")?;
    let username = user.get("username").and_then(Value::as_str).unwrap_or_default();
    Ok(Account { id: id.to_string(), username: username.to_string() })
}

#[derive(Deserialize)]
struct NewToken {
    bot: String,
}

async fn tokens_list(State(app): State<Shared>, headers: HeaderMap) -> Response {
    let Some(public) = &app.public else { return StatusCode::NOT_FOUND.into_response() };
    match account(public, &headers).await {
        Ok(who) => Json(json!({ "ok": true, "username": who.username, "bots": public.tokens.bots_of(&who.id).await })).into_response(),
        Err(error) => bad(error),
    }
}

async fn tokens_create(State(app): State<Shared>, headers: HeaderMap, Json(args): Json<NewToken>) -> Response {
    let Some(public) = &app.public else { return StatusCode::NOT_FOUND.into_response() };
    let who = match account(public, &headers).await {
        Ok(who) => who,
        Err(error) => return bad(error),
    };
    match public.tokens.create(&who.id, &who.username, &args.bot).await {
        Ok(token) => {
            app.host.log("info", format!("{} made the sandbox bot {}", who.username, args.bot));
            Json(json!({ "ok": true, "bot": args.bot, "token": token })).into_response()
        }
        Err(error) => bad(error),
    }
}

async fn tokens_delete(State(app): State<Shared>, headers: HeaderMap, Json(args): Json<NewToken>) -> Response {
    let Some(public) = &app.public else { return StatusCode::NOT_FOUND.into_response() };
    let who = match account(public, &headers).await {
        Ok(who) => who,
        Err(error) => return bad(error),
    };
    match public.tokens.delete(&who.id, &args.bot).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(error) => bad(error),
    }
}

fn for_docs_site(path: &str) -> bool {
    path.starts_with("/dev/") || path.starts_with("/rspc/")
}

fn cors(mut response: Response, origin: &str) -> Response {
    let headers = response.headers_mut();
    if let Ok(origin) = HeaderValue::from_str(origin) {
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    headers.insert(ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST"));
    headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("authorization, content-type"));
    headers.insert("access-control-allow-private-network", HeaderValue::from_static("true"));
    headers.insert(VARY, HeaderValue::from_static("origin"));
    response
}

/// Locally only this machine may talk to devhost: checking Host stops DNS
/// rebinding, and checking Origin stops other sites in the developer's
/// browser from driving it. The sandbox answers on its own host name. In
/// both, the docs site may reach the page endpoints and the API, and no
/// other site may.
async fn guard(State(app): State<Shared>, request: Request, next: Next) -> Result<Response, StatusCode> {
    let own: Vec<String> = match &app.public {
        Some(public) => vec![public.host.clone()],
        None => vec![format!("localhost:{}", app.port), format!("127.0.0.1:{}", app.port)],
    };
    let host_ok = request
        .headers()
        .get(HOST)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| own.iter().any(|o| o == h));
    if !host_ok {
        return Err(StatusCode::FORBIDDEN);
    }
    let Some(origin) = request.headers().get(ORIGIN).and_then(|o| o.to_str().ok()).map(String::from) else {
        return Ok(next.run(request).await);
    };
    let scheme = if app.public.is_some() { "https" } else { "http" };
    if own.iter().any(|o| origin == format!("{scheme}://{o}")) {
        return Ok(next.run(request).await);
    }
    if !(for_docs_site(request.uri().path()) && app.docs_origins.iter().any(|o| *o == origin)) {
        return Err(StatusCode::FORBIDDEN);
    }
    let response = if request.method() == Method::OPTIONS {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(request).await
    };
    Ok(cors(response, &origin))
}

fn env_list(name: &str) -> Vec<String> {
    std::env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(|o| o.trim().trim_end_matches('/').to_string())
        .filter(|o| !o.is_empty())
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .init();

    let port: u16 = std::env::var("DEVHOST_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787);
    let mut docs_origins: Vec<String> = DOCS_ORIGINS.iter().map(|o| o.to_string()).collect();
    docs_origins.extend(env_list("DEVHOST_PLAYGROUND_ORIGINS"));

    let public = match std::env::var("DEVHOST_PUBLIC_HOST").ok().filter(|h| !h.is_empty()) {
        None => None,
        Some(host) => {
            let data = PathBuf::from(std::env::var("DEVHOST_DATA_DIR").unwrap_or_else(|_| "/data".into()));
            std::fs::create_dir_all(&data)?;
            let tawks_app = std::env::var("TAWKS_APP_URL").unwrap_or_else(|_| "https://tawks.app".into());
            let tawks_rspc = std::env::var("TAWKS_RSPC_URL").unwrap_or_else(|_| "https://rspc.tawks.app/rspc".into());
            Some(Public {
                host,
                tawks_app: tawks_app.trim_end_matches('/').to_string(),
                tawks_rspc: tawks_rspc.trim_end_matches('/').to_string(),
                tokens: Tokens::load(data.join("tokens.json"))?,
                http: reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?,
            })
        }
    };
    let bind: IpAddr = if public.is_some() { Ipv4Addr::UNSPECIFIED.into() } else { Ipv4Addr::LOCALHOST.into() };
    let public_host = public.as_ref().map(|p| p.host.clone());

    let app: Shared = Arc::new(App { host: Host::new(), public, port, docs_origins });
    let scheduler = app.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(250));
        loop {
            tick.tick().await;
            scheduler.host.run_due_jobs().await;
        }
    });
    if app.public.is_some() {
        let janitor = app.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            loop {
                tick.tick().await;
                let gone = janitor.host.expire(SESSION_IDLE).await;
                if gone > 0 {
                    tracing::info!(gone, "expired idle sandbox sessions");
                }
            }
        });
    }

    let router = Router::new()
        .route("/", get(page))
        .route("/rspc/:procedure", post(rspc))
        .route("/events", get(events))
        .route("/dev/state", get(dev_state))
        .route("/dev/stream", get(dev_stream))
        .route("/dev/start", post(dev_start))
        .route("/dev/act", post(dev_act))
        .route("/dev/workflows", get(dev_workflows))
        .route("/dev/run", post(dev_run))
        .route("/sandbox/token", get(token_page))
        .route("/sandbox/tokens", get(tokens_list).post(tokens_create))
        .route("/sandbox/tokens/delete", post(tokens_delete))
        .layer(middleware::from_fn_with_state(app.clone(), guard))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind((bind, port)).await?;
    match public_host {
        Some(host) => println!("tawks sandbox for https://{host} on port {port}"),
        None => {
            println!("tawks devhost on http://localhost:{port}");
            println!("  RSPC_URL=http://localhost:{port}/rspc");
            println!("  EVENTS_WS_URL=ws://localhost:{port}/events");
            println!("  BOT_TOKEN=<any name, different for each bot>");
        }
    }
    axum::serve(listener, router).await?;
    Ok(())
}
