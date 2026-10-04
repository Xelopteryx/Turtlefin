//! Client minimal pour l'API Jellyfin 10.11 (REST).

use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};

use crate::config::Saved;

/// Erreur dédiée : le jeton est refusé (401).
#[derive(Debug)]
pub struct Unauthorized;

impl std::fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session expirée ou identifiants refusés")
    }
}
impl std::error::Error for Unauthorized {}

/// Taille d'image demandée au serveur.
#[derive(Clone, Copy, Debug)]
pub enum Size {
    /// Remplit exactement largeur x hauteur (recadrage côté serveur).
    Fill(u32, u32),
    /// Largeur maximale, proportions conservées (logos).
    MaxWidth(u32),
}

// ---------------------------------------------------------------------------
// Types JSON
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthResult {
    access_token: String,
    user: AuthUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthUser {
    id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemsResp {
    items: Vec<Item>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct UserData {
    pub playback_position_ticks: i64,
    pub played: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct MediaStream {
    pub index: i32,
    #[serde(rename = "Type")]
    pub kind: String,
    pub codec: Option<String>,
    pub language: Option<String>,
    pub is_external: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct MediaSource {
    pub id: String,
    pub media_streams: Vec<MediaStream>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(rename = "Type")]
    pub kind: String,
    pub is_folder: bool,
    pub collection_type: Option<String>,
    pub overview: Option<String>,
    pub official_rating: Option<String>,
    pub community_rating: Option<f32>,
    pub run_time_ticks: Option<i64>,
    pub production_year: Option<u32>,
    pub index_number: Option<u32>,
    pub parent_index_number: Option<u32>,
    pub series_id: Option<String>,
    pub series_name: Option<String>,
    pub series_primary_image_tag: Option<String>,
    pub season_id: Option<String>,
    pub season_name: Option<String>,
    pub parent_logo_item_id: Option<String>,
    pub parent_logo_image_tag: Option<String>,
    pub image_tags: Option<HashMap<String, String>>,
    pub user_data: Option<UserData>,
    pub media_sources: Option<Vec<MediaSource>>,
}

/// Ce dont l'interface a besoin pour afficher une carte.
#[derive(Clone, Debug)]
pub struct CardInfo {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub img_id: Option<String>,
    pub img_tag: Option<String>,
}

/// (id de l'élément portant l'image, étiquette de version de l'image)
pub type ImageRef = (String, Option<String>);

impl Item {
    fn primary_tag(&self) -> Option<String> {
        self.image_tags.as_ref()?.get("Primary").cloned()
    }

    fn minutes(&self) -> Option<i64> {
        self.run_time_ticks.filter(|t| *t > 0).map(|t| t / 600_000_000)
    }

    /// Carte « poster » : les épisodes affichent le poster de la série.
    pub fn card(&self) -> CardInfo {
        let is_episode = self.kind == "Episode";

        let title = if is_episode {
            self.series_name.clone().unwrap_or_else(|| self.name.clone())
        } else {
            self.name.clone()
        };

        let subtitle = if is_episode {
            match (self.parent_index_number, self.index_number) {
                (Some(s), Some(e)) => format!("S{s}E{e} · {}", self.name),
                _ => self.name.clone(),
            }
        } else {
            self.production_year.map(|y| y.to_string()).unwrap_or_default()
        };

        let (img_id, img_tag) = match (is_episode, &self.series_id) {
            (true, Some(sid)) => (Some(sid.clone()), self.series_primary_image_tag.clone()),
            _ => {
                let tag = self.primary_tag();
                (tag.as_ref().map(|_| self.id.clone()), tag)
            }
        };

        CardInfo { id: self.id.clone(), title, subtitle, img_id, img_tag }
    }

    /// Carte pour la rangée « enfants » d'une fiche (saisons, épisodes, contenu).
    /// Un épisode affiche sa propre vignette 16:9.
    pub fn child_card(&self) -> CardInfo {
        if self.kind != "Episode" {
            return self.card();
        }
        let title = match self.index_number {
            Some(n) => format!("{n}. {}", self.name),
            None => self.name.clone(),
        };
        let mut parts: Vec<String> = Vec::new();
        if let Some(m) = self.minutes() {
            parts.push(format!("{m} min"));
        }
        if self.user_data.as_ref().map(|u| u.played).unwrap_or(false) {
            parts.push("vu".to_string());
        }
        let tag = self.primary_tag();
        CardInfo {
            id: self.id.clone(),
            title,
            subtitle: parts.join(" · "),
            img_id: tag.as_ref().map(|_| self.id.clone()),
            img_tag: tag,
        }
    }

    /// (titre, sous-titre) de l'en-tête de fiche.
    pub fn titles(&self) -> (String, String) {
        match self.kind.as_str() {
            "Episode" => {
                let title = self.series_name.clone().unwrap_or_else(|| self.name.clone());
                let sub = match (self.parent_index_number, self.index_number) {
                    (Some(s), Some(e)) => format!("S{s}E{e} · {}", self.name),
                    _ => self.name.clone(),
                };
                (title, sub)
            }
            "Season" => (self.series_name.clone().unwrap_or_else(|| self.name.clone()), self.name.clone()),
            _ => (self.name.clone(), String::new()),
        }
    }

    /// Langues audio de la première source (« FR », « JA »...).
    pub fn audio_langs(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let Some(src) = self.media_sources.as_ref().and_then(|v| v.first()) else { return out };
        for st in src.media_streams.iter().filter(|s| s.kind == "Audio") {
            let Some(code) = st.language.as_deref() else { continue };
            let label = match code {
                "fre" | "fra" => "FR".to_string(),
                "eng" => "EN".to_string(),
                "jpn" => "JA".to_string(),
                "ger" | "deu" => "DE".to_string(),
                "spa" => "ES".to_string(),
                "ita" => "IT".to_string(),
                "por" => "PT".to_string(),
                "kor" => "KO".to_string(),
                "chi" | "zho" => "ZH".to_string(),
                "rus" => "RU".to_string(),
                other => other.to_uppercase(),
            };
            if !out.contains(&label) {
                out.push(label);
            }
        }
        out
    }

    /// Année · classification · durée · note · langues audio
    pub fn misc_line(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(y) = self.production_year {
            parts.push(y.to_string());
        }
        if let Some(r) = self.official_rating.as_ref().filter(|r| !r.is_empty()) {
            parts.push(r.clone());
        }
        if let Some(m) = self.minutes().filter(|m| *m > 0) {
            parts.push(if m >= 60 { format!("{}h{:02}", m / 60, m % 60) } else { format!("{m} min") });
        }
        if let Some(c) = self.community_rating {
            parts.push(format!("{c:.1}/10"));
        }
        let langs = self.audio_langs();
        if !langs.is_empty() {
            parts.push(format!("Audio : {}", langs.join(", ")));
        }
        parts.join("  ·  ")
    }

    /// Boutons de la fiche : (libellé, action). Actions : "play", "back", "open:<id>".
    pub fn buttons(&self) -> Vec<(String, String)> {
        let mut b: Vec<(String, String)> = Vec::new();
        if matches!(self.kind.as_str(), "Movie" | "Episode" | "Series" | "Season") {
            let resume = self
                .user_data
                .as_ref()
                .map(|u| u.playback_position_ticks > 0)
                .unwrap_or(false);
            b.push((if resume { "Reprendre" } else { "Lecture" }.to_string(), "play".to_string()));
        }
        if self.kind == "Episode" {
            if let Some(id) = &self.season_id {
                b.push(("Voir la saison".to_string(), format!("open:{id}")));
            }
        }
        if matches!(self.kind.as_str(), "Episode" | "Season") {
            if let Some(id) = &self.series_id {
                b.push(("Voir la série".to_string(), format!("open:{id}")));
            }
        }
        b.push(("Retour".to_string(), "back".to_string()));
        b
    }

    /// Poster de la fiche, par ordre de préférence. Un épisode affiche le poster
    /// de sa saison (comme ton script Portrait_Poster.js), sinon celui de la série.
    pub fn poster_candidates(&self) -> Vec<ImageRef> {
        let mut v: Vec<ImageRef> = Vec::new();
        if self.kind == "Episode" {
            if let Some(sid) = &self.season_id {
                v.push((sid.clone(), None));
            }
            if let Some(sid) = &self.series_id {
                v.push((sid.clone(), self.series_primary_image_tag.clone()));
            }
            if let Some(t) = self.primary_tag() {
                v.push((self.id.clone(), Some(t)));
            }
        } else {
            if let Some(t) = self.primary_tag() {
                v.push((self.id.clone(), Some(t)));
            }
            if let Some(sid) = &self.series_id {
                v.push((sid.clone(), self.series_primary_image_tag.clone()));
            }
        }
        v
    }

    /// Logo : le sien, sinon celui du parent (série).
    pub fn logo_candidate(&self) -> Option<ImageRef> {
        if let Some(t) = self.image_tags.as_ref().and_then(|m| m.get("Logo")) {
            return Some((self.id.clone(), Some(t.clone())));
        }
        match (&self.parent_logo_item_id, &self.parent_logo_image_tag) {
            (Some(id), Some(tag)) => Some((id.clone(), Some(tag.clone()))),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    pub server: String,
    pub token: String,
    pub user_id: String,
    pub user_name: String,
    pub device_id: String,
}

fn normalize_server(s: &str) -> String {
    let s = s.trim().trim_end_matches('/');
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else {
        format!("http://{s}")
    }
}

fn auth_header(device_id: &str, token: Option<&str>) -> String {
    let mut h = format!(
        "MediaBrowser Client=\"Turtlefin\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\"",
        std::env::consts::OS,
        device_id,
        env!("CARGO_PKG_VERSION")
    );
    if let Some(t) = token {
        h.push_str(&format!(", Token=\"{t}\""));
    }
    h
}

fn build_http() -> Result<reqwest::Client> {
    let mut b = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("Turtlefin/", env!("CARGO_PKG_VERSION")));
    // À n'utiliser qu'en test : TURTLEFIN_INSECURE=1 accepte tout certificat.
    if std::env::var("TURTLEFIN_INSECURE").is_ok() {
        b = b.danger_accept_invalid_certs(true);
    }
    Ok(b.build()?)
}

fn image_cache_path(item_id: &str, kind: &str, tag: Option<&str>, size_key: &str) -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "turtlefin").map(|d| {
        d.cache_dir()
            .join("img")
            .join(format!("{item_id}_{kind}_{}_{size_key}.bin", tag.unwrap_or("none")))
    })
}

impl Client {
    pub fn from_saved(s: &Saved) -> Result<Self> {
        Ok(Self {
            http: build_http()?,
            server: normalize_server(&s.server),
            token: s.token.clone(),
            user_id: s.user_id.clone(),
            user_name: s.user_name.clone(),
            device_id: s.device_id.clone(),
        })
    }

    pub fn to_saved(&self) -> Saved {
        Saved {
            server: self.server.clone(),
            user_name: self.user_name.clone(),
            user_id: self.user_id.clone(),
            token: self.token.clone(),
            device_id: self.device_id.clone(),
        }
    }

    pub async fn login(server: &str, user: &str, pw: &str, device_id: &str) -> Result<Self> {
        let server = normalize_server(server);
        let http = build_http()?;
        let resp = http
            .post(format!("{server}/Users/AuthenticateByName"))
            .header("Authorization", auth_header(device_id, None))
            .json(&serde_json::json!({ "Username": user, "Pw": pw }))
            .send()
            .await
            .map_err(|e| anyhow!("Impossible de joindre {server} : {e}"))?;

        let status = resp.status();
        if status.as_u16() == 401 {
            return Err(anyhow!("Identifiant ou mot de passe incorrect"));
        }
        if !status.is_success() {
            return Err(anyhow!("Le serveur a répondu {status}"));
        }
        let r: AuthResult = resp.json().await?;
        Ok(Self {
            http,
            server,
            token: r.access_token,
            user_id: r.user.id,
            user_name: r.user.name,
            device_id: device_id.to_string(),
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let resp = self
            .http
            .get(format!("{}{}", self.server, path))
            .query(query)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        match resp.status().as_u16() {
            401 => Err(Unauthorized.into()),
            c if !(200..300).contains(&c) => Err(anyhow!("Le serveur a répondu {c} sur {path}")),
            _ => Ok(resp.json::<T>().await?),
        }
    }

    /// Bibliothèques de l'utilisateur.
    pub async fn views(&self) -> Result<Vec<Item>> {
        let r: ItemsResp = self.get("/UserViews", &[("userId", &self.user_id)]).await?;
        Ok(r.items)
    }

    /// « Reprendre la lecture ».
    pub async fn resume(&self) -> Result<Vec<Item>> {
        let r: ItemsResp = self
            .get(
                "/UserItems/Resume",
                &[("userId", &self.user_id), ("Limit", "16"), ("MediaTypes", "Video")],
            )
            .await?;
        Ok(r.items)
    }

    /// « À suivre ».
    pub async fn next_up(&self) -> Result<Vec<Item>> {
        let r: ItemsResp = self
            .get("/Shows/NextUp", &[("userId", &self.user_id), ("Limit", "16")])
            .await?;
        Ok(r.items)
    }

    /// Derniers ajouts d'une bibliothèque (cet endpoint renvoie un tableau directement).
    pub async fn latest(&self, parent_id: &str) -> Result<Vec<Item>> {
        self.get(
            "/Items/Latest",
            &[("userId", &self.user_id), ("parentId", parent_id), ("Limit", "16")],
        )
        .await
    }

    /// Un élément avec ses métadonnées (résumé, note, durée, état de lecture...).
    pub async fn item(&self, id: &str) -> Result<Item> {
        match self.get::<Item>(&format!("/Items/{id}"), &[("userId", &self.user_id)]).await {
            Ok(i) => Ok(i),
            Err(e) if e.downcast_ref::<Unauthorized>().is_some() => Err(e),
            // Ancienne route, au cas où.
            Err(_) => self.get(&format!("/Users/{}/Items/{id}", self.user_id), &[]).await,
        }
    }

    /// Enfants directs d'un dossier/série/saison.
    pub async fn children(&self, parent_id: &str, sort_by: &str, limit: u32) -> Result<Vec<Item>> {
        let limit = limit.to_string();
        let r: ItemsResp = self
            .get(
                "/Items",
                &[
                    ("userId", &self.user_id),
                    ("parentId", parent_id),
                    ("SortBy", sort_by),
                    ("SortOrder", "Ascending"),
                    ("Limit", &limit),
                ],
            )
            .await?;
        Ok(r.items)
    }

    /// Prochain épisode à regarder d'une série.
    pub async fn next_up_for(&self, series_id: &str) -> Result<Option<Item>> {
        let r: ItemsResp = self
            .get(
                "/Shows/NextUp",
                &[("userId", &self.user_id), ("seriesId", series_id), ("Limit", "1")],
            )
            .await?;
        Ok(r.items.into_iter().next())
    }

    /// Épisodes d'une série, éventuellement d'une seule saison.
    pub async fn episodes(&self, series_id: &str, season_id: Option<&str>) -> Result<Vec<Item>> {
        let mut q: Vec<(&str, &str)> = vec![("userId", &self.user_id)];
        if let Some(s) = season_id {
            q.push(("seasonId", s));
        }
        let r: ItemsResp = self.get(&format!("/Shows/{series_id}/Episodes"), &q).await?;
        Ok(r.items)
    }

    /// Flux brut (lecture directe) lu par mpv.
    pub fn stream_url(&self, item_id: &str, media_source_id: &str) -> String {
        format!(
            "{}/Videos/{}/stream?static=true&mediaSourceId={}&deviceId={}&api_key={}",
            self.server, item_id, media_source_id, self.device_id, self.token
        )
    }

    /// Sous-titre externe, si son format est connu de mpv.
    pub fn subtitle_url(&self, item_id: &str, media_source_id: &str, st: &MediaStream) -> Option<String> {
        let fmt = match st.codec.as_deref()?.to_lowercase().as_str() {
            "srt" | "subrip" => "srt",
            "ass" | "ssa" => "ass",
            "vtt" | "webvtt" => "vtt",
            _ => return None,
        };
        Some(format!(
            "{}/Videos/{}/{}/Subtitles/{}/Stream.{}?api_key={}",
            self.server, item_id, media_source_id, st.index, fmt, self.token
        ))
    }

    /// Rapport de lecture au serveur : "/Sessions/Playing", ".../Progress" ou ".../Stopped".
    pub async fn report(&self, endpoint: &str, body: &serde_json::Value) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}{}", self.server, endpoint))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(body)
            .send()
            .await?;
        if resp.status().as_u16() == 401 {
            return Err(Unauthorized.into());
        }
        Ok(())
    }

    /// Image (Primary, Logo...) avec cache disque.
    pub async fn image(&self, item_id: &str, kind: &str, tag: Option<&str>, size: Size) -> Result<Vec<u8>> {
        let size_key = match size {
            Size::Fill(w, h) => format!("f{w}x{h}"),
            Size::MaxWidth(w) => format!("m{w}"),
        };
        let cache = image_cache_path(item_id, kind, tag, &size_key);
        if let Some(p) = &cache {
            if let Ok(b) = tokio::fs::read(p).await {
                return Ok(b);
            }
        }

        let mut q: Vec<(String, String)> = vec![("quality".into(), "85".into())];
        match size {
            Size::Fill(w, h) => {
                q.push(("fillWidth".into(), w.to_string()));
                q.push(("fillHeight".into(), h.to_string()));
            }
            Size::MaxWidth(w) => q.push(("maxWidth".into(), w.to_string())),
        }
        if let Some(t) = tag {
            q.push(("tag".into(), t.to_string()));
        }

        let resp = self
            .http
            .get(format!("{}/Items/{}/Images/{}", self.server, item_id, kind))
            .query(&q)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("image {} ({}) : {}", item_id, kind, resp.status()));
        }
        let bytes = resp.bytes().await?.to_vec();

        if let Some(p) = &cache {
            if let Some(dir) = p.parent() {
                let _ = tokio::fs::create_dir_all(dir).await;
            }
            let _ = tokio::fs::write(p, &bytes).await;
        }
        Ok(bytes)
    }

    /// Première image disponible parmi plusieurs candidats.
    pub async fn image_first(&self, candidates: &[ImageRef], kind: &str, size: Size) -> Option<Vec<u8>> {
        for (id, tag) in candidates {
            if let Ok(b) = self.image(id, kind, tag.as_deref(), size).await {
                return Some(b);
            }
        }
        None
    }
}
