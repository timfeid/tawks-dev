//! Songs players upload themselves. The server checks every file before
//! anything is stored, then hands the files to tawks, which keeps them in
//! protected storage and decides who may reach them. An upload is private to
//! its uploader: it shows up only in rooms they are in.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::audio;
use crate::formats::IniHints;
use crate::library::{ART, STEMS, SongInfo, Source, read_chart_file, read_entry};

pub const MAX_FILES: usize = 16;
pub const MAX_TOTAL_BYTES: usize = 100 * 1024 * 1024;
pub const MAX_AUDIO_BYTES: usize = 40 * 1024 * 1024;
pub const MAX_CHART_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_INI_BYTES: usize = 64 * 1024;
pub const MAX_ART_BYTES: usize = 2 * 1024 * 1024;
pub const MIN_LENGTH_MS: u64 = 10_000;
pub const MAX_LENGTH_MS: u64 = 12 * 60 * 1000;
pub const MAX_TEXT_CHARS: usize = 120;
/// Audio formats an upload may use; the server's own folder also reads wav and flac.
pub const UPLOAD_AUDIO: [&str; 3] = ["ogg", "opus", "mp3"];

/// What tawks keeps about an upload, beside the files themselves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadedSong {
    pub info: SongInfo,
    /// `notes.mid` or `notes.chart`.
    pub chart: String,
    pub hints: IniHints,
    /// Published file names, such as `guitar` or `album`, to stored file names.
    pub files: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct StoredFile {
    pub name: String,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct Validated {
    pub song: UploadedSong,
    pub files: Vec<StoredFile>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Default)]
pub struct UploadForm {
    pub name: Option<String>,
    pub artist: Option<String>,
    /// The "I own this or have the right to use it" box.
    pub rights: bool,
    pub files: Vec<(String, Vec<u8>)>,
}

/// An upload as tawks returns it. `official`, `uploaded_by` and
/// `uploader_name` come from tawks, never from the stored song.
#[derive(Clone, Debug, Deserialize)]
pub struct UploadRecord {
    pub id: String,
    pub uploaded_by: String,
    #[serde(default)]
    pub uploader_name: Option<String>,
    #[serde(default)]
    pub official: bool,
    pub song: UploadedSong,
}

impl UploadRecord {
    pub fn info(&self) -> SongInfo {
        let mut info = self.song.info.clone();
        info.id = self.id.clone();
        info.source = Source::Upload;
        info.official = self.official;
        info.uploaded_by = Some(self.uploaded_by.clone());
        info.uploader_name = self.uploader_name.clone();
        info
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UploaderStatus {
    pub can_upload: bool,
    #[serde(default)]
    pub reason: Option<String>,
    pub songs: u32,
    pub max_songs: u32,
}

/// Where uploads live. tawks in production; memory in tests.
#[async_trait]
pub trait SongBackend: Send + Sync {
    /// Uploads by these users that are still up, plus every official song.
    async fn list(&self, user_ids: &[String]) -> Result<Vec<UploadRecord>>;
    async fn get(&self, id: &str) -> Result<Option<UploadRecord>>;
    async fn fetch(&self, id: &str, file: &str) -> Result<Vec<u8>>;
    async fn create(&self, user_id: &str, validated: &Validated) -> Result<String>;
    async fn delete(&self, id: &str, user_id: &str) -> Result<()>;
    async fn report(
        &self,
        id: &str,
        reporter: &str,
        lobby_id: &str,
        reason: &str,
        details: &str,
    ) -> Result<()>;
    async fn status(&self, user_id: &str) -> Result<UploaderStatus>;
}

fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag && !c.is_control() => out.push(c),
            _ => {}
        }
    }
    out.trim().chars().take(MAX_TEXT_CHARS).collect()
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or_default() {
        "ogg" | "opus" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "mid" => "audio/midi",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "text/plain; charset=utf-8",
    }
}

