use anyhow::{Result, anyhow};
use dashmap::DashMap;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::{ServerActionHandler, TawksBot};
use tokio::sync::{mpsc, oneshot};
use workflow::service::{
    ProcessWorkflowActionArgs, WorkflowResourceWithContext,
};
use workflow::{server_action::ServerActionResult, service::StartWorkflowFromInstanceArgs};

use crate::{GameEffect, GameLogic};

/// One per lobby. Message-driven actor.
/// A revision for each game view, increasing across lobbies and restarts, so
/// the workflow engine can drop a view that arrives after a newer one.
fn next_view_rev() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0);
    let mut prev = LAST.load(Ordering::Relaxed);
    loop {
        let next = now.max(prev + 1);
        match LAST.compare_exchange_weak(prev, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(seen) => prev = seen,
        }
    }
}

enum Msg<G: GameLogic<Effect = GameEffect>> {
    /// Normal game action (vote, accuse, tick, etc.)
    /// Reply returns: (caller_view, effects_emitted_by_update)
    Action {
        user_instance_id: String,
        player_id: String,
        action: G::Action,
        reply: oneshot::Sender<Result<(G::View, Vec<GameEffect>, u64)>>,
    },

    /// Run arbitrary code against &mut game state and return a ServerActionResult directly.
    /// This is the "state handler" escape hatch for workflows that must return CompleteWorkflow, etc.
    WithGame {
        user_id: String,
        f: Box<dyn FnOnce(&mut G, &str) -> Result<ServerActionResult> + Send + 'static>,
        reply: oneshot::Sender<Result<ServerActionResult>>,
    },
}

type LobbyTx<G> = mpsc::Sender<Msg<G>>;

pub struct GameEngine<G: GameLogic<Effect = GameEffect>> {
    lobbies: Arc<DashMap<String, LobbyTx<G>>>,
    bot: TawksBot,
    lobby_tasks: Arc<DashMap<String, tokio::task::JoinHandle<()>>>,
    init_args: G::InitArgs,
}

impl<G: GameLogic<Effect = GameEffect>> Clone for GameEngine<G> {
    fn clone(&self) -> Self {
        Self {
            lobbies: self.lobbies.clone(),
            bot: self.bot.clone(),
            lobby_tasks: self.lobby_tasks.clone(),
            init_args: self.init_args.clone(),
        }
    }
}

