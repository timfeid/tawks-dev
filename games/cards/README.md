# Cards

The collectible card game tawks runs in lobbies: the rules engine, card catalog, AI, draft, simulator and balance tools.

## Running the bot

```sh
CARDGAME_DATABASE_URL=postgres://... scripts/playground-db.sh   # once, to build the database
cargo run --release -p cardgame --bin cards-bot
```

| Setting | |
| --- | --- |
| `RSPC_URL`, `EVENTS_WS_URL` | tawks, or devhost (`http://localhost:8787/rspc`, `ws://localhost:8787/events`) |
| `BOT_TOKEN` | the bot's token; `BOT_TOKEN_TYPE` defaults to `Bot` |
| `CARDGAME_DATABASE_URL` | the Cards database |
| `CARDS_ACTIVITY_URL` | optional; where the Cards client is hosted, to offer Cards in lobby game menus |

`cards-migrate apply` writes the whole current schema, so a new database needs nothing else. `migrations/` holds the changes made by hand to databases created before a change; apply a new one there and add the same change to `src/bin/cards-migrate.rs`.

`scripts/playground-db.sh` writes the schema, the card catalog, the starter decks and the pack types. Run it again after changing the catalog. Store products live in the tawks database and are only written when `TAWKS_DATABASE_URL` is set.

## Tools

Each is a binary in `src/bin`, run with `cargo run -p cardgame --bin <name>`. `cards-migrate`, `manage_packs`, `validate_data` and the `check_*` tools read `CARDGAME_DATABASE_URL`. `balance_check`, `run_gauntlet` and `operator_autotune` simulate games without a database.

Card and deck art (`generate_card_images`, `generate_deck_images`) needs `--features art` and `OPENAI_API_KEY`, `S3_BUCKET_NAME` and `CARD_ART_BASE_URL` (the public URL the bucket is served from), plus the usual AWS credentials. `S3_REGION` defaults to `us-east-1`.
