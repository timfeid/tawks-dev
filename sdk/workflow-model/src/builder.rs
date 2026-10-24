use crate::{
    ActionType, ButtonType, CardLayout, ChartType, CreateWorkflowDefinition, DisplayType,
    DisplayVisual, IconLibrary, NodeCondition, NodesMap, NotificationVariant, StickyPosition,
    SubmitWhen, TableHeader, TimeDisplayOpts, TransformOp, WorkflowAction, WorkflowDisplay,
    WorkflowInput, WorkflowNode, WorkflowVisual, service::InjectWorkflowPlace,
};
use serde_json::Value;
use std::collections::HashMap;

//
// ────────────────────────────── CORE BUILDERS ──────────────────────────────
//

#[derive(Default)]
pub struct WorkflowDefinitionBuilder {
    id: String,
    name: String,
    description: Option<String>,
    starting_server_action: Option<String>,
    nodes: NodesMap,
    actions: Vec<WorkflowAction>,
    responses: HashMap<String, serde_json::Value>,
    context: HashMap<String, serde_json::Value>,
    multi_instance_per_user: bool,
}

impl WorkflowDefinitionBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn starting_server_action(mut self, action_id: impl Into<String>) -> Self {
        self.starting_server_action = Some(action_id.into());
        self
    }

    pub fn context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    pub fn response(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.responses.insert(key.into(), value);
        self
    }

    pub fn responses(mut self, map: HashMap<String, serde_json::Value>) -> Self {
        self.responses.extend(map);
        self
    }

    pub fn add_node(mut self, id: impl Into<String>, node: WorkflowNode) -> Self {
        self.nodes.0.insert(id.into(), node);
        self
    }

    pub fn add_action(mut self, action: WorkflowAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn add_actions(mut self, actions: impl IntoIterator<Item = WorkflowAction>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Opt this workflow definition into multi-instance-per-user behavior.
    /// Default is single-instance: every `start_workflow*` reuses the
    /// existing non-terminal instance for the same user (or attached to the
    /// same parent + inject slot for child workflows). Call this for
    /// workflows that legitimately want multiple concurrent instances per
    /// user — e.g. a per-download progress workflow, a per-conversation
    /// chat session, etc.
    pub fn multi_instance_per_user(mut self) -> Self {
        self.multi_instance_per_user = true;
        self
    }

    pub fn build(self) -> CreateWorkflowDefinition {
        CreateWorkflowDefinition {
            id: self.id,
            name: self.name,
            description: self.description,
            starting_server_action: self.starting_server_action,
            nodes: self.nodes,
            responses: self.responses,
            context: self.context,
            actions: self.actions,
            multi_instance_per_user: self.multi_instance_per_user,
        }
    }
}

#[derive(Default)]
pub struct WorkflowNodeBuilder {
    title: String,
    description: Option<String>,
    visuals: Vec<WorkflowVisual>,
    layout: Option<String>,
    condition: Option<NodeCondition>,
    parent_id: Option<String>,
}

impl WorkflowNodeBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn visual(mut self, visual: WorkflowVisual) -> Self {
        self.visuals.push(visual);
        self
    }

    pub fn visuals(mut self, visuals: impl IntoIterator<Item = WorkflowVisual>) -> Self {
        self.visuals.extend(visuals);
        self
    }

    pub fn condition(mut self, condition: NodeCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn layout(mut self, layout: impl Into<String>) -> Self {
        self.layout = Some(layout.into());
        self
    }

    pub fn parent_id(mut self, parent: impl Into<String>) -> Self {
        self.parent_id = Some(parent.into());
        self
    }

    pub fn build(self) -> WorkflowNode {
        WorkflowNode {
            title: self.title,
            description: self.description,
            visuals: self.visuals,
            layout: self.layout,
            condition: self.condition,
            parent_id: self.parent_id,
        }
    }
}

#[derive(Default, Clone)]
pub struct WorkflowVisualBuilder {
    id: String,
    display: Option<DisplayVisual>,
}

impl WorkflowVisualBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    pub fn display(mut self, display: DisplayVisual) -> Self {
        self.display = Some(display);
        self
    }

    // ── DisplayVisual variants ───────────────────────────────────────────────

    pub fn animation(
        mut self,
        id: impl Into<String>,
        animation_type: impl Into<String>,
        trigger: Option<String>,
        delay_ms: Option<u32>,
        meta: HashMap<String, Value>,
    ) -> Self {
        self.display = Some(DisplayVisual::Animation {
            id: id.into(),
            animation_type: animation_type.into(),
            trigger,
            delay_ms,
            meta,
        });
        self
    }

    pub fn loading(
        mut self,
        action_id: impl Into<String>,
        inputs: HashMap<String, String>,
    ) -> Self {
        self.display = Some(DisplayVisual::Loading {
            action_id: action_id.into(),
            inputs,
        });
        self
    }

    pub fn notification(
        mut self,
        variant: NotificationVariant,
        content: Vec<WorkflowDisplay>,
    ) -> Self {
        self.display = Some(DisplayVisual::Notification { variant, content });
        self
    }

    pub fn modal(mut self, content: Vec<WorkflowDisplay>) -> Self {
        self.display = Some(DisplayVisual::Modal { content });
        self
    }

    pub fn sticky(mut self, position: StickyPosition, content: Vec<WorkflowDisplay>) -> Self {
        self.display = Some(DisplayVisual::Sticky { position, content });
        self
    }

    pub fn tile(mut self, content: Vec<WorkflowDisplay>) -> Self {
        self.display = Some(DisplayVisual::Tile { content });
        self
    }

    pub fn block(mut self, class: impl Into<String>, content: Vec<WorkflowDisplay>) -> Self {
        self.display = Some(DisplayVisual::Block {
            class: class.into(),
            content,
        });
        self
    }

    pub fn redirect(mut self, url_key: impl Into<String>) -> Self {
        self.display = Some(DisplayVisual::Redirect {
            url_key: url_key.into(),
        });
        self
    }

    pub fn build(self) -> WorkflowVisual {
        WorkflowVisual {
            id: self.id,
            display: self.display.expect("Display must be set"),
        }
    }

    pub fn chat(mut self, chat_id: &str) -> Self {
        self.display = Some(DisplayVisual::Chat {
            chat_id: chat_id.to_string(),
        });
        self
    }
}

