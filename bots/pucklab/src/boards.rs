//! PuckLab's leaderboards on the PuckLab Online channel: Online Ranked, Linked
//! Career and Lab Rivals. The bot reads them from PuckLab for everyone at
//! once, so viewers (guests too) never reach PuckLab themselves. Every board and
//! roster is sent with the page, so switching tabs or rosters changes only a
//! selection response.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use serde_json::{Value, json};
use tawks_bot::TawksBot;
use tawks_bot::game::PuckLabLeadersReport;
use tawks_bot::workflow::builder::{Cond, WorkflowDisplayBuilder};
use tawks_bot::workflow::ui::{self, Gap, Size, TabStyle, Tone};
use tawks_bot::workflow::{Act, NodeCondition, TableHeader, WorkflowDisplay};
use tokio::sync::Mutex;

use crate::client::{ApiError, PuckLab};
use crate::nhl::neutral;

/// Bot-scoped boards with each account's Tawks link. Until PuckLab serves it
/// the public boards stand in, with plain names and no Lab table.
pub const SERVICE_PATH: &str = "/v1/tawks/leaderboards";
pub const WORKFLOW: &str = "pucklab_my_club";
pub const SELECT_ACTION: &str = "board";
const FRESH_FOR: Duration = Duration::from_secs(60);
const RESEND_EVERY: Duration = Duration::from_secs(600);
const CLASSIC: &str = "Classic roster";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Ranked,
    Career,
    Lab,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Ranked, Kind::Career, Kind::Lab];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Ranked => "ranked",
            Kind::Career => "career",
            Kind::Lab => "lab",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Ranked => "Online Ranked",
            Kind::Career => "Linked Career",
            Kind::Lab => "Lab Rivals",
        }
    }

    fn list(self) -> String {
        format!("plb_{}", self.key())
    }

    pub fn selection(self) -> String {
        format!("plb_{}_pick", self.key())
    }
}

pub struct Snapshot {
    pub data: Value,
    /// True when the entries carry Tawks links (the bot-scoped endpoint).
    pub linked: bool,
}

fn optional(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ApiError>()
        .is_some_and(|e| matches!(e.status, 401 | 403 | 404) || e.pucklab_link_required)
}

pub async fn fetch(api: &PuckLab) -> Result<Snapshot> {
    match api.service_get(SERVICE_PATH, &[("count", "25")]).await {
        Ok(data) => Ok(Snapshot { data, linked: true }),
        Err(error) if optional(&error) => {
            let ranked = api.public_get("/v1/ranked/leaders").await?;
            let career = api.public_get("/v1/career/leaders").await?;
            Ok(Snapshot {
                data: json!({"ranked": ranked, "career": career, "lab": {"rosters": []}}),
                linked: false,
            })
        }
        Err(error) => Err(error),
    }
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|v| v.round() as i64))
        .unwrap_or(0)
}

fn record(entry: &Value) -> String {
    format!(
        "{}-{}-{}",
        int(&entry["wins"]),
        int(&entry["losses"]),
        int(&entry["otl"])
    )
}

fn user(entry: &Value) -> String {
    text(&entry["tawks_user_id"]).to_owned()
}

fn roster_label(roster: &Value) -> String {
    let id = text(&roster["id"]).to_ascii_lowercase();
    if roster["nhl"].as_bool() == Some(true) || id.contains("nhl") {
        return CLASSIC.into();
    }
    neutral(text(&roster["name"]), CLASSIC)
}

