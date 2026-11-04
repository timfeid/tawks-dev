//! Labelled dropdowns: `InputType::Select` with fixed `options` or an
//! `option_label`. What a client submits for one is checked here against the
//! choices the server can know, and desktop builds that bundle an older
//! renderer get a working stand-in, since the server can't tell who is asking.
//! Older `Select`s, which draw their choices with `content`, are left alone.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Value, json};

use crate::condition::{eval_condition, nested_value};
use crate::{NodeCondition, ui};

/// The first renderer that draws `options`, `label` and `option_label`.
pub const DROPDOWN_RENDERER: u32 = 3;

/// How many choices the old-client hint and a refusal spell out.
const NAMED_CHOICES: usize = 12;

struct Dropdown<'a> {
    id: &'a str,
    required: bool,
    label: Option<&'a str>,
    select: &'a Value,
}

fn is_dropdown(select: &Value) -> bool {
    select
        .get("options")
        .and_then(Value::as_array)
        .is_some_and(|options| !options.is_empty())
        || select.get("option_label").and_then(Value::as_str).is_some()
}

fn select_of(input: &Value) -> Option<&Value> {
    input
        .get("input")?
        .get("type")?
        .get("Select")
        .filter(|select| is_dropdown(select))
}

fn dropdowns<'a>(value: &'a Value, found: &mut Vec<Dropdown<'a>>) {
    match value {
        Value::Array(items) => items.iter().for_each(|item| dropdowns(item, found)),
        Value::Object(map) => {
            if let Some(input) = map.get("Input") {
                if let (Some(id), Some(select)) =
                    (input.get("id").and_then(Value::as_str), select_of(input))
                {
                    found.push(Dropdown {
                        id,
                        required: input["input"]["required"].as_bool().unwrap_or(false),
                        label: select.get("label").and_then(Value::as_str),
                        select,
                    });
                }
            }
            map.values().for_each(|item| dropdowns(item, found));
        }
        _ => {}
    }
}

/// Every input id on the page, and the keys its buttons carry themselves.
/// Those are what a page really sends; anything else is a press it never made.
fn page_keys(value: &Value, ids: &mut HashSet<String>, carried: &mut HashSet<String>) {
    match value {
        Value::Array(items) => items.iter().for_each(|item| page_keys(item, ids, carried)),
        Value::Object(map) => {
            if let Some(id) = map
                .get("Input")
                .and_then(|input| input.get("id"))
                .and_then(Value::as_str)
            {
                ids.insert(id.to_string());
            }
            for key in ["inputs", "input_map"] {
                if let Some(given) = map.get(key).and_then(Value::as_object) {
                    carried.extend(given.keys().cloned());
                }
            }
            map.values().for_each(|item| page_keys(item, ids, carried));
        }
        _ => {}
    }
}

fn fill(template: &str, scope: &HashMap<String, Value>) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else {
            break;
        };
        out.push_str(&rest[..start]);
        match nested_value(scope, rest[start + 1..start + len].trim()) {
            Some(Value::String(text)) => out.push_str(text),
            Some(Value::Null) | None => {}
            Some(other) => out.push_str(&other.to_string()),
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    out
}

fn text_of(select: &Value, key: &str) -> String {
    select
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// `(value, label)` of each choice, or `None` when the server can't know them,
/// such as a list that only exists inside the row of another list.
fn choices(
    select: &Value,
    responses: &HashMap<String, Value>,
    context: &HashMap<String, Value>,
) -> Option<Vec<(String, String)>> {
    if let Some(options) = select
        .get("options")
        .and_then(Value::as_array)
        .filter(|options| !options.is_empty())
    {
        return Some(
            options
                .iter()
                .map(|option| (text_of(option, "value"), text_of(option, "label")))
                .collect(),
        );
    }
    let rows = nested_value(responses, select.get("items_key")?.as_str()?)?.as_array()?;
    let name = text_of(select, "as");
    let value = text_of(select, "value");
    let label = text_of(select, "option_label");
    let filter: Option<NodeCondition> = select
        .get("filter")
        .filter(|filter| !filter.is_null())
        .and_then(|filter| serde_json::from_value(filter.clone()).ok());
    let mut scope = responses.clone();
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        scope.insert(name.clone(), row.clone());
        if filter
            .as_ref()
            .is_none_or(|filter| eval_condition(filter, &scope, context))
        {
            out.push((fill(&value, &scope), fill(&label, &scope)));
        }
    }
    Some(out)
}

