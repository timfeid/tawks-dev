# Display reference

This page lists the visuals and displays you can use in a bot's workflow: what each one draws, which response keys it reads, and which actions it fires. Read [Workflows](04-workflows.md) first for templates, keys and actions.

Conventions used below:

- **Template** means a string interpolated with `{path}` placeholders. **Key** means a bare response path such as `pz_grid` or `item.cover`.
- Build pages from the [building blocks](#building-blocks) in `workflow_model::ui` (`tawks_bot::workflow::ui` in a bot). tawks styles every one of them, so your page matches every other one and keeps matching as the look changes. There are no classes to write; see [Styling](#styling).
- An **action** is the id of a `WorkflowAction` in your definition, never a server action id. That includes fields named `server_action_id` and `opened_server_action_id`.
- Inside `Items`, `List`, `Table`, `Carousel`, `Segments`, `AvatarStack` and the list inputs, the current item is in scope under the `as` name, along with every response key and `$user_id`.
- The examples show the JSON tawks receives, as `{ "display": <display> }` with any sample `responses` beside it. A display is `{ "type": { <Variant>: { .. } } }` with an optional `mark`.

## Building blocks

Each block says what something is, such as a heading, a panel or the main call to action, and tawks decides how it looks. Bring them in with `use workflow_model::ui::*;` and put them in a `Page` visual.

```rust
WorkflowVisualBuilder::new("page").page(PageWidth::Normal, vec![
    hero("Every game counts.")
        .eyebrow("NHL Legacy")
        .lead("Your rivalries. Your records. A place for every final horn.")
        .actions(vec![link_primary("/download", "Get the app"), link_secondary("/niche/hockey", "See the board")])
        .build(),
    section("What we've improved")
        .lead("All of it is live for everyone today.")
        .content(vec![grid(2, vec![
            feature("Current rosters", "All 32 teams and 823 players.").with_icon("users"),
            feature("Online again", "Ranked, Quick and Team Play.").with_icon("globe").href("/niche/hockey/play"),
        ])])
        .build(),
    panel(vec![subheading("Before you start"), body("Requires your own copy of the game.")]).tone(Tone::Primary),
]).build()
```

Every block is a plain `WorkflowDisplay`, so blocks and the lower level displays further down this page mix freely. Modifiers such as `.tone(..)` and `.meta(..)` change only the blocks they apply to and leave any other display as it was, so they chain safely.

### Pages

`WorkflowVisualBuilder::page(width, content)` is the column every channel page sits in. `PageWidth::Normal` suits most pages, `Narrow` a form, `Wide` a board or a table and `Full` a game that fills the screen.

```rust
WorkflowVisualBuilder::new("lobby")
    .page(PageWidth::Wide, content)
    .backdrop(Backdrop::Dark)
    .typeface(Typeface::Condensed)
    .build()
```

| Option | What it does |
| --- | --- |
| `.centered()` | Centres the content up and down, for a page that is mostly one thing. |
| `.backdrop(Backdrop)` | `Default` is the app's own background. `Dark` and `Black` suit a game. |
| `.typeface(Typeface)` | The font that `.themed()` text on the page uses: `Default`, `Condensed` (tall, for sports and lobbies) or `Spooky`. Other text keeps the app's font. |

### Structure

| Block | Builder | What it is |
| --- | --- | --- |
| `Hero { eyebrow, title, lead, aside, actions }` | `hero(title)` then `.eyebrow`, `.lead`, `.aside`, `.actions`, `.build()` | The top of a page. `aside` sits beside the title, for a share button; `actions` is the row of links or buttons under the lead. |
| `Section { eyebrow, title, lead, aside, content }` | `section(title)` then `.eyebrow`, `.lead`, `.aside`, `.content`, `.build()` | A part of the page under a rule, with its own heading. `aside` sits across from the heading. |
| `Stack { direction, gap, align, justify, wrap, content }` | `column(gap, content)`, `row(gap, content)` | Children in a column, or in a row that wraps with its children centred on one line. Adjust with `.align`, `.justify`, `.between()`, `.wrap()`, `.nowrap()` and `.gap`. |
| `Grid { columns, gap, content, fixed }` | `grid(columns, content)` | Equal columns, fewer as the space narrows and one on a phone. `.fixed()` keeps every column on a phone too, for boards and tile grids. |
| `Items { items_key, as, layout, content }` | `items(items_key, as, layout, content)` | `content` once per item of the array at `items_key`, laid out by `layout` (see [Lists and picks](#lists-and-picks)). |
| `Panel { tone, content }` | `panel(content)` | A rounded, bordered box. `.tone(Tone::Primary)` highlights it; `Warning` and `Danger` call attention to it. |
| `Split { media, content, rail }` | `split(media, content)`, `rail(content, side)` | Media beside content when there is room, above it when not. As a `rail`, the side column comes after the content and shows only when there is room for both, like a list of trending stories beside a feed. |
| `Backdrop { color, image, content }` | `backdrop(color, image, content)` | `content` on a band in `color` (a hex colour), with `image` large and faint in its corner and a dark wash over both. |
| `Calendar { items_key }` | `calendar(items_key)` | A month, seven days to a row under the weekday names. Each item is a day: `day`, and optionally `outside`, `today`, `href`, `image`, `label`, `detail` and `tone`. |
| `Divider { label }` | `divider()`, `labelled_divider(label)` | A rule, with an optional label in the middle. |
| `ActionBar { content }` | `action_bar(content)` | Buttons held at the foot of the page. |
| `Overlay { corner, content }` | `overlay(corner, content)` | Floats over the game canvas just before it, in one `Corner` (`TopLeft`, `TopRight`, `BottomLeft`, `BottomRight`, `Top`, `Bottom`), with the canvas still playable around it. Docks to the bottom on a phone. |
| `Condition { condition, content }` | `when(condition, content)` | Shows `content` while the condition holds, adding nothing around it. See [Condition](#condition). |

`Gap` runs `None`, `Tight`, `Normal`, `Loose` and `Section`. `Align` is `Stretch`, `Start`, `Center`, `End` or `Baseline`, and `Justify` is `Start`, `Center`, `End` or `Between`.

```display
{
  "display": {
    "type": {
      "Panel": {
        "tone": "Primary",
        "content": [
          { "type": { "Typography": { "text": "Steam Deck + Linux", "role": "Eyebrow", "tone": "" } } },
          { "type": { "Typography": { "text": "NHL Legacy, meet Steam Deck.", "role": "Title", "tone": "" } } },
          { "type": { "Typography": { "text": "A Desktop Mode installer and a Steam library shortcut.", "role": "Body", "tone": "" } } },
          { "type": { "Link": { "href": "/download", "style": "Primary", "content": [
            { "type": { "Typography": { "text": "Get the preview", "role": "Body", "tone": "" } } }
          ] } } }
        ]
      }
    }
  }
}
```

### Text

`Typography { text, role, tone, style }` is text styled by what it is. Build it with `display_title`, `title`, `heading`, `subheading`, `lead`, `body`, `fine`, `eyebrow`, `code` or `stat_text`, or with `text(role, s)`. `text` is a template, and the text is always escaped, never parsed as HTML.

The roles, largest first:

| Role | Builder | Use |
| --- | --- | --- |
| `Display` | `display_title` | The page title. `Hero` uses it. |
| `Title` | `title` | A large title inside the page, such as a highlighted panel's. |
| `Heading` | `heading` | A section heading. `Section` uses it. |
| `Subheading` | `subheading` | The title of an item, a row or a panel. |
| `Headline` | `headline` | The title of a story in a feed: larger than a subheading, smaller than a heading. |
| `Lead` | `lead` | The introduction under a title. |
| `Body` | `body` | Everything else. |
| `Fine` | `fine` | Requirements, captions and footnotes. |
| `Eyebrow` | `eyebrow` | A short uppercase label above a title. |
| `Code` | `code` | Something to copy, such as a command. |
| `Stat` | `stat_text` | A number that matters, such as a score. |

Text modifiers, which set fields of `style`:

| Modifier | Effect |
| --- | --- |
| `.tone(Tone)`, `.tone_from("{key}")` | The colour. |
| `.numeric()` | Figures of equal width, so numbers line up. |
| `.clamp(lines)` | Cuts the text off after this many lines. |
| `.centered()`, `.end_aligned()` | Centres the text, or aligns it to the end. |
| `.struck()` | Strikes the text through. |
| `.keep_lines()` | Keeps the line breaks in the text. |
| `.themed()` | Uses the page's `typeface`. |

```display
{
  "responses": {
    "score": 1240
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Col",
        "gap": "Tight",
        "content": [
          { "type": { "Typography": { "text": "Tonight", "role": "Eyebrow", "tone": "" } } },
          { "type": { "Typography": { "text": "{score}", "role": "Stat", "tone": "Primary", "style": { "numeric": true } } } },
          { "type": { "Typography": { "text": "Points this season", "role": "Fine", "tone": "Muted" } } }
        ]
      }
    }
  }
}
```

### Content

| Block | Builder | What it is |
| --- | --- | --- |
| `Feature { icon, title, body, href }` | `feature(title, body)` then `.with_icon`, `.href` | An icon beside a title and a line or two. With `.href` it becomes a card that opens the link. |
| `Chip { icon, text, act, state }` | `chip(text)` then `.with_icon`, `.on_press`, `.state` | A small rounded label. With `.on_press` it is a toggle, such as a filter; `State::Selected` shows it as on, with a cross to turn it off. |
| `Badge { text, tone, icon, variant }` | `badge(text, tone)` then `.with_icon` | A small label in a tone. `tone` may be a `{key}`. Hidden while its text is empty. |
| `Notice { tone, icon, title, body, actions }` | `notice(tone, body)` then `.titled`, `.with_icon`, `.actions` | A notice across the page: an error, a hint, a call to sign in. |
| `Empty { icon, title, body, loading, actions }` | `empty(title)`, `loading(title)` then `.titled` (the body), `.with_icon`, `.actions` | What a page shows with nothing in it yet. `loading(title)` is the pulsing line a page shows while its data loads. |
| `Stat { value, label, tone }` | `stat(value, label)` then `.tone` | A number over its label. Both are templates. |
| `Meter { label, value, percent, tone }` | `meter(percent)` then `.labelled`, `.valued`, `.tone` | A thin bar filled to `percent`, a template for a number from 0 to 100, with an optional label and value text above it. |
| `Segments { items_key, as, state, label }` | `segments(items_key, as, state)` then `.labelled` | A row of small segments, one per item, each coloured by the `state` template, such as a streak or the rounds of a match. |
| `Progress { progress_percentage_key, tone }` | `progress(key, tone)` | A bar filled to the number at the key, from 0 to 100. |
| `Countdown { until, as, text, role, tone, .. }` | `countdown(until, as, text)` then `.role`, `.tone` | Live time left. See [Countdown](#countdown). |
| `Time { key, opts, role, tone }` | `time(key, opts)` then `.role`, `.tone` | A date written out. See [Time](#time). |
| `ScoreTicker { value_key, icon }` | `score_ticker(key, icon)` | A running total that pops up as it grows. See [ScoreTicker](#scoreticker). |
| `Icon { lib, name, size, tone, .. }` | `icon(name, size)` then `.tone` | A Lucide icon, `Inherit` (the size of the text around it), `Small`, `Medium` or `Large`. See [Icon](#icon) for the names. |
| `Log { text }` | `log(text)` | Lines of output in a scrolling monospace box that keeps to the end. |

```rust
column(Gap::Normal, vec![
    notice(Tone::Warning, "The draft closes in an hour.").titled("Last call").with_icon("clock"),
    grid(3, vec![
        stat("{wins}", "Wins").tone(Tone::Success),
        stat("{losses}", "Losses"),
        stat("{streak}", "Streak"),
    ]),
    meter("{xp_pct}").labelled("Level {level}").valued("{xp} / {xp_next}"),
    segments("last_ten", "g", "{g.state}").labelled("Last ten games"),
])
```

```display
{
  "responses": {
    "wins": 12,
    "losses": 4,
    "xp_pct": 60,
    "last_five": [
      { "state": "Success" },
      { "state": "Success" },
      { "state": "Danger" },
      { "state": "Success" },
      { "state": "Pending" }
    ]
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Col",
        "gap": "Normal",
        "content": [
          { "type": { "Notice": { "tone": "Warning", "icon": "clock", "title": "Last call", "body": "The draft closes in an hour.", "actions": [] } } },
          { "type": { "Grid": { "columns": 2, "gap": "Normal", "content": [
            { "type": { "Stat": { "value": "{wins}", "label": "Wins", "tone": "Success" } } },
            { "type": { "Stat": { "value": "{losses}", "label": "Losses", "tone": "" } } }
          ] } } },
          { "type": { "Meter": { "label": "Level 3", "value": "{xp_pct}%", "percent": "{xp_pct}", "tone": "Primary" } } },
          { "type": { "Segments": { "items_key": "last_five", "as": "g", "state": "{g.state}", "label": "Last five games" } } }
        ]
      }
    }
  }
}
```

```display
{
  "display": {
    "type": {
      "Empty": {
        "icon": "trophy",
        "title": "No results yet",
        "body": "Finished games show up here.",
        "loading": false,
        "actions": [
          { "type": { "Action": { "label": "Start a game", "icon": null, "look": "Primary", "act": { "action": "start" }, "full_width": false, "submit": false, "scroll_to": null } } }
        ]
      }
    }
  }
}
```

### Lists and picks

| Block | Builder | What it is |
| --- | --- | --- |
| `Items { items_key, as, layout, content }` | `items(items_key, as, layout, content)` | Repeats `content` for each item of the array at `items_key`, with the item in scope as `as`. |
| `Row { lead, title, meta, trail, state, href, act, accent }` | `list_row(title)` then `.lead_with`, `.meta`, `.trail`, `.state`, `.href`, `.on_press`, `.accent_from` | One line of a list: a rank, avatar or icon at the start, a title over a meta line, and a value at the end. With `.href` it opens a link; with `.on_press` it runs an action. `.accent_from` washes it in a colour (see Table). |
| `Choice { content, state, act, direction }` | `choice(content, act)` then `.state`, `.across()` | A tile to pick: answers, slots, picks. Its `state` shows whether it is picked, right, wrong or used up. `.across()` lays its content out in a row. |

`Layout` decides how `Items` lays out its items:

| Layout | Effect |
| --- | --- |
| `Stack { gap }` | A column. |
| `Row { gap }` | A row that wraps. |
| `Grid { columns, gap, fixed }` | A grid, like `grid(..)`. `.fixed()` on the `Items` keeps the columns on a phone. |
| `Divided` | Rows with a rule between them, the usual choice for a list of `Row`s. |
| `Scroll { gap, rows }` | A column that shows about `rows` rows and scrolls the rest. |

```rust
items("standings", "s", Layout::Divided, vec![
    list_row("{s.name}")
        .lead_with(vec![stat_text("{s.rank}")])
        .meta("{s.record}")
        .trail(vec![stat_text("{s.points}").numeric()])
        .state_from("{s.state}"),
])

items("answers", "a", Layout::Grid { columns: 2, gap: Gap::Tight, fixed: false }, vec![
    choice(vec![body("{a.text}")], act("answer").input("answer", "{a.id}")).state_from("{a.state}"),
]).fixed()
```

Send each item's `state` from your bot, such as `"Highlight"` for the viewer's own row or `"Selected"` for their pick, and the page follows it.

```display
{
  "responses": {
    "standings": [
      { "rank": 1, "name": "Ana", "record": "4-0", "points": 12, "state": "" },
      { "rank": 2, "name": "Ben", "record": "3-1", "points": 9, "state": "Highlight" },
      { "rank": 3, "name": "Cy", "record": "1-3", "points": 5, "state": "Muted" }
    ]
  },
  "display": {
    "type": {
      "Items": {
        "items_key": "standings",
        "as": "s",
        "layout": "Divided",
        "content": [
          {
            "type": {
              "Row": {
                "lead": [{ "type": { "Typography": { "text": "{s.rank}", "role": "Stat", "tone": "" } } }],
                "title": "{s.name}",
                "meta": "{s.record}",
                "trail": [{ "type": { "Typography": { "text": "{s.points}", "role": "Stat", "tone": "", "style": { "numeric": true } } } }],
                "state": "{s.state}",
                "href": null,
                "act": null
              }
            }
          }
        ]
      }
    }
  }
}
```

```display
{
  "responses": {
    "answers": [
      { "id": "a", "text": "Toronto", "state": "" },
      { "id": "b", "text": "Montreal", "state": "Selected" },
      { "id": "c", "text": "Boston", "state": "" },
      { "id": "d", "text": "Detroit", "state": "Disabled" }
    ]
  },
  "display": {
    "type": {
      "Items": {
        "items_key": "answers",
        "as": "a",
        "layout": { "Grid": { "columns": 2, "gap": "Tight", "fixed": true } },
        "content": [
          {
            "type": {
              "Choice": {
                "content": [{ "type": { "Typography": { "text": "{a.text}", "role": "Body", "tone": "" } } }],
                "state": "{a.state}",
                "act": { "action": "answer", "inputs": { "answer": "{a.id}" } },
                "direction": "Col"
              }
            }
          }
        ]
      }
    }
  }
}
```

### Links, buttons and navigation

| Block | Builder | What it is |
| --- | --- | --- |
| `Link { href, content, style, thumb }` | `link_primary`, `link_secondary`, `link_inline`, `link_card(href, content)`, `link_entry(href, content)`, `link(href, style, content)`, then `.thumb(src)` | A link. See the styles below and [Link](#link) for what `href` may be. |
| `Action { label, icon, look, act, full_width, submit, scroll_to }` | `primary_action(label, act)`, `secondary_action(label, act)`, `icon_action(icon, label, act)`, `action(label, look, act)` | A button that runs `act`. Adjust with `.with_icon`, `.full_width()`, `.submits()` (sends the form it sits in) and `.scroll_to(id)` (scrolls to the element with that HTML id, or the display marked with that name, after the press). |
| `TabBar { style, tabs }` | `tabs(style, tabs)` with `tab(label, active, act)` and `tab_link(label, active, href)`, each with an optional `.icon(name)` | Switches between views. A tab shows as current while its `active` condition holds. `TabStyle::Bottom` sits fixed at the foot of the page; `Underline` and `Segmented` sit in place. |
| `ButtonGroup { buttons }` | `button_group(buttons)` with `group_button(label, act)` | A compact segmented strip of small buttons, each running its own `act`, for quick picks such as 1, 3 or 5 tickets. Unlike a tab bar, no button stays current. |
| `Stepper { title, subtitle, previous, next }` | `stepper(title, previous, next)` then `.meta` (the subtitle) | A label between back and forward buttons, for stepping through days or weeks. Pass `None` to leave a button out. |

Link styles: `Primary` is the page's main call to action, `Secondary` an outlined button beside it, `Inline` text in the primary colour, `Card` a whole panel that opens the link and lights up under the pointer, `Entry` an entry in a list (a story in a feed) with room above and below and no box of its own, and `Plain` adds nothing around its content.

Button looks (`ActionLook`): `Primary`, `Secondary`, `Ghost`, `Danger`, `Icon` (round, showing only its icon; the label is read out to screen readers) and `Link`.

`Act { action, inputs, disabled }` is what pressing something does. Build it with `act(action)`, add inputs with `.input(key, value)` or `.inputs(map)`, and grey the button out with `.disabled_when(condition)`. Input values are resolved against the scope: strings are interpolated, and arrays and objects are walked, so you can send structured data. While the action is pending the button ignores further presses.

```rust
column(Gap::Normal, vec![
    tabs(TabStyle::Segmented, vec![
        tab("Play", Cond::response_equals("view", json!("play")), act("view").input("view", "play")).icon("play"),
        tab("Board", Cond::response_equals("view", json!("board")), act("view").input("view", "board")),
    ]),
    stepper("{day_label}", Some(act("day").input("step", -1)), Some(act("day").input("step", 1))),
    row(Gap::Tight, vec![
        primary_action("Submit", act("submit").disabled_when(Cond::response_equals("can_submit", json!(false)))),
        secondary_action("Skip", act("skip")),
        icon_action("share-2", "Share", act("share")),
    ]),
])
```

```display
{
  "responses": {
    "view": "play",
    "day_label": "Tuesday"
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Col",
        "gap": "Normal",
        "content": [
          {
            "type": {
              "TabBar": {
                "style": "Segmented",
                "tabs": [
                  { "label": "Play", "icon": "play", "active": { "ResponseEquals": { "field": "view", "value": "play" } }, "act": { "action": "view", "inputs": { "view": "play" } }, "href": null },
                  { "label": "Board", "icon": null, "active": { "ResponseEquals": { "field": "view", "value": "board" } }, "act": { "action": "view", "inputs": { "view": "board" } }, "href": null }
                ]
              }
            }
          },
          { "type": { "Stepper": { "title": "{day_label}", "subtitle": null, "previous": { "action": "day", "inputs": { "step": -1 } }, "next": { "action": "day", "inputs": { "step": 1 } } } } },
          {
            "type": {
              "Stack": {
                "direction": "Row",
                "gap": "Tight",
                "align": "Center",
                "wrap": true,
                "content": [
                  { "type": { "Action": { "label": "Submit", "icon": null, "look": "Primary", "act": { "action": "submit" }, "full_width": false, "submit": false, "scroll_to": null } } },
                  { "type": { "Action": { "label": "Skip", "icon": null, "look": "Secondary", "act": { "action": "skip" }, "full_width": false, "submit": false, "scroll_to": null } } },
                  { "type": { "Action": { "label": "Share", "icon": "share-2", "look": "Icon", "act": { "action": "share" }, "full_width": false, "submit": false, "scroll_to": null } } }
                ]
              }
            }
          }
        ]
      }
    }
  }
}
```

### People and pictures

| Block | Builder | What it is |
| --- | --- | --- |
| `Person { avatar, name, meta, size }` | `person(avatar, name)` then `.meta`, `.size` | Someone's avatar beside their name. `avatar` is an image URL template; with none, the name's first letter stands in. |
| `AvatarStack { items_key, as, avatar, name, max }` | `avatar_stack(items_key, as, avatar, name)` then `.max` | Overlapping avatars for a list, with a count of the rest past `max` (5 by default). |
| `Media { src, alt, aspect, size, shape, fit, badge, caption, tint, caption_placement }` | `media(src, alt, aspect, size)`, `avatar(src, alt, size)`, `logo(src, alt, size)` then `.shape`, `.fit`, `.badge`, `.caption`, `.caption_below`, `.tone` | An image shaped by `aspect` and `size`. `.badge(..)` puts something over its top corner, `.caption(..)` lays content across its foot over a shade, `.caption_below(..)` puts it in a band beneath the picture so a face or scene stays clear, and `.tone(..)` washes the picture in a tone's colour. `avatar` is square and round; `logo` is square and shows the whole image uncropped. |
| `Image { src, alt_text }` | `image(src, alt)` | A plain image at its own proportions. See [Image](#image). |
| `Discussion { chat, message, reactions, comments, start, suggested, inline, title }` | `discussion(chat, message, reactions_key, comments_key)` then `.starts_with`, `.suggested`, `.inline`, `.about` | Reactions and comments on something the page shows, such as a story: its reactions with counts, the emoji that fit it (`suggested` names a list, best first; hovering shows the first and hovering that opens the rest and a picker), then a comments button with its count that opens the comments in a popout pointing at it, or with `.inline()` lays them out on the page. `.about(title)` names what they are about beside them. They are a thread on the chat message `chat` and `message` resolve to, so they arrive live, can be moderated and follow blocks. `reactions` names a list of `emoji`, `count` and `me`, and `comments` a number, as the page last knew them. While `message` is empty the first reaction or comment runs `start`, which answers by filling it in. Guests see the counts and are asked to sign in. |

`Size` runs `ExtraSmall`, `Small`, `Medium`, `Large`, `ExtraLarge` and `Fill`. `Aspect` is `Auto`, `Square`, `Video`, `Poster` (2:3), `Portrait` (3:4), `Card` (a playing card) or `Wide`. `Shape` is `Rounded`, `Round` or `Square`, `Fit` is `Cover` or `Contain`, and `CaptionPlacement` is `Over` (the default) or `Below`.

```display
{
  "responses": {
    "role": { "name": "The Witness", "art": "https://docs.tawks.app/img/docs/hoard.webp", "tone": "Violet" }
  },
  "display": {
    "type": {
      "Split": {
        "media": [
          {
            "type": {
              "Media": {
                "src": "{role.art}",
                "alt": "{role.name}",
                "aspect": "Card",
                "size": "Medium",
                "shape": "Rounded",
                "fit": "Cover",
                "badge": [{ "type": { "Badge": { "variant": "", "text": "Night", "tone": "Info" } } }],
                "caption": [{ "type": { "Typography": { "text": "{role.name}", "role": "Subheading", "tone": "" } } }],
                "tint": "{role.tone}"
              }
            }
          }
        ],
        "content": [
          { "type": { "Person": { "avatar": "", "name": "Ana", "meta": "Host", "size": "Small" } } },
          { "type": { "Typography": { "text": "You see one card each night.", "role": "Body", "tone": "" } } }
        ]
      }
    }
  }
}
```

### Modifiers

Each modifier applies to the blocks named and leaves any other display unchanged.

| Modifier | Applies to | Effect |
| --- | --- | --- |
| `.tone(Tone)`, `.tone_from(template)` | Text, icons, panels, notices, stats, meters, badges, progress, countdowns, times; media (as a tint) | The colour. |
| `.state(State)`, `.state_from(template)` | Rows, choices | How it stands against the rest. |
| `.with_icon(name)` | Features, chips, notices, empty states, badges, buttons | The icon. |
| `.href(to)` | Features, rows | Opens a link when pressed. |
| `.on_press(act)` | Rows | Runs an action when pressed. |
| `.titled(s)` | Notices (the title), empty states (the body) | Extra text. |
| `.actions(content)` | Notices, empty states | The buttons. |
| `.lead_with(content)`, `.trail(content)` | Rows | What sits at the start and the end. |
| `.meta(s)` | Rows, people, steppers | The line under the title or name. |
| `.labelled(s)` | Meters, segments | The label. |
| `.valued(s)` | Meters | The value text. |
| `.across()` | Choices | Lays the content out in a row. |
| `.size(Size)` | People, media, popovers, carousel items | The size. |
| `.shape(Shape)`, `.fit(Fit)`, `.badge(content)`, `.caption(content)`, `.caption_below(content)` | Media | See [People and pictures](#people-and-pictures). |
| `.full_width()`, `.submits()`, `.scroll_to(id)` | Buttons | See [Links, buttons and navigation](#links-buttons-and-navigation). |
| `.max(n)` | Avatar stacks | Avatars shown before the count. |
| `.numeric()`, `.clamp(n)`, `.centered()`, `.end_aligned()`, `.struck()`, `.keep_lines()`, `.themed()` | Text | See [Text](#text). |
| `.gap(Gap)` | Stacks, grids | The space between children. |
| `.fixed()` | Grids, `Items` laid out as a grid | Keeps the columns on a phone. |
| `.role(TextRole)` | Countdowns, times | The text role. |
| `.align(Align)`, `.justify(Justify)`, `.between()`, `.wrap()`, `.nowrap()` | Stacks | Alignment and wrapping. `.between()` spreads a row's children to its two ends. |
| `.mark(name)` | Any display | A name for animations. See [Marks](#marks). |

### Tones and states

Tones come from the theme. `Default` leaves the colour to the block, so a heading stays bright and body text stays muted; an empty tone is the same. `Strong`, `Muted` and `Faint` brighten or quiet text. `Primary`, `Success`, `Warning`, `Danger`, `Info` and `Live` carry meaning. The named accents `Red`, `Orange`, `Amber`, `Yellow`, `Lime`, `Green`, `Emerald`, `Teal`, `Cyan`, `Sky`, `Blue`, `Indigo`, `Violet`, `Purple`, `Fuchsia`, `Pink`, `Rose`, `Gold` and `Slate` are for things told apart by colour: puzzle groups, alliances, answers, metrics.

States say how a row, a choice, a tab or a table row stands relative to the rest:

| State | Use |
| --- | --- |
| `Default` | Nothing special. |
| `Selected` | Picked by the viewer. |
| `Highlight` | Worth noticing, such as the viewer's own row. |
| `Muted` | Set aside: used, eliminated, out of play. |
| `Disabled` | Can't be chosen right now. |
| `Success`, `Danger`, `Warning` | Right, wrong, or at risk. |
| `Live` | Happening now. |
| `Pending` | Waiting on something. |

Any field that takes a tone or a state also takes a `{key}` template, so your bot can send `"Selected"` or `"Success"` in its responses and the page picks it up: `list_row("{s.name}").state_from("{s.state}")`.

### Marks

`WorkflowDisplay` has an optional `mark`, a name an [animation](#animation) finds the display by, as `[data-mark~="name"]`. It takes `{key}` templates and several names separated by spaces, so each item of a list can carry its own:

```rust
items("players", "p", Layout::Divided, vec![list_row("{p.name}").mark("player player-{p.id}")])
```

An animation with the id `[data-mark~="player-ana"]` then plays on Ana's row.

```display
{
  "responses": {
    "winner": "ana"
  },
  "display": {
    "mark": "winner-{winner}",
    "type": { "Typography": { "text": "Ana wins the round", "role": "Title", "tone": "Success" } }
  }
}
```

## Visuals

Visuals are the top-level regions of a node, built with `WorkflowVisualBuilder::new(id)`. Only the current node's visuals are shown.

| Variant | Builder | What it does |
| --- | --- | --- |
| `Page { width, content, center, backdrop, typeface }` | `.page(width, content)` then `.centered()`, `.backdrop(..)`, `.typeface(..)` | The standard page column, holding [building blocks](#building-blocks). Its HTML id is the visual id. Almost every page is one page visual. See [Pages](#pages). |
| `Block { content }` | `.block(content)` | A plain column holding `content`, such as a card posted into a chat. Use `Page` for a channel page. |
| `Sticky { position, content }` | `.sticky(position, content)` | `StickyPosition::Top` (the page's tabs) and `Toolbar` (its buttons) go in the page's dock: a bar floating at the bottom center of a wide screen, with the tabs first and the buttons after, that becomes a row of footer buttons on a phone. Tabs in the dock take its look whatever `TabStyle` they were given. `Bottom` pins the content to the bottom of the page, above the dock. |
| `Modal { content }` | `.modal(content)` | A dialog. Use it in a workflow you start into a modal with `.start_injected_workflow(..)` and `InjectWorkflowPlace::Modal`. `Bottom` sticky visuals in the same node appear as the dialog's footer. |
| `Tile { content }` | `.tile(content)` | A named slot. It shows `content` until a child workflow is started into it with `InjectWorkflowPlace::Tile { name, .. }`, where `name` matches the visual id. Then it shows that workflow. |
| `Notification { variant, content }` | `.notification(variant, content)` | A message box at the top of the page, also shown once per node as a toast. Variants: `Success`, `Info`, `Warning`, `Error`. |
| `Animation { id, animation_type, trigger, delay_ms, meta }` | `.animation(..)` | Plays an animation on every element matching `id`, which is a CSS selector and a template. See below. |
| `Chat { chat_id }` | `.chat(chat_id)` | A full chat area for the chat id (a template). |

### Animation

`animation_type` is one of `pulse`, `winner`, `loser`, `score_add`, `reveal`, `link`, `fog`, `sus` and `executed`. `pulse`, `winner` and `loser` work on any element; the rest were made for specific first-party games. `delay_ms` delays the start. String values in `meta` are interpolated.

An animation plays once per configuration, so re-rendering the page does not restart it. Set `meta.run_key` to a value that changes when you want it to play again, such as a round number, or set `meta.repeat` to `"always"` to play it on every update.

Target elements by the HTML id of a visual (`#my_block`) or by a [mark](#marks) you put on a display (`[data-mark~="pz-score"]`).

## Lower level displays

These displays came before the building blocks. They all still work, and some, such as forms, inputs, tables and carousels, have no block that replaces them. Where a block does the same job, the section says so; prefer the block on new pages. Build these with `WorkflowDisplayBuilder::new()`, a variant method, and `.build()`.

### Layout

#### Div

`Div { content }`, builder `.div(content)`. A plain wrapper around its children with no layout of its own, kept for older bots. Use `column`, `row` or `grid` instead, which space their children for you.

#### Condition

`Condition { condition, content }`, builders `when(condition, content)` and `.condition(cond, content)`. Renders `content` only when the condition holds against the current scope, with no wrapper of its own.

Display conditions see responses only. Compute booleans in your bot and test them with `Cond::response_equals`. See [Conditions](04-workflows.md#conditions).

```display
{
  "responses": {
    "finished": true,
    "score": 7
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Col",
        "gap": "Tight",
        "content": [
          {
            "type": {
              "Condition": {
                "condition": { "ResponseEquals": { "field": "finished", "value": true } },
                "content": [
                  { "type": { "Typography": { "text": "You finished with {score} points.", "role": "Body", "tone": "Success" } } }
                ]
              }
            }
          },
          {
            "type": {
              "Condition": {
                "condition": { "Not": { "ResponseEquals": { "field": "finished", "value": true } } },
                "content": [
                  { "type": { "Typography": { "text": "Keep going!", "role": "Body", "tone": "" } } }
                ]
              }
            }
          }
        ]
      }
    }
  }
}
```

#### List

`List { items_key, as, content }`, builder `.list(items_key, as, content)`. Lower level than `Items`, which also chooses the layout.

- Reads: the array at `items_key`. Anything other than an array renders nothing.
- Renders `content` once per item in a column with a small gap. Each item is in scope as `as`.

```display
{
  "responses": {
    "players": [
      { "name": "Ana", "score": 12 },
      { "name": "Ben", "score": 9 },
      { "name": "Cy", "score": 5 }
    ]
  },
  "display": {
    "type": {
      "List": {
        "items_key": "players",
        "as": "p",
        "content": [
          {
            "type": {
              "Row": {
                "lead": [],
                "title": "{p.name}",
                "meta": null,
                "trail": [{ "type": { "Typography": { "text": "{p.score} pts", "role": "Body", "tone": "Muted" } } }],
                "state": "",
                "href": null,
                "act": null
              }
            }
          }
        ]
      }
    }
  }
}
```

#### Table

`Table { items_key, as, headers, row, row_state, row_accent, section_key, section }`, builder `.table(items_key, as, headers, row)`, then `.accent_from(..)` for `row_accent` and `.sections(key, children)` for `section_key` and `section`. Build headers with `table_header(label, key, width, align)`.

- Reads: the array at `items_key`.
- Renders a horizontally scrolling table. Each header shows its `label`, with `width` as its CSS width. A header with `numeric: true` lines its figures up and aligns them right, and one with `wide_only: true` leaves its column out on narrow screens.
- Each item becomes a row with one cell per display in `row`, with the item in scope as `as`. `row_state` is a `{key}` template naming each row's [state](#tones-and-states), such as `"{p.state}"` to highlight the viewer's row.
- `row_accent` washes each row in a colour, such as the team the viewer follows: `#rgb`, `#rrggbb`, or a `{key}` template like `"{p.accent}"` naming one per item. The page draws a faint wash of it with a bar along the row's start, so text stays readable. Anything else is refused when the workflow is registered, and a template that resolves to something other than a hex colour, or to nothing, leaves the row plain.
- `section_key` splits one table into sections so its columns line up across them. It is a `{key}` template per item, like `"{p.section}"`. An item it fills in draws the `section` displays once across every column instead of the row's cells. Items it leaves empty, or that lack the key, are ordinary rows.

```display
{
  "responses": {
    "players": [
      { "name": "Ana", "score": 12, "wins": 4, "state": "", "accent": "#CE1126" },
      { "name": "Ben", "score": 9, "wins": 3, "state": "Highlight" },
      { "name": "Cy", "score": 5, "wins": 1, "state": "" }
    ]
  },
  "display": {
    "type": {
      "Table": {
        "items_key": "players",
        "as": "p",
        "headers": [
          { "label": "Player", "key": "name", "width": null, "align": null },
          { "label": "Wins", "key": "wins", "width": "5rem", "align": "right", "numeric": true, "wide_only": true },
          { "label": "Score", "key": "score", "width": "5rem", "align": "right", "numeric": true }
        ],
        "row": [
          { "type": { "Typography": { "text": "{p.name}", "role": "Body", "tone": "" } } },
          { "type": { "Typography": { "text": "{p.wins}", "role": "Body", "tone": "Muted" } } },
          { "type": { "Typography": { "text": "{p.score}", "role": "Body", "tone": "Strong" } } }
        ],
        "row_state": "{p.state}",
        "row_accent": "{p.accent}"
      }
    }
  }
}
```

#### Carousel

`Carousel { items_key, as, content, on_select_action, input_map, active_key, item_size }`. Builders: `.carousel(items_key, as, content)` for a plain swipeable strip, and `.carousel_select(items_key, as, content, action_id, input_map, active_key)` for one that reports the chosen slide.

- Reads: the array at `items_key`. With `active_key`, the first item whose `active_key` field is truthy is the starting slide, and the carousel scrolls to follow it when your bot changes it.
- Renders one slide per item. `item_size`, set with `.size(..)`, is each slide's width.
- Fires: when a swipe settles on a new slide, `on_select_action` with `input_map` values interpolated against that slide's item.

#### Card

`Card { content, layout }`, builder `.card(content, layout)`. The card has its own fixed styling. For a new page, `panel(..)` or `link_card(..)` usually fits better.

- `CardLayout::Vertical`: a plain card with `content`.
- `CardLayout::Image { image_key }`: a 16:9 image read from `image_key` above the content.
- `CardLayout::ActionImage { image_key, update_responses_key, from_input_key, action }`: the whole card is a button. Clicking reads the value at `from_input_key` and fires `action` with `{ <update_responses_key>: value }`.

```display
{
  "display": {
    "type": {
      "Card": {
        "content": [
          { "type": { "Typography": { "text": "Daily puzzle #37", "role": "Subheading", "tone": "" } } },
          { "type": { "Typography": { "text": "Find four groups of four.", "role": "Body", "tone": "" } } }
        ],
        "layout": "Vertical"
      }
    }
  }
}
```

#### Link

`Link { href, content, style }`. Builders: the `link_*` blocks above, or `.link(href, content)` for a `Plain` link.

- `href` is a template. An `http` or `https` URL opens in a new tab, or in the system browser in the desktop app. `#top` scrolls the page back to the top. Anything else is an in-app path such as `/niche/hello/hello-channel`.
- While `href` still has an unresolved `{...}`, the content renders without a link.
- `style` is `Plain` (the default, adding nothing around the content), `Inline`, `Primary`, `Secondary`, `Card` or `Entry`.
- `thumb`, when set, is a picture template shown as a small square on the right of the content, and left out while it resolves to nothing.

```display
{
  "display": {
    "type": {
      "Link": {
        "href": "https://tawks.app",
        "style": "Inline",
        "content": [
          { "type": { "Typography": { "text": "Open tawks", "role": "Body", "tone": "" } } }
        ]
      }
    }
  }
}
```

#### Popover

`Popover { trigger, content, opened_server_action_id, closed_server_action_id, width }`, builder `.popover(opened, closed, trigger, content)`.

- `trigger` is a list of displays, drawn inside the button that opens the popover.
- `content` is a list of visuals, usually one `Block`, rendered like a small page inside the popover. `width` is a `Size`, `Medium` by default; set it with `.size(..)`.
- Fires: `opened_server_action_id` when it opens and `closed_server_action_id` when it closes, both with empty inputs. Pass `None` for either to fire nothing. Picking an item in a `CommandPalette` inside the popover closes it.

### Text and media

#### Text

`Text { text }`, builder `.text(text)`. The interpolated text with no role, kept for older bots. Use the [text blocks](#text) instead, which size and colour the text by what it is. Text is always escaped, never parsed as HTML. Renders nothing if the result is empty.

```display
{
  "responses": {
    "name": "Ada"
  },
  "display": {
    "type": {
      "Text": {
        "text": "Hello, {name}!"
      }
    }
  }
}
```

#### Badge

`Badge { variant, text, tone, icon }`. Builders: `badge(text, tone)` from the blocks, or the older `.badge(variant, text)`. A small label with the interpolated text, hidden while the text is empty. With a `tone`, the badge is drawn in that tone and `variant` is ignored. Without one, `variant` picks the older look: `default`, `secondary`, `destructive`, `outline` or `glass`.

```display
{
  "display": {
    "type": {
      "Stack": {
        "direction": "Row",
        "gap": "Tight",
        "wrap": true,
        "content": [
          { "type": { "Badge": { "variant": "", "text": "Live", "tone": "Live" } } },
          { "type": { "Badge": { "variant": "", "text": "Won", "tone": "Success", "icon": "trophy" } } },
          { "type": { "Badge": { "variant": "", "text": "Week 3", "tone": "Muted" } } },
          { "type": { "Badge": { "variant": "outline", "text": "outline" } } }
        ]
      }
    }
  }
}
```

#### Icon

`Icon { lib, name, aria_label, url, size, tone }`. Builders: `icon(name, size)` from the blocks, `.lucide_icon(name, aria_label)`, or `.icon(lib, name, aria_label, url)` for the other libraries.

- `IconLibrary::Lucide`: `name` is a template naming a [Lucide](https://lucide.dev) icon from this set: `activity`, `arrow-left`, `arrow-right`, `arrow-up`, `audio-lines`, `badge-check`, `bell`, `brain`, `calendar`, `calendar-off`, `chat`, `check`, `check-circle-2`, `chef-hat`, `chevron-left`, `chevron-right`, `clock`, `cloud`, `compass`, `copy`, `cpu`, `crown`, `external-link`, `fast-forward`, `file-plus`, `funnel`, `game`, `gauge`, `ghost`, `gift`, `hard-drive`, `hash`, `heart`, `image`, `info`, `key-round`, `layers`, `layout-grid`, `lightbulb`, `link`, `loader`, `lock`, `memory-stick`, `message-circle`, `message-square`, `mic`, `minus`, `monitor`, `moon`, `music`, `newspaper`, `paperclip`, `pencil`, `pin`, `play`, `plus`, `popcorn`, `puzzle`, `receipt-text`, `refresh-cw`, `refrigerator`, `search`, `server`, `share-2`, `shield`, `shopping-cart`, `shuffle`, `sliders-horizontal`, `smartphone`, `smile`, `sparkles`, `star`, `terminal`, `thermometer`, `trees`, `trending-up`, `trophy`, `tv`, `type`, `user`, `user-round`, `users`, `waves`, `x`, `x-circle`. Other names draw a small placeholder square.
- `IconLibrary::Url`: an image loaded from `url`, for your own icons.
- `size` is `Inherit` (the size of the text around it, the default), `Small`, `Medium` or `Large`. `tone` colours it.

```display
{
  "display": {
    "type": {
      "Stack": {
        "direction": "Row",
        "gap": "Normal",
        "content": [
          { "type": { "Icon": { "lib": "Lucide", "name": "trophy", "aria_label": "trophy", "url": null, "size": "Medium", "tone": "Gold" } } },
          { "type": { "Icon": { "lib": "Lucide", "name": "star", "aria_label": "star", "url": null, "size": "Medium", "tone": "Primary" } } },
          { "type": { "Icon": { "lib": "Lucide", "name": "heart", "aria_label": "heart", "url": null, "size": "Medium", "tone": "Rose" } } },
          { "type": { "Icon": { "lib": "Lucide", "name": "puzzle", "aria_label": "puzzle", "url": null, "size": "Medium", "tone": "Muted" } } },
          { "type": { "Icon": { "lib": "Lucide", "name": "music", "aria_label": "music", "url": null, "size": "Medium", "tone": "Muted" } } },
          { "type": { "Icon": { "lib": "Lucide", "name": "users", "aria_label": "users", "url": null, "size": "Medium", "tone": "Muted" } } }
        ]
      }
    }
  }
}
```

#### Image

`Image { src, alt_text }`, builders `image(src, alt)` and `.image(src, alt_text)`. Both are templates. Renders nothing until `src` fully resolves, so a missing image simply does not appear. For a picture with a set shape, size or caption, use `media(..)`.

```display
{
  "display": {
    "type": {
      "Image": {
        "src": "https://docs.tawks.app/img/docs/hoard.webp",
        "alt_text": "Hoard running in devhost"
      }
    }
  }
}
```

#### Video

`Video { src, poster_src }`, builder `.video(src, poster_src)`. Both are templates. Plays direct files (`.mp4`, `.webm`, `.mov`, `.ogg`) natively and other URLs as HLS streams. A tap toggles play, and a double tap on either side seeks ten seconds.

#### AudioClip

`AudioClip { src, seconds, marks }`, builder `.audio_clip(src, seconds, marks)`. A play button and progress bar that plays only the first `seconds` of `src`. `src` and `seconds` are templates, and `seconds` must resolve to a number. `marks` are fixed points drawn along the bar. A name-the-song game can raise `seconds` with each wrong guess.

#### Wysiwyg (display)

`Wysiwyg { content_key }`, builder `.wysiyg(content_key)` (note the spelling). Reads a rich text document at `content_key` and renders it read-only as formatted text. The document is [TipTap](https://tiptap.dev) JSON (`{ "type": "doc", "content": [...] }`) using the StarterKit nodes, images and `http`/`https` links. It is the same format the rich text input produces.

#### UserAvatar

`UserAvatar { key }`, builder `.user_avatar(key)`. Reads a user object (`id`, `username`, `avatar_url`) at `key`, such as a `UserResource` you copied from `$user`, and draws a 32 pixel round avatar that falls back to the first letter of the username. `person(avatar, name)` does the same from two templates and adds the name beside it.

```display
{
  "responses": {
    "player": {
      "id": "u1",
      "username": "ana",
      "avatar_url": ""
    }
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Row",
        "gap": "Tight",
        "align": "Center",
        "content": [
          { "type": { "UserAvatar": { "key": "player" } } },
          { "type": { "Typography": { "text": "{player.username}", "role": "Body", "tone": "" } } }
        ]
      }
    }
  }
}
```

#### Time

`Time { key, opts, role, tone }`, builders `time(key, opts)` and `.time(key, opts)`. Reads a date at `key`, or interpolates `key` as a template if that path is missing. Anything under a day old shows as relative time ("5 minutes ago"), older dates as "Apr 5, 7pm". `TimeDisplayOpts::timeago()` always shows relative time. `role` and `tone` style it like text; the block defaults to `Fine`.

```display
{
  "responses": {
    "posted_at": "2026-10-20T19:00:00Z"
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Row",
        "gap": "Tight",
        "content": [
          { "type": { "Typography": { "text": "Posted", "role": "Fine", "tone": "" } } },
          { "type": { "Time": { "key": "posted_at", "opts": { "timeago": false }, "role": "Fine", "tone": "" } } }
        ]
      }
    }
  }
}
```

#### Countdown

`Countdown { until, as, text, shake_under_ms_left, role, tone }`. Builders: `countdown(until, as, text)` from the blocks, then `.role` and `.tone`, or `.countdown(until, as, text, shake_under_ms_left)`.

- `until` is a template that must produce a date string.
- Every second (more often near the end), the remaining time is put in scope as `as` with the fields `days`, `hours`, `minutes`, `seconds`, `ms` and `total_seconds`, and the `text` template is rendered. Example: `countdown("{round_ends_at}", "t", "{t.minutes}:{t.seconds}").role(TextRole::Stat)`.
- Under `shake_under_ms_left` milliseconds, the text shakes and pulses.

```display
{
  "responses": {
    "season_ends_at": "2027-01-01T00:00:00Z"
  },
  "display": {
    "type": {
      "Countdown": {
        "until": "{season_ends_at}",
        "as": "t",
        "text": "{t.days}d {t.hours}h {t.minutes}m {t.seconds}s",
        "shake_under_ms_left": 5000,
        "role": "Stat",
        "tone": "Primary"
      }
    }
  }
}
```

#### Progress

`Progress { progress_percentage_key, tone }`, builders `progress(key, tone)` and `.progress(key)`. A bar filled to the number at the key, from 0 to 100. `tone` may be a `{key}`, so you can colour it by state. `meter(..)` is the slimmer bar with a label and value.

```display
{
  "responses": {
    "done_pct": 65
  },
  "display": {
    "type": {
      "Progress": {
        "progress_percentage_key": "done_pct",
        "tone": "Primary"
      }
    }
  }
}
```

#### ScoreTicker

`ScoreTicker { value_key, icon }`, builder `score_ticker(value_key, icon)`. Shows the number at `value_key` beside `icon`, any display, and plays a short pop animation when it goes up. Hidden while the key is missing.

```display
{
  "responses": {
    "score": 42
  },
  "display": {
    "type": {
      "ScoreTicker": {
        "value_key": "score",
        "icon": {
          "type": { "Icon": { "lib": "Lucide", "name": "trophy", "aria_label": "trophy", "url": null, "size": "Small", "tone": "Gold" } }
        }
      }
    }
  }
}
```

#### Loader

`Loader`, builder `.loader()`. Shows the ghost loader after a half second delay, so quick loads never flash it. `loading(title)` is the block for a page waiting on its data.

#### Tooltip

`Tooltip { tooltip_html, view_display }`, builder `.tooltip(tooltip_html, view_display)`. Hovering `view_display` shows a tooltip. `tooltip_html` is interpolated and inserted as HTML, so escape anything that came from users before you put it there.

```display
{
  "display": {
    "type": {
      "Tooltip": {
        "tooltip_html": "<b>4</b> wins this week",
        "view_display": [
          { "type": { "Typography": { "text": "Hover for details", "role": "Fine", "tone": "" } } }
        ]
      }
    }
  }
}
```

#### ShareResult

`ShareResult { value, label }`, builder `.share_result(value, label)`. Reads an object `{ path, title, text }` at `value`, which `ShareCard::view()` produces (see [Share cards](03-bots.md#share-cards)). Draws a share button labelled `label`. On phones it opens the system share sheet; elsewhere it copies the text and a short link. Hidden while the value is missing.

### Actions and inputs

#### Button

`Button { content, input_map, action_id, variant, size, type }`. Builders: `.button(content, input_map, action_id, variant, size)` and `.submit(...)` with the same arguments. The `Action` block (`primary_action` and the rest) replaces it on new pages.

- Renders `content` inside a button.
- Fires: `action_id` with `input_map`. Each value is resolved against the scope: strings are interpolated, and arrays and objects are walked recursively, so you can send structured data.
- While the action is pending, the button dims and ignores clicks until the page updates or 15 seconds pass.
- `.submit` inside a `Form` submits the form, which fires the form's action with all of its inputs. Outside a form it behaves like `.button`.
- `variant`: `default`, `destructive`, `outline`, `secondary`, `ghost`, `hoveronly`, `link`, `iconButton`. `size`: `default`, `sm`, `md`, `lg`, `icon`, `icon-lg`, `icon-sm`, `icon-xs`.

```display
{
  "display": {
    "type": {
      "Stack": {
        "direction": "Row",
        "gap": "Tight",
        "wrap": true,
        "content": [
          {
            "type": {
              "Button": {
                "content": [{ "type": { "Text": { "text": "Default" } } }],
                "input_map": null,
                "action_id": "example",
                "variant": "default",
                "size": "default",
                "type": "button"
              }
            }
          },
          {
            "type": {
              "Button": {
                "content": [{ "type": { "Text": { "text": "Outline" } } }],
                "input_map": { "choice": "outline" },
                "action_id": "example",
                "variant": "outline",
                "size": "default",
                "type": "button"
              }
            }
          },
          {
            "type": {
              "Button": {
                "content": [{ "type": { "Text": { "text": "Delete" } } }],
                "input_map": null,
                "action_id": "example",
                "variant": "destructive",
                "size": "sm",
                "type": "button"
              }
            }
          }
        ]
      }
    }
  }
}
```

#### TooltipButton

`TooltipButton { tooltip_content, content, input_map, action_id, variant, size }`, builder `.tooltip_button(tooltip_content, content, input_map, action_id, variant, size)`. A button with a hover tooltip made of displays. Fires `action_id` with `input_map` values interpolated as strings. It has no pending state, so use it for actions that are safe to repeat.

#### Form

`Form { server_action_id, content, submit_when }`, builder `.form(action_id, content, submit_when)`.

- Renders a form around `content`, which usually holds `Input` displays and a button that submits it: `primary_action(label, act(action_id)).submits()`, or the older `.submit(..)`.
- `SubmitWhen::OnSubmit`: pressing Enter or the submit button fires the action with every input in the form, keyed by input id. Text fields are cleared afterwards.
- `SubmitWhen::OnChange`: each change fires the action 350 ms after the last keystroke, sending only `{ <input id>: value }` for the field that changed. Useful for live search.

```display
{
  "responses": {
    "name": "there"
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Col",
        "gap": "Normal",
        "content": [
          { "type": { "Typography": { "text": "Hello, {name}!", "role": "Heading", "tone": "" } } },
          {
            "type": {
              "Form": {
                "server_action_id": "wave",
                "content": [
                  {
                    "type": {
                      "Input": {
                        "id": "name",
                        "input": {
                          "readonly": null,
                          "type": { "Text": { "placeholder": "Your name" } },
                          "default_value": null,
                          "required": false,
                          "on_change_action": null
                        }
                      }
                    }
                  },
                  { "type": { "Action": { "label": "Wave", "icon": null, "look": "Primary", "act": { "action": "wave" }, "full_width": false, "submit": true, "scroll_to": null } } }
                ],
                "submit_when": "OnSubmit"
              }
            }
          }
        ]
      }
    }
  }
}
```

#### Input

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
| `Select { filter, items_key, as, value, content, selected_content, unselected_content }` | `InputTypeBuilder::select(..)` | Through a `Form` or `on_change_action`. A dropdown of the items at `items_key`. `value` is a template giving each item's value, `filter` hides items, `content` draws each option, `selected_content` the closed state and `unselected_content` the placeholder. |
| `SelectFromList { items_key, as, value, content, columns }` | `InputTypeBuilder::select_from_list(items_key, as, value, content, columns)` | Through a `Form`. Each item is drawn as a tile holding `content`, and the chosen one shows as `Selected`. `columns` lays the tiles out in that many columns; `0` stacks them. |
| `MultiSelectFromList { items_key, as, value, content, columns, min, max }` | `InputTypeBuilder::multi_select_from_list(items_key, as, value, content, columns, min, max)` | Through `on_change_action`, as an array. Tiles as in `SelectFromList`, with every chosen one `Selected`. Check the count in your handler. |
| `Switch` | `InputTypeBuilder::switch()` | Through `on_change_action`, which it needs to do anything. |
| `Wysiwyg` | `InputTypeBuilder::wysiwyg()` | Through `on_change_action`. A rich text editor producing TipTap JSON. |
| `ServerActionLoader { target }` | `InputTypeBuilder::server_action_loader(target)` | Fires `target` once when it appears, sending all current responses, and shows "Loading data...". Handy for loading a section lazily. |

```display
{
  "responses": {
    "guess": 42,
    "fruits": [
      { "id": "apple", "name": "Apple" },
      { "id": "pear", "name": "Pear" },
      { "id": "plum", "name": "Plum" }
    ],
    "pick": "pear"
  },
  "display": {
    "type": {
      "Stack": {
        "direction": "Col",
        "gap": "Normal",
        "content": [
          {
            "type": {
              "Input": {
                "id": "note",
                "input": {
                  "readonly": null,
                  "type": { "Text": { "placeholder": "A text input" } },
                  "default_value": null,
                  "required": false,
                  "on_change_action": null
                }
              }
            }
          },
          {
            "type": {
              "Input": {
                "id": "guess",
                "input": {
                  "readonly": null,
                  "type": { "Number": { "min": 1, "max": 100 } },
                  "default_value": null,
                  "required": false,
                  "on_change_action": null
                }
              }
            }
          },
          {
            "type": {
              "Input": {
                "id": "pick",
                "input": {
                  "readonly": null,
                  "type": {
                    "SelectFromList": {
                      "items_key": "fruits",
                      "as": "f",
                      "value": "{f.id}",
                      "columns": 3,
                      "content": [
                        { "type": { "Typography": { "text": "{f.name}", "role": "Body", "tone": "" } } }
                      ]
                    }
                  },
                  "default_value": null,
                  "required": false,
                  "on_change_action": null
                }
              }
            }
          }
        ]
      }
    }
  }
}
```

#### CommandPalette

`CommandPalette { key }`, builder `.command_palette(key)`. Reads a `CommandPaletteConfig` at `key`. Build it with `CommandPaletteBuilder`, `CommandGroupBuilder` and `CommandItemBuilder`, then put `serde_json::to_value(config)` in responses.

- Renders a keyboard-navigable command list, grouped under each group's `heading`.
- Fires: picking an item fires its `action_id`, with `action_input_map` values interpolated against the item's `data` (set with `CommandItemBuilder::with_data`).
- Leave item icons unset.

For search over a large list, use a `Form` with `SubmitWhen::OnChange` around a text input and show the results with `Items`.

## First-party game canvases

The battlefield (cards), party, tracklines, remote play, FPS, dots, word wheel, snake and railyard displays, along with a few media views built for specific channels, are reserved for first-party games. Each expects data and actions that only its own game produces. To build a game of your own, compose it from the displays above as described in [Building games](06-games.md), or bring your own web app with the `activity` display described in [Game activities](07-tables.md). An `overlay(..)` placed right after a canvas floats over it.

## Styling

tawks styles every display itself, the same way everywhere, so a page says what something is and never how it looks. There are no classes and no CSS. Pick the block that fits, then use [tones and states](#tones-and-states), text roles, `Gap`, `Size` and the other options above for emphasis and spacing. A page built this way matches the rest of the app and keeps matching as the look changes.

Older bots that still send a `class` field on their displays keep working: tawks ignores the class and draws the display as it would without one.
