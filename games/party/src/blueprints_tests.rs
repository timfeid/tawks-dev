use std::collections::{BTreeMap, HashSet};

use super::*;
use crate::blueprints_catalog::{CATALOG, Shape};
use crate::bout::{Contestant, Input, TICK_DT};

fn people(count: u8) -> Vec<Contestant> {
    (0..count).map(Contestant::person).collect()
}

fn bots(skills: &[f32]) -> Vec<Contestant> {
    skills
        .iter()
        .enumerate()
        .map(|(seat, skill)| Contestant::bot(seat as u8, *skill))
        .collect()
}

fn say(word: usize) -> Input {
    Input {
        fire: true,
        yaw: word as f32,
        ..Default::default()
    }
}

fn pick(word: usize) -> Input {
    Input {
        pitch: PICKED,
        ..say(word)
    }
}

fn wrong_for(game: &Blueprints) -> usize {
    let answer = vocabulary()[game.answer_term()].object;
    vocabulary()
        .iter()
        .position(|t| t.object != answer)
        .unwrap()
}

/// Ticks with nobody pressing anything.
fn idle(game: &mut Blueprints, seconds: f32, events: &mut Vec<Value>) {
    for _ in 0..(seconds / TICK_DT).round() as usize {
        game.step(&[], TICK_DT, events);
    }
}

fn press(game: &mut Blueprints, seat: u8, input: Input, events: &mut Vec<Value>) {
    game.step(&[(seat, input)], TICK_DT, events);
}

#[test]
fn every_object_is_buildable_and_named_once() {
    let mut names = HashSet::new();
    let mut words = HashSet::new();
    assert!(
        CATALOG.len() >= 40,
        "enough objects that bouts rarely repeat"
    );
    for object in CATALOG {
        assert!(names.insert(object.name), "{} listed twice", object.name);
        assert!((0.0..=1.0).contains(&object.difficulty));
        let parts = (object.build)();
        assert!(
            parts.len() >= 4,
            "{} needs more than {} parts",
            object.name,
            parts.len()
        );
        for part in &parts {
            for value in part.at.iter().chain(&part.size).chain(&part.turn) {
                assert!(value.is_finite(), "{} has a broken number", object.name);
            }
            assert!(
                part.size.iter().all(|s| *s > 0.0),
                "{} has a flat part",
                object.name
            );
            assert!(Shape::from_name(part.shape.name()) == Some(part.shape));
        }
        for word in object.terms {
            assert!(words.insert(normalize(word)), "{word} names two objects");
        }
    }
}

#[test]
fn every_term_resolves_to_itself_and_its_object() {
    for term in vocabulary() {
        let found = resolve(term.word).unwrap_or_else(|| panic!("{} does not resolve", term.word));
        assert_eq!(
            vocabulary()[found].object,
            term.object,
            "{} lands on the wrong object",
            term.word
        );
    }
}

fn means(text: &str) -> Option<&'static str> {
    resolve(text).map(|index| CATALOG[vocabulary()[index].object].name)
}

#[test]
fn typing_forgives_typos_synonyms_plurals_and_filler() {
    assert_eq!(means("bycicle"), Some("Bicycle"));
    assert_eq!(means("teapott"), Some("Teapot"));
    assert_eq!(means("hellicopter"), Some("Helicopter"));
    assert_eq!(means("girafe"), Some("Giraffe"));
    assert_eq!(means("couch"), Some("Sofa"));
    assert_eq!(means("bike"), Some("Bicycle"));
    assert_eq!(means("cars"), Some("Car"));
    assert_eq!(means("A Teapot"), Some("Teapot"));
    assert_eq!(means("  Fire-Hydrant! "), Some("Fire Hydrant"));
    assert_eq!(means("firehydrant"), Some("Fire Hydrant"));
    assert_eq!(means("a big red car"), Some("Car"));
    assert_eq!(means("lightbulb"), Some("Light Bulb"));
    assert_eq!(means("ice cream cone"), Some("Ice Cream"));
    assert_eq!(means("traffic cone"), Some("Traffic Cone"));
    assert_eq!(means("Pinapple"), Some("Pineapple"));
}

