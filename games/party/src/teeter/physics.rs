//! A small 2D rigid-box solver for Tower Teeter: boxes in the vertical plane,
//! sequential impulses with warm starting, in the manner of Box2D Lite. It is
//! plain f32 arithmetic in a fixed order, so the same inputs replay the same
//! towers on the server.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const fn new(x: f32, y: f32) -> Self {
        V2 { x, y }
    }
    pub fn dot(self, o: V2) -> f32 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: V2) -> f32 {
        self.x * o.y - self.y * o.x
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    fn abs(self) -> V2 {
        V2::new(self.x.abs(), self.y.abs())
    }
}

impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2::new(self.x + o.x, self.y + o.y)
    }
}
impl std::ops::Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2::new(self.x - o.x, self.y - o.y)
    }
}
impl std::ops::Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, s: f32) -> V2 {
        V2::new(self.x * s, self.y * s)
    }
}
impl std::ops::Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2::new(-self.x, -self.y)
    }
}

/// Angular velocity crossed with an arm.
fn spin(w: f32, r: V2) -> V2 {
    V2::new(-w * r.y, w * r.x)
}

#[derive(Clone, Copy, Debug)]
struct Rot {
    c: f32,
    s: f32,
}

impl Rot {
    fn new(angle: f32) -> Self {
        Rot {
            c: angle.cos(),
            s: angle.sin(),
        }
    }
    fn col1(self) -> V2 {
        V2::new(self.c, self.s)
    }
    fn col2(self) -> V2 {
        V2::new(-self.s, self.c)
    }
    fn apply(self, v: V2) -> V2 {
        V2::new(self.c * v.x - self.s * v.y, self.s * v.x + self.c * v.y)
    }
    fn inverse(self, v: V2) -> V2 {
        V2::new(self.c * v.x + self.s * v.y, -self.s * v.x + self.c * v.y)
    }
}

#[derive(Clone, Debug)]
pub struct Body {
    /// The centre of mass.
    pub pos: V2,
    pub angle: f32,
    pub vel: V2,
    pub spin: f32,
    pub half: V2,
    /// Where the box's centre sits relative to the centre of mass, in the
    /// body's own frame. Furniture is rarely balanced in the middle.
    pub offset: V2,
    pub inv_mass: f32,
    pub inv_inertia: f32,
    pub friction: f32,
}

impl Body {
    pub fn dynamic(half: V2, mass: f32, com: V2, friction: f32) -> Self {
        let inertia =
            mass * (4.0 * (half.x * half.x + half.y * half.y)) / 12.0 + mass * com.dot(com);
        Body {
            pos: V2::default(),
            angle: 0.0,
            vel: V2::default(),
            spin: 0.0,
            half,
            offset: -com,
            inv_mass: 1.0 / mass,
            inv_inertia: 1.0 / inertia,
            friction,
        }
    }

    pub fn kinematic(half: V2, friction: f32) -> Self {
        Body {
            pos: V2::default(),
            angle: 0.0,
            vel: V2::default(),
            spin: 0.0,
            half,
            offset: V2::default(),
            inv_mass: 0.0,
            inv_inertia: 0.0,
            friction,
        }
    }

    /// The box's centre in the world.
    pub fn center(&self) -> V2 {
        self.pos + Rot::new(self.angle).apply(self.offset)
    }

    pub fn corners(&self) -> [V2; 4] {
        let r = Rot::new(self.angle);
        let c = self.center();
        let h = self.half;
        [
            c + r.apply(V2::new(-h.x, -h.y)),
            c + r.apply(V2::new(h.x, -h.y)),
            c + r.apply(V2::new(h.x, h.y)),
            c + r.apply(V2::new(-h.x, h.y)),
        ]
    }

    pub fn bounds(&self) -> (V2, V2) {
        let cs = self.corners();
        let mut lo = cs[0];
        let mut hi = cs[0];
        for c in &cs[1..] {
            lo = V2::new(lo.x.min(c.x), lo.y.min(c.y));
            hi = V2::new(hi.x.max(c.x), hi.y.max(c.y));
        }
        (lo, hi)
    }

