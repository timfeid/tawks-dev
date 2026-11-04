import type {
  Act,
  ActionLook,
  Align,
  Aspect,
  Backdrop,
  CaptionPlacement,
  ChartConfig,
  ChartType,
  Corner,
  CreateWorkflowDefinition,
  DisplayType,
  Fit,
  FlexDirection,
  Gap,
  GroupButton,
  IconSize,
  InputType,
  JsonValue,
  Justify,
  Layout,
  LinkStyle,
  NodeCondition,
  PageWidth,
  Shape,
  Size,
  State,
  Tab,
  TabStyle,
  TextAlign,
  TextRole,
  TimeDisplayOpts,
  Tone,
  Typeface,
  WorkflowAction,
  WorkflowDisplay,
  WorkflowNode,
  WorkflowVisual,
} from '@tawks/workflow-types';

/**
 * The building blocks every page is made of, mirroring `workflow_model::ui`.
 * Each says what something is (a heading, a panel, a primary link) and tawks
 * decides how it looks. Each returns the plain JSON tawks expects, so you can
 * mix them with hand-written displays from the Display reference.
 *
 * Fields that take a tone or a state also take a `{key}` template, so a bot
 * can send `"Selected"` or `"Success"` in its responses and the page picks it
 * up: `listRow('{s.name}', { state: '{s.state}' })`.
 */

type Map = Partial<Record<string, JsonValue>>;
type Template<T extends string> = T | (string & {});

/** `mark` names a display for animations, which find it as `[data-mark~="name"]`. */
export type Marked = { mark?: string };
export type ToneOption = { tone?: Template<Tone> };
export type StateOption = { state?: Template<State> };

const block = (type: DisplayType, mark?: string): WorkflowDisplay => (mark === undefined ? { type } : { type, mark });

/** A copy of `display` named `name` for animations. */
export const mark = (display: WorkflowDisplay, name: string): WorkflowDisplay => ({ ...display, mark: name });

export type TextOptions = Marked &
  ToneOption & { numeric?: boolean; clamp?: number; align?: TextAlign; struck?: boolean; lines?: boolean; themed?: boolean };

/** Text styled by what it is. `{key}` shows a response, such as `"Hello, {name}!"`. */
export const text = (role: TextRole, value: string, o: TextOptions = {}): WorkflowDisplay =>
  block(
    {
      Typography: {
        text: value,
        role,
        tone: o.tone ?? '',
        style: {
          numeric: o.numeric ?? false,
          clamp: o.clamp ?? 0,
          align: o.align ?? 'Start',
          struck: o.struck ?? false,
          lines: o.lines ?? false,
          themed: o.themed ?? false,
        },
      },
    },
    o.mark,
  );

/** The page's own title. */
export const displayTitle = (s: string, o?: TextOptions) => text('Display', s, o);
export const title = (s: string, o?: TextOptions) => text('Title', s, o);
export const heading = (s: string, o?: TextOptions) => text('Heading', s, o);
export const subheading = (s: string, o?: TextOptions) => text('Subheading', s, o);
/** The title of a story in a feed. */
export const headline = (s: string, o?: TextOptions) => text('Headline', s, o);
export const lead = (s: string, o?: TextOptions) => text('Lead', s, o);
export const body = (s: string, o?: TextOptions) => text('Body', s, o);
/** Small print: requirements, captions and footnotes. */
export const fine = (s: string, o?: TextOptions) => text('Fine', s, o);
/** A short uppercase label above a title. */
export const eyebrow = (s: string, o?: TextOptions) => text('Eyebrow', s, o);
export const code = (s: string, o?: TextOptions) => text('Code', s, o);
/** A number that matters, such as a score. */
export const statText = (s: string, o?: TextOptions) => text('Stat', s, o);

export type StackOptions = Marked & { align?: Align; justify?: Justify; wrap?: boolean };

const stack = (direction: FlexDirection, gap: Gap, content: WorkflowDisplay[], o: StackOptions, align: Align, wrap: boolean) =>
  block(
    { Stack: { direction, gap, align: o.align ?? align, justify: o.justify ?? 'Start', wrap: o.wrap ?? wrap, content } },
    o.mark,
  );

export const column = (gap: Gap, content: WorkflowDisplay[], o: StackOptions = {}) => stack('Col', gap, content, o, 'Stretch', false);

