use serde::{Deserialize, Serialize};

use super::report::{ActionTraceEntry, MatchReport, TerminationReason};

/// A flag indicating a specific type of degenerate game behavior was detected.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DegeneracyFlag {
    /// Game ended on or before round 3 with a decisive winner.
    EarlyKill,
    /// 8+ consecutive stack-related actions in a row (storm-like combo).
    StackStorm,
    /// A repeating cycle of identical actions was detected in the trace.
    RepetitiveTrace,
    /// 10+ consecutive rounds with no combat interaction (no attacks declared).
    NoInteractionLock,
    /// Game terminated due to repeated state (loop detection).
    RepeatedStateTermination,
}

impl std::fmt::Display for DegeneracyFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EarlyKill => write!(f, "early_kill"),
            Self::StackStorm => write!(f, "stack_storm"),
            Self::RepetitiveTrace => write!(f, "repetitive_trace"),
            Self::NoInteractionLock => write!(f, "no_interaction_lock"),
            Self::RepeatedStateTermination => write!(f, "repeated_state_termination"),
        }
    }
}

/// A bundle containing everything needed to reproduce a flagged game.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReproBundle {
    pub seed: u64,
    pub deck_a: String,
    pub deck_b: String,
    pub flags: Vec<DegeneracyFlag>,
    pub termination_reason: TerminationReason,
    pub rounds: usize,
    pub actions_taken: usize,
    pub trace: Vec<ActionTraceEntry>,
}

/// Per-flag count for aggregated reporting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DegeneracyFlagCount {
    pub flag: DegeneracyFlag,
    pub count: usize,
}

/// Aggregated degeneracy summary across a matrix or series run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DegeneracySummary {
    pub flagged_matches: usize,
    pub total_matches: usize,
    pub flag_counts: Vec<DegeneracyFlagCount>,
    pub repro_bundles: Vec<ReproBundle>,
}

/// Run all degeneracy detectors on a completed match report.
/// Returns the list of flags found (empty if the game was clean).
pub fn detect_degeneracy(report: &MatchReport) -> Vec<DegeneracyFlag> {
    let mut flags = Vec::new();

    if detect_early_kill(report) {
        flags.push(DegeneracyFlag::EarlyKill);
    }
    if detect_stack_storm(&report.trace) {
        flags.push(DegeneracyFlag::StackStorm);
    }
    if detect_repetitive_trace(&report.trace) {
        flags.push(DegeneracyFlag::RepetitiveTrace);
    }
    if detect_no_interaction_lock(report) {
        flags.push(DegeneracyFlag::NoInteractionLock);
    }
    if detect_repeated_state_termination(report) {
        flags.push(DegeneracyFlag::RepeatedStateTermination);
    }

    flags.sort();
    flags.dedup();
    flags
}

/// Build a repro bundle from a flagged match report.
pub fn build_repro_bundle(report: &MatchReport, flags: Vec<DegeneracyFlag>) -> ReproBundle {
    ReproBundle {
        seed: report.seed,
        deck_a: report.deck_a.clone(),
        deck_b: report.deck_b.clone(),
        flags,
        termination_reason: report.termination_reason.clone(),
        rounds: report.rounds,
        actions_taken: report.actions_taken,
        trace: report.trace.clone(),
    }
}

/// Build an aggregated summary from a collection of match reports.
pub fn build_degeneracy_summary(reports: &[MatchReport]) -> DegeneracySummary {
    let mut flag_counts = std::collections::BTreeMap::new();
    let mut repro_bundles = Vec::new();
    let mut flagged_matches = 0;

    for report in reports {
        let flags = detect_degeneracy(report);
        if !flags.is_empty() {
            flagged_matches += 1;
            for flag in &flags {
                *flag_counts.entry(flag.clone()).or_insert(0usize) += 1;
            }
            repro_bundles.push(build_repro_bundle(report, flags));
        }
    }

    DegeneracySummary {
        flagged_matches,
        total_matches: reports.len(),
        flag_counts: flag_counts
            .into_iter()
            .map(|(flag, count)| DegeneracyFlagCount { flag, count })
            .collect(),
        repro_bundles,
    }
}

// ── Detectors ────────────────────────────────────────────────────────────────