fn title_case(word: &str) -> String {
    let lower = word.to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn streak(value: i64) -> String {
    match value {
        0 => "-".into(),
        n if n > 0 => format!("W{n}"),
        n => format!("L{}", -n),
    }
}

fn ranked_row(rank: usize, entry: &Value) -> Value {
    let team = neutral(text(&entry["team"]["name"]), "");
    json!({
        "rank": rank, "user": user(entry),
        "name": neutral_person(text(&entry["name"])),
        "meta": team,
        "tier": title_case(text(&entry["tier"])),
        "rating": int(&entry["rating"]), "games": int(&entry["games"]),
        "record": record(entry), "streak": streak(int(&entry["streak"])),
    })
}

fn career_row(rank: usize, entry: &Value) -> Value {
    let name = match text(&entry["name"]).trim() {
        "" => text(&entry["owner"]),
        name => name,
    };
    json!({
        "rank": rank, "user": user(entry), "name": neutral_person(name),
        "meta": neutral(text(&entry["team"]), ""),
        "position": text(&entry["position"]), "overall": int(&entry["overall"]),
        "games": int(&entry["games"]), "goals": int(&entry["goals"]),
        "assists": int(&entry["assists"]), "points": int(&entry["points"]),
    })
}

fn lab_row(rank: usize, entry: &Value) -> Value {
    let owner = neutral_person(text(&entry["owner"]));
    let diff = int(&entry["goals_for"]) - int(&entry["goals_against"]);
    json!({
        "rank": rank, "user": user(entry),
        "name": neutral(text(&entry["club"]), &format!("{owner}'s club")),
        "meta": owner,
        "games": int(&entry["games"]), "record": record(entry),
        "goal_diff": format!("{diff:+}"), "points": int(&entry["points"]),
    })
}

fn neutral_person(name: &str) -> String {
    match name.trim() {
        "" => "PuckLab player".into(),
        name => name.into(),
    }
}

/// An account id as one URL path segment.
fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b => format!("%{b:02X}"),
        })
        .collect()
}

/// A player's public PuckLab page: their skater, team and career. Without an
/// account it is PuckLab's own leaderboards, so every name has a link and the
/// table needs no condition around it (each one deepens the rendered tree).
pub fn player_page(api: &PuckLab, account: &str) -> String {
    match account {
        "" => api.page("/leaderboards"),
        account => api.page(&format!("/skaters/{}", segment(account))),
    }
}

fn club_page(api: &PuckLab, roster: &str, account: &str) -> String {
    match (roster, account) {
        ("", _) | (_, "") => api.page("/leaderboards"),
        (roster, account) => api.page(&format!("/clubs/{}/{}", segment(roster), segment(account))),
    }
}

fn boards(
    groups: &Value,
    label: impl Fn(&Value) -> String,
    row: fn(usize, &Value) -> Value,
    counted: &str,
    profile: impl Fn(&str, &str) -> String,
) -> Vec<Value> {
    groups
        .as_array()
        .into_iter()
        .flatten()
        .map(|group| {
            let id = group["id"]
                .as_str()
                .or(group["league"].as_str())
                .unwrap_or_default();
            let rows: Vec<Value> = group["leaders"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .map(|(i, entry)| {
                    let account = text(&entry["account"]);
                    let mut row = row(i + 1, entry);
                    row["account"] = json!(account);
                    row["profile"] = json!(profile(id, account));
                    row
                })
                .collect();
            let players = int(&group["players"]).max(rows.len() as i64);
            json!({
                "id": id, "label": label(group), "empty": rows.is_empty(),
                "count": format!("{players} {counted}"), "rows": rows,
            })
        })
        .filter(|board| !board["id"].as_str().unwrap_or_default().is_empty())
        .collect()
}

/// The board data responses, without anyone's selection.
pub fn render(snapshot: &Snapshot, api: &PuckLab) -> HashMap<String, Value> {
    let data = &snapshot.data;
    let player = |_: &str, account: &str| player_page(api, account);
    let ranked = boards(
        &data["ranked"]["rosters"],
        roster_label,
        ranked_row,
        "players",
        player,
    );
    let career = boards(
        &data["career"]["leagues"],
        |league| neutral(text(&league["name"]), "Career league"),
        career_row,
        "skaters",
        player,
    );
    let lab = boards(
        &data["lab"]["rosters"],
        roster_label,
        lab_row,
        "clubs",
        |roster, account| club_page(api, roster, account),
    );
    let mut responses = HashMap::new();
    for (kind, list) in [
        (Kind::Ranked, ranked),
        (Kind::Career, career),
        (Kind::Lab, lab),
    ] {
        responses.insert(format!("{}_many", kind.list()), json!(list.len() > 1));
        responses.insert(kind.list(), Value::Array(list));
    }
    responses.insert(
        "plb_career_open".into(),
        json!(data["career"]["enabled"].as_bool().unwrap_or(false)),
    );
    responses.insert("plb_loaded".into(), json!(true));
    responses
}

/// Keeps a valid board and roster picked: the viewer's own pick when it still
/// exists, otherwise the first.
pub fn selection(
    rendered: &HashMap<String, Value>,
    current: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut picks = HashMap::new();
    for kind in Kind::ALL {
        let ids: Vec<&str> = rendered
            .get(&kind.list())
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|board| board["id"].as_str())
            .collect();
        let mine = current.get(&kind.selection()).and_then(Value::as_str);
        let pick = mine
            .filter(|id| ids.contains(id))
            .or(ids.first().copied())
            .unwrap_or_default();
        picks.insert(kind.selection(), json!(pick));
    }
    let board = current.get("pl_board").and_then(Value::as_str);
    if !Kind::ALL.iter().any(|kind| Some(kind.key()) == board) {
        picks.insert("pl_board".into(), json!(Kind::Ranked.key()));
    }
    picks
}

