use crate::card::*;
use crate::deck::{DeckCatalog, DeckTemplateEntry, build_deck_catalog};
use crate::types::*;
use crate::user_card::{CardAuditEntry, UserCardInstance};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use tawks_data::pagination::{Cursor, Model, Node, PaginationArgs, WithPagination};
use tawks_data::repository::{CursorDirection, Repository};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardCursor {
    pub id: String,
}

impl Display for CardCursor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl Cursor for CardCursor {
    type CursorType = CardCursor;
    fn sort_key(&self) -> String {
        self.id.clone()
    }
    fn id(&self) -> String {
        self.id.clone()
    }
    fn decode(encoded: &str) -> Option<Self::CursorType> {
        let decoded = base64::decode(encoded).ok()?;
        serde_json::from_slice(&decoded).ok()
    }
    fn encode(cursor: &Self::CursorType) -> String {
        let json = serde_json::to_vec(cursor).unwrap();
        base64::encode(json)
    }
}

#[derive(Clone)]
pub struct CardArgs {
    pub pagination: PaginationArgs,
    pub owner_id: Option<String>,
}

impl WithPagination for CardArgs {
    type Meta = ();
    type CursorType = CardCursor;

    fn pagination(&self) -> PaginationArgs {
        self.pagination.clone()
    }

    fn get_meta(&self) -> Self::Meta {
        ()
    }

    fn to_cursor(&self, id: String, _sort_key: String) -> Self::CursorType {
        CardCursor { id }
    }
}

impl Model<Card> for Card {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn to_node(&self) -> Card {
        self.clone()
    }
}

impl Node for Card {
    fn id(&self) -> String {
        self.id.clone()
    }
}

#[derive(Debug)] // Add Debug derive here
pub struct CardRepository {
    pool: Arc<PgPool>,
}

#[derive(Debug, Clone)]
pub struct DeckRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_id: Option<String>,
    pub base_deck_id: Option<String>,
    pub format: DeckFormat,
    pub operator_card_id: Option<String>,
    pub image_url: Option<String>,
}

fn enum_to_string<T: Serialize>(value: &T) -> Result<String> {
    let json = serde_json::to_value(value)?;
    match &json {
        serde_json::Value::String(s) => Ok(s.clone()),
        // For nested enums like CardKind::Relic, extract the top-level key
        serde_json::Value::Object(map) => map
            .keys()
            .next()
            .map(|k| k.clone())
            .ok_or_else(|| anyhow!("Empty object when serializing enum")),
        _ => Err(anyhow!("Expected enum to serialize as string or object")),
    }
}

fn parse_enum_value<T: DeserializeOwned>(value: &str) -> Result<T> {
    // Try as a quoted string first (simple enums like "instant")
    serde_json::from_str(&format!("\"{}\"", value))
        // Fall back to raw JSON (e.g. already a JSON object string)
        .or_else(|_| serde_json::from_str(value))
        // Fall back to wrapping as a newtype variant with a default inner value
        // e.g. "artifact" -> {"artifact":"object"} for CardKind::Relic
        .or_else(|_| serde_json::from_str(&format!("{{\"{}\":\"object\"}}", value)))
        .map_err(|err| anyhow!("Invalid enum value {value}: {err}"))
}

fn grant_side_suffix(side: GrantSide) -> &'static str {
    match side {
        GrantSide::Mine => "",
        GrantSide::Opponents => "__opponents",
        GrantSide::All => "__all",
    }
}

fn parse_grant_side_suffix(suffix: &str) -> Result<GrantSide> {
    match suffix {
        "" => Ok(GrantSide::Mine),
        "__opponents" => Ok(GrantSide::Opponents),
        "__all" => Ok(GrantSide::All),
        other => Err(anyhow!("Unknown grant side suffix: {other}")),
    }
}

fn split_grant_side_suffix(rest: &str) -> (&str, &str) {
    for sep in ["__opponents", "__all"] {
        if let Some(stripped) = rest.strip_suffix(sep) {
            return (stripped, sep);
        }
    }
    (rest, "")
}

fn grant_filter_to_string(filter: &Option<GrantFilter>) -> Result<Option<String>> {
    match filter {
        None => Ok(None),
        Some(GrantFilter::UnitType(t)) => Ok(Some(enum_to_string(t)?)),
    }
}

fn grant_target_from_parts(side: GrantSide, unit_type: Option<String>) -> Result<GrantTarget> {
    let filter = unit_type
        .map(|value| parse_enum_value::<UnitType>(&value).map(GrantFilter::UnitType))
        .transpose()?;
    Ok(GrantTarget { side, filter })
}

fn split_trigger_event(event: &TriggerEvent) -> Result<(String, Option<String>)> {
    match event {
        TriggerEvent::AllyEntered(unit_type) => Ok((
            "ally_entered".to_string(),
            unit_type.as_ref().map(enum_to_string).transpose()?,
        )),
        other => Ok((enum_to_string(other)?, None)),
    }
}

fn parse_trigger_event(kind: &str, unit_type: Option<String>) -> Result<TriggerEvent> {
    if kind == "ally_entered" {
        let unit_type = unit_type
            .map(|value| parse_enum_value(&value))
            .transpose()?;
        Ok(TriggerEvent::AllyEntered(unit_type))
    } else {
        parse_enum_value(kind)
    }
}

fn split_expire_contract(expires: &ExpireContract) -> Result<(String, Option<i32>)> {
    match expires {
        ExpireContract::Never => Ok(("never".to_string(), None)),
        ExpireContract::EndOfTurn => Ok(("end_of_turn".to_string(), None)),
        ExpireContract::Turns(turns) => Ok(("turns".to_string(), Some(*turns as i32))),
    }
}

fn parse_expire_contract(kind: Option<String>, turns: Option<i32>) -> Result<ExpireContract> {
    let Some(kind) = kind else {
        return Err(anyhow!("Missing expire_contract"));
    };
    match kind.as_str() {
        "never" => Ok(ExpireContract::Never),
        "end_of_turn" => Ok(ExpireContract::EndOfTurn),
        "turns" => {
            let turns = turns.ok_or_else(|| anyhow!("Missing turns for expire_contract"))?;
            Ok(ExpireContract::Turns(turns as u8))
        }
        other => Err(anyhow!("Invalid expire_contract: {other}")),
    }
}

fn split_effect_target_spec(spec: &EffectTargetSpec) -> Result<(String, Option<String>)> {
    match spec {
        EffectTargetSpec::Targets => Ok(("targets".to_string(), None)),
        EffectTargetSpec::Source => Ok(("source".to_string(), None)),
        EffectTargetSpec::OwnerUnits(unit_type) => Ok((
            "owner_units".to_string(),
            unit_type.as_ref().map(enum_to_string).transpose()?,
        )),
        EffectTargetSpec::OpponentUnits(unit_type) => Ok((
            "opponent_units".to_string(),
            unit_type.as_ref().map(enum_to_string).transpose()?,
        )),
        EffectTargetSpec::AllUnits(unit_type) => Ok((
            "all_units".to_string(),
            unit_type.as_ref().map(enum_to_string).transpose()?,
        )),
        EffectTargetSpec::AdjacentUnits => Ok(("adjacent_units".to_string(), None)),
        EffectTargetSpec::LastCast => Ok(("last_cast".to_string(), None)),
    }
}

fn parse_effect_target_spec(
    kind: Option<String>,
    unit_type: Option<String>,
) -> Result<EffectTargetSpec> {
    let Some(kind) = kind else {
        return Err(anyhow!("Missing effect target spec kind"));
    };
    match kind.as_str() {
        "targets" => Ok(EffectTargetSpec::Targets),
        "source" | "self" => Ok(EffectTargetSpec::Source),
        "owner_units" => Ok(EffectTargetSpec::OwnerUnits(
            unit_type
                .map(|value| parse_enum_value(&value))
                .transpose()?,
        )),
        "opponent_units" => Ok(EffectTargetSpec::OpponentUnits(
            unit_type
                .map(|value| parse_enum_value(&value))
                .transpose()?,
        )),
        "all_units" => Ok(EffectTargetSpec::AllUnits(
            unit_type
                .map(|value| parse_enum_value(&value))
                .transpose()?,
        )),
        "adjacent_units" => Ok(EffectTargetSpec::AdjacentUnits),
        "last_cast" => Ok(EffectTargetSpec::LastCast),
        other => Err(anyhow!("Invalid effect target spec kind: {other}")),
    }
}

fn split_keyword(keyword: &Keyword) -> Result<(String, Option<i32>, Option<String>)> {
    match keyword {
        Keyword::Haste => Ok(("haste".to_string(), None, None)),
        Keyword::Deathtouch => Ok(("deathtouch".to_string(), None, None)),
        Keyword::Piercing => Ok(("piercing".to_string(), None, None)),
        Keyword::ReactiveArmor(amount) => Ok(("reactive_armor".to_string(), Some(*amount), None)),
        Keyword::Optics(unit_type) => {
            Ok(("optics".to_string(), None, Some(enum_to_string(unit_type)?)))
        }
        Keyword::Gravecast(n) => Ok(("gravecast".to_string(), Some(*n as i32), None)),
        Keyword::Firewall(amount) => Ok(("firewall".to_string(), Some(*amount), None)),
        Keyword::SignalJam => Ok(("signal_jam".to_string(), None, None)),
        Keyword::Stealth => Ok(("stealth".to_string(), None, None)),
        Keyword::Aero => Ok(("aero".to_string(), None, None)),
        Keyword::Throttle(amount) => Ok(("throttle".to_string(), Some(*amount), None)),
        Keyword::Glitch => Ok(("glitch".to_string(), None, None)),
        Keyword::Malware => Ok(("malware".to_string(), None, None)),
        Keyword::Ethereal => Ok(("ethereal".to_string(), None, None)),
        Keyword::Compiling => Ok(("compiling".to_string(), None, None)),
        Keyword::Trample => Ok(("trample".to_string(), None, None)),
        Keyword::Camouflage => Ok(("camouflage".to_string(), None, None)),
        Keyword::Interceptor => Ok(("interceptor".to_string(), None, None)),
        Keyword::Flash => Ok(("flash".to_string(), None, None)),
        Keyword::StaticPowerBuff(amount, unit_type) => Ok((
            "static_power_buff".to_string(),
            Some(*amount),
            unit_type.as_ref().map(enum_to_string).transpose()?,
        )),
        Keyword::StaticToughnessBuff(amount, unit_type) => Ok((
            "static_toughness_buff".to_string(),
            Some(*amount),
            unit_type.as_ref().map(enum_to_string).transpose()?,
        )),
        // Self-scaling buffs use the unit_type column to persist the
        // CardKind we scale with (we don't have a separate column for
        // CardKind on the keywords table). Readback parses the string
        // back to CardKind via `parse_enum_value`.
        Keyword::SelfPowerPerKind(amount, kind) => Ok((
            "self_power_per_kind".to_string(),
            Some(*amount),
            Some(enum_to_string(kind)?),
        )),
        Keyword::SelfToughnessPerKind(amount, kind) => Ok((
            "self_toughness_per_kind".to_string(),
            Some(*amount),
            Some(enum_to_string(kind)?),
        )),
        Keyword::Indestructible => Ok(("indestructible".to_string(), None, None)),
        Keyword::Vigilance => Ok(("vigilance".to_string(), None, None)),
        Keyword::Loyalty(amount) => Ok(("loyalty".to_string(), Some(*amount), None)),
        Keyword::ExtraInfrastructures(count) => Ok((
            "extra_infrastructures".to_string(),
            Some(*count as i32),
            None,
        )),
        Keyword::UntapDuringOtherTurns => Ok(("untap_during_other_turns".to_string(), None, None)),
        Keyword::StaticHasteGrant(target) => Ok((
            format!("static_haste_grant{}", grant_side_suffix(target.side)),
            None,
            grant_filter_to_string(&target.filter)?,
        )),
        Keyword::StaticTrampleGrant(target) => Ok((
            format!("static_trample_grant{}", grant_side_suffix(target.side)),
            None,
            grant_filter_to_string(&target.filter)?,
        )),
        Keyword::StaticStealthGrant(target) => Ok((
            format!("static_stealth_grant{}", grant_side_suffix(target.side)),
            None,
            grant_filter_to_string(&target.filter)?,
        )),
        Keyword::StaticIndestructibleGrant(target) => Ok((
            format!(
                "static_indestructible_grant{}",
                grant_side_suffix(target.side)
            ),
            None,
            grant_filter_to_string(&target.filter)?,
        )),
        Keyword::StaticKeywordGrant(keyword, target) => {
            let keyword_tag = match keyword.as_ref() {
                Keyword::Haste => "haste",
                Keyword::Trample => "trample",
                Keyword::Stealth => "stealth",
                Keyword::Indestructible => "indestructible",
                Keyword::Interceptor => "interceptor",
                Keyword::Aero => "aero",
                Keyword::Deathtouch => "deathtouch",
                Keyword::Vigilance => "vigilance",
                Keyword::Flash => "flash",
                Keyword::Camouflage => "camouflage",
                Keyword::Menace => "menace",
                Keyword::Wither => "wither",
                Keyword::Ethereal => "ethereal",
                _ => "unknown",
            };
            Ok((
                format!(
                    "static_keyword_grant_{keyword_tag}{}",
                    grant_side_suffix(target.side)
                ),
                None,
                grant_filter_to_string(&target.filter)?,
            ))
        }
        Keyword::StaticTapForMana(_mana_type, _amount, target) => Ok((
            format!("static_tap_for_mana{}", grant_side_suffix(target.side)),
            None,
            grant_filter_to_string(&target.filter)?,
        )),
        // New keywords
        Keyword::Menace => Ok(("menace".to_string(), None, None)),
        Keyword::Wither => Ok(("wither".to_string(), None, None)),
        Keyword::Absorb(amount) => Ok(("absorb".to_string(), Some(*amount), None)),
        Keyword::Modular(amount) => Ok(("modular".to_string(), Some(*amount), None)),
        Keyword::Evolve => Ok(("evolve".to_string(), None, None)),
        Keyword::Echo(_cost) => Ok(("echo".to_string(), None, None)),
        Keyword::Cascade => Ok(("cascade".to_string(), None, None)),
        Keyword::CostReduction(amount, _kind) => {
            Ok(("cost_reduction".to_string(), Some(*amount), None))
        }
        Keyword::Rampage(amount) => Ok(("rampage".to_string(), Some(*amount), None)),
    }
}