/// Game ended by round 3 with a decisive winner.
fn detect_early_kill(report: &MatchReport) -> bool {
    report.rounds <= 3
        && report.winner_player_id.is_some()
        && report.termination_reason == TerminationReason::Finished
}

/// 8+ consecutive stack-related actions (cast_spell, resolve_stack_top, activate_ability).
fn detect_stack_storm(trace: &[ActionTraceEntry]) -> bool {
    const STORM_THRESHOLD: usize = 8;

    let mut consecutive = 0usize;
    for entry in trace {
        if is_stack_action(&entry.action) {
            consecutive += 1;
            if consecutive >= STORM_THRESHOLD {
                return true;
            }
        } else {
            consecutive = 0;
        }
    }
    false
}

fn is_stack_action(action: &str) -> bool {
    action.starts_with("cast_spell:")
        || action.starts_with("activate_ability:")
        || action == "resolve_stack_top"
}

/// Detect repeating cycles of actions in the trace (cycle length 2..=6).
fn detect_repetitive_trace(trace: &[ActionTraceEntry]) -> bool {
    const MIN_REPETITIONS: usize = 3;

    let actions: Vec<&str> = trace.iter().map(|e| e.action.as_str()).collect();
    if actions.len() < 6 {
        return false;
    }

    for cycle_len in 2..=6 {
        let min_window = cycle_len * MIN_REPETITIONS;
        if actions.len() < min_window {
            continue;
        }

        // Check from the end of the trace for repeating cycles
        let tail = &actions[actions.len() - min_window..];
        let cycle = &tail[..cycle_len];
        let all_match = (0..MIN_REPETITIONS).all(|rep| {
            let offset = rep * cycle_len;
            (0..cycle_len).all(|i| tail[offset + i] == cycle[i])
        });
        if all_match {
            return true;
        }
    }
    false
}

/// 10+ consecutive rounds with no attack actions anywhere in the trace.
fn detect_no_interaction_lock(report: &MatchReport) -> bool {
    const NO_INTERACTION_THRESHOLD: usize = 10;

    if report.snapshots.len() < NO_INTERACTION_THRESHOLD {
        return false;
    }

    // Use snapshots to track which rounds had attacks
    let mut attack_rounds = std::collections::HashSet::new();

    let mut snapshot_index = 0;
    for entry in &report.trace {
        while snapshot_index + 1 < report.snapshots.len()
            && entry.index > snapshot_action_boundary(&report.snapshots, snapshot_index)
        {
            snapshot_index += 1;
        }
        if entry.action.starts_with("declare_attack:") || entry.action == "confirm_attackers" {
            let round = if snapshot_index < report.snapshots.len() {
                report.snapshots[snapshot_index].round
            } else {
                report.rounds
            };
            attack_rounds.insert(round);
        }
    }

    // Count maximum consecutive rounds without attacks
    let max_round = report.rounds;
    let mut max_consecutive = 0;
    let mut consecutive = 0;
    for round in 1..=max_round {
        if attack_rounds.contains(&round) {
            consecutive = 0;
        } else {
            consecutive += 1;
            max_consecutive = max_consecutive.max(consecutive);
        }
    }

    max_consecutive >= NO_INTERACTION_THRESHOLD
}

/// Rough boundary: action index where the next snapshot starts.
fn snapshot_action_boundary(_snapshots: &[super::report::RoundSnapshot], index: usize) -> usize {
    index + 1
}