/// The main Online Ranked ladder: PuckLab League first.
fn main_ladder(data: &Value) -> Option<&Value> {
    let rosters = data["ranked"]["rosters"].as_array()?;
    rosters
        .iter()
        .find(|r| r["kind"] == "league")
        .or_else(|| rosters.iter().find(|r| r["id"] == "tawks.sample"))
        .or_else(|| rosters.iter().find(|r| roster_label(r) != CLASSIC))
}

/// Who holds the PuckLab titles, or None to leave the current holders alone.
/// Only the bot-scoped boards carry Tawks links, and an empty or missing
/// ladder says nothing yet about who leads.
pub fn leaders(snapshot: &Snapshot) -> Option<PuckLabLeadersReport> {
    if !snapshot.linked {
        return None;
    }
    let ladder = main_ladder(&snapshot.data)?;
    let entries = ladder["leaders"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    if entries.is_empty() {
        return None;
    }
    let linked = |entry: &Value| Some(user(entry)).filter(|id| !id.is_empty());
    let leader = entries.first().and_then(linked);
    let streakers = ladder["streak_leaders"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(entries);
    let best = streakers
        .iter()
        .map(|e| int(&e["streak"]))
        .max()
        .unwrap_or(0);
    let mut hot_hand: Vec<String> = if best > 0 {
        streakers
            .iter()
            .filter(|e| int(&e["streak"]) == best)
            .filter_map(linked)
            .collect()
    } else {
        Vec::new()
    };
    hot_hand.dedup();
    hot_hand.truncate(tawks_game_model::pucklab_leaders::HOLDERS_MAX);
    Some(PuckLabLeadersReport { leader, hot_hand })
}

#[derive(Default)]
struct Cached {
    at: Option<Instant>,
    responses: HashMap<String, Value>,
    reported: Option<(PuckLabLeadersReport, Instant)>,
    broadcast: Option<(HashMap<String, Value>, Instant)>,
}

#[derive(Default)]
pub struct BoardCache {
    inner: Mutex<Cached>,
}

impl BoardCache {
    /// The latest boards, read from PuckLab when they are missing or stale.
    pub async fn responses(&self, api: &PuckLab) -> HashMap<String, Value> {
        let mut cached = self.inner.lock().await;
        if cached.at.is_none_or(|at| at.elapsed() > FRESH_FOR) {
            match fetch(api).await {
                Ok(snapshot) => {
                    cached.responses = render(&snapshot, api);
                    cached.at = Some(Instant::now());
                }
                Err(error) => tracing::warn!(%error, "PuckLab boards could not be read"),
            }
        }
        cached.responses.clone()
    }

    /// One pass: refresh the boards, push changes to open pages and report the
    /// title holders when they change.
    pub async fn sync(&self, bot: &TawksBot, api: &PuckLab) -> Result<()> {
        let snapshot = fetch(api).await?;
        let responses = render(&snapshot, api);
        let report = leaders(&snapshot);
        let mut cached = self.inner.lock().await;
        cached.responses = responses.clone();
        cached.at = Some(Instant::now());
        let stale = |sent: &Option<(HashMap<String, Value>, Instant)>| {
            sent.as_ref()
                .is_none_or(|(old, at)| *old != responses || at.elapsed() > RESEND_EVERY)
        };
        if stale(&cached.broadcast) {
            let args = json!({
                "workflow_definition_id": WORKFLOW,
                "filter": {"user_id": null, "response_equals": null, "include_completed": false},
                "updates": responses,
            });
            bot.sdk
                .web_client
                .mutate::<_, Value>("workflow_broadcast_updates", &args)
                .await?;
            cached.broadcast = Some((responses.clone(), Instant::now()));
        }
        if let Some(report) = report {
            let due = cached
                .reported
                .as_ref()
                .is_none_or(|(old, at)| *old != report || at.elapsed() > RESEND_EVERY);
            if due {
                bot.report_pucklab_leaders(&report).await?;
                cached.reported = Some((report, Instant::now()));
            }
        }
        Ok(())
    }
}

fn picked(kind: Kind) -> NodeCondition {
    Cond::response_equals("b.id", json!(format!("{{{}}}", kind.selection())))
}

fn showing(kind: Kind) -> NodeCondition {
    Cond::response_equals("pl_board", json!(kind.key()))
}

pub fn select(key: &str, value: &str) -> Act {
    ui::act(SELECT_ACTION).input(key, value)
}

fn header(label: &str, key: &str, numeric: bool, wide_only: bool) -> TableHeader {
    TableHeader {
        label: label.into(),
        key: key.into(),
        width: None,
        align: None,
        numeric,
        wide_only,
        image: None,
    }
}

fn table(columns: &[(&str, &str, bool)]) -> WorkflowDisplay {
    let headers = columns
        .iter()
        .map(|(key, label, wide)| {
            header(
                label,
                key,
                !matches!(*key, "name" | "tier" | "position" | "profile"),
                *wide,
            )
        })
        .collect();
    let cells = columns
        .iter()
        .map(|(key, _, _)| match *key {
            "name" => ui::person("", "{r.name}")
                .size(Size::ExtraSmall)
                .meta("{r.meta}")
                .user("{r.user}"),
            "profile" => profile_link("{r.profile}"),
            "rank" => ui::body("{r.rank}").numeric().tone(Tone::Muted),
            key => ui::body(format!("{{r.{key}}}")).numeric(),
        })
        .collect();
    WorkflowDisplayBuilder::new()
        .table("b.rows", "r", headers, cells)
        .build()
}

/// A small link to a player's PuckLab page, beside a name that opens their Tawks profile.
pub fn profile_link(href: &str) -> WorkflowDisplay {
    let mut icon = ui::icon("external-link", ui::IconSize::Small);
    if let tawks_bot::workflow::DisplayType::Icon { aria_label, .. } = &mut icon.r#type {
        *aria_label = Some("PuckLab profile".into());
    }
    ui::link(href, ui::LinkStyle::Inline, vec![icon])
}

fn columns(kind: Kind) -> WorkflowDisplay {
    match kind {
        Kind::Ranked => table(&[
            ("rank", "#", false),
            ("name", "Player", false),
            ("profile", "", false),
            ("rating", "Rating", false),
            ("tier", "Tier", true),
            ("games", "GP", true),
            ("record", "W-L-OTL", false),
            ("streak", "Streak", true),
        ]),
        Kind::Career => table(&[
            ("rank", "#", false),
            ("name", "Skater", false),
            ("profile", "", false),
            ("position", "Pos", true),
            ("overall", "OVR", true),
            ("games", "GP", false),
            ("goals", "G", false),
            ("assists", "A", true),
            ("points", "PTS", false),
        ]),
        Kind::Lab => table(&[
            ("rank", "#", false),
            ("name", "Club", false),
            ("profile", "", false),
            ("games", "GP", true),
            ("record", "W-L-OTL", false),
            ("goal_diff", "GD", true),
            ("points", "PTS", false),
        ]),
    }
}

fn empty_copy(kind: Kind) -> &'static str {
    match kind {
        Kind::Ranked => "No ranked games on this roster yet. Win one to take the top spot.",
        Kind::Career => "No Linked Career games yet.",
        Kind::Lab => "No Lab Rivals results yet.",
    }
}