fn parse_keyword(
    kind: Option<String>,
    amount: Option<i32>,
    unit_type: Option<String>,
) -> Result<Keyword> {
    let Some(kind) = kind else {
        return Err(anyhow!("Missing keyword kind"));
    };
    match kind.as_str() {
        "haste" => Ok(Keyword::Haste),
        "deathtouch" => Ok(Keyword::Deathtouch),
        "piercing" => Ok(Keyword::Piercing),
        "reactive_armor" => Ok(Keyword::ReactiveArmor(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
        )),
        "optics" => {
            let unit_type = unit_type.ok_or_else(|| anyhow!("Missing optics unit type"))?;
            Ok(Keyword::Optics(parse_enum_value(&unit_type)?))
        }
        "gravecast" => Ok(Keyword::Gravecast(amount.unwrap_or(1).max(1) as u8)),
        "firewall" => Ok(Keyword::Firewall(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
        )),
        "signal_jam" => Ok(Keyword::SignalJam),
        "stealth" => Ok(Keyword::Stealth),
        "aero" => Ok(Keyword::Aero),
        "throttle" => Ok(Keyword::Throttle(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
        )),
        "glitch" => Ok(Keyword::Glitch),
        "malware" => Ok(Keyword::Malware),
        "ethereal" => Ok(Keyword::Ethereal),
        "trample" => Ok(Keyword::Trample),
        "camouflage" => Ok(Keyword::Camouflage),
        "interceptor" => Ok(Keyword::Interceptor),
        "flash" => Ok(Keyword::Flash),
        "static_power_buff" => Ok(Keyword::StaticPowerBuff(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
            unit_type
                .map(|value| parse_enum_value(&value))
                .transpose()?,
        )),
        "static_toughness_buff" => Ok(Keyword::StaticToughnessBuff(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
            unit_type
                .map(|value| parse_enum_value(&value))
                .transpose()?,
        )),
        "self_power_per_kind" => {
            let kind_str = unit_type
                .ok_or_else(|| anyhow!("Missing scale card kind for self_power_per_kind"))?;
            Ok(Keyword::SelfPowerPerKind(
                amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
                parse_enum_value(&kind_str)?,
            ))
        }
        "self_toughness_per_kind" => {
            let kind_str = unit_type
                .ok_or_else(|| anyhow!("Missing scale card kind for self_toughness_per_kind"))?;
            Ok(Keyword::SelfToughnessPerKind(
                amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
                parse_enum_value(&kind_str)?,
            ))
        }
        "indestructible" => Ok(Keyword::Indestructible),
        "vigilance" => Ok(Keyword::Vigilance),
        "loyalty" => Ok(Keyword::Loyalty(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
        )),
        "extra_infrastructures" => Ok(Keyword::ExtraInfrastructures(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))? as u8,
        )),
        "untap_during_other_turns" => Ok(Keyword::UntapDuringOtherTurns),
        tag if tag.starts_with("static_haste_grant") => {
            let side = parse_grant_side_suffix(tag.trim_start_matches("static_haste_grant"))?;
            Ok(Keyword::StaticHasteGrant(grant_target_from_parts(
                side, unit_type,
            )?))
        }
        tag if tag.starts_with("static_trample_grant") => {
            let side = parse_grant_side_suffix(tag.trim_start_matches("static_trample_grant"))?;
            Ok(Keyword::StaticTrampleGrant(grant_target_from_parts(
                side, unit_type,
            )?))
        }
        tag if tag.starts_with("static_stealth_grant") => {
            let side = parse_grant_side_suffix(tag.trim_start_matches("static_stealth_grant"))?;
            Ok(Keyword::StaticStealthGrant(grant_target_from_parts(
                side, unit_type,
            )?))
        }
        tag if tag.starts_with("static_indestructible_grant") => {
            let side =
                parse_grant_side_suffix(tag.trim_start_matches("static_indestructible_grant"))?;
            Ok(Keyword::StaticIndestructibleGrant(grant_target_from_parts(
                side, unit_type,
            )?))
        }
        tag if tag.starts_with("static_keyword_grant_") => {
            let rest = tag.trim_start_matches("static_keyword_grant_");
            let (keyword_tag, side_suffix) = split_grant_side_suffix(rest);
            let granted = match keyword_tag {
                "haste" => Keyword::Haste,
                "trample" => Keyword::Trample,
                "stealth" => Keyword::Stealth,
                "indestructible" => Keyword::Indestructible,
                "interceptor" => Keyword::Interceptor,
                "aero" => Keyword::Aero,
                "deathtouch" => Keyword::Deathtouch,
                "vigilance" => Keyword::Vigilance,
                "flash" => Keyword::Flash,
                "camouflage" => Keyword::Camouflage,
                "menace" => Keyword::Menace,
                "wither" => Keyword::Wither,
                "ethereal" => Keyword::Ethereal,
                _ => return Err(anyhow!("Unknown granted keyword: {}", keyword_tag)),
            };
            let side = parse_grant_side_suffix(side_suffix)?;
            Ok(Keyword::StaticKeywordGrant(
                Box::new(granted),
                grant_target_from_parts(side, unit_type)?,
            ))
        }
        tag if tag.starts_with("static_tap_for_mana") => {
            let side = parse_grant_side_suffix(tag.trim_start_matches("static_tap_for_mana"))?;
            Ok(Keyword::StaticTapForMana(
                ManaType::Colorless,
                amount.unwrap_or(1),
                grant_target_from_parts(side, unit_type)?,
            ))
        }
        "menace" => Ok(Keyword::Menace),
        "wither" => Ok(Keyword::Wither),
        "absorb" => Ok(Keyword::Absorb(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
        )),
        "modular" => Ok(Keyword::Modular(
            amount.ok_or_else(|| anyhow!("Missing keyword amount"))?,
        )),
        "evolve" => Ok(Keyword::Evolve),
        "echo" => Ok(Keyword::Echo(ManaCost::default())),
        "cascade" => Ok(Keyword::Cascade),
        other => Err(anyhow!("Invalid keyword kind: {other}")),
    }
}

fn split_card_kind(kind: &CardKind) -> Result<(String, Option<String>, Option<String>)> {
    match kind {
        CardKind::Infrastructure => Ok(("infrastructure".to_string(), None, None)),
        CardKind::Unit(unit_type) => {
            Ok(("unit".to_string(), Some(enum_to_string(unit_type)?), None))
        }
        CardKind::Relic => Ok(("relic".to_string(), None, Some("object".to_string()))),
        CardKind::Instant => Ok(("instant".to_string(), None, None)),
        CardKind::Ritual => Ok(("ritual".to_string(), None, None)),
        CardKind::Aura => Ok(("aura".to_string(), None, None)),
        CardKind::Agent => Ok(("agent".to_string(), None, None)),
    }
}

fn assemble_card_kind(
    kind: &str,
    unit_type: Option<String>,
    _artifact_type: Option<String>,
) -> Result<CardKind> {
    match kind {
        "infrastructure" => Ok(CardKind::Infrastructure),
        "unit" => {
            let unit_type = unit_type.unwrap_or_else(|| "none".to_string());
            let unit_type: UnitType = parse_enum_value(&unit_type)?;
            Ok(CardKind::Unit(unit_type))
        }
        "relic" => Ok(CardKind::Relic),
        "instant" => Ok(CardKind::Instant),
        "ritual" => Ok(CardKind::Ritual),
        "aura" => Ok(CardKind::Aura),
        "agent" => Ok(CardKind::Agent),
        other => Err(anyhow!("Invalid card kind: {other}")),
    }
}

fn parse_json<T: DeserializeOwned>(
    value: Option<serde_json::Value>,
    default: serde_json::Value,
) -> Result<T> {
    let value = value.unwrap_or(default);
    serde_json::from_value(value).map_err(Into::into)
}

