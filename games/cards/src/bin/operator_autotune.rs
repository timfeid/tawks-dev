use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use cardgame::card::{Card, CardEffect};
use cardgame::card_catalog;
use cardgame::deck::operator_ids_for_color;
use cardgame::sim::config::{DraftMatrixConfig, MatrixConfig, OperatorMatrixConfig};
use cardgame::sim::patch::{
    AbilityPatch, BalancePatch, CardPatch, DeckEntryPatch, TriggerPatch,
    load_balance_patch_from_path,
};
use cardgame::sim::report::{
    DraftMatrixReport, MatrixReport, OperatorMatrixReport, TerminationCount, TerminationReason,
};
use cardgame::sim::runner::{
    run_draft_matrix_with_patch, run_matrix_with_patch, run_operator_matrix_with_patch,
};
use cardgame::types::{CardSupertype, GameFormat, ManaCost, ManaType};
use serde::Serialize;

const DEFAULT_OUT_DIR: &str = "target/cardgame-autotune-runs";
const DEFAULT_MATCHES: usize = 6;
const DEFAULT_GENERATIONS: usize = 6;
const DEFAULT_BEAM_WIDTH: usize = 8;
const DEFAULT_CANDIDATES_PER_GENERATION: usize = 40;
const DEFAULT_PROMOTION_MATCHES: usize = 24;
const DEFAULT_PROMOTION_COUNT: usize = 0;
const STRICT_BAND_LOW: f64 = 0.45;
const STRICT_BAND_HIGH: f64 = 0.55;
const NONE_WIN_RATE_DISTANCE: f64 = 1.0;

#[derive(Clone, Debug, Serialize)]
enum TuneMode {
    Operators,
    Cards,
    Draft,
    AllModes,
}

