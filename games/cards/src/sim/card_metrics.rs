//! Per-card telemetry collector for the sim.
//!
//! Watches successive game states and infers what happened to each card:
//! - Cast events (new stack/showcase item whose card_id wasn't there before).
//! - ETB / graveyard / exile / hand-return events (battlefield diff with
//!   graveyard/exile/hand cross-check to classify "where did it go").
//! - Ability activations and damage dealt.
//!
//! Aggregated per match and returned in `MatchReport.card_metrics`, keyed
//! by catalog `template_id`. That keeps rows aggregatable across matches
//! in a series/matrix (multiple copies of the same card collapse into one
//! row per `(template_id, side)`).
//!
//! "side" is `"winner"` or `"loser"` so downstream analysis can answer
//! "does X show up more in won games than lost?" without extra joins.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::state::{CardGameState, StackItem, StackItemKind};

use super::report::CardMetrics;

#[derive(Clone, Debug, Default)]
struct InstanceRecord {
    template_id: String,
    name: String,
    owner_id: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CardMetricsCollector {
    /// Per-instance cache so we can recognize instances across ticks.
    instances: HashMap<String, InstanceRecord>,
    /// (owner, template_id) → accumulated counters.
    counters: HashMap<(String, String), CardMetrics>,
    /// Stack+showcase instance ids seen last tick.
    prev_stack: BTreeSet<String>,
    /// Battlefield instance ids seen last tick (per player).
    prev_battlefield: BTreeMap<String, BTreeSet<String>>,
    /// Graveyard instance ids seen last tick (per player).
    prev_graveyard: BTreeMap<String, BTreeSet<String>>,
    /// Exile instance ids seen last tick (per player).
    prev_exile: BTreeMap<String, BTreeSet<String>>,
    /// Hand instance ids seen last tick (per player).
    prev_hand: BTreeMap<String, BTreeSet<String>>,
    /// Previous life totals for attributing damage/gain.
    prev_life: BTreeMap<String, i32>,
    /// Previous hand sizes per player, for attributing card-draw.
    prev_hand_size: BTreeMap<String, usize>,
    /// Previous damage counter per battlefield unit (instance id → damage).
    prev_unit_damage: BTreeMap<String, i32>,
    /// Previous battlefield-card ids per player (for token creation).
    prev_known_bf_ids: BTreeSet<String>,
    /// Triggers_played_this_turn lengths for detecting ability activations.
    prev_triggers_played_total: usize,
    /// Per-key counts of `triggers_played_this_turn` so we can
    /// attribute new activations to their source card by parsing the
    /// key (`ability::<card_id>::<ability_id>` or `agent::<card_id>`).
    prev_triggers_by_key: BTreeMap<String, usize>,
    /// Turn counter tracked via state.round; used to increment the
    /// per-card "rounds on battlefield" stat once per turn.
    last_round_seen: Option<usize>,
    /// Whether we've initialized from the first state snapshot.
    initialized: bool,
}

impl CardMetricsCollector {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Seed instance records from an initial state (opening hands, decks).
    pub(crate) fn record_initial_decks(&mut self, state: &CardGameState) {
        for (pid, player) in &state.players {
            // Count copies of each template in the starting library + hand.
            let mut deck_counts: HashMap<String, (String, usize)> = HashMap::new();
            for card in player
                .deck
                .iter()
                .chain(player.hand.iter())
                .chain(player.graveyard.iter())
                .chain(player.battlefield.iter())
                .chain(player.operator_zone.iter())
                .chain(player.exile.iter())
            {
                self.remember_instance(card, pid);
                let Some(tpl) = card.template_id.clone() else {
                    continue;
                };
                deck_counts
                    .entry(tpl)
                    .or_insert_with(|| (card.name.clone(), 0))
                    .1 += 1;
            }
            for (tpl, (name, qty)) in deck_counts {
                let slot = self
                    .counters
                    .entry((pid.clone(), tpl.clone()))
                    .or_insert_with(|| CardMetrics {
                        card_id: tpl.clone(),
                        card_name: name.clone(),
                        owner_deck_id: player.deck_id.clone().unwrap_or_default(),
                        side: String::new(),
                        ..Default::default()
                    });
                slot.copies_in_deck = qty;
                if slot.card_name.is_empty() {
                    slot.card_name = name;
                }
                if slot.owner_deck_id.is_empty() {
                    slot.owner_deck_id = player.deck_id.clone().unwrap_or_default();
                }
            }

            // Opening-hand presence: one increment per unique template
            // seen in the starting hand (so we measure "did it show up
            // in opener" not "how many copies were in it").
            let mut opener_seen: BTreeSet<String> = BTreeSet::new();
            for card in &player.hand {
                if let Some(tpl) = card.template_id.clone() {
                    if opener_seen.insert(tpl.clone()) {
                        if let Some(slot) = self.counters.get_mut(&(pid.clone(), tpl)) {
                            slot.in_opening_hand += 1;
                        }
                    }
                }
            }
        }
    }

