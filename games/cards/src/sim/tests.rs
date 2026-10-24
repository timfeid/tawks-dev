use std::sync::{Mutex, OnceLock};

use crate::action::CardGameAction;
use crate::types::ManaType;

use super::action_gen::sort_actions_deterministically;
use super::agent::{AgentProfile, profile_for_deck};
use super::config::{MatchConfig, MatrixConfig, ParticipantConfig};
use super::report::MatchReport;
use super::report::MatrixReport;
use super::report::TerminationReason;
use super::report::action_label;
use super::runner::{run_match, run_matrix};
use crate::types::GameFormat;

const SIM_TEST_STACK_SIZE: usize = 256 * 1024 * 1024;
static SIM_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn profile_defaults_follow_deck_identity() {
    assert_eq!(profile_for_deck("neon"), AgentProfile::Neon);
    assert_eq!(profile_for_deck("glitch"), AgentProfile::Glitch);
    assert_eq!(profile_for_deck("chrome"), AgentProfile::Chrome);
    assert_eq!(profile_for_deck("slag"), AgentProfile::Slag);
    assert_eq!(profile_for_deck("bio"), AgentProfile::Bio);
    assert_eq!(profile_for_deck("unknown"), AgentProfile::Generalist);
}

#[test]
fn seeded_match_is_deterministic() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new("p1", "neon", AgentProfile::Neon, None, false),
            ParticipantConfig::new("p2", "chrome", AgentProfile::Chrome, None, false),
        ],
        patch_path: None,
        mcts_iterations: 48,
        mcts_rollout_depth: 10,
        seed: 17,
        format: GameFormat::Classic,
        max_turns: 40,
        max_actions: 800,
        repeat_state_limit: 6,
        include_trace: false,
        enable_sideboard: false,
    };

    let first = run_match_with_large_stack(config.clone());
    let second = run_match_with_large_stack(config);

    assert_eq!(first.winner_deck_id, second.winner_deck_id);
    assert_eq!(first.actions_taken, second.actions_taken);
    assert_eq!(first.termination_reason, second.termination_reason);
}

#[test]
fn seeded_operator_match_is_deterministic() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new(
                "p1",
                "neon",
                AgentProfile::Neon,
                Some("echo_signal".to_string()),
                false,
            ),
            ParticipantConfig::new(
                "p2",
                "bio",
                AgentProfile::Bio,
                Some("verdant_warlord".to_string()),
                false,
            ),
        ],
        patch_path: None,
        mcts_iterations: 48,
        mcts_rollout_depth: 10,
        seed: 1,
        format: GameFormat::Operator,
        max_turns: 20,
        max_actions: 400,
        repeat_state_limit: 6,
        include_trace: false,
        enable_sideboard: false,
    };

    let first = run_match_with_large_stack(config.clone());
    let second = run_match_with_large_stack(config);

    assert_eq!(first.winner_deck_id, second.winner_deck_id);
    assert_eq!(first.rounds, second.rounds);
    assert_eq!(first.actions_taken, second.actions_taken);
    assert_eq!(first.termination_reason, second.termination_reason);
}

#[test]
fn echo_signal_vs_corruptor_seed_six_finishes_without_no_legal_action() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new(
                "p1",
                "glitch",
                AgentProfile::Glitch,
                Some("corruptor_of_data".to_string()),
                false,
            ),
            ParticipantConfig::new(
                "p2",
                "neon",
                AgentProfile::Neon,
                Some("echo_signal".to_string()),
                false,
            ),
        ],
        patch_path: None,
        mcts_iterations: 48,
        mcts_rollout_depth: 10,
        seed: 6,
        format: GameFormat::Operator,
        max_turns: 80,
        max_actions: 1_500,
        repeat_state_limit: 6,
        include_trace: false,
        enable_sideboard: false,
    };

    let report = run_match_with_large_stack(config);
    assert_ne!(report.termination_reason, TerminationReason::NoLegalAction);
}

#[test]
fn flux_oracle_vs_saheeli_seed_365_finishes_without_no_legal_action() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new(
                "p1",
                "neon",
                AgentProfile::Neon,
                Some("flux_oracle".to_string()),
                false,
            ),
            ParticipantConfig::new(
                "p2",
                "chrome",
                AgentProfile::Chrome,
                Some("saheeli_adept".to_string()),
                false,
            ),
        ],
        patch_path: None,
        mcts_iterations: 48,
        mcts_rollout_depth: 10,
        seed: 365,
        format: GameFormat::Operator,
        max_turns: 80,
        max_actions: 1_500,
        repeat_state_limit: 6,
        include_trace: false,
        enable_sideboard: false,
    };

    let report = run_match_with_large_stack(config);
    assert_ne!(report.termination_reason, TerminationReason::NoLegalAction);
}

