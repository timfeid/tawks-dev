//! The North America map: 26 real cities and 60 playable routes.
//! Geographic coordinates are projected by the client. Original city IDs remain
//! stable so existing routes, ticket IDs and running games keep their identity.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Black,
    White,
    /// A grey route takes any one color.
    Any,
}

impl TrackColor {
    pub const CARD_COLORS: [TrackColor; 8] = [
        TrackColor::Red,
        TrackColor::Orange,
        TrackColor::Yellow,
        TrackColor::Green,
        TrackColor::Blue,
        TrackColor::Purple,
        TrackColor::Black,
        TrackColor::White,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub id: &'static str,
    pub name: &'static str,
    pub longitude: f32,
    pub latitude: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: u16,
    pub from: &'static str,
    pub to: &'static str,
    pub length: u8,
    pub color: TrackColor,
    /// Routes sharing a `twin` are parallel tracks between the same cities.
    pub twin: Option<u16>,
}

pub const TRAINS_PER_PLAYER: u8 = 45;

pub fn route_points(length: u8) -> u16 {
    match length {
        1 => 1,
        2 => 2,
        3 => 4,
        4 => 7,
        5 => 10,
        6 => 15,
        _ => 0,
    }
}

const fn place(id: &'static str, name: &'static str, longitude: f32, latitude: f32) -> City {
    City { id, name, longitude, latitude }
}

pub const CITIES: &[City] = &[
    place("northgate", "Minneapolis", -93.2650, 44.9778),
    place("frostmere", "Seattle", -122.3321, 47.6062),
    place("kestrel_bay", "Boston", -71.0589, 42.3601),
    place("ashford", "Billings", -108.5007, 45.7833),
    place("glassmoor", "Chicago", -87.6298, 41.8781),
    place("westwick", "Portland", -122.6765, 45.5231),
    place("harrowdale", "Omaha", -95.9345, 41.2565),
    place("ironvale", "New York", -74.0060, 40.7128),
    place("brightwater", "Salt Lake City", -111.8910, 40.7608),
    place("oakridge", "Denver", -104.9903, 39.7392),
    place("thornfield", "Kansas City", -94.5786, 39.0997),
    place("ravenmoor", "Pittsburgh", -79.9959, 40.4406),
    place("saltmarsh", "San Francisco", -122.4194, 37.7749),
    place("millbrook", "Albuquerque", -106.6504, 35.0844),
    place("vantage", "Oklahoma City", -97.5164, 35.4676),
    place("cinderport", "Washington", -77.0369, 38.9072),
    place("larkspur", "Memphis", -90.0490, 35.1495),
    place("dunmere", "Los Angeles", -118.2437, 34.0522),
    place("fairhollow", "Dallas", -96.7970, 32.7767),
    place("quarry_hill", "New Orleans", -90.0715, 29.9511),
    place("umberly", "Atlanta", -84.3880, 33.7490),
    place("elmshore", "San Diego", -117.1611, 32.7157),
    place("pinecrest", "Phoenix", -112.0740, 33.4484),
    place("yarrow", "San Antonio", -98.4936, 29.4241),
    place("juniper_cross", "Houston", -95.3698, 29.7604),
    place("zephyr_point", "Miami", -80.1918, 25.7617),
];

const fn route(id: u16, from: &'static str, to: &'static str, length: u8, color: TrackColor, twin: Option<u16>) -> Route {
    Route { id, from, to, length, color, twin }
}

use TrackColor::*;

pub const ROUTES: &[Route] = &[
    route(1, "northgate", "frostmere", 4, Blue, None),
    route(2, "northgate", "kestrel_bay", 4, Red, None),
    route(3, "northgate", "ashford", 2, Any, Some(4)),
    route(4, "northgate", "ashford", 2, Any, Some(3)),
    route(5, "northgate", "glassmoor", 2, Yellow, None),
    route(6, "northgate", "harrowdale", 2, Green, None),
    route(7, "frostmere", "westwick", 2, White, None),
    route(8, "frostmere", "ashford", 3, Orange, None),
    route(9, "kestrel_bay", "glassmoor", 3, Purple, None),
    route(10, "kestrel_bay", "ironvale", 2, Any, None),
    route(11, "ashford", "westwick", 4, Black, None),
    route(12, "ashford", "harrowdale", 2, Red, None),
    route(13, "ashford", "brightwater", 2, Green, Some(14)),
    route(14, "ashford", "brightwater", 2, Purple, Some(13)),
    route(15, "glassmoor", "harrowdale", 2, Black, None),
    route(16, "glassmoor", "ravenmoor", 2, Orange, None),
    route(17, "glassmoor", "ironvale", 4, White, None),
    route(18, "westwick", "saltmarsh", 2, Any, None),
    route(19, "westwick", "brightwater", 2, Yellow, None),
    route(20, "harrowdale", "oakridge", 2, Any, None),
    route(21, "harrowdale", "thornfield", 2, Blue, None),
    route(22, "ironvale", "ravenmoor", 2, Green, None),
    route(23, "ironvale", "cinderport", 2, Blue, None),
    route(24, "brightwater", "oakridge", 2, Any, None),
    route(25, "brightwater", "saltmarsh", 3, Red, None),
    route(26, "brightwater", "millbrook", 2, Black, None),
    route(27, "oakridge", "thornfield", 3, White, None),
    route(28, "oakridge", "vantage", 2, Orange, None),
    route(29, "oakridge", "millbrook", 2, Purple, None),
    route(30, "thornfield", "ravenmoor", 3, Yellow, None),
    route(31, "thornfield", "vantage", 2, Green, None),
    route(32, "thornfield", "larkspur", 2, Red, None),
    route(33, "ravenmoor", "larkspur", 2, Black, None),
    route(34, "ravenmoor", "cinderport", 3, Any, None),
    route(35, "saltmarsh", "dunmere", 3, Orange, None),
    route(36, "millbrook", "dunmere", 2, Any, None),
    route(37, "millbrook", "fairhollow", 2, Blue, None),
    route(38, "vantage", "fairhollow", 2, Purple, None),
    route(39, "vantage", "larkspur", 3, Any, None),
    route(40, "vantage", "quarry_hill", 2, White, None),
    route(41, "cinderport", "umberly", 2, Purple, None),
    route(42, "larkspur", "quarry_hill", 2, Green, None),
    route(43, "larkspur", "umberly", 2, Orange, None),
    route(44, "dunmere", "elmshore", 2, Yellow, None),
    route(45, "dunmere", "pinecrest", 2, Red, None),
    route(46, "fairhollow", "pinecrest", 3, Black, None),
    route(47, "fairhollow", "yarrow", 2, Any, None),
    route(48, "fairhollow", "quarry_hill", 3, Yellow, None),
    route(49, "quarry_hill", "yarrow", 2, Blue, None),
    route(50, "quarry_hill", "juniper_cross", 2, Any, None),
    route(51, "quarry_hill", "umberly", 4, Red, None),
    route(52, "umberly", "zephyr_point", 3, Green, None),
    route(53, "elmshore", "pinecrest", 3, White, None),
    route(54, "pinecrest", "yarrow", 3, Purple, None),
    route(55, "yarrow", "juniper_cross", 3, Orange, None),
    route(56, "juniper_cross", "zephyr_point", 4, Black, None),
    route(57, "frostmere", "brightwater", 6, Green, None),
    route(58, "kestrel_bay", "ravenmoor", 5, Yellow, None),
    route(59, "saltmarsh", "elmshore", 5, Blue, None),
    route(60, "cinderport", "zephyr_point", 6, White, None),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: u16,
    pub from: String,
    pub to: String,
    pub points: u16,
}

pub fn city(id: &str) -> Option<&'static City> {
    CITIES.iter().find(|c| c.id == id)
}

