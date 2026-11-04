import type {
  ActionLook,
  Align,
  Aspect,
  Backdrop,
  Corner,
  Gap,
  IconSize,
  Justify,
  LinkStyle,
  PageWidth,
  Size,
  State,
  TabStyle,
  TextRole,
  TextStyle,
  Tone,
  Typeface,
} from '@tawks/workflow-types';
import { tabsLook } from '@tawks/ui/tabs/look.js';
import { interpolate } from './utils';

/**
 * How every building block looks. Workflows name what something is and this
 * is the one place that decides its classes, so every page matches. Every
 * class here is written out whole so Tailwind finds it.
 */

/** A label over its input. */
export const field = 'flex min-w-0 flex-col gap-1.5';

/** A table cell with a picture before its content. */
export const tableImageCell = 'flex min-w-0 items-center gap-2';
/** The same size in every row, with or without a picture, so columns stay put. */
export const tableImageSlot = 'size-6 shrink-0';
export const tableImage = 'size-full object-contain';

/** A dropdown's open list: long choices wrap instead of running off a phone. */
export const dropdownList = 'max-w-[calc(100vw-2rem)]';

/** What a `Dialog` holds, scrolling on its own when it is long. */
export const dialogBody = 'flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-y-auto';
export const dialogLayer = 'relative z-40';
export const dialogModal = 'px-3 py-6';
export const dialogModalBody = 'px-3';
export const dialogDrawer = 'flex flex-col';
export const dialogSheet = 'gap-3 px-4 pb-[calc(env(safe-area-inset-bottom)+1rem)]';
export const dialogSheetClose = 'absolute right-2 top-2 flex size-11 items-center justify-center rounded-md text-muted-foreground transition-colors hover:text-foreground';

export const gap: Record<Gap, string> = {
  None: 'gap-0',
  Tight: 'gap-1.5',
  Normal: 'gap-3',
  Loose: 'gap-5',
  Section: 'gap-10',
};

export const align: Record<Align, string> = {
  Stretch: 'items-stretch',
  Start: 'items-start',
  Center: 'items-center',
  End: 'items-end',
  Baseline: 'items-baseline',
};

export const justify: Record<Justify, string> = {
  Start: 'justify-start',
  Center: 'justify-center',
  End: 'justify-end',
  Between: 'justify-between',
};

type Paint = { text: string; soft: string; border: string; solid: string };