/// The dropdowns under `form` whose choices are known in `scope`: `(id, choices)`.
pub fn choices_in(
    form: &Value,
    scope: &HashMap<String, Value>,
) -> Vec<(String, Vec<(String, String)>)> {
    let mut found = Vec::new();
    dropdowns(form, &mut found);
    found
        .iter()
        .filter_map(|dropdown| {
            Some((
                dropdown.id.to_string(),
                choices(dropdown.select, scope, &HashMap::new())?,
            ))
        })
        .collect()
}

fn plain(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn named(choices: &[(String, String)]) -> String {
    let shown: Vec<&str> = choices
        .iter()
        .take(NAMED_CHOICES)
        .map(|(_, label)| label.as_str())
        .collect();
    let more = choices.len().saturating_sub(NAMED_CHOICES);
    if more > 0 {
        format!("{} and {more} more", shown.join(", "))
    } else {
        shown.join(", ")
    }
}

fn name_of<'a>(dropdown: &Dropdown<'a>) -> String {
    match dropdown.label.filter(|label| !label.trim().is_empty()) {
        Some(label) => label.to_string(),
        None => dropdown
            .id
            .rsplit('.')
            .next()
            .unwrap_or(dropdown.id)
            .replace('_', " "),
    }
}

/// The stored value of what was sent, or why it can't stand. A stand-in on an
/// older build is typed by hand, so a label or a different case is understood.
fn settle(
    dropdown: &Dropdown,
    given: &Value,
    known: Option<&[(String, String)]>,
) -> Result<Option<String>, String> {
    let name = name_of(dropdown);
    if given.is_null() || given.as_str().is_some_and(|text| text.trim().is_empty()) {
        return if dropdown.required {
            Err(format!("Pick a value for {name}."))
        } else {
            Ok(None)
        };
    }
    let Some(text) = plain(given) else {
        return Err(format!("{name} takes one of its choices."));
    };
    let Some(known) = known else { return Ok(None) };
    if known.iter().any(|(value, _)| *value == text) {
        return Ok(None);
    }
    let typed = text.trim();
    if let Some((value, _)) = known.iter().find(|(value, label)| {
        value.eq_ignore_ascii_case(typed) || label.eq_ignore_ascii_case(typed)
    }) {
        return Ok(Some(value.clone()));
    }
    Err(format!(
        "{typed} is not one of the choices for {name}. Pick from: {}.",
        named(known)
    ))
}

fn value_at<'a>(inputs: &'a mut HashMap<String, Value>, path: &[String]) -> Option<&'a mut Value> {
    let (head, rest) = path.split_first()?;
    let mut at = inputs.get_mut(head)?;
    for part in rest {
        at = at.as_object_mut()?.get_mut(part)?;
    }
    Some(at)
}

/// Where a press holds a value for dropdown `id`. Besides the id itself, a rule
/// reads an input by its bare name or under any rule's name, so a press that
/// sends it that way is checked too, unless the page really sends that key.
fn places(
    id: &str,
    inputs: &HashMap<String, Value>,
    ids: &HashSet<String>,
    carried: &HashSet<String>,
) -> Vec<Vec<String>> {
    let leaf = id.rsplit('.').next().unwrap_or(id);
    let mut found: Vec<Vec<String>> = Vec::new();
    if nested_value(inputs, id).is_some() {
        found.push(id.split('.').map(str::to_string).collect());
    }
    if id != leaf && inputs.contains_key(leaf) && !ids.contains(leaf) && !carried.contains(leaf) {
        found.push(vec![leaf.to_string()]);
    }
    for (key, value) in inputs {
        let path = format!("{key}.{leaf}");
        if path != id
            && value
                .as_object()
                .is_some_and(|object| object.contains_key(leaf))
            && !ids.contains(&path)
            && !carried.contains(key)
        {
            found.push(vec![key.clone(), leaf.to_string()]);
        }
    }
    found
}