#[test]
fn smoke_match_terminates_cleanly() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new("p1", "bio", AgentProfile::Bio, None, false),
            ParticipantConfig::new("p2", "slag", AgentProfile::Slag, None, false),
        ],
        patch_path: None,
        mcts_iterations: 48,
        mcts_rollout_depth: 10,
        seed: 9,
        format: GameFormat::Classic,
        max_turns: 50,
        max_actions: 1_000,
        repeat_state_limit: 6,
        include_trace: false,
        enable_sideboard: false,
    };

    let report = run_match_with_large_stack(config);
    assert_ne!(report.termination_reason, TerminationReason::UpdateError);
    assert_ne!(report.termination_reason, TerminationReason::NoLegalAction);
}

#[test]
fn traced_match_emits_snapshots() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new("p1", "bio", AgentProfile::Bio, None, false),
            ParticipantConfig::new("p2", "slag", AgentProfile::Slag, None, false),
        ],
        patch_path: None,
        mcts_iterations: 48,
        mcts_rollout_depth: 10,
        seed: 9,
        format: GameFormat::Classic,
        max_turns: 20,
        max_actions: 400,
        repeat_state_limit: 6,
        include_trace: true,
        enable_sideboard: false,
    };

    let report = run_match_with_large_stack(config);
    assert!(!report.snapshots.is_empty());
    assert!(!report.trace.is_empty());
}

#[test]
fn mcts_smoke_match_terminates_cleanly() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new("p1", "neon", AgentProfile::Neon, None, true),
            ParticipantConfig::new("p2", "chrome", AgentProfile::Chrome, None, false),
        ],
        patch_path: None,
        mcts_iterations: 8,
        mcts_rollout_depth: 6,
        seed: 17,
        format: GameFormat::Classic,
        max_turns: 20,
        max_actions: 400,
        repeat_state_limit: 6,
        include_trace: false,
        enable_sideboard: false,
    };

    let report = run_match_with_large_stack(config);
    assert_ne!(report.termination_reason, TerminationReason::UpdateError);
    assert_ne!(report.termination_reason, TerminationReason::NoLegalAction);
}

#[test]
fn seeded_mcts_match_is_deterministic() {
    let config = MatchConfig {
        participants: vec![
            ParticipantConfig::new("p1", "neon", AgentProfile::Neon, None, true),
            ParticipantConfig::new("p2", "chrome", AgentProfile::Chrome, None, false),
        ],
        patch_path: None,
        mcts_iterations: 8,
        mcts_rollout_depth: 6,
        seed: 17,
        format: GameFormat::Classic,
        max_turns: 20,
        max_actions: 400,
        repeat_state_limit: 6,
        include_trace: false,
        enable_sideboard: false,
    };

    let first = run_match_with_large_stack(config.clone());
    let second = run_match_with_large_stack(config);

    assert_eq!(first.winner_deck_id, second.winner_deck_id);
    assert_eq!(first.rounds, second.rounds);
    assert_eq!(first.actions_taken, second.actions_taken);
    assert_eq!(first.termination_reason, second.termination_reason);
}

#[test]
fn matrix_report_includes_pairwise_balance_metrics() {
    let mut config = MatrixConfig::default();
    config.decks = vec!["bio".into(), "slag".into(), "chrome".into()];
    config.matches_per_pair = 2;
    config.base_match.seed = 5;
    config.base_match.max_turns = 20;
    config.base_match.max_actions = 400;
    config.base_match.include_trace = true;

    let report = run_matrix_with_large_stack(config);
    assert_eq!(report.deck_metrics.len(), 3);
    assert_eq!(report.matchup_metrics.len(), 3);
    assert!(report.balance.finished_match_rate > 0.0);
    assert!(
        report
            .matchup_metrics
            .iter()
            .all(|matchup| matchup.matches == 2)
    );
}

#[test]
fn deterministic_action_sort_uses_action_labels() {
    let mut actions = vec![
        CardGameAction::PassPriority,
        CardGameAction::DeployNode {
            mana_type: ManaType::Chrome,
        },
        CardGameAction::CastSpell {
            card_id: "zeta".into(),
            targets: Vec::new(),
        },
        CardGameAction::CastSpell {
            card_id: "alpha".into(),
            targets: Vec::new(),
        },
        CardGameAction::NextStage,
    ];

    sort_actions_deterministically(&mut actions);

    let labels = actions.iter().map(action_label).collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "cast_spell:alpha:none",
            "cast_spell:zeta:none",
            "deploy_node:Chrome",
            "next_stage",
            "pass_priority",
        ]
    );
}