impl<G> GameEngine<G>
where
    G: GameLogic<Effect = GameEffect>,
{
    pub fn new(init_args: G::InitArgs, bot: TawksBot) -> Self {
        Self {
            lobbies: Arc::new(DashMap::new()),
            bot,
            lobby_tasks: Arc::new(DashMap::new()),
            init_args,
        }
    }

    // =========================================================================
    // Registration helpers
    // =========================================================================

    /// Standard route: mapper(ctx) -> G::Action, engine runs it, returns UpdateResponses { game: view }.
    pub async fn register_route<F>(self: Arc<Self>, action_name: &str, mapper: F) -> Result<String>
    where
        F: Fn(WorkflowResourceWithContext) -> Result<G::Action> + Send + Sync + 'static,
    {
        let engine = self.clone();
        let mapper = Arc::new(mapper);

        let handler: ServerActionHandler = Arc::new(Box::new(move |ctx| {
            let engine = engine.clone();
            let mapper = mapper.clone();

            Box::pin(async move {
                let lobby_id = ctx
                    .get_input_as::<String>("lobby_id")
                    .or(ctx
                        .get_input_as::<String>("channel_id")
                        .map(|c| format!("#channel-{}", c)))
                    .map_err(|_| anyhow!("Missing 'lobby_id'"))?;

                let action = match mapper(ctx.clone()) {
                    Ok(a) => a,
                    Err(e) => return Ok(ServerActionResult::Error(e.to_string())),
                };

                match engine
                    .handle_game_action_with_rev(
                        lobby_id,
                        ctx.workflow.instance_id.clone(),
                        ctx.workflow.user_id.clone(),
                        action,
                    )
                    .await
                {
                    Ok((view, _effects, rev)) => {
                        let mut responses = HashMap::new();
                        responses.insert("game".to_string(), serde_json::to_value(view).unwrap());
                        responses.insert("$rev".to_string(), serde_json::json!(rev));
                        Ok(ServerActionResult::UpdateResponses(responses))
                    }
                    Err(e) => Ok(ServerActionResult::Error(e.to_string())),
                }
            })
        }));

        self.bot.register_server_action(action_name, handler).await
    }

    /// Route variant: if the action emits GameEffect::CompleteWorkflow for this user,
    /// return ServerActionResult::CompleteWorkflow instead of UpdateResponses.
    ///
    /// Use this for ShowBound-style actions that need to complete a workflow immediately.
    pub async fn register_route_with_result<F>(
        self: Arc<Self>,
        action_name: &str,
        mapper: F,
    ) -> Result<String>
    where
        F: Fn(WorkflowResourceWithContext) -> Result<G::Action> + Send + Sync + 'static,
    {
        let engine = self.clone();
        let mapper = Arc::new(mapper);

        let handler: ServerActionHandler = Arc::new(Box::new(move |ctx| {
            let engine = engine.clone();
            let mapper = mapper.clone();

            Box::pin(async move {
                let lobby_id = ctx
                    .get_input_as::<String>("lobby_id")
                    .or(ctx
                        .get_input_as::<String>("channel_id")
                        .map(|c| format!("#channel-{}", c)))
                    .map_err(|_| anyhow!("Missing 'lobby_id'"))?;

                let action = match mapper(ctx.clone()) {
                    Ok(a) => a,
                    Err(e) => return Ok(ServerActionResult::Error(e.to_string())),
                };

                let user_id = ctx.workflow.user_id.clone();

                match engine
                    .handle_game_action_with_rev(
                        lobby_id,
                        ctx.workflow.instance_id.clone(),
                        user_id.clone(),
                        action,
                    )
                    .await
                {
                    Ok((view, effects, rev)) => {
                        for eff in effects {
                            if let GameEffect::CompleteWorkflow {
                                user_id: eff_user_id,
                                message,
                                responses,
                                close_active_modal,
                            } = eff
                            {
                                if eff_user_id == user_id {
                                    return Ok(ServerActionResult::CompleteWorkflow {
                                        message,
                                        responses,
                                        close_active_modal,
                                    });
                                }
                            }
                        }

                        // default fallback: update game view
                        let mut responses = HashMap::new();
                        responses.insert("game".to_string(), serde_json::to_value(view).unwrap());
                        responses.insert("$rev".to_string(), serde_json::json!(rev));
                        Ok(ServerActionResult::UpdateResponses(responses))
                    }
                    Err(e) => Ok(ServerActionResult::Error(e.to_string())),
                }
            })
        }));

        self.bot.register_server_action(action_name, handler).await
    }

    /// State handler: runs a closure against &mut game state (inside the lobby actor)
    /// and returns the ServerActionResult directly.
    ///
    /// Use this when you want to return CompleteWorkflow / Error / UpdateResponses
    /// without forcing it through G::Action.
    pub async fn register_state_handler<F>(
        self: Arc<Self>,
        action_name: &str,
        handler: F,
    ) -> Result<String>
    where
        F: Fn(&mut G, WorkflowResourceWithContext) -> Result<ServerActionResult>
            + Send
            + Sync
            + 'static,
    {
        let engine = self.clone();
        let handler = Arc::new(handler);

        let action_handler: ServerActionHandler = Arc::new(Box::new(move |ctx| {
            let engine = engine.clone();
            let handler = handler.clone();

            Box::pin(async move {
                let lobby_id = ctx
                    .get_input_as::<String>("lobby_id")
                    .or(ctx
                        .get_input_as::<String>("channel_id")
                        .map(|c| format!("#channel-{}", c)))
                    .map_err(|_| anyhow!("Missing 'lobby_id'"))?;

                // Ensure lobby actor exists
                let tx = engine.get_or_start_lobby(lobby_id.clone());

                let (reply_tx, reply_rx) = oneshot::channel();

                // Run handler inside lobby actor with &mut game
                tx.send(Msg::WithGame {
                    user_id: ctx.workflow.user_id.clone(),
                    f: Box::new(move |game: &mut G, _user_id: &str| handler(game, ctx)),
                    reply: reply_tx,
                })
                .await
                .map_err(|_| anyhow!("Lobby task is down"))?;

                match reply_rx.await {
                    Ok(Ok(res)) => Ok(res),
                    Ok(Err(e)) => Ok(ServerActionResult::Error(e.to_string())),
                    Err(_) => Ok(ServerActionResult::Error("Lobby dropped reply".into())),
                }
            })
        }));

        self.bot
            .register_server_action(action_name, action_handler)
            .await
    }

    // =========================================================================
    // Public entrypoint used by register_route/with_result
    // =========================================================================

    pub async fn handle_game_action_with_effects(
        &self,
        lobby_id: String,
        user_instance_id: String,
        player_id: String,
        action: G::Action,
    ) -> Result<(G::View, Vec<GameEffect>)> {
        let (view, effects, _) = self
            .handle_game_action_with_rev(lobby_id, user_instance_id, player_id, action)
            .await?;
        Ok((view, effects))
    }

    /// Like `handle_game_action_with_effects`, plus the view's revision.
    async fn handle_game_action_with_rev(
        &self,
        lobby_id: String,
        user_instance_id: String,
        player_id: String,
        action: G::Action,
    ) -> Result<(G::View, Vec<GameEffect>, u64)> {
        tracing::info!(
            lobby_id = %lobby_id,
            player_id = %player_id,
            "game action received"
        );
        let tx = self.get_or_start_lobby(lobby_id.clone());

        let (reply_tx, reply_rx) = oneshot::channel();

        tx.send(Msg::Action {
            user_instance_id,
            player_id,
            action,
            reply: reply_tx,
        })
        .await
        .map_err(|_| anyhow!("Lobby task is down"))?;

        reply_rx.await.map_err(|_| anyhow!("Lobby dropped reply"))?
    }

    // =========================================================================
    // Lobby actor startup
    // =========================================================================

    fn get_or_start_lobby(&self, lobby_id: String) -> LobbyTx<G> {
        if let Some(tx) = self.lobbies.get(&lobby_id).map(|e| e.value().clone()) {
            if !tx.is_closed() {
                return tx;
            }
            // The actor exited (zombie shutdown); drop the dead sender so the
            // lobby restarts instead of swallowing every later action.
            tracing::info!(lobby_id = %lobby_id, "restarting lobby actor after shutdown");
            self.lobbies.remove(&lobby_id);
        }

        let (tx, mut rx) = mpsc::channel::<Msg<G>>(128);

        // insert sender first (so concurrent callers share it)
        self.lobbies.insert(lobby_id.clone(), tx.clone());

        let bot = self.bot.clone();

        let lobby_id_for_task = lobby_id.clone();
        let args = self.init_args.clone();

        tokio::spawn(async move {
            let mut game = G::new(args);
            game.lobby_started(&lobby_id_for_task);
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            let mut consecutive_zombie_ticks: u32 = 0;
            const MAX_ZOMBIE_TICKS: u32 = 10;

            // Watchdog: if a tick, system action, or user action takes longer than this,
            // emit a loud error with context. Intended to catch runaway AI loops or effect
            // cascades that would otherwise pin the lobby actor forever. Threshold is
            // well above any legitimate tick cost for these games.
            const SLOW_WORK_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(2000);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Zombie detection: if no player has an instance ID,
                        // the game has no connected workflows. Stop ticking
                        // after MAX_ZOMBIE_TICKS to prevent infinite loops.
                        let has_any_instance = game.get_player_ids().iter().any(|pid| {
                            game.get_instance_id_for_player_id(pid).is_some()
                        });
                        if !has_any_instance {
                            consecutive_zombie_ticks += 1;
                            if consecutive_zombie_ticks >= MAX_ZOMBIE_TICKS {
                                tracing::warn!(
                                    lobby_id = %lobby_id_for_task,
                                    "lobby has no workflow instances after {} ticks, shutting down",
                                    MAX_ZOMBIE_TICKS,
                                );
                                break;
                            }
                            continue;
                        } else {
                            consecutive_zombie_ticks = 0;
                        }

                        let tick_start = std::time::Instant::now();
                        let actions = game.tick();
                        let tick_elapsed = tick_start.elapsed();
                        if tick_elapsed >= SLOW_WORK_THRESHOLD {
                            tracing::error!(
                                lobby_id = %lobby_id_for_task,
                                elapsed_ms = tick_elapsed.as_millis(),
                                action_count = actions.len(),
                                "game.tick() exceeded slow-work threshold"
                            );
                        }
                        if actions.is_empty() { continue; }

                        for action in actions {
                            let action_start = std::time::Instant::now();
                            // "system" tick actions; no reply
                            let _ = process_and_fanout::<G>(
                                &lobby_id_for_task,
                                &mut game,
                                &bot,
                                "system".to_string(),
                                "system".to_string(),
                                action,
                            ).await;
                            let action_elapsed = action_start.elapsed();
                            if action_elapsed >= SLOW_WORK_THRESHOLD {
                                tracing::error!(
                                    lobby_id = %lobby_id_for_task,
                                    elapsed_ms = action_elapsed.as_millis(),
                                    "system-tick action exceeded slow-work threshold"
                                );
                            }
                        }
                    }

                    msg = rx.recv() => {
                        let Some(msg) = msg else {
                            break;
                        };

                        match msg {
                            Msg::Action { user_instance_id, player_id, action, reply } => {
                                let action_start = std::time::Instant::now();
                                // process update + effects + fanout
                                let effects_res = process_and_fanout::<G>(
                                    &lobby_id_for_task,
                                    &mut game,
                                    &bot,
                                    user_instance_id,
                                    player_id.clone(),
                                    action,
                                ).await;
                                let action_elapsed = action_start.elapsed();
                                if action_elapsed >= SLOW_WORK_THRESHOLD {
                                    tracing::error!(
                                        lobby_id = %lobby_id_for_task,
                                        player_id = %player_id,
                                        elapsed_ms = action_elapsed.as_millis(),
                                        "player action exceeded slow-work threshold"
                                    );
                                }

                                match effects_res {
                                    Ok(effects) => {
                                        let rev = next_view_rev();
                                        let view = game.view_for(&player_id);
                                        let _ = reply.send(Ok((view, effects, rev)));
                                    }
                                    Err(e) => {
                                        let _ = reply.send(Err(e));
                                    }
                                }
                            }

                            Msg::WithGame { user_id, f, reply } => {
                                let with_start = std::time::Instant::now();
                                let res = f(&mut game, &user_id);
                                let with_elapsed = with_start.elapsed();
                                if with_elapsed >= SLOW_WORK_THRESHOLD {
                                    tracing::error!(
                                        lobby_id = %lobby_id_for_task,
                                        user_id = %user_id,
                                        elapsed_ms = with_elapsed.as_millis(),
                                        "WithGame handler exceeded slow-work threshold"
                                    );
                                }
                                let _ = reply.send(res);
                            }
                        }
                    }
                }
            }
        });

        tx
    }
}

