//! The objects Blueprints shows, each built from a handful of primitives so the
//! native client can draw it as lines, as a blueprint fill or in colour. The
//! y axis is up and every object stands on y = 0, facing +z.

use std::f32::consts::{FRAC_PI_2 as Q, PI};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Block,
    Ball,
    /// A cylinder along y.
    Tube,
    /// A cone along y with its tip up.
    Cone,
    /// A torus lying flat in the xz plane: `size[0]` across, `size[1]` thick.
    Ring,
    /// A triangular prism, the triangle standing in xy with its apex up and
    /// its length along z: roofs and tents.
    Prism,
    /// A right-angled prism, the upright edge on -x: sails and fins.
    Wedge,
}

impl Shape {
    pub fn name(self) -> &'static str {
        match self {
            Shape::Block => "block",
            Shape::Ball => "ball",
            Shape::Tube => "tube",
            Shape::Cone => "cone",
            Shape::Ring => "ring",
            Shape::Prism => "prism",
            Shape::Wedge => "wedge",
        }
    }

    pub fn from_name(name: &str) -> Option<Shape> {
        [
            Shape::Block,
            Shape::Ball,
            Shape::Tube,
            Shape::Cone,
            Shape::Ring,
            Shape::Prism,
            Shape::Wedge,
        ]
        .into_iter()
        .find(|shape| shape.name() == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub shape: Shape,
    pub at: [f32; 3],
    pub size: [f32; 3],
    /// Euler angles in radians, applied x, then y, then z.
    pub turn: [f32; 3],
    pub color: u32,
}

impl Part {
    fn turn(mut self, x: f32, y: f32, z: f32) -> Part {
        self.turn = [x, y, z];
        self
    }

    /// The radius of a ball that holds the part whichever way it is turned.
    pub fn reach(&self) -> f32 {
        let [x, y, z] = self.size;
        0.5 * (x * x + y * y + z * z).sqrt()
    }
}

fn part(shape: Shape, at: [f32; 3], size: [f32; 3], color: u32) -> Part {
    Part {
        shape,
        at,
        size,
        turn: [0.0; 3],
        color,
    }
}

fn block(x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32, color: u32) -> Part {
    part(Shape::Block, [x, y, z], [sx, sy, sz], color)
}

fn ball(x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32, color: u32) -> Part {
    part(Shape::Ball, [x, y, z], [sx, sy, sz], color)
}

fn tube(x: f32, y: f32, z: f32, across: f32, tall: f32, color: u32) -> Part {
    part(Shape::Tube, [x, y, z], [across, tall, across], color)
}

fn cone(x: f32, y: f32, z: f32, across: f32, tall: f32, color: u32) -> Part {
    part(Shape::Cone, [x, y, z], [across, tall, across], color)
}

fn ring(x: f32, y: f32, z: f32, across: f32, thick: f32, color: u32) -> Part {
    part(Shape::Ring, [x, y, z], [across, thick, across], color)
}

fn prism(x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32, color: u32) -> Part {
    part(Shape::Prism, [x, y, z], [sx, sy, sz], color)
}

fn wedge(x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32, color: u32) -> Part {
    part(Shape::Wedge, [x, y, z], [sx, sy, sz], color)
}

const WOOD: u32 = 0xb07a45;
const DARK_WOOD: u32 = 0x6e4526;
const WHITE: u32 = 0xf2f2ee;
const BLACK: u32 = 0x22252b;
const STEEL: u32 = 0x9aa4ae;
const RED: u32 = 0xd8353b;
const ORANGE: u32 = 0xf08a24;
const YELLOW: u32 = 0xf6c936;
const GREEN: u32 = 0x4caf50;
const LEAF: u32 = 0x2f8a3a;
const BLUE: u32 = 0x3b7dd8;
const SKY: u32 = 0x9fd4ff;
const PINK: u32 = 0xf28bb4;
const BROWN: u32 = 0x7a5232;
const GOLD: u32 = 0xe8b730;
const GREY: u32 = 0x6b7078;
const CREAM: u32 = 0xf3e3c3;
const PURPLE: u32 = 0x8e5bd6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Home,
    Tool,
    Vehicle,
    Animal,
    Outdoors,
    Food,
}

pub struct Object {
    pub name: &'static str,
    /// Words that count as this object; the first is how it is shown.
    pub terms: &'static [&'static str],
    pub kind: Kind,
    /// Roughly how much of the reveal an average player needs to see, from 0
    /// to 1. Bots read it; people are free to be quicker.
    pub difficulty: f32,
    pub build: fn() -> Vec<Part>,
}

