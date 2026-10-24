//! The data model of a tawks workflow: definitions, nodes, displays, inputs,
//! conditions and the builders bots use to write them.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::{Generics, Type, TypeCollection, datatype::DataType};
use std::collections::HashMap;

use crate::{
    service::{InjectWorkflowPlace, WorkflowResource},
    transform::TransformOp,
};

pub mod builder;
pub mod condition;
pub mod error;
pub mod path;
pub mod schedule;
pub mod server_action;
pub mod service;
pub mod transform;

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    Text {
        placeholder: Option<String>,
    },
    Wysiwyg,
    Select {
        filter: Option<NodeCondition>,
        items_key: String,
        r#as: String,
        value: String,
        content: Vec<WorkflowDisplay>,
        selected_content: Option<Vec<WorkflowDisplay>>,
        unselected_content: Option<Vec<WorkflowDisplay>>,
        class: String,
    },
    Switch,
    Checkbox {
        label: String,
    },
    Number {
        min: Option<u32>,
        max: Option<u32>,
    },
    TextArea {
        placeholder: Option<String>,
        rows: Option<u32>,
    },
    List {
        from_responses_id: String,
        item_key: String,
        display_field: String,
    },
    ServerActionLoader {
        target: String,
    },
    Display {
        content: String,
    },
    MultiSelectFromList {
        items_key: String,
        r#as: String,
        value: String,
        content: Vec<WorkflowDisplay>,
        class: String,
        min: Option<u32>,
        max: Option<u32>,
    },
    SelectFromList {
        items_key: String,
        r#as: String,
        value: String,
        class: String,
        selected_class: String,
        content: Vec<WorkflowDisplay>,
    },
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum CardLayout {
    Vertical,
    Image {
        image_key: String,
    },
    ActionImage {
        image_key: String,
        update_responses_key: String,
        from_input_key: String,
        action: String,
    },
}

