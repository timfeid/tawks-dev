//! Moving ink marks draw players through an unfolding page maze.
use crate::bout::{Bout, Contestant, Input, Stick};
use serde_json::{Value, json};

pub const KEY: &str = "margin_run";
pub const LIMIT: f32 = 62.0;
pub const HALF_X: f32 = 9.4;
pub const HALF_Z: f32 = 7.2;
pub const GAP_HALF: f32 = 1.55;
const SPEED: f32 = 5.8;
const MARK_RADIUS: f32 = 0.82;
const LEAVE_SECONDS: f32 = 4.0;
const SEAMS: [f32; 2] = [-3.15, 3.15];

fn folio(x: f32) -> usize {
    if x < SEAMS[0] {
        0
    } else if x < SEAMS[1] {
        1
    } else {
        2
    }
}
pub fn gap(seam: usize, time: f32) -> f32 {
    (time * 0.51 + seam as f32 * 2.2).sin() * 4.6
}
pub fn mark_position(section: usize, phase: f32, speed: f32, time: f32) -> (f32, f32) {
    let center = [-6.25, 0.0, 6.25][section];
    (
        center + (time * 0.72 + phase).sin() * 1.05,
        (time * speed + phase * 1.73).sin() * 5.55,
    )
}

#[derive(Clone)]
struct Player {
    seat: u8,
    skill: f32,
    x: f32,
    z: f32,
    vx: f32,
    vz: f32,
    yaw: f32,
    score: u32,
    catches: u32,
    blocked: u32,
    missing: f32,
    left: bool,
    planned: (f32, f32),
    plan_at: f32,
}

#[derive(Clone)]
struct Mark {
    section: usize,
    phase: f32,
    speed: f32,
    taken: u32,
}
impl Mark {
    fn pos(&self, time: f32) -> (f32, f32) {
        mark_position(self.section, self.phase, self.speed, time)
    }
}

pub struct MarginRun {
    players: Vec<Player>,
    marks: [Mark; 2],
    seed: u64,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}

impl MarginRun {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let place = (i + seed as usize % count) % count;
                let section = place % 3;
                let x = [-6.25, 0.0, 6.25][section];
                let z = -4.8 + (place / 3) as f32 * 9.6 / ((count + 2) / 3).max(1) as f32;
                Player {
                    seat: c.seat,
                    skill: c.skill.clamp(0.0, 1.0),
                    x,
                    z,
                    vx: 0.,
                    vz: 0.,
                    yaw: 0.,
                    score: 0,
                    catches: 0,
                    blocked: 0,
                    missing: 0.,
                    left: false,
                    planned: (x, z),
                    plan_at: -1.,
                }
            })
            .collect();
        let shift = (seed % 3) as usize;
        let marks = [
            Mark {
                section: shift,
                phase: seed as f32 * 0.31,
                speed: 0.72,
                taken: 0,
            },
            Mark {
                section: (shift + 2) % 3,
                phase: 2.3 + seed as f32 * 0.17,
                speed: 0.58,
                taken: 0,
            },
        ];
        Self {
            players,
            marks,
            seed,
            time: 0.,
            placed: None,
        }
    }

    fn finish(&mut self) {
        let mut ranked = self.players.iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            a.left
                .cmp(&b.left)
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| b.catches.cmp(&a.catches))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| q.left == p.left && q.score == p.score && q.catches == p.catches);
            if same {
                groups.last_mut().unwrap().push(p.seat);
            } else {
                groups.push(vec![p.seat]);
            }
        }
        self.placed = Some(groups);
    }
}

