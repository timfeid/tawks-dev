use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, ensure};
use serde_json::{Value, json};
use tawks_bot::workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowNodeBuilder,
    WorkflowVisualBuilder,
};
use tawks_bot::workflow::server_action::ServerActionResult;
use tawks_bot::workflow::service::WorkflowResourceWithContext;
use tawks_bot::workflow::ui::{self, Gap, TextRole, Tone};
use tawks_bot::workflow::{CreateWorkflowDefinition, NodeCondition, StickyPosition, WorkflowDisplay};
use tawks_bot::{ServerActionHandler, TawksBot};

use crate::boards::{self, BoardCache, Kind};
use crate::client::{ApiError, PuckLab};

/// The channel's own chat, shown on the Chat tab like a team hub's.
pub const CHAT_ID: &str = "hockey-pucklab";

pub(crate) fn text(role: TextRole, label: &str) -> WorkflowDisplay {
    ui::text(role, label)
}

/// The workflow owner comes from Tawks, not a response field, display name,
/// reconciler-supplied context or a caller-selected PuckLab account ID.
fn actor(ctx: &WorkflowResourceWithContext) -> Result<&str> {
    ensure!(
        ctx.workflow.inject.is_none(),
        "Open PuckLab Online yourself to see your stats."
    );
    ensure!(!ctx.workflow.user_id.is_empty(), "Sign in to Tawks first.");
    Ok(&ctx.workflow.user_id)
}

