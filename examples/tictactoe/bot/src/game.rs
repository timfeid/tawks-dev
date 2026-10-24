use std::collections::HashMap;

use anyhow::{Result, bail};
use game_framework::{GameEffect, GameLogic};
use serde::{Deserialize, Serialize};

const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];
const MARKS: [&str; 2] = ["X", "O"];

#[derive(Deserialize)]
pub enum Action {
    View { name: String },
    Input(Input),
}

/// What the client sends with `activity.send`.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    Move { cell: usize },
    Reset,
}

#[derive(Clone)]
struct Player {
    id: String,
    name: String,
}

pub struct TicTacToe {
    players: Vec<Player>,
    board: [Option<usize>; 9],
    turn: usize,
    viewers: HashMap<String, String>,
}

#[derive(Serialize, Clone)]
pub struct View {
    /// 0 for X, 1 for O, absent for spectators.
    seat: Option<usize>,
    players: Vec<PlayerView>,
    board: Vec<Option<&'static str>>,
    turn: usize,
    winner: Option<usize>,
    line: Option<[usize; 3]>,
    draw: bool,
}

#[derive(Serialize, Clone)]
struct PlayerView {
    name: String,
    mark: &'static str,
}

impl TicTacToe {
    fn winning_line(&self) -> Option<(usize, [usize; 3])> {
        LINES.iter().find_map(|line| match line.map(|i| self.board[i]) {
            [Some(a), Some(b), Some(c)] if a == b && b == c => Some((a, *line)),
            _ => None,
        })
    }

    fn over(&self) -> bool {
        self.winning_line().is_some() || self.board.iter().all(Option::is_some)
    }

    fn seat_of(&self, player_id: &str) -> Option<usize> {
        self.players.iter().position(|p| p.id == player_id)
    }

    fn play(&mut self, player_id: &str, cell: usize) -> Result<()> {
        let Some(seat) = self.seat_of(player_id) else { bail!("You are watching this game.") };
        if self.players.len() < 2 {
            bail!("Waiting for a second player.");
        }
        if self.over() {
            bail!("The game is over.");
        }
        if seat != self.turn {
            bail!("It's not your turn.");
        }
        if cell >= 9 || self.board[cell].is_some() {
            bail!("That square is taken.");
        }
        self.board[cell] = Some(seat);
        self.turn = 1 - self.turn;
        Ok(())
    }
}

impl GameLogic for TicTacToe {
    type Action = Action;
    type View = View;
    type Effect = GameEffect;
    type InitArgs = ();

    fn new(_: ()) -> Self {
        Self { players: Vec::new(), board: [None; 9], turn: 0, viewers: HashMap::new() }
    }

    fn record_instance_id(&mut self, player_id: &str, instance_id: &str) {
        if player_id != "system" {
            self.viewers.insert(player_id.to_string(), instance_id.to_string());
        }
    }

    fn get_instance_id_for_player_id(&self, player_id: &str) -> Option<String> {
        self.viewers.get(player_id).cloned()
    }

    fn get_player_ids(&self) -> Vec<String> {
        self.viewers.keys().cloned().collect()
    }

    fn update(&mut self, player_id: &str, action: Action) -> Result<Vec<GameEffect>> {
        match action {
            Action::View { name } => {
                if self.players.len() < 2 && self.seat_of(player_id).is_none() {
                    self.players.push(Player { id: player_id.to_string(), name });
                }
            }
            Action::Input(Input::Move { cell }) => self.play(player_id, cell)?,
            Action::Input(Input::Reset) => {
                if self.seat_of(player_id).is_none() {
                    bail!("Only players can start a new game.");
                }
                self.board = [None; 9];
                self.turn = 0;
            }
        }
        Ok(Vec::new())
    }

    fn view_for(&self, player_id: &str) -> View {
        let winning = self.winning_line();
        View {
            seat: self.seat_of(player_id),
            players: self
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| PlayerView { name: p.name.clone(), mark: MARKS[i] })
                .collect(),
            board: self.board.iter().map(|cell| cell.map(|seat| MARKS[seat])).collect(),
            turn: self.turn,
            winner: winning.map(|(seat, _)| seat),
            line: winning.map(|(_, line)| line),
            draw: winning.is_none() && self.board.iter().all(Option::is_some),
        }
    }
}