/// Checks what a client sent for the dropdowns on these nodes. Values a stand-in
/// spelled differently are replaced by the choice they name. The refusal is
/// written for the person pressing.
pub fn check_submitted(
    nodes: &Value,
    inputs: &mut HashMap<String, Value>,
    responses: &HashMap<String, Value>,
    context: &HashMap<String, Value>,
) -> Result<(), String> {
    if inputs.is_empty() {
        return Ok(());
    }
    let mut found = Vec::new();
    dropdowns(nodes, &mut found);
    if found.is_empty() {
        return Ok(());
    }
    let mut ids = HashSet::new();
    let mut carried = HashSet::new();
    page_keys(nodes, &mut ids, &mut carried);

    let mut grouped: BTreeMap<&str, Vec<&Dropdown>> = BTreeMap::new();
    for dropdown in &found {
        grouped.entry(dropdown.id).or_default().push(dropdown);
    }
    for (id, group) in grouped {
        for dropdown in &group {
            // Otherwise an earlier press could write the rows the next one is checked against.
            let root = text_of(dropdown.select, "items_key");
            let root = root.split('.').next().unwrap_or_default();
            if !root.is_empty()
                && inputs.contains_key(root)
                && !ids.contains(root)
                && !carried.contains(root)
            {
                return Err(format!("{} takes one of its choices.", name_of(dropdown)));
            }
        }
        let sets: Vec<Option<Vec<(String, String)>>> = group
            .iter()
            .map(|dropdown| choices(dropdown.select, responses, context))
            .collect();
        // Two dropdowns may share an id on different nodes; either's choices will do.
        let known: Option<Vec<(String, String)>> =
            sets.into_iter().collect::<Option<Vec<_>>>().map(|sets| {
                let mut seen = HashSet::new();
                sets.into_iter()
                    .flatten()
                    .filter(|(value, _)| seen.insert(value.clone()))
                    .collect()
            });
        for path in places(id, inputs, &ids, &carried) {
            let Some(at) = value_at(inputs, &path) else {
                continue;
            };
            if let Some(value) = settle(group[0], at, known.as_deref())? {
                *at = Value::String(value);
            }
        }
    }
    Ok(())
}

fn to_value(display: crate::WorkflowDisplay) -> Value {
    serde_json::to_value(display).unwrap_or(Value::Null)
}

fn is_renderer_gate(value: &Value) -> bool {
    value
        .get("type")
        .and_then(|kind| kind.get("Condition"))
        .and_then(|condition| condition.get("condition"))
        .is_some_and(|condition| condition.to_string().contains("$client.renderer"))
}

/// What an older build shows for a `Dialog`, which it can't draw: its contents
/// in a panel on the page, with its own stand-ins for anything inside.
fn dialog_stand_in(display: &Value) -> Option<Vec<Value>> {
    let dialog = display.get("type")?.get("Dialog")?;
    let mut content = dialog.get("content")?.clone();
    add_old_client_stand_ins(&mut content);
    let mut shown = vec![to_value(ui::subheading(text_of(dialog, "title")))];
    if let Some(description) = dialog.get("description").and_then(Value::as_str) {
        shown.push(to_value(ui::fine(description)));
    }
    shown.extend(content.as_array()?.iter().cloned());
    let shown: Vec<crate::WorkflowDisplay> = serde_json::from_value(Value::Array(shown)).ok()?;
    let current: crate::WorkflowDisplay = serde_json::from_value(display.clone()).ok()?;
    Some(vec![
        to_value(ui::when(
            ui::renderer_at_least(DROPDOWN_RENDERER),
            vec![current],
        )),
        to_value(ui::when(
            NodeCondition::Not(Box::new(ui::renderer_at_least(DROPDOWN_RENDERER))),
            vec![ui::panel(shown)],
        )),
    ])
}

