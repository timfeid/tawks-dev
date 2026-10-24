use std::sync::Arc;

use crate::card::{
    AbilityCost, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, StatModifierSpec,
};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use tawks_bot::{full_avatar_url, types::UserResource};
use tawks_data::pagination::PaginationArgs;

use crate::action::CardGameAction;
use crate::state::CardGameState;
use crate::types::{
    CardKind, EffectTargetSpec, ExpireContract, GameFormat, ManaCost, ManaType, ModifierAmount,
    StatType, TargetRef, TriggerEvent, TriggerLimit, UnitType,
};
use game_framework::engine::GameEngine;

pub mod action;
pub mod builder;
pub mod card;
pub mod card_catalog;
pub mod deck;
pub mod effect;
pub mod engine;
pub mod logic;
pub mod mana;
pub mod pack;
pub mod repository;
pub mod runtime;
pub mod sim;
pub mod state;
pub mod syndicate;
pub mod studio;
pub mod types;
pub mod user_card;
pub mod view;
pub mod workflows;

// Refactored logic modules
mod card_movement;
mod combat;
mod display;
mod effects;
mod game_flow;
mod handlers;
mod helpers;
mod predicates;
mod queries;
mod triggers;
mod view_constructors;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct CardGameActionIds {
    pub view: String,
    pub add_ai: String,
    pub join: String,
    pub set_format: String,
    pub set_mode: String,
    pub set_match_format: String,
    pub submit_sideboard: String,
    pub select_deck: String,
    pub start: String,
    pub play_land: String,
    pub deploy_node: String,
    pub tap_card: String,
    pub cast_spell: String,
    pub activate_ability: String,
    pub resolve_stack: String,
    pub cancel_stack: String,
    pub pass_priority: String,
    pub declare_attack: String,
    pub declare_block: String,
    pub scry_commit: String,
    pub search_commit: String,
    pub search_cancel: String,
    pub trigger_choice: String,
    pub trigger_target_select: String,
    pub trigger_target_cancel: String,
    pub discard_commit: String,
    pub discard: String,
    pub sacrifice: String,
    pub next_stage: String,
    pub confirm_attackers: String,
    pub confirm_blockers: String,
    pub ping: String,
    pub reorder_battlefield: String,
    pub save_card: String,
    pub list_cards: String,
    pub builder_load_deck: String,
    pub builder_add_card: String,
    pub builder_remove_card: String,
    pub builder_add_sideboard_card: String,
    pub builder_remove_sideboard_card: String,
    pub builder_set_operator: String,
    pub builder_clone_deck: String,
    pub builder_set_filter: String,
    pub builder_set_sort: String,
    pub builder_open: String,
    pub builder_close: String,
    pub builder_create_deck: String,
    pub builder_autobuild: String,
    pub browser_open: String,
    pub browser_close: String,
    pub browser_set_filter: String,
    pub browser_set_sort: String,
    pub browser_page: String,
    pub debug_setup: String,
    pub debug_ai_concede: String,
    pub list_pack_types: String,
    pub get_user_packs: String,
    pub open_pack: String,
    pub get_pack_opening_results: String,
    pub admin_grant_pack: String,
    pub reset: String,
    // Draft actions
    pub draft_start: String,
    pub draft_pick: String,
    pub draft_auto_pick: String,
    pub draft_confirm_pool: String,
    pub draft_keep_cards: String,
}