    /// Observe a state after an action has been applied and update counters.
    pub(crate) fn observe(&mut self, state: &CardGameState) {
        // Refresh instance memory with anything new we've seen.
        for (pid, player) in &state.players {
            for card in player
                .deck
                .iter()
                .chain(player.hand.iter())
                .chain(player.graveyard.iter())
                .chain(player.battlefield.iter())
                .chain(player.operator_zone.iter())
                .chain(player.exile.iter())
            {
                self.remember_instance(card, pid);
            }
            for item in &state.stack {
                if let StackItemKind::Spell(card) = &item.kind {
                    self.remember_instance(card, &item.owner_id);
                }
            }
            if let Some(entry) = &state.showcase {
                self.remember_instance(&entry.card, &entry.owner_id);
            }
            let _ = pid;
        }

        if !self.initialized {
            self.snapshot_prev(state);
            self.initialized = true;
            return;
        }

        let cur_stack = current_stack_ids(state);
        let cur_battlefield = current_battlefield_ids(state);
        let cur_graveyard = current_graveyard_ids(state);
        let cur_exile = current_exile_ids(state);
        let cur_hand = current_hand_ids(state);
        let cur_life: BTreeMap<String, i32> = state
            .players
            .iter()
            .map(|(pid, p)| (pid.clone(), p.life))
            .collect();

        // Collect diff events first (immutable borrow of self), then apply
        // the mutations in a second pass (mutable borrow). Splitting the
        // two phases keeps the borrow checker happy.
        #[derive(Debug)]
        enum Event {
            Cast(InstanceRecord),
            Countered(InstanceRecord),
            Entered(InstanceRecord),
            Destroyed(InstanceRecord),
            Exiled(InstanceRecord),
            Bounced(InstanceRecord),
            Discarded(InstanceRecord),
            Milled(InstanceRecord),
            Damage(InstanceRecord, i32),
            UnitDamage(InstanceRecord, i32),
            TokenCreated(InstanceRecord),
            CardsDrawn(InstanceRecord, usize),
            LifeGained(InstanceRecord, i32),
            Survived(InstanceRecord),
            Ability(InstanceRecord),
        }
        let mut events: Vec<Event> = Vec::new();

        // Casts: instances present in cur_stack that weren't in prev_stack.
        for iid in cur_stack.difference(&self.prev_stack) {
            if let Some(meta) = self.instances.get(iid).cloned() {
                events.push(Event::Cast(meta));
            }
        }

        // Counters: instances that left the stack WITHOUT entering the
        // battlefield, graveyard, or hand afterward.
        for iid in self.prev_stack.difference(&cur_stack) {
            let Some(meta) = self.instances.get(iid).cloned() else {
                continue;
            };
            let in_bf = owner_collection_contains(&cur_battlefield, &meta.owner_id, iid);
            let in_grave = owner_collection_contains(&cur_graveyard, &meta.owner_id, iid);
            let in_hand = owner_collection_contains(&cur_hand, &meta.owner_id, iid);
            let in_exile = owner_collection_contains(&cur_exile, &meta.owner_id, iid);
            if !in_bf && !in_grave && !in_hand && !in_exile {
                events.push(Event::Countered(meta));
            }
        }

        // Battlefield diffs: entered or left.
        for (pid, cur_ids) in &cur_battlefield {
            let empty = BTreeSet::new();
            let prev_ids = self.prev_battlefield.get(pid).unwrap_or(&empty);
            for iid in cur_ids.difference(prev_ids) {
                if let Some(meta) = self.instances.get(iid).cloned() {
                    events.push(Event::Entered(meta));
                }
            }
            for iid in prev_ids.difference(cur_ids) {
                let Some(meta) = self.instances.get(iid).cloned() else {
                    continue;
                };
                let in_grave = owner_collection_contains(&cur_graveyard, &meta.owner_id, iid);
                let in_exile = owner_collection_contains(&cur_exile, &meta.owner_id, iid);
                let in_hand = owner_collection_contains(&cur_hand, &meta.owner_id, iid);
                let prev_grave = self
                    .prev_graveyard
                    .get(&meta.owner_id)
                    .map(|s| s.contains(iid))
                    .unwrap_or(false);
                let prev_exile = self
                    .prev_exile
                    .get(&meta.owner_id)
                    .map(|s| s.contains(iid))
                    .unwrap_or(false);
                let prev_hand = self
                    .prev_hand
                    .get(&meta.owner_id)
                    .map(|s| s.contains(iid))
                    .unwrap_or(false);
                if in_grave && !prev_grave {
                    events.push(Event::Destroyed(meta));
                } else if in_exile && !prev_exile {
                    events.push(Event::Exiled(meta));
                } else if in_hand && !prev_hand {
                    events.push(Event::Bounced(meta));
                } else {
                    events.push(Event::Destroyed(meta));
                }
            }
        }

        // Graveyard growth from hand (discards/mills).
        for (pid, cur_ids) in &cur_graveyard {
            let empty = BTreeSet::new();
            let prev_ids = self.prev_graveyard.get(pid).unwrap_or(&empty);
            for iid in cur_ids.difference(prev_ids) {
                let Some(meta) = self.instances.get(iid).cloned() else {
                    continue;
                };
                let was_on_bf = self
                    .prev_battlefield
                    .get(&meta.owner_id)
                    .map(|s| s.contains(iid))
                    .unwrap_or(false);
                if was_on_bf {
                    continue;
                }
                let was_in_hand = self
                    .prev_hand
                    .get(&meta.owner_id)
                    .map(|s| s.contains(iid))
                    .unwrap_or(false);
                if was_in_hand {
                    events.push(Event::Discarded(meta));
                } else {
                    events.push(Event::Milled(meta));
                }
            }
        }

        // Helper: the last-resolved opponent spell, used for attribution
        // when a state change can't be traced any other way. We pick the
        // first candidate (stack items are stored newest-last, and
        // `difference` preserves set order).
        let last_resolved_opponent_spell =
            |opponent_of: &str, instances: &HashMap<String, InstanceRecord>| {
                self.prev_stack
                    .difference(&cur_stack)
                    .find(|iid| {
                        instances
                            .get(*iid)
                            .map(|m| m.owner_id != opponent_of)
                            .unwrap_or(false)
                    })
                    .and_then(|iid| instances.get(iid).cloned())
            };

        // Life deltas → damage dealt (loss) or life gained (gain).
        for (pid, cur_l) in &cur_life {
            let prev = self.prev_life.get(pid).copied().unwrap_or(*cur_l);
            if *cur_l < prev {
                let delta = prev - *cur_l;
                if let Some(meta) = last_resolved_opponent_spell(pid.as_str(), &self.instances) {
                    events.push(Event::Damage(meta, delta));
                }
            } else if *cur_l > prev {
                let delta = *cur_l - prev;
                // Attribute life gain to the most-recent spell resolved
                // by THIS player (same-side).
                let same_side = self
                    .prev_stack
                    .difference(&cur_stack)
                    .find(|iid| {
                        self.instances
                            .get(*iid)
                            .map(|m| m.owner_id == *pid)
                            .unwrap_or(false)
                    })
                    .and_then(|iid| self.instances.get(iid).cloned());
                if let Some(meta) = same_side {
                    events.push(Event::LifeGained(meta, delta));
                }
            }
        }

        // Unit damage attribution: diff per-instance damage counters
        // on the battlefield. Any unit whose `damage` field increased
        // got hit by something this tick; attribute to the most-recent
        // opponent-side resolved spell/ability when available.
        for (_pid, player) in &state.players {
            for card in &player.battlefield {
                let prev = self.prev_unit_damage.get(&card.id).copied().unwrap_or(0);
                if card.damage > prev {
                    let delta = card.damage - prev;
                    let victim_owner = card.template_id.clone().or_else(|| Some(card.id.clone()));
                    let _ = victim_owner;
                    // Find the owner id from instance records.
                    let owner = self
                        .instances
                        .get(&card.id)
                        .map(|m| m.owner_id.clone())
                        .unwrap_or_default();
                    if let Some(meta) =
                        last_resolved_opponent_spell(owner.as_str(), &self.instances)
                    {
                        events.push(Event::UnitDamage(meta, delta));
                    }
                }
            }
        }

        // Token creation: any battlefield card whose instance id we've
        // never seen before AND whose template_id is None is a freshly
        // minted token. Attribute to the most recently resolved
        // same-side spell (most tokens are summoned by a resolving
        // effect on that player's side).
        for (pid, player) in &state.players {
            for card in &player.battlefield {
                if card.template_id.is_some() {
                    continue;
                }
                if self.prev_known_bf_ids.contains(&card.id) {
                    continue;
                }
                // Same-side most-recent resolved spell.
                let maybe_source = self
                    .prev_stack
                    .difference(&cur_stack)
                    .find(|iid| {
                        self.instances
                            .get(*iid)
                            .map(|m| m.owner_id == *pid)
                            .unwrap_or(false)
                    })
                    .and_then(|iid| self.instances.get(iid).cloned());
                if let Some(meta) = maybe_source {
                    events.push(Event::TokenCreated(meta));
                }
            }
        }

        // Cards-drawn attribution: if a player's hand grew by N and
        // their last-resolved spell was same-side, attribute the draw
        // to it. Untargeted card draw from turn upkeep (round flip)
        // is NOT attributed to any spell.
        if let Some(last_round) = self.last_round_seen {
            if last_round == state.round {
                for (pid, player) in &state.players {
                    let prev = self
                        .prev_hand_size
                        .get(pid)
                        .copied()
                        .unwrap_or(player.hand.len());
                    if player.hand.len() > prev {
                        let delta = player.hand.len() - prev;
                        let same_side = self
                            .prev_stack
                            .difference(&cur_stack)
                            .find(|iid| {
                                self.instances
                                    .get(*iid)
                                    .map(|m| m.owner_id == *pid)
                                    .unwrap_or(false)
                            })
                            .and_then(|iid| self.instances.get(iid).cloned());
                        if let Some(meta) = same_side {
                            events.push(Event::CardsDrawn(meta, delta));
                        }
                    }
                }
            }
        }

        // Ability activations: the engine records fired-this-turn keys
        // in `triggers_played_this_turn`. Keys are formatted like
        // `ability::<card_id>::<ability_id>` or `agent::<card_id>`.
        // Diff the per-key counts to attribute activations per source.
        let mut cur_triggers: BTreeMap<String, usize> = BTreeMap::new();
        for (k, v) in &state.triggers_played_this_turn {
            cur_triggers.insert(k.clone(), *v);
        }
        for (key, cur_count) in &cur_triggers {
            let prev_count = self.prev_triggers_by_key.get(key).copied().unwrap_or(0);
            if *cur_count <= prev_count {
                continue;
            }
            let delta = cur_count - prev_count;
            let card_id = extract_card_id_from_trigger_key(key);
            if let Some(card_id) = card_id {
                if let Some(meta) = self.instances.get(&card_id).cloned() {
                    for _ in 0..delta {
                        events.push(Event::Ability(meta.clone()));
                    }
                }
            }
        }

        // Round tick: for every unit on the battlefield at the start of
        // a new round, bump `rounds_on_battlefield` so we can compute
        // average survivability.
        if self.last_round_seen != Some(state.round) {
            for (_pid, player) in &state.players {
                for card in &player.battlefield {
                    if let Some(meta) = self.instances.get(&card.id).cloned() {
                        events.push(Event::Survived(meta));
                    }
                }
            }
        }

        // Apply all events.
        for event in events {
            match event {
                Event::Cast(meta) => {
                    let record = self.record_mut(&meta);
                    record.cast_count += 1;
                    if record.first_cast_round.is_none() {
                        record.first_cast_round = Some(state.round);
                    }
                }
                Event::Countered(meta) => self.record_mut(&meta).times_countered += 1,
                Event::Entered(meta) => self.record_mut(&meta).entered_battlefield += 1,
                Event::Destroyed(meta) => self.record_mut(&meta).times_destroyed += 1,
                Event::Exiled(meta) => self.record_mut(&meta).times_exiled += 1,
                Event::Bounced(meta) => self.record_mut(&meta).times_bounced += 1,
                Event::Discarded(meta) => self.record_mut(&meta).times_discarded += 1,
                Event::Milled(meta) => self.record_mut(&meta).times_milled += 1,
                Event::Damage(meta, n) => self.record_mut(&meta).damage_dealt += n,
                Event::UnitDamage(meta, n) => self.record_mut(&meta).unit_damage_dealt += n,
                Event::TokenCreated(meta) => self.record_mut(&meta).tokens_created += 1,
                Event::CardsDrawn(meta, n) => self.record_mut(&meta).cards_drawn += n,
                Event::LifeGained(meta, n) => self.record_mut(&meta).life_gained += n,
                Event::Survived(meta) => self.record_mut(&meta).rounds_on_battlefield += 1,
                Event::Ability(meta) => self.record_mut(&meta).ability_activations += 1,
            }
        }

        // Track ability-activation volume globally (for future use).
        let total_triggered: usize = state
            .triggers_played_this_turn
            .iter()
            .map(|(_, count)| *count)
            .sum();

        // Refresh unit-damage snapshot and known-battlefield ids.
        let mut new_unit_damage = BTreeMap::new();
        let mut new_known_bf = self.prev_known_bf_ids.clone();
        for (_pid, player) in &state.players {
            for card in &player.battlefield {
                new_unit_damage.insert(card.id.clone(), card.damage);
                new_known_bf.insert(card.id.clone());
            }
        }

        // Refresh hand sizes.
        let new_hand_sizes: BTreeMap<String, usize> = state
            .players
            .iter()
            .map(|(pid, p)| (pid.clone(), p.hand.len()))
            .collect();

        // Roll forward.
        self.prev_stack = cur_stack;
        self.prev_battlefield = cur_battlefield;
        self.prev_graveyard = cur_graveyard;
        self.prev_exile = cur_exile;
        self.prev_hand = cur_hand;
        self.prev_life = cur_life;
        self.prev_triggers_played_total = total_triggered;
        self.prev_unit_damage = new_unit_damage;
        self.prev_known_bf_ids = new_known_bf;
        self.prev_hand_size = new_hand_sizes;
        self.prev_triggers_by_key = cur_triggers;
        self.last_round_seen = Some(state.round);
    }