fn has_dropdown(display: &Value) -> bool {
    display
        .get("type")
        .and_then(|kind| kind.get("Input"))
        .and_then(select_of)
        .is_some()
}

/// What an older build shows in place of a dropdown. It already draws a dropdown
/// of rows, so those keep theirs and gain what it can't show. Fixed options have
/// nothing it can read, so it gets a text box the server holds to the choices.
fn stand_in(display: &mut Value) -> Option<Vec<Value>> {
    let older = |content| {
        ui::when(
            NodeCondition::Not(Box::new(ui::renderer_at_least(DROPDOWN_RENDERER))),
            content,
        )
    };
    let input = display.get_mut("type")?.get_mut("Input")?;
    let select = select_of(input)?.clone();
    let id = input.get("id")?.as_str()?.to_string();
    let label = select
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let placeholder = select
        .get("placeholder")
        .and_then(Value::as_str)
        .map(str::to_string);
    let options = select
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if options.is_empty() {
        let fields = input.get_mut("input")?.get_mut("type")?.get_mut("Select")?;
        if fields
            .get("content")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            fields["content"] = json!([to_value(ui::body(text_of(&select, "option_label")))]);
        }
        if fields.get("unselected_content").is_none_or(Value::is_null) {
            if let Some(placeholder) = &placeholder {
                fields["unselected_content"] = json!([to_value(ui::fine(placeholder.clone()))]);
            }
        }
        if label.is_empty() {
            return None;
        }
        return Some(vec![
            to_value(older(vec![ui::fine(label)])),
            display.clone(),
        ]);
    }
    let pairs: Vec<(String, String)> = options
        .iter()
        .map(|option| (text_of(option, "value"), text_of(option, "label")))
        .collect();
    let hint = if label.is_empty() {
        format!(
            "Type one of: {}. Update Tawks to pick from a list.",
            named(&pairs)
        )
    } else {
        format!(
            "{label}: type one of {}. Update Tawks to pick from a list.",
            named(&pairs)
        )
    };
    let text_input = json!({
        "type": { "Input": {
            "id": id,
            "input": {
                "readonly": input["input"].get("readonly").cloned().unwrap_or(Value::Null),
                "type": { "Text": { "placeholder": placeholder.or(Some(label)).filter(|text| !text.is_empty()) } },
                "default_value": input["input"].get("default_value").cloned().unwrap_or(Value::Null),
                "required": input["input"].get("required").cloned().unwrap_or(Value::Bool(false)),
                "on_change_action": Value::Null,
            }
        }}
    });
    let text_input: crate::WorkflowDisplay = serde_json::from_value(text_input).ok()?;
    let current: crate::WorkflowDisplay = serde_json::from_value(display.clone()).ok()?;
    Some(vec![
        to_value(ui::when(
            ui::renderer_at_least(DROPDOWN_RENDERER),
            vec![current],
        )),
        to_value(older(vec![ui::fine(hint), text_input])),
    ])
}

