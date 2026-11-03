//! The museum floor: its walls, where the paintings hang, line of sight for
//! the flashlights and a walking grid for the bots.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub const HALF_X: f32 = 18.0;
pub const HALF_Z: f32 = 13.0;
/// Half the thickness of an inside wall.
pub const THICK: f32 = 0.3;
pub const BODY: f32 = 0.45;
const CELL: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wall {
    pub x: f32,
    pub z: f32,
    pub hx: f32,
    pub hz: f32,
}

impl Wall {
    fn contains(&self, x: f32, z: f32, pad: f32) -> bool {
        (x - self.x).abs() < self.hx + pad && (z - self.z).abs() < self.hz + pad
    }

    /// Whether the segment from a to b passes through this wall grown by `pad`.
    pub fn crosses(&self, a: (f32, f32), b: (f32, f32), pad: f32) -> bool {
        let (min_x, max_x) = (self.x - self.hx - pad, self.x + self.hx + pad);
        let (min_z, max_z) = (self.z - self.hz - pad, self.z + self.hz + pad);
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let mut t0 = 0.0f32;
        let mut t1 = 1.0f32;
        for (start, delta, lo, hi) in [(a.0, dx, min_x, max_x), (a.1, dz, min_z, max_z)] {
            if delta.abs() < 1e-6 {
                if start < lo || start > hi {
                    return false;
                }
            } else {
                let (mut near, mut far) = ((lo - start) / delta, (hi - start) / delta);
                if near > far {
                    std::mem::swap(&mut near, &mut far);
                }
                t0 = t0.max(near);
                t1 = t1.min(far);
                if t0 > t1 {
                    return false;
                }
            }
        }
        true
    }
}

/// A spot on a wall a painting can hang from, facing into the room.
#[derive(Clone, Copy, Debug)]
pub struct Hook {
    pub x: f32,
    pub z: f32,
    pub nx: f32,
    pub nz: f32,
}

/// Galleries either side of a central hall, each split by a half wall, with a
/// free-standing display wall in the middle of the hall.
pub fn walls(scale: f32) -> Vec<Wall> {
    let s = scale;
    let mut walls = Vec::new();
    for side in [-1.0f32, 1.0] {
        let x = side * 6.0 * s;
        walls.push(Wall {
            x,
            z: -9.75 * s,
            hx: THICK,
            hz: 3.25 * s,
        });
        walls.push(Wall {
            x,
            z: 0.0,
            hx: THICK,
            hz: 2.0 * s,
        });
        walls.push(Wall {
            x,
            z: 9.75 * s,
            hx: THICK,
            hz: 3.25 * s,
        });
        walls.push(Wall {
            x: side * 15.0 * s,
            z: 0.0,
            hx: 3.0 * s,
            hz: THICK,
        });
    }
    walls.push(Wall {
        x: 0.0,
        z: 0.0,
        hx: 3.0 * s,
        hz: THICK,
    });
    walls
}

/// Every place a painting can hang, enough for a full room of guards.
pub fn hooks(scale: f32) -> Vec<Hook> {
    let s = scale;
    let (hx, hz) = (HALF_X * s, HALF_Z * s);
    let outer = |x: f32, z: f32, nx: f32, nz: f32| Hook { x, z, nx, nz };
    let inner = |x: f32, z: f32, nx: f32, nz: f32| Hook {
        x: x * s + nx * THICK,
        z: z * s + nz * THICK,
        nx,
        nz,
    };
    vec![
        outer(-12.0 * s, hz, 0.0, -1.0),
        outer(0.0, hz, 0.0, -1.0),
        outer(12.0 * s, hz, 0.0, -1.0),
        outer(-12.0 * s, -hz, 0.0, 1.0),
        outer(0.0, -hz, 0.0, 1.0),
        outer(12.0 * s, -hz, 0.0, 1.0),
        outer(-hx, 6.5 * s, 1.0, 0.0),
        outer(-hx, -6.5 * s, 1.0, 0.0),
        outer(hx, 6.5 * s, -1.0, 0.0),
        outer(hx, -6.5 * s, -1.0, 0.0),
        inner(-15.0, 0.0, 0.0, 1.0),
        inner(-15.0, 0.0, 0.0, -1.0),
        inner(15.0, 0.0, 0.0, 1.0),
        inner(15.0, 0.0, 0.0, -1.0),
        inner(0.0, 0.0, 0.0, 1.0),
        inner(0.0, 0.0, 0.0, -1.0),
        inner(-6.0, 0.0, -1.0, 0.0),
        inner(6.0, 0.0, 1.0, 0.0),
    ]
}

