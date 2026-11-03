//! What a chart may say about itself. The rows come from `chart_data_key`; this
//! only names which of their fields to draw and how to write them. Colours and
//! sizes belong to the renderer, so there is no way to ask for them here.

use serde::{Deserialize, Serialize};
use specta::Type;

pub const MAX_CHART_SERIES: usize = 8;
pub const MAX_CHART_LABEL_CHARS: usize = 60;

/// How the x values read. `Auto` looks at the values.
#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartXFormat {
    #[default]
    Auto,
    Text,
    Number,
    Date,
    DateTime,
}

/// How the y values read. `Percent` takes 0 to 100 and adds the sign.
#[derive(Type, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartYFormat {
    #[default]
    Number,
    Compact,
    Percent,
}

/// One line, area or set of bars, read from one field of each row.
#[derive(Type, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartSeries {
    pub key: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Type, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartConfig {
    #[serde(default)]
    pub x_label: Option<String>,
    #[serde(default)]
    pub y_label: Option<String>,
    #[serde(default)]
    pub x_format: ChartXFormat,
    #[serde(default)]
    pub y_format: ChartYFormat,
    /// Several value fields on each row. Leave empty to draw `y` alone.
    #[serde(default)]
    pub series: Vec<ChartSeries>,
    /// A field on each row whose values name the series, for rows such as
    /// `{week, team, points}`. The value still comes from `y`.
    #[serde(default)]
    pub split_by: Option<String>,
}

impl ChartConfig {
    pub fn labelled(mut self, x_label: impl Into<String>, y_label: impl Into<String>) -> Self {
        self.x_label = Some(x_label.into());
        self.y_label = Some(y_label.into());
        self
    }

    pub fn split_by(mut self, field: impl Into<String>) -> Self {
        self.split_by = Some(field.into());
        self
    }

    pub fn series(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.series.push(ChartSeries {
            key: key.into(),
            label: Some(label.into()),
        });
        self
    }

    pub fn x_format(mut self, format: ChartXFormat) -> Self {
        self.x_format = format;
        self
    }

    pub fn y_format(mut self, format: ChartYFormat) -> Self {
        self.y_format = format;
        self
    }
}