/** A row that wraps when it runs out of room, its children centred on one line. `justify: 'Between'` spreads them to the ends. */
export const row = (gap: Gap, content: WorkflowDisplay[], o: StackOptions = {}) => stack('Row', gap, content, o, 'Center', true);

/** Children in `columns` equal columns, one on a phone unless `fixed`. */
export const grid = (columns: number, content: WorkflowDisplay[], o: Marked & { gap?: Gap; fixed?: boolean } = {}) =>
  block({ Grid: { columns, gap: o.gap ?? 'Loose', content, fixed: o.fixed ?? false } }, o.mark);

/** A rounded, bordered box; `tone: 'Primary'` highlights it. */
export const panel = (content: WorkflowDisplay[], o: Marked & ToneOption = {}) => block({ Panel: { tone: o.tone ?? '', content } }, o.mark);

/** An icon beside a title and a line or two; a card to open when it has an `href`. */
export const feature = (title: string, body: string, o: Marked & { icon?: string; href?: string } = {}) =>
  block({ Feature: { icon: o.icon ?? null, title, body, href: o.href ?? null } }, o.mark);

/** A small rounded label; with `act` a toggle that a `Selected` state shows as on. */
export const chip = (value: string, o: Marked & { icon?: string; act?: Act; state?: string } = {}) =>
  block(
    { Chip: { icon: o.icon ?? null, text: value, ...(o.act ? { act: o.act } : {}), ...(o.state ? { state: o.state } : {}) } },
    o.mark,
  );

/** A Lucide icon by name; `label` is read out to screen readers. */
export const icon = (name: string, size: IconSize = 'Inherit', o: Marked & ToneOption & { label?: string } = {}) =>
  block({ Icon: { lib: 'Lucide', name, aria_label: o.label ?? null, url: null, size, tone: o.tone ?? '' } }, o.mark);

/** `thumb` is a picture template shown as a small square on the right of the content. */
export const link = (href: string, style: LinkStyle, content: WorkflowDisplay[], o: Marked & { thumb?: string } = {}) =>
  block({ Link: { href, content, style, ...(o.thumb ? { thumb: o.thumb } : {}) } }, o.mark);

/** The page's main call to action. */
export const linkPrimary = (href: string, label: string, o?: Marked) => link(href, 'Primary', [body(label)], o);
export const linkSecondary = (href: string, label: string, o?: Marked) => link(href, 'Secondary', [body(label)], o);
export const linkInline = (href: string, label: string, o?: Marked) => link(href, 'Inline', [body(label)], o);
/** A whole panel that opens `href`. */
export const linkCard = (href: string, content: WorkflowDisplay[], o?: Marked & { thumb?: string }) => link(href, 'Card', content, o);
/** An entry in a list, such as a story in a feed. */
export const linkEntry = (href: string, content: WorkflowDisplay[], o?: Marked & { thumb?: string }) => link(href, 'Entry', content, o);

export const image = (src: string, alt: string, o: Marked = {}) => block({ Image: { src, alt_text: alt } }, o.mark);

/** A small label in `tone`, which may be a `{key}`; hidden while its text is empty. */
export const badge = (value: string, tone: Template<Tone> = 'Default', o: Marked & { icon?: string } = {}) =>
  block({ Badge: { variant: '', text: value, tone, icon: o.icon ?? null } }, o.mark);

/** A bar filled to the percentage at `key`. */
export const progress = (key: string, tone: Template<Tone> = 'Default', o: Marked = {}) =>
  block({ Progress: { progress_percentage_key: key, tone } }, o.mark);

export type TimedOptions = Marked & ToneOption & { role?: TextRole };

/** A countdown to the time at `until`, showing `text` with the time left as `{as}`. */
export const countdown = (until: string, as: string, value: string, o: TimedOptions & { shakeUnderMsLeft?: number } = {}) =>
  block(
    { Countdown: { until, as, text: value, shake_under_ms_left: o.shakeUnderMsLeft ?? null, role: o.role ?? 'Body', tone: o.tone ?? '' } },
    o.mark,
  );

/** The time at `key`, written the way `opts` says. */
export const time = (key: string, opts: TimeDisplayOpts = { timeago: true }, o: TimedOptions = {}) =>
  block({ Time: { opts, key, role: o.role ?? 'Fine', tone: o.tone ?? '' } }, o.mark);

