//! What the table takes from the System Reference Document 5.2.1, and the
//! guard that keeps the source's product names out of anything imported.

pub const ATTRIBUTION: &str = "This work includes material from the System Reference Document 5.2.1 (\"SRD 5.2.1\") by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is licensed under the Creative Commons Attribution 4.0 International License, available at https://creativecommons.org/licenses/by/4.0/legalcode.";

pub const COMPATIBILITY: &str = "5E compatible";

pub struct Condition {
    pub name: &'static str,
    pub summary: &'static str,
}

/// The conditions of SRD 5.2.1's rules glossary, summarized for a sheet's chips.
pub const CONDITIONS: &[Condition] = &[
    Condition {
        name: "Blinded",
        summary: "Can't see. Attacks against you have advantage; your attacks have disadvantage.",
    },
    Condition {
        name: "Charmed",
        summary: "Can't attack the charmer, who has advantage on social checks against you.",
    },
    Condition {
        name: "Deafened",
        summary: "Can't hear, and fail any check that needs hearing.",
    },
    Condition {
        name: "Exhaustion",
        summary: "Each level takes 2 from your d20 tests and 5 feet from your speed. Six levels is death.",
    },
    Condition {
        name: "Frightened",
        summary: "Disadvantage on checks and attacks while the source is in sight; you can't move closer to it.",
    },
    Condition {
        name: "Grappled",
        summary: "Speed 0, and disadvantage on attacks against anyone but the grappler.",
    },
    Condition {
        name: "Incapacitated",
        summary: "No actions, bonus actions or reactions; concentration ends; you can't speak.",
    },
    Condition {
        name: "Invisible",
        summary: "Hard to find and to target. Your attacks have advantage; attacks against you have disadvantage.",
    },
    Condition {
        name: "Paralyzed",
        summary: "Incapacitated, speed 0, fail Strength and Dexterity saves. Close hits are critical.",
    },
    Condition {
        name: "Petrified",
        summary: "Turned to stone: incapacitated, resistant to all damage, and immune to poison.",
    },
    Condition {
        name: "Poisoned",
        summary: "Disadvantage on attack rolls and ability checks.",
    },
    Condition {
        name: "Prone",
        summary: "Crawl or spend half your speed to stand. Your attacks have disadvantage; close attacks against you have advantage.",
    },
    Condition {
        name: "Restrained",
        summary: "Speed 0. Your attacks and Dexterity saves have disadvantage; attacks against you have advantage.",
    },
    Condition {
        name: "Stunned",
        summary: "Incapacitated, and you fail Strength and Dexterity saves. Attacks against you have advantage.",
    },
    Condition {
        name: "Unconscious",
        summary: "Incapacitated and prone, you drop what you hold. Close hits are critical.",
    },
];

/// Product names the SRD mentions that must never reach the table.
fn forbidden() -> [String; 3] {
    [
        ["D", "&", "D"].concat(),
        ["D", "&amp;", "D"].concat(),
        ["Dungeons ", "&", " Dragons"].concat(),
    ]
}

/// Rewrites a passage of SRD text so it names the game generically, as
/// "The three main pillars of [the product] play" becomes "...of the game play".
pub fn scrub(text: &str) -> String {
    let mut out = text.to_string();
    for name in forbidden().iter().rev() {
        for (from, to) in [
            (format!("{name}'s"), "the game's".to_string()),
            (name.clone(), "the game".to_string()),
        ] {
            out = out.replace(&from, &to);
        }
    }
    out
}

pub fn mentions_product(text: &str) -> bool {
    let lower = text.to_lowercase();
    forbidden().iter().any(|name| lower.contains(&name.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_rewrites_product_mentions() {
        let d = ["D", "&", "D"].concat();
        let sample = format!("The three main pillars of {d} play are social interaction, exploration and combat.");
        let clean = scrub(&sample);
        assert_eq!(
            clean,
            "The three main pillars of the game play are social interaction, exploration and combat."
        );
        assert!(!mentions_product(&clean));
        assert!(mentions_product(&sample));
        let long = ["Dungeons ", "&", " Dragons"].concat();
        assert!(!mentions_product(&scrub(&format!("Welcome to {long}. {d}'s rules"))));
    }

    #[test]
    fn imported_rows_never_name_the_product() {
        for condition in CONDITIONS {
            assert!(!mentions_product(condition.name), "{}", condition.name);
            assert!(!mentions_product(condition.summary), "{}", condition.name);
            assert_eq!(scrub(condition.summary), condition.summary);
        }
        assert_eq!(CONDITIONS.len(), 15);
    }

    /// Product names may appear in the code that strips them and nowhere else
    /// the table ships: server, client, docs.
    #[test]
    fn nothing_shipped_names_the_product() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut stack = vec![
            root.join("src"),
            root.join("client/src"),
            root.join("README.md"),
            root.join("client/index.html"),
        ];
        let mut checked = 0;
        while let Some(path) = stack.pop() {
            if path.is_dir() {
                for entry in std::fs::read_dir(&path).unwrap() {
                    stack.push(entry.unwrap().path());
                }
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            checked += 1;
            let others = [
                ["dungeons", " and ", "dragons"].concat(),
                ["roll", "20"].concat(),
                ["foundry", " vtt"].concat(),
            ];
            for (n, line) in text.lines().enumerate() {
                let lower = line.to_lowercase();
                let banned = mentions_product(line) || others.iter().any(|other| lower.contains(other.as_str()));
                assert!(!banned, "{}:{} names another product: {line}", path.display(), n + 1);
            }
        }
        assert!(checked > 5);
    }
}
