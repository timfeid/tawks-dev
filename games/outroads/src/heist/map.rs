//! The map the heist is refereed on: roads as a graph for AI cops, building
//! walls for line of sight, street lamps for darkness, and the spots parked
//! cars and plate readers go. Built from the same vector tiles the clients
//! draw, so both sides agree on where things are.

use std::collections::{HashMap, HashSet};

use super::geo::Frame;
use super::mvt::{Feature, Layer, ring_area};

pub type P = (f64, f64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoadClass {
    Motorway,
    Trunk,
    Primary,
    Secondary,
    Tertiary,
    Minor,
    Service,
    Track,
}

impl RoadClass {
    pub fn parse(s: &str) -> Option<RoadClass> {
        Some(match s {
            "motorway" => RoadClass::Motorway,
            "trunk" => RoadClass::Trunk,
            "primary" => RoadClass::Primary,
            "secondary" => RoadClass::Secondary,
            "tertiary" => RoadClass::Tertiary,
            "minor" | "busway" | "raceway" => RoadClass::Minor,
            "service" => RoadClass::Service,
            "track" => RoadClass::Track,
            _ => return None,
        })
    }

    /// A speed limit by road class, meters per second. OpenMapTiles carries no posted limits.
    pub fn limit(self) -> f64 {
        match self {
            RoadClass::Motorway => 29.0,
            RoadClass::Trunk => 24.5,
            RoadClass::Primary => 20.0,
            RoadClass::Secondary => 18.0,
            RoadClass::Tertiary => 15.6,
            RoadClass::Minor => 11.2,
            RoadClass::Service | RoadClass::Track => 7.0,
        }
    }

    /// Half the drawn width, as the client draws it.
    pub fn half(self, ramp: bool) -> f64 {
        let w: f64 = match self {
            RoadClass::Motorway => 15.0,
            RoadClass::Trunk => 13.0,
            RoadClass::Primary => 11.5,
            RoadClass::Secondary => 10.0,
            RoadClass::Tertiary => 9.0,
            RoadClass::Minor => 7.0,
            RoadClass::Service => 4.5,
            RoadClass::Track => 3.5,
        };
        if ramp { w.min(7.5) / 2.0 } else { w / 2.0 }
    }

    fn lamps(self) -> bool {
        !matches!(self, RoadClass::Service | RoadClass::Track)
    }

    pub fn major(self) -> bool {
        matches!(self, RoadClass::Motorway | RoadClass::Trunk | RoadClass::Primary)
    }
}

#[derive(Clone, Debug)]
pub struct Seg {
    pub a: P,
    pub b: P,
    pub class: RoadClass,
    pub ramp: bool,
    pub half: f64,
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Debug)]
pub struct Named {
    pub a: P,
    pub b: P,
    pub name: String,
    pub refs: String,
}

#[derive(Clone, Debug)]
pub struct Building {
    pub centre: P,
    pub ring: Vec<P>,
    pub area: f64,
}

const CELL: f64 = 50.0;
const LAMP_EVERY: f64 = 38.0;

#[derive(Default)]
pub struct Map {
    pub segs: Vec<Seg>,
    pub named: Vec<Named>,
    pub buildings: Vec<Building>,
    pub lamps: Vec<P>,
    /// Graph nodes and, for each, the segments leaving it.
    pub nodes: Vec<P>,
    pub edges: Vec<Vec<usize>>,
    node_at: HashMap<(i64, i64), usize>,
    seg_grid: HashMap<(i32, i32), Vec<usize>>,
    wall_grid: HashMap<(i32, i32), Vec<(P, P)>>,
    building_grid: HashMap<(i32, i32), Vec<usize>>,
    lamp_grid: HashMap<(i32, i32), Vec<usize>>,
    pub tiles: HashSet<(u32, u32)>,
}

fn cell(p: P) -> (i32, i32) {
    ((p.0 / CELL).floor() as i32, (p.1 / CELL).floor() as i32)
}

fn cells_of(a: P, b: P) -> impl Iterator<Item = (i32, i32)> {
    let (ax, az) = cell((a.0.min(b.0), a.1.min(b.1)));
    let (bx, bz) = cell((a.0.max(b.0), a.1.max(b.1)));
    (ax..=bx).flat_map(move |x| (az..=bz).map(move |z| (x, z)))
}

