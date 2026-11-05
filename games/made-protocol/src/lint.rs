//! Write-time checks on game.js. They keep games deterministic and honest
//! about what they can reach; the sandbox itself is the fence.

const BANNED: &[(&str, &str)] = &[
    ("Date", "the clock; use ctx.time"),
    ("performance", "the clock; use ctx.time"),
    ("fetch", "the network"),
    ("WebSocket", "the network"),
    ("XMLHttpRequest", "the network"),
    ("eval", "eval"),
    ("Function", "the Function constructor"),
    ("document", "the page"),
    ("window", "the page"),
    ("parent", "the page"),
    ("postMessage", "the page"),
    ("localStorage", "storage"),
    ("sessionStorage", "storage"),
    ("indexedDB", "storage"),
    ("require", "other modules"),
    ("process", "the host"),
    ("__host", "the kit's internals"),
    ("setTimeout", "timers; use ctx.time in tick"),
    ("setInterval", "timers; use ctx.time in tick"),
];

const KIT: &str = "@tawks/made-kit";
pub const SIM_KIT: &str = "@tawks/made-kit/sim";

#[derive(Debug, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Punct(char),
}

fn tokens(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '\'' || c == '"' || c == '`' {
            let quote = c;
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1;
                }
                if let Some(&ch) = chars.get(i) {
                    s.push(ch);
                }
                i += 1;
            }
            i += 1;
            out.push(Token::Str(s));
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            out.push(Token::Ident(chars[start..i].iter().collect()));
        } else {
            if !c.is_whitespace() {
                out.push(Token::Punct(c));
            }
            i += 1;
        }
    }
    out
}

/// The problems with a game.js, as sentences for the agent; empty when it passes.
pub fn lint(src: &str) -> Vec<String> {
    lint_against(src, KIT)
}

/// The same checks for a sim.js, whose one import is the sim kit.
pub fn lint_sim(src: &str) -> Vec<String> {
    lint_against(src, SIM_KIT)
}

fn lint_against(src: &str, kit: &str) -> Vec<String> {
    let toks = tokens(src);
    let mut problems = Vec::new();
    let mut push = |p: String| {
        if !problems.contains(&p) {
            problems.push(p);
        }
    };
    for (i, t) in toks.iter().enumerate() {
        let prev_dot = i > 0 && toks[i - 1] == Token::Punct('.');
        match t {
            Token::Ident(name) => {
                if !prev_dot {
                    if let Some((_, what)) = BANNED.iter().find(|(b, _)| b == name) {
                        push(format!("`{name}` reaches {what}"));
                    }
                }
                if name == "Math" {
                    if let (Some(Token::Punct('.')), Some(Token::Ident(m))) =
                        (toks.get(i + 1), toks.get(i + 2))
                    {
                        if m == "random" {
                            push("`Math.random` is not repeatable; use ctx.rand".into());
                        }
                    }
                }
                if name == "import" {
                    match toks.get(i + 1) {
                        Some(Token::Punct('(')) => push(format!(
                            "`import()` loads other code; only `{kit}` may be imported"
                        )),
                        Some(Token::Punct('.')) => push("`import.meta` is not available".into()),
                        _ => {}
                    }
                }
                if name == "from" || name == "import" {
                    if let Some(Token::Str(spec)) = toks.get(i + 1) {
                        if spec != kit {
                            push(format!("`{spec}` cannot be imported; only `{kit}`"));
                        }
                    }
                }
            }
            Token::Str(s) => {
                let lower = s.to_lowercase();
                if lower.contains("http://")
                    || lower.contains("https://")
                    || lower.contains("wss://")
                    || lower.contains("ws://")
                {
                    push("strings may not hold URLs".into());
                }
            }
            Token::Punct(_) => {}
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::{lint, lint_sim};

    #[test]
    fn the_example_passes() {
        assert!(lint(include_str!("../../../sdk/made-kit/examples/thin-ice.js")).is_empty());
    }

    #[test]
    fn banned_things_are_named() {
        let p = lint(
            "import x from 'fs'; const t = Date.now(); Math.random(); fetch('https://x.y'); import('a');",
        );
        assert!(p.iter().any(|p| p.contains("`fs`")), "{p:?}");
        assert!(p.iter().any(|p| p.contains("Date")));
        assert!(p.iter().any(|p| p.contains("Math.random")));
        assert!(p.iter().any(|p| p.contains("fetch")));
        assert!(p.iter().any(|p| p.contains("URL")));
        assert!(p.iter().any(|p| p.contains("import()")));
    }

    #[test]
    fn a_sim_imports_only_the_sim_kit() {
        assert!(lint_sim(include_str!("../../../sdk/made-kit/examples/spring-lab.js")).is_empty());
        let p = lint_sim("import { defineGame } from '@tawks/made-kit';");
        assert!(p.iter().any(|p| p.contains("cannot be imported")), "{p:?}");
    }

    #[test]
    fn words_in_text_and_properties_are_fine() {
        assert!(lint("const rules = 'Do not fall out the window'; // Date\nstate.parent = 1; p.document = 2;").is_empty());
    }
}