pub static CATALOG: &[Object] = &[
    Object {
        name: "Chair",
        terms: &["chair", "dining chair", "seat"],
        kind: Kind::Home,
        difficulty: 0.42,
        build: chair,
    },
    Object {
        name: "Table",
        terms: &["table", "dining table", "desk", "kitchen table"],
        kind: Kind::Home,
        difficulty: 0.45,
        build: table,
    },
    Object {
        name: "Desk Lamp",
        terms: &["desk lamp", "lamp", "reading lamp", "anglepoise"],
        kind: Kind::Home,
        difficulty: 0.5,
        build: lamp,
    },
    Object {
        name: "Teapot",
        terms: &["teapot", "tea pot", "kettle", "tea kettle"],
        kind: Kind::Home,
        difficulty: 0.45,
        build: teapot,
    },
    Object {
        name: "Mug",
        terms: &[
            "mug",
            "coffee mug",
            "cup",
            "coffee cup",
            "tea cup",
            "teacup",
        ],
        kind: Kind::Home,
        difficulty: 0.4,
        build: mug,
    },
    Object {
        name: "Umbrella",
        terms: &["umbrella", "parasol", "brolly"],
        kind: Kind::Home,
        difficulty: 0.38,
        build: umbrella,
    },
    Object {
        name: "Alarm Clock",
        terms: &["alarm clock", "clock", "alarm"],
        kind: Kind::Home,
        difficulty: 0.48,
        build: alarm_clock,
    },
    Object {
        name: "Light Bulb",
        terms: &["light bulb", "lightbulb", "bulb"],
        kind: Kind::Home,
        difficulty: 0.42,
        build: light_bulb,
    },
    Object {
        name: "Bed",
        terms: &["bed", "double bed"],
        kind: Kind::Home,
        difficulty: 0.44,
        build: bed,
    },
    Object {
        name: "Sofa",
        terms: &["sofa", "couch", "settee", "loveseat"],
        kind: Kind::Home,
        difficulty: 0.45,
        build: sofa,
    },
    Object {
        name: "Piano",
        terms: &["piano", "upright piano", "keyboard"],
        kind: Kind::Home,
        difficulty: 0.55,
        build: piano,
    },
    Object {
        name: "Guitar",
        terms: &["guitar", "acoustic guitar"],
        kind: Kind::Home,
        difficulty: 0.4,
        build: guitar,
    },
    Object {
        name: "Ladder",
        terms: &["ladder", "step ladder", "stepladder"],
        kind: Kind::Tool,
        difficulty: 0.35,
        build: ladder,
    },
    Object {
        name: "Watering Can",
        terms: &[
            "watering can",
            "water can",
            "watering pot",
            "sprinkling can",
        ],
        kind: Kind::Tool,
        difficulty: 0.55,
        build: watering_can,
    },
    Object {
        name: "Hammer",
        terms: &["hammer", "claw hammer"],
        kind: Kind::Tool,
        difficulty: 0.4,
        build: hammer,
    },
    Object {
        name: "Key",
        terms: &["key", "door key"],
        kind: Kind::Tool,
        difficulty: 0.45,
        build: key,
    },
    Object {
        name: "Hourglass",
        terms: &["hourglass", "hour glass", "sand timer", "egg timer"],
        kind: Kind::Tool,
        difficulty: 0.5,
        build: hourglass,
    },
    Object {
        name: "Trophy",
        terms: &["trophy", "award", "prize", "trophy cup"],
        kind: Kind::Tool,
        difficulty: 0.48,
        build: trophy,
    },
    Object {
        name: "Car",
        terms: &["car", "automobile", "auto", "sedan", "taxi"],
        kind: Kind::Vehicle,
        difficulty: 0.36,
        build: car,
    },
    Object {
        name: "Bicycle",
        terms: &["bicycle", "bike", "cycle", "push bike"],
        kind: Kind::Vehicle,
        difficulty: 0.42,
        build: bicycle,
    },
    Object {
        name: "Airplane",
        terms: &[
            "airplane",
            "aeroplane",
            "plane",
            "jet",
            "aircraft",
            "airliner",
        ],
        kind: Kind::Vehicle,
        difficulty: 0.35,
        build: airplane,
    },
    Object {
        name: "Helicopter",
        terms: &["helicopter", "chopper", "heli"],
        kind: Kind::Vehicle,
        difficulty: 0.42,
        build: helicopter,
    },
    Object {
        name: "Sailboat",
        terms: &[
            "sailboat",
            "sail boat",
            "sailing boat",
            "yacht",
            "boat",
            "ship",
            "dinghy",
        ],
        kind: Kind::Vehicle,
        difficulty: 0.4,
        build: sailboat,
    },
    Object {
        name: "Rocket",
        terms: &[
            "rocket",
            "rocket ship",
            "rocketship",
            "spaceship",
            "missile",
        ],
        kind: Kind::Vehicle,
        difficulty: 0.35,
        build: rocket,
    },
    Object {
        name: "Steam Train",
        terms: &[
            "steam train",
            "train",
            "locomotive",
            "steam engine",
            "engine",
        ],
        kind: Kind::Vehicle,
        difficulty: 0.5,
        build: train,
    },
    Object {
        name: "Hot Air Balloon",
        terms: &["hot air balloon", "balloon", "air balloon"],
        kind: Kind::Vehicle,
        difficulty: 0.4,
        build: hot_air_balloon,
    },
    Object {
        name: "School Bus",
        terms: &["school bus", "bus", "coach"],
        kind: Kind::Vehicle,
        difficulty: 0.45,
        build: bus,
    },
    Object {
        name: "Submarine",
        terms: &["submarine", "sub", "u boat"],
        kind: Kind::Vehicle,
        difficulty: 0.5,
        build: submarine,
    },
    Object {
        name: "Duck",
        terms: &["duck", "rubber duck", "rubber ducky", "duckling", "ducky"],
        kind: Kind::Animal,
        difficulty: 0.42,
        build: duck,
    },
    Object {
        name: "Penguin",
        terms: &["penguin"],
        kind: Kind::Animal,
        difficulty: 0.45,
        build: penguin,
    },
    Object {
        name: "Giraffe",
        terms: &["giraffe"],
        kind: Kind::Animal,
        difficulty: 0.38,
        build: giraffe,
    },
    Object {
        name: "Snail",
        terms: &["snail"],
        kind: Kind::Animal,
        difficulty: 0.5,
        build: snail,
    },
    Object {
        name: "Turtle",
        terms: &["turtle", "tortoise"],
        kind: Kind::Animal,
        difficulty: 0.48,
        build: turtle,
    },
    Object {
        name: "Fish",
        terms: &["fish", "goldfish"],
        kind: Kind::Animal,
        difficulty: 0.45,
        build: fish,
    },
    Object {
        name: "Octopus",
        terms: &["octopus", "squid"],
        kind: Kind::Animal,
        difficulty: 0.48,
        build: octopus,
    },
    Object {
        name: "Lighthouse",
        terms: &["lighthouse", "light house", "beacon"],
        kind: Kind::Outdoors,
        difficulty: 0.45,
        build: lighthouse,
    },
    Object {
        name: "Windmill",
        terms: &["windmill", "wind mill", "mill"],
        kind: Kind::Outdoors,
        difficulty: 0.42,
        build: windmill,
    },
    Object {
        name: "House",
        terms: &["house", "home", "cottage", "hut", "cabin"],
        kind: Kind::Outdoors,
        difficulty: 0.35,
        build: house,
    },
    Object {
        name: "Tent",
        terms: &["tent", "camping tent"],
        kind: Kind::Outdoors,
        difficulty: 0.45,
        build: tent,
    },
    Object {
        name: "Fire Hydrant",
        terms: &["fire hydrant", "hydrant", "fireplug", "fire plug"],
        kind: Kind::Outdoors,
        difficulty: 0.55,
        build: fire_hydrant,
    },
    Object {
        name: "Traffic Cone",
        terms: &["traffic cone", "cone", "pylon", "road cone", "safety cone"],
        kind: Kind::Outdoors,
        difficulty: 0.4,
        build: traffic_cone,
    },
    Object {
        name: "Mailbox",
        terms: &[
            "mailbox",
            "mail box",
            "postbox",
            "post box",
            "letterbox",
            "letter box",
        ],
        kind: Kind::Outdoors,
        difficulty: 0.55,
        build: mailbox,
    },
    Object {
        name: "Park Bench",
        terms: &["park bench", "bench"],
        kind: Kind::Outdoors,
        difficulty: 0.45,
        build: bench,
    },
    Object {
        name: "Tree",
        terms: &["tree", "oak", "oak tree"],
        kind: Kind::Outdoors,
        difficulty: 0.3,
        build: tree,
    },
    Object {
        name: "Cactus",
        terms: &["cactus", "cacti", "saguaro"],
        kind: Kind::Outdoors,
        difficulty: 0.38,
        build: cactus,
    },
    Object {
        name: "Snowman",
        terms: &["snowman", "snow man", "frosty"],
        kind: Kind::Outdoors,
        difficulty: 0.36,
        build: snowman,
    },
    Object {
        name: "Mushroom",
        terms: &["mushroom", "toadstool", "fungus"],
        kind: Kind::Food,
        difficulty: 0.4,
        build: mushroom,
    },
    Object {
        name: "Ice Cream",
        terms: &[
            "ice cream",
            "ice cream cone",
            "icecream",
            "gelato",
            "sundae",
        ],
        kind: Kind::Food,
        difficulty: 0.42,
        build: ice_cream,
    },
    Object {
        name: "Apple",
        terms: &["apple"],
        kind: Kind::Food,
        difficulty: 0.4,
        build: apple,
    },
    Object {
        name: "Carrot",
        terms: &["carrot"],
        kind: Kind::Food,
        difficulty: 0.45,
        build: carrot,
    },
    Object {
        name: "Pineapple",
        terms: &["pineapple", "ananas"],
        kind: Kind::Food,
        difficulty: 0.45,
        build: pineapple,
    },
];

fn chair() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 0.95, 0.0, 1.1, 0.14, 1.05, WOOD),
        block(-0.47, 1.65, -0.46, 0.12, 1.4, 0.12, DARK_WOOD),
        block(0.47, 1.65, -0.46, 0.12, 1.4, 0.12, DARK_WOOD),
        block(0.0, 2.15, -0.46, 1.06, 0.36, 0.09, WOOD),
        block(0.0, 1.6, -0.46, 1.0, 0.14, 0.08, WOOD),
        block(0.0, 0.35, 0.0, 0.9, 0.06, 0.06, DARK_WOOD),
    ];
    for (x, z) in [(-0.45, -0.44), (0.45, -0.44), (-0.45, 0.44), (0.45, 0.44)] {
        parts.push(tube(x, 0.45, z, 0.11, 0.9, DARK_WOOD));
    }
    parts
}

fn table() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 1.5, 0.0, 3.2, 0.16, 1.9, WOOD),
        block(0.0, 1.32, 0.0, 2.9, 0.2, 1.6, DARK_WOOD),
        tube(0.4, 1.78, 0.1, 0.35, 0.4, BLUE),
        ball(0.4, 2.1, 0.1, 0.5, 0.45, 0.5, PINK),
        tube(-0.8, 1.6, -0.2, 0.7, 0.04, WHITE),
    ];
    for (x, z) in [(-1.4, -0.75), (1.4, -0.75), (-1.4, 0.75), (1.4, 0.75)] {
        parts.push(block(x, 0.71, z, 0.16, 1.42, 0.16, DARK_WOOD));
    }
    parts
}

fn lamp() -> Vec<Part> {
    vec![
        tube(0.0, 0.07, 0.0, 1.1, 0.14, BLACK),
        ball(0.0, 0.2, 0.0, 0.3, 0.2, 0.3, BLACK),
        tube(0.32, 0.95, 0.0, 0.09, 1.6, STEEL).turn(0.0, 0.0, -0.42),
        ball(0.65, 1.7, 0.0, 0.2, 0.2, 0.2, BLACK),
        tube(0.2, 2.2, 0.0, 0.09, 1.3, STEEL).turn(0.0, 0.0, 0.95),
        ball(-0.3, 2.55, 0.0, 0.18, 0.18, 0.18, BLACK),
        cone(-0.6, 2.25, 0.0, 0.85, 0.7, RED).turn(0.0, 0.0, -0.8),
        ball(-0.85, 2.0, 0.0, 0.3, 0.3, 0.3, YELLOW),
        block(0.25, 0.15, 0.0, 0.18, 0.08, 0.12, WHITE),
    ]
}