fn user_card_instance_from_row(row: &sqlx::postgres::PgRow) -> Result<UserCardInstance> {
    let visual_spec_value: serde_json::Value = row.try_get("visual_spec")?;
    let visual_spec: VisualSpec = serde_json::from_value(visual_spec_value)?;

    Ok(UserCardInstance {
        instance_id: row.try_get("instance_id")?,
        card_id: row.try_get("card_id")?,
        owner_id: row.try_get("owner_id")?,
        visual_spec,
        is_tradeable: row.try_get("is_tradeable")?,
        is_bound: row.try_get("is_bound")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn audit_entry_from_row(row: &sqlx::postgres::PgRow) -> Result<CardAuditEntry> {
    let event_kind_str: String = row.try_get("event_kind")?;
    let event_kind: AuditEventKind = parse_enum_value(&event_kind_str)?;

    let prev_spec: Option<serde_json::Value> = row.try_get("previous_visual_spec")?;
    let prev_spec = prev_spec
        .filter(|v| !v.is_null())
        .map(serde_json::from_value)
        .transpose()?;

    let new_spec: Option<serde_json::Value> = row.try_get("new_visual_spec")?;
    let new_spec = new_spec
        .filter(|v| !v.is_null())
        .map(serde_json::from_value)
        .transpose()?;

    Ok(CardAuditEntry {
        log_id: row.try_get("log_id")?,
        instance_id: row.try_get("instance_id")?,
        event_kind,
        actor_id: row.try_get("actor_id")?,
        previous_owner_id: row.try_get("previous_owner_id")?,
        new_owner_id: row.try_get("new_owner_id")?,
        previous_visual_spec: prev_spec,
        new_visual_spec: new_spec,
        metadata: row.try_get("metadata")?,
        created_at: row.try_get("created_at")?,
    })
}

fn card_from_row(row: &sqlx::postgres::PgRow) -> Result<Card> {
    let kind: String = row.try_get("kind")?;
    let unit_type: Option<String> = row.try_get("unit_type")?;
    let artifact_type: Option<String> = row.try_get("artifact_type")?;
    let kind = assemble_card_kind(&kind, unit_type, artifact_type)?;
    let supertype: Option<String> = row.try_get("supertype")?;
    let supertype = if let Some(value) = supertype {
        parse_enum_value(&value)?
    } else {
        CardSupertype::Basic
    };

    let rarity: Option<String> = row.try_get("rarity")?;
    let rarity = if let Some(value) = rarity {
        parse_enum_value(&value)?
    } else {
        CardRarity::Public
    };

    let raw_categories: Vec<String> = row.try_get("categories")?;
    let categories = raw_categories
        .iter()
        .map(|c| parse_enum_value(c))
        .collect::<Result<Vec<CardCategory>>>()?;

    let cost = ManaCost {
        neon: row.try_get("cost_neon")?,
        glitch: row.try_get("cost_glitch")?,
        chrome: row.try_get("cost_chrome")?,
        slag: row.try_get("cost_slag")?,
        bio: row.try_get("cost_bio")?,
        colorless: row.try_get("cost_colorless")?,
    };

    let attachment: Option<AttachmentSpec> =
        parse_json(row.try_get("attachment")?, serde_json::Value::Null)?;
    let keywords: Vec<Keyword> =
        parse_json(row.try_get("keywords")?, serde_json::Value::Array(vec![]))?;
    let has_haste = keywords
        .iter()
        .any(|keyword| matches!(keyword, Keyword::Haste));
    let raw_colors: Vec<String> = row.try_get("colors")?;
    let colors = raw_colors
        .iter()
        .filter_map(|value| {
            if let Some(mana) = mana_type_for_color_value(value) {
                Some(mana_color_for_type(&mana).to_string())
            } else if value.starts_with('#') {
                Some(value.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(Card {
        id: row.try_get("card_id")?,
        name: row.try_get("name")?,
        description: row
            .try_get::<Option<String>, _>("description")?
            .unwrap_or_default(),
        auto_description: String::new(), // Will be populated after triggers/abilities are loaded
        rarity,
        categories,
        visual_spec: row
            .try_get::<Option<serde_json::Value>, _>("instance_visual_spec")
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_value(v).ok()),
        template_id: None,
        cost,
        kind,
        supertype,
        power: row.try_get("power")?,
        toughness: row.try_get("toughness")?,
        tapped: row.try_get("tapped")?,
        enters_tapped: row.try_get("enters_tapped")?,
        triggers: Vec::new(),
        abilities: Vec::new(),
        keywords,
        image_url: row.try_get("image_url")?,
        colors,
        attached_to: None,
        remembered_player_id: None,
        attachment,
        summoning_sickness: false,
        haste: has_haste,
        burnout: false,
        echo_pending: false,
        malware_infected: false,
        loyalty: row.try_get("loyalty")?,
        counters: 0,
        counters_by_kind: std::collections::HashMap::new(),
        damage: 0,
        gravecast_uses: 0,
        frozen_turns: 0,
        absorb_used_this_turn: 0,
        creator_id: None,
        created_at: row.try_get("created_at").ok(),
        owned: row.try_get::<bool, _>("owned").unwrap_or(false),
        obtained_at: row.try_get("obtained_at").ok().flatten(),
    })
}

fn flatten_effects(effect: &CardEffect, out: &mut Vec<CardEffect>) {
    match effect {
        CardEffect::Sequence(effects) => {
            for effect in effects {
                flatten_effects(effect, out);
            }
        }
        CardEffect::ApplyStatModifiers(specs) => {
            for spec in specs {
                out.push(CardEffect::ApplyStatModifier(spec.clone()));
            }
        }
        other => out.push(other.clone()),
    }
}

fn assemble_effect(mut effects: Vec<CardEffect>) -> Option<CardEffect> {
    match effects.len() {
        0 => None,
        1 => Some(effects.remove(0)),
        _ => Some(CardEffect::Sequence(effects)),
    }
}

fn parse_modifier_amount(row: &sqlx::postgres::PgRow) -> Result<ModifierAmount> {
    let stat_amount_dynamic: Option<serde_json::Value> =
        row.try_get("stat_amount_dynamic").unwrap_or(None);
    if let Some(dynamic_json) = stat_amount_dynamic {
        return Ok(serde_json::from_value(dynamic_json)?);
    }
    let stat_amount: Option<i32> = row.try_get("stat_amount")?;
    let stat_amount =
        stat_amount.ok_or_else(|| anyhow!("Missing stat_amount or stat_amount_dynamic"))?;
    Ok(ModifierAmount::Fixed(stat_amount))
}

fn parse_stat_modifier_required(row: &sqlx::postgres::PgRow) -> Result<StatModifierSpec> {
    let stat_type: Option<String> = row.try_get("stat_type")?;
    let target_kind: Option<String> = row.try_get("target_spec_kind")?;
    let target_unit_type: Option<String> = row.try_get("target_unit_type")?;
    let expires_kind: Option<String> = row.try_get("expires_kind")?;
    let expires_turns: Option<i32> = row.try_get("expires_turns")?;

    let stat_type = stat_type.ok_or_else(|| anyhow!("Missing stat_type"))?;
    let amount = parse_modifier_amount(row)?;

    let target = parse_effect_target_spec(target_kind, target_unit_type)?;
    let expires = parse_expire_contract(expires_kind, expires_turns)?;

    Ok(StatModifierSpec {
        stat: parse_enum_value(&stat_type)?,
        amount,
        expires,
        target,
    })
}

fn parse_stat_modifier_optional(row: &sqlx::postgres::PgRow) -> Result<Option<StatModifierSpec>> {
    let stat_type: Option<String> = row.try_get("stat_type")?;
    if stat_type.is_none() {
        return Ok(None);
    }
    let target_kind: Option<String> = row.try_get("target_spec_kind")?;
    let target_unit_type: Option<String> = row.try_get("target_unit_type")?;
    let expires_kind: Option<String> = row.try_get("expires_kind")?;
    let expires_turns: Option<i32> = row.try_get("expires_turns")?;

    let stat_type = stat_type.ok_or_else(|| anyhow!("Missing stat_type"))?;
    let amount = parse_modifier_amount(row)?;

    let target = parse_effect_target_spec(target_kind, target_unit_type)?;
    let expires = parse_expire_contract(expires_kind, expires_turns)?;

    Ok(Some(StatModifierSpec {
        stat: parse_enum_value(&stat_type)?,
        amount,
        expires,
        target,
    }))
}

fn parse_effect_row(
    row: &sqlx::postgres::PgRow,
    emblem_keywords: Option<&Vec<Keyword>>,
) -> Result<CardEffect> {
    let kind: String = row.try_get("effect_kind")?;
    match kind.as_str() {
        "remember_target_player" => Ok(CardEffect::RememberTargetPlayer),
        "damage_opponent" | "damage_player" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for damage_player"))?;
            Ok(CardEffect::DamagePlayer(amount))
        }
        "damage_unit" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for damage_unit"))?;
            Ok(CardEffect::DamageUnit(amount))
        }
        "damage_target" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for damage_target"))?;
            Ok(CardEffect::DamageTarget(amount))
        }
        "damage_player_per_hand_card" => Ok(CardEffect::DamagePlayerPerHandCard),
        "draw_cards" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for draw_cards"))?;
            Ok(CardEffect::DrawCards(count as usize))
        }
        "gain_life" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for gain_life"))?;
            Ok(CardEffect::GainLife(amount))
        }
        "heat" => {
            let amount: Option<i32> = row.try_get("amount")?;
            Ok(CardEffect::Heat(amount.ok_or_else(|| anyhow!("Missing amount for heat"))?))
        }
        "heat_opponents" => {
            let amount: Option<i32> = row.try_get("amount")?;
            Ok(CardEffect::HeatOpponents(
                amount.ok_or_else(|| anyhow!("Missing amount for heat_opponents"))?,
            ))
        }
        "cash_opponents" => {
            let amount: Option<i32> = row.try_get("amount")?;
            Ok(CardEffect::CashOpponents(
                amount.ok_or_else(|| anyhow!("Missing amount for cash_opponents"))?,
            ))
        }
        "counter_top" => Ok(CardEffect::CounterTop),
        "add_mana" => {
            let mana_type: Option<String> = row.try_get("mana_type")?;
            let amount: Option<i32> = row.try_get("amount")?;
            let mana_type = mana_type.ok_or_else(|| anyhow!("Missing mana_type for add_mana"))?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for add_mana"))?;
            let mana_type: ManaType = parse_enum_value(&mana_type)?;
            Ok(CardEffect::AddMana(mana_type, amount))
        }
        "create_token" => {
            let token_name: Option<String> = row.try_get("token_name")?;
            let token_power: Option<i32> = row.try_get("token_power")?;
            let token_toughness: Option<i32> = row.try_get("token_toughness")?;
            let token_unit_type: Option<String> = row.try_get("token_unit_type")?;
            Ok(CardEffect::CreateToken(TokenSpec::new(
                token_name.ok_or_else(|| anyhow!("Missing token_name"))?,
                token_power.ok_or_else(|| anyhow!("Missing token_power"))?,
                token_toughness.ok_or_else(|| anyhow!("Missing token_toughness"))?,
                parse_enum_value(
                    &token_unit_type.ok_or_else(|| anyhow!("Missing token_unit_type"))?,
                )?,
            )))
        }
        "create_infrastructure" => {
            let spec: Option<serde_json::Value> = row.try_get("effect_predicate")?;
            let spec = spec.ok_or_else(|| anyhow!("Missing spec for create_infrastructure"))?;
            Ok(CardEffect::CreateInfrastructure(serde_json::from_value(
                spec,
            )?))
        }
        "apply_stat_modifier" => Ok(CardEffect::ApplyStatModifier(parse_stat_modifier_required(
            row,
        )?)),
        "reveal_top_if_unit" => Ok(CardEffect::RevealTopIf {
            predicate: Predicate::IsUnit,
            on_match: RevealOutcome::Deploy,
            on_fail: RevealOutcome::ToTop,
        }),
        "reveal_top_if" => {
            let payload: Option<serde_json::Value> = row.try_get("effect_predicate")?;
            let payload = payload.ok_or_else(|| anyhow!("Missing payload for reveal_top_if"))?;
            #[derive(Deserialize)]
            struct RevealPayload {
                predicate: Predicate,
                #[serde(default)]
                on_match: RevealOutcome,
                #[serde(default)]
                on_fail: RevealOutcome,
            }
            let parsed: RevealPayload = serde_json::from_value(payload)?;
            Ok(CardEffect::RevealTopIf {
                predicate: parsed.predicate,
                on_match: parsed.on_match,
                on_fail: parsed.on_fail,
            })
        }
        "freeze" => {
            let expires_kind: Option<String> = row.try_get("expires_kind")?;
            let expires_turns: Option<i32> = row.try_get("expires_turns")?;
            Ok(CardEffect::Freeze(parse_expire_contract(
                expires_kind,
                expires_turns,
            )?))
        }
        "refund_mana" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for refund_mana"))?;
            Ok(CardEffect::RefundMana(amount))
        }
        "loot" => {
            let draw: Option<i32> = row.try_get("amount")?;
            let discard: Option<i32> = row.try_get("count")?;
            let draw = draw.ok_or_else(|| anyhow!("Missing draw for loot"))?;
            let discard = discard.ok_or_else(|| anyhow!("Missing discard for loot"))?;
            Ok(CardEffect::Loot(draw as usize, discard as usize))
        }
        "wheel" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for wheel"))?;
            Ok(CardEffect::Wheel(count as usize))
        }
        "scry" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for scry"))?;
            Ok(CardEffect::Scry(count as usize))
        }
        "search" => {
            let predicate: Option<serde_json::Value> = row.try_get("effect_predicate")?;
            let predicate = predicate.ok_or_else(|| anyhow!("Missing predicate for search"))?;
            Ok(CardEffect::Search(serde_json::from_value(predicate)?))
        }
        "search_to_battlefield" => {
            let predicate: Option<serde_json::Value> = row.try_get("effect_predicate")?;
            let predicate =
                predicate.ok_or_else(|| anyhow!("Missing predicate for search_to_battlefield"))?;
            Ok(CardEffect::SearchToBattlefield(serde_json::from_value(
                predicate,
            )?))
        }
        "bounce" => Ok(CardEffect::Bounce),
        "board_wipe" => Ok(CardEffect::BoardWipe),
        "destroy_target_player_units" => Ok(CardEffect::DestroyTargetPlayerUnits),
        "destroy_units_with_toughness_at_least" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| {
                anyhow!("Missing amount for destroy_units_with_toughness_at_least")
            })?;
            Ok(CardEffect::DestroyUnitsWithToughnessAtLeast(amount))
        }
        "copy_last_cast" => Ok(CardEffect::CopyLastCast),
        "purify" => Ok(CardEffect::Purify),
        "grant_keyword" => {
            let keyword_kind: Option<String> = row.try_get("keyword_kind")?;
            let keyword_amount: Option<i32> = row.try_get("keyword_amount")?;
            let keyword_unit_type: Option<String> = row.try_get("keyword_unit_type")?;
            let expires_kind: Option<String> = row.try_get("expires_kind")?;
            let expires_turns: Option<i32> = row.try_get("expires_turns")?;
            let keyword = parse_keyword(keyword_kind, keyword_amount, keyword_unit_type)?;
            let expires = parse_expire_contract(expires_kind, expires_turns)?;
            Ok(CardEffect::GrantKeyword(keyword, expires))
        }
        "discard" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for discard"))?;
            Ok(CardEffect::Discard(count as usize))
        }
        "discard_self_random" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for discard_self_random"))?;
            Ok(CardEffect::DiscardSelfRandom(count as usize))
        }
        "steal_mana" => {
            let mana_type: Option<String> = row.try_get("mana_type")?;
            let amount: Option<i32> = row.try_get("amount")?;
            let mana_type = mana_type.ok_or_else(|| anyhow!("Missing mana_type for steal_mana"))?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for steal_mana"))?;
            Ok(CardEffect::StealMana(parse_enum_value(&mana_type)?, amount))
        }
        "shuffle_library" => {
            let target_scope: Option<String> = row.try_get("target_scope")?;
            let target_scope = target_scope.ok_or_else(|| anyhow!("Missing target_scope"))?;
            Ok(CardEffect::ShuffleLibrary(parse_enum_value(&target_scope)?))
        }
        "forbid_name" => Ok(CardEffect::ForbidName),
        "fight" => Ok(CardEffect::Fight),
        "tap_target" => Ok(CardEffect::TapTarget),
        "untap_target" => Ok(CardEffect::UntapTarget),
        "destroy_target" => Ok(CardEffect::DestroyTarget),
        "exile_target" => Ok(CardEffect::ExileTarget),
        "equip" => Ok(CardEffect::Equip),
        "add_mana_scaling" => {
            let mana_type: Option<String> = row.try_get("mana_type")?;
            let scale_unit_type: Option<String> = row.try_get("scale_unit_type")?;
            let scale_card_kind: Option<String> = row.try_get("scale_card_kind")?;
            let mana_type =
                mana_type.ok_or_else(|| anyhow!("Missing mana_type for add_mana_scaling"))?;
            Ok(CardEffect::AddManaScaling(
                parse_enum_value(&mana_type)?,
                scale_unit_type
                    .map(|value| parse_enum_value(&value))
                    .transpose()?,
                scale_card_kind
                    .map(|value| parse_enum_value(&value))
                    .transpose()?,
            ))
        }
        "modify_loyalty" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for modify_loyalty"))?;
            Ok(CardEffect::ModifyLoyalty(amount))
        }
        "emblem" => {
            let keywords = emblem_keywords.cloned().unwrap_or_default();
            let stat = parse_stat_modifier_optional(row)?;
            Ok(CardEffect::Emblem(keywords, stat))
        }
        "mill" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for mill"))?;
            Ok(CardEffect::Mill(count as usize))
        }
        "bounce_all_attacking" => Ok(CardEffect::BounceAllAttacking),
        "bounce_all_units" => Ok(CardEffect::BounceAllUnits),
        "bounce_all_opponent_permanents" => Ok(CardEffect::BounceAllOpponentPermanents),
        "copy_unit" => Ok(CardEffect::CopyUnit),
        "search_graveyard" => {
            let predicate: Option<serde_json::Value> = row.try_get("effect_predicate")?;
            let predicate =
                predicate.ok_or_else(|| anyhow!("Missing predicate for search_graveyard"))?;
            Ok(CardEffect::SearchGraveyard(serde_json::from_value(
                predicate,
            )?))
        }
        "add_counters" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for add_counters"))?;
            Ok(CardEffect::AddCounters(count))
        }
        "sacrifice_this" => Ok(CardEffect::SacrificeThis),
        "reanimate_named_card" => {
            let name: Option<String> = row.try_get("token_name")?;
            let name =
                name.ok_or_else(|| anyhow!("Missing token_name for reanimate_named_card"))?;
            Ok(CardEffect::ReanimateNamedCard(name))
        }
        "remove_counters" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for remove_counters"))?;
            Ok(CardEffect::RemoveCounters(count))
        }
        "reanimate_target_unit" => Ok(CardEffect::ReanimateTargetUnit),
        "pump_by_mana" => Ok(CardEffect::PumpByMana),
        "siphon" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for siphon"))?;
            Ok(CardEffect::Siphon(amount))
        }
        "transmute_mana" => {
            let mana_type: Option<String> = row.try_get("mana_type")?;
            let to_type: Option<String> = row.try_get("token_name")?;
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for transmute_mana"))?;
            let from = mana_type.ok_or_else(|| anyhow!("Missing mana_type for transmute_mana"))?;
            let to = to_type.ok_or_else(|| anyhow!("Missing token_name for transmute_mana"))?;
            Ok(CardEffect::TransmuteMana(
                parse_enum_value(&from)?,
                parse_enum_value(&to)?,
                amount,
            ))
        }
        "swap" => Ok(CardEffect::Swap),
        "phase" => Ok(CardEffect::Phase),
        "mutate_target" => {
            let stat = parse_stat_modifier_optional(row)?;
            Ok(CardEffect::MutateTarget(stat.ok_or_else(|| {
                anyhow!("Missing stat modifier for mutate_target")
            })?))
        }
        "polymorph" => Ok(CardEffect::Polymorph),
        "surveil" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for surveil"))?;
            Ok(CardEffect::Surveil(count as usize))
        }
        "transmit" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for transmit"))?;
            Ok(CardEffect::Transmit(count as usize))
        }
        "entomb" => {
            let predicate: Option<serde_json::Value> = row.try_get("effect_predicate")?;
            let predicate = predicate.ok_or_else(|| anyhow!("Missing predicate for entomb"))?;
            Ok(CardEffect::Entomb(serde_json::from_value(predicate)?))
        }
        "recycle" => Ok(CardEffect::Recycle),
        "damage_split" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for damage_split"))?;
            Ok(CardEffect::DamageSplit(amount))
        }
        "damage_each_opponent_unit" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount =
                amount.ok_or_else(|| anyhow!("Missing amount for damage_each_opponent_unit"))?;
            Ok(CardEffect::DamageEachOpponentUnit(amount))
        }
        "damage_player_per_battlefield_kind" => {
            let scale_card_kind: Option<String> = row.try_get("scale_card_kind")?;
            let kind = scale_card_kind.ok_or_else(|| {
                anyhow!("Missing scale_card_kind for damage_player_per_battlefield_kind")
            })?;
            Ok(CardEffect::DamagePlayerPerBattlefieldKind(
                parse_enum_value(&kind)?,
            ))
        }
        "create_tokens_per_battlefield_kind" => {
            let token_name: Option<String> = row.try_get("token_name")?;
            let token_power: Option<i32> = row.try_get("token_power")?;
            let token_toughness: Option<i32> = row.try_get("token_toughness")?;
            let token_unit_type: Option<String> = row.try_get("token_unit_type")?;
            let scale_card_kind: Option<String> = row.try_get("scale_card_kind")?;
            let token = TokenSpec::new(
                token_name.ok_or_else(|| anyhow!("Missing token_name"))?,
                token_power.ok_or_else(|| anyhow!("Missing token_power"))?,
                token_toughness.ok_or_else(|| anyhow!("Missing token_toughness"))?,
                parse_enum_value(
                    &token_unit_type.ok_or_else(|| anyhow!("Missing token_unit_type"))?,
                )?,
            );
            let kind = scale_card_kind.ok_or_else(|| {
                anyhow!("Missing scale_card_kind for create_tokens_per_battlefield_kind")
            })?;
            Ok(CardEffect::CreateTokensPerBattlefieldKind(
                token,
                parse_enum_value(&kind)?,
            ))
        }
        "reflect" => {
            let amount: Option<i32> = row.try_get("amount")?;
            let amount = amount.ok_or_else(|| anyhow!("Missing amount for reflect"))?;
            Ok(CardEffect::Reflect(amount))
        }
        "conscript" => Ok(CardEffect::Conscript),
        "scavenge" => {
            let count: Option<i32> = row.try_get("count")?;
            let count = count.ok_or_else(|| anyhow!("Missing count for scavenge"))?;
            Ok(CardEffect::Scavenge(count))
        }
        "assimilate" => {
            let target_unit_type: Option<String> = row.try_get("target_unit_type")?;
            let target_unit_type = target_unit_type
                .ok_or_else(|| anyhow!("Missing target_unit_type for assimilate"))?;
            Ok(CardEffect::Assimilate(parse_enum_value(&target_unit_type)?))
        }
        "sequence" => Err(anyhow!(
            "Sequence effects should be flattened before storage"
        )),
        "apply_stat_modifiers" => Err(anyhow!(
            "ApplyStatModifiers should be flattened before storage"
        )),
        other => Err(anyhow!("Unsupported effect kind: {other}")),
    }
}

