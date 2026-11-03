//! Seeded all-bot nights for every Cops & Robbers format at matched skill.
//!
//! cargo test -p party --release --test city_balance -- --ignored --nocapture
//! CITY_BALANCE_NIGHTS sets nights per format (default 40); CITY_BALANCE_SEED
//! starts the seeds elsewhere, for a fresh validation run;
//! CITY_BALANCE_SKILL sets every bot's skill (default steady, 0.4).
//! CITY_BALANCE_TRACE_SEED prints ten-second snapshots for one engine seed;
//! the engine seed is (CITY_BALANCE_SEED + night index) * 7 + 1.
use party::{
    board::{Board, Map},
    bout::TICK_DT,
    city_heist::{
        CheckKind, CityConfig, CityGame,
        formats::FORMATS,
        stats::{JobRecord, Phase},
    },
};

const KINDS: [CheckKind; 8] = [
    CheckKind::Wires,
    CheckKind::Cameras,
    CheckKind::Dial,
    CheckKind::Stuff,
    CheckKind::Pressure,
    CheckKind::Lockpick,
    CheckKind::Steady,
    CheckKind::LugNuts,
];

struct Night {
    score: [u16; 2],
    jobs: Vec<JobRecord>,
    passes: [u32; 8],
    fails: [u32; 8],
    slips: [u32; 2],
}

fn play(police: u8, crew: u8, seed: u64, skill: f32) -> Night {
    let players = police + crew;
    let seats: Vec<_> = (0..players).map(|s| (s, true)).collect();
    let ids: Vec<_> = (0..players).collect();
    let config = CityConfig {
        police,
        jobs: 1,
        skill: Some(skill),
        ..Default::default()
    };
    let mut game = CityGame::seeded(&seats, config, true, seed);
    // CITY_TUNE_1v2="power,difficulty,police speed,jail,ping[,dial,bags,carry,inspect,delay]" tries a
    // format's numbers without a rebuild.
    if let Ok(tune) = std::env::var(format!("CITY_TUNE_{police}v{crew}")) {
        let v: Vec<f32> = tune.split(',').filter_map(|n| n.parse().ok()).collect();
        if let [power, difficulty, speed, jail, ping, ..] = v[..] {
            let t = &mut game.tuning;
            (t.power_seconds, t.check_difficulty, t.police_speed) = (power, difficulty, speed);
            (t.jail_seconds, t.ping_cooldown) = (jail, ping);
            if let Some(dial) = v.get(5) {
                t.dial_numbers = *dial as u8;
            }
            if let Some(bags) = v.get(6) {
                t.bags = *bags as usize;
            }
            if let Some(carry) = v.get(7) {
                t.carry_speed = *carry;
            }
            if let Some(inspect) = v.get(8) {
                t.inspect_period = *inspect;
            }
            if let Some(delay) = v.get(9) {
                t.inspect_delay = *delay;
            }
        }
    }
    let mut board = Board::for_room(Map::Frozen, &ids, 10, seed);
    board.use_manual_events();
    let trace = std::env::var("CITY_BALANCE_TRACE_SEED")
        .ok()
        .and_then(|n| n.parse::<u64>().ok())
        .is_some_and(|n| n == seed);
    let mut trace_interval = 0;
    while game.resolved.is_none() && game.time < 400. {
        let world = TICK_DT * game.time_scale();
        board.traffic_tick_city(
            world,
            &game.patrol_traffic_obstacles(),
            &game.patrol_sirens(),
        );
        game.tick_streets(TICK_DT, &[], &ids, &mut board);
        if trace && game.time as u32 / 10 > trace_interval {
            trace_interval = game.time as u32 / 10;
            let v = game.view_for(Some(police));
            println!(
                "trace {police}v{crew} seed {seed} time {:.0} job {:?} score {:?} power {:.0?} cameras {:.0?} vault {:.2?} checks {:?}/{:?}",
                game.time,
                v.job,
                v.night_score,
                v.power_seconds,
                v.cameras_seconds,
                v.vault_progress,
                game.check_passes,
                game.check_fails,
            );
            for a in v.actors {
                let check = game.view_for(Some(a.seat)).check.and_then(|c| c.kind);
                let stuck = game.view_for(Some(a.seat)).car_stuck;
                println!(
                    "  seat {} {:?} pos ({:.1?},{:.1?}) jailed {} carrying {} acting {} driving {} stuck {} velocity ({:.1},{:.1}) check {:?}",
                    a.seat, a.side, a.x, a.z, a.jailed, a.carrying, a.acting, a.driving, stuck,
                    a.velocity[0], a.velocity[1], check,
                );
            }
        }
    }
    Night {
        score: game.view_for(None).night_score,
        jobs: game.jobs.clone(),
        passes: game.check_passes,
        fails: game.check_fails,
        slips: game.slips,
    }
}

#[derive(Default)]
struct Summary {
    nights: u32,
    crew: u32,
    police: u32,
    draws: u32,
    score_counts: std::collections::BTreeMap<[u16; 2], u32>,
    zero_zero_phases: [u32; 4],
    lengths: Vec<f32>,
    jobs: u32,
    delivered: u32,
    /// Jobs police won, or that ran out the night, by the furthest phase.
    stopped: [u32; 3],
    wipes: u32,
    arrests: u32,
    passes: [u32; 8],
    fails: [u32; 8],
    slips: [u32; 2],
}