//
// ────────────────────────────── DISPLAY BUILDERS ────────────────────────────
//

pub struct WorkflowDisplayBuilder {
    class: String,
    kind: Option<DisplayType>,
}

impl WorkflowDisplayBuilder {
    pub fn user_avatar(mut self, key: impl Into<String>) -> Self {
        self.kind = Some(crate::DisplayType::UserAvatar { key: key.into() });
        self
    }

    pub fn command_palette(mut self, key: impl Into<String>) -> Self {
        self.kind = Some(crate::DisplayType::CommandPalette { key: key.into() });
        self
    }

    pub fn section_title(title: impl Into<String>) -> WorkflowDisplay {
        WorkflowDisplayBuilder::new("text-lg font-semibold px-4")
            .text(title)
            .build()
    }

    pub fn new(class: impl Into<String>) -> Self {
        Self {
            class: class.into(),
            kind: None,
        }
    }

    pub fn loader(mut self) -> Self {
        self.kind = Some(DisplayType::Loader);
        self
    }

    pub fn popover(
        mut self,
        opened_server_action_id: Option<&str>,
        closed_server_action_id: Option<&str>,
        trigger: Vec<WorkflowDisplay>,
        content: Vec<WorkflowVisual>,
        content_class: impl Into<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Popover {
            trigger,
            opened_server_action_id: opened_server_action_id
                .map(|opened_server_action_id| opened_server_action_id.into()),
            closed_server_action_id: closed_server_action_id
                .map(|closed_server_action_id| closed_server_action_id.into()),
            content,
            content_class: content_class.into(),
        });
        self
    }

    pub fn searchable(
        mut self,
        data_url: impl Into<String>,
        input_key: impl Into<String>,
        target_key: impl Into<String>,
        r#as: impl Into<String>,
        placeholder: Option<impl Into<String>>,
        list_content: Vec<WorkflowDisplay>,
    ) -> Self {
        self.kind = Some(DisplayType::Searchable {
            data_url: data_url.into(),
            target_key: target_key.into(),
            r#as: r#as.into(),
            placeholder: placeholder.map(|placeholder| placeholder.into()),
            list_content,
            input_key: input_key.into(),
        });
        self
    }

    pub fn bar_chart(
        mut self,
        chart_data_key: impl Into<String>,
        config: serde_json::Value,
        x: impl Into<String>,
        y: impl Into<String>,
        axis: Option<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Chart {
            r#type: ChartType::Bar,
            chart_data_key: chart_data_key.into(),
            config,
            x: x.into(),
            y: y.into(),
            axis,
        });
        self
    }

    pub fn area_chart(
        mut self,
        chart_data_key: impl Into<String>,
        config: serde_json::Value,
        x: impl Into<String>,
        y: impl Into<String>,
        axis: Option<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Chart {
            r#type: ChartType::Area,
            chart_data_key: chart_data_key.into(),
            config,
            x: x.into(),
            y: y.into(),
            axis,
        });
        self
    }

    pub fn line_chart(
        mut self,
        chart_data_key: impl Into<String>,
        config: serde_json::Value,
        x: impl Into<String>,
        y: impl Into<String>,
        axis: Option<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Chart {
            r#type: ChartType::Line,
            chart_data_key: chart_data_key.into(),
            config,
            x: x.into(),
            y: y.into(),
            axis,
        });
        self
    }

    pub fn build(self) -> WorkflowDisplay {
        WorkflowDisplay {
            class: self.class,
            r#type: self.kind.unwrap_or(DisplayType::Div { content: vec![] }),
        }
    }

    // ── DisplayType variants ────────────────────────────────────────────────

    pub fn countdown(
        mut self,
        until: impl Into<String>,
        r#as: impl Into<String>,
        text: impl Into<String>,
        shake_under_ms_left: Option<u32>,
    ) -> Self {
        self.kind = Some(DisplayType::Countdown {
            until: until.into(),
            r#as: r#as.into(),
            text: text.into(),
            shake_under_ms_left,
        });
        self
    }

    pub fn tile(mut self, workflow: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::Tile {
            workflow: workflow.into(),
        });
        self
    }

    pub fn tooltip_button(
        mut self,
        tooltip_content: Vec<WorkflowDisplay>,
        content: Vec<WorkflowDisplay>,
        input_map: Option<HashMap<String, String>>,
        action_id: impl Into<String>,
        variant: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        self.kind = Some(DisplayType::TooltipButton {
            tooltip_content,
            content,
            input_map,
            action_id: action_id.into(),
            variant: variant.into(),
            size: size.into(),
        });
        self
    }

    pub fn tooltip(
        mut self,
        tooltip_html: impl Into<String>,
        class: impl Into<String>,
        view_display: Vec<WorkflowDisplay>,
    ) -> Self {
        self.kind = Some(DisplayType::Tooltip {
            class: class.into(),
            tooltip_html: tooltip_html.into(),
            view_display,
        });
        self
    }

    pub fn input(mut self, id: impl Into<String>, input: WorkflowInput) -> Self {
        self.kind = Some(DisplayType::Input {
            id: id.into(),
            input,
        });
        self
    }

    pub fn mutlimedia_lobby(mut self) -> Self {
        self.kind = Some(DisplayType::MultimediaLobby);
        self
    }

    pub fn condition(mut self, condition: NodeCondition, content: Vec<WorkflowDisplay>) -> Self {
        self.kind = Some(DisplayType::Condition { condition, content });
        self
    }

    pub fn submit(
        mut self,
        content: Vec<WorkflowDisplay>,
        input_map: Option<HashMap<String, Value>>,
        action_id: impl Into<String>,
        variant: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Button {
            content,
            input_map,
            action_id: action_id.into(),
            variant: variant.into(),
            size: size.into(),
            r#type: ButtonType::Submit,
        });
        self
    }

    pub fn button(
        mut self,
        content: Vec<WorkflowDisplay>,
        input_map: Option<HashMap<String, Value>>,
        action_id: impl Into<String>,
        variant: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Button {
            content,
            input_map,
            action_id: action_id.into(),
            variant: variant.into(),
            size: size.into(),
            r#type: ButtonType::Button,
        });
        self
    }

    pub fn progress(
        mut self,
        progress_percentage_key: impl Into<String>,
        percentage_class: impl Into<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Progress {
            progress_percentage_key: progress_percentage_key.into(),
            percentage_class: percentage_class.into(),
        });
        self
    }

    pub fn badge(mut self, variant: impl Into<String>, text: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::Badge {
            variant: variant.into(),
            text: text.into(),
        });
        self
    }

    pub fn reveal_cards(mut self, reveal_card_keys: Vec<String>) -> Self {
        self.kind = Some(DisplayType::RevealCards { reveal_card_keys });
        self
    }

    pub fn select_cards(mut self, card_id_keys: Vec<String>) -> Self {
        self.kind = Some(DisplayType::SelectCards { card_id_keys });
        self
    }

    pub fn time(mut self, key: impl Into<String>, opts: TimeDisplayOpts) -> Self {
        self.kind = Some(DisplayType::Time {
            key: key.into(),
            opts,
        });
        self
    }

    pub fn score_ticker(
        mut self,
        value_key: impl Into<String>,
        lucide_icon: impl Into<String>,
        icon_class: impl Into<String>,
        container_class: Option<impl Into<String>>,
        pop_text_class: Option<impl Into<String>>,
        pop_container_class: Option<impl Into<String>>,
    ) -> Self {
        self.kind = Some(DisplayType::ScoreTicker {
            value_key: value_key.into(),
            icon: Box::new(
                WorkflowDisplayBuilder::new(icon_class.into())
                    .lucide_icon(lucide_icon, None)
                    .build(),
            ),
            container_class: container_class.map(|c| c.into()),
            pop_text_class: pop_text_class.map(|c| c.into()),
            pop_container_class: pop_container_class.map(|c| c.into()),
        });
        self
    }

    pub fn form(
        mut self,
        server_action_id: impl Into<String>,
        content: Vec<WorkflowDisplay>,
        submit_when: SubmitWhen,
    ) -> Self {
        self.kind = Some(DisplayType::Form {
            server_action_id: server_action_id.into(),
            content,
            submit_when,
        });
        self
    }

    pub fn activity(mut self, state: impl Into<String>, action: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::Activity {
            state: state.into(),
            action: action.into(),
        });
        self
    }

    pub fn battlefield_canvas(mut self) -> Self {
        self.kind = Some(DisplayType::BattlefieldCanvas);
        self
    }

    /// The 3D rail map for Tracklines; the client reads the game from `responses.game`.
    pub fn tracklines_canvas(mut self) -> Self {
        self.kind = Some(DisplayType::TracklinesCanvas);
        self
    }

    /// The party stage for real-time lobby minigames; the client reads its
    /// socket address and join pass from `responses.party`.
    pub fn party_canvas(mut self) -> Self {
        self.kind = Some(DisplayType::PartyCanvas);
        self
    }

    /// The remote play stage; the client reads its relay address and pass
    /// from `responses.remote_play`.
    pub fn remote_play_canvas(mut self) -> Self {
        self.kind = Some(DisplayType::RemotePlayCanvas);
        self
    }

    /// NHL feed selector and HLS player; reads `tn_date` or `tg_id`.
    pub fn nhl_streams(mut self) -> Self {
        self.kind = Some(DisplayType::NhlStreams);
        self
    }

    /// Render an FPS WebTransport client. The four `*_key` arguments name
    /// keys in `workflow.responses` from which the client reads its server
    /// URL, the match id, its per-player token, and the self-signed cert
    /// hash (hex-encoded SHA-256 of the cert DER).
    pub fn fps_canvas(
        mut self,
        server_url_key: impl Into<String>,
        match_id_key: impl Into<String>,
        token_key: impl Into<String>,
        cert_hash_key: impl Into<String>,
    ) -> Self {
        self.kind = Some(DisplayType::FpsCanvas {
            server_url_key: server_url_key.into(),
            match_id_key: match_id_key.into(),
            token_key: token_key.into(),
            cert_hash_key: cert_hash_key.into(),
        });
        self
    }

    pub fn link(mut self, href: impl Into<String>, content: Vec<WorkflowDisplay>) -> Self {
        self.kind = Some(DisplayType::Link {
            href: href.into(),
            content,
        });
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::Text { text: text.into() });
        self
    }

    pub fn lucide_icon(mut self, name: impl Into<String>, aria_label: Option<&str>) -> Self {
        let mut icon = IconBuilder::lucide(name);
        icon.aria_label = aria_label.map(|a| a.into());
        self.kind = Some(icon.into_display_type());
        self
    }

    pub fn icon(
        mut self,
        lib: IconLibrary,
        name: impl Into<String>,
        aria_label: Option<String>,
        url: Option<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Icon {
            lib,
            name: name.into(),
            aria_label,
            url,
        });
        self
    }

    pub fn card(mut self, content: Vec<WorkflowDisplay>, layout: CardLayout) -> Self {
        self.kind = Some(DisplayType::Card { content, layout });
        self
    }

    pub fn video(mut self, src: impl Into<String>, poster_src: Option<String>) -> Self {
        self.kind = Some(DisplayType::Video {
            src: src.into(),
            poster_src,
        });
        self
    }

    pub fn audio_clip(
        mut self,
        src: impl Into<String>,
        seconds: impl Into<String>,
        marks: Vec<f64>,
    ) -> Self {
        self.kind = Some(DisplayType::AudioClip {
            src: src.into(),
            seconds: seconds.into(),
            marks,
        });
        self
    }

    pub fn dots_board(mut self, board: impl Into<String>, action: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::DotsBoard {
            board: board.into(),
            action: action.into(),
        });
        self
    }

    pub fn word_wheel(mut self, puzzle: impl Into<String>, action: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::WordWheel {
            puzzle: puzzle.into(),
            action: action.into(),
        });
        self
    }

    pub fn snake_run(mut self, level: impl Into<String>, action: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::SnakeRun {
            level: level.into(),
            action: action.into(),
        });
        self
    }

    pub fn share_result(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::ShareResult {
            value: value.into(),
            label: label.into(),
        });
        self
    }

    pub fn railyard_board(mut self, board: impl Into<String>, action: impl Into<String>, save_action: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::RailyardBoard {
            board: board.into(),
            action: action.into(),
            save_action: save_action.into(),
        });
        self
    }

    pub fn image(mut self, src: impl Into<String>, alt_text: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::Image {
            src: src.into(),
            alt_text: alt_text.into(),
        });
        self
    }

    pub fn list(
        mut self,
        items_key: impl Into<String>,
        r#as: impl Into<String>,
        content: Vec<WorkflowDisplay>,
    ) -> Self {
        self.kind = Some(DisplayType::List {
            items_key: items_key.into(),
            r#as: r#as.into(),
            content,
        });
        self
    }

    pub fn div(mut self, content: Vec<WorkflowDisplay>) -> Self {
        self.kind = Some(DisplayType::Div { content });
        self
    }

    pub fn carousel(
        mut self,
        items_key: impl Into<String>,
        r#as: impl Into<String>,
        content: Vec<WorkflowDisplay>,
    ) -> Self {
        self.kind = Some(DisplayType::Carousel {
            items_key: items_key.into(),
            r#as: r#as.into(),
            content,
            on_select_action: None,
            input_map: None,
            active_key: None,
        });
        self
    }

    /// A carousel whose swipe selection runs a workflow action.
    pub fn carousel_select(
        mut self,
        items_key: impl Into<String>,
        r#as: impl Into<String>,
        content: Vec<WorkflowDisplay>,
        action_id: impl Into<String>,
        input_map: Option<HashMap<String, Value>>,
        active_key: impl Into<String>,
    ) -> Self {
        self.kind = Some(DisplayType::Carousel {
            items_key: items_key.into(),
            r#as: r#as.into(),
            content,
            on_select_action: Some(action_id.into()),
            input_map,
            active_key: Some(active_key.into()),
        });
        self
    }

    pub fn table(
        mut self,
        items_key: impl Into<String>,
        r#as: impl Into<String>,
        headers: Vec<TableHeader>,
        row: Vec<WorkflowDisplay>,
    ) -> Self {
        self.kind = Some(DisplayType::Table {
            items_key: items_key.into(),
            r#as: r#as.into(),
            headers,
            row,
        });
        self
    }

    pub fn wysiyg(mut self, content_key: impl Into<String>) -> Self {
        self.kind = Some(DisplayType::Wysiwyg {
            content_key: content_key.into(),
        });
        self
    }
}

