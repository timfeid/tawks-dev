//! The same flat frame the client draws in: meters east (x) and south (z)
//! of the heist's origin, Web Mercator scaled by the origin's latitude.

use std::f64::consts::PI;

pub const TILE_ZOOM: u32 = 14;
const EARTH_R: f64 = 6_378_137.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

pub fn tile_of(p: LatLon, z: u32) -> (f64, f64) {
    let n = 2f64.powi(z as i32);
    let x = (p.lon + 180.0) / 360.0 * n;
    let r = p.lat.to_radians();
    let y = (1.0 - (r.tan() + 1.0 / r.cos()).ln() / PI) / 2.0 * n;
    (x, y)
}

pub fn lat_lon_of_tile(x: f64, y: f64, z: u32) -> LatLon {
    let n = 2f64.powi(z as i32);
    LatLon {
        lon: x / n * 360.0 - 180.0,
        lat: (PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees(),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub origin: LatLon,
    pub meters_per_tile: f64,
    ox: f64,
    oy: f64,
}

impl Frame {
    pub fn new(origin: LatLon) -> Frame {
        let (ox, oy) = tile_of(origin, TILE_ZOOM);
        let scale = origin.lat.to_radians().cos();
        Frame {
            origin,
            meters_per_tile: 2.0 * PI * EARTH_R / 2f64.powi(TILE_ZOOM as i32) * scale,
            ox,
            oy,
        }
    }

    /// World meters of a point inside tile (tx, ty) given in tile units of `extent`.
    pub fn from_tile(&self, tx: u32, ty: u32, extent: f64, px: f64, py: f64) -> (f64, f64) {
        (
            (tx as f64 + px / extent - self.ox) * self.meters_per_tile,
            (ty as f64 + py / extent - self.oy) * self.meters_per_tile,
        )
    }

    /// The zoom 14 tile holding a world point.
    pub fn tile_at(&self, x: f64, z: f64) -> (u32, u32) {
        (
            (self.ox + x / self.meters_per_tile).floor() as u32,
            (self.oy + z / self.meters_per_tile).floor() as u32,
        )
    }

    pub fn to_world(&self, p: LatLon) -> (f64, f64) {
        let (x, y) = tile_of(p, TILE_ZOOM);
        (
            (x - self.ox) * self.meters_per_tile,
            (y - self.oy) * self.meters_per_tile,
        )
    }

    pub fn to_lat_lon(&self, x: f64, z: f64) -> LatLon {
        lat_lon_of_tile(
            self.ox + x / self.meters_per_tile,
            self.oy + z / self.meters_per_tile,
            TILE_ZOOM,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_points_round_trip_and_the_origin_is_zero() {
        let f = Frame::new(LatLon {
            lat: 40.6865,
            lon: -74.144,
        });
        let (x, z) = f.to_world(f.origin);
        assert!(x.abs() < 1e-6 && z.abs() < 1e-6);
        let p = LatLon {
            lat: 40.70,
            lon: -74.20,
        };
        let (x, z) = f.to_world(p);
        assert!(x < -4000.0 && z < -1000.0, "{x} {z}");
        let back = f.to_lat_lon(x, z);
        assert!((back.lat - p.lat).abs() < 1e-9 && (back.lon - p.lon).abs() < 1e-9);
        assert!((f.meters_per_tile - 1855.0).abs() < 10.0, "{}", f.meters_per_tile);
    }
}
