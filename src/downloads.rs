//! Téléchargements (lecture hors ligne).
//!
//! Chaque élément téléchargé a son dossier `<données>/turtlefin/downloads/<id>/` :
//! - `media.<ext>` : le fichier original (`/Items/{id}/Download`, droit « téléchargement » requis) ;
//! - `poster.jpg`, `thumb.jpg` : affiche et vignette 16:9 ;
//! - `sub_<n>.<ext>` : sous-titres externes ;
//! - `info.json` : titre, sous-titre, résumé, sous-titres (écrit en dernier : sa présence signifie
//!   « téléchargement complet »).
//! Pendant le transfert, le média s'appelle `media.<ext>.part`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::api::{Client, Size};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub overview: String,
    /// Nom du fichier média dans le dossier de l'élément.
    pub media: String,
    /// Sous-titres externes : (nom de fichier, langue).
    pub subs: Vec<(String, String)>,
    pub size: u64,
}

impl Entry {
    pub fn dir(&self) -> PathBuf {
        root().join(&self.id)
    }
    pub fn media_path(&self) -> PathBuf {
        self.dir().join(&self.media)
    }
    pub fn poster_path(&self) -> PathBuf {
        self.dir().join("poster.jpg")
    }
}

pub fn root() -> PathBuf {
    directories::ProjectDirs::from("", "", "turtlefin")
        .map(|d| d.data_dir().join("downloads"))
        .unwrap_or_else(|| PathBuf::from("downloads"))
}

/// Éléments complètement téléchargés, du plus récent au plus ancien.
pub fn list() -> Vec<Entry> {
    let mut out: Vec<(std::time::SystemTime, Entry)> = Vec::new();
    let Ok(dirs) = std::fs::read_dir(root()) else { return Vec::new() };
    for d in dirs.flatten() {
        let info = d.path().join("info.json");
        let Ok(text) = std::fs::read_to_string(&info) else { continue };
        let Ok(e) = serde_json::from_str::<Entry>(&text) else { continue };
        if !e.media_path().exists() {
            continue;
        }
        let t = std::fs::metadata(&info).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        out.push((t, e));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, e)| e).collect()
}

pub fn get(id: &str) -> Option<Entry> {
    let text = std::fs::read_to_string(root().join(id).join("info.json")).ok()?;
    let e: Entry = serde_json::from_str(&text).ok()?;
    e.media_path().exists().then_some(e)
}

pub fn exists(id: &str) -> bool {
    get(id).is_some()
}

pub fn remove(id: &str) {
    if !id.is_empty() && !id.contains(['/', '\\', '.']) {
        let _ = std::fs::remove_dir_all(root().join(id));
    }
}

async fn save_bytes(path: &Path, bytes: &[u8]) {
    let _ = tokio::fs::write(path, bytes).await;
}

/// Télécharge un film / un épisode. `progress(fraction)` est appelé au fil du transfert.
pub async fn download(client: &Client, id: &str, progress: impl Fn(f32)) -> Result<()> {
    if exists(id) {
        return Ok(());
    }
    let item = client.item(id).await?;
    let dir = root().join(id);
    tokio::fs::create_dir_all(&dir).await?;

    // Affiche et vignette (le cache d'images de Jellyfin sert aussi ici).
    if let Some(b) = client.image_first(&item.poster_candidates(), "Primary", Size::Fill(400, 600)).await {
        save_bytes(&dir.join("poster.jpg"), &b).await;
    }
    let card = item.card();
    if let Some(b) = client.image_any(&card.thumbs, Size::Fill(400, 225)).await {
        save_bytes(&dir.join("thumb.jpg"), &b).await;
    }

    // Sous-titres externes.
    let source = item.media_sources.as_ref().and_then(|v| v.first());
    let ms_id = source.map(|m| m.id.clone()).unwrap_or_else(|| item.id.clone());
    let mut subs: Vec<(String, String)> = Vec::new();
    let http = reqwest::Client::builder().connect_timeout(Duration::from_secs(15)).build()?;
    for (n, st) in source.into_iter().flat_map(|s| s.media_streams.iter()).filter(|s| s.kind == "Subtitle" && s.is_external).enumerate() {
        let Some(url) = client.subtitle_url(&item.id, &ms_id, st) else { continue };
        let ext = url.rsplit('.').next().and_then(|e| e.split('?').next()).unwrap_or("srt").to_string();
        if let Ok(r) = http.get(&url).send().await {
            if let Ok(b) = r.bytes().await {
                let name = format!("sub_{n}.{ext}");
                save_bytes(&dir.join(&name), &b).await;
                subs.push((name, st.language.clone().unwrap_or_default()));
            }
        }
    }

    // Le média, par morceaux, dans un fichier provisoire.
    let ext = source
        .and_then(|s| s.container.clone())
        .and_then(|c| c.split(',').next().map(str::to_string))
        .filter(|c| !c.is_empty() && c.chars().all(|ch| ch.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "mkv".to_string());
    let media = format!("media.{ext}");
    let part = dir.join(format!("{media}.part"));
    let mut resp = http.get(client.download_url(&item.id)).header("Authorization", client.auth()).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("le serveur refuse le téléchargement ({})", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&part).await?;
    let mut done: u64 = 0;
    let mut last = std::time::Instant::now();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        if total > 0 && last.elapsed() > Duration::from_millis(500) {
            last = std::time::Instant::now();
            progress(done as f32 / total as f32);
        }
    }
    file.flush().await?;
    drop(file);
    tokio::fs::rename(&part, dir.join(&media)).await?;

    let (title, subtitle) = item.titles();
    let entry = Entry {
        id: item.id.clone(),
        kind: item.kind.clone(),
        title,
        subtitle,
        overview: item.overview.clone().unwrap_or_default(),
        media,
        subs,
        size: done,
    };
    tokio::fs::write(dir.join("info.json"), serde_json::to_string_pretty(&entry)?).await?;
    progress(1.0);
    Ok(())
}
