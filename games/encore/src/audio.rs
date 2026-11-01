//! Reads how long an uploaded Ogg (Vorbis or Opus) or MP3 file plays, walking
//! the container from start to end so a file that only looks like audio in its
//! first bytes is refused.

/// The playing time in milliseconds, or None when the bytes are not a
/// well-formed file of the kind its extension claims.
pub fn duration_ms(bytes: &[u8], extension: &str) -> Option<u64> {
    match extension {
        "ogg" | "opus" => ogg_duration_ms(bytes),
        "mp3" => mp3_duration_ms(bytes),
        _ => None,
    }
}

fn ogg_duration_ms(bytes: &[u8]) -> Option<u64> {
    let mut pos = 0;
    let mut first_serial = None;
    let mut rate_and_skip = None;
    let mut last_granule = None;
    while pos < bytes.len() {
        let header = bytes.get(pos..pos + 27)?;
        if &header[..4] != b"OggS" || header[4] != 0 {
            return None;
        }
        let granule = u64::from_le_bytes(header[6..14].try_into().ok()?);
        let serial = u32::from_le_bytes(header[14..18].try_into().ok()?);
        let segments = header[26] as usize;
        let table = bytes.get(pos + 27..pos + 27 + segments)?;
        let body_start = pos + 27 + segments;
        let body_len: usize = table.iter().map(|s| *s as usize).sum();
        let body = bytes.get(body_start..body_start + body_len)?;
        let first = *first_serial.get_or_insert(serial);
        if rate_and_skip.is_none() {
            rate_and_skip = Some(codec_rate(body)?);
        }
        if serial == first && granule != u64::MAX {
            last_granule = Some(granule);
        }
        pos = body_start + body_len;
    }
    let (rate, skip) = rate_and_skip?;
    let samples = last_granule?.saturating_sub(skip);
    (rate > 0).then(|| samples * 1000 / rate)
}

/// The sample rate the granule positions count in, and the samples to skip.
fn codec_rate(packet: &[u8]) -> Option<(u64, u64)> {
    if packet.len() >= 16 && packet[0] == 1 && &packet[1..7] == b"vorbis" {
        let rate = u32::from_le_bytes(packet[12..16].try_into().ok()?);
        return Some((rate as u64, 0));
    }
    if packet.len() >= 19 && &packet[..8] == b"OpusHead" {
        let pre_skip = u16::from_le_bytes(packet[10..12].try_into().ok()?);
        return Some((48_000, pre_skip as u64));
    }
    None
}

const MPEG1_BITRATES: [u32; 15] = [
    0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];
const MPEG2_BITRATES: [u32; 15] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
const MIN_MP3_FRAMES: usize = 20;

fn skip_id3v2(bytes: &[u8]) -> usize {
    if bytes.len() < 10 || &bytes[..3] != b"ID3" {
        return 0;
    }
    let size = bytes[6..10]
        .iter()
        .fold(0usize, |acc, b| (acc << 7) | (*b as usize & 0x7f));
    let footer = if bytes[5] & 0x10 != 0 { 10 } else { 0 };
    10 + size + footer
}

/// One Layer III frame header: its length in bytes, samples and sample rate.
fn mp3_frame(header: &[u8]) -> Option<(usize, u64, u64)> {
    if header.len() < 4 || header[0] != 0xff || header[1] & 0xe0 != 0xe0 {
        return None;
    }
    let version = (header[1] >> 3) & 0b11;
    let layer = (header[1] >> 1) & 0b11;
    if version == 0b01 || layer != 0b01 {
        return None;
    }
    let bitrate_index = (header[2] >> 4) as usize;
    let rate_index = ((header[2] >> 2) & 0b11) as usize;
    if bitrate_index == 0 || bitrate_index == 15 || rate_index == 3 {
        return None;
    }
    let padding = ((header[2] >> 1) & 1) as usize;
    let mpeg1 = version == 0b11;
    let bitrate = if mpeg1 {
        MPEG1_BITRATES
    } else {
        MPEG2_BITRATES
    }[bitrate_index] as usize
        * 1000;
    let rate = match version {
        0b11 => [44_100, 48_000, 32_000][rate_index],
        0b10 => [22_050, 24_000, 16_000][rate_index],
        _ => [11_025, 12_000, 8_000][rate_index],
    };
    let (coefficient, samples) = if mpeg1 { (144, 1152) } else { (72, 576) };
    let length = coefficient * bitrate / rate + padding;
    Some((length, samples, rate as u64))
}

