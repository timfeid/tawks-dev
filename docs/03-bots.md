# Writing a bot

A tawks bot is a program you run yourself. It logs in with a bot token, keeps a WebSocket open to the events server, and calls the tawks API over HTTPS. Most bots do three things:

1. Register **server actions**: named handlers that tawks calls when a user does something on one of your pages.
2. Register **workflow definitions**: the pages users see, built from nodes, displays and actions. [Workflows](04-workflows.md) explains the model.
3. React to **events**: chat messages, typing, reactions and so on.

A channel shows your page once the channel's workflow is set to one of your registered workflows.

This page covers the Rust SDK (crate `tawks-bot`, imported as `tawks_bot`) first. The JavaScript SDK, `@tawks/bot`, does the same work and has its own section at the end. If you have not set up a project and a token yet, start with [Your first bot](02-first-bot.md).

## What you need

### A bot account and token

Bot accounts are issued by the tawks team for now. When yours is created you get:

- the bot's **user id**, such as `hellobot`. Your workflow and server action ids are built from it.
- the **token**. Keep it secret and load it from the environment.
- membership in the niche where the bot will run, if it will post in that niche's chats.

### Endpoints

| Variable | Value |
| --- | --- |
| `RSPC_URL` | `https://rspc.tawks.app/rspc` |
| `EVENTS_WS_URL` | `wss://events.tawks.app/` |
| `BOT_TOKEN` | Your bot token |

### Crates

Add `tawks-bot` as described in [Your first bot](02-first-bot.md). The example below also uses these:

