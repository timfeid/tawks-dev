# Display reference

This page lists the visuals and displays you can use in a bot's workflow: what each one draws, which response keys it reads, and which actions it fires. Read [Workflows](04-workflows.md) first for templates, keys and actions.

Conventions used below:

- **Template** means a string interpolated with `{path}` placeholders. **Key** means a bare response path such as `pz_grid` or `item.cover`.
- Every display has a `class` field, the first argument to `WorkflowDisplayBuilder::new`. It is a template, so `"{w.class}"` works. Where a display has fixed styling, the entry says so.
- An **action** is the id of a `WorkflowAction` in your definition, never a server action id. That includes fields named `server_action_id` and `opened_server_action_id`.
- Inside `List`, `Table`, `Carousel` and the list inputs, the current item is in scope under the `as` name, along with every response key and `$user_id`.

## Visuals

Visuals are the top-level regions of a node, built with `WorkflowVisualBuilder::new(id)`. Only the current node's visuals are shown.

| Variant | Builder | What it does |
| --- | --- | --- |
| `Block { class, content }` | `.block(class, content)` | A column (`flex w-full flex-col` plus your `class`) holding `content`. Its HTML id is the visual id. Almost every page is one block. |
| `Sticky { position, content }` | `.sticky(position, content)` | `StickyPosition::Top` pins the content to the top of the page and `Bottom` to the bottom. `Toolbar` places it in the channel's title bar, next to the notification bell. |
| `Modal { content }` | `.modal(content)` | A dialog. Use it in a workflow you start into a modal with `.start_injected_workflow(..)` and `InjectWorkflowPlace::Modal`. `Bottom` sticky visuals in the same node appear as the dialog's footer. |
| `Tile { content }` | `.tile(content)` | A named slot. It shows `content` until a child workflow is started into it with `InjectWorkflowPlace::Tile { name, .. }`, where `name` matches the visual id. Then it shows that workflow. |
| `Notification { variant, content }` | `.notification(variant, content)` | A message box at the top of the page, also shown once per node as a toast. Variants: `success`, `info`, `warning`, `error`. |
| `Animation { id, animation_type, trigger, delay_ms, meta }` | `.animation(..)` | Plays an animation on every element matching `id`, which is a CSS selector and a template. See below. |
| `Chat { chat_id }` | `.chat(chat_id)` | A full chat area for the chat id (a template). |

### Animation

`animation_type` is one of `pulse`, `winner`, `loser`, `score_add`, `reveal`, `link`, `fog`, `sus` and `executed`. `pulse`, `winner` and `loser` work on any element; the rest were made for specific first-party games. `delay_ms` delays the start. String values in `meta` are interpolated.

An animation plays once per configuration, so re-rendering the page does not restart it. Set `meta.run_key` to a value that changes when you want it to play again, such as a round number, or set `meta.repeat` to `"always"` to play it on every update.

Target elements by the HTML id of a visual (`#my_block`) or by a marker class you add to a display (`.pz-score`).

## Layout

### Div

`Div { content }`, builder `.div(content)`. A `div` with your class around its children.

If the class contains the token `stuck-aware`, a `stuck` class is toggled on the element while it is pinned by `position: sticky`, so you can style that state with `[&.stuck]:...`.

### Condition

`Condition { condition, content }`, builder `.condition(cond, content)`. Renders `content` only when the condition holds against the current scope. With an empty class the children render with no wrapper; with a class they get a wrapping `div`.