//
// ────────────────────────────── INPUT BUILDERS ─────────────────────────────
//

#[derive(Default)]
pub struct WorkflowInputBuilder {
    readonly: Option<NodeCondition>,
    r#type: Option<crate::InputType>,
    default_value: Option<Value>,
    required: bool,
    on_change_action: Option<(String, HashMap<String, String>)>,
}

impl WorkflowInputBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn readonly(mut self, cond: NodeCondition) -> Self {
        self.readonly = Some(cond);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn default_value(mut self, value: Value) -> Self {
        self.default_value = Some(value);
        self
    }

    pub fn on_change_action(
        mut self,
        action_id: impl Into<String>,
        map: HashMap<String, String>,
    ) -> Self {
        self.on_change_action = Some((action_id.into(), map));
        self
    }

    pub fn r#type(mut self, t: crate::InputType) -> Self {
        self.r#type = Some(t);
        self
    }

    pub fn build(self) -> WorkflowInput {
        WorkflowInput {
            readonly: self.readonly,
            r#type: self.r#type.expect("input type must be set"),
            default_value: self.default_value,
            required: self.required,
            on_change_action: self.on_change_action,
        }
    }
}

pub struct InputTypeBuilder; // static namespace of ctors

impl InputTypeBuilder {
    pub fn text(placeholder: Option<String>) -> crate::InputType {
        crate::InputType::Text { placeholder }
    }

