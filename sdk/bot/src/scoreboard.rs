//! A sport lane for the ticker: one tab per game, kept in step with whatever
//! scoreboard the bot reads. The sport maps its games into [`Matchup`]s and
//! decides which score changes deserve a flash.

use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::warn;

use crate::web_client::WebClient;

#[derive(Debug, Clone, PartialEq)]
pub struct Side {
    pub abbrev: String,
    /// `None` before the game starts.
    pub score: Option<i64>,
    /// A `#rrggbb` team colour for the tab's background.
    pub color: Option<String>,
    /// The team's room, so its fans can see the game first.
    pub channel_id: Option<String>,
    /// An `https` logo drawn beside the team's name.
    pub logo: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Not started; the tag is the start time, such as `7:00 PM` or `Sun 1:00 PM`.
    Upcoming { starts: String },
    /// On now; the clock is shown after the score, such as `2nd 14:32`.
    Live { clock: String },
    /// Over; the tag is `Final`, `Final/OT` and so on.
    Final { label: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Matchup {
    /// Unique within the lane.
    pub id: String,
    pub away: Side,
    pub home: Side,
    pub phase: Phase,
    /// An in-app path, starting with `/`.
    pub url: Option<String>,
    /// Kickoff or puck drop as RFC 3339, so the bar can run in date order.
    pub starts_at: Option<String>,
    /// Where to watch, such as `ESPN`, shown beside the start time.
    pub tv: Option<String>,
    /// Whether everyone who picked the lane's league sees it. A game that is
    /// not, such as an unranked college game, shows only to people who follow
    /// one of its `teams`.
    pub featured: bool,
    /// The teams' ids in the league, for people who follow teams there.
    pub teams: Vec<String>,
}

impl Matchup {
    pub fn new(id: impl Into<String>, away: Side, home: Side, phase: Phase) -> Self {
        Self {
            id: id.into(),
            away,
            home,
            phase,
            url: None,
            starts_at: None,
            tv: None,
            featured: true,
            teams: Vec::new(),
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self.phase, Phase::Live { .. })
    }

    /// `"7 KC - BUF 3"`: scores on the outside, names facing the middle, and
    /// the winner in bold once it is over.
    pub fn title(&self) -> String {
        let started = !matches!(self.phase, Phase::Upcoming { .. });
        let over = matches!(self.phase, Phase::Final { .. });
        let away = started.then_some(self.away.score.unwrap_or(0));
        let home = started.then_some(self.home.score.unwrap_or(0));
        let side = |abbrev: &str, score: Option<i64>, won: bool, is_away: bool| {
            let text = match score {
                Some(score) if is_away => format!("{score} {abbrev}"),
                Some(score) => format!("{abbrev} {score}"),
                None => abbrev.to_string(),
            };
            if won { format!("**{text}**") } else { text }
        };
        format!(
            "{} - {}",
            side(&self.away.abbrev, away, over && away > home, true),
            side(&self.home.abbrev, home, over && home > away, false),
        )
    }

    /// Both teams' logos as `away,home`, or none unless each side has one.
    pub fn logos(&self) -> Option<String> {
        let usable = |logo: &Option<String>| {
            logo.clone()
                .filter(|url| url.starts_with("https://") && !url.contains(','))
        };
        Some(format!(
            "{},{}",
            usable(&self.away.logo)?,
            usable(&self.home.logo)?
        ))
    }

    fn tag_and_detail(&self) -> (Option<String>, Option<String>) {
        match &self.phase {
            Phase::Upcoming { starts } => (Some(starts.clone()), self.tv.clone()),
            Phase::Live { clock } => (None, Some(clock.chars().take(40).collect())),
            Phase::Final { label } => (Some(label.clone()), None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Away,
    Home,
}

/// A live game's score went up since the last sync.
#[derive(Debug, Clone)]
pub struct Scored {
    pub matchup: Matchup,
    pub team: Team,
    pub points: i64,
}

impl Scored {
    pub fn side(&self) -> &Side {
        match self.team {
            Team::Away => &self.matchup.away,
            Team::Home => &self.matchup.home,
        }
    }

    /// `"KC 7 - BUF 3"`.
    pub fn score_line(&self) -> String {
        let m = &self.matchup;
        format!(
            "{} {} - {} {}",
            m.away.abbrev,
            m.away.score.unwrap_or(0),
            m.home.abbrev,
            m.home.score.unwrap_or(0)
        )
    }
}

pub struct ScoreLane {
    prefix: String,
    niche_id: String,
    league: Option<String>,
    lifetime_secs: i32,
    shown: HashMap<String, String>,
    scores: HashMap<String, (i64, i64)>,
}

impl ScoreLane {
    /// `prefix` names the lane's ticker keys, such as `nhl`.
    pub fn new(prefix: impl Into<String>, niche_id: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            niche_id: niche_id.into(),
            league: None,
            lifetime_secs: 12 * 3600,
            shown: HashMap::new(),
            scores: HashMap::new(),
        }
    }

    /// Tags the lane's games with a league (`nhl`, `nfl`, `ncaaf` or `nba`),
    /// so they show to the people who picked it rather than to the hub's members.
    pub fn league(mut self, league: impl Into<String>) -> Self {
        self.league = Some(league.into());
        self
    }

    fn key(&self, id: &str) -> String {
        format!("{}-{id}", self.prefix)
    }

    /// Shows `games` in order, drops tabs for games no longer in it, and
    /// returns every score that went up in a live game since the last call.
    pub async fn sync(&mut self, web: &WebClient, games: &[Matchup]) -> Vec<Scored> {
        for (position, game) in games.iter().enumerate() {
            let title = game.title();
            let (tag, detail) = game.tag_and_detail();
            let label = format!(
                "{title}|{tag:?}|{detail:?}|{position}|{:?}|{}|{:?}",
                game.starts_at, game.featured, game.teams
            );
            if self.shown.get(&game.id) == Some(&label) {
                continue;
            }
            let background = match (&game.away.color, &game.home.color) {
                (Some(away), Some(home)) => Some(format!("{away},{home}")),
                _ => None,
            };
            let image = game.logos();
            let channel_ids: Vec<&String> = [&game.away.channel_id, &game.home.channel_id]
                .into_iter()
                .flatten()
                .collect();
            let item = json!({
                "key": self.key(&game.id), "channel_ids": channel_ids, "niche_id": self.niche_id, "title": title, "label": tag,
                "background": background, "detail": detail, "url": game.url, "live": game.is_live(),
                "position": position as i32, "expires_in": self.lifetime_secs, "starts_at": game.starts_at, "image": image,
                "league": self.league, "featured": game.featured, "teams": game.teams,
            });
            match web.mutate::<_, Value>("ticker_set", &item).await {
                Ok(_) => {
                    self.shown.insert(game.id.clone(), label);
                }
                Err(error) => {
                    warn!(%error, game = %game.id, "setting the game's ticker item failed")
                }
            }
        }
        let gone: Vec<String> = self
            .shown
            .keys()
            .filter(|id| !games.iter().any(|g| &g.id == *id))
            .cloned()
            .collect();
        for id in gone {
            if let Err(error) = web
                .mutate::<_, Value>("ticker_remove", &json!({ "key": self.key(&id) }))
                .await
            {
                warn!(%error, game = %id, "removing an old game's ticker item failed");
            }
            self.shown.remove(&id);
            self.scores.remove(&id);
        }
        self.scored(games)
    }

    fn scored(&mut self, games: &[Matchup]) -> Vec<Scored> {
        let mut scored = Vec::new();
        for game in games {
            let now = (game.away.score.unwrap_or(0), game.home.score.unwrap_or(0));
            if let (true, Some(before)) = (game.is_live(), self.scores.get(&game.id)) {
                for (team, points) in [
                    (Team::Away, now.0 - before.0),
                    (Team::Home, now.1 - before.1),
                ] {
                    if points > 0 {
                        scored.push(Scored {
                            matchup: game.clone(),
                            team,
                            points,
                        });
                    }
                }
            }
            self.scores.insert(game.id.clone(), now);
        }
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(phase: Phase, away: Option<i64>, home: Option<i64>) -> Matchup {
        let side = |abbrev: &str, score| Side {
            abbrev: abbrev.into(),
            score,
            color: None,
            channel_id: None,
            logo: None,
        };
        Matchup::new("1", side("KC", away), side("BUF", home), phase)
    }

    #[test]
    fn titles_put_scores_outside_and_bold_the_winner() {
        assert_eq!(
            game(
                Phase::Upcoming {
                    starts: "1:00 PM".into()
                },
                None,
                None
            )
            .title(),
            "KC - BUF"
        );
        assert_eq!(
            game(Phase::Live { clock: "Q2".into() }, Some(7), Some(3)).title(),
            "7 KC - BUF 3"
        );
        assert_eq!(
            game(
                Phase::Final {
                    label: "Final".into()
                },
                Some(17),
                Some(24)
            )
            .title(),
            "17 KC - **BUF 24**"
        );
        assert_eq!(
            game(
                Phase::Final {
                    label: "Final".into()
                },
                Some(20),
                Some(20)
            )
            .title(),
            "20 KC - BUF 20"
        );
    }

    #[test]
    fn upcoming_games_carry_where_to_watch_beside_the_start() {
        let mut g = game(
            Phase::Upcoming {
                starts: "7:00 PM ET".into(),
            },
            None,
            None,
        );
        assert_eq!(g.tag_and_detail(), (Some("7:00 PM ET".into()), None));
        g.tv = Some("ESPN".into());
        assert_eq!(
            g.tag_and_detail(),
            (Some("7:00 PM ET".into()), Some("ESPN".into()))
        );
        g.phase = Phase::Live {
            clock: "Q3 5:12".into(),
        };
        assert_eq!(g.tag_and_detail(), (None, Some("Q3 5:12".into())));
    }

    #[test]
    fn logos_go_out_as_away_home_only_when_both_sides_have_one() {
        let mut g = game(
            Phase::Upcoming {
                starts: "1:00 PM".into(),
            },
            None,
            None,
        );
        assert_eq!(g.logos(), None);
        g.away.logo = Some("https://x/kc.png".into());
        assert_eq!(g.logos(), None);
        g.home.logo = Some("https://x/buf.png".into());
        assert_eq!(
            g.logos().as_deref(),
            Some("https://x/kc.png,https://x/buf.png")
        );
        g.home.logo = Some("http://x/buf.png".into());
        assert_eq!(g.logos(), None);
    }

    #[test]
    fn only_live_score_rises_count_and_the_first_sight_is_a_baseline() {
        let mut lane = ScoreLane::new("t", "n");
        assert!(
            lane.scored(&[game(Phase::Live { clock: "Q1".into() }, Some(0), Some(7))])
                .is_empty()
        );
        let scored = lane.scored(&[game(Phase::Live { clock: "Q1".into() }, Some(3), Some(7))]);
        assert_eq!(scored.len(), 1);
        assert_eq!(
            (
                scored[0].team,
                scored[0].points,
                scored[0].score_line().as_str()
            ),
            (Team::Away, 3, "KC 3 - BUF 7")
        );
        assert!(
            lane.scored(&[game(
                Phase::Final {
                    label: "Final".into()
                },
                Some(10),
                Some(7)
            )])
            .is_empty()
        );
    }
}
