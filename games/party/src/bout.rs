pub use tawks_bout::*;

const fn spec(
    key: &'static str,
    name: &'static str,
    rules: &'static str,
    controls: Controls,
    create: fn(&[Contestant], u64) -> Box<dyn Bout>,
) -> BoutSpec {
    BoutSpec {
        key,
        name,
        rules,
        controls,
        format: Format::FreeForAll,
        min_players: 2,
        max_players: 8,
        create,
    }
}

fn paint(
    variant: crate::paint_variant::PaintVariant,
    contestants: &[Contestant],
    seed: u64,
) -> Box<dyn Bout> {
    Box::new(crate::paint_variant::PaintPlaylist::new(
        variant,
        contestants,
        seed,
    ))
}

pub static BOUTS: &[BoutSpec] = &[
    bumper_balls::SPEC,
    spec(
        "shooter",
        "Paint Blasters",
        "Claim the arena with fast paint fights.",
        Controls::FirstPerson,
        |c, s| Box::new(crate::shooter::PaintBlasters::new(c, s)),
    ),
    spec(
        "dodgeball",
        "Dodgeball",
        "Catch, throw, and knock your rivals out.",
        Controls::FirstPerson,
        |c, s| Box::new(crate::dodgeball::Dodgeball::new(c, s)),
    ),
    spec(
        "race",
        "Rocker Rally",
        "Pump left and right. Steer onto boost pads.",
        Controls::Move,
        |c, s| Box::new(crate::race::RockerRally::new(c, s)),
    ),
    spec(
        "redlight",
        "Red Light Rush",
        "Run on green. Freeze on red or lose checkpoint progress.",
        Controls::Move,
        |c, s| Box::new(crate::redlight::RedLightRush::new(c, s)),
    ),
    spec(
        "crumble",
        "Crumble Run",
        "Every tile you stand on gives way. Keep moving.",
        Controls::Move,
        |c, s| Box::new(crate::crumble::CrumbleRun::new(c, s)),
    ),
    spec(
        "hotpotato",
        "Hot Potato Panic",
        "Touch someone to pass the bomb before the fuse ends.",
        Controls::Move,
        |c, s| Box::new(crate::potato::HotPotato::new(c, s)),
    ),
    spec(
        crate::relay::KEY,
        "Bomb Relay",
        "Carry the bomb through the gates and touch the next runner. The fuse gets shorter every round.",
        Controls::Move,
        |c, s| Box::new(crate::relay::BombRelay::new(c, s)),
    ),
    spec(
        "balloon",
        "Balloon Brawl",
        "Dash into a rival to burst a balloon. Lose four and you are out.",
        Controls::MoveAction,
        |c, s| Box::new(crate::balloon::BalloonBrawl::new(c, s)),
    ),
    spec(
        "sumo",
        "Sumo",
        "Shove your rivals off the platform. It keeps shrinking.",
        Controls::Move,
        |c, s| Box::new(crate::sumo::Sumo::new(c, s)),
    ),
    spec(
        "ledge",
        "Ledge Shuffle",
        "Shove rivals off a ledge that crumbles from the rim in. Stand still to brace. Last one up wins.",
        Controls::MoveAction,
        |c, s| Box::new(crate::ledge::LedgeShuffle::new(c, s)),
    ),
    spec(
        "coins",
        "Coin Scramble",
        "Grab the coins raining in. Charge a rival to shake their coins loose.",
        Controls::MoveAction,
        |c, s| Box::new(crate::coins::CoinScramble::new(c, s)),
    ),
    spec(
        "target",
        "Target Frenzy",
        "Race to hit the shared wall light. Faster reactions score more.",
        Controls::FirstPerson,
        |c, s| paint(crate::paint_variant::PaintVariant::TargetFrenzy, c, s),
    ),
    spec(
        "popgun",
        "Popgun Knockout",
        "One hit knocks you out. Last player standing wins.",
        Controls::FirstPerson,
        |c, s| paint(crate::paint_variant::PaintVariant::PopgunKnockout, c, s),
    ),
    spec(
        "ricochet",
        "Ricochet Rumble",
        "Use every piece of cover in a paint rumble.",
        Controls::FirstPerson,
        |c, s| paint(crate::paint_variant::PaintVariant::RicochetRumble, c, s),
    ),
    BoutSpec {
        format: Format::Teams,
        ..spec(
            "retake",
            "Site Retake",
            "Attackers defend the planted core. Defenders retake and defuse.",
            Controls::FirstPerson,
            |c, s| paint(crate::paint_variant::PaintVariant::SiteRetake, c, s),
        )
    },
    spec(
        crate::hill::KEY,
        "King of the Hill",
        "Hold the summit while the slope and everyone else push you off.",
        Controls::Move,
        |c, s| Box::new(crate::hill::KingOfTheHill::new(c, s)),
    ),
    BoutSpec {
        format: Format::Teams,
        ..spec(
            crate::speedball::KEY,
            "Speedball",
            "Hang the centre flag at their station or take everyone out. First to three rounds.",
            Controls::FirstPerson,
            |c, s| Box::new(crate::speedball::Speedball::new(c, s)),
        )
    },
    spec(
        crate::memory::KEY,
        "Memory Tiles",
        "Watch the tiles light up, then walk them back in order and stomp each one. One wrong tile and you are out.",
        Controls::MoveAction,
        |c, s| Box::new(crate::memory::MemoryTiles::new(c, s)),
    ),
    spec(
        crate::rising::KEY,
        "Rising Water",
        "The arena is flooding. Jump up the platforms before they crumble. Last one above water wins.",
        Controls::MoveAction,
        |c, s| Box::new(crate::rising::RisingWater::new(c, s)),
    ),
    spec(
        crate::minecart::KEY,
        "Minecart Rush",
        "Lean into each junction to take the short branch. Ram a rival and you both get knocked back.",
        Controls::Move,
        |c, s| Box::new(crate::minecart::MinecartRush::new(c, s)),
    ),
    spec(
        "tank",
        "Tank Clash",
        "Bounce shells off the walls. Most wrecks wins.",
        Controls::MoveAim,
        |c, s| Box::new(crate::tank::TankClash::new(c, s)),
    ),
    spec(
        "titan",
        "Titan Siege",
        "Dodge the wind-up, then strike the core while it is winded.",
        Controls::MoveAction,
        |c, s| Box::new(crate::titan::TitanSiege::new(c, s)),
    ),
    BoutSpec {
        format: Format::Teams,
        ..spec(
            crate::tug::KEY,
            "Tug of War",
            "Alternate your presses to heave the rope. The side it hauls over the water goes in.",
            Controls::MoveAction,
            |c, s| Box::new(crate::tug::TugOfWar::new(c, s)),
        )
    },
    spec(
        crate::buzzer::KEY,
        "Buzzer Round",
        "Quick questions for everyone at once. The first right answer scores. One guess each.",
        Controls::Move,
        |c, s| Box::new(crate::buzzer::BuzzerRound::new(c, s)),
    ),
    spec(
        crate::ghosts::KEY,
        "Ghost Busting",
        "Hold a ghost in your vacuum's cone to suck it in. Gold is worth three; the big one takes two of you.",
        Controls::FirstPerson,
        |c, s| Box::new(crate::ghosts::GhostBusting::new(c, s)),
    ),
];

pub fn find(key: &str) -> Option<&'static BoutSpec> {
    BOUTS.iter().find(|spec| spec.key == key)
}

pub fn create(key: &str, contestants: &[Contestant], seed: u64) -> Option<Box<dyn Bout>> {
    find(key).map(|spec| (spec.create)(contestants, seed))
}