```toml
[dependencies]
anyhow = "1"
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time", "signal"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

`tawks_bot` re-exports the workflow types and builders as `tawks_bot::workflow`, so you do not need a separate dependency for them.

## A complete bot

This bot registers a page with a greeting and a "Wave" button that counts clicks, and answers `!ping` in chat with `pong`. Its shape is the one every tawks bot follows: build the bot, register actions, register workflows, run the event loop, and reconnect with backoff when the session ends.

```rust
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde_json::{Value, json};
use tawks_bot::types::{BotOptions, Event, MessageEvent, UserResource};
use tawks_bot::workflow::CreateWorkflowDefinition;
use tawks_bot::workflow::builder::{
    WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use tawks_bot::workflow::server_action::ServerActionResult;
use tawks_bot::workflow::service::WorkflowResourceWithContext;
use tawks_bot::{ServerActionHandler, TawksBot, TawksSdk, shutdown_token, with_jitter};
use tokio::{select, time::sleep};
use tracing::{error, info, warn};

fn handler<F, Fut>(f: F) -> ServerActionHandler
where
    F: Fn(WorkflowResourceWithContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<ServerActionResult>> + Send + 'static,
{
    Arc::new(Box::new(move |ctx| Box::pin(f(ctx))))
}

async fn load(ctx: WorkflowResourceWithContext) -> Result<ServerActionResult> {
    let name = ctx
        .ctx_required_as::<UserResource>("$user")
        .map(|u| if u.display_name.trim().is_empty() { u.username } else { u.display_name })
        .unwrap_or_else(|_| "there".to_string());
    Ok(ServerActionResult::UpdateResponses(HashMap::from([
        ("greeting".to_string(), json!(format!("Hello, {name}!"))),
        ("wave_label".to_string(), json!("No waves yet")),
    ])))
}

async fn wave(ctx: WorkflowResourceWithContext) -> Result<ServerActionResult> {
    let waves = ctx
        .workflow
        .responses
        .get("waves")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + 1;
    Ok(ServerActionResult::UpdateResponses(HashMap::from([
        ("waves".to_string(), json!(waves)),
        ("wave_label".to_string(), json!(format!("Waved {waves} times"))),
    ])))
}

struct Actions {
    load: String,
    wave: String,
}

async fn register_actions(bot: &TawksBot) -> Result<Actions> {
    Ok(Actions {
        load: bot.register_server_action("hello.load", handler(load)).await?,
        wave: bot.register_server_action("hello.wave", handler(wave)).await?,
    })
}

fn hello_workflow(actions: &Actions) -> CreateWorkflowDefinition {
    let text = |class: &str, s: &str| WorkflowDisplayBuilder::new(class).text(s).build();
    let wave_button = WorkflowDisplayBuilder::new("")
        .button(vec![text("", "Wave")], None, "wave", "default", "sm")
        .build();

    let main = WorkflowNodeBuilder::new("Hello")
        .layout("w-full")
        .visual(
            WorkflowVisualBuilder::new("hello_root")
                .block(
                    "mx-auto flex w-full max-w-xl flex-col gap-3 p-4",
                    vec![
                        text("text-2xl font-bold", "{greeting}"),
                        text("text-sm text-zinc-400", "{wave_label}"),
                        wave_button,
                    ],
                )
                .build(),
        )
        .build();

    WorkflowDefinitionBuilder::new("hello", "Hello")
        .description("A minimal tawks bot page.")
        .starting_server_action(&actions.load)
        .response("greeting", json!("Loading..."))
        .add_action(
            WorkflowActionBuilder::new("wave")
                .run_server_action()
                .target(&actions.wave)
                .progress_node(false)
                .build(),
        )
        .add_node("main", main)
        .build()
}

async fn on_event(event: Event, sdk: Arc<TawksSdk>) {
    match event {
        Event::Connect => info!("connected"),
        Event::Disconnect => warn!("disconnected"),
        Event::Error { error } => error!(%error, "event stream error"),
        Event::Message { message: MessageEvent::ChatMessage { message, chat_id } } => {
            if message.user.is_bot || message.content.trim() != "!ping" {
                return;
            }
            let reply = json!({
                "chat_id": chat_id,
                "contents": { "text": "pong", "attachments": null },
            });
            if let Err(e) = sdk
                .web_client
                .mutate::<_, Value>("chat_create_message", &reply)
                .await
            {
                warn!(error = %e, "reply failed");
            }
        }
        Event::Message { .. } => {}
    }
}

fn env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("{key} is required"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    let opts = BotOptions {
        rspc_url: env("RSPC_URL")?,
        events_ws_url: env("EVENTS_WS_URL")?,
        token: env("BOT_TOKEN")?,
        token_type: Some("Bot".to_string()),
    };

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(30);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            let actions = register_actions(&bot).await?;
            let workflow_id = bot.register_workflow(hello_workflow(&actions)).await?;
            info!(%workflow_id, "set a channel's workflow_id to this value");

            let sdk = bot.sdk.clone();
            bot.start(move |event, _| on_event(event, sdk.clone())).await
        };

        select! {
            res = session => match res {
                Ok(()) => warn!("session ended; reconnecting"),
                Err(e) => error!(error = %e, "session failed; reconnecting"),
            },
            _ = shutdown.cancelled() => return Ok(()),
        }

        select! {
            _ = sleep(with_jitter(backoff)) => {},
            _ = shutdown.cancelled() => return Ok(()),
        }
        backoff = (backoff * 2).min(backoff_max);
    }
}
```

Run it with:

```sh
RSPC_URL=https://rspc.tawks.app/rspc \
EVENTS_WS_URL=wss://events.tawks.app/ \
BOT_TOKEN=your-token \
cargo run
```

The log line `set a channel's workflow_id to this value` prints `wf-hello-<your bot's user id>`. The rest of this page explains each part.

## Authentication

`TawksBot::new(BotOptions)` sets up an HTTP client that sends `Authorization: <token_type> <token>` on every call:

```rust
pub struct BotOptions {
    pub rspc_url: String,
    pub events_ws_url: String,
    pub token: String,
    pub token_type: Option<String>,
}
```

Set `token_type` to `Some("Bot".into())`. The API reads `Bot <token>` as a bot token. If you leave `token_type` as `None`, the SDK sends `Bearer`, which the API treats as a user session, and your calls fail as unauthorized.

You do not need to handle the WebSocket login yourself. When `start` connects, the SDK exchanges your bot token for a short-lived session token and uses it to open the events socket. Each call to `start` fetches a fresh one, so reconnects keep working.

## Startup and reconnecting

`TawksBot::start` runs until the socket closes, errors, or the SDK decides your registrations need refreshing (see [Keeping registrations alive](#keeping-registrations-alive)). It does not reconnect on its own, so wrap the whole session in a loop as the example does:

1. Create a new `TawksBot`.
2. Register your server actions, then your workflows.
3. Call `start`.
4. When `start` returns, sleep with backoff and go around again.

Create a fresh `TawksBot` for every pass. `start` stops for good once the process receives Ctrl-C, so reusing an old instance after that returns immediately.

Two helpers from the crate root support the loop:

- `shutdown_token()` returns a `CancellationToken` that is cancelled on Ctrl-C, SIGINT or SIGTERM.
- `with_jitter(base)` adds or removes up to 10% of `base`, so many bots do not reconnect in lockstep.

It also helps to reset the backoff to its starting value once a session has stayed up for 30 seconds or so. The example leaves that out for brevity.

## Server actions

A server action is a named async handler. tawks calls it when a user presses a button, when a workflow starts, or when a scheduled job fires.

### Registering

```rust
pub async fn register_server_action(
    &self,
    action_id: impl Into<String>,
    handler: ServerActionHandler,
) -> anyhow::Result<String>
```

The id you pass is local to your bot. The API stores it as `sa-<action_id>-<bot user id>` and returns that canonical id. Use the returned string anywhere a workflow refers to the action, as the example does with `Actions { load, wave }`.

`ServerActionHandler` is a boxed closure:

```rust
pub type ServerActionHandler = Arc<
    Box<
        dyn Fn(WorkflowResourceWithContext)
                -> Pin<Box<dyn Future<Output = anyhow::Result<ServerActionResult>> + Send>>
            + Send
            + Sync,
    >,