#[test]
fn short_words_must_be_exact_and_nonsense_is_unknown() {
    assert_eq!(
        means("cat"),
        None,
        "three letters forgive nothing: cat is not car"
    );
    assert_eq!(means("bux"), None);
    assert_eq!(means("qwertyuiop"), None);
    assert_eq!(means(""), None);
    assert_eq!(means("   "), None);
    assert_eq!(means("car"), Some("Car"));
    assert_eq!(means("bus"), Some("School Bus"));
}

#[test]
fn first_right_answer_scores_most_and_later_ones_less() {
    let mut game = Blueprints::new(&people(5), 3);
    let mut events = Vec::new();
    idle(&mut game, 3.0, &mut events);
    let answer = game.answer_term();
    press(&mut game, 2, say(answer), &mut events);
    press(&mut game, 0, say(answer), &mut events);
    press(&mut game, 4, say(answer), &mut events);
    press(&mut game, 1, say(answer), &mut events);
    assert_eq!(game.points(2), POINTS[0]);
    assert_eq!(game.points(0), POINTS[1]);
    assert_eq!(game.points(4), POINTS[2]);
    assert_eq!(game.points(1), POINTS[3]);
    assert_eq!(game.points(3), 0);
    press(&mut game, 2, say(answer), &mut events);
    assert_eq!(
        game.points(2),
        POINTS[0],
        "a second right answer adds nothing"
    );
    let solved: Vec<_> = events.iter().filter(|e| e["kind"] == "solved").collect();
    assert_eq!(solved.len(), 4);
    assert!(
        solved.iter().all(|e| e.get("word").is_none()),
        "a right word is never broadcast"
    );
}

#[test]
fn any_synonym_of_the_object_counts() {
    let mut game = Blueprints::new(&people(2), 11);
    let mut events = Vec::new();
    let object = vocabulary()[game.answer_term()].object;
    let last = vocabulary()
        .iter()
        .rposition(|t| t.object == object)
        .unwrap();
    press(&mut game, 0, say(last), &mut events);
    assert_eq!(game.points(0), POINTS[0]);
}

#[test]
fn a_wrong_guess_locks_you_out_for_two_seconds() {
    let mut game = Blueprints::new(&people(2), 5);
    let mut events = Vec::new();
    let answer = game.answer_term();
    let wrong = wrong_for(&game);
    press(&mut game, 0, say(wrong), &mut events);
    assert_eq!(events.last().unwrap()["kind"], "miss");
    assert!(
        events.last().unwrap()["word"].is_string(),
        "a wrong word is shown to the room"
    );
    idle(&mut game, LOCKOUT_SECONDS - 0.2, &mut events);
    press(&mut game, 0, say(answer), &mut events);
    assert_eq!(game.points(0), 0, "still locked out");
    assert_eq!(game.snapshot()["seats"][0]["state"], "locked");
    idle(&mut game, 0.3, &mut events);
    press(&mut game, 0, say(answer), &mut events);
    assert_eq!(game.points(0), POINTS[0], "free again after the lockout");
}

#[test]
fn an_unknown_word_still_costs_the_lockout() {
    let mut game = Blueprints::new(&people(2), 5);
    let mut events = Vec::new();
    press(
        &mut game,
        1,
        Input {
            fire: true,
            yaw: UNKNOWN,
            ..Default::default()
        },
        &mut events,
    );
    assert_eq!(events.last().unwrap()["kind"], "miss");
    assert!(events.last().unwrap()["word"].is_null());
    let answer = game.answer_term();
    press(&mut game, 1, say(answer), &mut events);
    assert_eq!(game.points(1), 0);
}

#[test]
fn malformed_presses_are_ignored_without_a_lockout() {
    let mut game = Blueprints::new(&people(2), 5);
    let mut events = Vec::new();
    for yaw in [0.5, -3.0, 99_999.0, f32::NAN, f32::INFINITY] {
        press(
            &mut game,
            0,
            Input {
                fire: true,
                yaw,
                ..Default::default()
            },
            &mut events,
        );
    }
    assert!(events.is_empty(), "nothing happened: {events:?}");
    let answer = game.answer_term();
    press(
        &mut game,
        0,
        Input {
            fire: false,
            ..say(answer)
        },
        &mut events,
    );
    assert_eq!(game.points(0), 0, "no fire, no guess");
    press(&mut game, 0, say(answer), &mut events);
    assert_eq!(game.points(0), POINTS[0]);
}

