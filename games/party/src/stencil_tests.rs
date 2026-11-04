use super::*;
use crate::bout::{Contestant, TICK_DT};

fn people(count: u8) -> Vec<Contestant> {
    (0..count).map(Contestant::person).collect()
}

fn bots(count: u8, skill: f32) -> Vec<Contestant> {
    (0..count).map(|s| Contestant::bot(s, skill)).collect()
}

/// A scripted hand that runs its own pen along the outline `offset` off the
/// line at `pace` of the torch's top speed, either way round.
struct Tracer {
    seat: u8,
    lead: f32,
    offset: f32,
    pace: f32,
    reverse: bool,
    round: usize,
}

impl Tracer {
    fn new(seat: u8, offset: f32, pace: f32) -> Self {
        Tracer {
            seat,
            lead: 0.0,
            offset,
            pace,
            reverse: false,
            round: 0,
        }
    }

    fn input(&mut self, game: &StencilRush) -> Input {
        if game.round != self.round {
            self.round = game.round;
            self.lead = 0.0;
        }
        let shape = game.shape();
        if game.stage == Stage::Cut {
            self.lead = (self.lead + TORCH_SPEED * self.pace * TICK_DT).min(shape.length);
        }
        let s = if self.reverse {
            shape.length - self.lead
        } else {
            self.lead
        };
        let on = shape.at(s);
        let t = shape.tangent(s);
        Input {
            yaw: on.0 - t.1 * self.offset,
            pitch: on.1 + t.0 * self.offset,
            ..Default::default()
        }
    }
}

fn run_round(game: &mut StencilRush, tracers: &mut [Tracer], events: &mut Vec<Value>) {
    let round = game.round;
    for _ in 0..((PREVIEW_SECONDS + CUT_SECONDS + 1.0) / TICK_DT) as usize {
        if game.round != round || game.stage == Stage::Tally || game.placements().is_some() {
            return;
        }
        let inputs: Vec<(u8, Input)> = tracers
            .iter_mut()
            .map(|t| (t.seat, t.input(game)))
            .collect();
        game.step(&inputs, TICK_DT, events);
    }
}

fn play_bots(game: &mut StencilRush, seats: &[u8], events: &mut Vec<Value>) -> usize {
    let mut ticks = 0;
    while game.placements().is_none() && ticks < 6_000 {
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|s| (*s, game.bot_input(*s, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, events);
        ticks += 1;
    }
    ticks
}

#[test]
fn every_round_has_a_new_closed_outline_inside_the_sheet() {
    let game = StencilRush::new(&people(2), 3);
    assert_eq!(game.shapes.len(), ROUNDS);
    for (shape, kind) in game.shapes.iter().zip(SHAPES) {
        assert_eq!(shape.kind, kind);
        assert!(
            shape
                .points
                .iter()
                .all(|p| p.0.abs() < HALF_W * 0.9 && p.1.abs() < HALF_H * 0.9)
        );
        assert!(
            (2.5..7.0).contains(&shape.length),
            "{kind} is {}",
            shape.length
        );
        let gap = dist(*shape.points.last().unwrap(), shape.points[0]);
        assert!(gap < 0.05, "{kind} closes on itself");
    }
    let other = StencilRush::new(&people(2), 4);
    assert_ne!(
        game.shapes[1].points, other.shapes[1].points,
        "seeds draw different shapes"
    );
    let star = &game.shapes[1];
    let sharpest = (0..star.points.len())
        .map(|i| star.turn(star.cum[i], 0.03))
        .fold(0.0f32, f32::max);
    assert!(sharpest > 1.0, "the star has real corners, {sharpest}");
}

#[test]
fn a_clean_quick_cut_scores_near_full_marks_and_a_sloppy_one_far_less() {
    let mut game = StencilRush::new(&people(2), 9);
    let mut tracers = [Tracer::new(0, 0.0, 0.97), Tracer::new(1, 0.07, 0.97)];
    let mut events = Vec::new();
    run_round(&mut game, &mut tracers, &mut events);
    assert_eq!(game.stage, Stage::Tally);
    let clean = game.player(0).unwrap();
    let sloppy = game.player(1).unwrap();
    assert!(clean.finished.is_some() && sloppy.finished.is_some());
    assert!(clean.accuracy() > 0.97, "clean {}", clean.accuracy());
    assert!(sloppy.accuracy() < 0.75, "sloppy {}", sloppy.accuracy());
    assert!(clean.points > 90, "clean {}", clean.points);
    assert!(sloppy.points < clean.points / 2 + ROUND_BONUS);
    assert_eq!(game.winners, vec![0]);
    assert_eq!(clean.wins, 1);
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "round_over" && e["winners"] == json!([0]))
    );
}

