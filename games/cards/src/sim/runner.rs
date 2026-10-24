use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use game_framework::GameLogic;
use rayon::prelude::*;

use crate::action::CardGameAction;
use crate::state::{CardGameInitArgs, CardGameState, PendingAction};
use crate::types::{CardGameStage, CardKind, CombatPhase, GameFormat};

use super::action_gen::legal_actions_for_player;
use super::agent::{Agent, ArchetypeAgent, profile_for_deck};
use super::card_metrics::CardMetricsCollector;
use super::config::{
    DraftMatrixConfig, GauntletConfig, MatchConfig, MatrixConfig, OperatorCombo,
    OperatorMatrixConfig, ParticipantConfig,
};
use super::degeneracy::detect_degeneracy;
use super::mcts::{MctsAgent, MctsConfig};
use super::patch::{BalancePatch, load_balance_patch_from_path};
use super::report::{
    ActionTraceEntry, BalanceMetrics, ColorMetrics, DeckMetrics, DraftColorMetrics,
    DraftMatrixReport, MatchReport, MatchupMetrics, MatrixReport, OperatorDiagnostics,
    OperatorMatchupMetrics, OperatorMatrixReport, OperatorMetrics, PlayerSummary, RoundSnapshot,
    SeriesReport, TerminationCount, TerminationReason, action_label,
};

pub fn run_match(config: &MatchConfig) -> MatchReport {
    run_match_with_patch(config, None)
}

pub fn run_match_with_patch(
    config: &MatchConfig,
    patch_override: Option<&BalancePatch>,
) -> MatchReport {
    match run_match_inner(config, patch_override) {
        Ok(report) => report,
        Err(error) => MatchReport {
            seed: config.seed,
            deck_a: config.participants[0].deck_id.clone(),
            deck_b: config.participants[1].deck_id.clone(),
            agent_a: config.participants[0].agent.clone(),
            agent_b: config.participants[1].agent.clone(),
            winner_player_id: None,
            winner_deck_id: None,
            drawn: false,
            rounds: 0,
            actions_taken: 0,
            termination_reason: TerminationReason::UpdateError,
            error: Some(error.to_string()),
            final_state: Vec::new(),
            snapshots: Vec::new(),
            trace: Vec::new(),
            degeneracy_flags: Vec::new(),
            card_metrics: Vec::new(),
        },
    }
}