#[test]
fn controller_choices_open_with_the_blueprint() {
    let mut game = Blueprints::new(&people(2), 8);
    let mut events = Vec::new();
    assert!(game.snapshot()["choices"].is_null());
    let answer = game.answer_term();
    press(&mut game, 0, pick(answer), &mut events);
    assert_eq!(
        game.points(0),
        0,
        "a pick before the choices open does not count"
    );
    assert!(events.is_empty(), "nor lock anyone out");
    idle(&mut game, CHOICES_AT, &mut events);
    let choices = game.snapshot()["choices"].as_array().unwrap().clone();
    assert_eq!(choices.len(), 4);
    let terms: Vec<usize> = choices
        .iter()
        .map(|c| c["term"].as_u64().unwrap() as usize)
        .collect();
    assert!(terms.contains(&answer), "the answer is one of the four");
    let distinct: HashSet<usize> = terms.iter().map(|t| vocabulary()[*t].object).collect();
    assert_eq!(distinct.len(), 4, "four different objects");
    press(&mut game, 0, pick(answer), &mut events);
    assert_eq!(game.points(0), POINTS[0]);
}

#[test]
fn the_first_answer_starts_a_short_clock_for_everyone_else() {
    let mut game = Blueprints::new(&people(3), 2);
    let mut events = Vec::new();
    idle(&mut game, 2.0, &mut events);
    let answer = game.answer_term();
    press(&mut game, 1, say(answer), &mut events);
    assert!(
        (game.snapshot()["deadline"].as_f64().unwrap() as f32 - (2.0 + AFTER_FIRST_SECONDS)).abs()
            < 0.05
    );
    idle(&mut game, AFTER_FIRST_SECONDS - 0.2, &mut events);
    assert!(!game.revealing());
    idle(&mut game, 0.3, &mut events);
    assert!(game.revealing(), "the others ran out of time");
    let revealed = events.iter().find(|e| e["kind"] == "revealed").unwrap();
    assert_eq!(revealed["solvers"], json!([1]));
}

#[test]
fn everyone_right_reveals_at_once() {
    let mut game = Blueprints::new(&people(2), 4);
    let mut events = Vec::new();
    let answer = game.answer_term();
    press(&mut game, 0, say(answer), &mut events);
    assert!(!game.revealing());
    press(&mut game, 1, say(answer), &mut events);
    assert!(game.revealing());
    idle(&mut game, REVEAL_SECONDS + 0.1, &mut events);
    assert_eq!(game.round(), 1);
    assert!(!game.revealing());
    assert_eq!(
        game.snapshot()["seats"][0]["state"],
        "guessing",
        "a fresh object for everyone"
    );
}

#[test]
fn nobody_knowing_it_moves_on_after_the_full_look() {
    let mut game = Blueprints::new(&people(2), 4);
    let mut events = Vec::new();
    idle(&mut game, LOOK_SECONDS - 0.1, &mut events);
    assert!(!game.revealing());
    idle(&mut game, 0.2, &mut events);
    assert!(game.revealing());
    assert_eq!(game.snapshot()["seats"][1]["state"], "missed");
}