/** A running total at `key` that pops up as it grows, with `icon` beside it. */
export const scoreTicker = (key: string, iconDisplay: WorkflowDisplay, o: Marked = {}) =>
  block({ ScoreTicker: { value_key: key, icon: iconDisplay } }, o.mark);

/** Floats `content` over the game canvas just before it. */
export const overlay = (corner: Corner, content: WorkflowDisplay[], o: Marked = {}) => block({ Overlay: { corner, content } }, o.mark);

/** Shows `content` while `condition` holds, adding nothing around it. */
export const when = (condition: NodeCondition, content: WorkflowDisplay[], o: Marked = {}) =>
  block({ Condition: { condition, content } }, o.mark);

/** What pressing something does: run the workflow action `action` with `inputs`, greyed out while `disabled` holds. */
export const act = (action: string, o: { inputs?: Map; disabled?: NodeCondition } = {}): Act => ({
  action,
  inputs: o.inputs ?? null,
  disabled: o.disabled ?? null,
});

export type ActionOptions = Marked & { icon?: string; fullWidth?: boolean; submit?: boolean; scrollTo?: string };

/** A button labelled `label` that runs `act`. `submit` makes it send the form it sits in. */
export const action = (label: string, look: ActionLook, run: Act, o: ActionOptions = {}) =>
  block(
    {
      Action: {
        label,
        icon: o.icon ?? null,
        look,
        act: run,
        full_width: o.fullWidth ?? false,
        submit: o.submit ?? false,
        scroll_to: o.scrollTo ?? null,
      },
    },
    o.mark,
  );

export const primaryAction = (label: string, run: Act, o?: ActionOptions) => action(label, 'Primary', run, o);
export const secondaryAction = (label: string, run: Act, o?: ActionOptions) => action(label, 'Secondary', run, o);
/** A round button showing only `icon`; `label` is read out to screen readers. */
export const iconAction = (iconName: string, label: string, run: Act, o: ActionOptions = {}) =>
  action(label, 'Icon', run, { ...o, icon: iconName });

/** Inside a form, a primary button that sends the form to the workflow action `actionId`. */
export const submit = (label: string, actionId: string, o: ActionOptions = {}) => primaryAction(label, act(actionId), { ...o, submit: true });

export type DialogOptions = Marked & {
  icon?: string;
  look?: 'Primary' | 'Secondary' | 'Ghost';
  description?: string;
  /** `Modal` for a short focused task, `Drawer` from the side to keep the page in view. Both are a bottom sheet on a phone. */
  style?: 'Modal' | 'Drawer';
  /** Closes it once a form inside has been sent and not refused. */
  closeOnSubmit?: boolean;
};

/** A button labelled `label` that opens `content` over the page, titled `title`. */
export const dialog = (label: string, title: string, content: WorkflowDisplay[], o: DialogOptions = {}) =>
  block(
    {
      Dialog: {
        label,
        icon: o.icon ?? null,
        look: o.look ?? 'Primary',
        title,
        description: o.description ?? null,
        style: o.style ?? 'Modal',
        close_on_submit: o.closeOnSubmit ?? false,
        content,
      },
    },
    o.mark,
  );

/** A notice across the page: an error, a hint, a call to sign in. */
export const notice = (tone: Template<Tone>, value: string, o: Marked & { icon?: string; title?: string; actions?: WorkflowDisplay[] } = {}) =>
  block({ Notice: { tone, icon: o.icon ?? null, title: o.title ?? null, body: value, actions: o.actions ?? [] } }, o.mark);

export const tabs = (style: TabStyle, items: Tab[], o: Marked = {}) => block({ TabBar: { style, tabs: items } }, o.mark);

/** A tab that runs `act` and shows as current while `active` holds. */
export const tab = (label: string, active: NodeCondition, run: Act, o: { icon?: string } = {}): Tab => ({
  label,
  icon: o.icon ?? null,
  active,
  act: run,
  href: null,
});

/** A tab that opens `href`. */
export const tabLink = (label: string, active: NodeCondition, href: string, o: { icon?: string } = {}): Tab => ({
  label,
  icon: o.icon ?? null,
  active,
  act: null,
  href,
});

/** A compact segmented strip of buttons, such as 1, 3 or 5 tickets; none stays current. */
export const buttonGroup = (buttons: GroupButton[], o: Marked = {}) => block({ ButtonGroup: { buttons } }, o.mark);

