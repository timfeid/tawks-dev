# tawks-dev

SDKs, examples and docs for building bots and games on [tawks](https://tawks.app). Read the docs, try the playground and call the API from your browser at [tawks-dev.vercel.app](https://tawks-dev.vercel.app).

| Path | What it is |
| --- | --- |
| `sdk/bot` | `tawks-bot`, the Rust bot SDK: connect, register workflows and server actions, talk in lobbies |
| `sdk/workflow-model` | the workflow data model and builders bots use to describe their pages |
| `sdk/framework` | `game-framework`, the turn-based game engine the first-party lobby games are built on |
| `sdk/bot-js` | `@tawks/bot`, the JavaScript and TypeScript SDK for bots |
| `sdk/activity` | `@tawks/activity-sdk`, for a web game running in a tawks activity frame |
| `sdk/bout` | `tawks-bout` and `@tawks/bout-sdk`, the contract for Tawkathon bouts (simulation and renderer) |
| `examples/hello-js` | a page with a server action and a chat command, in plain JavaScript |
| `examples/tictactoe` | a two-player activity: rules in a bot, board in a web app |
| `examples/hot-potato` | a 2 to 6 player activity with a hidden fuse, a turn clock and lobby announcements |
| `examples/hoard` | a 2 to 4 player card game: hidden hands, a shared market, cards that steal and shield, spectators and animated moves |
| `examples/bumper-balls` | the Tawkathon bout tawks itself runs, simulation and renderer |
| `games/cards` | Cards, the collectible card game tawks runs in lobbies: rules engine, card catalog, AI, draft, simulator and balance tools |
| `games/midnight` | Midnight, a hidden-role party game for lobbies, built from workflow pages |
| `games/tracklines` | Tracklines, a 3D railway game for lobbies |
| `games/canvases` | `@tawks/game-canvases`, the 3D canvases that draw Cards and Tracklines |
| `sdk/data` | `tawks-data`, small Postgres helpers for cursor pagination |
| `tools/devhost` | a local stand-in for tawks: run your bot and play your game as several fake players |
| `tools/gen-types` | generates `web/workflow-types` and the playground's templates from the Rust model |
| `web` | the workflow renderer, UI kit and editor that tawks draws pages with |
| `site` | the docs site and the workflow playground |
| `bots/tawks-dev` | the bot that runs the Tawks Dev niche and publishes `docs/` to it |
| `docs` | the developer docs, also readable in the Tawks Dev niche |

## Getting started

Read `docs/01-overview.md`, then `docs/02-first-bot.md`. You need a bot token from tawks.

```sh
cargo test --workspace
pnpm install
```

To try a game without tawks, run devhost, point a bot at it, and open `http://localhost:8787` (set `DEVHOST_PORT` for another port). Give each bot its own `BOT_TOKEN`; any name works. The tic-tac-toe example, whose client runs on port 5190:

```sh
cargo run -p tawks-devhost
RSPC_URL=http://localhost:8787/rspc EVENTS_WS_URL=ws://localhost:8787/events BOT_TOKEN=tictactoe cargo run -p tictactoe-activity
pnpm --filter @tawks/example-tictactoe dev
```

To build a page from workflow displays, open the [playground](https://tawks-dev.vercel.app/playground), or run it yourself with `pnpm --filter @tawks/site dev` and open `http://localhost:5300/playground`. It runs pages in the browser, answers server actions from mocks you write, and gives you the Rust for what you built. With devhost running, the plug button loads the pages your bot registered and sends their server actions to your bot.

## Contributing

Issues and pull requests are welcome. Bouts and games added here are reviewed before they ship on tawks.

## License

MIT
