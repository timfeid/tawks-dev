use crate::card::*;
use crate::state::*;
use crate::types::*;
use std::collections::HashSet;

impl CardGameState {
    pub(crate) fn ordered_player_ids(&self) -> Vec<String> {
        let mut ordered = self
            .turn_order
            .iter()
            .filter(|player_id| self.players.contains_key(player_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        let mut extras = self
            .players
            .keys()
            .filter(|player_id| !ordered.iter().any(|ordered_id| ordered_id == *player_id))
            .cloned()
            .collect::<Vec<_>>();
        extras.sort();
        ordered.extend(extras);
        ordered
    }

    pub fn resolve_modifier_amount(&self, amount: &ModifierAmount, owner_id: &str) -> i32 {
        match amount {
            ModifierAmount::Fixed(value) => *value,
            ModifierAmount::Dynamic {
                source,
                multiplier,
                offset,
            } => {
                let count = self.resolve_dynamic_source(source, owner_id);
                count * multiplier + offset
            }
        }
    }

    fn resolve_dynamic_source(&self, source: &DynamicSource, owner_id: &str) -> i32 {
        match source {
            DynamicSource::CardsInZone {
                zone,
                scope,
                kind_filter,
                unit_type_filter,
            } => {
                let player_ids = self.resolve_player_scope(scope, owner_id);
                let mut total = 0;
                for pid in &player_ids {
                    if let Some(player) = self.players.get(pid) {
                        let cards = match zone {
                            CardZone::Hand => &player.hand,
                            CardZone::Battlefield => &player.battlefield,
                            CardZone::Graveyard => &player.graveyard,
                            CardZone::Exile => &player.exile,
                            CardZone::OperatorZone => &player.operator_zone,
                            CardZone::Library => &player.deck,
                            _ => continue,
                        };
                        total += cards
                            .iter()
                            .filter(|card| {
                                let kind_ok = kind_filter
                                    .as_ref()
                                    .map(|k| {
                                        std::mem::discriminant(&card.kind)
                                            == std::mem::discriminant(k)
                                    })
                                    .unwrap_or(true);
                                let type_ok = unit_type_filter
                                    .as_ref()
                                    .map(|t| matches!(&card.kind, CardKind::Unit(ct) if ct == t))
                                    .unwrap_or(true);
                                kind_ok && type_ok
                            })
                            .count() as i32;
                    }
                }
                total
            }
            DynamicSource::DistinctCardTypes { zone, scope } => {
                let player_ids = self.resolve_player_scope(scope, owner_id);
                let mut kinds: HashSet<std::mem::Discriminant<CardKind>> = HashSet::new();
                for pid in &player_ids {
                    if let Some(player) = self.players.get(pid) {
                        let cards = match zone {
                            CardZone::Hand => &player.hand,
                            CardZone::Battlefield => &player.battlefield,
                            CardZone::Graveyard => &player.graveyard,
                            CardZone::Exile => &player.exile,
                            CardZone::OperatorZone => &player.operator_zone,
                            CardZone::Library => &player.deck,
                            _ => continue,
                        };
                        for card in cards {
                            kinds.insert(std::mem::discriminant(&card.kind));
                        }
                    }
                }
                kinds.len() as i32
            }
            DynamicSource::ManaTotal { scope } => {
                let player_ids = self.resolve_player_scope(scope, owner_id);
                player_ids
                    .iter()
                    .filter_map(|pid| self.players.get(pid))
                    .map(|p| {
                        let pool = &p.mana_pool;
                        pool.neon
                            + pool.glitch
                            + pool.chrome
                            + pool.slag
                            + pool.bio
                            + pool.colorless
                    })
                    .sum()
            }
            DynamicSource::LifeTotal { scope } => {
                let player_ids = self.resolve_player_scope(scope, owner_id);
                player_ids
                    .iter()
                    .filter_map(|pid| self.players.get(pid))
                    .map(|p| p.life)
                    .sum()
            }
            DynamicSource::DamageDealtThisTurn => self
                .damage_dealt_this_turn
                .get(owner_id)
                .copied()
                .unwrap_or(0),
        }
    }

    fn resolve_player_scope(&self, scope: &PlayerScope, owner_id: &str) -> Vec<String> {
        let ordered_ids = self.ordered_player_ids();
        match scope {
            PlayerScope::Owner => vec![owner_id.to_string()],
            PlayerScope::Opponent => ordered_ids
                .into_iter()
                .filter(|id| id.as_str() != owner_id)
                .collect(),
            PlayerScope::All => ordered_ids,
        }
    }

    pub fn modifier_amount_for_card(&self, card_id: &str, stat: StatType) -> i32 {
        self.effect_manager
            .effects
            .iter()
            .filter(|effect| matches!(&effect.target, EffectTarget::Card(id) if id == card_id))
            .filter(|effect| match stat {
                StatType::Power | StatType::Toughness => {
                    effect.stat == stat || effect.stat == StatType::Both
                }
                _ => effect.stat == stat,
            })
            .map(|effect| self.resolve_modifier_amount(&effect.amount, &effect.owner_id))
            .sum()
    }

    pub fn has_keyword_for_card(&self, card_id: &str, stat: StatType) -> bool {
        self.effect_manager
            .effects
            .iter()
            .filter(|effect| matches!(&effect.target, EffectTarget::Card(id) if id == card_id))
            .any(|effect| {
                effect.stat == stat
                    && self.resolve_modifier_amount(&effect.amount, &effect.owner_id) > 0
            })
    }

    /// Returns the effective keywords for a card, including grants from allied permanents.
    pub fn effective_keywords_for_card(&self, owner_id: &str, card: &Card) -> Vec<Keyword> {
        let mut keywords = card.keywords.clone();
        let unit_type = match &card.kind {
            CardKind::Unit(kind) => Some(kind),
            _ => None,
        };
        for source_owner_id in self.ordered_player_ids() {
            let Some(source_player) = self.players.get(&source_owner_id) else {
                continue;
            };
            for other in &source_player.battlefield {
                if other.id == card.id {
                    continue;
                }
                for kw in &other.keywords {
                    let granted = match kw {
                        Keyword::StaticHasteGrant(t) => Some((Keyword::Haste, t)),
                        Keyword::StaticTrampleGrant(t) => Some((Keyword::Trample, t)),
                        Keyword::StaticStealthGrant(t) => Some((Keyword::Stealth, t)),
                        Keyword::StaticIndestructibleGrant(t) => Some((Keyword::Indestructible, t)),
                        Keyword::StaticKeywordGrant(keyword, t) => Some((*keyword.clone(), t)),
                        _ => None,
                    };
                    if let Some((keyword, target)) = granted {
                        if target.matches(&source_owner_id, owner_id, unit_type)
                            && !keywords.contains(&keyword)
                        {
                            keywords.push(keyword);
                        }
                    }
                }
            }
        }
        keywords
    }

    pub fn is_active_player(&self, player_id: &str) -> bool {
        self.active_player_id
            .as_ref()
            .map(|id| id == player_id)
            .unwrap_or(false)
    }

    pub fn active_player_mut(&mut self) -> Option<&mut PlayerState> {
        let Some(active_id) = self.active_player_id.clone() else {
            return None;
        };
        self.players.get_mut(&active_id)
    }

    pub fn infrastructure_count(player: &PlayerState) -> i32 {
        // Grid nodes + any special infrastructure on battlefield
        player.grid.len() as i32
            + player
                .battlefield
                .iter()
                .filter(|card| card.kind == CardKind::Infrastructure)
                .count() as i32
    }

    pub fn has_target_kind(&self, targets: &[TargetRef], kind: &TargetKind) -> bool {
        match kind {
            TargetKind::Player => targets
                .iter()
                .any(|target| matches!(target, TargetRef::Player(_))),
            TargetKind::Unit => targets
                .iter()
                .any(|target| matches!(target, TargetRef::Card(_))),
            TargetKind::Card => targets
                .iter()
                .any(|target| matches!(target, TargetRef::Card(_))),
            TargetKind::PlayerOrUnit => !targets.is_empty(),
        }
    }

    pub fn has_target_unit_type(&self, targets: &[TargetRef], unit_type: &UnitType) -> bool {
        targets.iter().any(|target| match target {
            TargetRef::Card(card_id) => self
                .find_battlefield_card_owner(card_id)
                .and_then(|(owner_id, _)| {
                    self.players.get(&owner_id).and_then(|player| {
                        player.battlefield.iter().find(|card| &card.id == card_id)
                    })
                })
                .map(|card| matches!(&card.kind, CardKind::Unit(kind) if *kind == *unit_type))
                .unwrap_or(false),
            _ => false,
        })
    }

    pub fn find_card_name(&self, card_id: &str) -> Option<String> {
        if let Some(player) = self.players.get(card_id) {
            return Some(player.name.clone());
        }
        for player_id in self.ordered_player_ids() {
            let Some(player) = self.players.get(&player_id) else {
                continue;
            };
            if let Some(card) = player.battlefield.iter().find(|card| card.id == card_id) {
                return Some(card.name.clone());
            }
            if let Some(card) = player.graveyard.iter().find(|card| card.id == card_id) {
                return Some(card.name.clone());
            }
            if let Some(card) = player.hand.iter().find(|card| card.id == card_id) {
                return Some(card.name.clone());
            }
        }
        None
    }

    pub fn find_card_with_owner_and_zone(&self, card_id: &str) -> Option<(String, Card, CardZone)> {
        for player_id in self.ordered_player_ids() {
            let Some(player) = self.players.get(&player_id) else {
                continue;
            };
            if let Some(card) = player.battlefield.iter().find(|card| card.id == card_id) {
                return Some((player_id.clone(), card.clone(), CardZone::Battlefield));
            }
            if let Some(card) = player.graveyard.iter().find(|card| card.id == card_id) {
                return Some((player_id.clone(), card.clone(), CardZone::Graveyard));
            }
            if let Some(card) = player.hand.iter().find(|card| card.id == card_id) {
                return Some((player_id.clone(), card.clone(), CardZone::Hand));
            }
            if let Some(card) = player.operator_zone.iter().find(|card| card.id == card_id) {
                return Some((player_id.clone(), card.clone(), CardZone::OperatorZone));
            }
            if let Some(card) = player.exile.iter().find(|card| card.id == card_id) {
                return Some((player_id.clone(), card.clone(), CardZone::Exile));
            }
            if let Some(card) = player.deck.iter().find(|card| card.id == card_id) {
                return Some((player_id.clone(), card.clone(), CardZone::Library));
            }
        }
        None
    }

    pub fn can_target_card(&self, source_player_id: &str, target_card_id: &str) -> bool {
        if let Some((owner_id, _)) = self.find_battlefield_card_owner(target_card_id) {
            if let Some(player) = self.players.get(&owner_id) {
                if let Some(card) = player.battlefield.iter().find(|c| c.id == target_card_id) {
                    // Camouflage: Cannot be targeted by opponent spells/abilities
                    if card.keywords.contains(&Keyword::Camouflage) && source_player_id != owner_id
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn attachment_modifiers_for_card(&self, card_id: &str) -> (i32, i32, i32, i32, bool) {
        let mut power = 0;
        let mut toughness = 0;
        let mut armor = 0;
        let mut shield = 0;
        let mut deathtouch = false;
        for player_id in self.ordered_player_ids() {
            let Some(player) = self.players.get(&player_id) else {
                continue;
            };
            for card in &player.battlefield {
                if card.attached_to.as_deref() != Some(card_id) {
                    continue;
                }
                let Some(attachment) = &card.attachment else {
                    continue;
                };
                for modifier in &attachment.modifiers {
                    let amt = match &modifier.amount {
                        ModifierAmount::Fixed(v) => *v,
                        _ => 0, // Attachment modifiers are always fixed
                    };
                    match modifier.stat {
                        StatType::Power => power += amt,
                        StatType::Toughness => toughness += amt,
                        StatType::Both => {
                            power += amt;
                            toughness += amt;
                        }
                        StatType::Armor => armor += amt,
                        StatType::Shield => shield += amt,
                        StatType::Deathtouch => {
                            if amt > 0 {
                                deathtouch = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        (power, toughness, armor, shield, deathtouch)
    }

    /// Static lord-style power/toughness buffs from other permanents on the
    /// same controller's battlefield. E.g. Lord of the Grid's
    /// `StaticPowerBuff(1, Some(Drone))` adds +1/+0 to every Drone its
    /// controller owns.
    /// Self-scaling buffs: `SelfPowerPerKind(n, kind)` /
    /// `SelfToughnessPerKind(n, kind)` on this card grant +n power /
    /// toughness for each card of the given kind the owner has on the
    /// battlefield.
    pub fn self_scaling_stat_buffs_for_card(&self, card: &Card) -> (i32, i32) {
        let Some((owner_id, _)) = self.find_battlefield_card_owner(&card.id) else {
            return (0, 0);
        };
        let Some(player) = self.players.get(&owner_id) else {
            return (0, 0);
        };
        let mut power = 0i32;
        let mut toughness = 0i32;
        for kw in &card.keywords {
            match kw {
                Keyword::SelfPowerPerKind(amt, kind) => {
                    let count = player
                        .battlefield
                        .iter()
                        .filter(|c| c.kind == *kind)
                        .count() as i32;
                    power += amt * count;
                }
                Keyword::SelfToughnessPerKind(amt, kind) => {
                    let count = player
                        .battlefield
                        .iter()
                        .filter(|c| c.kind == *kind)
                        .count() as i32;
                    toughness += amt * count;
                }
                _ => {}
            }
        }
        (power, toughness)
    }

    pub fn static_stat_buffs_for_card(&self, card: &Card) -> (i32, i32) {
        let Some((owner_id, _)) = self.find_battlefield_card_owner(&card.id) else {
            return (0, 0);
        };
        let unit_type = match &card.kind {
            CardKind::Unit(kind) => Some(kind.clone()),
            _ => None,
        };
        let mut power = 0i32;
        let mut toughness = 0i32;
        let Some(player) = self.players.get(&owner_id) else {
            return (0, 0);
        };
        for other in &player.battlefield {
            if other.id == card.id {
                continue;
            }
            for kw in &other.keywords {
                match kw {
                    Keyword::StaticPowerBuff(amt, target_type) => {
                        let matches_type = target_type
                            .as_ref()
                            .map(|t| unit_type.as_ref() == Some(t))
                            .unwrap_or(true);
                        if matches_type {
                            power += amt;
                        }
                    }
                    Keyword::StaticToughnessBuff(amt, target_type) => {
                        let matches_type = target_type
                            .as_ref()
                            .map(|t| unit_type.as_ref() == Some(t))
                            .unwrap_or(true);
                        if matches_type {
                            toughness += amt;
                        }
                    }
                    _ => {}
                }
            }
        }
        (power, toughness)
    }

    pub fn card_combat_stats(&self, card: &Card) -> (i32, i32, i32, i32, bool, bool, bool) {
        let attachment_bonus = self.attachment_modifiers_for_card(&card.id);
        let static_buffs = self.static_stat_buffs_for_card(card);
        let self_scaling = self.self_scaling_stat_buffs_for_card(card);
        let power = card.power.unwrap_or(0)
            + self.modifier_amount_for_card(&card.id, StatType::Power)
            + attachment_bonus.0
            + static_buffs.0
            + self_scaling.0
            + card.counters;
        let toughness = card.toughness.unwrap_or(0)
            + self.modifier_amount_for_card(&card.id, StatType::Toughness)
            + attachment_bonus.1
            + static_buffs.1
            + self_scaling.1
            + card.counters;
        let armor = self.modifier_amount_for_card(&card.id, StatType::Armor) + attachment_bonus.2;
        let shield = self.modifier_amount_for_card(&card.id, StatType::Shield) + attachment_bonus.3;
        let has_deathtouch = card.keywords.contains(&Keyword::Deathtouch)
            || self.has_keyword_for_card(&card.id, StatType::Deathtouch)
            || attachment_bonus.4;
        let is_indestructible = card.keywords.contains(&Keyword::Indestructible)
            || self.has_keyword_for_card(&card.id, StatType::Indestructible);
        let has_trample = card.keywords.contains(&Keyword::Trample)
            || self.has_keyword_for_card(&card.id, StatType::Trample);
        (
            power,
            toughness,
            armor,
            shield,
            has_deathtouch,
            is_indestructible,
            has_trample,
        )
    }

    pub fn find_battlefield_card_owner(&self, card_id: &str) -> Option<(String, CardKind)> {
        for player_id in self.ordered_player_ids() {
            let Some(player) = self.players.get(&player_id) else {
                continue;
            };
            if let Some(card) = player.battlefield.iter().find(|card| card.id == card_id) {
                return Some((player_id, card.kind.clone()));
            }
        }
        None
    }
}
