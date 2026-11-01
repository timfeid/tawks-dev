//! The same NHL list as Tawks' rich presence guard (libs/rich-presence/src/nhl.rs).
//! PuckLab boards show only PuckLab's own names, so a roster, league, team or
//! club that reads like a real NHL club gets a neutral label instead.

const CODES: &[&str] = &[
    "ANA", "ARI", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA",
    "LAK", "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL",
    "TBL", "TOR", "UTA", "VAN", "VGK", "WPG", "WSH", "LA", "NJ", "SJ", "TB", "MON", "WAS", "CLB",
    "CLS", "NAS", "LV", "VEG", "UTAH", "ATL", "PHX", "HFD", "QUE", "WIN", "MNS", "CLR", "KCS",
    "AFM", "CLE", "OAK", "CGS", "CSE", "PHA", "PIR",
];

const NAMES: &[&str] = &[
    "ducks",
    "bruins",
    "sabres",
    "hurricanes",
    "blue jackets",
    "flames",
    "blackhawks",
    "avalanche",
    "stars",
    "red wings",
    "oilers",
    "panthers",
    "kings",
    "wild",
    "canadiens",
    "habs",
    "devils",
    "predators",
    "islanders",
    "rangers",
    "senators",
    "flyers",
    "penguins",
    "kraken",
    "sharks",
    "blues",
    "lightning",
    "maple leafs",
    "leafs",
    "mammoth",
    "utah hockey club",
    "canucks",
    "golden knights",
    "jets",
    "capitals",
    "coyotes",
    "thrashers",
    "whalers",
    "nordiques",
    "north stars",
    "rockies",
    "scouts",
    "seals",
    "barons",
    "nhl",
];

pub fn is_nhl_code(abbr: &str) -> bool {
    CODES
        .iter()
        .any(|code| code.eq_ignore_ascii_case(abbr.trim()))
}

pub fn has_nhl_name(name: &str) -> bool {
    let words: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    NAMES.iter().any(|nickname| {
        let parts: Vec<&str> = nickname.split(' ').collect();
        words
            .windows(parts.len())
            .any(|window| window == parts.as_slice())
    })
}

/// `name` when it reads as PuckLab's own, otherwise `fallback`.
pub fn neutral(name: &str, fallback: &str) -> String {
    let name = name.trim();
    if name.is_empty() || is_nhl_code(name) || has_nhl_name(name) {
        fallback.to_owned()
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_clubs_get_the_fallback_and_pucklab_names_stay() {
        assert_eq!(neutral("Toronto Maple Leafs", "Club"), "Club");
        assert_eq!(neutral("BOS", "Club"), "Club");
        assert_eq!(
            neutral("NHL Legacy 2026", "Classic roster"),
            "Classic roster"
        );
        assert_eq!(
            neutral("PuckLab League", "Classic roster"),
            "PuckLab League"
        );
        assert_eq!(neutral("Ashmere Harriers", "Club"), "Ashmere Harriers");
        assert_eq!(neutral("  ", "Club"), "Club");
    }
}