>;
```

Writing that out for every action is noisy, so define a small adapter like `handler` in the example. If your handlers share state, capture an `Arc` of it in the adapter.

### What a handler receives

`WorkflowResourceWithContext` has two fields:

- `workflow: WorkflowResource`: the instance. You will mostly use `responses` (a `HashMap<String, Value>` holding the page's state), `user_id`, `instance_id` and `current_node_id`.
- `context: HashMap<String, Value>`: values tawks attaches. It holds `$user` (the acting user as a `UserResource`) and `$anonymous` (true for guests).

When a channel starts your workflow, these responses are filled in for you: `channel_id`, `niche_id`, `niche_slug`, `channel_slug`, `feed_url` and `page_url_prefix`, plus `permissions` for users who can manage pages in that niche.

Button inputs land in `responses` too. A button with inputs `{"view": "board"}` makes `ctx.workflow.responses["view"]` equal `"board"` when your action runs.

Helpers on `WorkflowResourceWithContext`:

| Method | Reads |
| --- | --- |
| `input_str(key) -> Option<String>` | responses first, then context |
| `input_required_str(key)` | same, errors if missing |
| `get_input_as::<T>(key)` | responses only, deserialized |
| `ctx_required_as::<T>(key)` | responses first, then context, deserialized |
| `ctx_get(key) -> Option<&Value>` | context only |
| `ctx_get_bool`, `ctx_get_u64`, `ctx_get_i64`, `ctx_get_array` | context only, with defaults |

Keys can be dotted paths. Because `ctx_required_as` looks in responses before context, never name one of your own responses `$user`.

### What a handler returns

These `ServerActionResult` variants cover almost every bot:

| Variant | Effect |
| --- | --- |
| `UpdateResponses(HashMap<String, Value>)` | Merges the map into the instance's responses and re-renders. |
| `UpdateContext(HashMap<String, Value>)` | Merges into context. |
| `UpdateContextAndResponses { context, responses }` | Both. |
| `ScheduleJobs { context, inputs, jobs, reconcile_node }` | Merges `inputs` into responses and schedules jobs that run actions later. |
| `CompleteWorkflow { responses, close_active_modal, message }` | Ends the instance. |
| `Error(String)` | Shows the message to the user. |
| `None` | Does nothing. |

Merges are shallow: a top-level key you send replaces the old value whole. If you include a numeric `$rev` and it is lower than the one the instance already holds, tawks drops your update as stale. That is useful when handlers can finish out of order.

If your handler returns `Err`, the SDK replies with `ServerActionResult::Error(err.to_string())`. The user sees that text, so keep error messages short and free of internal detail.

tawks waits up to 60 seconds for a reply. Each request runs on its own task, so a slow handler does not hold up other users.

## Workflow definitions

A workflow definition describes a page: its starting state, its nodes (views) and its actions. [Workflows](04-workflows.md) covers the model in depth; this section covers registration.

```rust
pub async fn register_workflow(
    &self,
    workflow_definition: CreateWorkflowDefinition,
) -> anyhow::Result<String>
```

The returned id is `wf-<definition id>-<bot user id>`. For the example, with a bot whose user id is `hellobot`, that is `wf-hello-hellobot`.

Registration checks that every `run_server_action` target is one of your registered server actions and that every node target names a node in the definition, so register your actions first. Registering an identical definition again is cheap. If it changed, tawks stores it as a new version.

A quick tour of the builders, all in `tawks_bot::workflow::builder`:

- `WorkflowDefinitionBuilder::new(id, name)` starts a definition. `.starting_server_action(id)` names the action that runs when an instance starts; use it to fill in the page. `.response(key, value)` sets an initial response.
- `WorkflowNodeBuilder::new(title)` is a view. It holds one or more visuals, an optional `condition` and an optional `parent_id`.
- `WorkflowVisualBuilder::new(id)` wraps displays in a region. `.block(class, displays)` is the usual choice.
- `WorkflowDisplayBuilder::new(class)` makes one element: `.text(s)`, `.div(children)`, `.button(..)`, `.list(..)` and many more, listed in the [Display reference](05-displays.md).
- `WorkflowActionBuilder::new(id)` is something a button can trigger. `.run_server_action().target(server_action_id)` calls your bot.

A button's `action_id` is the workflow action's id (`"wave"`), not the server action id. The workflow action's `target` is the server action id.

## Showing your workflow in a channel

A channel shows the workflow named in its settings. For a bot workflow, that is the id `register_workflow` returned:

```
wf-<definition id>-<bot user id>
```

The owner part is the bot's **user id**, not its username or display name.

There are two ways to set it:

1. **Channel settings.** The niche owner, or anyone allowed to manage channels in that niche, opens the channel's settings and picks your workflow from the list. It appears there once your bot has registered it.
2. **The API.** Call `channel_update` with `{ "channel_id": ..., "workflow_id": "wf-hello-hellobot" }`. The caller needs the same channel management rights.

A channel also has a lobby workflow setting, which works the same way for the workflow that starts when someone opens a lobby there. See [Building games](06-games.md).

If your bot is offline when a user opens the channel, the user sees "This workflow is unavailable right now because no connected bot can handle it." Requests that were sent to your bot but never answered are sent again once the bot registers those actions again.

## Keeping registrations alive

Server action registrations are leases that last 45 seconds. While `start` is running, the SDK renews them every 15 seconds. On each renewal it also checks that tawks still has all of your actions and workflows. If anything is missing, for example after a tawks restart, it ends the session so your loop reconnects and registers everything again. That is why registration belongs inside the reconnect loop rather than before it.

Run one process per bot token. A second process with the same token receives the same server action requests, and both would answer.

## Events

### The callback

```rust
pub async fn start<F, Fut>(&self, on_event: F) -> anyhow::Result<()>
where
    F: FnMut(Event, &TawksSdk) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
