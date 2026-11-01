//! A PuckLab Lab club's jersey as design data: colours, stripes and an
//! optional drawn emblem. The shape matches the rich presence jersey, so one
//! renderer serves both. It carries no names, abbreviations or images, and a
//! crest is only path data of PuckLab's own Jersey Lab emblems.
use serde::{Deserialize, Serialize};
use specta::Type;

pub const STRIPES_MAX: usize = 4;
pub const CREST_PATHS_MAX: usize = 8;
pub const CREST_PATH_MAX: usize = 4096;
/// Serialized size cap for a whole design.
pub const DESIGN_BYTES_MAX: usize = 36 * 1024;
pub const REPORTS_MAX: usize = 100;

#[derive(Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Jersey {
    pub body: String,
    #[serde(default)]
    pub sleeves: Option<String>,
    #[serde(default)]
    pub yoke: Option<String>,
    #[serde(default)]
    pub collar: Option<String>,
    /// The colour of the plain mark drawn when there is no crest.
    #[serde(default)]
    pub trim: Option<String>,
    #[serde(default)]
    pub sleeve_stripes: Vec<JerseyStripe>,
    #[serde(default)]
    pub hem_stripes: Vec<JerseyStripe>,
    #[serde(default)]
    pub crest: Option<JerseyCrest>,
}

#[derive(Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JerseyStripe {
    pub pattern: StripePattern,
    pub color: String,
}

#[derive(Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StripePattern {
    Single,
    Double,
    Triple,
    Bold,
    ThickThin,
    ThinThickThin,
}

#[derive(Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JerseyCrest {
    Emblem {
        /// SVG path data, drawn filled in `color`.
        paths: Vec<String>,
        view_box: [f32; 4],
        color: String,
        #[serde(default)]
        back: Option<String>,
    },
}

/// The bot's answer for one person: their club's current home jersey, or
/// none when they have no Lab club or no linked PuckLab account.
#[derive(Type, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PuckLabJerseyReport {
    pub user_id: String,
    pub jersey: Option<Jersey>,
}

pub fn color(value: &str) -> Result<String, &'static str> {
    let ok = value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|b| b.is_ascii_hexdigit());
    ok.then(|| value.to_ascii_uppercase())
        .ok_or("Jersey colours must be #RRGGBB")
}

fn optional_color(value: Option<String>) -> Result<Option<String>, &'static str> {
    value.as_deref().map(color).transpose()
}

pub fn path_data(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= CREST_PATH_MAX
        && path
            .bytes()
            .all(|b| b"MmLlHhVvCcSsQqTtAaZzEe0123456789.,- ".contains(&b))
}

impl Jersey {
    /// Normalised colours, or the reason the design is refused.
    pub fn sanitized(self) -> Result<Self, &'static str> {
        let stripes = |list: Vec<JerseyStripe>| -> Result<Vec<JerseyStripe>, &'static str> {
            if list.len() > STRIPES_MAX {
                return Err("Too many jersey stripes");
            }
            list.into_iter()
                .map(|s| {
                    Ok(JerseyStripe {
                        pattern: s.pattern,
                        color: color(&s.color)?,
                    })
                })
                .collect()
        };
        let crest = match self.crest {
            None => None,
            Some(JerseyCrest::Emblem {
                paths,
                view_box,
                color: fill,
                back,
            }) => {
                if paths.is_empty()
                    || paths.len() > CREST_PATHS_MAX
                    || !paths.iter().all(|p| path_data(p))
                    || !view_box.iter().all(|v| v.is_finite() && v.abs() <= 1000.0)
                    || view_box[2] <= 0.0
                    || view_box[3] <= 0.0
                {
                    return Err("Invalid jersey crest");
                }
                Some(JerseyCrest::Emblem {
                    paths,
                    view_box,
                    color: color(&fill)?,
                    back: optional_color(back)?,
                })
            }
        };
        let jersey = Jersey {
            body: color(&self.body)?,
            sleeves: optional_color(self.sleeves)?,
            yoke: optional_color(self.yoke)?,
            collar: optional_color(self.collar)?,
            trim: optional_color(self.trim)?,
            sleeve_stripes: stripes(self.sleeve_stripes)?,
            hem_stripes: stripes(self.hem_stripes)?,
            crest,
        };
        let size = serde_json::to_vec(&jersey).map_or(usize::MAX, |bytes| bytes.len());
        if size > DESIGN_BYTES_MAX {
            return Err("Jersey design is too large");
        }
        Ok(jersey)
    }
}

