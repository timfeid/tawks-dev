# Trivia

Daily quizzes in channels and live multiplayer rounds in lobbies.

```sh
cargo run --release -p trivia --bin trivia-bot
```

| Setting | |
| --- | --- |
| `RSPC_URL`, `EVENTS_WS_URL` | tawks, or devhost (`http://localhost:8787/rspc`, `ws://localhost:8787/events`) |
| `BOT_TOKEN` | the bot's token; `BOT_TOKEN_TYPE` defaults to `Bot` |
| `TRIVIA_DATABASE_URL` | the Trivia database; the bot creates and migrates its tables on start |
| `OPENAI_API_KEY` | optional; without it every quiz is a built-in sample. `OPENAI_MODEL` picks the model |

`cargo test -p trivia` also runs the database tests when `TRIVIA_DATABASE_URL` is set.