pub struct Floor {
    pub half_x: f32,
    pub half_z: f32,
    pub walls: Vec<Wall>,
    cols: usize,
    rows: usize,
    open: Vec<bool>,
}

impl Floor {
    pub fn new(scale: f32) -> Self {
        let (half_x, half_z) = (HALF_X * scale, HALF_Z * scale);
        let walls = walls(scale);
        let cols = (half_x * 2.0 / CELL).floor() as usize;
        let rows = (half_z * 2.0 / CELL).floor() as usize;
        let mut floor = Floor {
            half_x,
            half_z,
            walls,
            cols,
            rows,
            open: Vec::new(),
        };
        floor.open = (0..cols * rows)
            .map(|i| {
                let (x, z) = floor.center(i % cols, i / cols);
                floor.standable(x, z)
            })
            .collect();
        floor
    }

    fn standable(&self, x: f32, z: f32) -> bool {
        x.abs() < self.half_x - BODY
            && z.abs() < self.half_z - BODY
            && !self.walls.iter().any(|w| w.contains(x, z, BODY + 0.05))
    }

    fn center(&self, col: usize, row: usize) -> (f32, f32) {
        let width = self.half_x * 2.0 / self.cols as f32;
        let depth = self.half_z * 2.0 / self.rows as f32;
        (
            -self.half_x + (col as f32 + 0.5) * width,
            -self.half_z + (row as f32 + 0.5) * depth,
        )
    }

    fn cell(&self, x: f32, z: f32) -> (usize, usize) {
        let col = ((x + self.half_x) / (self.half_x * 2.0) * self.cols as f32).floor();
        let row = ((z + self.half_z) / (self.half_z * 2.0) * self.rows as f32).floor();
        (
            col.clamp(0.0, self.cols as f32 - 1.0) as usize,
            row.clamp(0.0, self.rows as f32 - 1.0) as usize,
        )
    }

    /// Keep a body inside the building, for the ghost that drifts through
    /// the inside walls.
    pub fn hold_inside(&self, x: &mut f32, z: &mut f32, vx: &mut f32, vz: &mut f32) {
        let (ex, ez) = (self.half_x - BODY, self.half_z - BODY);
        if x.abs() > ex {
            *x = x.clamp(-ex, ex);
            *vx = 0.0;
        }
        if z.abs() > ez {
            *z = z.clamp(-ez, ez);
            *vz = 0.0;
        }
    }

    /// Whether a point is inside a wall.
    pub fn in_wall(&self, x: f32, z: f32) -> bool {
        self.walls.iter().any(|w| w.contains(x, z, 0.0))
    }

    /// Keep a body out of the walls and inside the building.
    pub fn collide(&self, x: &mut f32, z: &mut f32, vx: &mut f32, vz: &mut f32) {
        let (ex, ez) = (self.half_x - BODY, self.half_z - BODY);
        if x.abs() > ex {
            *x = x.clamp(-ex, ex);
            *vx = 0.0;
        }
        if z.abs() > ez {
            *z = z.clamp(-ez, ez);
            *vz = 0.0;
        }
        for wall in &self.walls {
            if !wall.contains(*x, *z, BODY) {
                continue;
            }
            let push_x = wall.hx + BODY - (*x - wall.x).abs();
            let push_z = wall.hz + BODY - (*z - wall.z).abs();
            if push_x < push_z {
                *x += push_x * if *x >= wall.x { 1.0 } else { -1.0 };
                *vx = 0.0;
            } else {
                *z += push_z * if *z >= wall.z { 1.0 } else { -1.0 };
                *vz = 0.0;
            }
        }
    }

    /// Whether light gets from a to b without a wall in the way.
    pub fn sees(&self, a: (f32, f32), b: (f32, f32)) -> bool {
        !self.walls.iter().any(|w| w.crosses(a, b, 0.0))
    }