    pub fn wysiwyg() -> crate::InputType {
        crate::InputType::Wysiwyg
    }

    pub fn select(
        filter: Option<NodeCondition>,
        items_key: impl Into<String>,
        r#as: impl Into<String>,
        value: impl Into<String>,
        content: Vec<WorkflowDisplay>,
        selected_content: Option<Vec<WorkflowDisplay>>,
        unselected_content: Option<Vec<WorkflowDisplay>>,
        class: impl Into<String>,
    ) -> crate::InputType {
        crate::InputType::Select {
            filter,
            items_key: items_key.into(),
            r#as: r#as.into(),
            value: value.into(),
            content,
            selected_content,
            unselected_content,
            class: class.into(),
        }
    }

    pub fn switch() -> crate::InputType {
        crate::InputType::Switch
    }

    pub fn checkbox(label: impl Into<String>) -> crate::InputType {
        crate::InputType::Checkbox {
            label: label.into(),
        }
    }

    pub fn number(min: Option<u32>, max: Option<u32>) -> crate::InputType {
        crate::InputType::Number { min, max }
    }

    pub fn text_area(placeholder: Option<String>, rows: Option<u32>) -> crate::InputType {
        crate::InputType::TextArea { placeholder, rows }
    }

    pub fn list(
        from_responses_id: impl Into<String>,
        item_key: impl Into<String>,
        display_field: impl Into<String>,
    ) -> crate::InputType {
        crate::InputType::List {
            from_responses_id: from_responses_id.into(),
            item_key: item_key.into(),
            display_field: display_field.into(),
        }
    }