#[derive(Clone, Debug, Serialize)]
struct CliConfig {
    out_dir: PathBuf,
    base_patch: Option<PathBuf>,
    matches: usize,
    draft_count: usize,
    generations: usize,
    beam_width: usize,
    candidates_per_generation: usize,
    seed: u64,
    operator_filter: Vec<String>,
    card_filter: Vec<String>,
    include_mirrors: bool,
    promotion_matches: usize,
    promotion_count: usize,
    mode: TuneMode,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from(DEFAULT_OUT_DIR),
            base_patch: None,
            matches: DEFAULT_MATCHES,
            draft_count: 20,
            generations: DEFAULT_GENERATIONS,
            beam_width: DEFAULT_BEAM_WIDTH,
            candidates_per_generation: DEFAULT_CANDIDATES_PER_GENERATION,
            seed: 1,
            operator_filter: Vec::new(),
            card_filter: Vec::new(),
            include_mirrors: false,
            promotion_matches: DEFAULT_PROMOTION_MATCHES,
            promotion_count: DEFAULT_PROMOTION_COUNT,
            mode: TuneMode::Operators,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum ManaField {
    Neon,
    Glitch,
    Chrome,
    Slag,
    Bio,
    Colorless,
}

impl ManaField {
    fn label(self) -> &'static str {
        match self {
            ManaField::Neon => "neon",
            ManaField::Glitch => "glitch",
            ManaField::Chrome => "chrome",
            ManaField::Slag => "slag",
            ManaField::Bio => "bio",
            ManaField::Colorless => "colorless",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum EffectField {
    Damage,
    Draw,
    LootDraw,
    LootDiscard,
    Scry,
    Counters,
    TokenPower,
    TokenToughness,
}

impl EffectField {
    fn label(self) -> &'static str {
        match self {
            EffectField::Damage => "damage",
            EffectField::Draw => "draw",
            EffectField::LootDraw => "loot_draw",
            EffectField::LootDiscard => "loot_discard",
            EffectField::Scry => "scry",
            EffectField::Counters => "counters",
            EffectField::TokenPower => "token_power",
            EffectField::TokenToughness => "token_toughness",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum EffectPath {
    Direct {
        field: EffectField,
    },
    Sequence {
        effect_index: usize,
        field: EffectField,
    },
}

impl EffectPath {
    fn label(self) -> String {
        match self {
            EffectPath::Direct { field } => field.label().to_string(),
            EffectPath::Sequence {
                effect_index,
                field,
            } => format!("effect_{effect_index}.{}", field.label()),
        }
    }

    fn field(self) -> EffectField {
        match self {
            EffectPath::Direct { field } | EffectPath::Sequence { field, .. } => field,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum KnobField {
    ManaCost(ManaField),
    Power,
    Toughness,
    Loyalty,
    AbilityManaCost {
        ability_index: usize,
        mana: ManaField,
    },
    TriggerEffect {
        trigger_index: usize,
        path: EffectPath,
    },
    AbilityEffect {
        ability_index: usize,
        path: EffectPath,
    },
    DeckQuantity {
        deck_id: String,
    },
}

impl KnobField {
    fn label(&self) -> String {
        match self {
            KnobField::ManaCost(mana) => format!("mana_cost.{}", mana.label()),
            KnobField::Power => "power".to_string(),
            KnobField::Toughness => "toughness".to_string(),
            KnobField::Loyalty => "loyalty".to_string(),
            KnobField::AbilityManaCost {
                ability_index,
                mana,
            } => format!("ability_{ability_index}.mana_cost.{}", mana.label()),
            KnobField::TriggerEffect {
                trigger_index,
                path,
            } => format!("trigger_{trigger_index}.{}", path.label()),
            KnobField::AbilityEffect {
                ability_index,
                path,
            } => format!("ability_{ability_index}.{}", path.label()),
            KnobField::DeckQuantity { deck_id } => format!("deck_qty.{deck_id}"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct KnobSpec {
    card_id: String,
    field: KnobField,
    baseline: i32,
    min: i32,
    max: i32,
}

impl KnobSpec {
    fn label(&self) -> String {
        format!("{}.{}", self.card_id, self.field.label())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Candidate {
    values: Vec<i32>,
}

impl Candidate {
    fn baseline(specs: &[KnobSpec]) -> Self {
        Self {
            values: specs.iter().map(|spec| spec.baseline).collect(),
        }
    }

    fn signature(&self, specs: &[KnobSpec]) -> String {
        let parts: Vec<_> = self
            .values
            .iter()
            .zip(specs)
            .filter(|(value, spec)| **value != spec.baseline)
            .map(|(value, spec)| format!("{}={value}", spec.label()))
            .collect();
        if parts.is_empty() {
            "baseline".to_string()
        } else {
            parts.join("|")
        }
    }

    fn changed_knobs(&self, specs: &[KnobSpec]) -> Vec<String> {
        self.values
            .iter()
            .zip(specs)
            .filter(|(value, spec)| **value != spec.baseline)
            .map(|(value, spec)| format!("{}={value}", spec.label()))
            .collect()
    }

    fn neighbors(&self, specs: &[KnobSpec]) -> Vec<Self> {
        let mut neighbors = Vec::new();
        for (index, spec) in specs.iter().enumerate() {
            let value = self.values[index];
            if value > spec.min {
                let mut candidate = self.clone();
                candidate.values[index] -= 1;
                neighbors.push(candidate);
            }
            if value < spec.max {
                let mut candidate = self.clone();
                candidate.values[index] += 1;
                neighbors.push(candidate);
            }
        }
        neighbors
    }
}

#[derive(Clone, Debug, Serialize)]
struct CandidateScore {
    abnormal_terminations: usize,
    strict_band_misses: usize,
    strict_band_distance_sum: f64,
    balance_score: f64,
    intra_color_spread_sum: f64,
}

#[derive(Clone, Debug, Serialize)]
struct OperatorOutlier {
    combo_label: String,
    win_rate: Option<f64>,
    distance_to_band: f64,
}

#[derive(Clone, Debug, Serialize)]
struct CandidateSummary {
    stage: String,
    candidate_index: usize,
    generation: usize,
    signature: String,
    changed_knobs: Vec<String>,
    score: CandidateScore,
    outliers: Vec<OperatorOutlier>,
    finished_match_rate: f64,
    decisive_match_rate: f64,
    unfinished_match_rate: f64,
    termination_counts: Vec<TerminationCount>,
    artifact_dir: String,
}

#[derive(Clone, Debug, Serialize)]
struct PendingCandidateArtifact {
    stage: String,
    candidate_index: usize,
    generation: usize,
    signature: String,
    changed_knobs: Vec<String>,
    artifact_dir: String,
    status: String,
    completed_reports: Vec<String>,
}

#[derive(Clone, Debug)]
struct EvaluatedCandidate {
    candidate: Candidate,
    summary: CandidateSummary,
}

#[derive(Clone, Debug, Serialize)]
struct StageSummary {
    stage: String,
    matches_per_pair: usize,
    evaluated_candidates: usize,
    leaderboard: Vec<CandidateSummary>,
    best: CandidateSummary,
}

#[derive(Clone, Debug, Serialize)]
struct RunSummary {
    run_dir: String,
    config: CliConfig,
    knobs: Vec<KnobSpec>,
    screening: StageSummary,
    promotion: Option<StageSummary>,
}

#[derive(Clone, Debug, Serialize)]
struct AllModesArtifactReport {
    classic: MatrixReport,
    operator: OperatorMatrixReport,
    draft: DraftMatrixReport,
}

fn prepare_candidate_artifact_dir(
    run_dir: &Path,
    stage: &str,
    candidate_index: usize,
    generation: usize,
    signature: &str,
    changed_knobs: Vec<String>,
    patch: &BalancePatch,
    candidate_patch: &BalancePatch,
    base_patch: Option<&BalancePatch>,
) -> Result<PathBuf> {
    let artifact_dir = run_dir
        .join("candidates")
        .join(format!("{stage}_{candidate_index:04}"));
    fs::create_dir_all(&artifact_dir)
        .with_context(|| format!("create artifact dir {}", artifact_dir.display()))?;
    write_json(artifact_dir.join("patch.json"), patch)?;
    if base_patch.is_some() {
        write_json(artifact_dir.join("delta_patch.json"), candidate_patch)?;
    }
    write_candidate_artifact_status(
        &artifact_dir,
        stage,
        candidate_index,
        generation,
        signature,
        &changed_knobs,
        "pending",
        &[],
    )?;
    Ok(artifact_dir)
}

fn write_candidate_artifact_status(
    artifact_dir: &Path,
    stage: &str,
    candidate_index: usize,
    generation: usize,
    signature: &str,
    changed_knobs: &[String],
    status: &str,
    completed_reports: &[&str],
) -> Result<()> {
    write_json(
        artifact_dir.join("candidate.json"),
        &PendingCandidateArtifact {
            stage: stage.to_string(),
            candidate_index,
            generation,
            signature: signature.to_string(),
            changed_knobs: changed_knobs.to_vec(),
            artifact_dir: artifact_dir.display().to_string(),
            status: status.to_string(),
            completed_reports: completed_reports.iter().map(|s| s.to_string()).collect(),
        },
    )
}

fn main() -> Result<()> {
    let config = parse_args()?;

    match config.mode {
        TuneMode::Operators => run_operator_mode(config),
        TuneMode::Cards => run_card_mode(config),
        TuneMode::Draft => run_draft_mode(config),
        TuneMode::AllModes => run_all_modes_mode(config),
    }
}

fn run_operator_mode(config: CliConfig) -> Result<()> {
    let mut catalog = load_operator_catalog();
    let base_patch = load_base_patch(config.base_patch.as_deref())?;
    if let Some(base_patch) = base_patch.as_ref() {
        apply_patch_to_catalog(&mut catalog, base_patch)?;
    }
    let knobs = filter_card_knob_specs(
        filter_knob_specs(build_knob_specs(&catalog)?, &config.operator_filter),
        &config.card_filter,
    );
    let run_dir = create_run_dir(&config.out_dir, &config)?;
    write_json(run_dir.join("run_config.json"), &config)?;
    write_json(run_dir.join("knobs.json"), &knobs)?;

    let screening = run_search_stage(
        "screening",
        config.matches,
        &config,
        &catalog,
        &knobs,
        &run_dir,
        base_patch.as_ref(),
    )?;
    write_json(
        run_dir.join("screening_leaderboard.json"),
        &screening.leaderboard,
    )?;

    let promotion = if config.promotion_count > 0 && config.promotion_matches > config.matches {
        let promoted = run_promotion_stage(
            &config,
            &catalog,
            &knobs,
            &run_dir,
            &screening,
            base_patch.as_ref(),
        )?;
        write_json(
            run_dir.join("promotion_leaderboard.json"),
            &promoted.leaderboard,
        )?;
        Some(promoted)
    } else {
        None
    };

    let summary = RunSummary {
        run_dir: run_dir.display().to_string(),
        config,
        knobs,
        screening,
        promotion,
    };
    write_json(run_dir.join("summary.json"), &summary)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn run_card_mode(config: CliConfig) -> Result<()> {
    let mut catalog = load_full_catalog();
    let base_patch = load_base_patch(config.base_patch.as_deref())?;
    if let Some(base_patch) = base_patch.as_ref() {
        apply_patch_to_catalog(&mut catalog, base_patch)?;
    }
    let knobs = filter_card_knob_specs(
        build_card_mode_knob_specs(&catalog, &config.card_filter)?,
        &config.card_filter,
    );
    let run_dir = create_run_dir(&config.out_dir, &config)?;
    write_json(run_dir.join("run_config.json"), &config)?;
    write_json(run_dir.join("knobs.json"), &knobs)?;

    eprintln!(
        "Card balance mode: {} knobs across {} cards",
        knobs.len(),
        knobs
            .iter()
            .map(|k| &k.card_id)
            .collect::<BTreeSet<_>>()
            .len()
    );

    let screening = run_card_search_stage(
        "screening",
        config.matches,
        &config,
        &catalog,
        &knobs,
        &run_dir,
        base_patch.as_ref(),
    )?;
    write_json(
        run_dir.join("screening_leaderboard.json"),
        &screening.leaderboard,
    )?;

    let promotion = if config.promotion_count > 0 && config.promotion_matches > config.matches {
        let promoted = run_card_promotion_stage(
            &config,
            &catalog,
            &knobs,
            &run_dir,
            &screening,
            base_patch.as_ref(),
        )?;
        write_json(
            run_dir.join("promotion_leaderboard.json"),
            &promoted.leaderboard,
        )?;
        Some(promoted)
    } else {
        None
    };

    let summary = RunSummary {
        run_dir: run_dir.display().to_string(),
        config,
        knobs,
        screening,
        promotion,
    };
    write_json(run_dir.join("summary.json"), &summary)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn run_draft_mode(config: CliConfig) -> Result<()> {
    let mut catalog = load_full_catalog();
    let base_patch = load_base_patch(config.base_patch.as_deref())?;
    if let Some(base_patch) = base_patch.as_ref() {
        apply_patch_to_catalog(&mut catalog, base_patch)?;
    }
    let knobs = filter_card_knob_specs(
        build_card_mode_knob_specs(&catalog, &config.card_filter)?,
        &config.card_filter,
    );
    let run_dir = create_run_dir(&config.out_dir, &config)?;
    write_json(run_dir.join("run_config.json"), &config)?;
    write_json(run_dir.join("knobs.json"), &knobs)?;

    eprintln!(
        "Draft balance mode: {} knobs across {} cards",
        knobs.len(),
        knobs
            .iter()
            .map(|k| &k.card_id)
            .collect::<BTreeSet<_>>()
            .len()
    );

    let screening = run_draft_search_stage(
        "screening",
        config.matches,
        &config,
        &catalog,
        &knobs,
        &run_dir,
        base_patch.as_ref(),
    )?;
    write_json(
        run_dir.join("screening_leaderboard.json"),
        &screening.leaderboard,
    )?;

    let promotion = if config.promotion_count > 0 && config.promotion_matches > config.matches {
        let promoted = run_draft_promotion_stage(
            &config,
            &catalog,
            &knobs,
            &run_dir,
            &screening,
            base_patch.as_ref(),
        )?;
        write_json(
            run_dir.join("promotion_leaderboard.json"),
            &promoted.leaderboard,
        )?;
        Some(promoted)
    } else {
        None
    };

    let summary = RunSummary {
        run_dir: run_dir.display().to_string(),
        config,
        knobs,
        screening,
        promotion,
    };
    write_json(run_dir.join("summary.json"), &summary)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn run_all_modes_mode(config: CliConfig) -> Result<()> {
    let mut catalog = load_full_catalog();
    let base_patch = load_base_patch(config.base_patch.as_deref())?;
    if let Some(base_patch) = base_patch.as_ref() {
        apply_patch_to_catalog(&mut catalog, base_patch)?;
    }
    let knobs = filter_card_knob_specs(
        build_all_modes_knob_specs(&catalog, &config.card_filter)?,
        &config.card_filter,
    );
    let run_dir = create_run_dir(&config.out_dir, &config)?;
    write_json(run_dir.join("run_config.json"), &config)?;
    write_json(run_dir.join("knobs.json"), &knobs)?;

    eprintln!(
        "All-modes balance mode: {} knobs across {} cards",
        knobs.len(),
        knobs
            .iter()
            .map(|k| &k.card_id)
            .collect::<BTreeSet<_>>()
            .len()
    );
    let effective_operator_filter = infer_operator_filter_from_cards(&catalog, &config.card_filter);
    if config.operator_filter.is_empty() && !effective_operator_filter.is_empty() {
        eprintln!(
            "All-modes operator scope auto-filtered to: {}",
            effective_operator_filter.join(",")
        );
    }

    let screening = run_all_modes_search_stage(
        "screening",
        &config,
        &catalog,
        &knobs,
        &run_dir,
        base_patch.as_ref(),
        config.matches,
    )?;
    write_json(
        run_dir.join("screening_leaderboard.json"),
        &screening.leaderboard,
    )?;

    let promotion = if config.promotion_count > 0 && config.promotion_matches > config.matches {
        let promoted = run_all_modes_promotion_stage(
            &config,
            &catalog,
            &knobs,
            &run_dir,
            &screening,
            base_patch.as_ref(),
        )?;
        write_json(
            run_dir.join("promotion_leaderboard.json"),
            &promoted.leaderboard,
        )?;
        Some(promoted)
    } else {
        None
    };

    let summary = RunSummary {
        run_dir: run_dir.display().to_string(),
        config,
        knobs,
        screening,
        promotion,
    };
    write_json(run_dir.join("summary.json"), &summary)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn parse_args() -> Result<CliConfig> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        std::process::exit(0);
    }

    let mut config = CliConfig::default();
    let mut index = 0usize;
    while index < args.len() {
        let flag = &args[index];
        let value = args.get(index + 1).cloned();

        match flag.as_str() {
            "--out-dir" => config.out_dir = PathBuf::from(required_value(flag, value)?),
            "--base-patch" => config.base_patch = Some(PathBuf::from(required_value(flag, value)?)),
            "--matches" => config.matches = required_value(flag, value)?.parse()?,
            "--draft-count" => config.draft_count = required_value(flag, value)?.parse()?,
            "--generations" => config.generations = required_value(flag, value)?.parse()?,
            "--beam-width" => config.beam_width = required_value(flag, value)?.parse()?,
            "--candidates-per-generation" => {
                config.candidates_per_generation = required_value(flag, value)?.parse()?
            }
            "--seed" => config.seed = required_value(flag, value)?.parse()?,
            "--operators" => config.operator_filter = parse_csv(&required_value(flag, value)?),
            "--cards" => config.card_filter = parse_csv(&required_value(flag, value)?),
            "--promotion-matches" => {
                config.promotion_matches = required_value(flag, value)?.parse()?
            }
            "--promotion-count" => config.promotion_count = required_value(flag, value)?.parse()?,
            "--mode" => {
                let value = required_value(flag, value)?;
                config.mode = match value.as_str() {
                    "operators" => TuneMode::Operators,
                    "cards" => TuneMode::Cards,
                    "draft" => TuneMode::Draft,
                    "all-modes" | "all_modes" | "all" => TuneMode::AllModes,
                    other => {
                        return Err(anyhow!(
                            "Unknown mode: {other} (use: operators, cards, draft, all-modes)"
                        ));
                    }
                };
            }
            "--include-mirrors" => {
                config.include_mirrors = true;
                index += 1;
                continue;
            }
            other => return Err(anyhow!("Unknown flag: {other}")),
        }

        index += 2;
    }

    if config.matches == 0 {
        return Err(anyhow!("--matches must be >= 1"));
    }
    if config.draft_count == 0 {
        return Err(anyhow!("--draft-count must be >= 1"));
    }
    if config.beam_width == 0 {
        return Err(anyhow!("--beam-width must be >= 1"));
    }
    if config.candidates_per_generation == 0 {
        return Err(anyhow!("--candidates-per-generation must be >= 1"));
    }

    Ok(config)
}

fn print_usage() {
    eprintln!(
        "Usage: cargo run -p cardgame --bin operator_autotune -- [options]\n\
         \n\
         Options:\n\
           --out-dir <path>                   Artifact root (default: {DEFAULT_OUT_DIR})\n\
           --base-patch <path>                Apply a baseline balance patch before search\n\
           --matches <n>                      Screening matches per pairing (default: {DEFAULT_MATCHES})\n\
           --draft-count <n>                  Draft pools to generate in draft mode (default: 20)\n\
           --generations <n>                  Beam-search generations (default: {DEFAULT_GENERATIONS})\n\
           --beam-width <n>                   Frontier width per generation (default: {DEFAULT_BEAM_WIDTH})\n\
           --candidates-per-generation <n>    Cap new candidates per generation (default: {DEFAULT_CANDIDATES_PER_GENERATION})\n\
           --seed <n>                         Base deterministic seed (default: 1)\n\
           --operators <a,b,...>              Filter operator IDs or deck colors\n\
           --cards <a,b,...>                  Filter searched knobs to specific card IDs\n\
           --promotion-matches <n>            Higher-sample promotion matches (default: {DEFAULT_PROMOTION_MATCHES})\n\
           --promotion-count <n>              Number of screening finalists to promote (default: {DEFAULT_PROMOTION_COUNT})\n\
           --mode <operators|cards|draft|all-modes> Tune mode (default: operators)\n\
           --include-mirrors                  Include mirror matchups\n"
    );
}

fn required_value(flag: &str, value: Option<String>) -> Result<String> {
    value.ok_or_else(|| anyhow!("{flag} requires a value"))
}

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn load_operator_catalog() -> BTreeMap<String, Card> {
    let mut catalog = BTreeMap::new();
    extend_catalog(&mut catalog, card_catalog::neon::all_cards());
    extend_catalog(&mut catalog, card_catalog::glitch::all_cards());
    extend_catalog(&mut catalog, card_catalog::chrome::all_cards());
    extend_catalog(&mut catalog, card_catalog::slag::all_cards());
    extend_catalog(&mut catalog, card_catalog::bio::all_cards());
    extend_catalog(&mut catalog, card_catalog::multicolor::all_cards());
    catalog
}

fn extend_catalog(catalog: &mut BTreeMap<String, Card>, entries: Vec<(Card, i32)>) {
    for (card, _) in entries {
        if card.supertype == CardSupertype::Apex {
            catalog.insert(card.id.clone(), card);
        }
    }
}

fn load_full_catalog() -> BTreeMap<String, Card> {
    let mut catalog = BTreeMap::new();
    for entries in [
        card_catalog::neon::all_cards(),
        card_catalog::glitch::all_cards(),
        card_catalog::chrome::all_cards(),
        card_catalog::slag::all_cards(),
        card_catalog::bio::all_cards(),
        card_catalog::multicolor::all_cards(),
    ] {
        for (card, _) in entries {
            catalog.insert(card.id.clone(), card);
        }
    }
    catalog
}

/// Build stat-only knobs for all non-operator cards in the catalog.
/// For units: power ±1, toughness ±1.
/// For all cards with colorless cost >= 1: colorless mana ±1.
/// These changes apply to all game modes (draft, build, pre-built).
fn build_card_knob_specs(catalog: &BTreeMap<String, Card>) -> Vec<KnobSpec> {
    let mut knobs = Vec::new();

    for (card_id, card) in catalog {
        // Skip operator cards
        if card.supertype == CardSupertype::Apex {
            continue;
        }

        // Power knob for units
        if let Some(power) = card.power {
            knobs.push(KnobSpec {
                card_id: card_id.clone(),
                field: KnobField::Power,
                baseline: power,
                min: (power - 1).max(0),
                max: power + 1,
            });
        }

        // Toughness knob for units
        if let Some(toughness) = card.toughness {
            knobs.push(KnobSpec {
                card_id: card_id.clone(),
                field: KnobField::Toughness,
                baseline: toughness,
                min: (toughness - 1).max(1),
                max: toughness + 1,
            });
        }

        // Colorless mana knob (only if card has colorless cost and total >= 2)
        let total_cost = card.cost.neon
            + card.cost.glitch
            + card.cost.chrome
            + card.cost.slag
            + card.cost.bio
            + card.cost.colorless;
        if card.cost.colorless > 0 && total_cost >= 2 {
            knobs.push(KnobSpec {
                card_id: card_id.clone(),
                field: KnobField::ManaCost(ManaField::Colorless),
                baseline: card.cost.colorless,
                min: (card.cost.colorless - 1).max(0),
                max: card.cost.colorless + 1,
            });
        }
    }

    knobs
}

fn build_card_mode_knob_specs(
    catalog: &BTreeMap<String, Card>,
    card_filter: &[String],
) -> Result<Vec<KnobSpec>> {
    let mut knobs = build_card_knob_specs(catalog);
    knobs.extend(build_shortlisted_card_payload_knob_specs(
        catalog,
        card_filter,
    )?);
    knobs.sort_by(|left, right| left.label().cmp(&right.label()));
    Ok(knobs)
}

fn build_all_modes_knob_specs(
    catalog: &BTreeMap<String, Card>,
    card_filter: &[String],
) -> Result<Vec<KnobSpec>> {
    let mut knobs = build_card_mode_knob_specs(catalog, card_filter)?;
    knobs.extend(build_knob_specs(catalog)?);
    knobs.sort_by(|left, right| left.label().cmp(&right.label()));
    Ok(knobs)
}

fn build_shortlisted_card_payload_knob_specs(
    catalog: &BTreeMap<String, Card>,
    card_filter: &[String],
) -> Result<Vec<KnobSpec>> {
    let mut knobs = Vec::new();
    for card_id in card_filter {
        let card = require_card(catalog, card_id)?;
        for (index, trigger) in card.triggers.iter().enumerate() {
            for path in effect_paths(&trigger.effect) {
                knobs.push(trigger_effect_knob(catalog, card_id, index, path)?);
            }
        }
        for (index, ability) in card.abilities.iter().enumerate() {
            for path in effect_paths(&ability.effect) {
                knobs.push(ability_effect_knob(catalog, card_id, index, path)?);
            }
        }
    }
    Ok(knobs)
}

const EFFECT_FIELDS: [EffectField; 8] = [
    EffectField::Damage,
    EffectField::Draw,
    EffectField::LootDraw,
    EffectField::LootDiscard,
    EffectField::Scry,
    EffectField::Counters,
    EffectField::TokenPower,
    EffectField::TokenToughness,
];

/// Every tunable number in an effect, whether it stands alone or is a step in a sequence.
fn effect_paths(effect: &CardEffect) -> Vec<EffectPath> {
    let fields = |effect: &CardEffect| -> Vec<EffectField> {
        EFFECT_FIELDS.into_iter().filter(|field| effect_value(effect, *field).is_ok()).collect()
    };
    match effect {
        CardEffect::Sequence(steps) => steps
            .iter()
            .enumerate()
            .flat_map(|(effect_index, step)| {
                fields(step).into_iter().map(move |field| EffectPath::Sequence { effect_index, field })
            })
            .collect(),
        other => fields(other).into_iter().map(|field| EffectPath::Direct { field }).collect(),
    }
}

/// A curated knob whose card has since changed shape is skipped, so a
/// rebalance can't stop a tuning run.
fn skip_stale_knob(knob: Result<KnobSpec>) -> Option<KnobSpec> {
    knob.map_err(|error| eprintln!("skipping a stale operator knob: {error}")).ok()
}

fn build_knob_specs(catalog: &BTreeMap<String, Card>) -> Result<Vec<KnobSpec>> {
    let knobs: Vec<Result<KnobSpec>> = vec![
        colorless_mana_knob(catalog, "echo_signal"),
        primary_color_mana_knob(catalog, "echo_signal"),
        toughness_knob(catalog, "echo_signal"),
        trigger_effect_knob(
            catalog,
            "echo_signal",
            0,
            EffectPath::Sequence {
                effect_index: 0,
                field: EffectField::Scry,
            },
        ),
        trigger_effect_knob(
            catalog,
            "echo_signal",
            0,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::Draw,
            },
        ),
        colorless_mana_knob(catalog, "filter_parter_of_streams"),
        primary_color_mana_knob(catalog, "filter_parter_of_streams"),
        loyalty_knob(catalog, "filter_parter_of_streams"),
        ability_effect_knob(
            catalog,
            "filter_parter_of_streams",
            1,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::Damage,
            },
        ),
        loyalty_knob(catalog, "chronos_temporal_hacker"),
        colorless_mana_knob(catalog, "chronos_temporal_hacker"),
        primary_color_mana_knob(catalog, "chronos_temporal_hacker"),
        ability_effect_knob(
            catalog,
            "chronos_temporal_hacker",
            0,
            EffectPath::Sequence {
                effect_index: 2,
                field: EffectField::Draw,
            },
        ),
        colorless_mana_knob(catalog, "entropy_dreadhorde_general"),
        primary_color_mana_knob(catalog, "entropy_dreadhorde_general"),
        loyalty_knob(catalog, "entropy_dreadhorde_general"),
        ability_effect_knob(
            catalog,
            "entropy_dreadhorde_general",
            0,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::TokenPower,
            },
        ),
        ability_effect_knob(
            catalog,
            "entropy_dreadhorde_general",
            0,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::TokenToughness,
            },
        ),
        loyalty_knob(catalog, "entropy_weaver"),
        primary_color_mana_knob(catalog, "entropy_weaver"),
        ability_effect_knob(
            catalog,
            "entropy_weaver",
            0,
            EffectPath::Sequence {
                effect_index: 2,
                field: EffectField::Draw,
            },
        ),
        colorless_mana_knob(catalog, "null_lord_of_the_void"),
        primary_color_mana_knob(catalog, "null_lord_of_the_void"),
        ability_colorless_mana_knob(catalog, "null_lord_of_the_void", 0),
        ability_mana_knob(catalog, "null_lord_of_the_void", 0, ManaField::Glitch),
        colorless_mana_knob(catalog, "sai_forgemaster"),
        primary_color_mana_knob(catalog, "sai_forgemaster"),
        toughness_knob(catalog, "sai_forgemaster"),
        ability_effect_knob(
            catalog,
            "sai_forgemaster",
            0,
            EffectPath::Direct {
                field: EffectField::TokenPower,
            },
        ),
        ability_effect_knob(
            catalog,
            "sai_forgemaster",
            0,
            EffectPath::Direct {
                field: EffectField::TokenToughness,
            },
        ),
        colorless_mana_knob(catalog, "emry_salvager"),
        primary_color_mana_knob(catalog, "emry_salvager"),
        toughness_knob(catalog, "emry_salvager"),
        colorless_mana_knob(catalog, "pyra_torch_of_defiance"),
        primary_color_mana_knob(catalog, "pyra_torch_of_defiance"),
        loyalty_knob(catalog, "pyra_torch_of_defiance"),
        ability_effect_knob(
            catalog,
            "pyra_torch_of_defiance",
            0,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::Damage,
            },
        ),
        ability_effect_knob(
            catalog,
            "pyra_torch_of_defiance",
            1,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::Damage,
            },
        ),
        ability_effect_knob(
            catalog,
            "pyra_torch_of_defiance",
            2,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::Draw,
            },
        ),
        colorless_mana_knob(catalog, "ignitor_god_of_the_forge"),
        primary_color_mana_knob(catalog, "ignitor_god_of_the_forge"),
        trigger_effect_knob(
            catalog,
            "ignitor_god_of_the_forge",
            0,
            EffectPath::Direct {
                field: EffectField::Damage,
            },
        ),
        colorless_mana_knob(catalog, "mayhem_forgefiend"),
        primary_color_mana_knob(catalog, "mayhem_forgefiend"),
        power_knob(catalog, "mayhem_forgefiend"),
        toughness_knob(catalog, "mayhem_forgefiend"),
        trigger_effect_knob(
            catalog,
            "mayhem_forgefiend",
            0,
            EffectPath::Direct {
                field: EffectField::Damage,
            },
        ),
        colorless_mana_knob(catalog, "verdania_natures_fury"),
        primary_color_mana_knob(catalog, "verdania_natures_fury"),
        loyalty_knob(catalog, "verdania_natures_fury"),
        ability_effect_knob(
            catalog,
            "verdania_natures_fury",
            0,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::TokenPower,
            },
        ),
        ability_effect_knob(
            catalog,
            "verdania_natures_fury",
            0,
            EffectPath::Sequence {
                effect_index: 1,
                field: EffectField::TokenToughness,
            },
        ),
        colorless_mana_knob(catalog, "briar_champion_of_wilds"),
        primary_color_mana_knob(catalog, "briar_champion_of_wilds"),
        ability_effect_knob(
            catalog,
            "briar_champion_of_wilds",
            0,
            EffectPath::Sequence {
                effect_index: 2,
                field: EffectField::TokenPower,
            },
        ),
        ability_effect_knob(
            catalog,
            "briar_champion_of_wilds",
            0,
            EffectPath::Sequence {
                effect_index: 2,
                field: EffectField::TokenToughness,
            },
        ),
    ];
    Ok(knobs.into_iter().filter_map(skip_stale_knob).collect())
}

fn filter_knob_specs(knobs: Vec<KnobSpec>, operator_filter: &[String]) -> Vec<KnobSpec> {
    if operator_filter.is_empty() {
        return knobs;
    }

    let operator_decks = operator_deck_by_id();
    knobs
        .into_iter()
        .filter(|knob| {
            operator_filter.iter().any(|filter| {
                filter == &knob.card_id
                    || operator_decks
                        .get(knob.card_id.as_str())
                        .is_some_and(|deck_id| filter == deck_id)
            })
        })
        .collect()
}

fn filter_card_knob_specs(knobs: Vec<KnobSpec>, card_filter: &[String]) -> Vec<KnobSpec> {
    if card_filter.is_empty() {
        return knobs;
    }

    let wanted: BTreeSet<&str> = card_filter.iter().map(String::as_str).collect();
    knobs
        .into_iter()
        .filter(|knob| wanted.contains(knob.card_id.as_str()))
        .collect()
}

fn operator_deck_by_id() -> BTreeMap<&'static str, &'static str> {
    let mut mapping = BTreeMap::new();
    for deck_id in ["neon", "glitch", "chrome", "slag", "bio"] {
        for operator_id in operator_ids_for_color(deck_id) {
            mapping.insert(operator_id, deck_id);
        }
    }
    mapping
}

fn colorless_mana_knob(catalog: &BTreeMap<String, Card>, card_id: &str) -> Result<KnobSpec> {
    mana_knob(catalog, card_id, ManaField::Colorless)
}

fn primary_color_mana_knob(catalog: &BTreeMap<String, Card>, card_id: &str) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let mana = primary_operator_mana_field(card_id, &card.cost)
        .ok_or_else(|| anyhow!("{card_id} missing primary colored mana cost for autotune knob"))?;
    mana_knob(catalog, card_id, mana)
}

fn mana_knob(catalog: &BTreeMap<String, Card>, card_id: &str, mana: ManaField) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let baseline = mana_cost_field(&card.cost, mana);
    Ok(KnobSpec {
        card_id: card_id.to_string(),
        field: KnobField::ManaCost(mana),
        baseline,
        min: (baseline - 1).max(0),
        max: baseline + 1,
    })
}

fn ability_colorless_mana_knob(
    catalog: &BTreeMap<String, Card>,
    card_id: &str,
    ability_index: usize,
) -> Result<KnobSpec> {
    ability_mana_knob(catalog, card_id, ability_index, ManaField::Colorless)
}

fn ability_mana_knob(
    catalog: &BTreeMap<String, Card>,
    card_id: &str,
    ability_index: usize,
    mana: ManaField,
) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let ability = card
        .abilities
        .get(ability_index)
        .ok_or_else(|| anyhow!("{card_id} missing ability index {ability_index}"))?;
    let mana_cost = ability.cost.get_mana();
    let baseline = mana_cost_field(&mana_cost, mana);
    Ok(KnobSpec {
        card_id: card_id.to_string(),
        field: KnobField::AbilityManaCost {
            ability_index,
            mana,
        },
        baseline,
        min: (baseline - 1).max(0),
        max: baseline + 1,
    })
}

fn power_knob(catalog: &BTreeMap<String, Card>, card_id: &str) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let power = card
        .power
        .ok_or_else(|| anyhow!("{card_id} missing power for autotune knob"))?;
    Ok(KnobSpec {
        card_id: card_id.to_string(),
        field: KnobField::Power,
        baseline: power,
        min: (power - 1).max(0),
        max: power + 1,
    })
}

fn toughness_knob(catalog: &BTreeMap<String, Card>, card_id: &str) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let toughness = card
        .toughness
        .ok_or_else(|| anyhow!("{card_id} missing toughness for autotune knob"))?;
    Ok(KnobSpec {
        card_id: card_id.to_string(),
        field: KnobField::Toughness,
        baseline: toughness,
        min: (toughness - 1).max(0),
        max: toughness + 1,
    })
}

fn loyalty_knob(catalog: &BTreeMap<String, Card>, card_id: &str) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let loyalty = card
        .loyalty
        .ok_or_else(|| anyhow!("{card_id} missing loyalty for autotune knob"))?;
    Ok(KnobSpec {
        card_id: card_id.to_string(),
        field: KnobField::Loyalty,
        baseline: loyalty,
        min: (loyalty - 1).max(1),
        max: loyalty + 1,
    })
}

fn trigger_effect_knob(
    catalog: &BTreeMap<String, Card>,
    card_id: &str,
    trigger_index: usize,
    path: EffectPath,
) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let trigger = card
        .triggers
        .get(trigger_index)
        .ok_or_else(|| anyhow!("{card_id} missing trigger index {trigger_index}"))?;
    let baseline = effect_path_value(&trigger.effect, path)?;
    let (min, max) = effect_knob_bounds(path.field(), baseline);
    Ok(KnobSpec {
        card_id: card_id.to_string(),
        field: KnobField::TriggerEffect {
            trigger_index,
            path,
        },
        baseline,
        min,
        max,
    })
}

fn ability_effect_knob(
    catalog: &BTreeMap<String, Card>,
    card_id: &str,
    ability_index: usize,
    path: EffectPath,
) -> Result<KnobSpec> {
    let card = require_card(catalog, card_id)?;
    let ability = card
        .abilities
        .get(ability_index)
        .ok_or_else(|| anyhow!("{card_id} missing ability index {ability_index}"))?;
    let baseline = effect_path_value(&ability.effect, path)?;
    let (min, max) = effect_knob_bounds(path.field(), baseline);
    Ok(KnobSpec {
        card_id: card_id.to_string(),
        field: KnobField::AbilityEffect {
            ability_index,
            path,
        },
        baseline,
        min,
        max,
    })
}

fn require_card<'a>(catalog: &'a BTreeMap<String, Card>, card_id: &str) -> Result<&'a Card> {
    catalog
        .get(card_id)
        .ok_or_else(|| anyhow!("missing card {card_id} in catalog"))
}

