//! The building blocks every page is made of. Each one is styled by tawks
//! itself, the same way everywhere, so a workflow says what something is (a
//! heading, a panel, a primary link, the viewer's own row) and never how it
//! looks.
//!
//! ```ignore
//! use workflow_model::ui::*;
//!
//! WorkflowVisualBuilder::new("page").page(PageWidth::Normal, vec![
//!     hero("A hockey classic, back online.")
//!         .lead("Your hockey game. Your friends. Your place on the board.")
//!         .actions(vec![link_primary("/download", "Get the app")])
//!         .build(),
//!     section("What we've improved").content(vec![
//!         grid(2, vec![feature("Current rosters", "All 32 teams.")]),
//!     ]).build(),
//! ]);
//! ```
//!
//! Fields that take a tone or a state also take a `{key}` template, so a bot
//! can send `"Selected"` or `"Success"` in its responses and the page picks it
//! up: `row("{s.name}").state_from("{s.state}")`.

use std::collections::HashMap;

use serde_json::Value;

pub use crate::{
    Accent, Act, ActionLook, Align, GroupButton, Aspect, Backdrop, CaptionPlacement, Fit, FlexDirection, Gap, IconSize, Justify, Layout, LinkStyle,
    Corner, PageWidth, Shape, Size, State, Tab, TabStyle, TextAlign, TextRole, TextStyle, Tone, Typeface,
};
use crate::{ChartConfig, ChartType, DisplayType, IconLibrary, NodeCondition, WorkflowDisplay};

fn display(r#type: DisplayType) -> WorkflowDisplay {
    WorkflowDisplay { r#type, mark: None }
}

pub fn text(role: TextRole, text: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Typography { text: text.into(), role, tone: String::new(), style: TextStyle::default() })
}

pub fn display_title(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Display, s)
}

pub fn title(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Title, s)
}

pub fn heading(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Heading, s)
}

pub fn subheading(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Subheading, s)
}

pub fn headline(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Headline, s)
}

pub fn lead(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Lead, s)
}

pub fn body(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Body, s)
}

pub fn fine(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Fine, s)
}

pub fn eyebrow(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Eyebrow, s)
}

pub fn code(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Code, s)
}

pub fn stat_text(s: impl Into<String>) -> WorkflowDisplay {
    text(TextRole::Stat, s)
}

fn stack(direction: FlexDirection, gap: Gap, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Stack { direction, gap, align: Align::Stretch, justify: Justify::Start, wrap: false, content })
}

pub fn column(gap: Gap, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    stack(FlexDirection::Col, gap, content)
}

/// A row that wraps when it runs out of room, its children centred on one
/// line, which is what nearly every row of buttons, chips or stats wants.
pub fn row(gap: Gap, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    stack(FlexDirection::Row, gap, content).align(Align::Center).wrap()
}

pub fn grid(columns: u8, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Grid { columns, gap: Gap::Loose, content, fixed: false })
}

pub fn panel(content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Panel { tone: String::new(), content })
}

pub fn feature(title: impl Into<String>, body: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Feature { icon: None, title: title.into(), body: body.into(), href: None })
}

pub fn chip(text: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Chip { icon: None, text: text.into(), act: None, state: None })
}

pub fn icon(name: impl Into<String>, size: IconSize) -> WorkflowDisplay {
    display(DisplayType::Icon {
        lib: IconLibrary::Lucide,
        name: name.into(),
        aria_label: None,
        url: None,
        size,
        tone: String::new(),
    })
}

pub fn link(href: impl Into<String>, style: LinkStyle, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Link { href: href.into(), content, style, thumb: None })
}

pub fn link_primary(href: impl Into<String>, label: impl Into<String>) -> WorkflowDisplay {
    link(href, LinkStyle::Primary, vec![body(label)])
}

/// "Play now" for a first-party game: starts it in the desktop app when it is
/// installed there, and opens `fallback_url` everywhere else.
pub fn play_game(game: impl Into<String>, label: impl Into<String>, fallback_url: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::PlayGame { game: game.into(), label: label.into(), fallback_url: fallback_url.into() })
}

pub fn link_secondary(href: impl Into<String>, label: impl Into<String>) -> WorkflowDisplay {
    link(href, LinkStyle::Secondary, vec![body(label)])
}

pub fn link_inline(href: impl Into<String>, label: impl Into<String>) -> WorkflowDisplay {
    link(href, LinkStyle::Inline, vec![body(label)])
}

pub fn image(src: impl Into<String>, alt: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Image { src: src.into(), alt_text: alt.into() })
}

/// A small label in `tone`, which may be a `{key}`; hidden while its text is empty.
pub fn badge(text: impl Into<String>, tone: Tone) -> WorkflowDisplay {
    display(DisplayType::Badge { variant: String::new(), text: text.into(), tone: tone.into(), icon: None })
}

/// A bar filled to the percentage at `key`.
pub fn progress(key: impl Into<String>, tone: Tone) -> WorkflowDisplay {
    display(DisplayType::Progress { progress_percentage_key: key.into(), tone: tone.into() })
}

