use crate::card::*;
use crate::types::*;

/// The "Card Builder" studio that allows mixing and matching modular components.
pub struct CardStudio {
    builder: CardBuilder,
}

impl CardStudio {
    pub fn begin(id: &str, name: &str, kind: CardKind) -> Self {
        Self {
            builder: CardBuilder::new(id, name, kind),
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.builder = self.builder.description(desc);
        self
    }

    pub fn cost(mut self, cost: ManaCost) -> Self {
        self.builder = self.builder.cost(cost);
        self
    }

    pub fn stats(mut self, power: i32, toughness: i32) -> Self {
        self.builder = self.builder.stats(power, toughness);
        self
    }

    pub fn image(mut self, url: &str) -> Self {
        self.builder = self.builder.image(url);
        self
    }

    pub fn keyword(mut self, keyword: Keyword) -> Self {
        self.builder = self.builder.keyword(keyword);
        self
    }

    /// Adds a modular ability composed of Trigger, Action, and Constraint.
    pub fn with_ability(mut self, module: AbilityModule) -> Self {
        match module.trigger {
            Trigger::None => {
                // If no trigger, it's an activated ability or a static effect
                let id = format!("ability-{}", rand::random::<u32>());
                self.builder = self.builder.ability(CardAbility {
                    id,
                    name: "Activated Ability".to_string(),
                    cost: AbilityCost::default(), // TODO: Support modular costs
                    effect: module.action,
                    target_spec: None, // TODO: Support modular targets
                    once_per_turn: false,
                    uses_stack: true,
                    tap_targets: false,
                    predicate: module.constraint,
                });
            }
            Trigger::Event(event) => {
                self.builder = self.builder.trigger(CardTrigger {
                    event,
                    effect: module.action,
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
            }
        }
        self
    }

    pub fn build(self) -> Card {
        self.builder.build()
    }
}

/// A modular ability module that combines Trigger, Action, and Constraint.
pub struct AbilityModule {
    pub trigger: Trigger,
    pub action: CardEffect,
    pub constraint: Option<Predicate>,
}

pub enum Trigger {
    None,
    Event(TriggerEvent),
}

/// The "Trigger" module providing building blocks.
pub struct Triggers;
impl Triggers {
    pub fn none() -> Trigger {
        Trigger::None
    }
    pub fn on_play() -> Trigger {
        Trigger::Event(TriggerEvent::Entered)
    }
    pub fn on_enter_battlefield() -> Trigger {
        Trigger::Event(TriggerEvent::Entered)
    }
    pub fn on_leave_battlefield() -> Trigger {
        Trigger::Event(TriggerEvent::Left)
    }
    pub fn on_death() -> Trigger {
        Trigger::Event(TriggerEvent::Died)
    }
    pub fn on_attack() -> Trigger {
        Trigger::Event(TriggerEvent::Attacked)
    }
    pub fn on_damage_dealt() -> Trigger {
        Trigger::Event(TriggerEvent::DealtDamage)
    }
    pub fn on_damage_received() -> Trigger {
        Trigger::Event(TriggerEvent::ReceivedDamage)
    }
    pub fn on_tap() -> Trigger {
        Trigger::Event(TriggerEvent::Tapped)
    }
    pub fn on_upkeep() -> Trigger {
        Trigger::Event(TriggerEvent::Upkeep)
    }
    pub fn on_draw_step() -> Trigger {
        Trigger::Event(TriggerEvent::DrawStep)
    }
}

/// The "Action" module providing building blocks.
pub struct Actions;
impl Actions {
    pub fn damage_player(amount: i32) -> CardEffect {
        CardEffect::DamagePlayer(amount)
    }
    pub fn damage_unit(amount: i32) -> CardEffect {
        CardEffect::DamageUnit(amount)
    }
    pub fn damage_target(amount: i32) -> CardEffect {
        CardEffect::DamageTarget(amount)
    }
    pub fn draw(count: usize) -> CardEffect {
        CardEffect::DrawCards(count)
    }
    pub fn add_mana(mana_type: ManaType, amount: i32) -> CardEffect {
        CardEffect::AddMana(mana_type, amount)
    }
    pub fn freeze(turns: u8) -> CardEffect {
        CardEffect::Freeze(ExpireContract::Turns(turns))
    }
    pub fn refund(percentage_basis_points: i32) -> CardEffect {
        CardEffect::RefundMana(percentage_basis_points)
    }
    pub fn loot(draw: usize, discard: usize) -> CardEffect {
        CardEffect::Loot(draw, discard)
    }
    pub fn board_wipe() -> CardEffect {
        CardEffect::BoardWipe
    }
    pub fn bounce() -> CardEffect {
        CardEffect::Bounce
    }
    pub fn destroy_target() -> CardEffect {
        CardEffect::DestroyTarget
    }
    pub fn exile_target() -> CardEffect {
        CardEffect::ExileTarget
    }
}

/// The "Constraint" module providing building blocks.
pub struct Constraints;
impl Constraints {
    pub fn none() -> Option<Predicate> {
        None
    }
    pub fn life_below(amount: i32) -> Option<Predicate> {
        Some(Predicate::Condition(Condition::LifeLessThan(amount)))
    }
    pub fn target_is_unit() -> Option<Predicate> {
        Some(Predicate::Condition(Condition::TargetIsKind(
            TargetKind::Unit,
        )))
    }
    pub fn once_per_turn() -> Option<Predicate> {
        Some(Predicate::Condition(Condition::OncePerTurn))
    }
}

impl AbilityModule {
    pub fn new(trigger: Trigger, action: CardEffect, constraint: Option<Predicate>) -> Self {
        Self {
            trigger,
            action,
            constraint,
        }
    }
}
