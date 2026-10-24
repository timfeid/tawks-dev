use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CandidateScore {
    abnormal_terminations: usize,
    strict_band_misses: usize,
    strict_band_distance_sum: f64,
    balance_score: f64,
    intra_color_spread_sum: f64,
}

#[derive(Debug, Deserialize)]
struct OperatorOutlier {
    combo_label: String,
    win_rate: Option<f64>,
    distance_to_band: f64,
}

#[derive(Debug, Deserialize)]
struct CandidateSummary {
    signature: String,
    changed_knobs: Vec<String>,
    score: CandidateScore,
    outliers: Vec<OperatorOutlier>,
}

#[derive(Debug, Deserialize)]
struct StageSummary {
    best: CandidateSummary,
}

#[derive(Debug, Deserialize)]
struct RunSummary {
    screening: StageSummary,
}

#[derive(Debug)]
struct ComparisonTarget {
    source_path: PathBuf,
    summary: CandidateSummary,
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 {
        bail!(
            "usage: cargo run -p cardgame --bin compare_autotune_runs -- <left-run-or-summary> <right-run-or-summary>"
        );
    }

    let left = load_target(&args[0])?;
    let right = load_target(&args[1])?;

    print_target("Left", &left);
    println!();
    print_target("Right", &right);
    println!();
    print_delta(&left, &right);

    Ok(())
}

fn load_target(raw_path: &str) -> Result<ComparisonTarget> {
    let source_path = resolve_summary_path(Path::new(raw_path))?;
    let contents = fs::read_to_string(&source_path)
        .with_context(|| format!("read {}", source_path.display()))?;

    if let Ok(run_summary) = serde_json::from_str::<RunSummary>(&contents) {
        return Ok(ComparisonTarget {
            source_path,
            summary: run_summary.screening.best,
        });
    }

    if let Ok(candidate_summary) = serde_json::from_str::<CandidateSummary>(&contents) {
        return Ok(ComparisonTarget {
            source_path,
            summary: candidate_summary,
        });
    }

    bail!(
        "unsupported summary format in {} (expected run summary or candidate summary)",
        source_path.display()
    );
}

fn resolve_summary_path(path: &Path) -> Result<PathBuf> {
    if path.is_dir() {
        let summary_path = path.join("summary.json");
        if summary_path.exists() {
            return Ok(summary_path);
        }
        bail!("no summary.json found in {}", path.display());
    }

    if path.file_name().is_some_and(|name| {
        name == "summary.json" || name == "report.json" || name == "candidate.json"
    }) {
        return Ok(path.to_path_buf());
    }

    let summary_path = path.join("summary.json");
    if summary_path.exists() {
        return Ok(summary_path);
    }

    Ok(path.to_path_buf())
}

fn print_target(label: &str, target: &ComparisonTarget) {
    println!("{label}: {}", target.source_path.display());
    println!("  Signature: {}", target.summary.signature);
    if target.summary.changed_knobs.is_empty() {
        println!("  Changed knobs: baseline");
    } else {
        println!(
            "  Changed knobs: {}",
            target.summary.changed_knobs.join(", ")
        );
    }
    print_score("  Score", &target.summary.score);
    print_top_outliers("  Top outliers", &target.summary.outliers);
}

fn print_score(label: &str, score: &CandidateScore) {
    println!(
        "{label}: misses={} distance={:.4} balance={:.4} abnormal={} spread={:.4}",
        score.strict_band_misses,
        score.strict_band_distance_sum,
        score.balance_score,
        score.abnormal_terminations,
        score.intra_color_spread_sum
    );
}

fn print_top_outliers(label: &str, outliers: &[OperatorOutlier]) {
    if outliers.is_empty() {
        println!("{label}: none");
        return;
    }

    println!("{label}:");
    for outlier in outliers.iter().take(5) {
        let win_rate = outlier
            .win_rate
            .map(|rate| format!("{:.1}%", rate * 100.0))
            .unwrap_or_else(|| "n/a".to_string());
        println!(
            "    {} -> win_rate={} distance={:.4}",
            outlier.combo_label, win_rate, outlier.distance_to_band
        );
    }
}

fn print_delta(left: &ComparisonTarget, right: &ComparisonTarget) {
    let ordering = compare_scores(&left.summary.score, &right.summary.score);
    let winner = match ordering {
        Ordering::Less => "Left wins",
        Ordering::Greater => "Right wins",
        Ordering::Equal => "Tie",
    };

    println!("Comparison: {winner}");
    println!(
        "  Delta (right - left): misses={:+} distance={:+.4} balance={:+.4} abnormal={:+} spread={:+.4}",
        right.summary.score.strict_band_misses as isize
            - left.summary.score.strict_band_misses as isize,
        right.summary.score.strict_band_distance_sum - left.summary.score.strict_band_distance_sum,
        right.summary.score.balance_score - left.summary.score.balance_score,
        right.summary.score.abnormal_terminations as isize
            - left.summary.score.abnormal_terminations as isize,
        right.summary.score.intra_color_spread_sum - left.summary.score.intra_color_spread_sum
    );
}

fn compare_scores(left: &CandidateScore, right: &CandidateScore) -> Ordering {
    left.abnormal_terminations
        .cmp(&right.abnormal_terminations)
        .then_with(|| left.strict_band_misses.cmp(&right.strict_band_misses))
        .then_with(|| {
            cmp_f64(
                left.strict_band_distance_sum,
                right.strict_band_distance_sum,
            )
        })
        .then_with(|| cmp_f64(left.balance_score, right.balance_score))
        .then_with(|| cmp_f64(left.intra_color_spread_sum, right.intra_color_spread_sum))
}

fn cmp_f64(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}