    pub fn server_action_loader(target: impl Into<String>) -> crate::InputType {
        crate::InputType::ServerActionLoader {
            target: target.into(),
        }
    }

    pub fn display(content: impl Into<String>) -> crate::InputType {
        crate::InputType::Display {
            content: content.into(),
        }
    }

    pub fn multi_select_from_list(
        items_key: impl Into<String>,
        r#as: impl Into<String>,
        value: impl Into<String>,
        content: Vec<WorkflowDisplay>,
        class: impl Into<String>,
        min: Option<u32>,
        max: Option<u32>,
    ) -> crate::InputType {
        crate::InputType::MultiSelectFromList {
            items_key: items_key.into(),
            r#as: r#as.into(),
            value: value.into(),
            content,
            class: class.into(),
            min,
            max,
        }
    }

    pub fn select_from_list(
        items_key: impl Into<String>,
        r#as: impl Into<String>,
        value: impl Into<String>,
        class: impl Into<String>,
        selected_class: impl Into<String>,
        content: Vec<WorkflowDisplay>,
    ) -> crate::InputType {
        crate::InputType::SelectFromList {
            items_key: items_key.into(),
            r#as: r#as.into(),
            value: value.into(),
            class: class.into(),
            selected_class: selected_class.into(),
            content,
        }
    }
}