```

`Event` has four variants: `Connect`, `Disconnect`, `Error { error }` and `Message { message: MessageEvent }`.

To make API calls from the callback, clone `bot.sdk` (an `Arc<TawksSdk>`) into the closure, as the example does. The `&TawksSdk` argument cannot be moved into the returned future.

The callback runs inline with the event reader. Spawn a task for anything slow so the bot keeps reading events.

The SDK sends a heartbeat every 25 seconds, which is what shows your bot as online in member lists.

### Message events

| `MessageEvent` variant | When |
| --- | --- |
| `ChatMessage { message, chat_id }` | A message was posted in a chat your bot can see. `message` has `id`, `user`, `content`, `chat_id` and `parent_message_id`. |
| `WorkflowRequestServerAction { id, workflow, action_id }` | Only for actions you have no handler for. |
| `Error { message }` | The events server reported a problem. |
| `Other` | Everything else, such as typing indicators and reactions. |

Server action requests for actions you registered never reach the callback: the SDK runs your handler and sends the reply. Full `WorkflowCreated` and `WorkflowUpdated` events are skipped as well.

Most bots that serve pages listen for nothing here and only log `Connect`, `Disconnect` and `Error`. A command-style bot needs `ChatMessage`. Check `message.user.is_bot` so you never answer yourself or another bot, and check `chat_id` so you only act in the chats you serve.

## Calling the API

`bot.sdk.web_client` is a small rspc client:

```rust
pub async fn mutate<TIn: Serialize + Debug, TOut: DeserializeOwned>(&self, path: &str, input: &TIn) -> Result<TOut>
pub async fn query<TIn: Serialize + Debug, TOut: DeserializeOwned>(&self, path: &str, input: &TIn) -> Result<TOut>
```

`path` is the procedure name, such as `chat_create_message`. An error from the API becomes an `Err` carrying the server's message.

`TawksBot` wraps a few procedures directly:

| Method | Procedure |
| --- | --- |
| `start_workflow_on_behalf(StartWorkflowOnBehalfArgs)` | `workflow_start_on_behalf` |
| `start_workflow_from_instance(StartWorkflowFromInstanceArgs)` | `workflow_start_from_instance` |
| `workflow_process_action(ProcessWorkflowActionArgs)` | `workflow_process_action` |
| `workflow_restart(instance_id, input)` | `workflow_restart` |
| `speak(lobby_id, text)` | `lobby_speak` |

### Sending a chat message

Call `chat_create_message` with a chat id and contents:

```rust
let reply = json!({
    "chat_id": "hello-chat",
    "contents": { "text": "Puck drop in 10 minutes", "attachments": null },
});
let _: serde_json::Value = bot.sdk.web_client.mutate("chat_create_message", &reply).await?;
```

For a chat channel, the chat id is the channel id. Group chats and DMs have their own ids, which arrive as `chat_id` on chat events. Posting in a friend group's chat requires your bot to be a member of that group. Mentions, push notifications and link previews work the same as for a person's message.

### Pushing live updates to open pages

To change the page for everyone who has it open, without waiting for a click, call `workflow_broadcast_updates`:

```rust
let args = json!({
    "workflow_definition_id": "wf-hello-hellobot",
    "filter": { "user_id": null, "response_equals": null, "include_completed": false },
    "updates": { "wave_label": "Someone waved!" },
});
let _: serde_json::Value = bot.sdk.web_client.mutate("workflow_broadcast_updates", &args).await?;
```

`filter.user_id` limits it to one user's instances. `filter.response_equals` takes a `[key, value]` pair and limits it to instances whose response matches. You can only broadcast to workflows your bot owns. Live scores and countdowns are the usual reason to use this.

## Notifications

`tawks_bot::notify::Notifier` sends push and in-app notifications for one channel. You declare notification kinds, users opt in to each kind from the channel's bell menu, and you send to the users who opted in.

```rust
use tawks_bot::notify::{KindInput, Notifier};