fn run_match_with_large_stack(config: MatchConfig) -> MatchReport {
    let _guard = SIM_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("lock sim test mutex");

    std::thread::Builder::new()
        .name("cardgame-sim-test".to_string())
        .stack_size(SIM_TEST_STACK_SIZE)
        .spawn(move || run_match(&config))
        .expect("spawn sim test thread")
        .join()
        .expect("join sim test thread")
}

fn run_matrix_with_large_stack(config: MatrixConfig) -> MatrixReport {
    let _guard = SIM_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("lock sim test mutex");

    std::thread::Builder::new()
        .name("cardgame-matrix-test".to_string())
        .stack_size(SIM_TEST_STACK_SIZE)
        .spawn(move || run_matrix(&config))
        .expect("spawn matrix test thread")
        .join()
        .expect("join matrix test thread")
}

/// Regression guard: a seeded match across representative deck pairings
/// should not fill the trace with the AI targeting its own permanents.
/// Protects against future regressions of the targeting-logic fixes
/// (sim_force_resolve_pending, depth-propagated score bonuses,
/// friendly-fire penalty, threat-score). Threshold is loose to survive
/// legitimate self-bounce-for-ETB-value plays.
#[test]
fn ai_rarely_self_targets_hostile_spells() {
    fn self_target_count(trace: &[super::report::ActionTraceEntry]) -> usize {
        trace
            .iter()
            .filter(|entry| {
                // Trace format example:
                //   cast_spell:p1-derez-9:card:p1-lord_of_the_grid-12
                let action = &entry.action;
                if !action.starts_with("cast_spell:") || !action.contains(":card:") {
                    return false;
                }
                let parts: Vec<&str> = action.split(':').collect();
                if parts.len() < 4 {
                    return false;
                }
                let spell_owner = parts[1].split('-').next().unwrap_or("");
                let target_owner = parts[3].split('-').next().unwrap_or("");
                !spell_owner.is_empty() && !target_owner.is_empty() && spell_owner == target_owner
            })
            .count()
    }

    let pairings: &[(&str, &str, AgentProfile, AgentProfile)] = &[
        ("neon", "slag", AgentProfile::Neon, AgentProfile::Slag),
        ("neon", "chrome", AgentProfile::Neon, AgentProfile::Chrome),
        (
            "glitch",
            "chrome",
            AgentProfile::Glitch,
            AgentProfile::Chrome,
        ),
        ("chrome", "slag", AgentProfile::Chrome, AgentProfile::Slag),
        ("glitch", "bio", AgentProfile::Glitch, AgentProfile::Bio),
    ];

    let seeds: &[u64] = &[1, 7, 42, 100, 300];
    let mut total_self_targets = 0usize;
    let mut matches_run = 0usize;
    for (deck_a, deck_b, agent_a, agent_b) in pairings {
        for seed in seeds {
            let config = MatchConfig {
                participants: vec![
                    ParticipantConfig::new("p1", *deck_a, agent_a.clone(), None, false),
                    ParticipantConfig::new("p2", *deck_b, agent_b.clone(), None, false),
                ],
                patch_path: None,
                mcts_iterations: 48,
                mcts_rollout_depth: 10,
                seed: *seed,
                format: GameFormat::Classic,
                max_turns: 30,
                max_actions: 600,
                repeat_state_limit: 6,
                include_trace: true,
                enable_sideboard: false,
            };
            let report = run_match_with_large_stack(config);
            total_self_targets += self_target_count(&report.trace);
            matches_run += 1;
        }
    }

    // Baseline before the targeting fixes averaged ~1.4 self-targets per
    // match. After the fixes (sim_force_resolve_pending, depth-propagated
    // score bonuses, friendly-fire penalty, threat-scoring) the rate
    // should stay well under the pre-fix baseline. Leave slack for
    // catalog tweaks — normal balance changes shouldn't regress above
    // 1.2/match.
    let avg = total_self_targets as f64 / matches_run as f64;
    assert!(
        avg < 1.2,
        "AI self-targeted hostile spells {total_self_targets} times across {matches_run} matches (avg {avg:.2}/match); target should be < 1.2/match"
    );
}