async fn insert_effect_rows(
    tx: &mut Transaction<'_, Postgres>,
    owner_kind: &str,
    card_id: Option<&str>,
    ability_id: Option<&str>,
    trigger_id: Option<&str>,
    effects: &[CardEffect],
) -> Result<()> {
    for (index, effect) in effects.iter().enumerate() {
        let effect_id = Ulid::new().to_string();
        let mut effect_kind = "";
        let mut mana_type: Option<String> = None;
        let mut amount: Option<i32> = None;
        let mut count: Option<i32> = None;
        let mut target_scope: Option<String> = None;
        let mut target_spec_kind: Option<String> = None;
        let mut target_unit_type: Option<String> = None;
        let mut scale_unit_type: Option<String> = None;
        let mut scale_card_kind: Option<String> = None;
        let mut keyword_kind: Option<String> = None;
        let mut keyword_amount: Option<i32> = None;
        let mut keyword_unit_type: Option<String> = None;
        let mut token_name: Option<String> = None;
        let mut token_power: Option<i32> = None;
        let mut token_toughness: Option<i32> = None;
        let mut token_unit_type: Option<String> = None;
        let mut stat_type: Option<String> = None;
        let mut stat_amount: Option<i32> = None;
        let mut stat_amount_dynamic: Option<serde_json::Value> = None;
        let mut expires_kind: Option<String> = None;
        let mut expires_turns: Option<i32> = None;
        let mut effect_predicate: Option<serde_json::Value> = None;
        let mut emblem_keywords: Option<Vec<Keyword>> = None;

        let mut set_stat_fields = |spec: &StatModifierSpec| -> Result<()> {
            // Only allow database-supported stat types (Power, Toughness, Both)
            // Runtime-only types (Armor, Shield, Deathtouch, Trample, etc.) should not be persisted
            use crate::types::StatType;
            match spec.stat {
                StatType::Power | StatType::Toughness | StatType::Both => {
                    stat_type = Some(enum_to_string(&spec.stat)?);
                    match &spec.amount {
                        ModifierAmount::Fixed(value) => {
                            stat_amount = Some(*value);
                            stat_amount_dynamic = None;
                        }
                        dynamic => {
                            stat_amount = None;
                            stat_amount_dynamic = Some(serde_json::to_value(dynamic)?);
                        }
                    }
                    let (expires, turns) = split_expire_contract(&spec.expires)?;
                    expires_kind = Some(expires);
                    expires_turns = turns;
                    let (target_kind, unit_type) = split_effect_target_spec(&spec.target)?;
                    target_spec_kind = Some(target_kind);
                    target_unit_type = unit_type;
                    Ok(())
                }
                _ => Err(anyhow!(
                    "Cannot persist runtime-only StatType {:?}. Use CardEffect::GrantKeyword instead.",
                    spec.stat
                )),
            }
        };

        match effect {
            CardEffect::RememberTargetPlayer => {
                effect_kind = "remember_target_player";
            }
            CardEffect::DamagePlayer(value) => {
                effect_kind = "damage_player";
                amount = Some(*value);
            }
            CardEffect::DamagePlayerPerHandCard => {
                effect_kind = "damage_player_per_hand_card";
            }
            CardEffect::DamagePlayerPerBattlefieldKind(kind) => {
                effect_kind = "damage_player_per_battlefield_kind";
                scale_card_kind = Some(enum_to_string(kind)?);
            }
            CardEffect::CreateTokensPerBattlefieldKind(token, kind) => {
                effect_kind = "create_tokens_per_battlefield_kind";
                token_name = Some(token.name.clone());
                token_power = Some(token.power);
                token_toughness = Some(token.toughness);
                token_unit_type = Some(enum_to_string(&token.unit_type)?);
                scale_card_kind = Some(enum_to_string(kind)?);
            }
            CardEffect::DamageUnit(value) => {
                effect_kind = "damage_unit";
                amount = Some(*value);
            }
            CardEffect::DamageTarget(value) => {
                effect_kind = "damage_target";
                amount = Some(*value);
            }
            CardEffect::DrawCards(value) => {
                effect_kind = "draw_cards";
                count = Some(*value as i32);
            }
            CardEffect::Heat(value) => {
                effect_kind = "heat";
                amount = Some(*value);
            }
            CardEffect::HeatOpponents(value) => {
                effect_kind = "heat_opponents";
                amount = Some(*value);
            }
            CardEffect::CashOpponents(value) => {
                effect_kind = "cash_opponents";
                amount = Some(*value);
            }
            CardEffect::GainLife(value) => {
                effect_kind = "gain_life";
                amount = Some(*value);
            }
            CardEffect::CounterTop => {
                effect_kind = "counter_top";
            }
            CardEffect::AddMana(mana, value) => {
                effect_kind = "add_mana";
                mana_type = Some(enum_to_string(mana)?);
                amount = Some(*value);
            }
            CardEffect::CreateToken(token) => {
                effect_kind = "create_token";
                token_name = Some(token.name.clone());
                token_power = Some(token.power);
                token_toughness = Some(token.toughness);
                token_unit_type = Some(enum_to_string(&token.unit_type)?);
            }
            CardEffect::CreateInfrastructure(spec) => {
                effect_kind = "create_infrastructure";
                effect_predicate = Some(serde_json::to_value(spec)?);
            }
            CardEffect::ApplyStatModifier(spec) => {
                effect_kind = "apply_stat_modifier";
                set_stat_fields(spec)?;
            }
            CardEffect::ApplyStatModifiers(_) => {
                return Err(anyhow!(
                    "ApplyStatModifiers must be flattened before storage"
                ));
            }
            CardEffect::Sequence(_) => {
                return Err(anyhow!("Sequence must be flattened before storage"));
            }
            CardEffect::RevealTopIf {
                predicate,
                on_match,
                on_fail,
            } => {
                effect_kind = "reveal_top_if";
                effect_predicate = Some(serde_json::json!({
                    "predicate": predicate,
                    "on_match": on_match,
                    "on_fail": on_fail,
                }));
            }
            CardEffect::Freeze(expires) => {
                effect_kind = "freeze";
                let (expires, turns) = split_expire_contract(expires)?;
                expires_kind = Some(expires);
                expires_turns = turns;
            }
            CardEffect::RefundMana(value) => {
                effect_kind = "refund_mana";
                amount = Some(*value);
            }
            CardEffect::Loot(draw, discard) => {
                effect_kind = "loot";
                amount = Some(*draw as i32);
                count = Some(*discard as i32);
            }
            CardEffect::Wheel(draw_count) => {
                effect_kind = "wheel";
                count = Some(*draw_count as i32);
            }
            CardEffect::Scry(value) => {
                effect_kind = "scry";
                count = Some(*value as i32);
            }
            CardEffect::Search(predicate) => {
                effect_kind = "search";
                effect_predicate = Some(serde_json::to_value(predicate)?);
            }
            CardEffect::SearchToBattlefield(predicate) => {
                effect_kind = "search_to_battlefield";
                effect_predicate = Some(serde_json::to_value(predicate)?);
            }
            CardEffect::Bounce => {
                effect_kind = "bounce";
            }
            CardEffect::BoardWipe => {
                effect_kind = "board_wipe";
            }
            CardEffect::DestroyTargetPlayerUnits => {
                effect_kind = "destroy_target_player_units";
            }
            CardEffect::DestroyUnitsWithToughnessAtLeast(min_toughness) => {
                effect_kind = "destroy_units_with_toughness_at_least";
                amount = Some(*min_toughness);
            }
            CardEffect::CopyLastCast => {
                effect_kind = "copy_last_cast";
            }
            CardEffect::Purify => {
                effect_kind = "purify";
            }
            CardEffect::GrantKeyword(keyword, expires) => {
                effect_kind = "grant_keyword";
                let (kind, amount_value, unit_type) = split_keyword(keyword)?;
                keyword_kind = Some(kind);
                keyword_amount = amount_value;
                keyword_unit_type = unit_type;
                let (expires, turns) = split_expire_contract(expires)?;
                expires_kind = Some(expires);
                expires_turns = turns;
            }
            CardEffect::Discard(value) => {
                effect_kind = "discard";
                count = Some(*value as i32);
            }
            CardEffect::DiscardSelfRandom(value) => {
                effect_kind = "discard_self_random";
                count = Some(*value as i32);
            }
            CardEffect::StealMana(mana, value) => {
                effect_kind = "steal_mana";
                mana_type = Some(enum_to_string(mana)?);
                amount = Some(*value);
            }
            CardEffect::ShuffleLibrary(scope) => {
                effect_kind = "shuffle_library";
                target_scope = Some(enum_to_string(scope)?);
            }
            CardEffect::ForbidName => {
                effect_kind = "forbid_name";
            }
            CardEffect::Fight => {
                effect_kind = "fight";
            }
            CardEffect::TapTarget => {
                effect_kind = "tap_target";
            }
            CardEffect::UntapTarget => {
                effect_kind = "untap_target";
            }
            CardEffect::DestroyTarget => {
                effect_kind = "destroy_target";
            }
            CardEffect::ExileTarget => {
                effect_kind = "exile_target";
            }
            CardEffect::Equip => {
                effect_kind = "equip";
            }
            CardEffect::AddManaScaling(mana, unit_type, card_kind) => {
                effect_kind = "add_mana_scaling";
                mana_type = Some(enum_to_string(mana)?);
                scale_unit_type = unit_type.as_ref().map(enum_to_string).transpose()?;
                scale_card_kind = card_kind.as_ref().map(enum_to_string).transpose()?;
            }
            CardEffect::ModifyLoyalty(value) => {
                effect_kind = "modify_loyalty";
                amount = Some(*value);
            }
            CardEffect::Emblem(keywords, stat) => {
                effect_kind = "emblem";
                if let Some(stat) = stat.as_ref() {
                    set_stat_fields(stat)?;
                }
                emblem_keywords = Some(keywords.clone());
            }
            CardEffect::SacrificeThis => {
                effect_kind = "sacrifice_this";
            }
            CardEffect::ReanimateNamedCard(name) => {
                effect_kind = "reanimate_named_card";
                token_name = Some(name.clone()); // Reusing token_name field for card name
            }
            CardEffect::CopyUnit => {
                effect_kind = "copy_unit";
            }
            CardEffect::Mill(value) => {
                effect_kind = "mill";
                count = Some(*value as i32);
            }
            CardEffect::BounceAllUnits => {
                effect_kind = "bounce_all_units";
            }
            CardEffect::BounceAllOpponentPermanents => {
                effect_kind = "bounce_all_opponent_permanents";
            }
            CardEffect::BounceAllAttacking => {
                effect_kind = "bounce_all_attacking";
            }
            CardEffect::SearchGraveyard(predicate) => {
                effect_kind = "search_graveyard";
                effect_predicate = Some(serde_json::to_value(predicate)?);
            }
            CardEffect::ReanimateTargetUnit => {
                effect_kind = "reanimate_target_unit";
            }
            CardEffect::AddCounters(value) => {
                effect_kind = "add_counters";
                count = Some(*value);
            }
            CardEffect::RemoveCounters(value) => {
                effect_kind = "remove_counters";
                count = Some(*value);
            }
            CardEffect::PumpByMana => {
                effect_kind = "pump_by_mana";
            }
            // New effects
            CardEffect::Siphon(value) => {
                effect_kind = "siphon";
                amount = Some(*value);
            }
            CardEffect::TransmuteMana(from, to, value) => {
                effect_kind = "transmute_mana";
                mana_type = Some(enum_to_string(from)?);
                token_name = Some(enum_to_string(to)?);
                amount = Some(*value);
            }
            CardEffect::Swap => {
                effect_kind = "swap";
            }
            CardEffect::Phase => {
                effect_kind = "phase";
            }
            CardEffect::MutateTarget(spec) => {
                effect_kind = "mutate_target";
                set_stat_fields(spec)?;
            }
            CardEffect::Polymorph => {
                effect_kind = "polymorph";
            }
            CardEffect::Surveil(value) => {
                effect_kind = "surveil";
                count = Some(*value as i32);
            }
            CardEffect::Transmit(value) => {
                effect_kind = "transmit";
                count = Some(*value as i32);
            }
            CardEffect::Entomb(predicate) => {
                effect_kind = "entomb";
                effect_predicate = Some(serde_json::to_value(predicate)?);
            }
            CardEffect::Recycle => {
                effect_kind = "recycle";
            }
            CardEffect::DamageSplit(value) => {
                effect_kind = "damage_split";
                amount = Some(*value);
            }
            CardEffect::DamageEachOpponentUnit(value) => {
                effect_kind = "damage_each_opponent_unit";
                amount = Some(*value);
            }
            CardEffect::Reflect(value) => {
                effect_kind = "reflect";
                amount = Some(*value);
            }
            CardEffect::Conscript => {
                effect_kind = "conscript";
            }
            CardEffect::Scavenge(value) => {
                effect_kind = "scavenge";
                count = Some(*value);
            }
            CardEffect::Assimilate(unit_type) => {
                effect_kind = "assimilate";
                target_unit_type = Some(enum_to_string(unit_type)?);
            }
        };

        sqlx::query(
            r#"
            INSERT INTO card_effects (
                effect_id,
                owner_kind,
                card_id,
                ability_id,
                trigger_id,
                effect_kind,
                mana_type,
                amount,
                count,
                target_scope,
                target_spec_kind,
                target_unit_type,
                scale_unit_type,
                scale_card_kind,
                keyword_kind,
                keyword_amount,
                keyword_unit_type,
                token_name,
                token_power,
                token_toughness,
                token_unit_type,
                stat_type,
                stat_amount,
                stat_amount_dynamic,
                expires_kind,
                expires_turns,
                effect_predicate,
                sort_order
            ) VALUES (
                $1,
                $2::effect_owner_kind,
                $3,
                $4,
                $5,
                $6::card_effect_kind,
                $7::mana_type,
                $8,
                $9,
                $10::target_scope,
                $11::effect_target_spec_kind,
                $12::unit_type,
                $13::unit_type,
                $14::card_kind,
                $15::keyword_kind,
                $16,
                $17::unit_type,
                $18,
                $19,
                $20,
                $21::unit_type,
                $22::stat_type,
                $23,
                $24,
                $25::expire_contract,
                $26,
                $27,
                $28
            )
            "#,
        )
        .bind(&effect_id)
        .bind(owner_kind)
        .bind(card_id)
        .bind(ability_id)
        .bind(trigger_id)
        .bind(effect_kind)
        .bind(mana_type.as_deref())
        .bind(amount)
        .bind(count)
        .bind(target_scope.as_deref())
        .bind(target_spec_kind.as_deref())
        .bind(target_unit_type.as_deref())
        .bind(scale_unit_type.as_deref())
        .bind(scale_card_kind.as_deref())
        .bind(keyword_kind.as_deref())
        .bind(keyword_amount)
        .bind(keyword_unit_type.as_deref())
        .bind(token_name.as_deref())
        .bind(token_power)
        .bind(token_toughness)
        .bind(token_unit_type.as_deref())
        .bind(stat_type.as_deref())
        .bind(stat_amount)
        .bind(stat_amount_dynamic)
        .bind(expires_kind.as_deref())
        .bind(expires_turns)
        .bind(effect_predicate)
        .bind(index as i32)
        .execute(&mut **tx)
        .await?;

        if let Some(keywords) = emblem_keywords.as_ref() {
            for (keyword_index, keyword) in keywords.iter().enumerate() {
                let (kind, amount_value, unit_type) = split_keyword(keyword)?;
                sqlx::query(
                    r#"
                    INSERT INTO card_effect_keywords (
                        effect_id,
                        keyword_kind,
                        keyword_amount,
                        keyword_unit_type,
                        sort_order
                    ) VALUES (
                        $1,
                        $2::keyword_kind,
                        $3,
                        $4::unit_type,
                        $5
                    )
                    "#,
                )
                .bind(&effect_id)
                .bind(&kind)
                .bind(amount_value)
                .bind(unit_type.as_deref())
                .bind(keyword_index as i32)
                .execute(&mut **tx)
                .await?;
            }
        }
    }
    Ok(())
}