fn sim_debug_enabled() -> bool {
    std::env::var("CARDGAME_SIM_DEBUG")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn sim_debug(message: impl AsRef<str>) {
    if sim_debug_enabled() {
        eprintln!("[sim] {}", message.as_ref());
    }
}

fn operator_matrix_progress_enabled() -> bool {
    std::env::var("CARDGAME_OPERATOR_MATRIX_PROGRESS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn operator_matrix_progress(message: impl AsRef<str>) {
    if operator_matrix_progress_enabled() {
        eprintln!("[operator-matrix] {}", message.as_ref());
    }
}

pub fn run_series(config: &GauntletConfig) -> SeriesReport {
    let mut results = Vec::new();

    for index in 0..config.matches {
        let mut match_config = config.base_match.clone();
        match_config.seed = config.base_match.seed + index as u64;

        if config.alternate_seats && index % 2 == 1 {
            match_config.participants.swap(0, 1);
        }

        let report = run_match(&match_config);
        results.push(report);
    }

    build_series_report(config, results)
}

pub fn run_matrix(config: &MatrixConfig) -> MatrixReport {
    run_matrix_with_patch(config, None)
}

pub fn run_matrix_with_patch(
    config: &MatrixConfig,
    patch_override: Option<&BalancePatch>,
) -> MatrixReport {
    let mut builder = MatrixReportBuilder::new(config);

    for (left_index, deck_a) in config.decks.iter().enumerate() {
        let right_start = if config.include_mirrors {
            left_index
        } else {
            left_index + 1
        };

        for deck_b in config.decks.iter().skip(right_start) {
            let gauntlet = GauntletConfig {
                base_match: MatchConfig {
                    participants: vec![
                        ParticipantConfig::new(
                            "p1",
                            deck_a.clone(),
                            profile_for_deck(deck_a),
                            config.base_match.participants[0].operator.clone(),
                            config.base_match.participants[0].uses_mcts,
                        ),
                        ParticipantConfig::new(
                            "p2",
                            deck_b.clone(),
                            profile_for_deck(deck_b),
                            config.base_match.participants[1].operator.clone(),
                            config.base_match.participants[1].uses_mcts,
                        ),
                    ],
                    patch_path: config.base_match.patch_path.clone(),
                    mcts_iterations: config.base_match.mcts_iterations,
                    mcts_rollout_depth: config.base_match.mcts_rollout_depth,
                    seed: config.base_match.seed,
                    format: config.base_match.format.clone(),
                    max_turns: config.base_match.max_turns,
                    max_actions: config.base_match.max_actions,
                    repeat_state_limit: config.base_match.repeat_state_limit,
                    include_trace: config.base_match.include_trace,
                    enable_sideboard: config.base_match.enable_sideboard,
                },
                matches: config.matches_per_pair,
                alternate_seats: config.alternate_seats,
            };

            let mut accumulator = MatchupAccumulator::new(deck_a.clone(), deck_b.clone());
            let reports: Vec<MatchReport> = (0..gauntlet.matches)
                .into_par_iter()
                .map(|index| {
                    let mut match_config = gauntlet.base_match.clone();
                    match_config.seed = gauntlet.base_match.seed + index as u64;

                    if gauntlet.alternate_seats && index % 2 == 1 {
                        match_config.participants.swap(0, 1);
                    }

                    run_match_with_patch(&match_config, patch_override)
                })
                .collect();
            for report in &reports {
                accumulator.record(report);
            }

            builder.push_metrics(accumulator.finish());
        }
    }

    builder.finish()
}

pub fn build_series_report(config: &GauntletConfig, results: Vec<MatchReport>) -> SeriesReport {
    let mut wins_by_deck: BTreeMap<String, usize> = BTreeMap::new();

    for report in &results {
        if let Some(deck_id) = &report.winner_deck_id {
            *wins_by_deck.entry(deck_id.clone()).or_default() += 1;
        }
    }

    SeriesReport {
        matches: config.matches,
        deck_a: config.base_match.participants[0].deck_id.clone(),
        deck_b: config.base_match.participants[1].deck_id.clone(),
        wins_by_deck: wins_by_deck.into_iter().collect(),
        results,
    }
}

pub fn build_matrix_report(
    config: &MatrixConfig,
    matchup_series: Vec<SeriesReport>,
) -> MatrixReport {
    let mut builder = MatrixReportBuilder::new(config);

    for series in matchup_series {
        builder.push_metrics(summarize_matchup(&series));
    }

    builder.finish()
}

pub fn build_matrix_report_from_matchups(
    config: &MatrixConfig,
    matchup_metrics: Vec<MatchupMetrics>,
) -> MatrixReport {
    let mut builder = MatrixReportBuilder::new(config);

    for metrics in matchup_metrics {
        builder.push_metrics(metrics);
    }

    builder.finish()
}

/// Run a match using pre-built card lists instead of deck IDs.
/// The `deck_a_label` and `deck_b_label` are used for reporting (e.g. "draft-1-p1").
pub fn run_match_with_decks(
    config: &MatchConfig,
    deck_a_cards: Vec<crate::card::Card>,
    deck_b_cards: Vec<crate::card::Card>,
    patch_override: Option<&BalancePatch>,
) -> MatchReport {
    match run_match_with_decks_inner(config, deck_a_cards, deck_b_cards, patch_override) {
        Ok(report) => report,
        Err(error) => MatchReport {
            seed: config.seed,
            deck_a: config.participants[0].deck_id.clone(),
            deck_b: config.participants[1].deck_id.clone(),
            agent_a: config.participants[0].agent.clone(),
            agent_b: config.participants[1].agent.clone(),
            winner_player_id: None,
            winner_deck_id: None,
            drawn: false,
            rounds: 0,
            actions_taken: 0,
            termination_reason: TerminationReason::UpdateError,
            error: Some(error.to_string()),
            final_state: Vec::new(),
            snapshots: Vec::new(),
            trace: Vec::new(),
            degeneracy_flags: Vec::new(),
            card_metrics: Vec::new(),
        },
    }
}

fn run_match_with_decks_inner(
    config: &MatchConfig,
    deck_a_cards: Vec<crate::card::Card>,
    deck_b_cards: Vec<crate::card::Card>,
    patch_override: Option<&BalancePatch>,
) -> Result<MatchReport> {
    let patch = patch_override;
    let state = initialize_state_with_decks(config, deck_a_cards, deck_b_cards, patch)?;
    run_game_loop(config, state)
}

/// Run a draft matrix: generate N draft pools, auto-draft, play matches, report per-color balance.
pub fn run_draft_matrix(config: &DraftMatrixConfig) -> DraftMatrixReport {
    run_draft_matrix_with_patch(config, None)
}

pub fn run_draft_matrix_with_patch(
    config: &DraftMatrixConfig,
    patch_override: Option<&BalancePatch>,
) -> DraftMatrixReport {
    use crate::helpers::get_card_color_identity;
    use crate::sim::draft::run_sim_draft;
    use crate::types::ManaType;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let colors = ["neon", "glitch", "chrome", "slag", "bio"];
    let mut color_wins: BTreeMap<String, usize> = BTreeMap::new();
    let mut color_losses: BTreeMap<String, usize> = BTreeMap::new();
    let mut color_cards: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_matches = 0usize;
    let mut all_terminations: BTreeMap<TerminationReason, usize> = BTreeMap::new();
    let mut total_rounds = 0usize;
    let mut total_actions = 0usize;
    let mut finished_matches = 0usize;
    let mut decisive_matches = 0usize;
    let mut drawn_matches = 0usize;
    let mut first_player_wins = 0usize;
    let mut first_player_decisive = 0usize;

    // Phase 1 (serial): generate all drafts, count colors, build work items.
    struct DraftWorkItem {
        match_config: MatchConfig,
        deck_a: Vec<crate::card::Card>,
        deck_b: Vec<crate::card::Card>,
        color_a: String,
        color_b: String,
        match_index: usize,
    }
    let mut work_items: Vec<DraftWorkItem> =
        Vec::with_capacity(config.draft_count * config.matches_per_draft);

    for draft_index in 0..config.draft_count {
        let draft_seed = config.base_seed + draft_index as u64 * 1000;
        let (deck_a, deck_b) = run_sim_draft(draft_seed);

        let color_a = dominant_color(&deck_a);
        let color_b = dominant_color(&deck_b);

        for card in &deck_a {
            for mana in get_card_color_identity(card) {
                if mana != ManaType::Colorless {
                    let color_name = mana_type_to_color_name(&mana);
                    *color_cards.entry(color_name).or_default() += 1;
                }
            }
        }
        for card in &deck_b {
            for mana in get_card_color_identity(card) {
                if mana != ManaType::Colorless {
                    let color_name = mana_type_to_color_name(&mana);
                    *color_cards.entry(color_name).or_default() += 1;
                }
            }
        }

        for match_index in 0..config.matches_per_draft {
            let mut match_config = config.base_match.clone();
            match_config.seed = draft_seed + match_index as u64 + 1;
            match_config.participants[0].deck_id = format!("draft-{draft_index}-p1");
            match_config.participants[1].deck_id = format!("draft-{draft_index}-p2");
            match_config.participants[0].agent = super::agent::AgentProfile::Generalist;
            match_config.participants[1].agent = super::agent::AgentProfile::Generalist;

            let (mut da, mut db) = (deck_a.clone(), deck_b.clone());
            let mut ca = color_a.clone();
            let mut cb = color_b.clone();
            if config.alternate_seats && match_index % 2 == 1 {
                match_config.participants.swap(0, 1);
                std::mem::swap(&mut da, &mut db);
                std::mem::swap(&mut ca, &mut cb);
            }
            work_items.push(DraftWorkItem {
                match_config,
                deck_a: da,
                deck_b: db,
                color_a: ca,
                color_b: cb,
                match_index,
            });
        }
    }

    // Phase 2 (parallel): run matches. Each work item is an independent task.
    let total_jobs = work_items.len();
    let done = AtomicUsize::new(0);
    let show_progress = std::env::var("CARDGAME_GAUNTLET_QUIET").is_err();

    struct MatchOutcome {
        rounds: usize,
        actions_taken: usize,
        termination: TerminationReason,
        is_finished: bool,
        is_draw: bool,
        is_decisive: bool,
        winner_is_p1: bool,
        // Colors in the seats that actually played (post-swap).
        color_p1: String,
        color_p2: String,
    }

    let outcomes: Vec<MatchOutcome> = work_items
        .into_par_iter()
        .with_min_len(1)
        .map(|w| {
            let report = run_match_with_decks(&w.match_config, w.deck_a, w.deck_b, patch_override);
            let winner_is_p1 = report.winner_player_id.as_deref() == Some("p1");
            let outcome = MatchOutcome {
                rounds: report.rounds,
                actions_taken: report.actions_taken,
                termination: report.termination_reason.clone(),
                is_finished: report.is_finished(),
                is_draw: report.is_draw(),
                is_decisive: report.is_decisive(),
                winner_is_p1,
                color_p1: w.color_a,
                color_p2: w.color_b,
            };
            if show_progress {
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                eprint!("\r[{}/{}] draft matches", n, total_jobs);
            }
            let _ = w.match_index; // kept on struct for future use / debugging
            outcome
        })
        .collect();
    if show_progress && total_jobs > 0 {
        eprintln!();
    }

    // Phase 3 (serial): reduce outcomes into the aggregate counters.
    for outcome in &outcomes {
        total_matches += 1;
        total_rounds += outcome.rounds;
        total_actions += outcome.actions_taken;
        *all_terminations
            .entry(outcome.termination.clone())
            .or_default() += 1;

        if outcome.is_finished {
            finished_matches += 1;
            if outcome.is_draw {
                drawn_matches += 1;
            } else if outcome.is_decisive {
                decisive_matches += 1;
                let (winner_color, loser_color) = if outcome.winner_is_p1 {
                    (&outcome.color_p1, &outcome.color_p2)
                } else {
                    (&outcome.color_p2, &outcome.color_p1)
                };
                *color_wins.entry(winner_color.clone()).or_default() += 1;
                *color_losses.entry(loser_color.clone()).or_default() += 1;
                if outcome.winner_is_p1 {
                    first_player_wins += 1;
                }
                first_player_decisive += 1;
            }
        }
    }

    // Build color metrics
    let mut color_metrics: Vec<DraftColorMetrics> = colors
        .iter()
        .map(|&color| {
            let wins = color_wins.get(color).copied().unwrap_or(0);
            let losses = color_losses.get(color).copied().unwrap_or(0);
            let total = wins + losses;
            DraftColorMetrics {
                color: color.to_string(),
                total_cards_drafted: color_cards.get(color).copied().unwrap_or(0),
                wins,
                losses,
                win_rate: if total > 0 {
                    Some(wins as f64 / total as f64)
                } else {
                    None
                },
            }
        })
        .collect();
    color_metrics.sort_by(|a, b| a.color.cmp(&b.color));

    // Compute balance score from color win rates
    let win_rates: Vec<f64> = color_metrics.iter().filter_map(|m| m.win_rate).collect();
    let balance_score = if win_rates.len() >= 2 {
        let max_wr = win_rates.iter().cloned().fold(f64::MIN, f64::max);
        let min_wr = win_rates.iter().cloned().fold(f64::MAX, f64::min);
        max_wr - min_wr
    } else {
        0.0
    };

    let avg_rounds = if finished_matches > 0 {
        total_rounds as f64 / finished_matches as f64
    } else {
        0.0
    };
    let avg_actions = if finished_matches > 0 {
        total_actions as f64 / finished_matches as f64
    } else {
        0.0
    };

    let termination_counts: Vec<TerminationCount> = all_terminations
        .into_iter()
        .map(|(reason, count)| TerminationCount { reason, count })
        .collect();

    DraftMatrixReport {
        draft_count: config.draft_count,
        matches_per_draft: config.matches_per_draft,
        total_matches,
        color_metrics,
        balance: BalanceMetrics {
            balance_score,
            pairwise_imbalance: 0.0,
            finished_match_rate: if total_matches > 0 {
                finished_matches as f64 / total_matches as f64
            } else {
                0.0
            },
            decisive_match_rate: if total_matches > 0 {
                decisive_matches as f64 / total_matches as f64
            } else {
                0.0
            },
            draw_match_rate: if total_matches > 0 {
                drawn_matches as f64 / total_matches as f64
            } else {
                0.0
            },
            unfinished_match_rate: if total_matches > 0 {
                (total_matches - finished_matches) as f64 / total_matches as f64
            } else {
                0.0
            },
            first_player_win_rate: if first_player_decisive > 0 {
                Some(first_player_wins as f64 / first_player_decisive as f64)
            } else {
                None
            },
            average_rounds: avg_rounds,
            average_actions: avg_actions,
            termination_counts,
        },
    }
}

fn dominant_color(deck: &[crate::card::Card]) -> String {
    use crate::helpers::get_card_color_identity;
    use crate::types::{CardKind, ManaType};

    let mut color_counts: BTreeMap<String, usize> = BTreeMap::new();
    for card in deck {
        if matches!(card.kind, CardKind::Infrastructure) {
            continue;
        }
        for mana in get_card_color_identity(card) {
            if mana != ManaType::Colorless {
                *color_counts
                    .entry(mana_type_to_color_name(&mana))
                    .or_default() += 1;
            }
        }
    }
    color_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color)
        .unwrap_or_else(|| "colorless".to_string())
}

fn mana_type_to_color_name(mana: &crate::types::ManaType) -> String {
    use crate::types::ManaType;
    match mana {
        ManaType::Neon => "neon".to_string(),
        ManaType::Glitch => "glitch".to_string(),
        ManaType::Chrome => "chrome".to_string(),
        ManaType::Slag => "slag".to_string(),
        ManaType::Bio => "bio".to_string(),
        ManaType::Colorless => "colorless".to_string(),
    }
}

fn initialize_state_with_decks(
    config: &MatchConfig,
    deck_a_cards: Vec<crate::card::Card>,
    deck_b_cards: Vec<crate::card::Card>,
    patch: Option<&BalancePatch>,
) -> Result<CardGameState> {
    let mut state = <CardGameState as GameLogic>::new(CardGameInitArgs {
        deck_seed: Some(config.seed),
        format: config.format.clone(),
        ruleset: crate::sim::runner::ruleset_for_config(config),
        ..Default::default()
    });

    join_player(&mut state, "p1")?;
    join_player(&mut state, "p2")?;
    // Use a valid deck ID for initialization, then swap in the actual cards below
    select_deck(&mut state, "p1", "neon")?;
    select_deck(&mut state, "p2", "chrome")?;

    // Apply patch to the pre-built cards if needed
    let mut final_deck_a = deck_a_cards;
    let mut final_deck_b = deck_b_cards;
    if let Some(patch) = patch {
        patch.apply_to_deck_cards(&mut final_deck_a);
        patch.apply_to_deck_cards(&mut final_deck_b);
    }

    let player_a = state
        .players
        .get_mut("p1")
        .ok_or_else(|| anyhow!("p1 not found"))?;
    player_a.deck = final_deck_a;
    // select_deck() above set deck_id to the placeholder color used for
    // state init ("neon"/"chrome"); overwrite with the caller-supplied
    // label so downstream card_metrics attribution isn't all tagged with
    // the placeholder.
    player_a.deck_id = Some(config.participants[0].deck_id.clone());
    let player_b = state
        .players
        .get_mut("p2")
        .ok_or_else(|| anyhow!("p2 not found"))?;
    player_b.deck = final_deck_b;
    player_b.deck_id = Some(config.participants[1].deck_id.clone());

    state.update("p1", CardGameAction::StartGame)?;
    state.tick();
    Ok(state)
}

fn run_match_inner(
    config: &MatchConfig,
    patch_override: Option<&BalancePatch>,
) -> Result<MatchReport> {
    let loaded_patch = if patch_override.is_some() {
        None
    } else {
        load_requested_patch(config.patch_path.as_deref())?
    };
    let patch = patch_override.or(loaded_patch.as_ref());
    let state = match config.format {
        GameFormat::Classic => initialize_state(config, patch)?,
        GameFormat::Operator => initialize_operator_state(config, patch)?,
    };
    run_game_loop(config, state)
}

fn run_game_loop(config: &MatchConfig, mut state: CardGameState) -> Result<MatchReport> {
    let agent_a = build_agent(
        config.participants[0].agent.clone(),
        config.participants[0].uses_mcts,
        config,
    );
    let agent_b = build_agent(
        config.participants[1].agent.clone(),
        config.participants[1].uses_mcts,
        config,
    );
    let mut actions_taken = 0usize;
    let mut trace = Vec::new();
    let mut snapshots = Vec::new();
    let mut fingerprints: BTreeMap<String, usize> = BTreeMap::new();
    let mut last_snapshot_round: Option<usize> = None;
    let mut card_metrics = CardMetricsCollector::new();
    card_metrics.record_initial_decks(&state);

    // Wall-clock timeout: defaults to 300s (5 min) per match. Pod sims with
    // bad MCTS branching can hang on tail pods for hours otherwise. Override
    // with CARDGAME_MATCH_WALL_SECS=0 to disable, or any positive value.
    let match_start = std::time::Instant::now();
    let max_match_wall_secs: u64 = std::env::var("CARDGAME_MATCH_WALL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    let mut last_progress_log = std::time::Instant::now();

    loop {
        // Log periodically if a match is running long, to spot hung pods early
        if match_start.elapsed().as_secs() >= 30 && last_progress_log.elapsed().as_secs() >= 30 {
            eprintln!(
                "  [match] still running at {:.0}s — round {} / actions {}",
                match_start.elapsed().as_secs_f64(),
                state.round,
                actions_taken
            );
            last_progress_log = std::time::Instant::now();
        }
        if max_match_wall_secs > 0 && match_start.elapsed().as_secs() > max_match_wall_secs {
            let winner_id = state.winner_info().0;
            let card_metrics_rows = card_metrics.clone().finalize(&state, winner_id.as_deref());
            return Ok(finished_report_with_metrics(
                config,
                &state,
                actions_taken,
                snapshots,
                trace,
                TerminationReason::LoopDetected,
                Some(format!("wall-clock timeout ({}s)", max_match_wall_secs)),
                card_metrics_rows,
            ));
        }
        state.tick();
        card_metrics.observe(&state);

        if config.include_trace && last_snapshot_round != Some(state.round) {
            snapshots.push(capture_round_snapshot(&state));
            last_snapshot_round = Some(state.round);
        }

        if state.stage == CardGameStage::Finished {
            let winner_id = state.winner_info().0;
            let card_metrics_rows = card_metrics.clone().finalize(&state, winner_id.as_deref());
            return Ok(finished_report_with_metrics(
                config,
                &state,
                actions_taken,
                snapshots,
                trace,
                TerminationReason::Finished,
                None,
                card_metrics_rows,
            ));
        }

        if state.round > config.max_turns {
            let winner_id = state.winner_info().0;
            let card_metrics_rows = card_metrics.clone().finalize(&state, winner_id.as_deref());
            return Ok(finished_report_with_metrics(
                config,
                &state,
                actions_taken,
                snapshots,
                trace,
                TerminationReason::MaxTurnsExceeded,
                None,
                card_metrics_rows,
            ));
        }

        if state.effects_cap_exceeded {
            let winner_id = state.winner_info().0;
            let card_metrics_rows = card_metrics.clone().finalize(&state, winner_id.as_deref());
            return Ok(finished_report_with_metrics(
                config,
                &state,
                actions_taken,
                snapshots,
                trace,
                TerminationReason::LoopDetected,
                Some("effects cap exceeded".to_string()),
                card_metrics_rows,
            ));
        }

        if actions_taken >= config.max_actions {
            let winner_id = state.winner_info().0;
            let card_metrics_rows = card_metrics.clone().finalize(&state, winner_id.as_deref());
            return Ok(finished_report_with_metrics(
                config,
                &state,
                actions_taken,
                snapshots,
                trace,
                TerminationReason::MaxActionsExceeded,
                None,
                card_metrics_rows,
            ));
        }

        let fingerprint = state_fingerprint(&state);
        let seen = fingerprints.entry(fingerprint).or_default();
        *seen += 1;
        if *seen >= config.repeat_state_limit {
            let winner_id = state.winner_info().0;
            let card_metrics_rows = card_metrics.clone().finalize(&state, winner_id.as_deref());
            return Ok(finished_report_with_metrics(
                config,
                &state,
                actions_taken,
                snapshots,
                trace,
                TerminationReason::LoopDetected,
                None,
                card_metrics_rows,
            ));
        }

        let actor_id = match next_actor_id(&state) {
            Some(actor_id) => actor_id,
            None => {
                return Ok(finished_report(
                    config,
                    &state,
                    actions_taken,
                    snapshots,
                    trace,
                    TerminationReason::NoLegalAction,
                    Some("No actor available for current state".to_string()),
                ));
            }
        };

        let legal_actions = legal_actions_for_player(&state, &actor_id);
        if legal_actions.is_empty() {
            // If a pending action can't be fulfilled (e.g. discard with empty hand,
            // sacrifice with no units, trigger target with no valid targets), auto-resolve
            // it by clearing the pending state rather than ending the game. After resolving,
            // re-evaluate from the top since the correct next actor may have changed.
            if try_resolve_unfulfillable_pending(&mut state, &actor_id) {
                continue;
            }
        }
        if legal_actions.is_empty() {
            return Ok(finished_report(
                config,
                &state,
                actions_taken,
                snapshots,
                trace,
                TerminationReason::NoLegalAction,
                Some(format!(
                    "No legal action generated for actor {actor_id}; {}",
                    no_legal_action_context(&state, &actor_id)
                )),
            ));
        }

        let action = if actor_id == "p1" {
            agent_a.choose_action(&state, &actor_id, &legal_actions)
        } else {
            agent_b.choose_action(&state, &actor_id, &legal_actions)
        }
        .or_else(|| legal_actions.first().cloned())
        .ok_or_else(|| anyhow!("Action generation returned an empty set"))?;

        sim_debug(format!(
            "seed={} round={} actions={} stage={:?} actor={} legal={} chosen={}",
            config.seed,
            state.round,
            actions_taken,
            state.stage,
            actor_id,
            legal_actions.len(),
            action_label(&action)
        ));

        if config.include_trace {
            trace.push(ActionTraceEntry {
                index: actions_taken + 1,
                actor_id: actor_id.clone(),
                action: action_label(&action),
            });
        }

        state
            .update(&actor_id, action)
            .map_err(|error| anyhow!("update failed for {actor_id}: {error}"))?;
        actions_taken += 1;
    }
}

fn build_agent(
    profile: super::agent::AgentProfile,
    use_mcts: bool,
    config: &MatchConfig,
) -> Box<dyn Agent> {
    if use_mcts {
        Box::new(MctsAgent::new(
            profile,
            MctsConfig {
                iterations: config.mcts_iterations,
                rollout_depth: config.mcts_rollout_depth,
                ..MctsConfig::default()
            },
        ))
    } else {
        Box::new(ArchetypeAgent::new(profile))
    }
}

/// Unified match state initializer. Handles both classic and operator formats for any N
/// participants (N>=2). Builds decks, joins players, assigns operators, starts the game.
/// Syndicate is chosen when any seat plays a `syn_` deck.
pub fn ruleset_for_config(config: &MatchConfig) -> crate::types::Ruleset {
    if config
        .participants
        .iter()
        .any(|p| crate::types::Ruleset::for_deck_id(&p.deck_id) == crate::types::Ruleset::Syndicate)
    {
        crate::types::Ruleset::Syndicate
    } else {
        crate::types::Ruleset::ManaClash
    }
}

fn initialize_match_state(
    config: &MatchConfig,
    patch: Option<&BalancePatch>,
) -> Result<CardGameState> {
    use crate::deck::{
        DeckTemplates, build_deck_with_templates_and_patch, build_operator_sim_deck_with_patch,
    };

    if config.participants.len() < 2 {
        return Err(anyhow!("MatchConfig needs at least 2 participants"));
    }
    if matches!(config.format, GameFormat::Operator) && config.participants.len() > 8 {
        return Err(anyhow!(
            "Operator format capped at 8 players (got {})",
            config.participants.len()
        ));
    }

    let mut state = <CardGameState as GameLogic>::new(CardGameInitArgs {
        deck_seed: Some(config.seed),
        format: config.format.clone(),
        ruleset: crate::sim::runner::ruleset_for_config(config),
        ..Default::default()
    });

    // Join all participants in turn order.
    for participant in &config.participants {
        join_player(&mut state, &participant.id)?;
    }

    match config.format {
        GameFormat::Classic => {
            // Classic: select deck per participant, optionally apply patch, optionally rebuild for sideboard.
            for participant in &config.participants {
                select_deck(&mut state, &participant.id, &participant.deck_id)?;
            }
            if let Some(patch) = patch {
                for (idx, participant) in config.participants.iter().enumerate() {
                    let opp_idx = (idx + 1) % config.participants.len();
                    let opp_deck = if config.enable_sideboard {
                        Some(config.participants[opp_idx].deck_id.as_str())
                    } else {
                        None
                    };
                    let deck = build_deck_with_templates_and_patch(
                        &participant.id,
                        &participant.deck_id,
                        state.init.deck_seed,
                        &DeckTemplates::default(),
                        opp_deck,
                        Some(patch),
                    )?;
                    let player = state
                        .players
                        .get_mut(&participant.id)
                        .ok_or_else(|| anyhow!("{} not found", participant.id))?;
                    player.deck = deck;
                }
            }
            if config.enable_sideboard {
                for (idx, participant) in config.participants.iter().enumerate() {
                    let opp_idx = (idx + 1) % config.participants.len();
                    rebuild_deck_with_sideboard(
                        &mut state,
                        &participant.id,
                        &participant.deck_id,
                        &config.participants[opp_idx].deck_id,
                        patch,
                    )?;
                }
            }
        }
        GameFormat::Operator => {
            // Operator: build deck + operator card per participant.
            for participant in &config.participants {
                let operator_id = participant.operator.as_deref().ok_or_else(|| {
                    anyhow!("operator required for Operator format ({})", participant.id)
                })?;
                let (deck, op_card) = build_operator_sim_deck_with_patch(
                    &participant.id,
                    &participant.deck_id,
                    operator_id,
                    state.init.deck_seed,
                    patch,
                )?;
                let player = state
                    .players
                    .get_mut(&participant.id)
                    .ok_or_else(|| anyhow!("{} not found", participant.id))?;
                player.deck_id = Some(participant.deck_id.clone());
                player.deck_format = Some(crate::types::DeckFormat::Operator);
                player.deck = deck;
                player.operator_zone = vec![op_card.clone()];
                player.operator_id = Some(op_card.id.clone());
            }
        }
    }

    // Any participant can call StartGame (host semantics).
    state.update(&config.participants[0].id, CardGameAction::StartGame)?;
    state.tick();
    Ok(state)
}

// Deprecated wrappers kept temporarily for any external callers.
#[doc(hidden)]
fn initialize_state(config: &MatchConfig, patch: Option<&BalancePatch>) -> Result<CardGameState> {
    initialize_match_state(config, patch)
}

#[doc(hidden)]
fn initialize_operator_state(
    config: &MatchConfig,
    patch: Option<&BalancePatch>,
) -> Result<CardGameState> {
    initialize_match_state(config, patch)
}

fn join_player(state: &mut CardGameState, player_id: &str) -> Result<()> {
    state.update(
        player_id,
        CardGameAction::Join {
            name: player_id.to_string(),
            avatar: "https://example.com/avatar.png".to_string(),
            instance_id: format!("{player_id}-instance"),
        },
    )?;
    Ok(())
}

fn select_deck(state: &mut CardGameState, player_id: &str, deck_id: &str) -> Result<()> {
    state.update(
        player_id,
        CardGameAction::SelectDeck {
            deck_id: deck_id.to_string(),
        },
    )?;
    Ok(())
}

fn rebuild_deck_with_sideboard(
    state: &mut CardGameState,
    player_id: &str,
    deck_id: &str,
    opponent_deck_id: &str,
    patch: Option<&BalancePatch>,
) -> Result<()> {
    use crate::deck::{DeckTemplates, build_deck_with_templates_and_patch};
    let deck = build_deck_with_templates_and_patch(
        player_id,
        deck_id,
        state.init.deck_seed,
        &DeckTemplates::default(),
        Some(opponent_deck_id),
        patch,
    )?;
    let player = state
        .players
        .get_mut(player_id)
        .ok_or_else(|| anyhow!("Player not found"))?;
    player.deck = deck;
    Ok(())
}

fn load_requested_patch(path: Option<&str>) -> Result<Option<BalancePatch>> {
    match path {
        Some(path) => Ok(Some(load_balance_patch_from_path(path)?)),
        None => Ok(None),
    }
}

fn finished_report(
    config: &MatchConfig,
    state: &CardGameState,
    actions_taken: usize,
    snapshots: Vec<RoundSnapshot>,
    trace: Vec<ActionTraceEntry>,
    termination_reason: TerminationReason,
    error: Option<String>,
) -> MatchReport {
    finished_report_with_metrics(
        config,
        state,
        actions_taken,
        snapshots,
        trace,
        termination_reason,
        error,
        Vec::new(),
    )
}

fn finished_report_with_metrics(
    config: &MatchConfig,
    state: &CardGameState,
    actions_taken: usize,
    snapshots: Vec<RoundSnapshot>,
    trace: Vec<ActionTraceEntry>,
    termination_reason: TerminationReason,
    error: Option<String>,
    card_metrics: Vec<crate::sim::report::CardMetrics>,
) -> MatchReport {
    let winner_player_id = state.winner_info().0;
    let winner_deck_id = winner_player_id
        .as_ref()
        .and_then(|player_id| state.players.get(player_id))
        .and_then(|player| player.deck_id.clone());
    let drawn = termination_reason == TerminationReason::Finished
        && winner_player_id.is_none()
        && winner_deck_id.is_none();

    let mut report = MatchReport {
        seed: config.seed,
        deck_a: config.participants[0].deck_id.clone(),
        deck_b: config.participants[1].deck_id.clone(),
        agent_a: config.participants[0].agent.clone(),
        agent_b: config.participants[1].agent.clone(),
        winner_player_id,
        winner_deck_id,
        drawn,
        rounds: state.round,
        actions_taken,
        termination_reason,
        error,
        final_state: capture_player_summaries(state),
        snapshots,
        trace,
        degeneracy_flags: Vec::new(),
        card_metrics,
    };
    report.degeneracy_flags = detect_degeneracy(&report);
    report
}

pub(crate) fn next_actor_id(state: &CardGameState) -> Option<String> {
    for player_id in &state.turn_order {
        let player = state.players.get(player_id)?;
        if player.has_pending_discard()
            || player.has_pending_sacrifice()
            || player.has_pending_trigger_choice()
            || player.has_pending_trigger_target()
            || !player.scry.is_empty()
            || player.library_search_active
        {
            return Some(player_id.clone());
        }
    }

    if state.stage == CardGameStage::Combat
        && state.combat_phase == CombatPhase::DeclareBlockers
        && let Some(blocker_id) = &state.combat_blocker_id
    {
        return Some(blocker_id.clone());
    }

    if (!state.stack.is_empty() || state.showcase.is_some())
        && let Some(priority_holder) = state.effective_priority_holder()
    {
        return Some(priority_holder.to_string());
    }

    state.active_player_id.clone()
}

fn state_fingerprint(state: &CardGameState) -> String {
    let mut parts = vec![
        format!("stage:{:?}", state.stage),
        format!("round:{}", state.round),
        format!("active:{:?}", state.active_player_id),
        format!("priority:{:?}", state.priority_holder),
        format!("effective_priority:{:?}", state.effective_priority_holder()),
        format!("priority_passes:{}", state.consecutive_priority_passes),
        format!("combat_phase:{:?}", state.combat_phase),
    ];

    // Use template_id (or id if no template) for fingerprinting. Unique
    // instance IDs would make tokens break loop detection: each sac+
    // regenerate cycle would produce different IDs and register as a
    // "different" state, bypassing `repeat_state_limit`.
    fn card_key(card: &crate::card::Card) -> &str {
        card.template_id.as_deref().unwrap_or(card.id.as_str())
    }
    for player_id in &state.turn_order {
        if let Some(player) = state.players.get(player_id) {
            let hand = player
                .hand
                .iter()
                .map(card_key)
                .collect::<Vec<_>>()
                .join(",");
            let battlefield = player
                .battlefield
                .iter()
                .map(|card| format!("{}:{}:{}", card_key(card), card.tapped, card.counters))
                .collect::<Vec<_>>()
                .join(",");
            let graveyard = player
                .graveyard
                .iter()
                .map(card_key)
                .collect::<Vec<_>>()
                .join(",");
            let scry = player
                .scry
                .iter()
                .map(card_key)
                .collect::<Vec<_>>()
                .join(",");
            let pending_actions = player
                .pending_actions
                .iter()
                .map(|action| match action {
                    crate::state::PendingAction::TriggerChoice { source_card_id, .. } => {
                        format!("trigger_choice:{source_card_id}")
                    }
                    crate::state::PendingAction::TriggerTargetSelection {
                        source_card_id,
                        target_kind,
                        ..
                    } => format!("trigger_target:{source_card_id}:{target_kind:?}"),
                    crate::state::PendingAction::Discard { amount, .. } => {
                        format!("discard:{amount}")
                    }
                    crate::state::PendingAction::Sacrifice {
                        source_card_id,
                        remaining,
                        ..
                    } => format!("sacrifice:{source_card_id}:{remaining}"),
                })
                .collect::<Vec<_>>()
                .join(",");

            parts.push(format!(
                "player:{player_id}:life={}:hand=[{}]:battlefield=[{}]:graveyard=[{}]:scry=[{}]:search_active={}:pending_actions=[{}]:pending_effects={}:mana={}",
                player.life,
                hand,
                battlefield,
                graveyard,
                scry,
                player.library_search_active,
                pending_actions,
                player.pending_effects.len(),
                mana_pool_total(&player.mana_pool)
            ));
        }
    }

    parts.push(format!(
        "combat_attackers=[{}]",
        state.combat_attackers.join(",")
    ));
    let mut combat_targets = state
        .combat_targets
        .iter()
        .map(|(attacker_id, target)| format!("{attacker_id}:{}", target_ref_key(target)))
        .collect::<Vec<_>>();
    combat_targets.sort();
    parts.push(format!("combat_targets=[{}]", combat_targets.join(",")));

    let mut combat_blockers = state
        .combat_blockers
        .iter()
        .map(|(attacker_id, blockers)| {
            let mut blockers = blockers.clone();
            blockers.sort();
            format!("{attacker_id}:{}", blockers.join("+"))
        })
        .collect::<Vec<_>>();
    combat_blockers.sort();
    parts.push(format!("combat_blockers=[{}]", combat_blockers.join(",")));

    parts.push(format!(
        "stack=[{}]",
        state
            .stack
            .iter()
            .map(|item| match &item.kind {
                crate::state::StackItemKind::Spell(card) => format!("spell:{}", card.id),
                crate::state::StackItemKind::Ability {
                    source_card_id,
                    ability,
                } => format!("ability:{}:{}", source_card_id, ability.id),
            })
            .collect::<Vec<_>>()
            .join(",")
    ));

    parts.join("|")
}

fn mana_pool_total(pool: &crate::mana::ManaPool) -> i32 {
    pool.neon + pool.glitch + pool.chrome + pool.slag + pool.bio + pool.colorless
}

fn target_ref_key(target: &crate::types::TargetRef) -> String {
    match target {
        crate::types::TargetRef::Player(id) => format!("p:{id}"),
        crate::types::TargetRef::Card(id) => format!("c:{id}"),
    }
}

/// When a player has a pending action that can't be fulfilled (e.g. forced to discard
/// with an empty hand, forced to sacrifice with no units, or trigger needs a target that
/// doesn't exist), clear the pending action so the game can continue. Returns true if
/// a pending action was resolved.
pub(crate) fn try_resolve_unfulfillable_pending(state: &mut CardGameState, actor_id: &str) -> bool {
    let Some(player) = state.players.get(actor_id) else {
        return false;
    };

    // Pending discard but hand is empty — nothing to discard, clear it
    if player.has_pending_discard() && player.hand.is_empty() {
        let player = state.players.get_mut(actor_id).unwrap();
        player.take_pending_discard();
        let pending_effects = std::mem::take(&mut player.pending_effects);
        for (effect, targets) in pending_effects {
            state.apply_effect(actor_id, &effect, &targets);
        }
        state.try_finish_end_step(actor_id);
        sim_debug(format!(
            "auto-resolved unfulfillable discard for {actor_id} (empty hand)"
        ));
        return true;
    }

    // Pending sacrifice but no eligible units on battlefield
    if player.has_pending_sacrifice() {
        let has_target = if let Some(PendingAction::Sacrifice { source_card_id, .. }) =
            player.pending_sacrifice()
        {
            player
                .battlefield
                .iter()
                .any(|c| c.id != *source_card_id && matches!(c.kind, CardKind::Unit(_)))
        } else {
            false
        };

        if !has_target {
            let player = state.players.get_mut(actor_id).unwrap();
            player.take_pending_sacrifice();
            sim_debug(format!(
                "auto-resolved unfulfillable sacrifice for {actor_id} (no eligible units)"
            ));
            return true;
        }
    }

    // Pending trigger target but no valid targets exist
    if player.has_pending_trigger_target() {
        let player = state.players.get_mut(actor_id).unwrap();
        player.take_pending_trigger_target();
        sim_debug(format!(
            "auto-resolved unfulfillable trigger target for {actor_id} (no valid targets)"
        ));
        return true;
    }

    false
}

fn no_legal_action_context(state: &CardGameState, actor_id: &str) -> String {
    let Some(actor) = state.players.get(actor_id) else {
        return "actor_missing=true".to_string();
    };

    let pending_actions = actor
        .pending_actions
        .iter()
        .map(|action| match action {
            crate::state::PendingAction::TriggerChoice { source_card_id, .. } => {
                format!("trigger_choice:{source_card_id}")
            }
            crate::state::PendingAction::TriggerTargetSelection {
                source_card_id,
                target_kind,
                ..
            } => format!("trigger_target:{source_card_id}:{target_kind:?}"),
            crate::state::PendingAction::Discard { amount, .. } => {
                format!("discard:{amount}")
            }
            crate::state::PendingAction::Sacrifice {
                source_card_id,
                remaining,
                ..
            } => format!("sacrifice:{source_card_id}:{remaining}"),
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "stage={:?} round={} active={:?} priority={:?} stack_len={} scry_len={} search_active={} hand_size={} battlefield_size={} pending_actions=[{}] pending_effects={}",
        state.stage,
        state.round,
        state.active_player_id,
        state.effective_priority_holder(),
        state.stack.len(),
        actor.scry.len(),
        actor.library_search_active,
        actor.hand.len(),
        actor.battlefield.len(),
        pending_actions,
        actor.pending_effects.len()
    )
}

fn summarize_matchup(series: &SeriesReport) -> MatchupMetrics {
    let mut accumulator = MatchupAccumulator::new(series.deck_a.clone(), series.deck_b.clone());
    for result in &series.results {
        accumulator.record(result);
    }
    accumulator.finish()
}

fn accumulate_matchup(
    matchup: &MatchupMetrics,
    deck_rollups: &mut BTreeMap<String, DeckAccumulator>,
    global: &mut GlobalAccumulator,
) {
    global.matches += matchup.matches;
    global.finished_matches += matchup.finished_matches;
    global.decisive_matches += matchup.decisive_matches;
    global.drawn_matches += matchup.drawn_matches;
    global.first_player_wins += matchup.first_player_wins;
    global.total_rounds += matchup.average_rounds * matchup.matches as f64;
    global.total_actions += matchup.average_actions * matchup.matches as f64;

    for termination in &matchup.termination_counts {
        *global
            .termination_counts
            .entry(termination.reason.clone())
            .or_default() += termination.count;
    }

    let wins_by_deck: BTreeMap<_, _> = matchup.wins_by_deck.iter().cloned().collect();
    if matchup.deck_a == matchup.deck_b {
        return;
    }

    let decks = [matchup.deck_a.clone(), matchup.deck_b.clone()];

    for deck_id in decks {
        let accumulator = deck_rollups.entry(deck_id.clone()).or_default();
        accumulator.matches += matchup.matches;
        accumulator.finished_matches += matchup.finished_matches;
        accumulator.decisive_matches += matchup.decisive_matches;
        accumulator.drawn_matches += matchup.drawn_matches;
        accumulator.wins += wins_by_deck.get(&deck_id).copied().unwrap_or_default();
        accumulator.unfinished_matches += matchup.unfinished_matches;
    }

    if matchup.decisive_matches > 0 {
        let deck_a_wins = wins_by_deck
            .get(&matchup.deck_a)
            .copied()
            .unwrap_or_default();
        let deck_b_wins = wins_by_deck
            .get(&matchup.deck_b)
            .copied()
            .unwrap_or_default();
        if let Some(accumulator) = deck_rollups.get_mut(&matchup.deck_a) {
            accumulator.losses += deck_b_wins;
        }
        if let Some(accumulator) = deck_rollups.get_mut(&matchup.deck_b) {
            accumulator.losses += deck_a_wins;
        }
    }
}

fn average(total: usize, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

#[derive(Clone, Debug, Default)]
struct DeckAccumulator {
    matches: usize,
    finished_matches: usize,
    decisive_matches: usize,
    drawn_matches: usize,
    wins: usize,
    losses: usize,
    unfinished_matches: usize,
}

impl DeckAccumulator {
    fn into_metrics(self, deck_id: String) -> DeckMetrics {
        DeckMetrics {
            deck_id,
            matches: self.matches,
            finished_matches: self.finished_matches,
            decisive_matches: self.decisive_matches,
            drawn_matches: self.drawn_matches,
            wins: self.wins,
            losses: self.losses,
            unfinished_matches: self.unfinished_matches,
            win_rate: rate(self.wins, self.decisive_matches),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct GlobalAccumulator {
    matches: usize,
    finished_matches: usize,
    decisive_matches: usize,
    drawn_matches: usize,
    first_player_wins: usize,
    total_rounds: f64,
    total_actions: f64,
    termination_counts: BTreeMap<TerminationReason, usize>,
}

impl GlobalAccumulator {
    fn into_balance(
        self,
        deck_metrics: &[DeckMetrics],
        matchup_metrics: &[MatchupMetrics],
    ) -> BalanceMetrics {
        let balance_score = deck_metrics
            .iter()
            .filter_map(|deck| deck.win_rate)
            .map(|win_rate| (win_rate - 0.5).abs())
            .sum();
        let pairwise_imbalance = matchup_metrics
            .iter()
            .filter_map(|matchup| matchup.deck_a_win_rate)
            .map(|win_rate| (win_rate - 0.5).abs())
            .sum();

        BalanceMetrics {
            balance_score,
            pairwise_imbalance,
            finished_match_rate: if self.matches == 0 {
                0.0
            } else {
                self.finished_matches as f64 / self.matches as f64
            },
            decisive_match_rate: if self.matches == 0 {
                0.0
            } else {
                self.decisive_matches as f64 / self.matches as f64
            },
            draw_match_rate: if self.matches == 0 {
                0.0
            } else {
                self.drawn_matches as f64 / self.matches as f64
            },
            unfinished_match_rate: if self.matches == 0 {
                0.0
            } else {
                (self.matches - self.finished_matches) as f64 / self.matches as f64
            },
            first_player_win_rate: rate(self.first_player_wins, self.decisive_matches),
            average_rounds: if self.matches == 0 {
                0.0
            } else {
                self.total_rounds / self.matches as f64
            },
            average_actions: if self.matches == 0 {
                0.0
            } else {
                self.total_actions / self.matches as f64
            },
            termination_counts: self
                .termination_counts
                .into_iter()
                .map(|(reason, count)| TerminationCount { reason, count })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchupAccumulator {
    deck_a: String,
    deck_b: String,
    matches: usize,
    finished_matches: usize,
    decisive_matches: usize,
    drawn_matches: usize,
    unfinished_matches: usize,
    first_player_wins: usize,
    total_rounds: usize,
    total_actions: usize,
    wins_by_deck: BTreeMap<String, usize>,
    termination_counts: BTreeMap<TerminationReason, usize>,
}

impl MatchupAccumulator {
    pub fn new(deck_a: String, deck_b: String) -> Self {
        Self {
            deck_a,
            deck_b,
            matches: 0,
            finished_matches: 0,
            decisive_matches: 0,
            drawn_matches: 0,
            unfinished_matches: 0,
            first_player_wins: 0,
            total_rounds: 0,
            total_actions: 0,
            wins_by_deck: BTreeMap::new(),
            termination_counts: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, result: &MatchReport) {
        self.matches += 1;
        self.total_rounds += result.rounds;
        self.total_actions += result.actions_taken;
        *self
            .termination_counts
            .entry(result.termination_reason.clone())
            .or_default() += 1;

        if result.is_finished() {
            self.finished_matches += 1;
            if result.is_draw() {
                self.drawn_matches += 1;
            } else if let Some(winner_deck_id) = &result.winner_deck_id {
                self.decisive_matches += 1;
                *self.wins_by_deck.entry(winner_deck_id.clone()).or_default() += 1;
                if result.winner_player_id.as_deref() == Some("p1") {
                    self.first_player_wins += 1;
                }
            }
        } else {
            self.unfinished_matches += 1;
        }
    }

    pub fn finish(self) -> MatchupMetrics {
        let deck_a_wins = self
            .wins_by_deck
            .get(&self.deck_a)
            .copied()
            .unwrap_or_default();

        MatchupMetrics {
            deck_a: self.deck_a.clone(),
            deck_b: self.deck_b.clone(),
            matches: self.matches,
            finished_matches: self.finished_matches,
            decisive_matches: self.decisive_matches,
            drawn_matches: self.drawn_matches,
            unfinished_matches: self.unfinished_matches,
            wins_by_deck: self.wins_by_deck.into_iter().collect(),
            first_player_wins: self.first_player_wins,
            first_player_win_rate: rate(self.first_player_wins, self.decisive_matches),
            average_rounds: average(self.total_rounds, self.matches),
            average_actions: average(self.total_actions, self.matches),
            deck_a_win_rate: if self.deck_a == self.deck_b {
                None
            } else {
                rate(deck_a_wins, self.decisive_matches)
            },
            termination_counts: self
                .termination_counts
                .into_iter()
                .map(|(reason, count)| TerminationCount { reason, count })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct MatrixReportBuilder<'a> {
    config: &'a MatrixConfig,
    matchup_metrics: Vec<MatchupMetrics>,
    deck_rollups: BTreeMap<String, DeckAccumulator>,
    global: GlobalAccumulator,
}

impl<'a> MatrixReportBuilder<'a> {
    fn new(config: &'a MatrixConfig) -> Self {
        Self {
            config,
            matchup_metrics: Vec::new(),
            deck_rollups: config
                .decks
                .iter()
                .cloned()
                .map(|deck_id| (deck_id, DeckAccumulator::default()))
                .collect(),
            global: GlobalAccumulator::default(),
        }
    }

    fn push_metrics(&mut self, metrics: MatchupMetrics) {
        accumulate_matchup(&metrics, &mut self.deck_rollups, &mut self.global);
        self.matchup_metrics.push(metrics);
    }

    fn finish(self) -> MatrixReport {
        let deck_metrics: Vec<_> = self
            .deck_rollups
            .into_iter()
            .map(|(deck_id, accumulator)| accumulator.into_metrics(deck_id))
            .collect();
        let balance = self
            .global
            .into_balance(&deck_metrics, &self.matchup_metrics);

        MatrixReport {
            decks: self.config.decks.clone(),
            matches_per_pair: self.config.matches_per_pair,
            alternate_seats: self.config.alternate_seats,
            include_mirrors: self.config.include_mirrors,
            deck_metrics,
            matchup_metrics: self.matchup_metrics,
            balance,
            degeneracy: None,
        }
    }
}

fn capture_round_snapshot(state: &CardGameState) -> RoundSnapshot {
    RoundSnapshot {
        round: state.round,
        active_player_id: state.active_player_id.clone(),
        stage: state.stage.clone(),
        players: capture_player_summaries(state),
    }
}

fn capture_player_summaries(state: &CardGameState) -> Vec<PlayerSummary> {
    state
        .turn_order
        .iter()
        .filter_map(|player_id| {
            let player = state.players.get(player_id)?;
            let battlefield_power = player
                .battlefield
                .iter()
                .map(|card| effective_power(state, card))
                .sum();

            Some(PlayerSummary {
                player_id: player_id.clone(),
                deck_id: player.deck_id.clone(),
                life: player.life,
                hand_size: player.hand.len(),
                battlefield_size: player.battlefield.len(),
                battlefield_power,
                graveyard_size: player.graveyard.len(),
                mana_sources: player.grid.len()
                    + player
                        .battlefield
                        .iter()
                        .filter(|card| matches!(card.kind, crate::types::CardKind::Infrastructure))
                        .count(),
                operator_id: player.operator_id.clone(),
                operator_cast_count: player.operator_cast_count,
                operator_first_cast_round: player.operator_first_cast_round,
                operator_damage_total: player.operator_damage.values().sum(),
                tokens_created: player.tokens_created,
                loyalty_activations: player.loyalty_activations,
            })
        })
        .collect()
}

fn effective_power(state: &CardGameState, card: &crate::card::Card) -> i32 {
    card.power.unwrap_or(0)
        + state.modifier_amount_for_card(&card.id, crate::types::StatType::Power)
}

// ---------------------------------------------------------------------------
// Operator matrix
// ---------------------------------------------------------------------------

pub fn expand_operator_combos(config: &OperatorMatrixConfig) -> Vec<OperatorCombo> {
    use crate::deck::operator_ids_for_color;

    let mut combos = Vec::new();
    for deck_id in &config.decks {
        for op_id in operator_ids_for_color(deck_id) {
            if config.operator_filter.is_empty()
                || config
                    .operator_filter
                    .iter()
                    .any(|f| f == op_id || f == deck_id)
            {
                combos.push(OperatorCombo {
                    deck_id: deck_id.clone(),
                    operator_id: op_id.to_string(),
                });
            }
        }
    }
    combos
}

pub fn run_operator_matrix(config: &OperatorMatrixConfig) -> OperatorMatrixReport {
    run_operator_matrix_with_patch(config, None)
}

pub fn run_operator_matrix_with_patch(
    config: &OperatorMatrixConfig,
    patch_override: Option<&BalancePatch>,
) -> OperatorMatrixReport {
    let combos = expand_operator_combos(config);
    let total_matchups = if config.include_mirrors {
        combos.len() * (combos.len() + 1) / 2
    } else {
        combos.len().saturating_sub(1) * combos.len() / 2
    };
    let mut builder = OperatorMatrixReportBuilder::new(config, &combos);
    let mut matchup_index = 0usize;

    for (left_index, combo_a) in combos.iter().enumerate() {
        let right_start = if config.include_mirrors {
            left_index
        } else {
            left_index + 1
        };

        for combo_b in combos.iter().skip(right_start) {
            matchup_index += 1;
            operator_matrix_progress(format!(
                "starting matchup {matchup_index}/{total_matchups}: {} vs {}",
                combo_a.label(),
                combo_b.label()
            ));
            let mut accumulator = OperatorMatchupAccumulator::new(combo_a.label(), combo_b.label());

            let reports: Vec<MatchReport> = (0..config.matches_per_pair)
                .into_par_iter()
                .map(|index| {
                    let mut match_config = config.base_match.clone();
                    match_config.format = GameFormat::Operator;
                    match_config.participants[0].deck_id = combo_a.deck_id.clone();
                    match_config.participants[1].deck_id = combo_b.deck_id.clone();
                    match_config.participants[0].agent = profile_for_deck(&combo_a.deck_id);
                    match_config.participants[1].agent = profile_for_deck(&combo_b.deck_id);
                    match_config.participants[0].operator = Some(combo_a.operator_id.clone());
                    match_config.participants[1].operator = Some(combo_b.operator_id.clone());
                    match_config.seed = config.base_match.seed + index as u64;

                    if config.alternate_seats && index % 2 == 1 {
                        match_config.participants.swap(0, 1);
                    }

                    run_match_with_patch(&match_config, patch_override)
                })
                .collect();

            for report in &reports {
                operator_matrix_progress(format!(
                    "  match seed={} {} vs {} termination={:?} rounds={} actions={}",
                    report.seed,
                    combo_a.label(),
                    combo_b.label(),
                    report.termination_reason,
                    report.rounds,
                    report.actions_taken
                ));
                builder.push_match_diagnostics(report, combo_a, combo_b);
                accumulator.record(report, combo_a, combo_b);
            }

            builder.push_matchup(accumulator.finish());
        }
    }

    builder.finish()
}

/// Build operator matrix matches for supervised execution (used by the CLI).
pub fn build_operator_matrix_matchups(
    config: &OperatorMatrixConfig,
) -> (Vec<OperatorCombo>, Vec<OperatorMatchupPlan>) {
    let combos = expand_operator_combos(config);
    let mut plans = Vec::new();

    for (left_index, combo_a) in combos.iter().enumerate() {
        let right_start = if config.include_mirrors {
            left_index
        } else {
            left_index + 1
        };

        for combo_b in combos.iter().skip(right_start) {
            let mut matches = Vec::new();
            for index in 0..config.matches_per_pair {
                let mut match_config = config.base_match.clone();
                match_config.format = GameFormat::Operator;
                match_config.participants[0].deck_id = combo_a.deck_id.clone();
                match_config.participants[1].deck_id = combo_b.deck_id.clone();
                match_config.participants[0].agent = profile_for_deck(&combo_a.deck_id);
                match_config.participants[1].agent = profile_for_deck(&combo_b.deck_id);
                match_config.participants[0].operator = Some(combo_a.operator_id.clone());
                match_config.participants[1].operator = Some(combo_b.operator_id.clone());
                match_config.seed = config.base_match.seed + index as u64;

                if config.alternate_seats && index % 2 == 1 {
                    match_config.participants.swap(0, 1);
                }

                matches.push(match_config);
            }

            plans.push(OperatorMatchupPlan {
                combo_a: combo_a.clone(),
                combo_b: combo_b.clone(),
                matches,
            });
        }
    }

    (combos, plans)
}

pub fn build_operator_matrix_report_from_matchups(
    config: &OperatorMatrixConfig,
    combos: &[OperatorCombo],
    matchup_metrics: Vec<OperatorMatchupMetrics>,
) -> OperatorMatrixReport {
    let mut builder = OperatorMatrixReportBuilder::new(config, combos);
    for metrics in matchup_metrics {
        builder.push_matchup(metrics);
    }
    builder.finish()
}

#[derive(Clone, Debug)]
pub struct OperatorMatchupPlan {
    pub combo_a: OperatorCombo,
    pub combo_b: OperatorCombo,
    pub matches: Vec<MatchConfig>,
}

pub struct OperatorMatchupAccumulator {
    combo_a: String,
    combo_b: String,
    matches: usize,
    finished_matches: usize,
    decisive_matches: usize,
    drawn_matches: usize,
    unfinished_matches: usize,
    first_player_wins: usize,
    total_rounds: usize,
    total_actions: usize,
    wins_a: usize,
    wins_b: usize,
    termination_counts: BTreeMap<TerminationReason, usize>,
}

impl OperatorMatchupAccumulator {
    pub fn new(combo_a: String, combo_b: String) -> Self {
        Self {
            combo_a,
            combo_b,
            matches: 0,
            finished_matches: 0,
            decisive_matches: 0,
            drawn_matches: 0,
            unfinished_matches: 0,
            first_player_wins: 0,
            total_rounds: 0,
            total_actions: 0,
            wins_a: 0,
            wins_b: 0,
            termination_counts: BTreeMap::new(),
        }
    }

    pub fn record(
        &mut self,
        result: &MatchReport,
        combo_a: &OperatorCombo,
        combo_b: &OperatorCombo,
    ) {
        self.matches += 1;
        self.total_rounds += result.rounds;
        self.total_actions += result.actions_taken;
        *self
            .termination_counts
            .entry(result.termination_reason.clone())
            .or_default() += 1;

        if result.is_finished() {
            self.finished_matches += 1;
            if result.is_draw() {
                self.drawn_matches += 1;
            } else if result.winner_deck_id.is_some() {
                self.decisive_matches += 1;
                // In seat-alternated matches, the deck_a in the report may be swapped.
                // Use the winner_player_id to determine which combo won.
                match result.winner_player_id.as_deref() {
                    Some("p1") => {
                        if result.deck_a == combo_a.deck_id {
                            self.wins_a += 1;
                        } else {
                            self.wins_b += 1;
                        }
                        self.first_player_wins += 1;
                    }
                    Some("p2") => {
                        if result.deck_b == combo_a.deck_id && combo_a.deck_id != combo_b.deck_id {
                            self.wins_a += 1;
                        } else if result.deck_b == combo_b.deck_id {
                            self.wins_b += 1;
                        } else {
                            self.wins_b += 1;
                        }
                    }
                    _ => {
                        self.drawn_matches += 1;
                        self.decisive_matches = self.decisive_matches.saturating_sub(1);
                    }
                }
            }
        } else {
            self.unfinished_matches += 1;
        }
    }

    pub fn finish(self) -> OperatorMatchupMetrics {
        OperatorMatchupMetrics {
            combo_a: self.combo_a,
            combo_b: self.combo_b,
            matches: self.matches,
            finished_matches: self.finished_matches,
            decisive_matches: self.decisive_matches,
            drawn_matches: self.drawn_matches,
            unfinished_matches: self.unfinished_matches,
            wins_a: self.wins_a,
            wins_b: self.wins_b,
            combo_a_win_rate: rate(self.wins_a, self.decisive_matches),
            first_player_wins: self.first_player_wins,
            first_player_win_rate: rate(self.first_player_wins, self.decisive_matches),
            average_rounds: average(self.total_rounds, self.matches),
            average_actions: average(self.total_actions, self.matches),
            termination_counts: self
                .termination_counts
                .into_iter()
                .map(|(reason, count)| TerminationCount { reason, count })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct OperatorAccumulator {
    matches: usize,
    finished_matches: usize,
    decisive_matches: usize,
    drawn_matches: usize,
    wins: usize,
    losses: usize,
    unfinished_matches: usize,
}

#[derive(Clone, Debug, Default)]
struct OperatorDiagnosticAccumulator {
    finished_matches: usize,
    /// Number of finished matches where the operator was cast at least once.
    cast_matches: usize,
    /// Sum of first-cast rounds (among matches where the operator was cast).
    first_cast_round_sum: usize,
    /// Sum of (cast_count - 1) for matches where cast.
    recast_sum: usize,
    loyalty_activation_sum: usize,
    operator_damage_sum: i32,
    tokens_created_sum: usize,
    /// Decisive matches where this combo's player was first (seat p1).
    first_seat_decisive: usize,
    first_seat_wins: usize,
    /// Decisive matches where this combo's player was second (seat p2).
    second_seat_decisive: usize,
    second_seat_wins: usize,
    /// Sum of rounds across finished matches.
    rounds_sum: usize,
}

impl OperatorDiagnosticAccumulator {
    fn record_sample(
        &mut self,
        summary: &PlayerSummary,
        report: &MatchReport,
        was_first_player: bool,
    ) {
        if !report.is_finished() {
            return;
        }
        self.finished_matches += 1;
        self.rounds_sum += report.rounds;

        if summary.operator_cast_count > 0 {
            self.cast_matches += 1;
            if let Some(round) = summary.operator_first_cast_round {
                self.first_cast_round_sum += round;
            }
            self.recast_sum += summary.operator_cast_count.saturating_sub(1);
        }

        self.loyalty_activation_sum += summary.loyalty_activations;
        self.operator_damage_sum += summary.operator_damage_total;
        self.tokens_created_sum += summary.tokens_created;

        if report.is_decisive() {
            let won = report
                .winner_player_id
                .as_deref()
                .map(|w| w == summary.player_id)
                .unwrap_or(false);
            if was_first_player {
                self.first_seat_decisive += 1;
                if won {
                    self.first_seat_wins += 1;
                }
            } else {
                self.second_seat_decisive += 1;
                if won {
                    self.second_seat_wins += 1;
                }
            }
        }
    }

    fn into_diagnostics(self) -> Option<OperatorDiagnostics> {
        if self.finished_matches == 0 {
            return None;
        }
        let fm = self.finished_matches as f64;
        Some(OperatorDiagnostics {
            cast_rate: Some(self.cast_matches as f64 / fm),
            average_first_cast_round: if self.cast_matches > 0 {
                Some(self.first_cast_round_sum as f64 / self.cast_matches as f64)
            } else {
                None
            },
            average_recast_count: if self.cast_matches > 0 {
                Some(self.recast_sum as f64 / self.cast_matches as f64)
            } else {
                None
            },
            average_loyalty_activations: Some(self.loyalty_activation_sum as f64 / fm),
            average_operator_damage: Some(self.operator_damage_sum as f64 / fm),
            average_tokens_created: Some(self.tokens_created_sum as f64 / fm),
            first_seat_win_rate: rate(self.first_seat_wins, self.first_seat_decisive),
            second_seat_win_rate: rate(self.second_seat_wins, self.second_seat_decisive),
            average_game_length: Some(self.rounds_sum as f64 / fm),
        })
    }
}

struct OperatorMatrixReportBuilder<'a> {
    config: &'a OperatorMatrixConfig,
    combo_labels: Vec<String>,
    combo_decks: BTreeMap<String, String>, // label -> deck_id
    operator_rollups: BTreeMap<String, OperatorAccumulator>,
    diagnostic_rollups: BTreeMap<String, OperatorDiagnosticAccumulator>,
    matchup_metrics: Vec<OperatorMatchupMetrics>,
    global: GlobalAccumulator,
}

impl<'a> OperatorMatrixReportBuilder<'a> {
    fn new(config: &'a OperatorMatrixConfig, combos: &[OperatorCombo]) -> Self {
        let combo_labels: Vec<_> = combos.iter().map(|c| c.label()).collect();
        let combo_decks: BTreeMap<_, _> = combos
            .iter()
            .map(|c| (c.label(), c.deck_id.clone()))
            .collect();
        let operator_rollups = combos
            .iter()
            .map(|c| (c.label(), OperatorAccumulator::default()))
            .collect();
        let diagnostic_rollups = combos
            .iter()
            .map(|c| (c.label(), OperatorDiagnosticAccumulator::default()))
            .collect();

        Self {
            config,
            combo_labels,
            combo_decks,
            operator_rollups,
            diagnostic_rollups,
            matchup_metrics: Vec::new(),
            global: GlobalAccumulator::default(),
        }
    }

    fn push_matchup(&mut self, metrics: OperatorMatchupMetrics) {
        self.global.matches += metrics.matches;
        self.global.finished_matches += metrics.finished_matches;
        self.global.decisive_matches += metrics.decisive_matches;
        self.global.drawn_matches += metrics.drawn_matches;
        self.global.first_player_wins += metrics.first_player_wins;
        self.global.total_rounds += metrics.average_rounds * metrics.matches as f64;
        self.global.total_actions += metrics.average_actions * metrics.matches as f64;

        for tc in &metrics.termination_counts {
            *self
                .global
                .termination_counts
                .entry(tc.reason.clone())
                .or_default() += tc.count;
        }

        if metrics.combo_a != metrics.combo_b {
            if let Some(acc) = self.operator_rollups.get_mut(&metrics.combo_a) {
                acc.matches += metrics.matches;
                acc.finished_matches += metrics.finished_matches;
                acc.decisive_matches += metrics.decisive_matches;
                acc.drawn_matches += metrics.drawn_matches;
                acc.wins += metrics.wins_a;
                acc.losses += metrics.wins_b;
                acc.unfinished_matches += metrics.unfinished_matches;
            }
            if let Some(acc) = self.operator_rollups.get_mut(&metrics.combo_b) {
                acc.matches += metrics.matches;
                acc.finished_matches += metrics.finished_matches;
                acc.decisive_matches += metrics.decisive_matches;
                acc.drawn_matches += metrics.drawn_matches;
                acc.wins += metrics.wins_b;
                acc.losses += metrics.wins_a;
                acc.unfinished_matches += metrics.unfinished_matches;
            }
        }

        self.matchup_metrics.push(metrics);
    }

    fn push_match_diagnostics(
        &mut self,
        report: &MatchReport,
        combo_a: &OperatorCombo,
        combo_b: &OperatorCombo,
    ) {
        let label_a = combo_a.label();
        let label_b = combo_b.label();

        // Determine which player corresponds to which combo in this (possibly seat-swapped) match.
        let (player_id_for_a, player_id_for_b) = if report.deck_a == combo_a.deck_id {
            ("p1", "p2")
        } else {
            ("p2", "p1")
        };

        for summary in &report.final_state {
            if summary.player_id == player_id_for_a {
                if let Some(acc) = self.diagnostic_rollups.get_mut(&label_a) {
                    acc.record_sample(summary, report, summary.player_id == "p1");
                }
            } else if summary.player_id == player_id_for_b {
                if let Some(acc) = self.diagnostic_rollups.get_mut(&label_b) {
                    acc.record_sample(summary, report, summary.player_id == "p1");
                }
            }
        }
    }

    fn finish(self) -> OperatorMatrixReport {
        let operator_metrics: Vec<OperatorMetrics> = self
            .combo_labels
            .iter()
            .filter_map(|label| {
                let acc = self.operator_rollups.get(label)?;
                let deck_id = self.combo_decks.get(label)?.clone();
                let operator_id = label.strip_prefix(&format!("{deck_id}:"))?.to_string();
                let diagnostics = self
                    .diagnostic_rollups
                    .get(label)
                    .cloned()
                    .and_then(|d| d.into_diagnostics());
                Some(OperatorMetrics {
                    combo_label: label.clone(),
                    deck_id,
                    operator_id,
                    matches: acc.matches,
                    finished_matches: acc.finished_matches,
                    decisive_matches: acc.decisive_matches,
                    drawn_matches: acc.drawn_matches,
                    wins: acc.wins,
                    losses: acc.losses,
                    unfinished_matches: acc.unfinished_matches,
                    win_rate: rate(acc.wins, acc.decisive_matches),
                    diagnostics,
                })
            })
            .collect();

        // Build color metrics with intra-color spread
        let mut color_accumulators: BTreeMap<String, OperatorAccumulator> = BTreeMap::new();
        let mut color_win_rates: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        for om in &operator_metrics {
            let color_acc = color_accumulators.entry(om.deck_id.clone()).or_default();
            color_acc.matches += om.matches;
            color_acc.finished_matches += om.finished_matches;
            color_acc.decisive_matches += om.decisive_matches;
            color_acc.drawn_matches += om.drawn_matches;
            color_acc.wins += om.wins;
            color_acc.losses += om.losses;
            color_acc.unfinished_matches += om.unfinished_matches;

            if let Some(wr) = om.win_rate {
                color_win_rates
                    .entry(om.deck_id.clone())
                    .or_default()
                    .push(wr);
            }
        }

        let color_metrics: Vec<ColorMetrics> = self
            .config
            .decks
            .iter()
            .filter_map(|deck_id| {
                let acc = color_accumulators.remove(deck_id)?;
                let rates = color_win_rates.get(deck_id);
                let spread = rates.and_then(|rs| {
                    if rs.len() < 2 {
                        return None;
                    }
                    let min = rs.iter().cloned().fold(f64::INFINITY, f64::min);
                    let max = rs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    Some(max - min)
                });

                Some(ColorMetrics {
                    color: deck_id.clone(),
                    matches: acc.matches,
                    finished_matches: acc.finished_matches,
                    decisive_matches: acc.decisive_matches,
                    drawn_matches: acc.drawn_matches,
                    wins: acc.wins,
                    losses: acc.losses,
                    win_rate: rate(acc.wins, acc.decisive_matches),
                    intra_color_spread: spread,
                })
            })
            .collect();

        // Build fake DeckMetrics/MatchupMetrics for balance calculation
        let deck_metrics_for_balance: Vec<DeckMetrics> = operator_metrics
            .iter()
            .map(|om| DeckMetrics {
                deck_id: om.combo_label.clone(),
                matches: om.matches,
                finished_matches: om.finished_matches,
                decisive_matches: om.decisive_matches,
                drawn_matches: om.drawn_matches,
                wins: om.wins,
                losses: om.losses,
                unfinished_matches: om.unfinished_matches,
                win_rate: om.win_rate,
            })
            .collect();

        let matchup_metrics_for_balance: Vec<MatchupMetrics> = self
            .matchup_metrics
            .iter()
            .map(|mm| MatchupMetrics {
                deck_a: mm.combo_a.clone(),
                deck_b: mm.combo_b.clone(),
                matches: mm.matches,
                finished_matches: mm.finished_matches,
                decisive_matches: mm.decisive_matches,
                drawn_matches: mm.drawn_matches,
                unfinished_matches: mm.unfinished_matches,
                wins_by_deck: vec![
                    (mm.combo_a.clone(), mm.wins_a),
                    (mm.combo_b.clone(), mm.wins_b),
                ],
                first_player_wins: mm.first_player_wins,
                first_player_win_rate: mm.first_player_win_rate,
                average_rounds: mm.average_rounds,
                average_actions: mm.average_actions,
                deck_a_win_rate: mm.combo_a_win_rate,
                termination_counts: mm.termination_counts.clone(),
            })
            .collect();

        let balance = self
            .global
            .into_balance(&deck_metrics_for_balance, &matchup_metrics_for_balance);

        OperatorMatrixReport {
            combos: self.combo_labels,
            matches_per_pair: self.config.matches_per_pair,
            alternate_seats: self.config.alternate_seats,
            include_mirrors: self.config.include_mirrors,
            operator_metrics,
            color_metrics,
            matchup_metrics: self.matchup_metrics,
            balance,
        }
    }
}