    /// Produce the final per-card rows, stamping `side` based on winner.
    pub(crate) fn finalize(
        mut self,
        state: &CardGameState,
        winner_id: Option<&str>,
    ) -> Vec<CardMetrics> {
        // Mark end-of-game survivors.
        for (pid, player) in &state.players {
            for card in &player.battlefield {
                let Some(tpl) = card.template_id.clone() else {
                    continue;
                };
                let meta = InstanceRecord {
                    template_id: tpl.clone(),
                    name: card.name.clone(),
                    owner_id: pid.clone(),
                };
                let record = self.record_mut(&meta);
                record.alive_on_battlefield_at_end += 1;
            }
        }

        let mut rows: Vec<CardMetrics> = self
            .counters
            .into_iter()
            .map(|((pid, _), mut metrics)| {
                metrics.side = match winner_id {
                    Some(w) if w == pid => "winner".to_string(),
                    Some(_) => "loser".to_string(),
                    None => "draw".to_string(),
                };
                metrics
            })
            .collect();
        rows.sort_by(|a, b| {
            a.owner_deck_id
                .cmp(&b.owner_deck_id)
                .then_with(|| a.card_id.cmp(&b.card_id))
                .then_with(|| a.side.cmp(&b.side))
        });
        rows
    }

    fn remember_instance(&mut self, card: &crate::card::Card, owner_id: &str) {
        let Some(tpl) = card.template_id.clone() else {
            return;
        };
        self.instances
            .entry(card.id.clone())
            .or_insert_with(|| InstanceRecord {
                template_id: tpl,
                name: card.name.clone(),
                owner_id: owner_id.to_string(),
            });
    }

