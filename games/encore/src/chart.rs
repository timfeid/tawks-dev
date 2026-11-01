//! One chart shape for the client, built from either Clone Hero format:
//! `.chart` text files or `notes.mid`. Times are milliseconds from the start
//! of the audio.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Instrument {
    Guitar,
    Bass,
    Rhythm,
    Keys,
    Drums,
    Vocals,
}

impl Instrument {
    pub const ALL: [Instrument; 6] = [
        Instrument::Guitar,
        Instrument::Bass,
        Instrument::Rhythm,
        Instrument::Keys,
        Instrument::Drums,
        Instrument::Vocals,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Instrument::Guitar => "guitar",
            Instrument::Bass => "bass",
            Instrument::Rhythm => "rhythm",
            Instrument::Keys => "keys",
            Instrument::Drums => "drums",
            Instrument::Vocals => "vocals",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl Difficulty {
    pub const ALL: [Difficulty; 4] = [
        Difficulty::Easy,
        Difficulty::Medium,
        Difficulty::Hard,
        Difficulty::Expert,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
            Difficulty::Expert => "expert",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteKind {
    Strum,
    Hopo,
    Tap,
}

/// Lane bits 0 to 4 are the frets green to orange, or on drums the kick then
/// red, yellow, blue and green pads. Bit 5 is an open strum.
pub const OPEN: u8 = 1 << 5;

#[derive(Clone, Debug, Serialize)]
pub struct Note {
    pub t: f64,
    pub len: f64,
    pub lanes: u8,
    pub kind: NoteKind,
}

#[derive(Clone, Debug, Serialize)]
pub struct Track {
    pub instrument: Instrument,
    pub difficulty: Difficulty,
    pub notes: Vec<Note>,
    pub star_power: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Beat {
    pub t: f64,
    pub bar: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Section {
    pub t: f64,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Chart {
    pub tracks: Vec<Track>,
    pub beats: Vec<Beat>,
    pub sections: Vec<Section>,
    pub length: f64,
    /// Sung lines, when the chart has vocals.
    pub lyrics: Vec<LyricLine>,
    /// The notes to sing, when the chart has a vocal part.
    pub vocals: Vec<VocalNote>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VocalNote {
    pub t: f64,
    pub len: f64,
    /// MIDI note number.
    pub pitch: u8,
    /// Spoken or shouted: any voice counts, whatever the pitch.
    pub talky: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Lyric {
    pub t: f64,
    pub text: String,
    /// A syllable that runs straight into the next one, with no space.
    pub joins: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LyricLine {
    pub start: f64,
    pub end: f64,
    pub words: Vec<Lyric>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Force {
    /// `.chart` N 5: the opposite of what the note would naturally be.
    Flip,
    Hopo,
    Strum,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawNote {
    pub tick: u64,
    pub len: u64,
    /// 0 to 4, or 5 for an open note.
    pub lane: u8,
}

#[derive(Debug)]
pub(crate) struct RawTrack {
    pub instrument: Instrument,
    pub difficulty: Difficulty,
    pub notes: Vec<RawNote>,
    /// Inclusive tick ranges.
    pub forces: Vec<(u64, u64, Force)>,
    pub taps: Vec<(u64, u64)>,
    pub star_power: Vec<(u64, u64)>,
}

impl RawTrack {
    pub fn new(instrument: Instrument, difficulty: Difficulty) -> Self {
        RawTrack {
            instrument,
            difficulty,
            notes: Vec::new(),
            forces: Vec::new(),
            taps: Vec::new(),
            star_power: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct TempoMap {
    pub resolution: u64,
    /// Tick and microseconds per quarter note, sorted.
    pub tempos: Vec<(u64, f64)>,
    /// Tick and beats per bar, sorted.
    pub signatures: Vec<(u64, u32)>,
}

impl TempoMap {
    fn anchors(&self) -> Vec<(u64, f64, f64)> {
        let mut tempos = self.tempos.clone();
        if tempos.first().is_none_or(|(tick, _)| *tick != 0) {
            tempos.insert(0, (0, 500_000.0));
        }
        let mut out = Vec::with_capacity(tempos.len());
        let mut ms = 0.0;
        for (i, (tick, us)) in tempos.iter().enumerate() {
            if i > 0 {
                let (prev_tick, _, prev_us) = out[i - 1];
                ms += (tick - prev_tick) as f64 * prev_us / self.resolution as f64 / 1000.0;
            }
            out.push((*tick, ms, *us));
        }
        out
    }
}

struct Clock {
    resolution: f64,
    anchors: Vec<(u64, f64, f64)>,
    offset: f64,
}

impl Clock {
    fn ms(&self, tick: u64) -> f64 {
        let i = self
            .anchors
            .partition_point(|(t, _, _)| *t <= tick)
            .saturating_sub(1);
        let (anchor, ms, us) = self.anchors[i];
        ms + (tick - anchor) as f64 * us / self.resolution / 1000.0 + self.offset
    }
}

pub(crate) struct Options {
    pub hopo_threshold: u64,
    pub sustain_cutoff: u64,
    pub offset_ms: f64,
}

pub(crate) fn build(
    tempo: &TempoMap,
    tracks: Vec<RawTrack>,
    sections: Vec<(u64, String)>,
    options: Options,
) -> Chart {
    let clock = Clock {
        resolution: tempo.resolution.max(1) as f64,
        anchors: tempo.anchors(),
        offset: options.offset_ms,
    };
    let mut out_tracks = Vec::new();
    let mut last_tick = 0;
    for raw in tracks {
        let track = build_track(&clock, raw, &options);
        if let Some(track) = track {
            out_tracks.push(track.0);
            last_tick = last_tick.max(track.1);
        }
    }
    out_tracks.sort_by_key(|t| (t.instrument, t.difficulty));

    let resolution = tempo.resolution.max(1);
    let mut beats = Vec::new();
    let mut signatures = tempo.signatures.clone();
    if signatures.first().is_none_or(|(tick, _)| *tick != 0) {
        signatures.insert(0, (0, 4));
    }
    let mut tick = 0;
    let mut since_change = 0u32;
    let mut sig = 0;
    while tick <= last_tick + resolution * 4 {
        while sig + 1 < signatures.len() && signatures[sig + 1].0 <= tick {
            sig += 1;
            since_change = 0;
        }
        let per_bar = signatures[sig].1.max(1);
        beats.push(Beat {
            t: clock.ms(tick),
            bar: since_change % per_bar == 0,
        });
        since_change += 1;
        tick += resolution;
    }

    let length = out_tracks
        .iter()
        .flat_map(|t| t.notes.last().map(|n| n.t + n.len))
        .fold(0.0, f64::max);
    Chart {
        tracks: out_tracks,
        beats,
        sections: sections
            .into_iter()
            .map(|(tick, name)| Section {
                t: clock.ms(tick),
                name,
            })
            .collect(),
        length,
        lyrics: Vec::new(),
        vocals: Vec::new(),
    }
}

/// Sung notes from `(start, end, pitch)` spans; a lyric marked `#` or `^` is spoken.
pub(crate) fn vocal_notes(
    tempo: &TempoMap,
    offset_ms: f64,
    spans: &[(u64, u64, u8)],
    lyrics: &[(u64, String)],
) -> Vec<VocalNote> {
    let clock = Clock {
        resolution: tempo.resolution.max(1) as f64,
        anchors: tempo.anchors(),
        offset: offset_ms,
    };
    let spoken: std::collections::HashSet<u64> = lyrics
        .iter()
        .filter(|(_, raw)| raw.contains('#') || raw.contains('^'))
        .map(|(tick, _)| *tick)
        .collect();
    let mut notes: Vec<VocalNote> = spans
        .iter()
        .map(|(start, end, pitch)| {
            let t = clock.ms(*start);
            VocalNote {
                t,
                len: clock.ms(*end) - t,
                pitch: *pitch,
                talky: spoken.contains(start),
            }
        })
        .collect();
    notes.sort_by(|a, b| a.t.total_cmp(&b.t));
    notes
}

/// Lines are a gap this long apart when a chart has no phrase markers.
const LYRIC_GAP_MS: f64 = 1_800.0;
const LYRIC_HOLD_MS: f64 = 600.0;

/// Cleans Rock Band lyric markup: `-` joins a syllable to the next, `=` is a
/// literal hyphen, `+` continues the previous note, and `#^*%$/` mark pitch.
/// The text is empty for a held note, which may still join (`+-`).
fn clean_lyric(raw: &str) -> Option<(String, bool)> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "+" || raw.starts_with('[') {
        return None;
    }
    let raw: String = raw
        .chars()
        .filter(|c| !matches!(c, '#' | '^' | '*' | '%' | '$' | '/' | '"' | '+'))
        .collect();
    let joins = raw.ends_with('-') || raw.ends_with('=');
    let text: String = raw
        .trim_end_matches('-')
        .chars()
        .map(|c| match c {
            '=' => '-',
            '_' => ' ',
            '§' => ' ',
            c => c,
        })
        .collect();
    let text = text.trim().to_string();
    (!text.is_empty() || joins).then_some((text, joins))
}

pub(crate) fn lyrics(
    tempo: &TempoMap,
    offset_ms: f64,
    words: &[(u64, String)],
    phrases: &[(u64, u64)],
) -> Vec<LyricLine> {
    let clock = Clock {
        resolution: tempo.resolution.max(1) as f64,
        anchors: tempo.anchors(),
        offset: offset_ms,
    };
    let mut sorted = words.to_vec();
    sorted.sort_by_key(|(tick, _)| *tick);
    let mut words: Vec<(u64, String, bool)> = Vec::new();
    for (tick, raw) in &sorted {
        let Some((text, joins)) = clean_lyric(raw) else {
            continue;
        };
        if text.is_empty() {
            // A held note like `+-` joins whatever came before it to what comes next.
            if let Some(last) = words.last_mut() {
                last.2 |= joins;
            }
            continue;
        }
        words.push((*tick, text, joins));
    }
    let mut phrases = phrases.to_vec();
    phrases.sort();

    let mut lines: Vec<LyricLine> = Vec::new();
    for (tick, text, joins) in words {
        let t = clock.ms(tick);
        let phrase = phrases
            .iter()
            .find(|(start, end)| tick >= *start && tick <= *end);
        let same_line = match (lines.last(), phrase) {
            (Some(line), Some((start, _))) => line.start == clock.ms(*start),
            (Some(line), None) => {
                phrases.is_empty()
                    && t - line.words.last().map(|w| w.t).unwrap_or(line.start) < LYRIC_GAP_MS
            }
            (None, _) => false,
        };
        let word = Lyric { t, text, joins };
        if same_line {
            let line = lines.last_mut().unwrap();
            line.words.push(word);
            if phrase.is_none() {
                line.end = t + LYRIC_HOLD_MS;
            }
        } else {
            let (start, end) = match phrase {
                Some((start, end)) => (clock.ms(*start), clock.ms(*end)),
                None => (t, t + LYRIC_HOLD_MS),
            };
            lines.push(LyricLine {
                start,
                end,
                words: vec![word],
            });
        }
    }
    lines
}

fn in_ranges(ranges: &[(u64, u64)], tick: u64) -> bool {
    ranges
        .iter()
        .any(|(start, end)| tick >= *start && tick <= *end)
}

fn build_track(clock: &Clock, mut raw: RawTrack, options: &Options) -> Option<(Track, u64)> {
    if raw.notes.is_empty() {
        return None;
    }
    raw.notes.sort_by_key(|n| (n.tick, n.lane));
    let drums = raw.instrument == Instrument::Drums;

    let mut chords: Vec<(u64, u64, u8)> = Vec::new();
    for note in &raw.notes {
        let bit = if note.lane >= 5 { OPEN } else { 1 << note.lane };
        let len = if drums || note.len < options.sustain_cutoff {
            0
        } else {
            note.len
        };
        match chords.last_mut() {
            Some((tick, chord_len, lanes)) if *tick == note.tick => {
                *lanes |= bit;
                *chord_len = (*chord_len).max(len);
            }
            _ => chords.push((note.tick, len, bit)),
        }
    }

    let flips: HashSet<u64> = raw
        .forces
        .iter()
        .filter(|(_, _, f)| *f == Force::Flip)
        .map(|(tick, _, _)| *tick)
        .collect();
    let mut notes = Vec::with_capacity(chords.len());
    let mut previous: Option<(u64, u8)> = None;
    let mut last_tick = 0;
    for (tick, len, lanes) in chords {
        let kind = if drums {
            NoteKind::Strum
        } else {
            let mut hopo = match previous {
                Some((prev_tick, prev_lanes)) => {
                    lanes.count_ones() == 1
                        && lanes != prev_lanes
                        && tick - prev_tick <= options.hopo_threshold
                }
                None => false,
            };
            if flips.contains(&tick) {
                hopo = !hopo;
            }
            for (start, end, force) in &raw.forces {
                if tick >= *start && tick <= *end {
                    match force {
                        Force::Hopo => hopo = true,
                        Force::Strum => hopo = false,
                        Force::Flip => {}
                    }
                }
            }
            if in_ranges(&raw.taps, tick) && lanes & OPEN == 0 {
                NoteKind::Tap
            } else if hopo {
                NoteKind::Hopo
            } else {
                NoteKind::Strum
            }
        };
        let t = clock.ms(tick);
        notes.push(Note {
            t,
            len: if len > 0 {
                clock.ms(tick + len) - t
            } else {
                0.0
            },
            lanes,
            kind,
        });
        previous = Some((tick, lanes));
        last_tick = tick + len;
    }

    let star_power = raw
        .star_power
        .iter()
        .map(|(start, end)| [clock.ms(*start), clock.ms(*end)])
        .collect();
    Some((
        Track {
            instrument: raw.instrument,
            difficulty: raw.difficulty,
            notes,
            star_power,
        },
        last_tick,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempo(bpm: f64) -> TempoMap {
        TempoMap {
            resolution: 192,
            tempos: vec![(0, 60_000_000.0 / bpm)],
            signatures: vec![(0, 4)],
        }
    }

    fn options() -> Options {
        Options {
            hopo_threshold: 65,
            sustain_cutoff: 16,
            offset_ms: 0.0,
        }
    }

    #[test]
    fn lyrics_join_syllables_and_follow_phrases() {
        let words = vec![
            (0, "Hel-#".to_string()),
            (48, "lo#".to_string()),
            (96, "+".to_string()),
            (120, "wor".to_string()),
            (140, "+-".to_string()),
            (150, "ld".to_string()),
            (192, "again".to_string()),
            (960, "a-".to_string()),
            (1000, "gain".to_string()),
        ];
        let lines = lyrics(&tempo(120.0), 0.0, &words, &[(0, 400), (900, 1200)]);
        assert_eq!(lines.len(), 2);
        let first: Vec<_> = lines[0]
            .words
            .iter()
            .map(|w| (w.text.as_str(), w.joins))
            .collect();
        assert_eq!(
            first,
            vec![
                ("Hel", true),
                ("lo", false),
                ("wor", true),
                ("ld", false),
                ("again", false)
            ]
        );
        assert_eq!(lines[0].end.round(), 1042.0);
        let loose = lyrics(&tempo(120.0), 0.0, &words, &[]);
        assert_eq!(loose.len(), 2);
    }

    #[test]
    fn ticks_follow_tempo_changes() {
        let map = TempoMap {
            resolution: 192,
            tempos: vec![(0, 500_000.0), (192, 250_000.0)],
            signatures: vec![],
        };
        let clock = Clock {
            resolution: 192.0,
            anchors: map.anchors(),
            offset: 100.0,
        };
        assert_eq!(clock.ms(0), 100.0);
        assert_eq!(clock.ms(192), 600.0);
        assert_eq!(clock.ms(384), 850.0);
    }

    #[test]
    fn close_single_notes_on_new_frets_are_hopos() {
        let mut track = RawTrack::new(Instrument::Guitar, Difficulty::Expert);
        for (tick, lane) in [(0, 0), (48, 1), (96, 1), (288, 2), (336, 3), (336, 4)] {
            track.notes.push(RawNote { tick, len: 0, lane });
        }
        let chart = build(&tempo(120.0), vec![track], vec![], options());
        let kinds: Vec<_> = chart.tracks[0].notes.iter().map(|n| n.kind).collect();
        assert_eq!(
            kinds,
            vec![
                NoteKind::Strum,
                NoteKind::Hopo,
                NoteKind::Strum,
                NoteKind::Strum,
                NoteKind::Strum
            ]
        );
        assert_eq!(chart.tracks[0].notes[4].lanes, 0b11000);
    }

    #[test]
    fn forces_and_taps_override_the_natural_kind() {
        let mut track = RawTrack::new(Instrument::Guitar, Difficulty::Expert);
        for (tick, lane) in [(0, 0), (48, 1), (400, 2), (800, 3)] {
            track.notes.push(RawNote { tick, len: 0, lane });
        }
        track.forces.push((48, 48, Force::Flip));
        track.forces.push((400, 400, Force::Hopo));
        track.taps.push((800, 900));
        let chart = build(&tempo(120.0), vec![track], vec![], options());
        let kinds: Vec<_> = chart.tracks[0].notes.iter().map(|n| n.kind).collect();
        assert_eq!(
            kinds,
            vec![
                NoteKind::Strum,
                NoteKind::Strum,
                NoteKind::Hopo,
                NoteKind::Tap
            ]
        );
    }

    #[test]
    fn short_sustains_are_dropped_and_beats_mark_bars() {
        let mut track = RawTrack::new(Instrument::Guitar, Difficulty::Easy);
        track.notes.push(RawNote {
            tick: 0,
            len: 8,
            lane: 0,
        });
        track.notes.push(RawNote {
            tick: 192,
            len: 192,
            lane: 5,
        });
        let chart = build(
            &tempo(120.0),
            vec![track],
            vec![(0, "Intro".into())],
            options(),
        );
        let notes = &chart.tracks[0].notes;
        assert_eq!(notes[0].len, 0.0);
        assert_eq!(notes[1].len, 500.0);
        assert_eq!(notes[1].lanes, OPEN);
        assert!(chart.beats[0].bar && !chart.beats[1].bar && chart.beats[4].bar);
        assert_eq!(chart.length, 1000.0);
    }
}