pub fn route_by_id(id: u16) -> Option<&'static Route> {
    ROUTES.iter().find(|r| r.id == id)
}

/// Shortest track length between two cities over the whole map, used to
/// price tickets.
pub fn shortest_length(from: &str, to: &str) -> Option<u32> {
    let mut best: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    let mut heap = std::collections::BinaryHeap::new();
    best.insert(from, 0);
    heap.push(std::cmp::Reverse((0u32, from)));
    while let Some(std::cmp::Reverse((dist, at))) = heap.pop() {
        if at == to {
            return Some(dist);
        }
        if best.get(at).copied().unwrap_or(u32::MAX) < dist {
            continue;
        }
        for r in ROUTES {
            let next = if r.from == at {
                r.to
            } else if r.to == at {
                r.from
            } else {
                continue;
            };
            let nd = dist + r.length as u32;
            if nd < best.get(next).copied().unwrap_or(u32::MAX) {
                best.insert(next, nd);
                heap.push(std::cmp::Reverse((nd, next)));
            }
        }
    }
    None
}

/// The ticket deck: one ticket per interesting city pair, priced by the
/// shortest track length between them. Pairs closer than five track pieces
/// are left out, since a ticket should take real work.
pub fn tickets() -> Vec<Ticket> {
    const PAIRS: &[(&str, &str)] = &[
        ("northgate", "yarrow"),
        ("northgate", "umberly"),
        ("frostmere", "cinderport"),
        ("frostmere", "fairhollow"),
        ("frostmere", "quarry_hill"),
        ("kestrel_bay", "dunmere"),
        ("kestrel_bay", "yarrow"),
        ("kestrel_bay", "millbrook"),
        ("ashford", "juniper_cross"),
        ("ashford", "umberly"),
        ("ashford", "cinderport"),
        ("glassmoor", "elmshore"),
        ("glassmoor", "pinecrest"),
        ("westwick", "larkspur"),
        ("westwick", "zephyr_point"),
        ("westwick", "juniper_cross"),
        ("harrowdale", "zephyr_point"),
        ("harrowdale", "elmshore"),
        ("ironvale", "dunmere"),
        ("ironvale", "fairhollow"),
        ("ironvale", "pinecrest"),
        ("brightwater", "quarry_hill"),
        ("brightwater", "umberly"),
        ("oakridge", "zephyr_point"),
        ("oakridge", "cinderport"),
        ("thornfield", "elmshore"),
        ("ravenmoor", "pinecrest"),
        ("saltmarsh", "larkspur"),
        ("saltmarsh", "juniper_cross"),
        ("millbrook", "cinderport"),
        ("vantage", "zephyr_point"),
        ("dunmere", "umberly"),
        ("northgate", "saltmarsh"),
        ("glassmoor", "yarrow"),
        ("frostmere", "larkspur"),
        ("kestrel_bay", "elmshore"),
    ];
    PAIRS
        .iter()
        .enumerate()
        .filter_map(|(i, (a, b))| {
            let length = shortest_length(a, b)?;
            if length < 5 {
                return None;
            }
            Some(Ticket { id: i as u16 + 1, from: a.to_string(), to: b.to_string(), points: length as u16 })
        })
        .collect()
}