enum Kind {
    Ini,
    Chart,
    Audio,
    Art,
}

/// Keeps the files Encore can play and says which it set aside.
fn classify(name: &str) -> Option<Kind> {
    match name {
        "song.ini" => return Some(Kind::Ini),
        "notes.mid" | "notes.chart" => return Some(Kind::Chart),
        _ if ART.contains(&name) => return Some(Kind::Art),
        _ => {}
    }
    let (stem, extension) = name.rsplit_once('.')?;
    (STEMS.contains(&stem) && UPLOAD_AUDIO.contains(&extension)).then_some(Kind::Audio)
}

fn base_name(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim()
        .to_ascii_lowercase()
}

fn image_ok(name: &str, bytes: &[u8]) -> bool {
    if name.ends_with(".png") {
        bytes.starts_with(b"\x89PNG\r\n\x1a\n")
    } else {
        bytes.starts_with(&[0xff, 0xd8, 0xff])
    }
}

fn megabytes(bytes: usize) -> usize {
    bytes / (1024 * 1024)
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<TempDir> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "encore-upload-{}-{nanos}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(TempDir(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ini_text(name: &str, artist: &str) -> String {
    let clean = |s: &str| s.replace(['\r', '\n'], " ");
    format!(
        "[song]\nname = {}\nartist = {}\n",
        clean(name),
        clean(artist)
    )
}

fn ini_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
        .map(|(_, v)| plain(v))
        .filter(|v| !v.is_empty())
}

