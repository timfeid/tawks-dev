//! AI police: they patrol the road graph, answer pings, chase, call for
//! backup, block ramps, lay spikes ahead of a car and try to spin it out.

use rand::Rng;
use serde::Serialize;

use super::map::{Map, P, closest_on, dist};

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    Patrol,
    Respond { x: f64, z: f64 },
    Chase,
    Block { x: f64, z: f64, h: f64 },
}

#[derive(Clone, Debug, Serialize)]
pub struct Cop {
    pub id: String,
    pub x: f64,
    pub z: f64,
    pub h: f64,
    pub speed: f64,
    pub mode: Mode,
    pub siren: bool,
    #[serde(skip)]
    seg: Option<usize>,
    #[serde(skip)]
    toward: usize,
    #[serde(skip)]
    t: f64,
    #[serde(skip)]
    pub pit_ready: f64,
    #[serde(skip)]
    recent: Vec<usize>,
}

/// Where the cop is chasing: the car's spot, heading and speed.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub x: f64,
    pub z: f64,
    pub h: f64,
    pub speed: f64,
}

/// Off the graph and straight at the car when this close.
pub const DIRECT: f64 = 55.0;
pub const SIGHT: f64 = 90.0;
pub const CONE: f64 = 1.2;

fn heading(dx: f64, dz: f64) -> f64 {
    dx.atan2(-dz)
}

fn wrap(a: f64) -> f64 {
    a.sin().atan2(a.cos())
}

impl Cop {
    pub fn new(id: String, map: &Map, at: P) -> Cop {
        let mut cop = Cop {
            id,
            x: at.0,
            z: at.1,
            h: 0.0,
            speed: 0.0,
            mode: Mode::Patrol,
            siren: false,
            seg: None,
            toward: 0,
            t: 0.0,
            pit_ready: 0.0,
            recent: Vec::new(),
        };
        cop.snap(map);
        cop
    }

    /// Puts the cop back on the nearest road.
    pub fn snap(&mut self, map: &Map) {
        if let Some((seg, q, _)) = map.nearest_road((self.x, self.z), 400.0) {
            let s = &map.segs[seg];
            let (_, t) = closest_on(q, s.a, s.b);
            self.seg = Some(seg);
            self.toward = s.to;
            self.t = t;
            self.x = q.0;
            self.z = q.1;
        }
    }

    pub fn on_graph(&self) -> bool {
        self.seg.is_some()
    }

    fn cruise(&self, map: &Map) -> f64 {
        let limit = self.seg.map(|s| map.segs[s].class.limit()).unwrap_or(13.0);
        match self.mode {
            Mode::Patrol => limit * 0.85,
            Mode::Respond { .. } => (limit * 1.35).min(34.0),
            Mode::Chase => (limit * 1.7).clamp(24.0, 40.0),
            Mode::Block { .. } => 0.0,
        }
    }

    fn goal(&self, target: Option<Target>) -> Option<P> {
        match self.mode {
            Mode::Respond { x, z } | Mode::Block { x, z, .. } => Some((x, z)),
            Mode::Chase => target.map(|t| (t.x, t.z)),
            Mode::Patrol => None,
        }
    }

    /// Picks the next road at a junction: toward the goal, or a wander that favours bigger roads.
    fn next_seg(&mut self, map: &Map, node: usize, goal: Option<P>, rng: &mut impl Rng) -> Option<usize> {
        let options: Vec<usize> = map.edges[node]
            .iter()
            .copied()
            .filter(|s| Some(*s) != self.seg)
            .collect();
        if options.is_empty() {
            return self.seg;
        }
        let pick = match goal {
            Some(g) => {
                let scored: Vec<(usize, f64)> = options
                    .iter()
                    .map(|&s| {
                        let far = map.nodes[map.other_end(s, node)];
                        let penalty = if self.recent.contains(&s) { 120.0 } else { 0.0 };
                        (s, dist(far, g) + penalty + rng.random_range(0.0..8.0))
                    })
                    .collect();
                scored.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0
            }
            None => {
                let weights: Vec<f64> = options
                    .iter()
                    .map(|s| {
                        let base = if map.segs[*s].class.major() { 3.0 } else { 1.0 };
                        if self.recent.contains(s) { base * 0.2 } else { base }
                    })
                    .collect();
                let total: f64 = weights.iter().sum();
                let mut roll = rng.random_range(0.0..total);
                let mut chosen = options[0];
                for (s, w) in options.iter().zip(weights) {
                    if roll < w {
                        chosen = *s;
                        break;
                    }
                    roll -= w;
                }
                chosen
            }
        };
        self.recent.push(pick);
        if self.recent.len() > 6 {
            self.recent.remove(0);
        }
        Some(pick)
    }