fn teapot() -> Vec<Part> {
    vec![
        tube(0.0, 0.08, 0.0, 1.2, 0.16, WHITE),
        ball(0.0, 0.85, 0.0, 1.8, 1.4, 1.8, SKY),
        ball(0.0, 1.55, 0.0, 0.9, 0.32, 0.9, WHITE),
        ball(0.0, 1.78, 0.0, 0.24, 0.24, 0.24, BLUE),
        tube(1.15, 1.15, 0.0, 0.22, 1.1, SKY).turn(0.0, 0.0, -0.85),
        cone(1.6, 1.55, 0.0, 0.3, 0.2, SKY).turn(0.0, 0.0, -0.85),
        ring(-0.98, 0.95, 0.0, 0.85, 0.14, BLUE).turn(Q, 0.0, 0.0),
        ring(0.0, 0.85, 0.0, 1.82, 0.08, BLUE),
    ]
}

fn mug() -> Vec<Part> {
    vec![
        tube(0.0, 0.65, 0.0, 1.1, 1.3, RED),
        ring(0.68, 0.68, 0.0, 0.7, 0.15, RED).turn(Q, 0.0, 0.0),
        tube(0.0, 1.26, 0.0, 0.98, 0.04, BROWN),
        ring(0.0, 1.3, 0.0, 1.1, 0.06, WHITE),
        block(0.0, 0.8, 0.56, 0.4, 0.4, 0.02, WHITE),
    ]
}

fn umbrella() -> Vec<Part> {
    let mut parts = vec![
        cone(0.0, 2.65, 0.0, 3.4, 0.9, PURPLE),
        tube(0.0, 1.45, 0.0, 0.08, 2.9, BLACK),
        tube(0.0, 3.2, 0.0, 0.06, 0.35, BLACK),
        ring(0.2, 0.02, 0.0, 0.42, 0.09, BLACK).turn(Q, 0.0, 0.0),
    ];
    for i in 0..6 {
        let angle = i as f32 * PI / 3.0;
        parts.push(ball(
            1.62 * angle.cos(),
            2.2,
            1.62 * angle.sin(),
            0.1,
            0.1,
            0.1,
            BLACK,
        ));
    }
    parts
}

fn alarm_clock() -> Vec<Part> {
    vec![
        tube(0.0, 1.2, 0.0, 1.7, 0.5, RED).turn(Q, 0.0, 0.0),
        tube(0.0, 1.2, 0.26, 1.45, 0.04, WHITE).turn(Q, 0.0, 0.0),
        block(0.0, 1.42, 0.3, 0.07, 0.45, 0.02, BLACK),
        block(0.17, 1.2, 0.3, 0.38, 0.06, 0.02, BLACK).turn(0.0, 0.0, 0.3),
        ball(-0.6, 2.05, 0.0, 0.6, 0.4, 0.6, STEEL).turn(0.0, 0.0, 0.5),
        ball(0.6, 2.05, 0.0, 0.6, 0.4, 0.6, STEEL).turn(0.0, 0.0, -0.5),
        tube(0.0, 2.1, 0.0, 0.08, 0.3, STEEL),
        block(0.0, 2.25, 0.0, 0.6, 0.06, 0.08, STEEL),
        tube(-0.55, 0.25, 0.0, 0.1, 0.5, STEEL).turn(0.0, 0.0, -0.4),
        tube(0.55, 0.25, 0.0, 0.1, 0.5, STEEL).turn(0.0, 0.0, 0.4),
    ]
}

fn light_bulb() -> Vec<Part> {
    vec![
        ball(0.0, 2.05, 0.0, 1.5, 1.5, 1.5, YELLOW),
        tube(0.0, 1.25, 0.0, 0.75, 0.6, YELLOW),
        tube(0.0, 0.8, 0.0, 0.62, 0.5, STEEL),
        ring(0.0, 0.62, 0.0, 0.66, 0.08, STEEL),
        ring(0.0, 0.8, 0.0, 0.66, 0.08, STEEL),
        ring(0.0, 0.98, 0.0, 0.66, 0.08, STEEL),
        cone(0.0, 0.42, 0.0, 0.5, 0.2, BLACK).turn(PI, 0.0, 0.0),
        ring(0.0, 2.0, 0.0, 0.35, 0.04, ORANGE).turn(Q, 0.0, 0.0),
        tube(-0.12, 1.65, 0.0, 0.03, 0.5, GREY),
        tube(0.12, 1.65, 0.0, 0.03, 0.5, GREY),
    ]
}

fn bed() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 0.5, 0.0, 2.3, 0.4, 3.6, DARK_WOOD),
        block(0.0, 0.88, 0.05, 2.15, 0.36, 3.4, WHITE),
        block(0.0, 1.3, -1.82, 2.4, 1.9, 0.16, WOOD),
        block(0.0, 0.85, 1.82, 2.4, 0.9, 0.14, WOOD),
        ball(-0.5, 1.15, -1.3, 0.85, 0.3, 0.55, WHITE),
        ball(0.5, 1.15, -1.3, 0.85, 0.3, 0.55, WHITE),
        block(0.0, 1.08, 0.5, 2.22, 0.08, 2.3, BLUE),
    ];
    for (x, z) in [(-1.05, -1.7), (1.05, -1.7), (-1.05, 1.7), (1.05, 1.7)] {
        parts.push(block(x, 0.15, z, 0.18, 0.3, 0.18, DARK_WOOD));
    }
    parts
}

fn sofa() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 0.62, 0.0, 3.3, 0.5, 1.4, GREEN),
        block(0.0, 1.3, -0.55, 3.3, 1.05, 0.38, GREEN),
        block(-1.55, 1.0, 0.0, 0.32, 0.85, 1.4, GREEN),
        block(1.55, 1.0, 0.0, 0.32, 0.85, 1.4, GREEN),
        block(-0.68, 0.98, 0.12, 1.32, 0.22, 1.1, LEAF),
        block(0.68, 0.98, 0.12, 1.32, 0.22, 1.1, LEAF),
        block(-0.68, 1.45, -0.3, 1.3, 0.75, 0.2, LEAF),
        block(0.68, 1.45, -0.3, 1.3, 0.75, 0.2, LEAF),
        ball(1.1, 1.3, 0.05, 0.6, 0.55, 0.25, YELLOW).turn(0.3, 0.0, 0.2),
    ];
    for (x, z) in [(-1.5, -0.55), (1.5, -0.55), (-1.5, 0.55), (1.5, 0.55)] {
        parts.push(cone(x, 0.18, z, 0.18, 0.36, DARK_WOOD).turn(PI, 0.0, 0.0));
    }
    parts
}

fn piano() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 1.55, 0.0, 3.0, 2.5, 0.9, BLACK),
        block(0.0, 1.25, 0.66, 3.0, 0.16, 0.5, BLACK),
        block(0.0, 1.36, 0.66, 2.7, 0.06, 0.42, WHITE),
        block(-1.42, 0.62, 0.62, 0.14, 1.24, 0.14, BLACK),
        block(1.42, 0.62, 0.62, 0.14, 1.24, 0.14, BLACK),
        block(0.0, 2.15, 0.48, 1.4, 0.7, 0.04, CREAM),
        block(0.0, 0.7, 1.7, 1.8, 0.14, 0.7, BLACK),
        block(-0.8, 0.35, 1.7, 0.12, 0.7, 0.6, BLACK),
        block(0.8, 0.35, 1.7, 0.12, 0.7, 0.6, BLACK),
    ];
    for i in 0..12 {
        if i % 7 == 2 || i % 7 == 6 {
            continue;
        }
        let x = -1.25 + i as f32 * 0.225;
        parts.push(block(x, 1.42, 0.58, 0.1, 0.06, 0.24, BLACK));
    }
    parts
}

fn guitar() -> Vec<Part> {
    let mut parts = vec![
        ball(0.0, 0.95, 0.0, 1.55, 1.55, 0.38, ORANGE),
        ball(0.0, 2.0, 0.0, 1.15, 1.1, 0.38, ORANGE),
        tube(0.0, 1.45, 0.19, 0.45, 0.04, BLACK).turn(Q, 0.0, 0.0),
        block(0.0, 3.25, 0.05, 0.24, 2.2, 0.12, DARK_WOOD),
        block(0.0, 4.55, 0.03, 0.36, 0.55, 0.1, DARK_WOOD),
        block(0.0, 0.75, 0.2, 0.55, 0.1, 0.06, BLACK),
    ];
    for i in 0..6 {
        let x = -0.1 + i as f32 * 0.04;
        parts.push(block(x, 2.6, 0.13, 0.012, 3.7, 0.012, STEEL));
    }
    for i in 0..3 {
        parts.push(tube(-0.2, 4.4 + i as f32 * 0.16, 0.03, 0.06, 0.14, STEEL).turn(0.0, 0.0, Q));
        parts.push(tube(0.2, 4.4 + i as f32 * 0.16, 0.03, 0.06, 0.14, STEEL).turn(0.0, 0.0, Q));
    }
    parts
}