fn is_field_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('.').all(|part| {
            !part.is_empty()
                && !matches!(part, "__proto__" | "prototype" | "constructor")
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

fn check_label(issues: &mut Vec<String>, name: &str, label: Option<&str>) {
    let Some(label) = label else { return };
    if label.chars().count() > MAX_CHART_LABEL_CHARS {
        issues.push(format!(
            "{name} is longer than {MAX_CHART_LABEL_CHARS} characters"
        ));
    }
    if label.contains('{') || label.contains('}') {
        issues.push(format!(
            "{name} is plain text, so it cannot hold a template"
        ));
    }
}

/// Everything wrong with a chart's fields, in words for whoever wrote it.
pub fn chart_issues(data_key: &str, config: &ChartConfig, x: &str, y: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if !is_field_path(data_key) {
        issues.push(format!(
            "chart_data_key must be a path such as sys.points_by_week, not \"{data_key}\""
        ));
    }
    for (name, field) in [("x", x)]
        .into_iter()
        .chain((!y.is_empty()).then_some(("y", y)))
    {
        if !is_field_path(field) {
            issues.push(format!(
                "{name} must name a field of the rows, not \"{field}\""
            ));
        }
    }
    check_label(&mut issues, "x_label", config.x_label.as_deref());
    check_label(&mut issues, "y_label", config.y_label.as_deref());

    if config.series.is_empty() && y.is_empty() {
        issues.push("a chart needs y, or series listing the fields to draw".to_string());
    }
    if config.series.len() > MAX_CHART_SERIES {
        issues.push(format!("at most {MAX_CHART_SERIES} series"));
    }
    for (index, series) in config.series.iter().enumerate() {
        if !is_field_path(&series.key) {
            issues.push(format!(
                "series[{index}].key must name a field of the rows, not \"{}\"",
                series.key
            ));
        }
        if config.series[..index]
            .iter()
            .any(|earlier| earlier.key == series.key)
        {
            issues.push(format!("series[{index}].key {} is used twice", series.key));
        }
        check_label(
            &mut issues,
            &format!("series[{index}].label"),
            series.label.as_deref(),
        );
    }
    if !config.series.is_empty() && !y.is_empty() && !config.series.iter().any(|s| s.key == y) {
        issues.push(format!(
            "y is {y}, which series does not list; leave y empty when series names the fields"
        ));
    }
    if let Some(split) = &config.split_by {
        if !is_field_path(split) {
            issues.push(format!(
                "split_by must name a field of the rows, not \"{split}\""
            ));
        }
        if !config.series.is_empty() {
            issues.push(
                "use split_by when one field names the series, or series when each has its own field, not both"
                    .to_string(),
            );
        }
        if y.is_empty() {
            issues.push("split_by needs y, the field holding each value".to_string());
        }
    }
    issues
}

/// The fields a chart reads from each row: x and then the value fields.
pub fn chart_fields<'a>(
    config: &'a ChartConfig,
    x: &'a str,
    y: &'a str,
) -> (&'a str, Vec<&'a str>) {
    let values = if config.series.is_empty() {
        vec![y]
    } else {
        config.series.iter().map(|s| s.key.as_str()).collect()
    };
    (x, values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_plain_chart_needs_no_config() {
        let config: ChartConfig = serde_json::from_value(json!({})).unwrap();
        assert_eq!(config, ChartConfig::default());
        assert!(chart_issues("sys.weeks", &config, "week", "points").is_empty());
    }

    #[test]
    fn the_whole_shape_reads_and_passes() {
        let config: ChartConfig = serde_json::from_value(json!({
            "x_label": "Week", "y_label": "Points", "x_format": "Number", "y_format": "Compact",
            "series": [{ "key": "for", "label": "For" }, { "key": "against" }],
        }))
        .unwrap();
        assert_eq!(config.series.len(), 2);
        assert!(chart_issues("data.scores.rows", &config, "week", "").is_empty());
    }

    #[test]
    fn a_format_that_does_not_exist_does_not_parse() {
        assert!(serde_json::from_value::<ChartConfig>(json!({ "y_format": "Rainbow" })).is_err());
    }

    fn found(key: &str, config: serde_json::Value, x: &str, y: &str) -> String {
        let config: ChartConfig = serde_json::from_value(config).unwrap();
        chart_issues(key, &config, x, y).join("\n")
    }

    #[test]
    fn paths_cannot_reach_outside_a_row() {
        assert!(found("sys.{x}", json!({}), "week", "points").contains("chart_data_key"));
        assert!(found("sys.w", json!({}), "__proto__", "points").contains("x must name"));
        assert!(found("sys.w", json!({}), "week", "a.constructor").contains("y must name"));
        assert!(found("sys.w", json!({}), "{g.week}", "points").contains("x must name"));
    }

    #[test]
    fn labels_stay_short_plain_text() {
        let long = "x".repeat(61);
        assert!(found("k", json!({ "x_label": long }), "x", "y").contains("longer"));
        assert!(found("k", json!({ "y_label": "{sys.a}" }), "x", "y").contains("template"));
    }

    #[test]
    fn series_are_capped_and_distinct() {
        let many: Vec<_> = (0..9).map(|i| json!({ "key": format!("s{i}") })).collect();
        assert!(found("k", json!({ "series": many }), "x", "").contains("at most 8"));
        let twice = json!({ "series": [{ "key": "a" }, { "key": "a" }] });
        assert!(found("k", twice, "x", "").contains("used twice"));
        assert!(found("k", json!({}), "x", "").contains("needs y"));
        let elsewhere = json!({ "series": [{ "key": "a" }] });
        assert!(found("k", elsewhere, "x", "b").contains("does not list"));
    }

    #[test]
    fn split_by_stands_alone_and_needs_a_value() {
        assert_eq!(found("k", json!({ "split_by": "team" }), "x", "y"), "");
        let both = json!({ "split_by": "team", "series": [{ "key": "a" }] });
        assert!(found("k", both, "x", "").contains("not both"));
        assert!(found("k", json!({ "split_by": "team" }), "x", "").contains("needs y"));
    }

    #[test]
    fn fields_read_are_x_then_the_values() {
        let plain = ChartConfig::default();
        assert_eq!(chart_fields(&plain, "w", "p"), ("w", vec!["p"]));
        let two = ChartConfig::default().series("a", "A").series("b", "B");
        assert_eq!(chart_fields(&two, "w", ""), ("w", vec!["a", "b"]));
    }
}
