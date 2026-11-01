//! Reads every chart in a song folder and prints what each one holds:
//! `cargo run -p encore --example scan -- /path/to/songs`

use encore::chart::{Difficulty, NoteKind};
use encore::library::Library;

fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).expect("pass the song folder");
    let library = Library::scan(std::path::Path::new(&root))?;
    let songs: Vec<serde_json::Value> = serde_json::from_str(library.catalog_json())?;
    let mut failed = 0;
    for song in &songs {
        let id = song["id"].as_str().unwrap_or_default();
        let title = format!(
            "{} - {}",
            song["artist"].as_str().unwrap_or("?"),
            song["name"].as_str().unwrap_or("?")
        );
        match library.chart(id) {
            Ok(chart) => {
                let parts: Vec<String> = chart
                    .tracks
                    .iter()
                    .filter(|t| t.difficulty == Difficulty::Expert)
                    .map(|t| {
                        let hopos = t.notes.iter().filter(|n| n.kind != NoteKind::Strum).count();
                        format!(
                            "{}:{} notes, {} hopo/tap, {} sp, first {:.0}ms",
                            t.instrument.key(),
                            t.notes.len(),
                            hopos,
                            t.star_power.len(),
                            t.notes.first().map(|n| n.t).unwrap_or(0.0)
                        )
                    })
                    .collect();
                println!(
                    "{title}\n  length {:.0}s (ini {}s), {} sections, {} lyric lines, stems {}\n  {}",
                    chart.length / 1000.0,
                    song["length_ms"].as_u64().unwrap_or(0) / 1000,
                    chart.sections.len(),
                    chart.lyrics.len(),
                    song["stems"],
                    parts.join("\n  ")
                );
            }
            Err(error) => {
                failed += 1;
                println!("{title}\n  FAILED: {error}");
            }
        }
    }
    println!("{} songs, {failed} failed", songs.len());
    Ok(())
}