    /// The highest point of the box on the vertical line at `x`, if the line
    /// crosses it.
    pub fn top_at(&self, x: f32) -> Option<f32> {
        let cs = self.corners();
        let mut best: Option<f32> = None;
        for i in 0..4 {
            let (a, b) = (cs[i], cs[(i + 1) % 4]);
            let (lo, hi) = if a.x <= b.x { (a, b) } else { (b, a) };
            if x < lo.x || x > hi.x {
                continue;
            }
            let y = if (hi.x - lo.x).abs() < 1e-6 {
                lo.y.max(hi.y)
            } else {
                lo.y + (hi.y - lo.y) * (x - lo.x) / (hi.x - lo.x)
            };
            best = Some(best.map_or(y, |b: f32| b.max(y)));
        }
        best
    }

    pub fn speed(&self) -> f32 {
        self.vel.len() + self.spin.abs() * self.half.len()
    }
}

#[derive(Clone, Copy, Debug)]
struct Contact {
    pos: V2,
    normal: V2,
    separation: f32,
    /// Where the contact sits on the first body, to carry impulses over.
    local: V2,
    pn: f32,
    pt: f32,
    mass_n: f32,
    mass_t: f32,
    bias: f32,
}

#[derive(Clone, Debug)]
pub struct Arbiter {
    pub a: usize,
    pub b: usize,
    contacts: Vec<Contact>,
    friction: f32,
}

impl Arbiter {
    pub fn touching(&self) -> bool {
        !self.contacts.is_empty()
    }
}

const ALLOWED_PENETRATION: f32 = 0.01;
const BIAS_FACTOR: f32 = 0.2;
pub const GRAVITY: f32 = -9.8;

#[derive(Clone, Copy)]
enum Axis {
    FaceAX,
    FaceAY,
    FaceBX,
    FaceBY,
}

fn incident_edge(h: V2, pos: V2, rot: Rot, normal: V2) -> [V2; 2] {
    let n = -rot.inverse(normal);
    let na = n.abs();
    let local = if na.x > na.y {
        if n.x > 0.0 {
            [V2::new(h.x, -h.y), V2::new(h.x, h.y)]
        } else {
            [V2::new(-h.x, h.y), V2::new(-h.x, -h.y)]
        }
    } else if n.y > 0.0 {
        [V2::new(h.x, h.y), V2::new(-h.x, h.y)]
    } else {
        [V2::new(-h.x, -h.y), V2::new(h.x, -h.y)]
    };
    [pos + rot.apply(local[0]), pos + rot.apply(local[1])]
}

fn clip(input: [V2; 2], normal: V2, offset: f32) -> Option<[V2; 2]> {
    let d0 = normal.dot(input[0]) - offset;
    let d1 = normal.dot(input[1]) - offset;
    let mut out = Vec::with_capacity(2);
    if d0 <= 0.0 {
        out.push(input[0]);
    }
    if d1 <= 0.0 {
        out.push(input[1]);
    }
    if d0 * d1 < 0.0 {
        let t = d0 / (d0 - d1);
        out.push(input[0] + (input[1] - input[0]) * t);
    }
    (out.len() >= 2).then(|| [out[0], out[1]])
}

