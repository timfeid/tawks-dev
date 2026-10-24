# Syndicate

A crime card game on the Mana Clash engine. Same combat, stack, triggers,
tokens, packs, draft and simulation tooling; a different resource system.

## Rules

- **Cash, not mana.** Every card costs cash. Cash sits in
  `mana_pool.colorless` and persists across turns. Each turn start a player
  collects 2 base income plus the income printed on their rackets. Starting
  cash is 2.
- **Credit.** A player may go up to 4 into debt to play a card. Debt charges
  interest at turn start, 1 per 3 owed rounded up. Past the limit at turn
  start the loan shark repossesses: the most expensive crew is taken and the
  debt shrinks by its cost, repeating until back under the line.
- **Rackets** are `CardKind::Infrastructure`. One per turn, they cost cash,
  and they pay at the start of the owner's turn through an ordinary
  `TurnStart` trigger with `AddMana(Colorless, n)`.
- **Heat.** `Heat(n)` raises the caster's heat, `HeatOpponents(n)` raises
  everyone else's. At 10 or more heat at the end of the player's own turn
  they are raided: their best racket is destroyed, or they take 4 damage if
  they have none, and heat halves.
- **No grid.** `DeployNode` is rejected under Syndicate.
- Standing (life) starts at 20. Elimination is unchanged.

Constants live in `src/syndicate.rs`. The ruleset is chosen by the deck:
any deck id starting with `syn_` puts the table under Syndicate rules
(`Ruleset::for_deck_id`). The simulation infers it the same way.

## Families

Five families, one 26-card list each, plus an 8-card neutral pool for a
34-card deck. Colours reuse the Mana Clash palette for frames.

| deck id | family | colour | play style |
|---|---|---|---|
| syn_old_blood | Old Blood | chrome | big bodies, protection, punishes attackers |
| syn_street | Street | neon | cheap wide crews, haste, runs hot |
| syn_cartel | Cartel | bio | rackets that print cash, grifts, expensive bombs |
| syn_firm | The Firm | glitch | control, discard, bounce, heat as a weapon and a shield |
| syn_wolves | Wolves | slag | sacrifice value, death triggers, comfortable in debt |

Catalog: `src/card_catalog/syndicate/`. Authoring guide:
`docs/SYNDICATE_CARD_AUTHORING.md`.

## Balance, 2026-09-14

Matrix of all ten pairings, 60 matches per pairing per seed, archetype agents.

Tuning seeds 1 to 5, 3000 matches: balance score 0.081, average game 7.9 rounds.

| family | win rate |
|---|---|
| Firm | 0.523 |
| Wolves | 0.505 |
| Street | 0.494 |
| Cartel | 0.478 |
| Old Blood | 0.475 |

Validation seeds 6 to 10, 3000 matches never used for tuning: balance score
0.085, every family between 0.470 and 0.517. For comparison the Mana Clash
five-colour matrix scored 0.225 on 200 matches the same day.

Reproduce:

```
cargo build --release -p cardgame --bin run_gauntlet
./target/release/run_gauntlet --matrix \
  --decks syn_old_blood,syn_street,syn_cartel,syn_firm,syn_wolves \
  --matches 60 --seed 6 --max-turns 60 --timeout-seconds 30 > out.json
```

## Deploy notes

The three new effect kinds need the Postgres enum extended on any existing
card game database before starter decks are seeded:

```sql
ALTER TYPE card_effect_kind ADD VALUE IF NOT EXISTS 'heat';
ALTER TYPE card_effect_kind ADD VALUE IF NOT EXISTS 'heat_opponents';
ALTER TYPE card_effect_kind ADD VALUE IF NOT EXISTS 'cash_opponents';
```

Fresh databases get them from `src/bin/migrate.rs`.
