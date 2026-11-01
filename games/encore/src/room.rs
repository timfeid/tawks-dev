//! One lobby's band: who plays what, the song, and the shared start time.
//! Every player judges their own notes against their own audio, so the room
//! only agrees on when the song starts and passes scores around.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::chart::{Difficulty, Instrument};
use crate::library::{SongInfo, Source};

pub const LOADING_TIMEOUT_MS: f64 = 45_000.0;
pub const COUNT_IN_MS: f64 = 3_000.0;
pub const OUTRO_MS: f64 = 8_000.0;
/// How long a player who dropped out of a song keeps their seat.
pub const AWAY_GRACE_MS: f64 = 30_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartChoice {
    pub instrument: Instrument,
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub score: u64,
    pub streak: u32,
    pub best_streak: u32,
    pub multiplier: u32,
    pub hit: u32,
    pub missed: u32,
    pub total: u32,
    pub star_meter: f32,
    pub star_active: bool,
    /// The player's stem is silent because they are missing notes.
    pub muted: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Player {
    pub conn: u64,
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
    pub part: Option<PartChoice>,
    pub ready: bool,
    pub loaded: bool,
    /// In the song being played, as opposed to watching it.
    pub playing: bool,
    pub done: bool,
    pub progress: Progress,
    /// Disconnected mid-song; the seat is kept for a while in case they come back.
    pub away: bool,
    #[serde(skip)]
    pub away_since: f64,
    #[serde(skip)]
    pub joined: u64,
    /// May reach the server's own song folder.
    #[serde(skip)]
    pub library: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Lobby,
    Loading,
    Playing,
    Results,
}

#[derive(Serialize)]
pub struct RoomView<'a> {
    pub phase: Phase,
    pub song: Option<&'a SongInfo>,
    pub picked_by: Option<&'a str>,
    pub host: Option<u64>,
    pub start_at: Option<f64>,
    pub ends_at: Option<f64>,
    pub players: Vec<&'a Player>,
}

/// A finished part, to keep as the player's best.
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
    pub user_id: String,
    pub song_id: String,
    pub part: PartChoice,
    pub score: u64,
    pub progress: Progress,
    /// How many played the song, this player included.
    pub band_size: usize,
    /// Only official songs earn rewards.
    pub official: bool,
}

/// Who is in a room and what song it has, for the HTTP routes to check
/// uploads and library songs against.
#[derive(Clone, Debug, Default)]
pub struct RoomPresence {
    pub users: HashSet<String>,
    pub song: Option<String>,
}

/// Play rewards, rankings and score credit come only from official
/// (licensed or original) songs; the library and player uploads earn nothing.
pub fn earns_rewards(song: &SongInfo) -> bool {
    song.official
}

pub struct Room {
    pub phase: Phase,
    pub song: Option<SongInfo>,
    pub picked_by: Option<String>,
    pub players: BTreeMap<u64, Player>,
    pub length_ms: f64,
    pub loading_since: f64,
    pub start_at: Option<f64>,
    pub ends_at: Option<f64>,
    pub dirty: bool,
    joins: u64,
}

impl Default for Room {
    fn default() -> Self {
        Room {
            phase: Phase::Lobby,
            song: None,
            picked_by: None,
            players: BTreeMap::new(),
            length_ms: 0.0,
            loading_since: 0.0,
            start_at: None,
            ends_at: None,
            dirty: true,
            joins: 0,
        }
    }
}