    /// Whether a body can walk straight from a to b.
    pub fn walkable(&self, a: (f32, f32), b: (f32, f32)) -> bool {
        !self.walls.iter().any(|w| w.crosses(a, b, BODY + 0.1))
    }

    pub fn is_open(&self, x: f32, z: f32) -> bool {
        self.standable(x, z)
    }

    /// Open cells, for picking somewhere to go.
    pub fn open_spots(&self) -> Vec<(f32, f32)> {
        (0..self.cols * self.rows)
            .filter(|i| self.open[*i])
            .map(|i| self.center(i % self.cols, i / self.cols))
            .collect()
    }

    /// Waypoints from `from` to `to` around the walls, ending at `to`.
    pub fn route(&self, from: (f32, f32), to: (f32, f32)) -> Vec<(f32, f32)> {
        if self.walkable(from, to) {
            return vec![to];
        }
        let start = self.nearest_open(self.cell(from.0, from.1));
        let goal = self.nearest_open(self.cell(to.0, to.1));
        let index = |c: (usize, usize)| c.1 * self.cols + c.0;
        let mut came: Vec<Option<usize>> = vec![None; self.cols * self.rows];
        let mut cost = vec![f32::INFINITY; self.cols * self.rows];
        let mut queue = BinaryHeap::new();
        cost[index(start)] = 0.0;
        queue.push(Step {
            score: 0.0,
            cell: index(start),
        });
        let estimate = |i: usize| {
            let (c, r) = (i % self.cols, i / self.cols);
            ((c as f32 - goal.0 as f32).powi(2) + (r as f32 - goal.1 as f32).powi(2)).sqrt()
        };
        while let Some(Step { cell, .. }) = queue.pop() {
            if cell == index(goal) {
                break;
            }
            let (c, r) = ((cell % self.cols) as i32, (cell / self.cols) as i32);
            for (dc, dr) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (nc, nr) = (c + dc, r + dr);
                if nc < 0 || nr < 0 || nc >= self.cols as i32 || nr >= self.rows as i32 {
                    continue;
                }
                let next = nr as usize * self.cols + nc as usize;
                if !self.open[next] {
                    continue;
                }
                if dc != 0 && dr != 0 {
                    let side_a = r as usize * self.cols + nc as usize;
                    let side_b = nr as usize * self.cols + c as usize;
                    if !self.open[side_a] || !self.open[side_b] {
                        continue;
                    }
                }
                let step = if dc != 0 && dr != 0 { 1.414 } else { 1.0 };
                let total = cost[cell] + step;
                if total < cost[next] {
                    cost[next] = total;
                    came[next] = Some(cell);
                    queue.push(Step {
                        score: total + estimate(next),
                        cell: next,
                    });
                }
            }
        }
        let mut cells = vec![index(goal)];
        while let Some(prev) = came[*cells.last().unwrap()] {
            cells.push(prev);
            if cells.len() > self.cols * self.rows {
                break;
            }
        }
        cells.reverse();
        let mut points: Vec<(f32, f32)> = cells
            .iter()
            .map(|&i| self.center(i % self.cols, i / self.cols))
            .collect();
        points.push(to);
        let mut smooth = Vec::new();
        let mut at = from;
        let mut i = 0;
        while i < points.len() {
            let mut far = i;
            for j in (i..points.len()).rev() {
                if self.walkable(at, points[j]) {
                    far = j;
                    break;
                }
            }
            smooth.push(points[far]);
            at = points[far];
            i = far + 1;
        }
        smooth
    }

    fn nearest_open(&self, cell: (usize, usize)) -> (usize, usize) {
        if self.open[cell.1 * self.cols + cell.0] {
            return cell;
        }
        let mut best = cell;
        let mut best_d = i64::MAX;
        for i in 0..self.cols * self.rows {
            if !self.open[i] {
                continue;
            }
            let (c, r) = ((i % self.cols) as i64, (i / self.cols) as i64);
            let d = (c - cell.0 as i64).pow(2) + (r - cell.1 as i64).pow(2);
            if d < best_d {
                best_d = d;
                best = (c as usize, r as usize);
            }
        }
        best
    }
}

#[derive(PartialEq)]
struct Step {
    score: f32,
    cell: usize,
}
impl Eq for Step {}
impl Ord for Step {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