#[derive(Clone, Debug, Deserialize)]
struct CardDraft {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    image_url: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    unit_type: Option<UnitType>,
    #[serde(default)]
    cost: ManaCost,
    #[serde(default)]
    power: Option<i32>,
    #[serde(default)]
    toughness: Option<i32>,
    #[serde(default)]
    loyalty: Option<i32>,
    #[serde(default)]
    trigger_one: TriggerDraft,
    #[serde(default)]
    trigger_two: TriggerDraft,
    #[serde(default)]
    trigger_three: TriggerDraft,
    #[serde(default)]
    ability_one: AbilityDraft,
    #[serde(default)]
    ability_two: AbilityDraft,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct EffectDraft {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    mana_type: Option<ManaType>,
    #[serde(default)]
    amount: Option<i32>,
    #[serde(default)]
    count: Option<i32>,
    #[serde(default)]
    stat: Option<StatType>,
    #[serde(default)]
    expires: Option<ExpireContract>,
    #[serde(default)]
    target: Option<EffectTargetSpec>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TriggerDraft {
    #[serde(default)]
    event: Option<TriggerEvent>,
    #[serde(default)]
    effect: EffectDraft,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct AbilityCostDraft {
    #[serde(default)]
    mana: ManaCost,
    #[serde(default)]
    tap: bool,
    #[serde(default)]
    life: i32,
    #[serde(default)]
    tap_other: usize,
    #[serde(default)]
    sacrifice: usize,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct AbilityDraft {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    cost: AbilityCostDraft,
    #[serde(default)]
    effect: EffectDraft,
}

impl CardDraft {
    fn into_card(self) -> Result<Card> {
        let id = normalize_card_id(self.id.as_deref(), &self.name)?;
        let kind = normalize_card_kind(self.kind.as_deref(), self.unit_type)?;

        let name = self.name.trim();
        if name.is_empty() {
            return Err(anyhow!("Card name is required"));
        }

        validate_cost(&self.cost)?;
        validate_kind_fields(&kind, self.power, self.toughness, self.loyalty)?;

        let mut builder = CardBuilder::new(&id, name, kind.clone())
            .description(self.description.trim())
            .cost(self.cost);

        if let Some(url) = self
            .image_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            builder = builder.image(url);
        }

        if let (Some(power), Some(toughness)) = (self.power, self.toughness) {
            builder = builder.stats(power, toughness);
        }

        if let Some(loyalty) = self.loyalty {
            builder = builder.loyalty(loyalty);
        }

        let triggers = build_triggers(&[self.trigger_one, self.trigger_two, self.trigger_three])?;
        let abilities = build_abilities(&[self.ability_one, self.ability_two])?;

        let mut card = builder.build();
        card.triggers = triggers;
        card.abilities = abilities;

        Ok(card)
    }
}

fn validate_cost(cost: &ManaCost) -> Result<()> {
    let negative = [
        ("neon", cost.neon),
        ("glitch", cost.glitch),
        ("chrome", cost.chrome),
        ("slag", cost.slag),
        ("bio", cost.bio),
        ("colorless", cost.colorless),
    ]
    .into_iter()
    .find(|(_, value)| *value < 0);

    if let Some((label, value)) = negative {
        return Err(anyhow!(
            "Mana cost {label} cannot be negative (got {value})"
        ));
    }

    Ok(())
}

fn validate_kind_fields(
    kind: &CardKind,
    power: Option<i32>,
    toughness: Option<i32>,
    loyalty: Option<i32>,
) -> Result<()> {
    match kind {
        CardKind::Unit(_) => {
            if power.is_none() || toughness.is_none() {
                return Err(anyhow!("Unit cards require power and toughness"));
            }
        }
        _ => {
            if power.is_some() || toughness.is_some() {
                return Err(anyhow!("Only unit cards can have power/toughness"));
            }
        }
    }

    match kind {
        CardKind::Agent => {
            if loyalty.is_none() {
                return Err(anyhow!("Agent cards require loyalty"));
            }
        }
        _ => {
            if loyalty.is_some() {
                return Err(anyhow!("Only agent cards can have loyalty"));
            }
        }
    }

    Ok(())
}

fn normalize_card_id(id: Option<&str>, name: &str) -> Result<String> {
    if let Some(value) = id.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(value.to_string());
    }

    let slug = slugify_name(name);
    if slug.is_empty() {
        return Err(anyhow!("Card id or name is required"));
    }

    Ok(slug)
}

fn slugify_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !out.is_empty() && !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    out
}

fn normalize_card_kind(kind: Option<&str>, unit_type: Option<UnitType>) -> Result<CardKind> {
    let kind = kind.unwrap_or("infrastructure");
    let kind = if kind.is_empty() {
        "infrastructure"
    } else {
        kind
    };

    match kind {
        "infrastructure" => Ok(CardKind::Infrastructure),
        "unit" => Ok(CardKind::Unit(unit_type.unwrap_or(UnitType::None))),
        "relic" => Ok(CardKind::Relic),
        "instant" => Ok(CardKind::Instant),
        "ritual" => Ok(CardKind::Ritual),
        "aura" => Ok(CardKind::Aura),
        "agent" => Ok(CardKind::Agent),
        other => Err(anyhow!("Invalid card kind: {other}")),
    }
}

fn build_effect(draft: &EffectDraft) -> Result<Option<CardEffect>> {
    let kind = draft.kind.as_deref().unwrap_or("").trim().to_string();
    if kind.is_empty() {
        return Ok(None);
    }

    match kind.as_str() {
        "remember_target_player" => Ok(Some(CardEffect::RememberTargetPlayer)),
        "add_mana" => {
            let mana_type = draft
                .mana_type
                .clone()
                .ok_or_else(|| anyhow!("Add Mana requires a mana type"))?;
            let amount = draft
                .amount
                .ok_or_else(|| anyhow!("Add Mana requires an amount"))?;
            Ok(Some(CardEffect::AddMana(mana_type, amount)))
        }
        "draw_cards" => {
            let count = draft
                .count
                .ok_or_else(|| anyhow!("Draw Cards requires a count"))?;
            Ok(Some(CardEffect::DrawCards(count as usize)))
        }
        "damage_opponent" | "damage_player" => {
            let amount = draft
                .amount
                .ok_or_else(|| anyhow!("Damage Player requires an amount"))?;
            Ok(Some(CardEffect::DamagePlayer(amount)))
        }
        "damage_unit" => {
            let amount = draft
                .amount
                .ok_or_else(|| anyhow!("Damage Unit requires an amount"))?;
            Ok(Some(CardEffect::DamageUnit(amount)))
        }
        "damage_target" => {
            let amount = draft
                .amount
                .ok_or_else(|| anyhow!("Damage Target requires an amount"))?;
            Ok(Some(CardEffect::DamageTarget(amount)))
        }
        "gain_life" => {
            let amount = draft
                .amount
                .ok_or_else(|| anyhow!("Gain Life requires an amount"))?;
            Ok(Some(CardEffect::GainLife(amount)))
        }
        "freeze" => {
            let expires = draft.expires.clone().unwrap_or(ExpireContract::Turns(1));
            Ok(Some(CardEffect::Freeze(expires)))
        }
        "discard" => {
            let count = draft
                .count
                .ok_or_else(|| anyhow!("Discard requires a count"))?;
            Ok(Some(CardEffect::Discard(count as usize)))
        }
        "scry" => {
            let count = draft
                .count
                .ok_or_else(|| anyhow!("Scry requires a count"))?;
            Ok(Some(CardEffect::Scry(count as usize)))
        }
        "loot" => {
            let count = draft
                .count
                .ok_or_else(|| anyhow!("Loot requires a count"))?;
            let amount = draft.amount.unwrap_or(1);
            Ok(Some(CardEffect::Loot(count as usize, amount as usize)))
        }
        "apply_stat_modifier" => {
            let stat = draft
                .stat
                .clone()
                .ok_or_else(|| anyhow!("Stat modifier requires a stat"))?;
            let amount = draft
                .amount
                .ok_or_else(|| anyhow!("Stat modifier requires an amount"))?;
            let expires = draft.expires.clone().unwrap_or(ExpireContract::EndOfTurn);
            let target = draft.target.clone().unwrap_or(EffectTargetSpec::Targets);
            Ok(Some(CardEffect::ApplyStatModifier(StatModifierSpec {
                stat,
                amount: ModifierAmount::Fixed(amount),
                expires,
                target,
            })))
        }
        "bounce" => Ok(Some(CardEffect::Bounce)),
        "board_wipe" => Ok(Some(CardEffect::BoardWipe)),
        "destroy_target_player_units" => Ok(Some(CardEffect::DestroyTargetPlayerUnits)),
        "damage_player_per_hand_card" => Ok(Some(CardEffect::DamagePlayerPerHandCard)),
        "destroy_units_with_toughness_at_least" => {
            let amount = draft
                .amount
                .ok_or_else(|| anyhow!("Filtered board wipe requires an amount"))?;
            Ok(Some(CardEffect::DestroyUnitsWithToughnessAtLeast(amount)))
        }
        "tap_target" => Ok(Some(CardEffect::TapTarget)),
        "destroy_target" => Ok(Some(CardEffect::DestroyTarget)),
        "exile_target" => Ok(Some(CardEffect::ExileTarget)),
        "equip" => Ok(Some(CardEffect::Equip)),
        "purify" => Ok(Some(CardEffect::Purify)),
        other => Err(anyhow!("Unsupported effect kind: {other}")),
    }
}

fn build_triggers(triggers: &[TriggerDraft]) -> Result<Vec<CardTrigger>> {
    let mut out = Vec::new();
    for draft in triggers {
        let effect = build_effect(&draft.effect)?;
        let has_event = draft.event.is_some();
        if effect.is_none() && !has_event {
            continue;
        }
        let event = draft
            .event
            .clone()
            .ok_or_else(|| anyhow!("Trigger event is required when adding a trigger effect"))?;
        let effect = effect.ok_or_else(|| anyhow!("Trigger effect is required"))?;
        out.push(CardTrigger {
            event,
            effect,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        });
    }
    Ok(out)
}

fn build_abilities(abilities: &[AbilityDraft]) -> Result<Vec<CardAbility>> {
    let mut out = Vec::new();
    for (index, draft) in abilities.iter().enumerate() {
        let effect = build_effect(&draft.effect)?;
        let name = draft.name.as_deref().unwrap_or("").trim().to_string();
        let has_cost = draft.cost.mana.total() > 0;

        if effect.is_none() && name.is_empty() && !has_cost {
            continue;
        }

        let effect =
            effect.ok_or_else(|| anyhow!("Ability effect is required when adding an ability"))?;
        if name.is_empty() {
            return Err(anyhow!("Ability name is required"));
        }

        let id_slug = slugify_name(&name);
        let id = if id_slug.is_empty() {
            format!("ability-{}", index + 1)
        } else {
            format!("ability-{}-{}", index + 1, id_slug)
        };

        let mut cost = AbilityCost::new().with_mana(draft.cost.mana.clone());
        if draft.cost.tap {
            cost = cost.with_tap();
        }
        if draft.cost.life > 0 {
            cost = cost.with_life(draft.cost.life);
        }
        if draft.cost.tap_other > 0 {
            cost = cost.with_cost(crate::card::Cost::TapOther(draft.cost.tap_other));
        }
        if draft.cost.sacrifice > 0 {
            cost = cost.with_cost(crate::card::Cost::Sacrifice(draft.cost.sacrifice));
        }

        out.push(CardAbility {
            id,
            name,
            cost,
            effect,
            target_spec: None,
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        });
    }
    Ok(out)
}

pub async fn register_routes(
    engine: Arc<GameEngine<CardGameState>>,
    prefix: &str,
) -> Result<CardGameActionIds> {
    let reorder_battlefield = engine
        .clone()
        .register_route(&format!("{}.reorder", prefix), |ctx| {
            let card_ids = ctx
                .get_input_as::<Vec<String>>("card_ids")
                .map_err(|_| anyhow!("Missing card_ids"))?;
            Ok(CardGameAction::ReorderBattlefield { card_ids })
        })
        .await?;

    let ping = engine
        .clone()
        .register_route(&format!("{}.ping", prefix), |ctx| {
            let x = ctx
                .get_input_as::<f32>("x")
                .map_err(|_| anyhow!("Missing x"))?;
            let z = ctx
                .get_input_as::<f32>("z")
                .map_err(|_| anyhow!("Missing z"))?;
            Ok(CardGameAction::Ping { x, z })
        })
        .await?;

    let view = engine
        .clone()
        .register_route(&format!("{}.view", prefix), |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            Ok(CardGameAction::View {
                name: user.display_name.clone(),
                instance_id: ctx.workflow.instance_id,
            })
        })
        .await?;