    pub fn step(&mut self, map: &Map, dt: f64, target: Option<Target>, rng: &mut impl Rng) {
        if let Mode::Block { x, z, h } = self.mode {
            self.speed = 0.0;
            let d = dist((self.x, self.z), (x, z));
            if d > 2.0 {
                let step = (25.0 * dt).min(d);
                self.x += (x - self.x) / d * step;
                self.z += (z - self.z) / d * step;
                self.h = heading(x - self.x, z - self.z);
            } else {
                self.h = h;
            }
            return;
        }
        if let (Mode::Chase, Some(t)) = (self.mode, target) {
            let d = dist((self.x, self.z), (t.x, t.z));
            if d < DIRECT {
                // Close in: aim a little ahead of the car, off the road graph if need be.
                let lead = (d / 30.0).min(1.0) * 0.6;
                let aim = (t.x + t.h.sin() * t.speed * lead, t.z - t.h.cos() * t.speed * lead);
                let want = (t.speed + 4.0).clamp(6.0, 40.0);
                self.speed += (want - self.speed).clamp(-14.0 * dt, 9.0 * dt);
                let dh = wrap(heading(aim.0 - self.x, aim.1 - self.z) - self.h);
                self.h = wrap(self.h + dh.clamp(-2.2 * dt, 2.2 * dt));
                self.x += self.h.sin() * self.speed * dt;
                self.z -= self.h.cos() * self.speed * dt;
                self.seg = None;
                return;
            }
        }
        if self.seg.is_none() {
            self.snap(map);
        }
        let Some(mut seg) = self.seg else { return };
        let want = self.cruise(map);
        self.speed += (want - self.speed).clamp(-12.0 * dt, 6.0 * dt);
        let mut travel = self.speed * dt;
        // Head for the road point nearest the goal, since that is where a car can get.
        let goal = self
            .goal(target)
            .map(|g| map.nearest_road(g, 250.0).map(|(_, q, _)| q).unwrap_or(g));
        for _ in 0..8 {
            let s = &map.segs[seg];
            let len = dist(s.a, s.b).max(0.01);
            let forward = self.toward == s.to;
            let left = (1.0 - self.t) * len;
            if travel < left {
                self.t += travel / len;
                break;
            }
            travel -= left;
            let node = self.toward;
            match self.next_seg(map, node, goal, rng) {
                Some(next) => {
                    seg = next;
                    self.seg = Some(next);
                    self.toward = map.other_end(next, node);
                    self.t = 0.0;
                }
                None => break,
            }
            let _ = forward;
        }
        let s = &map.segs[seg];
        let (from, to) = if self.toward == s.to { (s.a, s.b) } else { (s.b, s.a) };
        let (dx, dz) = (to.0 - from.0, to.1 - from.1);
        let len = dx.hypot(dz).max(0.01);
        let lane = s.half * 0.45;
        self.x = from.0 + dx * self.t - dz / len * lane;
        self.z = from.1 + dz * self.t + dx / len * lane;
        self.h = heading(dx, dz);
        if let (Mode::Respond { .. }, Some(g)) = (self.mode, goal)
            && dist((self.x, self.z), g) < 40.0
        {
            self.mode = Mode::Patrol;
        }
    }

