# Longtable

A table for 5E compatible games in a tawks lobby: dice rolled on the server, an initiative order the GM runs, and simple character sheets, all shared live.

- `src/dice.rs` reads rolls the way people type them: `2d6+3`, `4d6dl1`, `2d20kh1+5`, `d%`, `1d20+4 adv stealth`. Words after the formula are its label; `adv` and `dis` turn its d20 into two and keep the higher or lower.
- `src/table.rs` is one lobby's table: the GM seat, sheets, the initiative order and the roll log, with who may change what and what each person sees. Hidden GM rolls reach players as "The GM rolled in secret", with no dice in them.
- `src/sheet.rs` and `src/initiative.rs` hold the sheet and order rules. Players change their own sheets; the GM changes any of them and runs the order.
- `src/server.rs` is the table socket on the party port, `/party/longtable/ws`: one room per lobby, written through to Postgres a second after each change, so a restart picks the table back up.
- `src/runtime.rs` registers the lobby game and two chat commands, `/roll <formula>` and `/init` (`add Goblin 12`, `add Mira +3`, `next`, `remove Goblin`, `clear`).
- `client/` is the activity frame: a dice tray, the initiative order and the sheets. On a phone they are tabs.

The first person to open a lobby's table runs it, and can hand the GM seat to anyone who has sat down. The lobby's owner can always take it back.

When the order moves on, the server asks tawks to light up the stage tile of whoever is up (`set_lobby_turn`), so faces in the lobby show the turn too.

## Running it

It runs inside the tawks games service, which mounts `LongtableServer::router()` on the party port and calls `runtime::register` with its bot.

| Setting | What it does |
| --- | --- |
| `LONGTABLE_DATABASE_URL` | The Postgres database for tables and chat orders. The table is off without it. |
| `LONGTABLE_ACTIVITY_URL` | Where the built client is served. Without it the game stays out of the lobby menu; the chat commands still work. |
| `LONGTABLE_SECRET` | Signs seat passes. Defaults to one derived from `PARTY_SECRET`. |

To work on the client without tawks:

```sh
cargo run -p longtable --example dev
pnpm --filter @tawks/longtable-activity dev
```

and open the printed links, one per player.

## Rules content

Condition names and summaries come from the System Reference Document 5.2.1. The table's credits screen carries its attribution. `src/srd.rs` rewrites the source's product names out of anything imported, and a test fails if one reaches the server, the client or this file.
