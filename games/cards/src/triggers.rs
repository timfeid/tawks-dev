//! Trigger handling and event processing

use crate::card::{Card, CardEffect, CardTrigger};
use crate::state::*;
use crate::types::*;

/// Trigger handling functions
impl CardGameState {
    fn trigger_debug(&self, message: impl AsRef<str>) {
        if std::env::var("CARDGAME_SIM_DEBUG")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            eprintln!("[triggers] {}", message.as_ref());
        }
    }

    pub fn apply_all_battlefield_triggers_for_event(
        &mut self,
        event: &TriggerEvent,
        targets: &[TargetRef],
    ) {
        let battlefield_cards: Vec<(String, String, String, Vec<CardTrigger>)> = self
            .players
            .iter()
            .flat_map(|(pid, player)| {
                player
                    .battlefield
                    .iter()
                    .map(|card| {
                        (
                            pid.clone(),
                            card.id.clone(),
                            card.name.clone(),
                            card.triggers.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        for (pid, cid, name, triggers) in battlefield_cards {
            self.apply_triggers_for_event(&pid, &cid, Some(&name), &triggers, event, targets);
        }
    }

    pub fn apply_battlefield_triggers_for_player_event(
        &mut self,
        player_id: &str,
        event: &TriggerEvent,
        targets: &[TargetRef],
    ) {
        let battlefield_cards: Vec<(String, String, String, Vec<CardTrigger>)> = self
            .players
            .get(player_id)
            .map(|player| {
                player
                    .battlefield
                    .iter()
                    .map(|card| {
                        (
                            player_id.to_string(),
                            card.id.clone(),
                            card.name.clone(),
                            card.triggers.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        for (pid, cid, name, triggers) in battlefield_cards {
            self.apply_triggers_for_event(&pid, &cid, Some(&name), &triggers, event, targets);
        }
    }

    pub fn apply_battlefield_triggers_for_event_except_player(
        &mut self,
        excluded_player_id: &str,
        event: &TriggerEvent,
        targets: &[TargetRef],
    ) {
        let battlefield_cards: Vec<(String, String, String, Vec<CardTrigger>)> = self
            .players
            .iter()
            .filter(|(pid, _)| pid.as_str() != excluded_player_id)
            .flat_map(|(pid, player)| {
                player
                    .battlefield
                    .iter()
                    .map(|card| {
                        (
                            pid.clone(),
                            card.id.clone(),
                            card.name.clone(),
                            card.triggers.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        for (pid, cid, name, triggers) in battlefield_cards {
            self.apply_triggers_for_event(&pid, &cid, Some(&name), &triggers, event, targets);
        }
    }

    pub fn trigger_key(card_id: &str, event: &TriggerEvent) -> String {
        format!("trigger::{}::{:?}", card_id, event)
    }

    /// Returns the TargetKind needed if this effect requires target selection.
    fn effect_target_kind(effect: &CardEffect) -> Option<TargetKind> {
        match effect {
            CardEffect::DamagePlayer(_) => Some(TargetKind::Player),
            CardEffect::DamageUnit(_) => Some(TargetKind::Unit),
            CardEffect::DamageTarget(_) => Some(TargetKind::PlayerOrUnit),
            CardEffect::Bounce => Some(TargetKind::Card),
            CardEffect::Freeze(_) => Some(TargetKind::Unit),
            CardEffect::TapTarget => Some(TargetKind::Unit),
            CardEffect::DestroyTarget => Some(TargetKind::Card),
            CardEffect::ExileTarget => Some(TargetKind::Card),
            CardEffect::CopyUnit => Some(TargetKind::Unit),
            CardEffect::Siphon(_) => Some(TargetKind::Player),
            CardEffect::Swap => Some(TargetKind::Unit),
            CardEffect::Phase => Some(TargetKind::Card),
            CardEffect::MutateTarget(_) => Some(TargetKind::Unit),
            CardEffect::Polymorph => Some(TargetKind::Unit),
            CardEffect::DamageSplit(_) => Some(TargetKind::PlayerOrUnit),
            CardEffect::Reflect(_) => Some(TargetKind::Unit),
            CardEffect::Conscript => Some(TargetKind::Unit),
            CardEffect::Scavenge(_) => Some(TargetKind::Unit),
            CardEffect::Assimilate(_) => Some(TargetKind::Unit),
            // GrantKeyword in a trigger context applies team-wide (all units
            // you control) — effects.rs handles the empty-targets case. The
            // auto-description matches this ("Units you control gain ...").
            // Recurse into Sequence to find the first effect that needs a target
            CardEffect::Sequence(effects) => {
                effects.iter().find_map(|e| Self::effect_target_kind(e))
            }
            _ => None,
        }
    }

    /// Tries to auto-resolve a single target when there's only one valid choice.
    fn auto_resolve_target(&self, owner_id: &str, target_kind: &TargetKind) -> Option<TargetRef> {
        match target_kind {
            TargetKind::Player => {
                let opponents: Vec<_> = self
                    .players
                    .keys()
                    .filter(|id| id.as_str() != owner_id)
                    .collect();
                if opponents.len() == 1 {
                    Some(TargetRef::Player(opponents[0].clone()))
                } else {
                    None
                }
            }
            _ => None, // Unit/mixed targets always require selection
        }
    }

    pub fn apply_triggers_for_event(
        &mut self,
        player_id: &str,
        source_id: &str,
        source_name: Option<&str>,
        triggers: &[CardTrigger],
        event: &TriggerEvent,
        targets: &[TargetRef],
    ) {
        for trigger in triggers {
            if &trigger.event == event {
                self.trigger_debug(format!(
                    "event={:?} player={} source={} effect={:?} targets={:?}",
                    event, player_id, source_id, trigger.effect, targets
                ));
                if let Some(predicate) = &trigger.predicate {
                    // Check predicate - for enter/leave battlefield events, check against the source card itself
                    let predicate_targets =
                        if matches!(event, TriggerEvent::Entered | TriggerEvent::Left) {
                            vec![TargetRef::Card(source_id.to_string())]
                        } else {
                            targets.to_vec()
                        };
                    if !self.evaluate_predicate(
                        player_id,
                        Some(source_id),
                        &predicate_targets,
                        predicate,
                    ) {
                        continue;
                    }
                }

                if trigger.limit == TriggerLimit::OncePerTurn {
                    let key = Self::predicate_key(source_id, &format!("{:?}", event));
                    if self.triggers_played_this_turn.contains_key(&key) {
                        continue;
                    }
                    self.triggers_played_this_turn.insert(key, 1);
                }

                let can_pay_optional = if let Some(cost) = &trigger.optional_cost {
                    let player = self.players.get(player_id).unwrap();
                    player.mana_pool.can_afford(&cost.get_mana()) && player.life > cost.get_life()
                } else {
                    true
                };
                if !can_pay_optional {
                    continue;
                }

                if trigger.is_optional {
                    // "May do X with target" triggers (no optional_cost, effect
                    // needs a target) go straight to a cancellable target
                    // selection — the cancel button is the "no" answer. All
                    // other optional triggers keep the Accept/Decline modal.
                    let needs_target = targets.is_empty()
                        && trigger.optional_cost.is_none()
                        && Self::effect_target_kind(&trigger.effect).is_some()
                        && !matches!(trigger.effect, CardEffect::AddCounters(_));
                    if needs_target {
                        let target_kind = Self::effect_target_kind(&trigger.effect).unwrap();
                        let name = source_name
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| source_id.to_string());
                        if let Some(player) = self.players.get_mut(player_id) {
                            player
                                .pending_actions
                                .push(PendingAction::TriggerTargetSelection {
                                    source_card_id: source_id.to_string(),
                                    source_card_name: name,
                                    effect: trigger.effect.clone(),
                                    target_kind,
                                    optional: true,
                                });
                        }
                        continue;
                    }

                    if let Some(player) = self.players.get_mut(player_id) {
                        let name = source_name
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| source_id.to_string());
                        player.pending_actions.push(PendingAction::TriggerChoice {
                            source_card_id: source_id.to_string(),
                            source_card_name: name,
                            effect: trigger.effect.clone(),
                            targets: targets.to_vec(),
                            optional_cost: trigger.optional_cost.clone(),
                        });
                    }
                    continue;
                }

                if let Some(cost) = &trigger.optional_cost {
                    let player = self.players.get_mut(player_id).unwrap();
                    player.mana_pool.spend(&cost.get_mana());
                    player.life -= cost.get_life();
                }

                // `DamageTarget` is a free-choice "any target" effect. In a trigger
                // context the event-provided targets (e.g. the discarder) are context
                // used by predicates, not the intended damage target — always prompt
                // the trigger owner for a fresh selection.
                let effect_targets: &[TargetRef] =
                    if matches!(trigger.effect, CardEffect::DamageTarget(_)) {
                        &[]
                    } else {
                        targets
                    };

                // Check if the effect needs target selection
                if effect_targets.is_empty() {
                    // Self-targeting effects (like AddCounters) use the source card as target
                    if matches!(trigger.effect, CardEffect::AddCounters(_)) {
                        let name = source_name.unwrap_or(source_id).to_string();
                        self.trigger_log.push(TriggerLogEntry {
                            source_card_name: name,
                            source_card_id: source_id.to_string(),
                            effect_description: Self::effect_text(&trigger.effect, None),
                            owner_id: player_id.to_string(),
                        });
                        self.apply_effect_with_source(
                            player_id,
                            Some(source_id),
                            &trigger.effect,
                            &[TargetRef::Card(source_id.to_string())],
                        );
                        continue;
                    }
                    if let Some(target_kind) = Self::effect_target_kind(&trigger.effect) {
                        if let Some(auto_target) = self.auto_resolve_target(player_id, &target_kind)
                        {
                            let name = source_name.unwrap_or(source_id).to_string();
                            self.trigger_log.push(TriggerLogEntry {
                                source_card_name: name,
                                source_card_id: source_id.to_string(),
                                effect_description: Self::effect_text(&trigger.effect, None),
                                owner_id: player_id.to_string(),
                            });
                            self.apply_effect_with_source(
                                player_id,
                                Some(source_id),
                                &trigger.effect,
                                &[auto_target],
                            );
                        } else {
                            let name = source_name
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| source_id.to_string());
                            if let Some(player) = self.players.get_mut(player_id) {
                                player.pending_actions.push(
                                    PendingAction::TriggerTargetSelection {
                                        source_card_id: source_id.to_string(),
                                        source_card_name: name,
                                        effect: trigger.effect.clone(),
                                        target_kind,
                                        optional: false,
                                    },
                                );
                            }
                        }
                        continue;
                    }
                }

                let name = source_name.unwrap_or(source_id).to_string();
                self.trigger_log.push(TriggerLogEntry {
                    source_card_name: name,
                    source_card_id: source_id.to_string(),
                    effect_description: Self::effect_text(&trigger.effect, None),
                    owner_id: player_id.to_string(),
                });
                self.apply_effect_with_source(
                    player_id,
                    Some(source_id),
                    &trigger.effect,
                    effect_targets,
                );
            }
        }
    }

    fn handle_card_entry(
        &mut self,
        owner_id: &str,
        card: &Card,
        targets: &[TargetRef],
        counts_as_played: bool,
    ) {
        self.trigger_debug(format!(
            "entered owner={} card={} name={} played={} targets={:?}",
            owner_id, card.id, card.name, counts_as_played, targets
        ));

        // Modular(N): enters with N +1/+1 counters. Applied before triggers and
        // state-based checks so a 0/0 Modular unit doesn't die immediately.
        let modular_amount = card.keywords.iter().find_map(|kw| {
            if let Keyword::Modular(n) = kw {
                Some(*n)
            } else {
                None
            }
        });
        if let Some(amount) = modular_amount
            && amount > 0
            && let Some(owner) = self.players.get_mut(owner_id)
            && let Some(on_bf) = owner.battlefield.iter_mut().find(|c| c.id == card.id)
        {
            on_bf.counters = on_bf.counters.saturating_add(amount);
        }

        // Echo(X): mark the unit as owing the echo tax at the owner's
        // next upkeep. `game_flow::start_turn` reads `echo_pending` and
        // forces a pay-or-sacrifice decision.
        let has_echo = card.keywords.iter().any(|k| matches!(k, Keyword::Echo(_)));
        if has_echo
            && let Some(owner) = self.players.get_mut(owner_id)
            && let Some(on_bf) = owner.battlefield.iter_mut().find(|c| c.id == card.id)
        {
            on_bf.echo_pending = true;
        }

        self.apply_triggers_for_event(
            owner_id,
            &card.id,
            Some(&card.name),
            &card.triggers,
            &TriggerEvent::Entered,
            targets,
        );

        // Check for "OnOtherUnitPlayed" triggers only for cards that were actually played.
        if counts_as_played && let CardKind::Unit(entered_type) = &card.kind {
            // Collect triggers first to avoid borrowing issues
            let mut other_triggers = Vec::new();
            for (pid, player) in &self.players {
                for other_card in &player.battlefield {
                    if other_card.id == card.id {
                        continue;
                    }
                    other_triggers.push((
                        pid.clone(),
                        other_card.id.clone(),
                        other_card.name.clone(),
                        other_card.triggers.clone(),
                    ));
                }
            }

            // Pass the mover's card id in targets so trigger predicates
            // (e.g. `TargetIsApex`) can filter on the entering card.
            let mover_targets = vec![TargetRef::Card(card.id.clone())];
            for (pid, cid, name, triggers) in other_triggers {
                self.apply_triggers_for_event(
                    &pid,
                    &cid,
                    Some(&name),
                    &triggers,
                    &TriggerEvent::AllyEntered(Some(entered_type.clone())),
                    &mover_targets,
                );
                self.apply_triggers_for_event(
                    &pid,
                    &cid,
                    Some(&name),
                    &triggers,
                    &TriggerEvent::AllyEntered(None),
                    &mover_targets,
                );
            }

            // Evolve: when a unit with Evolve sees an ally enter with
            // greater base power or toughness, put a +1/+1 counter on
            // the Evolve unit. Only the owner's own units trigger.
            let entering_power = card.power.unwrap_or(0);
            let entering_toughness = card.toughness.unwrap_or(0);
            let evolve_targets: Vec<String> = self
                .players
                .get(owner_id)
                .map(|p| {
                    p.battlefield
                        .iter()
                        .filter(|c| {
                            c.id != card.id
                                && c.keywords.contains(&Keyword::Evolve)
                                && matches!(c.kind, CardKind::Unit(_))
                                && (entering_power > c.power.unwrap_or(0) + c.counters
                                    || entering_toughness > c.toughness.unwrap_or(0) + c.counters)
                        })
                        .map(|c| c.id.clone())
                        .collect()
                })
                .unwrap_or_default();
            for cid in evolve_targets {
                if let Some(owner) = self.players.get_mut(owner_id)
                    && let Some(c) = owner.battlefield.iter_mut().find(|c| c.id == cid)
                {
                    c.counters = c.counters.saturating_add(1);
                }
            }
        }

        // TokenCreated: fires on owner's other battlefield cards when a token
        // enters. Tokens are identified by having no template_id.
        if card.is_token() {
            let other_triggers: Vec<_> = self
                .players
                .get(owner_id)
                .map(|p| {
                    p.battlefield
                        .iter()
                        .filter(|c| c.id != card.id)
                        .map(|c| (c.id.clone(), c.name.clone(), c.triggers.clone()))
                        .collect()
                })
                .unwrap_or_default();
            for (cid, name, triggers) in other_triggers {
                self.apply_triggers_for_event(
                    owner_id,
                    &cid,
                    Some(&name),
                    &triggers,
                    &TriggerEvent::TokenCreated,
                    &[],
                );
            }
        }

        if card.supertype == CardSupertype::Apex {
            let mut duplicates = Vec::new();
            if let Some(owner) = self.players.get(owner_id) {
                for other in &owner.battlefield {
                    if other.id == card.id {
                        continue;
                    }
                    if other.supertype == CardSupertype::Apex && other.name == card.name {
                        duplicates.push(other.id.clone());
                    }
                }
            }
            for other_id in duplicates {
                self.remove_from_battlefield_and_attachments(owner_id, &other_id);
            }
        }
    }

    pub fn handle_card_entered(&mut self, owner_id: &str, card: &Card, targets: &[TargetRef]) {
        self.handle_card_entry(owner_id, card, targets, false);
    }

    pub fn handle_card_played(&mut self, owner_id: &str, card: &Card, targets: &[TargetRef]) {
        self.handle_card_entry(owner_id, card, targets, true);
    }
}