fn roster_switch(kind: Kind) -> WorkflowDisplay {
    ui::when(
        Cond::response_equals(&format!("{}_many", kind.list()), json!(true)),
        vec![ui::items(
            &kind.list(),
            "b",
            ui::Layout::Row { gap: Gap::Tight },
            vec![
                ui::when(
                    picked(kind),
                    vec![ui::chip("{b.label}").tone(Tone::Primary)],
                ),
                ui::when(
                    Cond::not(picked(kind)),
                    vec![ui::action(
                        "{b.label}",
                        ui::ActionLook::Ghost,
                        select(&kind.selection(), "{b.id}"),
                    )],
                ),
            ],
        )],
    )
}

fn board(kind: Kind) -> WorkflowDisplay {
    let mut content = vec![roster_switch(kind)];
    if kind == Kind::Career {
        content.push(ui::when(
            Cond::response_equals("plb_career_open", json!(false)),
            vec![ui::fine(
                "Linked Career is not open on PuckLab yet. Its leaders appear here when it opens.",
            )],
        ));
    }
    content.push(ui::when(
        Cond::response_list_empty(&kind.list()),
        vec![ui::empty(empty_copy(kind)).with_icon("trophy")],
    ));
    content.push(ui::items(
        &kind.list(),
        "b",
        ui::Layout::Stack { gap: Gap::Normal },
        vec![ui::when(
            picked(kind),
            vec![
                ui::row(
                    Gap::Normal,
                    vec![ui::subheading("{b.label}"), ui::fine("{b.count}")],
                )
                .between(),
                ui::when(
                    Cond::response_equals("b.empty", json!(true)),
                    vec![ui::empty(empty_copy(kind)).with_icon("trophy")],
                ),
                ui::when(
                    Cond::response_equals("b.empty", json!(false)),
                    vec![columns(kind)],
                ),
            ],
        )],
    ));
    ui::when(showing(kind), vec![ui::column(Gap::Normal, content)])
}