export const groupButton = (label: string, run: Act): GroupButton => ({ label, act: run });

/** A label between back and forward buttons, for stepping through days or weeks. */
export const stepper = (title: string, previous: Act | null, next: Act | null, o: Marked & { subtitle?: string } = {}) =>
  block({ Stepper: { title, subtitle: o.subtitle ?? null, previous, next } }, o.mark);

export type ListRowOptions = Marked &
  StateOption & { lead?: WorkflowDisplay[]; meta?: string; trail?: WorkflowDisplay[]; href?: string; act?: Act };

/** One line of a list: `lead` (a rank, an avatar), the title over `meta`, and `trail` on the right. */
export const listRow = (title: string, o: ListRowOptions = {}) =>
  block(
    {
      Row: {
        lead: o.lead ?? [],
        title,
        meta: o.meta ?? null,
        trail: o.trail ?? [],
        state: o.state ?? '',
        href: o.href ?? null,
        act: o.act ?? null,
      },
    },
    o.mark,
  );

/** A tile to pick, styled by `state`; `across` lays its content in a row. */
export const choice = (content: WorkflowDisplay[], run: Act, o: Marked & StateOption & { across?: boolean } = {}) =>
  block({ Choice: { content, state: o.state ?? '', act: run, direction: o.across ? 'Row' : 'Col' } }, o.mark);

/** A number over its label. */
export const stat = (value: string, label: string, o: Marked & ToneOption = {}) =>
  block({ Stat: { value, label, tone: o.tone ?? '' } }, o.mark);

/** A thin bar filled to `percent`, a template for a number from 0 to 100. */
export const meter = (percent: string, o: Marked & ToneOption & { label?: string; value?: string } = {}) =>
  block({ Meter: { label: o.label ?? null, value: o.value ?? null, percent, tone: o.tone ?? '' } }, o.mark);

export type ChartOptions = Marked & ChartConfig;

/** A line, bar or area chart of the rows at `itemsKey`; `x` and `y` name fields of each row. */
export const chart = (type: ChartType, itemsKey: string, x: string, y: string, o: ChartOptions = {}) => {
  const { mark, ...config } = o;
  return block({ Chart: { type, chart_data_key: itemsKey, config, x, y, axis: null } }, mark);
};

/** One segment per item of `itemsKey`, each coloured by the `state` template. */
export const segments = (itemsKey: string, as: string, state: string, o: Marked & { label?: string } = {}) =>
  block({ Segments: { items_key: itemsKey, as, state, label: o.label ?? null } }, o.mark);

export type EmptyOptions = Marked & { icon?: string; body?: string; actions?: WorkflowDisplay[] };

const emptyState = (title: string, loading: boolean, o: EmptyOptions) =>
  block({ Empty: { icon: o.icon ?? null, title, body: o.body ?? null, loading, actions: o.actions ?? [] } }, o.mark);

/** What a page shows with nothing in it yet. */
export const empty = (title: string, o: EmptyOptions = {}) => emptyState(title, false, o);

/** The pulsing line a page shows while its data loads. */
export const loading = (title: string, o: EmptyOptions = {}) => emptyState(title, true, o);

/** `avatar` is an image URL template; with none, the name's first letter stands in.
 * `user` is a user id template whose profile card pressing the person opens. */
/**
 * A person a hub's bot knows by their `m_...` id, or a template holding it. Tawks
 * draws their current name and picture. In text, write `@[m_...]`: `hub.who(id)`.
 */
export const member = (id: string, o: Marked & { size?: Size } = {}) =>
  block({ Member: { member: id, size: o.size ?? 'Medium' } }, o.mark);

export const person = (avatar: string, name: string, o: Marked & { meta?: string; size?: Size; user?: string } = {}) =>
  block({ Person: { avatar, name, meta: o.meta ?? null, size: o.size ?? 'Medium', ...(o.user ? { user: o.user } : {}) } }, o.mark);

/** Overlapping avatars for a list, with a count of the rest past `max`. */
export const avatarStack = (itemsKey: string, as: string, avatar: string, name: string, o: Marked & { max?: number } = {}) =>
  block({ AvatarStack: { items_key: itemsKey, as, avatar, name, max: o.max ?? 5 } }, o.mark);

