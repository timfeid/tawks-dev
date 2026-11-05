//! Dice formulas such as `2d6+3`, `4d6dl1`, `2d20kh1+5` or `1d20+4 adv stealth`,
//! parsed once and rolled on the server.

use rand::Rng;
use serde::{Deserialize, Serialize};

pub const MAX_DICE_PER_TERM: u32 = 100;
pub const MAX_DICE: u32 = 200;
pub const MAX_SIDES: u32 = 1000;
pub const MAX_TERMS: usize = 12;
pub const MAX_FLAT: i64 = 1000;
const MAX_LABEL_CHARS: usize = 60;
const MAX_INPUT_CHARS: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Keep {
    Highest(u32),
    Lowest(u32),
    DropHighest(u32),
    DropLowest(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Dice {
        negative: bool,
        count: u32,
        sides: u32,
        keep: Option<Keep>,
    },
    Flat {
        negative: bool,
        value: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Advantage,
    Disadvantage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formula {
    pub terms: Vec<Term>,
    pub label: Option<String>,
    pub edge: Option<Edge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Die {
    pub value: u32,
    pub kept: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Part {
    Dice {
        negative: bool,
        count: u32,
        sides: u32,
        dice: Vec<Die>,
        subtotal: i64,
    },
    Flat {
        negative: bool,
        value: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Natural {
    Twenty,
    One,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roll {
    pub formula: String,
    pub label: Option<String>,
    pub edge: Option<Edge>,
    pub parts: Vec<Part>,
    pub total: i64,
    pub natural: Option<Natural>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiceError(pub String);

impl std::fmt::Display for DiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DiceError {}

fn err<T>(message: impl Into<String>) -> Result<T, DiceError> {
    Err(DiceError(message.into()))
}

fn formula_word(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    let allowed = lower
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, 'd' | 'k' | 'h' | 'l' | '%' | '+' | '-'));
    let has_shape = lower.chars().any(|c| c.is_ascii_digit()) || lower == "+" || lower == "-" || lower == "d%";
    allowed && has_shape
}

fn edge_word(word: &str) -> Option<Edge> {
    match word.to_ascii_lowercase().as_str() {
        "adv" | "advantage" => Some(Edge::Advantage),
        "dis" | "disadv" | "disadvantage" => Some(Edge::Disadvantage),
        _ => None,
    }
}

/// Reads a roll as people type it. Words after the formula that are not
/// `adv` or `dis` become the label.
pub fn parse(input: &str) -> Result<Formula, DiceError> {
    let input = input.trim();
    if input.chars().count() > MAX_INPUT_CHARS {
        return err("That roll is too long");
    }
    let words: Vec<&str> = input.split_whitespace().collect();
    let mut formula_text = String::new();
    let mut rest = 0;
    for word in &words {
        if !formula_word(word) {
            break;
        }
        formula_text.push_str(word);
        rest += 1;
    }
    let mut advantage = false;
    let mut disadvantage = false;
    let mut label_words = Vec::new();
    for word in &words[rest..] {
        match edge_word(word) {
            Some(Edge::Advantage) => advantage = true,
            Some(Edge::Disadvantage) => disadvantage = true,
            None => label_words.push(*word),
        }
    }
    let edge = match (advantage, disadvantage) {
        (true, false) => Some(Edge::Advantage),
        (false, true) => Some(Edge::Disadvantage),
        _ => None,
    };
    if formula_text.is_empty() {
        if advantage || disadvantage {
            formula_text.push_str("1d20");
        } else {
            return err("Type a roll like 1d20+5 or 2d6");
        }
    }
    let mut terms = parse_terms(&formula_text)?;
    if let Some(edge) = edge {
        apply_edge(&mut terms, edge)?;
    }
    let label: String = label_words.join(" ").chars().take(MAX_LABEL_CHARS).collect();
    Ok(Formula {
        terms,
        label: (!label.is_empty()).then_some(label),
        edge,
    })
}

fn apply_edge(terms: &mut [Term], edge: Edge) -> Result<(), DiceError> {
    let first_d20 = terms.iter_mut().find(|term| {
        matches!(
            term,
            Term::Dice {
                sides: 20,
                count: 1,
                keep: None,
                negative: false
            }
        )
    });
    let Some(Term::Dice { count, keep, .. }) = first_d20 else {
        return err("Advantage and disadvantage need a single d20 in the roll");
    };
    *count = 2;
    *keep = Some(match edge {
        Edge::Advantage => Keep::Highest(1),
        Edge::Disadvantage => Keep::Lowest(1),
    });
    Ok(())
}

struct Cursor<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl Cursor<'_> {
    fn number(&mut self) -> Result<Option<i64>, DiceError> {
        let mut digits = String::new();
        while let Some(c) = self.chars.peek().copied().filter(char::is_ascii_digit) {
            digits.push(c);
            self.chars.next();
            if digits.len() > 6 {
                return err("That number is too big");
            }
        }
        Ok((!digits.is_empty()).then(|| digits.parse().unwrap_or(0)))
    }

    fn eat(&mut self, want: char) -> bool {
        if self.chars.peek() == Some(&want) {
            self.chars.next();
            true
        } else {
            false
        }
    }
}

fn parse_terms(text: &str) -> Result<Vec<Term>, DiceError> {
    let lower = text.to_ascii_lowercase();
    let mut cursor = Cursor {
        chars: lower.chars().peekable(),
    };
    let mut terms = Vec::new();
    let mut negative = false;
    if cursor.eat('-') {
        negative = true;
    } else {
        cursor.eat('+');
    }
    loop {
        terms.push(parse_term(&mut cursor, negative)?);
        if terms.len() > MAX_TERMS {
            return err(format!("Use at most {MAX_TERMS} parts in one roll"));
        }
        match cursor.chars.next() {
            None => break,
            Some('+') => negative = false,
            Some('-') => negative = true,
            Some(c) => return err(format!("I can't read \"{c}\" in that roll")),
        }
    }
    let dice: u32 = terms
        .iter()
        .map(|term| match term {
            Term::Dice { count, .. } => *count,
            Term::Flat { .. } => 0,
        })
        .sum();
    if dice == 0 {
        return err("Put at least one die in the roll, like 1d20");
    }
    if dice > MAX_DICE {
        return err(format!("Roll at most {MAX_DICE} dice at once"));
    }
    Ok(terms)
}

fn parse_term(cursor: &mut Cursor, negative: bool) -> Result<Term, DiceError> {
    let count = cursor.number()?;
    if !cursor.eat('d') {
        return match count {
            Some(value) if value <= MAX_FLAT => Ok(Term::Flat { negative, value }),
            Some(_) => err(format!("Keep added numbers at {MAX_FLAT} or less")),
            None => err("Something is missing in that roll"),
        };
    }
    let count = count.unwrap_or(1);
    let sides = if cursor.eat('%') {
        100
    } else {
        match cursor.number()? {
            Some(sides) => sides,
            None => return err("Say how many sides, like d6 or d20"),
        }
    };
    if count < 1 || count > MAX_DICE_PER_TERM as i64 {
        return err(format!("Roll between 1 and {MAX_DICE_PER_TERM} dice of a kind"));
    }
    if sides < 2 || sides > MAX_SIDES as i64 {
        return err(format!("Dice have between 2 and {MAX_SIDES} sides"));
    }
    let (count, sides) = (count as u32, sides as u32);
    let keep = parse_keep(cursor, count)?;
    Ok(Term::Dice {
        negative,
        count,
        sides,
        keep,
    })
}

fn parse_keep(cursor: &mut Cursor, count: u32) -> Result<Option<Keep>, DiceError> {
    let keep = if cursor.eat('k') {
        if cursor.eat('l') {
            Keep::Lowest(0)
        } else {
            cursor.eat('h');
            Keep::Highest(0)
        }
    } else if cursor.eat('d') {
        if cursor.eat('h') {
            Keep::DropHighest(0)
        } else if cursor.eat('l') {
            Keep::DropLowest(0)
        } else {
            return err("Use dl or dh to drop dice, like 4d6dl1");
        }
    } else {
        return Ok(None);
    };
    let n = cursor.number()?.unwrap_or(1);
    let n = u32::try_from(n).unwrap_or(u32::MAX);
    let keep = match keep {
        Keep::Highest(_) | Keep::Lowest(_) if n == 0 || n > count => {
            return err(format!("Keep between 1 and {count} of those dice"));
        }
        Keep::DropHighest(_) | Keep::DropLowest(_) if n == 0 || n >= count => {
            return err("Drop fewer dice than you roll");
        }
        Keep::Highest(_) => Keep::Highest(n),
        Keep::Lowest(_) => Keep::Lowest(n),
        Keep::DropHighest(_) => Keep::DropHighest(n),
        Keep::DropLowest(_) => Keep::DropLowest(n),
    };
    Ok(Some(keep))
}

impl Formula {
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (i, term) in self.terms.iter().enumerate() {
            let negative = match term {
                Term::Dice { negative, .. } | Term::Flat { negative, .. } => *negative,
            };
            if negative {
                out.push('-');
            } else if i > 0 {
                out.push('+');
            }
            match term {
                Term::Flat { value, .. } => out.push_str(&value.to_string()),
                Term::Dice { count, sides, keep, .. } => {
                    out.push_str(&format!("{count}d{sides}"));
                    match keep {
                        Some(Keep::Highest(n)) => out.push_str(&format!("kh{n}")),
                        Some(Keep::Lowest(n)) => out.push_str(&format!("kl{n}")),
                        Some(Keep::DropHighest(n)) => out.push_str(&format!("dh{n}")),
                        Some(Keep::DropLowest(n)) => out.push_str(&format!("dl{n}")),
                        None => {}
                    }
                }
            }
        }
        out
    }

    pub fn roll(&self, rng: &mut impl Rng) -> Roll {
        let mut parts = Vec::with_capacity(self.terms.len());
        let mut total = 0i64;
        for term in &self.terms {
            match *term {
                Term::Flat { negative, value } => {
                    total += if negative { -value } else { value };
                    parts.push(Part::Flat { negative, value });
                }
                Term::Dice {
                    negative,
                    count,
                    sides,
                    keep,
                } => {
                    let values: Vec<u32> = (0..count).map(|_| rng.random_range(1..=sides)).collect();
                    let kept = kept_mask(&values, keep);
                    let subtotal: i64 = values
                        .iter()
                        .zip(&kept)
                        .filter(|(_, kept)| **kept)
                        .map(|(value, _)| *value as i64)
                        .sum();
                    total += if negative { -subtotal } else { subtotal };
                    let dice = values
                        .into_iter()
                        .zip(kept)
                        .map(|(value, kept)| Die { value, kept })
                        .collect();
                    parts.push(Part::Dice {
                        negative,
                        count,
                        sides,
                        dice,
                        subtotal,
                    });
                }
            }
        }
        let natural = natural(&parts);
        Roll {
            formula: self.text(),
            label: self.label.clone(),
            edge: self.edge,
            parts,
            total,
            natural,
        }
    }
}

fn kept_mask(values: &[u32], keep: Option<Keep>) -> Vec<bool> {
    let Some(keep) = keep else {
        return vec![true; values.len()];
    };
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by_key(|&i| (values[i], i));
    let n = values.len();
    let (from, to) = match keep {
        Keep::Highest(k) => (n - k as usize, n),
        Keep::Lowest(k) => (0, k as usize),
        Keep::DropHighest(k) => (0, n - k as usize),
        Keep::DropLowest(k) => (k as usize, n),
    };
    let mut mask = vec![false; n];
    for &i in &order[from..to] {
        mask[i] = true;
    }
    mask
}

/// A natural 20 or 1 counts only when the roll's d20s leave one kept die.
fn natural(parts: &[Part]) -> Option<Natural> {
    let mut kept_d20 = parts.iter().filter_map(|part| match part {
        Part::Dice {
            sides: 20,
            dice,
            negative: false,
            ..
        } => Some(dice.iter().filter(|d| d.kept)),
        _ => None,
    });
    let first = kept_d20.next()?;
    if kept_d20.next().is_some() {
        return None;
    }
    let kept: Vec<&Die> = first.collect();
    match kept.as_slice() {
        [die] if die.value == 20 => Some(Natural::Twenty),
        [die] if die.value == 1 => Some(Natural::One),
        _ => None,
    }
}

pub fn roll(input: &str, rng: &mut impl Rng) -> Result<Roll, DiceError> {
    Ok(parse(input)?.roll(rng))
}

impl Roll {
    /// One line for chat, such as `1d20+5 stealth: (14) + 5 = 19`.
    pub fn summary(&self) -> String {
        let mut pieces = Vec::new();
        for (i, part) in self.parts.iter().enumerate() {
            let (negative, text) = match part {
                Part::Flat { negative, value } => (*negative, value.to_string()),
                Part::Dice { negative, dice, .. } => {
                    let shown: Vec<String> = dice
                        .iter()
                        .map(|d| {
                            if d.kept {
                                d.value.to_string()
                            } else {
                                format!("~~{}~~", d.value)
                            }
                        })
                        .collect();
                    (*negative, format!("({})", shown.join(", ")))
                }
            };
            let sign = match (i, negative) {
                (_, true) => "- ",
                (0, false) => "",
                _ => "+ ",
            };
            pieces.push(format!("{sign}{text}"));
        }
        let mut head = self.formula.clone();
        match self.edge {
            Some(Edge::Advantage) => head.push_str(" with advantage"),
            Some(Edge::Disadvantage) => head.push_str(" with disadvantage"),
            None => {}
        }
        if let Some(label) = &self.label {
            head.push_str(&format!(" {label}"));
        }
        let mut line = format!("{head}: {} = **{}**", pieces.join(" "), self.total);
        match self.natural {
            Some(Natural::Twenty) => line.push_str(" (natural 20)"),
            Some(Natural::One) => line.push_str(" (natural 1)"),
            None => {}
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(20261105)
    }

    #[test]
    fn reads_common_formulas() {
        assert_eq!(parse("2d6+3").unwrap().text(), "2d6+3");
        assert_eq!(parse(" 1D20 + 5 ").unwrap().text(), "1d20+5");
        assert_eq!(parse("d20").unwrap().text(), "1d20");
        assert_eq!(parse("d%").unwrap().text(), "1d100");
        assert_eq!(parse("4d6dl1").unwrap().text(), "4d6dl1");
        assert_eq!(parse("2d20kh1+5").unwrap().text(), "2d20kh1+5");
        assert_eq!(parse("2d20k1").unwrap().text(), "2d20kh1");
        assert_eq!(parse("3d8kl2-1").unwrap().text(), "3d8kl2-1");
        assert_eq!(parse("-1+1d4").unwrap().text(), "-1+1d4");
        assert_eq!(parse("1d8+1d6+2").unwrap().text(), "1d8+1d6+2");
    }

    #[test]
    fn keeps_labels_and_reads_advantage_words() {
        let f = parse("1d20+3 stealth").unwrap();
        assert_eq!(f.text(), "1d20+3");
        assert_eq!(f.label.as_deref(), Some("stealth"));
        let f = parse("8d6 fire").unwrap();
        assert_eq!(f.label.as_deref(), Some("fire"));
        let f = parse("1d20+4 adv perception check").unwrap();
        assert_eq!(f.text(), "2d20kh1+4");
        assert_eq!(f.edge, Some(Edge::Advantage));
        assert_eq!(f.label.as_deref(), Some("perception check"));
        let f = parse("1d20 dis").unwrap();
        assert_eq!(f.text(), "2d20kl1");
        assert_eq!(parse("adv").unwrap().text(), "2d20kh1");
        let both = parse("1d20 adv dis").unwrap();
        assert_eq!(both.text(), "1d20");
        assert_eq!(both.edge, None);
    }

    #[test]
    fn refuses_bad_rolls() {
        for bad in [
            "",
            "abc",
            "d",
            "1d",
            "1d0",
            "1d1",
            "0d6",
            "101d6",
            "1d1001",
            "1d6+",
            "1d6++2",
            "++",
            "1d20kh2",
            "4d6dl4",
            "4d6dl0",
            "2d6dx",
            "1d6*2",
            "5",
            "99999999d6",
            "1d6+5000",
            "2d6 adv",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should be refused");
        }
        let many = vec!["1d2"; MAX_TERMS + 1].join("+");
        assert!(parse(&many).is_err());
        assert!(parse("100d6+100d6+1d6").is_err(), "too many dice in all");
        assert!(parse("100d6+100d6").is_ok());
        assert!(parse(&"x".repeat(MAX_INPUT_CHARS + 1)).is_err());
    }

    #[test]
    fn stays_within_bounds() {
        let mut rng = rng();
        for (formula, low, high) in [
            ("3d6+2", 5, 20),
            ("4d6dl1", 3, 18),
            ("2d20kh1+5", 6, 25),
            ("1d4-1d4", -3, 3),
            ("10d10kl3", 3, 30),
            ("d%", 1, 100),
        ] {
            let parsed = parse(formula).unwrap();
            for _ in 0..5000 {
                let roll = parsed.roll(&mut rng);
                assert!((low..=high).contains(&roll.total), "{formula} gave {}", roll.total);
            }
        }
    }

    #[test]
    fn faces_come_up_evenly() {
        let mut rng = rng();
        let d6 = parse("1d6").unwrap();
        let mut counts = [0u32; 6];
        let n = 60_000;
        for _ in 0..n {
            counts[(d6.roll(&mut rng).total - 1) as usize] += 1;
        }
        for count in counts {
            assert!((9_500..=10_500).contains(&count), "uneven d6: {counts:?}");
        }
        let two = parse("2d6").unwrap();
        let mean = (0..n).map(|_| two.roll(&mut rng).total).sum::<i64>() as f64 / n as f64;
        assert!((mean - 7.0).abs() < 0.05, "2d6 mean {mean}");
    }

    #[test]
    fn advantage_shifts_the_average() {
        let mut rng = rng();
        let n = 50_000;
        let mean = |f: &str, rng: &mut StdRng| {
            let parsed = parse(f).unwrap();
            (0..n).map(|_| parsed.roll(rng).total).sum::<i64>() as f64 / n as f64
        };
        let plain = mean("1d20", &mut rng);
        let adv = mean("1d20 adv", &mut rng);
        let dis = mean("1d20 dis", &mut rng);
        assert!((plain - 10.5).abs() < 0.1, "plain {plain}");
        assert!((adv - 13.825).abs() < 0.12, "advantage {adv}");
        assert!((dis - 7.175).abs() < 0.12, "disadvantage {dis}");
    }

    #[test]
    fn advantage_keeps_the_higher_die() {
        let mut rng = rng();
        let parsed = parse("1d20 adv").unwrap();
        for _ in 0..500 {
            let roll = parsed.roll(&mut rng);
            let Part::Dice { dice, subtotal, .. } = &roll.parts[0] else {
                panic!()
            };
            assert_eq!(dice.len(), 2);
            assert_eq!(dice.iter().filter(|d| d.kept).count(), 1);
            assert_eq!(*subtotal, dice.iter().map(|d| d.value as i64).max().unwrap());
        }
    }

    #[test]
    fn marks_naturals_only_on_a_lone_kept_d20() {
        let mut rng = rng();
        let parsed = parse("1d20+5").unwrap();
        let mut twenties = 0;
        for _ in 0..4000 {
            let roll = parsed.roll(&mut rng);
            let Part::Dice { dice, .. } = &roll.parts[0] else {
                panic!()
            };
            match dice[0].value {
                20 => {
                    twenties += 1;
                    assert_eq!(roll.natural, Some(Natural::Twenty));
                }
                1 => assert_eq!(roll.natural, Some(Natural::One)),
                _ => assert_eq!(roll.natural, None),
            }
        }
        assert!(twenties > 120);
        let two = parse("2d20").unwrap();
        assert!((0..500).all(|_| two.roll(&mut rng).natural.is_none()));
    }

    #[test]
    fn summary_shows_dropped_dice() {
        let roll = Roll {
            formula: "2d20kh1+5".into(),
            label: Some("stealth".into()),
            edge: Some(Edge::Advantage),
            parts: vec![
                Part::Dice {
                    negative: false,
                    count: 2,
                    sides: 20,
                    dice: vec![Die { value: 4, kept: false }, Die { value: 15, kept: true }],
                    subtotal: 15,
                },
                Part::Flat {
                    negative: false,
                    value: 5,
                },
            ],
            total: 20,
            natural: None,
        };
        assert_eq!(
            roll.summary(),
            "2d20kh1+5 with advantage stealth: (~~4~~, 15) + 5 = **20**"
        );
    }
}
