//! Syndicate ruleset: cash instead of mana, debt with interest, and heat.
//!
//! Cash lives in `mana_pool.colorless` so the affordability checks, the
//! `AddMana(Colorless, n)` income triggers and the client counter all work
//! unchanged. A negative balance is debt.

use crate::card::{Card, CardEffect};
use crate::state::{CardGameState, GameLogKind, PlayerState};
use crate::types::{CardKind, ManaCost, TriggerEvent};

/// Cash every crew collects at the start of its turn before rackets pay out.
pub const BASE_INCOME: i32 = 2;
/// How far into debt a player may go to play a card.
pub const DEBT_LIMIT: i32 = 4;
/// Interest charged at turn start per this many dollars of debt (rounded up).
pub const INTEREST_PER: i32 = 3;
/// Heat at or above this at end of turn triggers a raid.
pub const HEAT_LINE: i32 = 10;
/// Life lost in a raid when there is no racket left to take.
pub const RAID_DAMAGE: i32 = 4;
/// Cash each player starts with.
pub const STARTING_CASH: i32 = 2;

pub fn is_racket(card: &Card) -> bool {
    matches!(card.kind, CardKind::Infrastructure)
}

/// Income printed on a racket: the colourless amount of its turn-start trigger.
pub fn racket_income(card: &Card) -> i32 {
    card.triggers
        .iter()
        .filter(|t| matches!(t.event, TriggerEvent::TurnStart | TriggerEvent::Upkeep))
        .map(|t| match &t.effect {
            CardEffect::AddMana(_, n) => *n,
            _ => 0,
        })
        .sum()
}

pub fn cash_cost(cost: &ManaCost) -> i32 {
    cost.total()
}

pub fn can_pay(player: &PlayerState, cost: &ManaCost) -> bool {
    player.mana_pool.colorless - cash_cost(cost) >= -DEBT_LIMIT
}

pub fn pay(player: &mut PlayerState, cost: &ManaCost) -> bool {
    if !can_pay(player, cost) {
        return false;
    }
    player.mana_pool.colorless -= cash_cost(cost);
    true
}

/// Turn-start bankroll step: interest on debt, then base income. Racket
/// income arrives through their own turn-start triggers.
pub fn collect_income(player: &mut PlayerState) {
    let debt = -player.mana_pool.colorless;
    if debt > 0 {
        let interest = (debt + INTEREST_PER - 1) / INTEREST_PER;
        player.mana_pool.colorless -= interest;
    }
    player.mana_pool.colorless += BASE_INCOME;
}

impl CardGameState {
    /// The loan shark collects: while a player is past the debt limit at
    /// turn start, their most expensive crew member is taken and the debt
    /// shrinks by that card's cost.
    pub(crate) fn syndicate_repossess(&mut self, player_id: &str) {
        loop {
            let Some(player) = self.players.get(player_id) else {
                return;
            };
            if player.mana_pool.colorless >= -DEBT_LIMIT {
                return;
            }
            let Some(victim) = player
                .battlefield
                .iter()
                .filter(|c| matches!(c.kind, CardKind::Unit(_)))
                .max_by_key(|c| cash_cost(&c.cost))
                .map(|c| (c.id.clone(), c.name.clone(), cash_cost(&c.cost)))
            else {
                // Nothing left to take: the debt is written off to the limit.
                if let Some(player) = self.players.get_mut(player_id) {
                    player.mana_pool.colorless = -DEBT_LIMIT;
                }
                return;
            };
            self.remove_from_battlefield_and_attachments(player_id, &victim.0);
            if let Some(player) = self.players.get_mut(player_id) {
                player.mana_pool.colorless += victim.2.max(1);
            }
            self.log(
                GameLogKind::Other,
                player_id,
                format!("the loan shark takes {}", victim.1),
            );
        }
    }