fn ladder() -> Vec<Part> {
    let mut parts = vec![
        block(-0.55, 2.2, 0.0, 0.14, 4.4, 0.14, YELLOW),
        block(0.55, 2.2, 0.0, 0.14, 4.4, 0.14, YELLOW),
    ];
    for i in 0..8 {
        parts.push(tube(0.0, 0.45 + i as f32 * 0.5, 0.0, 0.09, 1.1, STEEL).turn(0.0, 0.0, Q));
    }
    parts
}

fn watering_can() -> Vec<Part> {
    vec![
        tube(0.0, 0.6, 0.0, 1.3, 1.2, GREEN),
        ring(0.0, 1.2, 0.0, 1.3, 0.08, LEAF),
        tube(1.05, 1.05, 0.0, 0.14, 1.6, GREEN).turn(0.0, 0.0, -0.95),
        cone(1.72, 1.55, 0.0, 0.42, 0.3, LEAF).turn(0.0, 0.0, -0.95 + PI),
        tube(1.8, 1.6, 0.0, 0.42, 0.04, STEEL).turn(0.0, 0.0, -0.95),
        ring(-0.1, 1.45, 0.0, 1.1, 0.12, LEAF).turn(Q, 0.0, 0.0),
        ring(-0.62, 0.8, 0.0, 0.6, 0.1, LEAF).turn(Q, 0.0, Q),
    ]
}

fn hammer() -> Vec<Part> {
    vec![
        block(0.15, 2.5, 0.0, 0.9, 0.36, 0.36, STEEL),
        tube(0.65, 2.5, 0.0, 0.36, 0.2, STEEL).turn(0.0, 0.0, Q),
        block(-0.5, 2.62, 0.08, 0.55, 0.18, 0.12, STEEL).turn(0.0, 0.0, -0.45),
        block(-0.5, 2.62, -0.08, 0.55, 0.18, 0.12, STEEL).turn(0.0, 0.0, -0.45),
        tube(0.15, 1.3, 0.0, 0.22, 2.2, WOOD),
        tube(0.15, 0.45, 0.0, 0.27, 0.8, RED),
    ]
}

fn key() -> Vec<Part> {
    vec![
        ring(0.0, 1.0, 0.0, 1.1, 0.2, GOLD).turn(Q, 0.0, 0.0),
        tube(0.68, 1.0, 0.0, 0.2, 0.25, GOLD).turn(0.0, 0.0, Q),
        block(1.75, 1.0, 0.0, 2.0, 0.18, 0.1, GOLD),
        block(2.45, 0.78, 0.0, 0.18, 0.3, 0.1, GOLD),
        block(2.15, 0.82, 0.0, 0.14, 0.22, 0.1, GOLD),
        block(1.85, 0.78, 0.0, 0.16, 0.3, 0.1, GOLD),
        block(2.72, 0.85, 0.0, 0.1, 0.18, 0.1, GOLD),
    ]
}

fn hourglass() -> Vec<Part> {
    let mut parts = vec![
        tube(0.0, 0.08, 0.0, 1.45, 0.16, WOOD),
        tube(0.0, 2.52, 0.0, 1.45, 0.16, WOOD),
        cone(0.0, 0.72, 0.0, 1.1, 1.15, SKY),
        cone(0.0, 1.88, 0.0, 1.1, 1.15, SKY).turn(PI, 0.0, 0.0),
        cone(0.0, 0.45, 0.0, 0.85, 0.55, YELLOW),
        cone(0.0, 1.7, 0.0, 0.55, 0.42, YELLOW).turn(PI, 0.0, 0.0),
        tube(0.0, 1.0, 0.0, 0.03, 0.6, YELLOW),
    ];
    for i in 0..3 {
        let angle = i as f32 * 2.0 * PI / 3.0;
        parts.push(tube(
            0.6 * angle.cos(),
            1.3,
            0.6 * angle.sin(),
            0.08,
            2.3,
            DARK_WOOD,
        ));
    }
    parts
}

fn trophy() -> Vec<Part> {
    vec![
        block(0.0, 0.3, 0.0, 1.2, 0.6, 1.2, DARK_WOOD),
        block(0.0, 0.3, 0.61, 0.7, 0.25, 0.02, GOLD),
        tube(0.0, 0.7, 0.0, 0.7, 0.2, GOLD),
        tube(0.0, 1.1, 0.0, 0.22, 0.6, GOLD),
        ball(0.0, 1.45, 0.0, 0.4, 0.25, 0.4, GOLD),
        cone(0.0, 2.05, 0.0, 1.5, 1.3, GOLD).turn(PI, 0.0, 0.0),
        ring(0.0, 2.68, 0.0, 1.5, 0.1, GOLD),
        ring(-0.8, 2.15, 0.0, 0.7, 0.1, GOLD).turn(Q, 0.0, 0.0),
        ring(0.8, 2.15, 0.0, 0.7, 0.1, GOLD).turn(Q, 0.0, 0.0),
    ]
}

fn car() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 0.8, 0.0, 1.9, 0.7, 4.2, RED),
        block(0.0, 1.45, -0.25, 1.65, 0.62, 2.1, RED),
        block(0.0, 1.45, -0.25, 1.68, 0.44, 1.8, SKY),
        block(0.0, 1.45, 0.82, 1.5, 0.44, 0.04, SKY).turn(-0.5, 0.0, 0.0),
        block(0.0, 0.55, 2.12, 1.95, 0.22, 0.12, STEEL),
        block(0.0, 0.55, -2.12, 1.95, 0.22, 0.12, STEEL),
        ball(-0.65, 0.9, 2.08, 0.32, 0.22, 0.1, YELLOW),
        ball(0.65, 0.9, 2.08, 0.32, 0.22, 0.1, YELLOW),
        block(-0.7, 0.95, -2.1, 0.3, 0.18, 0.06, ORANGE),
        block(0.7, 0.95, -2.1, 0.3, 0.18, 0.06, ORANGE),
    ];
    for (x, z) in [(-0.95, 1.35), (0.95, 1.35), (-0.95, -1.35), (0.95, -1.35)] {
        parts.push(tube(x, 0.45, z, 0.9, 0.34, BLACK).turn(0.0, 0.0, Q));
        parts.push(tube(x * 1.02, 0.45, z, 0.45, 0.36, STEEL).turn(0.0, 0.0, Q));
    }
    parts
}

fn bicycle() -> Vec<Part> {
    vec![
        ring(0.0, 0.85, 1.15, 1.7, 0.1, BLACK).turn(0.0, 0.0, Q),
        ring(0.0, 0.85, -1.15, 1.7, 0.1, BLACK).turn(0.0, 0.0, Q),
        tube(0.0, 0.85, 1.15, 0.16, 0.12, STEEL).turn(0.0, 0.0, Q),
        tube(0.0, 0.85, -1.15, 0.16, 0.12, STEEL).turn(0.0, 0.0, Q),
        tube(0.0, 1.6, 0.15, 0.1, 1.4, RED).turn(Q, 0.0, 0.0),
        tube(0.0, 1.1, 0.27, 0.1, 1.39, RED).turn(1.12, 0.0, 0.0),
        tube(0.0, 1.27, -0.47, 0.1, 1.0, RED).turn(-0.26, 0.0, 0.0),
        tube(0.0, 0.83, -0.75, 0.08, 0.8, RED).turn(Q, 0.0, 0.0),
        tube(0.0, 1.22, -0.85, 0.08, 0.96, RED).turn(0.67, 0.0, 0.0),
        tube(0.0, 1.07, 1.05, 0.09, 0.5, STEEL).turn(-0.42, 0.0, 0.0),
        tube(0.0, 1.48, 0.9, 0.12, 0.46, RED).turn(-0.2, 0.0, 0.0),
        tube(0.0, 1.85, 0.85, 0.08, 1.0, BLACK).turn(0.0, 0.0, Q),
        block(0.0, 1.83, -0.62, 0.3, 0.1, 0.5, BLACK),
        ring(0.0, 0.8, -0.35, 0.45, 0.06, GREY).turn(0.0, 0.0, Q),
        block(0.25, 0.8, -0.35, 0.05, 0.05, 0.6, GREY).turn(0.9, 0.0, 0.0),
    ]
}

