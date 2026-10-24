use crate::card::*;
use crate::deck::build_deck_catalog;
use crate::handlers::DRAFT_WINNER_PICK_COUNT;
use crate::state::*;
use crate::types::*;
use crate::view::*;
use std::collections::{HashMap, HashSet};

/// View construction functions - builds view models for the UI
impl CardGameState {
    /// Short display label for a keyword (e.g. "Stealth", "Haste"). Used in
    /// the stat-modifier provenance list to show what a Lord-style source
    /// grants. Takes the first token of the full keyword text before the " — ".
    fn keyword_short_label(keyword: &Keyword) -> String {
        let full = Self::keyword_text(keyword);
        full.split(" — ").next().unwrap_or(&full).to_string()
    }

    pub fn deck_choices() -> Vec<DeckChoiceView> {
        let catalog = build_deck_catalog();

        catalog
            .into_iter()
            .map(|deck| {
                // Compute colors from deck's card composition
                let mut mana_types = HashSet::new();

                for entry in &deck.cards {
                    let cost = &entry.card.cost;
                    if cost.neon > 0 {
                        mana_types.insert(ManaType::Neon);
                    }
                    if cost.bio > 0 {
                        mana_types.insert(ManaType::Bio);
                    }
                    if cost.slag > 0 {
                        mana_types.insert(ManaType::Slag);
                    }
                    if cost.glitch > 0 {
                        mana_types.insert(ManaType::Glitch);
                    }
                    if cost.chrome > 0 {
                        mana_types.insert(ManaType::Chrome);
                    }
                }

                // Convert to sorted hex colors for consistent ordering
                let mut colors: Vec<String> = mana_types
                    .iter()
                    .map(|mt| mana_color_for_type(mt).to_string())
                    .collect();
                colors.sort();

                // Fallback to colorless if no colors found
                if colors.is_empty() {
                    colors.push("#64748b".to_string()); // Slate gray for colorless
                }

                DeckChoiceView {
                    id: deck.id,
                    name: deck.name,
                    description: deck.description,
                    colors,
                    image_url: None,
                }
            })
            .collect()
    }

    /// Compute the set of valid target IDs (card IDs for units, player IDs
    /// for players) for the given target spec cast by `owner_id`. Mirrors
    /// the server-side validation in `validate_targets_with_spec` so the
    /// UI highlights exactly what the backend would accept.
    pub fn valid_targets_for_spec(&self, owner_id: &str, spec: &TargetSpec) -> Vec<String> {
        let mut ids = Vec::new();
        let scope = &spec.scope;
        let kind = &spec.kind;

        // Players.
        if matches!(kind, TargetKind::Player | TargetKind::PlayerOrUnit) {
            for pid in self.players.keys() {
                let ok = match scope {
                    TargetScope::SelfPlayer => pid == owner_id,
                    TargetScope::Opponent => pid != owner_id,
                    TargetScope::Any | TargetScope::AnyPlayerOrUnit => true,
                };
                if ok {
                    ids.push(pid.clone());
                }
            }
        }

        // Battlefield units.
        if matches!(kind, TargetKind::Unit | TargetKind::PlayerOrUnit) {
            for (pid, player) in &self.players {
                for c in &player.battlefield {
                    if !matches!(c.kind, CardKind::Unit(_)) {
                        continue;
                    }
                    let scope_ok = match scope {
                        TargetScope::SelfPlayer => pid == owner_id,
                        TargetScope::Opponent => pid != owner_id,
                        TargetScope::Any | TargetScope::AnyPlayerOrUnit => true,
                    };
                    if !scope_ok {
                        continue;
                    }
                    // Hexproof / Camouflage from the caster's perspective.
                    if !self.can_target_card(owner_id, &c.id) {
                        continue;
                    }
                    ids.push(c.id.clone());
                }
            }
        }

        ids
    }

