use std::sync::Arc;
use ulid::Ulid;

use crate::roles::{Alliance, RoleCard};

pub const UNMARKED_ROLE_ID: &str = "unmarked";

pub fn unmarked_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,

        id: Ulid::new().to_string(),
        role_id: UNMARKED_ROLE_ID.to_string(),

        name: "The Unmarked".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/unmarked.png".into(),

        // Keep your old intent: “default / lowest priority in night order”
        priority: 100,
        alliance: Alliance::Unmarked,

        enabled,
        night_duration: 0,

        // No workflow, no registration, no night behavior
        night_action: None,
        register: None,
        on_night_start: None,
    })
}
