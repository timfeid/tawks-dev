//! Readers for the two Clone Hero chart formats.

use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use crate::chart::{
    Chart, Difficulty, Force, Instrument, Options, RawNote, RawTrack, TempoMap, build, lyrics,
    vocal_notes,
};

/// Settings from `song.ini` that change how a chart reads.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IniHints {
    pub delay_ms: f64,
    pub hopo_frequency: Option<u64>,
    pub eighth_note_hopo: bool,
    pub sustain_cutoff: Option<u64>,
}

pub fn read_chart_text(text: &str, hints: IniHints) -> Result<Chart> {
    let text = text.trim_start_matches('\u{feff}');
    let mut tempo = TempoMap {
        resolution: 192,
        ..Default::default()
    };
    let mut offset_s = 0.0;
    let mut sections = Vec::new();
    let mut words: Vec<(u64, String)> = Vec::new();
    let mut phrases: Vec<(u64, u64)> = Vec::new();
    let mut phrase_start: Option<u64> = None;
    let mut tracks: HashMap<(Instrument, Difficulty), RawTrack> = HashMap::new();

    let mut current: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            current = Some(line[1..line.len() - 1].to_string());
            continue;
        }
        if line == "{" || line == "}" || line.is_empty() {
            continue;
        }
        let Some(section) = current.as_deref() else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match section {
            "Song" => match key {
                "Resolution" => tempo.resolution = value.parse().unwrap_or(192),
                "Offset" => offset_s = value.parse().unwrap_or(0.0),
                _ => {}
            },
            "SyncTrack" => {
                let Ok(tick) = key.parse::<u64>() else {
                    continue;
                };
                let mut parts = value.split_whitespace();
                match (
                    parts.next(),
                    parts.next().and_then(|v| v.parse::<f64>().ok()),
                ) {
                    (Some("B"), Some(milli_bpm)) if milli_bpm > 0.0 => {
                        tempo.tempos.push((tick, 60_000_000_000.0 / milli_bpm));
                    }
                    (Some("TS"), Some(beats)) => tempo.signatures.push((tick, beats as u32)),
                    _ => {}
                }
            }
            "Events" => {
                let Ok(tick) = key.parse::<u64>() else {
                    continue;
                };
                if let Some(rest) = value.strip_prefix('E') {
                    let event = rest.trim().trim_matches('"');
                    if let Some(name) = event.strip_prefix("section ") {
                        sections.push((tick, name.replace('_', " ")));
                    } else if let Some(word) = event.strip_prefix("lyric ") {
                        words.push((tick, word.to_string()));
                    } else if event == "phrase_start" {
                        if let Some(start) = phrase_start.replace(tick) {
                            phrases.push((start, tick.saturating_sub(1)));
                        }
                    } else if event == "phrase_end" {
                        if let Some(start) = phrase_start.take() {
                            phrases.push((start, tick));
                        }
                    }
                }
            }
            name => {
                let Some((difficulty, instrument)) = chart_track(name) else {
                    continue;
                };
                let Ok(tick) = key.parse::<u64>() else {
                    continue;
                };
                let mut parts = value.split_whitespace();
                let kind = parts.next();
                let a = parts.next().and_then(|v| v.parse::<u64>().ok());
                let b = parts
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
                let track = tracks
                    .entry((instrument, difficulty))
                    .or_insert_with(|| RawTrack::new(instrument, difficulty));
                match (kind, a) {
                    (Some("N"), Some(lane)) => chart_note(track, tick, lane, b),
                    (Some("S"), Some(2)) => track.star_power.push((tick, tick + b)),
                    _ => {}
                }
            }
        }
    }
    tempo.tempos.sort_by_key(|(tick, _)| *tick);
    tempo.signatures.sort_by_key(|(tick, _)| *tick);
    let resolution = tempo.resolution.max(1);
    let options = Options {
        hopo_threshold: hints
            .hopo_frequency
            .unwrap_or(resolution / if hints.eighth_note_hopo { 2 } else { 3 } + resolution / 192),
        sustain_cutoff: hints.sustain_cutoff.unwrap_or(0),
        offset_ms: offset_s * 1000.0 + hints.delay_ms,
        chord_subset_strums: false,
    };
    if let Some(start) = phrase_start {
        phrases.push((start, start + resolution * 8));
    }
    let offset = options.offset_ms;
    let mut chart = build(&tempo, tracks.into_values().collect(), sections, options);
    chart.lyrics = lyrics(&tempo, offset, &words, &phrases);
    Ok(chart)
}