/// The Leaderboards tab: a board switcher, then the picked board.
pub fn page() -> Vec<WorkflowDisplay> {
    vec![
        ui::tabs(
            TabStyle::Segmented,
            Kind::ALL
                .iter()
                .map(|kind| ui::tab(kind.label(), showing(*kind), select("pl_board", kind.key())))
                .collect(),
        ),
        ui::when(
            Cond::not(Cond::response_exists("plb_loaded")),
            vec![ui::loading("Getting the PuckLab leaderboards")],
        ),
        ui::when(
            Cond::response_exists("plb_loaded"),
            Kind::ALL.iter().map(|kind| board(*kind)).collect(),
        ),
        ui::fine(
            "Names that open a profile belong to PuckLab accounts linked to Tawks. The arrow opens their PuckLab page.",
        ),
    ]
}

/// Defaults for a new page, before the first read.
pub fn defaults() -> Vec<(String, Value)> {
    let mut values = vec![("pl_board".to_string(), json!(Kind::Ranked.key()))];
    for kind in Kind::ALL {
        values.push((kind.list(), json!([])));
        values.push((kind.selection(), json!("")));
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api() -> PuckLab {
        PuckLab::new(
            "https://pucklab.example.com",
            "test-bot-secret-not-a-user-token-123".into(),
            1,
            1,
        )
        .unwrap()
    }

    fn snapshot(linked: bool) -> Snapshot {
        Snapshot {
            linked,
            data: json!({
                "ranked": {"rosters": [
                    {"id": "nhl-legacy", "name": "NHL 2026", "nhl": true, "players": 1,
                     "leaders": [{"account": "a9", "name": "zed", "rating": 1001.6, "tier": "SILVER", "games": 6, "wins": 3, "losses": 3, "otl": 0, "streak": -1, "team": {"name": "Boston Bruins"}}]},
                    {"id": "tawks.sample", "name": "PuckLab League", "kind": "league", "players": 3, "leaders": [
                        {"account": "a1", "name": "ana", "tawks_user_id": "USR-ana", "rating": 1210, "tier": "GOLD", "games": 9, "wins": 7, "losses": 1, "otl": 1, "streak": 4, "team": {"name": "Toronto Maple Leafs"}},
                        {"account": "a2", "name": "bo", "tawks_user_id": null, "rating": 1100, "tier": "GOLD", "games": 7, "wins": 4, "losses": 3, "otl": 0, "streak": 4, "team": {"name": "Ashmere Harriers"}},
                        {"account": "a3", "name": "cy", "tawks_user_id": "USR-cy", "rating": 990, "tier": "PLACEMENT", "games": 3, "wins": 1, "losses": 2, "otl": 0, "streak": 0}
                    ]}
                ]},
                "career": {"enabled": false, "leagues": []},
                "lab": {"rosters": [{"id": "nhl-legacy", "name": "NHL Legacy", "nhl": true, "players": 1, "leaders": [
                    {"account": "a1", "owner": "ana", "tawks_user_id": "USR-ana", "club": "Rangers", "games": 4, "wins": 3, "losses": 1, "otl": 0, "goals_for": 14, "goals_against": 9, "points": 6}
                ]}]}
            }),
        }
    }

    #[test]
    fn boards_use_neutral_labels_and_keep_links() {
        let rendered = render(&snapshot(true), &api());
        let ranked = rendered["plb_ranked"].as_array().unwrap();
        assert_eq!(ranked[0]["label"], CLASSIC);
        assert_eq!(ranked[1]["label"], "PuckLab League");
        let ana = &ranked[1]["rows"][0];
        assert_eq!(ana["user"], "USR-ana");
        assert_eq!(ana["meta"], "", "an NHL team name never reaches the page");
        assert_eq!(ana["tier"], "Gold");
        assert_eq!(ana["record"], "7-1-1");
        assert_eq!(ana["streak"], "W4");
        assert_eq!(ranked[1]["rows"][1]["user"], "");
        assert_eq!(ranked[1]["rows"][1]["meta"], "Ashmere Harriers");
        assert_eq!(ranked[0]["rows"][0]["rating"], 1002);
        let lab = &rendered["plb_lab"][0];
        assert_eq!(lab["label"], CLASSIC);
        assert_eq!(lab["rows"][0]["name"], "ana's club");
        assert_eq!(lab["rows"][0]["goal_diff"], "+5");
        assert_eq!(rendered["plb_ranked_many"], true);
        assert_eq!(rendered["plb_career"], json!([]));
        assert_eq!(ana["profile"], "https://pucklab.example.com/skaters/a1");
        assert_eq!(
            lab["rows"][0]["profile"],
            "https://pucklab.example.com/clubs/nhl-legacy/a1"
        );
        let encoded = serde_json::to_string(&rendered).unwrap();
        for banned in ["Bruins", "Maple Leafs", "Rangers", "NHL"] {
            assert!(!encoded.contains(banned), "{banned}");
        }
    }

    #[test]
    fn profile_links_escape_account_ids() {
        assert_eq!(
            player_page(&api(), "a b/c?d"),
            "https://pucklab.example.com/skaters/a%20b%2Fc%3Fd"
        );
        assert_eq!(
            player_page(&api(), ""),
            "https://pucklab.example.com/leaderboards"
        );
    }

    #[test]
    fn selections_keep_valid_picks_and_default_to_the_first_board() {
        let rendered = render(&snapshot(true), &api());
        let fresh = selection(&rendered, &HashMap::new());
        assert_eq!(fresh["plb_ranked_pick"], "nhl-legacy");
        assert_eq!(fresh["plb_career_pick"], "");
        assert_eq!(fresh["pl_board"], "ranked");
        let mine = HashMap::from([
            ("plb_ranked_pick".to_string(), json!("tawks.sample")),
            ("pl_board".to_string(), json!("lab")),
        ]);
        let kept = selection(&rendered, &mine);
        assert_eq!(kept["plb_ranked_pick"], "tawks.sample");
        assert!(!kept.contains_key("pl_board"));
        let gone = HashMap::from([("plb_ranked_pick".to_string(), json!("removed"))]);
        assert_eq!(selection(&rendered, &gone)["plb_ranked_pick"], "nhl-legacy");
    }

    #[test]
    fn the_league_ladder_names_the_title_holders() {
        let report = leaders(&snapshot(true)).unwrap();
        assert_eq!(report.leader.as_deref(), Some("USR-ana"));
        assert_eq!(
            report.hot_hand,
            vec!["USR-ana".to_string()],
            "unlinked co-holders are skipped"
        );
        assert!(
            leaders(&snapshot(false)).is_none(),
            "public boards cannot name Tawks users"
        );
        let mut unlinked_top = snapshot(true);
        unlinked_top.data["ranked"]["rosters"][1]["leaders"][0]["tawks_user_id"] = Value::Null;
        assert_eq!(
            leaders(&unlinked_top).unwrap().leader,
            None,
            "the title is not passed down to second place"
        );
    }

    #[test]
    fn nothing_is_reported_until_linked_boards_name_a_leader() {
        assert!(
            leaders(&snapshot(false)).is_none(),
            "public boards cannot name Tawks users"
        );
        let ladder = json!({"ranked": {"rosters": [{"id": "tawks.sample", "name": "PuckLab League", "kind": "league", "leaders": []}]}});
        for linked in [false, true] {
            let empty = Snapshot {
                linked,
                data: ladder.clone(),
            };
            assert!(
                leaders(&empty).is_none(),
                "an empty ladder leaves the holders alone (linked: {linked})"
            );
        }
        let missing = Snapshot {
            linked: true,
            data: json!({"ranked": {"rosters": []}}),
        };
        assert!(leaders(&missing).is_none());
    }

    #[test]
    fn streak_leaders_from_pucklab_win_over_the_page() {
        let mut data = snapshot(true);
        data.data["ranked"]["rosters"][1]["streak_leaders"] =
            json!([{"tawks_user_id": "USR-far", "streak": 9}]);
        assert_eq!(
            leaders(&data).unwrap().hot_hand,
            vec!["USR-far".to_string()]
        );
    }

    #[test]
    fn the_page_sends_no_classes_and_every_tab_selects_a_board() {
        let encoded = serde_json::to_string(&page()).unwrap();
        assert!(!encoded.contains("\"class"));
        for kind in Kind::ALL {
            assert!(encoded.contains(&format!("\"pl_board\":\"{}\"", kind.key())));
        }
    }
}