/** The theme tones and the named accents, each as text, a soft fill, a border and a solid fill. */
const paint: Record<Exclude<Tone, 'Default'>, Paint> = {
  Strong: { text: 'text-foreground', soft: 'bg-foreground/[0.05]', border: 'border-foreground/20', solid: 'bg-foreground' },
  Muted: { text: 'text-muted-foreground', soft: 'bg-foreground/[0.03]', border: 'border-foreground/10', solid: 'bg-muted-foreground' },
  Faint: { text: 'text-muted-foreground/70', soft: 'bg-foreground/[0.02]', border: 'border-foreground/5', solid: 'bg-muted-foreground/50' },
  Primary: { text: 'text-primary', soft: 'bg-primary/10', border: 'border-primary/30', solid: 'bg-primary' },
  Success: { text: 'text-emerald-400', soft: 'bg-emerald-400/10', border: 'border-emerald-400/30', solid: 'bg-emerald-400' },
  Warning: { text: 'text-amber-400', soft: 'bg-amber-400/10', border: 'border-amber-400/30', solid: 'bg-amber-400' },
  Danger: { text: 'text-red-400', soft: 'bg-red-400/10', border: 'border-red-400/30', solid: 'bg-red-400' },
  Info: { text: 'text-sky-400', soft: 'bg-sky-400/10', border: 'border-sky-400/30', solid: 'bg-sky-400' },
  Live: { text: 'text-rose-400', soft: 'bg-rose-400/10', border: 'border-rose-400/30', solid: 'bg-rose-400' },
  Red: { text: 'text-red-400', soft: 'bg-red-400/10', border: 'border-red-400/30', solid: 'bg-red-400' },
  Orange: { text: 'text-orange-400', soft: 'bg-orange-400/10', border: 'border-orange-400/30', solid: 'bg-orange-400' },
  Amber: { text: 'text-amber-400', soft: 'bg-amber-400/10', border: 'border-amber-400/30', solid: 'bg-amber-400' },
  Yellow: { text: 'text-yellow-300', soft: 'bg-yellow-300/10', border: 'border-yellow-300/30', solid: 'bg-yellow-300' },
  Lime: { text: 'text-lime-400', soft: 'bg-lime-400/10', border: 'border-lime-400/30', solid: 'bg-lime-400' },
  Green: { text: 'text-green-400', soft: 'bg-green-400/10', border: 'border-green-400/30', solid: 'bg-green-400' },
  Emerald: { text: 'text-emerald-400', soft: 'bg-emerald-400/10', border: 'border-emerald-400/30', solid: 'bg-emerald-400' },
  Teal: { text: 'text-teal-400', soft: 'bg-teal-400/10', border: 'border-teal-400/30', solid: 'bg-teal-400' },
  Cyan: { text: 'text-cyan-400', soft: 'bg-cyan-400/10', border: 'border-cyan-400/30', solid: 'bg-cyan-400' },
  Sky: { text: 'text-sky-400', soft: 'bg-sky-400/10', border: 'border-sky-400/30', solid: 'bg-sky-400' },
  Blue: { text: 'text-blue-400', soft: 'bg-blue-400/10', border: 'border-blue-400/30', solid: 'bg-blue-400' },
  Indigo: { text: 'text-indigo-400', soft: 'bg-indigo-400/10', border: 'border-indigo-400/30', solid: 'bg-indigo-400' },
  Violet: { text: 'text-violet-400', soft: 'bg-violet-400/10', border: 'border-violet-400/30', solid: 'bg-violet-400' },
  Purple: { text: 'text-purple-400', soft: 'bg-purple-400/10', border: 'border-purple-400/30', solid: 'bg-purple-400' },
  Fuchsia: { text: 'text-fuchsia-400', soft: 'bg-fuchsia-400/10', border: 'border-fuchsia-400/30', solid: 'bg-fuchsia-400' },
  Pink: { text: 'text-pink-400', soft: 'bg-pink-400/10', border: 'border-pink-400/30', solid: 'bg-pink-400' },
  Rose: { text: 'text-rose-400', soft: 'bg-rose-400/10', border: 'border-rose-400/30', solid: 'bg-rose-400' },
  Gold: { text: 'text-[#d6c49a]', soft: 'bg-[#bcaa87]/10', border: 'border-[#bcaa87]/40', solid: 'bg-[#bcaa87]' },
  Slate: { text: 'text-slate-300', soft: 'bg-slate-400/10', border: 'border-slate-400/30', solid: 'bg-slate-400' },
};

/** A tone field, which may be a `{key}` template, as a tone name; anything unknown is `Default`. */
export function toneOf(value: string | null | undefined, inputs: Record<string, unknown>): Tone {
  const name = value ? interpolate(inputs, value) : '';
  return name in paint ? (name as Tone) : 'Default';
}

export function toneText(t: Tone, fallback: Exclude<Tone, 'Default'> = 'Strong'): string {
  return paint[t === 'Default' ? fallback : t].text;
}

export function toneSoft(t: Tone, fallback: Exclude<Tone, 'Default'> = 'Muted'): string {
  const p = paint[t === 'Default' ? fallback : t];
  return `${p.soft} ${p.border}`;
}

type BadgeTone = 'default' | 'secondary' | 'success' | 'warning' | 'danger' | 'info' | 'violet';

const badgeTone: Record<Tone, BadgeTone> = {
  Default: 'secondary',
  Strong: 'secondary',
  Muted: 'secondary',
  Faint: 'secondary',
  Slate: 'secondary',
  Primary: 'default',
  Success: 'success',
  Lime: 'success',
  Green: 'success',
  Emerald: 'success',
  Teal: 'success',
  Warning: 'warning',
  Orange: 'warning',
  Amber: 'warning',
  Yellow: 'warning',
  Gold: 'warning',
  Danger: 'danger',
  Live: 'danger',
  Red: 'danger',
  Rose: 'danger',
  Pink: 'danger',
  Info: 'info',
  Cyan: 'info',
  Sky: 'info',
  Blue: 'info',
  Indigo: 'violet',
  Violet: 'violet',
  Purple: 'violet',
  Fuchsia: 'violet',
};

export function badgeVariant(t: Tone): BadgeTone {
  return badgeTone[t];
}

export function toneSolid(t: Tone, fallback: Exclude<Tone, 'Default'> = 'Primary'): string {
  return paint[t === 'Default' ? fallback : t].solid;
}

const STATES = ['Default', 'Selected', 'Highlight', 'Muted', 'Disabled', 'Success', 'Danger', 'Warning', 'Live', 'Pending'];

