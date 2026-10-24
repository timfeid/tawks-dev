# Your first bot

This page takes you from nothing to a bot that serves a page in a channel you own. The page greets whoever opens it and counts clicks on a "Wave" button. You write and run the bot in your own repository, against the hosted tawks at https://tawks.app.

Read [What you can build on tawks](01-overview.md) first if words like niche, workflow or server action are new to you.

## 1. Get a bot account and a token

Making bots is by invitation. A niche's admins can give your role the **Create Bots** permission; once you have it, open **Settings**, then **Bots**:

1. Pick the niche the bot will live in, choose a username and a display name, and create it. The username becomes the bot's **user id**, and every workflow your bot registers is named after it, as `wf-<workflow name>-<bot user id>`.
2. Press **New token** and copy the token. It starts with `tbot_` and is shown only once; tawks keeps just a fingerprint of it. Treat it like a password and keep it out of your repository and out of chat. You can revoke a token there at any time, and it stops working at once.

Each person can own one bot at a time unless the tawks team raises the limit. If no niche has given you the permission yet, ask in the `dev-chat` channel of the Tawks Dev niche. You can also build and play your game locally with no account at all; see [Game activities](07-tables.md).

Check the token with one request:

```sh
curl -s -X POST https://rspc.tawks.app/rspc/auth_whoami \
  -H 'Authorization: Bot <your token>'
```

A working token returns your bot's user, with `"is_bot":true`:

```json
{"jsonrpc":"2.0","id":null,"result":{"type":"response","data":{"user":{"id":"<bot user id>","username":"<bot username>","display_name":"<bot name>","is_bot":true,...},"niches":[]}}}
```

The header must start with `Bot`. The same token sent as `Bearer <token>` comes back as `Unauthorized`.

## 2. Get a niche to test in

You need a niche you control, so you can attach your bot's page to a channel without affecting anyone else. Niches and channels cannot be created from the app yet, so ask for one in `dev-chat` at the same time as your bot account. Ask for:

- a niche, with your own account as its owner.
- at least one channel for your bot's page, ideally of type `tiles`. Keep your `chat` channel for chat.
- your bot added as a member, if you plan to have it post in the niche's chat.

Once you own the niche, you manage it yourself from its settings: roles, channel names and permissions, which workflow each channel shows, and invites for people who want to try your bot.

## 3. Add the SDK