fn airplane() -> Vec<Part> {
    vec![
        tube(0.0, 1.6, 0.0, 1.0, 6.0, WHITE).turn(Q, 0.0, 0.0),
        ball(0.0, 1.6, 3.0, 1.0, 1.0, 1.4, WHITE),
        cone(0.0, 1.75, -3.65, 1.0, 1.3, WHITE).turn(-Q, 0.0, 0.0),
        block(0.0, 1.35, 0.3, 7.6, 0.12, 1.4, STEEL),
        block(0.0, 1.8, -3.6, 2.6, 0.08, 0.7, STEEL),
        wedge(0.0, 2.6, -3.75, 1.0, 1.4, 0.08, RED).turn(0.0, -Q, 0.0),
        tube(-1.6, 1.05, 0.55, 0.45, 1.0, GREY).turn(Q, 0.0, 0.0),
        tube(1.6, 1.05, 0.55, 0.45, 1.0, GREY).turn(Q, 0.0, 0.0),
        block(0.0, 1.85, 3.15, 0.7, 0.22, 0.3, SKY),
        block(-0.48, 1.75, 0.0, 0.05, 0.16, 4.0, BLUE),
        block(0.48, 1.75, 0.0, 0.05, 0.16, 4.0, BLUE),
        tube(0.0, 0.6, 2.4, 0.08, 0.6, BLACK),
        tube(0.0, 0.3, 2.4, 0.35, 0.15, BLACK).turn(0.0, 0.0, Q),
    ]
}

fn helicopter() -> Vec<Part> {
    vec![
        ball(0.0, 1.55, 0.3, 1.9, 1.7, 2.8, YELLOW),
        ball(0.0, 1.7, 1.15, 1.5, 1.1, 1.3, SKY),
        tube(0.0, 1.9, -2.55, 0.32, 3.4, YELLOW).turn(Q, 0.0, 0.0),
        wedge(0.0, 2.35, -4.1, 0.6, 0.9, 0.08, YELLOW).turn(0.0, -Q, 0.0),
        block(0.12, 2.1, -4.15, 0.04, 0.9, 0.12, BLACK),
        tube(0.0, 2.6, 0.3, 0.16, 0.5, GREY),
        block(0.0, 2.88, 0.3, 7.0, 0.05, 0.28, BLACK),
        block(0.0, 2.88, 0.3, 0.28, 0.05, 7.0, BLACK),
        tube(-0.85, 0.22, 0.3, 0.12, 3.0, BLACK).turn(Q, 0.0, 0.0),
        tube(0.85, 0.22, 0.3, 0.12, 3.0, BLACK).turn(Q, 0.0, 0.0),
        tube(-0.6, 0.55, 0.9, 0.08, 0.75, BLACK).turn(0.0, 0.0, 0.6),
        tube(0.6, 0.55, 0.9, 0.08, 0.75, BLACK).turn(0.0, 0.0, -0.6),
        tube(-0.6, 0.55, -0.3, 0.08, 0.75, BLACK).turn(0.0, 0.0, 0.6),
        tube(0.6, 0.55, -0.3, 0.08, 0.75, BLACK).turn(0.0, 0.0, -0.6),
    ]
}

fn sailboat() -> Vec<Part> {
    vec![
        ball(0.0, 0.55, 0.0, 1.5, 1.0, 4.2, WHITE),
        block(0.0, 0.85, -0.2, 1.3, 0.12, 3.2, WOOD),
        block(0.0, 0.32, 0.0, 1.52, 0.18, 4.0, BLUE),
        tube(0.0, 2.9, 0.2, 0.12, 4.2, DARK_WOOD),
        wedge(0.0, 2.95, -0.7, 1.8, 3.6, 0.04, WHITE).turn(0.0, Q, 0.0),
        wedge(0.0, 2.6, 0.9, 1.4, 3.0, 0.04, RED).turn(0.0, -Q, 0.0),
        tube(0.0, 1.25, -0.75, 0.08, 1.9, DARK_WOOD).turn(Q, 0.0, 0.0),
        wedge(0.0, 5.15, 0.0, 0.4, 0.3, 0.02, RED).turn(0.0, -Q, 0.0),
        block(0.0, 1.0, -1.25, 0.9, 0.3, 0.8, WOOD),
    ]
}

fn rocket() -> Vec<Part> {
    let mut parts = vec![
        tube(0.0, 2.45, 0.0, 1.1, 3.6, WHITE),
        cone(0.0, 4.85, 0.0, 1.1, 1.2, RED),
        tube(0.0, 0.5, 0.0, 0.9, 0.3, GREY),
        cone(0.0, 0.25, 0.0, 0.8, 0.5, GREY).turn(PI, 0.0, 0.0),
        ring(0.0, 3.2, 0.52, 0.5, 0.08, STEEL).turn(Q, 0.0, 0.0),
        tube(0.0, 3.2, 0.53, 0.4, 0.04, SKY).turn(Q, 0.0, 0.0),
        ring(0.0, 4.25, 0.0, 1.12, 0.07, RED),
        cone(0.0, -0.25, 0.0, 0.7, 0.9, ORANGE).turn(PI, 0.0, 0.0),
    ];
    for i in 0..4 {
        let angle = i as f32 * Q;
        parts.push(
            wedge(
                0.82 * angle.cos(),
                1.1,
                0.82 * angle.sin(),
                0.6,
                1.3,
                0.06,
                RED,
            )
            .turn(0.0, -angle, 0.0),
        );
    }
    parts
}

fn train() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 0.6, -0.1, 1.5, 0.3, 5.0, BLACK),
        tube(0.0, 1.45, 0.75, 1.35, 3.0, GREEN).turn(Q, 0.0, 0.0),
        tube(0.0, 1.45, 2.27, 1.0, 0.06, GREY).turn(Q, 0.0, 0.0),
        block(0.0, 1.85, -1.6, 1.6, 2.2, 1.6, GREEN),
        block(0.0, 3.0, -1.6, 1.85, 0.12, 1.95, BLACK),
        block(0.0, 2.15, -1.6, 1.62, 0.6, 0.8, SKY),
        tube(0.0, 2.55, 1.6, 0.42, 0.9, BLACK),
        cone(0.0, 3.1, 1.6, 0.75, 0.4, BLACK).turn(PI, 0.0, 0.0),
        ball(0.0, 2.2, 0.45, 0.5, 0.45, 0.5, GOLD),
        wedge(0.0, 0.45, 2.55, 0.6, 0.6, 1.4, RED).turn(0.0, -Q, 0.0),
        ball(0.0, 1.8, 2.3, 0.3, 0.3, 0.12, YELLOW),
    ];
    for z in [1.5, 0.4, -0.7] {
        for x in [-0.78, 0.78] {
            parts.push(tube(x, 0.55, z, 0.95, 0.14, RED).turn(0.0, 0.0, Q));
        }
    }
    parts.push(block(-0.86, 0.55, 0.4, 0.04, 0.1, 2.3, STEEL));
    parts.push(block(0.86, 0.55, 0.4, 0.04, 0.1, 2.3, STEEL));
    parts
}

fn hot_air_balloon() -> Vec<Part> {
    let mut parts = vec![
        ball(0.0, 4.4, 0.0, 3.4, 3.7, 3.4, RED),
        ring(0.0, 4.6, 0.0, 3.35, 0.2, YELLOW),
        ring(0.0, 3.6, 0.0, 3.0, 0.2, YELLOW),
        cone(0.0, 2.35, 0.0, 1.9, 1.3, RED).turn(PI, 0.0, 0.0),
        block(0.0, 0.45, 0.0, 1.0, 0.75, 1.0, WOOD),
        ring(0.0, 0.84, 0.0, 1.2, 0.1, DARK_WOOD),
        tube(0.0, 1.25, 0.0, 0.3, 0.3, STEEL),
    ];
    for (x, z) in [(-0.45, -0.45), (0.45, -0.45), (-0.45, 0.45), (0.45, 0.45)] {
        parts.push(tube(x * 1.25, 1.3, z * 1.25, 0.04, 1.0, BROWN).turn(-z * 0.4, 0.0, x * 0.4));
    }
    parts
}