export function stateOf(value: string | null | undefined, inputs: Record<string, unknown>): State {
  const name = value ? interpolate(inputs, value) : '';
  return STATES.includes(name) ? (name as State) : 'Default';
}

/** A row's accent colour, or null unless it resolves to `#rgb` or `#rrggbb`. */
export function accentOf(value: string | null | undefined, inputs: Record<string, unknown>): string | null {
  const color = value ? interpolate(inputs, value).trim() : '';
  return /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i.test(color) ? color : null;
}

/** Kept faint so text holds its contrast on either theme; the edge carries the colour. */
export function accentFill(color: string): string {
  return `color-mix(in srgb, ${color} 16%, transparent)`;
}

export function accentEdge(color: string): string {
  return `inset 3px 0 0 ${color}`;
}

/** A row in a list. */
export const rowState: Record<State, string> = {
  Default: 'border-foreground/10 bg-foreground/[0.02]',
  Selected: 'border-primary/50 bg-primary/10',
  Highlight: 'border-primary/30 bg-primary/[0.06]',
  Muted: 'border-foreground/5 bg-transparent opacity-50',
  Disabled: 'border-foreground/5 bg-transparent opacity-40',
  Success: 'border-emerald-400/30 bg-emerald-400/[0.06]',
  Danger: 'border-red-400/30 bg-red-400/[0.06]',
  Warning: 'border-amber-400/30 bg-amber-400/[0.06]',
  Live: 'border-rose-400/30 bg-rose-400/[0.06]',
  Pending: 'border-dashed border-foreground/15 bg-transparent',
};

/** A tile to pick. */
export const choiceState: Record<State, string> = {
  Default: 'border-foreground/10 bg-foreground/[0.03] text-foreground hover:border-foreground/25 hover:bg-foreground/[0.06]',
  Selected: 'border-primary bg-primary/15 text-foreground ring-1 ring-primary/40',
  Highlight: 'border-primary/40 bg-primary/[0.06] text-foreground hover:bg-primary/10',
  Muted: 'border-foreground/5 bg-transparent text-muted-foreground opacity-60',
  Disabled: 'border-foreground/5 bg-transparent text-muted-foreground opacity-40 cursor-not-allowed',
  Success: 'border-emerald-400/60 bg-emerald-400/15 text-foreground',
  Danger: 'border-red-400/60 bg-red-400/15 text-foreground',
  Warning: 'border-amber-400/60 bg-amber-400/15 text-foreground',
  Live: 'border-rose-400/60 bg-rose-400/15 text-foreground',
  Pending: 'border-dashed border-foreground/20 bg-transparent text-muted-foreground',
};

/** A small segment in a strip. */
export const segmentState: Record<State, string> = {
  Default: 'bg-foreground/10',
  Selected: 'bg-primary',
  Highlight: 'bg-primary/60',
  Muted: 'bg-foreground/5',
  Disabled: 'bg-foreground/5',
  Success: 'bg-emerald-400',
  Danger: 'bg-red-400',
  Warning: 'bg-amber-400',
  Live: 'bg-rose-400 animate-pulse',
  Pending: 'bg-foreground/20',
};

const role: Record<TextRole, { cls: string; tone: Exclude<Tone, 'Default'> }> = {
  Display: { cls: 'text-3xl font-bold tracking-tight sm:text-4xl', tone: 'Strong' },
  Title: { cls: 'text-2xl font-bold tracking-tight sm:text-3xl', tone: 'Strong' },
  Heading: { cls: 'text-xl font-bold tracking-tight', tone: 'Strong' },
  Subheading: { cls: 'text-sm font-semibold', tone: 'Strong' },
  Headline: { cls: 'text-base leading-snug font-semibold sm:text-lg', tone: 'Strong' },
  Lead: { cls: 'max-w-2xl text-base leading-relaxed', tone: 'Muted' },
  Body: { cls: 'text-sm leading-relaxed', tone: 'Muted' },
  Fine: { cls: 'text-xs leading-relaxed', tone: 'Faint' },
  Eyebrow: { cls: 'text-xs font-semibold uppercase tracking-wider', tone: 'Primary' },
  Code: { cls: 'overflow-x-auto rounded-lg bg-black/30 px-3 py-2 font-mono text-sm', tone: 'Strong' },
  Stat: { cls: 'text-xl font-bold tabular-nums', tone: 'Strong' },
};