    /// End-of-turn police check for the active player.
    pub(crate) fn syndicate_raid_check(&mut self, player_id: &str) {
        let Some(player) = self.players.get(player_id) else {
            return;
        };
        if player.heat < HEAT_LINE {
            return;
        }
        let racket = player
            .battlefield
            .iter()
            .filter(|c| is_racket(c))
            .max_by_key(|c| (racket_income(c), cash_cost(&c.cost)))
            .map(|c| (c.id.clone(), c.name.clone()));
        match racket {
            Some((id, name)) => {
                self.remove_from_battlefield_and_attachments(player_id, &id);
                self.log(
                    GameLogKind::Other,
                    player_id,
                    format!("is raided and loses {}", name),
                );
            }
            None => {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.life -= RAID_DAMAGE;
                }
                self.log(
                    GameLogKind::Other,
                    player_id,
                    format!("is raided and takes {} damage", RAID_DAMAGE),
                );
            }
        }
        if let Some(player) = self.players.get_mut(player_id) {
            player.heat /= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::CardGameAction;
    use crate::card::CardBuilder;
    use crate::card_catalog::syndicate::{cash, crew, racket, STREET};
    use crate::state::CardGameInitArgs;
    use crate::types::{CardGameStage, CombatPhase, ManaType, Ruleset};
    use game_framework::GameLogic;

    fn join(state: &mut CardGameState, id: &str) {
        state
            .update(
                id,
                CardGameAction::Join {
                    name: id.to_string(),
                    avatar: String::new(),
                    instance_id: format!("instance-{id}"),
                },
            )
            .unwrap();
    }

    fn setup() -> CardGameState {
        let mut state = CardGameState::new(CardGameInitArgs::default());
        join(&mut state, "p1");
        join(&mut state, "p2");
        for p in ["p1", "p2"] {
            state
                .update(
                    p,
                    CardGameAction::SelectDeck {
                        deck_id: "syn_street".into(),
                    },
                )
                .unwrap();
        }
        state.update("p1", CardGameAction::StartGame).unwrap();
        assert_eq!(state.stage, CardGameStage::Main);
        assert_eq!(state.ruleset, Ruleset::Syndicate);
        state
    }

    fn pass_turn(state: &mut CardGameState, player_id: &str) {
        state.update(player_id, CardGameAction::NextStage).unwrap();
        state
            .update(player_id, CardGameAction::ConfirmAttackers)
            .unwrap();
        while state.combat_phase == CombatPhase::DeclareBlockers {
            let blocker = state.combat_blocker_id.clone().unwrap();
            state
                .update(&blocker, CardGameAction::ConfirmBlockers)
                .unwrap();
        }
        state.update(player_id, CardGameAction::NextStage).unwrap();
    }

    fn cash_of(state: &CardGameState, p: &str) -> i32 {
        state.players[p].mana_pool.colorless
    }

    #[test]
    fn starting_cash_and_no_grid() {
        let mut state = setup();
        assert_eq!(cash_of(&state, "p1"), STARTING_CASH + BASE_INCOME);
        assert_eq!(state.players["p1"].heat, 0);
        assert!(state.players["p1"].grid.is_empty());
        assert!(
            state
                .update(
                    "p1",
                    CardGameAction::DeployNode {
                        mana_type: ManaType::Neon
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn credit_lets_you_overspend_up_to_the_limit() {
        let mut state = setup();
        let big = crew("syn_test_big", "Big", 6, 5, 5, STREET).build();
        let small = crew("syn_test_small", "Small", 1, 1, 1, STREET).build();
        {
            let p = state.players.get_mut("p1").unwrap();
            p.hand.clear();
            p.hand.push(big.clone());
            p.hand.push(small.clone());
            p.mana_pool.colorless = 2;
        }
        assert!(state.can_cast_card("p1", &big));
        state
            .update(
                "p1",
                CardGameAction::CastSpell {
                    card_id: big.id.clone(),
                    targets: vec![],
                },
            )
            .unwrap();
        assert_eq!(cash_of(&state, "p1"), 2 - 6);
        assert!(!state.can_cast_card("p1", &small), "past the credit line");
    }

    #[test]
    fn interest_then_income_then_racket_pay() {
        let mut state = setup();
        {
            let p = state.players.get_mut("p1").unwrap();
            p.mana_pool.colorless = -3;
            let mut r = racket("syn_test_racket", "Racket", 2, 2, STREET).build();
            r.id = "p1-racket".into();
            p.battlefield.push(r);
        }
        pass_turn(&mut state, "p1");
        pass_turn(&mut state, "p2");
        // -3 debt: interest 1, base income 2, racket 2.
        assert_eq!(cash_of(&state, "p1"), -3 - 1 + BASE_INCOME + 2);
    }

    #[test]
    fn loan_shark_takes_crew_past_the_limit() {
        let mut state = setup();
        {
            let p = state.players.get_mut("p1").unwrap();
            p.mana_pool.colorless = -9;
            let mut c = crew("syn_test_muscle", "Muscle", 4, 3, 3, STREET).build();
            c.id = "p1-muscle".into();
            p.battlefield.push(c);
        }
        pass_turn(&mut state, "p1");
        pass_turn(&mut state, "p2");
        let p = &state.players["p1"];
        assert!(p.battlefield.iter().all(|c| c.id != "p1-muscle"));
        assert_eq!(p.mana_pool.colorless, -DEBT_LIMIT);
    }

    #[test]
    fn raid_takes_the_best_racket_and_halves_heat() {
        let mut state = setup();
        {
            let p = state.players.get_mut("p1").unwrap();
            p.heat = HEAT_LINE;
            let mut small = racket("syn_test_r1", "Small", 1, 1, STREET).build();
            small.id = "p1-small".into();
            let mut big = racket("syn_test_r2", "Big", 3, 2, STREET).build();
            big.id = "p1-big".into();
            p.battlefield.push(small);
            p.battlefield.push(big);
        }
        pass_turn(&mut state, "p1");
        let p = &state.players["p1"];
        assert!(p.battlefield.iter().any(|c| c.id == "p1-small"));
        assert!(p.battlefield.iter().all(|c| c.id != "p1-big"));
        assert_eq!(p.heat, HEAT_LINE / 2);
    }

    #[test]
    fn raid_without_rackets_costs_life() {
        let mut state = setup();
        let life = state.players["p1"].life;
        state.players.get_mut("p1").unwrap().heat = HEAT_LINE + 2;
        pass_turn(&mut state, "p1");
        assert_eq!(state.players["p1"].life, life - RAID_DAMAGE);
    }

    #[test]
    fn heat_effects_move_the_counter() {
        let mut state = setup();
        let hot = CardBuilder::new("syn_test_hot", "Hot", crate::types::CardKind::Ritual)
            .cost(cash(0))
            .effect(
                CardEffect::Sequence(vec![CardEffect::Heat(3), CardEffect::HeatOpponents(2)]),
                None,
            )
            .build();
        {
            let p = state.players.get_mut("p1").unwrap();
            p.hand.clear();
            p.hand.push(hot.clone());
        }
        state
            .update(
                "p1",
                CardGameAction::CastSpell {
                    card_id: hot.id.clone(),
                    targets: vec![],
                },
            )
            .unwrap();
        while !state.stack.is_empty() {
            let holder = state.priority_holder.clone().unwrap_or("p1".into());
            state.update(&holder, CardGameAction::PassPriority).unwrap();
        }
        assert_eq!(state.players["p1"].heat, 3);
        assert_eq!(state.players["p2"].heat, 2);
    }

    #[test]
    fn every_family_deck_is_34_cash_cards() {
        use crate::card_catalog::syndicate::FAMILY_DECKS;
        use crate::deck::{build_deck, deck_definitions};
        let defs = deck_definitions();
        for id in FAMILY_DECKS {
            assert!(defs.iter().any(|d| d.id == *id), "{id} missing from deck definitions");
            let deck = build_deck("p1", id, Some(1));
            assert_eq!(deck.len(), 34, "{id} should be 34 cards");
            for card in &deck {
                let c = &card.cost;
                assert_eq!(
                    c.neon + c.glitch + c.chrome + c.slag + c.bio,
                    0,
                    "{} has a coloured cost",
                    card.id
                );
                assert!(card.cost.total() <= 6, "{} costs more than 6", card.id);
            }
        }
    }
}
