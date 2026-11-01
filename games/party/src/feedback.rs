//! Ratings of minigame bouts. Each Tawkathon, and each run of bouts played on
//! their own, gets one Linear issue, opened by its first rating, and every
//! rating after it is a comment on that issue.
//! Ratings/notes are always saved through the API first; Linear is optional.
//! The client is acknowledged only after the durable database write succeeds.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Value, json};
use tawks_bot::web_client::WebClient;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::room::BoutFeedback;
use crate::server::{FeedbackSink,FeedbackFuture};

#[derive(Clone,Debug)]
pub struct FeedbackReceipt{pub rewarded:bool}

const LINEAR_URL: &str = "https://api.linear.app/graphql";

/// A Linear API key and the team the issues are opened in.
#[derive(Clone)]
pub struct LinearConfig {
    pub api_key: String,
    pub team_id: String,
}

impl LinearConfig {
    /// From `LINEAR_API_KEY` and `LINEAR_TEAM_ID`, when both are set.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("LINEAR_API_KEY").ok().filter(|v| !v.trim().is_empty())?;
        let team_id = std::env::var("LINEAR_TEAM_ID").ok().filter(|v| !v.trim().is_empty())?;
        Some(Self { api_key, team_id })
    }
}

/// Cheap to clone: each rating's work runs on a clone in its own task.
#[derive(Clone)]
pub struct FeedbackLog {
    web: Arc<WebClient>,
    linear: Option<LinearConfig>,
    http: reqwest::Client,
    /// The issue for each Tawkathon, by its party id. Held across the create
    /// so two first ratings at once do not open two issues.
    issues: Arc<Mutex<HashMap<String, String>>>,
}

impl FeedbackLog {
    pub fn new(web: Arc<WebClient>, linear: Option<LinearConfig>) -> Arc<Self> {
        if linear.is_none() {
            info!("LINEAR_API_KEY or LINEAR_TEAM_ID not set; bout feedback is saved in the database; Linear delivery is disabled");
        }
        Arc::new(Self { web, linear, http: reqwest::Client::new(), issues: Arc::new(Mutex::new(HashMap::new())) })
    }

    async fn persist(&self,feedback:&BoutFeedback)->Result<FeedbackReceipt,String>{
        let mut reference=format!("tawkathon:{}:{}",feedback.party_id,feedback.bout.number);
        if feedback.follow_up{reference.push_str(":note");}
        let args=json!({
            "lobby_id":feedback.match_id,"user_id":feedback.user_id,"reference_id":reference,
            "with_note":!feedback.note.is_empty(),"top_up":feedback.follow_up,
            "feedback":{
                "party_id":feedback.party_id,"bout_number":feedback.bout.number,"bout_round":feedback.bout.round,
                "game_key":feedback.bout.key,"display_name":feedback.name,"rating":feedback.rating,
                "note":feedback.note,"place":feedback.place,"player_count":feedback.bout.players.len()
            }
        });
        let value:Value=tokio::time::timeout(std::time::Duration::from_secs(15),self.web.mutate("bout_feedback_save",&args))
            .await.map_err(|_|"Saving feedback timed out".to_string())?.map_err(|error|error.to_string())?;
        if value["saved"]!=true{return Err("The feedback service did not confirm storage".into());}
        info!(feedback_id=?value["id"],party=%feedback.party_id,bout=feedback.bout.number,"bout feedback durably saved");
        Ok(FeedbackReceipt{rewarded:value["rewarded"]==true})
    }

    async fn file(&self, feedback: &BoutFeedback) -> Result<(), String> {
        let Some(linear) = &self.linear else {
            info!(?feedback, "bout feedback");
            return Ok(());
        };
        let issue = {
            let mut issues = self.issues.lock().await;
            match issues.get(&feedback.party_id) {
                Some(id) => id.clone(),
                None => {
                    let id = self.open_issue(linear, feedback).await?;
                    issues.insert(feedback.party_id.clone(), id.clone());
                    id
                }
            }
        };
        self.graphql(
            linear,
            "mutation($input: CommentCreateInput!) { commentCreate(input: $input) { success } }",
            json!({ "input": { "issueId": issue, "body": comment(feedback) } }),
        )
        .await
        .map(|_| ())
    }