const clamp: Record<number, string> = {
  1: 'line-clamp-1',
  2: 'line-clamp-2',
  3: 'line-clamp-3',
  4: 'line-clamp-4',
  5: 'line-clamp-5',
  6: 'line-clamp-6',
};

/** The room of that many lines of the text's own line height. */
const reserve: Record<number, string> = {
  1: 'min-h-[1lh]',
  2: 'min-h-[2lh]',
  3: 'min-h-[3lh]',
  4: 'min-h-[4lh]',
  5: 'min-h-[5lh]',
  6: 'min-h-[6lh]',
};

export function text(r: TextRole, t: Tone, style?: TextStyle | null): string {
  const look = role[r];
  const parts = [look.cls, toneText(t, look.tone)];
  if (style) {
    if (style.numeric) parts.push('tabular-nums whitespace-nowrap');
    if (style.clamp) parts.push(clamp[Math.min(style.clamp, 6)], 'min-w-0');
    if (style.clamp && style.reserve) parts.push(reserve[Math.min(style.clamp, 6)]);
    if (style.align === 'Center') parts.push('text-center');
    if (style.align === 'End') parts.push('text-right');
    if (style.struck) parts.push('line-through');
    if (style.lines) parts.push('whitespace-pre-line');
    if (style.themed) parts.push('font-themed');
  }
  return parts.join(' ');
}

export function panel(t: Tone): string {
  if (t === 'Default') return 'rounded-2xl border border-foreground/10 bg-foreground/[0.03] p-4 sm:p-5';
  if (t === 'Primary') return 'rounded-2xl border border-primary/25 bg-primary/5 p-5 sm:p-7';
  return `rounded-2xl border p-4 sm:p-5 ${toneSoft(t)}`;
}

// overflow-wrap: a page's break-anywhere must not split a button's label mid-word, as in "Searc h".
const button = 'inline-flex items-center justify-center gap-2 rounded-full px-5 py-2.5 text-sm font-semibold transition-colors pointer-coarse:min-h-11 [overflow-wrap:normal]';

export const link: Record<LinkStyle, string> = {
  Plain: '',
  Inline: 'text-primary hover:underline [&_*]:text-inherit',
  Primary: `${button} bg-primary text-primary-foreground hover:opacity-90 [&_*]:text-inherit [&_*]:font-semibold`,
  Secondary: `${button} border border-foreground/20 text-foreground hover:bg-foreground/5 [&_*]:text-inherit [&_*]:font-semibold`,
  Card: 'block rounded-2xl border border-foreground/10 bg-foreground/[0.03] p-4 transition-colors hover:border-foreground/20 hover:bg-foreground/[0.06] sm:p-5',
  Entry: 'block -mx-3 rounded-xl px-3 py-3 transition-colors hover:bg-foreground/[0.04]',
};

/** A wash of colour over a picture, such as a role's alliance. */
export function tint(t: Tone): string {
  return t === 'Default' ? '' : `${paint[t].solid} mix-blend-color opacity-50`;
}

/** "Play now" for an installed game, next to a page's title. */
export const playGame = `${link.Primary} shrink-0 whitespace-nowrap`;

export const action: Record<ActionLook, string> = {
  Primary: `${button} bg-primary text-primary-foreground hover:opacity-90`,
  Secondary: `${button} border border-foreground/20 text-foreground hover:bg-foreground/5`,
  Ghost: `${button} text-muted-foreground hover:bg-foreground/5 hover:text-foreground`,
  Danger: `${button} bg-red-500/90 text-white hover:bg-red-500`,
  // The extra layer makes a 44px target on touch without changing how big it looks next to a field.
  Icon: 'relative inline-flex size-9 shrink-0 items-center justify-center rounded-full border border-foreground/15 text-foreground transition-colors hover:bg-foreground/5 pointer-coarse:after:absolute pointer-coarse:after:-inset-1',
  Link: 'inline-flex items-center gap-1.5 text-sm font-semibold text-muted-foreground transition-colors hover:text-foreground',
};

export const iconSize: Record<IconSize, string> = {
  Inherit: '',
  Small: 'size-4',
  Medium: 'size-5',
  Large: 'size-6',
};

/**
 * A page runs past the full-height box it sits in, so padding the host puts
 * under that box never reaches the page's end. The page leaves room for the
 * host's dock itself, from the height the host gives as --workflow-dock-height.
 */
