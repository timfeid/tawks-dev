//! The hand-built boards. Each is a map file shared with the client, in
//! `open/games/canvases/src/party/board/maps`: the island's coastline, its
//! land, its landmarks and scenery, and the paths the board is laid along.
//! The server only reads the paths; the rest is for drawing.
//!
//! The first path is the lap, closed, starting at the start space. Every
//! other path leaves the lap at the space nearest its first point and rejoins
//! it at the space nearest its last, unless it ends in a warp. With
//! `join_paths`, either endpoint can join an existing trail to form shared
//! crossroads. A point can
//! carry the kinds of space for its stretch of path; otherwise the path's
//! own kinds run along it.

use serde::Deserialize;

use crate::board::{Space, SpaceKind};

/// The gap between one space and the next along a path, in world units.
pub const SPACING: f32 = 3.1;

#[derive(Debug, Deserialize)]
pub struct Design {
    pub paths: Vec<PathDesign>,
    #[serde(default)]
    pub roads:Vec<RoadDesign>,
    #[serde(default)]
    pub specials: Specials,
}

#[derive(Debug,Deserialize)]
pub struct RoadDesign {pub points:Vec<(f32,f32)>,pub width:f32}
impl RoadDesign {
 pub fn contains(&self,at:(f32,f32),margin:f32)->bool {self.points.windows(2).any(|p|{let (dx,dz)=(p[1].0-p[0].0,p[1].1-p[0].1);let length=dx*dx+dz*dz;let t=if length>0.{((at.0-p[0].0)*dx+(at.1-p[0].1)*dz)/length}else{0.}.clamp(0.,1.);(at.0-p[0].0-dx*t).hypot(at.1-p[0].1-dz*t)<self.width*0.5+margin})}
}

/// A board's own things for the vacuum, off the path: Palm Island's coconut
/// palms, the flotsam points out in Shipwreck Cove's water, Frozen Peak's
/// snowman. In world units.
#[derive(Debug, Default, Deserialize)]
pub struct Specials {
    #[serde(default)]
    pub palms: Vec<(f32, f32)>,
    #[serde(default)]
    pub flotsam: Vec<(f32, f32)>,
    #[serde(default)]
    pub snowmen: Vec<(f32, f32)>,
}

/// A board's specials, from its map file.
pub fn specials(source: &str) -> Specials {
    serde_json::from_str::<Design>(source).expect("a board's map file reads").specials
}

#[derive(Debug, Deserialize)]
pub struct PathDesign {
    pub id: String,
    #[serde(default)]
    pub spacing: Option<f32>,
    #[serde(default)]
    pub closed: bool,
    pub points: Vec<Point>,
    #[serde(default)]
    pub kinds: Vec<SpaceKind>,
    /// Runs underground between its first and last spaces.
    #[serde(default)]
    pub under: bool,
    /// Shut until the last rounds.
    #[serde(default)]
    pub gated: bool,
    /// Ends in a teleport to the lap space nearest this point.
    #[serde(default)]
    pub warp_to: Option<(f32, f32)>,
    /// Connect to an existing trail as well as the outer lap. This allows
    /// proper shared crossroads rather than disconnected crossing lines.
    #[serde(default)]
    pub join_paths: bool,
}

/// A point on a path, with the kinds of space for the stretch that starts there.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Point {
    At(f32, f32),
    With(f32, f32, Vec<SpaceKind>),
}

impl Point {
    fn at(&self) -> (f32, f32) {
        match self {
            Point::At(x, z) | Point::With(x, z, _) => (*x, *z),
        }
    }

    fn kinds(&self) -> Option<&[SpaceKind]> {
        match self {
            Point::With(_, _, kinds) if !kinds.is_empty() => Some(kinds),
            _ => None,
        }
    }
}

/// Spaces every `SPACING` along a path, each with the kind it gets.
fn lay(path: &PathDesign) -> Vec<((f32, f32), SpaceKind)> {
    let points: Vec<(f32, f32)> = path.points.iter().map(Point::at).collect();
    let segments = if path.closed { points.len() } else { points.len() - 1 };
    let spacing=path.spacing.filter(|v|v.is_finite()).unwrap_or(SPACING).clamp(SPACING,12.0);
    let mut out = Vec::new();
    let mut carry = 0.0f32;
    for i in 0..segments {
        let (ax, az) = points[i];
        let (bx, bz) = points[(i + 1) % points.len()];
        let length = ((bx - ax).powi(2) + (bz - az).powi(2)).sqrt();
        let own = path.points[i].kinds();
        let mut along = carry;
        let mut in_segment = 0;
        while along < length {
            let t = along / length;
            let kind = match own {
                Some(kinds) => kinds[in_segment % kinds.len()],
                None if !path.kinds.is_empty() => path.kinds[out.len() % path.kinds.len()],
                None => SpaceKind::Coin,
            };
            out.push(((ax + (bx - ax) * t, az + (bz - az) * t), kind));
            along += spacing;
            in_segment += 1;
        }
        carry = along - length;
    }
    if path.closed && out.len() > 1 {
        let (first, _) = out[0];
        let (last, _) = *out.last().unwrap();
        if ((last.0 - first.0).powi(2) + (last.1 - first.1).powi(2)).sqrt() < spacing * 0.75 {
            out.pop();
        }
    }
    out
}