/// A countdown to the time at `until`, showing `text` with the time left as `{as}`.
pub fn countdown(until: impl Into<String>, r#as: impl Into<String>, text: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Countdown {
        until: until.into(),
        r#as: r#as.into(),
        text: text.into(),
        shake_under_ms_left: None,
        role: TextRole::Body,
        tone: String::new(),
    })
}

/// The time at `key`, written the way `opts` says.
pub fn time(key: impl Into<String>, opts: crate::TimeDisplayOpts) -> WorkflowDisplay {
    display(DisplayType::Time { key: key.into(), opts, role: TextRole::Fine, tone: String::new() })
}

/// A running total at `key` that pops up as it grows, with `icon` beside it.
pub fn score_ticker(key: impl Into<String>, icon: WorkflowDisplay) -> WorkflowDisplay {
    display(DisplayType::ScoreTicker { value_key: key.into(), icon: Box::new(icon) })
}

/// Floats `content` over the game canvas just before it.
pub fn overlay(corner: Corner, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Overlay { corner, content })
}

/// Whether this client's renderer is new enough for `DisplayType::Discussion`
/// and `ButtonGroup`. Desktop builds bundle their renderer and stay behind the
/// site until they update; only builds from `$client.renderer` on say so.
fn renderer_is_current() -> NodeCondition {
    NodeCondition::Or(vec![
        NodeCondition::ResponseEquals { field: "$client.desktop".into(), value: serde_json::json!(false) },
        NodeCondition::ResponseExists("$client.renderer".into()),
    ])
}

/// Shows `content` to every client but desktop builds too old to draw it,
/// which would put an "is drawn by tawks itself" box in its place.
pub fn on_current_renderer(content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    when(renderer_is_current(), content)
}

/// What those older desktop builds show instead.
pub fn on_older_renderer(content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    when(NodeCondition::Not(Box::new(renderer_is_current())), content)
}

/// A button labelled `label` that opens `content` over the page as a modal,
/// titled `title`. `.drawer()` and `.closes_on_submit()` change how it behaves.
pub fn dialog(label: impl Into<String>, title: impl Into<String>, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Dialog {
        label: label.into(),
        icon: None,
        look: crate::DialogButton::Primary,
        title: title.into(),
        description: None,
        style: crate::DialogStyle::Modal,
        close_on_submit: false,
        content,
    })
}

/// Whether this client's renderer is at least version `renderer`. Builds from
/// before `$client.renderer` existed never are.
pub fn renderer_at_least(renderer: u32) -> NodeCondition {
    NodeCondition::Gte { field: "$client.renderer".into(), value: serde_json::json!(renderer) }
}

/// Reactions and a comments button for the chat message whose chat and id
/// `chat` and `message` resolve to; `reactions_key` and `comments_key` name
/// what the page last knew. See `DisplayType::Discussion`.
pub fn discussion(
    chat: impl Into<String>,
    message: impl Into<String>,
    reactions_key: impl Into<String>,
    comments_key: impl Into<String>,
) -> WorkflowDisplay {
    display(DisplayType::Discussion {
        chat: chat.into(),
        message: message.into(),
        reactions: reactions_key.into(),
        comments: comments_key.into(),
        start: None,
        suggested: None,
        inline: false,
        title: None,
    })
}

/// A whole panel that opens `href`.
pub fn link_card(href: impl Into<String>, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    link(href, LinkStyle::Card, content)
}

/// An entry in a list that opens `href`; see `LinkStyle::Entry`.
pub fn link_entry(href: impl Into<String>, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    link(href, LinkStyle::Entry, content)
}

/// Shows `content` while `condition` holds, adding nothing around it.
pub fn when(condition: NodeCondition, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Condition { condition, content })
}

/// What pressing something does.
pub fn act(action: impl Into<String>) -> Act {
    Act { action: action.into(), inputs: None, disabled: None }
}

impl Act {
    pub fn input(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.inputs.get_or_insert_with(HashMap::new).insert(key.into(), value.into());
        self
    }

    pub fn inputs(mut self, inputs: HashMap<String, Value>) -> Self {
        self.inputs = Some(inputs);
        self
    }

    pub fn disabled_when(mut self, condition: NodeCondition) -> Self {
        self.disabled = Some(condition);
        self
    }
}

/// A button labelled `label` that runs `act`.
pub fn action(label: impl Into<String>, look: ActionLook, act: Act) -> WorkflowDisplay {
    display(DisplayType::Action {
        label: label.into(),
        icon: None,
        look,
        act,
        full_width: false,
        submit: false,
        scroll_to: None,
    })
}

pub fn primary_action(label: impl Into<String>, act: Act) -> WorkflowDisplay {
    action(label, ActionLook::Primary, act)
}

pub fn secondary_action(label: impl Into<String>, act: Act) -> WorkflowDisplay {
    action(label, ActionLook::Secondary, act)
}

/// A round button showing only `icon`; `label` is read out to screen readers.
pub fn icon_action(icon: impl Into<String>, label: impl Into<String>, act: Act) -> WorkflowDisplay {
    action(label, ActionLook::Icon, act).with_icon(icon)
}

/// A compact segmented strip of `buttons`, such as 1, 3 or 5 tickets.
pub fn button_group(buttons: Vec<GroupButton>) -> WorkflowDisplay {
    display(DisplayType::ButtonGroup { buttons })
}

