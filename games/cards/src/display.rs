use crate::card::*;
use crate::state::*;
use crate::types::*;

/// Lowercase the first character of a string (for joining effect clauses).
fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Display and formatting functions for card game UI
impl CardGameState {
    fn predicate_contains_condition(predicate: &Predicate, condition: &Condition) -> bool {
        match predicate {
            Predicate::Condition(value) => value == condition,
            Predicate::All(predicates) | Predicate::Any(predicates) => predicates
                .iter()
                .any(|predicate| Self::predicate_contains_condition(predicate, condition)),
            Predicate::CardKind(_) | Predicate::IsUnit | Predicate::Not(_) => false,
        }
    }

    fn trigger_predicate_text(event: &TriggerEvent, predicate: &Predicate) -> Option<String> {
        match event {
            TriggerEvent::Discarded => {
                let matches_remembered_player = Self::predicate_contains_condition(
                    predicate,
                    &Condition::TargetMatchesRememberedPlayer,
                );
                let is_opponent = Self::predicate_contains_condition(
                    predicate,
                    &Condition::TargetIsOpponentPlayer,
                );
                let is_last_card = Self::predicate_contains_condition(
                    predicate,
                    &Condition::TargetPlayerHandSizeExactly(0),
                );

                if matches_remembered_player {
                    return Some(if is_last_card {
                        "When chosen player discards their last card".to_string()
                    } else {
                        "When chosen player discards".to_string()
                    });
                }

                match (is_opponent, is_last_card) {
                    (true, true) => Some("When an opponent discards their last card".to_string()),
                    (true, false) => Some("When an opponent discards".to_string()),
                    (false, true) => Some("When a player discards their last card".to_string()),
                    (false, false) => None,
                }
            }
            _ => None,
        }
    }

