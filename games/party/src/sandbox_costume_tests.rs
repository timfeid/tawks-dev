use crate::protocol::ClientMsg;
use crate::room::{DevAction, Phase, Room, RoomError};
use crate::roster::{Costume, PetLook, Wardrobe, cosmetics, pet_items, validate_sandbox_costume};

fn host() -> Room {
    let mut room = Room::new("sandbox-look", 42);
    room.connect("host", "Host", "").unwrap();
    room.dev_tools = true;
    room
}

#[test]
fn sandbox_costume_old_and_draft_socket_messages_deserialize() {
    assert!(matches!(serde_json::from_str::<ClientMsg>(r#"{"type":"start_sandbox"}"#).unwrap(), ClientMsg::StartSandbox { costume: None }));
    let msg = serde_json::from_str::<ClientMsg>(r#"{"type":"start_sandbox","costume":{"color":"clover","model_id":"model-robot"}}"#).unwrap();
    assert!(matches!(msg, ClientMsg::StartSandbox {costume: Some(costume)} if costume.model_id.as_deref() == Some("model-robot")));
}

#[test]
fn sandbox_costume_all_catalog_looks_can_be_tried_without_ownership() {
    for item in cosmetics() {
        let mut json = serde_json::json!({"color":"clover"});
        json[if item.slot == "model" { "model_id" } else { &item.slot }] = serde_json::json!(item.id);
        let draft: Costume = serde_json::from_value(json).unwrap();
        let mut room = host();
        room.start_sandbox_with_preview("host", draft, None).unwrap();
        assert!(room.sandbox);
        let member = room.members.iter().find(|member| member.user_id == "host").unwrap();
        let look = serde_json::to_value(member.costume.as_ref().unwrap()).unwrap();
        if item.slot == "extra" { assert_eq!(look["extras"], serde_json::json!([item.id])); }
        else { assert_eq!(look[if item.slot == "model" { "model_id" } else { &item.slot }], item.id); }
    }
}

#[test]
fn sandbox_costume_preview_is_host_dev_only_and_restores_original_before_normal_play() {
    let draft = Costume { color: "clover".into(), model_id: Some("model-robot".into()), ..Costume::default() };
    let mut room = host();
    room.dev_tools = false;
    assert_eq!(room.start_sandbox_with_preview("host", draft.clone(), None), Err(RoomError::NoDevTools));
    room.dev_tools = true;
    room.connect("friend", "Friend", "").unwrap();
    assert_eq!(room.start_sandbox_with_preview("friend", draft.clone(), None), Err(RoomError::NotHost));
    let original = room.members.iter().find(|member| member.user_id == "host").unwrap().clone();
    room.start_sandbox_with_preview("host", draft.clone(), None).unwrap();
    assert_eq!(room.start_sandbox_with_preview("host", draft, None), Err(RoomError::WrongPhase));
    let friend = room.members.iter().find(|member| member.user_id == "friend").unwrap();
    assert!(friend.costume.is_none(), "only the sender's own seat is changed");
    room.dev("host", DevAction::Exit).unwrap();
    let restored = room.members.iter().find(|member| member.user_id == "host").unwrap();
    assert_eq!(restored.character, original.character);
    assert!(restored.costume.is_none(), "the preview is not retained as an applied account look");
    assert_eq!(room.phase, Phase::Lobby);
    room.start_party("host").unwrap();
    assert!(!room.sandbox);
    assert!(room.members.iter().find(|member| member.user_id == "host").unwrap().costume.is_none());
}

#[test]
fn sandbox_costume_rejects_unknown_imports_forged_slots_and_invalid_colors_atomically() {
    let wardrobe = Wardrobe { costume: Costume {color:"sunny".into(),model_id:Some("PMD-validated-owned".into()),..Costume::default()}, owned: vec![], set_up: true, jersey: None };
    let imported = Costume {color:"sunny".into(),model_id:Some("PMD-validated-owned".into()),..Costume::default()};
    assert!(validate_sandbox_costume(&imported, None).is_err());
    assert!(validate_sandbox_costume(&imported, Some(&wardrobe)).is_ok());
    for draft in [
        Costume {model_id:Some("PMD-other-owner".into()),..imported.clone()},
        Costume {model_id:Some("model-made-up".into()),..imported.clone()},
        Costume {color:"#ffffff".into(),..imported.clone()},
        Costume {color:"sunny".into(),hat:Some("model-robot".into()),..Costume::default()},
        Costume {color:"sunny".into(),model_id:Some("model-robot".into()),hat:Some("hat-cap".into()),..Costume::default()},
    ] {
        let mut room = host();
        assert!(room.start_sandbox_with_preview("host", draft, Some(&wardrobe)).is_err());
        assert!(!room.sandbox);
        assert_eq!(room.phase, Phase::Lobby);
        assert_eq!(room.members.len(), 1, "no bots or board are added on invalid preview");
        assert!(room.members[0].costume.is_none());
    }
}

#[test]
fn sandbox_costume_restores_saved_account_outfit_including_none_vs_some() {
    let mut room = host();
    let saved = Costume {color:"sunny".into(),hat:Some("hat-cap".into()),..Costume::default()};
    room.apply_costume("host", saved);
    let original = serde_json::to_value(room.members[0].costume.as_ref().unwrap()).unwrap();
    room.start_sandbox_with_preview("host", Costume {color:"clover".into(),model_id:Some("model-knight".into()),..Costume::default()}, None).unwrap();
    room.dev("host", DevAction::Exit).unwrap();
    assert_eq!(serde_json::to_value(room.members[0].costume.as_ref().unwrap()).unwrap(), original);
}

#[test]
fn a_previewed_pucklab_jersey_shows_the_synced_design_never_the_clients() {
    let synced = serde_json::json!({"body":"#123B74","sleeve_stripes":[],"hem_stripes":[]});
    let wardrobe = Wardrobe { costume: Costume::default(), owned: vec!["shirt-pucklab".into()], set_up: true, jersey: Some(synced.clone()) };
    let forged = Costume {color:"sunny".into(),shirt:Some("shirt-pucklab".into()),jersey:Some(serde_json::json!({"body":"#FF0000"})),..Costume::default()};
    let mut room = host();
    room.start_sandbox_with_preview("host", forged.clone(), Some(&wardrobe)).unwrap();
    assert_eq!(room.members[0].costume.as_ref().unwrap().jersey, Some(synced));
    let mut room = host();
    room.start_sandbox_with_preview("host", forged, None).unwrap();
    assert_eq!(room.members[0].costume.as_ref().unwrap().jersey, None, "no synced design draws the default jersey");
}

#[test]
fn included_items_are_never_registered_for_sale() {
    let included: Vec<_> = cosmetics().iter().filter(|item| item.included_with.is_some()).map(|item| item.id.as_str()).collect();
    assert_eq!(included, ["shirt-pucklab"]);
}

#[test]
fn sandbox_events_are_manual_and_crawler_preview_can_return_to_player() {
    let mut room=Room::new("manual-sandbox",17);
    room.connect("host","Host","").unwrap();
    room.dev_tools=true;
    room.start_sandbox("host").unwrap();
    assert!(room.board.as_ref().unwrap().candy_van_view().is_none());
    for _ in 0..crate::room::TICK_HZ*10 {room.step();}
    assert!(room.board.as_ref().unwrap().candy_van_view().is_none());
    room.dev("host",DevAction::Crawler).unwrap();
    assert_eq!(room.board.as_ref().unwrap().crawler_view().controller,Some(0));
    room.dev("host",DevAction::Crawler).unwrap();
    assert_eq!(room.board.as_ref().unwrap().crawler_view().controller,None);
    assert_eq!(room.board.as_ref().unwrap().turn_seat(),0);
    room.dev("host",DevAction::CandyVan).unwrap();
    assert!(room.board_paused());
}

#[test]
fn a_pet_look_rides_along_with_the_costume_and_only_catalog_gear_previews() {
    let look: Costume = serde_json::from_value(serde_json::json!({"color":"clover","pet":{"id":"pet-fizz","hat":"pet-hat-party","neck":"pet-neck-bell","coat":"pet-coat-frost"}})).unwrap();
    let mut room = host();
    room.apply_costume("host", look.clone());
    let view = serde_json::to_value(&room.members[0]).unwrap();
    assert_eq!(view["costume"]["pet"], serde_json::json!({"id":"pet-fizz","coat":"pet-coat-frost","hat":"pet-hat-party","neck":"pet-neck-bell"}), "clients draw what the pet wears");
    assert!(serde_json::to_value(Costume { color: "clover".into(), ..Costume::default() }).unwrap().get("pet").is_none());
    assert!(validate_sandbox_costume(&look, None).is_ok());
    for item in pet_items() {
        let mut pet = PetLook::default();
        *match item.slot.as_str() { "pet" => &mut pet.id, "pet-coat" => &mut pet.coat, "pet-hat" => &mut pet.hat, _ => &mut pet.neck } = Some(item.id.clone());
        assert!(validate_sandbox_costume(&Costume { color: "clover".into(), pet, ..Costume::default() }, None).is_ok(), "{}", item.id);
    }
    for pet in [PetLook { hat: Some("hat-cap".into()), ..PetLook::default() }, PetLook { neck: Some("pet-hat-party".into()), ..PetLook::default() }, PetLook { id: Some("pet-dragon".into()), ..PetLook::default() }] {
        assert!(validate_sandbox_costume(&Costume { color: "clover".into(), pet, ..Costume::default() }, None).is_err());
    }
}
