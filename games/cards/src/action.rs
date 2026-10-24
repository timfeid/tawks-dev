use crate::card::Card;
use crate::types::TargetRef;
use serde::{Deserialize, Serialize};
use tawks_data::pagination::PaginationArgs;

/// One sideboard swap: exchange a main-deck card for a sideboard card (or
/// vice versa). The engine validates totals match and that the player owns
/// the necessary copies.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SideboardSwapEntry {
    /// Template id of the card to move OUT of the main deck (into sideboard).
    pub main_out: String,
    /// Template id of the card to move IN from the sideboard.
    pub side_in: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CardGameAction {
    View {
        name: String,
        instance_id: String,
    },
    AddAi,
    Join {
        name: String,
        avatar: String,
        instance_id: String,
    },
    SetGameFormat {
        format: crate::types::GameFormat,
    },
    SetGameMode {
        mode: crate::types::GameMode,
    },
    SelectDeck {
        deck_id: String,
    },
    StartGame,
    PlayLand {
        card_id: String,
    },
    DeployNode {
        mana_type: crate::types::ManaType,
    },
    TapCard {
        card_id: String,
    },
    CastSpell {
        card_id: String,
        #[serde(default)]
        targets: Vec<TargetRef>,
    },
    ActivateAbility {
        card_id: String,
        ability_id: String,
        #[serde(default)]
        targets: Vec<TargetRef>,
    },
    DeclareAttack {
        card_id: String,
        #[serde(default)]
        target: Option<TargetRef>,
    },
    DeclareBlock {
        attacker_id: String,
        blocker_id: String,
    },
    ScryCommit {
        #[serde(default)]
        top: Vec<String>,
        #[serde(default)]
        bottom: Vec<String>,
    },
    SearchCommit {
        card_id: String,
    },
    SearchCancel,
    TriggerChoice {
        accept: bool,
    },
    TriggerTargetSelect {
        target: TargetRef,
    },
    TriggerTargetCancel,
    DiscardCommit {
        card_ids: Vec<String>,
    },
    Discard {
        card_id: String,
    },
    Sacrifice {
        card_id: String,
    },
    ResolveStackTop,
    CancelStackTop,
    PassPriority,
    NextStage,
    ConfirmAttackers,
    ConfirmBlockers,
    Ping {
        x: f32,
        z: f32,
    },
    ReorderBattlefield {
        card_ids: Vec<String>,
    },
    SaveCard {
        card: Card,
    },
    ListCards {
        pagination: PaginationArgs,
    },
    BuilderLoadDeck {
        deck_id: String,
    },
    BuilderAddCard {
        deck_id: String,
        card_id: String,
    },
    BuilderRemoveCard {
        deck_id: String,
        card_id: String,
    },
    /// Add a card to the sideboard zone of the deck being edited.
    BuilderAddSideboardCard {
        deck_id: String,
        card_id: String,
    },
    /// Remove a card from the sideboard zone of the deck being edited.
    BuilderRemoveSideboardCard {
        deck_id: String,
        card_id: String,
    },
    BuilderSetOperator {
        deck_id: String,
        card_id: String,
    },
    BuilderCloneDeck {
        deck_id: String,
    },
    BuilderSetFilter {
        kind: Option<String>,
    },
    BuilderSetSort {
        sort: Option<String>,
    },
    BuilderOpen,
    BuilderClose,
    BuilderCreateDeck,
    BuilderAutobuild,
    // Card browser actions
    BrowserOpen {
        #[serde(default)]
        target_player_id: Option<String>,
    },
    BrowserClose,
    BrowserSetFilter {
        kind: Option<String>,
    },
    BrowserSetSort {
        sort: Option<String>,
    },
    BrowserPage {
        delta: i32,
    },
    DebugSetup,
    /// Every AI at the table drops to 0 life, so a test can reach a win.
    /// Refused unless the games service runs with `CARDGAME_DEBUG_ACTIONS=1`.
    DebugAiConcede,
    // Draft actions
    /// Lobby: toggle the upcoming match's best-of format. Only the host may
    /// invoke this pre-match.
    SetMatchFormat {
        format: crate::types::MatchFormat,
    },
    /// Submit this player's sideboard swap between games of a Bo3 match.
    /// Pairs of (from_zone, card_template_id) — `In` pulls a copy from the
    /// sideboard pool into the main deck; `Out` removes a main-deck copy to
    /// the sideboard. Must leave both sides with the same total card count.
    SubmitSideboard {
        #[serde(default)]
        swaps: Vec<SideboardSwapEntry>,
    },
    /// Back to an empty lobby; the host may do it any time, anyone once a game is finished.
    Reset,
    DraftStart,
    DraftPick {
        index: usize,
    },
    DraftAutoPick,
    DraftConfirmPool,
    DraftKeepCards {
        card_ids: Vec<String>,
    },
    // Pack actions
    ListPackTypes,
    GetUserPacks,
    OpenPack {
        pack_instance_id: String,
    },
    GetPackOpeningResults {
        pack_instance_id: String,
    },
    AdminGrantPack {
        user_id: String,
        pack_type_id: String,
    },
    AiExecute {
        player_id: String,
        action: Box<CardGameAction>,
    },
}