    pub fn card_view_for(&self, owner_id: &str, card: &Card, zone: Option<CardZone>) -> CardView {
        let unit_type = match &card.kind {
            CardKind::Unit(kind) => Some(kind.clone()),
            _ => None,
        };
        let mut kind_label = Self::kind_label(&card.kind).to_string();
        if card.supertype == CardSupertype::Apex {
            kind_label = format!("apex {}", kind_label);
        }
        let attached_to_name = card
            .attached_to
            .as_ref()
            .and_then(|id| self.find_card_name(id));
        let remembered_player_name = card
            .remembered_player_id
            .as_ref()
            .and_then(|id| self.find_card_name(id));
        let attachment_bonus = card
            .attached_to
            .as_ref()
            .map(|_| (0, 0, 0, 0, false))
            .unwrap_or_else(|| self.attachment_modifiers_for_card(&card.id));

        // Calculate static "Lord" buffs from other units
        let mut static_p = 0;
        let mut static_t = 0;
        let mut granted_keywords: Vec<Keyword> = Vec::new();
        let mut stat_modifiers: Vec<StatModifierSourceView> = Vec::new();
        if let Some(player) = self.players.get(owner_id) {
            for other in &player.battlefield {
                if other.id == card.id {
                    continue;
                }
                let mut lord_p = 0i32;
                let mut lord_t = 0i32;
                let mut lord_grants: Vec<Keyword> = Vec::new();
                for kw in &other.keywords {
                    match kw {
                        Keyword::StaticPowerBuff(amt, target_type) => {
                            let matches_type = target_type
                                .as_ref()
                                .map(|t| unit_type.as_ref() == Some(t))
                                .unwrap_or(true);
                            if matches_type {
                                static_p += amt;
                                lord_p += amt;
                            }
                        }
                        Keyword::StaticToughnessBuff(amt, target_type) => {
                            let matches_type = target_type
                                .as_ref()
                                .map(|t| unit_type.as_ref() == Some(t))
                                .unwrap_or(true);
                            if matches_type {
                                static_t += amt;
                                lord_t += amt;
                            }
                        }
                        Keyword::StaticHasteGrant(target) => {
                            if target.matches(owner_id, owner_id, unit_type.as_ref())
                                && !card.keywords.contains(&Keyword::Haste)
                            {
                                granted_keywords.push(Keyword::Haste);
                                lord_grants.push(Keyword::Haste);
                            }
                        }
                        Keyword::StaticTrampleGrant(target) => {
                            if target.matches(owner_id, owner_id, unit_type.as_ref())
                                && !card.keywords.contains(&Keyword::Trample)
                            {
                                granted_keywords.push(Keyword::Trample);
                                lord_grants.push(Keyword::Trample);
                            }
                        }
                        Keyword::StaticStealthGrant(target) => {
                            if target.matches(owner_id, owner_id, unit_type.as_ref())
                                && !card.keywords.contains(&Keyword::Stealth)
                            {
                                granted_keywords.push(Keyword::Stealth);
                                lord_grants.push(Keyword::Stealth);
                            }
                        }
                        Keyword::StaticIndestructibleGrant(target) => {
                            if target.matches(owner_id, owner_id, unit_type.as_ref())
                                && !card.keywords.contains(&Keyword::Indestructible)
                            {
                                granted_keywords.push(Keyword::Indestructible);
                                lord_grants.push(Keyword::Indestructible);
                            }
                        }
                        Keyword::StaticKeywordGrant(keyword, target) => {
                            if target.matches(owner_id, owner_id, unit_type.as_ref())
                                && !card.keywords.contains(keyword)
                            {
                                granted_keywords.push(*keyword.clone());
                                lord_grants.push(*keyword.clone());
                            }
                        }
                        _ => {}
                    }
                }
                let has_grants = !lord_grants.is_empty();
                if lord_p != 0 || lord_t != 0 || has_grants {
                    stat_modifiers.push(StatModifierSourceView {
                        source_card_id: Some(other.id.clone()),
                        source_card_name: Some(other.name.clone()),
                        power_mod: lord_p,
                        toughness_mod: lord_t,
                        label: other.name.clone(),
                        granted_keywords: lord_grants
                            .iter()
                            .map(Self::keyword_short_label)
                            .collect(),
                    });
                }
            }
        }

        // Collect effect manager modifiers grouped by owner
        {
            let mut effect_groups: HashMap<String, (i32, i32)> = HashMap::new();
            for effect in &self.effect_manager.effects {
                if !matches!(&effect.target, EffectTarget::Card(id) if id == &card.id) {
                    continue;
                }
                let amt = self.resolve_modifier_amount(&effect.amount, &effect.owner_id);
                if amt == 0 {
                    continue;
                }
                let entry = effect_groups
                    .entry(effect.owner_id.clone())
                    .or_insert((0, 0));
                match effect.stat {
                    StatType::Power => entry.0 += amt,
                    StatType::Toughness => entry.1 += amt,
                    StatType::Both => {
                        entry.0 += amt;
                        entry.1 += amt;
                    }
                    _ => {}
                }
            }
            for (eid, (pm, tm)) in effect_groups {
                if pm != 0 || tm != 0 {
                    let source_name = self.find_card_name(&eid);
                    stat_modifiers.push(StatModifierSourceView {
                        source_card_id: Some(eid),
                        source_card_name: source_name.clone(),
                        power_mod: pm,
                        toughness_mod: tm,
                        label: source_name.unwrap_or_else(|| "Spell effect".to_string()),
                        granted_keywords: Vec::new(),
                    });
                }
            }
        }

        // Collect attachment modifiers with source info
        if card.attached_to.is_none() {
            for player in self.players.values() {
                for attached in &player.battlefield {
                    if attached.attached_to.as_deref() != Some(&card.id) {
                        continue;
                    }
                    let Some(attachment) = &attached.attachment else {
                        continue;
                    };
                    let mut att_p = 0i32;
                    let mut att_t = 0i32;
                    for modifier in &attachment.modifiers {
                        let amt = match &modifier.amount {
                            ModifierAmount::Fixed(v) => *v,
                            _ => 0,
                        };
                        match modifier.stat {
                            StatType::Power => att_p += amt,
                            StatType::Toughness => att_t += amt,
                            StatType::Both => {
                                att_p += amt;
                                att_t += amt;
                            }
                            _ => {}
                        }
                    }
                    if att_p != 0 || att_t != 0 {
                        stat_modifiers.push(StatModifierSourceView {
                            source_card_id: Some(attached.id.clone()),
                            source_card_name: Some(attached.name.clone()),
                            power_mod: att_p,
                            toughness_mod: att_t,
                            label: attached.name.clone(),
                            granted_keywords: Vec::new(),
                        });
                    }
                }
            }
        }

        let (scaling_p, scaling_t) = self.self_scaling_stat_buffs_for_card(card);
        if scaling_p != 0 || scaling_t != 0 {
            static_p += scaling_p;
            static_t += scaling_t;
            stat_modifiers.push(StatModifierSourceView {
                source_card_id: None,
                source_card_name: None,
                power_mod: scaling_p,
                toughness_mod: scaling_t,
                label: "Its own ability".to_string(),
                granted_keywords: Vec::new(),
            });
        }

        // Counters modifier
        if card.counters > 0 {
            stat_modifiers.push(StatModifierSourceView {
                source_card_id: None,
                source_card_name: None,
                power_mod: card.counters,
                toughness_mod: card.counters,
                label: "+1/+1 counters".to_string(),
                granted_keywords: Vec::new(),
            });
        }

        // Only units and agents carry combat stats — relics, auras, etc.
        // should never expose a power/toughness, even if the server-side
        // Card struct has default 0/0 values on them.
        let has_combat_stats = matches!(card.kind, CardKind::Unit(_) | CardKind::Agent);
        let power_modifier = self.modifier_amount_for_card(&card.id, StatType::Power)
            + attachment_bonus.0
            + static_p
            + card.counters;
        let power = if has_combat_stats {
            card.power.map(|base| base + power_modifier).or_else(|| {
                if power_modifier != 0 {
                    Some(power_modifier)
                } else {
                    None
                }
            })
        } else {
            None
        };

        let toughness_modifier = self.modifier_amount_for_card(&card.id, StatType::Toughness)
            + attachment_bonus.1
            + static_t
            + card.counters;
        let toughness = if has_combat_stats {
            card.toughness
                .map(|base| base + toughness_modifier)
                .or_else(|| {
                    if toughness_modifier != 0 {
                        Some(toughness_modifier)
                    } else {
                        None
                    }
                })
        } else {
            None
        };
        let armor = self.modifier_amount_for_card(&card.id, StatType::Armor) + attachment_bonus.2;
        let shield = self.modifier_amount_for_card(&card.id, StatType::Shield) + attachment_bonus.3;
        let mut keywords = card.keywords.clone();
        for kw in granted_keywords {
            if !keywords.contains(&kw) {
                keywords.push(kw);
            }
        }
        if self.has_keyword_for_card(&card.id, StatType::Deathtouch) || attachment_bonus.4 {
            if !keywords.contains(&Keyword::Deathtouch) {
                keywords.push(Keyword::Deathtouch);
            }
        }
        if self.has_keyword_for_card(&card.id, StatType::Trample) {
            if !keywords.contains(&Keyword::Trample) {
                keywords.push(Keyword::Trample);
            }
        }
        if self.has_keyword_for_card(&card.id, StatType::Stealth) {
            if !keywords.contains(&Keyword::Stealth) {
                keywords.push(Keyword::Stealth);
            }
        }
        if self.has_keyword_for_card(&card.id, StatType::Camouflage) {
            if !keywords.contains(&Keyword::Camouflage) {
                keywords.push(Keyword::Camouflage);
            }
        }
        if self.has_keyword_for_card(&card.id, StatType::Indestructible) {
            if !keywords.contains(&Keyword::Indestructible) {
                keywords.push(Keyword::Indestructible);
            }
        }
        if self.has_keyword_for_card(&card.id, StatType::Aero) {
            if !keywords.contains(&Keyword::Aero) {
                keywords.push(Keyword::Aero);
            }
        }

        let colors = if card.colors.is_empty() {
            default_card_colors(card)
        } else {
            let mut normalized = Vec::new();
            for value in &card.colors {
                if let Some(mana) = mana_type_for_color_value(value) {
                    normalized.push(mana_color_for_type(&mana).to_string());
                } else if value.starts_with('#') {
                    normalized.push(value.to_string());
                }
            }
            if normalized.is_empty() {
                default_card_colors(card)
            } else {
                normalized
            }
        };

        let can_cast_from_zone = match zone {
            Some(CardZone::Hand) => true,
            Some(CardZone::OperatorZone) => true,
            Some(CardZone::Graveyard) => card
                .gravecast_limit()
                .is_some_and(|limit| card.gravecast_uses < limit),
            _ => false,
        };

        let can_cast = if can_cast_from_zone {
            self.can_cast_card(owner_id, card)
        } else {
            false
        };

        let can_play_land = if zone == Some(CardZone::Hand) {
            let result = self.can_play_infrastructure(owner_id, card);
            if matches!(card.kind, CardKind::Infrastructure) {
                tracing::debug!(
                    "[VIEW] Card '{}' (kind={:?}) in hand: can_play_land={}, stage={:?}, is_active={}",
                    card.name,
                    card.kind,
                    result,
                    self.stage,
                    self.is_active_player(owner_id),
                );
            }
            result
        } else {
            false
        };

        let on_battlefield = zone == Some(CardZone::Battlefield);

        // Cards in hand can be voluntarily discarded during main phase
        let can_discard = zone == Some(CardZone::Hand)
            && self.active_player_id.as_deref() == Some(owner_id)
            && matches!(
                self.stage,
                CardGameStage::Main | CardGameStage::PostCombatMain
            );

        // Build abilities vec and effect_text up front so rule_lines can
        // be composed from them on the backend.
        let view_abilities: Vec<CardAbilityView> = card
            .abilities
            .iter()
            .filter(|ability| !ability.is_spell_effect_ability())
            .map(|ability| {
                let (can_activate, inactive_reason) = if !on_battlefield {
                    (false, Some("Not on battlefield".into()))
                } else {
                    match self.can_activate_ability(owner_id, card, ability) {
                        None => (true, None),
                        Some(reason) => (false, Some(reason)),
                    }
                };
                let valid_target_ids = ability
                    .target_spec
                    .as_ref()
                    .map(|spec| self.valid_targets_for_spec(owner_id, spec))
                    .unwrap_or_default();
                CardAbilityView {
                    id: ability.id.clone(),
                    name: ability.name.clone(),
                    cost: ability.cost.clone(),
                    effect_text: Self::effect_text(&ability.effect, ability.target_spec.as_ref()),
                    target_spec: ability.target_spec.clone(),
                    valid_target_ids,
                    once_per_turn: ability.once_per_turn,
                    tap_targets: ability.tap_targets,
                    can_activate,
                    inactive_reason,
                }
            })
            .collect();
        let view_effect_text: Option<String> = {
            let relevant_event = TriggerEvent::Entered;
            card.triggers
                .iter()
                .find(|t| t.event == relevant_event)
                .map(|t| Self::effect_text(&t.effect, None))
        };
        let rule_lines = card.auto_rule_lines();

        CardView {
            id: card.id.clone(),
            name: card.name.clone(),
            description: card.description.clone(),
            auto_description: card.auto_description.clone(),
            rarity: card.rarity.clone(),
            categories: card.categories.clone(),
            visual_spec: card.visual_spec.clone(),
            cost: card.cost.clone(),
            kind_tag: Self::kind_tag(&card.kind).to_string(),
            kind_label,
            frame_style: Self::frame_style_for_card(card).to_string(),
            supertype: card.supertype.clone(),
            unit_type,
            power,
            toughness,
            base_power: card.power,
            base_toughness: card.toughness,
            stat_modifiers,
            armor,
            shield,
            keywords,
            rule_lines,
            image_url: card.image_url.clone(),
            colors,
            effect_text: view_effect_text,
            target_spec: {
                if matches!(card.kind, CardKind::Unit(_)) {
                    // Unit ETB effects use trigger targeting, not cast-time targeting.
                    // Exception: RememberTargetPlayer still needs cast-time targeting.
                    card.abilities
                        .iter()
                        .find(|a| {
                            a.is_spell_effect_ability()
                                && matches!(a.effect, CardEffect::RememberTargetPlayer)
                        })
                        .and_then(|a| a.target_spec.clone())
                } else {
                    card.abilities
                        .iter()
                        .find(|a| a.is_spell_effect_ability())
                        .and_then(|a| a.target_spec.clone())
                }
            },
            valid_target_ids: {
                // Only worth computing when the card is actually a valid
                // cast target-source right now.
                if can_cast {
                    let spec = card
                        .abilities
                        .iter()
                        .find(|a| a.is_spell_effect_ability())
                        .and_then(|a| a.target_spec.clone());
                    spec.as_ref()
                        .map(|s| self.valid_targets_for_spec(owner_id, s))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            },
            tapped: card.tapped,
            abilities: view_abilities,
            attached_to: card.attached_to.clone(),
            attached_to_name,
            remembered_player_id: card.remembered_player_id.clone(),
            remembered_player_name,
            can_cast,
            can_play_land,
            playable: can_cast || can_play_land,
            can_attack: on_battlefield && self.can_attack_card(owner_id, card),
            can_tap: on_battlefield && self.can_tap_card(owner_id, card),
            can_block: on_battlefield && self.can_block_card(owner_id, card),
            can_discard,
            summoning_sickness: card.summoning_sickness,
            haste: card.haste
                || (on_battlefield
                    && self
                        .effective_keywords_for_card(owner_id, card)
                        .contains(&Keyword::Haste)),
            burnout: card.burnout,
            malware_infected: card.malware_infected,
            loyalty: card.loyalty,
            counters: card.counters,
            damage: card.damage,
            zone,
            created_at: card.created_at.map(|dt| dt.to_rfc3339()),
            owned: card.owned,
            owner_name: if owner_id.is_empty() {
                None
            } else {
                self.players.get(owner_id).map(|p| p.name.clone())
            },
            obtained_at: card.obtained_at.map(|dt| dt.to_rfc3339()),
            value: Some({
                let base = match card.rarity {
                    CardRarity::Public => 0.10,
                    CardRarity::Private => 0.25,
                    CardRarity::Restricted => 1.00,
                    CardRarity::Classified => 5.00,
                    CardRarity::Singularity => 25.00,
                };
                if let Some(vs) = &card.visual_spec {
                    let wear_factor = 1.0 - vs.wear as f64 * 0.5;
                    let holo_factor = if vs.holo { 3.0 } else { 1.0 };
                    (base * wear_factor * holo_factor * 100.0).round() / 100.0
                } else {
                    base
                }
            }),
            ability_source_card_id: None,
            target_ids: Vec::new(),
        }
    }

    /// Lightweight card view for the collection browser — skips gameplay
    /// state (can_activate, modifiers) but still includes keywords, abilities,
    /// and effect_text so the card's rule-text renders correctly.
    pub fn card_browser_view_for(&self, card: &Card) -> CardView {
        let unit_type = match &card.kind {
            CardKind::Unit(kind) => Some(kind.clone()),
            _ => None,
        };
        let mut kind_label = Self::kind_label(&card.kind).to_string();
        if card.supertype == CardSupertype::Apex {
            kind_label = format!("apex {}", kind_label);
        }
        let colors = if card.colors.is_empty() {
            default_card_colors(card)
        } else {
            let mut normalized = Vec::new();
            for value in &card.colors {
                if let Some(mana) = mana_type_for_color_value(value) {
                    normalized.push(mana_color_for_type(&mana).to_string());
                } else if value.starts_with('#') {
                    normalized.push(value.to_string());
                }
            }
            if normalized.is_empty() {
                default_card_colors(card)
            } else {
                normalized
            }
        };
        let keywords = card.keywords.clone();
        let abilities: Vec<CardAbilityView> = card
            .abilities
            .iter()
            .filter(|a| !a.is_spell_effect_ability())
            .map(|ability| CardAbilityView {
                id: ability.id.clone(),
                name: ability.name.clone(),
                cost: ability.cost.clone(),
                effect_text: Self::effect_text(&ability.effect, ability.target_spec.as_ref()),
                target_spec: ability.target_spec.clone(),
                // Browser / non-live views don't have a play context so
                // there's nothing to compute targets against.
                valid_target_ids: Vec::new(),
                once_per_turn: ability.once_per_turn,
                tap_targets: ability.tap_targets,
                can_activate: false,
                inactive_reason: Some("Browser view".into()),
            })
            .collect();
        let effect_text: Option<String> = {
            let relevant_event = TriggerEvent::Entered;
            card.triggers
                .iter()
                .find(|t| t.event == relevant_event)
                .map(|t| Self::effect_text(&t.effect, None))
        };
        let rule_lines = card.auto_rule_lines();
        CardView {
            id: card.id.clone(),
            name: card.name.clone(),
            description: card.description.clone(),
            auto_description: card.auto_description.clone(),
            rarity: card.rarity.clone(),
            categories: Vec::new(),
            visual_spec: card.visual_spec.clone(),
            cost: card.cost.clone(),
            kind_tag: Self::kind_tag(&card.kind).to_string(),
            kind_label,
            frame_style: Self::frame_style_for_card(card).to_string(),
            supertype: card.supertype.clone(),
            unit_type,
            power: card.power,
            toughness: card.toughness,
            base_power: card.power,
            base_toughness: card.toughness,
            stat_modifiers: Vec::new(),
            armor: 0,
            shield: 0,
            keywords,
            rule_lines,
            image_url: card.image_url.clone(),
            colors,
            effect_text,
            target_spec: None,
            valid_target_ids: Vec::new(),
            tapped: false,
            abilities,
            attached_to: None,
            attached_to_name: None,
            remembered_player_id: None,
            remembered_player_name: None,
            can_cast: false,
            can_play_land: false,
            playable: false,
            can_attack: false,
            can_tap: false,
            can_block: false,
            can_discard: false,
            summoning_sickness: false,
            haste: false,
            burnout: false,
            malware_infected: false,
            loyalty: card.loyalty,
            counters: 0,
            damage: 0,
            zone: Some(CardZone::Library),
            created_at: card.created_at.map(|dt| dt.to_rfc3339()),
            owned: card.owned,
            owner_name: None,
            obtained_at: card.obtained_at.map(|dt| dt.to_rfc3339()),
            value: Some({
                let base = match card.rarity {
                    CardRarity::Public => 0.10,
                    CardRarity::Private => 0.25,
                    CardRarity::Restricted => 1.00,
                    CardRarity::Classified => 5.00,
                    CardRarity::Singularity => 25.00,
                };
                if let Some(vs) = &card.visual_spec {
                    let wear_factor = 1.0 - vs.wear as f64 * 0.5;
                    let holo_factor = if vs.holo { 3.0 } else { 1.0 };
                    (base * wear_factor * holo_factor * 100.0).round() / 100.0
                } else {
                    base
                }
            }),
            ability_source_card_id: None,
            target_ids: Vec::new(),
        }
    }

    /// Build the draft pyramid view for a given player.
    pub fn draft_view_for(&self, player_id: &str) -> Option<DraftPyramidView> {
        let draft = self.draft.as_ref()?;

        let pyramid: Vec<Option<CardView>> = draft
            .pyramid
            .iter()
            .map(|slot| slot.as_ref().map(|card| self.card_browser_view_for(card)))
            .collect();

        let available_indices = draft.available_indices();

        let my_drafted = draft.drafted.get(player_id).cloned().unwrap_or_default();
        let my_drafted_cards: Vec<CardView> = my_drafted
            .iter()
            .map(|c| self.card_browser_view_for(c))
            .collect();
        let my_drafted_count = my_drafted_cards.len();

        let mut opponent_drafted_count = 0;
        let mut opponent_rarity_counts: HashMap<String, usize> = HashMap::new();
        for (pid, cards) in &draft.drafted {
            if pid != player_id {
                opponent_drafted_count += cards.len();
                for card in cards {
                    let rarity_key = format!("{:?}", card.rarity).to_lowercase();
                    *opponent_rarity_counts.entry(rarity_key).or_insert(0) += 1;
                }
            }
        }

        // Timer based on the highest row with an available card
        let pick_timer_seconds = if let Some(min_tier) = available_indices
            .iter()
            .map(|&idx| draft.tier_of(idx))
            .min()
        {
            draft.timer_for_row(min_tier)
        } else {
            20
        };

        let current_picker_name = draft
            .current_picker
            .as_ref()
            .and_then(|pid| self.players.get(pid))
            .map(|p| p.name.clone());

        Some(DraftPyramidView {
            pyramid,
            available_indices,
            current_picker_id: draft.current_picker.clone(),
            current_picker_name,
            is_my_turn: draft.current_picker.as_deref() == Some(player_id),
            my_drafted_cards,
            my_drafted_count,
            opponent_drafted_count,
            opponent_rarity_counts,
            packs_completed: draft.packs_completed,
            total_packs: draft.total_packs,
            pick_timer_seconds,
            auto_draft_enabled: draft.auto_draft.contains(player_id),
        })
    }

    pub fn winner_info(&self) -> (Option<String>, Option<String>) {
        let winner_id = self.turn_order.first().cloned();
        let winner_name = winner_id
            .as_ref()
            .and_then(|id| self.draft.as_ref()?.player_names.get(id).cloned())
            .or_else(|| {
                winner_id
                    .as_ref()
                    .and_then(|id| self.players.get(id).map(|player| player.name.clone()))
            });
        (winner_id, winner_name)
    }

    pub fn draft_awards_view_for(&self, player_id: &str) -> Option<DraftAwardsView> {
        if self.stage != CardGameStage::Awards {
            return None;
        }

        let draft = self.draft.as_ref()?;
        let (winner_id, winner_name) = self.winner_info();
        let role = if winner_id.as_deref() == Some(player_id) {
            "winner"
        } else if draft.drafted.contains_key(player_id) {
            "loser"
        } else {
            "spectator"
        };
        let submitted = draft.award_selections.contains_key(player_id);

        if role == "winner" {
            // Winner picks first — show all drafted cards from all players
            let mut all_cards = Vec::new();
            for (pid, drafted_cards) in &draft.drafted {
                let owner_name = draft
                    .player_names
                    .get(pid)
                    .cloned()
                    .unwrap_or_else(|| pid.clone());
                for card in drafted_cards {
                    let mut view = self.card_view_for(pid, card, Some(CardZone::Lobby));
                    view.owner_name = Some(owner_name.clone());
                    all_cards.push(view);
                }
            }

            let total_cards = all_cards.len();
            return Some(DraftAwardsView {
                winner_id,
                winner_name,
                role: role.to_string(),
                selection_mode: "claim".to_string(),
                cards: all_cards,
                required_count: total_cards.min(DRAFT_WINNER_PICK_COUNT),
                submitted,
                waiting_on: Vec::new(),
            });
        }

        if role == "loser" {
            // Loser waits for winner to pick, then gets the rest automatically
            let winner_submitted = winner_id
                .as_deref()
                .map(|wid| draft.award_selections.contains_key(wid))
                .unwrap_or(false);

            let waiting_on = if !winner_submitted {
                winner_name.clone().into_iter().collect()
            } else {
                Vec::new()
            };

            return Some(DraftAwardsView {
                winner_id,
                winner_name: winner_name.clone(),
                role: role.to_string(),
                selection_mode: "none".to_string(),
                cards: Vec::new(),
                required_count: 0,
                submitted: winner_submitted,
                waiting_on,
            });
        }

        Some(DraftAwardsView {
            winner_id,
            winner_name,
            role: role.to_string(),
            selection_mode: "none".to_string(),
            cards: Vec::new(),
            required_count: 0,
            submitted,
            waiting_on: Vec::new(),
        })
    }

    pub fn stack_item_card_view(&self, item: &StackItem) -> CardView {
        let target_ids: Vec<String> = item
            .targets
            .iter()
            .map(|t| match t {
                crate::types::TargetRef::Card(id) => id.clone(),
                crate::types::TargetRef::Player(id) => id.clone(),
            })
            .collect();
        match &item.kind {
            StackItemKind::Spell(card) => {
                let mut view = self.card_view_for(&item.owner_id, card, Some(CardZone::Stack));
                view.target_ids = target_ids;
                view
            }
            StackItemKind::Ability {
                ability,
                source_card_id,
            } => CardView {
                id: ability.id.clone(),
                name: ability.name.clone(),
                description: "Activated ability".to_string(),
                auto_description: String::new(),
                rarity: CardRarity::Public,
                categories: vec![],
                visual_spec: None,
                cost: ability.cost.get_mana(),
                kind_tag: "ability".to_string(),
                kind_label: "ability".to_string(),
                frame_style: "ability".to_string(),
                supertype: CardSupertype::Basic,
                unit_type: None,
                power: None,
                toughness: None,
                base_power: None,
                base_toughness: None,
                stat_modifiers: vec![],
                armor: 0,
                shield: 0,
                keywords: vec![],
                rule_lines: Vec::new(),
                image_url: None,
                colors: vec![],
                effect_text: Some(Self::effect_text(
                    &ability.effect,
                    ability.target_spec.as_ref(),
                )),
                target_spec: ability.target_spec.clone(),
                valid_target_ids: Vec::new(),
                tapped: false,
                abilities: vec![],
                attached_to: None,
                attached_to_name: None,
                remembered_player_id: None,
                remembered_player_name: None,
                can_cast: false,
                can_play_land: false,
                playable: false,
                can_attack: false,
                can_tap: false,
                can_block: false,
                can_discard: false,
                summoning_sickness: false,
                haste: false,
                burnout: false,
                malware_infected: false,
                loyalty: None,
                counters: 0,
                damage: 0,
                zone: Some(CardZone::Stack),
                created_at: None,
                owned: false,
                owner_name: None,
                obtained_at: None,
                value: None,
                ability_source_card_id: Some(source_card_id.clone()),
                target_ids,
            },
        }
    }
}