impl Room {
    pub fn view(&self) -> RoomView<'_> {
        self.view_for(true)
    }

    /// The room as one player sees it: a library song stays hidden from
    /// anyone without library access.
    pub fn view_for(&self, library: bool) -> RoomView<'_> {
        let mut players: Vec<&Player> = self.players.values().collect();
        players.sort_by_key(|p| p.joined);
        let song = self
            .song
            .as_ref()
            .filter(|song| library || song.source != Source::Library);
        RoomView {
            phase: self.phase,
            song,
            picked_by: self.picked_by.as_deref(),
            host: players.iter().find(|p| !p.away).map(|p| p.conn),
            start_at: self.start_at,
            ends_at: self.ends_at,
            players,
        }
    }

    pub fn presence(&self) -> RoomPresence {
        RoomPresence {
            users: self
                .players
                .values()
                .filter(|p| !p.away)
                .map(|p| p.user_id.clone())
                .collect(),
            song: self.song.as_ref().map(|s| s.id.clone()),
        }
    }

    fn present(&self, user_id: &str) -> bool {
        self.players.values().any(|p| !p.away && p.user_id == user_id)
    }

    /// Why this song cannot be played in the room as it is now, if it can't.
    fn unplayable(&self, song: &SongInfo) -> Option<&'static str> {
        match song.source {
            Source::Library if self.players.values().any(|p| !p.library) => {
                Some("Everyone in the room needs library access for that song")
            }
            Source::Upload if !song.official => match &song.uploaded_by {
                Some(uploader) if self.present(uploader) => None,
                _ => Some("That song can only be played while its uploader is in the room"),
            },
            _ => None,
        }
    }

    fn clear_song(&mut self) {
        self.song = None;
        self.picked_by = None;
        for player in self.players.values_mut() {
            player.ready = false;
        }
        self.dirty = true;
    }

    /// Drops a song that was deleted or taken down, unless it is mid-play.
    pub fn forget(&mut self, song_id: &str) {
        let current = self.song.as_ref().is_some_and(|s| s.id == song_id);
        if current && matches!(self.phase, Phase::Lobby | Phase::Results) {
            self.phase = Phase::Lobby;
            self.reset_round();
            self.clear_song();
        }
    }

    pub fn join(&mut self, conn: u64, user_id: &str, name: &str, avatar_url: &str, library: bool) {
        let returning = self.players.iter().find(|(_, p)| p.away && p.user_id == user_id).map(|(c, _)| *c);
        if let Some(old) = returning.and_then(|old| self.players.remove(&old)) {
            self.players.insert(conn, Player { conn, away: false, library, ..old });
            self.dirty = true;
            return;
        }
        self.joins += 1;
        self.players.insert(
            conn,
            Player {
                conn,
                user_id: user_id.to_string(),
                name: name.to_string(),
                avatar_url: avatar_url.to_string(),
                part: None,
                ready: false,
                loaded: false,
                playing: false,
                done: false,
                progress: Progress::default(),
                away: false,
                away_since: 0.0,
                joined: self.joins,
                library,
            },
        );
        self.dirty = true;
    }

    pub fn leave(&mut self, conn: u64, now: f64) {
        let mid_song = matches!(self.phase, Phase::Loading | Phase::Playing);
        match self.players.get_mut(&conn) {
            Some(player) if mid_song && player.playing => {
                player.away = true;
                player.away_since = now;
            }
            _ => {
                self.players.remove(&conn);
            }
        }
        self.dirty = true;
    }

    pub fn pick(&mut self, conn: u64, song: SongInfo, length_ms: f64) -> Result<(), String> {
        if !matches!(self.phase, Phase::Lobby | Phase::Results) {
            return Err("Wait for this song to finish".into());
        }
        let picker = self
            .players
            .get(&conn)
            .ok_or("You are not in this room")?
            .name
            .clone();
        if let Some(reason) = self.unplayable(&song) {
            return Err(reason.into());
        }
        let available: Vec<Instrument> = song.parts.iter().map(|p| p.instrument).collect();
        self.reset_round();
        for player in self.players.values_mut() {
            if player
                .part
                .is_some_and(|part| !available.contains(&part.instrument))
            {
                player.part = None;
            }
        }
        self.song = Some(song);
        self.length_ms = length_ms;
        self.picked_by = Some(picker);
        self.phase = Phase::Lobby;
        self.dirty = true;
        Ok(())
    }

    pub fn set_part(&mut self, conn: u64, part: Option<PartChoice>) -> Result<(), String> {
        if self.phase != Phase::Lobby && self.phase != Phase::Results {
            return Err("Parts are picked between songs".into());
        }
        if let (Some(part), Some(song)) = (part, &self.song) {
            if !song.parts.iter().any(|p| p.instrument == part.instrument) {
                return Err("This song has no part for that".into());
            }
        }
        let player = self
            .players
            .get_mut(&conn)
            .ok_or("You are not in this room")?;
        player.part = part;
        if part.is_none() {
            player.ready = false;
        }
        self.dirty = true;
        Ok(())
    }

    pub fn set_ready(&mut self, conn: u64, ready: bool) -> Result<(), String> {
        if self.phase != Phase::Lobby {
            return Err("Not now".into());
        }
        let player = self
            .players
            .get_mut(&conn)
            .ok_or("You are not in this room")?;
        if ready && player.part.is_none() {
            return Err("Pick a part first".into());
        }
        player.ready = ready;
        self.dirty = true;
        Ok(())
    }

    pub fn loaded(&mut self, conn: u64) {
        if let Some(player) = self.players.get_mut(&conn) {
            player.loaded = true;
            self.dirty = true;
        }
    }

    pub fn progress(&mut self, conn: u64, progress: Progress, done: bool) {
        if self.phase != Phase::Playing {
            return;
        }
        if let Some(player) = self.players.get_mut(&conn).filter(|p| p.playing && !p.done) {
            player.progress = progress;
            player.done = done;
            self.dirty = true;
        }
    }

    /// The host ends the song early; nothing from it counts.
    pub fn stop(&mut self, conn: u64) -> Result<(), String> {
        if self.view().host != Some(conn) {
            return Err("Only the host can end the song".into());
        }
        if matches!(self.phase, Phase::Loading | Phase::Playing) {
            self.reset_round();
            self.phase = Phase::Lobby;
            self.dirty = true;
        }
        Ok(())
    }

    /// Back to picking a song once everyone has seen the results.
    pub fn again(&mut self) {
        if self.phase == Phase::Results {
            self.reset_round();
            self.phase = Phase::Lobby;
            self.dirty = true;
        }
    }

    fn reset_round(&mut self) {
        self.start_at = None;
        self.ends_at = None;
        for player in self.players.values_mut() {
            player.ready = false;
            player.loaded = false;
            player.playing = false;
            player.done = false;
            player.progress = Progress::default();
        }
    }

    /// Moves the round along; returns the parts that finished when a song ends.
    pub fn tick(&mut self, now: f64) -> Vec<Finished> {
        let mid_song = matches!(self.phase, Phase::Loading | Phase::Playing);
        let before = self.players.len();
        self.players.retain(|_, p| !p.away || (mid_song && now - p.away_since < AWAY_GRACE_MS));
        if self.players.len() != before {
            self.dirty = true;
        }
        match self.phase {
            Phase::Lobby => {
                if self.song.as_ref().is_some_and(|song| self.unplayable(song).is_some()) {
                    self.clear_song();
                }
                // People without a part are watching; everyone with one has to be ready.
                let ready = self.players.values().any(|p| p.ready)
                    && self.players.values().all(|p| p.ready || p.part.is_none());
                if self.song.is_some() && ready {
                    self.phase = Phase::Loading;
                    self.loading_since = now;
                    for player in self.players.values_mut() {
                        player.playing = player.ready;
                        player.loaded = false;
                    }
                    self.dirty = true;
                }
            }
            Phase::Loading => {
                let band: Vec<&Player> = self.players.values().filter(|p| p.playing).collect();
                if band.is_empty() {
                    self.phase = Phase::Lobby;
                    self.dirty = true;
                } else if band.iter().all(|p| p.loaded)
                    || now - self.loading_since > LOADING_TIMEOUT_MS
                {
                    let start = now + COUNT_IN_MS;
                    self.start_at = Some(start);
                    self.ends_at = Some(start + self.length_ms + OUTRO_MS);
                    self.phase = Phase::Playing;
                    self.dirty = true;
                }
            }
            Phase::Playing => {
                let band: Vec<&Player> = self.players.values().filter(|p| p.playing).collect();
                let over = self.ends_at.is_some_and(|end| now > end);
                if band.is_empty() || over || band.iter().all(|p| p.done) {
                    self.phase = Phase::Results;
                    self.dirty = true;
                    let Some(song) = &self.song else {
                        return Vec::new();
                    };
                    let band_size = band.len();
                    let official = earns_rewards(song);
                    return band
                        .iter()
                        .filter_map(|p| {
                            Some(Finished {
                                user_id: p.user_id.clone(),
                                song_id: song.id.clone(),
                                part: p.part?,
                                score: p.progress.score,
                                progress: p.progress.clone(),
                                band_size,
                                official,
                            })
                        })
                        .collect();
                }
            }
            Phase::Results => {}
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Part;

    fn song() -> SongInfo {
        SongInfo {
            id: "s1".into(),
            name: "Tune".into(),
            artist: "Band".into(),
            album: None,
            year: None,
            genre: None,
            charter: None,
            length_ms: 60_000,
            preview_ms: 0,
            parts: vec![Part {
                instrument: Instrument::Guitar,
                intensity: Some(3),
            }],
            stems: vec!["song".into()],
            art: false,
            source: Source::Library,
            official: false,
            uploaded_by: None,
            uploader_name: None,
        }
    }

    fn upload(by: &str, official: bool) -> SongInfo {
        SongInfo {
            id: "ENS-1".into(),
            source: Source::Upload,
            official,
            uploaded_by: Some(by.into()),
            ..song()
        }
    }

    fn play_through(room: &mut Room, conn: u64) -> Vec<Finished> {
        room.set_part(conn, Some(GUITAR)).unwrap();
        room.set_ready(conn, true).unwrap();
        room.tick(0.0);
        room.loaded(conn);
        room.tick(1.0);
        room.progress(conn, Progress { score: 100, ..Default::default() }, true);
        room.tick(2.0)
    }

    #[test]
    fn library_songs_need_everyone_to_have_access() {
        let mut room = Room::default();
        room.join(1, "owner", "Owner", "", true);
        room.join(2, "friend", "Friend", "", false);
        assert!(room.pick(1, song(), 60_000.0).is_err());
        room.leave(2, 0.0);
        room.pick(1, song(), 60_000.0).unwrap();
        assert!(room.view_for(true).song.is_some());

        room.join(3, "stranger", "Stranger", "", false);
        assert!(room.view_for(false).song.is_none());
        assert_eq!(room.presence().song.as_deref(), Some("s1"));
        room.tick(0.0);
        assert!(room.song.is_none());
        assert!(room.presence().song.is_none());
    }

    #[test]
    fn an_upload_plays_only_while_its_uploader_is_in_the_room() {
        let mut room = Room::default();
        room.join(1, "ann", "Ann", "", false);
        room.join(2, "bo", "Bo", "", false);
        assert!(room.pick(1, upload("cy", false), 60_000.0).is_err());
        room.pick(1, upload("bo", false), 60_000.0).unwrap();
        assert!(room.view_for(false).song.is_some());
        let users = room.presence().users;
        assert!(users.contains("ann") && users.contains("bo"));

        room.leave(2, 0.0);
        room.tick(0.0);
        assert!(room.song.is_none());
        assert!(!room.presence().users.contains("bo"));

        room.pick(1, upload("cy", true), 60_000.0).unwrap();
        assert!(room.song.is_some());
    }

    #[test]
    fn only_official_songs_earn_rewards() {
        for (info, official) in [(song(), false), (upload("ann", false), false), (upload("ann", true), true)] {
            assert_eq!(earns_rewards(&info), official);
            let mut room = Room::default();
            room.join(1, "ann", "Ann", "", true);
            room.pick(1, info, 60_000.0).unwrap();
            let finished = play_through(&mut room, 1);
            assert_eq!(finished.len(), 1);
            assert_eq!(finished[0].official, official);
        }
    }

    #[test]
    fn a_deleted_song_leaves_the_room() {
        let mut room = Room::default();
        room.join(1, "ann", "Ann", "", false);
        room.pick(1, upload("ann", false), 60_000.0).unwrap();
        room.forget("other");
        assert!(room.song.is_some());
        room.forget("ENS-1");
        assert!(room.song.is_none());
    }

    const GUITAR: PartChoice = PartChoice {
        instrument: Instrument::Guitar,
        difficulty: Difficulty::Expert,
    };

    #[test]
    fn a_round_runs_from_lobby_to_results() {
        let mut room = Room::default();
        room.join(1, "u1", "Ann", "", true);
        room.join(2, "u2", "Bo", "", true);
        room.pick(2, song(), 60_000.0).unwrap();
        assert_eq!(room.view().host, Some(1));
        assert!(room.set_ready(1, true).is_err());
        assert!(
            room.set_part(
                1,
                Some(PartChoice {
                    instrument: Instrument::Drums,
                    ..GUITAR
                })
            )
            .is_err()
        );
        room.set_part(1, Some(GUITAR)).unwrap();
        room.set_part(2, Some(GUITAR)).unwrap();
        room.set_ready(1, true).unwrap();
        room.tick(0.0);
        assert_eq!(room.phase, Phase::Lobby);
        room.set_ready(2, true).unwrap();
        room.tick(0.0);
        assert_eq!(room.phase, Phase::Loading);

        room.loaded(1);
        room.tick(1_000.0);
        assert_eq!(room.phase, Phase::Loading);
        room.loaded(2);
        room.tick(2_000.0);
        assert_eq!(room.phase, Phase::Playing);
        assert_eq!(room.start_at, Some(2_000.0 + COUNT_IN_MS));

        room.join(3, "u3", "Cy", "", true);
        let score = Progress {
            score: 900,
            ..Default::default()
        };
        room.progress(1, score.clone(), true);
        room.progress(3, score.clone(), true);
        assert!(room.tick(3_000.0).is_empty());
        room.progress(
            2,
            Progress {
                score: 400,
                ..Default::default()
            },
            true,
        );
        let finished = room.tick(4_000.0);
        assert_eq!(room.phase, Phase::Results);
        assert_eq!(finished.len(), 2);
        assert_eq!(finished[0].score, 900);

        room.again();
        assert_eq!(room.phase, Phase::Lobby);
        assert!(
            room.players
                .values()
                .all(|p| !p.ready && p.progress.score == 0)
        );
    }

    #[test]
    fn a_dropped_player_keeps_their_seat_for_a_while() {
        let mut room = Room::default();
        room.join(1, "u1", "Ann", "", true);
        room.pick(1, song(), 60_000.0).unwrap();
        room.set_part(1, Some(GUITAR)).unwrap();
        room.set_ready(1, true).unwrap();
        room.tick(0.0);
        room.loaded(1);
        room.tick(1.0);
        room.progress(1, Progress { score: 700, ..Default::default() }, false);
        room.leave(1, 2_000.0);
        room.tick(3_000.0);
        assert_eq!(room.phase, Phase::Playing);
        assert!(room.players[&1].away);

        room.join(7, "u1", "Ann", "", true);
        assert!(!room.players.contains_key(&1));
        let back = &room.players[&7];
        assert!(back.playing && !back.away && back.progress.score == 700);

        room.leave(7, 4_000.0);
        room.tick(4_000.0 + AWAY_GRACE_MS + 1.0);
        assert!(room.players.is_empty());
    }

    #[test]
    fn only_the_host_can_end_a_song() {
        let mut room = Room::default();
        room.join(1, "u1", "Ann", "", true);
        room.join(2, "u2", "Bo", "", true);
        room.pick(1, song(), 60_000.0).unwrap();
        room.set_part(1, Some(GUITAR)).unwrap();
        room.set_ready(1, true).unwrap();
        room.tick(0.0);
        room.loaded(1);
        room.tick(1.0);
        assert_eq!(room.phase, Phase::Playing);
        assert!(room.stop(2).is_err());
        room.stop(1).unwrap();
        assert_eq!(room.phase, Phase::Lobby);
        assert!(room.start_at.is_none() && room.players.values().all(|p| !p.playing && !p.ready));
        assert!(room.tick(2.0).is_empty());
    }

    #[test]
    fn a_stuck_load_starts_anyway_and_a_song_ends_on_time() {
        let mut room = Room::default();
        room.join(1, "u1", "Ann", "", true);
        room.join(2, "u2", "Watcher", "", true);
        room.pick(1, song(), 10_000.0).unwrap();
        room.set_part(1, Some(GUITAR)).unwrap();
        room.set_ready(1, true).unwrap();
        room.tick(0.0);
        assert!(!room.players[&2].playing);
        room.tick(LOADING_TIMEOUT_MS + 1.0);
        assert_eq!(room.phase, Phase::Playing);
        let end = room.ends_at.unwrap();
        room.tick(end + 1.0);
        assert_eq!(room.phase, Phase::Results);
    }
}