    fn record_mut(&mut self, meta: &InstanceRecord) -> &mut CardMetrics {
        self.counters
            .entry((meta.owner_id.clone(), meta.template_id.clone()))
            .or_insert_with(|| CardMetrics {
                card_id: meta.template_id.clone(),
                card_name: meta.name.clone(),
                owner_deck_id: String::new(),
                side: String::new(),
                ..Default::default()
            })
    }

    fn snapshot_prev(&mut self, state: &CardGameState) {
        self.prev_stack = current_stack_ids(state);
        self.prev_battlefield = current_battlefield_ids(state);
        self.prev_graveyard = current_graveyard_ids(state);
        self.prev_exile = current_exile_ids(state);
        self.prev_hand = current_hand_ids(state);
        self.prev_life = state
            .players
            .iter()
            .map(|(pid, p)| (pid.clone(), p.life))
            .collect();
        self.prev_triggers_played_total = state
            .triggers_played_this_turn
            .iter()
            .map(|(_, count)| *count)
            .sum();
    }
}

fn current_stack_ids(state: &CardGameState) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for item in &state.stack {
        if let StackItem {
            kind: StackItemKind::Spell(card),
            ..
        } = item
        {
            out.insert(card.id.clone());
        }
    }
    if let Some(entry) = &state.showcase {
        out.insert(entry.card.id.clone());
    }
    out
}