/// Checks an upload against the limits and reads it the way the library
/// reads a song folder, so anything that passes plays.
pub fn validate(form: UploadForm) -> Result<Validated, String> {
    if !form.rights {
        return Err("Confirm that you own this song or have the right to use it".into());
    }
    if form.files.len() > MAX_FILES * 4 {
        return Err("That is too many files for one song".into());
    }
    let mut kept: Vec<(String, Kind, Vec<u8>)> = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = HashSet::new();
    let mut total = 0usize;
    for (raw_name, bytes) in form.files {
        let name = base_name(&raw_name);
        let Some(kind) = classify(&name) else {
            skipped.push(name);
            continue;
        };
        let stem = name
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(&name)
            .to_string();
        let slot = match kind {
            Kind::Ini => "ini".to_string(),
            Kind::Chart => "chart".to_string(),
            Kind::Art => "album".to_string(),
            _ => stem,
        };
        if !seen.insert(slot) {
            return Err(format!("{name} is in the upload twice"));
        }
        let limit = match kind {
            Kind::Ini => MAX_INI_BYTES,
            Kind::Chart => MAX_CHART_BYTES,
            Kind::Audio => MAX_AUDIO_BYTES,
            Kind::Art => MAX_ART_BYTES,
        };
        if bytes.len() > limit {
            return Err(format!(
                "{name} is over the {} MB limit",
                megabytes(limit).max(1)
            ));
        }
        if bytes.is_empty() {
            return Err(format!("{name} is empty"));
        }
        total += bytes.len();
        kept.push((name, kind, bytes));
    }
    if total > MAX_TOTAL_BYTES {
        return Err(format!(
            "A song can be up to {} MB in all",
            megabytes(MAX_TOTAL_BYTES)
        ));
    }
    if kept.len() > MAX_FILES {
        return Err(format!("A song can have up to {MAX_FILES} files"));
    }
    if !kept.iter().any(|(_, k, _)| matches!(k, Kind::Chart)) {
        return Err("Add the chart: notes.mid or notes.chart".into());
    }
    if !kept.iter().any(|(_, k, _)| matches!(k, Kind::Audio)) {
        return Err(
            "Add the audio as Ogg, Opus or MP3 files named like song.ogg or guitar.opus".into(),
        );
    }

    let mut longest = 0u64;
    for (name, kind, bytes) in &kept {
        match kind {
            Kind::Audio => {
                let extension = name.rsplit('.').next().unwrap_or_default();
                let ms = audio::duration_ms(bytes, extension)
                    .ok_or_else(|| format!("{name} is not a readable Ogg, Opus or MP3 file"))?;
                if ms > MAX_LENGTH_MS {
                    return Err(format!(
                        "{name} is longer than {} minutes",
                        MAX_LENGTH_MS / 60_000
                    ));
                }
                longest = longest.max(ms);
            }
            Kind::Art if !image_ok(name, bytes) => {
                return Err(format!("{name} is not a PNG or JPEG image"));
            }
            _ => {}
        }
    }
    if longest < MIN_LENGTH_MS {
        return Err(format!(
            "Songs have to be at least {} seconds long",
            MIN_LENGTH_MS / 1000
        ));
    }

    let ini = kept
        .iter()
        .find(|(_, k, _)| matches!(k, Kind::Ini))
        .map(|(_, _, b)| {
            String::from_utf8_lossy(b)
                .trim_start_matches('\u{feff}')
                .to_string()
        });
    let field = |value: &Option<String>, key: &str| {
        value
            .as_deref()
            .map(plain)
            .filter(|v| !v.is_empty())
            .or_else(|| ini.as_deref().and_then(|text| ini_value(text, key)))
    };
    let name = field(&form.name, "name").ok_or("Give the song a title")?;
    let artist = field(&form.artist, "artist").ok_or("Say who the artist is")?;

    let temp = TempDir::new().map_err(|_| "The upload could not be read".to_string())?;
    let mut names = Vec::new();
    for (file, kind, bytes) in &kept {
        if matches!(kind, Kind::Ini) {
            continue;
        }
        let path = temp.0.join(file);
        std::fs::write(&path, bytes).map_err(|_| "The upload could not be read".to_string())?;
        names.push((file.clone(), path));
    }
    let ini_path = temp.0.join("song.ini");
    let ini_body = match &ini {
        Some(text) => text.clone(),
        None => ini_text(&name, &artist),
    };
    std::fs::write(&ini_path, ini_body).map_err(|_| "The upload could not be read".to_string())?;
    names.push(("song.ini".into(), ini_path.clone()));

    let entry = read_entry("upload", &temp.0, &ini_path, names)
        .map_err(|e| format!("The chart did not read: {e}"))?;
    let chart_name = entry
        .chart_path()
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let chart_bytes = kept
        .iter()
        .find(|(n, _, _)| *n == chart_name)
        .map(|(_, _, b)| b.as_slice())
        .unwrap_or_default();
    let chart = read_chart_file(&chart_name, chart_bytes, entry.hints)
        .map_err(|e| format!("The chart did not read: {e}"))?;
    if chart.tracks.is_empty() && chart.vocals.is_empty() {
        return Err("The chart has no notes to play".into());
    }
    if chart.length > (MAX_LENGTH_MS + 60_000) as f64 {
        return Err(format!(
            "The chart runs longer than {} minutes",
            MAX_LENGTH_MS / 60_000
        ));
    }

    let mut info = entry.info.clone();
    info.name = name;
    info.artist = artist;
    info.length_ms = longest;
    info.preview_ms = info.preview_ms.min(longest);
    info.source = Source::Upload;
    info.official = false;
    info.uploaded_by = None;
    info.uploader_name = None;
    for field in [
        &mut info.album,
        &mut info.year,
        &mut info.genre,
        &mut info.charter,
    ] {
        *field = field.as_deref().map(plain).filter(|v| !v.is_empty());
    }

    let mut published = BTreeMap::new();
    for (stem, path) in &entry.files {
        if let Some(file) = path.file_name() {
            published.insert(stem.clone(), file.to_string_lossy().to_string());
        }
    }
    let art = kept
        .iter()
        .find(|(_, k, _)| matches!(k, Kind::Art))
        .map(|(n, _, _)| n.clone());
    info.art = art.is_some();
    if let Some(art) = art {
        published.insert("album".into(), art);
    }
    let stored: HashSet<&String> = published
        .values()
        .chain(std::iter::once(&chart_name))
        .collect();
    let files = kept
        .into_iter()
        .filter(|(name, _, _)| stored.contains(name))
        .map(|(name, _, bytes)| StoredFile {
            content_type: content_type(&name),
            name,
            bytes,
        })
        .collect();
    Ok(Validated {
        song: UploadedSong {
            info,
            chart: chart_name,
            hints: entry.hints,
            files: published,
        },
        files,
        skipped,
    })
}

