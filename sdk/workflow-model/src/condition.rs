//! How a node's condition is checked against an instance's responses and
//! context. The engine and devhost both use this.

use std::collections::HashMap;

use crate::NodeCondition;

pub fn eval_condition(
    cond: &NodeCondition,
    responses: &HashMap<String, serde_json::Value>,
    context: &HashMap<String, serde_json::Value>,
) -> bool {
    use serde_json::Value;
    match cond {
        NodeCondition::Always => true,
        NodeCondition::ResponseExists(field) => nested_value(responses, field).is_some(),
        NodeCondition::Gte { field, value } => match (nested_value(responses, field), value) {
            (Some(Value::Number(a)), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
                (Some(a), Some(b)) => a >= b,
                _ => false,
            },
            (Some(Value::String(a)), Value::String(b)) => a >= b,
            (Some(Value::Bool(a)), Value::Bool(b)) => *a || !*b,
            _ => false,
        },
        NodeCondition::ContextEquals { field, value } => {
            nested_value(context, field) == Some(value)
        }
        NodeCondition::ResponseEquals { field, value } => {
            nested_value(responses, field) == Some(value)
        }
        NodeCondition::ResponseListEmpty(field) => match nested_value(responses, field) {
            Some(Value::Array(items)) => items.is_empty(),
            _ => true,
        },
        NodeCondition::SomeInList {
            list,
            r#as,
            condition,
        } => match nested_value(responses, list) {
            Some(Value::Array(items)) => items.iter().any(|item| {
                let mut scoped = responses.clone();
                scoped.insert(r#as.clone(), item.clone());
                eval_condition(condition, &scoped, context)
            }),
            _ => false,
        },
        NodeCondition::Or(conds) => conds.iter().any(|c| eval_condition(c, responses, context)),
        NodeCondition::And(conds) => conds.iter().all(|c| eval_condition(c, responses, context)),
        NodeCondition::Not(inner) => !eval_condition(inner, responses, context),
    }
}

pub fn nested_value<'a>(
    map: &'a HashMap<String, serde_json::Value>,
    dotted_path: &str,
) -> Option<&'a serde_json::Value> {
    let mut parts = dotted_path.split('.');
    let mut current = map.get(parts.next()?)?;
    for part in parts {
        match current {
            serde_json::Value::Object(obj) => current = obj.get(part)?,
            _ => return None,
        }
    }
    Some(current)
}
