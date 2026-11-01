//! Bots and the API deploy separately, so a bot can send a display variant
//! the running API does not know yet. Reading it as `Unknown` keeps the rest
//! of the page; a known variant with a malformed body is still an error.

use serde::{Deserialize, Deserializer, de::DeserializeOwned, de::Error};
use serde_json::Value;

pub(crate) trait Tolerant: DeserializeOwned {
    const KIND: &'static str;

    fn unknown(name: String) -> Self;
}

pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Tolerant,
{
    let value = Value::deserialize(deserializer)?;
    let tag = match &value {
        Value::String(tag) => Some(tag.clone()),
        Value::Object(map) if map.len() == 1 => map.keys().next().cloned(),
        _ => None,
    };
    T::deserialize(value).or_else(|err| match tag {
        Some(tag) if is_unknown_variant(&err, &tag) => {
            tracing::warn!(variant = %tag, "skipping a workflow {} this build does not know: {tag}", T::KIND);
            Ok(T::unknown(tag))
        }
        _ => Err(D::Error::custom(err)),
    })
}

fn is_unknown_variant(err: &serde_json::Error, tag: &str) -> bool {
    err.to_string()
        .starts_with(&format!("unknown variant `{tag}`"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{DisplayType, DisplayVisual, WorkflowDisplay, WorkflowVisual};

    #[test]
    fn an_unknown_display_reads_as_unknown_and_keeps_its_siblings() {
        let display: WorkflowDisplay = serde_json::from_value(json!({
            "type": { "Div": { "content": [
                { "type": { "WordCloud": { "items_key": "words" } } },
                { "type": { "Text": { "text": "still here" } } },
                { "type": "SomeUnitDisplay" },
            ] } }
        }))
        .expect("tolerated");
        let DisplayType::Div { content } = display.r#type else {
            panic!("not a Div")
        };
        assert!(matches!(&content[0].r#type, DisplayType::Unknown { name } if name == "WordCloud"));
        assert!(matches!(&content[1].r#type, DisplayType::Text { text } if text == "still here"));
        assert!(
            matches!(&content[2].r#type, DisplayType::Unknown { name } if name == "SomeUnitDisplay")
        );
    }

    #[test]
    fn a_known_display_with_a_bad_body_is_still_an_error() {
        let err = serde_json::from_value::<WorkflowDisplay>(json!({
            "type": { "Text": { "txt": "typo" } }
        }))
        .expect_err("malformed Text");
        assert!(err.to_string().contains("missing field `text`"), "{err}");
    }

    #[test]
    fn an_unknown_visual_reads_as_unknown() {
        let visual: WorkflowVisual = serde_json::from_value(json!({
            "id": "v",
            "display": { "Hologram": { "content": [] } }
        }))
        .expect("tolerated");
        assert!(matches!(visual.display, DisplayVisual::Unknown { name } if name == "Hologram"));
    }

    #[test]
    fn unknown_round_trips() {
        let display = WorkflowDisplay {
            r#type: DisplayType::Unknown {
                name: "WordCloud".into(),
            },
            mark: None,
        };
        let back: WorkflowDisplay =
            serde_json::from_value(serde_json::to_value(&display).unwrap()).unwrap();
        assert!(matches!(back.r#type, DisplayType::Unknown { name } if name == "WordCloud"));
    }
}
