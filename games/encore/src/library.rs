//! The song folders: one per song, each with a `song.ini`, a chart and the
//! audio split into stems.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::chart::{Chart, Instrument};
use crate::formats::{IniHints, read_chart_text, read_midi};

pub(crate) const AUDIO_EXTENSIONS: [&str; 5] = ["ogg", "opus", "mp3", "wav", "flac"];
pub(crate) const STEMS: [&str; 13] = [
    "song", "guitar", "bass", "rhythm", "keys", "drums", "drums_1", "drums_2", "drums_3",
    "drums_4", "vocals", "vocals_1", "crowd",
];
pub(crate) const ART: [&str; 3] = ["album.png", "album.jpg", "album.jpeg"];
const CACHED_CHARTS: usize = 48;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Part {
    pub instrument: Instrument,
    /// The charter's 0 to 6 rating, when the ini gives one.
    pub intensity: Option<i32>,
}

/// Where a song comes from: the server's own folder, or a player's upload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Library,
    Upload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SongInfo {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub charter: Option<String>,
    pub length_ms: u64,
    pub preview_ms: u64,
    pub parts: Vec<Part>,
    /// Stem names the client can fetch, such as `guitar` or `drums_1`.
    pub stems: Vec<String>,
    pub art: bool,
    #[serde(default)]
    pub source: Source,
    /// Licensed or original; only these songs earn rewards.
    #[serde(default)]
    pub official: bool,
    #[serde(default)]
    pub uploaded_by: Option<String>,
    #[serde(default)]
    pub uploader_name: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum ChartFile {
    Midi(PathBuf),
    Text(PathBuf),
}

pub(crate) struct Entry {
    pub(crate) info: SongInfo,
    dir: PathBuf,
    pub(crate) files: HashMap<String, PathBuf>,
    chart: ChartFile,
    pub(crate) hints: IniHints,
}

impl Entry {
    pub(crate) fn chart_path(&self) -> &Path {
        match &self.chart {
            ChartFile::Midi(path) | ChartFile::Text(path) => path,
        }
    }
}

/// A chart from its file name (`notes.mid` or `notes.chart`) and bytes.
pub fn read_chart_file(name: &str, bytes: &[u8], hints: IniHints) -> Result<Chart> {
    if name.to_ascii_lowercase().ends_with(".mid") {
        read_midi(bytes, hints)
    } else {
        read_chart_text(&String::from_utf8_lossy(bytes), hints)
    }
}

pub struct Library {
    entries: BTreeMap<String, Entry>,
    catalog: String,
    charts: Mutex<Vec<(String, Arc<String>)>>,
}

impl Library {
    pub fn scan(root: &Path) -> Result<Library> {
        if !root.is_dir() {
            bail!("{} is not a folder", root.display());
        }
        let mut entries = BTreeMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut ini = None;
            let mut names = Vec::new();
            for item in read.flatten() {
                let path = item.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.eq_ignore_ascii_case("song.ini") {
                        ini = Some(path.clone());
                    }
                    names.push((name.to_string(), path));
                }
            }
            let Some(ini) = ini else { continue };
            let relative = dir
                .strip_prefix(root)
                .unwrap_or(&dir)
                .to_string_lossy()
                .to_string();
            match read_entry(&relative, &dir, &ini, names) {
                Ok(entry) => {
                    entries.insert(entry.info.id.clone(), entry);
                }
                Err(error) => warn!(song = %relative, %error, "skipping a song"),
            }
        }
        let mut infos: Vec<&SongInfo> = entries.values().map(|e| &e.info).collect();
        infos.sort_by(|a, b| {
            (a.artist.to_lowercase(), a.name.to_lowercase())
                .cmp(&(b.artist.to_lowercase(), b.name.to_lowercase()))
        });
        let catalog = serde_json::to_string(&infos)?;
        info!(songs = entries.len(), root = %root.display(), "song library scanned");
        Ok(Library {
            entries,
            catalog,
            charts: Mutex::new(Vec::new()),
        })
    }

    pub fn empty() -> Library {
        Library {
            entries: BTreeMap::new(),
            catalog: "[]".into(),
            charts: Mutex::new(Vec::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every song, sorted by artist then title, as JSON.
    pub fn catalog_json(&self) -> &str {
        &self.catalog
    }

    pub fn song(&self, id: &str) -> Option<&SongInfo> {
        self.entries.get(id).map(|e| &e.info)
    }

    /// A stem or the album art by its published name, such as `guitar` or `album`.
    pub fn file(&self, id: &str, name: &str) -> Option<PathBuf> {
        let entry = self.entries.get(id)?;
        if name == "album" {
            return ART
                .iter()
                .map(|art| entry.dir.join(art))
                .find(|p| p.is_file());
        }
        entry.files.get(name).cloned()
    }

    pub fn chart(&self, id: &str) -> Result<Chart> {
        let entry = self.entries.get(id).context("no such song")?;
        let path = entry.chart_path();
        read_chart_file(&path.to_string_lossy(), &std::fs::read(path)?, entry.hints)
    }

    /// The chart as JSON, kept for the songs played most recently.
    pub fn chart_json(&self, id: &str) -> Result<Arc<String>> {
        if let Some((_, json)) = self
            .charts
            .lock()
            .unwrap()
            .iter()
            .find(|(key, _)| key == id)
        {
            return Ok(json.clone());
        }
        let json = Arc::new(serde_json::to_string(&self.chart(id)?)?);
        let mut charts = self.charts.lock().unwrap();
        charts.retain(|(key, _)| key != id);
        charts.push((id.to_string(), json.clone()));
        if charts.len() > CACHED_CHARTS {
            charts.remove(0);
        }
        Ok(json)
    }
}

/// FNV-1a, so a song keeps its id across restarts and builds.
fn song_id(relative: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in relative.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn parse_ini(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.starts_with('[') || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            out.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }
    out
}

/// Strips Clone Hero color tags such as `<color=#ff0000>` from display text.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

pub(crate) fn read_entry(
    relative: &str,
    dir: &Path,
    ini_path: &Path,
    names: Vec<(String, PathBuf)>,
) -> Result<Entry> {
    let ini = parse_ini(&String::from_utf8_lossy(&std::fs::read(ini_path)?));
    let lookup: HashMap<String, PathBuf> = names
        .into_iter()
        .map(|(n, p)| (n.to_lowercase(), p))
        .collect();
    let chart = if let Some(path) = lookup.get("notes.mid") {
        ChartFile::Midi(path.clone())
    } else if let Some(path) = lookup.get("notes.chart") {
        ChartFile::Text(path.clone())
    } else {
        bail!("no notes.mid or notes.chart");
    };

    let mut files = HashMap::new();
    for stem in STEMS {
        for extension in AUDIO_EXTENSIONS {
            if let Some(path) = lookup.get(&format!("{stem}.{extension}")) {
                files.insert(stem.to_string(), path.clone());
                break;
            }
        }
    }
    if files.is_empty() {
        bail!("no audio");
    }
    let mut stems: Vec<String> = files.keys().cloned().collect();
    stems.sort();

    let text = |key: &str| ini.get(key).map(|v| plain(v)).filter(|v| !v.is_empty());
    let number = |key: &str| ini.get(key).and_then(|v| v.trim().parse::<i64>().ok());
    let mut parts = Vec::new();
    for (instrument, key) in [
        (Instrument::Guitar, "diff_guitar"),
        (Instrument::Bass, "diff_bass"),
        (Instrument::Rhythm, "diff_rhythm"),
        (Instrument::Keys, "diff_keys"),
        (Instrument::Drums, "diff_drums"),
        (Instrument::Vocals, "diff_vocals"),
    ] {
        if let Some(value) = number(key).filter(|v| *v >= 0) {
            parts.push(Part {
                instrument,
                intensity: Some(value as i32),
            });
        }
    }
    let hints = IniHints {
        delay_ms: number("delay").unwrap_or(0) as f64,
        hopo_frequency: number("hopo_frequency")
            .filter(|v| *v > 0)
            .map(|v| v as u64),
        eighth_note_hopo: ini
            .get("eighthnote_hopo")
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        sustain_cutoff: number("sustain_cutoff_threshold")
            .filter(|v| *v > 0)
            .map(|v| v as u64),
    };
    let mut entry = Entry {
        info: SongInfo {
            id: song_id(relative),
            name: text("name")
                .unwrap_or_else(|| relative.rsplit('/').next().unwrap_or(relative).to_string()),
            artist: text("artist").unwrap_or_else(|| "Unknown artist".into()),
            album: text("album"),
            year: text("year"),
            genre: text("genre"),
            charter: text("charter").or_else(|| text("frets")),
            length_ms: number("song_length").unwrap_or(0).max(0) as u64,
            preview_ms: number("preview_start_time").unwrap_or(0).max(0) as u64,
            parts,
            stems,
            art: ART.iter().any(|art| lookup.contains_key(*art)),
            source: Source::Library,
            official: false,
            uploaded_by: None,
            uploader_name: None,
        },
        dir: dir.to_path_buf(),
        files,
        chart,
        hints,
    };
    // Some rips mark every part -1 even with a full chart, so the chart has the last word.
    if entry.info.parts.is_empty() {
        let chart = match &entry.chart {
            ChartFile::Midi(path) => read_midi(&std::fs::read(path)?, hints)?,
            ChartFile::Text(path) => {
                read_chart_text(&String::from_utf8_lossy(&std::fs::read(path)?), hints)?
            }
        };
        let mut found: Vec<Instrument> = chart.tracks.iter().map(|t| t.instrument).collect();
        found.dedup();
        if !chart.vocals.is_empty() {
            found.push(Instrument::Vocals);
        }
        entry.info.parts = found
            .into_iter()
            .map(|instrument| Part {
                instrument,
                intensity: None,
            })
            .collect();
    }
    // `.chart` files carry lyrics but never the notes to sing.
    if matches!(entry.chart, ChartFile::Text(_)) {
        entry
            .info
            .parts
            .retain(|p| p.instrument != Instrument::Vocals);
    }
    if entry.info.parts.is_empty() {
        bail!("no playable parts");
    }
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_song_folders() {
        let root = std::env::temp_dir().join(format!("encore-library-{}", std::process::id()));
        let song = root.join("Pack").join("Band - Tune (Charter)");
        std::fs::create_dir_all(&song).unwrap();
        std::fs::write(
            song.join("song.ini"),
            "[song]\nname = <color=#f00>Tune</color>\nartist = Band\nsong_length = 61000\ndiff_guitar = 3\ndiff_drums = -1\n",
        )
        .unwrap();
        std::fs::write(
            song.join("notes.chart"),
            "[Song]\n{\n Resolution = 192\n}\n[ExpertSingle]\n{\n 0 = N 0 0\n}\n",
        )
        .unwrap();
        std::fs::write(song.join("Guitar.ogg"), b"x").unwrap();
        std::fs::write(song.join("song.opus"), b"x").unwrap();
        std::fs::create_dir_all(root.join("Empty")).unwrap();
        let rip = root.join("Rip");
        std::fs::create_dir_all(&rip).unwrap();
        std::fs::write(
            rip.join("song.ini"),
            "[song]\nname = Zed\nartist = Zz\ndiff_guitar = -1\ndiff_bass = -1\n",
        )
        .unwrap();
        std::fs::write(
            rip.join("notes.chart"),
            "[ExpertDoubleBass]\n{\n 0 = N 1 0\n}\n",
        )
        .unwrap();
        std::fs::write(rip.join("song.ogg"), b"x").unwrap();

        let library = Library::scan(&root).unwrap();
        assert_eq!(library.len(), 2);
        let info: Vec<serde_json::Value> = serde_json::from_str(library.catalog_json()).unwrap();
        assert_eq!(info[0]["name"], "Tune");
        assert_eq!(info[0]["parts"].as_array().unwrap().len(), 1);
        assert_eq!(info[0]["stems"], serde_json::json!(["guitar", "song"]));
        let id = info[0]["id"].as_str().unwrap();
        assert!(library.file(id, "guitar").unwrap().ends_with("Guitar.ogg"));
        assert!(library.file(id, "../song.ini").is_none());
        assert_eq!(library.chart(id).unwrap().tracks.len(), 1);
        assert_eq!(
            info[1]["parts"],
            serde_json::json!([{ "instrument": "bass", "intensity": null }])
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