/// Uploads stored through the tawks API. Every call is bot-only on the API
/// side; files move through short-lived signed links.
pub struct ApiBackend {
    bot: Arc<RwLock<Option<Arc<tawks_bot::TawksSdk>>>>,
    http: reqwest::Client,
}

impl ApiBackend {
    pub fn new(bot: Arc<RwLock<Option<Arc<tawks_bot::TawksSdk>>>>) -> Self {
        ApiBackend {
            bot,
            http: reqwest::Client::new(),
        }
    }

    fn sdk(&self) -> Result<Arc<tawks_bot::TawksSdk>> {
        self.bot
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow!("Encore is still connecting to tawks"))
    }
}

/// The message tawks gave, without the client's prefix.
pub fn api_message(error: &anyhow::Error) -> String {
    let text = error.to_string();
    match text.split_once(" response: ") {
        Some((head, message)) if head.starts_with("rspc") => message.to_string(),
        _ => text,
    }
}

#[derive(Deserialize)]
struct CreatedUpload {
    id: String,
    uploads: Vec<SignedPut>,
}

#[derive(Deserialize)]
struct SignedPut {
    name: String,
    url: String,
}

#[async_trait]
impl SongBackend for ApiBackend {
    async fn list(&self, user_ids: &[String]) -> Result<Vec<UploadRecord>> {
        self.sdk()?
            .web_client
            .query("encore_song_list", &json!({ "user_ids": user_ids }))
            .await
    }

    async fn get(&self, id: &str) -> Result<Option<UploadRecord>> {
        self.sdk()?.web_client.query("encore_song_get", &id).await
    }

    async fn fetch(&self, id: &str, file: &str) -> Result<Vec<u8>> {
        let url: String = self
            .sdk()?
            .web_client
            .query("encore_song_file_link", &json!({ "id": id, "name": file }))
            .await?;
        let response = self.http.get(url).send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }

    async fn create(&self, user_id: &str, validated: &Validated) -> Result<String> {
        let sdk = self.sdk()?;
        let files: Vec<_> = validated
            .files
            .iter()
            .map(|f| json!({ "name": f.name, "bytes": f.bytes.len() as f64, "content_type": f.content_type }))
            .collect();
        let created: CreatedUpload = sdk
            .web_client
            .mutate(
                "encore_upload_create",
                &json!({
                    "user_id": user_id,
                    "title": validated.song.info.name,
                    "artist": validated.song.info.artist,
                    "song": validated.song,
                    "files": files,
                    "rights_accepted": true,
                }),
            )
            .await?;
        let mut sent = Ok(());
        for upload in &created.uploads {
            let Some(file) = validated.files.iter().find(|f| f.name == upload.name) else {
                sent = Err(anyhow!("tawks asked for a file that is not in the upload"));
                break;
            };
            let result = self
                .http
                .put(&upload.url)
                .header(reqwest::header::CONTENT_TYPE, file.content_type)
                .body(file.bytes.clone())
                .send()
                .await
                .and_then(|r| r.error_for_status());
            if let Err(error) = result {
                sent = Err(anyhow!("storing {} failed: {error}", file.name));
                break;
            }
        }
        let ok = sent.is_ok();
        let finished: Result<serde_json::Value> = sdk
            .web_client
            .mutate(
                "encore_upload_finish",
                &json!({ "id": created.id, "user_id": user_id, "ok": ok }),
            )
            .await;
        sent?;
        finished?;
        Ok(created.id)
    }