fn bus() -> Vec<Part> {
    let mut parts = vec![
        block(0.0, 1.45, -0.4, 2.3, 2.1, 6.4, YELLOW),
        block(0.0, 0.95, 3.35, 2.0, 1.1, 1.1, YELLOW),
        block(0.0, 0.5, 3.95, 2.1, 0.22, 0.14, BLACK),
        block(0.0, 2.05, 2.8, 2.1, 0.75, 0.04, SKY),
        block(0.0, 0.85, -0.4, 2.32, 0.1, 6.4, BLACK),
        block(0.0, 2.6, -0.4, 2.2, 0.1, 6.3, YELLOW),
        block(1.18, 1.55, 2.4, 0.04, 1.3, 0.7, BLACK),
        ball(-0.7, 1.15, 3.92, 0.3, 0.3, 0.08, WHITE),
        ball(0.7, 1.15, 3.92, 0.3, 0.3, 0.08, WHITE),
        tube(-1.25, 1.7, 2.0, 0.5, 0.04, RED).turn(0.0, 0.0, Q),
    ];
    for i in 0..6 {
        let z = 1.6 - i as f32 * 0.95;
        parts.push(block(-1.16, 2.0, z, 0.02, 0.6, 0.75, SKY));
        parts.push(block(1.16, 2.0, z, 0.02, 0.6, 0.75, SKY));
    }
    for (x, z) in [(-1.05, 2.4), (1.05, 2.4), (-1.05, -2.6), (1.05, -2.6)] {
        parts.push(tube(x, 0.48, z, 0.95, 0.3, BLACK).turn(0.0, 0.0, Q));
    }
    parts
}

fn submarine() -> Vec<Part> {
    let mut parts = vec![
        tube(0.0, 1.2, 0.0, 1.6, 4.4, YELLOW).turn(Q, 0.0, 0.0),
        ball(0.0, 1.2, 2.2, 1.6, 1.6, 1.4, YELLOW),
        cone(0.0, 1.2, -2.8, 1.6, 1.4, YELLOW).turn(-Q, 0.0, 0.0),
        block(0.0, 2.3, 0.4, 0.6, 0.8, 1.4, YELLOW),
        tube(0.0, 3.0, 0.8, 0.1, 0.8, GREY),
        tube(0.0, 3.35, 0.95, 0.1, 0.35, GREY).turn(Q, 0.0, 0.0),
        block(0.0, 1.2, -3.55, 0.06, 0.9, 0.18, GREY),
        block(0.0, 1.2, -3.55, 0.9, 0.06, 0.18, GREY),
        wedge(0.0, 2.05, -2.6, 0.8, 0.7, 0.06, YELLOW).turn(0.0, -Q, 0.0),
        block(0.0, 1.2, -2.5, 2.4, 0.05, 0.5, YELLOW),
    ];
    for z in [1.4, 0.5, -0.4, -1.3] {
        parts.push(ring(0.8, 1.3, z, 0.42, 0.07, GREY).turn(0.0, 0.0, Q));
        parts.push(ring(-0.8, 1.3, z, 0.42, 0.07, GREY).turn(0.0, 0.0, Q));
    }
    parts
}

fn duck() -> Vec<Part> {
    vec![
        ball(0.0, 0.75, 0.0, 1.7, 1.2, 2.2, YELLOW),
        ball(0.0, 1.85, 0.7, 1.05, 1.0, 1.0, YELLOW),
        ball(0.0, 1.72, 1.32, 0.6, 0.2, 0.55, ORANGE),
        cone(0.0, 1.25, -1.05, 0.5, 0.7, YELLOW).turn(-1.0, 0.0, 0.0),
        ball(-0.82, 0.95, -0.1, 0.18, 0.6, 1.1, ORANGE),
        ball(0.82, 0.95, -0.1, 0.18, 0.6, 1.1, ORANGE),
        ball(-0.33, 2.0, 1.12, 0.16, 0.2, 0.08, BLACK),
        ball(0.33, 2.0, 1.12, 0.16, 0.2, 0.08, BLACK),
    ]
}

fn penguin() -> Vec<Part> {
    vec![
        ball(0.0, 1.3, 0.0, 1.4, 2.4, 1.2, BLACK),
        ball(0.0, 1.2, 0.22, 1.05, 1.95, 0.85, WHITE),
        ball(0.0, 2.65, 0.05, 0.95, 0.9, 0.9, BLACK),
        ball(0.0, 2.55, 0.32, 0.6, 0.5, 0.4, WHITE),
        cone(0.0, 2.6, 0.62, 0.22, 0.4, ORANGE).turn(Q, 0.0, 0.0),
        ball(-0.2, 2.78, 0.43, 0.14, 0.16, 0.06, BLACK),
        ball(0.2, 2.78, 0.43, 0.14, 0.16, 0.06, BLACK),
        ball(-0.75, 1.45, 0.0, 0.18, 1.3, 0.5, BLACK).turn(0.0, 0.0, -0.35),
        ball(0.75, 1.45, 0.0, 0.18, 1.3, 0.5, BLACK).turn(0.0, 0.0, 0.35),
        ball(-0.3, 0.07, 0.35, 0.45, 0.14, 0.7, ORANGE),
        ball(0.3, 0.07, 0.35, 0.45, 0.14, 0.7, ORANGE),
    ]
}

fn giraffe() -> Vec<Part> {
    let mut parts = vec![
        ball(0.0, 2.5, 0.0, 1.35, 1.3, 2.6, YELLOW),
        tube(0.0, 4.0, 1.25, 0.42, 2.8, YELLOW).turn(0.45, 0.0, 0.0),
        ball(0.0, 5.3, 1.95, 0.55, 0.55, 1.05, YELLOW),
        ball(0.0, 5.2, 2.45, 0.42, 0.38, 0.35, CREAM),
        tube(-0.14, 5.75, 1.75, 0.08, 0.4, BROWN),
        tube(0.14, 5.75, 1.75, 0.08, 0.4, BROWN),
        ball(-0.14, 5.97, 1.75, 0.14, 0.14, 0.14, BROWN),
        ball(0.14, 5.97, 1.75, 0.14, 0.14, 0.14, BROWN),
        ball(-0.33, 5.5, 1.6, 0.3, 0.1, 0.16, YELLOW),
        ball(0.33, 5.5, 1.6, 0.3, 0.1, 0.16, YELLOW),
        tube(0.0, 2.3, -1.4, 0.07, 1.2, BROWN).turn(-0.3, 0.0, 0.0),
        block(0.0, 4.1, 0.95, 0.12, 2.6, 0.2, BROWN).turn(0.45, 0.0, 0.0),
    ];
    for (x, z) in [(-0.42, 0.85), (0.42, 0.85), (-0.42, -0.85), (0.42, -0.85)] {
        parts.push(tube(x, 1.0, z, 0.22, 2.0, YELLOW));
        parts.push(tube(x, 0.08, z, 0.26, 0.16, BROWN));
    }
    for (x, y, z) in [
        (0.66, 2.6, 0.5),
        (0.6, 2.3, -0.5),
        (-0.66, 2.6, 0.2),
        (-0.6, 2.4, -0.7),
        (0.0, 3.12, 0.0),
        (0.0, 3.05, -0.8),
    ] {
        parts.push(ball(x, y, z, 0.32, 0.32, 0.32, BROWN));
    }
    parts
}

fn snail() -> Vec<Part> {
    vec![
        ball(0.0, 0.25, 0.4, 0.8, 0.45, 2.8, CREAM),
        ball(0.0, 0.55, 1.65, 0.6, 0.6, 0.6, CREAM),
        ball(0.0, 1.35, -0.2, 0.95, 1.7, 1.7, BROWN),
        ring(0.0, 1.35, -0.2, 1.2, 0.3, ORANGE).turn(0.0, 0.0, Q),
        ring(0.0, 1.4, -0.15, 0.55, 0.28, WOOD).turn(0.0, 0.0, Q),
        tube(-0.18, 1.1, 1.85, 0.07, 0.8, CREAM).turn(0.4, 0.0, 0.3),
        tube(0.18, 1.1, 1.85, 0.07, 0.8, CREAM).turn(0.4, 0.0, -0.3),
        ball(-0.3, 1.5, 2.0, 0.16, 0.16, 0.16, BLACK),
        ball(0.3, 1.5, 2.0, 0.16, 0.16, 0.16, BLACK),
    ]
}

fn turtle() -> Vec<Part> {
    let mut parts = vec![
        ball(0.0, 0.75, 0.0, 2.2, 1.3, 2.6, GREEN),
        block(0.0, 0.42, 0.0, 2.0, 0.14, 2.3, CREAM),
        ball(0.0, 0.85, 1.65, 0.65, 0.6, 0.8, LEAF),
        ball(-0.2, 1.0, 1.95, 0.1, 0.12, 0.08, BLACK),
        ball(0.2, 1.0, 1.95, 0.1, 0.12, 0.08, BLACK),
        cone(0.0, 0.55, -1.4, 0.25, 0.5, LEAF).turn(-Q, 0.0, 0.0),
        ring(0.0, 1.2, 0.0, 1.3, 0.12, LEAF),
        ring(0.0, 0.9, 0.0, 2.05, 0.12, LEAF),
    ];
    for (x, z) in [(-0.95, 0.85), (0.95, 0.85), (-0.95, -0.85), (0.95, -0.85)] {
        parts.push(ball(x, 0.25, z, 0.5, 0.45, 0.6, LEAF));
    }
    parts
}

