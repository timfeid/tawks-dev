//! `/roll` and `/init` for games played in a chat channel, answered by the
//! bot in that chat.

use rand::Rng;

use crate::dice;
use crate::initiative::{Combatant, Tracker};

pub const INIT_HELP: &str =
    "Try /init add Goblin 12, /init add Mira +3 (rolls a d20), /init next, /init remove Goblin, /init clear";

pub fn roll_reply(who: &str, input: &str, rng: &mut impl Rng) -> Result<String, String> {
    let input = if input.trim().is_empty() { "1d20" } else { input };
    let roll = dice::roll(input, rng).map_err(|e| e.to_string())?;
    Ok(format!("**{who}** rolled {}", roll.summary()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitScore {
    Fixed(i32),
    Bonus(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitCommand {
    Show,
    Add { name: String, score: InitScore },
    Next,
    Prev,
    Remove { name: String },
    Clear,
    Help,
}

pub fn parse_init(input: &str) -> Result<InitCommand, String> {
    let words: Vec<&str> = input.split_whitespace().collect();
    let Some(first) = words.first() else {
        return Ok(InitCommand::Show);
    };
    let rest = &words[1..];
    match first.to_ascii_lowercase().as_str() {
        "list" | "show" => Ok(InitCommand::Show),
        "next" | "n" => Ok(InitCommand::Next),
        "prev" | "back" => Ok(InitCommand::Prev),
        "clear" | "end" | "reset" => Ok(InitCommand::Clear),
        "help" => Ok(InitCommand::Help),
        "remove" | "rm" | "drop" => {
            let name = rest.join(" ");
            if name.is_empty() {
                return Err("Say who to remove, like /init remove Goblin".into());
            }
            Ok(InitCommand::Remove { name })
        }
        "add" => {
            let (name_words, score) = match rest.last() {
                Some(last) if last.starts_with('+') || last.starts_with('-') => match last.parse::<i32>() {
                    Ok(bonus) => (&rest[..rest.len() - 1], InitScore::Bonus(bonus)),
                    Err(_) => return Err(format!("{last} is not a bonus, like +3")),
                },
                Some(last) if last.chars().all(|c| c.is_ascii_digit()) => match last.parse::<i32>() {
                    Ok(init) => (&rest[..rest.len() - 1], InitScore::Fixed(init)),
                    Err(_) => return Err(format!("{last} is too big")),
                },
                _ => (rest, InitScore::Bonus(0)),
            };
            let name = name_words.join(" ");
            if name.is_empty() {
                return Err("Say who to add, like /init add Goblin 12".into());
            }
            Ok(InitCommand::Add { name, score })
        }
        _ => Err(INIT_HELP.into()),
    }
}

/// Runs one `/init` command and returns the reply to post.
pub fn run_init(tracker: &mut Tracker, command: InitCommand, rng: &mut impl Rng) -> Result<String, String> {
    let mut prefix = String::new();
    match command {
        InitCommand::Show => {}
        InitCommand::Help => return Ok(INIT_HELP.into()),
        InitCommand::Add { name, score } => {
            let (init, bonus) = match score {
                InitScore::Fixed(init) => (init, 0),
                InitScore::Bonus(bonus) => {
                    let roll = rng.random_range(1..=20) + bonus;
                    prefix = format!("{name} rolled {roll} for initiative.\n");
                    (roll, bonus)
                }
            };
            tracker.add(Combatant {
                id: String::new(),
                name,
                init,
                bonus,
                sheet_id: None,
                owner: None,
                hidden: false,
            })?;
            tracker.sort();
        }
        InitCommand::Next => {
            if tracker.entries.is_empty() {
                return Err("Nobody is in the order yet. /init add Goblin 12".into());
            }
            tracker.next();
        }
        InitCommand::Prev => tracker.prev(),
        InitCommand::Remove { name } => {
            let found = tracker
                .entries
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(&name))
                .map(|c| c.id.clone());
            let Some(id) = found else {
                return Err(format!("{name} is not in the order"));
            };
            tracker.remove(&id);
        }
        InitCommand::Clear => {
            tracker.clear();
            return Ok("Initiative cleared.".into());
        }
    }
    Ok(format!("{prefix}{}", render(tracker)))
}

pub fn render(tracker: &Tracker) -> String {
    if tracker.entries.is_empty() {
        return format!("The order is empty. {INIT_HELP}");
    }
    let mut lines = vec![if tracker.round == 0 {
        "Initiative, not started. /init next begins round 1.".to_string()
    } else {
        format!("Initiative, round {}", tracker.round)
    }];
    for (i, c) in tracker.entries.iter().enumerate() {
        if tracker.current.as_deref() == Some(c.id.as_str()) {
            lines.push(format!("{}. **{} {}** (up)", i + 1, c.name, c.init));
        } else {
            lines.push(format!("{}. {} {}", i + 1, c.name, c.init));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn reads_init_commands() {
        assert_eq!(parse_init("").unwrap(), InitCommand::Show);
        assert_eq!(
            parse_init("add Goblin Boss 12").unwrap(),
            InitCommand::Add {
                name: "Goblin Boss".into(),
                score: InitScore::Fixed(12)
            }
        );
        assert_eq!(
            parse_init("add Mira +3").unwrap(),
            InitCommand::Add {
                name: "Mira".into(),
                score: InitScore::Bonus(3)
            }
        );
        assert_eq!(
            parse_init("add Ogre").unwrap(),
            InitCommand::Add {
                name: "Ogre".into(),
                score: InitScore::Bonus(0)
            }
        );
        assert_eq!(parse_init("NEXT").unwrap(), InitCommand::Next);
        assert!(parse_init("add").is_err());
        assert!(parse_init("add +3").is_err());
        assert!(parse_init("dance").is_err());
        assert!(parse_init("remove").is_err());
    }

    #[test]
    fn a_chat_order_runs_and_renders() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut t = Tracker::default();
        run_init(&mut t, parse_init("add Goblin 12").unwrap(), &mut rng).unwrap();
        run_init(&mut t, parse_init("add Mira 17").unwrap(), &mut rng).unwrap();
        let out = run_init(&mut t, parse_init("next").unwrap(), &mut rng).unwrap();
        assert_eq!(out, "Initiative, round 1\n1. **Mira 17** (up)\n2. Goblin 12");
        run_init(&mut t, parse_init("next").unwrap(), &mut rng).unwrap();
        let out = run_init(&mut t, parse_init("next").unwrap(), &mut rng).unwrap();
        assert!(out.starts_with("Initiative, round 2\n1. **Mira"));
        let out = run_init(&mut t, parse_init("add Wolf +2").unwrap(), &mut rng).unwrap();
        assert!(out.starts_with("Wolf rolled "));
        assert!(run_init(&mut t, parse_init("remove Bandit").unwrap(), &mut rng).is_err());
        run_init(&mut t, parse_init("remove goblin").unwrap(), &mut rng).unwrap();
        assert_eq!(t.entries.len(), 2);
        assert_eq!(
            run_init(&mut t, InitCommand::Clear, &mut rng).unwrap(),
            "Initiative cleared."
        );
        assert!(run_init(&mut t, InitCommand::Next, &mut rng).is_err());
    }

    #[test]
    fn roll_replies_name_the_roller() {
        let mut rng = StdRng::seed_from_u64(1);
        let reply = roll_reply("Pat", "1d20+3 stealth", &mut rng).unwrap();
        assert!(reply.starts_with("**Pat** rolled 1d20+3 stealth: ("), "{reply}");
        assert!(
            roll_reply("Pat", "", &mut rng)
                .unwrap()
                .starts_with("**Pat** rolled 1d20: ")
        );
        assert!(roll_reply("Pat", "banana", &mut rng).is_err());
    }
}
