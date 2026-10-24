use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use specta::Type;
use std::collections::HashMap;

enum PathRef<'a> {
    Responses(&'a str),
    Context(&'a str),
}

fn parse_path(path: &str) -> PathRef<'_> {
    if let Some(rest) = path.strip_prefix("$context.") {
        PathRef::Context(rest)
    } else {
        PathRef::Responses(path)
    }
}

#[derive(Debug, Clone)]
pub enum TransformEffect {
    RunServerAction {
        action_id: String,
        input_map: Option<HashMap<String, String>>,
        reconcile_node: bool,
    },
    // StartWorkflow {
    //     workflow_id: String,
    //     input_map: Option<HashMap<String, String>>,
    //     context: HashMap<String, Value>,
    //     responses: HashMap<String, Value>,
    //     complete_workflow: bool,
    //     inject_place: Option<InjectWorkflowPlace>,
    // },
    Cancel, // close modal / end injected workflow
    NextNode,
    PreviousNode,
    Noop,
}

// transform.rs
pub fn apply_input_map_into_responses(
    responses: &mut HashMap<String, Value>,
    context: &HashMap<String, Value>,
    input_map: &HashMap<String, String>,
) {
    for (key, path) in input_map.iter() {
        let v = get_path_any(responses, context, path)
            .cloned()
            .unwrap_or(json!(null));
        responses.insert(key.clone(), v);
    }
}

