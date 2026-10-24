use std::collections::HashMap;

use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;

use crate::{OptionResource, QuestionResource, TriviaResource, scoring};
use game_framework::{GameEffect, GameLogic};

const DEFAULT_REVEAL_MS: i64 = 5_000;
const DB_SCORE_ACTION: &str = "db_score.added";
const SCORE_REASON: &str = "TRIVIA_SCORE";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriviaStage {
    Lobby,
    Question,
    Reveal,
    Finished,
}

#[derive(Clone, Debug)]
pub struct TriviaInitArgs {
    pub question_duration_ms: i64,
    pub reveal_duration_ms: i64,
}

impl Default for TriviaInitArgs {
    fn default() -> Self {
        Self {
            question_duration_ms: scoring::DEFAULT_TOTAL_MS,
            reveal_duration_ms: DEFAULT_REVEAL_MS,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriviaAction {
    View,
    Join {
        name: String,
        avatar: String,
        instance_id: String,
    },
    SetTrivia {
        trivia: TriviaResource,
    },
    Begin,
    SubmitAnswer {
        answer_id: String,
    },
    Reveal,
    Advance,
    Leave,
}

#[derive(Clone, Debug)]
struct TriviaPlayer {
    id: String,
    name: String,
    avatar_url: String,
    instance_id: String,
    total_score: i64,
    answered: bool,
    answer_id: Option<String>,
    last_points: i64,
    last_correct: bool,
    join_index: usize,
    audit: Vec<TriviaAnswerAuditView>,
    score_recorded: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TriviaPlayerView {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub answered: bool,
    pub total_score: i64,
    pub rank: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TriviaAnswerRevealView {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub answer_label: String,
    pub points_awarded: i64,
    pub is_correct: bool,
    pub is_me: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TriviaAnswerAuditView {
    pub question: String,
    pub user_answer: Option<String>,
    pub correct_answer: String,
    pub no_answer: bool,
    pub is_correct: bool,
    pub points_awarded: i64,
    pub options: Vec<OptionResource>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TriviaGameView {
    pub stage: TriviaStage,
    pub trivia: Option<TriviaResource>,
    pub current_question: Option<QuestionResource>,
    pub question_number: usize,
    pub total_questions: usize,
    pub question_ends_at: Option<String>,
    pub reveal_ends_at: Option<String>,
    pub answered_count: usize,
    pub players: Vec<TriviaPlayerView>,
    pub reveal_answers: Vec<TriviaAnswerRevealView>,
    #[serde(default)]
    pub correct_answer_label: String,
    pub viewer_joined: bool,
    pub viewer_answered: bool,
    pub viewer_score: i64,
    pub viewer_audit: Vec<TriviaAnswerAuditView>,
    pub leaderboard: Vec<TriviaPlayerView>,
}

#[derive(Clone, Debug)]
pub struct TriviaGameState {
    stage: TriviaStage,
    trivia: Option<TriviaResource>,
    current_question_index: usize,
    question_ends_at: Option<String>,
    reveal_ends_at: Option<String>,
    players: HashMap<String, TriviaPlayer>,
    join_counter: usize,
    question_duration_ms: i64,
    reveal_duration_ms: i64,
}

impl Default for TriviaGameState {
    fn default() -> Self {
        Self {
            stage: TriviaStage::Lobby,
            trivia: None,
            current_question_index: 0,
            question_ends_at: None,
            reveal_ends_at: None,
            players: HashMap::new(),
            join_counter: 0,
            question_duration_ms: scoring::DEFAULT_TOTAL_MS,
            reveal_duration_ms: DEFAULT_REVEAL_MS,
        }
    }
}

impl TriviaGameState {
    fn current_question(&self) -> Option<QuestionResource> {
        self.trivia
            .as_ref()
            .and_then(|t| t.questions.get(self.current_question_index).cloned())
    }

    fn reset_player_round_state(&mut self) {
        for player in self.players.values_mut() {
            player.answered = false;
            player.answer_id = None;
            player.last_points = 0;
            player.last_correct = false;
        }
    }

    fn start_question(&mut self, index: usize) {
        self.current_question_index = index;
        self.stage = TriviaStage::Question;
        self.question_ends_at = Some(scoring::question_ends_at_ms(
            self.question_duration_ms,
            Utc::now(),
        ));
        self.reveal_ends_at = None;
        self.reset_player_round_state();
    }

    fn reveal_answers(&mut self) {
        let question = self.current_question();
        let (correct_value, correct_label, option_labels) =
            if let Some(question) = question.as_ref() {
                let correct_value = question.correct_value.clone();
                let correct_label = question
                    .options
                    .iter()
                    .find(|opt| opt.value == correct_value)
                    .map(|opt| opt.label.clone())
                    .unwrap_or_else(|| correct_value.clone());
                let option_labels: HashMap<String, String> = question
                    .options
                    .iter()
                    .map(|opt| (opt.value.clone(), opt.label.clone()))
                    .collect();
                (
                    Some(correct_value),
                    Some(correct_label),
                    Some(option_labels),
                )
            } else {
                (None, None, None)
            };
        self.stage = TriviaStage::Reveal;
        self.question_ends_at = None;
        self.reveal_ends_at = Some(scoring::question_ends_at_ms(
            self.reveal_duration_ms,
            Utc::now(),
        ));
        for player in self.players.values_mut() {
            if !player.answered {
                player.answer_id = None;
                player.last_points = 0;
                player.last_correct = false;
            }
            if let (Some(question), Some(correct_value), Some(correct_label), Some(option_labels)) = (
                question.as_ref(),
                correct_value.as_ref(),
                correct_label.as_ref(),
                option_labels.as_ref(),
            ) {
                let no_answer = player.answer_id.is_none();
                let user_answer = player
                    .answer_id
                    .as_deref()
                    .and_then(|answer_id| option_labels.get(answer_id).cloned());
                let correct_answer = correct_label.clone();
                let options: Vec<OptionResource> = question
                    .options
                    .iter()
                    .map(|opt| {
                        let mut next = opt.clone();
                        let selected = player
                            .answer_id
                            .as_deref()
                            .map(|id| id == opt.value)
                            .unwrap_or(false);
                        let status = if opt.value == *correct_value {
                            "correct"
                        } else if selected {
                            "wrong"
                        } else {
                            "neutral"
                        };
                        next.selected = Some(selected);
                        next.status = Some(status.into());
                        next
                    })
                    .collect();
                player.audit.push(TriviaAnswerAuditView {
                    question: question.text.clone(),
                    user_answer,
                    correct_answer,
                    no_answer,
                    is_correct: player.last_correct,
                    points_awarded: player.last_points,
                    options,
                });
            }
        }
    }

    fn advance_question(&mut self) -> Vec<GameEffect> {
        let total = self.trivia.as_ref().map(|t| t.questions.len()).unwrap_or(0);
        let next_idx = self.current_question_index + 1;
        if next_idx >= total {
            self.stage = TriviaStage::Finished;
            self.question_ends_at = None;
            self.reveal_ends_at = None;
            let mut effects = Vec::new();
            let trivia_id = self.trivia.as_ref().map(|t| t.id.clone());
            for player in self.players.values_mut() {
                if player.score_recorded {
                    continue;
                }
                player.score_recorded = true;
                let mut inputs = HashMap::new();
                inputs.insert("amount".into(), json!(player.total_score));
                inputs.insert("reason".into(), json!(SCORE_REASON));
                inputs.insert("reference_id".into(), json!(player.instance_id.clone()));
                if let Some(trivia_id) = trivia_id.as_ref() {
                    inputs.insert("meta".into(), json!({ "trivia_id": trivia_id }));
                }
                effects.push(GameEffect::RunServerAction {
                    instance_id: player.instance_id.clone(),
                    action_id: DB_SCORE_ACTION.into(),
                    inputs,
                });
            }
            return effects;
        }
        self.start_question(next_idx);
        vec![]
    }

    fn correct_answer_label(&self, question: &QuestionResource) -> String {
        question
            .options
            .iter()
            .find(|opt| opt.value == question.correct_value)
            .map(|opt| opt.label.clone())
            .unwrap_or_default()
    }

    fn answer_label(&self, question: &QuestionResource, answer_id: Option<&str>) -> String {
        let Some(answer_id) = answer_id else {
            return "No answer".to_string();
        };
        question
            .options
            .iter()
            .find(|opt| opt.value == answer_id)
            .map(|opt| opt.label.clone())
            .unwrap_or_else(|| "Invalid answer".to_string())
    }
}

impl GameLogic for TriviaGameState {
    type Action = TriviaAction;
    type View = TriviaGameView;
    type Effect = GameEffect;
    type InitArgs = TriviaInitArgs;

    fn get_player_ids(&self) -> Vec<String> {
        self.players.keys().cloned().collect()
    }

    fn get_instance_id_for_player_id(&self, player_id: &str) -> Option<String> {
        self.players.get(player_id).map(|p| p.instance_id.clone())
    }

    fn new(args: Self::InitArgs) -> Self {
        Self {
            stage: TriviaStage::Lobby,
            question_duration_ms: args.question_duration_ms,
            reveal_duration_ms: args.reveal_duration_ms,
            ..Default::default()
        }
    }

    #[instrument(skip(self))]
    fn update(&mut self, player_id: &str, action: Self::Action) -> Result<Vec<Self::Effect>> {
        match action {
            TriviaAction::View => {}
            TriviaAction::Join {
                name,
                avatar,
                instance_id,
            } => {
                if let Some(existing) = self.players.get_mut(player_id) {
                    existing.instance_id = instance_id;
                    existing.name = name;
                    existing.avatar_url = avatar;
                    return Ok(vec![]);
                }
                let player = TriviaPlayer {
                    id: player_id.to_string(),
                    name,
                    avatar_url: avatar,
                    instance_id,
                    total_score: 0,
                    answered: false,
                    answer_id: None,
                    last_points: 0,
                    last_correct: false,
                    join_index: self.join_counter,
                    audit: Vec::new(),
                    score_recorded: false,
                };
                self.join_counter += 1;
                self.players.insert(player_id.to_string(), player);
            }
            TriviaAction::SetTrivia { trivia } => {
                if trivia.questions.is_empty() {
                    return Err(anyhow!("Trivia has no questions"));
                }
                // Only reset if we're in the lobby — don't wipe an in-progress game
                if self.stage == TriviaStage::Lobby || self.stage == TriviaStage::Finished {
                    self.trivia = Some(trivia);
                    self.stage = TriviaStage::Lobby;
                    self.current_question_index = 0;
                    self.question_ends_at = None;
                    self.reveal_ends_at = None;
                    for player in self.players.values_mut() {
                        player.total_score = 0;
                        player.answered = false;
                        player.answer_id = None;
                        player.last_points = 0;
                        player.last_correct = false;
                        player.audit.clear();
                        player.score_recorded = false;
                    }
                }
            }
            TriviaAction::Begin => {
                if self.trivia.is_none() {
                    return Err(anyhow!("Trivia not selected"));
                }
                if self.stage == TriviaStage::Question {
                    return Ok(vec![]);
                }
                self.start_question(0);
            }
            TriviaAction::SubmitAnswer { answer_id } => {
                if self.stage != TriviaStage::Question {
                    return Ok(vec![]);
                }
                let question = self
                    .current_question()
                    .ok_or_else(|| anyhow!("No active question"))?;
                let player = self
                    .players
                    .get_mut(player_id)
                    .ok_or_else(|| anyhow!("Player not joined"))?;
                if player.answered {
                    return Ok(vec![]);
                }
                let is_valid = question.options.iter().any(|opt| opt.value == answer_id);
                if !is_valid {
                    return Err(anyhow!("Invalid answer"));
                }
                let is_correct = answer_id == question.correct_value;
                let mut points_awarded = 0;
                if is_correct {
                    let ends_at = self.question_ends_at.as_deref();
                    let (points, _, _, _) = scoring::compute_time_scaled_points(
                        question.points,
                        self.question_duration_ms,
                        ends_at,
                        Utc::now(),
                    );
                    points_awarded = points;
                    player.total_score += points_awarded;
                }
                player.answered = true;
                player.answer_id = Some(answer_id);
                player.last_points = points_awarded;
                player.last_correct = is_correct;

                if self.players.values().all(|p| p.answered) {
                    self.reveal_answers();
                }
            }
            TriviaAction::Reveal => {
                if self.stage != TriviaStage::Question {
                    return Ok(vec![]);
                }
                self.reveal_answers();
            }
            TriviaAction::Advance => {
                if self.stage != TriviaStage::Reveal {
                    return Ok(vec![]);
                }
                return Ok(self.advance_question());
            }
            TriviaAction::Leave => {
                self.players.remove(player_id);
            }
        }

        Ok(vec![])
    }

    fn view_for(&self, player_id: &str) -> Self::View {
        let question = self.current_question();
        let total_questions = self.trivia.as_ref().map(|t| t.questions.len()).unwrap_or(0);
        let question_number = if question.is_some() && total_questions > 0 {
            self.current_question_index + 1
        } else {
            0
        };

        let viewer = self.players.get(player_id);
        let viewer_joined = viewer.is_some();
        let viewer_answered = viewer.map(|p| p.answered).unwrap_or(false);
        let viewer_score = viewer.map(|p| p.total_score).unwrap_or(0);
        let viewer_audit = viewer.map(|p| p.audit.clone()).unwrap_or_default();

        let mut players: Vec<_> = self
            .players
            .values()
            .map(|p| TriviaPlayerView {
                id: p.id.clone(),
                name: p.name.clone(),
                avatar_url: p.avatar_url.clone(),
                answered: p.answered,
                total_score: p.total_score,
                rank: None,
            })
            .collect();
        players.sort_by_key(|p| {
            self.players
                .get(&p.id)
                .map(|player| player.join_index)
                .unwrap_or(0)
        });

        let answered_count = players.iter().filter(|p| p.answered).count();

        let mut reveal_answers = Vec::new();
        let mut correct_answer_label = String::new();
        if let (Some(question), TriviaStage::Reveal | TriviaStage::Finished) =
            (question.as_ref(), &self.stage)
        {
            correct_answer_label = self.correct_answer_label(question);
            let mut ordered: Vec<_> = self.players.values().collect();
            ordered.sort_by_key(|p| p.join_index);
            for player in ordered {
                reveal_answers.push(TriviaAnswerRevealView {
                    id: player.id.clone(),
                    name: player.name.clone(),
                    avatar_url: player.avatar_url.clone(),
                    answer_label: self.answer_label(question, player.answer_id.as_deref()),
                    points_awarded: player.last_points,
                    is_correct: player.last_correct,
                    is_me: player.id == player_id,
                });
            }
        }

        let mut leaderboard = players.clone();
        leaderboard.sort_by(|a, b| b.total_score.cmp(&a.total_score));
        for (idx, entry) in leaderboard.iter_mut().enumerate() {
            entry.rank = Some(idx + 1);
        }

        TriviaGameView {
            stage: self.stage.clone(),
            trivia: self.trivia.clone(),
            current_question: question,
            question_number,
            total_questions,
            question_ends_at: self.question_ends_at.clone(),
            reveal_ends_at: self.reveal_ends_at.clone(),
            answered_count,
            players,
            reveal_answers,
            correct_answer_label,
            viewer_joined,
            viewer_answered,
            viewer_score,
            viewer_audit,
            leaderboard,
        }
    }

    fn tick(&mut self) -> Vec<Self::Action> {
        let now = Utc::now();
        match self.stage {
            TriviaStage::Question => {
                let should_reveal = self
                    .question_ends_at
                    .as_deref()
                    .and_then(scoring::parse_rfc3339_utc)
                    .map(|end| now >= end)
                    .unwrap_or(false);
                if should_reveal {
                    return vec![TriviaAction::Reveal];
                }
            }
            TriviaStage::Reveal => {
                let should_advance = self
                    .reveal_ends_at
                    .as_deref()
                    .and_then(scoring::parse_rfc3339_utc)
                    .map(|end| now >= end)
                    .unwrap_or(false);
                if should_advance {
                    return vec![TriviaAction::Advance];
                }
            }
            _ => {}
        }
        vec![]
    }
}

#[derive(Clone, Debug)]
pub struct TriviaMultiplayerActionIds {
    pub view: String,
    pub join: String,
    pub start: String,
    pub answer: String,
}

pub async fn register_routes(
    engine: std::sync::Arc<game_framework::engine::GameEngine<TriviaGameState>>,
    prefix: &str,
) -> Result<TriviaMultiplayerActionIds> {
    use tawks_bot::{full_avatar_url, types::UserResource};

    let view = engine
        .clone()
        .register_route(&format!("{}.view", prefix), |_| Ok(TriviaAction::View))
        .await?;

    let join = engine
        .clone()
        .register_route(&format!("{}.join", prefix), |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            Ok(TriviaAction::Join {
                name: user.display_name.clone(),
                avatar: full_avatar_url(&user),
                instance_id: ctx.workflow.instance_id,
            })
        })
        .await?;

    let start = engine
        .clone()
        .register_route(&format!("{}.start", prefix), |ctx| {
            let _ = ctx;
            Ok(TriviaAction::Begin)
        })
        .await?;

    let answer = engine
        .clone()
        .register_route(&format!("{}.answer", prefix), |ctx| {
            let answer_id = ctx
                .get_input_as::<String>("answer_id")
                .map_err(|_| anyhow!("Missing answer_id"))?;
            Ok(TriviaAction::SubmitAnswer { answer_id })
        })
        .await?;

    Ok(TriviaMultiplayerActionIds {
        view,
        join,
        start,
        answer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OptionResource;
    use chrono::{Duration, Utc};

    fn sample_trivia() -> TriviaResource {
        TriviaResource {
            id: "trivia-1".into(),
            topic: "space".into(),
            title: "Space Trivia".into(),
            description: "Test your space knowledge".into(),
            top_players: vec![],
            questions: vec![
                QuestionResource {
                    id: "q1".into(),
                    text: "Closest planet to the sun?".into(),
                    points: 100,
                    correct_value: "a1".into(),
                    options: vec![
                        OptionResource {
                            value: "a1".into(),
                            label: "Mercury".into(),
                            icon: "a".into(),
                            status: None,
                            selected: None,
                        },
                        OptionResource {
                            value: "a2".into(),
                            label: "Venus".into(),
                            icon: "b".into(),
                            status: None,
                            selected: None,
                        },
                    ],
                },
                QuestionResource {
                    id: "q2".into(),
                    text: "Largest planet?".into(),
                    points: 100,
                    correct_value: "b2".into(),
                    options: vec![
                        OptionResource {
                            value: "b1".into(),
                            label: "Saturn".into(),
                            icon: "a".into(),
                            status: None,
                            selected: None,
                        },
                        OptionResource {
                            value: "b2".into(),
                            label: "Jupiter".into(),
                            icon: "b".into(),
                            status: None,
                            selected: None,
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn start_and_answer_updates_view() {
        let mut state = TriviaGameState::new(TriviaInitArgs::default());
        let trivia = sample_trivia();

        state
            .update(
                "user-1",
                TriviaAction::Join {
                    name: "Ada".into(),
                    avatar: "https://example.com/a.png".into(),
                    instance_id: "instance-1".into(),
                },
            )
            .unwrap();

        state
            .update("user-1", TriviaAction::SetTrivia { trivia })
            .unwrap();
        state.update("user-1", TriviaAction::Begin).unwrap();

        assert_eq!(state.stage, TriviaStage::Question);

        state
            .update(
                "user-1",
                TriviaAction::SubmitAnswer {
                    answer_id: "a1".into(),
                },
            )
            .unwrap();

        let view = state.view_for("user-1");
        assert!(view.viewer_joined);
        assert!(view.viewer_answered);
        assert_eq!(view.answered_count, 1);
        assert!(view.viewer_score > 0);
        assert_eq!(view.question_number, 1);
    }

    #[test]
    fn reveal_when_all_players_answer() {
        let mut state = TriviaGameState::new(TriviaInitArgs::default());
        let trivia = sample_trivia();

        state
            .update(
                "user-1",
                TriviaAction::Join {
                    name: "Ada".into(),
                    avatar: "https://example.com/a.png".into(),
                    instance_id: "instance-1".into(),
                },
            )
            .unwrap();
        state
            .update(
                "user-2",
                TriviaAction::Join {
                    name: "Bob".into(),
                    avatar: "https://example.com/b.png".into(),
                    instance_id: "instance-2".into(),
                },
            )
            .unwrap();

        state
            .update("user-1", TriviaAction::SetTrivia { trivia })
            .unwrap();
        state.update("user-1", TriviaAction::Begin).unwrap();

        state
            .update(
                "user-1",
                TriviaAction::SubmitAnswer {
                    answer_id: "a1".into(),
                },
            )
            .unwrap();
        assert_eq!(state.stage, TriviaStage::Question);

        state
            .update(
                "user-2",
                TriviaAction::SubmitAnswer {
                    answer_id: "a1".into(),
                },
            )
            .unwrap();

        assert_eq!(state.stage, TriviaStage::Reveal);
        assert!(state.reveal_ends_at.is_some());
        assert!(state.question_ends_at.is_none());
    }

    #[test]
    fn invalid_answer_rejected() {
        let mut state = TriviaGameState::new(TriviaInitArgs::default());
        let trivia = sample_trivia();

        state
            .update(
                "user-1",
                TriviaAction::Join {
                    name: "Ada".into(),
                    avatar: "https://example.com/a.png".into(),
                    instance_id: "instance-1".into(),
                },
            )
            .unwrap();

        state
            .update("user-1", TriviaAction::SetTrivia { trivia })
            .unwrap();
        state.update("user-1", TriviaAction::Begin).unwrap();

        let err = state
            .update(
                "user-1",
                TriviaAction::SubmitAnswer {
                    answer_id: "nope".into(),
                },
            )
            .unwrap_err();
        assert!(err.to_string().contains("Invalid answer"));
    }

    #[test]
    fn tick_transitions_question_reveal_and_finished() {
        let mut state = TriviaGameState::new(TriviaInitArgs::default());
        let trivia = sample_trivia();

        state
            .update(
                "user-1",
                TriviaAction::Join {
                    name: "Ada".into(),
                    avatar: "https://example.com/a.png".into(),
                    instance_id: "instance-1".into(),
                },
            )
            .unwrap();

        state
            .update("user-1", TriviaAction::SetTrivia { trivia })
            .unwrap();
        state.update("user-1", TriviaAction::Begin).unwrap();

        state.question_ends_at = Some((Utc::now() - Duration::milliseconds(1)).to_rfc3339());
        let actions = state.tick();
        assert!(matches!(actions.as_slice(), [TriviaAction::Reveal]));
        if let Some(action) = actions.into_iter().next() {
            state.update("system", action).unwrap();
        }
        assert_eq!(state.stage, TriviaStage::Reveal);

        state.reveal_ends_at = Some((Utc::now() - Duration::milliseconds(1)).to_rfc3339());
        let actions = state.tick();
        assert!(matches!(actions.as_slice(), [TriviaAction::Advance]));
        if let Some(action) = actions.into_iter().next() {
            state.update("system", action).unwrap();
        }
        assert_eq!(state.stage, TriviaStage::Question);

        state.question_ends_at = Some((Utc::now() - Duration::milliseconds(1)).to_rfc3339());
        let actions = state.tick();
        if let Some(action) = actions.into_iter().next() {
            state.update("system", action).unwrap();
        }

        state.reveal_ends_at = Some((Utc::now() - Duration::milliseconds(1)).to_rfc3339());
        let actions = state.tick();
        if let Some(action) = actions.into_iter().next() {
            state.update("system", action).unwrap();
        }
        assert_eq!(state.stage, TriviaStage::Finished);
    }
}