    let join = engine
        .clone()
        .register_route(&format!("{}.join", prefix), |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            Ok(CardGameAction::Join {
                name: user.display_name.clone(),
                avatar: full_avatar_url(&user),
                instance_id: ctx.workflow.instance_id,
            })
        })
        .await?;

    let add_ai = engine
        .clone()
        .register_route(&format!("{}.add_ai", prefix), |_| Ok(CardGameAction::AddAi))
        .await?;

    let set_format = engine
        .clone()
        .register_route(&format!("{}.set_format", prefix), |ctx| {
            let format = ctx
                .get_input_as::<GameFormat>("format")
                .map_err(|_| anyhow!("Missing format"))?;
            Ok(CardGameAction::SetGameFormat { format })
        })
        .await?;

    let set_mode = engine
        .clone()
        .register_route(&format!("{}.set_mode", prefix), |ctx| {
            let mode = ctx
                .get_input_as::<crate::types::GameMode>("mode")
                .map_err(|_| anyhow!("Missing mode"))?;
            Ok(CardGameAction::SetGameMode { mode })
        })
        .await?;

    let set_match_format = engine
        .clone()
        .register_route(&format!("{}.set_match_format", prefix), |ctx| {
            let format = ctx
                .get_input_as::<crate::types::MatchFormat>("format")
                .map_err(|_| anyhow!("Missing format"))?;
            Ok(CardGameAction::SetMatchFormat { format })
        })
        .await?;

    let submit_sideboard = engine
        .clone()
        .register_route(&format!("{}.submit_sideboard", prefix), |ctx| {
            let swaps = ctx
                .get_input_as::<Vec<crate::action::SideboardSwapEntry>>("swaps")
                .unwrap_or_default();
            Ok(CardGameAction::SubmitSideboard { swaps })
        })
        .await?;

    let start = engine
        .clone()
        .register_route(&format!("{}.start", prefix), |_| {
            Ok(CardGameAction::StartGame)
        })
        .await?;

    let select_deck = engine
        .clone()
        .register_route(&format!("{}.select_deck", prefix), |ctx| {
            let deck_id = ctx
                .get_input_as::<String>("deck_id")
                .map_err(|_| anyhow!("Missing deck_id"))?;
            Ok(CardGameAction::SelectDeck { deck_id })
        })
        .await?;

    let play_land = engine
        .clone()
        .register_route(&format!("{}.play_land", prefix), |ctx| {
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::PlayLand { card_id })
        })
        .await?;

    let deploy_node = engine
        .clone()
        .register_route(&format!("{}.deploy_node", prefix), |ctx| {
            let mana_type = ctx
                .get_input_as::<ManaType>("mana_type")
                .map_err(|_| anyhow!("Missing mana_type"))?;
            Ok(CardGameAction::DeployNode { mana_type })
        })
        .await?;

    let tap_card = engine
        .clone()
        .register_route(&format!("{}.tap", prefix), |ctx| {
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::TapCard { card_id })
        })
        .await?;

    let cast_spell = engine
        .clone()
        .register_route(&format!("{}.cast", prefix), |ctx| {
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            let targets = ctx
                .get_input_as::<Vec<TargetRef>>("targets")
                .unwrap_or_default();
            Ok(CardGameAction::CastSpell { card_id, targets })
        })
        .await?;

    let resolve_stack = engine
        .clone()
        .register_route(&format!("{}.resolve_stack", prefix), |_| {
            Ok(CardGameAction::ResolveStackTop)
        })
        .await?;

    let cancel_stack = engine
        .clone()
        .register_route(&format!("{}.cancel_stack", prefix), |_| {
            Ok(CardGameAction::CancelStackTop)
        })
        .await?;

    let pass_priority = engine
        .clone()
        .register_route(&format!("{}.pass_priority", prefix), |_| {
            Ok(CardGameAction::PassPriority)
        })
        .await?;

    let declare_attack = engine
        .clone()
        .register_route(&format!("{}.attack", prefix), |ctx| {
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            let target = ctx.get_input_as::<TargetRef>("target").ok();
            Ok(CardGameAction::DeclareAttack { card_id, target })
        })
        .await?;

    let declare_block = engine
        .clone()
        .register_route(&format!("{}.block", prefix), |ctx| {
            let attacker_id = ctx
                .get_input_as::<String>("attacker_id")
                .map_err(|_| anyhow!("Missing attacker_id"))?;
            let blocker_id = ctx
                .get_input_as::<String>("blocker_id")
                .map_err(|_| anyhow!("Missing blocker_id"))?;
            Ok(CardGameAction::DeclareBlock {
                attacker_id,
                blocker_id,
            })
        })
        .await?;

    let scry_commit = engine
        .clone()
        .register_route(&format!("{}.scry_commit", prefix), |ctx| {
            let top = ctx.get_input_as::<Vec<String>>("top").unwrap_or_default();
            let bottom = ctx
                .get_input_as::<Vec<String>>("bottom")
                .unwrap_or_default();
            Ok(CardGameAction::ScryCommit { top, bottom })
        })
        .await?;

    let search_commit = engine
        .clone()
        .register_route(&format!("{}.search_commit", prefix), |ctx| {
            let card_id = ctx.get_input_as::<String>("card_id").unwrap_or_default();
            Ok(CardGameAction::SearchCommit { card_id })
        })
        .await?;

    let search_cancel = engine
        .clone()
        .register_route(&format!("{}.search_cancel", prefix), |_| {
            Ok(CardGameAction::SearchCancel)
        })
        .await?;

    let trigger_choice = engine
        .clone()
        .register_route(&format!("{}.trigger_choice", prefix), |ctx| {
            let accept = ctx.get_input_as::<bool>("accept").unwrap_or(false);
            Ok(CardGameAction::TriggerChoice { accept })
        })
        .await?;

    let trigger_target_select = engine
        .clone()
        .register_route(&format!("{}.trigger_target_select", prefix), |ctx| {
            let target_type = ctx
                .get_input_as::<String>("target_type")
                .unwrap_or_default();
            let target_id = ctx.get_input_as::<String>("target_id").unwrap_or_default();
            let target = match target_type.as_str() {
                "player" => TargetRef::Player(target_id),
                _ => TargetRef::Card(target_id),
            };
            Ok(CardGameAction::TriggerTargetSelect { target })
        })
        .await?;

    let trigger_target_cancel = engine
        .clone()
        .register_route(&format!("{}.trigger_target_cancel", prefix), |_ctx| {
            Ok(CardGameAction::TriggerTargetCancel)
        })
        .await?;

    let discard_commit = engine
        .clone()
        .register_route(&format!("{}.discard_commit", prefix), |ctx| {
            let card_ids = ctx
                .get_input_as::<Vec<String>>("card_ids")
                .unwrap_or_default();
            Ok(CardGameAction::DiscardCommit { card_ids })
        })
        .await?;

    let discard = engine
        .clone()
        .register_route(&format!("{}.discard", prefix), |ctx| {
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::Discard { card_id })
        })
        .await?;

    let sacrifice = engine
        .clone()
        .register_route(&format!("{}.sacrifice", prefix), |ctx| {
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::Sacrifice { card_id })
        })
        .await?;

    let activate_ability = engine
        .clone()
        .register_route(&format!("{}.activate", prefix), |ctx| {
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            let ability_id = ctx.get_input_as::<String>("ability_id").map_err(|e| {
                let keys: Vec<_> = ctx.workflow.responses.keys().collect();
                let raw = ctx.workflow.responses.get("ability_id");
                tracing::error!(
                    "Missing ability_id: {:?}, response keys: {:?}, raw: {:?}",
                    e,
                    keys,
                    raw
                );
                anyhow!("Missing ability_id")
            })?;
            let targets = ctx
                .get_input_as::<Vec<TargetRef>>("targets")
                .unwrap_or_default();
            Ok(CardGameAction::ActivateAbility {
                card_id,
                ability_id,
                targets,
            })
        })
        .await?;

    let next_stage = engine
        .clone()
        .register_route(&format!("{}.next", prefix), |_| {
            Ok(CardGameAction::NextStage)
        })
        .await?;

    let confirm_attackers = engine
        .clone()
        .register_route(&format!("{}.confirm_attackers", prefix), |_| {
            Ok(CardGameAction::ConfirmAttackers)
        })
        .await?;

    let confirm_blockers = engine
        .clone()
        .register_route(&format!("{}.confirm_blockers", prefix), |_| {
            Ok(CardGameAction::ConfirmBlockers)
        })
        .await?;

    let save_card = engine
        .clone()
        .register_route(&format!("{}.save_card", prefix), |ctx| {
            let draft = ctx
                .get_input_as::<CardDraft>("card")
                .map_err(|e| anyhow!("Invalid card input: {e}"))?;
            let card = draft.into_card()?;
            Ok(CardGameAction::SaveCard { card })
        })
        .await?;

    let list_cards = engine
        .clone()
        .register_route(&format!("{}.list_cards", prefix), |ctx| {
            let pagination = ctx
                .get_input_as::<PaginationArgs>("pagination")
                .unwrap_or_else(|_| PaginationArgs {
                    before: None,
                    after: None,
                    first: Some(20),
                    last: None,
                });
            Ok(CardGameAction::ListCards { pagination })
        })
        .await?;

    let builder_load_deck = engine
        .clone()
        .register_route(&format!("{}.builder_load_deck", prefix), |ctx| {
            let deck_id = ctx
                .get_input_as::<String>("deck_id")
                .map_err(|_| anyhow!("Missing deck_id"))?;
            Ok(CardGameAction::BuilderLoadDeck { deck_id })
        })
        .await?;

    let builder_add_card = engine
        .clone()
        .register_route(&format!("{}.builder_add_card", prefix), |ctx| {
            let deck_id = ctx
                .get_input_as::<String>("deck_id")
                .map_err(|_| anyhow!("Missing deck_id"))?;
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::BuilderAddCard { deck_id, card_id })
        })
        .await?;

    let builder_remove_card = engine
        .clone()
        .register_route(&format!("{}.builder_remove_card", prefix), |ctx| {
            let deck_id = ctx
                .get_input_as::<String>("deck_id")
                .map_err(|_| anyhow!("Missing deck_id"))?;
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::BuilderRemoveCard { deck_id, card_id })
        })
        .await?;

    let builder_add_sideboard_card = engine
        .clone()
        .register_route(&format!("{}.builder_add_sideboard_card", prefix), |ctx| {
            let deck_id = ctx
                .get_input_as::<String>("deck_id")
                .map_err(|_| anyhow!("Missing deck_id"))?;
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::BuilderAddSideboardCard { deck_id, card_id })
        })
        .await?;

    let builder_remove_sideboard_card = engine
        .clone()
        .register_route(
            &format!("{}.builder_remove_sideboard_card", prefix),
            |ctx| {
                let deck_id = ctx
                    .get_input_as::<String>("deck_id")
                    .map_err(|_| anyhow!("Missing deck_id"))?;
                let card_id = ctx
                    .get_input_as::<String>("card_id")
                    .map_err(|_| anyhow!("Missing card_id"))?;
                Ok(CardGameAction::BuilderRemoveSideboardCard { deck_id, card_id })
            },
        )
        .await?;

    let builder_set_operator = engine
        .clone()
        .register_route(&format!("{}.builder_set_operator", prefix), |ctx| {
            let deck_id = ctx
                .get_input_as::<String>("deck_id")
                .map_err(|_| anyhow!("Missing deck_id"))?;
            let card_id = ctx
                .get_input_as::<String>("card_id")
                .map_err(|_| anyhow!("Missing card_id"))?;
            Ok(CardGameAction::BuilderSetOperator { deck_id, card_id })
        })
        .await?;

    let builder_clone_deck = engine
        .clone()
        .register_route(&format!("{}.builder_clone_deck", prefix), |ctx| {
            let deck_id = ctx
                .get_input_as::<String>("deck_id")
                .map_err(|_| anyhow!("Missing deck_id"))?;
            Ok(CardGameAction::BuilderCloneDeck { deck_id })
        })
        .await?;

    let builder_set_filter = engine
        .clone()
        .register_route(&format!("{}.builder_set_filter", prefix), |ctx| {
            let kind = ctx.get_input_as::<Option<String>>("kind").unwrap_or(None);
            Ok(CardGameAction::BuilderSetFilter { kind })
        })
        .await?;

    let builder_set_sort = engine
        .clone()
        .register_route(&format!("{}.builder_set_sort", prefix), |ctx| {
            let sort = ctx.get_input_as::<Option<String>>("sort").unwrap_or(None);
            Ok(CardGameAction::BuilderSetSort { sort })
        })
        .await?;

    let builder_open = engine
        .clone()
        .register_route(&format!("{}.builder_open", prefix), |_| {
            Ok(CardGameAction::BuilderOpen)
        })
        .await?;

    let builder_close = engine
        .clone()
        .register_route(&format!("{}.builder_close", prefix), |_| {
            Ok(CardGameAction::BuilderClose)
        })
        .await?;

    let builder_create_deck = engine
        .clone()
        .register_route(&format!("{}.builder_create_deck", prefix), |_| {
            Ok(CardGameAction::BuilderCreateDeck)
        })
        .await?;

    let builder_autobuild = engine
        .clone()
        .register_route(&format!("{}.builder_autobuild", prefix), |_| {
            Ok(CardGameAction::BuilderAutobuild)
        })
        .await?;

    let browser_open = engine
        .clone()
        .register_route(&format!("{}.browser_open", prefix), |ctx| {
            let target_player_id = ctx
                .get_input_as::<Option<String>>("target_player_id")
                .unwrap_or(None);
            Ok(CardGameAction::BrowserOpen { target_player_id })
        })
        .await?;

    let browser_close = engine
        .clone()
        .register_route(&format!("{}.browser_close", prefix), |_| {
            Ok(CardGameAction::BrowserClose)
        })
        .await?;

    let browser_set_filter = engine
        .clone()
        .register_route(&format!("{}.browser_set_filter", prefix), |ctx| {
            let kind = ctx.get_input_as::<Option<String>>("kind").unwrap_or(None);
            Ok(CardGameAction::BrowserSetFilter { kind })
        })
        .await?;

    let browser_set_sort = engine
        .clone()
        .register_route(&format!("{}.browser_set_sort", prefix), |ctx| {
            let sort = ctx.get_input_as::<Option<String>>("sort").unwrap_or(None);
            Ok(CardGameAction::BrowserSetSort { sort })
        })
        .await?;

    let browser_page = engine
        .clone()
        .register_route(&format!("{}.browser_page", prefix), |ctx| {
            let delta = ctx.get_input_as::<i32>("delta").unwrap_or(1);
            Ok(CardGameAction::BrowserPage { delta })
        })
        .await?;

    let debug_setup = engine
        .clone()
        .register_route(&format!("{}.debug_setup", prefix), |_| {
            Ok(CardGameAction::DebugSetup)
        })
        .await?;

    let debug_ai_concede = engine
        .clone()
        .register_route(&format!("{}.debug_ai_concede", prefix), |_| {
            Ok(CardGameAction::DebugAiConcede)
        })
        .await?;

    let list_pack_types = engine
        .clone()
        .register_route(&format!("{}.list_pack_types", prefix), |_| {
            Ok(CardGameAction::ListPackTypes)
        })
        .await?;

    let get_user_packs = engine
        .clone()
        .register_route(&format!("{}.get_user_packs", prefix), |_| {
            Ok(CardGameAction::GetUserPacks)
        })
        .await?;

    let open_pack = engine
        .clone()
        .register_route(&format!("{}.open_pack", prefix), |ctx| {
            let pack_instance_id = ctx
                .get_input_as::<String>("pack_instance_id")
                .map_err(|_| anyhow!("Missing pack_instance_id"))?;
            Ok(CardGameAction::OpenPack { pack_instance_id })
        })
        .await?;

    let get_pack_opening_results = engine
        .clone()
        .register_route(&format!("{}.get_pack_opening_results", prefix), |ctx| {
            let pack_instance_id = ctx
                .get_input_as::<String>("pack_instance_id")
                .map_err(|_| anyhow!("Missing pack_instance_id"))?;
            Ok(CardGameAction::GetPackOpeningResults { pack_instance_id })
        })
        .await?;

    let admin_grant_pack = engine
        .clone()
        .register_route(&format!("{}.admin_grant_pack", prefix), |ctx| {
            let user_id = ctx
                .get_input_as::<String>("user_id")
                .map_err(|_| anyhow!("Missing user_id"))?;
            let pack_type_id = ctx
                .get_input_as::<String>("pack_type_id")
                .map_err(|_| anyhow!("Missing pack_type_id"))?;
            Ok(CardGameAction::AdminGrantPack {
                user_id,
                pack_type_id,
            })
        })
        .await?;

    let reset = engine
        .clone()
        .register_route(&format!("{}.reset", prefix), |_| Ok(CardGameAction::Reset))
        .await?;

    // Draft routes
    let draft_start = engine
        .clone()
        .register_route(&format!("{}.draft_start", prefix), |_| {
            Ok(CardGameAction::DraftStart)
        })
        .await?;

    let draft_pick = engine
        .clone()
        .register_route(&format!("{}.draft_pick", prefix), |ctx| {
            let index = ctx
                .get_input_as::<usize>("index")
                .map_err(|_| anyhow!("Missing index"))?;
            Ok(CardGameAction::DraftPick { index })
        })
        .await?;

    let draft_auto_pick = engine
        .clone()
        .register_route(&format!("{}.draft_auto_pick", prefix), |_| {
            Ok(CardGameAction::DraftAutoPick)
        })
        .await?;

    let draft_confirm_pool = engine
        .clone()
        .register_route(&format!("{}.draft_confirm_pool", prefix), |_| {
            Ok(CardGameAction::DraftConfirmPool)
        })
        .await?;

    let draft_keep_cards = engine
        .clone()
        .register_route(&format!("{}.draft_keep_cards", prefix), |ctx| {
            let card_ids = ctx
                .get_input_as::<Vec<String>>("card_ids")
                .map_err(|_| anyhow!("Missing card_ids"))?;
            Ok(CardGameAction::DraftKeepCards { card_ids })
        })
        .await?;

    Ok(CardGameActionIds {
        view,
        add_ai,
        join,
        set_format,
        set_mode,
        set_match_format,
        submit_sideboard,
        select_deck,
        start,
        play_land,
        deploy_node,
        tap_card,
        cast_spell,
        activate_ability,
        resolve_stack,
        cancel_stack,
        pass_priority,
        declare_attack,
        declare_block,
        scry_commit,
        search_commit,
        search_cancel,
        trigger_choice,
        trigger_target_select,
        trigger_target_cancel,
        discard_commit,
        discard,
        sacrifice,
        next_stage,
        confirm_attackers,
        confirm_blockers,
        ping,
        reorder_battlefield,
        save_card,
        list_cards,
        builder_load_deck,
        builder_add_card,
        builder_remove_card,
        builder_add_sideboard_card,
        builder_remove_sideboard_card,
        builder_set_operator,
        builder_clone_deck,
        builder_set_filter,
        builder_set_sort,
        builder_open,
        builder_close,
        builder_create_deck,
        builder_autobuild,
        browser_open,
        browser_close,
        browser_set_filter,
        browser_set_sort,
        browser_page,
        debug_setup,
        debug_ai_concede,
        list_pack_types,
        get_user_packs,
        open_pack,
        get_pack_opening_results,
        admin_grant_pack,
        reset,
        draft_start,
        draft_pick,
        draft_auto_pick,
        draft_confirm_pool,
        draft_keep_cards,
    })
}