fn nearest(spaces: &[Space], to: (f32, f32), skip: &[u8]) -> u8 {
    spaces
        .iter()
        .filter(|space| !skip.contains(&space.id))
        .min_by(|a, b| {
            let da = (a.x - to.0).powi(2) + (a.z - to.1).powi(2);
            let db = (b.x - to.0).powi(2) + (b.z - to.1).powi(2);
            da.total_cmp(&db)
        })
        .map(|space| space.id)
        .unwrap_or(0)
}

/// The spaces of a board, from its map file.
pub fn build(source: &str) -> Vec<Space> {
    let design: Design = serde_json::from_str(source).expect("a board's map file reads");
    let (lap_design, ways) = design.paths.split_first().expect("a board has a lap");
    assert!(lap_design.closed, "{}: the first path is the lap, and closed", lap_design.id);
    let safe=|point:&((f32,f32),SpaceKind)|!design.roads.iter().any(|road|road.contains(point.0,1.0));
    let lap:Vec<_> = lay(lap_design).into_iter().filter(safe).collect();
    let mut spaces: Vec<Space> = lap
        .iter()
        .enumerate()
        .map(|(i, ((x, z), kind))| Space {
            id: i as u8,
            x: *x,
            z: *z,
            kind: if i == 0 { SpaceKind::Start } else { *kind },
            next: vec![((i + 1) % lap.len()) as u8],
            link: None,
            gated: false,
            under: false,
            path: 0,
        })
        .collect();
    let lap_spaces = spaces.clone();
    let mut entries: Vec<u8> = Vec::new();
    for (index, way) in ways.iter().enumerate() {
        // The first point sits on an existing route; new spaces start one along.
        let mut laid: Vec<_> = lay(way).into_iter().filter(|point|way.under||safe(point)).skip(1).collect();
        assert!(laid.len() >= 2, "{}: a way is at least two spaces long", way.id);
        let anchors: Vec<Space> = if way.join_paths { spaces.iter().filter(|s| !s.under && !s.gated && s.link.is_none()).cloned().collect() } else { lap_spaces.clone() };
        let entry = nearest(&anchors, way.points[0].at(), if way.join_paths { &[] } else { &entries });
        let end = way.points.last().unwrap().at();
        let (rejoin, warp) = match way.warp_to {
            Some(to) => {
                let warp = nearest(&lap_spaces, to, &[]);
                (warp, Some(warp))
            }
            None => (nearest(&anchors, end, &[]), None),
        };
        // A returning trail ends short of the existing junction, not on top of it.
        let back = &spaces[rejoin as usize];
        while laid.len() > 2 && warp.is_none() {
            let ((x, z), _) = *laid.last().unwrap();
            if ((x - back.x).powi(2) + (z - back.z).powi(2)).sqrt() >= SPACING * 0.75 {
                break;
            }
            laid.pop();
        }
        if !design.roads.is_empty() && !way.under {laid.retain(|((x,z),_)|spaces.iter().filter(|s|!s.under).all(|s|(s.x-x).hypot(s.z-z)>=1.5));}
        if !design.roads.is_empty() && way.under {
            while laid.len()>2 && laid.first().is_some_and(|((x,z),_)|!safe(&((*x,*z),SpaceKind::Coin)) || spaces.iter().filter(|s|!s.under).any(|s|(s.x-x).hypot(s.z-z)<1.5)){laid.remove(0);}
            while laid.len()>2 && laid.last().is_some_and(|((x,z),_)|!safe(&((*x,*z),SpaceKind::Coin)) || spaces.iter().filter(|s|!s.under).any(|s|(s.x-x).hypot(s.z-z)<1.5)){laid.pop();}
        }
        let first = spaces.len();
        let count = laid.len();
        for (i, ((x, z), kind)) in laid.into_iter().enumerate() {
            let id = (first + i) as u8;
            let last = i + 1 == count;
            let mouth = i == 0 || last;
            spaces.push(Space {
                id,
                x,
                z,
                kind: if last && warp.is_some() { SpaceKind::Teleport } else { kind },
                next: vec![if last { rejoin } else { id + 1 }],
                link: if last { warp } else { None },
                gated: way.gated,
                // A tunnel's first and last spaces are its mouths, above ground.
                under: way.under && !mouth,
                path: (index + 1) as u8,
            });
        }
        spaces[entry as usize].next.push(first as u8);
        entries.push(entry);
    }
    for space in &mut spaces {if !space.under && matches!(space.kind,SpaceKind::Shop|SpaceKind::Cannon) && design.roads.iter().any(|road|road.contains((space.x,space.z),4.0)){space.kind=SpaceKind::Coin;}}
    assert!(spaces.len() < u8::MAX as usize, "a board's space ids fit in a byte");
    spaces
}
