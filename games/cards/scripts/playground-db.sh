#!/usr/bin/env bash
set -euo pipefail

# Builds a playground Cards database from the code: schema, the card catalog,
# the starter decks and pack types. Safe to run again; it updates in place.
#
#   CARDGAME_DATABASE_URL=postgres://... games/cards/scripts/playground-db.sh

: "${CARDGAME_DATABASE_URL:?set CARDGAME_DATABASE_URL to the playground database}"

# CARDS_BIN points at built tools (as in games/Dockerfile); otherwise cargo builds them.
run() {
  if [[ -n "${CARDS_BIN:-}" ]]; then
    "$CARDS_BIN/$1" "${@:2}"
  else
    cargo run -q --profile "${PLAYGROUND_PROFILE:-release}" -p cardgame --bin "$@"
  fi
}

run cards-migrate apply
run generate_operator_decks > /dev/null
run manage_packs sync-catalog > /dev/null
run manage_packs seed-defaults
