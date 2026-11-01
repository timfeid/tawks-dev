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
mod tolerant;
pub mod ui;

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
        /// Lays the options out in this many columns; 0 stacks them.
        #[serde(default)]
        columns: u8,
        min: Option<u32>,
        max: Option<u32>,
    },
    SelectFromList {
        items_key: String,
        r#as: String,
        value: String,
        content: Vec<WorkflowDisplay>,
        /// Lays the options out in this many columns; 0 stacks them.
        #[serde(default)]
        columns: u8,
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

/// Space between the children of a `Stack` or `Grid`.
#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gap {
    None,
    Tight,
    #[default]
    Normal,
    Loose,
    Section,
}

#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Stretch,
    Start,
    Center,
    End,
    Baseline,
}

#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    Between,
}

/// Declares a plain enum whose variants travel as their own names, so a
/// display field can hold one directly or a `{key}` template that resolves to
/// one from response data.
macro_rules! named {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name {
            #[default]
            $($(#[$vmeta])* $variant),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => stringify!($variant)),+ }
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.as_str().to_string()
            }
        }
    };
}

named! {
    /// A colour from the theme. `Default` leaves the colour to the display, so a
    /// heading stays bright and body text stays muted. The named accents after
    /// `Live` are for things told apart by colour: puzzle groups, alliances,
    /// answers, metrics.
    Tone {
        Default, Strong, Muted, Faint, Primary, Success, Warning, Danger, Info, Live,
        Red, Orange, Amber, Yellow, Lime, Green, Emerald, Teal, Cyan, Sky, Blue, Indigo,
        Violet, Purple, Fuchsia, Pink, Rose, Gold, Slate,
    }
}

named! {
    /// How a row, a tile or a tab stands relative to the rest.
    State {
        Default,
        /// Picked by the viewer.
        Selected,
        /// Worth noticing, such as the viewer's own row.
        Highlight,
        /// Set aside: used, eliminated, out of play.
        Muted,
        /// Can't be chosen right now.
        Disabled,
        Success,
        Danger,
        Warning,
        Live,
        Pending,
    }
}

/// What a piece of text is, which decides its size, weight and colour.
#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextRole {
    /// The page's own title.
    Display,
    /// A large title inside the page, such as a highlighted panel's.
    Title,
    /// A section heading.
    Heading,
    /// The title of an item, a row or a panel.
    Subheading,
    /// The title of a story in a feed: larger than a subheading, smaller than a heading.
    Headline,
    /// The introduction under a title.
    Lead,
    #[default]
    Body,
    /// Small print: requirements, captions and footnotes.
    Fine,
    /// A short uppercase label above a title.
    Eyebrow,
    /// Something to copy, such as a command.
    Code,
    /// A number that matters, such as a score.
    Stat,
}

#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageWidth {
    Narrow,
    #[default]
    Normal,
    Wide,
    Full,
}

#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStyle {
    /// Wraps its content and adds nothing, the way a link always used to.
    #[default]
    Plain,
    /// Text in the primary colour, underlined on hover.
    Inline,
    /// The page's main call to action.
    Primary,
    /// An outlined button beside a primary one.
    Secondary,
    /// A whole panel that opens the link, lighting up under the pointer.
    Card,
    /// An entry in a list, such as a story in a feed: no box of its own,
    /// room above and below, a light wash under the pointer. Sits well in a
    /// `Divided` list.
    Entry,
}

#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconSize {
    /// The size of the text around it.
    #[default]
    Inherit,
    Small,
    Medium,
    Large,
}

named! {
    Size { Medium, ExtraSmall, Small, Large, ExtraLarge, Fill }
}

named! {
    /// Proportions for media: `Poster` is 2:3, `Portrait` 3:4, `Card` a playing card.
    Aspect { Auto, Square, Video, Poster, Portrait, Card, Wide }
}

named! {
    Shape { Rounded, Round, Square }
}

named! {
    Fit { Cover, Contain }
}

named! {
    /// Where a media caption sits: over the foot of the picture on a shade, or
    /// in a band beneath it so the whole picture stays clear.
    CaptionPlacement { Over, Below }
}

named! {
    TabStyle { Bottom, Underline, Segmented }
}

named! {
    ActionLook { Primary, Secondary, Ghost, Danger, Icon, Link }
}

named! {
    TextAlign { Start, Center, End }
}

named! {
    /// What sits behind a page: the app's own background, or near black for a game.
    Backdrop { Default, Dark, Black }
}