    /// Whether this cop can see a point: in front, close enough, nothing in the way.
    pub fn sees(&self, map: &Map, p: P, range: f64, cone: f64) -> bool {
        let d = dist((self.x, self.z), p);
        if d > range {
            return false;
        }
        let off = wrap(heading(p.0 - self.x, p.1 - self.z) - self.h).abs();
        (d < 12.0 || off <= cone) && map.line_of_sight((self.x, self.z), p)
    }

    /// Whether this cop is beside the car's rear quarter, matching its speed: the spot a spin-out starts from.
    pub fn pit_position(&self, t: Target) -> Option<f64> {
        let (dx, dz) = (self.x - t.x, self.z - t.z);
        let (fx, fz) = (t.h.sin(), -t.h.cos());
        let along = dx * fx + dz * fz;
        // Positive to the car's right: right is (cos h, sin h).
        let side = dx * t.h.cos() + dz * t.h.sin();
        let ok = t.speed > 10.0
            && self.speed >= t.speed - 1.0
            && (-3.2..=0.6).contains(&along)
            && (1.2..=3.4).contains(&side.abs());
        ok.then_some(side.signum())
    }
}

/// A spike strip across the road ahead of a car, if there is road there.
pub fn spike_ahead(map: &Map, t: Target, ahead: f64) -> Option<(P, P)> {
    let p = (t.x + t.h.sin() * ahead, t.z - t.h.cos() * ahead);
    let (seg, q, _) = map.nearest_road(p, 45.0)?;
    let s = &map.segs[seg];
    let (dx, dz) = (s.b.0 - s.a.0, s.b.1 - s.a.1);
    let len = dx.hypot(dz).max(0.01);
    let w = s.half + 1.0;
    Some((
        (q.0 - dz / len * w, q.1 + dx / len * w),
        (q.0 + dz / len * w, q.1 - dx / len * w),
    ))
}

/// The nearest motorway ramp ahead of a car, within `range`.
pub fn ramp_ahead(map: &Map, t: Target, range: f64) -> Option<(P, f64)> {
    let (fx, fz) = (t.h.sin(), -t.h.cos());
    map.segs
        .iter()
        .filter(|s| s.ramp)
        .map(|s| ((s.a.0 + s.b.0) / 2.0, (s.a.1 + s.b.1) / 2.0, s))
        .filter(|(x, z, _)| {
            let (dx, dz) = (x - t.x, z - t.z);
            let d = dx.hypot(dz);
            d > 120.0 && d < range && (dx * fx + dz * fz) / d > 0.6
        })
        .min_by(|a, b| dist((a.0, a.1), (t.x, t.z)).total_cmp(&dist((b.0, b.1), (t.x, t.z))))
        .map(|(x, z, s)| {
            (
                (x, z),
                heading(s.b.0 - s.a.0, s.b.1 - s.a.1) + std::f64::consts::FRAC_PI_2,
            )
        })
}

/// Whether a car moving from `a` to `b` this step ran over a strip.
pub fn crossed(a: P, b: P, strip: (P, P)) -> bool {
    let d = |p: P, q: P, r: P| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (q1, q2) = strip;
    (d(q1, q2, a) > 0.0) != (d(q1, q2, b) > 0.0) && (d(a, b, q1) > 0.0) != (d(a, b, q2) > 0.0)
}

#[cfg(test)]
mod tests {
    use super::super::geo::{Frame, lat_lon_of_tile};
    use super::super::mvt::{
        decode,
        testing::{V, tile},
    };
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// A grid of streets every 512 tile units, with a building block in one square.
    fn grid() -> (Frame, Map) {
        let (tx, ty) = (4823, 6160);
        let frame = Frame::new(lat_lon_of_tile(tx as f64, ty as f64, 14));
        let mut roads = Vec::new();
        for i in 0..=8 {
            let c = (i * 512) as i64;
            let along: Vec<i64> = (0..=8).flat_map(|j| [c, j * 512]).collect();
            let across: Vec<i64> = (0..=8).flat_map(|j| [j * 512, c]).collect();
            roads.push((2, vec![("class", V::S("minor"))], vec![along]));
            roads.push((2, vec![("class", V::S("minor"))], vec![across]));
        }
        let buf = tile(&[("transportation", roads)]);
        let mut map = Map::default();
        map.add_tile(&frame, tx, ty, &decode(&buf, &[]));
        (frame, map)
    }

