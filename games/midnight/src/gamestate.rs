use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use rand::SeedableRng;
use rand::seq::IndexedRandom;
use rand::{rng, seq::SliceRandom};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, instrument, trace, warn};
use workflow::server_action::ServerActionResult;
use workflow::service::{InjectWorkflowPlace, StartWorkflowFromInstanceArgs};

use crate::roles::{self, Alliance, RoleCard, WorkflowStart};
use game_framework::{GameEffect, GameLogic};

pub fn canonical_workflow_id(id: &str) -> &str {
    // wf-<base>-<owner> -> <base>
    if let Some(rest) = id.strip_prefix("wf-") {
        if let Some((base, _owner)) = rest.rsplit_once('-') {
            return base;
        }
    }
    id
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerRole {
    player: PlayerResource,
    role: Arc<RoleCard>,
}
impl PlayerRole {
    pub fn new_from_hidden(index: usize, role: Arc<RoleCard>) -> Self {
        Self {
            player: PlayerResource::new_from_hidden(index),
            role,
        }
    }
    pub fn new(p: &Player) -> Self {
        Self {
            player: p.into(),
            role: p.effective_role_card(None),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stage {
    pub index: usize,
    pub role: Arc<RoleCard>,
    pub player_ids: Vec<String>,
    pub duration_seconds: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerResourceVote {
    pub player: PlayerResource,
    pub voted_role: Option<Arc<RoleCard>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerResource {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub is_host: bool,
    pub is_open_slot: bool,
    pub is_alive: bool,
    pub is_winner: bool,
    pub hidden: Option<usize>,
    pub executed: bool,
    pub is_sus: bool,
    pub vote_locked: bool,
    pub role: Option<Arc<RoleCard>>,
    pub accusations: Vec<PlayerResourceVote>,
    pub votes: Vec<PlayerResource>,
    pub class: String,
}

impl PlayerResource {
    pub fn new_from_hidden(index: usize) -> PlayerResource {
        PlayerResource {
            role: None,
            class: "".into(),
            vote_locked: false,
            id: format!("hidden{index}"),
            name: format!("Hidden {}", index + 1),
            avatar_url: "https://cdn.tawks.app/midnight/midnight.webp".into(),
            is_open_slot: false,
            accusations: vec![],
            votes: vec![],
            is_winner: false,
            hidden: Some(index),
            executed: false,
            is_host: false,
            is_alive: false,
            is_sus: false,
        }
    }
}

impl PartialEq for PlayerResource {
    #[instrument]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TurnResource {
    pub role: Arc<RoleCard>,
    pub ends_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VotingResultsResource {
    pub ends_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameResult {
    pub winners: Vec<String>,
    pub winning_team: Option<Alliance>,
    pub actions: Vec<GameHistoryEntry>,
}

impl Into<PlayerResource> for &Player {
    #[instrument]
    fn into(self) -> PlayerResource {
        PlayerResource {
            id: self.id.clone(),
            name: self.name.clone(),
            avatar_url: self.avatar_url.clone(),
            is_host: self.is_host,
            is_open_slot: false,
            is_alive: self.is_alive,
            is_winner: false,
            hidden: None,
            executed: !self.is_alive,
            is_sus: self.is_sus,
            vote_locked: self.vote_locked,
            role: None,
            accusations: vec![],
            votes: vec![],
            class: "".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameHistoryEntry {
    pub who: PlayerRole,
    pub action: String,
    pub what: Vec<PlayerRole>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoleTransactionVisibilty {
    Shown,
    Hidden,
}

impl RoleTransaction {
    fn new_hidden(card: Arc<RoleCard>, reason: &str) -> RoleTransaction {
        RoleTransaction {
            card,
            reason: reason.into(),
            visibility: RoleTransactionVisibilty::Hidden,
        }
    }

    fn new_shown(card: Arc<RoleCard>, reason: &str) -> RoleTransaction {
        RoleTransaction {
            card,
            reason: reason.into(),
            visibility: RoleTransactionVisibilty::Shown,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoleTransaction {
    pub card: Arc<RoleCard>,
    pub reason: String,
    pub visibility: RoleTransactionVisibilty,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub is_host: bool,
    pub is_alive: bool,
    pub is_sus: bool,
    pub vote_locked: bool,
    pub vote_target: Option<String>,
    pub role_card_stack: Vec<RoleTransaction>,
    pub accusations: HashMap<String, String>,
    instance_id: String,
}

impl Player {
    #[instrument(skip(self))]
    pub fn effective_role_card(&self, filter: Option<RoleTransactionVisibilty>) -> Arc<RoleCard> {
        self.role_card_stack
            .iter()
            .rev()
            .find(|c| {
                if let Some(filter) = &filter {
                    &c.visibility == filter
                } else {
                    true
                }
            })
            .cloned()
            .and_then(|x| Some(x.card))
            .expect("not in game yet?")
    }

    fn original_card(&self) -> Arc<RoleCard> {
        self.role_card_stack[0].clone().card
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GameStage {
    NotStarted,
    Lobby,
    Day,
    NightTurn,
    Ability { role_id: String },
    Voting,
    Results,
}

impl Default for GameStage {
    #[instrument]
    fn default() -> Self {
        Self::NotStarted
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct GameState {
    pub lobby_id: String,
    pub owner_user_id: String,
    pub is_locked: bool,
    pub hidden_role_count: usize,
    pub min_players: usize,

    pub stage: GameStage,
    pub players: HashMap<String, Player>,
    pub all_roles: Vec<Arc<RoleCard>>,

    pub hidden_cards: Vec<Arc<RoleCard>>,

    pub history: Vec<GameHistoryEntry>,

    pub stages_queue: VecDeque<Stage>,
    pub current_stage: Option<Stage>,

    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub timer_expires_at: Option<DateTime<Utc>>,

    pub winner: Option<Alliance>,
    sabotaged_inputs: HashMap<(String, String), (PlayerRole, HashMap<String, Value>)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum MidnightAction {
    Join {
        name: String,
        instance_id: String,
        avatar: String,
    },
    EnterAbility,
    ToggleLock,
    EnableRole {
        role_id: String,
    },
    StartGame,
    Sus {
        target_id: String,
    },
    Accuse {
        target_id: String,
        role_id: String,
    },
    Vote {
        target_id: String,
    },
    Reset,
    TickStageExpired,
    Tick,
    ShowBound,
}

#[derive(Default, Clone, Debug)]
pub struct MidnightInitArgs {}

impl GameLogic for GameState {
    #[instrument(skip(self))]
    fn get_player_ids(&self) -> Vec<String> {
        self.players.iter().map(|p| p.1.id.clone()).collect()
    }
    type Action = MidnightAction;
    type View = Value;

    type Effect = GameEffect;
    type InitArgs = MidnightInitArgs;
    fn get_instance_id_for_player_id(&self, player_id: &str) -> Option<String> {
        self.players.get(player_id).map(|p| p.instance_id.clone())
    }

    #[instrument]
    fn new(args: Self::InitArgs) -> Self {
        let mut this = Self::default();
        this.hidden_role_count = 3;
        this.min_players = 1;

        this.all_roles = vec![
            // roles::saboteur::saboteur_card(),
            roles::hollow::hollow_card(true),
            roles::hollow::hollow_card(false),
            roles::follower::follower_card(false),
            roles::spy::spy_card(true),
            roles::bound::bound_card(false),
            roles::bound::bound_card(false),
            roles::witness::witness_card(false),
            roles::lost::lost_card(false),
            roles::tangle::tangle_card(),
            roles::taker::taker_card(false),
            roles::lucid::lucid_card(true),
            roles::unmarked::unmarked_card(true),
            roles::unmarked::unmarked_card(false),
            roles::unmarked::unmarked_card(false),
        ];
        this
    }

    #[instrument(skip(self))]
    fn update(&mut self, player_id: &str, action: Self::Action) -> Result<Vec<Self::Effect>> {
        let mut effects = Vec::new();

        match action {
            MidnightAction::ShowBound => {
                let res = crate::roles::bound::resolve_bound_info(self, player_id)?;

                if let ServerActionResult::CompleteWorkflow {
                    message,
                    responses,
                    close_active_modal,
                } = res
                {
                    effects.push(GameEffect::CompleteWorkflow {
                        user_id: player_id.to_string(),
                        message,
                        responses,
                        close_active_modal,
                    });
                } else {
                }
            }
            MidnightAction::Join {
                name,
                avatar,
                instance_id,
            } => {
                trace!("Player joining: {player_id}");
                if self.stage == GameStage::NotStarted {
                    self.stage = GameStage::Lobby;
                }
                if self.stage == GameStage::Lobby
                    && (!self.is_locked || self.players.contains_key(player_id))
                {
                    let player = Player {
                        id: player_id.to_string(),
                        instance_id,
                        name,
                        avatar_url: avatar,
                        is_host: self.players.is_empty(),
                        is_alive: true,
                        ..Default::default()
                    };
                    let is_host = player.is_host;
                    self.players.insert(player_id.to_string(), player);
                    if is_host {
                        // self.add_dummy_bot();
                    }
                }
            }
            MidnightAction::ToggleLock => {
                trace!("Toggling lock by: {player_id}");

                if self.stage != GameStage::Voting {
                    return Err(anyhow!("Not voting stage"));
                }

                let me = self
                    .players
                    .get_mut(player_id)
                    .ok_or(anyhow!("No player"))?;

                if !me.vote_locked && me.vote_target.is_none() {
                    return Err(anyhow!("Select a vote target first"));
                }

                me.vote_locked = !me.vote_locked;

                if self.all_votes_ready() {
                    info!("All votes locked + selected, resolving immediately");
                    self.resolve_votes();
                    self.emit_milestone_events(&mut effects);
                }
            }

            MidnightAction::EnableRole { role_id } => {
                info!("Toggling role enable: {role_id} by {player_id}");
                self.ensure_host(player_id)?;
                if let Some(r) = self.all_roles.iter_mut().find(|r| r.id == role_id) {
                    // Views hold clones of the card, so copy it rather than give up.
                    let card = Arc::make_mut(r);
                    card.enabled = !card.enabled;
                    if card.enabled {
                        if let Some(text) = card.enabled_texts.first() {
                            effects.push(GameEffect::Speak { text: text.clone() });
                        }
                    }
                }
            }

            MidnightAction::StartGame => {
                trace!("Start game by: {player_id}");
                self.ensure_host(player_id)?;
                self.start_game(&mut effects)?;
            }
            MidnightAction::TickStageExpired => {
                trace!("TickStageExpired called");
                self.advance_stage(&mut effects);
            }
            MidnightAction::Sus { target_id } => {
                trace!("SUS: {target_id} by {player_id}");

                let me_name = self
                    .players
                    .get(player_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                let instance_ids: Vec<String> = self
                    .players
                    .values()
                    .map(|p| p.instance_id.clone())
                    .collect();

                for instance_id in instance_ids {
                    effects.push(GameEffect::InjectWorkflow(StartWorkflowFromInstanceArgs {
                        workflow_definition_id: "sus_animation_workflow".into(),
                        inputs: {
                            let mut input = HashMap::new();
                            input.insert("target_id".into(), json!(target_id));
                            input.insert("player".into(), json!({ "name": me_name }));
                            input
                        },
                        context: HashMap::new(),
                        instance_id,
                        attach_as: None,
                        inject: None,
                    }));
                }
            }
            MidnightAction::Accuse { target_id, role_id } => {
                trace!("Accuse: {player_id} accused {target_id} of being {role_id}");
                let me = self
                    .players
                    .get_mut(player_id)
                    .ok_or(anyhow!("No player"))?;
                me.accusations.insert(target_id, role_id);
            }
            MidnightAction::Vote { target_id } => {
                trace!("Vote: {target_id} by {player_id}");
                if self.stage == GameStage::NotStarted {
                    return Err(anyhow!("not started"));
                }
                let me = self
                    .players
                    .get_mut(player_id)
                    .ok_or(anyhow!("No player"))?;

                me.vote_target = Some(target_id);
            }
            MidnightAction::Reset => {
                trace!("Reset game by: {player_id}");
                self.ensure_host(player_id)?;

                for user_id in self.players.keys().into_iter() {
                    effects.push(GameEffect::EndGameCleanup {
                        user_id: user_id.to_string(),
                    });
                }
                self.reset_game();
            }
            MidnightAction::Tick => {
                // existing timer expiry logic
                if let Some(expiry) = self.timer_expires_at {
                    if Utc::now() > expiry {
                        self.advance_stage(&mut effects);
                    }
                }
            }

            _ => {
                trace!("Other action invoked: {:?}", action);
            }
        }

        Ok(effects)
    }

    #[instrument(skip(self))]
    fn view_for(&self, viewer_id: &str) -> Value {
        let winners: Vec<String> = if let Some(w) = &self.winner {
            self.players
                .values()
                .filter(|p| p.effective_role_card(None).alliance == *w)
                .map(|p| p.id.clone())
                .collect()
        } else {
            vec![]
        };

        let mut vote_counts: HashMap<String, usize> = HashMap::new();
        for p in self.players.values().filter(|p| p.is_alive) {
            if let Some(tid) = &p.vote_target {
                *vote_counts.entry(tid.clone()).or_default() += 1;
            }
        }

        let max_votes = vote_counts.values().max().cloned().unwrap_or(0);
        let dying_ids: Vec<String> = vote_counts
            .iter()
            .filter(|(_, c)| **c > 1 && **c == max_votes)
            .map(|(k, _)| k.clone())
            .collect();

        let self_card_filter = if self.stage == GameStage::Results {
            None
        } else {
            Some(RoleTransactionVisibilty::Shown)
        };

        let is_live = matches!(
            self.stage,
            GameStage::Day | GameStage::NightTurn | GameStage::Ability { .. } | GameStage::Voting
        );

        let player_resources: Vec<PlayerResource> = self
            .players
            .values()
            .map(|p| {
                let show_role = (p.id == viewer_id && is_live) || self.stage == GameStage::Results;

                let mut classes = vec![];
                if self.stage == GameStage::Voting && dying_ids.contains(&p.id) {
                    classes.push("dying".to_string());
                }
                if let Some(w) = &self.winner {
                    if p.effective_role_card(None).alliance == *w {
                        classes.push("winner".to_string());
                    } else {
                        classes.push("loser".to_string());
                    }
                }

                let incoming: Vec<PlayerResourceVote> = self
                    .players
                    .values()
                    .filter(|accuser| accuser.accusations.contains_key(&p.id))
                    .map(|accuser| {
                        let role_id = accuser.accusations.get(&p.id).cloned().unwrap_or_default();

                        let voted_role = self
                            .all_roles
                            .iter()
                            .find(|r| r.role_id == role_id)
                            .cloned();

                        PlayerResourceVote {
                            player: self.dumb_resource(accuser),
                            voted_role,
                        }
                    })
                    .collect();

                let votes: Vec<PlayerResource> = self
                    .players
                    .values()
                    .filter(|voter| voter.vote_target.as_deref() == Some(p.id.as_str()))
                    .map(|voter| self.dumb_resource(voter))
                    .collect();

                if self.stage == GameStage::Results && !p.is_alive {
                    classes.push("executed".to_string());
                }

                PlayerResource {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    avatar_url: p.avatar_url.clone(),
                    is_host: p.is_host,
                    is_open_slot: false,
                    hidden: None,
                    is_alive: p.is_alive,
                    is_winner: classes.contains(&"winner".to_string()),
                    executed: !p.is_alive,
                    is_sus: p.is_sus,
                    vote_locked: p.vote_locked,
                    role: if show_role {
                        Some(p.effective_role_card(self_card_filter.clone()))
                    } else {
                        None
                    },
                    accusations: incoming,
                    votes,
                    class: classes.join(" "),
                }
            })
            .collect();

        let turn = self.current_stage.as_ref().map(|s| TurnResource {
            role: s.role.clone(),
            ends_at: self
                .timer_expires_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        });

        let next_role = self.stages_queue.front().map(|s| s.role.clone());

        let voting_results = if self.stage == GameStage::Voting {
            Some(VotingResultsResource {
                ends_at: self
                    .timer_expires_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_default(),
            })
        } else {
            None
        };

        let result = if self.stage == GameStage::Results {
            Some(GameResult {
                winning_team: self.winner.clone(),
                actions: self.history.clone(),
                winners,
            })
        } else {
            None
        };

        json!({
            "is_live": is_live,
            "is_complete": self.stage == GameStage::Results,
            "is_role_reveal": self.stage == GameStage::Day,

            "role_reveal_until": self
                .timer_expires_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),

            "role": self.players.get(viewer_id).map(|p| is_live.then(|| p.effective_role_card(self_card_filter.clone()))),


            "owner_user_id": self.players.values().find(|p| p.is_host).and_then(|p| Some(p.id.clone())),
            "stage": format!("{:?}", self.stage),
            "players": player_resources,
            "all_roles": self.all_roles,
            "next_role": next_role,
            "turn": turn,
            "players_required": self.required_player_count(),
            "players_count": self.players.len(),
            "voting_results": voting_results,
            "result": result,
            "me": self.players.get(viewer_id).map(|p| json!({
                "role_details": is_live.then(|| p.effective_role_card(self_card_filter)),
                "can_vote": self.stage == GameStage::Voting && p.is_alive,
                "vote_locked": p.vote_locked,
                "has_voted": p.vote_target.is_some(),
            }))
        })
    }

    #[instrument(skip(self))]
    fn tick(&mut self) -> Vec<Self::Action> {
        if let Some(expiry) = self.timer_expires_at {
            if Utc::now() > expiry {
                return vec![MidnightAction::TickStageExpired];
            }
        }
        vec![]
    }
}

impl GameState {
    pub fn add_dummy_bot(&mut self) {
        let id = format!("BOT-{}", ulid::Ulid::new());

        self.players.insert(
            id.clone(),
            Player {
                id,
                name: "The Stranger".into(),
                avatar_url: "https://cdn.tawks.app/midnight/bot.png".into(),
                is_host: false,
                is_alive: true,
                is_sus: false,
                vote_locked: true, // important: never blocks voting
                vote_target: None,
                role_card_stack: vec![], // filled at game start
                accusations: HashMap::new(),
                instance_id: "bot".into(),
            },
        );
    }
    fn all_votes_ready(&self) -> bool {
        self.players
            .values()
            .filter(|p| p.is_alive)
            .all(|p| p.vote_locked && p.vote_target.is_some())
    }

    #[instrument(skip(self))]
    fn ensure_host(&self, id: &str) -> Result<()> {
        if self.players.get(id).map_or(false, |p| p.is_host) {
            Ok(())
        } else {
            Err(anyhow!("Host only"))
        }
    }

    #[instrument(skip(self))]
    fn reset_game(&mut self) {
        self.stage = GameStage::NotStarted;
        self.players = HashMap::new();
        self.winner = None;
        self.history.clear();
        self.current_stage = None;
        self.stages_queue.clear();
        self.timer_expires_at = None;
    }

    #[instrument(skip(self, p))]
    fn dumb_resource(&self, p: &Player) -> PlayerResource {
        p.into()
    }

    pub fn required_player_count(&self) -> usize {
        let enabled = self.all_roles.iter().filter(|c| c.enabled).count();
        enabled.saturating_sub(self.hidden_role_count)
    }

    #[instrument(skip(self, _effects))]
    fn start_game(&mut self, _effects: &mut Vec<GameEffect>) -> Result<()> {
        // ---- Validation: player count ----
        let alive_players = self.players.values().filter(|p| p.is_alive).count();
        if alive_players < self.min_players {
            return Err(anyhow!(format!(
                "Need at least {} players to start (have {})",
                self.min_players, alive_players
            )));
        }

        // ---- Validation: enabled cards count ----
        let enabled_cards: Vec<Arc<RoleCard>> = self
            .all_roles
            .iter()
            .filter(|r| r.enabled)
            .cloned()
            .collect();

        let total_cards_needed = alive_players + self.hidden_role_count;
        if enabled_cards.len() != total_cards_needed {
            return Err(anyhow!(format!(
                "Wrong number of enabled roles: need {} (players {} + hidden {}), have {} enabled.",
                total_cards_needed,
                alive_players,
                self.hidden_role_count,
                enabled_cards.len()
            )));
        }
        // ---- Shuffle & deal ----
        let mut deck = enabled_cards;
        deck.shuffle(&mut rng());

        for p in self.players.values_mut() {
            p.role_card_stack.clear();
            p.vote_locked = false;
            p.vote_target = None;
            p.accusations.clear();
            p.is_alive = true;
        }

        // Deal one shown card to each alive player
        for p in self.players.values_mut().filter(|p| p.is_alive) {
            let card = deck.pop().expect("validated length above");
            p.role_card_stack = vec![RoleTransaction {
                card,
                reason: "Start".into(),
                visibility: RoleTransactionVisibilty::Shown,
            }];
        }

        // Remaining cards become hidden cards
        // (Should be exactly hidden_role_count)
        self.hidden_cards = deck;
        if self.hidden_cards.len() != self.hidden_role_count {
            return Err(anyhow!(format!(
                "Internal error: expected {} hidden cards, got {}",
                self.hidden_role_count,
                self.hidden_cards.len()
            )));
        }

        // ---- Build night queue (dedup by role_id) ----
        self.generate_stages();

        // ---- Begin day phase ----
        self.stage = GameStage::Day;
        self.current_stage = None;
        self.timer_expires_at = Some(Utc::now() + Duration::seconds(52));
        Ok(())
    }

    #[instrument(skip(self))]
    fn generate_stages(&mut self) {
        let mut stages: Vec<Stage> = Vec::new();

        // Preserve order of all_roles, but only include each role_id once.
        let mut seen_role_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        let ordered_unique_roles: Vec<Arc<RoleCard>> = self
            .all_roles
            .iter()
            .filter(|r| r.enabled)
            .filter(|r| seen_role_ids.insert(r.role_id.clone()))
            .cloned()
            .collect();

        for role in ordered_unique_roles {
            let player_ids: Vec<String> = self
                .players
                .values()
                .filter(|p| p.is_alive && p.effective_role_card(None).role_id == role.role_id)
                .map(|p| p.id.clone())
                .collect();

            if player_ids.is_empty() {
                continue;
            }

            // If you want role-specific durations, keep night_duration;
            // otherwise you can clamp/min it like before.
            let dur = role.night_duration.max(0);
            let index = stages.len();

            stages.push(Stage {
                index,
                role,
                player_ids,
                duration_seconds: dur,
            });
        }

        self.stages_queue = std::collections::VecDeque::from(stages);
    }

    fn advance_stage(&mut self, effects: &mut Vec<GameEffect>) {
        if self.stage == GameStage::Day {}

        if let Some(next) = self.stages_queue.pop_front() {
            info!(
                "advance_stage: ENTER NightTurn role={} role_id={} card_id={} players={:?} duration_seconds={}",
                next.role.name,
                next.role.role_id,
                next.role.id,
                next.player_ids,
                next.duration_seconds
            );

            self.stage = GameStage::NightTurn;
            self.current_stage = Some(next.clone());
            self.timer_expires_at = Some(Utc::now() + Duration::seconds(next.duration_seconds));
            self.enter_ability_for_current_stage(effects);

            return;
        }

        match self.stage {
            GameStage::Day
            | GameStage::Ability { .. }
            | GameStage::NightTurn
            | GameStage::Lobby
            | GameStage::NotStarted => {
                if let Some(prev) = self.current_stage.take() {
                    for uid in prev.player_ids {
                        effects.push(GameEffect::ClearWorkflow {
                            user_id: uid,
                            workflow_id: "".into(),
                            attach_as: "game".into(),
                        });
                    }
                } else {
                    for uid in self.players.keys() {
                        effects.push(GameEffect::ClearWorkflow {
                            user_id: uid.clone(),
                            workflow_id: "".into(),
                            attach_as: "game".into(),
                        });
                    }
                }

                self.stage = GameStage::Voting;

                self.current_stage = None;
                self.timer_expires_at = Some(Utc::now() + Duration::seconds(60));
            }
            GameStage::Voting => {
                self.resolve_votes();
                self.emit_milestone_events(effects);
            }
            _ => {}
        }
    }

    #[instrument(skip(self))]
    fn resolve_votes(&mut self) {
        let mut vote_counts: HashMap<String, usize> = HashMap::new();

        for p in self.players.values().filter(|p| p.is_alive) {
            if !p.vote_locked {
                continue;
            }

            if let Some(tid) = &p.vote_target {
                *vote_counts.entry(tid.clone()).or_default() += 1;
            }
        }

        // As in the classic game, nobody dies unless someone draws more than one vote.
        let max_votes = vote_counts.values().max().cloned().unwrap_or(0);
        let executed_ids: Vec<String> = if max_votes > 1 {
            vote_counts.iter().filter(|(_, c)| **c == max_votes).map(|(id, _)| id.clone()).collect()
        } else {
            vec![]
        };

        let is_hollow = |p: &Player| p.effective_role_card(None).role_id == roles::hollow::HOLLOW_ROLE_ID;
        let hollows_in_play = self.players.values().any(|p| p.is_alive && is_hollow(p));
        let hollow_executed = executed_ids.iter().any(|id| self.players.get(id).is_some_and(is_hollow));

        for id in &executed_ids {
            if let Some(p) = self.players.get_mut(id) {
                p.is_alive = false;
            }
        }

        // The village wins by taking out a Hollow. With no Hollow among the
        // players, it wins only by sparing everyone; an innocent dying hands
        // the night to the Hollows' side, the Follower.
        self.winner = Some(if hollow_executed || (!hollows_in_play && executed_ids.is_empty()) {
            Alliance::Unmarked
        } else {
            Alliance::Hollow
        });

        self.stage = GameStage::Results;
        self.timer_expires_at = None;
    }

    fn emit_milestone_events(&self, effects: &mut Vec<GameEffect>) {
        if self.stage != GameStage::Results {
            return;
        }

        let winner = self.winner.as_ref();

        for (pid, player) in &self.players {
            let role = player.effective_role_card(None);
            let original = player.original_card();
            let is_winner = winner.map(|w| role.alliance == *w).unwrap_or(false);

            effects.push(GameEffect::MilestoneEvent {
                game_id: "midnight".into(),
                event_key: "game_completed".into(),
                user_id: pid.clone(),
                lobby_id: self.lobby_id.clone(),
                meta: HashMap::from([
                    ("won".into(), json!(is_winner)),
                    ("alliance".into(), json!(role.alliance)),
                    ("role_id".into(), json!(&role.role_id)),
                    ("original_role_id".into(), json!(&original.role_id)),
                    ("was_executed".into(), json!(!player.is_alive)),
                    (
                        "was_swapped".into(),
                        json!(role.role_id != original.role_id),
                    ),
                    ("player_count".into(), json!(self.players.len())),
                ]),
            });

            for (target_id, accused_role_id) in &player.accusations {
                let actual_role_id = self
                    .players
                    .get(target_id)
                    .map(|t| t.effective_role_card(None).role_id.clone());
                effects.push(GameEffect::MilestoneEvent {
                    game_id: "midnight".into(),
                    event_key: "accusation_resolved".into(),
                    user_id: pid.clone(),
                    lobby_id: self.lobby_id.clone(),
                    meta: HashMap::from([(
                        "was_correct".into(),
                        json!(actual_role_id.as_deref() == Some(accused_role_id.as_str())),
                    )]),
                });
            }
        }
    }

    #[instrument(skip(self, who, what))]
    pub fn add_history(&mut self, who: PlayerRole, action: &str, what: Vec<PlayerRole>) {
        self.history.push(GameHistoryEntry {
            who,
            action: action.to_string(),
            what,
        });
    }

    pub fn get_hidden_card(&self, position: usize) -> Result<Arc<RoleCard>> {
        self.hidden_cards
            .get(position)
            .cloned()
            .ok_or(anyhow!(format!("No hidden card at position {}", position)))
    }

    pub fn set_player_vote_locked(&mut self, player_id: &str) -> Result<()> {
        self.players
            .get_mut(player_id)
            .ok_or(anyhow!("no player"))?
            .vote_locked = true;
        Ok(())
    }

    pub fn set_player_role(&mut self, player_id: &String, role: Arc<RoleCard>) {
        if let Some(p) = self.players.get_mut(player_id) {
            p.role_card_stack
                .push(RoleTransaction::new_hidden(role, "Set by system"));
        } else {
            debug_assert!(false, "Player {} not found in set_player_role", player_id);
        }
    }

    pub fn set_hidden_card(&mut self, index: usize, role: Arc<RoleCard>) {
        if let Some(slot) = self.hidden_cards.get_mut(index) {
            *slot = role;
        } else {
            debug_assert!(false, "Hidden index {} out of range", index);
        }
    }

    pub fn swap_players(&mut self, player_a: &str, player_b: &str) -> Result<()> {
        if player_a == player_b {
            return Ok(()); // no-op
        }

        let mut player = self
            .players
            .remove(player_a)
            .ok_or_else(|| anyhow!(format!("Player {} not found", player_a)))?;

        let target_player = self
            .players
            .get_mut(player_b)
            .ok_or_else(|| anyhow!(format!("Player {} not found", player_b)))?;

        let current_cole = player.effective_role_card(None);

        player.role_card_stack.push(RoleTransaction::new_hidden(
            target_player.effective_role_card(None),
            "Swapped",
        ));

        target_player
            .role_card_stack
            .push(RoleTransaction::new_hidden(current_cole, "Swapped"));

        self.players.insert(player_a.to_string(), player);

        Ok(())
    }

    pub fn rob_player(&mut self, player_id: &str, target_player_id: &str) -> Result<()> {
        if player_id == target_player_id {
            return Ok(()); // no-op
        }

        let mut player = self
            .players
            .remove(player_id)
            .ok_or_else(|| anyhow!(format!("Player {} not found", player_id)))?;

        let target_player = self
            .players
            .get_mut(target_player_id)
            .ok_or_else(|| anyhow!(format!("Player {} not found", target_player_id)))?;

        let current_cole = player.effective_role_card(None);

        player.role_card_stack.push(RoleTransaction::new_shown(
            target_player.effective_role_card(None),
            "Taken card",
        ));

        target_player
            .role_card_stack
            .push(RoleTransaction::new_hidden(
                current_cole,
                "The Taken's card",
            ));

        self.players.insert(player_id.to_string(), player);

        Ok(())
    }

    pub fn swap_player_with_hidden(
        &mut self,
        player_id: &str,
        hidden_position: usize,
        visibility: RoleTransactionVisibilty,
        reason: &str,
    ) -> Result<()> {
        let player = self
            .players
            .get_mut(player_id)
            .ok_or(anyhow!(format!("Player {} not found", player_id)))?;

        let hidden_card = self
            .hidden_cards
            .get_mut(hidden_position)
            .ok_or(anyhow!(format!(
                "Hidden position {} not found",
                hidden_position
            )))?;

        let player_card = player.effective_role_card(None);
        player.role_card_stack.push(RoleTransaction {
            card: hidden_card.clone(),
            reason: reason.to_string(),
            visibility,
        });
        *hidden_card = player_card;

        Ok(())
    }

    pub fn get_sabotage_candidates<'a, I>(&self, role_ids: I) -> Vec<Arc<RoleCard>>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let role_ids: Vec<String> = role_ids
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect();

        self.all_roles
            .iter()
            .filter(|card| card.enabled && role_ids.iter().any(|rid| rid == &card.role_id))
            .cloned()
            .collect()
    }

    pub fn set_sabotage_inputs(
        &mut self,
        user_id: &str,
        workflow_id: &str,
        inputs: (PlayerRole, HashMap<String, Value>),
    ) {
        self.sabotaged_inputs
            .insert((user_id.to_string(), workflow_id.to_string()), inputs);
    }

    pub fn get_sabotage_inputs(
        &self,
        user_id: &str,
        workflow_id: &str,
    ) -> Option<(PlayerRole, HashMap<String, Value>)> {
        self.sabotaged_inputs
            .get(&(user_id.to_string(), workflow_id.to_string()))
            .cloned()
    }

    pub fn get_user_effective_role(
        &self,
        user_id: &str,
        filter: Option<RoleTransactionVisibilty>,
    ) -> Result<Arc<RoleCard>> {
        if let Some(player) = self.players.get(user_id) {
            return Ok(player.effective_role_card(filter));
        }

        Err(anyhow!(format!("Unable to find player with id {user_id}")))
    }

    pub fn clear_sabotage_inputs(&mut self, user_id: &str, workflow_id: &str) {
        self.sabotaged_inputs
            .remove(&(user_id.to_string(), workflow_id.to_string()));
    }

    fn enter_ability_for_current_stage(&mut self, effects: &mut Vec<GameEffect>) {
        let Some(stage) = self.current_stage.clone() else {
            return;
        };

        self.stage = GameStage::Ability {
            role_id: stage.role.role_id.clone(),
        };

        for uid in stage.player_ids.clone() {
            let Some(player) = self.players.get(&uid) else {
                continue;
            };
            if player.instance_id == "bot" {
                continue;
            }
            let hook_opt = stage.role.on_night_start.clone();
            let player_opt = self.players.get(&uid).cloned();

            if let (Some(hook), Some(player)) = (hook_opt, player_opt) {
                if let Some(WorkflowStart {
                    mut inputs,
                    workflow_id,
                    context,
                }) = hook(self, &player)
                {
                    let workflow_key = canonical_workflow_id(&workflow_id).to_string();

                    if let Some((_who, sabotage_inputs)) =
                        self.get_sabotage_inputs(&uid, &workflow_key)
                    {
                        for (k, v) in sabotage_inputs {
                            inputs.insert(k, v);
                        }
                        self.clear_sabotage_inputs(&uid, &workflow_key);
                    }

                    effects.push(GameEffect::InjectWorkflow(StartWorkflowFromInstanceArgs {
                        instance_id: player.instance_id,
                        workflow_definition_id: workflow_id,
                        inputs,
                        context,
                        attach_as: Some("game".into()),
                        inject: Some(InjectWorkflowPlace::Tile {
                            name: "games.midnight_tile".into(),
                            on_complete: None,
                        }),
                    }));
                }
            }
        }
    }

    pub fn pick_random_role<'a>(&'a self, roles: &'a [Arc<RoleCard>]) -> Option<&'a Arc<RoleCard>> {
        let mut rng = ChaCha12Rng::from_os_rng();
        roles.choose(&mut rng)
    }

    pub fn get_player_by_role(
        &self,
        role_id: &str,
        filter: Option<RoleTransactionVisibilty>,
    ) -> Result<Player> {
        Ok(self
            .players
            .values()
            .find(|p| p.effective_role_card(filter.clone()).role_id.as_str() == role_id)
            .ok_or(anyhow!(format!(
                "Unable to find player with role {role_id}"
            )))?
            .clone())
    }
}