let notifier = Notifier::new(
    &rspc_url,
    &format!("Bot {token}"),
    "hello-channel",
    "/niche/hello/hello-channel",
    vec![KindInput {
        key: "hellobot.daily".into(),
        label: "Daily hello".into(),
        description: Some("A greeting every morning".into()),
        kind: "reminder".into(),
        default_state: "off".into(),
        sort_order: Some(0),
    }],
)?;

notifier.register_kinds().await?;

let users = notifier.subscribers("hellobot.daily").await?;
notifier
    .send(
        "hellobot.daily",
        "hellobot.daily:2026-10-22",
        &users,
        "Good morning",
        Some("Come say hello"),
        Some(notifier.channel_url()),
    )
    .await?;
```

Things to know:

- `Notifier::new` takes the full `Authorization` header value (`Bot <token>`), not the bare token.
- `kind` is `"event"` or `"reminder"`.
- Every kind starts off for every user. Users turn it on themselves.
- Kind keys share one namespace across all bots, so prefix them with your bot's name.
- Always send to the list from `subscribers(event_key)`, so only people who opted in hear from you.
- `dedupe_key` is unique per user. Sending the same key to the same user twice delivers once, so put the date or an event id in it.
- `send_with_icon` takes one more argument: a push icon URL, or `Some("none")` for no icon.

Build the `Notifier` once at startup, outside the reconnect loop, and run your send schedule on its own task.

## Share cards

`tawks_bot::share::ShareCard` builds a result card for daily games, the kind people paste into group chats after they finish. The card's data travels inside the link, so nothing needs storing:

```rust
use tawks_bot::share::ShareCard;

let card = ShareCard {
    game: "hello".into(),
    name: "Hello".into(),
    day: "2026-10-22".into(),
    headline: "3 waves".into(),
    detail: String::new(),
    rows: vec!["ggyk".into()],
    lines: Vec::new(),
    play: "/niche/hello/hello-channel".into(),
};
let path = card.path();
let view = card.view();
```

- `path()` returns `/share/<game>/<payload>`. tawks renders a preview page and image for it.
- `view()` returns `{ path, title, text }`. Put it under a response key and show it with `WorkflowDisplayBuilder::new("").share_result("hello_share", "Share today's result")`.
- `rows` are strings of color letters from `tawks_bot::share::SQUARES` (`"gybporwk"`: green, yellow, blue, purple, orange, red, white, and `k` for empty).
- `lines` are short text rows shown under the headline.
- `play` must be an in-app path starting with `/niche/`.

## Scores

Users have a tawks score that shows on their profile. Score is awarded by first-party games for now. Your bot can keep its own points, streaks and leaderboards in responses or in your own storage and show them with the displays in the [Display reference](05-displays.md).

## The JavaScript SDK

`@tawks/bot` is the JavaScript and TypeScript SDK, in the [tawks-dev repository](https://github.com/timfeid/tawks-dev) under `sdk/bot-js`. It does what the Rust SDK does: registers your workflows and server actions, runs your handlers when people press buttons, keeps the events socket open with heartbeats, renews server action leases, and registers everything again after a dropped connection or a tawks restart. It has no dependencies and runs on Node 22 or later, Bun and Deno.

It is not on npm yet, so depend on it by git. The `examples/hello-js` folder in the repository is a working bot to copy.

### A complete JavaScript bot

```js
import { TawksBot, result, ui } from '@tawks/bot';