The Rust SDK is the crate `tawks-bot`, in the [tawks-dev repository](https://github.com/timfeid/tawks-dev). It is not on crates.io yet, so depend on it by git. It uses Rust edition 2024, so you need Rust 1.85 or newer.

Start a new binary crate:

```sh
cargo new hello-bot
cd hello-bot
```

Then add the dependencies to `Cargo.toml`:

```toml
[dependencies]
tawks-bot = { git = "https://github.com/timfeid/tawks-dev" }
anyhow = "1"
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time", "signal"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

When the crate is published, that line becomes `tawks-bot = "<version>"`.

## 4. Configure your environment

The SDK takes its settings as a `BotOptions` value and reads nothing from the environment on its own. The convention every tawks bot follows is to read these four variables:

| Variable | Value |
| --- | --- |
| `RSPC_URL` | `https://rspc.tawks.app/rspc` |
| `EVENTS_WS_URL` | `wss://events.tawks.app/` |
| `BOT_TOKEN` | Your bot token |
| `BOT_TOKEN_TYPE` | `Bot` |

`BOT_TOKEN_TYPE` must be `Bot`. If you pass no token type, the SDK falls back to `Bearer`, which tawks treats as a user sign-in, and every call fails with `Unauthorized`.

Export them in the shell you run the bot from:

```sh
export RSPC_URL=https://rspc.tawks.app/rspc
export EVENTS_WS_URL=wss://events.tawks.app/
export BOT_TOKEN=<your token>
export BOT_TOKEN_TYPE=Bot
```

If you prefer a `.env` file, load it with a crate such as `dotenvy` and add the file to `.gitignore`.

## 5. Write the bot

Replace `src/main.rs` with this:

```rust
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde_json::{Value, json};
use tawks_bot::types::{BotOptions, Event, UserResource};
use tawks_bot::workflow::CreateWorkflowDefinition;
use tawks_bot::workflow::builder::{
    WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use tawks_bot::workflow::server_action::ServerActionResult;
use tawks_bot::workflow::service::WorkflowResourceWithContext;
use tawks_bot::{ServerActionHandler, TawksBot, shutdown_token, with_jitter};
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

async fn on_event(event: Event) {
    match event {
        Event::Connect => info!("connected"),
        Event::Disconnect => warn!("disconnected"),
        Event::Error { error } => error!(%error, "event stream error"),
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
        token_type: Some(std::env::var("BOT_TOKEN_TYPE").unwrap_or_else(|_| "Bot".to_string())),
    };

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(30);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            let actions = register_actions(&bot).await?;
            let workflow_id = bot.register_workflow(hello_workflow(&actions)).await?;
            info!(%workflow_id, "registered");
            bot.start(|event, _| on_event(event)).await
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

What each part does:

- `load` is the workflow's **starting server action**. Tawks calls it when someone opens the page, and it reads the viewer from the `$user` context key to fill in the `greeting` response.
- `wave` runs when the "Wave" button is clicked. It reads the current count from the instance's responses and returns the new count. Each viewer has their own instance, so each person has their own count.
- `{greeting}` and `{wave_label}` in the text displays are templates. The page shows the current value of each response and redraws when it changes.
- The `loop` matters. `start` returns whenever the connection drops or tawks asks the bot to register again, and a fresh session registers everything before it reconnects. Keep registration inside the loop in every bot you write.

[Writing a bot](03-bots.md) covers each of these in depth, and [Workflows](04-workflows.md) explains nodes, actions and templates.

## 6. Run it

```sh
cargo run
```

A healthy start looks like this:

```text
... INFO hello_bot: registered workflow_id=wf-hello-<bot user id>
... INFO hello_bot: connected
```

Leave it running. Your page is only available while the bot is connected.

## 7. Attach the workflow to a channel

1. Open your niche at `https://tawks.app/niche/<your niche slug>`.
2. Click the info icon next to the niche name at the top of the channel sidebar to open the niche settings.
3. Under **Settings**, choose **Channels**, then select the channel you want your page in.
4. On the **Details** tab, open the **Workflow** list and pick **Hello (wf-hello-&lt;bot user id&gt;)**. The list shows every registered workflow, so look for the id your bot logged.
5. Click **Save**.

Editing a channel's details needs the Manage Channels permission. As the niche owner you have it.

To put a game in lobbies, build it as a [game activity](07-tables.md): a niche turns it on and it appears in the lobby's game menu. Leave the channel's **Lobby workflow** setting on Default; it controls how lobbies are created, not what plays in them. [Building games](06-games.md) covers the rest.

## 8. See it working

Open the channel. You should see "Hello, " followed by your display name, the line "No waves yet" and a **Wave** button. Click it and the line changes to "Waved 1 times", then counts up with every click.

Open the same channel on your phone or in the desktop app and you get the same page with no extra work.

If something is off:

| What you see | What to check |
| --- | --- |
| "This workflow is unavailable right now because no connected bot can handle it." | Your bot is not running or not connected. Check its log for `connected`. When it reconnects, open the channel again. |
| "No workflow for this channel" | The channel has no workflow selected. Go back to step 7. |
| Your workflow is missing from the Workflow list | The bot has not registered it yet. Start the bot, wait for the `registered` line, then reopen the settings. |
| `Unauthorized` in the bot's log | `BOT_TOKEN_TYPE` is not `Bot`, or the token was copied incorrectly. Run the `curl` check from step 1. |
| The page loads but the count does not change | The `wave` handler returned an error. The SDK sends the error back to tawks instead of new responses, so log errors inside your handlers to see them. |

## Next steps

- [Writing a bot](03-bots.md): server actions, events, chat replies, notifications, share cards and the reconnect loop in full.
- [Workflows](04-workflows.md): nodes, conditions, actions and templates.
- [Display reference](05-displays.md): every display your pages can use.
- [Building games](06-games.md) and [Game activities](07-tables.md): games that run in lobbies.

Questions, bugs in these docs and requests for features go to `dev-chat` in the Tawks Dev niche.
