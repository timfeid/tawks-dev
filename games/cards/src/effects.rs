//! Card effect application and stat modifiers

use crate::card::{
    AbilityCost, Card, CardAbility, CardEffect, StatModifierSpec, mana_color_for_type,
};
use crate::effect::StatModifierEffect;
use crate::state::*;
use crate::types::*;

/// Returns true if the effect tree contains a `CopyUnit` — used to strip
/// recursive copy triggers from copies so they can't cascade.
fn copy_effect_recurses(effect: &CardEffect) -> bool {
    match effect {
        CardEffect::CopyUnit => true,
        CardEffect::Sequence(effects) => effects.iter().any(copy_effect_recurses),
        _ => false,
    }
}

/// Effect application functions
impl CardGameState {
    fn sim_debug_enabled() -> bool {
        std::env::var("CARDGAME_SIM_DEBUG")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    fn sim_debug(&self, message: impl AsRef<str>) {
        if Self::sim_debug_enabled() {
            eprintln!("[effects] {}", message.as_ref());
        }
    }

    pub fn next_effect_id(&mut self) -> String {
        let id = format!("effect-{}", self.effect_id_counter);
        self.effect_id_counter = self.effect_id_counter.saturating_add(1);
        id
    }

    /// Deal `amount` damage to a unit, applying the target's Absorb(N)
    /// damage prevention if present. Updates `card.damage` and
    /// `card.absorb_used_this_turn`, and returns the amount of damage
    /// actually dealt (useful for Trample overflow math).
    pub fn damage_unit(&mut self, owner_id: &str, card_id: &str, amount: i32) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let Some(player) = self.players.get_mut(owner_id) else {
            return 0;
        };
        let Some(card) = player.battlefield.iter_mut().find(|c| c.id == card_id) else {
            return 0;
        };
        let absorb_capacity: i32 = card
            .keywords
            .iter()
            .map(|k| match k {
                Keyword::Absorb(n) => *n,
                _ => 0,
            })
            .sum();
        let absorb_remaining = (absorb_capacity - card.absorb_used_this_turn).max(0);
        let prevented = amount.min(absorb_remaining);
        card.absorb_used_this_turn += prevented;
        let dealt = amount - prevented;
        card.damage += dealt;
        dealt
    }

    pub fn add_stat_modifier(
        &mut self,
        owner_id: &str,
        target: EffectTarget,
        spec: &StatModifierSpec,
    ) {
        let id = self.next_effect_id();
        self.effect_manager.add_effect(StatModifierEffect {
            id,
            target,
            stat: spec.stat.clone(),
            amount: spec.amount.clone(),
            expires: spec.expires.clone(),
            owner_id: owner_id.to_string(),
        });
    }