    async fn open_issue(&self, linear: &LinearConfig, feedback: &BoutFeedback) -> Result<String, String> {
        let lobby = self
            .web
            .query::<_, Value>("lobby_find", &feedback.match_id)
            .await
            .ok()
            .and_then(|lobby| lobby.get("name")?.as_str().map(str::to_string))
            .unwrap_or_else(|| feedback.match_id.clone());
        let single = feedback.bout.round == 0;
        let body = self
            .graphql(
                linear,
                "mutation($input: IssueCreateInput!) { issueCreate(input: $input) { success issue { id identifier } } }",
                json!({ "input": {
                    "teamId": linear.team_id,
                    "title": format!("{} feedback: {lobby}", if single { "Bout" } else { "Tawkathon" }),
                    "description": format!(
                        "Ratings and notes left after each minigame of {}, in the lobby {lobby} (`{}`). Each comment is one person's rating of one bout.",
                        if single { "bouts played on their own" } else { "one Tawkathon" },
                        feedback.match_id,
                    ),
                } }),
            )
            .await?;
        let issue = &body["data"]["issueCreate"]["issue"];
        info!(identifier = %issue["identifier"], party = %feedback.party_id, "opened a Linear issue for a Tawkathon");
        issue["id"].as_str().map(str::to_string).ok_or_else(|| format!("no issue in {body}"))
    }

    async fn graphql(&self, linear: &LinearConfig, query: &str, variables: Value) -> Result<Value, String> {
        let response = self
            .http
            .post(LINEAR_URL)
            .header("Authorization", &linear.api_key)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let body: Value = response.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() || body.get("errors").is_some() {
            return Err(format!("Linear answered {status}: {body}"));
        }
        Ok(body)
    }
}

impl FeedbackSink for FeedbackLog {
    fn submit(&self,feedback:BoutFeedback)->FeedbackFuture{
        let this=self.clone();
        Box::pin(async move{
            let receipt=this.persist(&feedback).await?;
            // Optional external delivery never delays the durable receipt.
            if this.linear.is_some(){tokio::spawn(async move{if let Err(error)=this.file(&feedback).await{warn!(%error,"filing saved bout feedback to Linear failed");}});}
            Ok(receipt)
        })
    }
}

/// One rating as a Linear comment.
pub fn comment(feedback: &BoutFeedback) -> String {
    let name = crate::bout::find(feedback.bout.key).map(|spec| spec.name).unwrap_or(feedback.bout.key);
    let stars = format!("{}{}", "★".repeat(feedback.rating as usize), "☆".repeat(5 - feedback.rating as usize));
    let who = if feedback.name.is_empty() { "someone" } else { &feedback.name };
    if feedback.follow_up {
        return format!(
            "**{name}**, bout {}: {who} added a note to their {}/5\n\n> {}",
            feedback.bout.number,
            feedback.rating,
            feedback.note.replace('\n', "\n> "),
        );
    }
    let players = feedback.bout.players.len();
    let round = match feedback.bout.round {
        0 => String::new(),
        round => format!(" in round {round}"),
    };
    let mut body = format!(
        "**{name}**, bout {}{round}: {stars} {}/5\n{} finished {} of {players}",
        feedback.bout.number,
        feedback.rating,
        who,
        ordinal(feedback.place),
    );
    if !feedback.note.is_empty() {
        body.push_str("\n\n> ");
        body.push_str(&feedback.note.replace('\n', "\n> "));
    }
    body
}

fn ordinal(place: usize) -> String {
    let suffix = match (place % 10, place % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{place}{suffix}")
}