fn fish() -> Vec<Part> {
    vec![
        ball(0.0, 1.4, 0.0, 0.75, 1.5, 2.6, ORANGE),
        prism(0.0, 1.4, -1.6, 1.4, 1.0, 0.05, ORANGE).turn(Q, 0.0, Q),
        wedge(0.0, 2.25, -0.2, 1.0, 0.6, 0.05, ORANGE).turn(0.0, Q, 0.0),
        ball(-0.3, 1.0, 0.3, 0.06, 0.4, 0.6, ORANGE).turn(0.5, 0.0, 0.0),
        ball(0.3, 1.0, 0.3, 0.06, 0.4, 0.6, ORANGE).turn(0.5, 0.0, 0.0),
        ball(-0.3, 1.6, 0.85, 0.12, 0.26, 0.26, WHITE),
        ball(0.3, 1.6, 0.85, 0.12, 0.26, 0.26, WHITE),
        ball(-0.36, 1.6, 0.88, 0.06, 0.13, 0.13, BLACK),
        ball(0.36, 1.6, 0.88, 0.06, 0.13, 0.13, BLACK),
        ring(0.0, 1.25, 1.25, 0.3, 0.06, RED).turn(Q, 0.0, 0.0),
    ]
}

fn octopus() -> Vec<Part> {
    let mut parts = vec![
        ball(0.0, 2.2, -0.15, 1.8, 2.0, 1.9, PURPLE),
        ball(-0.35, 1.9, 0.75, 0.38, 0.45, 0.2, WHITE),
        ball(0.35, 1.9, 0.75, 0.38, 0.45, 0.2, WHITE),
        ball(-0.35, 1.88, 0.84, 0.16, 0.22, 0.08, BLACK),
        ball(0.35, 1.88, 0.84, 0.16, 0.22, 0.08, BLACK),
    ];
    for i in 0..8 {
        let angle = i as f32 * PI / 4.0 + PI / 8.0;
        let (c, s) = (angle.cos(), angle.sin());
        parts.push(tube(1.05 * c, 0.8, 1.05 * s, 0.32, 1.5, PURPLE).turn(-s * 0.75, 0.0, c * 0.75));
        parts.push(ball(1.6 * c, 0.2, 1.6 * s, 0.3, 0.3, 0.3, PINK));
    }
    parts
}

fn lighthouse() -> Vec<Part> {
    let mut parts = vec![
        ball(0.0, 0.0, 0.0, 3.2, 1.2, 3.0, GREY),
        tube(0.0, 2.8, 0.0, 1.5, 5.0, WHITE),
        tube(0.0, 5.42, 0.0, 2.1, 0.18, BLACK),
        ring(0.0, 5.85, 0.0, 2.0, 0.06, BLACK),
        tube(0.0, 5.9, 0.0, 1.0, 0.9, YELLOW),
        cone(0.0, 6.75, 0.0, 1.35, 0.85, RED),
        ball(0.0, 7.25, 0.0, 0.2, 0.2, 0.2, RED),
        block(0.0, 1.0, 0.76, 0.5, 0.9, 0.06, BROWN),
    ];
    for y in [1.6, 3.2, 4.7] {
        parts.push(tube(0.0, y, 0.0, 1.53, 0.55, RED));
    }
    parts
}

fn windmill() -> Vec<Part> {
    let mut parts = vec![
        tube(0.0, 2.0, 0.0, 2.3, 4.0, CREAM),
        cone(0.0, 4.6, 0.0, 2.6, 1.3, RED),
        block(0.0, 0.7, 1.16, 0.7, 1.3, 0.06, DARK_WOOD),
        block(0.0, 2.8, 1.16, 0.45, 0.5, 0.06, SKY),
        tube(0.0, 4.2, 1.25, 0.3, 0.6, DARK_WOOD).turn(Q, 0.0, 0.0),
    ];
    for i in 0..4 {
        let angle = i as f32 * Q + 0.35;
        let (c, s) = (angle.cos(), angle.sin());
        parts.push(
            block(1.55 * s, 4.2 + 1.55 * c, 1.6, 0.6, 2.7, 0.05, WOOD).turn(0.0, 0.0, -angle),
        );
        parts.push(
            block(0.8 * s, 4.2 + 0.8 * c, 1.62, 0.08, 1.6, 0.08, DARK_WOOD).turn(0.0, 0.0, -angle),
        );
    }
    parts
}

fn house() -> Vec<Part> {
    vec![
        block(0.0, 1.1, 0.0, 3.4, 2.2, 2.6, CREAM),
        prism(0.0, 2.85, 0.0, 3.9, 1.3, 3.0, RED),
        block(1.0, 3.1, -0.5, 0.45, 1.2, 0.45, BROWN),
        block(0.0, 0.65, 1.31, 0.75, 1.3, 0.06, DARK_WOOD),
        ball(0.22, 0.65, 1.35, 0.08, 0.08, 0.08, GOLD),
        block(-1.05, 1.35, 1.31, 0.7, 0.7, 0.06, SKY),
        block(1.05, 1.35, 1.31, 0.7, 0.7, 0.06, SKY),
        block(-1.05, 1.35, 1.34, 0.06, 0.7, 0.02, WHITE),
        block(1.05, 1.35, 1.34, 0.06, 0.7, 0.02, WHITE),
        block(0.0, 0.04, 1.75, 0.8, 0.08, 0.8, GREY),
    ]
}

fn tent() -> Vec<Part> {
    vec![
        prism(0.0, 1.0, 0.0, 2.6, 2.0, 3.2, ORANGE),
        prism(0.0, 0.7, 1.61, 1.1, 1.4, 0.02, BROWN),
        tube(0.0, 1.15, 1.75, 0.07, 2.3, GREY),
        tube(0.0, 1.15, -1.75, 0.07, 2.3, GREY),
        tube(0.0, 2.05, 2.4, 0.03, 1.6, CREAM).turn(-0.9, 0.0, 0.0),
        tube(0.0, 2.05, -2.4, 0.03, 1.6, CREAM).turn(0.9, 0.0, 0.0),
        block(0.0, 0.03, 0.0, 2.9, 0.06, 3.6, GREEN),
    ]
}

fn fire_hydrant() -> Vec<Part> {
    vec![
        tube(0.0, 0.1, 0.0, 1.15, 0.2, RED),
        tube(0.0, 0.95, 0.0, 0.85, 1.55, RED),
        ring(0.0, 1.72, 0.0, 0.95, 0.12, RED),
        ball(0.0, 1.8, 0.0, 0.85, 0.65, 0.85, RED),
        tube(0.0, 2.15, 0.0, 0.28, 0.25, GREY),
        tube(-0.55, 1.2, 0.0, 0.38, 0.4, RED).turn(0.0, 0.0, Q),
        tube(0.55, 1.2, 0.0, 0.38, 0.4, RED).turn(0.0, 0.0, Q),
        tube(-0.78, 1.2, 0.0, 0.26, 0.1, GREY).turn(0.0, 0.0, Q),
        tube(0.78, 1.2, 0.0, 0.26, 0.1, GREY).turn(0.0, 0.0, Q),
        tube(0.0, 1.05, 0.5, 0.5, 0.35, RED).turn(Q, 0.0, 0.0),
        tube(0.0, 1.05, 0.7, 0.32, 0.1, GREY).turn(Q, 0.0, 0.0),
    ]
}

fn traffic_cone() -> Vec<Part> {
    vec![
        block(0.0, 0.06, 0.0, 1.4, 0.12, 1.4, BLACK),
        cone(0.0, 1.05, 0.0, 1.05, 1.9, ORANGE),
        ring(0.0, 0.85, 0.0, 0.66, 0.16, WHITE),
        ring(0.0, 1.35, 0.0, 0.42, 0.13, WHITE),
    ]
}

fn mailbox() -> Vec<Part> {
    vec![
        block(0.0, 0.75, 0.0, 0.25, 1.5, 0.25, DARK_WOOD),
        block(0.0, 1.75, 0.0, 0.8, 0.6, 1.6, BLUE),
        tube(0.0, 2.05, 0.0, 0.8, 1.6, BLUE).turn(Q, 0.0, 0.0),
        block(0.0, 1.85, 0.81, 0.72, 0.85, 0.04, STEEL),
        block(0.46, 2.15, -0.2, 0.04, 0.12, 0.7, RED),
        block(0.46, 2.4, -0.5, 0.04, 0.4, 0.25, RED),
        block(0.0, 0.06, 0.0, 0.7, 0.12, 0.7, GREY),
    ]
}

