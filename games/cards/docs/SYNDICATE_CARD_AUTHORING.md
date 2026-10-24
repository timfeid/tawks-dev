# Syndicate card authoring

Syndicate is a crime-themed ruleset on the Mana Clash engine. Read this, then
`src/card_catalog/syndicate/street.rs` (the reference family) and
`src/card_catalog/syndicate/neutral.rs`.

## Rules the cards live under

- No mana. Every card costs cash: `cash(n)` puts `n` in `ManaCost.colorless`.
  Never set a coloured cost. Family identity comes from `.colors(&[FAMILY])`,
  which the helpers already do.
- Cash is a bankroll that persists across turns. Each player collects 2 base
  income at turn start plus whatever their rackets pay. Players may borrow up
  to a limit; debt charges interest and the loan shark repossesses crew.
- Heat is a per-player counter. `CardEffect::Heat(n)` raises the caster's heat
  (negative cools it). `CardEffect::HeatOpponents(n)` raises every opponent's.
  At 10 or more heat at the end of the player's own turn they are raided:
  their best racket is destroyed, or they take 4 damage if they have none,
  and heat halves. Violent cards should carry heat. Lawyers, bribes and
  crooked cops cool it.
- `CardEffect::CashOpponents(n)` gives every opponent `n` cash; negative takes
  it and can push them into debt. `CardEffect::StealMana(ManaType::Colorless, n)`
  moves cash from the opponent to you (a grift).
- Rackets are `CardKind::Infrastructure` built with `racket(id, name, cost,
  income, FAMILY)`. One racket may be played per turn and it costs cash.
  A racket's income arrives at the start of its owner's turn.
- Combat, blocking, the stack, tokens, counters, triggers and keywords are
  the Mana Clash ones. Standing (life) starts at 20.

## Helpers (in `card_catalog/syndicate/mod.rs`)

```rust
use super::{FAMILY_CONST, cash, crew, job, on, racket, trick};
racket(id, name, cost, income, FAMILY)          // Infrastructure with turn-start income
crew(id, name, cost, power, toughness, FAMILY)  // Unit(Human)
job(id, name, cost, FAMILY)                     // Ritual: sorcery-speed one-shot
trick(id, name, cost, FAMILY)                   // Instant: can respond
on(TriggerEvent::X, CardEffect::Y)              // CardTrigger with no predicate
```
Chain `.description("...")`, `.keyword(Keyword::X)`, `.trigger(on(..))`,
`.effect(effect, target_spec)`, `.rarity(CardRarity::X)`, then `.build()`.
`.effect(e, None)` on a job/trick runs `e` when the spell resolves; on a
crew or racket it runs when the card enters. `.effect(e, Some(TargetSpec::new(
TargetScope::Opponent, TargetKind::PlayerOrUnit, 1)))` makes it targeted.
`TargetScope` is `SelfPlayer | Opponent | Any | AnyPlayerOrUnit`;
`TargetKind` is `Player | Unit | Card | PlayerOrUnit`.
Family constants: `OLD_BLOOD`, `STREET`, `CARTEL`, `FIRM`, `WOLVES`.

## Effects that are safe to use

`DamagePlayer(n)`, `DamageTarget(n)` (needs a target spec), `DamageUnit(n)`,
`DamageEachOpponentUnit(n)`, `DrawCards(n)`, `Loot(draw, discard)`, `Scry(n)`,
`GainLife(n)`, `Siphon(n)` (targeted player), `CreateToken(TokenSpec::new(
"Name", p, t, UnitType::Human))`, `AddMana(ManaType::Colorless, n)` (gain cash),
`StealMana(ManaType::Colorless, n)`, `CashOpponents(n)`, `Heat(n)`,
`HeatOpponents(n)`, `Discard(n)` (opponent discards), `DiscardSelfRandom(n)`,
`Bounce` (targeted), `DestroyTarget` (targeted unit), `TapTarget`, `Fight`,
`BoardWipe`, `Mill(n)`, `Reflect(n)`,
`ApplyStatModifier(StatModifierSpec { stat: StatType::Power | Toughness | Both,
amount: ModifierAmount::Fixed(n), expires: ExpireContract::EndOfTurn | Never |
Turns(k), target: EffectTargetSpec::Targets | OwnerUnits(None) | OpponentUnits(None)
| AllUnits(None) | Source })`, `GrantKeyword(Keyword::X, ExpireContract::Y)`,
`Sequence(vec![..])` to chain several.

Keywords that read well as crime: `Haste` (Trigger Happy), `Trample`
(Ruthless), `Stealth` (cannot be blocked), `Deathtouch` (Wet Work),
`Vigilance`, `Menace`, `Indestructible` (Made), `Flash`, `Camouflage`
(cannot be targeted by opponents), `ReactiveArmor(n)`, `Absorb(n)`,
`Piercing`, `Wither`, `Evolve`, `Rampage(n)`, `Interceptor`.

Triggers: `Entered`, `Died`, `Attacked`, `Blocked`, `DealtDamage`,
`ReceivedDamage`, `TurnStart`, `Upkeep`, `EndOfTurn`, `AllyEntered(None)`,
`AllyDied`, `OpponentCast`, `LifeLost`, `Sacrificed`, `TokenCreated`.

## Constraints for a family file

- Exactly one `pub fn all_cards() -> Vec<(Card, i32)>`; the `i32` is how many
  copies the family deck runs. Quantities must sum to 26 across 10 to 13
  distinct cards, no card above 3 copies. Neutral adds 8 more for a 34-card deck.
- Roughly: 6 rackets, 13 to 15 crew, 4 to 5 jobs, 2 tricks.
- Cost curve: about a third of the deck at cost 1 to 2, a third at 3, the rest
  at 4 to 6. Nothing above 6. Rackets pay 1 income per 1 to 2 cost.
- Card ids: `syn_` plus a snake_case name, unique across all families. Names
  and flavour lines must not use emojis or em-dashes.
- Power level: a 2-cost crew is about 2/2, 3-cost about 3/3 or 2/2 with a
  small effect, 5-cost about 5/4 with an effect. Heat should make aggressive
  lines cost something: a 3-damage job should carry 2 heat.
- Do not add engine features. Only compose the effects and keywords listed.
  If something you want does not exist, pick a simpler card.
- Check with `cargo check -p cardgame` from the repo root. Touch only your file.
