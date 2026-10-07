//! Téléchargements (lecture hors ligne).
//!
//! Chaque élément téléchargé a son dossier `<données>/turtlefin/downloads/<id>/` :
//! - `media.<ext>` : le fichier original (`/Items/{id}/Download`, droit « téléchargement » requis) ;
//! - `poster.jpg`, `thumb.jpg` : affiche et vignette 16:9 ;
//! - `series.jpg`, `season.jpg` (épisodes) : affiches de la série et de la saison ;
//! - `sub_<n>.<ext>` : sous-titres externes ;
//! - `info.json` : titre, sous-titre, résumé, sous-titres (écrit en dernier : sa présence signifie
//!   « téléchargement complet »).
//! Pendant le transfert, le média s'appelle `media.<ext>.part` ; un transfert interrompu (réseau
//! coupé, appli fermée) reprend là où il s'était arrêté (requête `Range`). La file d'attente est
//! gardée dans `queue.json` et reprend au lancement suivant.

use crate::i18n::{tr, trf};
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
    /// Épisode : série et saison (regroupement de l'écran Téléchargements).
    pub series_id: String,
    pub series_name: String,
    pub series_overview: String,
    pub season_id: String,
    pub season_name: String,
    pub season_index: Option<u32>,
    pub episode_index: Option<u32>,
    pub year: Option<u32>,
    pub rating: Option<f32>,
    pub runtime_secs: f64,
    /// Lecture hors ligne : position (s), vu, date de la dernière lecture (RFC 3339).
    pub position: f64,
    pub played: bool,
    pub last_played: String,
    /// Position / « vu » à renvoyer au serveur.
    pub dirty: bool,
    /// Métadonnées complètes (série, saison, année...) : faux pour un téléchargement plus ancien.
    pub meta: bool,
    /// Version des métadonnées (2 : logo, fond, classification, résumé de saison, favori).
    pub meta_v: u32,
    pub official_rating: String,
    pub season_overview: String,
    /// Favori (état du compte au téléchargement, puis changements faits sur l'appareil).
    pub favorite: bool,
}

/// Version actuelle des métadonnées enregistrées.
const META_V: u32 = 3;

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
    /// Affiche de la série (sinon celle de l'élément).
    pub fn series_poster(&self) -> PathBuf {
        let p = self.dir().join("series.jpg");
        if p.exists() { p } else { self.poster_path() }
    }
    /// Affiche de la saison (sinon celle de la série).
    pub fn season_poster(&self) -> PathBuf {
        let p = self.dir().join("season.jpg");
        if p.exists() { p } else { self.series_poster() }
    }
    /// Logo (série ou film), s'il y en a un.
    pub fn logo_path(&self) -> Option<PathBuf> {
        let p = self.dir().join("logo.png");
        p.exists().then_some(p)
    }
    /// Image de fond (série ou film), s'il y en a une.
    pub fn backdrop_path(&self) -> Option<PathBuf> {
        let p = self.dir().join("backdrop.jpg");
        p.exists().then_some(p)
    }
    pub fn thumb_path(&self) -> PathBuf {
        // Épisode : son image à lui (still.jpg), sinon la vignette.
        let still = self.dir().join("still.jpg");
        if still.exists() {
            return still;
        }
        let p = self.dir().join("thumb.jpg");
        if p.exists() { p } else { self.poster_path() }
    }
}

/// Réécrit `info.json` (position, « vu », métadonnées complétées).
pub fn save(e: &Entry) {
    if let Ok(t) = serde_json::to_string_pretty(e) {
        let _ = std::fs::write(e.dir().join("info.json"), t);
    }
}

/// Fin d'une lecture hors ligne : position et « vu » gardés pour le serveur.
pub fn record_play(id: &str, pos: f64, dur: f64, ended: bool) {
    let Some(mut e) = get(id) else { return };
    if dur > 0.0 {
        e.runtime_secs = dur;
    }
    // Comme Jellyfin : vu au-delà de 90 %, reprise effacée sous 5 % ou après la fin.
    let played = ended || (dur > 0.0 && pos >= dur * 0.9);
    e.played = e.played || played;
    e.position = if played || (dur > 0.0 && pos < dur * 0.05) { 0.0 } else { pos };
    e.last_played = now_rfc3339();
    e.dirty = true;
    save(&e);
}

