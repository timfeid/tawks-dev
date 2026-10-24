use std::sync::Arc;

use ulid::Ulid;

use crate::gamestate::TurnContext;
use crate::roles::{Alliance, RoleCard};

pub fn echo_card() -> RoleCard {
    RoleCard {
        winner_check: None,
        id: Ulid::new().to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/echo.png".into(),
        priority: 5,
        alliance: Alliance::Unmarked,
        register: None,
        enabled: false,
        name: "The Echo".to_string(),
        night_duration: 0,
        night_ability: Some(Arc::new(|ctx: TurnContext| Box::pin(async move { None }))),
    }
}