    async fn delete(&self, id: &str, user_id: &str) -> Result<()> {
        let _: serde_json::Value = self
            .sdk()?
            .web_client
            .mutate(
                "encore_song_delete",
                &json!({ "id": id, "user_id": user_id }),
            )
            .await?;
        Ok(())
    }

    async fn report(
        &self,
        id: &str,
        reporter: &str,
        lobby_id: &str,
        reason: &str,
        details: &str,
    ) -> Result<()> {
        let _: serde_json::Value = self
            .sdk()?
            .web_client
            .mutate(
                "encore_song_report",
                &json!({ "id": id, "reporter_id": reporter, "lobby_id": lobby_id, "reason": reason, "details": details }),
            )
            .await?;
        Ok(())
    }

    async fn status(&self, user_id: &str) -> Result<UploaderStatus> {
        self.sdk()?
            .web_client
            .query("encore_uploader_status", &user_id)
            .await
    }
}

/// Uploaded files kept on local disk after their first fetch, so a band of
/// four does not pull every stem from storage four times.
pub struct FileCache {
    dir: PathBuf,
    max_bytes: u64,
    lock: Mutex<()>,
}

impl FileCache {
    pub fn new(dir: PathBuf, max_bytes: u64) -> Self {
        FileCache {
            dir,
            max_bytes,
            lock: Mutex::new(()),
        }
    }

    fn safe(part: &str) -> bool {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            && !part.starts_with('.')
    }

    pub fn path(&self, id: &str, file: &str) -> Option<PathBuf> {
        (Self::safe(id) && Self::safe(file)).then(|| self.dir.join(id).join(file))
    }

    pub async fn get(&self, backend: &dyn SongBackend, id: &str, file: &str) -> Result<PathBuf> {
        let path = self.path(id, file).context("bad file name")?;
        if path.is_file() {
            return Ok(path);
        }
        let bytes = backend.fetch(id, file).await?;
        let dir = path.parent().context("no folder")?.to_path_buf();
        tokio::fs::create_dir_all(&dir).await?;
        let temp = dir.join(format!(
            ".{file}.{}",
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        tokio::fs::write(&temp, &bytes).await?;
        tokio::fs::rename(&temp, &path).await?;
        self.trim();
        Ok(path)
    }

    pub fn forget(&self, id: &str) {
        if Self::safe(id) {
            let _ = std::fs::remove_dir_all(self.dir.join(id));
        }
    }

    /// Drops the songs fetched longest ago once the cache is over its size.
    fn trim(&self) {
        let _guard = self.lock.lock().unwrap();
        let Ok(read) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let mut songs: Vec<(std::time::SystemTime, u64, PathBuf)> = read
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| {
                let path = e.path();
                let (size, newest) = dir_stats(&path);
                (newest, size, path)
            })
            .collect();
        let mut total: u64 = songs.iter().map(|s| s.1).sum();
        songs.sort_by_key(|s| s.0);
        for (_, size, path) in songs {
            if total <= self.max_bytes {
                break;
            }
            let _ = std::fs::remove_dir_all(&path);
            total = total.saturating_sub(size);
        }
    }
}

fn dir_stats(dir: &Path) -> (u64, std::time::SystemTime) {
    let mut size = 0;
    let mut newest = std::time::UNIX_EPOCH;
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if let Ok(meta) = entry.metadata() {
            size += meta.len();
            if let Ok(modified) = meta.modified() {
                newest = newest.max(modified);
            }
        }
    }
    (size, newest)
}

#[cfg(test)]
#[derive(Default)]
pub struct MemoryBackend {
    pub songs: Mutex<Vec<(UploadRecord, std::collections::HashMap<String, Vec<u8>>)>>,
    pub reports: Mutex<Vec<(String, String, String)>>,
    pub blocked: Mutex<HashSet<String>>,
    next: AtomicU64,
}

