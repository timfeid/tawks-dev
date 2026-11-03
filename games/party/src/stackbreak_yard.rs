//! Each player shepherds one ball into shared block towers. Impacts knock
//! blocks away, a complete collapse pays a bonus, then the tower rebuilds.
use crate::bout::{Bout, Contestant, Input, Stick};
use serde_json::{Value, json};
use std::f32::consts::TAU;

pub const KEY: &str = "stackbreak_yard";
pub const LIMIT: f32 = 62.0;
pub const RADIUS: f32 = 10.0;
const WALK_SPEED: f32 = 6.0;
const BALL_RADIUS: f32 = 0.62;
const LEAVE_SECONDS: f32 = 4.0;
const TOWER_POS: [(f32, f32); 4] = [(-3.5, -3.5), (3.5, -3.5), (3.5, 3.5), (-3.5, 3.5)];

fn norm(x: f32, z: f32) -> (f32, f32) {
    let len = x.hypot(z).max(0.001);
    (x / len, z / len)
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
    ball_x: f32,
    ball_z: f32,
    ball_vx: f32,
    ball_vz: f32,
    points: u32,
    blocks: u32,
    collapses: u32,
    missing: f32,
    left: bool,
    held_fire: bool,
    last_shove: f32,
    plan: (f32, f32),
    plan_tower: (f32, f32),
    plan_at: f32,
}
#[derive(Clone)]
struct Tower {
    health: u8,
    rebuild: f32,
    cool: f32,
}
pub struct StackbreakYard {
    players: Vec<Player>,
    towers: [Tower; 4],
    seed: u64,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}

impl StackbreakYard {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let slot = (i + seed as usize % count) % count;
                let angle = TAU * slot as f32 / count as f32;
                let x = angle.sin() * 8.0;
                let z = angle.cos() * 8.0;
                Player {
                    seat: c.seat,
                    skill: c.skill.clamp(0., 1.),
                    x,
                    z,
                    vx: 0.,
                    vz: 0.,
                    yaw: angle,
                    ball_x: angle.sin() * 6.4,
                    ball_z: angle.cos() * 6.4,
                    ball_vx: 0.,
                    ball_vz: 0.,
                    points: 0,
                    blocks: 0,
                    collapses: 0,
                    missing: 0.,
                    left: false,
                    held_fire: false,
                    last_shove: -10.,
                    plan: (x, z),
                    plan_tower: (0., 0.),
                    plan_at: -1.,
                }
            })
            .collect();
        Self {
            players,
            towers: std::array::from_fn(|_| Tower {
                health: 3,
                rebuild: 0.,
                cool: 0.,
            }),
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
                .then_with(|| b.points.cmp(&a.points))
                .then_with(|| b.collapses.cmp(&a.collapses))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| {
                    q.left == p.left && q.points == p.points && q.collapses == p.collapses
                });
            if same {
                groups.last_mut().unwrap().push(p.seat);
            } else {
                groups.push(vec![p.seat]);
            }
        }
        self.placed = Some(groups);
    }
}