// =========================================================================
// Core processing inside lobby actor
// =========================================================================

async fn process_and_fanout<G>(
    lobby_id: &str,
    game: &mut G,
    bot: &TawksBot,
    user_instance_id: String,
    player_id: String,
    action: G::Action,
) -> Result<Vec<GameEffect>>
where
    G: GameLogic<Effect = GameEffect>,
{
    game.record_instance_id(&player_id, &user_instance_id);
    let should_fanout = game.should_fanout(&action);
    // A rejected action used to vanish here: the `?` propagated past the
    // completion log, so a refused move left no trace server-side and no
    // feedback client-side — indistinguishable from a dead button.
    let effects = match game.update(&player_id, action) {
        Ok(effects) => effects,
        Err(err) => {
            tracing::warn!(
                lobby_id = %lobby_id,
                player_id = %player_id,
                error = %err,
                "game.update rejected"
            );
            return Err(err);
        }
    };
    tracing::info!(
        lobby_id = %lobby_id,
        player_id = %player_id,
        effect_count = effects.len(),
        "game.update completed"
    );
    let user_ids = if should_fanout {
        game.get_player_ids()
    } else {
        vec![player_id.clone()]
    };
    let mut instance_by_user: HashMap<String, String> = HashMap::new();
    for uid in &user_ids {
        if let Some(iid) = game.get_instance_id_for_player_id(uid) {
            instance_by_user.insert(uid.clone(), iid);
        }
    }
    tracing::info!(
        lobby_id = %lobby_id,
        instances = instance_by_user.len(),
        users = user_ids.len(),
        "reconcile starting"
    );

    // bucket effects...
    let mut user_effects: HashMap<String, Vec<GameEffect>> = HashMap::new();
    let mut inject_effects_by_instance: HashMap<String, Vec<StartWorkflowFromInstanceArgs>> =
        HashMap::new();

    for effect in effects.iter().cloned() {
        match &effect {
            GameEffect::ClearWorkflow { user_id, .. }
            | GameEffect::CompleteWorkflow { user_id, .. }
            | GameEffect::EndGameCleanup { user_id, .. } => {
                user_effects
                    .entry(user_id.clone())
                    .or_default()
                    .push(effect);
            }
            GameEffect::InjectWorkflow(args) => {
                inject_effects_by_instance
                    .entry(args.instance_id.clone())
                    .or_default()
                    .push(args.clone());
            }
            GameEffect::RunServerAction {
                instance_id,
                action_id,
                inputs,
            } => {
                let _ = bot
                    .sdk()
                    .server_actions
                    .process(ProcessWorkflowActionArgs {
                        instance_id: instance_id.clone(),
                        action_id: action_id.clone(),
                        inputs: inputs.clone(),
                    })
                    .await;
            }
            GameEffect::Speak { text } => {
                bot.speak(lobby_id.into(), text.clone()).await?;
            }
            GameEffect::MilestoneEvent {
                game_id,
                event_key,
                user_id,
                meta,
                ..
            } => {
                let sdk = bot.sdk.clone();
                let game_id = game_id.clone();
                let event_key = event_key.clone();
                let user_id = user_id.clone();
                let lid = lobby_id.to_string();
                let meta = meta.clone();
                tokio::spawn(async move {
                    let _ = sdk
                        .web_client
                        .mutate::<_, ()>(
                            "milestone_event",
                            &json!({
                                "game_id": game_id,
                                "event_key": event_key,
                                "user_id": user_id,
                                "lobby_id": lid,
                                "meta": meta,
                            }),
                        )
                        .await;
                });
            }
        }
    }

    // If no users have workflow instances, this is a zombie game --
    // all sessions were cleaned up but the lobby actor is still alive.
    // Skip reconcile entirely to avoid a tight error loop.
    if instance_by_user.is_empty() {
        tracing::debug!(
            "reconcile skipped: no workflow instances for any user in {:?}",
            user_ids
        );
        return Ok(effects);
    }

    for user_id in user_ids {
        let Some(instance_id) = instance_by_user.get(&user_id).cloned() else {
            tracing::debug!("no instance id found for user_id {}", user_id);
            continue;
        };

        // Skip reconciling players that have pending cleanup/restart effects --
        // reconcile_node: true would fight with the workflow_restart.
        let has_cleanup = user_effects.get(&user_id).map_or(false, |effs| {
            effs.iter().any(|e| {
                matches!(
                    e,
                    GameEffect::EndGameCleanup { .. }
                        | GameEffect::ClearWorkflow { .. }
                        | GameEffect::CompleteWorkflow { .. }
                )
            })
        });

        let rev = next_view_rev();
        let view = game.view_for(&user_id);

        if !has_cleanup {
            // reconcile...
            bot.sdk()
                .web_client
                .mutate::<_, ()>(
                    "workflow_reconcile_instance",
                    &json!({
                        "instance_id": instance_id,
                        "inputs": { "game": view, "lobby_id": lobby_id, "$rev": rev },
                        "context": {},
                        "reconcile_node": true,
                        "target_user_id": user_id
                    }),
                )
                .await
                .ok();
        }

        // injects...
        if let Some(injects) = inject_effects_by_instance.get(&instance_id) {
            for args in injects {
                let mut args = args.clone();
                args.inputs
                    .insert("game".into(), serde_json::to_value(view.clone()).unwrap());
                args.inputs
                    .insert("lobby_id".into(), serde_json::to_value(lobby_id).unwrap());
                let _ = bot.start_workflow_from_instance(args).await;
            }
        }

        // per-user effects...
        if let Some(effs) = user_effects.get(&user_id) {
            for eff in effs {
                match eff {
                    GameEffect::EndGameCleanup { .. } => {
                        tracing::info!(
                            user = %user_id,
                            instance = %instance_id,
                            "EndGameCleanup"
                        );

                        // Final reconcile with reconcile_node=false so the
                        // client sees the end state. Previously workflow_restart
                        // caused zombie loops.
                        let _ = bot
                            .sdk()
                            .web_client
                            .mutate::<_, ()>(
                                "workflow_reconcile_instance",
                                &json!({
                                    "instance_id": instance_id,
                                    "inputs": { "game": view, "lobby_id": lobby_id, "$rev": rev },
                                    "context": {},
                                    "reconcile_node": false,
                                    "target_user_id": user_id
                                }),
                            )
                            .await;
                    }
                    GameEffect::CompleteWorkflow {
                        message, responses, ..
                    } => {
                        let mut inputs = responses.clone();
                        inputs.insert("game".into(), serde_json::to_value(view.clone()).unwrap());
                        inputs.insert("$rev".into(), serde_json::json!(rev));
                        inputs.insert("lobby_id".into(), serde_json::to_value(lobby_id).unwrap());
                        if let Err(error) = bot
                            .sdk()
                            .web_client
                            .mutate::<_, ()>(
                                "workflow_reconcile_instance",
                                &json!({
                                    "instance_id": instance_id,
                                    "inputs": inputs,
                                    "context": {},
                                    "reconcile_node": false,
                                    "complete_message": message,
                                    "target_user_id": user_id
                                }),
                            )
                            .await
                        {
                            tracing::warn!(
                                user = %user_id,
                                instance = %instance_id,
                                error = %error,
                                "failed to complete the player's workflow"
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(effects)
}