#[derive(Hash, Eq, PartialEq, Type, Debug, Clone, Serialize, Deserialize)]
pub enum Breakpoint {
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

#[derive(Hash, Eq, PartialEq, Type, Debug, Clone, Serialize, Deserialize)]
pub enum FlexDirection {
    Col,
    Row,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct SrcSet {
    value_key: String,
    media_key: String,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct TableHeader {
    pub label: String,
    pub key: String,
    pub width: Option<String>,
    pub align: Option<String>,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDisplay {
    pub class: String,
    pub r#type: DisplayType,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum IconLibrary {
    Lucide,
    Url,
    Emoji,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Bar,
    Line,
    Area,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum TimeDisplayOpts {
    // Matches: { timeago: true }
    TimeAgo {
        timeago: bool, // should be true
    },
    // Matches:
    // {} OR { timeago: false, format?: "...", showTimeForToday?: bool, timeForTodayFormat?: "..." }
    Format {
        #[serde(default)]
        timeago: Option<bool>, // omitted or false
        #[serde(default)]
        format: Option<String>,
        #[serde(default, rename = "showTimeForToday")]
        show_time_for_today: Option<bool>,
        #[serde(default, rename = "timeForTodayFormat")]
        time_for_today_format: Option<String>,
    },
}

impl Default for TimeDisplayOpts {
    fn default() -> Self {
        TimeDisplayOpts::Format {
            timeago: None,
            format: None,
            show_time_for_today: None,
            time_for_today_format: None,
        }
    }
}

// Convenience constructors
impl TimeDisplayOpts {
    pub fn timeago() -> Self {
        TimeDisplayOpts::TimeAgo { timeago: true }
    }
    pub fn formatted(
        format: Option<String>,
        show_time_for_today: Option<bool>,
        time_for_today_format: Option<String>,
    ) -> Self {
        TimeDisplayOpts::Format {
            timeago: Some(false),
            format,
            show_time_for_today,
            time_for_today_format,
        }
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum SubmitWhen {
    OnChange,
    OnSubmit,
}
#[derive(Type, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonType {
    Submit,
    Button,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct CommandPaletteConfig {
    #[serde(default)]
    pub placeholder: Option<String>,
    pub groups: Vec<CommandGroup>,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct CommandGroup {
    pub heading: Option<String>,
    pub items: Vec<CommandItem>,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct CommandItem {
    pub id: String,
    pub label: String,
    pub keywords: Vec<String>,

    pub icon: Option<String>,
    pub shortcut: Option<String>,

    pub action_id: String,
    pub action_input_map: Option<HashMap<String, String>>,
    pub data: HashMap<String, Value>,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum DisplayType {
    ScoreTicker {
        value_key: String,
        icon: Box<WorkflowDisplay>,
        container_class: Option<String>,
        pop_text_class: Option<String>,
        pop_container_class: Option<String>,
    },
    Form {
        server_action_id: String,
        content: Vec<WorkflowDisplay>,
        submit_when: SubmitWhen,
    },
    CommandPalette {
        key: String,
    },
    Popover {
        trigger: Vec<WorkflowDisplay>,
        content: Vec<WorkflowVisual>,
        opened_server_action_id: Option<String>,
        closed_server_action_id: Option<String>,
        content_class: String,
    },
    Searchable {
        data_url: String,
        target_key: String,
        r#as: String,
        placeholder: Option<String>,
        list_content: Vec<WorkflowDisplay>,
        input_key: String,
    },
    Time {
        opts: TimeDisplayOpts,
        key: String,
    },
    Icon {
        lib: IconLibrary,
        name: String,
        aria_label: Option<String>,
        url: Option<String>,
    },

    Chart {
        r#type: ChartType,
        chart_data_key: String,
        config: Value,
        x: String,
        y: String,
        axis: Option<String>,
    },

    Countdown {
        until: String,
        r#as: String,
        text: String,
        shake_under_ms_left: Option<u32>,
    },
    Tile {
        workflow: String,
    },
    TooltipButton {
        tooltip_content: Vec<WorkflowDisplay>,
        content: Vec<WorkflowDisplay>,
        input_map: Option<HashMap<String, String>>,
        action_id: String,
        variant: String,
        size: String,
    },
    Tooltip {
        class: String,
        tooltip_html: String,
        view_display: Vec<WorkflowDisplay>,
    },
    Input {
        id: String,
        input: WorkflowInput,
    },
    Condition {
        condition: NodeCondition,
        content: Vec<WorkflowDisplay>,
    },
    Button {
        content: Vec<WorkflowDisplay>,
        input_map: Option<HashMap<String, Value>>,
        action_id: String,
        variant: String,
        size: String,
        r#type: ButtonType,
    },
    Progress {
        progress_percentage_key: String,
        percentage_class: String,
    },
    Badge {
        variant: String,
        text: String,
    },
    RevealCards {
        reveal_card_keys: Vec<String>,
    },
    SelectCards {
        card_id_keys: Vec<String>,
    },
    Text {
        text: String,
    },
    Card {
        content: Vec<WorkflowDisplay>,
        layout: CardLayout,
    },
    UserAvatar {
        key: String,
    },
    Image {
        src: String,
        alt_text: String,
    },
    List {
        items_key: String,
        r#as: String,
        content: Vec<WorkflowDisplay>,
    },
    Div {
        content: Vec<WorkflowDisplay>,
    },
    Carousel {
        items_key: String,
        r#as: String,
        content: Vec<WorkflowDisplay>,
        /// Workflow action to run when the user swipes to a slide; `input_map`
        /// is interpolated against that slide's item. `active_key` names a
        /// boolean field on the item marking the initially selected slide.
        #[serde(default)]
        on_select_action: Option<String>,
        #[serde(default)]
        input_map: Option<HashMap<String, Value>>,
        #[serde(default)]
        active_key: Option<String>,
    },
    Table {
        items_key: String,
        r#as: String,
        headers: Vec<TableHeader>,
        row: Vec<WorkflowDisplay>,
    },
    Wysiwyg {
        content_key: String,
    },
    MultimediaLobby,
    Video {
        src: String,
        poster_src: Option<String>,
    },
    /// Plays the opening `seconds` of an audio file; `marks` are the
    /// unlock points shown along the bar.
    AudioClip {
        src: String,
        seconds: String,
        marks: Vec<f64>,
    },
    /// A connect-the-dots board read from the response `board`; dragging a path
    /// and letting go runs `action` with the path as input `path`.
    DotsBoard {
        board: String,
        action: String,
    },
    /// A letter wheel over a crossword read from the response `puzzle`; swiping
    /// a word runs `action` with the word as input `word`.
    WordWheel {
        puzzle: String,
        action: String,
    },
    /// A Snakebite run on the day's track read from the response `level`;
    /// a finished run submits its steering to `action` as input `inputs`.
    SnakeRun {
        level: String,
        action: String,
    },
    /// A Railyard puzzle read from the response `board`: players lay track,
    /// `save_action` keeps it as they go and `action` runs it (input
    /// `tracks` and `level` on both).
    RailyardBoard {
        board: String,
        action: String,
        save_action: String,
    },
    /// A "Share result" button for the response `value`, an object with
    /// `path`, `title` and `text` (see `tawks_bot::share`); hidden while the
    /// value is missing.
    ShareResult {
        value: String,
        label: String,
    },
    Loader,
    Link {
        href: String,
        content: Vec<WorkflowDisplay>,
    },
    /// The registered activity for this workflow, its web client in a
    /// sandboxed frame. The client gets the response `state` and sends each
    /// move to `action` as input `input`.
    Activity {
        state: String,
        action: String,
    },
    BattlefieldCanvas,
    TracklinesCanvas,
    PartyCanvas,
    RemotePlayCanvas,
    NhlStreams,
    FpsCanvas {
        server_url_key: String,
        match_id_key: String,
        token_key: String,
        cert_hash_key: String,
    },
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum NotificationVariant {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum StickyPosition {
    Top,
    Bottom,
    Toolbar,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum DisplayVisual {
    // Conditional {
    //     action_id: String,
    //     inputs: HashMap<String, String>,
    // },
    Animation {
        id: String,
        animation_type: String,
        trigger: Option<String>,
        delay_ms: Option<u32>,
        meta: HashMap<String, Value>,
    },
    Loading {
        action_id: String,
        inputs: HashMap<String, String>,
    },
    Notification {
        variant: NotificationVariant,
        content: Vec<WorkflowDisplay>,
    },
    Modal {
        content: Vec<WorkflowDisplay>,
    },
    Sticky {
        position: StickyPosition,
        content: Vec<WorkflowDisplay>,
    },
    Tile {
        content: Vec<WorkflowDisplay>,
    },
    Block {
        class: String,
        content: Vec<WorkflowDisplay>,
    },

    Chat {
        chat_id: String,
    },

    Redirect {
        url_key: String,
    },
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowVisual {
    pub id: String,
    pub display: DisplayVisual,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub readonly: Option<NodeCondition>,
    pub r#type: InputType,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
    pub on_change_action: Option<(String, HashMap<String, String>)>,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAction {
    pub id: String,
    pub action_type: ActionType,
    pub allow_while_loading: bool,
    pub target: Option<String>,
    pub progress_node: bool,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    NextNode,
    PreviousNode,
    Submit,
    Cancel,
    RunServerAction,
    TransformThenRun {
        ops: Vec<TransformOp>,
        reconcile_node: bool,
        server_action_id: String,
        input_map: Option<HashMap<String, String>>,
    },
    TransformResponses {
        ops: Vec<TransformOp>,
        reconcile_node: bool,
    },
    StartWorkflow {
        input_map: Option<HashMap<String, String>>,
        responses: HashMap<String, Value>,
        context: HashMap<String, Value>,
        complete_workflow: bool,
        inject_place: Option<InjectWorkflowPlace>,
        on_complete_ops: Vec<TransformOp>,
    },
    Noop,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowPredicate {
    ByWorkflowDefinitionIdAndUserId(String, String),
    And(Vec<WorkflowPredicate>),
    Or(Vec<WorkflowPredicate>),
}
impl WorkflowPredicate {
    pub fn evaluate(&self, definition: &WorkflowDefinition, workflow: &WorkflowResource) -> bool {
        match self {
            WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(workflow_definition_id, user_id) => {
                if &workflow.user_id != user_id {
                    return false;
                }

                let owner = definition.owner_id.clone().unwrap_or_default();
                let owned_id = format!("wf-{}-{}", workflow_definition_id, owner);
                &workflow.workflow_id == workflow_definition_id || workflow.workflow_id == owned_id
            }
            WorkflowPredicate::And(predicates) => predicates
                .iter()
                .all(|predicate| predicate.evaluate(definition, workflow)),
            WorkflowPredicate::Or(predicates) => predicates
                .iter()
                .any(|predicate| predicate.evaluate(definition, workflow)),
        }
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum NodeCondition {
    SomeInList {
        list: String,
        r#as: String,
        condition: Box<NodeCondition>,
    },
    Not(Box<NodeCondition>),
    And(Vec<NodeCondition>),
    Or(Vec<NodeCondition>),
    ResponseExists(String),
    ContextEquals {
        field: String,
        value: Value,
    },
    ResponseEquals {
        field: String,
        value: Value,
    },
    ResponseListEmpty(String),
    Always,
    Gte {
        field: String,
        value: Value,
    },
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub title: String,
    pub description: Option<String>,
    pub visuals: Vec<WorkflowVisual>,
    pub layout: Option<String>,
    pub condition: Option<NodeCondition>,
    pub parent_id: Option<String>,
}
impl WorkflowNode {
    pub fn parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: NodesMap,
    pub responses: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub actions: Vec<WorkflowAction>,
    pub starting_server_action: Option<String>,
    /// When true, every `start_workflow*` call creates a fresh instance.
    /// When false (default), the workflow runtime reuses an existing
    /// non-terminal instance for the same `(workflow_id, user_id)` (or
    /// `(workflow_id, user_id, parent_instance_id, inject)` for attached
    /// children) instead of creating a new one. Most workflows in this
    /// codebase are conceptually "the user's downloader page" — single
    /// instance per user — and benefit from the default. Opt in for
    /// workflows that legitimately want multiple concurrent instances per
    /// user (e.g. each download getting its own progress workflow).
    #[serde(default)]
    pub multi_instance_per_user: bool,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub owner_id: Option<String>,
    pub starting_server_action: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub responses: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub nodes: NodesMap,
    pub actions: Vec<WorkflowAction>,
    /// See `CreateWorkflowDefinition::multi_instance_per_user`. Mirrors that
    /// flag through the catalog so the manager can read it during
    /// dedupe / resume decisions.
    #[serde(default)]
    pub multi_instance_per_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct NodesMap(pub IndexMap<String, WorkflowNode>);
impl Type for NodesMap {
    fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
        <HashMap<String, WorkflowNode> as Type>::inline(type_map, generics)
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct ServerActionDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct AttachedWorkflowMeta {
    pub instance_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WorkflowDefinitionRef {
    pub definition_uid: String,
    pub scoped_workflow_id: String,
    pub workflow_key: String,
    pub owner_id: String,
    pub version: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRuntimeStatus {
    Active,
    Waiting,
    Completed,
    Cancelled,
    Failed,
}

impl WorkflowRuntimeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkflowRuntimeStatus::Active => "active",
            WorkflowRuntimeStatus::Waiting => "waiting",
            WorkflowRuntimeStatus::Completed => "completed",
            WorkflowRuntimeStatus::Cancelled => "cancelled",
            WorkflowRuntimeStatus::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "active" => Some(WorkflowRuntimeStatus::Active),
            "waiting" => Some(WorkflowRuntimeStatus::Waiting),
            "completed" => Some(WorkflowRuntimeStatus::Completed),
            "cancelled" => Some(WorkflowRuntimeStatus::Cancelled),
            "failed" => Some(WorkflowRuntimeStatus::Failed),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            WorkflowRuntimeStatus::Completed
                | WorkflowRuntimeStatus::Cancelled
                | WorkflowRuntimeStatus::Failed
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub definition_ref: WorkflowDefinitionRef,
    pub inject: Option<InjectWorkflowPlace>,
    pub workflow_id: String,
    pub instance_id: String,
    pub user_id: String,
    pub current_node_id: Option<String>,
    pub node_history: Vec<String>,
    pub responses: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub message_id: Option<String>,
    pub completed: bool,
    pub waiting: bool,
    pub status: WorkflowRuntimeStatus,
    pub complete_message: Option<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing, skip_deserializing)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing, skip_deserializing)]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,

    pub attached_workflows: HashMap<String, AttachedWorkflowMeta>,

    pub parent_instance_id: Option<String>,
    pub parent_attach_id: Option<String>,
    pub return_ops: Vec<TransformOp>,
    pub return_inject_response_as: Option<String>,
    pub return_reconcile_node: bool,

    /// Bumped on every saved change; a write made from an older read fails.
    #[serde(skip_serializing, skip_deserializing)]
    pub version: i64,
}

impl WorkflowState {
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    pub fn is_waiting(&self) -> bool {
        matches!(self.status, WorkflowRuntimeStatus::Waiting)
    }

    pub fn sync_runtime_status(&mut self) {
        let status = match self.status {
            WorkflowRuntimeStatus::Cancelled | WorkflowRuntimeStatus::Failed => self.status,
            _ if self.completed => WorkflowRuntimeStatus::Completed,
            _ if self.waiting => WorkflowRuntimeStatus::Waiting,
            _ => WorkflowRuntimeStatus::Active,
        };

        self.apply_status(status);
    }

    fn apply_status(&mut self, status: WorkflowRuntimeStatus) {
        self.status = status;
        self.completed = status.is_terminal();
        self.waiting = matches!(status, WorkflowRuntimeStatus::Waiting);

        if status.is_terminal() {
            if self.completed_at.is_none() {
                self.completed_at = Some(self.updated_at);
            }
        } else {
            self.completed_at = None;
        }
    }

    pub fn set_status(&mut self, status: WorkflowRuntimeStatus) {
        self.apply_status(status);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorkflowPreferences {
    pub user_id: String,
    pub workflow_id: String,
    pub saved_responses: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing, skip_deserializing)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::WorkflowResource;

    fn test_definition() -> WorkflowDefinition {
        WorkflowDefinition {
            multi_instance_per_user: false,
            id: "wf-approval-alice".to_string(),
            owner_id: Some("alice".to_string()),
            starting_server_action: None,
            name: "Approval".to_string(),
            description: None,
            responses: HashMap::new(),
            context: HashMap::new(),
            nodes: NodesMap::default(),
            actions: Vec::new(),
        }
    }

    fn test_resource(user_id: &str, workflow_id: &str) -> WorkflowResource {
        WorkflowResource {
            inject: None,
            instance_id: "inst-1".to_string(),
            workflow_id: workflow_id.to_string(),
            stale_definition: false,
            name: "Approval".to_string(),
            description: None,
            responses: HashMap::new(),
            completed: false,
            status: WorkflowRuntimeStatus::Active,
            complete_message: None,
            visuals: Vec::new(),
            layout: None,
            user_id: user_id.to_string(),
            current_node_id: "root".to_string(),
            waiting: false,
            attached_workflows: HashMap::new(),
            parent: None,
            created_at: "2026-03-27 00:00:00".to_string(),
            updated_at: "2026-03-27 00:00:00".to_string(),
            completed_at: None,
        }
    }

    #[test]
    fn workflow_predicate_and_composes_nested_checks() {
        let definition = test_definition();
        let workflow = test_resource("alice", "wf-approval-alice");

        let predicate = WorkflowPredicate::And(vec![
            WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(
                "approval".to_string(),
                "alice".to_string(),
            ),
            WorkflowPredicate::Or(vec![
                WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(
                    "different".to_string(),
                    "alice".to_string(),
                ),
                WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(
                    "approval".to_string(),
                    "alice".to_string(),
                ),
            ]),
        ]);

        assert!(predicate.evaluate(&definition, &workflow));
    }

    #[test]
    fn workflow_predicate_or_returns_false_when_no_branch_matches() {
        let definition = test_definition();
        let workflow = test_resource("alice", "wf-approval-alice");

        let predicate = WorkflowPredicate::Or(vec![
            WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(
                "approval".to_string(),
                "bob".to_string(),
            ),
            WorkflowPredicate::And(vec![
                WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(
                    "other".to_string(),
                    "alice".to_string(),
                ),
                WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(
                    "approval".to_string(),
                    "alice".to_string(),
                ),
            ]),
        ]);

        assert!(!predicate.evaluate(&definition, &workflow));
    }
}