impl Bout for StackbreakYard {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":RADIUS,"limit":LIMIT,"ball_radius":BALL_RADIUS,
            "towers":TOWER_POS.iter().enumerate().map(|(id,(x,z))|json!({"id":id,"x":x,"z":z})).collect::<Vec<_>>()})
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
        for tower in &mut self.towers {
            tower.cool = (tower.cool - dt).max(0.);
            if tower.health == 0 {
                tower.rebuild = (tower.rebuild - dt).max(0.);
                if tower.rebuild == 0. {
                    tower.health = 3;
                    events.push(json!({"kind":"stack_rebuild"}));
                }
            }
        }
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
                    events.push(json!({"kind":"stack_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.;
            let stick = input.stick.clamped();
            p.vx = stick.x * WALK_SPEED;
            p.vz = stick.z * WALK_SPEED;
            p.x += p.vx * dt;
            p.z += p.vz * dt;
            let distance = p.x.hypot(p.z);
            if distance > RADIUS - 0.45 {
                let s = (RADIUS - 0.45) / distance;
                p.x *= s;
                p.z *= s;
            }
            if p.vx.hypot(p.vz) > 0.1 {
                p.yaw = p.vx.atan2(p.vz);
            }
            let dx = p.ball_x - p.x;
            let dz = p.ball_z - p.z;
            let distance = dx.hypot(dz);
            let (nx, nz) = norm(dx, dz);
            if distance < BALL_RADIUS + 0.55 {
                p.x = p.ball_x - nx * (BALL_RADIUS + 0.55);
                p.z = p.ball_z - nz * (BALL_RADIUS + 0.55);
                let drive = (p.vx * nx + p.vz * nz).max(0.) * dt * 2.1;
                p.ball_vx += nx * drive;
                p.ball_vz += nz * drive;
            }
            let fire = input.fire || input.jump;
            if fire
                && !p.held_fire
                && self.time - p.last_shove >= 0.7
                && distance < BALL_RADIUS + 1.12
            {
                p.ball_vx += nx * 6.5;
                p.ball_vz += nz * 6.5;
                p.last_shove = self.time;
                events.push(json!({"kind":"stack_shove","seat":p.seat}));
            }
            p.held_fire = fire;
        }
        for p in &mut self.players {
            if p.left {
                continue;
            }
            let drag = (-0.52 * dt).exp();
            p.ball_vx *= drag;
            p.ball_vz *= drag;
            let speed = p.ball_vx.hypot(p.ball_vz);
            if speed > 10. {
                p.ball_vx *= 10. / speed;
                p.ball_vz *= 10. / speed;
            }
            p.ball_x += p.ball_vx * dt;
            p.ball_z += p.ball_vz * dt;
            let radius = p.ball_x.hypot(p.ball_z);
            if radius > RADIUS - BALL_RADIUS {
                let (nx, nz) = norm(p.ball_x, p.ball_z);
                p.ball_x = nx * (RADIUS - BALL_RADIUS);
                p.ball_z = nz * (RADIUS - BALL_RADIUS);
                let outward = p.ball_vx * nx + p.ball_vz * nz;
                if outward > 0. {
                    p.ball_vx -= nx * outward * 1.7;
                    p.ball_vz -= nz * outward * 1.7;
                }
            }
        }
        for i in 0..self.players.len() {
            for j in i + 1..self.players.len() {
                let (left, right) = self.players.split_at_mut(j);
                let a = &mut left[i];
                let b = &mut right[0];
                if a.left || b.left {
                    continue;
                }
                let dx = b.ball_x - a.ball_x;
                let dz = b.ball_z - a.ball_z;
                let distance = dx.hypot(dz);
                if distance >= BALL_RADIUS * 2. || distance < 0.001 {
                    continue;
                }
                let (nx, nz) = norm(dx, dz);
                let overlap = BALL_RADIUS * 2. - distance;
                a.ball_x -= nx * overlap * 0.5;
                a.ball_z -= nz * overlap * 0.5;
                b.ball_x += nx * overlap * 0.5;
                b.ball_z += nz * overlap * 0.5;
                let approach = (a.ball_vx - b.ball_vx) * nx + (a.ball_vz - b.ball_vz) * nz;
                if approach > 0. {
                    let hit = approach * 0.72;
                    a.ball_vx -= nx * hit;
                    a.ball_vz -= nz * hit;
                    b.ball_vx += nx * hit;
                    b.ball_vz += nz * hit;
                }
            }
        }
        for (id, tower) in self.towers.iter_mut().enumerate() {
            if tower.health == 0 || tower.cool > 0. {
                continue;
            }
            let (tx, tz) = TOWER_POS[id];
            let Some(player) = self
                .players
                .iter_mut()
                .filter(|p| !p.left)
                .filter(|p| {
                    (p.ball_x - tx).hypot(p.ball_z - tz) < BALL_RADIUS + 0.82
                        && p.ball_vx.hypot(p.ball_vz) > 1.5
                })
                .max_by(|a, b| {
                    a.ball_vx
                        .hypot(a.ball_vz)
                        .total_cmp(&b.ball_vx.hypot(b.ball_vz))
                })
            else {
                continue;
            };
            tower.health -= 1;
            tower.cool = 0.38;
            player.points += 1;
            player.blocks += 1;
            let (nx, nz) = norm(player.ball_x - tx, player.ball_z - tz);
            let dot = player.ball_vx * nx + player.ball_vz * nz;
            player.ball_vx -= nx * dot * 1.5;
            player.ball_vz -= nz * dot * 1.5;
            events.push(
                json!({"kind":"stack_hit","seat":player.seat,"tower":id,"health":tower.health}),
            );
            if tower.health == 0 {
                tower.rebuild = 3.4;
                player.points += 3;
                player.collapses += 1;
                events.push(json!({"kind":"stack_collapse","seat":player.seat,"tower":id}));
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
        if self.time >= self.players[index].plan_at {
            let p = &self.players[index];
            let tower = TOWER_POS
                .iter()
                .enumerate()
                .filter(|(id, _)| self.towers[*id].health > 0)
                .min_by(|a, b| {
                    let da = (p.ball_x - (a.1).0).hypot(p.ball_z - (a.1).1);
                    let db = (p.ball_x - (b.1).0).hypot(p.ball_z - (b.1).1);
                    da.total_cmp(&db)
                })
                .map(|(_, at)| *at)
                .unwrap_or((0., 0.));
            let (dx, dz) = norm(tower.0 - p.ball_x, tower.1 - p.ball_z);
            let noise = ((self
                .seed
                .wrapping_add(seat as u64 * 47)
                .wrapping_add(self.time as u64 * 29)
                % 97) as f32
                / 97.
                - 0.5)
                * (1. - p.skill)
                * 0.75;
            let target = (
                p.ball_x - dx * (BALL_RADIUS + 1.05) + noise,
                p.ball_z - dz * (BALL_RADIUS + 1.05) - noise,
            );
            let p = &mut self.players[index];
            p.plan = target;
            p.plan_tower = tower;
            p.plan_at = self.time + 0.16 + (1. - p.skill) * 0.43;
        }
        let p = &self.players[index];
        let to_plan = (p.plan.0 - p.x, p.plan.1 - p.z);
        let distance = to_plan.0.hypot(to_plan.1);
        let target = if distance > 0.42 {
            p.plan
        } else {
            (
                p.ball_x + (p.ball_x - p.x) * 0.3,
                p.ball_z + (p.ball_z - p.z) * 0.3,
            )
        };
        let (x, z) = norm(target.0 - p.x, target.1 - p.z);
        let near = (p.x - p.ball_x).hypot(p.z - p.ball_z) < BALL_RADIUS + 1.05;
        let (shot_x, shot_z) = norm(p.ball_x - p.x, p.ball_z - p.z);
        let (goal_x, goal_z) = norm(p.plan_tower.0 - p.ball_x, p.plan_tower.1 - p.ball_z);
        // Less practiced bots shove from a wider angle; skilled bots wait to line up.
        let required_alignment = (-1.9 + p.skill * 3.5).min(0.85);
        let aligned = shot_x * goal_x + shot_z * goal_z > required_alignment;
        let fire = near && aligned && !p.held_fire && self.time - p.last_shove >= 0.85;
        Input {
            stick: Stick { x, z },
            fire,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"vx":p.vx,"vz":p.vz,"yaw":p.yaw,
                "points":p.points,"blocks":p.blocks,"collapses":p.collapses,"left":p.left,
                "ball":{"x":p.ball_x,"z":p.ball_z,"vx":p.ball_vx,"vz":p.ball_vz}})).collect::<Vec<_>>(),
            "towers":self.towers.iter().enumerate().map(|(id,t)|json!({"id":id,"health":t.health,"rebuild":t.rebuild})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> StackbreakYard {
        StackbreakYard::new(&[Contestant::person(0), Contestant::person(1)], 7)
    }
    #[test]
    fn shove_moves_only_your_ball_and_spam_does_not_retrigger() {
        let mut g = game();
        let mut events = Vec::new();
        g.players[0].x = -1.8;
        g.players[0].z = 0.;
        g.players[0].ball_x = -0.5;
        g.players[0].ball_z = 0.;
        let other = g.players[1].ball_x;
        for _ in 0..30 {
            g.step(
                &[
                    (
                        0,
                        Input {
                            fire: true,
                            ..Default::default()
                        },
                    ),
                    (1, Input::default()),
                ],
                1. / 60.,
                &mut events,
            );
        }
        assert_eq!(
            events.iter().filter(|e| e["kind"] == "stack_shove").count(),
            1
        );
        assert_ne!(g.players[0].ball_x, -0.5);
        assert_eq!(g.players[1].ball_x, other);
    }
    #[test]
    fn tower_loses_blocks_collapses_and_rebuilds() {
        let mut g = game();
        let mut events = Vec::new();
        let (x, z) = TOWER_POS[0];
        g.players[0].ball_x = x - 0.85;
        g.players[0].ball_z = z;
        g.players[0].ball_vx = 5.;
        g.step(
            &[(0, Input::default()), (1, Input::default())],
            1. / 60.,
            &mut events,
        );
        assert_eq!(g.towers[0].health, 2);
        assert_eq!(g.players[0].points, 1);
        for _ in 0..2 {
            g.towers[0].cool = 0.;
            g.players[0].ball_x = x - 0.85;
            g.players[0].ball_z = z;
            g.players[0].ball_vx = 5.;
            g.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut events,
            );
        }
        assert_eq!(g.towers[0].health, 0);
        assert_eq!(g.players[0].collapses, 1);
        for _ in 0..230 {
            g.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut events,
            );
        }
        assert_eq!(g.towers[0].health, 3);
    }
    #[test]
    fn invalid_input_and_leaver_are_bounded() {
        let mut g = game();
        let mut events = Vec::new();
        for _ in 0..250 {
            g.step(
                &[(
                    0,
                    Input::from_stick(Stick {
                        x: f32::NAN,
                        z: f32::INFINITY,
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
    fn bots_finish_with_tower_hits() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.6))
            .collect::<Vec<_>>();
        let mut g = StackbreakYard::new(&seats, 3);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().map(|p| p.blocks).sum::<u32>() > 3);
    }

    #[test]
    fn skilled_bot_lines_up_behind_ball_before_shoving() {
        let mut g = StackbreakYard::new(&[Contestant::bot(0, 1.)], 7);
        let p = &mut g.players[0];
        p.ball_x = 0.;
        p.ball_z = 0.;
        p.plan_tower = (-3.5, 0.);
        p.plan_at = 100.;
        p.x = -1.4;
        p.z = 0.;
        assert!(
            !g.bot_input(0, 1. / 60.).fire,
            "do not shove away from the tower"
        );
        g.players[0].x = 1.4;
        assert!(g.bot_input(0, 1. / 60.).fire, "shove from behind the ball");
    }

    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.5))
            .collect::<Vec<_>>();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        let mut blocks = 0u32;
        for seed in 0..160 {
            let mut g = StackbreakYard::new(&seats, seed);
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
                scores[p.seat as usize] += p.points;
                blocks += p.blocks;
            }
        }
        println!(
            "Stackbreak Yard equal skill: first={first:?}, scores={scores:?}, blocks={blocks}"
        );
        assert!(blocks > 500);
        assert!(first.iter().max().unwrap() - first.iter().min().unwrap() <= 20);
        assert!(scores.iter().max().unwrap() - scores.iter().min().unwrap() <= 180);
    }
}
