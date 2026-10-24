use cardgame::sim::config::MatrixConfig;
use cardgame::sim::runner::run_matrix;
use std::collections::BTreeMap;

fn main() {
    let seeds = [42, 123, 7, 999, 314];
    let matches_per_pair = 20;
    let total_matches_per_pair = matches_per_pair * seeds.len();

    println!(
        "Running classic deck matrix ({} seeds x {} matches = {} per pair)...\n",
        seeds.len(),
        matches_per_pair,
        total_matches_per_pair
    );

    let mut deck_wins: BTreeMap<String, usize> = BTreeMap::new();
    let mut deck_losses: BTreeMap<String, usize> = BTreeMap::new();
    let mut matchup_wins: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new();
    let mut total_balance = 0.0;

    for &seed in &seeds {
        let mut config = MatrixConfig::default();
        config.matches_per_pair = matches_per_pair;
        config.base_match.seed = seed;
        let report = run_matrix(&config);
        total_balance += report.balance.balance_score;

        for deck in &report.deck_metrics {
            *deck_wins.entry(deck.deck_id.clone()).or_default() += deck.wins;
            *deck_losses.entry(deck.deck_id.clone()).or_default() += deck.losses;
        }
        for matchup in &report.matchup_metrics {
            let key = (matchup.deck_a.clone(), matchup.deck_b.clone());
            let entry = matchup_wins.entry(key).or_insert((0, 0));
            for (deck_id, wins) in &matchup.wins_by_deck {
                if deck_id == &matchup.deck_a {
                    entry.0 += wins;
                } else {
                    entry.1 += wins;
                }
            }
        }
    }

    println!("=== DECK WIN RATES (aggregated) ===");
    let mut deck_list: Vec<_> = deck_wins.keys().cloned().collect();
    deck_list.sort();
    for deck_id in &deck_list {
        let w = deck_wins[deck_id];
        let l = deck_losses[deck_id];
        let total = w + l;
        let wr = if total > 0 {
            w as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!("  {:8} — W:{:3} L:{:3} | WR: {:5.1}%", deck_id, w, l, wr);
    }

    println!("\n=== MATCHUP MATRIX (aggregated) ===");
    for ((a, b), (a_wins, b_wins)) in &matchup_wins {
        let total = a_wins + b_wins;
        let wr = if total > 0 {
            *a_wins as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  {:8} vs {:8} — {:4.0}% | {}:{} {}:{}",
            a, b, wr, a, a_wins, b, b_wins
        );
    }

    println!("\n=== AGGREGATE BALANCE ===");
    println!(
        "  Avg balance score: {:.3}",
        total_balance / seeds.len() as f64
    );
}