    #[test]
    fn a_patrol_keeps_to_the_roads() {
        let (_, map) = grid();
        let mut cop = Cop::new("ai-1".into(), &map, (400.0, 400.0));
        let mut rng = StdRng::seed_from_u64(2);
        for _ in 0..600 {
            cop.step(&map, 0.1, None, &mut rng);
            let (_, _, d) = map.nearest_road((cop.x, cop.z), 50.0).unwrap();
            assert!(d < 4.0, "{d}");
        }
        assert!(cop.speed > 5.0);
    }

    #[test]
    fn a_responding_cop_gets_to_the_ping_and_a_chasing_one_closes_in() {
        let (_, map) = grid();
        let mut cop = Cop::new("ai-1".into(), &map, (50.0, 50.0));
        let goal = (1500.0, 1300.0);
        cop.mode = Mode::Respond { x: goal.0, z: goal.1 };
        let mut rng = StdRng::seed_from_u64(4);
        let mut arrived = false;
        for _ in 0..3000 {
            cop.step(&map, 0.1, None, &mut rng);
            if cop.mode == Mode::Patrol {
                arrived = true;
                break;
            }
        }
        assert!(arrived, "stuck at {} {}", cop.x, cop.z);
        cop.mode = Mode::Chase;
        let target = Target {
            x: cop.x + 300.0,
            z: cop.z,
            h: 0.0,
            speed: 0.0,
        };
        for _ in 0..600 {
            cop.step(&map, 0.1, Some(target), &mut rng);
        }
        assert!(dist((cop.x, cop.z), (target.x, target.z)) < 10.0);
    }

    #[test]
    fn spikes_go_across_the_road_ahead_and_catch_a_car_driving_over() {
        let (frame, map) = grid();
        let start = frame.from_tile(4823, 6160, 4096.0, 1024.0, 2048.0);
        let t = Target {
            x: start.0,
            z: start.1,
            h: std::f64::consts::FRAC_PI_2,
            speed: 20.0,
        };
        let strip = spike_ahead(&map, t, 300.0).unwrap();
        let mid = ((strip.0.0 + strip.1.0) / 2.0, (strip.0.1 + strip.1.1) / 2.0);
        assert!((mid.0 - (start.0 + 300.0)).abs() < 50.0 && (mid.1 - start.1).abs() < 1.0);
        assert!(crossed((mid.0 - 2.0, mid.1), (mid.0 + 2.0, mid.1), strip));
        assert!(!crossed(
            (mid.0 - 2.0, mid.1 + 30.0),
            (mid.0 + 2.0, mid.1 + 30.0),
            strip
        ));
    }

    #[test]
    fn sight_needs_the_cone_and_a_pit_needs_the_rear_quarter() {
        let (_, map) = grid();
        let mut cop = Cop::new("ai-1".into(), &map, (0.0, 0.0));
        cop.x = 0.0;
        cop.z = 0.0;
        cop.h = std::f64::consts::FRAC_PI_2;
        assert!(cop.sees(&map, (60.0, 0.0), SIGHT, CONE));
        assert!(!cop.sees(&map, (-60.0, 0.0), SIGHT, CONE), "behind");
        assert!(!cop.sees(&map, (200.0, 0.0), SIGHT, CONE), "too far");
        let car = Target {
            x: 0.0,
            z: 0.0,
            h: 0.0,
            speed: 20.0,
        };
        cop.speed = 21.0;
        cop.x = 2.2;
        cop.z = 1.5;
        assert_eq!(cop.pit_position(car), Some(1.0));
        cop.z = -6.0;
        assert_eq!(cop.pit_position(car), None, "ahead of the car");
    }
}