#[cfg(test)]
#[async_trait]
impl SongBackend for MemoryBackend {
    async fn list(&self, user_ids: &[String]) -> Result<Vec<UploadRecord>> {
        Ok(self
            .songs
            .lock()
            .unwrap()
            .iter()
            .filter(|(r, _)| r.official || user_ids.contains(&r.uploaded_by))
            .map(|(r, _)| r.clone())
            .collect())
    }

    async fn get(&self, id: &str) -> Result<Option<UploadRecord>> {
        Ok(self
            .songs
            .lock()
            .unwrap()
            .iter()
            .find(|(r, _)| r.id == id)
            .map(|(r, _)| r.clone()))
    }

    async fn fetch(&self, id: &str, file: &str) -> Result<Vec<u8>> {
        self.songs
            .lock()
            .unwrap()
            .iter()
            .find(|(r, _)| r.id == id)
            .and_then(|(_, files)| files.get(file).cloned())
            .context("no such file")
    }

    async fn create(&self, user_id: &str, validated: &Validated) -> Result<String> {
        if self.blocked.lock().unwrap().contains(user_id) {
            anyhow::bail!("You can't upload songs any more");
        }
        let id = format!("ENS-{}", self.next.fetch_add(1, Ordering::Relaxed));
        let files = validated
            .files
            .iter()
            .map(|f| (f.name.clone(), f.bytes.clone()))
            .collect();
        let record = UploadRecord {
            id: id.clone(),
            uploaded_by: user_id.to_string(),
            uploader_name: Some(user_id.to_string()),
            official: false,
            song: validated.song.clone(),
        };
        self.songs.lock().unwrap().push((record, files));
        Ok(id)
    }

    async fn delete(&self, id: &str, user_id: &str) -> Result<()> {
        let mut songs = self.songs.lock().unwrap();
        let before = songs.len();
        songs.retain(|(r, _)| !(r.id == id && r.uploaded_by == user_id));
        if songs.len() == before {
            anyhow::bail!("That song is not yours");
        }
        Ok(())
    }

    async fn report(
        &self,
        id: &str,
        reporter: &str,
        _lobby_id: &str,
        reason: &str,
        _details: &str,
    ) -> Result<()> {
        self.reports
            .lock()
            .unwrap()
            .push((id.into(), reporter.into(), reason.into()));
        Ok(())
    }