named! {
    /// The typeface a page marks with `themed` text uses: the app's own,
    /// a tall condensed one for sports and lobbies, or a spooky one for Midnight.
    Typeface { Default, Condensed, Spooky }
}

named! {
    /// Where an overlay sits over the game canvas before it.
    Corner { BottomLeft, BottomRight, TopLeft, TopRight, Top, Bottom }
}

/// Finer control of a piece of text.
#[derive(Type, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextStyle {
    /// Figures of equal width, so numbers line up.
    #[serde(default)]
    pub numeric: bool,
    /// Cuts the text off after this many lines; 0 lets it run.
    #[serde(default)]
    pub clamp: u8,
    /// Keeps the room of all `clamp` lines even when the text is shorter,
    /// so texts side by side start and end at the same place.
    #[serde(default)]
    pub reserve: bool,
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default)]
    pub struck: bool,
    /// Keeps the line breaks in the text.
    #[serde(default)]
    pub lines: bool,
    /// The game's own typeface, where the page sets one.
    #[serde(default)]
    pub themed: bool,
}

/// What pressing something does: run `action` with `inputs`. `disabled`
/// greys it out while it holds.
#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct Act {
    pub action: String,
    #[serde(default)]
    pub inputs: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub disabled: Option<NodeCondition>,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub label: String,
    pub icon: Option<String>,
    /// Shows the tab as the current one while it holds.
    pub active: NodeCondition,
    pub act: Option<Act>,
    pub href: Option<String>,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct GroupButton {
    pub label: String,
    pub act: Act,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum Layout {
    Stack { gap: Gap },
    Row { gap: Gap },
    /// `fixed` keeps every column on a phone too, for boards and tile grids.
    Grid {
        columns: u8,
        gap: Gap,
        #[serde(default)]
        fixed: bool,
    },
    /// Rows with a rule between them.
    Divided,
    /// A stack that shows about `rows` rows and scrolls the rest.
    Scroll { gap: Gap, rows: u8 },
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
    /// Lines figures up and right-aligns them.
    #[serde(default)]
    pub numeric: bool,
    /// Leaves the column out on narrow screens.
    #[serde(default)]
    pub wide_only: bool,
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDisplay {
    #[serde(deserialize_with = "tolerant::deserialize")]
    pub r#type: DisplayType,
    /// A name animations find this display by, as `[data-mark~="name"]`.
    /// Takes `{key}` templates and several names separated by spaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<String>,
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
        #[serde(default)]
        width: Size,
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
        #[serde(default)]
        role: TextRole,
        #[serde(default)]
        tone: String,
    },
    Icon {
        lib: IconLibrary,
        name: String,
        aria_label: Option<String>,
        url: Option<String>,
        #[serde(default)]
        size: IconSize,
        #[serde(default)]
        tone: String,
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
        #[serde(default)]
        role: TextRole,
        #[serde(default)]
        tone: String,
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
        #[serde(default)]
        tone: String,
    },
    Badge {
        variant: String,
        text: String,
        #[serde(default)]
        tone: String,
        #[serde(default)]
        icon: Option<String>,
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
        #[serde(default)]
        item_size: Size,
    },
    Table {
        items_key: String,
        r#as: String,
        headers: Vec<TableHeader>,
        row: Vec<WorkflowDisplay>,
        /// A `{key}` naming each row's `State`.
        #[serde(default)]
        row_state: Option<String>,
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
        /// The result appearing means a game just finished, so the app may celebrate it.
        #[serde(default)]
        celebrate: bool,
    },
    Loader,
    Link {
        href: String,
        content: Vec<WorkflowDisplay>,
        #[serde(default)]
        style: LinkStyle,
        /// A picture (a template) shown as a small square beside the content,
        /// on its right; left out while it resolves to nothing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thumb: Option<String>,
    },
    /// Children in a column or a row, `gap` apart.
    Stack {
        direction: FlexDirection,
        gap: Gap,
        #[serde(default)]
        align: Align,
        #[serde(default)]
        justify: Justify,
        #[serde(default)]
        wrap: bool,
        content: Vec<WorkflowDisplay>,
    },
    /// Children in `columns` equal columns, fewer as the space narrows and
    /// one on a phone.
    Grid {
        columns: u8,
        gap: Gap,
        content: Vec<WorkflowDisplay>,
        /// Keeps every column on a phone instead of folding them up.
        #[serde(default)]
        fixed: bool,
    },
    /// A rounded, bordered box. `Primary` highlights it; `Warning` and
    /// `Danger` call attention to it.
    Panel {
        tone: String,
        content: Vec<WorkflowDisplay>,
    },
    /// The top of a page: an optional eyebrow, the title with `aside` beside
    /// it, the lead and a row of `actions`.
    Hero {
        eyebrow: Option<String>,
        title: String,
        lead: Option<String>,
        aside: Vec<WorkflowDisplay>,
        actions: Vec<WorkflowDisplay>,
    },
    /// A part of a page under a rule, with its heading, a lead and `aside`
    /// across from the heading.
    Section {
        eyebrow: Option<String>,
        title: String,
        lead: Option<String>,
        aside: Vec<WorkflowDisplay>,
        content: Vec<WorkflowDisplay>,
    },
    /// An icon beside a title and a line or two; a card to open when it has
    /// an `href`.
    Feature {
        icon: Option<String>,
        title: String,
        body: String,
        href: Option<String>,
    },
    /// A small rounded label with an optional icon. With `act` it is a
    /// toggle: pressing it runs `act`, and a `Selected` state shows it as on,
    /// with a cross, since pressing it then turns it off.
    Chip {
        icon: Option<String>,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        act: Option<Act>,
        /// A `State`, or a `{key}` naming one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<String>,
    },
    /// Text styled by what it is.
    Typography {
        text: String,
        role: TextRole,
        tone: String,
        #[serde(default)]
        style: TextStyle,
    },
    /// A notice across the page: an error, a hint, a call to sign in.
    Notice {
        tone: String,
        icon: Option<String>,
        title: Option<String>,
        body: String,
        actions: Vec<WorkflowDisplay>,
    },
    /// Switches between views; `Bottom` sits fixed at the foot of the page.
    TabBar {
        style: TabStyle,
        tabs: Vec<Tab>,
    },
    /// A compact segmented strip of buttons, each running its own act, for
    /// quick picks such as 1, 3 or 5 tickets. Unlike tabs, none stays current.
    ButtonGroup {
        buttons: Vec<GroupButton>,
    },
    /// A label between back and forward buttons, for stepping through days or weeks.
    Stepper {
        title: String,
        subtitle: Option<String>,
        previous: Option<Act>,
        next: Option<Act>,
    },
    /// One line in a list: `lead` (a rank, an avatar, an icon), a title over a
    /// meta line, and `trail` on the right. `state` may be a `{key}`.
    Row {
        lead: Vec<WorkflowDisplay>,
        title: String,
        meta: Option<String>,
        trail: Vec<WorkflowDisplay>,
        state: String,
        href: Option<String>,
        act: Option<Act>,
    },
    /// A tile to pick, styled by `state` (a `{key}` is fine): picks, answers, slots.
    Choice {
        content: Vec<WorkflowDisplay>,
        state: String,
        act: Act,
        direction: FlexDirection,
    },
    /// A number over its label.
    Stat {
        value: String,
        label: String,
        tone: String,
    },
    /// A thin bar filled to `percent` (a template for a number from 0 to 100).
    Meter {
        label: Option<String>,
        value: Option<String>,
        percent: String,
        tone: String,
    },
    /// A row of small segments, one per item, each coloured by its `state` template.
    Segments {
        items_key: String,
        r#as: String,
        state: String,
        label: Option<String>,
    },
    /// What a page shows with nothing in it yet, or while it loads.
    Empty {
        icon: Option<String>,
        title: String,
        body: Option<String>,
        loading: bool,
        actions: Vec<WorkflowDisplay>,
    },
    /// Someone's avatar beside their name; the avatar falls back to an initial.
    Person {
        avatar: String,
        name: String,
        meta: Option<String>,
        size: Size,
        /// A user id template; pressing the person then opens their profile card.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user: Option<String>,
    },
    /// Overlapping avatars for a list, with a count of the rest.
    AvatarStack {
        items_key: String,
        r#as: String,
        avatar: String,
        name: String,
        max: u8,
    },
    Divider {
        label: Option<String>,
    },
    /// Lines of output in a scrolling monospace box that keeps to the end.
    Log {
        text: String,
    },
    /// Media beside content when there is room, above it when not. As a
    /// `rail`, `media` is a side column after the content instead, shown only
    /// when there is room for both.
    Split {
        media: Vec<WorkflowDisplay>,
        content: Vec<WorkflowDisplay>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rail: Option<bool>,
    },
    /// Buttons held at the foot of the page.
    ActionBar {
        content: Vec<WorkflowDisplay>,
    },
    /// An image shaped by `aspect` and `size`, with an optional `badge` over a
    /// corner and a `caption` across its foot.
    Media {
        src: String,
        alt: String,
        aspect: Aspect,
        size: Size,
        shape: Shape,
        fit: Fit,
        badge: Vec<WorkflowDisplay>,
        caption: Vec<WorkflowDisplay>,
        /// Washes the picture in a tone's colour, such as a role's alliance.
        #[serde(default)]
        tint: String,
        #[serde(default)]
        caption_placement: CaptionPlacement,
    },
    /// Floats over the game canvas just before it, in one corner, with the
    /// canvas still playable around it. Docks to the bottom on a phone.
    Overlay {
        corner: Corner,
        content: Vec<WorkflowDisplay>,
    },
    /// A band in `color` with `image` large and faint in its corner, washed
    /// dark so `content` reads over it.
    Backdrop {
        color: String,
        image: String,
        content: Vec<WorkflowDisplay>,
    },
    /// A month, seven days to a row under the weekday names. Each item of
    /// `items_key` is one cell: `day`, and optionally `outside` (true for days
    /// of the months either side), `today`, `href`, `image`, `label`,
    /// `detail` and `tone`.
    Calendar {
        items_key: String,
    },
    /// Repeats `content` for each item of `items_key`, laid out by `layout`.
    Items {
        items_key: String,
        r#as: String,
        layout: Layout,
        content: Vec<WorkflowDisplay>,
    },
    /// A button. `look` decides how it is drawn; `full_width` stretches it;
    /// `submit` makes it send its form; `scroll_to` scrolls to an element id or a mark.
    Action {
        label: String,
        icon: Option<String>,
        look: ActionLook,
        act: Act,
        full_width: bool,
        submit: bool,
        scroll_to: Option<String>,
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
    /// An authenticated, live installed-game invitation in a conversation.
    GameInvite,
    /// A primary button that starts the first-party `game` when the desktop
    /// app has it installed, and otherwise opens `fallback_url`.
    PlayGame {
        game: String,
        label: String,
        fallback_url: String,
    },
    /// Reactions and comments on something the page shows, such as a story,
    /// kept as a thread on a chat message so they arrive live, can be
    /// moderated and follow blocks. `chat` and `message` are templates for
    /// that message's chat and id; while `message` resolves to nothing, the first reaction or
    /// comment runs `start`, which answers by filling it in. `reactions` and
    /// `comments` name what the page last knew: a list of `emoji`, `count`
    /// and `me`, and a number. `suggested` names a list of emoji that fit,
    /// best first: hovering shows the first, and hovering that shows the
    /// rest beside a picker for any other. `inline` lays the comments out on the page instead of
    /// behind a button; `title` says what they are about, such as the story's
    /// headline, beside the comments when they open.
    Discussion {
        chat: String,
        message: String,
        reactions: String,
        comments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<Act>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        suggested: Option<String>,
        #[serde(default)]
        inline: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// A display this build does not know, such as one a newer bot sends
    /// while the API is behind. It draws nothing.
    Unknown {
        name: String,
    },
}

impl tolerant::Tolerant for DisplayType {
    const KIND: &'static str = "display";

    fn unknown(name: String) -> Self {
        DisplayType::Unknown { name }
    }
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
    /// A plain container, such as a card posted into a chat.
    Block {
        content: Vec<WorkflowDisplay>,
    },
    /// The page column every channel page sits in.
    Page {
        width: PageWidth,
        content: Vec<WorkflowDisplay>,
        /// Centres the content up and down, for a page that is mostly one thing.
        #[serde(default)]
        center: bool,
        #[serde(default)]
        backdrop: Backdrop,
        #[serde(default)]
        typeface: Typeface,
    },

    Chat {
        chat_id: String,
    },

    Redirect {
        url_key: String,
    },
    /// A visual this build does not know. It draws nothing.
    Unknown {
        name: String,
    },
}

impl tolerant::Tolerant for DisplayVisual {
    const KIND: &'static str = "visual";

    fn unknown(name: String) -> Self {
        DisplayVisual::Unknown { name }
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowVisual {
    pub id: String,
    #[serde(deserialize_with = "tolerant::deserialize")]
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
