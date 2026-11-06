//! What each player is told: their side's view of the run, the parked cars
//! and fixed spots, and where the people and cars they can see are. Thieves
//! never learn more than a crew would, and cops only hear what police would.

use serde_json::{Value, json};

use super::game::{Audience, Heist, Kind, Phase, Place, Role};
use super::map::dist;
use super::rules::{GATE, GATE_RADIUS, TOWNS};

/// Thieves and cops see each other's moving parts only this close, unless in a chase.
const SEE_FAR: f64 = 650.0;

pub fn hears(heist: &Heist, user: &str, to: Audience) -> bool {
    let role = heist.players.get(user).and_then(|p| p.role);
    match to {
        Audience::All => true,
        Audience::Thieves => role == Some(Role::Thief),
        Audience::Cops => role == Some(Role::Cop),
    }
}

fn phase(heist: &Heist, now: f64) -> Value {
    match &heist.phase {
        Phase::Setup => json!({ "phase": "setup" }),
        Phase::Loading { .. } => json!({ "phase": "loading" }),
        Phase::Running { ends, .. } => json!({ "phase": "running", "left": (ends - now).max(0.0) }),
        Phase::Ended { results } => json!({ "phase": "ended", "results": results }),
    }
}

pub fn view(heist: &Heist, user: &str, now: f64) -> Value {
    let me = heist.players.get(user);
    let role = me.and_then(|p| p.role);
    let players: Vec<Value> = heist
        .players
        .iter()
        .map(|(id, p)| json!({ "id": id, "name": p.name, "role": p.role, "place": p.place, "arrested": p.arrested, "online": p.online }))
        .collect();
    let vehicles: Vec<Value> = heist
        .vehicles
        .iter()
        .filter(|v| v.kind != Kind::Parked || v.damage.window_broken)
        .map(|v| {
            json!({
                "id": v.id, "kind": v.kind, "class": v.class, "colour": v.colour, "x": v.x, "z": v.z, "h": v.h,
                "damage": v.damage, "handling": v.damage.handling(), "driver": v.driver, "riders": v.riders, "owner": v.owner,
                "siren": v.siren, "spotlight": v.spotlight,
            })
        })
        .collect();
    let pings: Vec<Value> = heist
        .pings
        .iter()
        .filter(|p| {
            let cops = p.kind.for_cops();
            match role {
                Some(Role::Cop) => cops,
                Some(Role::Thief) => !cops || matches!(p.kind, super::game::PingKind::Alarm),
                None => false,
            }
        })
        .map(|p| json!({ "id": p.id, "kind": p.kind, "x": p.x, "z": p.z, "text": p.text, "age": now - p.at, "by": p.by }))
        .collect();
    let pursuit = match heist.pursuit.phase {
        super::pursuit::Phase::Calm => "calm",
        super::pursuit::Phase::Pursuit { .. } => "pursuit",
        super::pursuit::Phase::Search { .. } => "search",
        super::pursuit::Phase::Cooldown { .. } => "cooldown",
    };
    let mut out = json!({
        "phase": phase(heist, now),
        "settings": heist.settings,
        "me": me.map(|p| json!({ "role": p.role, "place": p.place, "arrested": p.arrested })),
        "players": players,
        "vehicles": vehicles,
        "pings": pings,
        "pursuit": pursuit,
        "spikes": heist.spikes,
        "world": heist.world_version,
        "gate": { "x": GATE.0, "z": GATE.1, "r": GATE_RADIUS },
        "dev": heist.dev,
    });
    match role {
        Some(Role::Thief) => {
            if let Some(c) = &heist.contract {
                let towns: Vec<Value> = c
                    .towns
                    .iter()
                    .map(|&t| {
                        let (x, z) = heist.frame.to_world(TOWNS[t].centre.into());
                        json!({ "name": TOWNS[t].name, "county": TOWNS[t].county, "x": x, "z": z, "r": TOWNS[t].radius })
                    })
                    .collect();
                out["contract"] = json!({ "class": c.class, "shade": c.shade, "text": c.describe(), "towns": towns });
            }
            out["heat"] = json!(heist.crew_heat());
            out["reported"] = json!(heist.reported);
            if let Some(t) = &heist.take {
                out["take"] = json!({ "car": t.car, "thief": t.thief, "stage": t.stage, "alarm": t.alarm, "mine": t.thief == user, "elapsed": t.stage_elapsed(now) });
            }
        }
        Some(Role::Cop) => {
            if heist.reported {
                out["wanted"] = json!(heist.description());
            }
        }
        None => {}
    }
    out
}