fn mp3_duration_ms(bytes: &[u8]) -> Option<u64> {
    let mut pos = skip_id3v2(bytes);
    let start = pos;
    let mut frames = 0usize;
    let mut micros = 0u64;
    while let Some((length, samples, rate)) = bytes.get(pos..pos + 4).and_then(mp3_frame) {
        if length < 4 || pos + length > bytes.len() {
            break;
        }
        frames += 1;
        micros += samples * 1_000_000 / rate;
        pos += length;
    }
    let rest = &bytes[pos..];
    let trailing_ok = rest.is_empty() || (rest.len() <= 128 + 4 && rest.starts_with(b"TAG"));
    let covered = pos - start;
    let body = bytes.len() - start;
    if frames < MIN_MP3_FRAMES || (!trailing_ok && covered * 10 < body * 9) {
        return None;
    }
    Some(micros / 1000)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn page(granule: u64, serial: u32, body: &[u8]) -> Vec<u8> {
        let mut out = b"OggS".to_vec();
        out.push(0);
        out.push(0);
        out.extend_from_slice(&granule.to_le_bytes());
        out.extend_from_slice(&serial.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        let mut segments = Vec::new();
        let mut left = body.len();
        loop {
            let take = left.min(255);
            segments.push(take as u8);
            left -= take;
            if take < 255 {
                break;
            }
        }
        out.push(segments.len() as u8);
        out.extend_from_slice(&segments);
        out.extend_from_slice(body);
        out
    }

    pub fn vorbis(seconds: u64) -> Vec<u8> {
        let mut id = vec![1];
        id.extend_from_slice(b"vorbis");
        id.extend_from_slice(&0u32.to_le_bytes());
        id.push(2);
        id.extend_from_slice(&44_100u32.to_le_bytes());
        id.extend_from_slice(&[0; 14]);
        let mut out = page(0, 7, &id);
        out.extend(page(44_100 * seconds / 2, 7, &[0; 300]));
        out.extend(page(44_100 * seconds, 7, &[0; 300]));
        out
    }

    fn opus(seconds: u64) -> Vec<u8> {
        let mut head = b"OpusHead".to_vec();
        head.push(1);
        head.push(2);
        head.extend_from_slice(&312u16.to_le_bytes());
        head.extend_from_slice(&48_000u32.to_le_bytes());
        head.extend_from_slice(&[0; 3]);
        let mut out = page(0, 3, &head);
        out.extend(page(48_000 * seconds + 312, 3, &[0; 100]));
        out
    }

    pub fn mp3(frames: usize) -> Vec<u8> {
        let mut out = b"ID3\x04\x00\x00\x00\x00\x00\x05hello".to_vec();
        for _ in 0..frames {
            let mut frame = vec![0u8; 417];
            frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0x00]);
            out.extend(frame);
        }
        out
    }

    #[test]
    fn reads_vorbis_and_opus_lengths() {
        assert_eq!(duration_ms(&vorbis(90), "ogg"), Some(90_000));
        assert_eq!(duration_ms(&opus(42), "opus"), Some(42_000));
        let mut cut = vorbis(10);
        cut.truncate(cut.len() - 20);
        assert_eq!(duration_ms(&cut, "ogg"), None);
        assert_eq!(duration_ms(b"OggS not really", "ogg"), None);
    }

    #[test]
    fn reads_mp3_lengths_and_refuses_noise() {
        let ms = duration_ms(&mp3(1000), "mp3").unwrap();
        assert!((26_000..26_200).contains(&ms), "{ms}");
        assert_eq!(duration_ms(&mp3(5), "mp3"), None);
        let mut noisy = mp3(100);
        noisy.extend(vec![7u8; 100_000]);
        assert_eq!(duration_ms(&noisy, "mp3"), None);
        assert_eq!(duration_ms(&vorbis(5), "mp3"), None);
        assert_eq!(duration_ms(&mp3(100), "wav"), None);
    }
}