export const pageEnd = 'pb-[calc(1rem+var(--workflow-dock-height,0px))] sm:pb-[calc(1.5rem+var(--workflow-dock-height,0px))]';

// Long unbroken text such as a pasted URL wraps instead of running out of its card.
const wrapAnywhere = '[overflow-wrap:anywhere]';

// shrink-0: a full-height (centred) page squeezed to its box would push its top out of scroll reach and its padding off its end.
export const page: Record<PageWidth, string> = {
  Narrow: `mx-auto flex shrink-0 w-full max-w-2xl flex-col gap-8 p-4 sm:p-6 ${wrapAnywhere}`,
  Normal: `mx-auto flex shrink-0 w-full max-w-4xl flex-col gap-8 p-4 sm:p-6 ${wrapAnywhere}`,
  Wide: `mx-auto flex shrink-0 w-full max-w-6xl flex-col gap-8 p-4 sm:p-6 ${wrapAnywhere}`,
  Full: `flex shrink-0 w-full flex-col gap-8 p-4 sm:p-6 ${wrapAnywhere}`,
};

/** The host's title above a page that has none of its own, kept close to the line under it. */
export const pageTitle = '-mb-4 sm:-mb-5';

/**
 * A page's tabs and buttons. The tabs sit centered in a row of their own above
 * the first view, the same for a page, a chat or a game, unless the host app
 * puts them in its own header. Buttons float at the bottom on a wide screen
 * and dock to the bottom edge on a phone.
 */
export const dock = {
  shell: 'pointer-events-none sticky bottom-0 z-20 flex flex-col',
  floating: 'md:pb-4',
  bar: 'pointer-events-auto flex items-center justify-end gap-2 border-t border-foreground/10 bg-background/95 px-2 pt-1 pb-2 backdrop-blur md:mx-auto md:mb-4 md:w-fit md:max-w-[calc(100%-2rem)] md:rounded-full md:border md:p-1.5 md:shadow-2xl',
  actions: 'flex min-w-0 items-center gap-2',
  /** The tabs' row, centered on the whole area so a narrow page, a wide one and a chat all put them in one place. */
  above: 'flex min-w-0 shrink-0 flex-wrap items-center justify-center gap-2 px-4 pt-4 sm:px-6 sm:pt-6',
  /** A title with its main button: side by side on a wide screen, the button under the title on a phone. */
  titleGroup: 'flex min-w-0 flex-col items-start gap-3 md:flex-row md:items-center md:gap-3 md:whitespace-nowrap',
};

/** Sets the font `themed` text uses on a page. The fonts are loaded by the app. */
export const typeface: Record<Typeface, string> = {
  Default: '',
  Condensed: "[--font-themed:'Fjalla_One',sans-serif]",
  Spooky: "[--font-themed:'Jolly_Lodger',cursive]",
};

/** Where an overlay floats over the canvas before it; on a phone it docks to the bottom. */
export const corner: Record<Corner, string> = {
  BottomLeft: 'sm:bottom-4 sm:left-4',
  BottomRight: 'sm:right-4 sm:bottom-4',
  TopLeft: 'sm:top-4 sm:left-4',
  TopRight: 'sm:top-4 sm:right-4',
  Top: 'sm:top-4 sm:left-1/2 sm:-translate-x-1/2',
  Bottom: 'sm:bottom-4 sm:left-1/2 sm:-translate-x-1/2',
};

export const backdrop: Record<Backdrop, string> = {
  Default: '',
  Dark: 'bg-zinc-950',
  Black: 'bg-black',
};

/** Columns a grid falls back to as its container narrows. */
export function columns(n: number, fixed: boolean): string {
  if (fixed) {
    const exact: Record<number, string> = {
      1: 'grid-cols-1',
      2: 'grid-cols-2',
      3: 'grid-cols-3',
      4: 'grid-cols-4',
      5: 'grid-cols-5',
      6: 'grid-cols-6',
      7: 'grid-cols-7',
      8: 'grid-cols-8',
    };
    return exact[Math.min(Math.max(n, 1), 8)];
  }
  if (n <= 1) return 'grid-cols-1';
  if (n === 2) return 'grid-cols-1 @lg:grid-cols-2';
  if (n === 3) return 'grid-cols-1 @lg:grid-cols-2 @3xl:grid-cols-3';
  if (n === 4) return 'grid-cols-2 @3xl:grid-cols-4';
  if (n === 5) return 'grid-cols-2 @lg:grid-cols-3 @2xl:grid-cols-4 @4xl:grid-cols-5';
  return 'grid-cols-4 @2xl:grid-cols-8';
}