export const divider = (o: Marked = {}) => block({ Divider: { label: null } }, o.mark);
export const labelledDivider = (label: string, o: Marked = {}) => block({ Divider: { label } }, o.mark);

/** Lines of output in a scrolling monospace box that keeps to the end. */
export const log = (value: string, o: Marked = {}) => block({ Log: { text: value } }, o.mark);

/** Media beside content when there is room, above it when not. */
export const split = (mediaContent: WorkflowDisplay[], content: WorkflowDisplay[], o: Marked = {}) =>
  block({ Split: { media: mediaContent, content } }, o.mark);

/** `content` with `side` in a column after it, on screens wide enough for both. */
export const rail = (content: WorkflowDisplay[], side: WorkflowDisplay[], o: Marked = {}) =>
  block({ Split: { media: side, content, rail: true } }, o.mark);

/** Buttons held at the foot of the page. */
export const actionBar = (content: WorkflowDisplay[], o: Marked = {}) => block({ ActionBar: { content } }, o.mark);

export type MediaOptions = Marked & {
  shape?: Shape;
  fit?: Fit;
  badge?: WorkflowDisplay[];
  caption?: WorkflowDisplay[];
  /** Washes the picture in a tone's colour. */
  tint?: Template<Tone>;
  /** `Below` puts the caption in a band beneath the picture instead of over its foot. */
  captionPlacement?: CaptionPlacement;
};

/** An image shaped by `aspect` and `size`, with an optional `badge` over a corner and a `caption` across its foot. */
export const media = (src: string, alt: string, aspect: Aspect, size: Size, o: MediaOptions = {}) =>
  block(
    {
      Media: {
        src,
        alt,
        aspect,
        size,
        shape: o.shape ?? 'Rounded',
        fit: o.fit ?? 'Cover',
        badge: o.badge ?? [],
        caption: o.caption ?? [],
        tint: o.tint ?? '',
        caption_placement: o.captionPlacement ?? 'Over',
      },
    },
    o.mark,
  );

/** A logo or avatar: square, `size`, cut round. */
export const avatar = (src: string, alt: string, size: Size, o: MediaOptions = {}) => media(src, alt, 'Square', size, { shape: 'Round', ...o });

/** A team or brand logo: square, whole, not cropped. */
export const logo = (src: string, alt: string, size: Size, o: MediaOptions = {}) =>
  media(src, alt, 'Square', size, { shape: 'Square', fit: 'Contain', ...o });

/** One copy of `content` per item of the list at `itemsKey`, each item available as `{as.field}`. */
export const items = (itemsKey: string, as: string, layout: Layout, content: WorkflowDisplay[], o: Marked = {}) =>
  block({ Items: { items_key: itemsKey, as, layout, content } }, o.mark);

export type HeadingOptions = Marked & { eyebrow?: string; lead?: string; aside?: WorkflowDisplay[] };

/** The top of a page: an eyebrow, the title with `aside` beside it, the lead and a row of `actions`. */
export const hero = (title: string, o: HeadingOptions & { actions?: WorkflowDisplay[] } = {}) =>
  block(
    { Hero: { eyebrow: o.eyebrow ?? null, title, lead: o.lead ?? null, aside: o.aside ?? [], actions: o.actions ?? [] } },
    o.mark,
  );

/** A part of a page under a rule, with its heading, a lead and `aside` across from the heading. */
export const section = (title: string, content: WorkflowDisplay[], o: HeadingOptions = {}) =>
  block({ Section: { eyebrow: o.eyebrow ?? null, title, lead: o.lead ?? null, aside: o.aside ?? [], content } }, o.mark);

export type PageOptions = {
  id?: string;
  /** Centres the content up and down, for a page that is mostly one thing. */
  center?: boolean;
  backdrop?: Backdrop;
  /** The typeface `themed` text on the page uses. */
  typeface?: Typeface;
};

/** The page column a node's content sits in. */
export const page = (width: PageWidth, content: WorkflowDisplay[], o: PageOptions = {}): WorkflowVisual => ({
  id: o.id ?? 'page',
  display: {
    Page: { width, content, center: o.center ?? false, backdrop: o.backdrop ?? 'Default', typeface: o.typeface ?? 'Default' },
  },
});