/// Box against box: up to two contact points with the normal pointing from
/// `a` to `b`, and how far apart (negative is overlap) each is.
fn collide(a: &Body, b: &Body) -> Vec<(V2, V2, f32)> {
    let (ha, hb) = (a.half, b.half);
    let (pa, pb) = (a.center(), b.center());
    let (ra, rb) = (Rot::new(a.angle), Rot::new(b.angle));
    let dp = pb - pa;
    let da = ra.inverse(dp);
    let db = rb.inverse(dp);
    // C = RaT * Rb, as columns.
    let c11 = ra.col1().dot(rb.col1());
    let c12 = ra.col1().dot(rb.col2());
    let c21 = ra.col2().dot(rb.col1());
    let c22 = ra.col2().dot(rb.col2());
    let (a11, a12, a21, a22) = (c11.abs(), c12.abs(), c21.abs(), c22.abs());
    let face_a = V2::new(
        da.x.abs() - ha.x - (a11 * hb.x + a12 * hb.y),
        da.y.abs() - ha.y - (a21 * hb.x + a22 * hb.y),
    );
    if face_a.x > 0.0 || face_a.y > 0.0 {
        return Vec::new();
    }
    let face_b = V2::new(
        db.x.abs() - (a11 * ha.x + a21 * ha.y) - hb.x,
        db.y.abs() - (a12 * ha.x + a22 * ha.y) - hb.y,
    );
    if face_b.x > 0.0 || face_b.y > 0.0 {
        return Vec::new();
    }
    const RELATIVE: f32 = 0.95;
    const ABSOLUTE: f32 = 0.01;
    let mut axis = Axis::FaceAX;
    let mut separation = face_a.x;
    let mut normal = if da.x > 0.0 { ra.col1() } else { -ra.col1() };
    if face_a.y > RELATIVE * separation + ABSOLUTE * ha.y {
        axis = Axis::FaceAY;
        separation = face_a.y;
        normal = if da.y > 0.0 { ra.col2() } else { -ra.col2() };
    }
    if face_b.x > RELATIVE * separation + ABSOLUTE * hb.x {
        axis = Axis::FaceBX;
        separation = face_b.x;
        normal = if db.x > 0.0 { rb.col1() } else { -rb.col1() };
    }
    if face_b.y > RELATIVE * separation + ABSOLUTE * hb.y {
        axis = Axis::FaceBY;
        normal = if db.y > 0.0 { rb.col2() } else { -rb.col2() };
    }
    let (front_normal, front, side_normal, neg_side, pos_side, incident) = match axis {
        Axis::FaceAX => {
            let side_n = ra.col2();
            let side = pa.dot(side_n);
            (
                normal,
                pa.dot(normal) + ha.x,
                side_n,
                -side + ha.y,
                side + ha.y,
                incident_edge(hb, pb, rb, normal),
            )
        }
        Axis::FaceAY => {
            let side_n = ra.col1();
            let side = pa.dot(side_n);
            (
                normal,
                pa.dot(normal) + ha.y,
                side_n,
                -side + ha.x,
                side + ha.x,
                incident_edge(hb, pb, rb, normal),
            )
        }
        Axis::FaceBX => {
            let fnorm = -normal;
            let side_n = rb.col2();
            let side = pb.dot(side_n);
            (
                fnorm,
                pb.dot(fnorm) + hb.x,
                side_n,
                -side + hb.y,
                side + hb.y,
                incident_edge(ha, pa, ra, fnorm),
            )
        }
        Axis::FaceBY => {
            let fnorm = -normal;
            let side_n = rb.col1();
            let side = pb.dot(side_n);
            (
                fnorm,
                pb.dot(fnorm) + hb.y,
                side_n,
                -side + hb.x,
                side + hb.x,
                incident_edge(ha, pa, ra, fnorm),
            )
        }
    };
    let Some(clipped) = clip(incident, -side_normal, neg_side) else {
        return Vec::new();
    };
    let Some(clipped) = clip(clipped, side_normal, pos_side) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(2);
    for point in clipped {
        let sep = front_normal.dot(point) - front;
        if sep <= 0.0 {
            out.push((point - front_normal * sep, normal, sep));
        }
    }
    out
}

/// One raft's worth of bodies. Body 0 is the raft, moved by the sea rather
/// than by contacts.
#[derive(Clone, Debug, Default)]
pub struct World {
    pub bodies: Vec<Body>,
    pub arbiters: Vec<Arbiter>,
    pub iterations: u32,
}

impl World {
    pub fn new(iterations: u32) -> Self {
        World {
            bodies: Vec::new(),
            arbiters: Vec::new(),
            iterations,
        }
    }

    pub fn remove(&mut self, index: usize) {
        self.bodies.remove(index);
        self.arbiters.retain(|a| a.a != index && a.b != index);
        for arb in &mut self.arbiters {
            if arb.a > index {
                arb.a -= 1;
            }
            if arb.b > index {
                arb.b -= 1;
            }
        }
    }