fn field(row: &Value, key: &str) -> String {
    match &row[key] {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The viewer's own rows on the boards the bot already holds, one card per
/// board and roster they appear on.
fn my_stats(board: &HashMap<String, Value>, account: &str, user: &str) -> Vec<Value> {
    let mine = |row: &Value| {
        (!account.is_empty() && row["account"] == account)
            || (!user.is_empty() && row["user"] == user)
    };
    let mut cards = Vec::new();
    for kind in Kind::ALL {
        let Some(lists) = board.get(&format!("plb_{}", kind.key())).and_then(Value::as_array) else {
            continue;
        };
        for list in lists {
            let roster = field(list, "label");
            for row in list["rows"].as_array().into_iter().flatten().filter(|r| mine(r)) {
                let rank = format!("#{} on {roster}", field(row, "rank"));
                let (board, value, label, detail) = match kind {
                    Kind::Ranked => (
                        "Online Ranked",
                        field(row, "rating"),
                        "Rating",
                        format!("{} · {} · {} GP", field(row, "tier"), field(row, "record"), field(row, "games")),
                    ),
                    Kind::Career => (
                        "Linked Career",
                        field(row, "points"),
                        "Points",
                        format!("{} G · {} A · {} GP", field(row, "goals"), field(row, "assists"), field(row, "games")),
                    ),
                    Kind::Lab => (
                        "Lab Rivals",
                        field(row, "record"),
                        "W-L-OTL",
                        format!("{} · {} pts · {} GD", field(row, "name"), field(row, "points"), field(row, "goal_diff")),
                    ),
                };
                cards.push(json!({"board": board, "value": value, "label": label, "rank": rank, "detail": detail}));
            }
        }
    }
    cards
}

/// The viewer's part of the page: whether their PuckLab account is linked, and
/// their rows on the boards. One account read; the boards are already cached.
async fn me(api: &PuckLab, board: &HashMap<String, Value>, ctx: &WorkflowResourceWithContext) -> HashMap<String, Value> {
    let mut out = HashMap::from([
        ("pl_me_stats".to_string(), json!([])),
        ("pl_me_link".to_string(), json!(api.page("/account"))),
    ]);
    let Ok(subject) = actor(ctx) else {
        out.insert("pl_me".into(), json!("guest"));
        return out;
    };
    match api.get(subject, "/v1/account", &[], None).await {
        Ok(account) => {
            let id = account["account_id"].as_str().unwrap_or_default();
            let stats = my_stats(board, id, subject);
            out.insert("pl_me".into(), json!("linked"));
            out.insert("pl_me_name".into(), json!(account["name"].as_str().unwrap_or("Your PuckLab account")));
            out.insert("pl_me_profile".into(), json!(match id {
                "" => api.page("/my-career"),
                id => boards::player_page(api, id),
            }));
            out.insert("pl_me_ranked".into(), json!(!stats.is_empty()));
            out.insert("pl_me_stats".into(), json!(stats));
        }
        Err(error) if error.downcast_ref::<ApiError>().is_some_and(|e| e.pucklab_link_required) => {
            out.insert("pl_me".into(), json!("unlinked"));
        }
        Err(error) => {
            tracing::warn!(%error, "PuckLab account read failed");
            out.insert("pl_me".into(), json!("error"));
        }
    }
    out
}

pub async fn register(bot: &TawksBot, api: Arc<PuckLab>, board_cache: Arc<BoardCache>) -> Result<String> {
    let site = api.page("/");
    let handler: ServerActionHandler = Arc::new(Box::new(move |ctx| {
        let api = api.clone();
        let board_cache = board_cache.clone();
        Box::pin(async move {
            // The boards are public, so they load before any account check.
            let mut responses = board_cache.responses(&api).await;
            responses.extend(boards::selection(&responses, &ctx.workflow.responses));
            responses.extend(me(&api, &responses, &ctx).await);
            Ok(ServerActionResult::UpdateResponses(responses))
        })
    }));
    let refresh = bot.register_server_action("pucklab.club.refresh", handler).await?;
    bot.register_workflow(definition(&refresh, &site)).await
}

fn on_tab(tab: &str) -> NodeCondition {
    Cond::response_equals("pl_tab", json!(tab))
}

fn page_tabs() -> WorkflowDisplay {
    let tab = |label: &str, icon: &str, active: NodeCondition, key: &str| {
        ui::tab(label, active, boards::select("pl_tab", key)).icon(icon)
    };
    ui::tabs(
        ui::TabStyle::Underline,
        vec![
            tab("Leaderboards", "trophy", Cond::not(Cond::or([on_tab("stats"), on_tab("chat")])), "boards"),
            tab("My stats", "user", on_tab("stats"), "stats"),
            tab("Chat", "message-circle", on_tab("chat"), "chat"),
        ],
    )
}

fn me_is(state: &str) -> NodeCondition {
    Cond::response_equals("pl_me", json!(state))
}

fn stat_card() -> WorkflowDisplay {
    ui::panel(vec![ui::column(
        Gap::Tight,
        vec![
            ui::eyebrow("{s.board}"),
            ui::row(Gap::Tight, vec![ui::stat_text("{s.value}"), ui::fine("{s.label}")]),
            ui::body("{s.rank}"),
            ui::fine("{s.detail}"),
        ],
    )])
}

fn stats_page() -> Vec<WorkflowDisplay> {
    let deeper = ui::link_primary("{pl_me_profile}", "Full stats and club on PuckLab");
    vec![
        ui::when(Cond::not(Cond::response_exists("pl_me")), vec![ui::loading("Getting your PuckLab stats")]),
        ui::when(
            me_is("guest"),
            vec![ui::empty("Sign in to see your PuckLab stats.").with_icon("user")],
        ),
        ui::when(
            me_is("error"),
            vec![ui::notice(Tone::Warning, "Your PuckLab stats could not be loaded. Try again in a minute.")],
        ),
        ui::when(
            me_is("unlinked"),
            vec![ui::column(
                Gap::Normal,
                vec![
                    ui::heading("Link your PuckLab account"),
                    ui::body("Link PuckLab to Tawks and your rating, record and Lab Rivals results show up here."),
                    ui::row(Gap::Normal, vec![ui::link_secondary("{pl_me_link}", "Link PuckLab account")]),
                ],
            )],
        ),
        ui::when(
            me_is("linked"),
            vec![ui::column(
                Gap::Loose,
                vec![
                    ui::row(
                        Gap::Normal,
                        vec![
                            ui::column(
                                Gap::None,
                                vec![
                                    ui::eyebrow("Your PuckLab stats"),
                                    ui::heading("{pl_me_name}"),
                                    ui::fine("Your club, skater and jerseys are set up in PuckLab."),
                                ],
                            ),
                            deeper,
                        ],
                    )
                    .between()
                    .wrap(),
                    ui::when(
                        Cond::response_equals("pl_me_ranked", json!(true)),
                        vec![ui::items("pl_me_stats", "s", ui::Layout::Grid { columns: 3, gap: Gap::Normal, fixed: false }, vec![stat_card()])],
                    ),
                    ui::when(
                        Cond::response_equals("pl_me_ranked", json!(false)),
                        vec![ui::empty("You are not on a leaderboard yet. Play Online Ranked or Lab Rivals in PuckLab to get on one.").with_icon("trophy")],
                    ),
                ],
            )],
        ),
    ]
}

/// Every view opens with the same title and puts the tabs in the dock, as the
/// team hubs do.
fn view_node(name: &str, active: NodeCondition, site: &str, page: Vec<WorkflowDisplay>) -> tawks_bot::workflow::WorkflowNode {
    let mut content = vec![ui::row(
        Gap::Normal,
        vec![text(TextRole::Title, "PuckLab Online"), ui::play_game("pucklab", "Play now", site)],
    )];
    content.extend(page);
    WorkflowNodeBuilder::new(name)
        .condition(active)
        .visual(WorkflowVisualBuilder::new("pucklab-tabs").sticky(StickyPosition::Top, vec![page_tabs()]).build())
        .visual(WorkflowVisualBuilder::new("pucklab-page").page(tawks_bot::workflow::PageWidth::Wide, content).build())
        .build()
}

fn definition(refresh: &str, site: &str) -> CreateWorkflowDefinition {
    let main = view_node(
        "PuckLab Online",
        Cond::not(on_tab("chat")),
        site,
        vec![
            ui::when(Cond::not(on_tab("stats")), boards::page()),
            ui::when(on_tab("stats"), stats_page()),
        ],
    );
    let mut chat = view_node("PuckLab Online chat", on_tab("chat"), site, vec![]);
    chat.visuals.push(WorkflowVisualBuilder::new("pucklab-chat").chat(CHAT_ID).build());
    let mut builder = WorkflowDefinitionBuilder::new(boards::WORKFLOW, "PuckLab Online")
        .description("PuckLab leaderboards, your own stats and the PuckLab chat.")
        .starting_server_action(refresh)
        .response("pl_tab", json!("boards"))
        .add_node("main", main)
        .add_node("chat", chat)
        .add_action(
            WorkflowActionBuilder::new(boards::SELECT_ACTION)
                .transform_responses(vec![], true)
                .allow_while_loading()
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("refresh")
                .run_server_action()
                .target(refresh)
                .build(),
        );
    for (key, value) in boards::defaults() {
        builder = builder.response(&key, value);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> WorkflowResourceWithContext {
        serde_json::from_value(json!({
            "workflow": {"inject":null,"instance_id":"fixture-instance","workflow_id":"pucklab_my_club",
                "stale_definition":false,"name":"PuckLab Online","description":null,"responses":{},"completed":false,
                "status":"active","complete_message":null,"visuals":[],"layout":null,"user_id":"verified-owner",
                "current_node_id":"main","waiting":false,"attached_workflows":{},"parent":null,
                "created_at":"2026-10-27","updated_at":"2026-10-27","completed_at":null},
            "context":{}
        }))
        .unwrap()
    }

    #[test]
    fn caller_inputs_and_context_cannot_select_another_pucklab_account() {
        let mut ctx = context();
        ctx.context.insert("$user".into(), json!({"id":"another-player"}));
        ctx.workflow.responses.insert("user_id".into(), json!("another-player"));
        ctx.workflow.responses.insert("account_id".into(), json!("acct-other"));
        assert_eq!(actor(&ctx).unwrap(), "verified-owner");
        ctx.workflow.inject = Some(tawks_bot::workflow::service::InjectWorkflowPlace::This);
        assert!(actor(&ctx).is_err());
    }

    #[test]
    fn my_stats_come_from_my_own_rows() {
        let board = HashMap::from([
            ("plb_ranked".to_string(), json!([{"label": "PuckLab League", "rows": [
                {"rank": 1, "account": "a-me", "user": "", "rating": 1312, "tier": "Platinum", "record": "15-4-2", "games": 21},
                {"rank": 2, "account": "a-other", "user": "USR-other", "rating": 1100, "tier": "Gold", "record": "1-0-0", "games": 1}
            ]}])),
            ("plb_lab".to_string(), json!([{"label": "Classic roster", "rows": [
                {"rank": 3, "account": "", "user": "USR-me", "name": "Night Shift HC", "record": "6-1-1", "points": 13, "goal_diff": "+14"}
            ]}])),
        ]);
        let stats = my_stats(&board, "a-me", "USR-me");
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0]["value"], "1312");
        assert_eq!(stats[0]["rank"], "#1 on PuckLab League");
        assert_eq!(stats[0]["detail"], "Platinum · 15-4-2 · 21 GP");
        assert_eq!(stats[1]["board"], "Lab Rivals");
        assert_eq!(stats[1]["detail"], "Night Shift HC · 13 pts · +14 GD");
        assert!(my_stats(&board, "", "").is_empty());
    }

    #[test]
    fn the_page_edits_nothing_and_sends_no_classes() {
        let encoded = serde_json::to_string(&definition("fixture-action", "https://pucklab.example.com/")).unwrap();
        assert!(!encoded.contains("\"class"));
        assert!(!encoded.contains("\"Form\""), "club editing lives in PuckLab");
        assert!(!encoded.contains("\"Input\""));
        for tab in ["boards", "stats", "chat"] {
            assert!(encoded.contains(&format!("\"pl_tab\":\"{tab}\"")), "{tab}");
        }
        assert!(encoded.contains(CHAT_ID));
        assert!(encoded.contains(r#"{"PlayGame":{"game":"pucklab","label":"Play now","fallback_url":"https://pucklab.example.com/"}}"#));
        for banned in ["NHL", "HUT", "EA ", "\u{2014}"] {
            assert!(!encoded.contains(banned), "{banned}");
        }
    }
}