    pub fn reveal_outcome_text(outcome: &RevealOutcome) -> &'static str {
        match outcome {
            RevealOutcome::Deploy => "deploy it",
            RevealOutcome::ToHand => "put it into your hand",
            RevealOutcome::ToTop => "leave it on top",
            RevealOutcome::ToBottom => "put it on the bottom",
            RevealOutcome::ToGraveyard => "recycle it",
        }
    }

    pub fn predicate_text(predicate: &Predicate) -> String {
        match predicate {
            Predicate::CardKind(kind) => match kind {
                CardKind::Unit(UnitType::BioOrganism) => "Bio-Organism unit".to_string(),
                CardKind::Unit(UnitType::Human) => "Human unit".to_string(),
                CardKind::Unit(UnitType::Megafauna) => "Megafauna unit".to_string(),
                CardKind::Unit(UnitType::Anomaly) => "Anomaly unit".to_string(),
                CardKind::Unit(UnitType::Drone) => "Drone unit".to_string(),
                CardKind::Unit(UnitType::Virus) => "Virus unit".to_string(),
                CardKind::Unit(UnitType::Hacker) => "Hacker unit".to_string(),
                CardKind::Unit(UnitType::Node) => "Node unit".to_string(),
                CardKind::Unit(UnitType::None) => "unit".to_string(),
                CardKind::Infrastructure => "infrastructure".to_string(),
                CardKind::Relic => "relic".to_string(),
                CardKind::Instant => "instant".to_string(),
                CardKind::Ritual => "ritual".to_string(),
                CardKind::Aura => "aura".to_string(),
                CardKind::Agent => "agent".to_string(),
            },
            Predicate::All(predicates) => {
                let mut parts = Vec::new();
                for predicate in predicates {
                    let text = Self::predicate_text(predicate);
                    if !parts.contains(&text) {
                        parts.push(text);
                    }
                }
                parts.join(" and ")
            }
            Predicate::Any(predicates) => {
                let mut parts = Vec::new();
                for predicate in predicates {
                    let text = Self::predicate_text(predicate);
                    if !parts.contains(&text) {
                        parts.push(text);
                    }
                }
                parts.join(" or ")
            }
            Predicate::IsUnit => "unit".to_string(),
            Predicate::Not(pred) => format!("non-{}", Self::predicate_text(pred)),
            Predicate::Condition(Condition::TargetMatchesRememberedPlayer) => {
                "chosen player".to_string()
            }
            Predicate::Condition(_) => "matching card".to_string(),
        }
    }

    pub fn target_kind_text(spec: Option<&TargetSpec>) -> String {
        let Some(spec) = spec else {
            return "target".to_string();
        };

        if let Some(predicate) = &spec.predicate {
            match predicate {
                Predicate::CardKind(CardKind::Infrastructure) => {
                    return "target infrastructure".to_string();
                }
                Predicate::CardKind(CardKind::Unit(_)) => return "target unit".to_string(),
                _ => {}
            }
        }

        match spec.kind {
            TargetKind::Unit => "target unit".to_string(),
            TargetKind::Card => "target card".to_string(),
            TargetKind::Player => "target player".to_string(),
            TargetKind::PlayerOrUnit => "target player or unit".to_string(),
        }
    }

    pub fn effect_text(effect: &CardEffect, spec: Option<&TargetSpec>) -> String {
        match effect {
            CardEffect::RememberTargetPlayer => {
                format!("Choose {}.", Self::target_kind_text(spec))
            }
            CardEffect::DamagePlayer(amount) => format!("Deal {} damage to target player.", amount),
            CardEffect::DamageUnit(amount) => {
                format!("Deal {} damage to target unit.", amount)
            }
            CardEffect::DamageTarget(amount) => format!(
                "Deal {} damage to {}.",
                amount,
                Self::target_kind_text(spec)
            ),
            CardEffect::DrawCards(amount) => {
                if *amount == 1 {
                    "Draw 1 card.".to_string()
                } else {
                    format!("Draw {} cards.", amount)
                }
            }
            CardEffect::GainLife(amount) => format!("Gain {} life.", amount),
            CardEffect::Heat(amount) if *amount < 0 => format!("Cool {} heat.", -amount),
            CardEffect::Heat(amount) => format!("Take {} heat.", amount),
            CardEffect::HeatOpponents(amount) => format!("Each opponent takes {} heat.", amount),
            CardEffect::CashOpponents(amount) if *amount < 0 => {
                format!("Each opponent loses ${}.", -amount)
            }
            CardEffect::CashOpponents(amount) => format!("Each opponent gains ${}.", amount),
            CardEffect::CounterTop => "Counter target spell.".to_string(),
            CardEffect::AddMana(mana_type, amount) => {
                let type_str = match mana_type {
                    ManaType::Neon => "Neon",
                    ManaType::Glitch => "Glitch",
                    ManaType::Chrome => "Chrome",
                    ManaType::Slag => "Slag",
                    ManaType::Bio => "Bio",
                    ManaType::Colorless => "Colorless",
                };
                format!("Add {} {}.", amount, type_str)
            }
            CardEffect::CreateToken(token) => format!(
                "Create a {}/{} {} token.",
                token.power, token.toughness, token.name
            ),
            CardEffect::CreateInfrastructure(spec) => {
                let count = spec.count.max(1);
                let tapped = if spec.enters_tapped { " tapped" } else { "" };
                let mana_name = match spec.mana_type {
                    ManaType::Neon => "Neon",
                    ManaType::Glitch => "Glitch",
                    ManaType::Chrome => "Chrome",
                    ManaType::Slag => "Slag",
                    ManaType::Bio => "Bio",
                    ManaType::Colorless => "Colorless",
                };
                if count == 1 {
                    format!(
                        "Create a{} {} infrastructure that taps for {} {}.",
                        tapped, spec.name, spec.amount, mana_name
                    )
                } else {
                    format!(
                        "Create {}{} {} infrastructures that tap for {} {}.",
                        count, tapped, spec.name, spec.amount, mana_name
                    )
                }
            }
            CardEffect::ApplyStatModifier(spec) => {
                let amount_text = Self::modifier_amount_text(&spec.amount);
                format!(
                    "Give {} {:?} {} until {:?}.",
                    Self::target_kind_text(None),
                    spec.stat,
                    amount_text,
                    spec.expires
                )
            }
            CardEffect::ApplyStatModifiers(specs) => {
                // Check if any spec is dynamic
                let has_dynamic = specs
                    .iter()
                    .any(|s| matches!(s.amount, ModifierAmount::Dynamic { .. }));
                if has_dynamic {
                    // For dynamic modifiers, describe the source
                    if let Some(first) = specs.first() {
                        let amount_text = Self::modifier_amount_text(&first.amount);
                        let target = match &first.target {
                            EffectTargetSpec::AllUnits(_) => "Units",
                            EffectTargetSpec::Source => "Self",
                            EffectTargetSpec::Targets => "Target",
                            _ => "Units",
                        };
                        format!("{} get {}", target, amount_text)
                    } else {
                        "No effect.".to_string()
                    }
                } else {
                    // Try to combine power/toughness into a single "+X/+Y" format
                    let mut power_mod = 0i32;
                    let mut toughness_mod = 0i32;
                    for s in specs {
                        let amt = match &s.amount {
                            ModifierAmount::Fixed(v) => *v,
                            _ => 0,
                        };
                        match s.stat {
                            StatType::Power => power_mod += amt,
                            StatType::Toughness => toughness_mod += amt,
                            StatType::Both => {
                                power_mod += amt;
                                toughness_mod += amt;
                            }
                            _ => {}
                        }
                    }
                    let sign_p = if power_mod >= 0 { "+" } else { "" };
                    let sign_t = if toughness_mod >= 0 { "+" } else { "" };
                    let target = match specs.first().map(|s| &s.target) {
                        Some(EffectTargetSpec::AllUnits(_)) => "Units",
                        Some(EffectTargetSpec::Source) => "Self",
                        Some(EffectTargetSpec::Targets) => "Target",
                        _ => "Units",
                    };
                    format!(
                        "{} get {}{}/{}{}",
                        target, sign_p, power_mod, sign_t, toughness_mod
                    )
                }
            }
            CardEffect::Sequence(effects) => {
                // Group consecutive identical effects for cleaner text
                let mut groups: Vec<(&CardEffect, usize)> = Vec::new();
                for effect in effects {
                    if let Some(last) = groups.last_mut() {
                        if last.0 == effect {
                            last.1 += 1;
                            continue;
                        }
                    }
                    groups.push((effect, 1));
                }
                let parts: Vec<String> = groups
                    .iter()
                    .map(|(effect, count)| {
                        let text = if *count > 1 {
                            Self::effect_text_with_count(effect, spec, *count)
                        } else {
                            Self::effect_text(effect, spec)
                        };
                        text.trim_end_matches('.').to_string()
                    })
                    .collect();
                let joined = match parts.len() {
                    0 => String::new(),
                    1 => parts.into_iter().next().unwrap_or_default(),
                    2 => {
                        let mut iter = parts.into_iter();
                        let first = iter.next().unwrap_or_default();
                        let second = iter.next().unwrap_or_default();
                        format!("{} and {}", first, lowercase_first(&second))
                    }
                    _ => {
                        let last = parts.last().cloned().unwrap_or_default();
                        let head = &parts[..parts.len() - 1];
                        let head_text: Vec<String> = head
                            .iter()
                            .enumerate()
                            .map(|(i, s)| {
                                if i == 0 {
                                    s.clone()
                                } else {
                                    lowercase_first(s)
                                }
                            })
                            .collect();
                        format!("{}, and {}", head_text.join(", "), lowercase_first(&last))
                    }
                };
                if joined.is_empty() {
                    joined
                } else {
                    format!("{}.", joined)
                }
            }
            CardEffect::RevealTopIf {
                predicate,
                on_match,
                on_fail,
            } => format!(
                "Reveal the top card of your stack. If {}, {}. Otherwise, {}.",
                Self::predicate_text(predicate),
                Self::reveal_outcome_text(on_match),
                Self::reveal_outcome_text(on_fail)
            ),
            CardEffect::Freeze(expires) => format!(
                "Freeze {} until {:?}.",
                Self::target_kind_text(spec),
                expires
            ),
            CardEffect::RefundMana(percentage) => {
                format!("Refund {}% of mana cost.", *percentage as f32 / 100.0)
            }
            CardEffect::Loot(draw, discard) => {
                format!("Draw {}, then discard {}.", draw, discard)
            }
            CardEffect::Wheel(amount) => format!(
                "Each player discards their hand, then draws {} cards.",
                amount
            ),
            CardEffect::Scry(amount) => format!("Scry {}.", amount),
            CardEffect::Search(predicate) => {
                let pred_text = Self::predicate_text(predicate);
                format!("Search your library for a {}.", pred_text)
            }
            CardEffect::SearchToBattlefield(predicate) => {
                let pred_text = Self::predicate_text(predicate);
                format!(
                    "Search your library for a {} and put it onto the battlefield.",
                    pred_text
                )
            }
            CardEffect::Bounce => format!(
                "Return {} to its owner's hand.",
                Self::target_kind_text(spec)
            ),
            CardEffect::BoardWipe => "Destroy all units.".to_string(),
            CardEffect::DestroyTargetPlayerUnits => "Destroy all target player's units.".to_string(),
            CardEffect::DestroyUnitsWithToughnessAtLeast(min_toughness) => {
                format!(
                    "Destroy all units with toughness {} or greater.",
                    min_toughness
                )
            }
            CardEffect::CopyLastCast => "Copy the last spell or ability cast.".to_string(),
            CardEffect::Purify => "Remove all status effects.".to_string(),
            CardEffect::GrantKeyword(keyword, expires) => {
                let keyword_text = Self::keyword_text(keyword);
                if spec.is_none() {
                    return match expires {
                        ExpireContract::EndOfTurn => {
                            format!("Units you control gain {} until end of turn.", keyword_text)
                        }
                        ExpireContract::Never => {
                            format!("Units you control gain {} permanently.", keyword_text)
                        }
                        ExpireContract::Turns(n) => {
                            format!("Units you control gain {} for {} turns.", keyword_text, n)
                        }
                    };
                }

                let target = Self::target_kind_text(spec);
                match expires {
                    ExpireContract::EndOfTurn => {
                        format!("{} gets {} until end of turn.", target, keyword_text)
                    }
                    ExpireContract::Never => {
                        format!("{} gets {}.", target, keyword_text)
                    }
                    ExpireContract::Turns(n) => {
                        format!("{} gets {} for {} turns.", target, keyword_text, n)
                    }
                }
            }
            CardEffect::Discard(amount) => {
                if *amount == 1 {
                    "Opponent discards 1 card.".to_string()
                } else {
                    format!("Opponent discards {} cards.", amount)
                }
            }
            CardEffect::DiscardSelfRandom(amount) => {
                if *amount == 1 {
                    "Discard a card at random.".to_string()
                } else {
                    format!("Discard {} cards at random.", amount)
                }
            }
            CardEffect::StealMana(mana_type, amount) => {
                let mana_name = match mana_type {
                    ManaType::Neon => "Neon",
                    ManaType::Glitch => "Glitch",
                    ManaType::Chrome => "Chrome",
                    ManaType::Slag => "Slag",
                    ManaType::Bio => "Bio",
                    ManaType::Colorless => "Colorless",
                };
                format!("Steal {} {} mana from opponent.", amount, mana_name)
            }
            CardEffect::ShuffleLibrary(scope) => format!("Shuffle {:?} library.", scope),
            CardEffect::ForbidName => format!(
                "{}'s name cannot be cast next turn.",
                Self::target_kind_text(spec)
            ),
            CardEffect::Fight => "Target unit fights another target unit.".to_string(),
            CardEffect::TapTarget => format!("Tap {}.", Self::target_kind_text(spec)),
            CardEffect::UntapTarget => format!("Untap {}.", Self::target_kind_text(spec)),
            CardEffect::DestroyTarget => format!("Destroy {}.", Self::target_kind_text(spec)),
            CardEffect::ExileTarget => format!("Exile {}.", Self::target_kind_text(spec)),
            CardEffect::Equip => "Attach to target unit you control.".to_string(),
            CardEffect::AddManaScaling(mana_type, _, _) => {
                let mana_name = match mana_type {
                    ManaType::Neon => "Neon",
                    ManaType::Glitch => "Glitch",
                    ManaType::Chrome => "Chrome",
                    ManaType::Slag => "Slag",
                    ManaType::Bio => "Bio",
                    ManaType::Colorless => "Colorless",
                };
                format!("Add 1 {} for each matching unit you control.", mana_name)
            }
            CardEffect::ModifyLoyalty(amount) => format!("Add {} loyalty.", amount),
            CardEffect::Emblem(_, _) => "You get an emblem with persistent bonuses.".to_string(),
            CardEffect::SacrificeThis => "Sacrifice this permanent.".to_string(),
            CardEffect::ReanimateNamedCard(name) => {
                format!("Return {} from graveyard to battlefield.", name)
            }
            CardEffect::CopyUnit => "Copy target unit.".to_string(),
            CardEffect::Mill(amount) => {
                if *amount == 1 {
                    "Target player mills 1 card.".to_string()
                } else {
                    format!("Target player mills {} cards.", amount)
                }
            }
            CardEffect::BounceAllUnits => "Return all units to their owners' hands.".to_string(),
            CardEffect::BounceAllOpponentPermanents => {
                "Return all opponent permanents to their owners' hands.".to_string()
            }
            CardEffect::BounceAllAttacking => {
                "Return all attacking units to their owners' hands.".to_string()
            }
            CardEffect::SearchGraveyard(predicate) => {
                let pred_text = Self::predicate_text(predicate);
                format!("Search graveyard for a {}.", pred_text)
            }
            CardEffect::ReanimateTargetUnit => {
                "Return target unit from graveyard to battlefield.".to_string()
            }
            CardEffect::AddCounters(amount) => {
                if *amount == 1 {
                    "Add a +1/+1 counter.".to_string()
                } else {
                    format!("Add {} +1/+1 counters.", amount)
                }
            }
            CardEffect::RemoveCounters(amount) => {
                if *amount == 1 {
                    "Remove a +1/+1 counter.".to_string()
                } else {
                    format!("Remove {} +1/+1 counters.", amount)
                }
            }
            CardEffect::DamagePlayerPerHandCard => {
                "Deal damage to target player equal to the number of cards in their hand."
                    .to_string()
            }
            CardEffect::DamagePlayerPerBattlefieldKind(kind) => {
                format!(
                    "Deal damage to each opponent equal to the number of {} you control.",
                    Self::kind_label(kind)
                )
            }
            CardEffect::CreateTokensPerBattlefieldKind(token, kind) => {
                format!(
                    "Create a {}/{} {} token for each {} you control.",
                    token.power,
                    token.toughness,
                    token.name,
                    Self::kind_label(kind),
                )
            }
            CardEffect::PumpByMana => {
                "Your units get +X/+X until end of turn, where X is your total mana.".to_string()
            }

            // --- New effects ---
            CardEffect::Siphon(amount) => format!("Drain {} life from target player. You gain that much life.", amount),
            CardEffect::TransmuteMana(from, to, amount) => {
                let from_name = Self::mana_type_name(from);
                let to_name = Self::mana_type_name(to);
                format!("Convert {} {} mana to {}.", amount, from_name, to_name)
            }
            CardEffect::Swap => "Exchange control of two target units.".to_string(),
            CardEffect::Phase => format!("Phase out {}. It returns at your next upkeep.", Self::target_kind_text(spec)),
            CardEffect::MutateTarget(_) => format!("Permanently alter {}'s base stats.", Self::target_kind_text(spec)),
            CardEffect::Polymorph => "Destroy target unit. Its controller reveals cards until they hit a unit, then puts it onto the battlefield.".to_string(),
            CardEffect::Surveil(amount) => format!("Surveil {}. (Look at the top {} cards. Put any into your graveyard, rest on top.)", amount, amount),
            CardEffect::Transmit(amount) => format!("Look at the top {} cards. Put one into your hand, rest on bottom.", amount),
            CardEffect::Entomb(predicate) => {
                let pred_text = Self::predicate_text(predicate);
                format!("Search your library for a {} and put it into your graveyard.", pred_text)
            }
            CardEffect::Recycle => "Shuffle your graveyard into your library.".to_string(),
            CardEffect::DamageSplit(amount) => format!("Deal {} damage divided among any number of targets.", amount),
            CardEffect::DamageEachOpponentUnit(amount) => format!("Deal {} damage to each opponent unit.", amount),
            CardEffect::Reflect(amount) => format!("Deal {} damage to each opponent.", amount),
            CardEffect::Conscript => "Put target unit from an opponent's graveyard onto the battlefield under your control.".to_string(),
            CardEffect::Scavenge(amount) => format!("Exile a unit from your graveyard. Target unit gets {} +1/+1 counters.", amount),
            CardEffect::Assimilate(unit_type) => format!(
                "Take control of target opponent unit and make it a {}.",
                Self::unit_type_name(unit_type, false)
            ),
        }
    }

    fn mana_type_name(mana_type: &ManaType) -> &'static str {
        match mana_type {
            ManaType::Neon => "Neon",
            ManaType::Glitch => "Glitch",
            ManaType::Chrome => "Chrome",
            ManaType::Slag => "Slag",
            ManaType::Bio => "Bio",
            ManaType::Colorless => "Colorless",
        }
    }

    fn effect_text_with_count(
        effect: &CardEffect,
        spec: Option<&TargetSpec>,
        count: usize,
    ) -> String {
        match effect {
            CardEffect::CreateToken(token) => format!(
                "Create {} {}/{} {} tokens.",
                count, token.power, token.toughness, token.name
            ),
            CardEffect::DrawCards(n) => format!("Draw {} cards.", n * count),
            _ => {
                // Fall back to repeating the text
                let text = Self::effect_text(effect, spec);
                std::iter::repeat(text)
                    .take(count)
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }

    pub fn modifier_amount_text(amount: &ModifierAmount) -> String {
        match amount {
            ModifierAmount::Fixed(value) => {
                if *value >= 0 {
                    format!("+{}", value)
                } else {
                    format!("{}", value)
                }
            }
            ModifierAmount::Dynamic {
                source,
                multiplier,
                offset,
            } => {
                let source_text = match source {
                    DynamicSource::CardsInZone {
                        zone,
                        scope,
                        kind_filter,
                        unit_type_filter,
                    } => {
                        let zone_text = match zone {
                            CardZone::Graveyard => "graveyard",
                            CardZone::Battlefield => "battlefield",
                            CardZone::Hand => "hand",
                            CardZone::Exile => "exile",
                            _ => "zone",
                        };
                        let scope_text = match scope {
                            PlayerScope::Owner => "your",
                            PlayerScope::Opponent => "opponent's",
                            PlayerScope::All => "all",
                        };
                        let filter_text = unit_type_filter
                            .as_ref()
                            .map(|t| Self::unit_type_name(t, false))
                            .or_else(|| {
                                kind_filter.as_ref().map(|k| match k {
                                    CardKind::Infrastructure => "infrastructure".to_string(),
                                    CardKind::Unit(unit_type) => {
                                        Self::unit_type_name(unit_type, false)
                                    }
                                    CardKind::Relic => "relic".to_string(),
                                    CardKind::Instant => "instant".to_string(),
                                    CardKind::Ritual => "ritual".to_string(),
                                    CardKind::Aura => "aura".to_string(),
                                    CardKind::Agent => "agent".to_string(),
                                })
                            })
                            .unwrap_or_else(|| "card".to_string());
                        format!("{}s in {} {}", filter_text, scope_text, zone_text)
                    }
                    DynamicSource::DistinctCardTypes { zone, scope } => {
                        let zone_text = match zone {
                            CardZone::Graveyard => "graveyard",
                            CardZone::Battlefield => "battlefield",
                            _ => "zone",
                        };
                        let scope_text = match scope {
                            PlayerScope::Owner => "your",
                            PlayerScope::Opponent => "opponent's",
                            PlayerScope::All => "all",
                        };
                        format!("card types in {} {}", scope_text, zone_text)
                    }
                    DynamicSource::ManaTotal { scope } => match scope {
                        PlayerScope::Owner => "your total mana".to_string(),
                        _ => "total mana".to_string(),
                    },
                    DynamicSource::LifeTotal { scope } => match scope {
                        PlayerScope::Owner => "your life total".to_string(),
                        _ => "life total".to_string(),
                    },
                    DynamicSource::DamageDealtThisTurn => "damage dealt this turn".to_string(),
                };
                if *multiplier == 1 && *offset == 0 {
                    format!("+X/+X where X is the number of {}", source_text)
                } else if *multiplier != 1 && *offset == 0 {
                    format!("+{} for each {}", multiplier, source_text)
                } else {
                    format!("+{} for each {} +{}", multiplier, source_text, offset)
                }
            }
        }
    }

    pub fn unit_type_name(unit_type: &UnitType, plural: bool) -> String {
        match (unit_type, plural) {
            (UnitType::BioOrganism, false) => "Bio-Organism".to_string(),
            (UnitType::BioOrganism, true) => "Bio-Organisms".to_string(),
            (UnitType::Human, false) => "Human".to_string(),
            (UnitType::Human, true) => "Humans".to_string(),
            (UnitType::Megafauna, false) => "Megafauna".to_string(),
            (UnitType::Megafauna, true) => "Megafauna".to_string(),
            (UnitType::Anomaly, false) => "Anomaly".to_string(),
            (UnitType::Anomaly, true) => "Anomalies".to_string(),
            (UnitType::Drone, false) => "Drone".to_string(),
            (UnitType::Drone, true) => "Drones".to_string(),
            (UnitType::Virus, false) => "Virus".to_string(),
            (UnitType::Virus, true) => "Viruses".to_string(),
            (UnitType::Hacker, false) => "Hacker".to_string(),
            (UnitType::Hacker, true) => "Hackers".to_string(),
            (UnitType::Node, false) => "Node".to_string(),
            (UnitType::Node, true) => "Nodes".to_string(),
            (UnitType::None, false) => "unit".to_string(),
            (UnitType::None, true) => "units".to_string(),
        }
    }

    /// Renders a `GrantTarget` as the subject phrase of a static keyword line
    /// (e.g. "Drones you control", "Opponents' units", "All units").
    pub fn grant_target_subject(target: &crate::types::GrantTarget) -> String {
        use crate::types::{GrantFilter, GrantSide};
        let noun = match &target.filter {
            Some(GrantFilter::UnitType(t)) => Self::unit_type_name(t, true),
            None => "Units".to_string(),
        };
        match target.side {
            GrantSide::Mine => format!("{} you control", noun),
            GrantSide::Opponents => format!("Opponents' {}", noun.to_lowercase()),
            GrantSide::All => format!("All {}", noun.to_lowercase()),
        }
    }

    pub fn keyword_name(keyword: &Keyword) -> &'static str {
        match keyword {
            Keyword::Haste => "Haste",
            Keyword::Deathtouch => "Deathtouch",
            Keyword::Piercing => "Piercing",
            Keyword::Stealth => "Stealth",
            Keyword::Aero => "Aero",
            Keyword::Trample => "Trample",
            Keyword::Indestructible => "Indestructible",
            Keyword::Interceptor => "Interceptor",
            Keyword::Vigilance => "Vigilance",
            Keyword::Flash => "Flash",
            Keyword::Camouflage => "Camouflage",
            Keyword::Menace => "Menace",
            Keyword::Wither => "Wither",
            Keyword::Ethereal => "Ethereal",
            Keyword::Compiling => "Compiling",
            Keyword::SignalJam => "Signal Jam",
            Keyword::Gravecast(_) => "Gravecast",
            Keyword::Glitch => "Glitch",
            Keyword::Malware => "Malware",
            _ => "keyword",
        }
    }

    pub fn unit_type_label(unit_type: &UnitType) -> &'static str {
        match unit_type {
            UnitType::None => "unit",
            UnitType::BioOrganism => "unit • bio-organism",
            UnitType::Human => "unit • human",
            UnitType::Megafauna => "unit • megafauna",
            UnitType::Anomaly => "unit • anomaly",
            UnitType::Drone => "unit • drone",
            UnitType::Virus => "unit • virus",
            UnitType::Hacker => "unit • hacker",
            UnitType::Node => "unit • node",
        }
    }

    pub fn kind_label(kind: &CardKind) -> &'static str {
        match kind {
            CardKind::Unit(kind) => Self::unit_type_label(kind),
            CardKind::Infrastructure => "infrastructure",
            CardKind::Relic => "relic",
            CardKind::Instant => "instant",
            CardKind::Ritual => "ritual",
            CardKind::Aura => "aura",
            CardKind::Agent => "agent",
        }
    }

    pub fn kind_tag(kind: &CardKind) -> &'static str {
        match kind {
            CardKind::Infrastructure => "infrastructure",
            CardKind::Unit(_) => "unit",
            CardKind::Relic => "relic",
            CardKind::Instant => "instant",
            CardKind::Ritual => "ritual",
            CardKind::Aura => "aura",
            CardKind::Agent => "agent",
        }
    }

    pub fn frame_style_for_card(card: &Card) -> &'static str {
        if matches!(card.kind, CardKind::Infrastructure) {
            return "infra_hex";
        }
        if matches!(card.kind, CardKind::Relic) {
            return "artifact_cube";
        }
        if matches!(card.kind, CardKind::Aura)
            && card.attachment.is_some()
            && card.attached_to.is_some()
        {
            return "attachment_fx";
        }
        if matches!(card.kind, CardKind::Aura) {
            return "enchantment_card";
        }
        if matches!(card.kind, CardKind::Instant | CardKind::Ritual) {
            return "spell_card";
        }
        if matches!(card.kind, CardKind::Agent) {
            return "agent_card";
        }
        "card"
    }

    pub fn keyword_text(keyword: &Keyword) -> String {
        match keyword {
            Keyword::Haste => "Haste — can attack the turn it enters".to_string(),
            Keyword::Deathtouch => {
                "Deathtouch — any damage this deals destroys the target".to_string()
            }
            Keyword::Piercing => "Piercing — ignores Armor and Shield".to_string(),
            Keyword::ReactiveArmor(amount) => {
                format!("Reactive Armor {0} — reduce incoming damage by {0}", amount)
            }
            Keyword::Optics(unit_type) => format!(
                "Optics — can only be blocked by {}",
                Self::unit_type_name(unit_type, true)
            ),
            Keyword::Gravecast(n) => {
                if *n == 1 {
                    "Gravecast — can be cast once from the graveyard".to_string()
                } else {
                    format!("Gravecast {n} — can be cast {n} times from the graveyard")
                }
            }
            Keyword::Firewall(amount) => {
                format!("Firewall {0} — costs {0} extra to target this", amount)
            }
            Keyword::SignalJam => "Signal Jam — opponent can't use activated abilities".to_string(),
            Keyword::Stealth => "Stealth — cannot be blocked".to_string(),
            Keyword::Aero => "Aero — can only be blocked by Aero/Interceptor".to_string(),
            Keyword::Throttle(amount) => {
                format!("Throttle — opponent's spells cost {} more", amount)
            }
            Keyword::Glitch => "Glitch — nullify the unit's abilities".to_string(),
            Keyword::Malware => "Malware — units damaged by this lose all abilities".to_string(),
            Keyword::Ethereal => {
                "Ethereal — cannot be targeted by physical damage/spells".to_string()
            }
            Keyword::Compiling => {
                "Compiling — activated abilities unavailable the turn it enters".to_string()
            }
            Keyword::Trample => {
                "Trample — excess combat damage hits the defending player".to_string()
            }
            Keyword::Camouflage => {
                "Camouflage — cannot be targeted by opponent spells/abilities".to_string()
            }
            Keyword::Interceptor => "Interceptor — can block Aero units".to_string(),
            Keyword::Flash => "Flash — can be cast at instant speed".to_string(),
            Keyword::StaticPowerBuff(amount, Some(unit_type)) => {
                format!(
                    "{} get +{}/+0",
                    Self::unit_type_name(unit_type, true),
                    amount
                )
            }
            Keyword::StaticPowerBuff(amount, None) => {
                format!("Units get +{}/+0", amount)
            }
            Keyword::StaticToughnessBuff(amount, Some(unit_type)) => {
                format!(
                    "{} get +0/+{}",
                    Self::unit_type_name(unit_type, true),
                    amount
                )
            }
            Keyword::StaticToughnessBuff(amount, None) => {
                format!("Units get +0/+{}", amount)
            }
            Keyword::SelfPowerPerKind(amount, kind) => {
                format!(
                    "Gets +{}/+0 for each {} you control",
                    amount,
                    Self::kind_label(kind)
                )
            }
            Keyword::SelfToughnessPerKind(amount, kind) => {
                format!(
                    "Gets +0/+{} for each {} you control",
                    amount,
                    Self::kind_label(kind)
                )
            }
            Keyword::Indestructible => "Indestructible — cannot be destroyed".to_string(),
            Keyword::Vigilance => "Vigilance — does not tap when attacking".to_string(),
            Keyword::Loyalty(amount) => format!("Loyalty {}", amount),
            Keyword::ExtraInfrastructures(count) => {
                format!("Play {} additional infrastructure per turn", count)
            }
            Keyword::UntapDuringOtherTurns => "Untap during other players' turns".to_string(),
            Keyword::StaticHasteGrant(target) => {
                format!("{} have Haste", Self::grant_target_subject(target))
            }
            Keyword::StaticTrampleGrant(target) => {
                format!("{} have Trample", Self::grant_target_subject(target))
            }
            Keyword::StaticStealthGrant(target) => {
                format!("{} are unblockable", Self::grant_target_subject(target))
            }
            Keyword::StaticIndestructibleGrant(target) => {
                format!("{} have Indestructible", Self::grant_target_subject(target))
            }
            Keyword::StaticKeywordGrant(keyword, target) => {
                format!(
                    "{} have {}",
                    Self::grant_target_subject(target),
                    Self::keyword_name(keyword)
                )
            }
            Keyword::StaticTapForMana(mana_type, amount, target) => {
                let mana_name = match mana_type {
                    ManaType::Neon => "Neon",
                    ManaType::Glitch => "Glitch",
                    ManaType::Chrome => "Chrome",
                    ManaType::Slag => "Slag",
                    ManaType::Bio => "Bio",
                    ManaType::Colorless => "Colorless",
                };
                format!(
                    "{} can tap for {} {}",
                    Self::grant_target_subject(target),
                    amount,
                    mana_name
                )
            }

            // --- New keywords ---
            Keyword::Menace => "Menace — must be blocked by two or more units".to_string(),
            Keyword::Wither => "Wither — damage to units becomes -1/-1 counters".to_string(),
            Keyword::Absorb(amount) => format!(
                "Absorb {0} — prevent the first {0} damage each turn",
                amount
            ),
            Keyword::Modular(amount) => format!(
                "Modular {0} — enters with {0} +1/+1 counters; on death, move them to another unit",
                amount
            ),
            Keyword::Evolve => {
                "Evolve — when a stronger unit enters, put a +1/+1 counter here".to_string()
            }
            Keyword::Echo(cost) => {
                let cost_text =
                    Self::cost_text(&crate::card::AbilityCost::new().with_mana(cost.clone()));
                format!(
                    "Echo {} — at your next upkeep, pay the echo cost or sacrifice",
                    cost_text
                )
            }
            Keyword::Cascade => {
                "Cascade — when cast, exile cards until a cheaper one; cast it free".to_string()
            }
            Keyword::CostReduction(amount, Some(kind)) => {
                format!("Your {} cost {} less", Self::kind_label(kind), amount)
            }
            Keyword::CostReduction(amount, None) => {
                format!("Your spells cost {} less", amount)
            }
            Keyword::Rampage(amount) => format!(
                "Rampage {0} — when attacking, deal {0} damage to the defending player",
                amount
            ),
        }
    }

    pub fn trigger_event_text(event: &TriggerEvent) -> String {
        match event {
            TriggerEvent::Entered => "When enters".to_string(),
            TriggerEvent::Left => "When leaves".to_string(),
            TriggerEvent::Tapped => "When tapped".to_string(),
            TriggerEvent::AllyEntered(Some(unit_type)) => {
                format!("When a {} enters", Self::unit_type_name(unit_type, false))
            }
            TriggerEvent::AllyEntered(None) => "When another unit enters".to_string(),
            TriggerEvent::Died => "When dies".to_string(),
            TriggerEvent::Attacked => "When attacks".to_string(),
            TriggerEvent::Blocked => "When blocks".to_string(),
            TriggerEvent::DealtDamage => "When deals damage".to_string(),
            TriggerEvent::ReceivedDamage => "When damaged".to_string(),
            TriggerEvent::Discarded => "When a player discards".to_string(),
            TriggerEvent::EndOfTurn => "At end of turn".to_string(),
            TriggerEvent::TurnStart => "At turn start".to_string(),
            TriggerEvent::Upkeep => "At upkeep".to_string(),
            TriggerEvent::DrawStep => "At draw step".to_string(),
            TriggerEvent::CastInstantOrRitual => "When you cast an instant or ritual".to_string(),
            TriggerEvent::CastRelic => "When you cast a relic".to_string(),
            TriggerEvent::CastDuringOpponentTurn => {
                "When you cast a spell during opponent's turn".to_string()
            }
            TriggerEvent::CastAny => "When a player casts a spell".to_string(),
            TriggerEvent::OpponentCast => "When opponent casts a spell".to_string(),
            TriggerEvent::OpponentCastNonUnit => "When opponent casts a non-unit spell".to_string(),
            TriggerEvent::CounteredSpell => "When you counter a spell".to_string(),
            TriggerEvent::OpponentDrew => "When opponent draws a card".to_string(),
            TriggerEvent::DrewCard => "When you draw a card".to_string(),
            TriggerEvent::Sacrificed => "When a permanent is sacrificed".to_string(),
            TriggerEvent::AllyDied => "When a unit dies".to_string(),
            TriggerEvent::AnyAllyDealtDamage => "When your unit deals combat damage".to_string(),

            // --- New trigger events ---
            TriggerEvent::LifeLost => "When you lose life".to_string(),
            TriggerEvent::LifeGained => "When you gain life".to_string(),
            TriggerEvent::GraveyardEntry => "When a card enters your graveyard".to_string(),
            TriggerEvent::OpponentUnitDied => "When an opponent's unit dies".to_string(),
            TriggerEvent::TokenCreated => "When a token enters your battlefield".to_string(),
            TriggerEvent::Equipped => "When equipment is attached".to_string(),
            TriggerEvent::SearchedLibrary => "When you search your library".to_string(),
        }
    }

    pub fn cost_text(cost: &AbilityCost) -> String {
        let mut parts = Vec::new();

        for c in &cost.costs {
            match c {
                Cost::Tap => {
                    parts.push("Tap".to_string());
                }
                Cost::TapOther(count) => {
                    parts.push(format!("Tap {} other units", count));
                }
                Cost::Mana(mana) => {
                    let mana_parts: Vec<String> = vec![
                        if mana.neon > 0 {
                            Some(format!("{} Neon", mana.neon))
                        } else {
                            None
                        },
                        if mana.glitch > 0 {
                            Some(format!("{} Glitch", mana.glitch))
                        } else {
                            None
                        },
                        if mana.chrome > 0 {
                            Some(format!("{} Chrome", mana.chrome))
                        } else {
                            None
                        },
                        if mana.slag > 0 {
                            Some(format!("{} Slag", mana.slag))
                        } else {
                            None
                        },
                        if mana.bio > 0 {
                            Some(format!("{} Bio", mana.bio))
                        } else {
                            None
                        },
                        if mana.colorless > 0 {
                            Some(format!("{}", mana.colorless))
                        } else {
                            None
                        },
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    if !mana_parts.is_empty() {
                        parts.push(mana_parts.join(", "));
                    }
                }
                Cost::Sacrifice(count) => {
                    if *count == 1 {
                        parts.push("Sacrifice a unit".to_string());
                    } else {
                        parts.push(format!("Sacrifice {} units", count));
                    }
                }
                Cost::PayLife(amount) => {
                    parts.push(format!("Pay {} life", amount));
                }
                Cost::Discard(count) => {
                    if *count == 1 {
                        parts.push("Discard a card".to_string());
                    } else {
                        parts.push(format!("Discard {} cards", count));
                    }
                }
                Cost::Exile(count) => {
                    if *count == 1 {
                        parts.push("Exile a card".to_string());
                    } else {
                        parts.push(format!("Exile {} cards", count));
                    }
                }
                Cost::SpendCharge(count) => {
                    if *count == 1 {
                        parts.push("Spend a Charge".to_string());
                    } else {
                        parts.push(format!("Spend {} Charge", count));
                    }
                }
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join(", ")
        }
    }

    pub fn trigger_text(trigger: &CardTrigger) -> String {
        let mut event_text = Self::trigger_event_text(&trigger.event);
        let mut has_predicate = false;

        // Add predicate context if present
        if let Some(ref predicate) = trigger.predicate {
            has_predicate = true;
            if let Some(special_text) = Self::trigger_predicate_text(&trigger.event, predicate) {
                event_text = special_text;
            } else {
                let pred_text = Self::predicate_text(predicate);
                // Make the wording more natural based on the trigger event
                event_text = match &trigger.event {
                    TriggerEvent::Entered => format!("Whenever a {} enters", pred_text),
                    TriggerEvent::AllyEntered(_) => {
                        format!("Whenever a {} enters", pred_text)
                    }
                    _ => format!("{} that is a {}", event_text, pred_text),
                };
            }
        }

        let mut effect_text = Self::effect_text(&trigger.effect, None);

        // Optional triggers without a free cost become "You may ..." — the
        // cancel button on the target-selection UI is the "no" choice. Lower
        // the first letter of the effect so "Destroy target unit" becomes
        // "destroy target unit" after "you may ".
        if trigger.is_optional && trigger.optional_cost.is_none() {
            let trimmed = effect_text.trim_end_matches('.').to_string();
            let lowered = {
                let mut chars = trimmed.chars();
                match chars.next() {
                    Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            };
            effect_text = format!("you may {lowered}");
        }

        // Prefix "Once per turn," when the trigger is limited.
        let limit_prefix = match trigger.limit {
            TriggerLimit::OncePerTurn => "Once per turn, ",
            TriggerLimit::Unlimited => "",
        };

        // For an unconditional `Entered` trigger (plain "when enters"), just
        // render the effect — the "when enters" prefix is redundant noise.
        // Keep the prefix when there's a predicate ("Whenever a <type> enters").
        let result = if matches!(trigger.event, TriggerEvent::Entered) && !has_predicate {
            let trimmed = effect_text.trim_end_matches('.');
            let mut chars = trimmed.chars();
            let capitalized = match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            };
            if limit_prefix.is_empty() {
                capitalized
            } else {
                // "Once per turn, destroy target unit"
                let lowered = {
                    let mut ch = capitalized.chars();
                    match ch.next() {
                        Some(c) => c.to_lowercase().collect::<String>() + ch.as_str(),
                        None => String::new(),
                    }
                };
                format!("{}{}", limit_prefix, lowered)
            }
        } else {
            format!(
                "{}{}, {}",
                limit_prefix,
                event_text,
                effect_text.trim_end_matches('.')
            )
        };

        result
    }

    pub fn ability_text(ability: &CardAbility) -> String {
        // For agent abilities (loyalty costs), extract the cost and combine with effect
        if ability.name.starts_with('[') {
            // Extract loyalty cost like "[+1]" or "[-3]"
            if let Some(close_bracket) = ability.name.find(']') {
                let loyalty_cost = &ability.name[..=close_bracket];

                // Filter out ModifyLoyalty from the effect for cleaner text
                let filtered_effect = Self::filter_modify_loyalty(&ability.effect);
                let effect = Self::effect_text(&filtered_effect, ability.target_spec.as_ref());

                return format!("{} {}", loyalty_cost, effect.trim_end_matches('.'));
            }
            // Fallback if we can't parse it
            return ability.name.clone();
        }

        let cost = Self::cost_text(&ability.cost);
        let effect = Self::effect_text(&ability.effect, ability.target_spec.as_ref());

        if cost.is_empty() {
            effect
        } else {
            format!("{}: {}", cost, effect.trim_end_matches('.'))
        }
    }

    fn filter_modify_loyalty(effect: &CardEffect) -> CardEffect {
        match effect {
            CardEffect::Sequence(effects) => {
                let filtered: Vec<CardEffect> = effects
                    .iter()
                    .filter(|e| !matches!(e, CardEffect::ModifyLoyalty(_)))
                    .cloned()
                    .collect();

                if filtered.len() == 1 {
                    filtered[0].clone()
                } else if filtered.is_empty() {
                    CardEffect::Sequence(vec![])
                } else {
                    CardEffect::Sequence(filtered)
                }
            }
            _ => effect.clone(),
        }
    }
}