fn bench() -> Vec<Part> {
    let mut parts = Vec::new();
    for i in 0..3 {
        parts.push(block(
            0.0,
            0.92,
            -0.25 + i as f32 * 0.25,
            3.6,
            0.08,
            0.2,
            WOOD,
        ));
    }
    for i in 0..2 {
        parts.push(
            block(0.0, 1.4 + i as f32 * 0.32, -0.48, 3.6, 0.22, 0.07, WOOD).turn(-0.2, 0.0, 0.0),
        );
    }
    for x in [-1.55, 1.55] {
        parts.push(block(x, 0.45, 0.25, 0.1, 0.9, 0.1, BLACK));
        parts.push(block(x, 0.75, -0.42, 0.1, 1.5, 0.1, BLACK).turn(-0.2, 0.0, 0.0));
        parts.push(block(x, 1.2, -0.05, 0.1, 0.08, 0.7, BLACK));
        parts.push(block(x, 0.04, -0.1, 0.1, 0.08, 0.8, BLACK));
    }
    parts
}

fn tree() -> Vec<Part> {
    vec![
        tube(0.0, 1.2, 0.0, 0.55, 2.4, BROWN),
        tube(0.35, 2.4, 0.0, 0.22, 1.0, BROWN).turn(0.0, 0.0, -0.6),
        ball(0.0, 3.4, 0.0, 2.4, 2.1, 2.4, GREEN),
        ball(0.9, 2.9, 0.4, 1.6, 1.4, 1.6, LEAF),
        ball(-0.9, 2.9, -0.3, 1.6, 1.5, 1.6, LEAF),
        ball(0.0, 4.3, 0.2, 1.5, 1.3, 1.5, GREEN),
        ball(0.6, 3.3, 1.05, 0.25, 0.25, 0.25, RED),
        ball(-0.5, 3.9, 0.9, 0.25, 0.25, 0.25, RED),
        ball(0.0, 0.05, 0.0, 1.2, 0.1, 1.2, BROWN),
    ]
}

fn cactus() -> Vec<Part> {
    vec![
        tube(0.0, 1.7, 0.0, 0.8, 3.4, GREEN),
        ball(0.0, 3.4, 0.0, 0.8, 0.8, 0.8, GREEN),
        tube(-0.7, 1.55, 0.0, 0.42, 0.7, GREEN).turn(0.0, 0.0, Q),
        tube(-1.05, 2.1, 0.0, 0.45, 1.2, GREEN),
        ball(-1.05, 2.7, 0.0, 0.45, 0.45, 0.45, GREEN),
        tube(0.7, 2.05, 0.0, 0.42, 0.7, GREEN).turn(0.0, 0.0, Q),
        tube(1.05, 2.45, 0.0, 0.45, 0.85, GREEN),
        ball(1.05, 2.88, 0.0, 0.45, 0.45, 0.45, GREEN),
        ball(0.0, 3.82, 0.0, 0.35, 0.22, 0.35, PINK),
        tube(0.0, 0.25, 0.0, 1.3, 0.5, ORANGE),
        tube(0.0, 0.48, 0.0, 1.2, 0.04, BROWN),
    ]
}

fn snowman() -> Vec<Part> {
    vec![
        ball(0.0, 0.9, 0.0, 2.0, 1.85, 2.0, WHITE),
        ball(0.0, 2.35, 0.0, 1.5, 1.4, 1.5, WHITE),
        ball(0.0, 3.5, 0.0, 1.1, 1.05, 1.1, WHITE),
        cone(0.0, 3.45, 0.75, 0.2, 0.7, ORANGE).turn(Q, 0.0, 0.0),
        ball(-0.2, 3.7, 0.48, 0.12, 0.12, 0.12, BLACK),
        ball(0.2, 3.7, 0.48, 0.12, 0.12, 0.12, BLACK),
        tube(0.0, 4.05, 0.0, 1.2, 0.08, BLACK),
        tube(0.0, 4.4, 0.0, 0.75, 0.7, BLACK),
        ring(0.0, 2.95, 0.0, 1.2, 0.2, RED),
        tube(-1.25, 2.6, 0.0, 0.07, 1.4, BROWN).turn(0.0, 0.0, 1.0),
        tube(1.25, 2.6, 0.0, 0.07, 1.4, BROWN).turn(0.0, 0.0, -1.0),
        ball(0.0, 2.5, 0.72, 0.14, 0.14, 0.1, BLACK),
        ball(0.0, 2.15, 0.74, 0.14, 0.14, 0.1, BLACK),
    ]
}

fn mushroom() -> Vec<Part> {
    let mut parts = vec![
        tube(0.0, 0.7, 0.0, 0.7, 1.4, CREAM),
        ball(0.0, 1.55, 0.0, 2.4, 1.3, 2.4, RED),
        tube(0.0, 1.5, 0.0, 2.15, 0.08, CREAM),
        ring(0.0, 1.05, 0.0, 0.85, 0.1, CREAM),
    ];
    for (x, y, z) in [
        (0.0, 2.2, 0.0),
        (0.75, 1.95, 0.3),
        (-0.65, 1.95, 0.45),
        (0.2, 1.9, -0.8),
        (-0.5, 1.95, -0.55),
        (0.55, 1.85, 0.75),
    ] {
        parts.push(ball(x, y, z, 0.38, 0.12, 0.38, WHITE));
    }
    parts
}

fn ice_cream() -> Vec<Part> {
    vec![
        cone(0.0, 1.15, 0.0, 1.1, 2.3, WOOD).turn(PI, 0.0, 0.0),
        ring(0.0, 2.2, 0.0, 1.15, 0.14, WOOD),
        ball(0.0, 2.6, 0.0, 1.25, 1.05, 1.25, PINK),
        ball(0.0, 3.4, 0.0, 1.05, 0.95, 1.05, CREAM),
        ball(0.0, 4.0, 0.0, 0.3, 0.3, 0.3, RED),
        tube(0.05, 4.25, 0.0, 0.03, 0.3, GREEN),
        block(0.3, 3.6, 0.35, 0.08, 0.2, 0.05, YELLOW),
        block(-0.3, 3.5, 0.4, 0.08, 0.2, 0.05, BLUE),
    ]
}

fn apple() -> Vec<Part> {
    vec![
        ball(0.0, 0.85, 0.0, 1.8, 1.6, 1.8, RED),
        ball(0.0, 1.6, 0.0, 0.5, 0.2, 0.5, RED),
        tube(0.05, 1.85, 0.0, 0.08, 0.45, BROWN).turn(0.0, 0.0, -0.2),
        ball(0.3, 1.85, 0.0, 0.5, 0.06, 0.25, GREEN).turn(0.0, 0.3, 0.3),
        ball(0.45, 1.05, 0.62, 0.28, 0.4, 0.1, WHITE),
    ]
}

fn carrot() -> Vec<Part> {
    let mut parts = vec![
        cone(0.0, 1.3, 0.0, 0.7, 2.6, ORANGE).turn(PI, 0.0, 0.0),
        ball(0.0, 2.6, 0.0, 0.7, 0.2, 0.7, ORANGE),
    ];
    for y in [0.8, 1.4, 2.0] {
        parts.push(ring(0.0, y, 0.0, 0.62 * (y / 2.6) + 0.04, 0.03, BROWN));
    }
    for (rx, rz) in [
        (0.0, 0.0),
        (0.35, 0.2),
        (-0.35, 0.2),
        (0.1, -0.4),
        (-0.1, -0.35),
    ] {
        parts.push(cone(-rz * 0.6, 3.15, rx * 0.6, 0.18, 1.1, LEAF).turn(rx, 0.0, rz));
    }
    parts
}

fn pineapple() -> Vec<Part> {
    let mut parts = vec![ball(0.0, 1.1, 0.0, 1.4, 2.0, 1.4, YELLOW)];
    for y in [0.55, 0.95, 1.35, 1.75] {
        let across = 1.4 * (1.0 - ((y - 1.1) / 1.0f32).powi(2)).sqrt() + 0.05;
        parts.push(ring(0.0, y, 0.0, across, 0.06, BROWN));
    }
    for i in 0..7 {
        let angle = i as f32 * 2.0 * PI / 7.0;
        let (c, s) = (angle.cos(), angle.sin());
        parts.push(cone(0.22 * c, 2.55, 0.22 * s, 0.22, 1.3, LEAF).turn(s * 0.35, 0.0, -c * 0.35));
    }
    parts.push(cone(0.0, 2.75, 0.0, 0.25, 1.6, LEAF));
    parts
}