/// Date et heure actuelles (UTC) au format attendu par Jellyfin.
fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Jours depuis 1970 -> date civile (algorithme de Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Renvoie au serveur les lectures faites hors ligne (position, « vu ») et les « vu » / favoris
/// décidés sur l'appareil. Rend le nombre de changements envoyés.
pub async fn sync(client: &Client) -> usize {
    let mut n = 0;
    for (id, f) in crate::config::all_flags() {
        if let (true, Some(v)) = (f.dirty_played, f.played) {
            if client.set_played(&id, v).await.is_ok() {
                crate::config::clear_dirty(&id, false);
                n += 1;
            }
        }
        if let (true, Some(v)) = (f.dirty_favorite, f.favorite) {
            if client.set_favorite(&id, v).await.is_ok() {
                crate::config::clear_dirty(&id, true);
                n += 1;
            }
        }
    }
    for mut e in list().into_iter().filter(|e| e.dirty) {
        let body = serde_json::json!({
            "PlaybackPositionTicks": (e.position * 10_000_000.0) as i64,
            "Played": e.played,
            "LastPlayedDate": e.last_played,
        });
        if client.set_user_data(&e.id, &body).await.is_ok() {
            e.dirty = false;
            save(&e);
            n += 1;
        }
    }
    n
}

/// Métadonnées et affiches de série / saison : `item` est l'élément tel que donné par le serveur.
async fn fill_meta(client: &Client, e: &mut Entry, item: &crate::api::Item) {
    e.meta = true;
    e.meta_v = META_V;
    e.official_rating = item.official_rating.clone().unwrap_or_default();
    if let Some(ud) = &item.user_data {
        e.favorite = ud.is_favorite;
    }
    let (title, subtitle) = item.titles();
    e.title = title;
    e.subtitle = subtitle;
    if let Some(o) = &item.overview {
        e.overview = o.clone();
    }
    e.series_id = item.series_id.clone().unwrap_or_default();
    e.series_name = item.series_name.clone().unwrap_or_default();
    e.season_id = item.season_id.clone().unwrap_or_default();
    e.season_name = item.season_name.clone().unwrap_or_default();
    e.season_index = item.parent_index_number;
    e.episode_index = item.index_number;
    e.year = item.production_year;
    e.rating = item.community_rating;
    if let Some(t) = item.run_time_ticks {
        e.runtime_secs = t as f64 / 1e7;
    }
    let dir = e.dir();
    if item.kind == "Episode" {
        if let Some(t) = item.image_tags.as_ref().and_then(|m| m.get("Primary")) {
            if let Some(b) = client.image_first(&[(item.id.clone(), Some(t.clone()))], "Primary", Size::Fill(400, 225)).await {
                save_bytes(&dir.join("still.jpg"), &b).await;
            }
        }
    }
    if !dir.join("poster.jpg").exists() {
        if let Some(b) = client.image_first(&item.poster_candidates(), "Primary", Size::Fill(400, 600)).await {
            save_bytes(&dir.join("poster.jpg"), &b).await;
        }
    }
    // Logo et fond : ceux de la série pour un épisode.
    let mut look = item.clone();
    if !e.series_id.is_empty() {
        if let Ok(series) = client.item(&e.series_id).await {
            e.series_overview = series.overview.clone().unwrap_or_default();
            if e.year.is_none() {
                e.year = series.production_year;
            }
            look = series;
        }
        let series_ref = [(e.series_id.clone(), item.series_primary_image_tag.clone())];
        if let Some(b) = client.image_first(&series_ref, "Primary", Size::Fill(400, 600)).await {
            save_bytes(&dir.join("series.jpg"), &b).await;
        }
    }
    if !e.season_id.is_empty() {
        if let Some(b) = client.image_first(&[(e.season_id.clone(), None)], "Primary", Size::Fill(400, 600)).await {
            save_bytes(&dir.join("season.jpg"), &b).await;
        }
        if let Ok(season) = client.item(&e.season_id).await {
            e.season_overview = season.overview.clone().unwrap_or_default();
        }
    }
    if let Some(logo) = look.logo_candidate().or_else(|| item.logo_candidate()) {
        if let Some(b) = client.image_first(&[logo], "Logo", Size::MaxWidth(500)).await {
            save_bytes(&dir.join("logo.png"), &b).await;
        }
    }
    if let Some(b) = client.backdrop(&look).await {
        save_bytes(&dir.join("backdrop.jpg"), &b).await;
    }
}

/// Téléchargement d'avant le regroupement par série : métadonnées complétées (une fois).
pub async fn enrich(client: &Client, id: &str) -> bool {
    let Some(mut e) = get(id) else { return false };
    if e.meta_v >= META_V {
        return false;
    }
    let Ok(item) = client.item(id).await else { return false };
    fill_meta(client, &mut e, &item).await;
    save(&e);
    true
}

/// Le serveur refuse le téléchargement (droit retiré, élément supprimé...) : réessayer ne sert à rien.
#[derive(Debug)]
pub struct Refused(pub String);
impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Refused {}

/// File d'attente gardée sur le disque : (id, titre), l'élément en cours en premier.
pub fn save_queue(q: &[(String, String)]) {
    let _ = std::fs::create_dir_all(root());
    let _ = std::fs::write(root().join("queue.json"), serde_json::to_string(q).unwrap_or_default());
}

pub fn load_queue() -> Vec<(String, String)> {
    std::fs::read_to_string(root().join("queue.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn root() -> PathBuf {
    crate::paths::data_dir()
        .map(|d| d.join("downloads"))
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
    // Transfert interrompu : on reprend après ce qui est déjà sur le disque.
    let have = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
    let mut req = http.get(client.download_url(&item.id)).header("Authorization", client.auth());
    if have > 0 {
        req = req.header("Range", format!("bytes={have}-"));
    }
    let mut resp = req.send().await?;
    let status = resp.status().as_u16();
    use tokio::io::AsyncWriteExt;
    let (mut file, mut done, total) = match status {
        206 => {
            let f = tokio::fs::OpenOptions::new().append(true).open(&part).await?;
            (Some(f), have, have + resp.content_length().unwrap_or(0))
        }
        // Le fichier provisoire était déjà complet.
        416 if have > 0 => (None, have, have),
        s if (200..300).contains(&s) => {
            let total = resp.content_length().unwrap_or(0);
            (Some(tokio::fs::File::create(&part).await?), 0, total)
        }
        s => return Err(Refused(trf("le serveur refuse le téléchargement ({})", &[&s])).into()),
    };
    if let Some(f) = file.as_mut() {
        if total > 0 {
            progress(done as f32 / total as f32);
        }
        let mut last = std::time::Instant::now();
        loop {
            // Réseau muet (connexion ouverte mais plus rien ne passe) : on n'attend pas indéfiniment.
            let chunk = match tokio::time::timeout(Duration::from_secs(30), resp.chunk()).await {
                Ok(c) => c?,
                Err(_) => return Err(anyhow!("{}", tr("plus aucune donnée reçue depuis 30 s"))),
            };
            let Some(chunk) = chunk else { break };
            f.write_all(&chunk).await?;
            done += chunk.len() as u64;
            if total > 0 && last.elapsed() > Duration::from_millis(500) {
                last = std::time::Instant::now();
                progress(done as f32 / total as f32);
            }
        }
        f.flush().await?;
    }
    drop(file);
    if total > 0 && done < total {
        return Err(anyhow!("{}", trf("transfert incomplet ({} octets sur {})", &[&done, &total])));
    }
    tokio::fs::rename(&part, dir.join(&media)).await?;

    let (title, subtitle) = item.titles();
    let mut entry = Entry {
        id: item.id.clone(),
        kind: item.kind.clone(),
        title,
        subtitle,
        overview: item.overview.clone().unwrap_or_default(),
        media,
        subs,
        size: done,
        ..Default::default()
    };
    if let Some(ud) = &item.user_data {
        entry.position = ud.playback_position_ticks as f64 / 1e7;
        entry.played = ud.played;
    }
    fill_meta(client, &mut entry, &item).await;
    tokio::fs::write(dir.join("info.json"), serde_json::to_string_pretty(&entry)?).await?;
    progress(1.0);
    Ok(())
}