    fn broadphase(&mut self) {
        let bounds: Vec<(V2, V2)> = self.bodies.iter().map(Body::bounds).collect();
        let mut next: Vec<Arbiter> = Vec::new();
        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let (a, b) = (&self.bodies[i], &self.bodies[j]);
                if a.inv_mass == 0.0 && b.inv_mass == 0.0 {
                    continue;
                }
                let (alo, ahi) = bounds[i];
                let (blo, bhi) = bounds[j];
                if ahi.x < blo.x || bhi.x < alo.x || ahi.y < blo.y || bhi.y < alo.y {
                    continue;
                }
                let found = collide(a, b);
                if found.is_empty() {
                    continue;
                }
                let ra = Rot::new(a.angle);
                let old = self.arbiters.iter().find(|arb| arb.a == i && arb.b == j);
                let contacts = found
                    .into_iter()
                    .map(|(pos, normal, separation)| {
                        let local = ra.inverse(pos - a.pos);
                        let carried = old.and_then(|arb| {
                            arb.contacts.iter().find(|c| (c.local - local).len() < 0.06)
                        });
                        Contact {
                            pos,
                            normal,
                            separation,
                            local,
                            pn: carried.map_or(0.0, |c| c.pn),
                            pt: carried.map_or(0.0, |c| c.pt),
                            mass_n: 0.0,
                            mass_t: 0.0,
                            bias: 0.0,
                        }
                    })
                    .collect();
                next.push(Arbiter {
                    a: i,
                    b: j,
                    contacts,
                    friction: (a.friction * b.friction).sqrt(),
                });
            }
        }
        self.arbiters = next;
    }

    pub fn step(&mut self, dt: f32) {
        self.broadphase();
        for body in &mut self.bodies {
            if body.inv_mass == 0.0 {
                continue;
            }
            body.vel.y += GRAVITY * dt;
            // A touch of drag so a settled stack stops humming.
            let damp = 1.0 / (1.0 + dt * 0.08);
            body.vel = body.vel * damp;
            body.spin *= 1.0 / (1.0 + dt * 0.4);
        }
        let inv_dt = 1.0 / dt;
        for k in 0..self.arbiters.len() {
            let arb = &mut self.arbiters[k];
            let (b1, b2) = (&self.bodies[arb.a], &self.bodies[arb.b]);
            for c in &mut arb.contacts {
                let r1 = c.pos - b1.pos;
                let r2 = c.pos - b2.pos;
                let rn1 = r1.dot(c.normal);
                let rn2 = r2.dot(c.normal);
                let kn = b1.inv_mass
                    + b2.inv_mass
                    + b1.inv_inertia * (r1.dot(r1) - rn1 * rn1)
                    + b2.inv_inertia * (r2.dot(r2) - rn2 * rn2);
                c.mass_n = 1.0 / kn;
                let t = V2::new(c.normal.y, -c.normal.x);
                let rt1 = r1.dot(t);
                let rt2 = r2.dot(t);
                let kt = b1.inv_mass
                    + b2.inv_mass
                    + b1.inv_inertia * (r1.dot(r1) - rt1 * rt1)
                    + b2.inv_inertia * (r2.dot(r2) - rt2 * rt2);
                c.mass_t = 1.0 / kt;
                c.bias = -BIAS_FACTOR * inv_dt * (c.separation + ALLOWED_PENETRATION).min(0.0);
            }
            let (ia, ib) = (arb.a, arb.b);
            for i in 0..self.arbiters[k].contacts.len() {
                let c = self.arbiters[k].contacts[i];
                let p = c.normal * c.pn + V2::new(c.normal.y, -c.normal.x) * c.pt;
                self.impulse(ia, ib, c.pos, p);
            }
        }
        for _ in 0..self.iterations {
            for k in 0..self.arbiters.len() {
                let (ia, ib, friction) = {
                    let arb = &self.arbiters[k];
                    (arb.a, arb.b, arb.friction)
                };
                for i in 0..self.arbiters[k].contacts.len() {
                    let mut c = self.arbiters[k].contacts[i];
                    let rel = |w: &World, c: &Contact| {
                        let (b1, b2) = (&w.bodies[ia], &w.bodies[ib]);
                        let r1 = c.pos - b1.pos;
                        let r2 = c.pos - b2.pos;
                        b2.vel + spin(b2.spin, r2) - b1.vel - spin(b1.spin, r1)
                    };
                    let vn = rel(self, &c).dot(c.normal);
                    let mut dpn = c.mass_n * (-vn + c.bias);
                    let pn0 = c.pn;
                    c.pn = (pn0 + dpn).max(0.0);
                    dpn = c.pn - pn0;
                    self.impulse(ia, ib, c.pos, c.normal * dpn);
                    let t = V2::new(c.normal.y, -c.normal.x);
                    let vt = rel(self, &c).dot(t);
                    let dpt = c.mass_t * -vt;
                    let max = friction * c.pn;
                    let pt0 = c.pt;
                    c.pt = (pt0 + dpt).clamp(-max, max);
                    self.impulse(ia, ib, c.pos, t * (c.pt - pt0));
                    self.arbiters[k].contacts[i] = c;
                }
            }
        }
        for body in &mut self.bodies {
            body.pos = body.pos + body.vel * dt;
            body.angle += body.spin * dt;
        }
    }

    fn impulse(&mut self, a: usize, b: usize, at: V2, p: V2) {
        let b1 = &mut self.bodies[a];
        let r1 = at - b1.pos;
        b1.vel = b1.vel - p * b1.inv_mass;
        b1.spin -= b1.inv_inertia * r1.cross(p);
        let b2 = &mut self.bodies[b];
        let r2 = at - b2.pos;
        b2.vel = b2.vel + p * b2.inv_mass;
        b2.spin += b2.inv_inertia * r2.cross(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor() -> Body {
        let mut f = Body::kinematic(V2::new(5.0, 0.25), 0.9);
        f.pos = V2::new(0.0, -0.25);
        f
    }

    fn crate_at(x: f32, y: f32) -> Body {
        let mut b = Body::dynamic(V2::new(0.5, 0.5), 1.0, V2::default(), 0.7);
        b.pos = V2::new(x, y);
        b
    }

    #[test]
    fn a_stack_of_boxes_settles_and_stays_up() {
        let mut w = World::new(10);
        w.bodies.push(floor());
        for i in 0..6 {
            w.bodies
                .push(crate_at(0.02 * i as f32, 0.52 + i as f32 * 1.02));
        }
        for _ in 0..600 {
            w.step(1.0 / 120.0);
        }
        let top = w.bodies.last().unwrap();
        assert!((top.pos.y - 5.5).abs() < 0.12, "top crate at {}", top.pos.y);
        assert!(top.pos.x.abs() < 0.3);
        assert!(top.speed() < 0.05, "settled, {}", top.speed());
    }

    #[test]
    fn an_overhanging_box_tips_off() {
        let mut w = World::new(10);
        w.bodies.push(floor());
        w.bodies.push(crate_at(0.0, 0.5));
        // Centre of mass past the edge of the one below.
        w.bodies.push(crate_at(0.8, 1.52));
        for _ in 0..360 {
            w.step(1.0 / 120.0);
        }
        assert!(w.bodies[2].pos.y < 1.2, "fell to {}", w.bodies[2].pos.y);
    }

    #[test]
    fn replays_are_identical() {
        let run = || {
            let mut w = World::new(8);
            w.bodies.push(floor());
            for i in 0..5 {
                let mut b = crate_at(0.13 * i as f32 - 0.3, 0.6 + i as f32 * 1.1);
                b.angle = 0.07 * i as f32;
                w.bodies.push(b);
            }
            for _ in 0..400 {
                w.step(1.0 / 120.0);
            }
            w.bodies
                .iter()
                .map(|b| (b.pos.x.to_bits(), b.pos.y.to_bits(), b.angle.to_bits()))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn top_at_follows_a_tilted_box() {
        let mut b = crate_at(0.0, 1.0);
        assert!((b.top_at(0.2).unwrap() - 1.5).abs() < 1e-4);
        assert!(b.top_at(0.7).is_none());
        b.angle = std::f32::consts::FRAC_PI_4;
        assert!((b.top_at(0.0).unwrap() - (1.0 + 0.5f32 * 2f32.sqrt())).abs() < 1e-4);
    }
}
