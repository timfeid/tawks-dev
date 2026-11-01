# PuckLab bot

This is an independent Tawks bot backed by PuckLab's existing service. It does
not own an inventory, currency ledger or gameplay simulation. The linked
PuckLab account is authoritative for both clients.

The channel workflow, `pucklab_my_club`, is PuckLab Online: the leaderboards,
a short read-only summary of your own standing, and the channel's chat. It
edits nothing. Clubs, cards, lineups, skaters, jerseys and rosters are built
and changed in PuckLab itself, and the page links there.
A Play now button beside the title starts PuckLab in the Tawks desktop app
when it is installed there, and opens the PuckLab site (`PUCKLAB_SITE_URL`)
everywhere else.

| Tab | What it shows |
| --- | --- |
| Leaderboards | Online Ranked, Linked Career and Lab Rivals, per roster or league. Each name opens the player's Tawks profile when their account is linked; the arrow beside it opens their PuckLab page (`/skaters/<account>`, or `/clubs/<roster>/<account>` for a Lab club). |
| My stats | Your rows from those same boards (rating and tier, Career points, Lab Rivals record and club), with a "Full stats and club on PuckLab" link to your PuckLab page. One `GET /v1/account` per open finds your account; nothing else is read. Without a link it says how to link. |
| Chat | The channel's own chat (chat id `hockey-pucklab`), the way the NHL team hubs show theirs. Migration 0159 gives the channel its chats row. |

## Leaderboards

The workflow opens on PuckLab Online's Leaderboards tab: Online Ranked, Linked
Career and Lab Rivals, each per roster or league. The bot reads them once a
minute for everyone and pushes changes to open pages, so viewers (guests too)
never call PuckLab. All boards and rosters travel with the page; a tab or
roster pick only sets `pl_board` or `plb_<board>_pick`. Roster, league, team
and club names that read like NHL clubs show neutral labels (`src/nhl.rs`, the
list Tawks' rich presence guard uses).

The bot asks for `GET /v1/tawks/leaderboards?count=25`, signed like every other
call but with the reserved subject `@service` instead of a Tawks user. PuckLab
accepts that subject on this one read-only route and resolves no account for
it. The answer is the public `/v1/ranked/leaders` and `/v1/career/leaders`
shapes plus `tawks_user_id` (the active Tawks link, or null) on every entry,
`kind` and `nhl` on each roster, `streak_leaders` per ranked roster and a
`lab.rosters[].leaders[]` table of Lab Rivals clubs. Until PuckLab serves it
the bot falls back to the public boards: plain names, no Lab table, and no
title reports. An empty ladder reports nothing either, so the current holders
stay until a linked account leads.

From the PuckLab League ladder the bot reports the PuckLab leader (first
place, when linked) and Hot hand (the longest current positive streak) to
Tawks through `pucklab_leaders_report`, which only the bot selling PuckLab may
call. `tests/leaderboards_stub.py` stands in for PuckLab on a local stack.

## Remaining feature work

- Rivals queue/challenge acceptance, Tawks-native launch/join, shared chat
  invite workflows, participation/final ingestion and live score/result widgets
  still need the trusted gameplay integration.
- Bot registration, discovery/channel publishing and deployment remain
  separate release work.

[COVERAGE.md](COVERAGE.md) inventories the concrete domain APIs and the
cross-system launch/chat boundaries.

## Configuration

The optional HUT capability workflow uses `PUCKLAB_HUT_BOT_ID`, an explicit
`PUCKLAB_HUT_NICHE_ID` (the actual Hockey niche ID), and optional
`PUCKLAB_HUT_EXCHANGE_ORIGIN` (defaults to `PUCKLAB_URL`). It describes the
targeted account-backed exchange. Provider registration remains hard-disabled
until the account-menu launcher is packaged; it adds no Play button or external
invitation. See [the provider contract](../../../docs/pucklab-hut-provider.md).

Run `cargo run -p pucklab-bot` with:

- `RSPC_URL`, `EVENTS_WS_URL`, `BOT_TOKEN`: a separate registered Tawks bot.
- `PUCKLAB_URL`: the authoritative PuckLab service origin. The bot's API calls go here.
- `PUCKLAB_SITE_URL`: the origin of every PuckLab link the bot shows people (defaults to `PUCKLAB_URL`).
- `PUCKLAB_BOT_SECRET`: a separate, random shared secret of at least 32 bytes,
  also configured on PuckLab. Never reuse an NHL or Tawks user/bot token.
- `PUCKLAB_BUILD`, `PUCKLAB_PROTOCOL`: explicitly supported service versions
  (`1`/`1` for the current pinned PuckLab source).
- `PUCKLAB_ROSTER`: the published NHL roster ID (`nhl-legacy` in the fixture).

Use the matching `tools/pucklab/prepare_service.py` adapter in the NHL development
repo. The bot signs a one-minute, single-use actor proof for each exact method,
request target and body. PuckLab verifies the proof, resolves the active Tawks
link, then uses its ordinary access, ownership, roster-hash and version rules.
The actor is the verified Tawks workflow owner. Inputs and context cannot
select another PuckLab account. Redirects are disabled; secrets require TLS
outside private/local networks. Bot secrets and proofs are not logged.

This command registers its workflow and listens for actions. It does not
publish channels or send invitations by itself. Service provisioning,
production account-provider configuration and workflow discovery still need
to be connected before release.

The upstream `/play` simulation endpoints are deliberately unavailable to the
bot. Real matches require the trusted gameplay contract, including online HUT
teams and jersey loading in Legacy. They are not implemented by this bot.

## Verification

`cargo test -p pucklab-bot` checks account selection, the boards, profile
links, your own stats and that the page carries no forms. The real
Rust-to-Python account-link integration runs with:

```sh
PUCKLAB_BOT_FIXTURE=/path/to/nhl-game-invites/tests/pucklab_bot_fixture.py \
PUCKLAB_SOURCE=/path/to/pucklab-source \
NHL_PUCKLAB_CATALOG=/path/to/exported/nhl-legacy \
cargo test -p pucklab-bot --test shared_account -- --include-ignored
```

For a local stack, `tests/leaderboards_stub.py` serves the boards and answers
`/v1/account` for the made-up players linked to the seeded dev people.