pub fn group_button(label: impl Into<String>, act: Act) -> GroupButton {
    GroupButton { label: label.into(), act }
}

pub fn notice(tone: Tone, body: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Notice { tone: tone.into(), icon: None, title: None, body: body.into(), actions: vec![] })
}

pub fn tabs(style: TabStyle, tabs: Vec<Tab>) -> WorkflowDisplay {
    display(DisplayType::TabBar { style, tabs })
}

/// A tab that runs `act` and shows as current while `active` holds.
pub fn tab(label: impl Into<String>, active: NodeCondition, act: Act) -> Tab {
    Tab { label: label.into(), icon: None, active, act: Some(act), href: None }
}

/// A tab that opens `href`.
pub fn tab_link(label: impl Into<String>, active: NodeCondition, href: impl Into<String>) -> Tab {
    Tab { label: label.into(), icon: None, active, act: None, href: Some(href.into()) }
}

impl Tab {
    pub fn icon(mut self, name: impl Into<String>) -> Self {
        self.icon = Some(name.into());
        self
    }
}

pub fn stepper(title: impl Into<String>, previous: Option<Act>, next: Option<Act>) -> WorkflowDisplay {
    display(DisplayType::Stepper { title: title.into(), subtitle: None, previous, next })
}

/// One line of a list. Add the rank or avatar with `.lead`, the value with
/// `.trail` and the meta line with `.meta`.
pub fn list_row(title: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Row {
        lead: vec![],
        title: title.into(),
        meta: None,
        trail: vec![],
        state: String::new(),
        href: None,
        act: None,
        accent: None,
    })
}

pub fn choice(content: Vec<WorkflowDisplay>, act: Act) -> WorkflowDisplay {
    display(DisplayType::Choice { content, state: String::new(), act, direction: FlexDirection::Col })
}

pub fn stat(value: impl Into<String>, label: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Stat { value: value.into(), label: label.into(), tone: String::new() })
}

pub fn meter(percent: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Meter { label: None, value: None, percent: percent.into(), tone: String::new() })
}

/// A line, bar or area chart of the rows at `items_key`. `x` and `y` name fields
/// of each row; `config` holds labels, formats and further series.
pub fn chart(
    r#type: ChartType,
    items_key: impl Into<String>,
    x: impl Into<String>,
    y: impl Into<String>,
    config: ChartConfig,
) -> WorkflowDisplay {
    display(DisplayType::Chart {
        r#type,
        chart_data_key: items_key.into(),
        config,
        x: x.into(),
        y: y.into(),
        axis: None,
    })
}

/// One segment per item of `items_key`, each coloured by the `state` template.
pub fn segments(items_key: impl Into<String>, r#as: impl Into<String>, state: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Segments { items_key: items_key.into(), r#as: r#as.into(), state: state.into(), label: None })
}

pub fn empty(title: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Empty { icon: None, title: title.into(), body: None, loading: false, actions: vec![] })
}

/// The pulsing line a page shows while its data loads.
pub fn loading(title: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Empty { icon: None, title: title.into(), body: None, loading: true, actions: vec![] })
}

/// `avatar` is an image URL template; with none, the name's first letter stands in.
pub fn person(avatar: impl Into<String>, name: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Person { avatar: avatar.into(), name: name.into(), meta: None, size: Size::Medium, user: None })
}

pub fn avatar_stack(
    items_key: impl Into<String>,
    r#as: impl Into<String>,
    avatar: impl Into<String>,
    name: impl Into<String>,
) -> WorkflowDisplay {
    display(DisplayType::AvatarStack {
        items_key: items_key.into(),
        r#as: r#as.into(),
        avatar: avatar.into(),
        name: name.into(),
        max: 5,
    })
}

pub fn divider() -> WorkflowDisplay {
    display(DisplayType::Divider { label: None })
}

pub fn labelled_divider(label: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Divider { label: Some(label.into()) })
}

pub fn log(text: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Log { text: text.into() })
}

pub fn split(media: Vec<WorkflowDisplay>, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Split { media, content, rail: None })
}

/// `content` with `rail` in a side column after it, on screens wide enough for both.
pub fn rail(content: Vec<WorkflowDisplay>, rail: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Split { media: rail, content, rail: Some(true) })
}

pub fn action_bar(content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::ActionBar { content })
}

pub fn media(src: impl Into<String>, alt: impl Into<String>, aspect: Aspect, size: Size) -> WorkflowDisplay {
    display(DisplayType::Media {
        src: src.into(),
        alt: alt.into(),
        aspect,
        size,
        shape: Shape::Rounded,
        fit: Fit::Cover,
        badge: vec![],
        caption: vec![],
        tint: String::new(),
        caption_placement: CaptionPlacement::Over,
    })
}

/// A logo or avatar: square, `size`, cut round.
pub fn avatar(src: impl Into<String>, alt: impl Into<String>, size: Size) -> WorkflowDisplay {
    media(src, alt, Aspect::Square, size).shape(Shape::Round)
}

/// A team or brand logo: square, whole, not cropped.
pub fn logo(src: impl Into<String>, alt: impl Into<String>, size: Size) -> WorkflowDisplay {
    media(src, alt, Aspect::Square, size).shape(Shape::Square).fit(Fit::Contain)
}