fn current_battlefield_ids(state: &CardGameState) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (pid, player) in &state.players {
        let set = out.entry(pid.clone()).or_default();
        for card in &player.battlefield {
            set.insert(card.id.clone());
        }
    }
    out
}

fn current_graveyard_ids(state: &CardGameState) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (pid, player) in &state.players {
        let set = out.entry(pid.clone()).or_default();
        for card in &player.graveyard {
            set.insert(card.id.clone());
        }
    }
    out
}

fn current_exile_ids(state: &CardGameState) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (pid, player) in &state.players {
        let set = out.entry(pid.clone()).or_default();
        for card in &player.exile {
            set.insert(card.id.clone());
        }
    }
    out
}

fn current_hand_ids(state: &CardGameState) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (pid, player) in &state.players {
        let set = out.entry(pid.clone()).or_default();
        for card in &player.hand {
            set.insert(card.id.clone());
        }
    }
    out
}

/// Parse a `triggers_played_this_turn` key into its source card id.
///
/// Known formats:
///   `ability::<card_id>::<ability_id>`
///   `agent::<card_id>`
/// Returns None for keys we don't recognize (trigger-limit keys etc).
fn extract_card_id_from_trigger_key(key: &str) -> Option<String> {
    if let Some(rest) = key.strip_prefix("ability::") {
        if let Some(sep) = rest.find("::") {
            return Some(rest[..sep].to_string());
        }
        return Some(rest.to_string());
    }
    if let Some(rest) = key.strip_prefix("agent::") {
        return Some(rest.to_string());
    }
    None
}

