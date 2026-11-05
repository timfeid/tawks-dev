//! The shapes made minigames share between the sandbox, the relay, the API
//! and the kit. `open/sdk/made-kit/src/scene.ts` is generated from these
//! types; `cargo test -p made-protocol` fails when it is stale, and
//! `MADE_WRITE_TS=1 cargo test -p made-protocol` rewrites it. The kit's own
//! frames (open/sdk/made-kit/test/fixtures/frames.json) are parsed strictly
//! here too, so the kit, Rust and TypeScript cannot drift apart.
//!
//! The contract, end to end:
//! - A game is one `game.js` against `@tawks/made-kit` (core.js). made-simd
//!   runs all of it: setup, tick at 60 Hz, bot for bot seats, and view plus
//!   effect at 20 Hz, producing [`SceneFrame`]s.
//! - A lobby running a made game shows `DisplayType::MadeCanvas`. Its workflow
//!   starts with the games bot's `made_join` action, which stores a
//!   [`MadeJoin`] at `responses.made`: the relay's `ws_url`, a pass, and the
//!   seat the pass plays (null to watch).
//! - The client opens `ws_url` (`/party/made/ws` on the party host), sends
//!   [`ClientMsg::Hello`], and gets [`RelayMsg::Welcome`] then frames as
//!   [`FrameDelta`]s. See [`protocol`] for input and rate limits.
//! - Roster costumes are the party wardrobe value the party canvas already
//!   reads, or null; the client gives each seat 1 to 8 its own colour, which
//!   is also what `Color::Seat` means.
//! - Sounds are semantic names ([`Sound`]); the client maps them to its
//!   sound files (bump to punch, splash to drip, crack to ice, and so on).
//! - Screenshots: the render service draws [`simd::RenderShot`]s with the
//!   kit renderer page; see [`simd::RenderRequest`] for the page contract.
//! - Simulations are the other kind of made activity; see [`sim`].

pub mod lint;
pub mod protocol;
pub mod scene;
pub mod sim;
pub mod simd;

pub use protocol::*;
pub use scene::*;

pub const KIT_VERSION: u32 = 1;

/// Every type the TypeScript side reads.
pub fn types() -> specta::TypeCollection {
    use specta::TypeCollection;
    let mut types = TypeCollection::default();
    types
        .register::<scene::SceneFrame>()
        .register::<scene::FrameDelta>()
        .register::<scene::Entity>()
        .register::<scene::EntityKind>()
        .register::<scene::Color>()
        .register::<scene::Pose>()
        .register::<scene::Ground>()
        .register::<scene::Sound>()
        .register::<scene::Effect>()
        .register::<scene::Hud>()
        .register::<protocol::ClientMsg>()
        .register::<protocol::RelayMsg>()
        .register::<protocol::Input>()
        .register::<protocol::MadeJoin>()
        .register::<protocol::Welcome>()
        .register::<simd::PlayoutReport>()
        .register::<simd::RenderRequest>()
        .register::<simd::RenderShot>()
        .register::<simd::RenderResponse>()
        .register::<sim::SimMeta>()
        .register::<sim::SimFrame>()
        .register::<sim::SimDelta>()
        .register::<sim::SimControl>()
        .register::<sim::SimWelcome>()
        .register::<sim::SimRenderShot>()
        .register::<sim::MadeKind>();
    types
}

#[cfg(test)]
mod tests {
    use specta_typescript::Typescript;

    const SCENE_TS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../sdk/made-kit/src/scene.ts"
    );

    #[test]
    fn scene_ts_matches_the_rust_types() {
        let header = "// Generated from open/games/made-protocol by `MADE_WRITE_TS=1 cargo test -p made-protocol`. Do not edit.\n\n";
        let body = Typescript::default()
            .export(&super::types())
            .expect("the made types export");
        let want = format!("{header}{body}");
        if std::env::var("MADE_WRITE_TS").is_ok() {
            std::fs::write(SCENE_TS, &want).expect("write scene.ts");
        }
        let have = std::fs::read_to_string(SCENE_TS).unwrap_or_default();
        assert!(
            have == want,
            "scene.ts is stale; run MADE_WRITE_TS=1 cargo test -p made-protocol"
        );
    }
}

#[cfg(test)]
mod kit_frames {
    use super::scene::{FrameDelta, SceneFrame};

    const FRAMES: &str = include_str!("../../../sdk/made-kit/test/fixtures/frames.json");

    #[test]
    fn frames_the_kit_builds_parse_strictly_and_pass_the_checks() {
        let all: Vec<serde_json::Value> = serde_json::from_str(FRAMES).unwrap();
        let (delta, frames) = all.split_last().unwrap();
        let frames: Vec<SceneFrame> = frames
            .iter()
            .map(|f| serde_json::from_value(f.clone()).expect("a kit frame matches SceneFrame"))
            .collect();
        for frame in &frames {
            frame.check().unwrap();
        }
        let delta: FrameDelta =
            serde_json::from_value(delta.clone()).expect("a kit delta matches FrameDelta");
        let mut rust = frames[1].diff(Some(&frames[0]));
        rust.del.sort();
        let mut kit = delta.clone();
        kit.del.sort();
        assert_eq!(rust, kit, "the kit and Rust diff frames the same way");
        assert_eq!(
            delta.apply(Some(&frames[0])).unwrap().entities.len(),
            frames[1].entities.len()
        );
        let custom = frames.last().unwrap();
        assert!(
            custom
                .entities
                .iter()
                .any(|e| e.kind == super::scene::EntityKind::Ball)
        );
        assert_eq!(custom.camera.follow, Some(2));
    }

    #[test]
    fn a_bad_frame_is_refused() {
        let mut frame: SceneFrame = serde_json::from_value(
            serde_json::from_str::<Vec<serde_json::Value>>(FRAMES).unwrap()[0].clone(),
        )
        .unwrap();
        frame.entities[0].x = f32::NAN;
        assert!(frame.check().is_err());
        let extra = r#"{"n":0,"t":0,"stage":{"water":true,"ground":null},"camera":{"half_x":1,"half_z":1,"pitch":55,"x":0,"z":0,"follow":null},"hud":{"objective":null,"alert":null,"timer":null,"scores":[],"meter":null},"entities":[{"id":"a","kind":"disc","x":0,"y":0,"z":0,"onclick":"x"}],"effects":[]}"#;
        assert!(serde_json::from_str::<SceneFrame>(extra).is_err());
    }
}