    pub fn apply_stat_modifier_spec(
        &mut self,
        owner_id: &str,
        source_id: Option<&str>,
        spec: &StatModifierSpec,
        targets: &[TargetRef],
    ) {
        match &spec.target {
            EffectTargetSpec::Targets => {
                for target in targets {
                    match target {
                        TargetRef::Card(card_id) => {
                            self.add_stat_modifier(
                                owner_id,
                                EffectTarget::Card(card_id.clone()),
                                spec,
                            );
                        }
                        TargetRef::Player(player_id) => {
                            self.add_stat_modifier(
                                owner_id,
                                EffectTarget::Player(player_id.clone()),
                                spec,
                            );
                        }
                    }
                }
            }
            EffectTargetSpec::Source => {
                if let Some(source_id) = source_id {
                    self.add_stat_modifier(
                        owner_id,
                        EffectTarget::Card(source_id.to_string()),
                        spec,
                    );
                }
            }
            EffectTargetSpec::OwnerUnits(filter) => {
                let mut card_ids = Vec::new();
                if let Some(owner) = self.players.get(owner_id) {
                    for card in &owner.battlefield {
                        if !matches!(&card.kind, CardKind::Unit(_)) {
                            continue;
                        }
                        if let Some(filter_type) = filter {
                            if !matches!(&card.kind, CardKind::Unit(kind) if kind == filter_type) {
                                continue;
                            }
                        }
                        card_ids.push(card.id.clone());
                    }
                }
                for card_id in card_ids {
                    self.add_stat_modifier(owner_id, EffectTarget::Card(card_id), spec);
                }
            }
            EffectTargetSpec::OpponentUnits(filter) => {
                let mut card_ids = Vec::new();
                for player_id in self.ordered_player_ids() {
                    let Some(player) = self.players.get(&player_id) else {
                        continue;
                    };
                    if player_id == owner_id {
                        continue;
                    }
                    for card in &player.battlefield {
                        if !matches!(&card.kind, CardKind::Unit(_)) {
                            continue;
                        }
                        if let Some(filter_type) = filter {
                            if !matches!(&card.kind, CardKind::Unit(kind) if kind == filter_type) {
                                continue;
                            }
                        }
                        card_ids.push(card.id.clone());
                    }
                }
                for card_id in card_ids {
                    self.add_stat_modifier(owner_id, EffectTarget::Card(card_id), spec);
                }
            }
            EffectTargetSpec::AllUnits(filter) => {
                let mut card_ids = Vec::new();
                for player_id in self.ordered_player_ids() {
                    let Some(player) = self.players.get(&player_id) else {
                        continue;
                    };
                    for card in &player.battlefield {
                        if !matches!(&card.kind, CardKind::Unit(_)) {
                            continue;
                        }
                        if let Some(filter_type) = filter {
                            if !matches!(&card.kind, CardKind::Unit(kind) if kind == filter_type) {
                                continue;
                            }
                        }
                        card_ids.push(card.id.clone());
                    }
                }
                for card_id in card_ids {
                    self.add_stat_modifier(owner_id, EffectTarget::Card(card_id), spec);
                }
            }
            EffectTargetSpec::AdjacentUnits => {
                let mut card_ids = Vec::new();
                for target in targets {
                    if let TargetRef::Card(target_id) = target {
                        if let Some((owner_id, _)) = self.find_battlefield_card_owner(target_id) {
                            if let Some(owner) = self.players.get(&owner_id) {
                                if let Some(idx) =
                                    owner.battlefield.iter().position(|c| &c.id == target_id)
                                {
                                    if idx > 0 {
                                        card_ids.push(owner.battlefield[idx - 1].id.clone());
                                    }
                                    if idx < owner.battlefield.len() - 1 {
                                        card_ids.push(owner.battlefield[idx + 1].id.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                for card_id in card_ids {
                    self.add_stat_modifier(owner_id, EffectTarget::Card(card_id), spec);
                }
            }
            EffectTargetSpec::LastCast => {
                if let Some(item) = self.history.last() {
                    match &item.kind {
                        StackItemKind::Spell(_card) => {
                            // Target the card itself if it stayed on battlefield?
                            // Or the player? Usually "Last Cast" in this context targets the player.
                            self.add_stat_modifier(
                                owner_id,
                                EffectTarget::Player(item.owner_id.clone()),
                                spec,
                            );
                        }
                        StackItemKind::Ability { source_card_id, .. } => {
                            self.add_stat_modifier(
                                owner_id,
                                EffectTarget::Card(source_card_id.clone()),
                                spec,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn apply_effect(&mut self, owner_id: &str, effect: &CardEffect, targets: &[TargetRef]) {
        self.apply_effect_with_source(owner_id, None, effect, targets);
    }

    pub fn apply_effect_with_source(
        &mut self,
        owner_id: &str,
        source_id: Option<&str>,
        effect: &CardEffect,
        targets: &[TargetRef],
    ) {
        // Guard against runaway effect recursion (e.g. two units that
        // each copy the other on enter — "Synthesis Reconstructor" loop).
        // 64 is comfortably above any legitimate Sequence/cascade depth
        // observed in play, but small enough to catch a runaway within
        // a few ms of CPU time.
        const MAX_EFFECT_DEPTH: u32 = 64;
        if self.effect_resolution_depth >= MAX_EFFECT_DEPTH {
            self.sim_debug(format!(
                "effect depth cap hit ({}) — aborting effect={:?}",
                MAX_EFFECT_DEPTH, effect
            ));
            return;
        }
        // Pragmatic total-effects cap per match. Infinite sac/regen combos
        // (e.g. Smelting Altar + death-trigger token generators) can cascade
        // without exceeding depth but still loop forever. 5000 effects is
        // comfortably above any normal match and catches runaway combos fast.
        const MAX_EFFECTS_PER_MATCH: u64 = 5000;
        if self.match_effects_total >= MAX_EFFECTS_PER_MATCH {
            self.effects_cap_exceeded = true;
            return;
        }
        self.match_effects_total = self.match_effects_total.saturating_add(1);
        self.effect_resolution_depth += 1;
        self.apply_effect_inner(owner_id, source_id, effect, targets);
        self.effect_resolution_depth = self.effect_resolution_depth.saturating_sub(1);
    }

    fn apply_effect_inner(
        &mut self,
        owner_id: &str,
        source_id: Option<&str>,
        effect: &CardEffect,
        targets: &[TargetRef],
    ) {
        self.sim_debug(format!(
            "owner={} source={} effect={:?} targets={:?}",
            owner_id,
            source_id.unwrap_or("<none>"),
            effect,
            targets
        ));
        match effect {
            CardEffect::RememberTargetPlayer => {
                let Some(source_id) = source_id else {
                    return;
                };
                let Some(TargetRef::Player(target_player_id)) = targets.first() else {
                    return;
                };
                if let Some(owner) = self.players.get_mut(owner_id) {
                    if let Some(card) = owner
                        .battlefield
                        .iter_mut()
                        .find(|card| card.id == source_id)
                    {
                        card.remembered_player_id = Some(target_player_id.clone());
                    }
                }
            }
            CardEffect::Sequence(effects) => {
                for (index, effect) in effects.iter().enumerate() {
                    // Apply the current effect
                    self.apply_effect_with_source(owner_id, source_id, effect, targets);

                    // Check if this effect requires pausing the sequence
                    let should_pause = match effect {
                        CardEffect::Scry(_)
                        | CardEffect::Loot(_, _)
                        | CardEffect::Search(_)
                        | CardEffect::SearchGraveyard(_)
                        | CardEffect::Scavenge(_)
                        | CardEffect::Surveil(_)
                        | CardEffect::Transmit(_) => self
                            .players
                            .get(owner_id)
                            .map(|p| {
                                !p.scry.is_empty()
                                    || p.library_search_active
                                    || p.has_pending_discard()
                            })
                            .unwrap_or(false),
                        _ => false,
                    };

                    if should_pause {
                        // Save remaining effects to be processed after user input
                        if let Some(player) = self.players.get_mut(owner_id) {
                            if !player.pending_effects.is_empty() || player.scry.is_empty() {
                                // If there are already pending effects or no scry, append remaining effects
                                let remaining: Vec<_> = effects[index + 1..]
                                    .iter()
                                    .cloned()
                                    .map(|effect| (effect, targets.to_vec()))
                                    .collect();
                                player.pending_effects.extend(remaining);
                            } else {
                                // Otherwise replace pending effects
                                player.pending_effects = effects[index + 1..]
                                    .iter()
                                    .cloned()
                                    .map(|effect| (effect, targets.to_vec()))
                                    .collect();
                            }
                        }
                        return;
                    }
                }
            }
            CardEffect::CounterTop => {
                if self.stack.pop().is_some() {
                    self.apply_battlefield_triggers_for_player_event(
                        owner_id,
                        &TriggerEvent::CounteredSpell,
                        &[],
                    );
                }
            }
            CardEffect::DamagePlayer(amount) => {
                // Damage-to-player events reset the display timer so the
                // client has time to play a hit animation before the game
                // auto-advances.
                if *amount > 0 {
                    self.last_stack_push_at = Some(chrono::Utc::now());
                }
                if let Some(TargetRef::Player(target_id)) = targets.first().cloned() {
                    if let Some(target) = self.players.get_mut(&target_id) {
                        target.life = (target.life - amount).max(0);
                    }
                } else if let Some(target_id) = self.default_defender_id(owner_id) {
                    if let Some(target) = self.players.get_mut(&target_id) {
                        target.life = (target.life - amount).max(0);
                    }
                }
            }
            CardEffect::DamagePlayerPerHandCard => {
                let target_id = targets
                    .first()
                    .and_then(|t| {
                        if let TargetRef::Player(id) = t {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| self.default_defender_id(owner_id));
                if let Some(target_id) = target_id {
                    let amount = self
                        .players
                        .get(&target_id)
                        .map(|p| p.hand.len() as i32)
                        .unwrap_or(0);
                    if let Some(target) = self.players.get_mut(&target_id) {
                        target.life = (target.life - amount).max(0);
                    }
                }
            }
            CardEffect::DamagePlayerPerBattlefieldKind(kind) => {
                let count = self
                    .players
                    .get(owner_id)
                    .map(|p| p.battlefield.iter().filter(|c| c.kind == *kind).count() as i32)
                    .unwrap_or(0);
                if count > 0 {
                    for (pid, player) in &mut self.players {
                        if pid != owner_id {
                            player.life = (player.life - count).max(0);
                        }
                    }
                }
            }
            CardEffect::CreateTokensPerBattlefieldKind(token, kind) => {
                let count = self
                    .players
                    .get(owner_id)
                    .map(|p| p.battlefield.iter().filter(|c| c.kind == *kind).count() as usize)
                    .unwrap_or(0);
                for _ in 0..count {
                    self.apply_effect_with_source(
                        owner_id,
                        source_id,
                        &CardEffect::CreateToken(token.clone()),
                        targets,
                    );
                }
            }
            CardEffect::DamageUnit(amount) => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        if let Some((target_owner, _)) = self.find_battlefield_card_owner(card_id) {
                            self.damage_unit(&target_owner, card_id, *amount);
                        }
                    }
                }
            }
            CardEffect::DamageTarget(amount) => {
                for target in targets {
                    match target {
                        TargetRef::Player(target_id) => {
                            if let Some(target) = self.players.get_mut(target_id) {
                                target.life = (target.life - amount).max(0);
                            }
                        }
                        TargetRef::Card(card_id) => {
                            if let Some((target_owner, _)) =
                                self.find_battlefield_card_owner(card_id)
                            {
                                self.damage_unit(&target_owner, card_id, *amount);
                            }
                        }
                    }
                }
            }
            CardEffect::DrawCards(count) => {
                for _ in 0..*count {
                    self.draw_card_for_player(owner_id);
                }
            }
            CardEffect::GainLife(amount) => {
                if let Some(owner) = self.players.get_mut(owner_id) {
                    owner.life += amount;
                }
            }
            CardEffect::Heat(amount) => {
                if let Some(owner) = self.players.get_mut(owner_id) {
                    owner.heat = (owner.heat + amount).max(0);
                }
            }
            CardEffect::HeatOpponents(amount) => {
                for (pid, player) in self.players.iter_mut() {
                    if pid != owner_id {
                        player.heat = (player.heat + amount).max(0);
                    }
                }
            }
            CardEffect::CashOpponents(amount) => {
                for (pid, player) in self.players.iter_mut() {
                    if pid != owner_id {
                        player.mana_pool.colorless += amount;
                    }
                }
            }
            CardEffect::AddMana(mana_type, amount) => {
                if let Some(owner) = self.players.get_mut(owner_id) {
                    let scaled = Self::scaled_mana_amount(mana_type, *amount);
                    match mana_type {
                        ManaType::Neon => owner.mana_pool.neon += scaled,
                        ManaType::Glitch => owner.mana_pool.glitch += scaled,
                        ManaType::Chrome => owner.mana_pool.chrome += scaled,
                        ManaType::Slag => owner.mana_pool.slag += scaled,
                        ManaType::Bio => owner.mana_pool.bio += scaled,
                        ManaType::Colorless => owner.mana_pool.colorless += scaled,
                    }
                }
            }
            CardEffect::CreateToken(token) => {
                let serial = self.next_spawn_serial();
                if let Some(owner) = self.players.get_mut(owner_id) {
                    // A game-wide serial keeps token ids unique after others
                    // die, and the same in the AI's planning copy.
                    let token_id = format!("{}-token-{serial}", owner_id);
                    owner.battlefield.push(Card {
                        id: token_id,
                        name: token.name.clone(),
                        description: "Token unit.".to_string(),
                        auto_description: String::new(),
                        rarity: CardRarity::Public,
                        categories: vec![],
                        visual_spec: None,
                        template_id: None,
                        cost: ManaCost::default(),
                        kind: CardKind::Unit(token.unit_type.clone()),
                        supertype: CardSupertype::Basic,
                        power: Some(token.power),
                        toughness: Some(token.toughness),
                        tapped: token.enters_tapped,
                        enters_tapped: token.enters_tapped,
                        triggers: token.triggers.clone(),
                        abilities: token.abilities.clone(),
                        keywords: vec![],
                        image_url: token.image_url().map(|s| s.to_string()),
                        colors: vec![],
                        attached_to: None,
                        remembered_player_id: None,
                        attachment: None,
                        summoning_sickness: true,
                        haste: false,
                        burnout: false,
                        echo_pending: false,
                        malware_infected: false,
                        loyalty: None,
                        counters: 0,
                        counters_by_kind: std::collections::HashMap::new(),
                        damage: 0,
                        gravecast_uses: 0,
                        frozen_turns: 0,
                        absorb_used_this_turn: 0,
                        creator_id: source_id.map(|s| s.to_string()),
                        created_at: None,
                        owned: false,
                        obtained_at: None,
                    });
                    owner.tokens_created += 1;
                    let entered = owner.battlefield.last().cloned();
                    if let Some(card) = entered {
                        self.sim_debug(format!(
                            "token_entered owner={} card={} name={}",
                            owner_id, card.id, card.name
                        ));
                        self.handle_card_entered(owner_id, &card, &[]);
                    }
                }
            }
            CardEffect::CreateInfrastructure(spec) => {
                let mut created = Vec::new();
                let count = spec.count.max(1);
                let serials: Vec<u64> = (0..count).map(|_| self.next_spawn_serial()).collect();
                if let Some(owner) = self.players.get_mut(owner_id) {
                    for serial in serials {
                        let infra_id = format!("{}-infra-{serial}", owner_id);
                        let card = Card {
                            id: infra_id,
                            name: spec.name.clone(),
                            description: format!("Tap: Add {} {:?}.", spec.amount, spec.mana_type),
                            auto_description: String::new(),
                            rarity: CardRarity::Public,
                            categories: vec![],
                            visual_spec: None,
                            template_id: None,
                            cost: ManaCost::default(),
                            kind: CardKind::Infrastructure,
                            supertype: CardSupertype::Basic,
                            power: None,
                            toughness: None,
                            tapped: spec.enters_tapped,
                            enters_tapped: spec.enters_tapped,
                            triggers: vec![],
                            abilities: vec![CardAbility {
                                id: format!("{}_tap", spec.name),
                                name: "Tap for mana".to_string(),
                                cost: AbilityCost::new().with_tap(),
                                effect: CardEffect::AddMana(spec.mana_type.clone(), spec.amount),
                                target_spec: None,
                                once_per_turn: false,
                                uses_stack: false,
                                tap_targets: false,
                                predicate: None,
                            }],
                            keywords: vec![],
                            image_url: None,
                            colors: vec![mana_color_for_type(&spec.mana_type).to_string()],
                            attached_to: None,
                            remembered_player_id: None,
                            attachment: None,
                            summoning_sickness: false,
                            haste: false,
                            burnout: false,
                            echo_pending: false,
                            malware_infected: false,
                            loyalty: None,
                            counters: 0,
                            counters_by_kind: std::collections::HashMap::new(),
                            damage: 0,
                            gravecast_uses: 0,
                            frozen_turns: 0,
                            absorb_used_this_turn: 0,
                            creator_id: source_id.map(|s| s.to_string()),
                            created_at: None,
                            owned: false,
                            obtained_at: None,
                        };
                        owner.battlefield.push(card.clone());
                        created.push(card);
                    }
                }
                for card in created {
                    self.sim_debug(format!(
                        "infrastructure_entered owner={} card={} name={}",
                        owner_id, card.id, card.name
                    ));
                    self.handle_card_entered(owner_id, &card, &[]);
                }
            }
            CardEffect::ApplyStatModifier(spec) => {
                self.apply_stat_modifier_spec(owner_id, source_id, spec, targets);
            }
            CardEffect::ApplyStatModifiers(specs) => {
                for spec in specs {
                    self.apply_stat_modifier_spec(owner_id, source_id, spec, targets);
                }
            }
            CardEffect::RevealTopIf {
                predicate,
                on_match,
                on_fail,
            } => {
                self.reveal_top_if(owner_id, predicate, *on_match, *on_fail);
            }
            CardEffect::Freeze(expires) => {
                // Freeze: tap the target now and prevent it from untapping
                // for N of its owner's untap steps. `frozen_turns` is
                // decremented by the untap step — only at 0 does the unit
                // actually untap.
                let turns = match expires {
                    ExpireContract::Turns(n) => *n,
                    ExpireContract::EndOfTurn => 1,
                    ExpireContract::Never => u8::MAX,
                };
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        if let Some((owner_pid, _)) = self.find_battlefield_card_owner(card_id) {
                            if let Some(player) = self.players.get_mut(&owner_pid) {
                                if let Some(card) =
                                    player.battlefield.iter_mut().find(|c| &c.id == card_id)
                                {
                                    card.tapped = true;
                                    card.frozen_turns = card.frozen_turns.max(turns);
                                }
                            }
                        }
                    }
                }
            }
            CardEffect::RefundMana(_percentage) => {
                // Logic implemented where triggers are called
            }
            CardEffect::Loot(draw, discard) => {
                for _ in 0..*draw {
                    self.draw_card_for_player(owner_id);
                }
                if let Some(player) = self.players.get_mut(owner_id) {
                    if *discard > 0 && !player.hand.is_empty() {
                        let actual = (*discard).min(player.hand.len());
                        player.pending_actions.push(PendingAction::Discard {
                            amount: actual,
                            source_description: Self::effect_text(effect, None),
                            source_card_id: source_id.map(str::to_string),
                        });
                    }
                }
            }
            CardEffect::Wheel(draw_count) => {
                // Each player discards their hand, then draws X cards
                // Collect player IDs to avoid borrow issues
                let player_ids = self.ordered_player_ids();

                for pid in player_ids {
                    let discard_amount = self.players.get(&pid).map(|p| p.hand.len()).unwrap_or(0);

                    if discard_amount > 0 {
                        if let Some(player) = self.players.get_mut(&pid) {
                            player.pending_actions.push(PendingAction::Discard {
                                amount: discard_amount,
                                source_description: Self::effect_text(effect, None),
                                source_card_id: source_id.map(str::to_string),
                            });
                            // Draw happens after discard
                            player
                                .pending_effects
                                .push((CardEffect::DrawCards(*draw_count), vec![]));
                        }
                    } else {
                        // Player has empty hand - skip discard, draw immediately
                        for _ in 0..*draw_count {
                            self.draw_card_for_player(&pid);
                        }
                    }
                }
            }
            CardEffect::Scry(amount) => {
                if let Some(player) = self.players.get_mut(owner_id) {
                    let count = (*amount).min(player.deck.len());
                    let start = player.deck.len().saturating_sub(count);
                    let mut scry_cards: Vec<Card> = player.deck[start..].iter().cloned().collect();
                    scry_cards.reverse();
                    player.scry = scry_cards;
                }
            }
            CardEffect::Search(predicate) => {
                let matching = self.players.get(owner_id).map(|player| {
                    player
                        .deck
                        .iter()
                        .filter(|card| Self::card_matches_predicate(card, predicate))
                        .cloned()
                        .collect::<Vec<_>>()
                });

                if let Some(matching) = matching {
                    if matching.is_empty() {
                        self.shuffle_deck_for_player(owner_id, "search_miss");
                    } else if let Some(player) = self.players.get_mut(owner_id) {
                        player.library_search_results = matching;
                        player.library_search_active = true;
                        player.card_browser_active = true;
                        player.card_browser_title = Some("Search Library".to_string());
                        player.card_browser_subtitle =
                            Some("Select a card to put into your hand".to_string());
                        player.collection_page = 0;
                        player.collection_total = player.library_search_results.len() as i32;
                    }
                }
            }
            CardEffect::SearchToBattlefield(predicate) => {
                // Auto-pick the first matching card from the library and put
                // it directly onto the battlefield. Used for finisher tutor
                // ultimates where bypassing the cost is the whole point.
                let chosen_id = self.players.get(owner_id).and_then(|player| {
                    player
                        .deck
                        .iter()
                        .find(|card| Self::card_matches_predicate(card, predicate))
                        .map(|c| c.id.clone())
                });

                if let Some(card_id) = chosen_id {
                    if let Some(player) = self.players.get_mut(owner_id) {
                        if let Some(pos) = player.deck.iter().position(|c| c.id == card_id) {
                            let mut card = player.deck.remove(pos);
                            card.tapped = card.enters_tapped;
                            card.summoning_sickness = true;
                            player.battlefield.push(card);
                        }
                    }
                    // Re-shuffle library after a tutor.
                    self.shuffle_deck_for_player(owner_id, "search_to_battlefield");
                } else {
                    // No matching card; just shuffle (mirrors miss handling on Search).
                    self.shuffle_deck_for_player(owner_id, "search_to_battlefield_miss");
                }
            }
            CardEffect::SearchGraveyard(predicate) => {
                let search_owner_id = targets
                    .iter()
                    .find_map(|target| match target {
                        TargetRef::Player(player_id) => Some(player_id.as_str()),
                        _ => None,
                    })
                    .unwrap_or(owner_id);

                let matching = self
                    .players
                    .get(search_owner_id)
                    .map(|player| {
                        player
                            .graveyard
                            .iter()
                            .filter(|card| Self::card_matches_predicate(card, predicate))
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if let Some(owner) = self.players.get_mut(owner_id) {
                    if matching.is_empty() {
                        owner.library_search_active = false;
                    } else {
                        owner.library_search_results = matching;
                        owner.library_search_active = true;
                        owner.card_browser_active = true;
                        owner.card_browser_title = Some("Search Graveyard".to_string());
                        owner.card_browser_subtitle =
                            Some("Select a card to return to your hand".to_string());
                        owner.collection_page = 0;
                        owner.collection_total = owner.library_search_results.len() as i32;
                    }
                }
            }
            CardEffect::Bounce => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        let owner_id = self.find_battlefield_card_owner(card_id).map(|(id, _)| id);
                        if let Some(owner_id) = owner_id {
                            let removed = {
                                let owner = self.players.get_mut(&owner_id);
                                owner
                                    .and_then(|owner| Self::remove_from_battlefield(owner, card_id))
                            };
                            if let Some(mut card) = removed {
                                self.apply_triggers_for_event(
                                    &owner_id,
                                    card_id,
                                    Some(&card.name),
                                    &card.triggers,
                                    &TriggerEvent::Left,
                                    &[],
                                );
                                card.summoning_sickness = false;
                                // Reset tapped/damage state so when the
                                // card is cast again from hand it doesn't
                                // come out already-tapped or pre-damaged.
                                card.tapped = false;
                                card.damage = 0;
                                if let Some(owner) = self.players.get_mut(&owner_id) {
                                    if card.is_token() {
                                        // Tokens cease to exist when leaving
                                        // the battlefield — drop the card.
                                    } else if self.format == GameFormat::Operator
                                        && owner.operator_id.as_deref() == Some(card.id.as_str())
                                    {
                                        card.attached_to = None;
                                        card.remembered_player_id = None;
                                        owner.operator_zone.push(card);
                                    } else {
                                        card.remembered_player_id = None;
                                        owner.hand.push(card);
                                    }
                                }
                                self.detach_attachments(card_id);
                            }
                        }
                    }
                }
            }
            CardEffect::DestroyTarget => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        if let Some((owner_id, _)) = self.find_battlefield_card_owner(card_id) {
                            self.remove_from_battlefield_and_attachments(&owner_id, card_id);
                        }
                    }
                }
            }
            CardEffect::ExileTarget => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        if let Some((owner_id, _)) = self.find_battlefield_card_owner(card_id) {
                            self.exile_from_battlefield_and_attachments(&owner_id, card_id);
                        }
                    }
                }
            }
            CardEffect::BoardWipe => {
                let mut to_remove = Vec::new();
                for pid in self.ordered_player_ids() {
                    let Some(player) = self.players.get(&pid) else {
                        continue;
                    };
                    for card in &player.battlefield {
                        if matches!(card.kind, CardKind::Unit(_)) {
                            to_remove.push((pid.clone(), card.id.clone()));
                        }
                    }
                }
                for (pid, cid) in to_remove {
                    self.remove_from_battlefield_and_attachments(&pid, &cid);
                }
            }
            CardEffect::DestroyTargetPlayerUnits => {
                let target_player_ids: Vec<String> = targets
                    .iter()
                    .filter_map(|t| match t {
                        TargetRef::Player(pid) => Some(pid.clone()),
                        _ => None,
                    })
                    .collect();
                let mut to_remove = Vec::new();
                for pid in target_player_ids {
                    let Some(player) = self.players.get(&pid) else {
                        continue;
                    };
                    for card in &player.battlefield {
                        if matches!(card.kind, CardKind::Unit(_)) {
                            to_remove.push((pid.clone(), card.id.clone()));
                        }
                    }
                }
                for (pid, cid) in to_remove {
                    self.remove_from_battlefield_and_attachments(&pid, &cid);
                }
            }
            CardEffect::DestroyUnitsWithToughnessAtLeast(min_toughness) => {
                let mut to_remove = Vec::new();
                for pid in self.ordered_player_ids() {
                    let Some(player) = self.players.get(&pid) else {
                        continue;
                    };
                    for card in &player.battlefield {
                        if !matches!(card.kind, CardKind::Unit(_)) {
                            continue;
                        }
                        if self.card_combat_stats(card).1 >= *min_toughness {
                            to_remove.push((pid.clone(), card.id.clone()));
                        }
                    }
                }
                for (pid, cid) in to_remove {
                    self.remove_from_battlefield_and_attachments(&pid, &cid);
                }
            }
            CardEffect::CopyLastCast => {
                if let Some(last) = self.history.last().cloned() {
                    self.stack.push(last);
                }
            }
            CardEffect::Purify => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        self.effect_manager.remove_effects_for_card(card_id);
                    }
                }
            }
            CardEffect::GrantKeyword(keyword, expires) => {
                let stat = match keyword {
                    Keyword::Deathtouch => Some(StatType::Deathtouch),
                    Keyword::Trample => Some(StatType::Trample),
                    Keyword::Aero => Some(StatType::Aero),
                    Keyword::Interceptor => Some(StatType::Interceptor),
                    Keyword::Stealth => Some(StatType::Stealth),
                    Keyword::Camouflage => Some(StatType::Camouflage),
                    Keyword::Indestructible => Some(StatType::Indestructible),
                    Keyword::Vigilance => Some(StatType::Vigilance),
                    Keyword::Haste => Some(StatType::Haste),
                    Keyword::ReactiveArmor(_) => Some(StatType::Armor),
                    _ => None,
                };
                if let Some(stat) = stat {
                    let modifier = StatModifierSpec {
                        stat,
                        amount: ModifierAmount::Fixed(match keyword {
                            Keyword::ReactiveArmor(x) => *x,
                            _ => 1,
                        }),
                        expires: expires.clone(),
                        target: EffectTargetSpec::Targets,
                    };

                    if targets.is_empty() {
                        let mut card_ids = Vec::new();
                        if let Some(owner) = self.players.get(owner_id) {
                            for card in &owner.battlefield {
                                if matches!(card.kind, CardKind::Unit(_)) {
                                    card_ids.push(card.id.clone());
                                }
                            }
                        }
                        for card_id in card_ids {
                            self.add_stat_modifier(
                                owner_id,
                                EffectTarget::Card(card_id),
                                &modifier,
                            );
                        }
                    } else {
                        for target in targets {
                            if let TargetRef::Card(card_id) = target {
                                self.add_stat_modifier(
                                    owner_id,
                                    EffectTarget::Card(card_id.clone()),
                                    &modifier,
                                );
                            }
                        }
                    }
                }
            }
            CardEffect::Discard(amount) => {
                for pid in self.ordered_player_ids() {
                    let Some(player) = self.players.get_mut(&pid) else {
                        continue;
                    };
                    if pid != owner_id {
                        if player.hand.is_empty() {
                            continue;
                        }
                        let actual = (*amount).min(player.hand.len());
                        player.pending_actions.push(PendingAction::Discard {
                            amount: actual,
                            source_description: Self::effect_text(effect, None),
                            source_card_id: source_id.map(str::to_string),
                        });
                    }
                }
            }
            CardEffect::DiscardSelfRandom(amount) => {
                use rand::rngs::StdRng;
                use rand::{SeedableRng, seq::SliceRandom};
                let seed = self.next_random_seed(owner_id, "discard_self_random");
                let Some(player) = self.players.get_mut(owner_id) else {
                    return;
                };
                if player.hand.is_empty() || *amount == 0 {
                    return;
                }
                let actual = (*amount).min(player.hand.len());
                let mut indices: Vec<usize> = (0..player.hand.len()).collect();
                match seed {
                    Some(seed) => {
                        let mut rng = StdRng::seed_from_u64(seed);
                        indices.shuffle(&mut rng);
                    }
                    None => {
                        let mut rng = rand::rng();
                        indices.shuffle(&mut rng);
                    }
                }
                indices.truncate(actual);
                indices.sort_unstable_by(|a, b| b.cmp(a));
                for idx in indices {
                    let card = player.hand.remove(idx);
                    player.graveyard.push(card);
                }
            }
            CardEffect::StealMana(_mana_type, amount) => {
                let mut stolen = 0;
                for pid in self.ordered_player_ids() {
                    let Some(_player) = self.players.get_mut(&pid) else {
                        continue;
                    };
                    if pid != owner_id {
                        // Simple logic for now
                        stolen += *amount;
                    }
                }
                if let Some(player) = self.players.get_mut(owner_id) {
                    player.mana_pool.colorless += stolen;
                }
            }
            CardEffect::ShuffleLibrary(scope) => {
                let target_ids: Vec<String> = match scope {
                    TargetScope::SelfPlayer => vec![owner_id.to_string()],
                    TargetScope::Opponent => self
                        .ordered_player_ids()
                        .into_iter()
                        .filter(|pid| pid != owner_id)
                        .collect(),
                    TargetScope::Any | TargetScope::AnyPlayerOrUnit => self.ordered_player_ids(),
                };

                for pid in target_ids {
                    self.shuffle_deck_for_player(&pid, "shuffle_library");
                }
            }
            CardEffect::ForbidName => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        // Find the card's name and its current owner
                        let mut name_to_forbid = None;
                        let mut target_owner_id = None;

                        for pid in self.ordered_player_ids() {
                            let Some(player) = self.players.get(&pid) else {
                                continue;
                            };
                            if let Some(card) = player.battlefield.iter().find(|c| &c.id == card_id)
                            {
                                name_to_forbid = Some(card.name.clone());
                                target_owner_id = Some(pid.clone());
                                break;
                            }
                            if let Some(card) = player.hand.iter().find(|c| &c.id == card_id) {
                                name_to_forbid = Some(card.name.clone());
                                target_owner_id = Some(pid.clone());
                                break;
                            }
                        }

                        if let (Some(name), Some(pid)) = (name_to_forbid, target_owner_id) {
                            self.forbidden_names
                                .entry(pid)
                                .or_default()
                                .push((name, self.round + 1));
                        }
                    }
                }
            }
            CardEffect::Fight => {
                let unit_ids: Vec<String> = targets
                    .iter()
                    .filter_map(|t| match t {
                        TargetRef::Card(id) => Some(id.clone()),
                        _ => None,
                    })
                    .collect();

                if unit_ids.len() == 2 {
                    let id1 = &unit_ids[0];
                    let id2 = &unit_ids[1];

                    let card1_stats = self
                        .find_battlefield_card_owner(id1)
                        .and_then(|(pid, _)| self.players.get(&pid))
                        .and_then(|p| p.battlefield.iter().find(|c| &c.id == id1))
                        .map(|c| self.card_combat_stats(c));

                    let card2_stats = self
                        .find_battlefield_card_owner(id2)
                        .and_then(|(pid, _)| self.players.get(&pid))
                        .and_then(|p| p.battlefield.iter().find(|c| &c.id == id2))
                        .map(|c| self.card_combat_stats(c));

                    if let (Some((p1, _, _, _, dt1, _, _)), Some((p2, _, _, _, dt2, _, _))) =
                        (card1_stats, card2_stats)
                    {
                        // Apply damage to each unit; state-based actions
                        // handle lethality using accumulated damage.
                        for (target_id, damage) in [(id2, p1), (id1, p2)] {
                            if damage > 0 {
                                if let Some((owner_id, _)) =
                                    self.find_battlefield_card_owner(target_id)
                                {
                                    if let Some(owner) = self.players.get_mut(&owner_id) {
                                        if let Some(card) = owner
                                            .battlefield
                                            .iter_mut()
                                            .find(|c| &c.id == target_id)
                                        {
                                            card.damage += damage;
                                        }
                                    }
                                }
                            }
                        }
                        // Deathtouch bypasses toughness — mark directly.
                        let mut dead = Vec::new();
                        if dt1 && p1 > 0 {
                            dead.push(id2.clone());
                        }
                        if dt2 && p2 > 0 {
                            dead.push(id1.clone());
                        }
                        for dead_id in dead {
                            if let Some((owner_id, _)) = self.find_battlefield_card_owner(&dead_id)
                            {
                                self.remove_from_battlefield_and_attachments(&owner_id, &dead_id);
                            }
                        }
                    }
                }
            }
            CardEffect::TapTarget => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        if let Some((owner_id, _)) = self.find_battlefield_card_owner(card_id) {
                            if let Some(owner) = self.players.get_mut(&owner_id) {
                                if let Some(card) = owner
                                    .battlefield
                                    .iter_mut()
                                    .find(|card| &card.id == card_id)
                                {
                                    card.tapped = true;
                                }
                            }
                        }
                    }
                }
            }
            CardEffect::UntapTarget => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        if let Some((owner_id, _)) = self.find_battlefield_card_owner(card_id) {
                            if let Some(owner) = self.players.get_mut(&owner_id) {
                                if let Some(card) = owner
                                    .battlefield
                                    .iter_mut()
                                    .find(|card| &card.id == card_id)
                                {
                                    card.tapped = false;
                                }
                            }
                        }
                    }
                }
            }
            CardEffect::Equip => {
                // Called when an Equip ability resolves without the stack
                // (uses_stack: false). Delegates to the same engine helper
                // the stack path uses.
                if let Some(source_id) = source_id {
                    if let Some(TargetRef::Card(target_id)) = targets.first() {
                        let _ = self.attach_equipment(owner_id, source_id, target_id);
                    }
                }
            }
            CardEffect::AddManaScaling(mana_type, target_type, target_kind) => {
                let count = if let Some(player) = self.players.get(owner_id) {
                    player
                        .battlefield
                        .iter()
                        .filter(|card| {
                            let kind_matches = target_kind
                                .as_ref()
                                .map(|k| &card.kind == k)
                                .unwrap_or(true);
                            let type_matches = target_type
                                .as_ref()
                                .map(|t| match &card.kind {
                                    CardKind::Unit(ct) => ct == t,
                                    _ => false,
                                })
                                .unwrap_or(true);
                            kind_matches && type_matches
                        })
                        .count()
                } else {
                    0
                };

                if let Some(player) = self.players.get_mut(owner_id) {
                    let scaled = Self::scaled_mana_amount(mana_type, count as i32);
                    match mana_type {
                        ManaType::Neon => player.mana_pool.neon += scaled,
                        ManaType::Glitch => player.mana_pool.glitch += scaled,
                        ManaType::Chrome => player.mana_pool.chrome += scaled,
                        ManaType::Slag => player.mana_pool.slag += scaled,
                        ManaType::Bio => player.mana_pool.bio += scaled,
                        ManaType::Colorless => player.mana_pool.colorless += scaled,
                    }
                }
            }
            CardEffect::ModifyLoyalty(amount) => {
                // Find the source card (usually the Agent activating the ability)
                // In our current engine, effect source is not explicitly passed to apply_effect easily
                // but we can look for Agents on owner's battlefield.
                // Simplified: Apply to ALL Agents owned by owner (usually just one)
                if let Some(player) = self.players.get_mut(owner_id) {
                    for card in &mut player.battlefield {
                        if matches!(card.kind, CardKind::Agent) {
                            if let Some(loyalty) = &mut card.loyalty {
                                *loyalty += amount;
                            }
                        }
                    }
                }
            }
            CardEffect::Emblem(keywords, stats) => {
                // Emblems are implemented as permanent stat modifiers/keyword grants
                // that target "OwnerUnits(None)".
                if let Some(stats) = stats {
                    self.add_stat_modifier(
                        owner_id,
                        EffectTarget::Player(owner_id.to_string()),
                        stats,
                    );
                }
                for kw in keywords {
                    let stat = match kw {
                        Keyword::Indestructible => Some(StatType::Indestructible),
                        Keyword::Vigilance => Some(StatType::Vigilance),
                        Keyword::Trample => Some(StatType::Trample),
                        _ => None,
                    };
                    if let Some(stat) = stat {
                        self.add_stat_modifier(
                            owner_id,
                            EffectTarget::Player(owner_id.to_string()),
                            &StatModifierSpec {
                                stat,
                                amount: ModifierAmount::Fixed(1),
                                expires: ExpireContract::Never,
                                target: EffectTargetSpec::OwnerUnits(None),
                            },
                        );
                    }
                }
            }
            CardEffect::SacrificeThis => {
                if let Some(source_id) = source_id {
                    if let Some((sacrifice_owner_id, _)) =
                        self.find_battlefield_card_owner(source_id)
                    {
                        self.remove_from_battlefield_and_attachments(
                            &sacrifice_owner_id,
                            source_id,
                        );
                        self.apply_all_battlefield_triggers_for_event(
                            &TriggerEvent::Sacrificed,
                            &[TargetRef::Player(sacrifice_owner_id)],
                        );
                    }
                }
            }
            CardEffect::ReanimateNamedCard(card_name) => {
                let mut reanimated = None;
                if let Some(player) = self.players.get_mut(owner_id) {
                    if let Some(idx) = player.graveyard.iter().position(|c| c.name == *card_name) {
                        let mut card = player.graveyard.remove(idx);
                        card.tapped = false;
                        card.attached_to = None;
                        card.remembered_player_id = None;
                        card.attachment = None;
                        card.summoning_sickness = matches!(card.kind, CardKind::Unit(_));
                        card.creator_id = source_id.map(|s| s.to_string());
                        player.battlefield.push(card.clone());
                        reanimated = Some(card);
                    }
                }
                if let Some(card) = reanimated {
                    self.sim_debug(format!(
                        "reanimate_entered owner={} card={} name={}",
                        owner_id, card.id, card.name
                    ));
                    self.handle_card_entered(owner_id, &card, &[]);
                }
            }
            CardEffect::CopyUnit => {
                let Some(TargetRef::Card(target_id)) = targets.first() else {
                    return;
                };

                let mut template = None;
                for player_id in self.ordered_player_ids() {
                    let Some(player) = self.players.get(&player_id) else {
                        continue;
                    };
                    if let Some(card) = player
                        .battlefield
                        .iter()
                        .find(|card| card.id == *target_id)
                        .cloned()
                    {
                        template = Some(card);
                        break;
                    }
                }

                let Some(template) = template else {
                    return;
                };

                let copy_id = self
                    .players
                    .get(owner_id)
                    .map(|player| format!("{}-copy-{}", template.id, player.battlefield.len()))
                    .unwrap_or_else(|| format!("{}-copy", template.id));

                let mut copy = template;
                copy.id = copy_id;
                copy.tapped = false;
                copy.attached_to = None;
                copy.remembered_player_id = None;
                copy.attachment = None;
                copy.summoning_sickness = matches!(copy.kind, CardKind::Unit(_));
                copy.creator_id = source_id.map(|s| s.to_string());
                // Strip recursive `CopyUnit` triggers from the copy so a
                // chain of mutually-copying permanents (e.g. two Synthesis
                // Reconstructors) can't pump new copies forever through the
                // pending-action queue. Other triggers (ETB draw, on-death,
                // etc.) are preserved so copies of other cards still work.
                copy.triggers.retain(|t| !copy_effect_recurses(&t.effect));

                if let Some(owner) = self.players.get_mut(owner_id) {
                    owner.battlefield.push(copy.clone());
                }
                self.sim_debug(format!(
                    "copy_entered owner={} card={} name={}",
                    owner_id, copy.id, copy.name
                ));
                self.handle_card_entered(owner_id, &copy, &[]);
            }
            CardEffect::Mill(count) => {
                let target_player_id = targets
                    .iter()
                    .find_map(|target| match target {
                        TargetRef::Player(player_id) => Some(player_id.clone()),
                        _ => None,
                    })
                    .or_else(|| self.default_defender_id(owner_id));

                if let Some(target_player_id) = target_player_id {
                    if let Some(player) = self.players.get_mut(&target_player_id) {
                        let actual = (*count).min(player.deck.len());
                        for _ in 0..actual {
                            if let Some(card) = player.deck.pop() {
                                player.graveyard.push(card);
                            }
                        }
                    }
                }
            }
            CardEffect::BounceAllUnits => {
                let targets: Vec<TargetRef> = self
                    .players
                    .values()
                    .flat_map(|player| {
                        player
                            .battlefield
                            .iter()
                            .filter(|card| matches!(card.kind, CardKind::Unit(_)))
                            .map(|card| TargetRef::Card(card.id.clone()))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                self.apply_effect(owner_id, &CardEffect::Bounce, &targets);
            }
            CardEffect::BounceAllOpponentPermanents => {
                let targets: Vec<TargetRef> = self
                    .players
                    .iter()
                    .filter(|(player_id, _)| player_id.as_str() != owner_id)
                    .flat_map(|(_, player)| {
                        player
                            .battlefield
                            .iter()
                            .map(|card| TargetRef::Card(card.id.clone()))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                self.apply_effect(owner_id, &CardEffect::Bounce, &targets);
            }
            CardEffect::BounceAllAttacking => {
                let targets: Vec<TargetRef> = self
                    .combat_attackers
                    .iter()
                    .cloned()
                    .map(TargetRef::Card)
                    .collect();
                self.apply_effect(owner_id, &CardEffect::Bounce, &targets);
            }
            CardEffect::ReanimateTargetUnit => {
                let search_owner_id = targets
                    .iter()
                    .find_map(|target| match target {
                        TargetRef::Player(player_id) => Some(player_id.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| owner_id.to_string());

                let mut reanimated = None;
                if let Some(player) = self.players.get_mut(&search_owner_id) {
                    if let Some(idx) = player
                        .graveyard
                        .iter()
                        .position(|c| matches!(c.kind, CardKind::Unit(_)))
                    {
                        let mut card = player.graveyard.remove(idx);
                        card.summoning_sickness = true;
                        card.tapped = false;
                        card.attached_to = None;
                        card.remembered_player_id = None;
                        card.attachment = None;
                        card.creator_id = source_id.map(|s| s.to_string());
                        reanimated = Some(card);
                    }
                }

                if let Some(card) = reanimated {
                    if let Some(owner) = self.players.get_mut(owner_id) {
                        owner.battlefield.push(card.clone());
                    }
                    self.sim_debug(format!(
                        "reanimate_target_entered owner={} card={} name={}",
                        owner_id, card.id, card.name
                    ));
                    self.handle_card_entered(owner_id, &card, &[]);
                }
            }
            CardEffect::AddCounters(count) => {
                // Add +1/+1 counters to the source card (self-targeting)
                // If explicit targets provided, use those; otherwise find source card
                if !targets.is_empty() {
                    for target in targets {
                        if let TargetRef::Card(card_id) = target {
                            for player_id in self.ordered_player_ids() {
                                let Some(player) = self.players.get_mut(&player_id) else {
                                    continue;
                                };
                                if let Some(card) =
                                    player.battlefield.iter_mut().find(|c| c.id == *card_id)
                                {
                                    card.counters += count;
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    // Self-targeting: find any card on the owner's battlefield that has this trigger
                    // The caller should pass source_id as a target for self-buffing
                }
            }
            CardEffect::RemoveCounters(count) => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        for player_id in self.ordered_player_ids() {
                            let Some(player) = self.players.get_mut(&player_id) else {
                                continue;
                            };
                            if let Some(card) = player
                                .battlefield
                                .iter_mut()
                                .find(|card| card.id == *card_id)
                            {
                                card.counters = (card.counters - *count).max(0);
                                break;
                            }
                        }
                    }
                }
            }
            CardEffect::PumpByMana => {
                for stat in [StatType::Power, StatType::Toughness] {
                    self.apply_stat_modifier_spec(
                        owner_id,
                        source_id,
                        &StatModifierSpec {
                            stat,
                            amount: ModifierAmount::Dynamic {
                                source: DynamicSource::ManaTotal {
                                    scope: PlayerScope::Owner,
                                },
                                multiplier: 1,
                                offset: 0,
                            },
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(None),
                        },
                        targets,
                    );
                }
            }

            // --- New effects ---
            CardEffect::Siphon(amount) => {
                let target_id = targets
                    .first()
                    .and_then(|t| {
                        if let TargetRef::Player(id) = t {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| self.default_defender_id(owner_id));
                if let Some(target_id) = target_id {
                    if let Some(target) = self.players.get_mut(&target_id) {
                        let actual = (*amount).min(target.life);
                        target.life = (target.life - amount).max(0);
                        if let Some(owner) = self.players.get_mut(owner_id) {
                            owner.life += actual;
                        }
                    }
                }
            }
            CardEffect::TransmuteMana(from, to, amount) => {
                if let Some(owner) = self.players.get_mut(owner_id) {
                    let available = match from {
                        ManaType::Neon => &mut owner.mana_pool.neon,
                        ManaType::Glitch => &mut owner.mana_pool.glitch,
                        ManaType::Chrome => &mut owner.mana_pool.chrome,
                        ManaType::Slag => &mut owner.mana_pool.slag,
                        ManaType::Bio => &mut owner.mana_pool.bio,
                        ManaType::Colorless => &mut owner.mana_pool.colorless,
                    };
                    let actual = (*amount).min(*available);
                    *available -= actual;
                    match to {
                        ManaType::Neon => owner.mana_pool.neon += actual,
                        ManaType::Glitch => owner.mana_pool.glitch += actual,
                        ManaType::Chrome => owner.mana_pool.chrome += actual,
                        ManaType::Slag => owner.mana_pool.slag += actual,
                        ManaType::Bio => owner.mana_pool.bio += actual,
                        ManaType::Colorless => owner.mana_pool.colorless += actual,
                    }
                }
            }
            CardEffect::Swap => {
                // Exchange control of two target units
                let unit_ids: Vec<String> = targets
                    .iter()
                    .filter_map(|t| match t {
                        TargetRef::Card(id) => Some(id.clone()),
                        _ => None,
                    })
                    .collect();

                if unit_ids.len() == 2 {
                    let id1 = &unit_ids[0];
                    let id2 = &unit_ids[1];

                    let owner1 = self.find_battlefield_card_owner(id1).map(|(id, _)| id);
                    let owner2 = self.find_battlefield_card_owner(id2).map(|(id, _)| id);

                    if let (Some(owner1), Some(owner2)) = (owner1, owner2) {
                        if owner1 != owner2 {
                            let card1 = self
                                .players
                                .get_mut(&owner1)
                                .and_then(|p| Self::remove_from_battlefield(p, id1));
                            let card2 = self
                                .players
                                .get_mut(&owner2)
                                .and_then(|p| Self::remove_from_battlefield(p, id2));

                            if let Some(card1) = card1 {
                                if let Some(p2) = self.players.get_mut(&owner2) {
                                    p2.battlefield.push(card1);
                                }
                            }
                            if let Some(card2) = card2 {
                                if let Some(p1) = self.players.get_mut(&owner1) {
                                    p1.battlefield.push(card2);
                                }
                            }
                        }
                    }
                }
            }
            CardEffect::Phase => {
                // Target permanent phases out — remove from battlefield, store in exile with a marker
                // It returns at next upkeep (handled in game_flow)
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        if let Some((card_owner_id, _)) = self.find_battlefield_card_owner(card_id)
                        {
                            let removed = self
                                .players
                                .get_mut(&card_owner_id)
                                .and_then(|p| Self::remove_from_battlefield(p, card_id));
                            if let Some(mut card) = removed {
                                card.tapped = false;
                                // Store in phased_out list (using exile zone with a tag)
                                if let Some(player) = self.players.get_mut(&card_owner_id) {
                                    player.exile.push(card);
                                }
                                self.phased_out
                                    .push((card_owner_id.clone(), card_id.clone()));
                            }
                        }
                    }
                }
            }
            CardEffect::MutateTarget(spec) => {
                // Permanently alter a unit's base stats (rewrites the card, can't be purified)
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        for player_id in self.ordered_player_ids() {
                            let Some(player) = self.players.get_mut(&player_id) else {
                                continue;
                            };
                            if let Some(card) =
                                player.battlefield.iter_mut().find(|c| c.id == *card_id)
                            {
                                let amount = match &spec.amount {
                                    ModifierAmount::Fixed(v) => *v,
                                    _ => 0,
                                };
                                match &spec.stat {
                                    StatType::Power => {
                                        if let Some(p) = &mut card.power {
                                            *p += amount;
                                        }
                                    }
                                    StatType::Toughness => {
                                        if let Some(t) = &mut card.toughness {
                                            *t += amount;
                                        }
                                    }
                                    StatType::Both => {
                                        if let Some(p) = &mut card.power {
                                            *p += amount;
                                        }
                                        if let Some(t) = &mut card.toughness {
                                            *t += amount;
                                        }
                                    }
                                    _ => {}
                                }
                                // Check if unit died from stat reduction
                                if card.toughness.unwrap_or(0) <= 0 {
                                    let dead_id = card.id.clone();
                                    let dead_owner = player_id.clone();
                                    drop(player);
                                    self.remove_from_battlefield_and_attachments(
                                        &dead_owner,
                                        &dead_id,
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
            }
            CardEffect::Polymorph => {
                for target in targets {
                    if let TargetRef::Card(card_id) = target {
                        let card_owner =
                            self.find_battlefield_card_owner(card_id).map(|(id, _)| id);
                        if let Some(card_owner) = card_owner {
                            self.remove_from_battlefield_and_attachments(&card_owner, card_id);
                            // Reveal cards from top of library until a unit is found
                            let mut revealed = Vec::new();
                            let mut found_unit = None;
                            if let Some(player) = self.players.get_mut(&card_owner) {
                                while let Some(card) = player.deck.pop() {
                                    if matches!(card.kind, CardKind::Unit(_)) {
                                        found_unit = Some(card);
                                        break;
                                    }
                                    revealed.push(card);
                                }
                                // Put revealed non-units on bottom of library
                                for card in revealed {
                                    player.deck.insert(0, card);
                                }
                            }
                            if let Some(mut unit) = found_unit {
                                unit.summoning_sickness = true;
                                unit.tapped = false;
                                if let Some(player) = self.players.get_mut(&card_owner) {
                                    player.battlefield.push(unit.clone());
                                }
                                self.handle_card_entered(&card_owner, &unit, &[]);
                            }
                        }
                    }
                }
            }
            CardEffect::Surveil(amount) => {
                // Like Scry but cards can go to graveyard instead of bottom
                // Implementation: use the same scry UI — cards put on "bottom" go to graveyard instead
                if let Some(player) = self.players.get_mut(owner_id) {
                    let count = (*amount).min(player.deck.len());
                    let start = player.deck.len().saturating_sub(count);
                    let mut scry_cards: Vec<Card> = player.deck[start..].iter().cloned().collect();
                    scry_cards.reverse();
                    player.scry = scry_cards;
                    player.surveil_active = true;
                }
            }
            CardEffect::Transmit(amount) => {
                // Look at top X cards, put one into hand, rest on bottom
                // Reuse the search UI: present top X as scry choices, selected one goes to hand
                if let Some(player) = self.players.get_mut(owner_id) {
                    let count = (*amount).min(player.deck.len());
                    let start = player.deck.len().saturating_sub(count);
                    let cards: Vec<Card> = player.deck[start..].iter().cloned().collect();
                    // Remove from deck
                    player.deck.truncate(start);
                    if !cards.is_empty() {
                        player.scry = cards;
                        player.library_search_active = true;
                    }
                }
            }
            CardEffect::Entomb(predicate) => {
                // Search library for card matching predicate, put into graveyard
                let found = self.players.get(owner_id).and_then(|player| {
                    player
                        .deck
                        .iter()
                        .position(|card| Self::card_matches_predicate(card, predicate))
                });

                if let Some(idx) = found {
                    if let Some(player) = self.players.get_mut(owner_id) {
                        let card = player.deck.remove(idx);
                        player.graveyard.push(card);
                    }
                    self.shuffle_deck_for_player(owner_id, "entomb");
                }
            }
            CardEffect::Recycle => {
                if let Some(player) = self.players.get_mut(owner_id) {
                    let graveyard_cards: Vec<Card> = player.graveyard.drain(..).collect();
                    player.deck.extend(graveyard_cards);
                }
                self.shuffle_deck_for_player(owner_id, "recycle");
            }
            CardEffect::DamageSplit(amount) => {
                // Deal X damage divided among targets. Simplified: deal X to each target.
                for target in targets {
                    match target {
                        TargetRef::Player(target_id) => {
                            if let Some(target) = self.players.get_mut(target_id) {
                                target.life = (target.life - amount).max(0);
                            }
                        }
                        TargetRef::Card(card_id) => {
                            if let Some((owner_id, _)) = self.find_battlefield_card_owner(card_id) {
                                if let Some(owner) = self.players.get_mut(&owner_id) {
                                    if let Some(card) =
                                        owner.battlefield.iter_mut().find(|c| &c.id == card_id)
                                    {
                                        card.damage += amount;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            CardEffect::DamageEachOpponentUnit(amount) => {
                let mut targets = Vec::new();
                for pid in self.ordered_player_ids() {
                    if pid == owner_id {
                        continue;
                    }
                    let Some(player) = self.players.get(&pid) else {
                        continue;
                    };
                    for card in &player.battlefield {
                        if matches!(card.kind, CardKind::Unit(_)) {
                            targets.push((pid.clone(), card.id.clone()));
                        }
                    }
                }
                for (pid, cid) in targets {
                    if let Some(player) = self.players.get_mut(&pid) {
                        if let Some(card) = player.battlefield.iter_mut().find(|c| c.id == cid) {
                            card.damage += amount;
                        }
                    }
                }
            }
            CardEffect::Reflect(amount) => {
                // Grant a temporary "reactive damage" buff via stat modifier
                // Simplified: deal the amount to each opponent
                for pid in self.ordered_player_ids() {
                    if pid != owner_id {
                        if let Some(player) = self.players.get_mut(&pid) {
                            player.life = (player.life - amount).max(0);
                        }
                    }
                }
            }
            CardEffect::Conscript => {
                // Reanimate target unit from opponent's graveyard under your control
                let mut reanimated = None;
                for pid in self.ordered_player_ids() {
                    if pid == owner_id {
                        continue;
                    }
                    if let Some(player) = self.players.get_mut(&pid) {
                        if let Some(idx) = player
                            .graveyard
                            .iter()
                            .position(|c| matches!(c.kind, CardKind::Unit(_)))
                        {
                            let mut card = player.graveyard.remove(idx);
                            card.summoning_sickness = true;
                            card.tapped = false;
                            card.attached_to = None;
                            card.remembered_player_id = None;
                            card.attachment = None;
                            reanimated = Some(card);
                            break;
                        }
                    }
                }
                if let Some(card) = reanimated {
                    if let Some(owner) = self.players.get_mut(owner_id) {
                        owner.battlefield.push(card.clone());
                    }
                    self.handle_card_entered(owner_id, &card, &[]);
                }
            }
            CardEffect::Scavenge(amount) => {
                // Exile a unit from your graveyard, target unit gets +X/+X counters
                // Show graveyard units as search options so the player can choose
                let matching: Vec<Card> = self
                    .players
                    .get(owner_id)
                    .map(|p| {
                        p.graveyard
                            .iter()
                            .filter(|c| matches!(c.kind, CardKind::Unit(_)))
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();

                if matching.is_empty() {
                    // No units in graveyard — skip
                } else if matching.len() == 1 {
                    // Only one option — auto-exile
                    let card_id = matching[0].id.clone();
                    if let Some(player) = self.players.get_mut(owner_id) {
                        if let Some(idx) = player.graveyard.iter().position(|c| c.id == card_id) {
                            let card = player.graveyard.remove(idx);
                            player.exile.push(card);
                        }
                    }
                    self.apply_effect_with_source(
                        owner_id,
                        source_id,
                        &CardEffect::AddCounters(*amount),
                        targets,
                    );
                } else {
                    // Multiple options — let the player choose
                    if let Some(player) = self.players.get_mut(owner_id) {
                        player.scry = matching;
                        player.library_search_active = true;
                        player.scavenge_pending = Some((*amount, targets.to_vec()));
                    }
                }
            }
            CardEffect::Assimilate(target_type) => {
                // Take control of target opponent unit and convert its
                // UnitType to the specified type (viral infestation /
                // indoctrination flavor — parameterized so any color can use it).
                for target in targets {
                    let TargetRef::Card(card_id) = target else {
                        continue;
                    };
                    let Some((current_owner, _)) = self.find_battlefield_card_owner(card_id) else {
                        continue;
                    };
                    if current_owner == owner_id {
                        continue;
                    }
                    let removed = self
                        .players
                        .get_mut(&current_owner)
                        .and_then(|p| Self::remove_from_battlefield(p, card_id));
                    let Some(mut card) = removed else {
                        continue;
                    };
                    if let CardKind::Unit(_) = card.kind {
                        card.kind = CardKind::Unit(target_type.clone());
                    }
                    card.tapped = false;
                    card.summoning_sickness = true;
                    card.attached_to = None;
                    card.attachment = None;
                    if let Some(new_owner) = self.players.get_mut(owner_id) {
                        new_owner.battlefield.push(card);
                    }
                }
            }
        }
    }

    pub(crate) fn card_matches_predicate(card: &Card, predicate: &Predicate) -> bool {
        match predicate {
            Predicate::CardKind(kind) => &card.kind == kind,
            Predicate::IsUnit => matches!(card.kind, CardKind::Unit(_)),
            Predicate::All(predicates) => predicates
                .iter()
                .all(|p| Self::card_matches_predicate(card, p)),
            Predicate::Any(predicates) => predicates
                .iter()
                .any(|p| Self::card_matches_predicate(card, p)),
            Predicate::Not(inner) => !Self::card_matches_predicate(card, inner),
            Predicate::Condition(condition) => match condition {
                Condition::TargetIsUnitType(unit_type) => {
                    matches!(&card.kind, CardKind::Unit(t) if t == unit_type)
                }
                Condition::TargetIsCardKind(kind) => &card.kind == kind,
                Condition::TargetIsApex => card.supertype == CardSupertype::Apex,
                _ => false,
            },
        }
    }
}
