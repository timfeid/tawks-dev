//! The engine-facing wrapper: seats and instances, AI turns on the tick,
//! and the effects that close everyone's workflow when the game ends.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use game_framework::{GameEffect, GameLogic};
use serde::{Deserialize, Serialize};

use crate::ai;
use crate::game::{Action, Card, Game, Stage};
use crate::view::{self, TracklinesView};

#[derive(Clone, Debug, Default)]
pub struct TracklinesInitArgs {
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TracklinesAction {
    View { name: String, avatar_url: String, instance_id: String },
    Join { name: String, avatar_url: String, instance_id: String },
    AddAi,
    Start,
    DrawCard { face_up: Option<usize> },
    ClaimRoute { route: u16, color: Option<Card> },
    DrawTickets,
    KeepTickets { keep: Vec<u16> },
    Reset,
    /// A tick-driven move for a practice AI seat.
    AiExecute { player_id: String, action: Action },
}

#[derive(Debug, Clone)]
pub struct TracklinesState {
    pub game: Game,
    pub viewer_instances: HashMap<String, String>,
    /// Instances that were watching when the last game finished; a view from
    /// any other instance starts a fresh table.
    pub finished_instances: HashSet<String>,
    pub finished_announced: bool,
}

const MAX_AI_MOVES_PER_TICK: usize = 6;

impl TracklinesState {
    fn is_ai(&self, id: &str) -> bool {
        self.game.player(id).map(|p| p.is_ai).unwrap_or(false)
    }

    fn reset_table(&mut self) {
        let seed = self.game.seed.wrapping_add(1);
        self.game = Game::new(seed);
        self.finished_instances.clear();
        self.finished_announced = false;
    }

    fn finish_effects(&mut self) -> Vec<GameEffect> {
        if self.game.stage != Stage::Finished || self.finished_announced {
            return Vec::new();
        }
        self.finished_announced = true;
        self.finished_instances = self.viewer_instances.values().cloned().collect();
        let winner = self
            .game
            .winner_id
            .as_ref()
            .and_then(|id| self.game.player(id))
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let mut effects = Vec::new();
        for p in &self.game.players {
            if p.is_ai {
                continue;
            }
            let message = if Some(&p.id) == self.game.winner_id.as_ref() {
                format!("You win with {} points!", p.score)
            } else {
                format!("{winner} wins. You finished with {} points.", p.score)
            };
            effects.push(GameEffect::CompleteWorkflow {
                user_id: p.id.clone(),
                message,
                responses: HashMap::new(),
                close_active_modal: false,
            });
            if Some(&p.id) == self.game.winner_id.as_ref() {
                let mut meta = HashMap::new();
                meta.insert("won".into(), serde_json::json!(true));
                meta.insert("score".into(), serde_json::json!(p.score));
                meta.insert("player_count".into(), serde_json::json!(self.game.players.len()));
                effects.push(GameEffect::MilestoneEvent {
                    game_id: "tracklines".into(),
                    event_key: "game_completed".into(),
                    user_id: p.id.clone(),
                    lobby_id: String::new(),
                    meta,
                });
            }
        }
        effects
    }

    fn apply_game(&mut self, player_id: &str, action: Action) -> Result<Vec<GameEffect>> {
        self.game.apply(player_id, action)?;
        Ok(self.finish_effects())
    }
}

impl GameLogic for TracklinesState {
    type Action = TracklinesAction;
    type View = TracklinesView;
    type Effect = GameEffect;
    type InitArgs = TracklinesInitArgs;

    fn new(args: Self::InitArgs) -> Self {
        let seed = args.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(7)
        });
        Self {
            game: Game::new(seed),
            viewer_instances: HashMap::new(),
            finished_instances: HashSet::new(),
            finished_announced: false,
        }
    }

    fn get_instance_id_for_player_id(&self, player_id: &str) -> Option<String> {
        self.viewer_instances.get(player_id).cloned()
    }

    fn get_player_ids(&self) -> Vec<String> {
        let mut ids: HashSet<String> = self.game.players.iter().map(|p| p.id.clone()).collect();
        ids.extend(self.viewer_instances.keys().cloned());
        ids.into_iter().collect()
    }

    fn should_fanout(&self, action: &Self::Action) -> bool {
        !matches!(action, TracklinesAction::View { .. })
    }

    fn update(&mut self, player_id: &str, action: Self::Action) -> Result<Vec<Self::Effect>> {
        match action {
            TracklinesAction::View { instance_id, .. } => {
                if self.game.stage == Stage::Finished && !self.finished_instances.contains(&instance_id) {
                    self.reset_table();
                }
                self.viewer_instances.insert(player_id.to_string(), instance_id);
                Ok(Vec::new())
            }
            TracklinesAction::Join { name, avatar_url, instance_id } => {
                self.viewer_instances.insert(player_id.to_string(), instance_id);
                self.apply_game(player_id, Action::Join { name, avatar_url })
            }
            TracklinesAction::AddAi => self.apply_game(player_id, Action::AddAi),
            TracklinesAction::Start => self.apply_game(player_id, Action::Start),
            TracklinesAction::DrawCard { face_up } => self.apply_game(player_id, Action::DrawCard { face_up }),
            TracklinesAction::ClaimRoute { route, color } => {
                self.apply_game(player_id, Action::ClaimRoute { route, color })
            }
            TracklinesAction::DrawTickets => self.apply_game(player_id, Action::DrawTickets),
            TracklinesAction::KeepTickets { keep } => self.apply_game(player_id, Action::KeepTickets { keep }),
            TracklinesAction::Reset => {
                let host = self.game.host_id.as_deref() == Some(player_id);
                if self.game.stage == Stage::Playing && !host {
                    return Err(anyhow!("Only the host can reset a game in progress"));
                }
                self.reset_table();
                Ok(Vec::new())
            }
            TracklinesAction::AiExecute { player_id: ai_id, action } => {
                if !self.is_ai(&ai_id) {
                    return Err(anyhow!("Not an AI seat"));
                }
                self.apply_game(&ai_id, action)
            }
        }
    }

    fn view_for(&self, player_id: &str) -> Self::View {
        view::build(self, player_id)
    }

    fn tick(&mut self) -> Vec<Self::Action> {
        if self.game.stage != Stage::Playing {
            return Vec::new();
        }
        let mut probe = self.game.clone();
        let mut out = Vec::new();
        for _ in 0..MAX_AI_MOVES_PER_TICK {
            let ids: Vec<String> = probe.players.iter().filter(|p| p.is_ai).map(|p| p.id.clone()).collect();
            let Some((id, action)) = ids.iter().find_map(|id| ai::next_action(&probe, id).map(|a| (id.clone(), a))) else {
                break;
            };
            if probe.apply(&id, action.clone()).is_err() {
                break;
            }
            out.push(TracklinesAction::AiExecute { player_id: id, action });
            if probe.stage != Stage::Playing {
                break;
            }
        }
        out
    }
}