#[test]
fn the_name_stays_off_the_wire_until_the_reveal() {
    let mut game = Blueprints::new(&people(2), 21);
    let mut events = Vec::new();
    let name = CATALOG[vocabulary()[game.answer_term()].object].name;
    let mut closeup_parts = 0;
    let mut full_parts = 0;
    while !game.revealing() {
        let snap = game.snapshot();
        let text = snap.to_string();
        assert!(
            !text.contains(&format!("\"{name}\"")) || game.clock() >= CHOICES_AT,
            "name leaked early"
        );
        assert!(snap["answer"].is_null());
        let coloured = snap["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| !p["c"].is_null());
        match snap["shot"].as_str().unwrap() {
            "closeup" => {
                closeup_parts = closeup_parts.max(snap["parts"].as_array().unwrap().len());
                assert!(!coloured, "colour only in the full shot");
            }
            "angle" | "blueprint" => assert!(!coloured),
            "full" => {
                full_parts = snap["parts"].as_array().unwrap().len();
                assert!(coloured);
            }
            other => panic!("unexpected shot {other}"),
        }
        idle(&mut game, 0.5, &mut events);
    }
    assert!(closeup_parts > 0, "the close-up shows something");
    assert!(closeup_parts < full_parts, "and less than the whole thing");
    assert_eq!(game.snapshot()["answer"]["name"], name);
}

#[test]
fn shots_run_closeup_angle_blueprint_full_with_a_moving_camera() {
    let mut game = Blueprints::new(&people(2), 6);
    let mut events = Vec::new();
    let mut seen = Vec::new();
    let mut last_eye: Option<Value> = None;
    for _ in 0..(LOOK_SECONDS / 0.5) as usize - 1 {
        let snap = game.snapshot();
        let shot = snap["shot"].as_str().unwrap().to_string();
        if seen.last() != Some(&shot) {
            seen.push(shot);
        }
        let eye = snap["camera"]["eye"].clone();
        assert!(
            eye.as_array()
                .unwrap()
                .iter()
                .all(|v| v.as_f64().unwrap().is_finite())
        );
        assert_ne!(Some(&eye), last_eye.as_ref(), "the camera drifts");
        last_eye = Some(eye);
        idle(&mut game, 0.5, &mut events);
    }
    assert_eq!(seen, ["closeup", "angle", "blueprint", "full"]);
}

#[test]
fn a_whole_bout_ranks_everyone_and_groups_ties() {
    let mut game = Blueprints::new(&people(4), 9);
    let mut events = Vec::new();
    for _ in 0..OBJECTS {
        let answer = game.answer_term();
        press(&mut game, 0, say(answer), &mut events);
        press(&mut game, 1, say(answer), &mut events);
        press(&mut game, 2, say(answer), &mut events);
        press(&mut game, 3, say(answer), &mut events);
        idle(&mut game, REVEAL_SECONDS + 0.1, &mut events);
    }
    assert_eq!(
        game.placements().unwrap(),
        vec![vec![0], vec![1], vec![2], vec![3]]
    );

    let mut tie = Blueprints::new(&people(3), 9);
    let mut events = Vec::new();
    for round in 0..OBJECTS {
        let answer = tie.answer_term();
        let (a, b) = if round % 2 == 0 { (0, 1) } else { (1, 0) };
        press(&mut tie, a, say(answer), &mut events);
        press(&mut tie, b, say(answer), &mut events);
        idle(&mut tie, LOOK_SECONDS + REVEAL_SECONDS, &mut events);
    }
    let placed = tie.placements().expect("finished");
    assert_eq!(placed.len(), 3, "{placed:?}");
    assert_eq!(placed[2], vec![2], "the player who never answered is last");
    assert_eq!(tie.points(0), 3 * POINTS[0] + 2 * POINTS[1]);
    assert_eq!(tie.points(1), 2 * POINTS[0] + 3 * POINTS[1]);

    let mut even = Blueprints::new(&people(3), 1);
    idle(
        &mut even,
        OBJECTS as f32 * (LOOK_SECONDS + REVEAL_SECONDS) + 1.0,
        &mut Vec::new(),
    );
    assert_eq!(
        even.placements().unwrap(),
        vec![vec![0, 1, 2]],
        "nobody scored: one shared place"
    );
}

#[test]
fn a_player_who_leaves_never_stalls_the_bout() {
    let mut game = Blueprints::new(&bots(&[0.7, 0.7, 0.7]), 13);
    let mut events = Vec::new();
    let mut ticks = 0;
    while game.placements().is_none() {
        let inputs: Vec<(u8, Input)> = [0u8, 1]
            .into_iter()
            .map(|seat| (seat, game.bot_input(seat, TICK_DT)))
            .chain(std::iter::once((2u8, Input::default())))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        ticks += 1;
        assert!(ticks < (OBJECTS as f32 * (LOOK_SECONDS + REVEAL_SECONDS) / TICK_DT) as usize + 10);
    }
    let placed = game.placements().unwrap();
    assert_eq!(placed.iter().flatten().count(), 3);
    assert_eq!(placed.last().unwrap(), &vec![2], "the one who left is last");
}

struct Run {
    seconds: f32,
    points: Vec<u32>,
    misses: Vec<u32>,
    solved: usize,
    first_at: Vec<f32>,
    placements: Vec<Vec<u8>>,
}

fn play(skills: &[f32], seed: u64) -> Run {
    let field = bots(skills);
    let mut game = Blueprints::new(&field, seed);
    let mut events = Vec::new();
    let mut ticks = 0usize;
    let mut first_at = Vec::new();
    let mut round_clock = 0.0;
    while game.placements().is_none() {
        let inputs: Vec<(u8, Input)> = field
            .iter()
            .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
            .collect();
        let before = events.len();
        game.step(&inputs, TICK_DT, &mut events);
        if !game.revealing() {
            round_clock = game.clock();
        }
        for event in &events[before..] {
            if event["kind"] == "solved" && event["rank"] == 0 {
                first_at.push(round_clock / LOOK_SECONDS);
            }
        }
        ticks += 1;
        assert!(ticks < 60 * 60 * 3, "seed {seed} never finished");
    }
    Run {
        seconds: ticks as f32 * TICK_DT,
        points: field.iter().map(|c| game.points(c.seat)).collect(),
        misses: field
            .iter()
            .map(|c| {
                game.snapshot()["seats"][c.seat as usize]["misses"]
                    .as_u64()
                    .unwrap() as u32
            })
            .collect(),
        solved: events
            .iter()
            .filter(|e| e["kind"] == "revealed" && !e["solvers"].as_array().unwrap().is_empty())
            .count(),
        first_at,
        placements: game.placements().unwrap(),
    }
}

#[test]
fn bots_of_every_skill_finish_and_sharper_ones_win_more() {
    let skills = [0.15, 0.4, 0.7, 1.0];
    let mut wins = [0u32; 4];
    let mut points = [0u32; 4];
    let mut misses = [0u32; 4];
    let mut seconds = Vec::new();
    let mut solved = 0;
    let mut first_at = Vec::new();
    let runs = 300;
    for seed in 0..runs {
        let run = play(&skills, 1000 + seed);
        for seat in &run.placements[0] {
            wins[*seat as usize] += 1;
        }
        for i in 0..4 {
            points[i] += run.points[i];
            misses[i] += run.misses[i];
        }
        seconds.push(run.seconds);
        solved += run.solved;
        first_at.extend(run.first_at);
    }
    seconds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    first_at.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = seconds.iter().sum::<f32>() / seconds.len() as f32;
    let solved_share = solved as f32 / (runs as usize * OBJECTS) as f32;
    let first_median = first_at[first_at.len() / 2];
    println!(
        "blueprints sim, {runs} bouts of relaxed/steady/sharp/ruthless bots:\n  wins {wins:?}\n  mean points {:?}\n  wrong guesses per bout {:?}\n  length mean {mean:.1}s, min {:.1}s, max {:.1}s\n  objects solved {:.0}%, first answer at a median {:.0}% of the look",
        points.map(|p| p as f32 / runs as f32),
        misses.map(|m| m as f32 / runs as f32),
        seconds[0],
        seconds[seconds.len() - 1],
        solved_share * 100.0,
        first_median * 100.0,
    );
    assert!(wins[3] > wins[0] * 3, "ruthless beats relaxed: {wins:?}");
    assert!(
        points[3] > points[2] && points[2] > points[1] && points[1] > points[0],
        "{points:?}"
    );
    assert!(
        wins[0] > 0 || points[0] > 0,
        "relaxed bots still score sometimes"
    );
    assert!(
        misses[0] > misses[3],
        "clumsy bots guess wrong more: {misses:?}"
    );
    assert!(
        (45.0..=95.0).contains(&mean),
        "a bout runs about a minute: {mean}"
    );
    assert!(
        solved_share > 0.9,
        "almost every object gets solved by someone"
    );
    assert!(
        (0.3..0.75).contains(&first_median),
        "solved partway through the reveal: {first_median}"
    );
}

#[test]
fn a_room_of_average_bots_spreads_the_wins() {
    let mut wins: BTreeMap<u8, u32> = BTreeMap::new();
    for seed in 0..200 {
        let run = play(&[0.4; 6], 50 + seed);
        for seat in &run.placements[0] {
            *wins.entry(*seat).or_default() += 1;
        }
    }
    assert_eq!(wins.len(), 6, "everyone wins some: {wins:?}");
    let most = *wins.values().max().unwrap();
    let least = *wins.values().min().unwrap();
    assert!(most < least * 3, "no seat is favoured: {wins:?}");
}

#[test]
fn eight_players_fit() {
    let run = play(&[0.15, 0.3, 0.4, 0.5, 0.6, 0.7, 0.85, 1.0], 77);
    assert_eq!(run.placements.iter().flatten().count(), 8);
    let snap = Blueprints::new(&bots(&[0.5; 8]), 1).snapshot();
    assert_eq!(snap["seats"].as_array().unwrap().len(), 8);
}