//
// ────────────────────────────── ACTION BUILDER ─────────────────────────────
//

#[derive(Default)]
pub struct WorkflowActionBuilder {
    id: String,
    progress_node: bool,
    action_type: Option<ActionType>,
    target: Option<String>,
    allow_while_loading: bool,
}

impl WorkflowActionBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            progress_node: true,
            ..Default::default()
        }
    }

    pub fn progress_node(mut self, progress: bool) -> Self {
        self.progress_node = progress;
        self
    }

    pub fn transform_then_run(
        mut self,
        ops: Vec<TransformOp>,
        reconcile_node: bool,
        server_action_id: impl Into<String>,
        input_map: Option<HashMap<String, String>>,
    ) -> Self {
        self.action_type = Some(ActionType::TransformThenRun {
            input_map,
            server_action_id: server_action_id.into(),
            ops,
            reconcile_node,
        });
        self
    }

    pub fn transform_responses(mut self, ops: Vec<TransformOp>, reconcile_node: bool) -> Self {
        self.action_type = Some(ActionType::TransformResponses {
            ops,
            reconcile_node,
        });
        self
    }

    pub fn next_node(mut self) -> Self {
        self.action_type = Some(ActionType::NextNode);
        self
    }

    pub fn previous_node(mut self) -> Self {
        self.action_type = Some(ActionType::PreviousNode);
        self
    }

    pub fn submit(mut self) -> Self {
        self.action_type = Some(ActionType::Submit);
        self
    }

    pub fn noop(mut self) -> Self {
        self.action_type = Some(ActionType::Noop);
        self
    }

    pub fn cancel(mut self) -> Self {
        self.action_type = Some(ActionType::Cancel);
        self
    }

    pub fn run_server_action(mut self) -> Self {
        self.action_type = Some(ActionType::RunServerAction);
        self
    }

    pub fn start_injected_workflow(
        mut self,
        input_map: Option<HashMap<String, String>>,
        context: HashMap<String, Value>,
        responses: HashMap<String, Value>,
        complete_workflow: bool,
        inject_place: InjectWorkflowPlace,
        on_complete_ops: Vec<TransformOp>,
    ) -> Self {
        self.action_type = Some(ActionType::StartWorkflow {
            input_map,
            complete_workflow,
            responses,
            context,
            inject_place: Some(inject_place),
            on_complete_ops,
        });
        self
    }

    pub fn start_workflow(
        mut self,
        input_map: Option<HashMap<String, String>>,
        context: HashMap<String, Value>,
        responses: HashMap<String, Value>,
        complete_workflow: bool,
    ) -> Self {
        self.action_type = Some(ActionType::StartWorkflow {
            input_map,
            complete_workflow,
            responses,
            context,
            inject_place: None,
            on_complete_ops: vec![],
        });
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn allow_while_loading(mut self) -> Self {
        self.allow_while_loading = true;
        self
    }

    pub fn build(self) -> WorkflowAction {
        WorkflowAction {
            progress_node: self.progress_node,
            id: self.id,
            action_type: self.action_type.expect("action_type must be set"),
            target: self.target,
            allow_while_loading: self.allow_while_loading,
        }
    }
}

