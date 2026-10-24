use serde::{Deserialize, Serialize};
use specta::Type;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::collections::HashMap;

// Circular dependency note is fine with Arc/Weak.
use crate::gamestate::{GameState, Player};
use workflow::server_action::ServerActionContext;

// pub mod assassin;
pub mod bound;
pub mod follower;
pub mod hollow;
pub mod lost;
pub mod lucid;
mod registrar;
pub mod saboteur;
pub mod spy;
pub mod taker;
pub mod tangle;
pub mod unmarked;
pub mod witness;

// pub mod witness;
pub use registrar::{BoxedStateHandler, RoleRegistrar};

// pub mod echo;
// ... (other modules)

// ==================================================================================
// 1. SHARED TYPES
// ==================================================================================

#[derive(Hash, Type, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Alliance {
    Hollow,
    Unmarked,
    Assassin,
    // Add others if needed
}

/// The structure returned by a Role when it wants to trigger a UI/Workflow.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowDefinitionWithInput {
    pub definition: String, // e.g. "bound_night_workflow"
    pub input: HashMap<String, serde_json::Value>,
}

// ==================================================================================
// 2. LOGIC CLOSURES (The "Brain")
// ==================================================================================

/// Defines what happens when a player performs a simple target action.
pub type NightActionLogic = Arc<
    dyn Fn(
            &mut GameState,
            &str, /* actor_id */
            &str, /* target_id */
        ) -> anyhow::Result<()>
        + Send
        + Sync,
>;

/// Checks if the team/role has won.
pub type WinnerCheck = Arc<dyn Fn(&GameState) -> bool + Send + Sync>;

/// ASYNC HOOK: Called when the game starts.
/// Used to register Server Actions and Workflow Definitions.
pub type RegisterHook =
    Arc<dyn Fn(Arc<dyn RoleRegistrar>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// ASYNC HOOK: Called when it is this role's turn.
/// Returns the Workflow (UI) to show the user.
pub type NightAbilityHook = Arc<
    dyn Fn(
            ServerActionContext,
        ) -> Pin<Box<dyn Future<Output = Option<WorkflowDefinitionWithInput>> + Send>>
        + Send
        + Sync,
>;

// ==================================================================================
// 3. ROLE CARD DATA
// ==================================================================================

#[derive(Clone, Debug)]

pub struct WorkflowStart {
    pub workflow_id: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub context: HashMap<String, serde_json::Value>,
}
pub type WorkflowHook = Arc<dyn Fn(&GameState, &Player) -> Option<WorkflowStart> + Send + Sync>;
#[derive(Clone)]
pub struct RoleCard {
    // Metadata
    pub id: String,
    pub role_id: String,
    pub name: String,
    pub description: String,
    pub alliance: Alliance,
    pub image_url: String,

    pub priority: i32,
    pub night_duration: i64,
    pub enabled: bool,

    pub night_action: Option<NightActionLogic>,
    pub winner_check: Option<WinnerCheck>,
    pub register: Option<RegisterHook>,
    pub on_night_start: Option<WorkflowHook>,
    pub enabled_texts: Vec<String>,
}

impl PartialEq for RoleCard {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl RoleCard {
    pub fn unknown() -> Arc<Self> {
        Arc::new(Self {
            id: "unknown".into(),
            role_id: "unknown".into(),
            name: "Unknown".into(),
            description: "???".into(),
            alliance: Alliance::Unmarked,
            priority: 0,
            enabled_texts: vec![],
            night_duration: 0,
            image_url: "".into(),
            enabled: false,
            night_action: None,
            winner_check: None,
            register: None,       // Default to None
            on_night_start: None, // Default to None
        })
    }
}

// ==================================================================================
// 4. SERIALIZATION (Skip Logic Fields)
// ==================================================================================

impl std::fmt::Debug for RoleCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoleCard")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

impl Serialize for RoleCard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // DTO excludes all function pointers
        #[derive(Serialize)]
        pub struct RoleCardDto<'a> {
            id: &'a str,
            name: &'a str,
            role_id: &'a str,
            description: &'a str,
            alliance: &'a Alliance,
            priority: i32,
            night_duration: i64,
            image_url: &'a str,
            enabled: bool,
        }

        let dto = RoleCardDto {
            id: &self.id,
            role_id: &self.role_id,
            name: &self.name,
            description: &self.description,
            alliance: &self.alliance,
            priority: self.priority,
            night_duration: self.night_duration,
            image_url: &self.image_url,
            enabled: self.enabled,
        };
        dto.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RoleCard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RoleCardDto {
            id: String,
            role_id: String,
            name: String,
            description: String,
            alliance: Alliance,
            priority: i32,
            night_duration: i64,
            image_url: String,
            #[serde(default)]
            enabled: bool,
        }
        let dto = RoleCardDto::deserialize(deserializer)?;

        Ok(RoleCard {
            id: dto.id,
            enabled_texts: vec![],
            role_id: dto.role_id,
            name: dto.name,
            description: dto.description,
            alliance: dto.alliance,
            priority: dto.priority,
            night_duration: dto.night_duration,
            image_url: dto.image_url,
            enabled: dto.enabled,

            // Initialize all logic hooks to None
            night_action: None,
            winner_check: None,
            register: None,
            on_night_start: None,
        })
    }
}
