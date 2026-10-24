use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use cardgame::sim::agent::{AgentProfile, profile_for_deck};
use cardgame::sim::config::{
    GauntletConfig, MatchConfig, MatrixConfig, OperatorMatrixConfig,
};
use cardgame::sim::degeneracy::build_degeneracy_summary;
use cardgame::sim::patch::{build_patch_evaluation_report, load_balance_patch_from_path};
use cardgame::sim::report::{
    MatchReport, MatrixReport, OperatorMatrixReport, PatchEvaluationReport, SeriesReport,
    TerminationReason,
};
use cardgame::sim::runner::{
    MatchupAccumulator, OperatorMatchupAccumulator, build_matrix_report_from_matchups,
    build_operator_matrix_matchups, build_operator_matrix_report_from_matchups,
    build_series_report, run_match, run_matrix, run_operator_matrix, run_series,
};
use cardgame::types::GameFormat;

const DEFAULT_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_CHILD_STACK_MB: u64 = 64;
const DEFAULT_CHILD_MEMORY_MB: u64 = 1024;
const CHILD_POLL_INTERVAL_MS: u64 = 25;

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .max(1)
}

#[derive(Clone, Debug)]
struct SafetyConfig {
    timeout_seconds: u64,
    child_stack_mb: u64,
    child_memory_mb: u64,
    jobs: usize,
    unsafe_in_process: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
            child_stack_mb: DEFAULT_CHILD_STACK_MB,
            child_memory_mb: DEFAULT_CHILD_MEMORY_MB,
            jobs: default_jobs(),
            unsafe_in_process: false,
        }
    }
}

#[derive(Clone, Debug)]
struct ScheduledMatch {
    sequence: usize,
    group_index: usize,
    config: MatchConfig,
}

#[derive(Debug)]
struct CompletedScheduledMatch {
    sequence: usize,
    group_index: usize,
    report: MatchReport,
}