/// Game terminated due to repeated state detection.
fn detect_repeated_state_termination(report: &MatchReport) -> bool {
    report.termination_reason == TerminationReason::LoopDetected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::agent::AgentProfile;
    use crate::sim::report::{MatchReport, TerminationReason};
    

    fn base_report() -> MatchReport {
        MatchReport {
            seed: 42,
            deck_a: "neon".into(),
            deck_b: "slag".into(),
            agent_a: AgentProfile::Neon,
            agent_b: AgentProfile::Slag,
            winner_player_id: Some("p1".into()),
            winner_deck_id: Some("neon".into()),
            drawn: false,
            rounds: 10,
            actions_taken: 50,
            termination_reason: TerminationReason::Finished,
            error: None,
            final_state: vec![],
            snapshots: vec![],
            trace: vec![],
            degeneracy_flags: vec![],
            card_metrics: vec![],
        }
    }

    fn trace_entry(index: usize, action: &str) -> ActionTraceEntry {
        ActionTraceEntry {
            index,
            actor_id: "p1".into(),
            action: action.into(),
        }
    }

    #[test]
    fn early_kill_detected_on_round_3_win() {
        let mut report = base_report();
        report.rounds = 3;
        let flags = detect_degeneracy(&report);
        assert!(flags.contains(&DegeneracyFlag::EarlyKill));
    }

    #[test]
    fn no_early_kill_on_round_5() {
        let report = base_report(); // rounds = 10
        let flags = detect_degeneracy(&report);
        assert!(!flags.contains(&DegeneracyFlag::EarlyKill));
    }

    #[test]
    fn no_early_kill_without_winner() {
        let mut report = base_report();
        report.rounds = 2;
        report.winner_player_id = None;
        report.winner_deck_id = None;
        report.termination_reason = TerminationReason::MaxTurnsExceeded;
        let flags = detect_degeneracy(&report);
        assert!(!flags.contains(&DegeneracyFlag::EarlyKill));
    }

    #[test]
    fn stack_storm_detected() {
        let mut report = base_report();
        report.trace = (0..10)
            .map(|i| {
                trace_entry(
                    i,
                    if i % 2 == 0 {
                        "cast_spell:fireball:player:p2"
                    } else {
                        "resolve_stack_top"
                    },
                )
            })
            .collect();
        let flags = detect_degeneracy(&report);
        assert!(flags.contains(&DegeneracyFlag::StackStorm));
    }

    #[test]
    fn no_stack_storm_with_interruptions() {
        let mut report = base_report();
        report.trace = vec![
            trace_entry(0, "cast_spell:x:none"),
            trace_entry(1, "resolve_stack_top"),
            trace_entry(2, "cast_spell:y:none"),
            trace_entry(3, "next_stage"),
            trace_entry(4, "cast_spell:z:none"),
            trace_entry(5, "resolve_stack_top"),
        ];
        let flags = detect_degeneracy(&report);
        assert!(!flags.contains(&DegeneracyFlag::StackStorm));
    }

    #[test]
    fn repetitive_trace_detected() {
        let mut report = base_report();
        // Create a repeating cycle of length 2, repeated 3 times
        let mut trace = Vec::new();
        for i in 0..6 {
            trace.push(trace_entry(
                i,
                if i % 2 == 0 {
                    "pass_priority"
                } else {
                    "next_stage"
                },
            ));
        }
        report.trace = trace;
        let flags = detect_degeneracy(&report);
        assert!(flags.contains(&DegeneracyFlag::RepetitiveTrace));
    }

    #[test]
    fn repeated_state_termination_detected() {
        let mut report = base_report();
        report.termination_reason = TerminationReason::LoopDetected;
        report.winner_player_id = None;
        report.winner_deck_id = None;
        let flags = detect_degeneracy(&report);
        assert!(flags.contains(&DegeneracyFlag::RepeatedStateTermination));
    }

    #[test]
    fn clean_game_has_no_flags() {
        let report = base_report();
        let flags = detect_degeneracy(&report);
        assert!(flags.is_empty());
    }

    #[test]
    fn build_repro_bundle_captures_all_fields() {
        let mut report = base_report();
        report.trace = vec![trace_entry(0, "pass_priority")];
        let flags = vec![DegeneracyFlag::EarlyKill];
        let bundle = build_repro_bundle(&report, flags.clone());
        assert_eq!(bundle.seed, 42);
        assert_eq!(bundle.deck_a, "neon");
        assert_eq!(bundle.deck_b, "slag");
        assert_eq!(bundle.flags, flags);
        assert_eq!(bundle.trace.len(), 1);
    }

    #[test]
    fn summary_aggregates_correctly() {
        let mut report_a = base_report();
        report_a.rounds = 2; // will trigger early_kill

        let report_b = base_report(); // clean

        let mut report_c = base_report();
        report_c.termination_reason = TerminationReason::LoopDetected;
        report_c.winner_player_id = None;
        report_c.winner_deck_id = None;

        let summary = build_degeneracy_summary(&[report_a, report_b, report_c]);
        assert_eq!(summary.total_matches, 3);
        assert_eq!(summary.flagged_matches, 2);
        assert_eq!(summary.repro_bundles.len(), 2);
    }
}