fn load_base_patch(path: Option<&Path>) -> Result<Option<BalancePatch>> {
    match path {
        Some(path) => load_balance_patch_from_path(path).map(Some),
        None => Ok(None),
    }
}

fn apply_patch_to_catalog(
    catalog: &mut BTreeMap<String, Card>,
    patch: &BalancePatch,
) -> Result<()> {
    let mut cards = catalog
        .values()
        .cloned()
        .map(|card| (card, 1))
        .collect::<Vec<_>>();
    patch.apply_to_catalog_cards(&mut cards)?;
    *catalog = cards
        .into_iter()
        .map(|(card, _)| (card.id.clone(), card))
        .collect();
    Ok(())
}

fn run_search_stage(
    stage: &str,
    matches_per_pair: usize,
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
) -> Result<StageSummary> {
    let matrix_config = build_matrix_config(config, matches_per_pair);
    let mut stage_counter = 0usize;
    let mut seen_signatures = BTreeSet::new();
    let baseline = Candidate::baseline(knobs);
    let baseline_eval = evaluate_candidate(
        stage,
        stage_counter,
        0,
        &baseline,
        &matrix_config,
        catalog,
        knobs,
        run_dir,
        base_patch,
    )?;
    stage_counter += 1;
    seen_signatures.insert(baseline_eval.summary.signature.clone());
    let mut evaluated = vec![baseline_eval.clone()];
    let mut frontier = vec![baseline_eval];

    for generation in 1..=config.generations {
        let candidate_limit = config.candidates_per_generation.max(config.beam_width);
        let next_candidates =
            gather_generation_candidates(&frontier, knobs, &seen_signatures, candidate_limit);
        if next_candidates.is_empty() {
            break;
        }

        let mut generation_results = Vec::new();
        for candidate in next_candidates {
            let result = evaluate_candidate(
                stage,
                stage_counter,
                generation,
                &candidate,
                &matrix_config,
                catalog,
                knobs,
                run_dir,
                base_patch,
            )?;
            stage_counter += 1;
            seen_signatures.insert(result.summary.signature.clone());
            generation_results.push(result);
        }
        if generation_results.is_empty() {
            break;
        }

        generation_results.sort_by(compare_evaluated_candidates);
        frontier = generation_results
            .iter()
            .take(config.beam_width)
            .cloned()
            .collect();
        evaluated.extend(generation_results);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .take(config.beam_width.max(10))
        .map(|candidate| candidate.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("autotune stage {stage} evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, stage, &best)?;

    Ok(StageSummary {
        stage: stage.to_string(),
        matches_per_pair,
        evaluated_candidates: evaluated.len(),
        leaderboard,
        best,
    })
}

fn run_promotion_stage(
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    screening: &StageSummary,
    base_patch: Option<&BalancePatch>,
) -> Result<StageSummary> {
    let matrix_config = build_matrix_config(config, config.promotion_matches);
    let mut promoted_candidates = Vec::new();
    let mut seen = BTreeSet::new();

    for summary in screening
        .leaderboard
        .iter()
        .take(config.promotion_count.max(1))
    {
        if seen.insert(summary.signature.clone()) {
            promoted_candidates.push(candidate_from_signature(summary, knobs)?);
        }
    }

    let mut evaluated = Vec::new();
    for (index, candidate) in promoted_candidates.iter().enumerate() {
        evaluated.push(evaluate_candidate(
            "promotion",
            index,
            0,
            candidate,
            &matrix_config,
            catalog,
            knobs,
            run_dir,
            base_patch,
        )?);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .map(|candidate| candidate.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("promotion stage evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, "promotion", &best)?;

    Ok(StageSummary {
        stage: "promotion".to_string(),
        matches_per_pair: config.promotion_matches,
        evaluated_candidates: leaderboard.len(),
        leaderboard,
        best,
    })
}

fn build_matrix_config(config: &CliConfig, matches_per_pair: usize) -> OperatorMatrixConfig {
    let mut matrix_config = OperatorMatrixConfig::default();
    matrix_config.base_match.seed = config.seed;
    matrix_config.base_match.format = GameFormat::Operator;
    matrix_config.matches_per_pair = matches_per_pair;
    matrix_config.include_mirrors = config.include_mirrors;
    matrix_config.operator_filter = config.operator_filter.clone();
    matrix_config
}

fn build_all_modes_operator_config(
    config: &CliConfig,
    matches_per_pair: usize,
    catalog: &BTreeMap<String, Card>,
) -> OperatorMatrixConfig {
    let mut matrix_config = build_matrix_config(config, matches_per_pair);
    if matrix_config.operator_filter.is_empty() {
        matrix_config.operator_filter =
            infer_operator_filter_from_cards(catalog, &config.card_filter);
    }
    matrix_config
}

fn infer_operator_filter_from_cards(
    catalog: &BTreeMap<String, Card>,
    card_filter: &[String],
) -> Vec<String> {
    let mut decks = BTreeSet::new();
    for card_id in card_filter {
        decks.extend(deck_ids_for_card(card_id, catalog));
    }
    decks.into_iter().collect()
}

fn deck_ids_for_card(card_id: &str, catalog: &BTreeMap<String, Card>) -> BTreeSet<String> {
    let mut deck_ids = BTreeSet::new();
    for deck_id in ["neon", "glitch", "chrome", "slag", "bio"] {
        if card_catalog_contains_card(deck_id, card_id) {
            deck_ids.insert(deck_id.to_string());
        }
    }

    if deck_ids.is_empty() {
        let Some(card) = catalog.get(card_id) else {
            return deck_ids;
        };
        for mana in card_color_identity(card) {
            if let Some(deck_id) = mana_type_to_deck_id(mana) {
                deck_ids.insert(deck_id.to_string());
            }
        }
    }

    deck_ids
}

fn card_catalog_contains_card(deck_id: &str, card_id: &str) -> bool {
    match deck_id {
        "neon" => card_catalog::neon::all_cards()
            .iter()
            .any(|(card, _)| card.id == card_id),
        "glitch" => card_catalog::glitch::all_cards()
            .iter()
            .any(|(card, _)| card.id == card_id),
        "chrome" => card_catalog::chrome::all_cards()
            .iter()
            .any(|(card, _)| card.id == card_id),
        "slag" => card_catalog::slag::all_cards()
            .iter()
            .any(|(card, _)| card.id == card_id),
        "bio" => card_catalog::bio::all_cards()
            .iter()
            .any(|(card, _)| card.id == card_id),
        _ => false,
    }
}

fn mana_type_to_deck_id(mana: ManaType) -> Option<&'static str> {
    match mana {
        ManaType::Neon => Some("neon"),
        ManaType::Glitch => Some("glitch"),
        ManaType::Chrome => Some("chrome"),
        ManaType::Slag => Some("slag"),
        ManaType::Bio => Some("bio"),
        ManaType::Colorless => None,
    }
}

fn card_color_identity(card: &Card) -> HashSet<ManaType> {
    let mut identity = HashSet::new();
    collect_mana_cost_colors(&card.cost, &mut identity);
    for ability in &card.abilities {
        collect_mana_cost_colors(&ability.cost.get_mana(), &mut identity);
    }
    for trigger in &card.triggers {
        if let Some(cost) = &trigger.optional_cost {
            collect_mana_cost_colors(&cost.get_mana(), &mut identity);
        }
    }
    identity
}

fn collect_mana_cost_colors(cost: &ManaCost, identity: &mut HashSet<ManaType>) {
    if cost.neon > 0 {
        identity.insert(ManaType::Neon);
    }
    if cost.glitch > 0 {
        identity.insert(ManaType::Glitch);
    }
    if cost.chrome > 0 {
        identity.insert(ManaType::Chrome);
    }
    if cost.slag > 0 {
        identity.insert(ManaType::Slag);
    }
    if cost.bio > 0 {
        identity.insert(ManaType::Bio);
    }
}

fn gather_generation_candidates(
    frontier: &[EvaluatedCandidate],
    knobs: &[KnobSpec],
    seen_signatures: &BTreeSet<String>,
    limit: usize,
) -> Vec<Candidate> {
    if let Some(candidates) =
        gather_first_generation_candidates_diversified(frontier, knobs, seen_signatures, limit)
    {
        return candidates;
    }

    let mut batches: Vec<Vec<Candidate>> = frontier
        .iter()
        .map(|candidate| candidate.candidate.neighbors(knobs))
        .collect();
    let mut cursors = vec![0usize; batches.len()];
    let mut selected = Vec::new();
    let mut selected_signatures = BTreeSet::new();

    loop {
        let mut progressed = false;
        for (batch_index, batch) in batches.iter_mut().enumerate() {
            while cursors[batch_index] < batch.len() {
                let candidate = batch[cursors[batch_index]].clone();
                cursors[batch_index] += 1;
                let signature = candidate.signature(knobs);
                if seen_signatures.contains(&signature) || !selected_signatures.insert(signature) {
                    continue;
                }
                selected.push(candidate);
                progressed = true;
                break;
            }
            if selected.len() >= limit {
                return selected;
            }
        }
        if !progressed {
            return selected;
        }
    }
}

fn gather_first_generation_candidates_diversified(
    frontier: &[EvaluatedCandidate],
    knobs: &[KnobSpec],
    seen_signatures: &BTreeSet<String>,
    limit: usize,
) -> Option<Vec<Candidate>> {
    if frontier.len() != 1 || frontier[0].summary.generation != 0 {
        return None;
    }

    let baseline_neighbors = frontier[0].candidate.neighbors(knobs);
    let mut group_order = Vec::new();
    let mut grouped: BTreeMap<String, Vec<VecDeque<Candidate>>> = BTreeMap::new();
    let mut per_card_knob_labels: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for candidate in baseline_neighbors {
        let Some(card_id) = first_changed_card_id(&candidate, knobs) else {
            continue;
        };
        let Some(knob_label) = first_changed_knob_label(&candidate, knobs) else {
            continue;
        };
        if !grouped.contains_key(&card_id) {
            group_order.push(card_id.clone());
        }
        let knob_labels = per_card_knob_labels.entry(card_id.clone()).or_default();
        let knob_index =
            if let Some(index) = knob_labels.iter().position(|label| label == &knob_label) {
                index
            } else {
                knob_labels.push(knob_label);
                knob_labels.len() - 1
            };
        let per_knob_queues = grouped.entry(card_id).or_default();
        while per_knob_queues.len() <= knob_index {
            per_knob_queues.push(VecDeque::new());
        }
        per_knob_queues[knob_index].push_back(candidate);
    }

    let mut selected = Vec::new();
    let mut selected_signatures = BTreeSet::new();
    let mut per_card_cursors: BTreeMap<String, usize> = BTreeMap::new();

    loop {
        let mut progressed = false;
        for card_id in &group_order {
            let Some(per_knob_queues) = grouped.get_mut(card_id) else {
                continue;
            };
            if per_knob_queues.is_empty() {
                continue;
            }
            let cursor = per_card_cursors.entry(card_id.clone()).or_insert(0);
            let queue_count = per_knob_queues.len();
            let mut card_progressed = false;
            for _ in 0..queue_count {
                let queue_index = *cursor % queue_count;
                *cursor = (*cursor + 1) % queue_count;
                let queue = &mut per_knob_queues[queue_index];
                while let Some(candidate) = queue.pop_front() {
                    let signature = candidate.signature(knobs);
                    if seen_signatures.contains(&signature)
                        || !selected_signatures.insert(signature)
                    {
                        continue;
                    }
                    selected.push(candidate);
                    progressed = true;
                    card_progressed = true;
                    break;
                }
                if card_progressed {
                    break;
                }
            }
            if selected.len() >= limit {
                return Some(selected);
            }
        }
        if !progressed {
            return Some(selected);
        }
    }
}

fn first_changed_card_id(candidate: &Candidate, knobs: &[KnobSpec]) -> Option<String> {
    candidate
        .values
        .iter()
        .zip(knobs)
        .find(|(value, spec)| **value != spec.baseline)
        .map(|(_, spec)| spec.card_id.clone())
}

fn first_changed_knob_label(candidate: &Candidate, knobs: &[KnobSpec]) -> Option<String> {
    candidate
        .values
        .iter()
        .zip(knobs)
        .find(|(value, spec)| **value != spec.baseline)
        .map(|(_, spec)| spec.label())
}

fn evaluate_candidate(
    stage: &str,
    candidate_index: usize,
    generation: usize,
    candidate: &Candidate,
    matrix_config: &OperatorMatrixConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
) -> Result<EvaluatedCandidate> {
    let signature = candidate.signature(knobs);
    let changed_knobs = candidate.changed_knobs(knobs);
    let candidate_patch = build_patch(candidate, knobs, catalog)?;
    let patch = merge_balance_patches(base_patch, &candidate_patch);
    patch.validate()?;
    let artifact_dir = prepare_candidate_artifact_dir(
        run_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        changed_knobs.clone(),
        &patch,
        &candidate_patch,
        base_patch,
    )?;
    let report = run_operator_matrix_with_patch(matrix_config, Some(&patch));
    let score = score_report(&report);
    let outliers = summarize_outliers(&report);
    write_json(artifact_dir.join("report.json"), &report)?;
    write_candidate_artifact_status(
        &artifact_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        &changed_knobs,
        "complete",
        &["operator"],
    )?;

    let summary = CandidateSummary {
        stage: stage.to_string(),
        candidate_index,
        generation,
        signature,
        changed_knobs,
        score,
        outliers,
        finished_match_rate: report.balance.finished_match_rate,
        decisive_match_rate: report.balance.decisive_match_rate,
        unfinished_match_rate: report.balance.unfinished_match_rate,
        termination_counts: report.balance.termination_counts.clone(),
        artifact_dir: artifact_dir.display().to_string(),
    };
    write_json(artifact_dir.join("summary.json"), &summary)?;

    Ok(EvaluatedCandidate {
        candidate: candidate.clone(),
        summary,
    })
}

fn build_patch(
    candidate: &Candidate,
    knobs: &[KnobSpec],
    catalog: &BTreeMap<String, Card>,
) -> Result<BalancePatch> {
    let mut card_patches: BTreeMap<String, CardPatch> = BTreeMap::new();
    let mut deck_entries: Vec<DeckEntryPatch> = Vec::new();

    for (value, spec) in candidate.values.iter().zip(knobs) {
        if *value == spec.baseline {
            continue;
        }

        match &spec.field {
            KnobField::DeckQuantity { deck_id } => {
                deck_entries.push(DeckEntryPatch {
                    deck_id: deck_id.clone(),
                    card_id: spec.card_id.clone(),
                    quantity: *value,
                });
            }
            _ => {
                let card = require_card(catalog, &spec.card_id)?;
                let patch = card_patches
                    .entry(spec.card_id.to_string())
                    .or_insert_with(|| CardPatch {
                        card_id: spec.card_id.to_string(),
                        mana_cost: None,
                        power: None,
                        toughness: None,
                        loyalty: None,
                        keywords: None,
                        add_keywords: Vec::new(),
                        remove_keywords: Vec::new(),
                        trigger_patches: Vec::new(),
                        ability_patches: Vec::new(),
                    });

                match &spec.field {
                    KnobField::ManaCost(mana_field) => {
                        let mut cost = patch.mana_cost.clone().unwrap_or_else(|| card.cost.clone());
                        set_mana_field(&mut cost, *mana_field, *value);
                        patch.mana_cost = Some(cost);
                    }
                    KnobField::Power => patch.power = Some(*value),
                    KnobField::Toughness => patch.toughness = Some(*value),
                    KnobField::Loyalty => patch.loyalty = Some(*value),
                    KnobField::AbilityManaCost {
                        ability_index,
                        mana,
                    } => {
                        let ability_patch =
                            get_or_insert_ability_patch(patch, card, *ability_index)?;
                        let mut cost = ability_patch
                            .mana_cost
                            .clone()
                            .unwrap_or_else(|| card.abilities[*ability_index].cost.get_mana());
                        set_mana_field(&mut cost, *mana, *value);
                        ability_patch.mana_cost = Some(cost);
                    }
                    KnobField::TriggerEffect {
                        trigger_index,
                        path,
                    } => {
                        let trigger_patch =
                            get_or_insert_trigger_patch(patch, card, *trigger_index)?;
                        let mut effect = trigger_patch
                            .effect
                            .clone()
                            .unwrap_or_else(|| card.triggers[*trigger_index].effect.clone());
                        set_effect_path_value(&mut effect, *path, *value)?;
                        trigger_patch.effect = Some(effect);
                    }
                    KnobField::AbilityEffect {
                        ability_index,
                        path,
                    } => {
                        let ability_patch =
                            get_or_insert_ability_patch(patch, card, *ability_index)?;
                        let mut effect = ability_patch
                            .effect
                            .clone()
                            .unwrap_or_else(|| card.abilities[*ability_index].effect.clone());
                        set_effect_path_value(&mut effect, *path, *value)?;
                        ability_patch.effect = Some(effect);
                    }
                    KnobField::DeckQuantity { .. } => unreachable!(),
                }
            }
        }
    }

    Ok(BalancePatch {
        cards: card_patches.into_values().collect(),
        deck_entries,
    })
}

fn merge_balance_patches(
    base_patch: Option<&BalancePatch>,
    candidate_patch: &BalancePatch,
) -> BalancePatch {
    let Some(base_patch) = base_patch else {
        return candidate_patch.clone();
    };

    let mut cards: BTreeMap<String, CardPatch> = base_patch
        .cards
        .iter()
        .cloned()
        .map(|patch| (patch.card_id.clone(), patch))
        .collect();
    for patch in &candidate_patch.cards {
        cards
            .entry(patch.card_id.clone())
            .and_modify(|existing| merge_card_patch(existing, patch))
            .or_insert_with(|| patch.clone());
    }

    let mut deck_entries: BTreeMap<(String, String), DeckEntryPatch> = base_patch
        .deck_entries
        .iter()
        .cloned()
        .map(|patch| ((patch.deck_id.clone(), patch.card_id.clone()), patch))
        .collect();
    for patch in &candidate_patch.deck_entries {
        deck_entries.insert(
            (patch.deck_id.clone(), patch.card_id.clone()),
            patch.clone(),
        );
    }

    BalancePatch {
        cards: cards.into_values().collect(),
        deck_entries: deck_entries.into_values().collect(),
    }
}

fn merge_card_patch(existing: &mut CardPatch, overlay: &CardPatch) {
    if overlay.mana_cost.is_some() {
        existing.mana_cost = overlay.mana_cost.clone();
    }
    if overlay.power.is_some() {
        existing.power = overlay.power;
    }
    if overlay.toughness.is_some() {
        existing.toughness = overlay.toughness;
    }
    if overlay.loyalty.is_some() {
        existing.loyalty = overlay.loyalty;
    }
    if overlay.keywords.is_some() {
        existing.keywords = overlay.keywords.clone();
    }
    merge_unique_values(&mut existing.add_keywords, &overlay.add_keywords);
    merge_unique_values(&mut existing.remove_keywords, &overlay.remove_keywords);

    let mut trigger_patches: BTreeMap<usize, TriggerPatch> = existing
        .trigger_patches
        .iter()
        .cloned()
        .map(|patch| (patch.index, patch))
        .collect();
    for patch in &overlay.trigger_patches {
        trigger_patches
            .entry(patch.index)
            .and_modify(|existing| merge_trigger_patch(existing, patch))
            .or_insert_with(|| patch.clone());
    }
    existing.trigger_patches = trigger_patches.into_values().collect();

    let mut ability_patches: BTreeMap<usize, AbilityPatch> = existing
        .ability_patches
        .iter()
        .cloned()
        .map(|patch| (patch.index, patch))
        .collect();
    for patch in &overlay.ability_patches {
        ability_patches
            .entry(patch.index)
            .and_modify(|existing| merge_ability_patch(existing, patch))
            .or_insert_with(|| patch.clone());
    }
    existing.ability_patches = ability_patches.into_values().collect();
}

fn merge_trigger_patch(existing: &mut TriggerPatch, overlay: &TriggerPatch) {
    if overlay.limit.is_some() {
        existing.limit = overlay.limit.clone();
    }
    if overlay.effect.is_some() {
        existing.effect = overlay.effect.clone();
    }
}

fn merge_ability_patch(existing: &mut AbilityPatch, overlay: &AbilityPatch) {
    if overlay.mana_cost.is_some() {
        existing.mana_cost = overlay.mana_cost.clone();
    }
    if overlay.once_per_turn.is_some() {
        existing.once_per_turn = overlay.once_per_turn;
    }
    if overlay.effect.is_some() {
        existing.effect = overlay.effect.clone();
    }
}

fn merge_unique_values<T>(existing: &mut Vec<T>, overlay: &[T])
where
    T: Clone + PartialEq,
{
    for value in overlay {
        if !existing.contains(value) {
            existing.push(value.clone());
        }
    }
}

fn score_classic_report(report: &MatrixReport) -> CandidateScore {
    let abnormal_terminations = report
        .balance
        .termination_counts
        .iter()
        .filter(|t| t.reason != TerminationReason::Finished)
        .map(|t| t.count)
        .sum();
    let strict_band_misses = report
        .deck_metrics
        .iter()
        .filter(|m| strict_band_distance(m.win_rate) > 0.0)
        .count();
    let strict_band_distance_sum: f64 = report
        .deck_metrics
        .iter()
        .map(|m| strict_band_distance(m.win_rate))
        .sum();

    CandidateScore {
        abnormal_terminations,
        strict_band_misses,
        strict_band_distance_sum,
        balance_score: report.balance.balance_score,
        intra_color_spread_sum: 0.0,
    }
}

fn summarize_classic_outliers(report: &MatrixReport) -> Vec<OperatorOutlier> {
    let mut outliers: Vec<_> = report
        .deck_metrics
        .iter()
        .filter_map(|m| {
            let distance = strict_band_distance(m.win_rate);
            if distance == 0.0 {
                None
            } else {
                Some(OperatorOutlier {
                    combo_label: m.deck_id.clone(),
                    win_rate: m.win_rate,
                    distance_to_band: distance,
                })
            }
        })
        .collect();
    outliers.sort_by(|a, b| b.distance_to_band.total_cmp(&a.distance_to_band));
    outliers
}

fn build_classic_matrix_config(config: &CliConfig, matches_per_pair: usize) -> MatrixConfig {
    let mut matrix_config = MatrixConfig::default();
    matrix_config.base_match.seed = config.seed;
    matrix_config.matches_per_pair = matches_per_pair;
    matrix_config
}

fn evaluate_card_candidate(
    stage: &str,
    candidate_index: usize,
    generation: usize,
    candidate: &Candidate,
    matrix_config: &MatrixConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
) -> Result<EvaluatedCandidate> {
    let signature = candidate.signature(knobs);
    let changed_knobs = candidate.changed_knobs(knobs);
    let candidate_patch = build_patch(candidate, knobs, catalog)?;
    let patch = merge_balance_patches(base_patch, &candidate_patch);
    patch.validate()?;
    let artifact_dir = prepare_candidate_artifact_dir(
        run_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        changed_knobs.clone(),
        &patch,
        &candidate_patch,
        base_patch,
    )?;
    let report = run_matrix_with_patch(matrix_config, Some(&patch));
    let score = score_classic_report(&report);
    let outliers = summarize_classic_outliers(&report);
    write_json(artifact_dir.join("report.json"), &report)?;
    write_candidate_artifact_status(
        &artifact_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        &changed_knobs,
        "complete",
        &["classic"],
    )?;

    let summary = CandidateSummary {
        stage: stage.to_string(),
        candidate_index,
        generation,
        signature,
        changed_knobs,
        score,
        outliers,
        finished_match_rate: report.balance.finished_match_rate,
        decisive_match_rate: report.balance.decisive_match_rate,
        unfinished_match_rate: report.balance.unfinished_match_rate,
        termination_counts: report.balance.termination_counts.clone(),
        artifact_dir: artifact_dir.display().to_string(),
    };
    write_json(artifact_dir.join("summary.json"), &summary)?;

    Ok(EvaluatedCandidate {
        candidate: candidate.clone(),
        summary,
    })
}

fn run_card_search_stage(
    stage: &str,
    matches_per_pair: usize,
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
) -> Result<StageSummary> {
    let matrix_config = build_classic_matrix_config(config, matches_per_pair);
    let mut stage_counter = 0usize;
    let mut seen_signatures = BTreeSet::new();
    let baseline = Candidate::baseline(knobs);
    let baseline_eval = evaluate_card_candidate(
        stage,
        stage_counter,
        0,
        &baseline,
        &matrix_config,
        catalog,
        knobs,
        run_dir,
        base_patch,
    )?;
    stage_counter += 1;
    eprintln!(
        "Baseline: band_misses={} distance={:.3} balance={:.3}",
        baseline_eval.summary.score.strict_band_misses,
        baseline_eval.summary.score.strict_band_distance_sum,
        baseline_eval.summary.score.balance_score,
    );
    seen_signatures.insert(baseline_eval.summary.signature.clone());
    let mut evaluated = vec![baseline_eval.clone()];
    let mut frontier = vec![baseline_eval];

    for generation in 1..=config.generations {
        let candidate_limit = config.candidates_per_generation.max(config.beam_width);
        let next_candidates =
            gather_generation_candidates(&frontier, knobs, &seen_signatures, candidate_limit);
        if next_candidates.is_empty() {
            eprintln!("Generation {generation}: no new candidates, stopping");
            break;
        }
        eprintln!(
            "Generation {generation}: evaluating {} candidates...",
            next_candidates.len()
        );

        let mut generation_results = Vec::new();
        for candidate in next_candidates {
            let result = evaluate_card_candidate(
                stage,
                stage_counter,
                generation,
                &candidate,
                &matrix_config,
                catalog,
                knobs,
                run_dir,
                base_patch,
            )?;
            stage_counter += 1;
            seen_signatures.insert(result.summary.signature.clone());
            generation_results.push(result);
        }
        if generation_results.is_empty() {
            break;
        }

        generation_results.sort_by(compare_evaluated_candidates);
        let best = &generation_results[0];
        eprintln!(
            "  Best: band_misses={} distance={:.3} balance={:.3} [{}]",
            best.summary.score.strict_band_misses,
            best.summary.score.strict_band_distance_sum,
            best.summary.score.balance_score,
            best.summary.signature,
        );

        frontier = generation_results
            .iter()
            .take(config.beam_width)
            .cloned()
            .collect();
        evaluated.extend(generation_results);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .take(config.beam_width.max(10))
        .map(|c| c.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("card autotune stage {stage} evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, stage, &best)?;

    Ok(StageSummary {
        stage: stage.to_string(),
        matches_per_pair,
        evaluated_candidates: evaluated.len(),
        leaderboard,
        best,
    })
}

fn run_card_promotion_stage(
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    screening: &StageSummary,
    base_patch: Option<&BalancePatch>,
) -> Result<StageSummary> {
    let matrix_config = build_classic_matrix_config(config, config.promotion_matches);
    let mut promoted_candidates = Vec::new();
    let mut seen = BTreeSet::new();

    for summary in screening
        .leaderboard
        .iter()
        .take(config.promotion_count.max(1))
    {
        if seen.insert(summary.signature.clone()) {
            promoted_candidates.push(candidate_from_signature(summary, knobs)?);
        }
    }

    let mut evaluated = Vec::new();
    for (index, candidate) in promoted_candidates.iter().enumerate() {
        evaluated.push(evaluate_card_candidate(
            "promotion",
            index,
            0,
            candidate,
            &matrix_config,
            catalog,
            knobs,
            run_dir,
            base_patch,
        )?);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .map(|candidate| candidate.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("card promotion stage evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, "promotion", &best)?;

    Ok(StageSummary {
        stage: "promotion".to_string(),
        matches_per_pair: config.promotion_matches,
        evaluated_candidates: leaderboard.len(),
        leaderboard,
        best,
    })
}

fn build_draft_matrix_config(config: &CliConfig, matches_per_draft: usize) -> DraftMatrixConfig {
    DraftMatrixConfig {
        base_match: cardgame::sim::config::MatchConfig::default(),
        draft_count: config.draft_count.max(1),
        matches_per_draft: matches_per_draft.max(5),
        alternate_seats: true,
        base_seed: config.seed,
    }
}

fn score_draft_report(report: &DraftMatrixReport) -> CandidateScore {
    let abnormal_terminations = report
        .balance
        .termination_counts
        .iter()
        .filter(|t| t.reason != TerminationReason::Finished)
        .map(|t| t.count)
        .sum();
    let strict_band_misses = report
        .color_metrics
        .iter()
        .filter(|m| draft_strict_band_distance(m) > 0.0)
        .count();
    let strict_band_distance_sum: f64 = report
        .color_metrics
        .iter()
        .map(draft_strict_band_distance)
        .sum();

    CandidateScore {
        abnormal_terminations,
        strict_band_misses,
        strict_band_distance_sum,
        balance_score: report.balance.balance_score,
        intra_color_spread_sum: 0.0,
    }
}

fn draft_strict_band_distance(metric: &cardgame::sim::report::DraftColorMetrics) -> f64 {
    if metric.wins + metric.losses == 0 {
        0.0
    } else {
        strict_band_distance(metric.win_rate)
    }
}

fn summarize_draft_outliers(report: &DraftMatrixReport) -> Vec<OperatorOutlier> {
    let mut outliers: Vec<_> = report
        .color_metrics
        .iter()
        .filter_map(|m| {
            let distance = draft_strict_band_distance(m);
            if distance == 0.0 {
                None
            } else {
                Some(OperatorOutlier {
                    combo_label: m.color.clone(),
                    win_rate: m.win_rate,
                    distance_to_band: distance,
                })
            }
        })
        .collect();
    outliers.sort_by(|a, b| b.distance_to_band.total_cmp(&a.distance_to_band));
    outliers
}

fn evaluate_draft_candidate(
    stage: &str,
    candidate_index: usize,
    generation: usize,
    candidate: &Candidate,
    draft_config: &DraftMatrixConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
) -> Result<EvaluatedCandidate> {
    let signature = candidate.signature(knobs);
    let changed_knobs = candidate.changed_knobs(knobs);
    let candidate_patch = build_patch(candidate, knobs, catalog)?;
    let patch = merge_balance_patches(base_patch, &candidate_patch);
    patch.validate()?;
    let artifact_dir = prepare_candidate_artifact_dir(
        run_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        changed_knobs.clone(),
        &patch,
        &candidate_patch,
        base_patch,
    )?;
    let report = run_draft_matrix_with_patch(draft_config, Some(&patch));
    let score = score_draft_report(&report);
    let outliers = summarize_draft_outliers(&report);
    write_json(artifact_dir.join("report.json"), &report)?;
    write_candidate_artifact_status(
        &artifact_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        &changed_knobs,
        "complete",
        &["draft"],
    )?;

    let summary = CandidateSummary {
        stage: stage.to_string(),
        candidate_index,
        generation,
        signature,
        changed_knobs,
        score,
        outliers,
        finished_match_rate: report.balance.finished_match_rate,
        decisive_match_rate: report.balance.decisive_match_rate,
        unfinished_match_rate: report.balance.unfinished_match_rate,
        termination_counts: report.balance.termination_counts.clone(),
        artifact_dir: artifact_dir.display().to_string(),
    };
    write_json(artifact_dir.join("summary.json"), &summary)?;

    Ok(EvaluatedCandidate {
        candidate: candidate.clone(),
        summary,
    })
}

fn run_draft_search_stage(
    stage: &str,
    matches_per_draft: usize,
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
) -> Result<StageSummary> {
    let draft_config = build_draft_matrix_config(config, matches_per_draft);
    let mut stage_counter = 0usize;
    let mut seen_signatures = BTreeSet::new();
    let baseline = Candidate::baseline(knobs);
    let baseline_eval = evaluate_draft_candidate(
        stage,
        stage_counter,
        0,
        &baseline,
        &draft_config,
        catalog,
        knobs,
        run_dir,
        base_patch,
    )?;
    stage_counter += 1;
    eprintln!(
        "Baseline: band_misses={} distance={:.3} balance={:.3}",
        baseline_eval.summary.score.strict_band_misses,
        baseline_eval.summary.score.strict_band_distance_sum,
        baseline_eval.summary.score.balance_score,
    );
    for outlier in &baseline_eval.summary.outliers {
        eprintln!(
            "  {} WR: {:.1}% (distance: {:.3})",
            outlier.combo_label,
            outlier.win_rate.unwrap_or(0.0) * 100.0,
            outlier.distance_to_band,
        );
    }
    seen_signatures.insert(baseline_eval.summary.signature.clone());
    let mut evaluated = vec![baseline_eval.clone()];
    let mut frontier = vec![baseline_eval];

    for generation in 1..=config.generations {
        let candidate_limit = config.candidates_per_generation.max(config.beam_width);
        let next_candidates =
            gather_generation_candidates(&frontier, knobs, &seen_signatures, candidate_limit);
        if next_candidates.is_empty() {
            eprintln!("Generation {generation}: no new candidates, stopping");
            break;
        }
        eprintln!(
            "Generation {generation}: evaluating {} candidates...",
            next_candidates.len()
        );

        let mut generation_results = Vec::new();
        for candidate in next_candidates {
            let result = evaluate_draft_candidate(
                stage,
                stage_counter,
                generation,
                &candidate,
                &draft_config,
                catalog,
                knobs,
                run_dir,
                base_patch,
            )?;
            stage_counter += 1;
            seen_signatures.insert(result.summary.signature.clone());
            generation_results.push(result);
        }
        if generation_results.is_empty() {
            break;
        }

        generation_results.sort_by(compare_evaluated_candidates);
        let best = &generation_results[0];
        eprintln!(
            "  Best: band_misses={} distance={:.3} balance={:.3} [{}]",
            best.summary.score.strict_band_misses,
            best.summary.score.strict_band_distance_sum,
            best.summary.score.balance_score,
            best.summary.signature,
        );

        frontier = generation_results
            .iter()
            .take(config.beam_width)
            .cloned()
            .collect();
        evaluated.extend(generation_results);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .take(config.beam_width.max(10))
        .map(|c| c.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("draft autotune stage {stage} evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, stage, &best)?;

    Ok(StageSummary {
        stage: stage.to_string(),
        matches_per_pair: matches_per_draft,
        evaluated_candidates: evaluated.len(),
        leaderboard,
        best,
    })
}

fn run_draft_promotion_stage(
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    screening: &StageSummary,
    base_patch: Option<&BalancePatch>,
) -> Result<StageSummary> {
    let draft_config = build_draft_matrix_config(config, config.promotion_matches);
    let mut promoted_candidates = Vec::new();
    let mut seen = BTreeSet::new();

    for summary in screening
        .leaderboard
        .iter()
        .take(config.promotion_count.max(1))
    {
        if seen.insert(summary.signature.clone()) {
            promoted_candidates.push(candidate_from_signature(summary, knobs)?);
        }
    }

    let mut evaluated = Vec::new();
    for (index, candidate) in promoted_candidates.iter().enumerate() {
        evaluated.push(evaluate_draft_candidate(
            "promotion",
            index,
            0,
            candidate,
            &draft_config,
            catalog,
            knobs,
            run_dir,
            base_patch,
        )?);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .map(|candidate| candidate.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("draft promotion stage evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, "promotion", &best)?;

    Ok(StageSummary {
        stage: "promotion".to_string(),
        matches_per_pair: config.promotion_matches,
        evaluated_candidates: leaderboard.len(),
        leaderboard,
        best,
    })
}

fn total_classic_matches(report: &MatrixReport) -> usize {
    report
        .matchup_metrics
        .iter()
        .map(|metric| metric.matches)
        .sum()
}

fn total_operator_matches(report: &OperatorMatrixReport) -> usize {
    report
        .matchup_metrics
        .iter()
        .map(|metric| metric.matches)
        .sum()
}

fn weighted_rate(entries: &[(usize, f64)]) -> f64 {
    let total_weight: usize = entries.iter().map(|(weight, _)| *weight).sum();
    if total_weight == 0 {
        0.0
    } else {
        entries
            .iter()
            .map(|(weight, value)| *weight as f64 * *value)
            .sum::<f64>()
            / total_weight as f64
    }
}

fn merge_termination_counts(
    groups: impl IntoIterator<Item = Vec<TerminationCount>>,
) -> Vec<TerminationCount> {
    let mut merged = BTreeMap::new();
    for group in groups {
        for count in group {
            *merged.entry(count.reason).or_insert(0usize) += count.count;
        }
    }
    merged
        .into_iter()
        .map(|(reason, count)| TerminationCount { reason, count })
        .collect()
}

fn score_all_modes_reports(
    classic: &MatrixReport,
    operator: &OperatorMatrixReport,
    draft: &DraftMatrixReport,
) -> CandidateScore {
    let classic_score = score_classic_report(classic);
    let operator_score = score_report(operator);
    let draft_score = score_draft_report(draft);

    CandidateScore {
        abnormal_terminations: classic_score.abnormal_terminations
            + operator_score.abnormal_terminations
            + draft_score.abnormal_terminations,
        strict_band_misses: classic_score.strict_band_misses
            + operator_score.strict_band_misses
            + draft_score.strict_band_misses,
        strict_band_distance_sum: classic_score.strict_band_distance_sum
            + operator_score.strict_band_distance_sum
            + draft_score.strict_band_distance_sum,
        balance_score: classic_score.balance_score
            + operator_score.balance_score
            + draft_score.balance_score,
        intra_color_spread_sum: operator_score.intra_color_spread_sum,
    }
}

fn summarize_all_modes_outliers(
    classic: &MatrixReport,
    operator: &OperatorMatrixReport,
    draft: &DraftMatrixReport,
) -> Vec<OperatorOutlier> {
    let mut outliers = Vec::new();
    outliers.extend(
        summarize_classic_outliers(classic)
            .into_iter()
            .map(|outlier| OperatorOutlier {
                combo_label: format!("classic:{}", outlier.combo_label),
                ..outlier
            }),
    );
    outliers.extend(
        summarize_outliers(operator)
            .into_iter()
            .map(|outlier| OperatorOutlier {
                combo_label: format!("operator:{}", outlier.combo_label),
                ..outlier
            }),
    );
    outliers.extend(
        summarize_draft_outliers(draft)
            .into_iter()
            .map(|outlier| OperatorOutlier {
                combo_label: format!("draft:{}", outlier.combo_label),
                ..outlier
            }),
    );
    outliers.sort_by(|left, right| {
        right
            .distance_to_band
            .total_cmp(&left.distance_to_band)
            .then_with(|| left.combo_label.cmp(&right.combo_label))
    });
    outliers
}

fn evaluate_all_modes_candidate(
    stage: &str,
    candidate_index: usize,
    generation: usize,
    candidate: &Candidate,
    classic_config: &MatrixConfig,
    operator_config: &OperatorMatrixConfig,
    draft_config: &DraftMatrixConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
) -> Result<EvaluatedCandidate> {
    let signature = candidate.signature(knobs);
    let changed_knobs = candidate.changed_knobs(knobs);
    let candidate_patch = build_patch(candidate, knobs, catalog)?;
    let patch = merge_balance_patches(base_patch, &candidate_patch);
    patch.validate()?;
    let artifact_dir = prepare_candidate_artifact_dir(
        run_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        changed_knobs.clone(),
        &patch,
        &candidate_patch,
        base_patch,
    )?;

    let classic_report = run_matrix_with_patch(classic_config, Some(&patch));
    write_candidate_artifact_status(
        &artifact_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        &changed_knobs,
        "classic_complete",
        &["classic"],
    )?;
    let operator_report = run_operator_matrix_with_patch(operator_config, Some(&patch));
    write_candidate_artifact_status(
        &artifact_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        &changed_knobs,
        "operator_complete",
        &["classic", "operator"],
    )?;
    let draft_report = run_draft_matrix_with_patch(draft_config, Some(&patch));

    let score = score_all_modes_reports(&classic_report, &operator_report, &draft_report);
    let outliers = summarize_all_modes_outliers(&classic_report, &operator_report, &draft_report);

    let classic_matches = total_classic_matches(&classic_report);
    let operator_matches = total_operator_matches(&operator_report);
    let draft_matches = draft_report.total_matches;

    write_json(artifact_dir.join("classic_report.json"), &classic_report)?;
    write_json(artifact_dir.join("operator_report.json"), &operator_report)?;
    write_json(artifact_dir.join("draft_report.json"), &draft_report)?;
    write_json(
        artifact_dir.join("report.json"),
        &AllModesArtifactReport {
            classic: classic_report.clone(),
            operator: operator_report.clone(),
            draft: draft_report.clone(),
        },
    )?;
    write_candidate_artifact_status(
        &artifact_dir,
        stage,
        candidate_index,
        generation,
        &signature,
        &changed_knobs,
        "complete",
        &["classic", "operator", "draft"],
    )?;

    let summary = CandidateSummary {
        stage: stage.to_string(),
        candidate_index,
        generation,
        signature,
        changed_knobs,
        score,
        outliers,
        finished_match_rate: weighted_rate(&[
            (classic_matches, classic_report.balance.finished_match_rate),
            (
                operator_matches,
                operator_report.balance.finished_match_rate,
            ),
            (draft_matches, draft_report.balance.finished_match_rate),
        ]),
        decisive_match_rate: weighted_rate(&[
            (classic_matches, classic_report.balance.decisive_match_rate),
            (
                operator_matches,
                operator_report.balance.decisive_match_rate,
            ),
            (draft_matches, draft_report.balance.decisive_match_rate),
        ]),
        unfinished_match_rate: weighted_rate(&[
            (
                classic_matches,
                classic_report.balance.unfinished_match_rate,
            ),
            (
                operator_matches,
                operator_report.balance.unfinished_match_rate,
            ),
            (draft_matches, draft_report.balance.unfinished_match_rate),
        ]),
        termination_counts: merge_termination_counts([
            classic_report.balance.termination_counts.clone(),
            operator_report.balance.termination_counts.clone(),
            draft_report.balance.termination_counts.clone(),
        ]),
        artifact_dir: artifact_dir.display().to_string(),
    };
    write_json(artifact_dir.join("summary.json"), &summary)?;

    Ok(EvaluatedCandidate {
        candidate: candidate.clone(),
        summary,
    })
}

fn run_all_modes_search_stage(
    stage: &str,
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    base_patch: Option<&BalancePatch>,
    matches: usize,
) -> Result<StageSummary> {
    let classic_config = build_classic_matrix_config(config, matches);
    let operator_config = build_all_modes_operator_config(config, matches, catalog);
    let draft_config = build_draft_matrix_config(config, matches);
    let mut stage_counter = 0usize;
    let mut seen_signatures = BTreeSet::new();
    let baseline = Candidate::baseline(knobs);
    let baseline_eval = evaluate_all_modes_candidate(
        stage,
        stage_counter,
        0,
        &baseline,
        &classic_config,
        &operator_config,
        &draft_config,
        catalog,
        knobs,
        run_dir,
        base_patch,
    )?;
    stage_counter += 1;
    eprintln!(
        "Baseline: band_misses={} distance={:.3} balance={:.3}",
        baseline_eval.summary.score.strict_band_misses,
        baseline_eval.summary.score.strict_band_distance_sum,
        baseline_eval.summary.score.balance_score,
    );
    seen_signatures.insert(baseline_eval.summary.signature.clone());
    let mut evaluated = vec![baseline_eval.clone()];
    let mut frontier = vec![baseline_eval];

    for generation in 1..=config.generations {
        let candidate_limit = config.candidates_per_generation.max(config.beam_width);
        let next_candidates =
            gather_generation_candidates(&frontier, knobs, &seen_signatures, candidate_limit);
        if next_candidates.is_empty() {
            break;
        }

        let mut generation_results = Vec::new();
        for candidate in next_candidates {
            let result = evaluate_all_modes_candidate(
                stage,
                stage_counter,
                generation,
                &candidate,
                &classic_config,
                &operator_config,
                &draft_config,
                catalog,
                knobs,
                run_dir,
                base_patch,
            )?;
            stage_counter += 1;
            seen_signatures.insert(result.summary.signature.clone());
            generation_results.push(result);
        }
        if generation_results.is_empty() {
            break;
        }

        generation_results.sort_by(compare_evaluated_candidates);
        frontier = generation_results
            .iter()
            .take(config.beam_width)
            .cloned()
            .collect();
        evaluated.extend(generation_results);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .take(config.beam_width.max(10))
        .map(|candidate| candidate.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("all-modes autotune stage {stage} evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, stage, &best)?;

    Ok(StageSummary {
        stage: stage.to_string(),
        matches_per_pair: matches,
        evaluated_candidates: evaluated.len(),
        leaderboard,
        best,
    })
}

fn run_all_modes_promotion_stage(
    config: &CliConfig,
    catalog: &BTreeMap<String, Card>,
    knobs: &[KnobSpec],
    run_dir: &Path,
    screening: &StageSummary,
    base_patch: Option<&BalancePatch>,
) -> Result<StageSummary> {
    let classic_config = build_classic_matrix_config(config, config.promotion_matches);
    let operator_config = build_matrix_config(config, config.promotion_matches);
    let draft_config = build_draft_matrix_config(config, config.promotion_matches);
    let mut promoted_candidates = Vec::new();
    let mut seen = BTreeSet::new();

    for summary in screening
        .leaderboard
        .iter()
        .take(config.promotion_count.max(1))
    {
        if seen.insert(summary.signature.clone()) {
            promoted_candidates.push(candidate_from_signature(summary, knobs)?);
        }
    }

    let mut evaluated = Vec::new();
    for (index, candidate) in promoted_candidates.iter().enumerate() {
        evaluated.push(evaluate_all_modes_candidate(
            "promotion",
            index,
            0,
            candidate,
            &classic_config,
            &operator_config,
            &draft_config,
            catalog,
            knobs,
            run_dir,
            base_patch,
        )?);
    }

    evaluated.sort_by(compare_evaluated_candidates);
    let leaderboard = evaluated
        .iter()
        .map(|candidate| candidate.summary.clone())
        .collect::<Vec<_>>();
    let best = evaluated
        .first()
        .ok_or_else(|| anyhow!("all-modes promotion stage evaluated no candidates"))?
        .summary
        .clone();

    write_best_artifacts(run_dir, "promotion", &best)?;

    Ok(StageSummary {
        stage: "promotion".to_string(),
        matches_per_pair: config.promotion_matches,
        evaluated_candidates: leaderboard.len(),
        leaderboard,
        best,
    })
}

fn get_or_insert_ability_patch<'a>(
    patch: &'a mut CardPatch,
    card: &Card,
    ability_index: usize,
) -> Result<&'a mut AbilityPatch> {
    if ability_index >= card.abilities.len() {
        return Err(anyhow!(
            "{} missing ability index {}",
            patch.card_id,
            ability_index
        ));
    }
    if let Some(index) = patch
        .ability_patches
        .iter()
        .position(|ability_patch| ability_patch.index == ability_index)
    {
        return Ok(&mut patch.ability_patches[index]);
    }

    patch.ability_patches.push(AbilityPatch {
        index: ability_index,
        mana_cost: None,
        once_per_turn: None,
        effect: None,
    });
    Ok(patch
        .ability_patches
        .last_mut()
        .expect("just pushed ability patch"))
}

fn get_or_insert_trigger_patch<'a>(
    patch: &'a mut CardPatch,
    card: &Card,
    trigger_index: usize,
) -> Result<&'a mut TriggerPatch> {
    if trigger_index >= card.triggers.len() {
        return Err(anyhow!(
            "{} missing trigger index {}",
            patch.card_id,
            trigger_index
        ));
    }
    if let Some(index) = patch
        .trigger_patches
        .iter()
        .position(|trigger_patch| trigger_patch.index == trigger_index)
    {
        return Ok(&mut patch.trigger_patches[index]);
    }

    patch.trigger_patches.push(TriggerPatch {
        index: trigger_index,
        limit: None,
        effect: None,
    });
    Ok(patch
        .trigger_patches
        .last_mut()
        .expect("just pushed trigger patch"))
}

fn primary_mana_field(cost: &ManaCost) -> Option<ManaField> {
    [
        (ManaField::Neon, cost.neon),
        (ManaField::Glitch, cost.glitch),
        (ManaField::Chrome, cost.chrome),
        (ManaField::Slag, cost.slag),
        (ManaField::Bio, cost.bio),
    ]
    .into_iter()
    .find_map(|(field, amount)| (amount > 0).then_some(field))
}

fn primary_operator_mana_field(card_id: &str, cost: &ManaCost) -> Option<ManaField> {
    primary_mana_field(cost).or_else(|| {
        operator_deck_by_id()
            .get(card_id)
            .and_then(|deck_id| mana_field_for_deck(deck_id))
    })
}

fn mana_field_for_deck(deck_id: &str) -> Option<ManaField> {
    match deck_id {
        "neon" => Some(ManaField::Neon),
        "glitch" => Some(ManaField::Glitch),
        "chrome" => Some(ManaField::Chrome),
        "slag" => Some(ManaField::Slag),
        "bio" => Some(ManaField::Bio),
        _ => None,
    }
}

fn mana_cost_field(cost: &ManaCost, field: ManaField) -> i32 {
    match field {
        ManaField::Neon => cost.neon,
        ManaField::Glitch => cost.glitch,
        ManaField::Chrome => cost.chrome,
        ManaField::Slag => cost.slag,
        ManaField::Bio => cost.bio,
        ManaField::Colorless => cost.colorless,
    }
}

fn set_mana_field(cost: &mut ManaCost, field: ManaField, value: i32) {
    match field {
        ManaField::Neon => cost.neon = value,
        ManaField::Glitch => cost.glitch = value,
        ManaField::Chrome => cost.chrome = value,
        ManaField::Slag => cost.slag = value,
        ManaField::Bio => cost.bio = value,
        ManaField::Colorless => cost.colorless = value,
    }
}

fn effect_knob_bounds(field: EffectField, baseline: i32) -> (i32, i32) {
    let min = match field {
        EffectField::TokenToughness => (baseline - 1).max(1),
        EffectField::Damage
        | EffectField::Draw
        | EffectField::LootDraw
        | EffectField::LootDiscard
        | EffectField::Scry
        | EffectField::Counters
        | EffectField::TokenPower => (baseline - 1).max(0),
    };
    (min, baseline + 1)
}

fn effect_path_value(effect: &CardEffect, path: EffectPath) -> Result<i32> {
    let target = effect_at_path(effect, path)?;
    effect_value(target, path.field())
}

fn set_effect_path_value(effect: &mut CardEffect, path: EffectPath, value: i32) -> Result<()> {
    let target = effect_at_path_mut(effect, path)?;
    set_effect_value(target, path.field(), value)
}

fn effect_at_path(effect: &CardEffect, path: EffectPath) -> Result<&CardEffect> {
    match path {
        EffectPath::Direct { .. } => Ok(effect),
        EffectPath::Sequence { effect_index, .. } => match effect {
            CardEffect::Sequence(effects) => effects.get(effect_index).ok_or_else(|| {
                anyhow!("effect index {effect_index} out of bounds for sequence autotune knob")
            }),
            other => Err(anyhow!(
                "expected sequence effect for autotune knob, found {other:?}"
            )),
        },
    }
}

fn effect_at_path_mut(effect: &mut CardEffect, path: EffectPath) -> Result<&mut CardEffect> {
    match path {
        EffectPath::Direct { .. } => Ok(effect),
        EffectPath::Sequence { effect_index, .. } => match effect {
            CardEffect::Sequence(effects) => effects.get_mut(effect_index).ok_or_else(|| {
                anyhow!("effect index {effect_index} out of bounds for sequence autotune knob")
            }),
            other => Err(anyhow!(
                "expected sequence effect for autotune knob, found {other:?}"
            )),
        },
    }
}

fn effect_value(effect: &CardEffect, field: EffectField) -> Result<i32> {
    match (effect, field) {
        (
            CardEffect::DamagePlayer(amount)
            | CardEffect::DamageUnit(amount)
            | CardEffect::DamageTarget(amount)
            | CardEffect::DamageEachOpponentUnit(amount)
            | CardEffect::DamageSplit(amount)
            | CardEffect::Siphon(amount),
            EffectField::Damage,
        ) => Ok(*amount),
        (CardEffect::DrawCards(amount), EffectField::Draw) => Ok(*amount as i32),
        (CardEffect::Loot(draw, _), EffectField::LootDraw) => Ok(*draw as i32),
        (CardEffect::Loot(_, discard), EffectField::LootDiscard) => Ok(*discard as i32),
        (CardEffect::Scry(amount), EffectField::Scry) => Ok(*amount as i32),
        (CardEffect::AddCounters(amount), EffectField::Counters) => Ok(*amount),
        (CardEffect::CreateToken(token), EffectField::TokenPower) => Ok(token.power),
        (CardEffect::CreateToken(token), EffectField::TokenToughness) => Ok(token.toughness),
        _ => Err(anyhow!(
            "effect {:?} does not support autotune field {:?}",
            effect,
            field
        )),
    }
}

fn set_effect_value(effect: &mut CardEffect, field: EffectField, value: i32) -> Result<()> {
    match (effect, field) {
        (
            CardEffect::DamagePlayer(amount)
            | CardEffect::DamageUnit(amount)
            | CardEffect::DamageTarget(amount)
            | CardEffect::DamageEachOpponentUnit(amount)
            | CardEffect::DamageSplit(amount)
            | CardEffect::Siphon(amount),
            EffectField::Damage,
        ) => {
            *amount = value;
            Ok(())
        }
        (CardEffect::DrawCards(amount), EffectField::Draw) => {
            *amount = value.max(0) as usize;
            Ok(())
        }
        (CardEffect::Loot(draw, _), EffectField::LootDraw) => {
            *draw = value.max(0) as usize;
            Ok(())
        }
        (CardEffect::Loot(_, discard), EffectField::LootDiscard) => {
            *discard = value.max(0) as usize;
            Ok(())
        }
        (CardEffect::Scry(amount), EffectField::Scry) => {
            *amount = value.max(0) as usize;
            Ok(())
        }
        (CardEffect::AddCounters(amount), EffectField::Counters) => {
            *amount = value.max(0);
            Ok(())
        }
        (CardEffect::CreateToken(token), EffectField::TokenPower) => {
            token.power = value.max(0);
            Ok(())
        }
        (CardEffect::CreateToken(token), EffectField::TokenToughness) => {
            token.toughness = value.max(1);
            Ok(())
        }
        (other, _) => Err(anyhow!(
            "effect {:?} does not support autotune field {:?}",
            other,
            field
        )),
    }
}

fn score_report(report: &OperatorMatrixReport) -> CandidateScore {
    let abnormal_terminations = report
        .balance
        .termination_counts
        .iter()
        .filter(|termination| termination.reason != TerminationReason::Finished)
        .map(|termination| termination.count)
        .sum();
    let strict_band_misses = report
        .operator_metrics
        .iter()
        .filter(|metric| strict_band_distance(metric.win_rate) > 0.0)
        .count();
    let strict_band_distance_sum = report
        .operator_metrics
        .iter()
        .map(|metric| strict_band_distance(metric.win_rate))
        .sum();
    let intra_color_spread_sum = report
        .color_metrics
        .iter()
        .filter_map(|metric| metric.intra_color_spread)
        .sum();

    CandidateScore {
        abnormal_terminations,
        strict_band_misses,
        strict_band_distance_sum,
        balance_score: report.balance.balance_score,
        intra_color_spread_sum,
    }
}

fn strict_band_distance(win_rate: Option<f64>) -> f64 {
    match win_rate {
        Some(rate) if rate < STRICT_BAND_LOW => STRICT_BAND_LOW - rate,
        Some(rate) if rate > STRICT_BAND_HIGH => rate - STRICT_BAND_HIGH,
        Some(_) => 0.0,
        None => NONE_WIN_RATE_DISTANCE,
    }
}

fn summarize_outliers(report: &OperatorMatrixReport) -> Vec<OperatorOutlier> {
    let mut outliers: Vec<_> = report
        .operator_metrics
        .iter()
        .filter_map(|metric| {
            let distance = strict_band_distance(metric.win_rate);
            if distance == 0.0 {
                None
            } else {
                Some(OperatorOutlier {
                    combo_label: metric.combo_label.clone(),
                    win_rate: metric.win_rate,
                    distance_to_band: distance,
                })
            }
        })
        .collect();

    outliers.sort_by(|left, right| {
        right
            .distance_to_band
            .total_cmp(&left.distance_to_band)
            .then_with(|| left.combo_label.cmp(&right.combo_label))
    });
    outliers
}

fn compare_evaluated_candidates(left: &EvaluatedCandidate, right: &EvaluatedCandidate) -> Ordering {
    compare_scores(&left.summary.score, &right.summary.score)
        .then_with(|| left.summary.signature.cmp(&right.summary.signature))
}

fn compare_scores(left: &CandidateScore, right: &CandidateScore) -> Ordering {
    left.abnormal_terminations
        .cmp(&right.abnormal_terminations)
        .then_with(|| left.strict_band_misses.cmp(&right.strict_band_misses))
        .then_with(|| {
            left.strict_band_distance_sum
                .total_cmp(&right.strict_band_distance_sum)
        })
        .then_with(|| left.balance_score.total_cmp(&right.balance_score))
        .then_with(|| {
            left.intra_color_spread_sum
                .total_cmp(&right.intra_color_spread_sum)
        })
}

fn candidate_from_signature(summary: &CandidateSummary, knobs: &[KnobSpec]) -> Result<Candidate> {
    let mut candidate = Candidate::baseline(knobs);
    if summary.signature == "baseline" {
        return Ok(candidate);
    }

    for part in summary.signature.split('|') {
        let (label, raw_value) = part
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid candidate signature component: {part}"))?;
        let value: i32 = raw_value.parse()?;
        let index = knobs
            .iter()
            .position(|spec| spec.label() == label)
            .ok_or_else(|| anyhow!("unknown knob label in signature: {label}"))?;
        candidate.values[index] = value;
    }

    Ok(candidate)
}

fn create_run_dir(root: &Path, config: &CliConfig) -> Result<PathBuf> {
    fs::create_dir_all(root).with_context(|| format!("create autotune root {}", root.display()))?;
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_secs();
    let base_name = format!(
        "run_{epoch}_seed{}_m{}_g{}_bw{}",
        config.seed, config.matches, config.generations, config.beam_width
    );
    let mut run_dir = root.join(&base_name);
    let mut suffix = 1usize;
    while run_dir.exists() {
        run_dir = root.join(format!("{base_name}_{suffix}"));
        suffix += 1;
    }
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("create run dir {}", run_dir.display()))?;
    Ok(run_dir)
}

fn write_best_artifacts(run_dir: &Path, stage: &str, best: &CandidateSummary) -> Result<()> {
    let best_dir = run_dir.join(format!("best_{stage}"));
    fs::create_dir_all(&best_dir)
        .with_context(|| format!("create best dir {}", best_dir.display()))?;

    let source_dir = PathBuf::from(&best.artifact_dir);
    fs::copy(source_dir.join("patch.json"), best_dir.join("patch.json")).with_context(|| {
        format!(
            "copy best patch from {} to {}",
            source_dir.display(),
            best_dir.display()
        )
    })?;
    fs::copy(source_dir.join("report.json"), best_dir.join("report.json")).with_context(|| {
        format!(
            "copy best report from {} to {}",
            source_dir.display(),
            best_dir.display()
        )
    })?;
    write_json(best_dir.join("summary.json"), best)?;
    Ok(())
}

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    let path = path.as_ref();
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One card shaped for a test, so the tests follow the knob machinery
    /// rather than whatever the card list holds this week.
    fn shaped_catalog(id: &str, shape: impl FnOnce(&mut Card)) -> BTreeMap<String, Card> {
        let mut card = load_full_catalog()
            .into_values()
            .find(|card| !card.triggers.is_empty())
            .expect("a card with a trigger");
        card.id = id.to_string();
        shape(&mut card);
        BTreeMap::from([(id.to_string(), card)])
    }
    use cardgame::sim::report::{
        BalanceMetrics, ColorMetrics, DraftColorMetrics, DraftMatrixReport, OperatorMetrics,
    };

    #[test]
    fn operator_autotune_scores_strict_band_and_abnormal_terminations() {
        let report = OperatorMatrixReport {
            combos: vec![
                "neon:echo_signal".to_string(),
                "chrome:sai_forgemaster".to_string(),
            ],
            matches_per_pair: 6,
            alternate_seats: true,
            include_mirrors: false,
            operator_metrics: vec![
                OperatorMetrics {
                    combo_label: "neon:echo_signal".to_string(),
                    deck_id: "neon".to_string(),
                    operator_id: "echo_signal".to_string(),
                    matches: 6,
                    finished_matches: 6,
                    decisive_matches: 6,
                    drawn_matches: 0,
                    wins: 3,
                    losses: 3,
                    unfinished_matches: 0,
                    win_rate: Some(0.5),
                    diagnostics: None,
                },
                OperatorMetrics {
                    combo_label: "chrome:sai_forgemaster".to_string(),
                    deck_id: "chrome".to_string(),
                    operator_id: "sai_forgemaster".to_string(),
                    matches: 6,
                    finished_matches: 4,
                    decisive_matches: 4,
                    drawn_matches: 0,
                    wins: 1,
                    losses: 3,
                    unfinished_matches: 2,
                    win_rate: Some(0.25),
                    diagnostics: None,
                },
            ],
            color_metrics: vec![
                ColorMetrics {
                    color: "neon".to_string(),
                    matches: 6,
                    finished_matches: 6,
                    decisive_matches: 6,
                    drawn_matches: 0,
                    wins: 3,
                    losses: 3,
                    win_rate: Some(0.5),
                    intra_color_spread: Some(0.08),
                },
                ColorMetrics {
                    color: "chrome".to_string(),
                    matches: 6,
                    finished_matches: 4,
                    decisive_matches: 4,
                    drawn_matches: 0,
                    wins: 1,
                    losses: 3,
                    win_rate: Some(0.25),
                    intra_color_spread: Some(0.04),
                },
            ],
            matchup_metrics: Vec::new(),
            balance: BalanceMetrics {
                balance_score: 0.25,
                pairwise_imbalance: 0.0,
                finished_match_rate: 0.66,
                decisive_match_rate: 0.66,
                draw_match_rate: 0.0,
                unfinished_match_rate: 0.34,
                first_player_win_rate: Some(0.5),
                average_rounds: 10.0,
                average_actions: 40.0,
                termination_counts: vec![
                    TerminationCount {
                        reason: TerminationReason::Finished,
                        count: 4,
                    },
                    TerminationCount {
                        reason: TerminationReason::NoLegalAction,
                        count: 2,
                    },
                ],
            },
        };

        let score = score_report(&report);
        assert_eq!(score.abnormal_terminations, 2);
        assert_eq!(score.strict_band_misses, 1);
        assert!((score.strict_band_distance_sum - 0.20).abs() < f64::EPSILON);
        assert!((score.intra_color_spread_sum - 0.12).abs() < f64::EPSILON);
    }

    #[test]
    fn operator_autotune_draft_scoring_ignores_unsampled_colors() {
        let report = DraftMatrixReport {
            draft_count: 2,
            matches_per_draft: 5,
            total_matches: 10,
            color_metrics: vec![
                DraftColorMetrics {
                    color: "bio".to_string(),
                    total_cards_drafted: 0,
                    wins: 0,
                    losses: 0,
                    win_rate: None,
                },
                DraftColorMetrics {
                    color: "slag".to_string(),
                    total_cards_drafted: 12,
                    wins: 6,
                    losses: 4,
                    win_rate: Some(0.6),
                },
            ],
            balance: BalanceMetrics {
                balance_score: 0.0,
                pairwise_imbalance: 0.0,
                finished_match_rate: 1.0,
                decisive_match_rate: 1.0,
                draw_match_rate: 0.0,
                unfinished_match_rate: 0.0,
                first_player_win_rate: Some(0.5),
                average_rounds: 10.0,
                average_actions: 40.0,
                termination_counts: vec![TerminationCount {
                    reason: TerminationReason::Finished,
                    count: 10,
                }],
            },
        };

        let score = score_draft_report(&report);
        assert_eq!(score.strict_band_misses, 1);
        assert!((score.strict_band_distance_sum - 0.05).abs() < f64::EPSILON);

        let outliers = summarize_draft_outliers(&report);
        assert_eq!(outliers.len(), 1);
        assert_eq!(outliers[0].combo_label, "slag");
        assert_eq!(outliers[0].win_rate, Some(0.6));
    }

    #[test]
    fn operator_autotune_build_patch_merges_card_fields() {
        let catalog = shaped_catalog("test_apex", |card| card.loyalty = Some(3));
        let knobs = vec![
            colorless_mana_knob(&catalog, "test_apex").expect("cost knob"),
            loyalty_knob(&catalog, "test_apex").expect("loyalty knob"),
        ];
        let candidate = Candidate { values: vec![4, 5] };

        let patch = build_patch(&candidate, &knobs, &catalog).expect("build patch");
        assert_eq!(patch.cards.len(), 1);
        let patch = &patch.cards[0];
        assert_eq!(patch.card_id, "test_apex");
        assert_eq!(patch.mana_cost.as_ref().map(|cost| cost.colorless), Some(4));
        assert_eq!(patch.loyalty, Some(5));
    }

    #[test]
    fn operator_autotune_build_patch_merges_direct_ability_effect_fields() {
        let catalog = load_operator_catalog();
        let knobs = vec![
            ability_effect_knob(
                &catalog,
                "sai_forgemaster",
                0,
                EffectPath::Direct {
                    field: EffectField::TokenPower,
                },
            )
            .expect("token power knob"),
            ability_effect_knob(
                &catalog,
                "sai_forgemaster",
                0,
                EffectPath::Direct {
                    field: EffectField::TokenToughness,
                },
            )
            .expect("token toughness knob"),
        ];
        let candidate = Candidate { values: vec![2, 3] };

        let patch = build_patch(&candidate, &knobs, &catalog).expect("build patch");
        assert_eq!(patch.cards.len(), 1);
        let ability_patch = &patch.cards[0].ability_patches[0];
        match ability_patch.effect.as_ref() {
            Some(CardEffect::CreateToken(token)) => {
                assert_eq!(token.power, 2);
                assert_eq!(token.toughness, 3);
            }
            other => panic!("expected token ability patch, found {other:?}"),
        }
    }

    #[test]
    fn operator_autotune_build_patch_merges_sequence_trigger_effect_fields() {
        let catalog = shaped_catalog("test_signal", |card| {
            card.triggers[0].effect = CardEffect::Sequence(vec![CardEffect::Scry(1), CardEffect::DrawCards(1)]);
        });
        let knobs = vec![
            trigger_effect_knob(
                &catalog,
                "test_signal",
                0,
                EffectPath::Sequence {
                    effect_index: 0,
                    field: EffectField::Scry,
                },
            )
            .expect("scry knob"),
            trigger_effect_knob(
                &catalog,
                "test_signal",
                0,
                EffectPath::Sequence {
                    effect_index: 1,
                    field: EffectField::Draw,
                },
            )
            .expect("draw knob"),
        ];
        let candidate = Candidate { values: vec![2, 0] };

        let patch = build_patch(&candidate, &knobs, &catalog).expect("build patch");
        assert_eq!(patch.cards.len(), 1);
        let trigger_patch = &patch.cards[0].trigger_patches[0];
        match trigger_patch.effect.as_ref() {
            Some(CardEffect::Sequence(effects)) => {
                assert_eq!(effects[0], CardEffect::Scry(2));
                assert_eq!(effects[1], CardEffect::DrawCards(0));
            }
            other => panic!("expected sequence trigger patch, found {other:?}"),
        }
    }

    #[test]
    fn operator_autotune_build_patch_merges_loot_trigger_effect_fields() {
        let catalog = load_full_catalog();
        let knobs = vec![
            trigger_effect_knob(
                &catalog,
                "archived_alive",
                0,
                EffectPath::Direct {
                    field: EffectField::LootDraw,
                },
            )
            .expect("loot draw knob"),
            trigger_effect_knob(
                &catalog,
                "archived_alive",
                0,
                EffectPath::Direct {
                    field: EffectField::LootDiscard,
                },
            )
            .expect("loot discard knob"),
        ];
        let candidate = Candidate { values: vec![4, 2] };

        let patch = build_patch(&candidate, &knobs, &catalog).expect("build patch");
        assert_eq!(patch.cards.len(), 1);
        let trigger_patch = &patch.cards[0].trigger_patches[0];
        match trigger_patch.effect.as_ref() {
            Some(CardEffect::Loot(draw, discard)) => {
                assert_eq!((*draw, *discard), (4, 2));
            }
            other => panic!("expected loot trigger patch, found {other:?}"),
        }
    }

    #[test]
    fn operator_autotune_build_patch_merges_counter_trigger_effect_fields() {
        let catalog = load_full_catalog();
        let knobs = vec![
            trigger_effect_knob(
                &catalog,
                "mycelium_weaver",
                0,
                EffectPath::Direct {
                    field: EffectField::Counters,
                },
            )
            .expect("counter knob"),
        ];
        let candidate = Candidate { values: vec![2] };

        let patch = build_patch(&candidate, &knobs, &catalog).expect("build patch");
        assert_eq!(patch.cards.len(), 1);
        let trigger_patch = &patch.cards[0].trigger_patches[0];
        match trigger_patch.effect.as_ref() {
            Some(CardEffect::AddCounters(amount)) => {
                assert_eq!(*amount, 2);
            }
            other => panic!("expected counter trigger patch, found {other:?}"),
        }
    }

    #[test]
    fn operator_autotune_candidate_signature_round_trips() {
        let catalog = load_operator_catalog();
        let knobs = vec![
            colorless_mana_knob(&catalog, "echo_signal").expect("cost knob"),
            toughness_knob(&catalog, "echo_signal").expect("toughness knob"),
        ];
        let candidate = Candidate { values: vec![2, 4] };
        let summary = CandidateSummary {
            stage: "screening".to_string(),
            candidate_index: 1,
            generation: 1,
            signature: candidate.signature(&knobs),
            changed_knobs: candidate.changed_knobs(&knobs),
            score: CandidateScore {
                abnormal_terminations: 0,
                strict_band_misses: 0,
                strict_band_distance_sum: 0.0,
                balance_score: 0.0,
                intra_color_spread_sum: 0.0,
            },
            outliers: Vec::new(),
            finished_match_rate: 1.0,
            decisive_match_rate: 1.0,
            unfinished_match_rate: 0.0,
            termination_counts: Vec::new(),
            artifact_dir: "/tmp".to_string(),
        };

        let rebuilt = candidate_from_signature(&summary, &knobs).expect("round trip signature");
        assert_eq!(candidate, rebuilt);
    }

    #[test]
    fn operator_autotune_filters_knobs_to_requested_operator_subset() {
        let catalog = load_operator_catalog();
        let knobs = build_knob_specs(&catalog).expect("knob specs");
        let wanted = knobs.first().expect("at least one operator knob").card_id.clone();
        let filtered = filter_knob_specs(knobs, &[wanted.clone()]);

        assert!(!filtered.is_empty());
        assert!(filtered.iter().all(|knob| knob.card_id == wanted));
    }

    #[test]
    fn operator_autotune_filters_knobs_to_requested_card_subset() {
        let catalog = load_full_catalog();
        let knobs = build_card_knob_specs(&catalog);
        let filtered = filter_card_knob_specs(
            knobs,
            &[String::from("echo_signal"), String::from("adaptive_polyp")],
        );

        assert!(!filtered.is_empty());
        assert!(
            filtered
                .iter()
                .all(|knob| { knob.card_id == "echo_signal" || knob.card_id == "adaptive_polyp" })
        );
    }

    #[test]
    fn operator_autotune_adds_payload_knobs_for_shortlisted_support_cards() {
        let catalog = shaped_catalog("test_support", |card| {
            card.triggers[0].effect = CardEffect::Sequence(vec![CardEffect::DrawCards(1)]);
        });
        let knobs = build_card_mode_knob_specs(&catalog, &[String::from("test_support")]).expect("card knobs");
        let labels = knobs.iter().map(KnobSpec::label).collect::<BTreeSet<_>>();

        assert!(labels.contains("test_support.trigger_0.effect_0.draw"));
    }

    #[test]
    fn operator_autotune_inferrs_operator_scope_from_shortlisted_cards() {
        let catalog = load_full_catalog();
        let inferred = infer_operator_filter_from_cards(
            &catalog,
            &[
                String::from("mycelium_weaver"),
                String::from("deep_grove_hermit"),
                String::from("primeval_colossus"),
            ],
        );

        assert_eq!(inferred, vec![String::from("bio")]);
    }

    #[test]
    fn operator_autotune_preserves_explicit_operator_scope_in_all_modes() {
        let catalog = load_full_catalog();
        let config = CliConfig {
            operator_filter: vec![String::from("echo_signal")],
            ..CliConfig::default()
        };
        let matrix = build_all_modes_operator_config(&config, 1, &catalog);

        assert_eq!(matrix.operator_filter, vec![String::from("echo_signal")]);
    }

    #[test]
    fn operator_autotune_diversifies_first_generation_across_cards() {
        let knobs = vec![
            KnobSpec {
                card_id: "alpha".to_string(),
                field: KnobField::ManaCost(ManaField::Colorless),
                baseline: 2,
                min: 1,
                max: 3,
            },
            KnobSpec {
                card_id: "alpha".to_string(),
                field: KnobField::TriggerEffect {
                    trigger_index: 0,
                    path: EffectPath::Direct {
                        field: EffectField::Draw,
                    },
                },
                baseline: 2,
                min: 1,
                max: 3,
            },
            KnobSpec {
                card_id: "beta".to_string(),
                field: KnobField::Power,
                baseline: 1,
                min: 0,
                max: 2,
            },
        ];
        let baseline = Candidate::baseline(&knobs);
        let frontier = vec![EvaluatedCandidate {
            candidate: baseline,
            summary: CandidateSummary {
                stage: "screening".to_string(),
                candidate_index: 0,
                generation: 0,
                signature: "baseline".to_string(),
                changed_knobs: Vec::new(),
                score: CandidateScore {
                    abnormal_terminations: 0,
                    strict_band_misses: 0,
                    strict_band_distance_sum: 0.0,
                    balance_score: 0.0,
                    intra_color_spread_sum: 0.0,
                },
                outliers: Vec::new(),
                finished_match_rate: 1.0,
                decisive_match_rate: 1.0,
                unfinished_match_rate: 0.0,
                termination_counts: Vec::new(),
                artifact_dir: "/tmp".to_string(),
            },
        }];

        let candidates = gather_generation_candidates(&frontier, &knobs, &BTreeSet::new(), 4);
        let signatures = candidates
            .iter()
            .map(|candidate| candidate.signature(&knobs))
            .collect::<Vec<_>>();

        assert_eq!(
            signatures,
            vec![
                "alpha.mana_cost.colorless=1",
                "beta.power=0",
                "alpha.trigger_0.draw=1",
                "beta.power=2",
            ]
        );
    }

    #[test]
    fn operator_autotune_diversifies_first_generation_across_knobs_within_card() {
        let knobs = vec![
            KnobSpec {
                card_id: "alpha".to_string(),
                field: KnobField::ManaCost(ManaField::Colorless),
                baseline: 2,
                min: 1,
                max: 3,
            },
            KnobSpec {
                card_id: "alpha".to_string(),
                field: KnobField::TriggerEffect {
                    trigger_index: 0,
                    path: EffectPath::Direct {
                        field: EffectField::Draw,
                    },
                },
                baseline: 2,
                min: 1,
                max: 3,
            },
        ];
        let baseline = Candidate::baseline(&knobs);
        let frontier = vec![EvaluatedCandidate {
            candidate: baseline,
            summary: CandidateSummary {
                stage: "screening".to_string(),
                candidate_index: 0,
                generation: 0,
                signature: "baseline".to_string(),
                changed_knobs: Vec::new(),
                score: CandidateScore {
                    abnormal_terminations: 0,
                    strict_band_misses: 0,
                    strict_band_distance_sum: 0.0,
                    balance_score: 0.0,
                    intra_color_spread_sum: 0.0,
                },
                outliers: Vec::new(),
                finished_match_rate: 1.0,
                decisive_match_rate: 1.0,
                unfinished_match_rate: 0.0,
                termination_counts: Vec::new(),
                artifact_dir: "/tmp".to_string(),
            },
        }];

        let candidates = gather_generation_candidates(&frontier, &knobs, &BTreeSet::new(), 4);
        let signatures = candidates
            .iter()
            .map(|candidate| candidate.signature(&knobs))
            .collect::<Vec<_>>();

        assert_eq!(
            signatures,
            vec![
                "alpha.mana_cost.colorless=1",
                "alpha.trigger_0.draw=1",
                "alpha.mana_cost.colorless=3",
                "alpha.trigger_0.draw=3",
            ]
        );
    }

    #[test]
    fn operator_autotune_preserves_knob_order_in_first_generation_neighbors() {
        let specs = vec![
            KnobSpec {
                card_id: "alpha".to_string(),
                field: KnobField::ManaCost(ManaField::Colorless),
                baseline: 2,
                min: 1,
                max: 3,
            },
            KnobSpec {
                card_id: "alpha".to_string(),
                field: KnobField::TriggerEffect {
                    trigger_index: 0,
                    path: EffectPath::Direct {
                        field: EffectField::Draw,
                    },
                },
                baseline: 2,
                min: 1,
                max: 3,
            },
            KnobSpec {
                card_id: "beta".to_string(),
                field: KnobField::Power,
                baseline: 1,
                min: 0,
                max: 2,
            },
        ];
        let baseline = Candidate::baseline(&specs);
        let neighbors = baseline.neighbors(&specs);
        let signatures = neighbors
            .iter()
            .map(|candidate| candidate.signature(&specs))
            .collect::<Vec<_>>();

        assert_eq!(
            signatures,
            vec![
                "alpha.mana_cost.colorless=1",
                "alpha.mana_cost.colorless=3",
                "alpha.trigger_0.draw=1",
                "alpha.trigger_0.draw=3",
                "beta.power=0",
                "beta.power=2",
            ]
        );
    }

    #[test]
    fn operator_autotune_merges_base_patch_with_candidate_patch() {
        let catalog = load_operator_catalog();
        let echo = catalog.get("echo_signal").expect("echo");
        let mut updated_cost = echo.cost.clone();
        updated_cost.colorless = 2;

        let base_patch = BalancePatch {
            cards: vec![CardPatch {
                card_id: "echo_signal".to_string(),
                mana_cost: None,
                power: None,
                toughness: Some(6),
                loyalty: None,
                keywords: None,
                add_keywords: Vec::new(),
                remove_keywords: Vec::new(),
                trigger_patches: Vec::new(),
                ability_patches: Vec::new(),
            }],
            deck_entries: Vec::new(),
        };
        let candidate_patch = BalancePatch {
            cards: vec![CardPatch {
                card_id: "echo_signal".to_string(),
                mana_cost: Some(updated_cost),
                power: None,
                toughness: None,
                loyalty: None,
                keywords: None,
                add_keywords: Vec::new(),
                remove_keywords: Vec::new(),
                trigger_patches: Vec::new(),
                ability_patches: Vec::new(),
            }],
            deck_entries: Vec::new(),
        };

        let merged = merge_balance_patches(Some(&base_patch), &candidate_patch);
        assert_eq!(merged.cards.len(), 1);
        let patch = &merged.cards[0];
        assert_eq!(patch.card_id, "echo_signal");
        assert_eq!(patch.toughness, Some(6));
        assert_eq!(patch.mana_cost.as_ref().map(|cost| cost.colorless), Some(2));
    }

    #[test]
    fn operator_autotune_applies_base_patch_before_building_knobs() {
        let mut catalog = load_operator_catalog();
        let echo = catalog.get("echo_signal").expect("echo");
        let mut tuned_cost = echo.cost.clone();
        tuned_cost.neon = 0;
        let base_patch = BalancePatch {
            cards: vec![CardPatch {
                card_id: "echo_signal".to_string(),
                mana_cost: Some(tuned_cost),
                power: None,
                toughness: Some(6),
                loyalty: None,
                keywords: None,
                add_keywords: Vec::new(),
                remove_keywords: Vec::new(),
                trigger_patches: Vec::new(),
                ability_patches: Vec::new(),
            }],
            deck_entries: Vec::new(),
        };

        apply_patch_to_catalog(&mut catalog, &base_patch).expect("apply patch");
        let knob = toughness_knob(&catalog, "echo_signal").expect("toughness knob");
        assert_eq!(knob.baseline, 6);
        let color_knob =
            primary_color_mana_knob(&catalog, "echo_signal").expect("primary color knob");
        assert_eq!(color_knob.baseline, 0);
        assert!(matches!(
            color_knob.field,
            KnobField::ManaCost(ManaField::Neon)
        ));
    }
}