/** Sends every input inside it to `actionId`, when submitted or, with `onChange`, as people type. */
export const form = (actionId: string, content: WorkflowDisplay[], o: Marked & { onChange?: boolean } = {}) =>
  block({ Form: { server_action_id: actionId, content, submit_when: o.onChange ? 'OnChange' : 'OnSubmit' } }, o.mark);

type InputOptions = Marked & { required?: boolean; default?: JsonValue };

const input = (id: string, type: InputType, o: InputOptions) =>
  block(
    {
      Input: {
        id,
        input: { readonly: null, type, default_value: o.default ?? null, required: o.required ?? false, on_change_action: null },
      },
    },
    o.mark,
  );

/** Saved under `id` in the responses. */
export const textInput = (id: string, placeholder: string | null = null, o: InputOptions = {}) => input(id, { Text: { placeholder } }, o);

export const numberInput = (id: string, min: number | null = null, max: number | null = null, o: InputOptions = {}) =>
  input(id, { Number: { min, max } }, o);

export const switchInput = (id: string, o: InputOptions = {}) => input(id, 'Switch', o);

/** A labelled dropdown of fixed choices. Sends the chosen `value`. */
export const dropdown = (
  id: string,
  label: string,
  options: { value: string; label: string }[],
  placeholder: string | null = null,
  o: InputOptions = {},
) =>
  input(
    id,
    { Select: { filter: null, items_key: '', as: '', value: '', content: [], selected_content: null, unselected_content: null, options, option_label: null, label, placeholder } },
    o,
  );

/** A labelled dropdown with one choice per row at `itemsKey`; `value` and `optionLabel` read the row as `as`. */
export const dropdownFrom = (
  id: string,
  label: string,
  itemsKey: string,
  as: string,
  value: string,
  optionLabel: string,
  placeholder: string | null = null,
  o: InputOptions = {},
) =>
  input(
    id,
    { Select: { filter: null, items_key: itemsKey, as, value, content: [], selected_content: null, unselected_content: null, options: [], option_label: optionLabel, label, placeholder } },
    o,
  );

type NodeOptions = { parent?: string; condition?: NodeCondition; description?: string };

/** A screen. Children of a node show after it; conditions pick between them. */
export const node = (title: string, visuals: WorkflowVisual[], o: NodeOptions = {}): WorkflowNode => ({
  title,
  description: o.description ?? null,
  visuals,
  condition: o.condition ?? null,
  parent_id: o.parent ?? null,
});

const workflowAction = (id: string, action_type: WorkflowAction['action_type'], target: string | null = null): WorkflowAction => ({
  id,
  action_type,
  allow_while_loading: false,
  target,
  progress_node: true,
});

export const actions = {
  /** Calls one of your server actions by the full id `serverAction()` returned. */
  run: (id: string, serverActionId: string) => workflowAction(id, 'RunServerAction', serverActionId),
  next: (id: string, target: string | null = null) => workflowAction(id, 'NextNode', target),
  back: (id: string) => workflowAction(id, 'PreviousNode'),
  submit: (id: string) => workflowAction(id, 'Submit'),
  cancel: (id: string) => workflowAction(id, 'Cancel'),
};

export const cond = {
  equals: (field: string, value: JsonValue): NodeCondition => ({ ResponseEquals: { field, value } }),
  exists: (field: string): NodeCondition => ({ ResponseExists: field }),
  atLeast: (field: string, value: JsonValue): NodeCondition => ({ Gte: { field, value } }),
  not: (condition: NodeCondition): NodeCondition => ({ Not: condition }),
  and: (...conditions: NodeCondition[]): NodeCondition => ({ And: conditions }),
  or: (...conditions: NodeCondition[]): NodeCondition => ({ Or: conditions }),
};

type DefinitionOptions = {
  description?: string;
  responses?: Map;
  context?: Map;
  actions?: WorkflowAction[];
  /** A server action run when someone opens the page, by its full id. */
  start?: string;
};

/** A whole workflow. `id` is short, such as `"hello"`; tawks scopes it to your bot. */
export const workflow = (
  id: string,
  name: string,
  nodes: Record<string, WorkflowNode>,
  o: DefinitionOptions = {},
): CreateWorkflowDefinition => ({
  id,
  name,
  description: o.description ?? null,
  nodes,
  responses: o.responses ?? {},
  context: o.context ?? {},
  actions: o.actions ?? [],
  starting_server_action: o.start ?? null,
  multi_instance_per_user: false,
});
