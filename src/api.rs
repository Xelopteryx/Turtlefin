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
    #[serde(default)]
    total_record_count: Option<u32>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct UserData {
    pub playback_position_ticks: i64,
    pub played: bool,
    /// Avancement de la lecture en cours (0-100), absent si rien n'est commencé.
    pub played_percentage: Option<f64>,
    pub is_favorite: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    #[serde(rename = "Type")]
    pub kind: Option<String>,
    pub primary_image_tag: Option<String>,
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
    /// Format du fichier (« mkv », « mov,mp4,m4a »...).
    pub container: Option<String>,
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
    pub backdrop_image_tags: Option<Vec<String>>,
    pub parent_thumb_item_id: Option<String>,
    pub parent_thumb_image_tag: Option<String>,
    pub parent_backdrop_item_id: Option<String>,
    pub parent_backdrop_image_tags: Option<Vec<String>>,
    pub user_data: Option<UserData>,
    pub media_sources: Option<Vec<MediaSource>>,
    pub people: Option<Vec<Person>>,
    pub provider_ids: Option<HashMap<String, String>>,
}

/// Ce dont l'interface a besoin pour afficher une carte.
#[derive(Clone, Debug)]
pub struct CardInfo {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub img_id: Option<String>,
    pub img_tag: Option<String>,
    /// Images pour une carte paysage (16:9), par ordre de préférence.
    pub thumbs: Vec<ImgCand>,
    /// Avancement de la lecture (0..1), 0 si rien n'est commencé.
    pub progress: f32,
    /// Note de la communauté (« 7.7 »), vide si absente.
    pub rating: String,
}

/// (id de l'élément portant l'image, étiquette de version de l'image)
pub type ImageRef = (String, Option<String>);