/// The parked cars and fixed spots, sent when they change.
pub fn world(heist: &Heist) -> Value {
    let parked: Vec<Value> = heist
        .vehicles
        .iter()
        .filter(|v| v.kind == Kind::Parked)
        .map(|v| json!({ "id": v.id, "class": v.class, "colour": v.colour, "x": v.x, "z": v.z, "h": v.h }))
        .collect();
    let readers: Vec<Value> = heist.readers.iter().map(|r| json!({ "x": r.0, "z": r.1 })).collect();
    json!({ "version": heist.world_version, "parked": parked, "readers": readers })
}

/// Moving cars, AI cops and people on foot this player can see.
pub fn poses(heist: &Heist, user: &str) -> Value {
    let role = heist.players.get(user).and_then(|p| p.role);
    let thief_spots: Vec<(f64, f64)> = heist
        .players
        .values()
        .filter(|p| p.role == Some(Role::Thief) && !p.arrested)
        .map(|p| (p.foot.x, p.foot.z))
        .collect();
    let mut police: Vec<(f64, f64)> = heist.cops.iter().map(|c| (c.x, c.z)).collect();
    police.extend(
        heist
            .players
            .values()
            .filter(|p| p.role == Some(Role::Cop))
            .map(|p| (p.foot.x, p.foot.z)),
    );
    let chase = heist.pursuit.active();
    let near = |p: (f64, f64), of: &[(f64, f64)]| of.iter().any(|q| dist(p, *q) < SEE_FAR);
    let crew_car = |kind: Kind| matches!(kind, Kind::Crew | Kind::Stolen);
    let visible = |p: (f64, f64), thief_side: bool| match (role, thief_side) {
        (Some(Role::Cop), true) => chase || near(p, &police),
        (Some(Role::Thief), false) => near(p, &thief_spots),
        _ => true,
    };
    let cars: Vec<Value> = heist
        .vehicles
        .iter()
        .filter(|v| v.kind != Kind::Parked && visible((v.x, v.z), crew_car(v.kind)))
        .map(|v| json!({ "id": v.id, "x": v.x, "z": v.z, "h": v.h, "s": v.s, "lights": v.lights && v.damage.lights.iter().any(|l| *l), "siren": v.siren, "spot": v.spotlight, "driver": v.driver }))
        .collect();
    let ai: Vec<Value> = heist
        .cops
        .iter()
        .filter(|c| visible((c.x, c.z), false))
        .map(|c| json!({ "id": c.id, "x": c.x, "z": c.z, "h": c.h, "s": c.speed, "siren": c.siren }))
        .collect();
    let feet: Vec<Value> = heist
        .players
        .iter()
        .filter(|(_, p)| p.place == Place::Foot && p.role.is_some() && visible((p.foot.x, p.foot.z), p.role == Some(Role::Thief)))
        .map(|(id, p)| json!({ "id": id, "x": p.foot.x, "z": p.foot.z, "h": p.foot.h, "s": p.foot.s, "stance": p.foot.stance, "flash": p.foot.flash, "role": p.role, "arrested": p.arrested }))
        .collect();
    json!({ "cars": cars, "ai": ai, "feet": feet })
}
