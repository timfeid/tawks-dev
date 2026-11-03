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
        crate::lineup::KEY,
        "Lineup",
        "Study the parade. The lights drop and someone changes. Click who changed, fast. Wrong clicks lock you out.",
        Controls::MoveAction,
        |c, s| Box::new(crate::lineup::Lineup::new(c, s)),
    ),
    spec(
        crate::ghosts::KEY,
        "Ghost Busting",
        "Hold a ghost in your vacuum's cone to suck it in. Gold is worth three; the big one takes two of you.",
        Controls::FirstPerson,
        |c, s| Box::new(crate::ghosts::GhostBusting::new(c, s)),
    ),
    spec(
        crate::stencil::KEY,
        "Stencil Rush",
        "Trace the glowing outline with your cutting torch. Stay on the line, finish fast. Best cut takes the round.",
        Controls::MoveAim,
        |c, s| Box::new(crate::stencil::StencilRush::new(c, s)),
    ),
    spec(
        crate::teeter::KEY,
        "Tower Teeter",
        "Stack furniture on your raft as the sea gets rougher. Tallest tower standing at the horn wins; pieces in the water cost height.",
        Controls::MoveAim,
        |c, s| Box::new(crate::teeter::TowerTeeter::new(c, s)),
    ),
    spec(
        crate::glass::KEY,
        "Glass Cannon",
        "Shoot marbles at the panes in your color. Hit a rival's color and they get the point.",
        Controls::FirstPerson,
        |c, s| Box::new(crate::glass::GlassCannon::new(c, s)),
    ),
    spec(
        crate::echo::KEY,
        "Echo Chamber",
        "Watch the chimes ring, then play them back. The room turns in between, so follow the colors, not the spots. Two misses and you are out.",
        Controls::MoveAction,
        |c, s| Box::new(crate::echo::EchoChamber::new(c, s)),
    ),
    spec(
        crate::lastfloor::KEY,
        "Last Floor",
        "Jump off the falling elevator onto a passing floor. Lower pays more. Ride to the bottom and you crash.",
        Controls::MoveAction,
        |c, s| Box::new(crate::lastfloor::LastFloor::new(c, s)),
    ),
    spec(
        crate::fuse::KEY,
        "Overcooked Fuse",
        "Hold the lit cannonball to earn. Pass it before it blows, or lose this fuse's haul.",
        Controls::MoveAim,
        |c, s| Box::new(crate::fuse::OvercookedFuse::new(c, s)),
    ),
    spec(
        crate::mugshot::KEY,
        "Mugshot",
        "A witness describes the suspect. Build the face from your kit, then vote for the best match in the lineup.",
        Controls::Move,
        |c, s| Box::new(crate::mugshot::Mugshot::new(c, s)),
    ),
    spec(
        crate::blueprints::KEY,
        "Blueprints",
        "Name the object as it is revealed. First right answer scores most; a wrong guess locks you out.",
        Controls::MoveAction,
        |c, s| Box::new(crate::blueprints::Blueprints::new(c, s)),
    ),
    spec(
        crate::diner::KEY,
        "High Stakes Diner",
        "Five cuts, one perfect at the buzzer. Read the sear, sizzle and smoke, pick it, and bet chips. Early locks pay better odds.",
        Controls::MoveAction,
        |c, s| Box::new(crate::diner::HighStakesDiner::new(c, s)),
    ),
    BoutSpec {
        format: Format::Teams,
        ..spec(
            crate::bridgecrew::KEY,
            "Bridge Crew",
            crate::bridgecrew::RULE,
            Controls::MoveAction,
            |c, s| Box::new(crate::bridgecrew::BridgeCrew::new(c, s)),
        )
    },
    BoutSpec {
        min_players: 3,
        max_players: 6,
        ..spec(
            crate::gallery::KEY,
            "Ghost in the Gallery",
            "One ghost steals paintings in the dark. Guards hold a flashlight on it to catch it. Three paintings wins it for the ghost.",
            Controls::MoveAim,
            |c, s| Box::new(crate::gallery::GhostGallery::new(c, s)),
        )
    },
    spec(
        crate::ravine_draft::KEY,
        "Ravine Draft",
        "Slide downhill through shifting wind corridors. Brake to line up, or keep speed for bonus points.",
        Controls::MoveAction,
        |c, s| Box::new(crate::ravine_draft::RavineDraft::new(c, s)),
    ),
    spec(
        crate::crowd_current::KEY,
        "Crowd Current",
        "Push the heavy parade float through the moving arch. Recent contributors score each delivery.",
        Controls::MoveAction,
        |c, s| Box::new(crate::crowd_current::CrowdCurrent::new(c, s)),
    ),
    spec(
        crate::cue_crowd::KEY,
        "Cue the Crowd",
        "Read the audience cue. Pick a puppet gesture and perform on the spotlight pulse for more applause.",
        Controls::MoveAction,
        |c, s| Box::new(crate::cue_crowd::CueCrowd::new(c, s)),
    ),
    spec(
        crate::parcel_panic::KEY,
        "Prism Foundry",
        "Slide a cooling mold between three forge channels. Quench each falling crystal at the gold line for more points.",
        Controls::MoveAction,
        |c, s| Box::new(crate::parcel_panic::ParcelPanic::new(c, s)),
    ),
    spec(
        crate::reef_bloom::KEY,
        "Reef Bloom",
        "Plant coral on open reef for 30 points. Living polyps score each growth beat, then fade. Tend the largest garden before the dive ends.",
        Controls::MoveAction,
        |c, s| Box::new(crate::reef_bloom::ReefBloom::new(c, s)),
    ),
    spec(
        crate::beacon::KEY,
        "Beacon Bloom",
        "Turn your lantern toward the drifting mote and pulse when its halos meet. Chain clean blooms for more points.",
        Controls::MoveAction,
        |c, s| Box::new(crate::beacon::BeaconBloom::new(c, s)),
    ),
    spec(
        crate::pigeon::KEY,
        "Pigeon Post",
        "Watch the marked letters reach the mailboxes. When the birds fly off, pick the box that received the called letter.",
        Controls::MoveAction,
        |c, s| Box::new(crate::pigeon::PigeonPost::new(c, s)),
    ),
    spec(
        crate::rime::KEY,
        "Rime Rink",
        "Slide across the ice and ring the lit floor bell before it moves. Carry your momentum through each turn.",
        Controls::Move,
        |c, s| Box::new(crate::rime::RimeRink::new(c, s)),
    ),
    spec(
        crate::orbit::KEY,
        "Orbit Oddjobs",
        "Catch loose hardware, steady a drifting panel, and shove scrap into the recycling ring. Three jobs, one short shift.",
        Controls::MoveAction,
        |c, s| Box::new(crate::orbit::OrbitOddjobs::new(c, s)),
    ),
    spec(
        crate::margin_run::KEY,
        "Margin Run",
        "Chase the moving ink marks through gaps in the folding pages. First touch scores.",
        Controls::Move,
        |c, s| Box::new(crate::margin_run::MarginRun::new(c, s)),
    ),
    spec(
        crate::flux_skip::KEY,
        "Flux Skip",
        "Move around the court and spring over each expanding energy wave. Clean timing scores extra.",
        Controls::MoveAction,
        |c, s| Box::new(crate::flux_skip::FluxSkip::new(c, s)),
    ),
    spec(
        crate::lantern::KEY,
        "Spark Courier",
        "Gather drifting sparks and bank them at the relay core. A full load scores a bigger bonus.",
        Controls::MoveAction,
        |c, s| Box::new(crate::lantern::LanternRun::new(c, s)),
    ),
    spec(
        crate::whirligig::KEY,
        "Whirligig Works",
        "Rotate the workshop ducts to carry wind from the bellows to the pinwheel. Complete as many circuits as you can before time runs out.",
        Controls::MoveAction,
        |c, s| Box::new(crate::whirligig::Whirligig::new(c, s)),
    ),
    spec(
        crate::stackbreak_yard::KEY,
        "Stackbreak Yard",
        "Shove your own ball into shared block towers. Topple a whole stack for a bonus before it rebuilds.",
        Controls::MoveAction,
        |c, s| Box::new(crate::stackbreak_yard::StackbreakYard::new(c, s)),
    ),
    spec(
        crate::wake::KEY,
        "Wake Mosaic",
        "Steer a skiff through shifting currents. Your colored wake claims water tiles for points; it fades and rivals can repaint it.",
        Controls::Move,
        |c, s| Box::new(crate::wake::WakeMosaic::new(c, s)),
    ),
    spec(
        crate::chimney_dash::KEY,
        "Chimney Dash",
        "Ride the rising draft and sweep soot when it crosses your work line. Clean more patches than your rivals before the chimney clears.",
        Controls::MoveAction,
        |c, s| Box::new(crate::chimney_dash::ChimneyDash::new(c, s)),
    ),
    spec(
        crate::signal_sweep::KEY,
        "Signal Sweep",
        "Disarm your lit signal stations in order before your personal alarm expires. Finish a route for bonus points.",
        Controls::MoveAction,
        |c, s| Box::new(crate::signal_sweep::SignalSweep::new(c, s)),
    ),
    spec(
        crate::shadow_stamp::KEY,
        "Shadow Stamp",
        "Line your moving shadow tip up with drifting floor seals, then stamp. A close hit scores extra.",
        Controls::MoveAction,
        |c, s| Box::new(crate::shadow_stamp::ShadowStamp::new(c, s)),
    ),
    spec(
        crate::garden_snapshot::KEY,
        "Garden Snapshot",
        "Frame the featured butterfly in the garden. A centered photo with open wings scores best; you get one shutter press each round.",
        Controls::MoveAction,
        |c, s| Box::new(crate::garden_snapshot::GardenSnapshot::new(c, s)),
    ),
    spec(
        crate::powder::KEY,
        "Powder Pop",
        "Build sled speed on an open snow bowl. Tap at a lift ramp, land clean, then use a different ramp to grow your combo.",
        Controls::MoveAction,
        |c, s| Box::new(crate::powder::PowderPop::new(c, s)),
    ),
    spec(
        crate::kite_net::KEY,
        "Kite Net",
        "Place three stakes around the wandering sprites. A compact triangle catches more points.",
        Controls::MoveAction,
        |c, s| Box::new(crate::kite_net::KiteNet::new(c, s)),
    ),
    spec(
        crate::dockside_tangle::KEY,
        "Dockside Tangle",
        "Swap two right hooks to uncross the harbor ropes. Clear as many knots as you can before the tide turns.",
        Controls::MoveAction,
        |c, s| Box::new(crate::dockside_tangle::DocksideTangle::new(c, s)),
    ),
    spec(
        crate::mirth::KEY,
        "Mirth Chain",
        "Choose an expression and broadcast near a bystander. Matching faces echo to their neighbors; longer chains score more.",
        Controls::MoveAction,
        |c, s| Box::new(crate::mirth::MirthChain::new(c, s)),
    ),
    spec(
        crate::mixer_circuit::KEY,
        "Mixer Circuit",
        "Exchange invitation stamps with different guests. Starting an exchange scores extra; complete your card for a bonus.",
        Controls::MoveAction,
        |c, s| Box::new(crate::mixer_circuit::MixerCircuit::new(c, s)),
    ),
];

pub fn find(key: &str) -> Option<&'static BoutSpec> {
    BOUTS.iter().find(|spec| spec.key == key)
}

/// The bout as a room plays it, with every bot on its form for the day.
pub fn create(key: &str, contestants: &[Contestant], seed: u64) -> Option<Box<dyn Bout>> {
    // A one-step skill gap plays differently in each bout; keep the favorite beatable.
    let scale = match key {
        "redlight" | "glass" => 1.5,
        "stencil" | "race" => 1.2,
        "lineup" | "blueprints" => 0.5,
        "crumble" => 0.4,
        _ => 1.0,
    };
    find(key).map(|spec| (spec.create)(&on_form_scaled(contestants, seed, scale), seed))
}
