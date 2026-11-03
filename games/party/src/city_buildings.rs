//! The city's solid geometry: every town building is a solid block, and the
//! heist buildings are shells of walls with doors and rooms inside. The
//! server walks, drives, routes and sees against this; the native client
//! draws the same shapes.
use serde::Serialize;
use std::sync::OnceLock;

/// How thick a wall is, in metres.
pub const WALL: f32 = 0.4;
/// How far a person keeps from a wall.
pub const BODY: f32 = 0.35;
/// How far a car's centre keeps from a building.
pub const CAR: f32 = 1.3;
pub const WALL_HEIGHT: f32 = 3.4;
const STREETS: [f32; 6] = [-64., -40., -16., 16., 40., 64.];
/// Town buildings stop short of the road they were placed beside.
const CURB: f32 = 2.6;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}
impl Rect {
    pub const fn new(x0: f32, z0: f32, x1: f32, z1: f32) -> Self {
        Self {
            min: [x0, z0],
            max: [x1, z1],
        }
    }
    pub fn contains(&self, at: (f32, f32), margin: f32) -> bool {
        at.0 > self.min[0] - margin
            && at.0 < self.max[0] + margin
            && at.1 > self.min[1] - margin
            && at.1 < self.max[1] + margin
    }
    pub fn center(&self) -> (f32, f32) {
        (
            (self.min[0] + self.max[0]) / 2.,
            (self.min[1] + self.max[1]) / 2.,
        )
    }
    /// Whether the segment from `a` to `b` passes through this rectangle
    /// grown by `margin`.
    pub fn crosses(&self, a: (f32, f32), b: (f32, f32), margin: f32) -> bool {
        let (mut t0, mut t1) = (0f32, 1f32);
        let d = (b.0 - a.0, b.1 - a.1);
        for (start, delta, lo, hi) in [
            (a.0, d.0, self.min[0] - margin, self.max[0] + margin),
            (a.1, d.1, self.min[1] - margin, self.max[1] + margin),
        ] {
            if delta.abs() < 1e-6 {
                if start <= lo || start >= hi {
                    return false;
                }
                continue;
            }
            let (mut near, mut far) = ((lo - start) / delta, (hi - start) / delta);
            if near > far {
                std::mem::swap(&mut near, &mut far);
            }
            t0 = t0.max(near);
            t1 = t1.min(far);
            if t0 >= t1 {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Bank,
    Power,
    Jail,
    Garage,
}

/// A wall line with openings. Bars block bodies but not sight.
struct Line {
    from: (f32, f32),
    to: (f32, f32),
    doors: &'static [(f32, f32)],
    bars: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Room {
    pub id: &'static str,
    pub area: Rect,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Door {
    /// Centre of the opening.
    pub at: [f32; 2],
    /// The wall runs along x (true) or along z.
    pub along_x: bool,
    pub width: f32,
    /// Leads outside rather than between two rooms.
    pub outer: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Solid {
    pub area: Rect,
    pub bars: bool,
}

/// A solid piece of furniture: it blocks bodies but not sight.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Furniture {
    pub area: Rect,
    pub height: f32,
    pub look: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Building {
    pub kind: Kind,
    pub area: Rect,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    pub walls: Vec<Solid>,
    pub furniture: Vec<Furniture>,
}

const DOOR: f32 = 2.4;

/// Wall lines of a heist building: its outline with the outer doors, then
/// any inner partitions.
fn shell(
    area: Rect,
    north: &'static [(f32, f32)],
    south: &'static [(f32, f32)],
    east: &'static [(f32, f32)],
    west: &'static [(f32, f32)],
) -> Vec<Line> {
    let [x0, z0] = area.min;
    let [x1, z1] = area.max;
    vec![
        Line {
            from: (x0, z1),
            to: (x1, z1),
            doors: north,
            bars: false,
        },
        Line {
            from: (x0, z0),
            to: (x1, z0),
            doors: south,
            bars: false,
        },
        Line {
            from: (x1, z0),
            to: (x1, z1),
            doors: east,
            bars: false,
        },
        Line {
            from: (x0, z0),
            to: (x0, z1),
            doors: west,
            bars: false,
        },
    ]
}

const fn item(x0: f32, z0: f32, x1: f32, z1: f32, height: f32, look: &'static str) -> Furniture {
    Furniture {
        area: Rect::new(x0, z0, x1, z1),
        height,
        look,
    }
}

fn build(
    kind: Kind,
    area: Rect,
    mut lines: Vec<Line>,
    inner: Vec<Line>,
    rooms: Vec<Room>,
    furniture: Vec<Furniture>,
) -> Building {
    let outer = lines.len();
    lines.extend(inner);
    let mut walls = Vec::new();
    let mut doors = Vec::new();
    let half = WALL / 2.;
    for (index, line) in lines.iter().enumerate() {
        let along_x = (line.to.1 - line.from.1).abs() < 1e-3;
        let (start, end, fixed) = if along_x {
            (
                line.from.0.min(line.to.0),
                line.from.0.max(line.to.0),
                line.from.1,
            )
        } else {
            (
                line.from.1.min(line.to.1),
                line.from.1.max(line.to.1),
                line.from.0,
            )
        };
        let mut cuts: Vec<(f32, f32)> = line
            .doors
            .iter()
            .map(|&(center, width)| (center - width / 2., center + width / 2.))
            .collect();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(center, width) in line.doors {
            doors.push(Door {
                at: if along_x {
                    [center, fixed]
                } else {
                    [fixed, center]
                },
                along_x,
                width,
                outer: index < outer,
            });
        }
        let mut cursor = start - half;
        for (lo, hi) in cuts.into_iter().chain([(end + half, end + half)]) {
            if lo > cursor + 1e-3 {
                let area = if along_x {
                    Rect::new(cursor, fixed - half, lo, fixed + half)
                } else {
                    Rect::new(fixed - half, cursor, fixed + half, lo)
                };
                walls.push(Solid {
                    area,
                    bars: line.bars,
                });
            }
            cursor = hi;
        }
    }
    Building {
        kind,
        area,
        rooms,
        doors,
        walls,
        furniture,
    }
}

pub const BANK_AREA: Rect = Rect::new(-3., 25., 11., 35.);
pub const POWER_AREA: Rect = Rect::new(-12.6, 27., -5.4, 35.);
pub const JAIL_AREA: Rect = Rect::new(25.6, -36., 31.4, -24.8);
pub const GARAGE_AREA: Rect = Rect::new(-31.2, -36., -25.9, -24.8);

/// The buildings you can walk into.
pub fn heist_buildings() -> &'static [Building] {
    static B: OnceLock<Vec<Building>> = OnceLock::new();
    B.get_or_init(|| {
        vec![
            build(
                Kind::Bank,
                BANK_AREA,
                shell(BANK_AREA, &[(1., DOOR)], &[], &[], &[]),
                vec![Line {
                    from: (5., 25.),
                    to: (5., 35.),
                    doors: &[(30., 2.)],
                    bars: false,
                }],
                vec![
                    Room {
                        id: "bank",
                        area: Rect::new(-3., 25., 5., 35.),
                    },
                    Room {
                        id: "vault",
                        area: Rect::new(5., 25., 11., 35.),
                    },
                ],
                vec![
                    item(-0.9, 26.5, -0.1, 31.5, 1.1, "counter"),
                    item(9.85, 26.2, 10.75, 27.8, 2.2, "safe"),
                    item(9.85, 32.2, 10.75, 33.8, 2.2, "safe"),
                    item(7.3, 25.5, 8.7, 26.4, 1., "table"),
                ],
            ),
            build(
                Kind::Power,
                POWER_AREA,
                shell(POWER_AREA, &[], &[(-9., DOOR)], &[], &[]),
                vec![],
                vec![Room {
                    id: "power",
                    area: POWER_AREA,
                }],
                vec![
                    item(-10.2, 34.15, -7.8, 34.75, 2.6, "panel"),
                    item(-12.2, 29.4, -11., 31.6, 2., "machine"),
                    item(-7., 29.4, -5.8, 31.6, 2., "machine"),
                ],
            ),
            build(
                Kind::Jail,
                JAIL_AREA,
                shell(JAIL_AREA, &[(28., DOOR)], &[], &[], &[]),
                vec![Line {
                    from: (25.6, -31.),
                    to: (31.4, -31.),
                    doors: &[],
                    bars: true,
                }],
                vec![
                    Room {
                        id: "jail",
                        area: Rect::new(25.6, -31., 31.4, -24.8),
                    },
                    Room {
                        id: "cell",
                        area: Rect::new(25.6, -36., 31.4, -31.),
                    },
                ],
                vec![
                    item(29.9, -28.7, 30.9, -26.3, 1.1, "desk"),
                    item(26.05, -28.3, 26.75, -25.7, 0.5, "bench"),
                ],
            ),
            build(
                Kind::Garage,
                GARAGE_AREA,
                shell(GARAGE_AREA, &[(-28., DOOR)], &[(-28., 3.)], &[], &[]),
                vec![],
                vec![Room {
                    id: "garage",
                    area: GARAGE_AREA,
                }],
                vec![item(-30.9, -26.9, -30.3, -25.1, 1.8, "shelf")],
            ),
        ]
    })
}

/// Half sizes of the town building models at scale one.
fn model_half(model: &str) -> Option<(f32, f32)> {
    Some(match model.strip_prefix("town-building-")? {
        "a" => (0.44, 0.47),
        "b" => (0.49, 0.47),
        "c" => (0.44, 0.55),
        "d" => (0.42, 0.45),
        "e" => (0.82, 0.5),
        "f" => (0.42, 0.52),
        "g" => (0.49, 0.46),
        "h" => (0.44, 0.5),
        "i" => (0.62, 0.65),
        "j" => (1.04, 0.67),
        "k" => (1.04, 0.47),
        "l" => (0.69, 0.7),
        "m" => (0.62, 0.62),
        "n" => (1.16, 0.91),
        "skyscraper-a" | "skyscraper-b" => (0.68, 0.68),
        "skyscraper-c" | "skyscraper-d" => (0.64, 0.69),
        "skyscraper-e" => (0.65, 0.62),
        _ => (0.5, 0.5),
    })
}

/// Pulls a building's edge back off any street it overhangs.
fn off_streets(mut r: Rect) -> Rect {
    for axis in 0..2 {
        let center = (r.min[axis] + r.max[axis]) / 2.;
        for s in STREETS {
            if r.min[axis] < s + CURB && r.max[axis] > s - CURB {
                if center >= s {
                    r.min[axis] = r.min[axis].max(s + CURB);
                } else {
                    r.max[axis] = r.max[axis].min(s - CURB);
                }
            }
        }
    }
    r
}

/// Every plain town building's footprint.
pub fn blocks() -> &'static [Rect] {
    static B: OnceLock<Vec<Rect>> = OnceLock::new();
    B.get_or_init(|| {
        let map: serde_json::Value = serde_json::from_str(include_str!(
            "../../canvases/src/party/board/maps/frozen.json"
        ))
        .expect("city layout");
        map["props"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| {
                let (hx, hz) = model_half(p["model"].as_str()?)?;
                let x = p["x"].as_f64()? as f32;
                let z = p["z"].as_f64()? as f32;
                let scale = p["scale"].as_f64().unwrap_or(1.) as f32;
                let quarter = p["yaw"].as_f64().unwrap_or(0.).sin().abs() > 0.5;
                let (hx, hz) = if quarter { (hz, hx) } else { (hx, hz) };
                Some(off_streets(Rect::new(
                    x - hx * scale,
                    z - hz * scale,
                    x + hx * scale,
                    z + hz * scale,
                )))
            })
            .collect()
    })
}

fn walls() -> impl Iterator<Item = &'static Solid> {
    heist_buildings().iter().flat_map(|b| b.walls.iter())
}
fn furniture() -> impl Iterator<Item = &'static Rect> {
    heist_buildings()
        .iter()
        .flat_map(|b| b.furniture.iter().map(|f| &f.area))
}

/// Whether a person can stand here.
pub fn open_for_body(at: (f32, f32)) -> bool {
    blocks().iter().all(|r| !r.contains(at, BODY))
        && walls().all(|w| !w.area.contains(at, BODY))
        && furniture().all(|f| !f.contains(at, BODY))
}

/// Whether a car can be here: never inside or against any building.
pub fn open_for_car(at: (f32, f32)) -> bool {
    blocks().iter().all(|r| !r.contains(at, CAR))
        && heist_buildings().iter().all(|b| !b.area.contains(at, CAR))
}

/// Whether a wall or building stands between two points.
pub fn sight_blocked(a: (f32, f32), b: (f32, f32)) -> bool {
    blocks().iter().any(|r| r.crosses(a, b, 0.1))
        || walls().any(|w| !w.bars && w.area.crosses(a, b, 0.))
}

/// Whether a body moving from `a` to `b` would pass through a wall.
pub fn body_blocked(a: (f32, f32), b: (f32, f32)) -> bool {
    blocks().iter().any(|r| r.crosses(a, b, BODY * 0.5))
        || walls().any(|w| w.area.crosses(a, b, BODY * 0.5))
        || furniture().any(|f| f.crosses(a, b, BODY * 0.5))
}

/// Whether a person could walk straight from `a` to `b`, keeping clear of
/// every wall and corner.
pub fn clear_walk(a: (f32, f32), b: (f32, f32)) -> bool {
    !blocks().iter().any(|r| r.crosses(a, b, BODY + 0.05))
        && !walls().any(|w| w.area.crosses(a, b, BODY + 0.05))
        && !furniture().any(|f| f.crosses(a, b, BODY + 0.05))
}

/// Which room this point is in, if any.
pub fn room_at(at: (f32, f32)) -> Option<&'static str> {
    heist_buildings()
        .iter()
        .flat_map(|b| b.rooms.iter())
        .find(|r| r.area.contains(at, 0.01))
        .map(|r| r.id)
}

/// Close enough to a spot to use it, and on the same side of every wall.
pub fn reach(at: (f32, f32), spot: (f32, f32), range: f32) -> bool {
    (at.0 - spot.0).hypot(at.1 - spot.1) < range && room_at(at) == room_at(spot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walls_are_solid_and_doors_are_open() {
        assert!(!open_for_body((-1., 35.)), "the bank's front wall");
        assert!(open_for_body((1., 35.)), "the bank's front door");
        assert!(open_for_body((5., 30.)), "the vault door");
        assert!(!open_for_body((5., 27.)), "beside the vault door");
        assert!(!open_for_body((28., -31.)), "the cell bars");
        assert!(open_for_body((28., -24.8)), "the jail door");
        assert!(open_for_body((-28., -36.)), "the garage's back door");
    }
    #[test]
    fn cars_stay_outside() {
        for b in heist_buildings() {
            for d in &b.doors {
                assert!(!open_for_car((d.at[0], d.at[1])));
            }
            assert!(!open_for_car(b.area.center()));
        }
        assert!(open_for_car((16., 30.)), "the street beside the bank");
    }
    #[test]
    fn bars_let_you_see_but_walls_do_not() {
        assert!(!sight_blocked((28., -29.), (28., -33.)), "through the bars");
        assert!(
            sight_blocked((0., 40.), (8., 30.)),
            "into the vault from the street"
        );
        assert!(
            !sight_blocked((1., 40.), (1., 30.)),
            "through the front door"
        );
    }
    #[test]
    fn rooms_and_reach() {
        assert_eq!(room_at((8., 30.)), Some("vault"));
        assert_eq!(room_at((0., 30.)), Some("bank"));
        assert_eq!(room_at((28., -33.)), Some("cell"));
        assert_eq!(room_at((0., 40.)), None);
        assert!(reach((7., 31.), (8., 30.), 2.3));
        assert!(!reach((4., 30.), (6., 30.), 2.3), "through the vault wall");
    }
    #[test]
    fn furniture_is_solid_but_leaves_every_station_and_door_clear() {
        assert!(!open_for_body((-0.5, 29.)), "the bank counter");
        assert!(
            !sight_blocked((-2., 29.), (1., 29.)),
            "you see over the counter"
        );
        for spot in [(8., 30.), (-9., 32.), (28., -29.), (-28.5, -30.)] {
            assert!(open_for_body(spot), "{spot:?}");
        }
        for b in heist_buildings() {
            for d in &b.doors {
                assert!(open_for_body((d.at[0], d.at[1])));
            }
        }
    }
    #[test]
    fn heist_buildings_clear_the_town() {
        for b in heist_buildings() {
            for r in blocks() {
                let apart = (0..2).any(|i| b.area.max[i] <= r.min[i] || b.area.min[i] >= r.max[i]);
                assert!(apart, "{:?} overlaps {:?}", b.kind, r);
            }
        }
    }
    #[test]
    fn buildings_stay_off_the_road() {
        for r in blocks() {
            for s in STREETS {
                for axis in 0..2 {
                    assert!(
                        r.max[axis] <= s - CURB + 1e-3 || r.min[axis] >= s + CURB - 1e-3,
                        "{r:?} on street {s}"
                    );
                }
            }
        }
    }
}