//
// ────────────────────────────── CONDITION HELPERS ──────────────────────────
//

pub struct Cond;

impl Cond {
    pub fn gte(field: impl Into<String>, value: serde_json::Value) -> NodeCondition {
        NodeCondition::Gte {
            field: field.into(),
            value,
        }
    }
    pub fn always() -> NodeCondition {
        NodeCondition::Always
    }
    pub fn not(inner: NodeCondition) -> NodeCondition {
        NodeCondition::Not(Box::new(inner))
    }
    pub fn and(list: Vec<NodeCondition>) -> NodeCondition {
        NodeCondition::And(list)
    }

    pub fn or(list: impl IntoIterator<Item = NodeCondition>) -> NodeCondition {
        NodeCondition::Or(list.into_iter().collect())
    }

    pub fn some_in_list(
        list: impl Into<String>,
        r#as: impl Into<String>,
        condition: NodeCondition,
    ) -> NodeCondition {
        NodeCondition::SomeInList {
            list: list.into(),
            r#as: r#as.into(),
            condition: Box::new(condition),
        }
    }
    pub fn response_exists(field: impl Into<String>) -> NodeCondition {
        NodeCondition::ResponseExists(field.into())
    }
    pub fn response_equals(field: impl Into<String>, value: serde_json::Value) -> NodeCondition {
        NodeCondition::ResponseEquals {
            field: field.into(),
            value,
        }
    }
    pub fn response_list_empty(field: impl Into<String>) -> NodeCondition {
        NodeCondition::ResponseListEmpty(field.into())
    }
    pub fn response_list_not_empty(field: impl Into<String>) -> NodeCondition {
        NodeCondition::Not(Box::new(NodeCondition::ResponseListEmpty(field.into())))
    }
}

//
// ────────────────────────────── SMALL UTILITIES ────────────────────────────
//

pub fn table_header(
    label: impl Into<String>,
    key: impl Into<String>,
    width: Option<String>,
    align: Option<String>,
) -> TableHeader {
    TableHeader {
        label: label.into(),
        key: key.into(),
        width,
        align,
    }
}

