//! Prints a board as the client sees it, for the canvas preview harness.
//!
//!     cargo run -p party --example board_json > board.json
//!     cargo run -p party --example board_json volcano > board-volcano.json

fn main() {
    let map = match std::env::args().nth(1) {
        Some(name) => serde_json::from_value(serde_json::Value::String(name)).expect("a map key, such as island or volcano"),
        None => party::board::Map::Island,
    };
    let seats: Vec<u8> = (0..4).collect();
    let mut board = party::board::Board::on(map, &seats, 5, 7);
    board.roll(board.turn_seat()).ok();
    println!("{}", serde_json::to_string(&board.view()).expect("the view is serialisable"));
}