Display conditions see responses only. Compute booleans in your bot and test them with `Cond::response_equals`. See [Conditions](04-workflows.md#conditions).

### List

`List { items_key, as, content }`, builder `.list(items_key, as, content)`.

- Reads: the array at `items_key`. Anything other than an array renders nothing.
- Renders `content` once per item inside one `div` with your class. Each item is in scope as `as`.

### Table

`Table { items_key, as, headers, row }`, builder `.table(items_key, as, headers, row)`. Build headers with `table_header(label, key, width, align)`.

- Reads: the array at `items_key`.
- Renders a horizontally scrolling `<table>` with your class. Each header shows its `label`, with `width` as its CSS width. Each item becomes a row with one cell per display in `row`, with the item in scope as `as`.

### Carousel

`Carousel { items_key, as, content, on_select_action, input_map, active_key }`. Builders: `.carousel(items_key, as, content)` for a plain swipeable strip, and `.carousel_select(items_key, as, content, action_id, input_map, active_key)` for one that reports the chosen slide.

- Reads: the array at `items_key`. With `active_key`, the first item whose `active_key` field is truthy is the starting slide, and the carousel scrolls to follow it when your bot changes it.
- Renders one slide per item. Your class goes on each slide, not on the whole carousel.
- Fires: when a swipe settles on a new slide, `on_select_action` with `input_map` values interpolated against that slide's item.

### Card

`Card { content, layout }`, builder `.card(content, layout)`. The card has its own fixed styling.

- `CardLayout::Vertical`: a plain card with `content`.
- `CardLayout::Image { image_key }`: a 16:9 image read from `image_key` above the content.
- `CardLayout::ActionImage { image_key, update_responses_key, from_input_key, action }`: the whole card is a button. Clicking reads the value at `from_input_key` and fires `action` with `{ <update_responses_key>: value }`.

### Link

`Link { href, content }`, builder `.link(href, content)`.

- `href` is a template. An `http` or `https` URL opens in a new tab, or in the system browser in the desktop app. `#top` scrolls the page back to the top. Anything else is an in-app path such as `/niche/hello/hello-channel`.
- While `href` still has an unresolved `{...}`, the content renders without a link.

### Popover

`Popover { trigger, content, opened_server_action_id, closed_server_action_id, content_class }`, builder `.popover(opened, closed, trigger, content, content_class)`.

- `trigger` is a list of displays. Your class goes on the trigger button.
- `content` is a list of visuals, usually one `Block`, rendered like a small page inside the popover. `content_class` styles the popover panel.
- Fires: `opened_server_action_id` when it opens and `closed_server_action_id` when it closes, both with empty inputs. Pass `None` for either to fire nothing. Picking an item in a `CommandPalette` inside the popover closes it.

## Text and media

### Text

`Text { text }`, builder `.text(text)`. A `div` with the interpolated text. Text is always escaped, never parsed as HTML. Renders nothing if the result is empty. `WorkflowDisplayBuilder::section_title(title)` is a shortcut for a text display with `text-lg font-semibold px-4`.

### Badge

`Badge { variant, text }`, builder `.badge(variant, text)`. A small pill with the interpolated text. Variants: `default`, `secondary`, `destructive`, `outline`, `glass`.

### Icon

`Icon { lib, name, aria_label, url }`. Builders: `.lucide_icon(name, aria_label)` for the common case, `.icon(lib, name, aria_label, url)` otherwise.

- `IconLibrary::Lucide`: `name` is a template naming a [Lucide](https://lucide.dev) icon from this set: `activity`, `arrow-left`, `arrow-right`, `arrow-up`, `audio-lines`, `badge-check`, `bell`, `brain`, `calendar`, `calendar-off`, `chat`, `check`, `check-circle-2`, `chef-hat`, `chevron-left`, `chevron-right`, `clock`, `cloud`, `compass`, `copy`, `cpu`, `crown`, `external-link`, `fast-forward`, `file-plus`, `funnel`, `game`, `gauge`, `ghost`, `gift`, `hard-drive`, `hash`, `heart`, `image`, `info`, `key-round`, `layers`, `layout-grid`, `lightbulb`, `link`, `loader`, `lock`, `memory-stick`, `message-circle`, `message-square`, `mic`, `minus`, `monitor`, `moon`, `music`, `newspaper`, `paperclip`, `pencil`, `pin`, `play`, `plus`, `popcorn`, `puzzle`, `receipt-text`, `refresh-cw`, `refrigerator`, `search`, `server`, `share-2`, `shield`, `shopping-cart`, `shuffle`, `sliders-horizontal`, `smartphone`, `smile`, `sparkles`, `star`, `terminal`, `thermometer`, `trees`, `trending-up`, `trophy`, `tv`, `type`, `user`, `user-round`, `users`, `waves`, `x`, `x-circle`. Other names draw a small placeholder square.
- `IconLibrary::Url`: an image loaded from `url`, for your own icons.

### Image

`Image { src, alt_text }`, builder `.image(src, alt_text)`. Both are templates. Renders nothing until `src` fully resolves, so a missing image simply does not appear.

### Video

`Video { src, poster_src }`, builder `.video(src, poster_src)`. Both are templates. Plays direct files (`.mp4`, `.webm`, `.mov`, `.ogg`) natively and other URLs as HLS streams. A tap toggles play, and a double tap on either side seeks ten seconds.

### AudioClip

`AudioClip { src, seconds, marks }`, builder `.audio_clip(src, seconds, marks)`. A play button and progress bar that plays only the first `seconds` of `src`. `src` and `seconds` are templates, and `seconds` must resolve to a number. `marks` are fixed points drawn along the bar. A name-the-song game can raise `seconds` with each wrong guess.

### Wysiwyg (display)

`Wysiwyg { content_key }`, builder `.wysiyg(content_key)` (note the spelling). Reads a rich text document at `content_key` and renders it read-only with `prose prose-invert` plus your class. The document is [TipTap](https://tiptap.dev) JSON (`{ "type": "doc", "content": [...] }`) using the StarterKit nodes, images and `http`/`https` links. It is the same format the rich text input produces.

### UserAvatar

`UserAvatar { key }`, builder `.user_avatar(key)`. Reads a user object (`id`, `username`, `avatar_url`) at `key`, such as a `UserResource` you copied from `$user`, and draws a 32 pixel round avatar that falls back to the first letter of the username. It has fixed styling.

### Time

`Time { key, opts }`, builder `.time(key, opts)`. Reads a date at `key`, or interpolates `key` as a template if that path is missing. Anything under a day old shows as relative time ("5 minutes ago"), older dates as "Apr 5, 7pm". `TimeDisplayOpts::timeago()` always shows relative time.

### Countdown

`Countdown { until, as, text, shake_under_ms_left }`, builder `.countdown(until, as, text, shake_under_ms_left)`.

- `until` is a template that must produce a date string.
- Every second (more often near the end), the remaining time is put in scope as `as` with the fields `days`, `hours`, `minutes`, `seconds`, `ms` and `total_seconds`, and the `text` template is rendered. Example: `.countdown("{round_ends_at}", "t", "{t.minutes}:{t.seconds}", Some(5000))`.
- Under `shake_under_ms_left` milliseconds, the text shakes and pulses.

### Progress

`Progress { progress_percentage_key, percentage_class }`, builder `.progress(key, percentage_class)`. A bar filled to the number at the key, from 0 to 100. `percentage_class` is a template applied to the filled part, so you can color it by state.

### ScoreTicker

`ScoreTicker { value_key, icon, container_class, pop_text_class, pop_container_class }`, builder `.score_ticker(value_key, lucide_icon, icon_class, container_class, pop_text_class, pop_container_class)`. Shows the number at `value_key` with a Lucide icon and plays a short pop animation when it goes up. Hidden while the key is missing.

### Loader

`Loader`, builder `.loader()`. Shows "Loading..." after a 250 ms delay, so quick loads never flash it.

### Tooltip

`Tooltip { class, tooltip_html, view_display }`, builder `.tooltip(tooltip_html, class, view_display)`. Hovering `view_display` shows a tooltip. `tooltip_html` is interpolated and inserted as HTML, so escape anything that came from users before you put it there. The display's own class goes on the trigger and the `class` field on the tooltip panel.

### ShareResult

`ShareResult { value, label }`, builder `.share_result(value, label)`. Reads an object `{ path, title, text }` at `value`, which `ShareCard::view()` produces (see [Share cards](03-bots.md#share-cards)). Draws a share button labelled `label`. On phones it opens the system share sheet; elsewhere it copies the text and a short link. Hidden while the value is missing. A non-empty class replaces the default button styling.

## Actions and inputs

### Button

`Button { content, input_map, action_id, variant, size, type }`. Builders: `.button(content, input_map, action_id, variant, size)` and `.submit(...)` with the same arguments.

- Renders `content` inside a button.
- Fires: `action_id` with `input_map`. Each value is resolved against the scope: strings are interpolated, and arrays and objects are walked recursively, so you can send structured data.
- While the action is pending, the button dims and ignores clicks until the page updates or 15 seconds pass.
- `.submit` inside a `Form` submits the form, which fires the form's action with all of its inputs. Outside a form it behaves like `.button`.
- `variant`: `default`, `destructive`, `outline`, `secondary`, `ghost`, `hoveronly`, `link`, `iconButton`. `size`: `default`, `sm`, `md`, `lg`, `icon`, `icon-lg`, `icon-sm`, `icon-xs`. Your class is merged on top, so conflicting classes in it win.
- A class token `scroll-to-<name>` scrolls the page to the element carrying `scroll-target-<name>` after the click.

### TooltipButton

`TooltipButton { tooltip_content, content, input_map, action_id, variant, size }`, builder `.tooltip_button(tooltip_content, content, input_map, action_id, variant, size)`. A button with a hover tooltip made of displays. Fires `action_id` with `input_map` values interpolated as strings. It has no pending state, so use it for actions that are safe to repeat.

### Form

`Form { server_action_id, content, submit_when }`, builder `.form(action_id, content, submit_when)`.

- Renders a `<form>` with your class around `content`, which usually holds `Input` displays and a `.submit` button.
- `SubmitWhen::OnSubmit`: pressing Enter or the submit button fires the action with every input in the form, keyed by input id. Text fields are cleared afterwards.
- `SubmitWhen::OnChange`: each change fires the action 350 ms after the last keystroke, sending only `{ <input id>: value }` for the field that changed. Useful for live search.

### Input

`Input { id, input }`, builder `.input(id, input)` with a `WorkflowInput` from `WorkflowInputBuilder`. `id` is the response key the value lands in when it is sent. The input shows the current value of `responses[id]`, and does not overwrite what the user is typing while the field has focus.

`WorkflowInput` fields:

- `type`: the `InputType`, built with `InputTypeBuilder`.
- `required`: marks text, number and text area fields as required.
- `readonly`: a condition; when it holds, the input is read-only. Every input is also read-only while a server action is pending.
- `on_change_action`: `(action_id, map)`. Supported by `Switch`, `Select`, `MultiSelectFromList` and the `Wysiwyg` input. Sends `{ <id>: new value }` plus each map entry, with map values interpolated against the scope. A key containing a dot updates a field inside an existing response object.
- `default_value`: used by the `Wysiwyg` input. For other inputs, seed the value with `.response(id, value)` on the definition or from a server action.

Input types:

| Type | Builder | How its value is sent |
| --- | --- | --- |
| `Text { placeholder }` | `InputTypeBuilder::text(..)` | Through an enclosing `Form`, as a string. |
| `TextArea { placeholder, rows }` | `InputTypeBuilder::text_area(..)` | Through a `Form`, as a string. |
| `Number { min, max }` | `InputTypeBuilder::number(..)` | Through a `Form`, as a number, or `null` when empty (`0` if required). |
| `Select { filter, items_key, as, value, content, selected_content, unselected_content, class }` | `InputTypeBuilder::select(..)` | Through a `Form` or `on_change_action`. A dropdown of the items at `items_key`. `value` is a template giving each item's value, `filter` hides items, `content` draws each option, `selected_content` the closed state and `unselected_content` the placeholder. |
| `SelectFromList { items_key, as, value, class, selected_class, content }` | `InputTypeBuilder::select_from_list(..)` | Through a `Form`. One button per item, with `selected_class` added to the chosen one. |
| `MultiSelectFromList { items_key, as, value, content, class, min, max }` | `InputTypeBuilder::multi_select_from_list(..)` | Through `on_change_action`, as an array. Check the count in your handler. |
| `Switch` | `InputTypeBuilder::switch()` | Through `on_change_action`, which it needs to do anything. |
| `Wysiwyg` | `InputTypeBuilder::wysiwyg()` | Through `on_change_action`. A rich text editor producing TipTap JSON. |
| `ServerActionLoader { target }` | `InputTypeBuilder::server_action_loader(target)` | Fires `target` once when it appears, sending all current responses, and shows "Loading data...". Handy for loading a section lazily. |

### CommandPalette

`CommandPalette { key }`, builder `.command_palette(key)`. Reads a `CommandPaletteConfig` at `key`. Build it with `CommandPaletteBuilder`, `CommandGroupBuilder` and `CommandItemBuilder`, then put `serde_json::to_value(config)` in responses.

- Renders a keyboard-navigable command list, grouped under each group's `heading`.
- Fires: picking an item fires its `action_id`, with `action_input_map` values interpolated against the item's `data` (set with `CommandItemBuilder::with_data`).
- Leave item icons unset.

For search over a large list, use a `Form` with `SubmitWhen::OnChange` around a text input and show the results with a `List`.

## First-party game canvases

The battlefield (cards), party, tracklines, remote play, FPS, dots, word wheel, snake and railyard displays, along with a few media views built for specific channels, are reserved for first-party games. Each expects data and actions that only its own game produces. To build a game of your own, compose it from the displays above as described in [Building games](06-games.md), or bring your own web app with the `activity` display described in [Game activities](07-tables.md).

## Styling

### Classes are Tailwind

Every `class` string, and every class you compute and send in responses, is a list of [Tailwind CSS](https://tailwindcss.com) v4 utility classes. The app's stylesheet ships a fixed set of utilities, so a class nobody else uses may not exist and will silently do nothing. Stick to common utilities and the theme tokens, and check new pages in the app. If a class has no effect, swap it for a nearby common one.

These are safe choices, drawn from what first-party bots use:

| Purpose | Classes |
| --- | --- |
| Layout | `flex`, `flex-col`, `flex-wrap`, `flex-1`, `grid`, `grid-cols-2` to `grid-cols-4`, `sm:grid-cols-2`, `md:flex-row`, `items-center`, `justify-between`, `justify-center`, `shrink-0`, `min-w-0`, `w-full`, `h-full`, `min-h-full`, `mx-auto`, `max-w-xl`, `max-w-2xl`, `max-w-3xl`, `aspect-square`, `aspect-video`, `overflow-hidden`, `overflow-x-auto`, `sticky`, `top-0`, `z-10` |
| Spacing | `gap-1`, `gap-2`, `gap-3`, `gap-4`, `gap-6`, `p-2`, `p-3`, `p-4`, `p-6`, `md:p-6`, `px-2`, `px-3`, `px-4`, `py-1`, `py-2`, `py-3`, `space-y-2` |
| Text | `text-xs`, `text-sm`, `text-base`, `text-lg`, `text-xl`, `text-2xl`, `text-3xl`, `text-[10px]`, `font-medium`, `font-semibold`, `font-bold`, `font-black`, `font-mono`, `uppercase`, `tracking-wide`, `tracking-wider`, `leading-tight`, `tabular-nums`, `truncate`, `text-center`, `text-right`, `whitespace-pre-wrap`, `break-words`, `line-through` |
| Color | `text-white`, `text-zinc-200`, `text-zinc-300`, `text-zinc-400`, `text-zinc-500`, `bg-zinc-900`, `bg-zinc-950`, `border-zinc-800`, `text-emerald-400`, `text-amber-300`, `text-red-400`, `text-sky-400`, `bg-emerald-500`, `bg-amber-400`, `bg-red-500` |
| Surfaces | `rounded-lg`, `rounded-xl`, `rounded-2xl`, `rounded-full`, `border`, `border-white/10`, `bg-white/[0.03]`, `bg-white/5`, `bg-white/10`, `hover:bg-white/10`, `bg-black/40`, `shadow-lg`, `divide-y`, `divide-white/10` |
| State and motion | `opacity-50`, `opacity-60`, `hover:opacity-80`, `disabled:opacity-50`, `transition`, `animate-pulse`, `animate-spin`, `animate-wiggle` |
| Rich text | `prose`, `prose-invert` |

### Theme tokens

These color tokens work with any color utility, for example `bg-primary`, `text-muted-foreground`, `border-border` or `bg-primary/10`:

| Token | Use |
| --- | --- |
| `background`, `foreground` | Page background and default text |
| `card`, `card-foreground` | Card surfaces |
| `popover`, `popover-foreground` | Popovers and menus |
| `primary`, `primary-foreground` | Brand accent and text on it |
| `secondary`, `secondary-foreground` | Secondary buttons and surfaces |
| `muted`, `muted-foreground` | Quiet surfaces and secondary text |
| `accent`, `accent-foreground` | Hover states |
| `destructive`, `destructive-foreground` | Errors and destructive actions |
| `border`, `input`, `ring` | Borders, input borders, focus rings |

tawks always uses its dark theme: a near-black `background`, light `foreground`, and a neon green `primary` (`#39FF14`) with dark `primary-foreground`.

Other theme additions:

- Fonts: the default sans stack is Noto Sans, Commissioner and Nunito Sans. `font-mono` is IBM Plex Mono.
- Breakpoint `3xl` at 120rem, on top of Tailwind's defaults.
- `animate-wiggle`, a slight rocking animation.
- The typography plugin, so `prose` and `prose-invert` work.

First-party bots mostly combine the stock palette with the theme: `text-white` and `text-zinc-400` for text, translucent surfaces such as `bg-white/[0.03]` with `border-white/10`, and `bg-primary` or `text-primary` for highlights. Following that keeps your bot visually in line with the rest of tawks.
