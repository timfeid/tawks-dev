//! Keeps each PuckLab jersey owner's Tawkathon shirt in step with their Lab
//! club's home uniform. Only colours, stripes and PuckLab's own Jersey Lab
//! emblems leave PuckLab; uploaded crest images never do, since they could
//! be anyone's logo.
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use anyhow::Result;
use serde_json::Value;
use tawks_bot::TawksBot;
use tawks_game_model::pucklab_jersey::{
    CREST_PATHS_MAX, Jersey, JerseyCrest, JerseyStripe, PuckLabJerseyReport, STRIPES_MAX,
    StripePattern, color,
};

use crate::client::{ApiError, PuckLab};

static CHANGED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

/// Someone saved their uniforms through Tawks; report them on the next pass.
pub fn changed(subject: &str) {
    if let Ok(mut changed) = CHANGED.lock() {
        changed.insert(subject.to_owned());
    }
}

fn hex(value: &Value) -> Option<String> {
    value.as_str().and_then(|v| color(v).ok())
}

fn pattern(value: &Value) -> Option<StripePattern> {
    serde_json::from_value(value.clone()).ok()
}

fn stripes(list: &Value) -> Vec<JerseyStripe> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|stripe| {
            Some(JerseyStripe {
                pattern: pattern(&stripe["pattern"])?,
                color: hex(&stripe["color"])?,
            })
        })
        .take(STRIPES_MAX)
        .collect()
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// A built-in emblem's SVG as path data: its viewBox and every `d`.
pub fn emblem(svg: &str, fill: String, back: Option<String>) -> Option<JerseyCrest> {
    let numbers: Vec<f32> = attribute(svg, "viewBox")?
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let view_box: [f32; 4] = numbers.try_into().ok()?;
    let paths: Vec<String> = svg
        .match_indices(" d=\"")
        .filter_map(|(at, _)| attribute(&svg[at + 1..], "d"))
        .map(|d| d.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    if paths.is_empty() || paths.len() > CREST_PATHS_MAX {
        return None;
    }
    Some(JerseyCrest::Emblem {
        paths,
        view_box,
        color: fill,
        back,
    })
}

/// The home uniform of a `/v1/editors/jersey` answer, as a Tawks design.
/// None until the club has saved a uniform.
pub fn design(editor: &Value) -> Option<Jersey> {
    let uniforms = editor["uniforms"].as_array()?;
    let home = uniforms
        .iter()
        .find(|u| {
            u["name"]
                .as_str()
                .is_some_and(|n| n.trim().eq_ignore_ascii_case("home"))
        })
        .or_else(|| uniforms.first())?;
    let recipe = &home["recipe"];
    if !recipe.is_object() {
        return None;
    }
    let numbers = &recipe["numbers"];
    let crest = recipe["crest"]
        .as_str()
        .filter(|c| c.starts_with("emblem:"))
        .and_then(|_| home["crest_svg"].as_str())
        .and_then(|svg| {
            emblem(
                svg,
                hex(&recipe["crest_color"]).unwrap_or_else(|| "#FFFFFF".into()),
                hex(&recipe["crest_back"]),
            )
        });
    let jersey = Jersey {
        body: hex(&recipe["jersey"]).unwrap_or_else(|| "#23333F".into()),
        sleeves: hex(&recipe["sleeves"]),
        yoke: hex(&recipe["yoke"]),
        collar: hex(&recipe["collar"]).or_else(|| hex(&numbers["outline"])),
        trim: hex(&recipe["crest_color"]).or_else(|| hex(&numbers["fill"])),
        sleeve_stripes: stripes(&recipe["sleeve_stripes"]),
        hem_stripes: stripes(&recipe["hem_stripes"]),
        crest,
    };
    jersey
        .clone()
        .sanitized()
        .or_else(|_| {
            Jersey {
                crest: None,
                ..jersey
            }
            .sanitized()
        })
        .ok()
}

/// Ok(None) for "no design": no club uniform, or no linked PuckLab account.
/// Err when PuckLab could not answer, so Tawks asks again later.
async fn read(api: &PuckLab, roster: &str, subject: &str) -> Result<Option<Jersey>> {
    match api
        .get(subject, "/v1/editors/jersey", &[("roster", roster)], None)
        .await
    {
        Ok(editor) => Ok(design(&editor)),
        Err(error)
            if error
                .downcast_ref::<ApiError>()
                .is_some_and(|e| e.pucklab_link_required || e.status == 404) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

/// One pass over the owners Tawks asked about and anyone who just saved.
pub async fn sync(bot: &TawksBot, api: &PuckLab, roster: &str) -> Result<usize> {
    let mut subjects: Vec<String> = bot.pucklab_jersey_checks().await?;
    let saved: Vec<String> = CHANGED
        .lock()
        .map(|mut c| c.drain().collect())
        .unwrap_or_default();
    for subject in saved {
        if !subjects.contains(&subject) {
            subjects.push(subject);
        }
    }
    let mut reports = Vec::new();
    for user_id in subjects {
        match read(api, roster, &user_id).await {
            Ok(jersey) => reports.push(PuckLabJerseyReport { user_id, jersey }),
            Err(error) => tracing::warn!(%error, "PuckLab jersey read failed"),
        }
    }
    for chunk in reports.chunks(tawks_game_model::pucklab_jersey::REPORTS_MAX) {
        bot.report_pucklab_jerseys(chunk).await?;
    }
    Ok(reports.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_home_uniform_becomes_the_design() {
        let editor = json!({"uniforms": [
            {"name": "Away", "recipe": {"jersey": "#ffffff"}},
            {"name": "Home", "recipe": {"jersey": "#123b74", "sleeves": "#0a0a0a", "yoke": "bad",
                "numbers": {"fill": "#ffcc00", "outline": "#000000"},
                "sleeve_stripes": [{"pattern": "double", "color": "#c93845"}, {"pattern": "zigzag", "color": "#ffffff"}],
                "hem_stripes": [{"pattern": "bold", "color": "#88ccff"}]}}
        ]});
        let jersey = design(&editor).unwrap();
        assert_eq!(jersey.body, "#123B74");
        assert_eq!(jersey.sleeves.as_deref(), Some("#0A0A0A"));
        assert_eq!(jersey.yoke, None);
        assert_eq!(jersey.collar.as_deref(), Some("#000000"));
        assert_eq!(jersey.trim.as_deref(), Some("#FFCC00"));
        assert_eq!(
            jersey.sleeve_stripes.len(),
            1,
            "unknown patterns are dropped"
        );
        assert_eq!(jersey.hem_stripes[0].pattern, StripePattern::Bold);
        assert!(jersey.crest.is_none());
    }

    #[test]
    fn only_built_in_emblems_become_crests() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 15 15"><path d="M7.5 0L15 15H0Z"/><path fill="#000" d="M1 1h2v2H1z"/></svg>"##;
        let emblem = json!({"uniforms": [{"name": "Home", "recipe": {"jersey": "#101820", "crest": "emblem:harbor", "crest_color": "#50e1d5", "crest_back": "#ffffff"}, "crest_svg": svg}]});
        match design(&emblem).unwrap().crest {
            Some(JerseyCrest::Emblem {
                paths,
                view_box,
                color,
                back,
            }) => {
                assert_eq!(paths, vec!["M7.5 0L15 15H0Z", "M1 1h2v2H1z"]);
                assert_eq!(view_box, [0.0, 0.0, 15.0, 15.0]);
                assert_eq!(color, "#50E1D5");
                assert_eq!(back.as_deref(), Some("#FFFFFF"));
            }
            None => panic!("emblem crest expected"),
        }
        let uploaded = json!({"uniforms": [{"name": "Home", "recipe": {"jersey": "#101820", "crest": "club"}, "crest_svg": svg, "crest_url": "/v1/jersey/a/crest.png"}]});
        assert!(
            design(&uploaded).unwrap().crest.is_none(),
            "uploaded crests stay in PuckLab"
        );
        let hostile = json!({"uniforms": [{"name": "Home", "recipe": {"jersey": "#101820", "crest": "emblem:x"}, "crest_svg": "<svg viewBox=\"0 0 15 15\"><path d=\"M0 0\"/><script>x</script><path d=\"url(javascript:1)\"/></svg>"}]});
        let jersey = design(&hostile).unwrap();
        assert!(
            jersey.crest.is_none(),
            "a crest that fails validation is dropped, the colours stay"
        );
        assert_eq!(jersey.body, "#101820");
    }

    #[test]
    fn no_saved_uniform_means_the_default_jersey() {
        assert!(design(&json!({"uniforms": []})).is_none());
        assert!(design(&json!({})).is_none());
        assert!(design(&json!({"uniforms": [{"name": "Home"}]})).is_none());
    }
}
