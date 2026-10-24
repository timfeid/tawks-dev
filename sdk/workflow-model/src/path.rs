use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSeg {
    Key(String),
    Index(usize),
}

fn parse_path(path: &str) -> Option<Vec<PathSeg>> {
    if path.is_empty() {
        return None;
    }

    let chars: Vec<char> = path.chars().collect();
    let mut segs: Vec<PathSeg> = Vec::new();
    let mut i = 0;

    fn read_ident(chars: &[char], i: &mut usize) -> Option<String> {
        let start = *i;
        while *i < chars.len() && chars[*i] != '.' && chars[*i] != '[' {
            *i += 1;
        }
        if *i == start {
            None
        } else {
            Some(chars[start..*i].iter().collect())
        }
    }

    while i < chars.len() {
        match chars[i] {
            '.' => {
                i += 1;
            }
            '[' => {
                i += 1;
                if i >= chars.len() {
                    return None;
                }

                if chars[i] == '"' {
                    i += 1;
                    let start = i;
                    while i < chars.len() && chars[i] != '"' {
                        i += 1;
                    }
                    if i >= chars.len() || chars[i] != '"' {
                        return None;
                    }
                    let key: String = chars[start..i].iter().collect();
                    i += 1;

                    if i >= chars.len() || chars[i] != ']' {
                        return None;
                    }
                    i += 1;
                    segs.push(PathSeg::Key(key));
                    continue;
                }

                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i == start {
                    return None;
                }
                let idx_str: String = chars[start..i].iter().collect();
                if i >= chars.len() || chars[i] != ']' {
                    return None;
                }
                i += 1;
                segs.push(PathSeg::Index(idx_str.parse::<usize>().ok()?));
            }
            _ => {
                let ident = read_ident(&chars, &mut i)?;
                segs.push(PathSeg::Key(ident));
            }
        }
    }

    Some(segs)
}

pub fn get_path_from_value<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let segs = parse_path(path)?;
    let mut cur = root;

    for seg in segs {
        match seg {
            PathSeg::Key(k) => {
                cur = cur.get(&k)?;
            }
            PathSeg::Index(n) => {
                cur = cur.get(n)?;
            }
        }
    }

    Some(cur)
}

fn walk_value<'a>(mut cur: &'a Value, segs: &[PathSeg]) -> Option<&'a Value> {
    for seg in segs {
        cur = match seg {
            PathSeg::Key(k) => cur.get(k)?,
            PathSeg::Index(i) => cur.get(*i)?,
        };
    }
    Some(cur)
}

fn walk_map_root<'a>(
    root: &'a std::collections::HashMap<String, Value>,
    segs: &[PathSeg],
) -> Option<&'a Value> {
    let (first, rest) = segs.split_first()?;
    let PathSeg::Key(k) = first else {
        return None;
    };

    let v = root.get(k)?;
    if rest.is_empty() {
        return Some(v);
    }
    walk_value(v, rest)
}

/// `$` keys such as `$user` are set by the server, so they come from the
/// context first; a response a client wrote under the same name never wins.
pub fn get_path_any<'a>(
    responses: &'a std::collections::HashMap<String, Value>,
    context: &'a std::collections::HashMap<String, Value>,
    arg: &str,
) -> Option<&'a Value> {
    let segs = parse_path(arg)?;
    if arg.starts_with('$') {
        return context
            .get(arg)
            .or_else(|| walk_map_root(context, &segs))
            .or_else(|| responses.get(arg))
            .or_else(|| walk_map_root(responses, &segs));
    }
    if let Some(v) = responses.get(arg) {
        return Some(v);
    }
    if let Some(v) = context.get(arg) {
        return Some(v);
    }
    walk_map_root(responses, &segs).or_else(|| walk_map_root(context, &segs))
}