/// (id de l'élément, type d'image : "Primary" / "Thumb" / "Backdrop", étiquette)
/// Type "Url" : `id` est une adresse complète (images hors Jellyfin).
pub type ImgCand = (String, &'static str, Option<String>);

/// Une demande Seerr de l'utilisateur.
#[derive(Clone, Debug)]
pub struct SeerrRequest {
    /// 1 en attente · 2 acceptée · 3 refusée · 4 en échec · 5 terminée
    pub status: i64,
    pub tv: bool,
    pub title: String,
    pub year: String,
    pub poster: Option<String>,
    /// Date de la demande (ISO 8601).
    pub created: String,
    /// Élément Jellyfin correspondant, une fois le média disponible.
    pub jellyfin_id: Option<String>,
    pub seasons: Vec<i64>,
}

impl SeerrRequest {
    pub fn card(&self) -> CardInfo {
        let mut parts: Vec<String> = vec![if self.tv { "Série" } else { "Film" }.to_string()];
        if !self.year.is_empty() {
            parts.push(self.year.clone());
        }
        if self.tv && !self.seasons.is_empty() {
            let s: Vec<String> = self.seasons.iter().map(|n| n.to_string()).collect();
            parts.push(format!("S{}", s.join(", ")));
        }
        // Demandé le JJ/MM
        if let (Some(m), Some(d)) = (self.created.get(5..7), self.created.get(8..10)) {
            parts.push(format!("{d}/{m}"));
        }
        CardInfo {
            // Disponible : on ouvre sa fiche Jellyfin ; sinon la carte ne mène nulle part.
            id: self.jellyfin_id.clone().unwrap_or_default(),
            title: self.title.clone(),
            subtitle: parts.join(" · "),
            img_id: self.poster.clone(),
            img_tag: None,
            thumbs: Vec::new(),
            progress: 0.0,
            rating: String::new(),
        }
    }
}

impl Item {
    fn primary_tag(&self) -> Option<String> {
        self.image_tags.as_ref()?.get("Primary").cloned()
    }

    /// Images 16:9 : vignette (Thumb) de la série pour un épisode, comme le client web,
    /// sinon image propre, puis fond (Backdrop).
    pub fn landscape_candidates(&self) -> Vec<ImgCand> {
        let mut v: Vec<ImgCand> = Vec::new();
        let own = |kind: &'static str| -> Option<ImgCand> {
            let tag = self.image_tags.as_ref()?.get(kind)?.clone();
            Some((self.id.clone(), kind, Some(tag)))
        };
        if self.kind == "Episode" {
            if let (Some(id), Some(tag)) = (&self.parent_thumb_item_id, &self.parent_thumb_image_tag) {
                v.push((id.clone(), "Thumb", Some(tag.clone())));
            }
            v.extend(own("Primary"));
        } else {
            v.extend(own("Thumb"));
        }
        if let Some(tag) = self.backdrop_image_tags.as_ref().and_then(|t| t.first()) {
            v.push((self.id.clone(), "Backdrop", Some(tag.clone())));
        } else if let (Some(id), Some(tag)) = (
            &self.parent_backdrop_item_id,
            self.parent_backdrop_image_tags.as_ref().and_then(|t| t.first()),
        ) {
            v.push((id.clone(), "Backdrop", Some(tag.clone())));
        }
        if self.kind != "Episode" {
            v.extend(own("Primary"));
        }
        v
    }

    fn progress(&self) -> f32 {
        let u = self.user_data.as_ref();
        match u.and_then(|u| u.played_percentage) {
            Some(p) if !u.map(|u| u.played).unwrap_or(false) => (p / 100.0).clamp(0.0, 1.0) as f32,
            _ => 0.0,
        }
    }

    fn rating(&self) -> String {
        self.community_rating.filter(|r| *r > 0.0).map(|r| format!("{r:.1}")).unwrap_or_default()
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

        CardInfo {
            id: self.id.clone(),
            title,
            subtitle,
            img_id,
            img_tag,
            thumbs: self.landscape_candidates(),
            progress: self.progress(),
            rating: self.rating(),
        }
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
            img_tag: tag.clone(),
            // Un épisode dans sa saison : sa propre image 16:9 d'abord.
            thumbs: tag.map(|t| (self.id.clone(), "Primary", Some(t))).into_iter().collect(),
            progress: self.progress(),
            rating: String::new(),
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

    /// Boutons de la fiche : (libellé, action, icône, actif). Les boutons à icône n'ont pas de
    /// libellé. Actions : "play", "fav", "played", "download", "open:<id>".
    pub fn buttons(&self, can_download: bool) -> Vec<(String, String, String, bool)> {
        let mut b: Vec<(String, String, String, bool)> = Vec::new();
        let ud = self.user_data.clone().unwrap_or_default();
        let playable = matches!(self.kind.as_str(), "Movie" | "Episode" | "Series" | "Season" | "Video" | "MusicVideo");
        if playable {
            b.push((String::new(), "play".into(), "play".into(), false));
            b.push((String::new(), "fav".into(), "heart".into(), ud.is_favorite));
            b.push((String::new(), "played".into(), "check".into(), ud.played));
            if can_download {
                b.push((String::new(), "download".into(), "download".into(), false));
            }
        }
        if self.kind == "Episode" {
            if let Some(id) = &self.season_id {
                b.push(("Voir la saison".into(), format!("open:{id}"), String::new(), false));
            }
        }
        if matches!(self.kind.as_str(), "Episode" | "Season") {
            if let Some(id) = &self.series_id {
                b.push(("Voir la série".into(), format!("open:{id}"), String::new(), false));
            }
        }
        b
    }

    /// Casting et équipe : cartes portrait (photo, nom, rôle).
    pub fn people_cards(&self) -> Vec<CardInfo> {
        let mut out = Vec::new();
        for p in self.people.iter().flatten().take(30) {
            let role = match (p.kind.as_deref(), p.role.as_deref().filter(|r| !r.is_empty())) {
                (Some("Actor") | Some("GuestStar"), Some(r)) => r.to_string(),
                (Some("Actor"), None) => "Acteur".into(),
                (Some("GuestStar"), None) => "Invité".into(),
                (Some("Director"), _) => "Réalisation".into(),
                (Some("Writer"), _) => "Scénario".into(),
                (Some("Producer"), _) => "Production".into(),
                (Some("Composer"), _) => "Musique".into(),
                (_, Some(r)) => r.to_string(),
                _ => String::new(),
            };
            out.push(CardInfo {
                // Pas encore de page « personne » : la carte ne mène nulle part.
                id: format!("person:{}", p.id),
                title: p.name.clone(),
                subtitle: role,
                img_id: p.primary_image_tag.as_ref().map(|_| p.id.clone()),
                img_tag: p.primary_image_tag.clone(),
                thumbs: Vec::new(),
                progress: 0.0,
                rating: String::new(),
            });
        }
        out
    }

    /// Identifiant TMDB (pour Seerr).
    pub fn tmdb(&self) -> Option<i64> {
        self.provider_ids.as_ref()?.get("Tmdb")?.parse().ok()
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
            ..Default::default()
        }
    }

    /// Change l'adresse utilisée (même serveur joint autrement : local / distant).
    pub fn set_server(&mut self, server: &str) {
        self.server = normalize_server(server);
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
            .get("/Shows/NextUp", &[("userId", &self.user_id), ("Limit", "16"), ("enableResumable", "false")])
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

    /// Favoris de l'utilisateur (films, séries, saisons, épisodes...).
    pub async fn favorites(&self) -> Result<Vec<Item>> {
        let r: ItemsResp = self
            .get(
                "/Items",
                &[
                    ("userId", &self.user_id),
                    ("Recursive", "true"),
                    ("Filters", "IsFavorite"),
                    ("IncludeItemTypes", "Movie,Series,Season,Episode,BoxSet,Video,MusicVideo"),
                    ("SortBy", "SortName"),
                    ("SortOrder", "Ascending"),
                    ("Limit", "200"),
                ],
            )
            .await?;
        Ok(r.items)
    }

    /// Une page du contenu d'une bibliothèque ou d'une collection, et le nombre total d'éléments.
    /// Films et séries : recherche récursive par type, comme le client web.
    pub async fn library_page(
        &self,
        parent_id: &str,
        collection_type: Option<&str>,
        start: u32,
        limit: u32,
    ) -> Result<(Vec<Item>, u32)> {
        let (start, limit) = (start.to_string(), limit.to_string());
        let mut q: Vec<(&str, &str)> = vec![
            ("userId", &self.user_id),
            ("parentId", parent_id),
            ("SortBy", "SortName"),
            ("SortOrder", "Ascending"),
            ("StartIndex", &start),
            ("Limit", &limit),
        ];
        match collection_type {
            Some("movies") => q.extend([("Recursive", "true"), ("IncludeItemTypes", "Movie")]),
            Some("tvshows") => q.extend([("Recursive", "true"), ("IncludeItemTypes", "Series")]),
            _ => {}
        }
        let r: ItemsResp = self.get("/Items", &q).await?;
        let total = r.total_record_count.unwrap_or(r.items.len() as u32);
        Ok((r.items, total))
    }

    /// Segments du média : (début et fin de l'intro, début du générique de fin), en secondes.
    /// Segments Jellyfin 10.10+ (que remplit le plugin Intro Skipper), sinon l'ancienne API du plugin.
    pub async fn segments(&self, id: &str) -> (Option<(f64, f64)>, Option<f64>) {
        let mut intro: Option<(f64, f64)> = None;
        let mut outro: Option<f64> = None;
        if let Ok(v) = self.get::<serde_json::Value>(&format!("/MediaSegments/{id}"), &[]).await {
            for s in v["Items"].as_array().into_iter().flatten() {
                let (Some(a), Some(b)) = (s["StartTicks"].as_f64(), s["EndTicks"].as_f64()) else { continue };
                match s["Type"].as_str() {
                    Some("Intro") if intro.is_none() => intro = Some((a / 1e7, b / 1e7)),
                    Some("Outro") | Some("Credits") => outro = Some(outro.map_or(a / 1e7, |o: f64| o.min(a / 1e7))),
                    _ => {}
                }
            }
        }
        if intro.is_none() || outro.is_none() {
            // Ancienne API d'Intro Skipper : {"Introduction": {...}, "Credits": {...}}.
            if let Ok(v) = self.get::<serde_json::Value>(&format!("/Episode/{id}/IntroSkipperSegments"), &[]).await {
                let pick = |o: &serde_json::Value| -> Option<(f64, f64)> {
                    if o["Valid"].as_bool() == Some(false) {
                        return None;
                    }
                    let a = o["Start"].as_f64().or(o["IntroStart"].as_f64())?;
                    let b = o["End"].as_f64().or(o["IntroEnd"].as_f64())?;
                    (b > a).then_some((a, b))
                };
                if intro.is_none() {
                    intro = pick(&v["Introduction"]);
                }
                if outro.is_none() {
                    outro = pick(&v["Credits"]).map(|c| c.0);
                }
            }
        }
        (intro, outro)
    }

    /// Image de fond d'un élément (Backdrop, sinon Thumb / Primary), pour l'écran de fin.
    pub async fn backdrop(&self, item: &Item) -> Option<Vec<u8>> {
        let mut c: Vec<ImgCand> = Vec::new();
        if let Some(t) = item.backdrop_image_tags.as_ref().and_then(|t| t.first()) {
            c.push((item.id.clone(), "Backdrop", Some(t.clone())));
        }
        if let (Some(id), Some(t)) = (&item.parent_backdrop_item_id, item.parent_backdrop_image_tags.as_ref().and_then(|t| t.first())) {
            c.push((id.clone(), "Backdrop", Some(t.clone())));
        }
        c.extend(item.landscape_candidates());
        self.image_any(&c, Size::Fill(640, 360)).await
    }

    /// « Plus de ce genre » (éléments similaires de la bibliothèque).
    pub async fn similar(&self, id: &str) -> Result<Vec<Item>> {
        let r: ItemsResp = self.get(&format!("/Items/{id}/Similar"), &[("userId", &self.user_id), ("limit", "16")]).await?;
        Ok(r.items)
    }

    /// Le compte a-t-il le droit de télécharger ?
    pub async fn can_download(&self) -> bool {
        let v: serde_json::Value = match self.get(&format!("/Users/{}", self.user_id), &[]).await {
            Ok(v) => v,
            Err(_) => return false,
        };
        v["Policy"]["EnableContentDownloading"].as_bool().unwrap_or(false)
    }

    async fn send_flag(&self, method: reqwest::Method, path: &str) -> Result<()> {
        let resp = self
            .http
            .request(method, format!("{}{}", self.server, path))
            .query(&[("userId", &self.user_id)])
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Le serveur a répondu {} sur {path}", resp.status()));
        }
        Ok(())
    }

    pub async fn set_favorite(&self, id: &str, on: bool) -> Result<()> {
        let m = if on { reqwest::Method::POST } else { reqwest::Method::DELETE };
        self.send_flag(m, &format!("/UserFavoriteItems/{id}")).await
    }

    pub async fn set_played(&self, id: &str, on: bool) -> Result<()> {
        let m = if on { reqwest::Method::POST } else { reqwest::Method::DELETE };
        self.send_flag(m, &format!("/UserPlayedItems/{id}")).await
    }

    /// Seerr : « similar » ou « recommendations » d'un film / d'une série TMDB.
    pub async fn seerr_related(&self, tv: bool, tmdb: i64, kind: &str) -> Vec<CardInfo> {
        let path = format!("/JellyfinEnhanced/jellyseerr/{}/{tmdb}/{kind}", if tv { "tv" } else { "movie" });
        let v: serde_json::Value = match self.get(&path, &[]).await {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for r in v["results"].as_array().into_iter().flatten().take(16) {
            let title = r["title"].as_str().or(r["name"].as_str()).unwrap_or("").to_string();
            if title.is_empty() {
                continue;
            }
            let year: String = r["releaseDate"].as_str().or(r["firstAirDate"].as_str()).unwrap_or("").chars().take(4).collect();
            let available = r["mediaInfo"]["status"].as_i64() == Some(5);
            let jf = r["mediaInfo"]["jellyfinMediaId"].as_str().filter(|s| !s.is_empty()).map(str::to_string);
            let mut sub = vec![year];
            if available {
                sub.push("disponible".into());
            }
            out.push(CardInfo {
                id: jf.unwrap_or_default(),
                title,
                subtitle: sub.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "),
                img_id: r["posterPath"].as_str().filter(|p| !p.is_empty()).map(|p| format!("https://image.tmdb.org/t/p/w342{p}")),
                img_tag: None,
                thumbs: Vec::new(),
                progress: 0.0,
                rating: r["voteAverage"].as_f64().filter(|v| *v > 0.0).map(|v| format!("{v:.1}")).unwrap_or_default(),
            });
        }
        out
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

    /// Fichier original à télécharger (lecture hors ligne). Le jeton doit passer par l'en-tête
    /// (voir `auth`) : ce point d'accès refuse `api_key` dans l'adresse (401).
    pub fn download_url(&self, item_id: &str) -> String {
        format!("{}/Items/{}/Download", self.server, item_id)
    }

    /// Valeur de l'en-tête Authorization de la session.
    pub fn auth(&self) -> String {
        auth_header(&self.device_id, Some(&self.token))
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

    /// Première image disponible parmi des candidats de types différents (Thumb, Backdrop...).
    /// Type "Url" : `id` est une adresse complète hors Jellyfin (affiches TMDB des demandes Seerr).
    pub async fn image_any(&self, candidates: &[ImgCand], size: Size) -> Option<Vec<u8>> {
        for (id, kind, tag) in candidates {
            let r = if *kind == "Url" { self.image_url(id).await } else { self.image(id, kind, tag.as_deref(), size).await };
            if let Ok(b) = r {
                return Some(b);
            }
        }
        None
    }

    /// Image hors Jellyfin (adresse complète), avec le même cache disque.
    async fn image_url(&self, url: &str) -> Result<Vec<u8>> {
        // Nom de fichier stable tiré de l'adresse (pas besoin d'un vrai hachage cryptographique).
        let key: String = url.chars().filter(|c| c.is_ascii_alphanumeric()).rev().take(48).collect();
        let cache = image_cache_path(&key, "Url", None, "orig");
        if let Some(p) = &cache {
            if let Ok(b) = tokio::fs::read(p).await {
                return Ok(b);
            }
        }
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("image {url} : {}", resp.status()));
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

    // -----------------------------------------------------------------------
    // Seerr (Jellyseerr / Overseerr), via le plugin Jellyfin Enhanced : il relaie les requêtes
    // avec la connexion Jellyfin de l'utilisateur, sans clé Seerr côté client.
    // -----------------------------------------------------------------------

    /// Identifiant Seerr de l'utilisateur si Seerr est configuré, joignable et relié à son compte
    /// Jellyfin. None sinon (plugin absent, Seerr désactivé ou injoignable, compte non relié).
    pub async fn seerr_user(&self) -> Option<i64> {
        let v: serde_json::Value = self.get("/JellyfinEnhanced/jellyseerr/user-status", &[]).await.ok()?;
        if v["active"].as_bool() != Some(true) || v["userFound"].as_bool() != Some(true) {
            return None;
        }
        match &v["jellyseerrUserId"] {
            serde_json::Value::Number(n) => n.as_i64(),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Demandes Seerr faites par l'utilisateur `seerr_user`, avec titre et affiche.
    pub async fn seerr_requests(&self, seerr_user: i64) -> Result<Vec<SeerrRequest>> {
        let v: serde_json::Value =
            self.get("/JellyfinEnhanced/jellyseerr/request", &[("take", "100"), ("filter", "all")]).await?;
        let mut out: Vec<SeerrRequest> = Vec::new();
        for r in v["results"].as_array().into_iter().flatten() {
            if r["requestedBy"]["id"].as_i64() != Some(seerr_user) {
                continue;
            }
            let media = &r["media"];
            let tv = r["type"].as_str() == Some("tv") || media["mediaType"].as_str() == Some("tv");
            let Some(tmdb) = media["tmdbId"].as_i64() else { continue };
            // Titre et affiche : fiche TMDB relayée par Seerr.
            let path = format!("/JellyfinEnhanced/jellyseerr/{}/{tmdb}", if tv { "tv" } else { "movie" });
            let d: serde_json::Value = self.get(&path, &[]).await.unwrap_or_default();
            let title = d[if tv { "name" } else { "title" }].as_str().unwrap_or("Sans titre").to_string();
            let year = d[if tv { "firstAirDate" } else { "releaseDate" }].as_str().unwrap_or("").chars().take(4).collect();
            let poster = d["posterPath"].as_str().filter(|p| !p.is_empty()).map(|p| format!("https://image.tmdb.org/t/p/w342{p}"));
            out.push(SeerrRequest {
                status: r["status"].as_i64().unwrap_or(0),
                tv,
                title,
                year,
                poster,
                created: r["createdAt"].as_str().unwrap_or("").to_string(),
                jellyfin_id: media["jellyfinMediaId"].as_str().filter(|s| !s.is_empty()).map(str::to_string),
                seasons: r["seasons"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|s| s["seasonNumber"].as_i64()).collect())
                    .unwrap_or_default(),
            });
        }
        Ok(out)
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