fn chart_track(name: &str) -> Option<(Difficulty, Instrument)> {
    let (difficulty, rest) = [
        ("Expert", Difficulty::Expert),
        ("Hard", Difficulty::Hard),
        ("Medium", Difficulty::Medium),
        ("Easy", Difficulty::Easy),
    ]
    .into_iter()
    .find_map(|(prefix, d)| name.strip_prefix(prefix).map(|rest| (d, rest)))?;
    let instrument = match rest {
        "Single" => Instrument::Guitar,
        "DoubleBass" => Instrument::Bass,
        "DoubleRhythm" => Instrument::Rhythm,
        "Keyboard" => Instrument::Keys,
        "Drums" => Instrument::Drums,
        _ => return None,
    };
    Some((difficulty, instrument))
}

fn chart_note(track: &mut RawTrack, tick: u64, lane: u64, len: u64) {
    if track.instrument == Instrument::Drums {
        let lane = match lane {
            0..=4 => lane as u8,
            5 => 4,
            _ => return,
        };
        track.notes.push(RawNote { tick, len: 0, lane });
        return;
    }
    match lane {
        0..=4 => track.notes.push(RawNote {
            tick,
            len,
            lane: lane as u8,
        }),
        5 => track.forces.push((tick, tick, Force::Flip)),
        6 => track.taps.push((tick, tick)),
        7 => track.notes.push(RawNote { tick, len, lane: 5 }),
        _ => {}
    }
}

const DIFFICULTY_BASES: [(Difficulty, u8); 4] = [
    (Difficulty::Easy, 60),
    (Difficulty::Medium, 72),
    (Difficulty::Hard, 84),
    (Difficulty::Expert, 96),
];
const TAP_NOTE: u8 = 104;
const STAR_POWER_NOTE: u8 = 116;

fn midi_track(name: &str) -> Option<Instrument> {
    match name.trim() {
        "PART GUITAR" | "T1 GEMS" => Some(Instrument::Guitar),
        "PART BASS" => Some(Instrument::Bass),
        "PART RHYTHM" => Some(Instrument::Rhythm),
        "PART KEYS" => Some(Instrument::Keys),
        "PART DRUMS" => Some(Instrument::Drums),
        _ => None,
    }
}

struct Span {
    key: u8,
    start: u64,
    end: u64,
}