const bot = new TawksBot({
  rspcUrl: process.env.RSPC_URL ?? 'https://rspc.tawks.app/rspc',
  eventsUrl: process.env.EVENTS_WS_URL ?? 'wss://events.tawks.app/',
  token: process.env.BOT_TOKEN,
});

const wave = await bot.serverAction('hello.wave', (ctx) => {
  const name = ctx.input('name')?.trim() || ctx.user.display_name;
  const waves = (ctx.number('waves') ?? 0) + 1;
  return result.responses({ name, waves, reply: `The bot has waved back ${waves} times.` });
});

const hello = ui.workflow(
  'hello',
  'Hello',
  {
    home: ui.node('Hello', [
      ui.page([
        ui.text('Hello, {name}!', 'text-2xl font-bold'),
        ui.form('wave', [ui.textInput('name', 'Your name'), ui.submit('Wave', 'wave')], { cls: 'flex gap-2' }),
        ui.text('{reply}', 'text-muted-foreground'),
      ]),
    ]),
  },
  { responses: { name: 'there', reply: '', waves: 0 }, actions: [ui.actions.run('wave', wave)] },
);

console.log('registered', await bot.workflow(hello));

bot.onChat(async (message) => {
  if (message.content.trim() === '!ping') await bot.say(message.chat_id, 'pong');
});

await bot.run();
```

Run it against devhost to try it without tawks, then open the page in the playground with the plug button:

```sh
RSPC_URL=http://localhost:8787/rspc EVENTS_WS_URL=ws://localhost:8787/events BOT_TOKEN=hellojs node index.js
```

### The pieces

| Call | What it does |
| --- | --- |
| `new TawksBot({ rspcUrl, eventsUrl, token })` | `tokenType` defaults to `Bot`. Pass `logger: null` to silence the SDK's own log lines. |
| `bot.serverAction(id, handler)` | Registers a server action and returns its full id, `sa-<id>-<your bot's user id>`, for workflow actions to target. |
| `bot.workflow(definition)` | Registers a workflow and returns its full id, which appears in channel settings. |
| `bot.activity({ workflow_id, name, client_url, ... })` | Registers a game activity. See [Game activities](07-tables.md). |
| `bot.run()` | Connects and stays connected until `bot.stop()`. Register before calling it. |
| `bot.onChat(listener)` | Chat messages from people. Messages from bots, yours included, are skipped. |
| `bot.say(chatId, text)` | Posts a chat message. For a chat channel, the chat id is the channel id. |
| `bot.on('connect' \| 'disconnect' \| 'error' \| 'message', listener)` | Connection events, and any event from tawks that is not a server action request for one of your handlers. |
| `bot.api.query(name, input)`, `bot.api.mutate(name, input)` | Any API procedure by name, as in the [API explorer](https://tawks-dev.vercel.app/api). |

A handler receives a `ServerActionContext` and returns a result. `ctx.input(key)` reads a response, including the inputs the page just sent, and `ctx.number(key)` also accepts numbers that arrive as strings from templates. `ctx.user` is the person using the page, and `ctx.anonymous` is true for a guest. Return `result.responses({...})`, `result.context({...})`, `result.complete(message)`, `result.error(message)` or `result.none()`, or any `ServerActionResult` as plain JSON. A handler that throws answers with its error message.

`ui` builds the displays most pages need: `text`, `div`, `image`, `badge`, `list`, `when`, `button`, `submit`, `form`, `textInput`, `numberInput`, `switchInput`, `page`, `node`, `workflow`, plus `ui.actions` and `ui.cond`. Each returns the plain JSON from the [Display reference](05-displays.md), so you can mix them with displays written by hand. In TypeScript every definition is checked against the types in `@tawks/workflow-types`.