/// Gives every dropdown in a definition what older builds need. Runs on what is
/// served, not on what is stored, so the author reads back what they wrote.
pub fn add_old_client_stand_ins(value: &mut Value) {
    match value {
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for mut item in std::mem::take(items) {
                let label_is_in = out.last().is_some_and(is_renderer_gate);
                if is_renderer_gate(&item) || (label_is_in && has_dropdown(&item)) {
                    out.push(item);
                } else if let Some(replacement) =
                    dialog_stand_in(&item).or_else(|| stand_in(&mut item))
                {
                    out.extend(replacement);
                } else {
                    add_old_client_stand_ins(&mut item);
                    out.push(item);
                }
            }
            *items = out;
        }
        Value::Object(map) => map.values_mut().for_each(add_old_client_stand_ins),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::eval_condition;

    fn stakes(required: bool, default: Value) -> Value {
        json!({ "type": { "Input": { "id": "wager.stake", "input": {
            "readonly": null, "default_value": default, "required": required, "on_change_action": null,
            "type": { "Select": {
                "filter": null, "selected_content": null, "unselected_content": null,
                "label": "Stake", "placeholder": "Pick a stake",
                "options": [{ "value": "5", "label": "$5" }, { "value": "10", "label": "$10" }]
            }}
        }}}})
    }

    fn games() -> Value {
        json!({ "type": { "Input": { "id": "pick.game", "input": {
            "readonly": null, "default_value": null, "required": false, "on_change_action": null,
            "type": { "Select": {
                "filter": null, "selected_content": null, "unselected_content": null,
                "label": "Game", "placeholder": "Pick a game",
                "items_key": "data.games.rows", "as": "g", "value": "{g.id}", "option_label": "{g.away} at {g.home}"
            }}
        }}}})
    }

    fn press(
        nodes: &Value,
        inputs: Value,
        responses: Value,
    ) -> Result<HashMap<String, Value>, String> {
        let mut inputs: HashMap<String, Value> = serde_json::from_value(inputs).unwrap();
        let responses: HashMap<String, Value> = serde_json::from_value(responses).unwrap();
        check_submitted(nodes, &mut inputs, &responses, &HashMap::new()).map(|_| inputs)
    }

    #[test]
    fn a_choice_goes_through_and_a_forged_value_is_refused() {
        let nodes = json!([stakes(true, Value::Null)]);
        assert!(press(&nodes, json!({ "wager": { "stake": "5" } }), json!({})).is_ok());
        let refused =
            press(&nodes, json!({ "wager": { "stake": "9999" } }), json!({})).unwrap_err();
        assert_eq!(
            refused,
            "9999 is not one of the choices for Stake. Pick from: $5, $10."
        );
        assert!(
            press(
                &nodes,
                json!({ "wager": { "stake": { "$gt": 0 } } }),
                json!({})
            )
            .is_err()
        );
        assert!(press(&nodes, json!({ "wager": { "stake": ["5"] } }), json!({})).is_err());
    }

    #[test]
    fn a_forged_value_cant_go_round_by_the_names_a_rule_also_reads() {
        let nodes = json!([stakes(false, Value::Null)]);
        assert!(press(&nodes, json!({ "stake": "9999" }), json!({})).is_err());
        assert!(
            press(
                &nodes,
                json!({ "other_rule": { "stake": "9999" } }),
                json!({})
            )
            .is_err()
        );
    }

    #[test]
    fn another_input_or_a_buttons_own_key_with_that_name_is_not_a_forgery() {
        let text = json!({ "type": { "Input": { "id": "note.stake", "input": { "readonly": null, "default_value": null, "required": false, "on_change_action": null, "type": { "Text": { "placeholder": "x" } } } } } });
        let nodes = json!([stakes(false, Value::Null), text]);
        assert!(
            press(
                &nodes,
                json!({ "note": { "stake": "free text" } }),
                json!({})
            )
            .is_ok()
        );
        let button = json!({ "act": { "action": "a", "inputs": { "row": "{r}" } } });
        let nodes = json!([stakes(false, Value::Null), button]);
        assert!(press(&nodes, json!({ "row": { "stake": 77 } }), json!({})).is_ok());
    }

    #[test]
    fn the_same_dropdown_twice_names_each_choice_once() {
        let nodes = json!([stakes(false, Value::Null), stakes(false, Value::Null)]);
        let refused = press(&nodes, json!({ "wager": { "stake": "9" } }), json!({})).unwrap_err();
        assert_eq!(
            refused,
            "9 is not one of the choices for Stake. Pick from: $5, $10."
        );
    }

    #[test]
    fn blank_is_refused_only_when_required() {
        let required = json!([stakes(true, Value::Null)]);
        assert_eq!(
            press(&required, json!({ "wager": { "stake": "" } }), json!({})).unwrap_err(),
            "Pick a value for Stake."
        );
        let optional = json!([stakes(false, Value::Null)]);
        assert!(press(&optional, json!({ "wager": { "stake": "" } }), json!({})).is_ok());
    }

    #[test]
    fn what_a_stand_in_typed_becomes_the_choice_it_names() {
        let nodes = json!([stakes(false, Value::Null)]);
        let sent = press(&nodes, json!({ "wager": { "stake": " $10 " } }), json!({})).unwrap();
        assert_eq!(sent["wager"]["stake"], json!("10"));
    }

    #[test]
    fn rows_are_the_choices_when_the_server_holds_them() {
        let nodes = json!([games()]);
        let held = json!({ "data": { "games": { "rows": [{ "id": "g1", "away": "BUF", "home": "KC" }] } } });
        assert!(press(&nodes, json!({ "pick": { "game": "g1" } }), held.clone()).is_ok());
        let refused = press(&nodes, json!({ "pick": { "game": "g2" } }), held.clone()).unwrap_err();
        assert!(refused.contains("BUF at KC"), "{refused}");
        assert!(press(&nodes, json!({ "pick": { "game": "anything" } }), json!({})).is_ok());
    }

    #[test]
    fn a_press_cant_write_the_rows_it_is_checked_against() {
        let nodes = json!([games()]);
        let forged =
            json!({ "data": { "games": { "rows": [{ "id": "x" }] } }, "pick": { "game": "x" } });
        assert!(press(&nodes, forged, json!({})).is_err());
    }

    #[test]
    fn a_filter_hides_a_row_from_the_choices() {
        let mut select = games();
        select["type"]["Input"]["input"]["type"]["Select"]["filter"] =
            json!({ "ResponseEquals": { "field": "g.open", "value": true } });
        let held = json!({ "data": { "games": { "rows": [{ "id": "a", "open": true }, { "id": "b", "open": false }] } } });
        let nodes = json!([select]);
        assert!(press(&nodes, json!({ "pick": { "game": "a" } }), held.clone()).is_ok());
        assert!(press(&nodes, json!({ "pick": { "game": "b" } }), held).is_err());
    }

    #[test]
    fn a_select_drawn_with_content_is_not_checked() {
        let nodes = json!([{ "type": { "Input": { "id": "role", "input": { "required": true, "type": { "Select": {
            "filter": null, "items_key": "roles", "as": "r", "value": "{r.id}", "content": [], "selected_content": null, "unselected_content": null
        }}}}}}]);
        assert!(
            press(
                &nodes,
                json!({ "role": "anything" }),
                json!({ "roles": [] })
            )
            .is_ok()
        );
    }

    #[test]
    fn dropdown_builders_write_what_is_checked_and_match_the_js_sdk() {
        let fixed = crate::builder::InputTypeBuilder::dropdown(
            "Stake",
            vec![crate::SelectOption::new("5", "$5")],
            Some("Pick".into()),
        );
        let rows = crate::builder::InputTypeBuilder::dropdown_from(
            "Game", "rows", "g", "{g.id}", "{g.name}", None,
        );
        let value = serde_json::to_value([&fixed, &rows]).unwrap();
        assert!(is_dropdown(&value[0]["Select"]));
        assert!(is_dropdown(&value[1]["Select"]));
        // The `dropdowns` entry in open/sdk/bot-js/src/builder.test.ts holds this same text.
        assert_eq!(
            serde_json::to_string(&[&fixed, &rows]).unwrap(),
            r#"[{"Select":{"filter":null,"items_key":"","as":"","value":"","content":[],"selected_content":null,"unselected_content":null,"options":[{"value":"5","label":"$5"}],"option_label":null,"label":"Stake","placeholder":"Pick"}},{"Select":{"filter":null,"items_key":"rows","as":"g","value":"{g.id}","content":[],"selected_content":null,"unselected_content":null,"options":[],"option_label":"{g.name}","label":"Game","placeholder":null}}]"#
        );
    }

    fn shown_to(display: &Value, client: Value) -> Vec<Value> {
        let responses = HashMap::from([("$client".to_string(), client)]);
        let condition: NodeCondition =
            serde_json::from_value(display["type"]["Condition"]["condition"].clone()).unwrap();
        if eval_condition(&condition, &responses, &HashMap::new()) {
            display["type"]["Condition"]["content"]
                .as_array()
                .unwrap()
                .clone()
        } else {
            Vec::new()
        }
    }

    #[test]
    fn older_builds_get_a_text_box_for_fixed_options() {
        let mut page = json!({ "content": [stakes(true, json!("5"))] });
        add_old_client_stand_ins(&mut page);
        let content = page["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        let current = json!({ "desktop": false, "renderer": DROPDOWN_RENDERER });
        let new_view = shown_to(&content[0], current.clone());
        assert!(new_view[0]["type"]["Input"]["input"]["type"]["Select"].is_object());
        assert!(shown_to(&content[1], current).is_empty());
        for old in [
            json!({ "desktop": true, "os": "windows" }),
            json!({ "desktop": false, "renderer": 2 }),
        ] {
            assert!(shown_to(&content[0], old.clone()).is_empty());
            let old_view = shown_to(&content[1], old);
            assert!(
                old_view[0]
                    .to_string()
                    .contains("Stake: type one of $5, $10. Update Tawks")
            );
            let input = &old_view[1]["type"]["Input"];
            assert_eq!(input["id"], "wager.stake");
            assert_eq!(
                input["input"]["type"]["Text"]["placeholder"],
                "Pick a stake"
            );
            assert_eq!(input["input"]["required"], true);
        }
    }

    #[test]
    fn older_builds_keep_a_dropdown_of_rows_and_gain_its_words() {
        let mut page = json!([games()]);
        add_old_client_stand_ins(&mut page);
        let content = page.as_array().unwrap();
        let select = &content[1]["type"]["Input"]["input"]["type"]["Select"];
        assert!(
            select["content"][0]
                .to_string()
                .contains("{g.away} at {g.home}")
        );
        assert!(
            select["unselected_content"][0]
                .to_string()
                .contains("Pick a game")
        );
        let label = shown_to(&content[0], json!({ "desktop": true }));
        assert!(label[0].to_string().contains("Game"));
    }

    #[test]
    fn stand_ins_go_in_once_and_the_page_still_parses() {
        let mut page =
            json!([{ "type": { "Div": { "content": [stakes(false, Value::Null), games()] } } }]);
        add_old_client_stand_ins(&mut page);
        let once = page.clone();
        add_old_client_stand_ins(&mut page);
        assert_eq!(page, once);
        let _: Vec<crate::WorkflowDisplay> = serde_json::from_value(page).unwrap();
    }

    #[test]
    fn older_builds_show_a_dialog_inline_with_a_stand_in_for_what_it_holds() {
        let dialog = serde_json::to_value(
            ui::dialog(
                "Start a wager",
                "New wager",
                vec![serde_json::from_value(stakes(false, Value::Null)).unwrap()],
            )
            .drawer(),
        )
        .unwrap();
        let mut page = json!([dialog]);
        add_old_client_stand_ins(&mut page);
        let content = page.as_array().unwrap();
        let current = json!({ "desktop": false, "renderer": DROPDOWN_RENDERER });
        let old = json!({ "desktop": true });
        let new_view = shown_to(&content[0], current.clone());
        assert_eq!(new_view[0]["type"]["Dialog"]["style"], "Drawer");
        assert!(
            new_view[0]["type"]["Dialog"]["content"][0]["type"]["Input"].is_object(),
            "the current renderer draws the dropdown itself"
        );
        assert!(shown_to(&content[0], old.clone()).is_empty());
        assert!(shown_to(&content[1], current).is_empty());
        let inline = shown_to(&content[1], old.clone());
        let held = inline[0]["type"]["Panel"]["content"].as_array().unwrap();
        assert!(held[0].to_string().contains("New wager"));
        let old_text = shown_to(&held[2], old);
        assert!(old_text[1]["type"]["Input"]["input"]["type"]["Text"].is_object());
        let once = page.clone();
        add_old_client_stand_ins(&mut page);
        assert_eq!(page, once);
    }

    #[test]
    fn a_long_list_is_named_only_in_part() {
        let many: Vec<(String, String)> =
            (0..20).map(|n| (n.to_string(), format!("o{n}"))).collect();
        assert!(named(&many).ends_with("o11 and 8 more"));
    }
}