fn owner_collection_contains(
    map: &BTreeMap<String, BTreeSet<String>>,
    owner: &str,
    iid: &str,
) -> bool {
    map.get(owner).map(|s| s.contains(iid)).unwrap_or(false)
}

/// Rolled-up per-card stats across many matches. One row per
/// (deck, card, side). `matches_seen` is the denominator for averaging.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AggregatedCardMetrics {
    pub card_id: String,
    pub card_name: String,
    pub owner_deck_id: String,
    pub side: String,
    pub matches_seen: usize,
    pub total_copies_in_deck: usize,
    pub in_opening_hand: usize,
    pub cast_count: usize,
    pub first_cast_round_sum: usize,
    pub first_cast_observations: usize,
    pub entered_battlefield: usize,
    pub times_destroyed: usize,
    pub times_exiled: usize,
    pub times_bounced: usize,
    pub times_sacrificed: usize,
    pub times_countered: usize,
    pub times_milled: usize,
    pub times_discarded: usize,
    pub ability_activations: usize,
    pub alive_on_battlefield_at_end: usize,
    pub damage_dealt: i32,
    pub unit_damage_dealt: i32,
    pub tokens_created: usize,
    pub rounds_on_battlefield: usize,
    pub cards_drawn: usize,
    pub life_gained: i32,
}

