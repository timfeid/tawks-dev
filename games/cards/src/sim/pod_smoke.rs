//! Smoke test: attempt to run a 4-player operator pod match.
//! Not called by production code; invoked via a bin or inline test.

#[cfg(test)]
mod tests {
    use crate::sim::agent::AgentProfile;
    use crate::sim::config::{MatchConfig, ParticipantConfig};
    use crate::sim::runner::run_match;
    use crate::types::GameFormat;

    fn four_player_config(seed: u64) -> MatchConfig {
        MatchConfig {
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
                    "bio",
                    AgentProfile::Bio,
                    Some("verdant_warlord".to_string()),
                    false,
                ),
                ParticipantConfig::new(
                    "p3",
                    "chrome",
                    AgentProfile::Chrome,
                    Some("bastion_prime".to_string()),
                    false,
                ),
                ParticipantConfig::new(
                    "p4",
                    "slag",
                    AgentProfile::Slag,
                    Some("pyra_torch_of_defiance".to_string()),
                    false,
                ),
            ],
            patch_path: None,
            mcts_iterations: 48,
            mcts_rollout_depth: 10,
            seed,
            format: GameFormat::Operator,
            max_turns: 40,
            max_actions: 800,
            repeat_state_limit: 6,
            include_trace: false,
            enable_sideboard: false,
        }
    }

    #[test]
    fn four_player_operator_pod_runs_to_completion() {
        let report = run_match(&four_player_config(1));
        eprintln!(
            "seed=1: term={:?}, rounds={}, actions={}, winner={:?}",
            report.termination_reason, report.rounds, report.actions_taken, report.winner_player_id
        );
        assert!(report.error.is_none(), "match errored: {:?}", report.error);
        assert!(report.rounds > 0);
    }

    /// Focused resampling: only run pods that include operators in CARDGAME_POD_TARGETS
    /// (comma-separated list of operator_ids). Much faster than full matrix when iterating
    /// on a specific operator — ~90 pods per target vs 800.
    ///
    /// Usage:
    ///   CARDGAME_POD_TARGETS=karn_chrome,entropy_dreadhorde_general \
    ///     cargo test -p cardgame --release --lib \
    ///     sim::pod_smoke::tests::pod_focused_measure -- --nocapture
    #[test]
    fn pod_focused_measure() {
        let targets_env = match std::env::var("CARDGAME_POD_TARGETS") {
            Ok(v) => v,
            Err(_) => {
                eprintln!("skip: set CARDGAME_POD_TARGETS=op1,op2,... to run focused measurement");
                return;
            }
        };
        let targets: Vec<String> = targets_env
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        eprintln!("focused targets: {targets:?}");

        use crate::deck::operator_ids_for_color;
        use std::collections::HashMap;

        let colors = ["neon", "glitch", "chrome", "slag", "bio"];
        let mut pool: Vec<(String, String)> = Vec::new();
        for color in &colors {
            for op_id in operator_ids_for_color(color) {
                pool.push((color.to_string(), op_id.to_string()));
            }
        }

        let agent_for = |color: &str| match color {
            "neon" => AgentProfile::Neon,
            "glitch" => AgentProfile::Glitch,
            "chrome" => AgentProfile::Chrome,
            "slag" => AgentProfile::Slag,
            "bio" => AgentProfile::Bio,
            _ => AgentProfile::Generalist,
        };

        fn splitmix(mut x: u64) -> u64 {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^ (x >> 31)
        }

        let num_pods: usize = std::env::var("CARDGAME_POD_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(800);

        let mut appearances: HashMap<String, u32> = HashMap::new();
        let mut wins: HashMap<String, u32> = HashMap::new();
        let mut pods_run = 0u32;

        for pod_idx in 0..num_pods {
            let mut rng = splitmix(42 + pod_idx as u64);
            let mut selected: Vec<usize> = Vec::with_capacity(4);
            while selected.len() < 4 {
                rng = splitmix(rng);
                let candidate = (rng as usize) % pool.len();
                if !selected.contains(&candidate) {
                    selected.push(candidate);
                }
            }

            // Only run pods that include at least one target operator.
            let pod_ops: Vec<&str> = selected.iter().map(|&idx| pool[idx].1.as_str()).collect();
            let contains_target = targets.iter().any(|t| pod_ops.iter().any(|op| op == t));
            if !contains_target {
                continue;
            }

            pods_run += 1;
            let participants: Vec<ParticipantConfig> = selected
                .iter()
                .enumerate()
                .map(|(seat, &idx)| {
                    let (deck, op) = &pool[idx];
                    ParticipantConfig::new(
                        format!("p{}", seat + 1),
                        deck.as_str(),
                        agent_for(deck),
                        Some(op.clone()),
                        true,
                    )
                })
                .collect();

            for (_seat, &idx) in selected.iter().enumerate() {
                let (_, op) = &pool[idx];
                *appearances.entry(op.clone()).or_insert(0) += 1;
            }

            let config = MatchConfig {
                participants,
                patch_path: None,
                mcts_iterations: 24,
                mcts_rollout_depth: 6,
                seed: pod_idx as u64 + 1,
                format: GameFormat::Operator,
                max_turns: 40,
                max_actions: 1200,
                repeat_state_limit: 6,
                include_trace: false,
                enable_sideboard: false,
            };

            let report = run_match(&config);
            if let Some(winner_id) = report.winner_player_id {
                let seat_idx: usize = winner_id
                    .strip_prefix('p')
                    .and_then(|s| s.parse::<usize>().ok())
                    .map(|n| n - 1)
                    .unwrap_or(0);
                if seat_idx < selected.len() {
                    let (_, op) = &pool[selected[seat_idx]];
                    *wins.entry(op.clone()).or_insert(0) += 1;
                }
            }
        }

        eprintln!(
            "\n=== Focused result ({} pods run of {} scanned) ===",
            pods_run, num_pods
        );
        for target in &targets {
            let app = appearances.get(target).copied().unwrap_or(0);
            let w = wins.get(target).copied().unwrap_or(0);
            let rate = if app > 0 {
                w as f64 / app as f64 * 100.0
            } else {
                0.0
            };
            eprintln!("  {target:<34} {rate:>5.1}%  ({w}/{app})");
        }
        eprintln!("\n=== All operators in sampled pods (appearance-ordered) ===");
        let mut rows: Vec<(String, u32, u32, f64)> = appearances
            .iter()
            .map(|(op, app)| {
                let w = wins.get(op).copied().unwrap_or(0);
                let rate = if *app > 0 {
                    w as f64 / *app as f64
                } else {
                    0.0
                };
                (op.clone(), *app, w, rate)
            })
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        for (op, app, w, rate) in &rows {
            let marker = if targets.contains(op) { " <--" } else { "" };
            eprintln!(
                "  {op:<34} {rate:>5.1}%  ({w}/{app}){marker}",
                rate = rate * 100.0
            );
        }
    }

    /// Sample random 4-operator pods and measure each operator's 1st-place rate.
    /// Only runs when CARDGAME_POD_MATRIX=1 (expensive; disabled by default).
    #[test]
    fn pod_matrix_1st_place_spread() {
        if std::env::var("CARDGAME_POD_MATRIX").ok().as_deref() != Some("1") {
            eprintln!("skip: set CARDGAME_POD_MATRIX=1 to run pod matrix sampler");
            return;
        }

        use crate::deck::operator_ids_for_color;
        use std::collections::HashMap;

        // Build (deck_id, operator_id) pool across all colors.
        let colors = ["neon", "glitch", "chrome", "slag", "bio"];
        let mut pool: Vec<(String, String)> = Vec::new();
        for color in &colors {
            for op_id in operator_ids_for_color(color) {
                pool.push((color.to_string(), op_id.to_string()));
            }
        }
        eprintln!("operator pool size: {}", pool.len());

        let agent_for = |color: &str| match color {
            "neon" => AgentProfile::Neon,
            "glitch" => AgentProfile::Glitch,
            "chrome" => AgentProfile::Chrome,
            "slag" => AgentProfile::Slag,
            "bio" => AgentProfile::Bio,
            _ => AgentProfile::Generalist,
        };

        let num_pods: usize = std::env::var("CARDGAME_POD_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        // Simple PRNG: mix seed with pod index for reproducibility.
        fn splitmix(mut x: u64) -> u64 {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^ (x >> 31)
        }

        let mut pod_selections: Vec<Vec<usize>> = Vec::with_capacity(num_pods);
        for pod_idx in 0..num_pods {
            let mut rng = splitmix(42 + pod_idx as u64);
            let mut selected: Vec<usize> = Vec::with_capacity(4);
            while selected.len() < 4 {
                rng = splitmix(rng);
                let candidate = (rng as usize) % pool.len();
                if !selected.contains(&candidate) {
                    selected.push(candidate);
                }
            }
            pod_selections.push(selected);
        }

        use rayon::prelude::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let progress = AtomicUsize::new(0);

        let results: Vec<(Vec<usize>, Option<usize>)> = pod_selections
            .par_iter()
            .enumerate()
            .map(|(pod_idx, selected)| {
                let participants: Vec<ParticipantConfig> = selected
                    .iter()
                    .enumerate()
                    .map(|(seat, &idx)| {
                        let (deck, op) = &pool[idx];
                        ParticipantConfig::new(
                            format!("p{}", seat + 1),
                            deck.as_str(),
                            agent_for(deck),
                            Some(op.clone()),
                            true,
                        )
                    })
                    .collect();

                let config = MatchConfig {
                    participants,
                    patch_path: None,
                    mcts_iterations: 24,
                    mcts_rollout_depth: 10,
                    seed: pod_idx as u64 + 1,
                    format: GameFormat::Operator,
                    max_turns: 30,
                    max_actions: 800,
                    repeat_state_limit: 6,
                    include_trace: false,
                    enable_sideboard: false,
                };

                let report = run_match(&config);
                let winner_seat = report.winner_player_id.and_then(|wid| {
                    wid.strip_prefix('p')
                        .and_then(|s| s.parse::<usize>().ok())
                        .map(|n| n - 1)
                });

                let done = progress.fetch_add(1, Ordering::Relaxed) + 1;
                if done % 10 == 0 {
                    eprintln!("  pod {}/{}", done, num_pods);
                }

                (selected.clone(), winner_seat)
            })
            .collect();

        let mut appearances: HashMap<String, u32> = HashMap::new();
        let mut wins: HashMap<String, u32> = HashMap::new();
        for (selected, winner_seat) in &results {
            for &idx in selected {
                let (_, op) = &pool[idx];
                *appearances.entry(op.clone()).or_insert(0) += 1;
            }
            if let Some(seat_idx) = winner_seat {
                if *seat_idx < selected.len() {
                    let (_, op) = &pool[selected[*seat_idx]];
                    *wins.entry(op.clone()).or_insert(0) += 1;
                }
            }
        }

        // Report.
        let mut rows: Vec<(String, u32, u32, f64)> = appearances
            .iter()
            .map(|(op, app)| {
                let w = wins.get(op).copied().unwrap_or(0);
                let rate = if *app > 0 {
                    w as f64 / *app as f64
                } else {
                    0.0
                };
                (op.clone(), *app, w, rate)
            })
            .collect();
        rows.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());
        eprintln!("\n=== Pod 1st-place rate (sorted ascending) ===");
        for (op, app, w, rate) in &rows {
            eprintln!("  {op:<34} {rate:>5.1}%  ({w}/{app})", rate = rate * 100.0);
        }
        let min_rate = rows.first().map(|r| r.3).unwrap_or(0.0);
        let max_rate = rows.last().map(|r| r.3).unwrap_or(0.0);
        eprintln!(
            "\nSpread: {:.1}pp  (min {:.1}%, max {:.1}%)",
            (max_rate - min_rate) * 100.0,
            min_rate * 100.0,
            max_rate * 100.0
        );
        eprintln!("Expected baseline (25% win rate in 4-player): operators within 15-35% range");
    }

    #[test]
    fn four_player_operator_pod_multiple_seeds() {
        let mut winners = std::collections::HashMap::new();
        for seed in 1..=20u64 {
            let report = run_match(&four_player_config(seed));
            if let Some(err) = &report.error {
                eprintln!("seed={seed} ERRORED: {err}");
                panic!("seed {seed} errored");
            }
            let winner = report
                .winner_player_id
                .clone()
                .unwrap_or_else(|| "(none)".to_string());
            *winners.entry(winner.clone()).or_insert(0) += 1;
            eprintln!(
                "seed={seed}: term={:?}, rounds={}, actions={}, winner={}",
                report.termination_reason, report.rounds, report.actions_taken, winner
            );
        }
        eprintln!("Winner distribution over 20 seeds: {winners:?}");
    }

    /// 4-player auto-draft sim: builds a pool from all color catalogs,
    /// runs the draft with `choose_auto_draft_pick` for each pick, then
    /// reports each player's drafted deck composition.
    ///
    /// Run with: cargo test -p cardgame --release --lib
    ///   sim::pod_smoke::tests::draft_sim_pod -- --nocapture
    #[test]
    fn draft_sim_pod() {
        use crate::card::Card;
        use crate::card_catalog;
        use crate::handlers::choose_auto_draft_pick;
        use crate::state::DraftState;
        use crate::types::CardSupertype;
        use rand::SeedableRng;
        use rand::seq::SliceRandom;
        use std::collections::HashMap;
        use std::time::Instant;

        let seed: u64 = std::env::var("CARDGAME_DRAFT_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(42);
        let player_count: usize = 4;

        // Build pool: all non-Apex cards from every color catalog.
        // (Apex operators have a separate selection in real play; for a
        // pure draft sim we focus on the regular pool.)
        let mut pool: Vec<Card> = Vec::new();
        let catalogs: Vec<Vec<(Card, i32)>> = vec![
            card_catalog::neon::all_cards(),
            card_catalog::glitch::all_cards(),
            card_catalog::chrome::all_cards(),
            card_catalog::slag::all_cards(),
            card_catalog::bio::all_cards(),
            card_catalog::multicolor::all_cards(),
        ];
        for catalog in &catalogs {
            for (card, copies) in catalog {
                if card.supertype == CardSupertype::Apex {
                    continue;
                }
                for i in 0..*copies as usize {
                    let mut c = card.clone();
                    c.template_id = Some(card.id.clone());
                    c.id = format!("draft-{}-{}", card.id, i);
                    pool.push(c);
                }
            }
        }

        // Shuffle the pool deterministically.
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        pool.shuffle(&mut rng);

        // Operator-style: 5 packs per player, capped at 11 minimum.
        let num_packs = (5 * player_count).max(11);
        let needed = num_packs * crate::state::DRAFT_PACK_SIZE;
        if pool.len() < needed {
            eprintln!(
                "Draft pool too small: have {}, need {} ({} packs × {} cards)",
                pool.len(),
                needed,
                num_packs,
                crate::state::DRAFT_PACK_SIZE
            );
            return;
        }
        pool.truncate(needed);

        let player_ids: Vec<String> = (0..player_count).map(|i| format!("p{}", i + 1)).collect();
        let player_names: HashMap<String, String> = player_ids
            .iter()
            .map(|id| (id.clone(), id.clone()))
            .collect();

        let mut draft = DraftState::new(pool.clone(), &player_ids, player_names, num_packs);
        if !draft.deal_next_pyramid(&player_ids) {
            eprintln!("Failed to deal first pack");
            return;
        }

        let t0 = Instant::now();
        let mut total_picks = 0usize;
        let max_picks = num_packs * crate::state::DRAFT_PACK_SIZE + 10;

        // Drive the draft loop: pick, advance picker, deal next pack as needed.
        while total_picks < max_picks {
            let picker = match draft.current_picker.clone() {
                Some(p) => p,
                None => break,
            };
            let idx = match choose_auto_draft_pick(&draft, &picker) {
                Some(i) => i,
                None => break,
            };
            if draft.pick(&picker, idx).is_none() {
                eprintln!("WARN: pick failed for {picker} idx={idx}");
                break;
            }
            total_picks += 1;

            if draft.is_pyramid_empty() {
                draft.packs_completed += 1;
                draft.first_picker_index += 1;
                if !draft.deal_next_pyramid(&player_ids) {
                    break;
                }
            } else {
                // Advance picker to next player (rotate within pack).
                let cur_idx = player_ids.iter().position(|id| id == &picker).unwrap_or(0);
                let next_idx = (cur_idx + 1) % player_ids.len();
                draft.current_picker = Some(player_ids[next_idx].clone());
            }
        }

        eprintln!(
            "\n=== Draft complete in {:.2}s ({} picks, {} packs) ===",
            t0.elapsed().as_secs_f64(),
            total_picks,
            draft.packs_completed,
        );

        // Report each player's drafted deck.
        for pid in &player_ids {
            let cards = draft.drafted.get(pid).cloned().unwrap_or_default();
            let mut color_counts: HashMap<crate::types::ManaType, usize> = HashMap::new();
            let mut kind_counts: HashMap<&'static str, usize> = HashMap::new();
            for card in &cards {
                if card.cost.neon > 0 {
                    *color_counts
                        .entry(crate::types::ManaType::Neon)
                        .or_insert(0) += 1;
                }
                if card.cost.glitch > 0 {
                    *color_counts
                        .entry(crate::types::ManaType::Glitch)
                        .or_insert(0) += 1;
                }
                if card.cost.chrome > 0 {
                    *color_counts
                        .entry(crate::types::ManaType::Chrome)
                        .or_insert(0) += 1;
                }
                if card.cost.slag > 0 {
                    *color_counts
                        .entry(crate::types::ManaType::Slag)
                        .or_insert(0) += 1;
                }
                if card.cost.bio > 0 {
                    *color_counts.entry(crate::types::ManaType::Bio).or_insert(0) += 1;
                }
                let kind = match card.kind {
                    crate::types::CardKind::Unit(_) => "unit",
                    crate::types::CardKind::Infrastructure => "infra",
                    crate::types::CardKind::Relic => "relic",
                    crate::types::CardKind::Instant => "instant",
                    crate::types::CardKind::Ritual => "ritual",
                    crate::types::CardKind::Aura => "aura",
                    crate::types::CardKind::Agent => "agent",
                };
                *kind_counts.entry(kind).or_insert(0) += 1;
            }
            let mut color_summary: Vec<(crate::types::ManaType, usize)> =
                color_counts.into_iter().collect();
            color_summary.sort_by(|a, b| b.1.cmp(&a.1));
            let mut kind_summary: Vec<(&str, usize)> = kind_counts.into_iter().collect();
            kind_summary.sort_by(|a, b| b.1.cmp(&a.1));
            eprintln!(
                "{}: {} cards | colors {:?} | kinds {:?}",
                pid,
                cards.len(),
                color_summary,
                kind_summary
            );

            // Top 8 most-played template_ids
            let mut tpl_counts: HashMap<String, usize> = HashMap::new();
            for card in &cards {
                let tpl = card.template_id.clone().unwrap_or_else(|| card.id.clone());
                *tpl_counts.entry(tpl).or_insert(0) += 1;
            }
            let mut top: Vec<(String, usize)> = tpl_counts.into_iter().collect();
            top.sort_by(|a, b| b.1.cmp(&a.1));
            let preview: Vec<String> = top
                .iter()
                .take(8)
                .map(|(id, n)| format!("{}×{}", n, id))
                .collect();
            eprintln!("    top picks: {}", preview.join(", "));
        }
    }
}