#[test]
fn drifting_off_the_line_throws_sparks() {
    let mut game = StencilRush::new(&people(1), 2);
    let mut tracer = [Tracer::new(0, 0.08, 0.8)];
    let mut sparked = false;
    let mut events = Vec::new();
    for _ in 0..((PREVIEW_SECONDS + 3.0) / TICK_DT) as usize {
        let inputs: Vec<_> = tracer
            .iter_mut()
            .map(|t| (t.seat, t.input(&game)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        sparked |= game.snapshot()["players"][0]["spark"] == true;
    }
    assert!(sparked);
}

#[test]
fn taking_your_time_costs_points_even_when_clean() {
    let mut quick = StencilRush::new(&people(2), 5);
    let mut tracers = [Tracer::new(0, 0.0, 0.95), Tracer::new(1, 0.0, 0.5)];
    run_round(&mut quick, &mut tracers, &mut Vec::new());
    let fast = quick.player(0).unwrap();
    let slow = quick.player(1).unwrap();
    assert!(
        slow.finished.is_some(),
        "a half-speed cut still finishes in time"
    );
    assert!(fast.points > slow.points);
}

#[test]
fn either_way_round_counts() {
    let mut game = StencilRush::new(&people(1), 12);
    let mut tracer = Tracer::new(0, 0.0, 0.9);
    tracer.reverse = true;
    let mut tracers = [tracer];
    run_round(&mut game, &mut tracers, &mut Vec::new());
    let p = game.player(0).unwrap();
    assert_eq!(p.dir, -1);
    assert!(p.finished.is_some());
    assert!(p.accuracy() > 0.95);
}

#[test]
fn the_torch_never_outruns_its_top_speed_and_ignores_garbage() {
    let mut game = StencilRush::new(&people(1), 1);
    let mut events = Vec::new();
    for _ in 0..((PREVIEW_SECONDS + 0.05) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(game.stage, Stage::Cut);
    let before = game.player(0).unwrap().torch;
    let far = Input {
        yaw: 50.0,
        pitch: -50.0,
        ..Default::default()
    };
    game.step(&[(0, far)], TICK_DT, &mut events);
    let after = game.player(0).unwrap().torch;
    assert!(dist(before, after) <= TORCH_SPEED * TICK_DT + 1e-5);
    assert_eq!(
        game.player(0).unwrap().aim,
        (HALF_W, -HALF_H),
        "aim is held to the sheet"
    );
    let bad = Input {
        yaw: f32::NAN,
        pitch: f32::INFINITY,
        ..Default::default()
    };
    game.step(&[(0, bad)], TICK_DT, &mut events);
    assert_eq!(
        game.player(0).unwrap().aim,
        (HALF_W, -HALF_H),
        "non-finite aim is ignored"
    );
    for _ in 0..240 {
        game.step(&[(0, far)], TICK_DT, &mut events);
    }
    let p = game.player(0).unwrap();
    assert!(p.torch.0 <= HALF_W && p.torch.1 >= -HALF_H);
    assert!(
        p.completion(game.shape().length) < 0.05,
        "racing off the sheet cuts nothing"
    );
}

#[test]
fn smaller_cutter_sweeps_faster_without_exceeding_its_own_limit() {
    let mut contestants = people(2);
    contestants[0].body_width = 0.7;
    contestants[1].body_width = 1.4;
    let mut game = StencilRush::new(&contestants, 1);
    let mut events = Vec::new();
    for _ in 0..((PREVIEW_SECONDS + 0.05) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    let start = game.player(0).unwrap().torch;
    let far = Input {
        yaw: 1.0,
        pitch: -0.7,
        ..Default::default()
    };
    game.step(&[(0, far), (1, far)], TICK_DT, &mut events);
    let small = dist(start, game.player(0).unwrap().torch);
    let broad = dist(start, game.player(1).unwrap().torch);
    assert!(small > broad, "small pet's torch should be quicker");
    assert!(small <= torch_speed(0.7) * TICK_DT + 1e-5);
    assert!(broad <= torch_speed(1.4) * TICK_DT + 1e-5);
}

#[test]
fn cutting_across_the_middle_is_no_shortcut() {
    let mut game = StencilRush::new(&people(1), 6);
    let mut events = Vec::new();
    for _ in 0..((PREVIEW_SECONDS + 0.05) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    let shape = game.shape().clone();
    let across = shape.at(shape.length * 0.5);
    for _ in 0..(4.0 / TICK_DT) as usize {
        game.step(
            &[(
                0,
                Input {
                    yaw: across.0,
                    pitch: across.1,
                    ..Default::default()
                },
            )],
            TICK_DT,
            &mut events,
        );
    }
    let p = game.player(0).unwrap();
    assert!(
        p.completion(shape.length) < 0.1,
        "progress {}",
        p.completion(shape.length)
    );
    assert!(p.accuracy() < 0.5);
}

#[test]
fn a_player_who_left_does_not_hold_the_round_open() {
    let mut game = StencilRush::new(&people(2), 8);
    let mut tracers = [Tracer::new(0, 0.0, 0.95)];
    let mut events = Vec::new();
    let mut ticks = 0;
    while game.stage != Stage::Tally {
        let inputs: Vec<_> = tracers
            .iter_mut()
            .map(|t| (t.seat, t.input(&game)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        ticks += 1;
    }
    let cut_seconds = ticks as f32 * TICK_DT - PREVIEW_SECONDS;
    assert!(
        cut_seconds < CUT_SECONDS - 1.0,
        "the round ended at {cut_seconds}"
    );
    let gone = game.player(1).unwrap();
    assert_eq!(gone.points, 0);
    assert!(gone.finished.is_none());
}

#[test]
fn nobody_finishing_still_ends_on_the_clock() {
    let mut game = StencilRush::new(&people(3), 8);
    let mut events = Vec::new();
    for _ in 0..((PREVIEW_SECONDS + CUT_SECONDS + 0.1) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(game.stage, Stage::Tally);
    assert!(game.winners.is_empty(), "no cut, no winner");
    assert!(game.players.iter().all(|p| p.points == 0));
}

#[test]
fn identical_cuts_tie_and_share_the_round() {
    let mut game = StencilRush::new(&people(3), 10);
    let mut events = Vec::new();
    for _ in 0..ROUNDS {
        let mut tracers = [
            Tracer::new(0, 0.0, 0.9),
            Tracer::new(1, 0.0, 0.9),
            Tracer::new(2, 0.1, 0.6),
        ];
        run_round(&mut game, &mut tracers, &mut events);
        while game.stage == Stage::Tally && game.placements().is_none() {
            game.step(&[], TICK_DT, &mut events);
        }
    }
    let placements = game.placements().expect("four rounds and done");
    assert_eq!(placements[0], vec![0, 1]);
    assert_eq!(placements[1], vec![2]);
    assert_eq!(game.player(0).unwrap().wins, ROUNDS as u32);
    assert_eq!(game.player(0).unwrap().history.len(), ROUNDS);
}

#[test]
fn bots_finish_every_round_inside_the_bout_length() {
    for count in [2u8, 4, 8] {
        let field = bots(count, 0.5);
        let seats: Vec<u8> = field.iter().map(|c| c.seat).collect();
        let mut game = StencilRush::new(&field, 21 + count as u64);
        let mut events = Vec::new();
        let ticks = play_bots(&mut game, &seats, &mut events);
        let seconds = ticks as f32 * TICK_DT;
        assert!(
            (40.0..=90.0).contains(&seconds),
            "{count} bots took {seconds}s"
        );
        let placements = game.placements().expect("bots finish");
        assert_eq!(placements.iter().flatten().count(), count as usize);
        let done = events.iter().filter(|e| e["kind"] == "cut_done").count();
        assert!(
            done >= count as usize * ROUNDS * 3 / 4,
            "{done} cuts finished"
        );
    }
}

#[test]
fn bot_hands_wobble_and_sometimes_overshoot() {
    let mut sparks = 0;
    let mut accuracy = Vec::new();
    for seed in 0..6 {
        let field = bots(4, 0.15);
        let mut game = StencilRush::new(&field, seed);
        let mut events = Vec::new();
        while game.placements().is_none() {
            let inputs: Vec<(u8, Input)> =
                (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
            game.step(&inputs, TICK_DT, &mut events);
            sparks += game.players.iter().filter(|p| p.spark).count();
            if game.stage == Stage::Tally && game.stage_time < TICK_DT * 1.5 {
                accuracy.extend(game.players.iter().map(|p| p.accuracy()));
            }
        }
    }
    let mean = accuracy.iter().sum::<f32>() / accuracy.len() as f32;
    assert!(sparks > 0, "relaxed bots stray off the line now and then");
    assert!(
        mean < 0.97 && mean > 0.4,
        "relaxed bots are human, mean accuracy {mean}"
    );
}

fn skill_totals(skill: f32, seeds: u64) -> f32 {
    let mut total = 0.0;
    for seed in 0..seeds {
        let field = bots(4, skill);
        let seats: Vec<u8> = (0..4).collect();
        let mut game = StencilRush::new(&field, 100 + seed);
        play_bots(&mut game, &seats, &mut Vec::new());
        total += game.players.iter().map(|p| p.total as f32).sum::<f32>() / 4.0;
    }
    total / seeds as f32
}

#[test]
fn sharper_bots_cut_cleaner() {
    let relaxed = skill_totals(0.15, 6);
    let ruthless = skill_totals(1.0, 6);
    assert!(
        ruthless > relaxed * 1.2,
        "ruthless {ruthless} against relaxed {relaxed}"
    );
}

/// cargo test -p party stencil::tests::balance -- --ignored --nocapture
#[test]
#[ignore]
fn balance() {
    let mut rows = Vec::new();
    for (name, skill) in crate::bout::SKILLS {
        let (mut acc, mut fin, mut pts, mut secs, mut over, mut n) =
            (0.0, 0.0, 0.0, 0.0, 0usize, 0.0);
        let seeds = 200;
        for seed in 0..seeds {
            let field = bots(4, *skill);
            let seats: Vec<u8> = (0..4).collect();
            let mut game = StencilRush::new(&field, 7_000 + seed);
            let mut events = Vec::new();
            let mut ticks = 0;
            while game.placements().is_none() && ticks < 6_000 {
                let inputs: Vec<(u8, Input)> = seats
                    .iter()
                    .map(|s| (*s, game.bot_input(*s, TICK_DT)))
                    .collect();
                game.step(&inputs, TICK_DT, &mut events);
                ticks += 1;
                if game.stage == Stage::Tally && game.stage_time < TICK_DT * 1.5 {
                    for p in &game.players {
                        acc += p.accuracy();
                        fin += p.finished.is_some() as u32 as f32;
                        pts += p.points as f32;
                        n += 1.0;
                    }
                }
            }
            if game.placements().is_none() {
                over += 1;
            }
            secs += ticks as f32 * TICK_DT;
        }
        rows.push(format!(
            "{name:>9}: accuracy {:.3}  finished {:.0}%  round points {:.1}  bout {:.1}s  unfinished bouts {over}",
            acc / n,
            100.0 * fin / n,
            pts / n,
            secs / seeds as f32
        ));
    }
    let mut mixed_wins = [0u32; 4];
    for seed in 0..400 {
        let field: Vec<Contestant> = crate::bout::SKILLS
            .iter()
            .enumerate()
            .map(|(i, (_, k))| Contestant::bot(i as u8, *k))
            .collect();
        let mut game = StencilRush::new(&field, 9_000 + seed);
        play_bots(&mut game, &[0, 1, 2, 3], &mut Vec::new());
        for seat in &game.placements().unwrap()[0] {
            mixed_wins[*seat as usize] += 1;
        }
    }
    rows.push(format!(
        "mixed field bout wins relaxed/steady/sharp/ruthless: {mixed_wins:?}"
    ));
    println!("{}", rows.join("\n"));
}
