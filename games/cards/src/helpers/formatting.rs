//! Formatting utilities for card game UI display

use crate::card::AbilityCost;
use crate::types::ManaCost;
use crate::view::SelectOption;

/// Builds select options from enum values with optional label overrides
pub fn build_select_options(values: Vec<String>, overrides: &[(&str, &str)]) -> Vec<SelectOption> {
    values
        .into_iter()
        .map(|value| SelectOption {
            label: label_for_enum_value(&value, overrides),
            value,
        })
        .collect()
}

/// Gets a display label for an enum value, checking overrides first then using title case
pub fn label_for_enum_value(value: &str, overrides: &[(&str, &str)]) -> String {
    for (key, label) in overrides {
        if *key == value {
            return (*label).to_string();
        }
    }
    title_case(value)
}

/// Converts snake_case to Title Case
pub fn title_case(value: &str) -> String {
    let mut out = String::new();
    for (idx, part) in value.split('_').enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    out
}

/// Formats a mana cost as human-readable strings (e.g. "3 Neon", "2 Glitch")
pub fn format_mana_cost(cost: &ManaCost) -> Vec<String> {
    let mut parts = Vec::new();
    if cost.neon > 0 {
        parts.push(format!("{} Neon", cost.neon));
    }
    if cost.glitch > 0 {
        parts.push(format!("{} Glitch", cost.glitch));
    }
    if cost.chrome > 0 {
        parts.push(format!("{} Chrome", cost.chrome));
    }
    if cost.slag > 0 {
        parts.push(format!("{} Slag", cost.slag));
    }
    if cost.bio > 0 {
        parts.push(format!("{} Bio", cost.bio));
    }
    if cost.colorless > 0 {
        parts.push(format!("{} Colorless", cost.colorless));
    }
    parts
}

/// Formats an ability cost as a human-readable string combining mana, tap, and life costs
pub fn format_ability_cost(cost: &AbilityCost) -> Option<String> {
    use crate::card::Cost;

    let mut parts = Vec::new();

    for cost_item in &cost.costs {
        match cost_item {
            Cost::Mana(mana) => {
                parts.extend(format_mana_cost(mana));
            }
            Cost::Tap => {
                parts.push("Tap".to_string());
            }
            Cost::TapOther(count) => {
                parts.push(format!("Tap {} other", count));
            }
            Cost::PayLife(life) => {
                parts.push(format!("Pay {} Life", life));
            }
            Cost::Sacrifice(count) => {
                parts.push(format!("Sacrifice {}", count));
            }
            Cost::Discard(count) => {
                parts.push(format!("Discard {}", count));
            }
            Cost::Exile(count) => {
                parts.push(format!("Exile {}", count));
            }
            Cost::SpendCharge(count) => {
                parts.push(if *count == 1 {
                    "Spend a Charge".to_string()
                } else {
                    format!("Spend {} Charge", count)
                });
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" + "))
    }
}