/**
 * A row that belongs to the `Entry` link before it, such as a story's
 * reactions: as close to the entry's content as its own blocks sit (a
 * `Normal` gap, under the entry's padding), and as far from what follows as
 * the entry is.
 */
export const entryFollower = '-mt-1 pb-3';

/** The block holding an `Entry` link and its follower, so one hover covers both. */
export const entryGroup = [
  '-mx-3',
  'rounded-xl',
  'px-3',
  'transition-colors',
  'hover:bg-foreground/[0.04]',
  '[&>a]:mx-0',
  '[&>a]:px-0',
  '[&>a:hover]:bg-transparent',
];

export const chip =
  'inline-flex items-center gap-2 rounded-full border border-foreground/10 bg-foreground/[0.03] py-1.5 pr-3.5 pl-2.5 text-sm font-medium text-foreground';

/** A chip that toggles something, such as a filter: off, or on with a cross to turn it off. */
export const chipToggle = {
  off: `${chip} transition-colors hover:border-foreground/25 hover:bg-foreground/[0.06]`,
  on: 'inline-flex items-center gap-2 rounded-full border border-primary/50 bg-primary/15 py-1.5 pr-3 pl-2.5 text-sm font-medium text-foreground transition-colors hover:bg-primary/20',
};

/** A link's thumbnail, beside its content on the right. */
export const thumb = 'size-20 shrink-0 rounded-xl bg-foreground/5 object-cover sm:size-26';

export const iconTile = 'flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary';

/** Every tab bar is the app's one glass pill; `Bottom` keeps it at the foot of the page. */
export const tabs: Record<TabStyle, { bar: string; tab: string; on: string; off: string }> = {
  Bottom: { ...tabsLook, bar: `${tabsLook.bar} sticky bottom-2 z-20 mt-auto self-center` },
  Underline: { ...tabsLook, bar: `${tabsLook.bar} self-start` },
  Segmented: { ...tabsLook, bar: `${tabsLook.bar} self-start` },
};

/** A workflow's own tabs, in their row above the page or in the host's header. */
export const dockedTabs = tabsLook;

/** Quick picks in one strip, drawn like segmented tabs but with none current. */
export const buttonGroup = {
  bar: 'inline-flex gap-1 self-start rounded-full border border-foreground/10 bg-foreground/[0.03] p-1',
  button: 'flex items-center rounded-full px-3.5 py-1.5 text-xs font-semibold whitespace-nowrap text-muted-foreground transition-colors hover:bg-primary hover:text-primary-foreground',
};

/** Fixed sizes for avatars, logos and thumbnails; `Fill` takes the width it is given. */
export const mediaSize: Record<Size, string> = {
  ExtraSmall: 'size-5',
  Small: 'size-8',
  Medium: 'size-12',
  Large: 'size-20',
  ExtraLarge: 'size-28',
  Fill: 'w-full',
};

/** Sizes for things with a width but their own height: posters, popovers, carousel items. */
export const width: Record<Size, string> = {
  ExtraSmall: 'w-16',
  Small: 'w-24',
  Medium: 'w-40',
  Large: 'w-64',
  ExtraLarge: 'w-96',
  Fill: 'w-full',
};

export const aspect: Record<Aspect, string> = {
  Auto: '',
  Square: 'aspect-square',
  Video: 'aspect-video',
  Poster: 'aspect-[2/3]',
  Portrait: 'aspect-[3/4]',
  Card: 'aspect-[5/7]',
  Wide: 'aspect-[21/9]',
};

export const personSize: Record<Size, { avatar: string; name: string }> = {
  ExtraSmall: { avatar: 'size-5 text-[10px]', name: 'text-xs' },
  Small: { avatar: 'size-6 text-[11px]', name: 'text-sm' },
  Medium: { avatar: 'size-8 text-xs', name: 'text-sm' },
  Large: { avatar: 'size-12 text-base', name: 'text-base' },
  ExtraLarge: { avatar: 'size-16 text-lg', name: 'text-lg' },
  Fill: { avatar: 'size-8 text-xs', name: 'text-sm' },
};

export const popoverWidth: Record<Size, string> = {
  ExtraSmall: 'w-48',
  Small: 'w-64',
  Medium: 'w-80',
  Large: 'w-96',
  ExtraLarge: 'w-[32rem]',
  Fill: 'w-(--bits-floating-anchor-width)',
};