impl Summary {
    fn add(&mut self, night: Night) {
        self.nights += 1;
        *self.score_counts.entry(night.score).or_default() += 1;
        if night.score == [0, 0] {
            for job in &night.jobs {
                self.zero_zero_phases[match job.phase {
                    Phase::Power => 0,
                    Phase::Vault => 1,
                    Phase::Carry => 2,
                    Phase::Delivered => 3,
                }] += 1;
            }
        }
        match night.score[1].cmp(&night.score[0]) {
            std::cmp::Ordering::Greater => self.crew += 1,
            std::cmp::Ordering::Less => self.police += 1,
            std::cmp::Ordering::Equal => self.draws += 1,
        }
        for job in &night.jobs {
            self.jobs += 1;
            self.arrests += job.arrests;
            if let Some(end) = job.ended {
                self.lengths.push(end - job.started);
            }
            if job.wiped {
                self.wipes += 1;
            }
            if job.bags_delivered > 0 {
                self.delivered += 1;
            } else {
                let at = match job.phase {
                    Phase::Power => 0,
                    Phase::Vault => 1,
                    Phase::Carry | Phase::Delivered => 2,
                };
                self.stopped[at] += 1;
            }
        }
        self.slips[0] += night.slips[0];
        self.slips[1] += night.slips[1];
        for i in 0..8 {
            self.passes[i] += night.passes[i];
            self.fails[i] += night.fails[i];
        }
    }
    /// Draws count half to each side.
    fn crew_rate(&self) -> f32 {
        (self.crew as f32 + self.draws as f32 / 2.) / self.nights.max(1) as f32
    }
    fn median_job(&mut self) -> f32 {
        self.lengths.sort_by(f32::total_cmp);
        self.lengths
            .get(self.lengths.len() / 2)
            .copied()
            .unwrap_or(0.)
    }
}

fn sweep(nights: u64, skill: f32) -> Vec<((u8, u8), Summary)> {
    std::thread::scope(|scope| {
        let runs: Vec<_> = FORMATS
            .iter()
            .filter(|(police, crew)| {
                std::env::var("CITY_BALANCE_ONLY")
                    .map(|only| only.split(',').any(|f| f == format!("{police}v{crew}")))
                    .unwrap_or(true)
            })
            .map(|&(police, crew)| {
                scope.spawn(move || {
                    let mut summary = Summary::default();
                    let base = std::env::var("CITY_BALANCE_SEED")
                        .ok()
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0);
                    for seed in 0..nights {
                        let night = play(police, crew, (base + seed) * 7 + 1, skill);
                        if std::env::var_os("CITY_BALANCE_DETAILS").is_some() {
                            println!(
                                "{police}v{crew} seed {} score {:?} phases {:?} arrests {:?} checks {:?}/{:?}",
                                base + seed,
                                night.score,
                                night.jobs.iter().map(|j| j.phase).collect::<Vec<_>>(),
                                night.jobs.iter().map(|j| j.arrests).collect::<Vec<_>>(),
                                night.passes,
                                night.fails,
                            );
                        }
                        summary.add(night);
                    }
                    ((police, crew), summary)
                })
            })
            .collect();
        runs.into_iter().map(|r| r.join().unwrap()).collect()
    })
}

fn pct(n: u32, of: u32) -> String {
    format!("{:.0}%", 100. * n as f32 / of.max(1) as f32)
}

#[test]
#[ignore = "plays hundreds of full nights; run on demand"]
fn formats_are_balanced() {
    let nights = std::env::var("CITY_BALANCE_NIGHTS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(40);
    let skill = std::env::var("CITY_BALANCE_SKILL")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0.4);
    let mut results = sweep(nights, skill);
    println!("{nights} nights per format, bot skill {skill}");
    println!(
        "| format | crew win | crew/police/draw | median job | jobs | delivered | stopped at power/vault/carry | wipes | arrests/job | cuffs slipped |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let mut out_of_band = Vec::new();
    for ((police, crew), s) in &mut results {
        let rate = s.crew_rate();
        if !(0.45..=0.55).contains(&rate) {
            out_of_band.push(format!("{police}v{crew} {:.0}%", rate * 100.));
        }
        let median = s.median_job();
        println!(
            "| {police}v{crew} | {:.0}% | {}/{}/{} | {median:.0}s | {} | {} | {}/{}/{} | {} | {:.2} | {}/{} |",
            rate * 100.,
            s.crew,
            s.police,
            s.draws,
            s.jobs,
            pct(s.delivered, s.jobs),
            pct(s.stopped[0], s.jobs),
            pct(s.stopped[1], s.jobs),
            pct(s.stopped[2], s.jobs),
            s.wipes,
            s.arrests as f32 / s.jobs.max(1) as f32,
            s.slips[1],
            s.slips[0],
        );
        if std::env::var_os("CITY_BALANCE_SCORES").is_some() {
            println!(
                "{police}v{crew} final scores: {:?}; 0-0 job phases: {:?}",
                s.score_counts, s.zero_zero_phases
            );
        }
    }
    println!();
    println!("| check | attempts | failed |");
    println!("|---|---|---|");
    for (i, kind) in KINDS.iter().enumerate() {
        let (pass, fail) = results
            .iter()
            .fold((0, 0), |(p, f), (_, s)| (p + s.passes[i], f + s.fails[i]));
        println!(
            "| {kind:?} | {} | {} |",
            pass + fail,
            pct(fail, pass + fail)
        );
    }
    assert!(
        out_of_band.is_empty(),
        "formats outside 45-55% crew wins: {}",
        out_of_band.join(", ")
    );
}

#[test]
fn a_seed_replays_the_same_night() {
    let a = play(1, 2, 11, 0.5);
    let b = play(1, 2, 11, 0.5);
    assert_eq!(a.score, b.score);
    assert_eq!(a.passes, b.passes);
    assert_eq!(a.fails, b.fails);
    assert_eq!(a.jobs.len(), b.jobs.len());
}