pub struct IconBuilder {
    lib: IconLibrary,
    name: String,
    aria_label: Option<String>,
    url: Option<String>,
}

impl IconBuilder {
    /// Generic constructor (use presets where possible)
    pub fn new(lib: IconLibrary, name: impl Into<String>) -> Self {
        Self {
            lib,
            name: name.into(),
            aria_label: None,
            url: None,
        }
    }

    /// Lucide preset with default size and stroke
    pub fn lucide(name: impl Into<String>) -> Self {
        Self::new(IconLibrary::Lucide, name)
    }

    /// URL-backed icon (e.g., remote PNG/SVG)
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            lib: IconLibrary::Url,
            name: String::new(),
            aria_label: None,
            url: Some(url.into()),
        }
    }

    // ── Fluent setters ─────────────────────────────────────────────────────
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn aria_label(mut self, aria: impl Into<String>) -> Self {
        self.aria_label = Some(aria.into());
        self
    }

    // ── Conversions ────────────────────────────────────────────────────────
    pub fn into_display_type(self) -> DisplayType {
        DisplayType::Icon {
            lib: self.lib,
            name: self.name,
            aria_label: self.aria_label,
            url: self.url,
        }
    }

    pub fn into_workflow_display(self) -> WorkflowDisplay {
        WorkflowDisplay {
            class: String::new(), // keep wrapper class empty by default
            r#type: self.into_display_type(),
        }
    }

    /// If you want to set a wrapper class on the WorkflowDisplay directly
    pub fn into_workflow_display_with_class(self, class: impl Into<String>) -> WorkflowDisplay {
        WorkflowDisplay {
            class: class.into(),
            r#type: self.into_display_type(),
        }
    }
}

//
// ────────────────────────────── COMMAND BUILDERS ───────────────────────────
//

#[derive(Default)]
pub struct CommandPaletteBuilder {
    placeholder: Option<String>,
    groups: Vec<crate::CommandGroup>,
    allow_search: bool,
}

impl CommandPaletteBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_search(mut self, allow: bool) -> Self {
        self.allow_search = allow;
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn add_group(mut self, group: crate::CommandGroup) -> Self {
        self.groups.push(group);
        self
    }

    pub fn groups(mut self, groups: Vec<crate::CommandGroup>) -> Self {
        self.groups = groups;
        self
    }

    pub fn build(self) -> crate::CommandPaletteConfig {
        crate::CommandPaletteConfig {
            placeholder: self.placeholder,
            groups: self.groups,
        }
    }
}

#[derive(Default)]
pub struct CommandGroupBuilder {
    heading: Option<String>,
    items: Vec<crate::CommandItem>,
}

impl CommandGroupBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn heading(mut self, text: impl Into<String>) -> Self {
        self.heading = Some(text.into());
        self
    }

    pub fn add_item(mut self, item: crate::CommandItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<crate::CommandItem>) -> Self {
        self.items = items;
        self
    }

    pub fn build(self) -> crate::CommandGroup {
        crate::CommandGroup {
            heading: self.heading,
            items: self.items,
        }
    }
}

pub struct CommandItemBuilder {
    id: String,
    label: String,
    keywords: Vec<String>,
    icon: Option<String>,
    shortcut: Option<String>,
    action: Option<(String, Option<HashMap<String, String>>)>,
    data: HashMap<String, Value>,
}

impl CommandItemBuilder {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            keywords: Vec::new(),
            icon: None,
            shortcut: None,
            action: None,
            data: HashMap::new(),
        }
    }

    pub fn keyword(mut self, kw: impl Into<String>) -> Self {
        self.keywords.push(kw.into());
        self
    }

    pub fn keywords(mut self, kws: Vec<String>) -> Self {
        self.keywords = kws;
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn action(
        mut self,
        action_id: impl Into<String>,
        input_map: Option<HashMap<String, String>>,
    ) -> Self {
        self.action = Some((action_id.into(), input_map));
        self
    }

    pub fn build(self) -> crate::CommandItem {
        let action = self.action.expect("CommandItem must have an action");
        crate::CommandItem {
            id: self.id,
            label: self.label,
            keywords: self.keywords,
            icon: self.icon,
            shortcut: self.shortcut,
            action_id: action.0,
            action_input_map: action.1,
            data: self.data,
        }
    }

    pub fn with_data(mut self, key: &str, val: impl Into<Value>) -> Self {
        self.data.insert(key.into(), val.into());
        self
    }
}