pub fn read_midi(bytes: &[u8], hints: IniHints) -> Result<Chart> {
    let smf = Smf::parse(bytes).map_err(|e| anyhow!("unreadable midi: {e}"))?;
    let resolution = match smf.header.timing {
        Timing::Metrical(ticks) => ticks.as_int() as u64,
        Timing::Timecode(..) => bail!("timecode midi is not supported"),
    };
    let mut tempo = TempoMap {
        resolution,
        ..Default::default()
    };
    let mut sections = Vec::new();
    let mut tracks = Vec::new();
    let mut words: Vec<(u64, String)> = Vec::new();
    let mut phrases: Vec<(u64, u64)> = Vec::new();
    let mut sung_notes: Vec<(u64, u64, u8)> = Vec::new();

    for (index, events) in smf.tracks.iter().enumerate() {
        let mut tick = 0u64;
        let mut name = String::new();
        let mut spans = Vec::new();
        let mut open: HashMap<u8, u64> = HashMap::new();
        let mut sysex: Vec<(u64, Vec<u8>)> = Vec::new();
        let mut enhanced_opens = false;
        let mut sung: Vec<(u64, String)> = Vec::new();
        for event in events {
            tick += event.delta.as_int() as u64;
            match event.kind {
                TrackEventKind::Meta(MetaMessage::TrackName(bytes)) if name.is_empty() => {
                    name = String::from_utf8_lossy(bytes).to_string();
                }
                TrackEventKind::Meta(MetaMessage::Tempo(us)) => {
                    tempo.tempos.push((tick, us.as_int() as f64))
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(beats, ..)) if index == 0 => {
                    tempo.signatures.push((tick, beats as u32));
                }
                TrackEventKind::Meta(MetaMessage::Lyric(bytes)) => {
                    sung.push((tick, String::from_utf8_lossy(bytes).to_string()));
                }
                TrackEventKind::Meta(MetaMessage::Text(bytes)) => {
                    let text = String::from_utf8_lossy(bytes);
                    if !text.trim_start().starts_with('[') {
                        sung.push((tick, text.to_string()));
                    }
                    let text = text.trim().trim_start_matches('[').trim_end_matches(']');
                    if text == "ENHANCED_OPENS" {
                        enhanced_opens = true;
                    } else if let Some(section) = text
                        .strip_prefix("section ")
                        .or_else(|| text.strip_prefix("prc_"))
                    {
                        if name == "EVENTS" {
                            sections.push((tick, section.replace('_', " ")));
                        }
                    }
                }
                TrackEventKind::SysEx(data) => sysex.push((tick, data.to_vec())),
                TrackEventKind::Midi { message, .. } => match message {
                    MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                        open.insert(key.as_int(), tick);
                    }
                    MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                        if let Some(start) = open.remove(&key.as_int()) {
                            spans.push(Span {
                                key: key.as_int(),
                                start,
                                end: tick,
                            });
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if name.trim() == "PART VOCALS" {
            sung_notes = spans
                .iter()
                .filter(|s| (36..=84).contains(&s.key) && s.end > s.start)
                .map(|s| (s.start, s.end, s.key))
                .collect();
            words = sung;
            phrases = spans
                .iter()
                .filter(|s| s.key == 105 || s.key == 106)
                .map(|s| (s.start, s.end))
                .collect();
            continue;
        }
        let Some(instrument) = midi_track(&name) else {
            continue;
        };
        tracks.extend(midi_difficulties(
            instrument,
            &spans,
            &sysex,
            enhanced_opens,
        ));
    }
    tempo.tempos.sort_by_key(|(tick, _)| *tick);
    tempo.signatures.sort_by_key(|(tick, _)| *tick);
    sections.sort_by_key(|(tick, _)| *tick);

    let options = Options {
        hopo_threshold: hints
            .hopo_frequency
            .unwrap_or(resolution / if hints.eighth_note_hopo { 2 } else { 3 })
            + 1,
        sustain_cutoff: hints.sustain_cutoff.unwrap_or(resolution / 3),
        offset_ms: hints.delay_ms,
        chord_subset_strums: true,
    };
    let offset = options.offset_ms;
    let mut chart = build(&tempo, tracks, sections, options);
    chart.vocals = vocal_notes(&tempo, offset, &sung_notes, &words);
    chart.lyrics = lyrics(&tempo, offset, &words, &phrases);
    Ok(chart)
}

/// Phase Shift sysex phrases: `50 53 00 00 <difficulty or ff> <type> <on>`,
/// where type 1 makes green notes open and type 4 makes notes taps.
fn sysex_ranges(sysex: &[(u64, Vec<u8>)], kind: u8, difficulty: u8) -> Vec<(u64, u64)> {
    let mut ranges = Vec::new();
    let mut started: Option<u64> = None;
    for (tick, data) in sysex {
        let data = data.strip_suffix(&[0xf7]).unwrap_or(data);
        if data.len() < 7 || data[..4] != [0x50, 0x53, 0x00, 0x00] {
            continue;
        }
        if data[5] != kind || (data[4] != difficulty && data[4] != 0xff) {
            continue;
        }
        match (data[6], started) {
            (1, None) => started = Some(*tick),
            (0, Some(start)) => {
                ranges.push((start, *tick));
                started = None;
            }
            _ => {}
        }
    }
    ranges
}

fn midi_difficulties(
    instrument: Instrument,
    spans: &[Span],
    sysex: &[(u64, Vec<u8>)],
    enhanced_opens: bool,
) -> Vec<RawTrack> {
    let star_power: Vec<(u64, u64)> = spans
        .iter()
        .filter(|s| s.key == STAR_POWER_NOTE)
        .map(|s| (s.start, s.end.saturating_sub(1).max(s.start)))
        .collect();
    let taps: Vec<(u64, u64)> = spans
        .iter()
        .filter(|s| s.key == TAP_NOTE)
        .map(|s| (s.start, s.end.saturating_sub(1).max(s.start)))
        .collect();

    let mut out = Vec::new();
    for (sysex_difficulty, (difficulty, base)) in DIFFICULTY_BASES.into_iter().enumerate() {
        let mut track = RawTrack::new(instrument, difficulty);
        track.star_power = star_power.clone();
        let opens = sysex_ranges(sysex, 1, sysex_difficulty as u8);
        for span in spans {
            let len = span.end - span.start;
            if instrument == Instrument::Drums {
                let lane = match span.key.checked_sub(base) {
                    Some(lane @ 0..=4) => lane,
                    Some(5) => 4,
                    _ => continue,
                };
                track.notes.push(RawNote {
                    tick: span.start,
                    len: 0,
                    lane,
                });
                continue;
            }
            if enhanced_opens && span.key + 1 == base {
                track.notes.push(RawNote {
                    tick: span.start,
                    len,
                    lane: 5,
                });
                continue;
            }
            match span.key.checked_sub(base) {
                Some(0)
                    if opens
                        .iter()
                        .any(|(a, b)| span.start >= *a && span.start <= *b) =>
                {
                    track.notes.push(RawNote {
                        tick: span.start,
                        len,
                        lane: 5,
                    });
                }
                Some(lane @ 0..=4) => track.notes.push(RawNote {
                    tick: span.start,
                    len,
                    lane,
                }),
                Some(5) => track.forces.push((
                    span.start,
                    span.end.saturating_sub(1).max(span.start),
                    Force::Hopo,
                )),
                Some(6) => track.forces.push((
                    span.start,
                    span.end.saturating_sub(1).max(span.start),
                    Force::Strum,
                )),
                _ => {}
            }
        }
        if instrument != Instrument::Drums {
            track.taps = taps.clone();
            track
                .taps
                .extend(sysex_ranges(sysex, 4, sysex_difficulty as u8));
        }
        if !track.notes.is_empty() {
            out.push(track);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{NoteKind, OPEN};

    const CHART: &str = "\u{feff}[Song]\n{\n  Resolution = 192\n  Offset = 0.5\n}\n[SyncTrack]\n{\n  0 = TS 4\n  0 = B 120000\n}\n[Events]\n{\n  0 = E \"section Intro\"\n}\n[ExpertSingle]\n{\n  0 = N 0 0\n  48 = N 1 0\n  192 = N 7 96\n  384 = N 2 0\n  384 = N 6 0\n  0 = S 2 384\n}\n[HardDrums]\n{\n  0 = N 0 0\n  0 = N 2 0\n  96 = N 5 0\n}\n";

    #[test]
    fn reads_chart_text() {
        let chart = read_chart_text(CHART, IniHints::default()).unwrap();
        assert_eq!(chart.sections[0].name, "Intro");
        let drums = &chart
            .tracks
            .iter()
            .find(|t| t.instrument == Instrument::Drums)
            .unwrap();
        assert_eq!(drums.difficulty, Difficulty::Hard);
        assert_eq!(drums.notes[0].lanes, 0b101);
        assert_eq!(drums.notes[1].lanes, 0b10000);
        let guitar = chart
            .tracks
            .iter()
            .find(|t| t.instrument == Instrument::Guitar)
            .unwrap();
        assert_eq!(guitar.notes[0].t, 500.0);
        assert_eq!(guitar.notes[1].kind, NoteKind::Hopo);
        assert_eq!(guitar.notes[2].lanes, OPEN);
        assert_eq!(guitar.notes[2].len, 250.0);
        assert_eq!(guitar.notes[3].kind, NoteKind::Tap);
        assert_eq!(guitar.star_power, vec![[500.0, 1500.0]]);
    }

    fn midi_file() -> Vec<u8> {
        use midly::num::{u4, u7, u15, u24, u28};
        use midly::{Header, MetaMessage, MidiMessage, Smf, Track, TrackEvent, TrackEventKind};
        let ev = |delta: u32, kind: TrackEventKind<'static>| TrackEvent {
            delta: u28::new(delta),
            kind,
        };
        let on = |key: u8| TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(100),
            },
        };
        let off = |key: u8| TrackEventKind::Midi {
            channel: u4::new(0),
            message: MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            },
        };
        let conductor: Track = vec![
            ev(
                0,
                TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
            ),
            ev(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ];
        let guitar: Track = vec![
            ev(
                0,
                TrackEventKind::Meta(MetaMessage::TrackName(b"PART GUITAR")),
            ),
            ev(0, on(116)),
            ev(0, on(96)),
            ev(10, off(96)),
            ev(470, on(98)),
            ev(0, on(101)),
            ev(960, off(98)),
            ev(0, off(101)),
            ev(0, off(116)),
            ev(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ];
        let vocals: Track = vec![
            ev(
                0,
                TrackEventKind::Meta(MetaMessage::TrackName(b"PART VOCALS")),
            ),
            ev(0, on(105)),
            ev(0, TrackEventKind::Meta(MetaMessage::Lyric(b"Hey#"))),
            ev(0, on(60)),
            ev(240, off(60)),
            ev(0, TrackEventKind::Meta(MetaMessage::Lyric(b"you"))),
            ev(0, on(64)),
            ev(240, off(64)),
            ev(0, off(105)),
            ev(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ];
        let smf = Smf {
            header: Header::new(midly::Format::Parallel, Timing::Metrical(u15::new(480))),
            tracks: vec![conductor, guitar, vocals],
        };
        let mut bytes = Vec::new();
        smf.write(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn reads_midi() {
        let chart = read_midi(&midi_file(), IniHints::default()).unwrap();
        assert_eq!(chart.tracks.len(), 1);
        let track = &chart.tracks[0];
        assert_eq!(track.difficulty, Difficulty::Expert);
        assert_eq!(track.notes[0].len, 0.0);
        assert_eq!(track.notes[1].t, 500.0);
        assert_eq!(track.notes[1].lanes, 0b100);
        assert_eq!(track.notes[1].len, 1000.0);
        assert_eq!(track.notes[1].kind, NoteKind::Hopo);
        assert_eq!(track.star_power.len(), 1);

        assert_eq!(chart.vocals.len(), 2);
        assert_eq!(
            (
                chart.vocals[0].pitch,
                chart.vocals[0].talky,
                chart.vocals[0].len
            ),
            (60, true, 250.0)
        );
        assert_eq!(
            (
                chart.vocals[1].pitch,
                chart.vocals[1].talky,
                chart.vocals[1].t
            ),
            (64, false, 250.0)
        );
        assert_eq!(chart.lyrics.len(), 1);
        assert_eq!(chart.lyrics[0].words.len(), 2);
    }
}
