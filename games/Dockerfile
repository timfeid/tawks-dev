# Cards and Trivia bots, plus the Cards database tools. Build from the
# repository root:
#   docker build -f games/Dockerfile -t tawks-games .
# then run `cards-bot` or `trivia-bot` with the settings in each game's README.
# `cards-playground-db` builds a fresh Cards database in CARDGAME_DATABASE_URL.
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p cardgame -p trivia \
      --bin cards-bot --bin cards-migrate --bin generate_operator_decks --bin manage_packs --bin trivia-bot

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 bot
COPY --from=build /src/target/release/cards-bot /src/target/release/trivia-bot /usr/local/bin/
COPY --from=build /src/target/release/cards-migrate /src/target/release/generate_operator_decks /src/target/release/manage_packs /usr/local/lib/cards/
COPY games/cards/scripts/playground-db.sh /usr/local/bin/cards-playground-db
USER bot
ENV CARDS_BIN=/usr/local/lib/cards RUST_LOG=info