pub fn closest_on(p: P, a: P, b: P) -> (P, f64) {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dz * dz;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((a.0 + dx * t, a.1 + dz * t), t)
}

pub fn dist(a: P, b: P) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn crosses(p1: P, p2: P, q1: P, q2: P) -> bool {
    let d = |a: P, b: P, c: P| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let (d1, d2) = (d(q1, q2, p1), d(q1, q2, p2));
    let (d3, d4) = (d(p1, p2, q1), d(p1, p2, q2));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

/// The parts of a line inside a box, as the client clips them.
fn clip(line: &[P], x0: f64, z0: f64, x1: f64, z1: f64) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    let mut current: Vec<P> = Vec::new();
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut inside = true;
        for (p, q) in [(-dx, a.0 - x0), (dx, x1 - a.0), (-dz, a.1 - z0), (dz, z1 - a.1)] {
            if p == 0.0 {
                if q < 0.0 {
                    inside = false;
                }
            } else {
                let r = q / p;
                if p < 0.0 { t0 = t0.max(r) } else { t1 = t1.min(r) }
            }
        }
        if !inside || t0 > t1 {
            if current.len() >= 2 {
                out.push(std::mem::take(&mut current));
            }
            current.clear();
            continue;
        }
        let pa = (a.0 + dx * t0, a.1 + dz * t0);
        let pb = (a.0 + dx * t1, a.1 + dz * t1);
        if current.is_empty() {
            current.push(pa);
        }
        current.push(pb);
        if t1 < 1.0 {
            out.push(std::mem::take(&mut current));
        }
    }
    if current.len() >= 2 {
        out.push(current);
    }
    out
}

impl Map {
    fn node(&mut self, p: P) -> usize {
        let key = ((p.0 * 2.0).round() as i64, (p.1 * 2.0).round() as i64);
        if let Some(&n) = self.node_at.get(&key) {
            return n;
        }
        self.nodes.push(p);
        self.edges.push(Vec::new());
        self.node_at.insert(key, self.nodes.len() - 1);
        self.nodes.len() - 1
    }

    fn add_seg(&mut self, a: P, b: P, class: RoadClass, ramp: bool) {
        if dist(a, b) < 0.05 {
            return;
        }
        let (from, to) = (self.node(a), self.node(b));
        let id = self.segs.len();
        self.segs.push(Seg {
            a,
            b,
            class,
            ramp,
            half: class.half(ramp),
            from,
            to,
        });
        self.edges[from].push(id);
        self.edges[to].push(id);
        for c in cells_of(a, b) {
            self.seg_grid.entry(c).or_default().push(id);
        }
    }