/// Fold a bag of `MatchReport::card_metrics` rows into per-card totals.
/// Instances from tests or hand-built decks (template_id == None) are
/// filtered out by the collector upstream — this layer just sums.
pub fn aggregate_card_metrics(
    match_reports: impl IntoIterator<Item = Vec<CardMetrics>>,
) -> Vec<AggregatedCardMetrics> {
    let mut by_key: BTreeMap<(String, String, String), AggregatedCardMetrics> = BTreeMap::new();
    for rows in match_reports {
        for row in rows {
            // Skip synthetic per-instance rows from tokens / infra that
            // slipped through without a template_id (their `card_id`
            // tends to include a per-player prefix like "p1-infra-…").
            if row.card_id.contains('-') && row.card_id.matches('-').count() >= 2 {
                continue;
            }
            let key = (
                row.owner_deck_id.clone(),
                row.card_id.clone(),
                row.side.clone(),
            );
            let slot = by_key.entry(key).or_insert_with(|| AggregatedCardMetrics {
                card_id: row.card_id.clone(),
                card_name: row.card_name.clone(),
                owner_deck_id: row.owner_deck_id.clone(),
                side: row.side.clone(),
                ..Default::default()
            });
            slot.matches_seen += 1;
            slot.total_copies_in_deck += row.copies_in_deck;
            slot.in_opening_hand += row.in_opening_hand;
            slot.cast_count += row.cast_count;
            if let Some(round) = row.first_cast_round {
                slot.first_cast_round_sum += round;
                slot.first_cast_observations += 1;
            }
            slot.entered_battlefield += row.entered_battlefield;
            slot.times_destroyed += row.times_destroyed;
            slot.times_exiled += row.times_exiled;
            slot.times_bounced += row.times_bounced;
            slot.times_sacrificed += row.times_sacrificed;
            slot.times_countered += row.times_countered;
            slot.times_milled += row.times_milled;
            slot.times_discarded += row.times_discarded;
            slot.ability_activations += row.ability_activations;
            slot.alive_on_battlefield_at_end += row.alive_on_battlefield_at_end;
            slot.damage_dealt += row.damage_dealt;
            slot.unit_damage_dealt += row.unit_damage_dealt;
            slot.tokens_created += row.tokens_created;
            slot.rounds_on_battlefield += row.rounds_on_battlefield;
            slot.cards_drawn += row.cards_drawn;
            slot.life_gained += row.life_gained;
        }
    }
    by_key.into_values().collect()
}

/// Render aggregated metrics as a CSV. Caller writes to a file/stdout.
pub fn render_card_metrics_csv(rows: &[AggregatedCardMetrics]) -> String {
    let mut out = String::new();
    out.push_str(
        "card_id,card_name,owner_deck_id,side,matches_seen,copies_in_deck,\
         in_opening_hand,cast_count,avg_first_cast_round,entered_battlefield,\
         times_destroyed,times_exiled,times_bounced,times_countered,\
         times_milled,times_discarded,ability_activations,alive_at_end,\
         damage_dealt,unit_damage_dealt,tokens_created,rounds_on_battlefield,\
         cards_drawn,life_gained\n",
    );
    for row in rows {
        let avg_first = if row.first_cast_observations > 0 {
            format!(
                "{:.2}",
                row.first_cast_round_sum as f64 / row.first_cast_observations as f64
            )
        } else {
            String::new()
        };
        let name = row.card_name.replace(',', ";");
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            row.card_id,
            name,
            row.owner_deck_id,
            row.side,
            row.matches_seen,
            row.total_copies_in_deck,
            row.in_opening_hand,
            row.cast_count,
            avg_first,
            row.entered_battlefield,
            row.times_destroyed,
            row.times_exiled,
            row.times_bounced,
            row.times_countered,
            row.times_milled,
            row.times_discarded,
            row.ability_activations,
            row.alive_on_battlefield_at_end,
            row.damage_dealt,
            row.unit_damage_dealt,
            row.tokens_created,
            row.rounds_on_battlefield,
            row.cards_drawn,
            row.life_gained,
        ));
    }
    out
}