pub fn get_path_any<'a>(
    responses: &'a HashMap<String, Value>,
    context: &'a HashMap<String, Value>,
    path: &str,
) -> Option<&'a Value> {
    match parse_path(path) {
        PathRef::Responses(p) => get_path(responses, p),
        PathRef::Context(p) => get_path(context, p),
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum TransformCond {
    Always,
    Not(Box<TransformCond>),
    And(Vec<TransformCond>),
    Or(Vec<TransformCond>),

    Exists { path: String },
    Equals { path: String, value: Value },
    ListNotEmpty { path: String },
    EqualsPath { left: String, right: String },
}

impl TransformCond {
    pub fn eval(
        &self,
        responses: &HashMap<String, Value>,
        context: &HashMap<String, Value>,
    ) -> bool {
        match self {
            TransformCond::EqualsPath { left, right } => {
                match (
                    get_path_any(responses, context, left),
                    get_path_any(responses, context, right),
                ) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                }
            }
            TransformCond::Always => true,
            TransformCond::Not(inner) => !inner.eval(responses, context),

            TransformCond::And(list) => list.iter().all(|c| c.eval(responses, context)),
            TransformCond::Or(list) => list.iter().any(|c| c.eval(responses, context)),
            TransformCond::Exists { path } => get_path_any(responses, context, path).is_some(),

            TransformCond::Equals { path, value } => get_path_any(responses, context, path)
                .map(|v| v == value)
                .unwrap_or(false),

            TransformCond::ListNotEmpty { path } => match get_path_any(responses, context, path) {
                Some(Value::Array(a)) => !a.is_empty(),
                _ => false,
            },
        }
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum TransformOp {
    When {
        cond: TransformCond,
        ops: Vec<TransformOp>,
        else_ops: Vec<TransformOp>,
    },

    Set {
        path: String,
        value: Value,
    },

    Remove {
        path: String,
    },

    MergeObject {
        path: String,
        value: Map<String, Value>,
    },

    Append {
        path: String,
        value: Value,
    },

    AppendUnique {
        path: String,
        value: Value,
    },

    FilterArray {
        path: String,
        as_key: String,
        predicate: TransformCond,
    },

    MapArray {
        path: String,
        as_key: String,
        template: Value,
    },

    Copy {
        from: String,
        to: String,
    },

    Move {
        from: String,
        to: String,
    },

    EmitRunServerAction {
        action_id: String,
        input_map: Option<HashMap<String, String>>,
        reconcile_node: bool,
    },

    EmitCancel,
    EmitNextNode,
    EmitPreviousNode,
    // todo
    // EmitStartWorkflow {
    // },
}

pub fn apply_transform_ops(
    responses: &mut HashMap<String, Value>,
    ops: &[TransformOp],
    context: &mut HashMap<String, Value>,
) -> Result<Vec<TransformEffect>, String> {
    let mut effects = Vec::new();

    for op in ops {
        apply_one(responses, context, op, &mut effects)?;
    }
    Ok(effects)
}

fn apply_one(
    responses: &mut HashMap<String, Value>,
    context: &mut HashMap<String, Value>,
    op: &TransformOp,
    effects: &mut Vec<TransformEffect>,
) -> Result<(), String> {
    match op {
        TransformOp::EmitRunServerAction {
            action_id,
            input_map,
            reconcile_node,
        } => {
            effects.push(TransformEffect::RunServerAction {
                action_id: action_id.clone(),
                input_map: input_map.clone(),
                reconcile_node: *reconcile_node,
            });
        }

        TransformOp::EmitCancel => {
            effects.push(TransformEffect::Cancel);
        }
        TransformOp::EmitNextNode => {
            effects.push(TransformEffect::NextNode);
        }
        TransformOp::EmitPreviousNode => {
            effects.push(TransformEffect::PreviousNode);
        }

        TransformOp::When {
            cond,
            ops,
            else_ops,
        } => {
            let branch = if cond.eval(responses, context) {
                ops
            } else {
                else_ops
            };
            for inner in branch {
                apply_one(responses, context, inner, effects)?;
            }
        }
        TransformOp::Set { path, value } => {
            set_path_any(responses, context, path, value.clone())?;
        }
        TransformOp::Remove { path } => {
            remove_path_any(responses, context, path)?;
        }
        TransformOp::MergeObject { path, value } => {
            let existing = get_path_mut_any(responses, context, path);
            match existing {
                Some(Value::Object(obj)) => {
                    for (k, v) in value.iter() {
                        obj.insert(k.clone(), v.clone());
                    }
                }
                _ => {
                    set_path_any(responses, context, path, Value::Object(value.clone()))?;
                }
            }
        }
        TransformOp::Append { path, value } => {
            let v = ensure_array_at_path_any(responses, context, path)?;
            v.push(value.clone());
        }
        TransformOp::AppendUnique { path, value } => {
            let v = ensure_array_at_path_any(responses, context, path)?;
            if !v.iter().any(|x| x == value) {
                v.push(value.clone());
            }
        }
        TransformOp::FilterArray {
            path,
            as_key,
            predicate,
        } => {
            let arr = match get_path_any(responses, context, path) {
                Some(Value::Array(a)) => a.clone(),
                None => vec![],
                _ => return Err(format!("FilterArray target is not an array: {path}")),
            };

            let mut out = Vec::new();
            for item in arr {
                // keep your “temp responses” approach, but predicates can see context via eval(..., context)
                let mut temp = responses.clone();
                temp.insert(as_key.clone(), item.clone());
                if predicate.eval(&temp, context) {
                    out.push(item);
                }
            }

            set_path_any(responses, context, path, Value::Array(out))?;
        }
        TransformOp::MapArray {
            path,
            as_key,
            template,
        } => {
            let arr = match get_path_any(responses, context, path) {
                Some(Value::Array(a)) => a.clone(),
                None => vec![],
                _ => return Err(format!("MapArray target is not an array: {path}")),
            };

            let mut out = Vec::new();
            for item in arr {
                let mut temp = responses.clone();
                temp.insert(as_key.clone(), item.clone());
                let mapped = interpolate_template_any(template, &temp, context);
                out.push(mapped);
            }

            set_path_any(responses, context, path, Value::Array(out))?;
        }
        TransformOp::Copy { from, to } => {
            if let Some(v) = get_path_any(responses, context, from).cloned() {
                set_path_any(responses, context, to, v)?;
            }
        }
        TransformOp::Move { from, to } => {
            if let Some(v) = get_path_any(responses, context, from).cloned() {
                set_path_any(responses, context, to, v)?;
                remove_path_any(responses, context, from)?;
            }
        }
    }

    Ok(())
}

///

///

fn split_path(path: &str) -> Vec<&str> {
    path.split('.').filter(|s| !s.is_empty()).collect()
}

fn parse_index(seg: &str) -> Option<usize> {
    seg.parse::<usize>().ok()
}

pub fn get_path<'a>(root: &'a HashMap<String, Value>, path: &str) -> Option<&'a Value> {
    let parts = split_path(path);
    let (first, rest) = parts.split_first()?;
    let mut cur = root.get(*first)?;

    for seg in rest {
        match cur {
            Value::Object(obj) => cur = obj.get(*seg)?,
            Value::Array(arr) => {
                let idx = parse_index(seg)?;
                cur = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

pub fn get_path_mut<'a>(root: &'a mut HashMap<String, Value>, path: &str) -> Option<&'a mut Value> {
    let parts = split_path(path);
    let (first, rest) = parts.split_first()?;
    let mut cur = root.get_mut(*first)?;

    for seg in rest {
        match cur {
            Value::Object(obj) => cur = obj.get_mut(*seg)?,
            Value::Array(arr) => {
                let idx = parse_index(seg)?;
                cur = arr.get_mut(idx)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

pub fn set_path(root: &mut HashMap<String, Value>, path: &str, value: Value) -> Result<(), String> {
    let parts = split_path(path);
    let (first, rest) = parts
        .split_first()
        .ok_or_else(|| "empty path".to_string())?;

    if rest.is_empty() {
        root.insert((*first).to_string(), value);
        return Ok(());
    }

    if !root.contains_key(*first) {
        let next = rest[0];
        if parse_index(next).is_some() {
            root.insert((*first).to_string(), Value::Array(vec![]));
        } else {
            root.insert((*first).to_string(), Value::Object(Map::new()));
        }
    }

    let mut cur = root
        .get_mut(*first)
        .ok_or_else(|| "failed to get root segment".to_string())?;

    for (i, seg) in rest.iter().enumerate() {
        let is_last = i == rest.len() - 1;
        if is_last {
            match cur {
                Value::Object(obj) => {
                    obj.insert((*seg).to_string(), value);
                    return Ok(());
                }
                Value::Array(arr) => {
                    let idx =
                        parse_index(seg).ok_or_else(|| format!("expected array index at {seg}"))?;
                    if idx >= arr.len() {
                        arr.resize(idx + 1, Value::Null);
                    }
                    arr[idx] = value;
                    return Ok(());
                }
                _ => return Err(format!("cannot set into non-container at segment {seg}")),
            }
        } else {
            let next_seg = rest[i + 1];
            let next_is_index = parse_index(next_seg).is_some();

            match cur {
                Value::Object(obj) => {
                    if !obj.contains_key(*seg) {
                        obj.insert(
                            (*seg).to_string(),
                            if next_is_index {
                                Value::Array(vec![])
                            } else {
                                Value::Object(Map::new())
                            },
                        );
                    }
                    cur = obj.get_mut(*seg).unwrap();
                }
                Value::Array(arr) => {
                    let idx =
                        parse_index(seg).ok_or_else(|| format!("expected array index at {seg}"))?;
                    if idx >= arr.len() {
                        arr.resize(idx + 1, Value::Null);
                    }
                    if arr[idx].is_null() {
                        arr[idx] = if next_is_index {
                            Value::Array(vec![])
                        } else {
                            Value::Object(Map::new())
                        };
                    }
                    cur = &mut arr[idx];
                }
                _ => return Err(format!("cannot traverse non-container at segment {seg}")),
            }
        }
    }

    Ok(())
}

fn set_path_any(
    responses: &mut HashMap<String, Value>,
    context: &mut HashMap<String, Value>,
    path: &str,
    value: Value,
) -> Result<(), String> {
    match parse_path(path) {
        PathRef::Responses(p) => set_path(responses, p, value),
        PathRef::Context(p) => set_path(context, p, value),
    }
}

fn get_path_mut_any<'a>(
    responses: &'a mut HashMap<String, Value>,
    context: &'a mut HashMap<String, Value>,
    path: &str,
) -> Option<&'a mut Value> {
    match parse_path(path) {
        PathRef::Responses(p) => get_path_mut(responses, p),
        PathRef::Context(p) => get_path_mut(context, p),
    }
}

fn remove_path_any(
    responses: &mut HashMap<String, Value>,
    context: &mut HashMap<String, Value>,
    path: &str,
) -> Result<(), String> {
    match parse_path(path) {
        PathRef::Responses(p) => remove_path(responses, p),
        PathRef::Context(p) => remove_path(context, p),
    }
}

pub fn remove_path(root: &mut HashMap<String, Value>, path: &str) -> Result<(), String> {
    let parts = split_path(path);
    let (first, rest) = parts
        .split_first()
        .ok_or_else(|| "empty path".to_string())?;

    if rest.is_empty() {
        root.remove(*first);
        return Ok(());
    }

    let mut cur = match root.get_mut(*first) {
        Some(v) => v,
        None => return Ok(()),
    };

    for (i, seg) in rest.iter().enumerate() {
        let is_last = i == rest.len() - 1;
        if is_last {
            match cur {
                Value::Object(obj) => {
                    obj.remove(*seg);
                }
                Value::Array(arr) => {
                    let idx = match parse_index(seg) {
                        Some(i) => i,
                        None => return Ok(()),
                    };
                    if idx < arr.len() {
                        arr[idx] = Value::Null;
                    }
                }
                _ => {}
            }
            return Ok(());
        } else {
            match cur {
                Value::Object(obj) => match obj.get_mut(*seg) {
                    Some(v) => cur = v,
                    None => return Ok(()),
                },
                Value::Array(arr) => {
                    let idx = match parse_index(seg) {
                        Some(i) => i,
                        None => return Ok(()),
                    };
                    match arr.get_mut(idx) {
                        Some(v) => cur = v,
                        None => return Ok(()),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    Ok(())
}

fn ensure_array_at_path_any<'a>(
    responses: &'a mut HashMap<String, Value>,
    context: &'a mut HashMap<String, Value>,
    path: &str,
) -> Result<&'a mut Vec<Value>, String> {
    match parse_path(path) {
        PathRef::Responses(p) => ensure_array_at_path(responses, p),
        PathRef::Context(p) => ensure_array_at_path(context, p),
    }
}

fn ensure_array_at_path<'a>(
    root: &'a mut HashMap<String, Value>,
    path: &str,
) -> Result<&'a mut Vec<Value>, String> {
    if get_path(root, path).is_none() {
        set_path(root, path, Value::Array(vec![]))?;
    }

    match get_path_mut(root, path) {
        Some(Value::Array(arr)) => Ok(arr),
        Some(_) => Err(format!("expected array at {path}")),
        None => Err(format!("missing path {path}")),
    }
}

///
///
///

fn interpolate_template_any(
    template: &Value,
    responses: &HashMap<String, Value>,
    context: &HashMap<String, Value>,
) -> Value {
    match template {
        Value::String(s) => Value::String(interpolate_str_any(s, responses, context)),
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| interpolate_template_any(v, responses, context))
                .collect(),
        ),
        Value::Object(obj) => {
            let mut out = Map::new();
            for (k, v) in obj.iter() {
                out.insert(k.clone(), interpolate_template_any(v, responses, context));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn interpolate_str_any(
    s: &str,
    responses: &HashMap<String, Value>,
    context: &HashMap<String, Value>,
) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            let mut key = String::new();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == '}' {
                    break;
                }
                key.push(nc);
            }

            let key = key.trim();
            if key.is_empty() {
                out.push_str("{}");
                continue;
            }

            let replacement = match get_path_any(responses, context, key) {
                Some(Value::String(v)) => v.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Bool(b)) => b.to_string(),
                Some(Value::Null) | None => "".to_string(),
                Some(other) => other.to_string(),
            };

            out.push_str(&replacement);
        } else {
            out.push(c);
        }
    }

    out
}