#[derive(Clone, Debug)]
struct MatchupPlan {
    deck_a: String,
    deck_b: String,
    matches: Vec<MatchConfig>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(());
    }

    let mut config = GauntletConfig::default();
    let mut deck_a: Option<String> = None;
    let mut deck_b: Option<String> = None;
    let mut agent_a: Option<AgentProfile> = None;
    let mut agent_b: Option<AgentProfile> = None;
    let mut matrix_mode = false;
    let mut operator_matrix_mode = false;
    let mut draft_mode = false;
    let mut draft_count: Option<usize> = None;
    let mut matches_per_draft: Option<usize> = None;
    let mut matrix_decks: Option<Vec<String>> = None;
    let mut operator_filter: Vec<String> = Vec::new();
    let mut include_mirrors = false;
    let mut internal_match_json = false;
    let mut degeneracy_mode = false;
    let mut patch_path: Option<String> = None;
    let mut evaluate_patch_path: Option<String> = None;
    let mut card_metrics_csv: Option<String> = None;
    let mut safety = SafetyConfig::default();
    let mut jobs_explicitly_set = false;

    let mut index = 0usize;
    while index < args.len() {
        let flag = &args[index];
        let value = args.get(index + 1).cloned();

        match flag.as_str() {
            "--internal-match-json" => {
                internal_match_json = true;
                index += 1;
                continue;
            }
            "--matrix" => {
                matrix_mode = true;
                index += 1;
                continue;
            }
            "--operator-matrix" => {
                operator_matrix_mode = true;
                index += 1;
                continue;
            }
            "--draft" => {
                draft_mode = true;
                index += 1;
                continue;
            }
            "--draft-count" => draft_count = Some(required_value(flag, value)?.parse()?),
            "--matches-per-draft" => {
                matches_per_draft = Some(required_value(flag, value)?.parse()?)
            }
            "--operators" => {
                operator_filter = parse_operator_list(&required_value(flag, value)?);
            }
            "--decks" => matrix_decks = Some(parse_deck_list(&required_value(flag, value)?)?),
            "--deck-a" => deck_a = Some(required_value(flag, value)?),
            "--deck-b" => deck_b = Some(required_value(flag, value)?),
            "--patch" => patch_path = Some(required_value(flag, value)?),
            "--evaluate-patch" => evaluate_patch_path = Some(required_value(flag, value)?),
            "--card-metrics-csv" => card_metrics_csv = Some(required_value(flag, value)?),
            "--agent-a" => agent_a = Some(parse_agent(&required_value(flag, value)?)?),
            "--agent-b" => agent_b = Some(parse_agent(&required_value(flag, value)?)?),
            "--mcts-a" => {
                config.base_match.participants[0].uses_mcts = true;
                index += 1;
                continue;
            }
            "--mcts-b" => {
                config.base_match.participants[1].uses_mcts = true;
                index += 1;
                continue;
            }
            "--mcts-iterations" => {
                config.base_match.mcts_iterations = required_value(flag, value)?.parse()?
            }
            "--mcts-rollout-depth" => {
                config.base_match.mcts_rollout_depth = required_value(flag, value)?.parse()?
            }
            "--seed" => config.base_match.seed = required_value(flag, value)?.parse()?,
            "--matches" => config.matches = required_value(flag, value)?.parse()?,
            "--max-turns" => config.base_match.max_turns = required_value(flag, value)?.parse()?,
            "--max-actions" => {
                config.base_match.max_actions = required_value(flag, value)?.parse()?
            }
            "--repeat-state-limit" => {
                config.base_match.repeat_state_limit = required_value(flag, value)?.parse()?
            }
            "--jobs" => {
                let jobs = parse_jobs(&required_value(flag, value)?)?;
                safety.jobs = jobs;
                jobs_explicitly_set = true;
            }
            "--timeout-seconds" => safety.timeout_seconds = required_value(flag, value)?.parse()?,
            "--child-stack-mb" => safety.child_stack_mb = required_value(flag, value)?.parse()?,
            "--child-memory-mb" => safety.child_memory_mb = required_value(flag, value)?.parse()?,
            "--trace" => {
                config.base_match.include_trace = true;
                index += 1;
                continue;
            }
            "--unsafe-in-process" => {
                safety.unsafe_in_process = true;
                index += 1;
                continue;
            }
            "--no-alternate-seats" => {
                config.alternate_seats = false;
                index += 1;
                continue;
            }
            "--include-mirrors" => {
                include_mirrors = true;
                index += 1;
                continue;
            }
            "--sideboard" => {
                config.base_match.enable_sideboard = true;
                index += 1;
                continue;
            }
            "--degeneracy" => {
                degeneracy_mode = true;
                config.base_match.include_trace = true;
                index += 1;
                continue;
            }
            "--classic" => {
                config.base_match.format = GameFormat::Classic;
                index += 1;
                continue;
            }
            "--operator" => {
                config.base_match.format = GameFormat::Operator;
                index += 1;
                continue;
            }
            "--operator-a" => {
                config.base_match.participants[0].operator = Some(required_value(flag, value)?);
            }
            "--operator-b" => {
                config.base_match.participants[1].operator = Some(required_value(flag, value)?);
            }
            other => return Err(anyhow!("Unknown flag: {other}")),
        }

        index += 2;
    }

    if let Some(deck) = deck_a {
        config.base_match.participants[0].deck_id = deck;
    }
    if let Some(deck) = deck_b {
        config.base_match.participants[1].deck_id = deck;
    }

    config.base_match.participants[0].agent =
        agent_a.unwrap_or_else(|| profile_for_deck(&config.base_match.participants[0].deck_id));
    config.base_match.participants[1].agent =
        agent_b.unwrap_or_else(|| profile_for_deck(&config.base_match.participants[1].deck_id));
    config.base_match.patch_path = patch_path.clone();

    if patch_path.is_some() && evaluate_patch_path.is_some() {
        return Err(anyhow!(
            "--patch and --evaluate-patch are mutually exclusive"
        ));
    }
    if let Some(ref patch_path) = patch_path {
        load_balance_patch_from_path(patch_path)?;
    }
    if let Some(ref patch_path) = evaluate_patch_path {
        load_balance_patch_from_path(patch_path)?;
    }

    let match_config = config.base_match.clone();

    if config.base_match.include_trace
        && !degeneracy_mode
        && (matrix_mode || operator_matrix_mode || config.matches > 1)
    {
        return Err(anyhow!(
            "--trace is only supported for single-match runs to keep memory bounded"
        ));
    }

    if internal_match_json {
        let report = run_match(&match_config);
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }

    if safety.unsafe_in_process && jobs_explicitly_set {
        return Err(anyhow!(
            "--jobs is only supported in supervised mode; remove --unsafe-in-process to use parallel workers safely"
        ));
    }
    if evaluate_patch_path.is_some() && operator_matrix_mode {
        return Err(anyhow!(
            "--evaluate-patch currently supports classic matrix mode only"
        ));
    }
    if evaluate_patch_path.is_some() && !matrix_mode {
        return Err(anyhow!("--evaluate-patch requires --matrix"));
    }

    let current_exe = std::env::current_exe().context("resolve run_gauntlet binary path")?;

    if draft_mode {
        use cardgame::sim::config::DraftMatrixConfig;
        use cardgame::sim::draft::run_sim_draft;
        use cardgame::sim::runner::{run_draft_matrix, run_match_with_decks};

        let mut d_config = DraftMatrixConfig::default();
        d_config.base_match = config.base_match.clone();
        d_config.base_seed = config.base_match.seed;
        d_config.alternate_seats = config.alternate_seats;
        d_config.draft_count = draft_count.unwrap_or(10);
        d_config.matches_per_draft = matches_per_draft.unwrap_or(4);

        // If the user wants per-card metrics we rerun the draft loop
        // and collect per-match card_metrics so they can be aggregated
        // to a CSV. Otherwise just emit the color-balance report.
        if let Some(path) = &card_metrics_csv {
            use std::collections::HashMap;
            // Dominant color = color of the card whose cost has the
            // most colored pips (summed across the deck). Ignores
            // colorless since every deck has plenty of colorless cost.
            fn dominant_color(deck: &[cardgame::card::Card]) -> String {
                let mut counts: HashMap<&'static str, i32> = HashMap::new();
                for card in deck {
                    let cost = &card.cost;
                    *counts.entry("neon").or_default() += cost.neon;
                    *counts.entry("glitch").or_default() += cost.glitch;
                    *counts.entry("chrome").or_default() += cost.chrome;
                    *counts.entry("slag").or_default() += cost.slag;
                    *counts.entry("bio").or_default() += cost.bio;
                }
                counts
                    .into_iter()
                    .max_by_key(|(_, c)| *c)
                    .filter(|(_, c)| *c > 0)
                    .map(|(name, _)| name.to_string())
                    .unwrap_or_else(|| "colorless".to_string())
            }
            use rayon::prelude::*;
            use std::sync::atomic::{AtomicUsize, Ordering};

            struct MetricsWorkItem {
                match_config: cardgame::sim::config::MatchConfig,
                deck_a: Vec<cardgame::card::Card>,
                deck_b: Vec<cardgame::card::Card>,
            }

            // Phase 1 (serial): build work items so parallel phase has no
            // shared mutable state.
            let mut work_items: Vec<MetricsWorkItem> = Vec::with_capacity(
                d_config.draft_count as usize * d_config.matches_per_draft as usize,
            );
            for draft_index in 0..d_config.draft_count {
                let draft_seed = d_config.base_seed + draft_index as u64 * 1000;
                let (deck_a, deck_b) = run_sim_draft(draft_seed);
                let color_a = dominant_color(&deck_a);
                let color_b = dominant_color(&deck_b);
                for match_index in 0..d_config.matches_per_draft {
                    let mut match_config = d_config.base_match.clone();
                    match_config.seed = draft_seed + match_index as u64 + 1;
                    // Tag deck_id with the dominant color so per-card
                    // analysis groups by the drafted deck's theme.
                    match_config.participants[0].deck_id = color_a.clone();
                    match_config.participants[1].deck_id = color_b.clone();
                    match_config.participants[0].agent =
                        cardgame::sim::agent::AgentProfile::Generalist;
                    match_config.participants[1].agent =
                        cardgame::sim::agent::AgentProfile::Generalist;
                    let (mut da, mut db) = (deck_a.clone(), deck_b.clone());
                    if d_config.alternate_seats && match_index % 2 == 1 {
                        match_config.participants.swap(0, 1);
                        std::mem::swap(&mut da, &mut db);
                    }
                    work_items.push(MetricsWorkItem {
                        match_config,
                        deck_a: da,
                        deck_b: db,
                    });
                }
            }

            // Phase 2 (parallel): run matches.
            let total_jobs = work_items.len();
            let done = AtomicUsize::new(0);
            let show_progress = std::env::var("CARDGAME_GAUNTLET_QUIET").is_err();
            let per_match: Vec<Vec<cardgame::sim::report::CardMetrics>> = work_items
                .into_par_iter()
                .with_min_len(1)
                .map(|w| {
                    let report = run_match_with_decks(&w.match_config, w.deck_a, w.deck_b, None);
                    if show_progress {
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        eprint!("\r[{}/{}] draft-metrics matches", n, total_jobs);
                    }
                    report.card_metrics
                })
                .collect();
            if show_progress && total_jobs > 0 {
                eprintln!();
            }
            write_card_metrics_csv(path, per_match)?;
        }

        let report = run_draft_matrix(&d_config);
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if operator_matrix_mode {
        let mut op_config = OperatorMatrixConfig::default();
        op_config.base_match = config.base_match.clone();
        op_config.base_match.format = GameFormat::Operator;
        op_config.matches_per_pair = config.matches.max(1);
        op_config.alternate_seats = config.alternate_seats;
        op_config.include_mirrors = include_mirrors;
        op_config.operator_filter = operator_filter;
        if let Some(ref decks) = matrix_decks {
            op_config.decks = decks.clone();
        }

        let report = if safety.unsafe_in_process {
            run_operator_matrix(&op_config)
        } else {
            run_operator_matrix_supervised(&current_exe, &op_config, &safety)
        };
        eprintln!(
            "[operator-matrix] {} combos, {} matchups, {} total matches",
            report.combos.len(),
            report.matchup_metrics.len(),
            report
                .matchup_metrics
                .iter()
                .map(|m| m.matches)
                .sum::<usize>()
        );
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if matrix_mode {
        let mut matrix_config = MatrixConfig::default();
        matrix_config.base_match = config.base_match.clone();
        matrix_config.matches_per_pair = config.matches.max(1);
        matrix_config.alternate_seats = config.alternate_seats;
        matrix_config.include_mirrors = include_mirrors;
        if let Some(decks) = matrix_decks {
            matrix_config.decks = decks;
        }

        if let Some(patch_path) = evaluate_patch_path {
            let report = evaluate_patch_matrix(&current_exe, &matrix_config, &safety, &patch_path);
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }

        let (report, per_match_metrics) = if safety.unsafe_in_process {
            let r = run_matrix(&matrix_config);
            // Unsafe in-process mode runs matches serially but we don't
            // retain per-match reports here; skip metrics dump.
            (r, Vec::<Vec<cardgame::sim::report::CardMetrics>>::new())
        } else {
            run_matrix_supervised_with_metrics(&current_exe, &matrix_config, &safety)
        };
        if degeneracy_mode {
            eprintln!(
                "[degeneracy] Matrix mode: per-match degeneracy_flags are included in each \
                 supervised child's MatchReport. Use series mode for full repro bundles."
            );
        }
        if let Some(path) = &card_metrics_csv {
            write_card_metrics_csv(path, per_match_metrics)?;
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if config.matches <= 1 {
        let report = if safety.unsafe_in_process {
            run_match(&match_config)
        } else {
            run_match_supervised(&current_exe, &match_config, &safety)
        };
        if degeneracy_mode && !report.degeneracy_flags.is_empty() {
            eprintln!("[degeneracy] Flags detected: {:?}", report.degeneracy_flags);
        }
        if let Some(path) = &card_metrics_csv {
            write_card_metrics_csv(path, vec![report.card_metrics.clone()])?;
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let (report, per_match_metrics) = if safety.unsafe_in_process {
            let r = run_series(&config);
            let metrics = r.results.iter().map(|m| m.card_metrics.clone()).collect();
            (r, metrics)
        } else {
            run_series_supervised_with_metrics(&current_exe, &config, &safety)
        };
        if degeneracy_mode {
            let summary = build_degeneracy_summary(&report.results);
            eprintln!(
                "[degeneracy] {}/{} matches flagged",
                summary.flagged_matches, summary.total_matches
            );
            for fc in &summary.flag_counts {
                eprintln!("  {}: {}", fc.flag, fc.count);
            }
            if !summary.repro_bundles.is_empty() {
                eprintln!(
                    "[degeneracy] Repro bundles: {}",
                    serde_json::to_string_pretty(&summary.repro_bundles)
                        .unwrap_or_else(|_| "<serialization error>".into())
                );
            }
        }
        if let Some(path) = &card_metrics_csv {
            write_card_metrics_csv(path, per_match_metrics)?;
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    Ok(())
}

fn write_card_metrics_csv(
    path: &str,
    per_match_metrics: Vec<Vec<cardgame::sim::report::CardMetrics>>,
) -> Result<()> {
    use cardgame::sim::card_metrics::{aggregate_card_metrics, render_card_metrics_csv};
    let aggregated = aggregate_card_metrics(per_match_metrics.into_iter());
    let csv = render_card_metrics_csv(&aggregated);
    std::fs::write(path, csv).with_context(|| format!("write card metrics csv to {path}"))?;
    eprintln!(
        "[card-metrics] Wrote {} aggregated rows to {}",
        aggregated.len(),
        path
    );
    Ok(())
}

fn run_series_supervised(
    current_exe: &Path,
    config: &GauntletConfig,
    safety: &SafetyConfig,
) -> SeriesReport {
    run_series_supervised_with_metrics(current_exe, config, safety).0
}

fn run_series_supervised_with_metrics(
    current_exe: &Path,
    config: &GauntletConfig,
    safety: &SafetyConfig,
) -> (SeriesReport, Vec<Vec<cardgame::sim::report::CardMetrics>>) {
    let tasks: Vec<_> = build_series_matches(config)
        .into_iter()
        .enumerate()
        .map(|(sequence, config)| ScheduledMatch {
            sequence,
            group_index: 0,
            config,
        })
        .collect();
    let completed = run_supervised_schedule(current_exe, tasks, safety);
    let mut ordered_results = Vec::with_capacity(completed.len());
    let mut per_match_metrics = Vec::with_capacity(completed.len());
    for completed in completed {
        per_match_metrics.push(completed.report.card_metrics.clone());
        ordered_results.push(completed.report);
    }

    (
        build_series_report(config, ordered_results),
        per_match_metrics,
    )
}

fn run_matrix_supervised(
    current_exe: &Path,
    config: &MatrixConfig,
    safety: &SafetyConfig,
) -> MatrixReport {
    run_matrix_supervised_with_metrics(current_exe, config, safety).0
}

fn run_matrix_supervised_with_metrics(
    current_exe: &Path,
    config: &MatrixConfig,
    safety: &SafetyConfig,
) -> (MatrixReport, Vec<Vec<cardgame::sim::report::CardMetrics>>) {
    let matchup_plans = build_matrix_matchups(config);
    let mut tasks = Vec::new();
    for (group_index, plan) in matchup_plans.iter().enumerate() {
        for match_config in &plan.matches {
            tasks.push(ScheduledMatch {
                sequence: tasks.len(),
                group_index,
                config: match_config.clone(),
            });
        }
    }
    let completed = run_supervised_schedule(current_exe, tasks, safety);

    let mut accumulators: Vec<_> = matchup_plans
        .iter()
        .map(|plan| MatchupAccumulator::new(plan.deck_a.clone(), plan.deck_b.clone()))
        .collect();
    let mut per_match_metrics = Vec::with_capacity(completed.len());
    for completed in completed {
        per_match_metrics.push(completed.report.card_metrics.clone());
        accumulators[completed.group_index].record(&completed.report);
    }
    let matchup_metrics = accumulators
        .into_iter()
        .map(MatchupAccumulator::finish)
        .collect();

    (
        build_matrix_report_from_matchups(config, matchup_metrics),
        per_match_metrics,
    )
}

fn evaluate_patch_matrix(
    current_exe: &Path,
    config: &MatrixConfig,
    safety: &SafetyConfig,
    patch_path: &str,
) -> PatchEvaluationReport {
    let baseline = if safety.unsafe_in_process {
        run_matrix(config)
    } else {
        run_matrix_supervised(current_exe, config, safety)
    };

    let mut patched_config = config.clone();
    patched_config.base_match.patch_path = Some(patch_path.to_string());
    let patched = if safety.unsafe_in_process {
        run_matrix(&patched_config)
    } else {
        run_matrix_supervised(current_exe, &patched_config, safety)
    };

    build_patch_evaluation_report(patch_path, baseline, patched)
}

fn run_operator_matrix_supervised(
    current_exe: &Path,
    config: &OperatorMatrixConfig,
    safety: &SafetyConfig,
) -> OperatorMatrixReport {
    let (combos, matchup_plans) = build_operator_matrix_matchups(config);
    let mut tasks = Vec::new();
    for (group_index, plan) in matchup_plans.iter().enumerate() {
        for match_config in &plan.matches {
            tasks.push(ScheduledMatch {
                sequence: tasks.len(),
                group_index,
                config: match_config.clone(),
            });
        }
    }

    eprintln!(
        "[operator-matrix] Scheduling {} matches across {} matchups ({} combos, {} workers)",
        tasks.len(),
        matchup_plans.len(),
        combos.len(),
        effective_jobs(safety, tasks.len())
    );

    let completed = run_supervised_schedule(current_exe, tasks, safety);

    let mut accumulators: Vec<_> = matchup_plans
        .iter()
        .map(|plan| OperatorMatchupAccumulator::new(plan.combo_a.label(), plan.combo_b.label()))
        .collect();
    for result in completed {
        let plan = &matchup_plans[result.group_index];
        accumulators[result.group_index].record(&result.report, &plan.combo_a, &plan.combo_b);
    }
    let matchup_metrics = accumulators
        .into_iter()
        .map(OperatorMatchupAccumulator::finish)
        .collect();

    build_operator_matrix_report_from_matchups(config, &combos, matchup_metrics)
}

fn build_series_matches(config: &GauntletConfig) -> Vec<MatchConfig> {
    let mut matches = Vec::with_capacity(config.matches);

    for index in 0..config.matches {
        let mut match_config = config.base_match.clone();
        match_config.seed = config.base_match.seed + index as u64;

        if config.alternate_seats && index % 2 == 1 {
            match_config.participants.swap(0, 1);
        }

        matches.push(match_config);
    }

    matches
}

fn build_matrix_matchups(config: &MatrixConfig) -> Vec<MatchupPlan> {
    let mut plans = Vec::new();

    for (left_index, deck_a) in config.decks.iter().enumerate() {
        let right_start = if config.include_mirrors {
            left_index
        } else {
            left_index + 1
        };

        for deck_b in config.decks.iter().skip(right_start) {
            let mut base_match = config.base_match.clone();
            base_match.participants[0].deck_id = deck_a.clone();
            base_match.participants[0].agent = profile_for_deck(deck_a);
            base_match.participants[1].deck_id = deck_b.clone();
            base_match.participants[1].agent = profile_for_deck(deck_b);
            let gauntlet = GauntletConfig {
                base_match,
                matches: config.matches_per_pair,
                alternate_seats: config.alternate_seats,
            };

            plans.push(MatchupPlan {
                deck_a: deck_a.clone(),
                deck_b: deck_b.clone(),
                matches: build_series_matches(&gauntlet),
            });
        }
    }

    plans
}

fn run_supervised_schedule(
    current_exe: &Path,
    tasks: Vec<ScheduledMatch>,
    safety: &SafetyConfig,
) -> Vec<CompletedScheduledMatch> {
    if tasks.is_empty() {
        return Vec::new();
    }

    if effective_jobs(safety, tasks.len()) == 1 {
        let mut completed = Vec::with_capacity(tasks.len());
        for task in tasks {
            let report = run_match_supervised(current_exe, &task.config, safety);
            completed.push(CompletedScheduledMatch {
                sequence: task.sequence,
                group_index: task.group_index,
                report,
            });
        }
        return completed;
    }

    let current_exe = current_exe.to_path_buf();
    let task_count = tasks.len();
    let jobs = effective_jobs(safety, task_count);
    let tasks = Arc::new(tasks);
    let next_index = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = mpsc::channel();
    let mut handles = Vec::with_capacity(jobs);

    for worker_index in 0..jobs {
        let current_exe = current_exe.clone();
        let next_index = Arc::clone(&next_index);
        let sender = sender.clone();
        let safety = safety.clone();
        let tasks = Arc::clone(&tasks);

        let handle = std::thread::Builder::new()
            .name(format!("gauntlet-worker-{worker_index}"))
            .spawn(move || {
                loop {
                    let task_index = next_index.fetch_add(1, Ordering::Relaxed);
                    if task_index >= tasks.len() {
                        break;
                    }

                    let task = &tasks[task_index];
                    let report = run_match_supervised(&current_exe, &task.config, &safety);
                    if sender
                        .send(CompletedScheduledMatch {
                            sequence: task.sequence,
                            group_index: task.group_index,
                            report,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("spawn gauntlet worker");
        handles.push(handle);
    }

    drop(sender);

    let mut completed = Vec::with_capacity(task_count);
    for result in receiver {
        completed.push(result);
    }

    let mut worker_panicked = false;
    for handle in handles {
        if handle.join().is_err() {
            worker_panicked = true;
        }
    }

    if completed.len() != task_count || worker_panicked {
        let mut seen = vec![false; task_count];
        for result in &completed {
            if result.sequence < task_count {
                seen[result.sequence] = true;
            }
        }

        let error = if worker_panicked {
            "parallel worker panicked before producing a result"
        } else {
            "parallel worker exited before producing a result"
        };

        for task in tasks.iter() {
            if !seen[task.sequence] {
                completed.push(CompletedScheduledMatch {
                    sequence: task.sequence,
                    group_index: task.group_index,
                    report: failure_report(
                        &task.config,
                        TerminationReason::UpdateError,
                        error.to_string(),
                    ),
                });
            }
        }
    }

    completed.sort_by_key(|result| result.sequence);
    completed
}

fn effective_jobs(safety: &SafetyConfig, task_count: usize) -> usize {
    safety.jobs.max(1).min(task_count.max(1))
}

fn run_match_supervised(
    current_exe: &Path,
    config: &MatchConfig,
    safety: &SafetyConfig,
) -> MatchReport {
    match run_match_supervised_inner(current_exe, config, safety) {
        Ok(report) => report,
        Err(error) => failure_report(config, TerminationReason::UpdateError, error.to_string()),
    }
}

fn run_match_supervised_inner(
    current_exe: &Path,
    config: &MatchConfig,
    safety: &SafetyConfig,
) -> Result<MatchReport> {
    let mut command = Command::new(current_exe);
    command
        .arg("--internal-match-json")
        .arg("--deck-a")
        .arg(&config.participants[0].deck_id)
        .arg("--deck-b")
        .arg(&config.participants[1].deck_id)
        .arg("--agent-a")
        .arg(agent_label(&config.participants[0].agent))
        .arg("--agent-b")
        .arg(agent_label(&config.participants[1].agent))
        .arg("--seed")
        .arg(config.seed.to_string())
        .arg("--max-turns")
        .arg(config.max_turns.to_string())
        .arg("--max-actions")
        .arg(config.max_actions.to_string())
        .arg("--repeat-state-limit")
        .arg(config.repeat_state_limit.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if config.include_trace {
        command.arg("--trace");
    }
    if config.enable_sideboard {
        command.arg("--sideboard");
    }
    if let Some(ref patch_path) = config.patch_path {
        command.arg("--patch").arg(patch_path);
    }
    if config.participants[0].uses_mcts {
        command.arg("--mcts-a");
    }
    if config.participants[1].uses_mcts {
        command.arg("--mcts-b");
    }
    command
        .arg("--mcts-iterations")
        .arg(config.mcts_iterations.to_string())
        .arg("--mcts-rollout-depth")
        .arg(config.mcts_rollout_depth.to_string());
    match config.format {
        GameFormat::Classic => {
            command.arg("--classic");
        }
        GameFormat::Operator => {
            command.arg("--operator");
        }
    }
    if let Some(op) = config.participants[0].operator.as_ref() {
        command.arg("--operator-a").arg(op);
    }
    if let Some(op) = config.participants[1].operator.as_ref() {
        command.arg("--operator-b").arg(op);
    }

    apply_child_limits(&mut command, safety)?;

    let mut child = command.spawn().with_context(|| {
        format!(
            "spawn supervised match {} vs {}",
            config.participants[0].deck_id, config.participants[1].deck_id
        )
    })?;
    let stdout_handle = spawn_output_reader(child.stdout.take());
    let stderr_handle = spawn_output_reader(child.stderr.take());

    let deadline = Instant::now() + Duration::from_secs(safety.timeout_seconds.max(1));
    loop {
        if let Some(status) = child.try_wait()? {
            let stdout = join_output_reader(stdout_handle)?;
            let stderr = join_output_reader(stderr_handle)?;
            if !status.success() {
                return Ok(failure_report(
                    config,
                    TerminationReason::ProcessCrashed,
                    child_failure_message(status.code(), &stderr, &stdout),
                ));
            }

            let report: MatchReport = serde_json::from_str(stdout.trim()).with_context(|| {
                format!(
                    "parse supervised child output; stdout={} stderr={}",
                    compact_output(&stdout),
                    compact_output(&stderr)
                )
            })?;
            return Ok(report);
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let stdout = join_output_reader(stdout_handle)?;
            let stderr = join_output_reader(stderr_handle)?;
            return Ok(failure_report(
                config,
                TerminationReason::TimedOut,
                format!(
                    "match exceeded {}s timeout; stderr={} stdout={}",
                    safety.timeout_seconds.max(1),
                    compact_output(&stderr),
                    compact_output(&stdout)
                ),
            ));
        }

        sleep(Duration::from_millis(CHILD_POLL_INTERVAL_MS));
    }
}

fn spawn_output_reader(
    pipe: Option<impl Read + Send + 'static>,
) -> std::thread::JoinHandle<Result<String>> {
    std::thread::spawn(move || {
        let Some(mut pipe) = pipe else {
            return Ok(String::new());
        };

        let mut output = String::new();
        pipe.read_to_string(&mut output)?;
        Ok(output)
    })
}

fn join_output_reader(handle: std::thread::JoinHandle<Result<String>>) -> Result<String> {
    handle
        .join()
        .map_err(|_| anyhow!("child output reader panicked"))?
}

fn failure_report(
    config: &MatchConfig,
    termination_reason: TerminationReason,
    error: String,
) -> MatchReport {
    MatchReport {
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
        termination_reason,
        error: Some(error),
        final_state: Vec::new(),
        snapshots: Vec::new(),
        trace: Vec::new(),
        degeneracy_flags: Vec::new(),
        card_metrics: Vec::new(),
    }
}

fn child_failure_message(code: Option<i32>, stderr: &str, stdout: &str) -> String {
    match code {
        Some(code) => format!(
            "child exited with status {code}; stderr={} stdout={}",
            compact_output(stderr),
            compact_output(stdout)
        ),
        None => format!(
            "child terminated by signal; stderr={} stdout={}",
            compact_output(stderr),
            compact_output(stdout)
        ),
    }
}

fn compact_output(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }

    const MAX_LEN: usize = 240;
    if trimmed.chars().count() <= MAX_LEN {
        trimmed.to_string()
    } else {
        format!("{}...", trimmed.chars().take(MAX_LEN).collect::<String>())
    }
}

fn agent_label(agent: &AgentProfile) -> &'static str {
    match agent {
        AgentProfile::Generalist => "generalist",
        AgentProfile::Neon => "neon",
        AgentProfile::Glitch => "glitch",
        AgentProfile::Chrome => "chrome",
        AgentProfile::Slag => "slag",
        AgentProfile::Bio => "bio",
    }
}

#[cfg(unix)]
fn apply_child_limits(command: &mut Command, safety: &SafetyConfig) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let stack_bytes = safety.child_stack_mb.saturating_mul(1024 * 1024);
    let memory_bytes = safety.child_memory_mb.saturating_mul(1024 * 1024);

    unsafe {
        command.pre_exec(move || {
            if stack_bytes > 0 {
                set_limit(libc::RLIMIT_STACK, stack_bytes)?;
            }
            if memory_bytes > 0 {
                set_limit(libc::RLIMIT_AS, memory_bytes)?;
            }
            Ok(())
        });
    }

    Ok(())
}

#[cfg(unix)]
fn set_limit(resource: libc::__rlimit_resource_t, bytes: u64) -> std::io::Result<()> {
    let limit = libc::rlimit {
        rlim_cur: bytes as libc::rlim_t,
        rlim_max: bytes as libc::rlim_t,
    };

    let result = unsafe { libc::setrlimit(resource, &limit) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn apply_child_limits(_command: &mut Command, _safety: &SafetyConfig) -> Result<()> {
    Ok(())
}

fn required_value(flag: &str, value: Option<String>) -> Result<String> {
    value.ok_or_else(|| anyhow!("Missing value for {flag}"))
}

fn parse_agent(value: &str) -> Result<AgentProfile> {
    match value {
        "generalist" => Ok(AgentProfile::Generalist),
        "neon" => Ok(AgentProfile::Neon),
        "glitch" => Ok(AgentProfile::Glitch),
        "chrome" => Ok(AgentProfile::Chrome),
        "slag" => Ok(AgentProfile::Slag),
        "bio" => Ok(AgentProfile::Bio),
        other => Err(anyhow!("Unknown agent profile: {other}")),
    }
}

fn parse_jobs(value: &str) -> Result<usize> {
    let jobs = value.parse::<usize>()?;
    if jobs == 0 {
        return Err(anyhow!("--jobs must be at least 1"));
    }
    Ok(jobs)
}

fn parse_operator_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_deck_list(value: &str) -> Result<Vec<String>> {
    let decks: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|deck| !deck.is_empty())
        .map(ToString::to_string)
        .collect();

    if decks.len() < 2 {
        return Err(anyhow!("--decks requires at least two deck ids"));
    }

    Ok(decks)
}

fn print_usage() {
    println!(
        "Usage: cargo run -p cardgame --bin run_gauntlet -- [options]

Options:
  --matrix                    Run a full deck-vs-deck matchup matrix
  --decks <a,b,c>             Deck ids for matrix mode (defaults to mono archetypes)
  --deck-a <id>               Base deck id for player A
  --deck-b <id>               Base deck id for player B
  --patch <path>              Apply a balance patch JSON to this run
  --evaluate-patch <path>     Run baseline matrix vs patched matrix and report the delta
  --agent-a <profile>         generalist|neon|glitch|chrome|slag|bio
  --agent-b <profile>         generalist|neon|glitch|chrome|slag|bio
  --mcts-a                    Use MCTS search for player A
  --mcts-b                    Use MCTS search for player B
  --mcts-iterations <n>       Root search iterations for MCTS agents
  --mcts-rollout-depth <n>    Rollout action cap for MCTS agents
  --seed <n>                  Starting seed
  --matches <n>               Number of matches to run or matches per matrix pairing
  --jobs <n>                  Concurrent supervised matches (default: available cores)
  --max-turns <n>             Turn cap
  --max-actions <n>           Action cap
  --repeat-state-limit <n>    Repeated fingerprint limit before loop detection
  --timeout-seconds <n>       Per-match child timeout (default: 10)
  --child-stack-mb <n>        Child stack limit in MB on Unix (default: 64)
  --child-memory-mb <n>       Child address-space limit in MB on Unix (default: 1024)
  --trace                     Include per-action trace for single-match runs only
  --unsafe-in-process         Disable child-process safety guards
  --no-alternate-seats        Keep seat order fixed across series
  --sideboard                 Enable sideboard swaps based on opponent deck
  --include-mirrors           Include mirror pairings in matrix mode
  --degeneracy                Run degeneracy detection (forces trace, reports flags)
  --operator                  Use Operator format (default is Classic)
  --operator-a <card_id>      Operator card ID for player A
  --operator-b <card_id>      Operator card ID for player B
  --operator-matrix           Run all 30 operator+deck combos in a round-robin matrix
  --operators <a,b,...>       Filter operator IDs or deck colors for --operator-matrix
  --card-metrics-csv <path>   Write per-card aggregate stats (casts, ETB, removals, damage) to CSV
  --help                      Show this message"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardgame::sim::config::ParticipantConfig;

    #[test]
    fn series_builder_alternates_seats_and_seeds() {
        let config = GauntletConfig {
            base_match: MatchConfig {
                participants: vec![
                    ParticipantConfig::new("p1", "neon", AgentProfile::Neon, None, false),
                    ParticipantConfig::new("p2", "chrome", AgentProfile::Chrome, None, false),
                ],
                patch_path: None,
                mcts_iterations: 48,
                mcts_rollout_depth: 10,
                seed: 11,
                format: GameFormat::Classic,
                max_turns: 20,
                max_actions: 400,
                repeat_state_limit: 6,
                include_trace: false,
                enable_sideboard: false,
            },
            matches: 4,
            alternate_seats: true,
        };

        let matches = build_series_matches(&config);
        assert_eq!(matches.len(), 4);
        assert_eq!(matches[0].seed, 11);
        assert_eq!(matches[1].seed, 12);
        assert_eq!(matches[0].participants[0].deck_id, "neon");
        assert_eq!(matches[1].participants[0].deck_id, "chrome");
        assert_eq!(matches[2].participants[0].deck_id, "neon");
        assert_eq!(matches[3].participants[0].deck_id, "chrome");
    }

    #[test]
    fn series_builder_swaps_operator_ids_with_seats() {
        let config = GauntletConfig {
            base_match: MatchConfig {
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
                seed: 7,
                format: GameFormat::Operator,
                max_turns: 20,
                max_actions: 400,
                repeat_state_limit: 6,
                include_trace: false,
                enable_sideboard: false,
            },
            matches: 2,
            alternate_seats: true,
        };

        let matches = build_series_matches(&config);
        assert_eq!(
            matches[0].participants[0].operator.as_deref(),
            Some("echo_signal")
        );
        assert_eq!(
            matches[0].participants[1].operator.as_deref(),
            Some("verdant_warlord")
        );
        assert_eq!(matches[1].participants[0].deck_id, "bio");
        assert_eq!(
            matches[1].participants[0].operator.as_deref(),
            Some("verdant_warlord")
        );
        assert_eq!(
            matches[1].participants[1].operator.as_deref(),
            Some("echo_signal")
        );
    }

    #[test]
    fn matrix_builder_expands_expected_pairings() {
        let mut config = MatrixConfig::default();
        config.decks = vec!["neon".into(), "chrome".into(), "bio".into()];
        config.matches_per_pair = 3;

        let plans = build_matrix_matchups(&config);
        assert_eq!(plans.len(), 3);
        assert_eq!(plans[0].deck_a, "neon");
        assert_eq!(plans[0].deck_b, "chrome");
        assert_eq!(plans[1].deck_a, "neon");
        assert_eq!(plans[1].deck_b, "bio");
        assert_eq!(plans[2].deck_a, "chrome");
        assert_eq!(plans[2].deck_b, "bio");
        assert!(plans.iter().all(|plan| plan.matches.len() == 3));
    }

    #[test]
    fn effective_jobs_respects_task_count() {
        let mut safety = SafetyConfig::default();
        safety.jobs = 64;
        assert_eq!(effective_jobs(&safety, 3), 3);
        assert_eq!(effective_jobs(&safety, 0), 1);
    }
}