impl CardRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> Arc<PgPool> {
        self.pool.clone()
    }

    pub async fn ensure_decks_seeded(&self) -> Result<Vec<DeckRecord>> {
        let existing = self.list_base_decks().await?;
        let catalog = build_deck_catalog();
        let existing_ids: HashSet<String> = existing.iter().map(|deck| deck.id.clone()).collect();
        let missing: Vec<DeckCatalog> = catalog
            .into_iter()
            .filter(|deck| !existing_ids.contains(&deck.id))
            .collect();

        if !missing.is_empty() {
            self.seed_deck_catalog(&missing).await?;
        }

        self.list_base_decks().await
    }

    pub async fn list_base_decks(&self) -> Result<Vec<DeckRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                deck_id,
                name,
                description,
                owner_id,
                base_deck_id,
                format::text AS format,
                operator_card_id,
                image_url
            FROM decks
            WHERE owner_id IS NULL
            ORDER BY deck_id
            "#,
        )
        .fetch_all(&*self.pool)
        .await?;

        let mut out = Vec::new();
        for row in rows {
            out.push(DeckRecord {
                id: row.try_get("deck_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                owner_id: row.try_get("owner_id")?,
                base_deck_id: row.try_get("base_deck_id")?,
                format: parse_enum_value(&row.try_get::<String, _>("format")?)?,
                operator_card_id: row.try_get("operator_card_id")?,
                image_url: row.try_get("image_url")?,
            });
        }
        Ok(out)
    }

    pub async fn list_decks_for_user(&self, owner_id: &str) -> Result<Vec<DeckRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                deck_id,
                name,
                description,
                owner_id,
                base_deck_id,
                format::text AS format,
                operator_card_id,
                image_url
            FROM decks
            WHERE owner_id IS NULL OR (owner_id = $1 AND base_deck_id IS NULL)
            ORDER BY owner_id NULLS FIRST, deck_id
            "#,
        )
        .bind(owner_id)
        .fetch_all(&*self.pool)
        .await?;

        let mut out = Vec::new();
        for row in rows {
            out.push(DeckRecord {
                id: row.try_get("deck_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                owner_id: row.try_get("owner_id")?,
                base_deck_id: row.try_get("base_deck_id")?,
                format: parse_enum_value(&row.try_get::<String, _>("format")?)?,
                operator_card_id: row.try_get("operator_card_id")?,
                image_url: row.try_get("image_url")?,
            });
        }
        Ok(out)
    }

    pub async fn get_user_deck_for_base(
        &self,
        owner_id: &str,
        base_deck_id: &str,
    ) -> Result<Option<DeckRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                deck_id,
                name,
                description,
                owner_id,
                base_deck_id,
                format::text AS format,
                operator_card_id,
                image_url
            FROM decks
            WHERE owner_id = $1 AND base_deck_id = $2
            "#,
        )
        .bind(owner_id)
        .bind(base_deck_id)
        .fetch_optional(&*self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DeckRecord {
                id: row.try_get("deck_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                owner_id: row.try_get("owner_id")?,
                base_deck_id: row.try_get("base_deck_id")?,
                format: parse_enum_value(&row.try_get::<String, _>("format")?)?,
                operator_card_id: row.try_get("operator_card_id")?,
                image_url: row.try_get("image_url")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_deck(&self, deck_id: &str) -> Result<DeckRecord> {
        let row = sqlx::query(
            r#"
            SELECT
                deck_id,
                name,
                description,
                owner_id,
                base_deck_id,
                format::text AS format,
                operator_card_id,
                image_url
            FROM decks
            WHERE deck_id = $1
            "#,
        )
        .bind(deck_id)
        .fetch_optional(&*self.pool)
        .await?;

        match row {
            Some(row) => Ok(DeckRecord {
                id: row.try_get("deck_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                owner_id: row.try_get("owner_id")?,
                base_deck_id: row.try_get("base_deck_id")?,
                format: parse_enum_value(&row.try_get::<String, _>("format")?)?,
                operator_card_id: row.try_get("operator_card_id")?,
                image_url: row.try_get("image_url")?,
            }),
            None => Err(anyhow!("Deck not found: {}", deck_id)),
        }
    }

    pub async fn clone_deck(
        &self,
        source_deck_id: &str,
        new_deck_id: &str,
        name: &str,
        description: &str,
    ) -> Result<DeckRecord> {
        self.clone_deck_with_owner(source_deck_id, new_deck_id, name, description, None, None)
            .await
    }

    pub async fn clone_deck_with_owner(
        &self,
        source_deck_id: &str,
        new_deck_id: &str,
        name: &str,
        description: &str,
        owner_id: Option<&str>,
        base_deck_id: Option<&str>,
    ) -> Result<DeckRecord> {
        let mut tx = self.pool.begin().await?;
        let source_row = sqlx::query(
            r#"
            SELECT format::text AS format, operator_card_id
            FROM decks
            WHERE deck_id = $1
            "#,
        )
        .bind(source_deck_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (format, operator_card_id) = match source_row {
            Some(row) => (
                parse_enum_value(&row.try_get::<String, _>("format")?)?,
                row.try_get::<Option<String>, _>("operator_card_id")?,
            ),
            None => {
                return Err(anyhow!("Deck not found: {}", source_deck_id));
            }
        };
        sqlx::query(
            r#"
            INSERT INTO decks (deck_id, name, description, owner_id, base_deck_id, format, operator_card_id)
            VALUES ($1, $2, $3, $4, $5, $6::deck_format, $7)
            "#,
        )
        .bind(new_deck_id)
        .bind(name)
        .bind(description)
        .bind(owner_id)
        .bind(base_deck_id)
        .bind(enum_to_string(&format)?)
        .bind(&operator_card_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO deck_cards (
                deck_id,
                card_id,
                quantity,
                sort_order,
                is_sideboard
            )
            SELECT
                $1,
                card_id,
                quantity,
                sort_order,
                is_sideboard
            FROM deck_cards
            WHERE deck_id = $2
            "#,
        )
        .bind(new_deck_id)
        .bind(source_deck_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(DeckRecord {
            id: new_deck_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            owner_id: owner_id.map(|value| value.to_string()),
            base_deck_id: base_deck_id.map(|value| value.to_string()),
            format,
            operator_card_id,
            image_url: None,
        })
    }

    pub async fn create_deck(
        &self,
        deck_id: &str,
        name: &str,
        description: &str,
    ) -> Result<DeckRecord> {
        self.create_deck_with_owner(deck_id, name, description, None, None)
            .await
    }

    pub async fn create_deck_with_owner(
        &self,
        deck_id: &str,
        name: &str,
        description: &str,
        owner_id: Option<&str>,
        base_deck_id: Option<&str>,
    ) -> Result<DeckRecord> {
        sqlx::query(
            r#"
            INSERT INTO decks (deck_id, name, description, owner_id, base_deck_id, format, operator_card_id)
            VALUES ($1, $2, $3, $4, $5, $6::deck_format, $7)
            ON CONFLICT (deck_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                owner_id = EXCLUDED.owner_id,
                base_deck_id = EXCLUDED.base_deck_id,
                updated_at = NOW()
            "#,
        )
        .bind(deck_id)
        .bind(name)
        .bind(description)
        .bind(owner_id)
        .bind(base_deck_id)
        .bind(enum_to_string(&DeckFormat::Classic)?)
        .bind::<Option<&str>>(None)
        .execute(&*self.pool)
        .await?;

        Ok(DeckRecord {
            id: deck_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            owner_id: owner_id.map(|value| value.to_string()),
            base_deck_id: base_deck_id.map(|value| value.to_string()),
            format: DeckFormat::Classic,
            operator_card_id: None,
            image_url: None,
        })
    }

    pub async fn update_deck_image_url(&self, deck_id: &str, image_url: &str) -> Result<()> {
        sqlx::query("UPDATE decks SET image_url = $1, updated_at = NOW() WHERE deck_id = $2")
            .bind(image_url)
            .bind(deck_id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn adjust_deck_card_quantity(
        &self,
        deck_id: &str,
        card_id: &str,
        delta: i32,
    ) -> Result<()> {
        self.adjust_deck_card_quantity_with_zone(deck_id, card_id, delta, false)
            .await
    }

    /// Like `adjust_deck_card_quantity` but targets either the main deck
    /// (is_sideboard = false) or the sideboard pool (is_sideboard = true).
    pub async fn adjust_deck_card_quantity_with_zone(
        &self,
        deck_id: &str,
        card_id: &str,
        delta: i32,
        is_sideboard: bool,
    ) -> Result<()> {
        if delta == 0 {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            r#"
            SELECT quantity
            FROM deck_cards
            WHERE deck_id = $1 AND card_id = $2 AND is_sideboard = $3
            "#,
        )
        .bind(deck_id)
        .bind(card_id)
        .bind(is_sideboard)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = row {
            let quantity: i32 = row.try_get("quantity")?;
            let next_quantity = quantity + delta;
            if next_quantity <= 0 {
                sqlx::query(
                    r#"
                    DELETE FROM deck_cards
                    WHERE deck_id = $1 AND card_id = $2 AND is_sideboard = $3
                    "#,
                )
                .bind(deck_id)
                .bind(card_id)
                .bind(is_sideboard)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    r#"
                    UPDATE deck_cards
                    SET quantity = $4
                    WHERE deck_id = $1 AND card_id = $2 AND is_sideboard = $3
                    "#,
                )
                .bind(deck_id)
                .bind(card_id)
                .bind(is_sideboard)
                .bind(next_quantity)
                .execute(&mut *tx)
                .await?;
            }
        } else if delta > 0 {
            let row = sqlx::query(
                r#"
                SELECT COALESCE(MAX(sort_order), 0) + 1 AS next_sort_order
                FROM deck_cards
                WHERE deck_id = $1 AND is_sideboard = $2
                "#,
            )
            .bind(deck_id)
            .bind(is_sideboard)
            .fetch_one(&mut *tx)
            .await?;
            let sort_order: i32 = row.try_get("next_sort_order")?;

            sqlx::query(
                r#"
                INSERT INTO deck_cards (deck_id, card_id, quantity, sort_order, is_sideboard)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(deck_id)
            .bind(card_id)
            .bind(delta)
            .bind(sort_order)
            .bind(is_sideboard)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("UPDATE decks SET updated_at = NOW() WHERE deck_id = $1")
            .bind(deck_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn set_deck_operator(
        &self,
        deck_id: &str,
        operator_card_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE decks
            SET operator_card_id = $2,
                updated_at = NOW()
            WHERE deck_id = $1
            "#,
        )
        .bind(deck_id)
        .bind(operator_card_id)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_deck_format(&self, deck_id: &str, format: DeckFormat) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE decks
            SET format = $2::deck_format,
                updated_at = NOW()
            WHERE deck_id = $1
            "#,
        )
        .bind(deck_id)
        .bind(enum_to_string(&format)?)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_deck_templates(&self, deck_id: &str) -> Result<Vec<DeckTemplateEntry>> {
        self.load_deck_templates_zone(deck_id, false).await
    }

    pub async fn load_sideboard_templates(&self, deck_id: &str) -> Result<Vec<DeckTemplateEntry>> {
        self.load_deck_templates_zone(deck_id, true).await
    }

    async fn load_deck_templates_zone(
        &self,
        deck_id: &str,
        is_sideboard: bool,
    ) -> Result<Vec<DeckTemplateEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT
                c.card_id,
                c.name,
                c.description,
                c.kind::text AS kind,
                c.supertype::text AS supertype,
                c.unit_type::text AS unit_type,
                c.artifact_type::text AS artifact_type,
                c.cost_neon,
                c.cost_glitch,
                c.cost_chrome,
                c.cost_slag,
                c.cost_bio,
                c.cost_colorless,
                c.power,
                c.toughness,
                c.target_spec,
                c.attachment,
                c.tapped,
                c.enters_tapped,
                c.keywords,
                c.image_url,
                c.colors::text[] AS colors,
                c.loyalty,
                c.rarity::text AS rarity,
                c.categories,
                dc.quantity
            FROM deck_cards dc
            JOIN cards c ON c.card_id = dc.card_id
            WHERE dc.deck_id = $1 AND dc.is_sideboard = $2
            ORDER BY dc.sort_order, c.card_id
            "#,
        )
        .bind(deck_id)
        .bind(is_sideboard)
        .fetch_all(&*self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut cards = Vec::new();
        let mut quantities = Vec::new();
        for row in rows {
            cards.push(card_from_row(&row)?);
            quantities.push(row.try_get::<i32, _>("quantity")?);
        }

        let cards = self.hydrate_cards(cards).await?;
        let mut out = Vec::with_capacity(cards.len());
        for (card, quantity) in cards.into_iter().zip(quantities) {
            out.push(DeckTemplateEntry { card, quantity });
        }
        Ok(out)
    }

    pub async fn replace_deck_templates(
        &self,
        deck_id: &str,
        templates: &[DeckTemplateEntry],
    ) -> Result<()> {
        self.replace_deck_templates_with_sideboard(deck_id, templates, &[])
            .await
    }

    pub async fn replace_deck_templates_with_sideboard(
        &self,
        deck_id: &str,
        main: &[DeckTemplateEntry],
        sideboard: &[DeckTemplateEntry],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM deck_cards WHERE deck_id = $1")
            .bind(deck_id)
            .execute(&mut *tx)
            .await?;

        for (index, entry) in main.iter().enumerate() {
            if entry.quantity <= 0 {
                continue;
            }

            sqlx::query(
                r#"
                INSERT INTO deck_cards (deck_id, card_id, quantity, sort_order, is_sideboard)
                VALUES ($1, $2, $3, $4, FALSE)
                "#,
            )
            .bind(deck_id)
            .bind(&entry.card.id)
            .bind(entry.quantity)
            .bind(index as i32)
            .execute(&mut *tx)
            .await?;
        }

        for (index, entry) in sideboard.iter().enumerate() {
            if entry.quantity <= 0 {
                continue;
            }

            sqlx::query(
                r#"
                INSERT INTO deck_cards (deck_id, card_id, quantity, sort_order, is_sideboard)
                VALUES ($1, $2, $3, $4, TRUE)
                "#,
            )
            .bind(deck_id)
            .bind(&entry.card.id)
            .bind(entry.quantity)
            .bind(index as i32)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("UPDATE decks SET updated_at = NOW() WHERE deck_id = $1")
            .bind(deck_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn seed_deck_catalog(&self, catalog: &[DeckCatalog]) -> Result<()> {
        let mut unique_cards: std::collections::HashMap<String, Card> =
            std::collections::HashMap::new();
        for deck in catalog {
            for entry in &deck.cards {
                unique_cards
                    .entry(entry.card.id.clone())
                    .or_insert_with(|| entry.card.clone());
            }
        }

        for card in unique_cards.values() {
            self.create_card(card).await?;
        }

        // Note: stale card cleanup is handled by ensure_all_catalog_cards_seeded
        // which runs after this and has the complete picture of all catalog cards.

        for deck in catalog {
            sqlx::query(
                r#"
                INSERT INTO decks (deck_id, name, description, format, operator_card_id)
                VALUES ($1, $2, $3, $4::deck_format, $5)
                ON CONFLICT (deck_id) DO UPDATE SET
                    name = EXCLUDED.name,
                    description = EXCLUDED.description,
                    format = EXCLUDED.format,
                    operator_card_id = EXCLUDED.operator_card_id,
                    updated_at = NOW()
                "#,
            )
            .bind(&deck.id)
            .bind(&deck.name)
            .bind(&deck.description)
            .bind(enum_to_string(&DeckFormat::Classic)?)
            .bind::<Option<&str>>(None)
            .execute(&*self.pool)
            .await?;

            let mut tx = self.pool.begin().await?;
            sqlx::query("DELETE FROM deck_cards WHERE deck_id = $1")
                .bind(&deck.id)
                .execute(&mut *tx)
                .await?;

            for (index, entry) in deck.cards.iter().enumerate() {
                sqlx::query(
                    r#"
                    INSERT INTO deck_cards (
                        deck_id,
                        card_id,
                        quantity,
                        sort_order,
                        is_sideboard
                    ) VALUES (
                        $1,
                        $2,
                        $3,
                        $4,
                        FALSE
                    )
                    ON CONFLICT (deck_id, card_id, is_sideboard) DO UPDATE SET
                        quantity = EXCLUDED.quantity,
                        sort_order = EXCLUDED.sort_order
                    "#,
                )
                .bind(&deck.id)
                .bind(&entry.card.id)
                .bind(entry.quantity)
                .bind(index as i32)
                .execute(&mut *tx)
                .await?;
            }

            // Default sideboard for this stock deck (may be empty).
            let default_sideboard =
                crate::deck::classic_default_sideboard_card_ids(deck.id.as_str());
            for (index, (card_id, quantity)) in default_sideboard.iter().enumerate() {
                sqlx::query(
                    r#"
                    INSERT INTO deck_cards (
                        deck_id,
                        card_id,
                        quantity,
                        sort_order,
                        is_sideboard
                    ) VALUES (
                        $1,
                        $2,
                        $3,
                        $4,
                        TRUE
                    )
                    ON CONFLICT (deck_id, card_id, is_sideboard) DO UPDATE SET
                        quantity = EXCLUDED.quantity,
                        sort_order = EXCLUDED.sort_order
                    "#,
                )
                .bind(&deck.id)
                .bind(card_id)
                .bind(*quantity)
                .bind(index as i32)
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
        }

        Ok(())
    }

    pub async fn list_enum_values(&self, enum_name: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT e.enumlabel AS value
            FROM pg_type t
            JOIN pg_enum e ON t.oid = e.enumtypid
            WHERE t.typname = $1
            ORDER BY e.enumsortorder
            "#,
        )
        .bind(enum_name)
        .fetch_all(&*self.pool)
        .await?;

        let mut values = Vec::with_capacity(rows.len());
        for row in rows {
            values.push(row.try_get::<String, _>("value")?);
        }

        if values.is_empty() {
            return Err(anyhow!("enum {enum_name} not found or has no values"));
        }

        Ok(values)
    }

    pub async fn create_card(&self, card: &Card) -> Result<()> {
        let (kind, unit_type, artifact_type) = split_card_kind(&card.kind)?;
        let color_identities = card_color_identities(card);
        let colors_for_storage = color_identities
            .into_iter()
            .map(|mana| enum_to_string(&mana))
            .collect::<Result<Vec<_>>>()?;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO cards (
                card_id, name, description, kind, supertype, unit_type, artifact_type,
                cost_neon, cost_glitch, cost_chrome, cost_slag, cost_bio, cost_colorless,
                power, toughness, target_spec, attachment, tapped, enters_tapped,
                keywords, image_url, colors, loyalty, rarity, categories
            ) VALUES (
                $1, $2, $3, $4::card_kind, $5::card_supertype, $6::unit_type, $7::artifact_type,
                $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19,
                $20, $21, $22::mana_type[], $23, $24::card_rarity, $25
            )
            ON CONFLICT (card_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                kind = EXCLUDED.kind,
                supertype = EXCLUDED.supertype,
                unit_type = EXCLUDED.unit_type,
                artifact_type = EXCLUDED.artifact_type,
                cost_neon = EXCLUDED.cost_neon,
                cost_glitch = EXCLUDED.cost_glitch,
                cost_chrome = EXCLUDED.cost_chrome,
                cost_slag = EXCLUDED.cost_slag,
                cost_bio = EXCLUDED.cost_bio,
                cost_colorless = EXCLUDED.cost_colorless,
                power = EXCLUDED.power,
                toughness = EXCLUDED.toughness,
                target_spec = EXCLUDED.target_spec,
                attachment = EXCLUDED.attachment,
                tapped = EXCLUDED.tapped,
                enters_tapped = EXCLUDED.enters_tapped,
                keywords = EXCLUDED.keywords,
                image_url = EXCLUDED.image_url,
                colors = EXCLUDED.colors,
                loyalty = EXCLUDED.loyalty,
                rarity = EXCLUDED.rarity,
                categories = EXCLUDED.categories,
                updated_at = NOW()
            "#,
        )
        .bind(&card.id)
        .bind(&card.name)
        .bind(&card.description)
        .bind(&kind)
        .bind(enum_to_string(&card.supertype)?)
        .bind(unit_type.as_deref())
        .bind(artifact_type.as_deref())
        .bind(card.cost.neon)
        .bind(card.cost.glitch)
        .bind(card.cost.chrome)
        .bind(card.cost.slag)
        .bind(card.cost.bio)
        .bind(card.cost.colorless)
        .bind(card.power)
        .bind(card.toughness)
        .bind(None::<serde_json::Value>) // target_spec removed - now in abilities
        .bind(
            card.attachment
                .as_ref()
                .map(|a| serde_json::to_value(a))
                .transpose()?,
        )
        .bind(card.tapped)
        .bind(card.enters_tapped)
        .bind(serde_json::to_value(&card.keywords)?)
        .bind(&card.image_url)
        .bind(&colors_for_storage)
        .bind(card.loyalty)
        .bind(enum_to_string(&card.rarity)?)
        .bind(
            card.categories
                .iter()
                .map(|c| enum_to_string(c))
                .collect::<Result<Vec<_>>>()?,
        )
        .execute(&mut *tx)
        .await?;

        // Delete effects first (no CASCADE on ability_id/trigger_id foreign keys)
        sqlx::query("DELETE FROM card_effects WHERE ability_id IN (SELECT ability_id FROM card_abilities WHERE card_id = $1)")
            .bind(&card.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM card_effects WHERE trigger_id IN (SELECT trigger_id FROM card_triggers WHERE card_id = $1)")
            .bind(&card.id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM card_abilities WHERE card_id = $1")
            .bind(&card.id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM card_triggers WHERE card_id = $1")
            .bind(&card.id)
            .execute(&mut *tx)
            .await?;

        for (index, ability) in card.abilities.iter().enumerate() {
            let (target_scope, target_kind, target_max, target_optional) =
                if let Some(spec) = ability.target_spec.as_ref() {
                    (
                        Some(enum_to_string(&spec.scope)?),
                        Some(enum_to_string(&spec.kind)?),
                        Some(spec.max_targets as i32),
                        spec.optional,
                    )
                } else {
                    (None, None, None, false)
                };

            sqlx::query(
                r#"
                INSERT INTO card_abilities (
                    ability_id,
                    card_id,
                    name,
                    cost,
                    target_scope,
                    target_kind,
                    target_max_targets,
                    target_optional,
                    once_per_turn,
                    uses_stack,
                    tap_targets,
                    predicate,
                    sort_order
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5::target_scope,
                    $6::target_kind,
                    $7,
                    $8,
                    $9,
                    $10,
                    $11,
                    $12,
                    $13
                )
                ON CONFLICT (ability_id) DO UPDATE SET
                    card_id = EXCLUDED.card_id,
                    name = EXCLUDED.name,
                    cost = EXCLUDED.cost,
                    target_scope = EXCLUDED.target_scope,
                    target_kind = EXCLUDED.target_kind,
                    target_max_targets = EXCLUDED.target_max_targets,
                    target_optional = EXCLUDED.target_optional,
                    once_per_turn = EXCLUDED.once_per_turn,
                    uses_stack = EXCLUDED.uses_stack,
                    tap_targets = EXCLUDED.tap_targets,
                    predicate = EXCLUDED.predicate,
                    sort_order = EXCLUDED.sort_order,
                    updated_at = NOW()
                "#,
            )
            .bind(&ability.id)
            .bind(&card.id)
            .bind(&ability.name)
            .bind(serde_json::to_value(&ability.cost)?)
            .bind(target_scope.as_deref())
            .bind(target_kind.as_deref())
            .bind(target_max)
            .bind(target_optional)
            .bind(ability.once_per_turn)
            .bind(ability.uses_stack)
            .bind(ability.tap_targets)
            .bind(
                ability
                    .predicate
                    .as_ref()
                    .map(|p| serde_json::to_value(p))
                    .transpose()?,
            )
            .bind(index as i32)
            .execute(&mut *tx)
            .await?;

            let mut effects = Vec::new();
            flatten_effects(&ability.effect, &mut effects);
            insert_effect_rows(&mut tx, "ability", None, Some(&ability.id), None, &effects).await?;
        }

        for (index, trigger) in card.triggers.iter().enumerate() {
            let trigger_id = format!("{}::trigger-{}", card.id, index + 1);
            let optional_cost_json = trigger
                .optional_cost
                .as_ref()
                .map(|cost| serde_json::to_value(cost))
                .transpose()?;

            let (event_kind, event_unit_type) = split_trigger_event(&trigger.event)?;

            sqlx::query(
                r#"
                INSERT INTO card_triggers (
                    trigger_id,
                    card_id,
                    event,
                    event_unit_type,
                    "limit",
                    predicate,
                    optional_cost,
                    is_optional,
                    sort_order
                ) VALUES (
                    $1,
                    $2,
                    $3::trigger_event,
                    $4::unit_type,
                    $5::trigger_limit,
                    $6,
                    $7,
                    $8,
                    $9
                )
                ON CONFLICT (trigger_id) DO UPDATE SET
                    card_id = EXCLUDED.card_id,
                    event = EXCLUDED.event,
                    event_unit_type = EXCLUDED.event_unit_type,
                    "limit" = EXCLUDED."limit",
                    predicate = EXCLUDED.predicate,
                    optional_cost = EXCLUDED.optional_cost,
                    is_optional = EXCLUDED.is_optional,
                    sort_order = EXCLUDED.sort_order,
                    updated_at = NOW()
                "#,
            )
            .bind(&trigger_id)
            .bind(&card.id)
            .bind(&event_kind)
            .bind(event_unit_type.as_deref())
            .bind(enum_to_string(&trigger.limit)?)
            .bind(serde_json::to_value(&trigger.predicate)?)
            .bind(optional_cost_json)
            .bind(trigger.is_optional)
            .bind(index as i32)
            .execute(&mut *tx)
            .await?;

            let mut effects = Vec::new();
            flatten_effects(&trigger.effect, &mut effects);
            insert_effect_rows(&mut tx, "trigger", None, None, Some(&trigger_id), &effects).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_card(&self, card: &Card) -> Result<()> {
        self.create_card(card).await
    }

    // --- User Card Instance Methods ---

    pub async fn create_user_card_instance(
        &self,
        instance: &UserCardInstance,
        actor_id: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO user_card_instances (instance_id, card_id, owner_id, visual_spec, is_tradeable, is_bound)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&instance.instance_id)
        .bind(&instance.card_id)
        .bind(&instance.owner_id)
        .bind(serde_json::to_value(&instance.visual_spec)?)
        .bind(instance.is_tradeable)
        .bind(instance.is_bound)
        .execute(&mut *tx)
        .await?;

        let log_id = Ulid::new().to_string();
        sqlx::query(
            r#"
            INSERT INTO card_audit_log (log_id, instance_id, event_kind, actor_id, new_owner_id, new_visual_spec, metadata)
            VALUES ($1, $2, 'created'::audit_event_kind, $3, $4, $5, $6)
            "#,
        )
        .bind(&log_id)
        .bind(&instance.instance_id)
        .bind(actor_id)
        .bind(&instance.owner_id)
        .bind(serde_json::to_value(&instance.visual_spec)?)
        .bind(metadata)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_user_card_instances(&self, owner_id: &str) -> Result<Vec<UserCardInstance>> {
        let rows = sqlx::query(
            r#"
            SELECT instance_id, card_id, owner_id, visual_spec, is_tradeable, is_bound, created_at, updated_at
            FROM user_card_instances
            WHERE owner_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(owner_id)
        .fetch_all(&*self.pool)
        .await?;

        rows.iter().map(user_card_instance_from_row).collect()
    }

    pub async fn get_user_card_instances_for_card(
        &self,
        owner_id: &str,
        card_id: &str,
    ) -> Result<Vec<UserCardInstance>> {
        let rows = sqlx::query(
            r#"
            SELECT instance_id, card_id, owner_id, visual_spec, is_tradeable, is_bound, created_at, updated_at
            FROM user_card_instances
            WHERE owner_id = $1 AND card_id = $2
            ORDER BY created_at
            "#,
        )
        .bind(owner_id)
        .bind(card_id)
        .fetch_all(&*self.pool)
        .await?;

        rows.iter().map(user_card_instance_from_row).collect()
    }

    pub async fn get_user_card_instance(
        &self,
        instance_id: &str,
    ) -> Result<Option<UserCardInstance>> {
        let row = sqlx::query(
            r#"
            SELECT instance_id, card_id, owner_id, visual_spec, is_tradeable, is_bound, created_at, updated_at
            FROM user_card_instances
            WHERE instance_id = $1
            "#,
        )
        .bind(instance_id)
        .fetch_optional(&*self.pool)
        .await?;

        match row {
            Some(ref r) => Ok(Some(user_card_instance_from_row(r)?)),
            None => Ok(None),
        }
    }

    pub async fn transfer_card_instance(
        &self,
        instance_id: &str,
        from_owner_id: &str,
        to_owner_id: &str,
        actor_id: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            UPDATE user_card_instances
            SET owner_id = $1, updated_at = NOW()
            WHERE instance_id = $2 AND owner_id = $3 AND is_tradeable = TRUE AND is_bound = FALSE
            "#,
        )
        .bind(to_owner_id)
        .bind(instance_id)
        .bind(from_owner_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!(
                "Card instance not found, not owned by sender, or not tradeable"
            ));
        }

        let log_id = Ulid::new().to_string();
        sqlx::query(
            r#"
            INSERT INTO card_audit_log (log_id, instance_id, event_kind, actor_id, previous_owner_id, new_owner_id)
            VALUES ($1, $2, 'transferred'::audit_event_kind, $3, $4, $5)
            "#,
        )
        .bind(&log_id)
        .bind(instance_id)
        .bind(actor_id)
        .bind(from_owner_id)
        .bind(to_owner_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_visual_spec(
        &self,
        instance_id: &str,
        new_spec: &VisualSpec,
        actor_id: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let old_row =
            sqlx::query("SELECT visual_spec FROM user_card_instances WHERE instance_id = $1")
                .bind(instance_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| anyhow!("Card instance not found"))?;

        let old_spec: serde_json::Value = old_row.try_get("visual_spec")?;

        sqlx::query(
            r#"
            UPDATE user_card_instances SET visual_spec = $1, updated_at = NOW() WHERE instance_id = $2
            "#,
        )
        .bind(serde_json::to_value(new_spec)?)
        .bind(instance_id)
        .execute(&mut *tx)
        .await?;

        let log_id = Ulid::new().to_string();
        sqlx::query(
            r#"
            INSERT INTO card_audit_log (log_id, instance_id, event_kind, actor_id, previous_visual_spec, new_visual_spec)
            VALUES ($1, $2, 'visual_spec_updated'::audit_event_kind, $3, $4, $5)
            "#,
        )
        .bind(&log_id)
        .bind(instance_id)
        .bind(actor_id)
        .bind(old_spec)
        .bind(serde_json::to_value(new_spec)?)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_audit_log(&self, instance_id: &str) -> Result<Vec<CardAuditEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT log_id, instance_id, event_kind::text AS event_kind, actor_id,
                   previous_owner_id, new_owner_id, previous_visual_spec, new_visual_spec,
                   metadata, created_at
            FROM card_audit_log
            WHERE instance_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(instance_id)
        .fetch_all(&*self.pool)
        .await?;

        rows.iter().map(audit_entry_from_row).collect()
    }

    // --- Instance-based Deck Methods ---

    pub async fn add_instance_to_deck(
        &self,
        deck_id: &str,
        instance_id: &str,
        is_operator: bool,
        sort_order: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO deck_card_instances (deck_id, instance_id, is_operator, sort_order)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (deck_id, instance_id) DO UPDATE SET
                is_operator = EXCLUDED.is_operator,
                sort_order = EXCLUDED.sort_order
            "#,
        )
        .bind(deck_id)
        .bind(instance_id)
        .bind(is_operator)
        .bind(sort_order)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_instance_from_deck(&self, deck_id: &str, instance_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM deck_card_instances WHERE deck_id = $1 AND instance_id = $2")
            .bind(deck_id)
            .bind(instance_id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_deck_instances(&self, deck_id: &str) -> Result<Vec<UserCardInstance>> {
        let rows = sqlx::query(
            r#"
            SELECT u.instance_id, u.card_id, u.owner_id, u.visual_spec,
                   u.is_tradeable, u.is_bound, u.created_at, u.updated_at
            FROM deck_card_instances d
            JOIN user_card_instances u ON u.instance_id = d.instance_id
            WHERE d.deck_id = $1
            ORDER BY d.sort_order, u.instance_id
            "#,
        )
        .bind(deck_id)
        .fetch_all(&*self.pool)
        .await?;

        rows.iter().map(user_card_instance_from_row).collect()
    }

    pub async fn get_deck_instance_count(&self, deck_id: &str) -> Result<i64> {
        let row =
            sqlx::query("SELECT COUNT(*) AS count FROM deck_card_instances WHERE deck_id = $1")
                .bind(deck_id)
                .fetch_one(&*self.pool)
                .await?;
        Ok(row.try_get("count")?)
    }

    pub async fn load_deck_instance_templates(
        &self,
        deck_id: &str,
    ) -> Result<Vec<(Card, UserCardInstance)>> {
        let rows = sqlx::query(
            r#"
            SELECT
                c.card_id,
                c.name,
                c.description,
                c.kind::text AS kind,
                c.supertype::text AS supertype,
                c.unit_type::text AS unit_type,
                c.artifact_type::text AS artifact_type,
                c.cost_neon, c.cost_glitch, c.cost_chrome, c.cost_slag, c.cost_bio, c.cost_colorless,
                c.power, c.toughness, c.target_spec, c.attachment,
                c.tapped, c.enters_tapped, c.keywords, c.image_url,
                c.colors::text[] AS colors, c.loyalty,
                c.rarity::text AS rarity, c.categories,
                u.instance_id, u.owner_id, u.visual_spec AS instance_visual_spec,
                u.is_tradeable, u.is_bound,
                u.created_at AS instance_created_at, u.updated_at AS instance_updated_at,
                d.is_operator, d.sort_order
            FROM deck_card_instances d
            JOIN user_card_instances u ON u.instance_id = d.instance_id
            JOIN cards c ON c.card_id = u.card_id
            WHERE d.deck_id = $1
            ORDER BY d.sort_order, u.instance_id
            "#,
        )
        .bind(deck_id)
        .fetch_all(&*self.pool)
        .await?;

        let mut results = Vec::new();
        let mut cards_to_hydrate = Vec::new();
        let mut instances = Vec::new();

        for row in &rows {
            cards_to_hydrate.push(card_from_row(row)?);

            let visual_spec_value: serde_json::Value = row.try_get("instance_visual_spec")?;
            let visual_spec: VisualSpec = serde_json::from_value(visual_spec_value)?;

            instances.push(UserCardInstance {
                instance_id: row.try_get("instance_id")?,
                card_id: row.try_get("card_id")?,
                owner_id: row.try_get("owner_id")?,
                visual_spec,
                is_tradeable: row.try_get("is_tradeable")?,
                is_bound: row.try_get("is_bound")?,
                created_at: row.try_get("instance_created_at")?,
                updated_at: row.try_get("instance_updated_at")?,
            });
        }

        let hydrated = self.hydrate_cards(cards_to_hydrate).await?;
        for (card, instance) in hydrated.into_iter().zip(instances) {
            results.push((card, instance));
        }

        Ok(results)
    }

    pub async fn get_card_by_id(&self, card_id: &str) -> Result<Option<Card>> {
        let row = sqlx::query(
            r#"
            SELECT
                card_id,
                name,
                description,
                kind::text AS kind,
                supertype::text AS supertype,
                unit_type::text AS unit_type,
                artifact_type::text AS artifact_type,
                cost_neon,
                cost_glitch,
                cost_chrome,
                cost_slag,
                cost_bio,
                cost_colorless,
                power,
                toughness,
                target_spec,
                attachment,
                tapped,
                enters_tapped,
                keywords,
                image_url,
                colors::text[] AS colors,
                loyalty,
                rarity::text AS rarity,
                categories
            FROM cards
            WHERE card_id = $1
            "#,
        )
        .bind(card_id)
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(row) = row {
            let card = card_from_row(&row)?;
            let mut cards = self.hydrate_cards(vec![card]).await?;
            Ok(cards.pop())
        } else {
            Ok(None)
        }
    }
}

impl Repository<Card, CardArgs> for CardRepository {
    async fn count(&self, _args: &CardArgs) -> Result<i32> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM cards")
            .fetch_one(&*self.pool)
            .await?;
        let count: i64 = row.try_get("count")?;
        Ok(count as i32)
    }

    async fn find(
        &self,
        after: Option<(CursorDirection, impl Cursor + Send)>,
        take: i32,
        args: &CardArgs,
    ) -> Result<Vec<Card>> {
        let has_owner = args.owner_id.is_some();
        let base_query = if has_owner {
            r#"
            SELECT
                c.card_id,
                c.name,
                c.description,
                c.kind::text AS kind,
                c.supertype::text AS supertype,
                c.unit_type::text AS unit_type,
                c.artifact_type::text AS artifact_type,
                c.cost_neon, c.cost_glitch, c.cost_chrome, c.cost_slag, c.cost_bio, c.cost_colorless,
                c.power, c.toughness, c.target_spec, c.attachment,
                c.tapped, c.enters_tapped, c.keywords, c.image_url,
                c.colors::text[] AS colors, c.loyalty,
                c.rarity::text AS rarity, c.categories,
                c.created_at,
                (o.card_id IS NOT NULL) AS owned,
                o.obtained_at,
                o.visual_spec AS instance_visual_spec
            FROM cards c
            LEFT JOIN (
                SELECT DISTINCT ON (card_id) card_id, created_at AS obtained_at, visual_spec
                FROM user_card_instances
                WHERE owner_id = $__OWNER__
                ORDER BY card_id, created_at ASC
            ) o ON o.card_id = c.card_id
            "#
        } else {
            r#"
            SELECT
                card_id,
                name,
                description,
                kind::text AS kind,
                supertype::text AS supertype,
                unit_type::text AS unit_type,
                artifact_type::text AS artifact_type,
                cost_neon, cost_glitch, cost_chrome, cost_slag, cost_bio, cost_colorless,
                power, toughness, target_spec, attachment,
                tapped, enters_tapped, keywords, image_url,
                colors::text[] AS colors, loyalty,
                rarity::text AS rarity, categories,
                created_at
            FROM cards
            "#
        };

        let is_before = matches!(
            after.as_ref().map(|(direction, _)| direction),
            Some(CursorDirection::Before)
        );

        let rows = if has_owner {
            let owner_id = args.owner_id.as_ref().unwrap();
            // With owner LEFT JOIN, owner_id is always $1
            let base = base_query.replace("$__OWNER__", "$1");
            match after {
                Some((CursorDirection::After, cursor)) => {
                    sqlx::query(&format!(
                        "{base} WHERE c.card_id > $2 ORDER BY c.card_id ASC LIMIT $3"
                    ))
                    .bind(owner_id)
                    .bind(cursor.id())
                    .bind(take as i64)
                    .fetch_all(&*self.pool)
                    .await?
                }
                Some((CursorDirection::Before, cursor)) => {
                    sqlx::query(&format!(
                        "{base} WHERE c.card_id < $2 ORDER BY c.card_id DESC LIMIT $3"
                    ))
                    .bind(owner_id)
                    .bind(cursor.id())
                    .bind(take as i64)
                    .fetch_all(&*self.pool)
                    .await?
                }
                None => {
                    sqlx::query(&format!("{base} ORDER BY c.card_id ASC LIMIT $2"))
                        .bind(owner_id)
                        .bind(take as i64)
                        .fetch_all(&*self.pool)
                        .await?
                }
            }
        } else {
            match after {
                Some((CursorDirection::After, cursor)) => {
                    sqlx::query(&format!(
                        "{base_query} WHERE card_id > $1 ORDER BY card_id ASC LIMIT $2"
                    ))
                    .bind(cursor.id())
                    .bind(take as i64)
                    .fetch_all(&*self.pool)
                    .await?
                }
                Some((CursorDirection::Before, cursor)) => {
                    sqlx::query(&format!(
                        "{base_query} WHERE card_id < $1 ORDER BY card_id DESC LIMIT $2"
                    ))
                    .bind(cursor.id())
                    .bind(take as i64)
                    .fetch_all(&*self.pool)
                    .await?
                }
                None => {
                    sqlx::query(&format!("{base_query} ORDER BY card_id ASC LIMIT $1"))
                        .bind(take as i64)
                        .fetch_all(&*self.pool)
                        .await?
                }
            }
        };

        let mut cards = Vec::new();
        for row in rows {
            cards.push(card_from_row(&row)?);
        }

        if is_before {
            cards.reverse();
        }

        self.hydrate_cards(cards).await
    }
}

impl CardRepository {
    pub async fn load_owned_card_templates(
        &self,
        owner_id: &str,
    ) -> Result<Vec<DeckTemplateEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT
                c.card_id,
                c.name,
                c.description,
                c.kind::text AS kind,
                c.supertype::text AS supertype,
                c.unit_type::text AS unit_type,
                c.artifact_type::text AS artifact_type,
                c.cost_neon,
                c.cost_glitch,
                c.cost_chrome,
                c.cost_slag,
                c.cost_bio,
                c.cost_colorless,
                c.power,
                c.toughness,
                c.target_spec,
                c.attachment,
                c.tapped,
                c.enters_tapped,
                c.keywords,
                c.image_url,
                c.colors::text[] AS colors,
                c.loyalty,
                c.rarity::text AS rarity,
                c.categories,
                COUNT(*)::int AS quantity
            FROM user_card_instances uci
            JOIN cards c ON c.card_id = uci.card_id
            WHERE uci.owner_id = $1
            GROUP BY
                c.card_id,
                c.name,
                c.description,
                c.kind,
                c.supertype,
                c.unit_type,
                c.artifact_type,
                c.cost_neon,
                c.cost_glitch,
                c.cost_chrome,
                c.cost_slag,
                c.cost_bio,
                c.cost_colorless,
                c.power,
                c.toughness,
                c.target_spec,
                c.attachment,
                c.tapped,
                c.enters_tapped,
                c.keywords,
                c.image_url,
                c.colors,
                c.loyalty,
                c.rarity,
                c.categories
            ORDER BY COUNT(*) DESC, c.name ASC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&*self.pool)
        .await?;

        let mut cards = Vec::with_capacity(rows.len());
        let mut quantities = Vec::with_capacity(rows.len());
        for row in rows {
            cards.push(card_from_row(&row)?);
            quantities.push(row.try_get::<i32, _>("quantity")?);
        }

        let cards = self.hydrate_cards(cards).await?;
        Ok(cards
            .into_iter()
            .zip(quantities)
            .map(|(card, quantity)| DeckTemplateEntry { card, quantity })
            .collect())
    }

    /// Returns all card instances owned by a user (no deduplication).
    /// Each instance gets its own entry with instance_id as the card id.
    pub async fn find_user_collection(
        &self,
        owner_id: &str,
        take: i32,
        offset: i32,
    ) -> Result<(Vec<Card>, i32)> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM user_card_instances WHERE owner_id = $1")
                .bind(owner_id)
                .fetch_one(&*self.pool)
                .await?;

        let rows = sqlx::query(
            r#"
            SELECT
                uci.instance_id,
                c.card_id,
                c.name,
                c.description,
                c.kind::text AS kind,
                c.supertype::text AS supertype,
                c.unit_type::text AS unit_type,
                c.artifact_type::text AS artifact_type,
                c.cost_neon, c.cost_glitch, c.cost_chrome, c.cost_slag, c.cost_bio, c.cost_colorless,
                c.power, c.toughness, c.target_spec, c.attachment,
                c.tapped, c.enters_tapped, c.keywords, c.image_url,
                c.colors::text[] AS colors, c.loyalty,
                c.rarity::text AS rarity, c.categories,
                uci.created_at,
                uci.visual_spec AS instance_visual_spec
            FROM user_card_instances uci
            JOIN cards c ON c.card_id = uci.card_id
            WHERE uci.owner_id = $1
            ORDER BY c.name ASC, uci.created_at ASC, uci.instance_id ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(owner_id)
        .bind(take as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await?;

        let mut cards = Vec::new();
        for row in &rows {
            let mut card = card_from_row(row)?;
            let instance_id: String = row.try_get("instance_id")?;
            card.template_id = Some(card.id.clone());
            card.id = instance_id;
            card.owned = true;
            card.obtained_at = row.try_get("created_at").ok();
            cards.push(card);
        }

        // Hydrate abilities/triggers so the browser can show full rules text.
        let cards = self.hydrate_collection_cards(cards).await?;
        Ok((cards, count as i32))
    }

    /// Returns all cards in the catalog with ownership info, paginated by offset.
    /// Unlike find_user_collection, this returns ALL cards (not just owned ones).
    pub async fn find_catalog(
        &self,
        owner_id: &str,
        take: i32,
        offset: i32,
        color_filters: &[String],
        kind_filters: &[String],
        sort: Option<&str>,
    ) -> Result<(Vec<Card>, i32)> {
        // Use $4 for colors and $5 for kinds; empty arrays match everything
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM cards c
               WHERE ($1::mana_type[] = '{}' OR c.colors && $1::mana_type[])
                 AND ($2::text[] = '{}' OR c.kind::text = ANY($2))"#,
        )
        .bind(color_filters)
        .bind(kind_filters)
        .fetch_one(&*self.pool)
        .await?;

        // Whitelist sort expressions to prevent SQL injection
        let desc = sort.map(|s| s.ends_with("_desc")).unwrap_or(false);
        let base_sort = sort.map(|s| s.trim_end_matches("_desc")).unwrap_or("name");
        let dir = if desc { "DESC" } else { "ASC" };
        let order_sql = match base_sort {
            "color" => format!("c.colors[1] {dir}, c.name ASC, c.card_id ASC"),
            "rarity" => format!(
                "CASE c.rarity::text \
                 WHEN 'singularity' THEN 0 \
                 WHEN 'classified' THEN 1 \
                 WHEN 'restricted' THEN 2 \
                 WHEN 'private' THEN 3 \
                 WHEN 'public' THEN 4 \
                 ELSE 5 END {dir}, c.name ASC, c.card_id ASC"
            ),
            "created_at" => format!("c.created_at {dir}, c.name ASC, c.card_id ASC"),
            _ => format!("c.name {dir}, c.card_id ASC"),
        };

        let select_sql = format!(
            r#"
            SELECT
                c.card_id,
                c.name,
                c.description,
                c.kind::text AS kind,
                c.supertype::text AS supertype,
                c.unit_type::text AS unit_type,
                c.artifact_type::text AS artifact_type,
                c.cost_neon, c.cost_glitch, c.cost_chrome, c.cost_slag, c.cost_bio, c.cost_colorless,
                c.power, c.toughness, c.target_spec, c.attachment,
                c.tapped, c.enters_tapped, c.keywords, c.image_url,
                c.colors::text[] AS colors, c.loyalty,
                c.rarity::text AS rarity, c.categories,
                c.created_at,
                (o.card_id IS NOT NULL) AS owned,
                o.obtained_at,
                o.visual_spec AS instance_visual_spec
            FROM cards c
            LEFT JOIN (
                SELECT DISTINCT ON (card_id) card_id, created_at AS obtained_at, visual_spec
                FROM user_card_instances
                WHERE owner_id = $1
                ORDER BY card_id, created_at ASC
            ) o ON o.card_id = c.card_id
            WHERE ($4::mana_type[] = '{{}}' OR c.colors && $4::mana_type[])
              AND ($5::text[] = '{{}}' OR c.kind::text = ANY($5))
            ORDER BY {}
            LIMIT $2 OFFSET $3
            "#,
            order_sql
        );

        let rows = sqlx::query(&select_sql)
            .bind(owner_id)
            .bind(take as i64)
            .bind(offset as i64)
            .bind(color_filters)
            .bind(kind_filters)
            .fetch_all(&*self.pool)
            .await?;

        let mut cards = Vec::new();
        for row in &rows {
            let mut card = card_from_row(row)?;
            // `hydrate_collection_cards` looks up by template_id; for catalog
            // rows that ARE the template, point template_id at the card's id.
            if card.template_id.is_none() {
                card.template_id = Some(card.id.clone());
            }
            cards.push(card);
        }

        // Hydrate abilities/triggers so the browser can show full rules text.
        let cards = self.hydrate_collection_cards(cards).await?;
        Ok((cards, count as i32))
    }

    /// Hydrates abilities/triggers for collection cards where multiple
    /// instances may share the same card_id (stored in template_id).
    async fn hydrate_collection_cards(&self, mut cards: Vec<Card>) -> Result<Vec<Card>> {
        let unique_card_ids: Vec<String> = cards
            .iter()
            .filter_map(|c| c.template_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if unique_card_ids.is_empty() {
            return Ok(cards);
        }

        let (abilities_by_card, ability_ids) = self.load_card_abilities(&unique_card_ids).await?;
        let ability_effects = self
            .load_effects_by_owner("ability", "ability_id", &ability_ids)
            .await?;

        let (triggers_by_card, trigger_ids) = self.load_card_triggers(&unique_card_ids).await?;
        let trigger_effects = self
            .load_effects_by_owner("trigger", "trigger_id", &trigger_ids)
            .await?;

        let mut abilities_by_card = abilities_by_card;
        for abilities in abilities_by_card.values_mut() {
            for ability in abilities.iter_mut() {
                let effects = ability_effects
                    .get(&ability.id)
                    .cloned()
                    .unwrap_or_default();
                ability.effect = assemble_effect(effects)
                    .ok_or_else(|| anyhow!("Ability {} missing effect", ability.id))?;
            }
        }

        let mut triggers_by_card = triggers_by_card;
        for triggers in triggers_by_card.values_mut() {
            for (trigger_id, trigger) in triggers.iter_mut() {
                let effects = trigger_effects.get(trigger_id).cloned().unwrap_or_default();
                trigger.effect = assemble_effect(effects)
                    .ok_or_else(|| anyhow!("Trigger {trigger_id} missing effect"))?;
            }
        }

        // Use get+clone (not remove) since multiple instances share the same card_id
        for card in cards.iter_mut() {
            if let Some(card_id) = &card.template_id {
                card.abilities = abilities_by_card.get(card_id).cloned().unwrap_or_default();
                card.triggers = triggers_by_card
                    .get(card_id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(_, trigger)| trigger)
                    .collect();
            }
        }

        Ok(cards)
    }

    async fn hydrate_cards(&self, mut cards: Vec<Card>) -> Result<Vec<Card>> {
        let card_ids: Vec<String> = cards.iter().map(|card| card.id.clone()).collect();
        if card_ids.is_empty() {
            return Ok(cards);
        }

        let (abilities_by_card, ability_ids) = self.load_card_abilities(&card_ids).await?;
        let ability_effects = self
            .load_effects_by_owner("ability", "ability_id", &ability_ids)
            .await?;

        let (triggers_by_card, trigger_ids) = self.load_card_triggers(&card_ids).await?;
        let trigger_effects = self
            .load_effects_by_owner("trigger", "trigger_id", &trigger_ids)
            .await?;

        let mut abilities_by_card = abilities_by_card;
        for abilities in abilities_by_card.values_mut() {
            for ability in abilities.iter_mut() {
                let effects = ability_effects
                    .get(&ability.id)
                    .cloned()
                    .unwrap_or_default();
                ability.effect = assemble_effect(effects)
                    .ok_or_else(|| anyhow!("Ability {} missing effect", ability.id))?;
            }
        }

        let mut triggers_by_card = triggers_by_card;
        for triggers in triggers_by_card.values_mut() {
            for (trigger_id, trigger) in triggers.iter_mut() {
                let effects = trigger_effects.get(trigger_id).cloned().unwrap_or_default();
                trigger.effect = assemble_effect(effects)
                    .ok_or_else(|| anyhow!("Trigger {trigger_id} missing effect"))?;
            }
        }

        for card in cards.iter_mut() {
            card.abilities = abilities_by_card.remove(&card.id).unwrap_or_default();
            card.triggers = triggers_by_card
                .remove(&card.id)
                .unwrap_or_default()
                .into_iter()
                .map(|(_, trigger)| trigger)
                .collect();
        }

        Ok(cards)
    }

    async fn load_effect_keywords(
        &self,
        effect_ids: &[String],
    ) -> Result<std::collections::HashMap<String, Vec<Keyword>>> {
        if effect_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT
                effect_id,
                keyword_kind::text AS keyword_kind,
                keyword_amount,
                keyword_unit_type::text AS keyword_unit_type,
                sort_order
            FROM card_effect_keywords
            WHERE effect_id = ANY($1)
            ORDER BY effect_id, sort_order
            "#,
        )
        .bind(effect_ids)
        .fetch_all(&*self.pool)
        .await?;

        let mut out: std::collections::HashMap<String, Vec<Keyword>> =
            std::collections::HashMap::new();
        for row in rows {
            let effect_id: String = row.try_get("effect_id")?;
            let keyword_kind: Option<String> = row.try_get("keyword_kind")?;
            let keyword_amount: Option<i32> = row.try_get("keyword_amount")?;
            let keyword_unit_type: Option<String> = row.try_get("keyword_unit_type")?;
            let keyword = parse_keyword(keyword_kind, keyword_amount, keyword_unit_type)?;
            out.entry(effect_id).or_default().push(keyword);
        }

        Ok(out)
    }

    async fn load_card_effects(
        &self,
        card_ids: &[String],
        owner_kind: &str,
    ) -> Result<std::collections::HashMap<String, Vec<CardEffect>>> {
        let rows = sqlx::query(
            r#"
            SELECT
                card_id,
                effect_id,
                effect_kind::text AS effect_kind,
                mana_type::text AS mana_type,
                amount,
                count,
                target_scope::text AS target_scope,
                target_spec_kind::text AS target_spec_kind,
                target_unit_type::text AS target_unit_type,
                scale_unit_type::text AS scale_unit_type,
                scale_card_kind::text AS scale_card_kind,
                keyword_kind::text AS keyword_kind,
                keyword_amount,
                keyword_unit_type::text AS keyword_unit_type,
                token_name,
                token_power,
                token_toughness,
                token_unit_type::text AS token_unit_type,
                stat_type::text AS stat_type,
                stat_amount,
                stat_amount_dynamic,
                expires_kind::text AS expires_kind,
                expires_turns,
                effect_predicate,
                sort_order
            FROM card_effects
            WHERE card_id = ANY($1) AND owner_kind = $2::effect_owner_kind
            ORDER BY card_id, sort_order
            "#,
        )
        .bind(card_ids)
        .bind(owner_kind)
        .fetch_all(&*self.pool)
        .await?;

        let mut effect_ids = Vec::new();
        for row in &rows {
            effect_ids.push(row.try_get::<String, _>("effect_id")?);
        }
        let keywords_by_effect = self.load_effect_keywords(&effect_ids).await?;

        let mut out: std::collections::HashMap<String, Vec<CardEffect>> =
            std::collections::HashMap::new();
        for row in rows {
            let card_id: String = row.try_get("card_id")?;
            let effect_id: String = row.try_get("effect_id")?;
            let keywords = keywords_by_effect.get(&effect_id);
            let effect = parse_effect_row(&row, keywords)?;
            out.entry(card_id).or_default().push(effect);
        }
        Ok(out)
    }

    async fn load_card_abilities(
        &self,
        card_ids: &[String],
    ) -> Result<(
        std::collections::HashMap<String, Vec<CardAbility>>,
        Vec<String>,
    )> {
        let rows = sqlx::query(
            r#"
            SELECT
                ability_id,
                card_id,
                name,
                cost,
                target_scope::text AS target_scope,
                target_kind::text AS target_kind,
                target_max_targets,
                target_optional,
                once_per_turn,
                uses_stack,
                tap_targets,
                predicate,
                sort_order
            FROM card_abilities
            WHERE card_id = ANY($1)
            ORDER BY card_id, sort_order
            "#,
        )
        .bind(card_ids)
        .fetch_all(&*self.pool)
        .await?;

        let mut out: std::collections::HashMap<String, Vec<CardAbility>> =
            std::collections::HashMap::new();
        let mut ability_ids = Vec::new();

        for row in rows {
            let ability_id: String = row.try_get("ability_id")?;
            let card_id: String = row.try_get("card_id")?;
            let target_scope: Option<String> = row.try_get("target_scope")?;
            let target_kind: Option<String> = row.try_get("target_kind")?;
            let target_max: Option<i32> = row.try_get("target_max_targets")?;
            let target_optional: bool = row.try_get("target_optional")?;

            let target_spec = if let (Some(scope), Some(kind), Some(max_targets)) =
                (target_scope, target_kind, target_max)
            {
                Some(TargetSpec {
                    scope: parse_enum_value(&scope)?,
                    kind: parse_enum_value(&kind)?,
                    max_targets: max_targets as usize,
                    optional: target_optional,
                    predicate: None,
                })
            } else {
                None
            };

            let predicate: Option<Predicate> =
                parse_json(row.try_get("predicate")?, serde_json::Value::Null)?;

            let cost: AbilityCost =
                parse_json(row.try_get("cost")?, serde_json::json!({"costs": []}))?;

            let ability = CardAbility {
                id: ability_id.clone(),
                name: row.try_get("name")?,
                cost,
                effect: CardEffect::TapTarget,
                target_spec,
                once_per_turn: row.try_get("once_per_turn")?,
                uses_stack: row.try_get("uses_stack")?,
                tap_targets: row.try_get("tap_targets")?,
                predicate,
            };

            out.entry(card_id).or_default().push(ability);
            ability_ids.push(ability_id);
        }

        Ok((out, ability_ids))
    }

    async fn load_card_triggers(
        &self,
        card_ids: &[String],
    ) -> Result<(
        std::collections::HashMap<String, Vec<(String, CardTrigger)>>,
        Vec<String>,
    )> {
        let rows = sqlx::query(
            r#"
            SELECT
                trigger_id,
                card_id,
                event::text AS event,
                event_unit_type::text AS event_unit_type,
                "limit"::text AS "limit",
                predicate,
                optional_cost,
                is_optional,
                sort_order
            FROM card_triggers
            WHERE card_id = ANY($1)
            ORDER BY card_id, sort_order
            "#,
        )
        .bind(card_ids)
        .fetch_all(&*self.pool)
        .await?;

        let mut out: std::collections::HashMap<String, Vec<(String, CardTrigger)>> =
            std::collections::HashMap::new();
        let mut trigger_ids = Vec::new();

        for row in rows {
            let trigger_id: String = row.try_get("trigger_id")?;
            let card_id: String = row.try_get("card_id")?;
            let event: String = row.try_get("event")?;
            let event_unit_type: Option<String> = row.try_get("event_unit_type")?;
            let limit: String = row.try_get("limit")?;
            let predicate: Option<Predicate> =
                parse_json(row.try_get("predicate")?, serde_json::Value::Null)?;

            let is_optional: bool = row.try_get("is_optional")?;

            let optional_cost: Option<AbilityCost> =
                parse_json(row.try_get("optional_cost")?, serde_json::Value::Null)?;

            let trigger = CardTrigger {
                event: parse_trigger_event(&event, event_unit_type)?,
                effect: CardEffect::TapTarget,
                limit: parse_enum_value(&limit)?,
                predicate,
                optional_cost,
                is_optional,
            };

            out.entry(card_id)
                .or_default()
                .push((trigger_id.clone(), trigger));
            trigger_ids.push(trigger_id);
        }

        Ok((out, trigger_ids))
    }

    async fn load_effects_by_owner(
        &self,
        owner_kind: &str,
        owner_column: &str,
        owner_ids: &[String],
    ) -> Result<std::collections::HashMap<String, Vec<CardEffect>>> {
        if owner_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let query = format!(
            r#"
            SELECT
                {owner_column} AS owner_id,
                effect_id,
                effect_kind::text AS effect_kind,
                mana_type::text AS mana_type,
                amount,
                count,
                target_scope::text AS target_scope,
                target_spec_kind::text AS target_spec_kind,
                target_unit_type::text AS target_unit_type,
                scale_unit_type::text AS scale_unit_type,
                scale_card_kind::text AS scale_card_kind,
                keyword_kind::text AS keyword_kind,
                keyword_amount,
                keyword_unit_type::text AS keyword_unit_type,
                token_name,
                token_power,
                token_toughness,
                token_unit_type::text AS token_unit_type,
                stat_type::text AS stat_type,
                stat_amount,
                stat_amount_dynamic,
                expires_kind::text AS expires_kind,
                expires_turns,
                effect_predicate,
                sort_order
            FROM card_effects
            WHERE {owner_column} = ANY($1) AND owner_kind = $2::effect_owner_kind
            ORDER BY {owner_column}, sort_order
            "#
        );

        let rows = sqlx::query(&query)
            .bind(owner_ids)
            .bind(owner_kind)
            .fetch_all(&*self.pool)
            .await?;

        let mut effect_ids = Vec::new();
        for row in &rows {
            effect_ids.push(row.try_get::<String, _>("effect_id")?);
        }
        let keywords_by_effect = self.load_effect_keywords(&effect_ids).await?;

        let mut out: std::collections::HashMap<String, Vec<CardEffect>> =
            std::collections::HashMap::new();
        for row in rows {
            let owner_id: String = row.try_get("owner_id")?;
            let effect_id: String = row.try_get("effect_id")?;
            let keywords = keywords_by_effect.get(&effect_id);
            let effect = parse_effect_row(&row, keywords)?;
            out.entry(owner_id).or_default().push(effect);
        }
        Ok(out)
    }
}

// ── Pack Methods ──────────────────────────────────────────────────────────────

use crate::pack::{PackInstance, PackOpeningResult, PackSlot, PackType};

impl CardRepository {
    // Pack types

    pub async fn list_pack_types(&self) -> Result<Vec<PackType>> {
        let rows = sqlx::query(
            r#"
            SELECT pack_type_id, name, description, active
            FROM pack_types
            WHERE active = TRUE
            ORDER BY created_at
            "#,
        )
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut pack_types = Vec::with_capacity(rows.len());
        for row in rows {
            let pack_type_id: String = row.try_get("pack_type_id")?;
            let slots = self.get_pack_slots(&pack_type_id).await?;
            pack_types.push(PackType {
                pack_type_id,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                active: row.try_get("active")?,
                slots,
            });
        }

        Ok(pack_types)
    }

    pub async fn get_pack_type(&self, pack_type_id: &str) -> Result<Option<PackType>> {
        let row = sqlx::query(
            r#"
            SELECT pack_type_id, name, description, active
            FROM pack_types
            WHERE pack_type_id = $1
            "#,
        )
        .bind(pack_type_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        match row {
            Some(row) => {
                let id: String = row.try_get("pack_type_id")?;
                let slots = self.get_pack_slots(&id).await?;
                Ok(Some(PackType {
                    pack_type_id: id,
                    name: row.try_get("name")?,
                    description: row.try_get("description")?,
                    active: row.try_get("active")?,
                    slots,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn create_pack_type(
        &self,
        pack_type_id: &str,
        name: &str,
        description: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pack_types (pack_type_id, name, description)
            VALUES ($1, $2, $3)
            ON CONFLICT (pack_type_id) DO UPDATE SET name = $2, description = $3, updated_at = now()
            "#,
        )
        .bind(pack_type_id)
        .bind(name)
        .bind(description)
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }

    // Pack slots

    pub async fn get_pack_slots(&self, pack_type_id: &str) -> Result<Vec<PackSlot>> {
        let rows = sqlx::query(
            r#"
            SELECT slot_id, pack_type_id, slot_index,
                   weight_public, weight_private, weight_restricted,
                   weight_classified, weight_singularity
            FROM pack_slots
            WHERE pack_type_id = $1
            ORDER BY slot_index
            "#,
        )
        .bind(pack_type_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut slots = Vec::with_capacity(rows.len());
        for row in rows {
            slots.push(PackSlot {
                slot_id: row.try_get("slot_id")?,
                pack_type_id: row.try_get("pack_type_id")?,
                slot_index: row.try_get("slot_index")?,
                weight_public: row.try_get("weight_public")?,
                weight_private: row.try_get("weight_private")?,
                weight_restricted: row.try_get("weight_restricted")?,
                weight_classified: row.try_get("weight_classified")?,
                weight_singularity: row.try_get("weight_singularity")?,
            });
        }

        Ok(slots)
    }

    pub async fn create_pack_slot(&self, slot: &PackSlot) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pack_slots (slot_id, pack_type_id, slot_index,
                                    weight_public, weight_private, weight_restricted,
                                    weight_classified, weight_singularity)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (pack_type_id, slot_index) DO UPDATE
            SET weight_public = $4, weight_private = $5, weight_restricted = $6,
                weight_classified = $7, weight_singularity = $8
            "#,
        )
        .bind(&slot.slot_id)
        .bind(&slot.pack_type_id)
        .bind(slot.slot_index)
        .bind(slot.weight_public)
        .bind(slot.weight_private)
        .bind(slot.weight_restricted)
        .bind(slot.weight_classified)
        .bind(slot.weight_singularity)
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }

    // Pack inventory

    pub async fn create_pack_instance(&self, pack: &PackInstance) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pack_inventory (pack_instance_id, pack_type_id, owner_id, source, source_ref)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&pack.pack_instance_id)
        .bind(&pack.pack_type_id)
        .bind(&pack.owner_id)
        .bind(&pack.source)
        .bind(&pack.source_ref)
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_user_packs(&self, owner_id: &str) -> Result<Vec<PackInstance>> {
        let rows = sqlx::query(
            r#"
            SELECT pack_instance_id, pack_type_id, owner_id, source, source_ref,
                   opened, opened_at, created_at
            FROM pack_inventory
            WHERE owner_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut packs = Vec::with_capacity(rows.len());
        for row in rows {
            packs.push(PackInstance {
                pack_instance_id: row.try_get("pack_instance_id")?,
                pack_type_id: row.try_get("pack_type_id")?,
                owner_id: row.try_get("owner_id")?,
                source: row.try_get("source")?,
                source_ref: row.try_get("source_ref")?,
                opened: row.try_get("opened")?,
                opened_at: row.try_get("opened_at")?,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(packs)
    }

    pub async fn get_pack_instance(&self, pack_instance_id: &str) -> Result<Option<PackInstance>> {
        let row = sqlx::query(
            r#"
            SELECT pack_instance_id, pack_type_id, owner_id, source, source_ref,
                   opened, opened_at, created_at
            FROM pack_inventory
            WHERE pack_instance_id = $1
            "#,
        )
        .bind(pack_instance_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        match row {
            Some(row) => Ok(Some(PackInstance {
                pack_instance_id: row.try_get("pack_instance_id")?,
                pack_type_id: row.try_get("pack_type_id")?,
                owner_id: row.try_get("owner_id")?,
                source: row.try_get("source")?,
                source_ref: row.try_get("source_ref")?,
                opened: row.try_get("opened")?,
                opened_at: row.try_get("opened_at")?,
                created_at: row.try_get("created_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn mark_pack_opened(&self, pack_instance_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE pack_inventory
            SET opened = TRUE, opened_at = now()
            WHERE pack_instance_id = $1
            "#,
        )
        .bind(pack_instance_id)
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }

    // Pack opening results

    pub async fn create_pack_opening_result(&self, result: &PackOpeningResult) -> Result<()> {
        let rarity_str = enum_to_string(&result.rarity)?;
        sqlx::query(
            r#"
            INSERT INTO pack_opening_results (result_id, pack_instance_id, slot_index,
                                              instance_id, card_id, rarity)
            VALUES ($1, $2, $3, $4, $5, $6::card_rarity)
            "#,
        )
        .bind(&result.result_id)
        .bind(&result.pack_instance_id)
        .bind(result.slot_index)
        .bind(&result.instance_id)
        .bind(&result.card_id)
        .bind(&rarity_str)
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }

    pub async fn get_pack_opening_results(
        &self,
        pack_instance_id: &str,
    ) -> Result<Vec<PackOpeningResult>> {
        let rows = sqlx::query(
            r#"
            SELECT por.result_id, por.pack_instance_id, por.slot_index, por.instance_id, por.card_id,
                   por.rarity::text as rarity, c.name as card_name, c.image_url as card_image_url,
                   uci.visual_spec as instance_visual_spec
            FROM pack_opening_results por
            LEFT JOIN cards c ON c.card_id = por.card_id
            LEFT JOIN user_card_instances uci ON uci.instance_id = por.instance_id
            WHERE por.pack_instance_id = $1
            ORDER BY por.slot_index
            "#,
        )
        .bind(pack_instance_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let rarity_str: String = row.try_get("rarity")?;
            let rarity: CardRarity = serde_json::from_value(serde_json::Value::String(rarity_str))?;
            let card_id: String = row.try_get("card_id")?;
            let full_card = self.get_card_by_id(&card_id).await?;
            results.push(PackOpeningResult {
                result_id: row.try_get("result_id")?,
                pack_instance_id: row.try_get("pack_instance_id")?,
                slot_index: row.try_get("slot_index")?,
                instance_id: row.try_get("instance_id")?,
                card_id: card_id.clone(),
                rarity,
                card_name: row.try_get::<String, _>("card_name").unwrap_or_default(),
                card_image_url: row
                    .try_get::<Option<String>, _>("card_image_url")
                    .unwrap_or(None),
                description: full_card
                    .as_ref()
                    .map(|c| c.description.clone())
                    .unwrap_or_default(),
                cost: full_card
                    .as_ref()
                    .map(|c| c.cost.clone())
                    .unwrap_or_default(),
                kind_tag: full_card
                    .as_ref()
                    .map(|c| crate::pack::pack_kind_tag(&c.kind).to_string())
                    .unwrap_or_default(),
                kind_label: full_card
                    .as_ref()
                    .map(|c| crate::pack::pack_kind_label(&c.kind).to_string())
                    .unwrap_or_default(),
                power: full_card.as_ref().and_then(|c| c.power),
                toughness: full_card.as_ref().and_then(|c| c.toughness),
                colors: full_card
                    .as_ref()
                    .map(|c| c.colors.clone())
                    .unwrap_or_default(),
                keywords: full_card
                    .as_ref()
                    .map(|c| c.keywords.clone())
                    .unwrap_or_default(),
                frame_style: full_card
                    .as_ref()
                    .map(|c| crate::pack::pack_frame_style(&c.kind).to_string())
                    .unwrap_or_default(),
                categories: full_card
                    .as_ref()
                    .map(|c| c.categories.clone())
                    .unwrap_or_default(),
                visual_spec: row
                    .try_get::<Option<serde_json::Value>, _>("instance_visual_spec")
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_value(v).ok()),
                loyalty: full_card.as_ref().and_then(|c| c.loyalty),
            });
        }

        Ok(results)
    }

    // Card pool query by rarity

    pub async fn get_cards_by_rarity(&self, rarity: &CardRarity) -> Result<Vec<Card>> {
        let rarity_str = enum_to_string(rarity)?;
        let rows = sqlx::query(
            r#"
            SELECT card_id FROM cards WHERE rarity = $1::card_rarity
            "#,
        )
        .bind(&rarity_str)
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut cards = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.try_get("card_id")?;
            if let Some(card) = self.get_card_by_id(&id).await? {
                cards.push(card);
            }
        }

        Ok(cards)
    }

    // Pack audit

    pub async fn create_pack_audit(
        &self,
        pack_instance_id: &str,
        event_kind: &str,
        actor_id: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let log_id = Ulid::new().to_string();
        sqlx::query(
            r#"
            INSERT INTO pack_audit_log (log_id, pack_instance_id, event_kind, actor_id, metadata)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&log_id)
        .bind(pack_instance_id)
        .bind(event_kind)
        .bind(actor_id)
        .bind(metadata.unwrap_or(serde_json::json!({})))
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }
}