    fn line(frame: &Frame, tx: u32, ty: u32, extent: f64, part: &[f64]) -> Vec<P> {
        part.chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| frame.from_tile(tx, ty, extent, c[0], c[1]))
            .collect()
    }

    /// Adds one tile's roads, names, buildings and lamps, keeping only what lies inside the tile.
    pub fn add_tile(&mut self, frame: &Frame, tx: u32, ty: u32, layers: &HashMap<String, Layer>) {
        if !self.tiles.insert((tx, ty)) {
            return;
        }
        let (x0, z0) = frame.from_tile(tx, ty, 1.0, 0.0, 0.0);
        let size = frame.meters_per_tile;
        let (x1, z1) = (x0 + size, z0 + size);
        let inside = |p: P| p.0 >= x0 && p.1 >= z0 && p.0 < x1 && p.1 < z1;
        if let Some(l) = layers.get("transportation") {
            for f in &l.features {
                let Some(class) = RoadClass::parse(f.str("class")) else {
                    continue;
                };
                if f.kind != 2 || f.str("brunnel") == "tunnel" {
                    continue;
                }
                let ramp = f.num("ramp") == Some(1.0);
                for part in &f.parts {
                    let raw = Self::line(frame, tx, ty, l.extent, part);
                    if class.lamps() {
                        for p in lamps_along(&raw, class.half(ramp)) {
                            if inside(p) {
                                let id = self.lamps.len();
                                self.lamps.push(p);
                                self.lamp_grid.entry(cell(p)).or_default().push(id);
                            }
                        }
                    }
                    for piece in clip(&raw, x0, z0, x1, z1) {
                        for w in piece.windows(2) {
                            self.add_seg(w[0], w[1], class, ramp);
                        }
                    }
                }
            }
        }
        if let Some(l) = layers.get("transportation_name") {
            for f in &l.features {
                if f.kind != 2 {
                    continue;
                }
                for part in &f.parts {
                    for piece in clip(&Self::line(frame, tx, ty, l.extent, part), x0, z0, x1, z1) {
                        for w in piece.windows(2) {
                            self.named.push(Named {
                                a: w[0],
                                b: w[1],
                                name: f.str("name").to_string(),
                                refs: f.str("ref").to_string(),
                            });
                        }
                    }
                }
            }
        }
        if let Some(l) = layers.get("building") {
            for f in &l.features {
                self.add_buildings(frame, tx, ty, l, f, &inside);
            }
        }
    }

    fn add_buildings(&mut self, frame: &Frame, tx: u32, ty: u32, l: &Layer, f: &Feature, inside: &dyn Fn(P) -> bool) {
        if f.kind != 3 {
            return;
        }
        for part in &f.parts {
            if ring_area(part) <= 0.0 {
                continue;
            }
            let ring = Self::line(frame, tx, ty, l.extent, part);
            if ring.len() < 4 {
                continue;
            }
            let n = (ring.len() - 1) as f64;
            let centre = ring[..ring.len() - 1]
                .iter()
                .fold((0.0, 0.0), |acc, p| (acc.0 + p.0 / n, acc.1 + p.1 / n));
            if !inside(centre) {
                continue;
            }
            let mut area = 0.0;
            for w in ring.windows(2) {
                area += w[0].0 * w[1].1 - w[1].0 * w[0].1;
            }
            let id = self.buildings.len();
            for w in ring.windows(2) {
                for c in cells_of(w[0], w[1]) {
                    self.wall_grid.entry(c).or_default().push((w[0], w[1]));
                }
            }
            self.building_grid.entry(cell(centre)).or_default().push(id);
            self.buildings.push(Building {
                centre,
                ring,
                area: area.abs() / 2.0,
            });
        }
    }

    /// The nearest road within `max` meters: its segment and the closest point on it.
    pub fn nearest_road(&self, p: P, max: f64) -> Option<(usize, P, f64)> {
        let mut best: Option<(usize, P, f64)> = None;
        let r = (max / CELL).ceil() as i32;
        let (cx, cz) = cell(p);
        for x in cx - r..=cx + r {
            for z in cz - r..=cz + r {
                for &id in self.seg_grid.get(&(x, z)).into_iter().flatten() {
                    let s = &self.segs[id];
                    let (q, _) = closest_on(p, s.a, s.b);
                    let d = dist(p, q);
                    if d <= max && best.is_none_or(|b| d < b.2) {
                        best = Some((id, q, d));
                    }
                }
            }
        }
        best
    }

    /// Whether a straight line between two points passes no building wall.
    pub fn line_of_sight(&self, a: P, b: P) -> bool {
        let steps = (dist(a, b) / (CELL * 0.5)).ceil().max(1.0) as i32;
        let mut seen: HashSet<(i32, i32)> = HashSet::new();
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let c = cell((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let k = (c.0 + dx, c.1 + dz);
                    if !seen.insert(k) {
                        continue;
                    }
                    for (q1, q2) in self.wall_grid.get(&k).into_iter().flatten() {
                        if crosses(a, b, *q1, *q2) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Meters to the nearest street lamp, up to `max`.
    pub fn lamp_distance(&self, p: P, max: f64) -> f64 {
        let mut best = max;
        let r = (max / CELL).ceil() as i32;
        let (cx, cz) = cell(p);
        for x in cx - r..=cx + r {
            for z in cz - r..=cz + r {
                for &id in self.lamp_grid.get(&(x, z)).into_iter().flatten() {
                    best = best.min(dist(p, self.lamps[id]));
                }
            }
        }
        best
    }

    /// Buildings with their middle within `r` of a point.
    pub fn buildings_near(&self, p: P, r: f64) -> Vec<usize> {
        let mut out = Vec::new();
        let k = (r / CELL).ceil() as i32;
        let (cx, cz) = cell(p);
        for x in cx - k..=cx + k {
            for z in cz - k..=cz + k {
                for &id in self.building_grid.get(&(x, z)).into_iter().flatten() {
                    if dist(p, self.buildings[id].centre) <= r {
                        out.push(id);
                    }
                }
            }
        }
        out
    }

    /// The other end of a segment from a node.
    pub fn other_end(&self, seg: usize, node: usize) -> usize {
        let s = &self.segs[seg];
        if s.from == node { s.to } else { s.from }
    }
}

/// Street lamp spots along a road, the way the client places them.
pub fn lamps_along(line: &[P], half: f64) -> Vec<P> {
    let mut out = Vec::new();
    let mut carry = LAMP_EVERY / 2.0;
    let mut side = 1.0;
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = dist(a, b);
        if len < 0.01 {
            continue;
        }
        let (dx, dz) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let mut t = carry;
        while t < len {
            let off = (half + 0.8) * side;
            out.push((a.0 + dx * t - dz * off, a.1 + dz * t + dx * off));
            side = -side;
            t += LAMP_EVERY;
        }
        carry = t - len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::geo::{Frame, LatLon, lat_lon_of_tile};
    use super::super::mvt::{
        decode,
        testing::{V, tile},
    };
    use super::*;

    pub fn sample() -> (Frame, Map) {
        let (tx, ty) = (4823, 6160);
        let frame = Frame::new(lat_lon_of_tile(tx as f64, ty as f64, 14));
        let buf = tile(&[
            (
                "transportation",
                vec![
                    (
                        2,
                        vec![("class", V::S("minor"))],
                        vec![vec![0, 2048, 2048, 2048, 4096, 2048]],
                    ),
                    (
                        2,
                        vec![("class", V::S("primary"))],
                        vec![vec![2048, 0, 2048, 2048, 2048, 4096]],
                    ),
                ],
            ),
            (
                "transportation_name",
                vec![(
                    2,
                    vec![("name", V::S("Main Street")), ("ref", V::S("US 1;US 9"))],
                    vec![vec![2048, 0, 2048, 4096]],
                )],
            ),
            (
                "building",
                vec![(
                    3,
                    vec![],
                    vec![vec![1000, 1900, 1100, 1900, 1100, 2000, 1000, 2000, 1000, 1900]],
                )],
            ),
        ]);
        let mut map = Map::default();
        map.add_tile(&frame, tx, ty, &decode(&buf, &[]));
        (frame, map)
    }

    #[test]
    fn roads_join_into_a_graph_with_a_crossing() {
        let (frame, map) = sample();
        assert!(!map.segs.is_empty());
        let centre = frame.from_tile(4823, 6160, 4096.0, 2048.0, 2048.0);
        let (_, q, d) = map.nearest_road(centre, 30.0).unwrap();
        assert!(d < 0.01 && dist(q, centre) < 0.01);
        let node = map.nodes.iter().position(|n| dist(*n, centre) < 1.0);
        assert!(
            node.is_some_and(|n| map.edges[n].len() == 4),
            "the crossing joins four ways"
        );
        assert!(!map.lamps.is_empty());
        assert_eq!(map.named[0].refs, "US 1;US 9");
        let _ = LatLon { lat: 0.0, lon: 0.0 };
    }

    #[test]
    fn walls_block_sight_and_lamps_light_the_street() {
        let (frame, map) = sample();
        let behind = frame.from_tile(4823, 6160, 4096.0, 1050.0, 1800.0);
        let front = frame.from_tile(4823, 6160, 4096.0, 1050.0, 2048.0);
        let beside = frame.from_tile(4823, 6160, 4096.0, 1400.0, 2048.0);
        assert!(!map.line_of_sight(behind, front));
        assert!(map.line_of_sight(beside, front));
        let on_road = frame.from_tile(4823, 6160, 4096.0, 300.0, 2048.0);
        assert!(map.lamp_distance(on_road, 100.0) < 30.0);
        assert_eq!(map.buildings_near(front, 60.0).len(), 1);
    }
}