impl PuckLabJerseyReport {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.user_id.is_empty()
            || self.user_id.len() > 200
            || self.user_id.chars().any(char::is_control)
        {
            return Err("Invalid user");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn design() -> Jersey {
        serde_json::from_value(json!({
            "body": "#123b74",
            "collar": "#ffffff",
            "sleeve_stripes": [{"pattern": "double", "color": "#c93845"}],
            "hem_stripes": [{"pattern": "thin_thick_thin", "color": "#ffffff"}],
            "crest": {"kind": "emblem", "paths": ["M7.5,0C5.5,0,4,1.567,4,3.5Z"], "view_box": [0, 0, 15, 15], "color": "#ffffff", "back": "#101820"}
        }))
        .unwrap()
    }

    #[test]
    fn a_design_keeps_its_shape_and_upper_cases_colours() {
        let clean = design().sanitized().unwrap();
        assert_eq!(clean.body, "#123B74");
        assert_eq!(clean.sleeve_stripes[0].color, "#C93845");
        let wire = serde_json::to_value(&clean).unwrap();
        assert_eq!(wire["crest"]["kind"], "emblem");
        assert_eq!(wire["hem_stripes"][0]["pattern"], "thin_thick_thin");
    }

    #[test]
    fn markup_names_and_images_never_get_through() {
        let mut bad = design();
        bad.body = "red".into();
        assert!(bad.sanitized().is_err());
        let mut script = design();
        script.crest = Some(JerseyCrest::Emblem {
            paths: vec!["M0 0\"/><script>".into()],
            view_box: [0.0, 0.0, 15.0, 15.0],
            color: "#FFFFFF".into(),
            back: None,
        });
        assert!(script.sanitized().is_err());
        assert!(serde_json::from_value::<Jersey>(
            json!({"body":"#000000","crest":{"kind":"image","url":"https://example.com/logo.png"}})
        )
        .is_err());
        assert!(serde_json::from_value::<Jersey>(
            json!({"body":"#000000","crest":{"kind":"club","name":"Leafs"}})
        )
        .is_err());
    }

    #[test]
    fn stripes_paths_and_size_are_capped() {
        let mut stripes = design();
        stripes.hem_stripes = vec![stripes.hem_stripes[0].clone(); STRIPES_MAX + 1];
        assert!(stripes.sanitized().is_err());
        let mut paths = design();
        paths.crest = Some(JerseyCrest::Emblem {
            paths: vec!["M0 0L1 1Z".into(); CREST_PATHS_MAX + 1],
            view_box: [0.0, 0.0, 15.0, 15.0],
            color: "#FFFFFF".into(),
            back: None,
        });
        assert!(paths.sanitized().is_err());
        let mut long = design();
        long.crest = Some(JerseyCrest::Emblem {
            paths: vec!["M0 0L1 1Z".repeat(CREST_PATH_MAX); 1],
            view_box: [0.0, 0.0, 15.0, 15.0],
            color: "#FFFFFF".into(),
            back: None,
        });
        assert!(long.sanitized().is_err());
        let mut flat = design();
        flat.crest = Some(JerseyCrest::Emblem {
            paths: vec!["M0 0L1 1Z".into()],
            view_box: [0.0, 0.0, 0.0, 15.0],
            color: "#FFFFFF".into(),
            back: None,
        });
        assert!(flat.sanitized().is_err());
    }

    #[test]
    fn reports_name_a_real_user() {
        let ok = PuckLabJerseyReport {
            user_id: "alice".into(),
            jersey: None,
        };
        assert!(ok.validate().is_ok());
        for user_id in ["", "a\nb"] {
            assert!(PuckLabJerseyReport {
                user_id: user_id.into(),
                jersey: None
            }
            .validate()
            .is_err());
        }
    }
}