pub fn backdrop(color: impl Into<String>, image: impl Into<String>, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    display(DisplayType::Backdrop {
        color: color.into(),
        image: image.into(),
        content,
        photo: String::new(),
        credit: String::new(),
    })
}

/// A backdrop that turns into a wide photo band, credited, when `photo`
/// interpolates to a picture; otherwise the plain band.
pub fn photo_backdrop(
    color: impl Into<String>,
    image: impl Into<String>,
    photo: impl Into<String>,
    credit: impl Into<String>,
    content: Vec<WorkflowDisplay>,
) -> WorkflowDisplay {
    display(DisplayType::Backdrop {
        color: color.into(),
        image: image.into(),
        content,
        photo: photo.into(),
        credit: credit.into(),
    })
}

pub fn calendar(items_key: impl Into<String>) -> WorkflowDisplay {
    display(DisplayType::Calendar {
        items_key: items_key.into(),
        accent: None,
        legend_key: String::new(),
    })
}

pub fn items(
    items_key: impl Into<String>,
    r#as: impl Into<String>,
    layout: Layout,
    content: Vec<WorkflowDisplay>,
) -> WorkflowDisplay {
    display(DisplayType::Items { items_key: items_key.into(), r#as: r#as.into(), layout, content })
}

pub fn hero(title: impl Into<String>) -> HeroBuilder {
    HeroBuilder { eyebrow: None, title: title.into(), lead: None, aside: vec![], actions: vec![] }
}

pub struct HeroBuilder {
    eyebrow: Option<String>,
    title: String,
    lead: Option<String>,
    aside: Vec<WorkflowDisplay>,
    actions: Vec<WorkflowDisplay>,
}

impl HeroBuilder {
    pub fn eyebrow(mut self, s: impl Into<String>) -> Self {
        self.eyebrow = Some(s.into());
        self
    }

    pub fn lead(mut self, s: impl Into<String>) -> Self {
        self.lead = Some(s.into());
        self
    }

    pub fn aside(mut self, content: Vec<WorkflowDisplay>) -> Self {
        self.aside = content;
        self
    }

    pub fn actions(mut self, content: Vec<WorkflowDisplay>) -> Self {
        self.actions = content;
        self
    }

    pub fn build(self) -> WorkflowDisplay {
        display(DisplayType::Hero {
            eyebrow: self.eyebrow,
            title: self.title,
            lead: self.lead,
            aside: self.aside,
            actions: self.actions,
        })
    }
}

pub fn section(title: impl Into<String>) -> SectionBuilder {
    SectionBuilder { eyebrow: None, title: title.into(), lead: None, aside: vec![], content: vec![] }
}

pub struct SectionBuilder {
    eyebrow: Option<String>,
    title: String,
    lead: Option<String>,
    aside: Vec<WorkflowDisplay>,
    content: Vec<WorkflowDisplay>,
}

impl SectionBuilder {
    pub fn eyebrow(mut self, s: impl Into<String>) -> Self {
        self.eyebrow = Some(s.into());
        self
    }

    pub fn lead(mut self, s: impl Into<String>) -> Self {
        self.lead = Some(s.into());
        self
    }

    pub fn aside(mut self, content: Vec<WorkflowDisplay>) -> Self {
        self.aside = content;
        self
    }

    pub fn content(mut self, content: Vec<WorkflowDisplay>) -> Self {
        self.content = content;
        self
    }

    pub fn build(self) -> WorkflowDisplay {
        display(DisplayType::Section {
            eyebrow: self.eyebrow,
            title: self.title,
            lead: self.lead,
            aside: self.aside,
            content: self.content,
        })
    }
}

/// Adjustments to a building block after it is made. Each applies only to
/// the blocks it names and leaves any other display unchanged, so they chain
/// freely.
impl WorkflowDisplay {
    /// The colour of text, icons, panels, notices, stats, meters, badges, progress,
    /// countdowns and times, or the wash over a picture.
    pub fn tone(self, value: Tone) -> Self {
        self.tone_from(value.as_str())
    }

    /// The tone from response data, such as `"{g.tone}"`, naming a `Tone`.
    pub fn tone_from(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        match &mut self.r#type {
            DisplayType::Typography { tone, .. }
            | DisplayType::Icon { tone, .. }
            | DisplayType::Panel { tone, .. }
            | DisplayType::Notice { tone, .. }
            | DisplayType::Stat { tone, .. }
            | DisplayType::Meter { tone, .. }
            | DisplayType::Badge { tone, .. }
            | DisplayType::Progress { tone, .. }
            | DisplayType::Countdown { tone, .. }
            | DisplayType::Time { tone, .. } => *tone = value,
            DisplayType::Media { tint, .. } => *tint = value,
            _ => {}
        }
        self
    }

    /// How a row, a choice or a chip stands against the rest.
    pub fn state(self, value: State) -> Self {
        self.state_from(value.as_str())
    }

    /// The state from response data, such as `"{s.state}"`, naming a `State`.
    pub fn state_from(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        match &mut self.r#type {
            DisplayType::Row { state, .. } | DisplayType::Choice { state, .. } => *state = value,
            DisplayType::Chip { state, .. } => *state = Some(value),
            _ => {}
        }
        self
    }

    /// Washes a row, each row of a table, or a calendar's today in a colour:
    /// `#rrggbb` or a `{key}` naming one per item, such as `"{r.accent}"`. A value that is
    /// neither is left out.
    pub fn accent_from(mut self, value: impl Into<String>) -> Self {
        let accent = match Accent::parse(value) {
            Ok(accent) => accent,
            Err(error) => {
                tracing::warn!("{error}");
                return self;
            }
        };
        match &mut self.r#type {
            DisplayType::Row { accent: slot, .. }
            | DisplayType::Table { row_accent: slot, .. }
            | DisplayType::Calendar { accent: slot, .. } => *slot = Some(accent),
            _ => {}
        }
        self
    }

    /// Draws the items of `key` as a calendar's legend under the month.
    pub fn legend(mut self, key: impl Into<String>) -> Self {
        if let DisplayType::Calendar { legend_key, .. } = &mut self.r#type {
            *legend_key = key.into();
        }
        self
    }

    /// Splits a table into sections inside one grid, so columns line up
    /// across them: items whose `key` (such as `"{r.section}"`) fills in draw
    /// `children` once across every column instead of the row's cells.
    pub fn sections(mut self, key: impl Into<String>, children: Vec<WorkflowDisplay>) -> Self {
        if let DisplayType::Table { section_key, section, .. } = &mut self.r#type {
            *section_key = Some(key.into());
            *section = children;
        }
        self
    }

    /// The icon of a feature, chip, notice, empty state or button.
    pub fn with_icon(mut self, name: impl Into<String>) -> Self {
        let name = Some(name.into());
        match &mut self.r#type {
            DisplayType::Feature { icon, .. }
            | DisplayType::Chip { icon, .. }
            | DisplayType::Notice { icon, .. }
            | DisplayType::Empty { icon, .. }
            | DisplayType::Badge { icon, .. }
            | DisplayType::Action { icon, .. } => *icon = name,
            _ => {}
        }
        self
    }

    /// Makes a feature a card, or a row a link, that opens `to`.
    pub fn href(mut self, to: impl Into<String>) -> Self {
        let to = Some(to.into());
        match &mut self.r#type {
            DisplayType::Feature { href, .. } | DisplayType::Row { href, .. } => *href = to,
            _ => {}
        }
        self
    }

    /// A small square picture on the right of a link's content, such as a
    /// story's thumbnail; a template, left out while it resolves to nothing.
    pub fn thumb(mut self, src: impl Into<String>) -> Self {
        if let DisplayType::Link { thumb, .. } = &mut self.r#type {
            *thumb = Some(src.into());
        }
        self
    }

    /// Makes a row or a chip run `act` when pressed.
    pub fn on_press(mut self, value: Act) -> Self {
        match &mut self.r#type {
            DisplayType::Row { act, .. } | DisplayType::Chip { act, .. } => *act = Some(value),
            _ => {}
        }
        self
    }

    /// A title on a notice; a body on an empty state.
    pub fn titled(mut self, value: impl Into<String>) -> Self {
        let value = Some(value.into());
        match &mut self.r#type {
            DisplayType::Notice { title, .. } => *title = value,
            DisplayType::Empty { body, .. } => *body = value,
            _ => {}
        }
        self
    }

    /// The buttons on a notice or an empty state.
    pub fn actions(mut self, content: Vec<WorkflowDisplay>) -> Self {
        match &mut self.r#type {
            DisplayType::Notice { actions, .. } | DisplayType::Empty { actions, .. } => *actions = content,
            _ => {}
        }
        self
    }

    /// A row's rank, avatar or icon; a stepper's subtitle goes through `.meta`.
    pub fn lead_with(mut self, content: Vec<WorkflowDisplay>) -> Self {
        if let DisplayType::Row { lead, .. } = &mut self.r#type {
            *lead = content;
        }
        self
    }

    /// What sits at the right of a row.
    pub fn trail(mut self, content: Vec<WorkflowDisplay>) -> Self {
        if let DisplayType::Row { trail, .. } = &mut self.r#type {
            *trail = content;
        }
        self
    }

    /// The line under a row's title, a person's name or a stepper's title.
    pub fn meta(mut self, value: impl Into<String>) -> Self {
        let value = Some(value.into());
        match &mut self.r#type {
            DisplayType::Row { meta, .. }
            | DisplayType::Person { meta, .. }
            | DisplayType::Stepper { subtitle: meta, .. } => *meta = value,
            _ => {}
        }
        self
    }

    /// Whose profile card pressing a person opens, as a user id template.
    pub fn user(mut self, id: impl Into<String>) -> Self {
        if let DisplayType::Person { user, .. } = &mut self.r#type {
            *user = Some(id.into());
        }
        self
    }

    /// The label and value text around a meter, or the label under segments.
    pub fn labelled(mut self, value: impl Into<String>) -> Self {
        let value = Some(value.into());
        match &mut self.r#type {
            DisplayType::Meter { label, .. } | DisplayType::Segments { label, .. } => *label = value,
            _ => {}
        }
        self
    }

    pub fn valued(mut self, text: impl Into<String>) -> Self {
        if let DisplayType::Meter { value, .. } = &mut self.r#type {
            *value = Some(text.into());
        }
        self
    }

    /// Lays a choice out in a row instead of a column.
    pub fn across(mut self) -> Self {
        if let DisplayType::Choice { direction, .. } = &mut self.r#type {
            *direction = FlexDirection::Row;
        }
        self
    }

    /// The size of a person, media, a popover or carousel items.
    pub fn size(mut self, value: Size) -> Self {
        match &mut self.r#type {
            DisplayType::Person { size, .. }
            | DisplayType::Media { size, .. }
            | DisplayType::Popover { width: size, .. }
            | DisplayType::Carousel { item_size: size, .. } => *size = value,
            _ => {}
        }
        self
    }

    pub fn shape(mut self, value: Shape) -> Self {
        if let DisplayType::Media { shape, .. } = &mut self.r#type {
            *shape = value;
        }
        self
    }

    pub fn fit(mut self, value: Fit) -> Self {
        if let DisplayType::Media { fit, .. } = &mut self.r#type {
            *fit = value;
        }
        self
    }

    /// Something over the top corner of media, such as a status badge.
    pub fn badge(mut self, content: Vec<WorkflowDisplay>) -> Self {
        if let DisplayType::Media { badge, .. } = &mut self.r#type {
            *badge = content;
        }
        self
    }

    /// A caption across the foot of media, over a shade.
    pub fn caption(mut self, content: Vec<WorkflowDisplay>) -> Self {
        if let DisplayType::Media { caption, .. } = &mut self.r#type {
            *caption = content;
        }
        self
    }

    /// A caption in a band beneath media, leaving the whole picture clear.
    pub fn caption_below(mut self, content: Vec<WorkflowDisplay>) -> Self {
        if let DisplayType::Media { caption, caption_placement, .. } = &mut self.r#type {
            *caption = content;
            *caption_placement = CaptionPlacement::Below;
        }
        self
    }

    /// Stretches a button across its container.
    pub fn full_width(mut self) -> Self {
        if let DisplayType::Action { full_width, .. } = &mut self.r#type {
            *full_width = true;
        }
        self
    }

    /// Opens a `Dialog` as a drawer from the side instead of a modal.
    pub fn drawer(mut self) -> Self {
        if let DisplayType::Dialog { style, .. } = &mut self.r#type {
            *style = crate::DialogStyle::Drawer;
        }
        self
    }

    /// Closes a `Dialog` once a form inside has been sent and not refused.
    pub fn closes_on_submit(mut self) -> Self {
        if let DisplayType::Dialog { close_on_submit, .. } = &mut self.r#type {
            *close_on_submit = true;
        }
        self
    }

    /// Makes a button send the form it sits in.
    pub fn submits(mut self) -> Self {
        if let DisplayType::Action { submit, .. } = &mut self.r#type {
            *submit = true;
        }
        self
    }

    /// Scrolls to the element with this id, or the display marked with this
    /// name, when the button is pressed.
    pub fn scroll_to(mut self, id: impl Into<String>) -> Self {
        if let DisplayType::Action { scroll_to, .. } = &mut self.r#type {
            *scroll_to = Some(id.into());
        }
        self
    }

    /// At most this many avatars before a count of the rest.
    pub fn max(mut self, value: u8) -> Self {
        if let DisplayType::AvatarStack { max, .. } = &mut self.r#type {
            *max = value;
        }
        self
    }

    /// Text adjustments; see `TextStyle`.
    pub fn numeric(self) -> Self {
        self.text_style(|style| style.numeric = true)
    }

    pub fn clamp(self, lines: u8) -> Self {
        self.text_style(|style| style.clamp = lines)
    }

    /// Keeps the room of all its clamped lines, so texts side by side line up.
    pub fn reserve_lines(self) -> Self {
        self.text_style(|style| style.reserve = true)
    }

    pub fn centered(self) -> Self {
        self.text_style(|style| style.align = TextAlign::Center)
    }

    pub fn end_aligned(self) -> Self {
        self.text_style(|style| style.align = TextAlign::End)
    }

    pub fn struck(self) -> Self {
        self.text_style(|style| style.struck = true)
    }

    pub fn keep_lines(self) -> Self {
        self.text_style(|style| style.lines = true)
    }

    pub fn themed(self) -> Self {
        self.text_style(|style| style.themed = true)
    }

    fn text_style(mut self, change: impl FnOnce(&mut TextStyle)) -> Self {
        if let DisplayType::Typography { style, .. } = &mut self.r#type {
            change(style);
        }
        self
    }

    /// The space between the children of a `Stack` or `Grid`.
    pub fn gap(mut self, value: Gap) -> Self {
        match &mut self.r#type {
            DisplayType::Stack { gap, .. } | DisplayType::Grid { gap, .. } => *gap = value,
            _ => {}
        }
        self
    }

    /// Keeps a grid's columns on a phone, for a grid or a list laid out as one.
    pub fn fixed(mut self) -> Self {
        match &mut self.r#type {
            DisplayType::Grid { fixed, .. } | DisplayType::Items { layout: Layout::Grid { fixed, .. }, .. } => {
                *fixed = true
            }
            _ => {}
        }
        self
    }

    /// The text role of a countdown or a time.
    pub fn role(mut self, value: TextRole) -> Self {
        match &mut self.r#type {
            DisplayType::Countdown { role, .. } | DisplayType::Time { role, .. } => *role = value,
            _ => {}
        }
        self
    }

    /// Shakes a countdown once less than `ms` is left.
    pub fn shake_under(mut self, ms: u32) -> Self {
        if let DisplayType::Countdown { shake_under_ms_left, .. } = &mut self.r#type {
            *shake_under_ms_left = Some(ms);
        }
        self
    }

    /// Names the display for animations, which find it as `[data-mark~="name"]`.
    pub fn mark(mut self, name: impl Into<String>) -> Self {
        self.mark = Some(name.into());
        self
    }

    pub fn align(mut self, value: Align) -> Self {
        if let DisplayType::Stack { align, .. } = &mut self.r#type {
            *align = value;
        }
        self
    }

    pub fn justify(mut self, value: Justify) -> Self {
        if let DisplayType::Stack { justify, .. } = &mut self.r#type {
            *justify = value;
        }
        self
    }

    /// Spreads a row's children to its two ends.
    pub fn between(self) -> Self {
        self.justify(Justify::Between)
    }

    pub fn wrap(mut self) -> Self {
        if let DisplayType::Stack { wrap, .. } = &mut self.r#type {
            *wrap = true;
        }
        self
    }

    /// Runs `act` before a discussion's first reaction or comment, to make the message they hang off.
    pub fn starts_with(mut self, value: Act) -> Self {
        if let DisplayType::Discussion { start, .. } = &mut self.r#type {
            *start = Some(value);
        }
        self
    }

    /// The response key holding the emoji that fit a discussion, best first.
    pub fn suggested(mut self, key: impl Into<String>) -> Self {
        if let DisplayType::Discussion { suggested, .. } = &mut self.r#type {
            *suggested = Some(key.into());
        }
        self
    }

    /// Lays a discussion's comments out on the page instead of behind a button.
    pub fn inline(mut self) -> Self {
        if let DisplayType::Discussion { inline, .. } = &mut self.r#type {
            *inline = true;
        }
        self
    }

    /// What a discussion is about, such as a story's headline, named beside its comments.
    pub fn about(mut self, value: impl Into<String>) -> Self {
        if let DisplayType::Discussion { title, .. } = &mut self.r#type {
            *title = Some(value.into());
        }
        self
    }

    pub fn nowrap(mut self) -> Self {
        if let DisplayType::Stack { wrap, .. } = &mut self.r#type {
            *wrap = false;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classes_from_older_bots_are_ignored() {
        let link: WorkflowDisplay =
            serde_json::from_value(json!({ "class": "underline", "type": { "Link": { "href": "/x", "content": [] } } }))
                .unwrap();
        assert!(matches!(link.r#type, DisplayType::Link { style: LinkStyle::Plain, .. }));

        let icon: WorkflowDisplay = serde_json::from_value(json!({
            "class": "size-4",
            "type": { "Icon": { "lib": "Lucide", "name": "star", "aria_label": null, "url": null } }
        }))
        .unwrap();
        assert!(matches!(&icon.r#type, DisplayType::Icon { size: IconSize::Inherit, tone, .. } if tone.is_empty()));
    }

    #[test]
    fn every_display_example_in_the_docs_reads() {
        let docs = include_str!("../../../docs/05-displays.md");
        let mut examples = 0;
        for block in docs.split("```display\n").skip(1) {
            let source = block.split("\n```").next().unwrap();
            let example: serde_json::Value = serde_json::from_str(source).expect("a display example is JSON");
            serde_json::from_value::<WorkflowDisplay>(example["display"].clone())
                .unwrap_or_else(|error| panic!("{error} in display example:\n{source}"));
            examples += 1;
        }
        assert!(examples > 10);
    }

    #[test]
    fn current_and_older_renderers_get_one_side_each() {
        let inputs = |client: serde_json::Value| std::collections::HashMap::from([("$client".to_string(), client)]);
        let current = |client| crate::condition::eval_condition(&renderer_is_current(), &inputs(client), &Default::default());
        assert!(current(json!({ "desktop": false })));
        assert!(current(json!({ "desktop": true, "renderer": 2 })));
        assert!(!current(json!({ "desktop": true })));
    }

    #[test]
    fn an_accent_is_only_ever_a_colour() {
        let row = list_row("Devils").accent_from("{r.accent}");
        assert!(matches!(&row.r#type, DisplayType::Row { accent: Some(a), .. } if a.as_str() == "{r.accent}"));
        let row = list_row("Devils").accent_from("red; background: url(x)");
        assert!(matches!(&row.r#type, DisplayType::Row { accent: None, .. }));

        for good in ["#CE1126", "#fff", "{r.accent}"] {
            assert!(serde_json::from_value::<Accent>(json!(good)).is_ok(), "{good}");
        }
        for bad in ["red", "#12345", "#CE1126; color: red", "{a}{b}", "{}", "url(x)", "{r.accent}x"] {
            assert!(serde_json::from_value::<Accent>(json!(bad)).is_err(), "{bad}");
        }
        let table = serde_json::from_value::<WorkflowDisplay>(json!({ "type": { "Table": {
            "items_key": "rows", "as": "r", "headers": [], "row": [], "row_accent": "expression(alert(1))"
        } } }));
        assert!(table.is_err());
        let plain = serde_json::to_value(list_row("Devils")).unwrap();
        assert!(plain["type"]["Row"].get("accent").is_none());
    }

    #[test]
    fn a_table_carries_its_sections_and_older_tables_still_read() {
        let table = crate::builder::WorkflowDisplayBuilder::new()
            .table("rows", "r", vec![], vec![body("{r.team}")])
            .build()
            .sections("{r.section}", vec![subheading("{r.label}")]);
        let value = serde_json::to_value(&table).unwrap();
        assert_eq!(value["type"]["Table"]["section_key"], "{r.section}");
        assert_eq!(value["type"]["Table"]["section"].as_array().unwrap().len(), 1);

        let older = serde_json::from_value::<WorkflowDisplay>(json!({ "type": { "Table": {
            "items_key": "rows", "as": "r", "headers": [], "row": []
        } } }))
        .unwrap();
        assert!(matches!(&older.r#type, DisplayType::Table { section_key: None, section, .. } if section.is_empty()));
        let plain = serde_json::to_value(&older).unwrap();
        assert!(plain["type"]["Table"].get("section").is_none());
    }

    #[test]
    fn modifiers_only_change_the_blocks_they_name() {
        let tinted = body("Text").tone(Tone::Danger);
        assert!(matches!(&tinted.r#type, DisplayType::Typography { tone, .. } if tone == "Danger"));

        let spread = row(Gap::Normal, vec![]).between();
        assert!(matches!(
            spread.r#type,
            DisplayType::Stack { justify: Justify::Between, align: Align::Center, wrap: true, .. }
        ));

        let untouched = feature("A", "B").tone(Tone::Danger).gap(Gap::None);
        assert!(matches!(untouched.r#type, DisplayType::Feature { .. }));

        let board = items("cells", "c", Layout::Grid { columns: 4, gap: Gap::Tight, fixed: false }, vec![]).fixed();
        assert!(matches!(board.r#type, DisplayType::Items { layout: Layout::Grid { fixed: true, .. }, .. }));

        let toggle = chip("Hide discussions").on_press(act("filter")).state(State::Selected);
        assert!(matches!(&toggle.r#type, DisplayType::Chip { act: Some(_), state: Some(state), .. } if state == "Selected"));
    }

    #[test]
    fn a_link_without_a_thumbnail_serializes_as_before() {
        let plain = serde_json::to_value(link_card("/a", vec![])).unwrap();
        assert!(plain["type"]["Link"].get("thumb").is_none());
        let story = serde_json::to_value(link_entry("/a", vec![]).thumb("{s.image}")).unwrap();
        assert_eq!(story["type"]["Link"]["thumb"], "{s.image}");
        assert_eq!(story["type"]["Link"]["style"], "Entry");
    }

    #[test]
    fn plain_chips_and_splits_serialize_as_before() {
        assert_eq!(serde_json::to_value(chip("Beta")).unwrap(), json!({ "type": { "Chip": { "icon": null, "text": "Beta" } } }));
        assert_eq!(serde_json::to_value(split(vec![], vec![])).unwrap(), json!({ "type": { "Split": { "media": [], "content": [] } } }));
        assert_eq!(serde_json::to_value(rail(vec![], vec![])).unwrap()["type"]["Split"]["rail"], true);
    }

    #[test]
    fn a_discussion_carries_its_start_and_suggestions() {
        let story = discussion("{s.chat}", "{s.discussion}", "s.reactions", "s.comments")
            .starts_with(act("feed").input("op", "discuss"))
            .suggested("s.suggested");
        let value = serde_json::to_value(&story).unwrap();
        assert_eq!(value["type"]["Discussion"]["message"], "{s.discussion}");
        assert_eq!(value["type"]["Discussion"]["suggested"], "s.suggested");
        assert_eq!(value["type"]["Discussion"]["inline"], false);
        assert_eq!(value["type"]["Discussion"]["start"]["action"], "feed");
        assert!(value["type"]["Discussion"].get("title").is_none());
        let read: WorkflowDisplay = serde_json::from_value(json!({
            "type": { "Discussion": { "chat": "{c}", "message": "{m}", "reactions": "r", "comments": "c" } }
        }))
        .unwrap();
        assert!(matches!(read.r#type, DisplayType::Discussion { inline: false, start: None, .. }));
        let page = discussion("{c}", "{m}", "r", "c").inline().about("{story.title}");
        assert!(matches!(&page.r#type, DisplayType::Discussion { inline: true, title: Some(t), .. } if t == "{story.title}"));
    }

    #[test]
    fn a_mark_is_left_out_until_it_is_set() {
        assert!(serde_json::to_value(body("Text")).unwrap().get("mark").is_none());
        assert_eq!(serde_json::to_value(body("Text").mark("winner")).unwrap()["mark"], "winner");
    }
}