impl Bout for MarginRun {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":12.0,"half_x":HALF_X,"half_z":HALF_Z,"seams":SEAMS,"gap_half":GAP_HALF,"limit":LIMIT})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        let dt = if dt.is_finite() {
            dt.clamp(0., 0.1)
        } else {
            0.
        };
        self.time += dt;
        for p in &mut self.players {
            if p.left {
                continue;
            }
            let Some(input) = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, i)| *i)
            else {
                p.missing += dt;
                p.vx = 0.;
                p.vz = 0.;
                if p.missing >= LEAVE_SECONDS {
                    p.left = true;
                    events.push(json!({"kind":"margin_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.;
            let stick = input.stick.clamped();
            p.vx = stick.x * SPEED;
            p.vz = stick.z * SPEED;
            let old_x = p.x;
            let mut x = (p.x + p.vx * dt).clamp(-HALF_X, HALF_X);
            let z = (p.z + p.vz * dt).clamp(-HALF_Z, HALF_Z);
            for (index, seam) in SEAMS.iter().enumerate() {
                if (old_x < *seam && x >= *seam || old_x > *seam && x <= *seam)
                    && (z - gap(index, self.time)).abs() > GAP_HALF
                {
                    x = if old_x < *seam {
                        *seam - 0.05
                    } else {
                        *seam + 0.05
                    };
                    p.blocked += 1;
                    if p.blocked % 30 == 1 {
                        events.push(json!({"kind":"margin_fold","seat":p.seat,"seam":index}));
                    }
                }
            }
            p.x = x;
            p.z = z;
            if p.vx.hypot(p.vz) > 0.1 {
                p.yaw = p.vx.atan2(p.vz);
            }
        }
        for (index, mark) in self.marks.iter_mut().enumerate() {
            let (x, z) = mark.pos(self.time);
            let closest = self
                .players
                .iter()
                .filter(|p| !p.left)
                .map(|p| (p.x - x).hypot(p.z - z))
                .fold(f32::INFINITY, f32::min);
            if closest <= MARK_RADIUS {
                // Contacts in the same tick share a catch; seat iteration order cannot break a tie.
                for player in self.players.iter_mut().filter(|p| !p.left) {
                    if (player.x - x).hypot(player.z - z) > closest + 0.03 {
                        continue;
                    }
                    player.score += 2;
                    player.catches += 1;
                    events.push(json!({"kind":"margin_catch","seat":player.seat,"mark":index}));
                }
                mark.taken += 1;
                mark.section =
                    (mark.section + 1 + (self.seed as usize + index + mark.taken as usize) % 2) % 3;
                mark.phase += (1.7 + index as f32 * 0.6).rem_euclid(6.28);
            }
        }
        if self.time >= LIMIT || self.players.iter().all(|p| p.left) {
            self.finish();
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self.players.iter().position(|p| p.seat == seat && !p.left) else {
            return Input::default();
        };
        let p = &self.players[index];
        if self.time >= p.plan_at {
            let (choice, _) = self
                .marks
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let (x, z) = m.pos(self.time + 0.25);
                    let cross = if folio(p.x) != m.section { 3.0 } else { 0.0 };
                    (i, (p.x - x).hypot(p.z - z) + cross)
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            let mark = &self.marks[choice];
            let (mx, mz) = mark.pos(self.time + 0.35);
            let target = if folio(p.x) == mark.section {
                (mx, mz)
            } else {
                let seam = if mark.section > folio(p.x) {
                    folio(p.x).min(1)
                } else {
                    folio(p.x).saturating_sub(1)
                };
                let open = gap(seam, self.time + 0.35);
                let x = SEAMS[seam]
                    + if mark.section > folio(p.x) {
                        0.55
                    } else {
                        -0.55
                    };
                (x, open)
            };
            let p = &mut self.players[index];
            p.planned = target;
            p.plan_at = self.time + 0.14 + (1.0 - p.skill) * 0.48;
        }
        let p = &self.players[index];
        let dx = p.planned.0 - p.x;
        let dz = p.planned.1 - p.z;
        let length = dx.hypot(dz).max(0.01);
        Input::from_stick(Stick {
            x: if length > 0.25 { dx / length } else { 0. },
            z: if length > 0.25 { dz / length } else { 0. },
        })
    }

    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),
            "gaps":[gap(0,self.time),gap(1,self.time)],
            "marks":self.marks.iter().enumerate().map(|(i,m)|{let (x,z)=m.pos(self.time);json!({"id":i,"x":x,"z":z,"section":m.section,"phase":m.phase,"speed":m.speed})}).collect::<Vec<_>>(),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"vx":p.vx,"vz":p.vz,"yaw":p.yaw,"score":p.score,"catches":p.catches,"left":p.left})).collect::<Vec<_>>()
        })
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> MarginRun {
        MarginRun::new(&[Contestant::person(0), Contestant::person(1)], 9)
    }
    #[test]
    fn folds_block_except_at_the_moving_gap() {
        let mut g = game();
        let mut events = Vec::new();
        g.players[0].x = -3.2;
        g.players[0].z = 6.8;
        g.step(
            &[
                (0, Input::from_stick(Stick { x: 1., z: 0. })),
                (1, Input::default()),
            ],
            0.1,
            &mut events,
        );
        assert!(g.players[0].x < SEAMS[0]);
        g.players[0].z = gap(0, g.time);
        g.players[0].x = -3.2;
        g.step(
            &[
                (0, Input::from_stick(Stick { x: 1., z: 0. })),
                (1, Input::default()),
            ],
            0.1,
            &mut events,
        );
        assert!(g.players[0].x > SEAMS[0]);
    }
    #[test]
    fn only_touching_the_mark_scores_and_respawns_elsewhere() {
        let mut g = game();
        let mut events = Vec::new();
        let old = g.marks[0].section;
        let (x, z) = g.marks[0].pos(0.);
        g.players[0].x = x;
        g.players[0].z = z;
        g.step(
            &[(0, Input::default()), (1, Input::default())],
            1. / 60.,
            &mut events,
        );
        assert_eq!(g.players[0].score, 2);
        assert_ne!(g.marks[0].section, old);
        assert_eq!(g.players[1].score, 0);
    }
    #[test]
    fn snapshot_exports_the_mark_motion_used_for_visual_prediction() {
        let g = game();
        let view = g.snapshot();
        let mark = &view["marks"][0];
        let (x, z) = mark_position(
            mark["section"].as_u64().unwrap() as usize,
            mark["phase"].as_f64().unwrap() as f32,
            mark["speed"].as_f64().unwrap() as f32,
            view["time"].as_f64().unwrap() as f32,
        );
        assert!((mark["x"].as_f64().unwrap() as f32 - x).abs() < 0.0001);
        assert!((mark["z"].as_f64().unwrap() as f32 - z).abs() < 0.0001);
    }
    #[test]
    fn invalid_input_is_clamped_and_leaver_finishes_last() {
        let mut g = game();
        let mut events = Vec::new();
        for _ in 0..250 {
            g.step(
                &[(
                    0,
                    Input::from_stick(Stick {
                        x: f32::INFINITY,
                        z: f32::NAN,
                    }),
                )],
                1. / 60.,
                &mut events,
            );
        }
        assert!(g.players[0].x.is_finite() && g.players[0].z.is_finite());
        assert!(g.players[1].left);
        while g.placements().is_none() {
            g.step(&[(0, Input::default())], 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().last(), Some(&vec![1]));
    }
    #[test]
    fn bots_finish_with_catches() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.6))
            .collect::<Vec<_>>();
        let mut g = MarginRun::new(&seats, 7);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().map(|p| p.catches).sum::<u32>() > 2);
    }

    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.5))
            .collect::<Vec<_>>();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        for seed in 0..160 {
            let mut g = MarginRun::new(&seats, seed);
            let mut events = Vec::new();
            while g.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                g.step(&inputs, 1. / 60., &mut events);
            }
            for &seat in &g.placements().unwrap()[0] {
                first[seat as usize] += 1;
            }
            for p in &g.players {
                scores[p.seat as usize] += p.score;
            }
        }
        println!("Margin Run equal skill: first={first:?}, total_scores={scores:?}");
        assert!(*first.iter().max().unwrap() - *first.iter().min().unwrap() <= 40);
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 500);
    }
}