    async fn status(&self, user_id: &str) -> Result<UploaderStatus> {
        let blocked = self.blocked.lock().unwrap().contains(user_id);
        Ok(UploaderStatus {
            can_upload: !blocked,
            reason: blocked.then(|| "Upload access was removed".into()),
            songs: self
                .songs
                .lock()
                .unwrap()
                .iter()
                .filter(|(r, _)| r.uploaded_by == user_id)
                .count() as u32,
            max_songs: 25,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::audio::tests::{mp3, vorbis};

    pub const CHART: &str =
        "[Song]\n{\n Resolution = 192\n}\n[ExpertSingle]\n{\n 0 = N 0 0\n 192 = N 1 0\n}\n";

    pub fn form(files: Vec<(&str, Vec<u8>)>) -> UploadForm {
        UploadForm {
            name: Some("My <b>Tune</b>".into()),
            artist: Some("Me".into()),
            rights: true,
            files: files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        }
    }

    pub fn good_upload() -> UploadForm {
        form(vec![
            ("My Song/notes.chart", CHART.as_bytes().to_vec()),
            ("My Song/Song.ogg", vorbis(30)),
            ("My Song/guitar.mp3", mp3(800)),
            ("My Song/readme.txt", b"hi".to_vec()),
        ])
    }

    #[test]
    fn accepts_a_song_folder() {
        let validated = validate(good_upload()).unwrap();
        let info = &validated.song.info;
        assert_eq!(info.name, "My Tune");
        assert_eq!(info.artist, "Me");
        assert_eq!(info.length_ms, 30_000);
        assert_eq!(info.source, Source::Upload);
        assert!(!info.official);
        assert_eq!(info.stems, vec!["guitar", "song"]);
        assert_eq!(validated.song.chart, "notes.chart");
        assert_eq!(validated.song.files["song"], "song.ogg");
        assert_eq!(validated.skipped, vec!["readme.txt"]);
        let mut names: Vec<&str> = validated.files.iter().map(|f| f.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["guitar.mp3", "notes.chart", "song.ogg"]);
    }

    #[test]
    fn refuses_uploads_that_break_the_rules() {
        let mut no_rights = good_upload();
        no_rights.rights = false;
        assert!(validate(no_rights).unwrap_err().contains("right to use"));

        let no_chart = form(vec![("song.ogg", vorbis(30))]);
        assert!(validate(no_chart).unwrap_err().contains("chart"));

        let wav = form(vec![
            ("notes.chart", CHART.as_bytes().to_vec()),
            ("song.wav", vec![1; 100]),
        ]);
        assert!(validate(wav).unwrap_err().contains("audio"));

        let fake = form(vec![
            ("notes.chart", CHART.as_bytes().to_vec()),
            ("song.ogg", vec![1; 5000]),
        ]);
        assert!(validate(fake).unwrap_err().contains("not a readable"));

        let short = form(vec![
            ("notes.chart", CHART.as_bytes().to_vec()),
            ("song.ogg", vorbis(3)),
        ]);
        assert!(validate(short).unwrap_err().contains("at least"));

        let long = form(vec![
            ("notes.chart", CHART.as_bytes().to_vec()),
            ("song.ogg", vorbis(13 * 60)),
        ]);
        assert!(validate(long).unwrap_err().contains("longer than"));

        let huge = form(vec![
            ("notes.chart", vec![b' '; MAX_CHART_BYTES + 1]),
            ("song.ogg", vorbis(30)),
        ]);
        assert!(validate(huge).unwrap_err().contains("limit"));

        let twice = form(vec![
            ("notes.chart", CHART.as_bytes().to_vec()),
            ("notes.mid", vec![1; 10]),
            ("song.ogg", vorbis(30)),
        ]);
        assert!(validate(twice).unwrap_err().contains("twice"));

        let empty_chart = form(vec![
            ("notes.chart", b"[Song]\n{\n}\n".to_vec()),
            ("song.ogg", vorbis(30)),
        ]);
        assert!(validate(empty_chart).is_err());

        let mut untitled = good_upload();
        untitled.name = None;
        assert!(validate(untitled).unwrap_err().contains("title"));

        let bad_art = form(vec![
            ("notes.chart", CHART.as_bytes().to_vec()),
            ("song.ogg", vorbis(30)),
            ("album.png", b"GIF89a".to_vec()),
        ]);
        assert!(validate(bad_art).unwrap_err().contains("image"));
    }

    #[test]
    fn takes_the_title_from_song_ini() {
        let mut upload = form(vec![
            ("notes.chart", CHART.as_bytes().to_vec()),
            ("song.opus", vorbis(20)),
            (
                "song.ini",
                b"[song]\nname = From Ini\nartist = Ini Band\ndiff_guitar = 2\n".to_vec(),
            ),
        ]);
        upload.name = None;
        upload.artist = Some(" ".into());
        let validated = validate(upload).unwrap();
        assert_eq!(validated.song.info.name, "From Ini");
        assert_eq!(validated.song.info.artist, "Ini Band");
        assert!(!validated.files.iter().any(|f| f.name == "song.ini"));
    }

    #[test]
    fn cache_paths_stay_inside_the_cache() {
        let cache = FileCache::new(PathBuf::from("/tmp/x"), 1);
        assert!(cache.path("ENS-1", "song.ogg").is_some());
        assert!(cache.path("..", "song.ogg").is_none());
        assert!(cache.path("ENS-1", "../../etc/passwd").is_none());
        assert!(cache.path("ENS-1", ".hidden").is_none());
    }
}
