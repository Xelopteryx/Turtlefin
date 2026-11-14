//! Client minimal pour l'API Jellyfin 10.11 (REST).

use crate::i18n::{tr, trf};
use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};

use crate::config::Saved;

/// Erreur dédiée : le jeton est refusé (401).
#[derive(Debug)]
pub struct Unauthorized;

impl std::fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", tr("Session expirée ou identifiants refusés"))
    }
}
impl std::error::Error for Unauthorized {}

/// Changement de mot de passe : le mot de passe actuel est refusé (403).
#[derive(Debug)]
pub struct WrongPassword;

impl std::fmt::Display for WrongPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", tr("Mot de passe actuel incorrect."))
    }
}
impl std::error::Error for WrongPassword {}

/// Filtres de la recherche avancée en plus des listes : années (« 2010,2011… »), note minimale, état
/// (0 tous · 1 pas vus · 2 vus · 3 favoris).
#[derive(Default)]
pub struct MoreFilters {
    pub years: String,
    pub min_rating: u8,
    pub status: u8,
}

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
    /// Séries / saisons : épisodes pas encore vus.
    pub unplayed_item_count: Option<i64>,
}

/// Élément dont seul le nom sert (studio).
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct Named {
    pub name: String,
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
    pub display_title: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
    pub is_forced: bool,
    /// Sous-titres en texte (SRT, ASS…), convertibles ; faux pour les images (PGS, DVD).
    pub is_text_subtitle_stream: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct MediaSource {
    pub id: String,
    pub media_streams: Vec<MediaStream>,
    /// Format du fichier (« mkv », « mov,mp4,m4a »...).
    pub container: Option<String>,
    /// « File » (fichier du serveur) ou « Http » (adresse distante : fichier .strm…).
    pub protocol: Option<String>,
    pub path: Option<String>,
    /// Taille du fichier (octets), si le serveur la connaît.
    pub size: Option<u64>,
}

/// Flux converti par le serveur (voir `Client::transcode`).
#[derive(Clone, Debug)]
pub struct Transcode {
    pub url: String,
    /// Session de lecture du serveur (rapports de lecture, arrêt de la conversion).
    pub session: String,
}

/// Débits maximaux proposés (kbit/s ; 0 : fichier original, sans limite).
pub const BITRATES: [u32; 11] = [0, 40_000, 20_000, 12_000, 8_000, 6_000, 4_000, 3_000, 2_000, 1_500, 1_000];

/// Taille d'image maximale demandée au serveur pour un débit : une image plus petite se convertit
/// bien plus vite (serveurs modestes) et reste nette à ce débit.
pub fn max_size(kbps: u32) -> (u32, u32) {
    match kbps {
        k if k >= 30_000 => (3840, 2160),
        k if k >= 8_000 => (1920, 1080),
        k if k >= 3_000 => (1280, 720),
        k if k >= 1_500 => (854, 480),
        _ => (640, 360),
    }
}

/// Retire le jeton (`ApiKey`, `api_key`) d'une adresse fournie par le serveur : il passe par
/// l'en-tête et ne doit apparaître ni dans les journaux ni dans les messages d'erreur.
fn strip_api_key(path: &str) -> String {
    let Some((base, query)) = path.split_once('?') else { return path.to_string() };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| {
            let key = p.split('=').next().unwrap_or("").to_ascii_lowercase();
            key != "apikey" && key != "api_key"
        })
        .collect();
    format!("{base}?{}", kept.join("&"))
}

impl MediaSource {
    /// Source distante (adresse web) : lue directement, sans le jeton du serveur Jellyfin (il
    /// n'a rien à faire chez un autre hôte, où une redirection l'aurait envoyé).
    pub fn remote_url(&self) -> Option<&str> {
        let p = self.path.as_deref()?;
        (self.protocol.as_deref() == Some("Http") && (p.starts_with("https://") || p.starts_with("http://"))).then_some(p)
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct Item {
    pub id: String,
    pub name: String,
    /// Tags, genres et studios : rendus pour une fiche (`/Items/{id}`) ; dans une liste, les tags
    /// seulement s'ils sont demandés (`Fields=Tags`, recherche avancée).
    pub tags: Vec<String>,
    pub genres: Vec<String>,
    pub studios: Vec<Named>,
    /// Fichier sur le serveur (bandes-annonces locales : leur langue est dans le nom du fichier).
    pub path: Option<String>,
    #[serde(rename = "Type")]
    pub kind: String,
    pub is_folder: bool,
    pub collection_type: Option<String>,
    pub overview: Option<String>,
    pub official_rating: Option<String>,
    pub community_rating: Option<f32>,
    pub run_time_ticks: Option<i64>,
    pub production_year: Option<u32>,
    /// Date de sortie (« 2010-09-27T00:00:00.0000000Z »).
    pub premiere_date: Option<String>,
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
    /// Entrée d'une liste de lecture (sert à l'en retirer).
    pub playlist_item_id: Option<String>,
    /// Bandes-annonces enregistrées sur le serveur (fichiers « -trailer », Trailarr…).
    pub local_trailer_count: Option<u32>,
    /// Bandes-annonces en ligne connues du serveur (YouTube, via TMDb, dans la langue de ses métadonnées).
    pub remote_trailers: Option<Vec<RemoteTrailer>>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct RemoteTrailer {
    pub url: Option<String>,
    pub name: Option<String>,
}

impl Item {
    /// Identifiant YouTube de la première bande-annonce en ligne (11 caractères vérifiés : l'adresse
    /// envoyée à yt-dlp est reconstruite, jamais reprise telle quelle du serveur).
    pub fn youtube_trailer(&self) -> Option<String> {
        self.remote_trailers.iter().flatten().filter_map(|t| t.url.as_deref()).find_map(|u| {
            let id = u
                .split_once("youtube.com/watch?v=")
                .or_else(|| u.split_once("youtu.be/"))
                .or_else(|| u.split_once("youtube.com/embed/"))
                .map(|(_, rest)| rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect::<String>())?;
            (id.len() == 11).then_some(id)
        })
    }
}

/// Nom de la liste de lecture « À regarder » du compte : le même quelle que soit la langue de
/// l'appareil, pour retrouver la même liste partout (et dans le client web de Jellyfin).
pub const WATCHLIST: &str = "À regarder";

/// Entrée de la liste « À regarder » : `key` est l'élément représenté (la série pour un épisode,
/// car une série inscrite telle quelle serait remplacée par tous ses épisodes), `item` celui inscrit.
#[derive(Clone, Debug)]
pub struct WatchEntry {
    pub entry: String,
    pub item: String,
    pub key: String,
    /// Saison de l'épisode inscrit (vide sinon) : signet de la fiche de cette saison.
    pub season: String,
}

/// Ce dont l'interface a besoin pour afficher une carte.
#[derive(Clone, Debug)]
pub struct CardInfo {
    /// Série d'un épisode (vide sinon) : au retour d'une fiche, l'affiche rejoint la carte de la même
    /// série même si l'épisode a changé (« À suivre » passé à l'épisode d'après).
    pub group: String,
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
    /// Proposé par Seerr mais absent du serveur (badge, ouvre la page Seerr).
    pub seerr: bool,
    /// État d'une demande Seerr : 0 aucun · 1 en attente · 2 acceptée · 3 refusée · 4 disponible · 5 échec
    pub status: i32,
    /// Épisodes restant à voir (séries, saisons), 0 sinon.
    pub count: i32,
    /// Vu en entier (film, épisode, série ou saison terminée) : coche sur l'affiche.
    pub played: bool,
    /// Collection (BoxSet) : ouverte en « distribuant » ses films (voir deal, app.slint).
    pub deck: bool,
    /// Résumé (épisodes d'une saison, montrés en liste) ; vide sinon.
    pub overview: String,
}

/// Appareil pilotable du compte (voir Client::remote_targets).
#[derive(Clone, Debug, Default)]
pub struct RemoteTarget {
    pub session: String,
    pub device: String,
    pub client: String,
    pub now_playing: String,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
}

/// (id de l'élément portant l'image, étiquette de version de l'image)
pub type ImageRef = (String, Option<String>);

/// (id de l'élément, type d'image : "Primary" / "Thumb" / "Backdrop", étiquette)
/// Type "Url" : `id` est une adresse complète (images hors Jellyfin).
pub type ImgCand = (String, &'static str, Option<String>);

/// Identifiant de carte d'un média Seerr absent du serveur : « seerr:movie:603 », « seerr:tv:1399 ».
pub fn seerr_id(tv: bool, tmdb: i64) -> String {
    format!("seerr:{}:{tmdb}", if tv { "tv" } else { "movie" })
}

/// Noms d'une liste de `/Items/Filters2` : des textes (tags) ou des objets `{Name, Id}` (genres).
fn filter_names(v: &serde_json::Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(|t| t.as_str().or(t["Name"].as_str()).map(str::to_string)).collect()
}

/// Carte d'un résultat de Seerr (film, série ou collection ; un film d'une collection n'a pas de `mediaType`).
fn seerr_card(r: &serde_json::Value) -> Option<CardInfo> {
    let id = r["id"].as_i64()?;
    let kind = r["mediaType"].as_str().unwrap_or("movie");
    let title = r["title"].as_str().or(r["name"].as_str()).unwrap_or("").to_string();
    let year: String = r["releaseDate"].as_str().or(r["firstAirDate"].as_str()).unwrap_or("").chars().take(4).collect();
    let label = match kind {
        "tv" => tr("Série"),
        "collection" => tr("Collection"),
        _ => tr("Film"),
    };
    Some(CardInfo {
        group: String::new(),
        id: if kind == "collection" { format!("seerrcol:{id}") } else { seerr_id(kind == "tv", id) },
        title,
        subtitle: [label.to_string(), year].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "),
        img_id: tmdb_img(r["posterPath"].as_str(), "w342"),
        img_tag: None,
        thumbs: Vec::new(),
        progress: 0.0,
        rating: r["voteAverage"].as_f64().filter(|v| *v > 0.0).map(|v| format!("{v:.1}")).unwrap_or_default(),
        seerr: true,
        status: 0,
        count: 0,
        played: false,
        deck: false,
        overview: String::new(),
    })
}

/// Listes proposées par les filtres de la recherche avancée.
#[derive(Clone, Debug, Default)]
pub struct FilterLists {
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub studios: Vec<String>,
}

fn tmdb_img(path: Option<&str>, size: &str) -> Option<String> {
    path.filter(|p| !p.is_empty()).map(|p| format!("https://image.tmdb.org/t/p/{size}{p}"))
}

/// Page Seerr d'un média (film ou série).
#[derive(Clone, Debug, Default)]
pub struct SeerrDetails {
    pub tv: bool,
    pub tmdb: i64,
    pub title: String,
    pub year: String,
    pub overview: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub genres: String,
    /// « 24 min · 12 épisodes », « 1h55 »
    pub length: String,
    pub rating: String,
    /// Lignes d'informations (libellé, valeur).
    pub facts: Vec<(String, String)>,
    /// 2 en attente · 3 en cours · 4 partiellement disponible · 5 disponible (0 : rien)
    pub status: i64,
    pub jellyfin_id: Option<String>,
    pub seasons: Vec<i64>,
    /// Saisons ni disponibles, ni demandées, ni en cours (série incomplète).
    pub missing_seasons: Vec<i64>,
    /// Distribution : (nom, personnage, photo)
    pub cast: Vec<(String, String, Option<String>)>,
}

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
    pub tmdb: i64,
    /// Média disponible sur le serveur (Seerr : statut média 5).
    pub available: bool,
    pub seasons: Vec<i64>,
}

/// Images d'aperçu d'une vidéo : des planches de `cols` x `rows` images de `w` x `h`, une toutes les
/// `interval_ms`.
#[derive(Clone)]
pub struct Trickplay {
    pub key: String,
    pub w: i32,
    pub h: i32,
    pub cols: i32,
    pub rows: i32,
    pub count: i32,
    pub interval_ms: i64,
}

impl CardInfo {
    /// Même carte, avec ces images en format paysage.
    pub fn with_thumbs(mut self, thumbs: Vec<ImgCand>) -> Self {
        self.thumbs = thumbs;
        self
    }
}

impl SeerrRequest {
    pub fn card(&self) -> CardInfo {
        let mut parts: Vec<String> = vec![if self.tv { tr("Série") } else { tr("Film") }.to_string()];
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
            group: String::new(),
            // Disponible : sa fiche Jellyfin ; sinon la page Seerr.
            id: self.jellyfin_id.clone().unwrap_or_else(|| seerr_id(self.tv, self.tmdb)),
            title: self.title.clone(),
            subtitle: parts.join(" · "),
            img_id: self.poster.clone(),
            img_tag: None,
            thumbs: Vec::new(),
            progress: 0.0,
            rating: String::new(),
            seerr: false,
            status: if self.available {
                4
            } else {
                match self.status {
                    1 => 1,
                    2 => 2,
                    3 => 3,
                    4 => 5,
                    5 => 4,
                    _ => 0,
                }
            },
            count: 0,
            played: false,
            deck: false,
            overview: String::new(),
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

    pub(crate) fn progress(&self) -> f32 {
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
            _ => match (self.primary_tag(), &self.series_id) {
                (Some(tag), _) => (Some(self.id.clone()), Some(tag)),
                // Saison sans affiche (découpage maison absent de TMDB) : celle de la série.
                (None, Some(sid)) if self.series_primary_image_tag.is_some() => {
                    (Some(sid.clone()), self.series_primary_image_tag.clone())
                }
                (None, _) => (None, None),
            },
        };

        CardInfo {
            group: if is_episode { self.series_id.clone().unwrap_or_default() } else { String::new() },
            id: self.id.clone(),
            title,
            subtitle,
            img_id,
            img_tag,
            thumbs: self.landscape_candidates(),
            progress: self.progress(),
            rating: self.rating(),
            seerr: false,
            status: 0,
            count: match self.kind.as_str() {
                "Series" | "Season" => self.user_data.as_ref().and_then(|u| u.unplayed_item_count).unwrap_or(0) as i32,
                _ => 0,
            },
            played: self.user_data.as_ref().is_some_and(|u| u.played),
            // Collection, ou bibliothèque (« Mes médias ») : ouverte en distribuant son contenu.
            deck: matches!(self.kind.as_str(), "BoxSet" | "CollectionFolder" | "UserView"),
            overview: self.overview.clone().unwrap_or_default(),
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
        let played = self.user_data.as_ref().map(|u| u.played).unwrap_or(false);
        let tag = self.primary_tag();
        CardInfo {
            group: String::new(),
            id: self.id.clone(),
            title,
            subtitle: parts.join(" · "),
            img_id: tag.as_ref().map(|_| self.id.clone()),
            img_tag: tag.clone(),
            // Un épisode dans sa saison : sa propre image 16:9 d'abord.
            thumbs: tag.map(|t| (self.id.clone(), "Primary", Some(t))).into_iter().collect(),
            progress: self.progress(),
            rating: String::new(),
            seerr: false,
            status: 0,
            count: 0,
            // Coche « vu » sur l'image, comme les films et les séries.
            played,
            deck: false,
            overview: self.overview.clone().unwrap_or_default(),
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
    /// Chiffres de l'en-tête de la fiche : note (sur 10, négative si absente), durée, date de sortie,
    /// classification (« 10+ », « PG »…). Une valeur vide n'est pas affichée.
    pub fn stats(&self) -> (f32, String, String, String) {
        let score = self.community_rating.filter(|c| *c > 0.0).unwrap_or(-1.0);
        let time = match self.minutes().filter(|m| *m > 0) {
            Some(m) if m >= 60 => format!("{}h{:02}", m / 60, m % 60),
            Some(m) => format!("{m} min"),
            None => String::new(),
        };
        let date = self
            .premiere_date
            .as_deref()
            .and_then(|d| {
                let (y, rest) = d.split_once('-')?;
                let (m, rest) = rest.split_once('-')?;
                let day: u32 = rest.get(..2)?.parse().ok()?;
                let month = [
                    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
                ]
                .get(m.parse::<usize>().ok()?.checked_sub(1)?)?;
                Some(format!("{day} {} {y}", tr(month)))
            })
            .or_else(|| self.production_year.map(|y| y.to_string()))
            .unwrap_or_default();
        let age = self
            .official_rating
            .as_ref()
            .filter(|r| !r.is_empty())
            .map(|r| match r.strip_prefix('-').or_else(|| r.strip_prefix("FR-")) {
                Some(n) if n.chars().all(|c| c.is_ascii_digit()) => format!("{n}+"),
                _ => r.clone(),
            })
            .unwrap_or_default();
        (score, time, date, age)
    }

    /// Ligne d'informations quand les chiffres sont affichés à part (voir `stats`) : les langues audio.
    pub fn misc_short(&self) -> String {
        let langs = self.audio_langs();
        if langs.is_empty() { String::new() } else { trf("Audio : {}", &[&langs.join(", ")]) }
    }

    pub fn misc_line(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(y) = self.production_year {
            parts.push(y.to_string());
        }
        if let Some(r) = self.official_rating.as_ref().filter(|r| !r.is_empty()) {
            // Classification française « -10 » (moins de 10 ans déconseillé) : affichée « 10+ ».
            let r = match r.strip_prefix('-').or_else(|| r.strip_prefix("FR-")) {
                Some(n) if n.chars().all(|c| c.is_ascii_digit()) => format!("{n}+"),
                _ => r.clone(),
            };
            parts.push(r);
        }
        if let Some(m) = self.minutes().filter(|m| *m > 0) {
            parts.push(if m >= 60 { format!("{}h{:02}", m / 60, m % 60) } else { format!("{m} min") });
        }
        if let Some(c) = self.community_rating {
            parts.push(format!("{c:.1}/10"));
        }
        let langs = self.audio_langs();
        if !langs.is_empty() {
            parts.push(trf("Audio : {}", &[&langs.join(", ")]));
        }
        parts.join("  ·  ")
    }

    /// Boutons de la fiche : (libellé, action, icône, actif). Les boutons à icône n'ont pas de
    /// libellé. Actions : "play", "fav", "watch", "played", "download", "open:<id>".
    /// Texte du bouton Lecture : « Reprendre à 12:03 » si la lecture est commencée.
    pub fn play_label(&self) -> String {
        let secs = self.user_data.as_ref().map_or(0, |u| u.playback_position_ticks) / 10_000_000;
        if secs < 5 || !matches!(self.kind.as_str(), "Movie" | "Episode" | "Video" | "MusicVideo") {
            return tr("Lecture").to_string();
        }
        let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
        let at = if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") };
        trf("Reprendre à {}", &[&at])
    }

    pub fn buttons(&self, can_download: bool) -> Vec<(String, String, String, bool)> {
        let mut b: Vec<(String, String, String, bool)> = Vec::new();
        let ud = self.user_data.clone().unwrap_or_default();
        let playable = matches!(self.kind.as_str(), "Movie" | "Episode" | "Series" | "Season" | "Video" | "MusicVideo");
        if playable {
            b.push((self.play_label(), "play".into(), "play".into(), false));
            b.push((String::new(), "fav".into(), "heart".into(), ud.is_favorite));
            // « À regarder » : état réel posé par la fiche (liste du compte, voir main::watch_has).
            b.push((String::new(), "watch".into(), "bookmark".into(), false));
            b.push((String::new(), "played".into(), "check".into(), ud.played));
            if can_download {
                b.push((String::new(), "download".into(), "download".into(), false));
            }
        }
        // La série d'abord, puis la saison (du plus large au plus précis).
        if matches!(self.kind.as_str(), "Episode" | "Season") {
            if let Some(id) = &self.series_id {
                // Saison : seul bouton de texte, il rejoint la ligne principale (« label »).
                let icon = if self.kind == "Season" { "label" } else { "" };
                b.push((tr("Voir la série").into(), format!("open:{id}"), icon.into(), false));
            }
        }
        if self.kind == "Episode" {
            if let Some(id) = &self.season_id {
                b.push((tr("Voir la saison").into(), format!("open:{id}"), String::new(), false));
            }
        }
        b
    }

    /// Casting et équipe : cartes portrait (photo, nom, rôle).
    #[allow(dead_code)]
    pub fn people_cards(&self) -> Vec<CardInfo> {
        let mut out = Vec::new();
        for p in self.people.iter().flatten().take(30) {
            let role = match (p.kind.as_deref(), p.role.as_deref().filter(|r| !r.is_empty())) {
                (Some("Actor") | Some("GuestStar"), Some(r)) => r.to_string(),
                (Some("Actor"), None) => tr("Acteur").into(),
                (Some("GuestStar"), None) => tr("Invité").into(),
                (Some("Director"), _) => tr("Réalisation").into(),
                (Some("Writer"), _) => tr("Scénario").into(),
                (Some("Producer"), _) => tr("Production").into(),
                (Some("Composer"), _) => tr("Musique").into(),
                (_, Some(r)) => r.to_string(),
                _ => String::new(),
            };
            out.push(CardInfo {
                group: String::new(),
                // Pas encore de page « personne » : la carte ne mène nulle part.
                id: format!("person:{}", p.id),
                title: p.name.clone(),
                subtitle: role,
                img_id: p.primary_image_tag.as_ref().map(|_| p.id.clone()),
                img_tag: p.primary_image_tag.clone(),
                thumbs: Vec::new(),
                progress: 0.0,
                rating: String::new(),
                seerr: false,
                status: 0,
                count: 0,
                played: false,
                deck: false,
                overview: String::new(),
            });
        }
        out
    }

    /// Clé des préférences de pistes : la série pour un épisode, l'élément sinon.
    pub fn pref_key(&self) -> String {
        match (&*self.kind, &self.series_id) {
            ("Episode" | "Season", Some(s)) => s.clone(),
            _ => self.id.clone(),
        }
    }

    /// Pistes audio ou sous-titres de la première source : (langue, libellé).
    pub fn streams(&self, kind: &str) -> Vec<(String, String)> {
        let Some(src) = self.media_sources.as_ref().and_then(|v| v.first()) else { return Vec::new() };
        let mut out: Vec<(String, String)> = Vec::new();
        for st in src.media_streams.iter().filter(|s| s.kind == kind) {
            let lang = st.language.clone().unwrap_or_default();
            let label = st
                .display_title
                .clone()
                .or_else(|| st.title.clone())
                .unwrap_or_else(|| if lang.is_empty() { tr("Inconnue").into() } else { lang.to_uppercase() });
            // Une entrée par langue : la préférence est enregistrée par langue.
            if !lang.is_empty() && out.iter().any(|(l, _)| *l == lang) {
                continue;
            }
            out.push((lang, label));
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
    /// Réponses gardées seulement, sans réseau (voir `cache_only`).
    cache_only: bool,
}

// Dernières réponses du serveur pour ce qui s'affiche en naviguant : en mémoire et sur le disque (un
// fichier par adresse, compte compris). Une page connue s'affiche tout de suite avec elles, puis se met à
// jour si le serveur répond autre chose (voir `Client::cache_only`, `changes`).
static RESP: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Vec<u8>>>>> =
    std::sync::LazyLock::new(Default::default);
static RESP_CHANGES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Compteur des réponses différentes de celles gardées : inchangé après un rechargement, la page est à jour.
pub fn changes() -> u64 {
    RESP_CHANGES.load(std::sync::atomic::Ordering::SeqCst)
}

/// Dossier des réponses gardées (Paramètres > À propos : vidé avec le cache d'images).
pub fn resp_cache_dir() -> Option<PathBuf> {
    crate::paths::cache_dir().map(|d| d.join("api"))
}

fn resp_file(key: &str) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut h);
    resp_cache_dir().map(|d| d.join(format!("{:016x}.json", h.finish())))
}

/// En mémoire : les dernières seulement (tout mettre en cache en lit des milliers ; le disque les garde).
fn resp_keep_in_memory(key: &str, body: std::sync::Arc<Vec<u8>>) {
    let mut m = RESP.lock().unwrap();
    if m.len() >= 300 {
        m.clear();
    }
    m.insert(key.to_string(), body);
}

fn resp_read(key: &str) -> Option<std::sync::Arc<Vec<u8>>> {
    if let Some(b) = RESP.lock().unwrap().get(key) {
        return Some(b.clone());
    }
    let b = std::sync::Arc::new(std::fs::read(resp_file(key)?).ok()?);
    resp_keep_in_memory(key, b.clone());
    Some(b)
}

fn resp_store(key: &str, body: &[u8]) {
    let same = resp_read(key).is_some_and(|old| old.as_slice() == body);
    if same {
        return;
    }
    RESP_CHANGES.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    resp_keep_in_memory(key, std::sync::Arc::new(body.to_vec()));
    if let Some(f) = resp_file(key) {
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
            // Contenu des bibliothèques du compte : fermé aux autres comptes de la machine.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
            }
        }
        let _ = std::fs::write(f, body);
    }
}

/// Seulement ce qui s'affiche en naviguant : ni comptes, ni tableau de bord, ni réglages du serveur.
fn resp_keep(path: &str) -> bool {
    path.starts_with("/Items")
        || path.starts_with("/UserViews")
        || path.starts_with("/UserItems")
        || path.starts_with("/Shows")
        || path.starts_with("/Playlists")
        || (path.starts_with("/Users/") && path.contains("/Items"))
        || (path.starts_with("/JellyfinEnhanced/jellyseerr/") && !path.ends_with("user-status"))
}

pub fn normalize_server(s: &str) -> String {
    let s = s.trim().trim_end_matches('/');
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else {
        format!("http://{s}")
    }
}

/// Nom de l'appareil montré aux autres applis du compte (Diffuser, tableau de bord du serveur) : nom
/// de la machine (Prometheus, Mjollnir…), sinon le système. Lettres, chiffres, espaces et tirets
/// seulement (il passe dans un en-tête HTTP).
pub fn device_name() -> &'static str {
    static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NAME.get_or_init(|| {
        let raw = if let Ok(n) = std::env::var("TURTLEFIN_DEVICE_NAME") {
            n
        } else if cfg!(windows) {
            std::env::var("COMPUTERNAME").unwrap_or_default()
        } else if cfg!(target_os = "android") {
            String::from("Android")
        } else {
            std::fs::read_to_string("/etc/hostname").unwrap_or_default()
        };
        // Lettres de toutes langues (« A12 de Clément ») : l'en-tête les encode (voir auth_header).
        let clean: String = raw.trim().chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '\'')).take(40).collect();
        if clean.is_empty() {
            std::env::consts::OS.to_string()
        } else if clean.chars().any(|c| c.is_lowercase() || c.is_ascii_digit()) {
            // Déjà en casse voulue, ou référence de modèle (« SM-A127F ») : telle quelle.
            clean
        } else {
            // « PROMETHEUS » -> « Prometheus » (Windows met le nom en majuscules).
            let mut c = clean.to_lowercase().chars().collect::<Vec<_>>();
            if let Some(f) = c.first_mut() {
                *f = f.to_ascii_uppercase();
            }
            c.into_iter().collect()
        }
    })
}

/// Encodage des valeurs de l'en-tête (comme encodeURIComponent, ce que fait l'appli web de Jellyfin ;
/// le serveur décode) : un en-tête HTTP ne porte que de l'ASCII.
fn header_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn auth_header(device_id: &str, token: Option<&str>) -> String {
    let mut h = format!(
        "MediaBrowser Client=\"Turtlefin\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\"",
        header_value(device_name()),
        device_id,
        env!("CARGO_PKG_VERSION")
    );
    if let Some(t) = token {
        h.push_str(&format!(", Token=\"{t}\""));
    }
    h
}

/// Constructeur de client HTTP de Turtlefin. Android : le vérificateur de certificats de reqwest
/// (rustls-platform-verifier) y exige un composant Java que Turtlefin n'a pas ; on lui donne donc
/// directement les autorités de certification du système, et celles ajoutées par l'utilisateur.
pub fn http_builder() -> reqwest::ClientBuilder {
    let b = reqwest::Client::builder();
    #[cfg(target_os = "android")]
    let b = b.tls_certs_only(android_roots());
    b
}

/// Client HTTP simple (sans délai ni redirection particuliers).
pub fn http_client() -> reqwest::Client {
    http_builder().build().unwrap_or_default()
}

/// Android : certificats d'autorité du système et de l'utilisateur (fichiers PEM), lus une fois.
#[cfg(target_os = "android")]
fn android_roots() -> Vec<reqwest::Certificate> {
    static ROOTS: std::sync::OnceLock<Vec<reqwest::Certificate>> = std::sync::OnceLock::new();
    ROOTS
        .get_or_init(|| {
            ["/system/etc/security/cacerts", "/apex/com.android.conscrypt/cacerts", "/data/misc/user/0/cacerts-added"]
                .iter()
                .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
                .filter_map(|e| std::fs::read(e.path()).ok())
                .filter_map(|pem| reqwest::Certificate::from_pem(&pem).ok())
                .collect()
        })
        .clone()
}

/// Redirections suivies (10 au plus), sauf de https vers http : une 307/308 renverrait en clair le
/// corps de la requête (le mot de passe à la connexion). reqwest retire déjà l'en-tête Authorization
/// quand la redirection change d'hôte.
pub fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|a| {
        let downgrade = a.previous().iter().any(|u| u.scheme() == "https") && a.url().scheme() != "https";
        if downgrade {
            a.error("redirection de https vers http refusée")
        } else if a.previous().len() >= 10 {
            a.error("trop de redirections")
        } else {
            a.follow()
        }
    })
}

fn build_http() -> Result<reqwest::Client> {
    let mut b = crate::api::http_builder()
        .timeout(Duration::from_secs(30))
        // Serveur éteint ou injoignable : échec en quelques secondes, pas au bout de 30.
        .connect_timeout(Duration::from_secs(5))
        .redirect(redirect_policy())
        .user_agent(concat!("Turtlefin/", env!("CARGO_PKG_VERSION")));
    // À n'utiliser qu'en test : TURTLEFIN_INSECURE=1 accepte tout certificat.
    if std::env::var("TURTLEFIN_INSECURE").is_ok() {
        b = b.danger_accept_invalid_certs(true);
    }
    Ok(b.build()?)
}

/// Taille maximale d'une réponse JSON, d'une image : un serveur défaillant (ou malveillant) ne peut
/// pas remplir la mémoire en envoyant une réponse sans fin.
const MAX_JSON: usize = 64 << 20;
const MAX_IMAGE: usize = 24 << 20;

/// Corps d'une réponse, refusé au-delà de `max` octets.
pub async fn body_limited(mut resp: reqwest::Response, max: usize) -> Result<Vec<u8>> {
    if resp.content_length().is_some_and(|n| n > max as u64) {
        return Err(anyhow!("{}", tr("réponse du serveur trop grande")));
    }
    let mut out = Vec::new();
    while let Some(c) = resp.chunk().await? {
        if out.len() + c.len() > max {
            return Err(anyhow!("{}", tr("réponse du serveur trop grande")));
        }
        out.extend_from_slice(&c);
    }
    Ok(out)
}

async fn json_limited<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    Ok(serde_json::from_slice(&body_limited(resp, MAX_JSON).await?)?)
}

/// Morceau d'adresse venu du serveur (identifiant, étiquette d'image) : tout ce qui n'est pas
/// lettre, chiffre, « - » ou « _ » est encodé (pas de « ../ », « ? » ou « # » glissés dans l'adresse).
pub fn seg(s: &str) -> std::borrow::Cow<'_, str> {
    seg_with(s, false)
}

/// Comme `seg`, en gardant les points (numéros de version « 1.2.3.0 » des extensions).
pub fn seg_dots(s: &str) -> std::borrow::Cow<'_, str> {
    seg_with(s, true)
}

fn seg_with(s: &str, dots: bool) -> std::borrow::Cow<'_, str> {
    // « . » ou « .. » : même encodés (« %2E%2E »), l'analyse de l'adresse les prend pour un retour
    // au dossier parent. Remplacés par un morceau sans effet (le vide reste vide : sans danger).
    if !s.is_empty() && s.trim_matches('.').is_empty() {
        return std::borrow::Cow::Borrowed("_");
    }
    let ok = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || (dots && b == b'.');
    if s.bytes().all(ok) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::new();
    for b in s.bytes() {
        if ok(b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Adresse joignable sur Internet : ni la machine elle-même, ni le réseau local, ni un VPN
/// (100.64.0.0/10 : Tailscale), ni une adresse réservée.
pub fn is_public_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v) => {
            let o = v.octets();
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_multicast()
                || v.is_documentation()
                || o[0] == 0
                || o[0] >= 240
                || (o[0] == 100 && (64..128).contains(&o[1]))
                || (o[0] == 192 && o[1] == 0 && o[2] == 0)
                || (o[0] == 198 && (18..20).contains(&o[1])))
        }
        std::net::IpAddr::V6(v) => {
            if let Some(v4) = v.to_ipv4_mapped() {
                return is_public_ip(std::net::IpAddr::V4(v4));
            }
            let s = v.segments();
            !(v.is_loopback()
                || v.is_unspecified()
                || v.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80
                || (s[0] == 0x2001 && s[1] == 0x0db8)
                || (s[0] == 0x64 && s[1] == 0xff9b))
        }
    }
}

/// Nom de fichier sûr tiré d'un texte du serveur (cache d'images) : pas de séparateur de dossier.
pub fn file_key(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(64).collect()
}

/// Dossier du cache d'images (Paramètres > À propos : taille, vidage).
pub fn image_cache_dir() -> Option<PathBuf> {
    crate::paths::cache_dir().map(|d| d.join("img"))
}

/// Cache d'images au-delà de `max` octets : les fichiers les moins récemment écrits partent jusqu'à
/// revenir aux trois quarts. Une affiche changée sur le serveur laisse sinon l'ancienne pour toujours
/// (carte SD du Pi, stockage du téléphone).
pub fn trim_image_cache(max: u64) {
    let Some(dir) = image_cache_dir() else { return };
    trim_dir(&dir, max);
}

/// Réponses gardées : même règle, avec leur propre limite.
pub fn trim_resp_cache(max: u64) {
    let Some(dir) = resp_cache_dir() else { return };
    trim_dir(&dir, max);
}

fn trim_dir(dir: &std::path::Path, max: u64) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok().filter(|m| m.is_file())?;
            Some((m.modified().unwrap_or(std::time::UNIX_EPOCH), m.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= max {
        return;
    }
    files.sort_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= max / 4 * 3 {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

fn image_cache_path(item_id: &str, kind: &str, tag: Option<&str>, size_key: &str) -> Option<PathBuf> {
    let (id, kind, tag) = (file_key(item_id), file_key(kind), file_key(tag.unwrap_or("none")));
    if id.is_empty() {
        return None;
    }
    image_cache_dir().map(|d| d.join(format!("{id}_{kind}_{tag}_{size_key}.bin")))
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
            cache_only: false,
        })
    }

    pub fn to_saved(&self) -> Saved {
        Saved {
            server: self.server.clone(),
            user_name: self.user_name.clone(),
            user_id: self.user_id.clone(),
            token: self.token.clone(),
            device_id: self.device_id.clone(),
            // La diffusion notée survit à la connexion : cast_restore la relit juste après.
            cast_last: crate::config::load().cast_last,
            ..Default::default()
        }
    }

    /// Change l'adresse utilisée (même serveur joint autrement : local / distant).
    pub fn set_server(&mut self, server: &str) {
        self.server = normalize_server(server);
    }

    /// Session à partir d'un compte enregistré.
    pub fn from_token(server: &str, user_id: &str, user_name: &str, token: &str, device_id: &str) -> Result<Self> {
        Ok(Self {
            http: build_http()?,
            server: normalize_server(server),
            token: token.to_string(),
            user_id: user_id.to_string(),
            user_name: user_name.to_string(),
            device_id: device_id.to_string(),
            cache_only: false,
        })
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
            .map_err(|e| anyhow!("{}", trf("Impossible de joindre {} : {}", &[&server, &e])))?;

        let status = resp.status();
        if status.as_u16() == 401 {
            return Err(anyhow!("{}", tr("Identifiant ou mot de passe incorrect")));
        }
        if !status.is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&status])));
        }
        let r: AuthResult = json_limited(resp).await?;
        Ok(Self {
            http,
            server,
            token: r.access_token,
            user_id: r.user.id,
            user_name: r.user.name,
            device_id: device_id.to_string(),
            cache_only: false,
        })
    }

    /// Ce client, limité aux réponses gardées : aucune requête, une adresse jamais vue est une erreur.
    pub fn cache_only(&self) -> Client {
        Client { cache_only: true, ..self.clone() }
    }

    /// Ce client, sur le réseau (pour ce qu'un affichage lancé sur le cache démarre à côté).
    pub fn live(&self) -> Client {
        Client { cache_only: false, ..self.clone() }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let keep = resp_keep(path);
        let key = if keep { format!("{}|{}|{path}|{query:?}", self.server, self.user_id) } else { String::new() };
        if self.cache_only {
            let body = keep.then(|| resp_read(&key)).flatten().ok_or_else(|| anyhow!("pas dans le cache"))?;
            return Ok(serde_json::from_slice(&body)?);
        }
        let resp = self
            .http
            .get(format!("{}{}", self.server, path))
            .query(query)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        match resp.status().as_u16() {
            401 => Err(Unauthorized.into()),
            c if !(200..300).contains(&c) => Err(anyhow!("{}", trf("Le serveur a répondu {} sur {}", &[&c, &path]))),
            _ if keep => {
                let body = body_limited(resp, MAX_JSON).await?;
                let v = serde_json::from_slice(&body)?;
                resp_store(&key, &body);
                Ok(v)
            }
            _ => json_limited(resp).await,
        }
    }

    /// Bibliothèques de l'utilisateur.
    pub async fn views(&self) -> Result<Vec<Item>> {
        let r: ItemsResp = self.get("/UserViews", &[("userId", &self.user_id)]).await?;
        let mut views = r.items;
        // Bibliothèque sans image sur le serveur : la vignette prend le fond du dernier film ou de la
        // dernière série ajoutés (plutôt qu'une case vide).
        let bare: Vec<(usize, tokio::task::JoinHandle<Option<Item>>)> = views
            .iter()
            .enumerate()
            .filter(|(_, v)| v.image_tags.as_ref().is_none_or(|t| !t.contains_key("Primary") && !t.contains_key("Thumb")))
            .map(|(i, v)| {
                let (c, id) = (self.clone(), v.id.clone());
                (i, tokio::spawn(async move { c.latest_with_backdrop(&id).await }))
            })
            .collect();
        for (i, h) in bare {
            if let Ok(Some(it)) = h.await {
                views[i].parent_backdrop_item_id = Some(it.id.clone());
                views[i].parent_backdrop_image_tags = it.backdrop_image_tags.clone();
            }
        }
        Ok(views)
    }

    /// Dernier film ou série ajouté à une bibliothèque, ayant une image de fond.
    async fn latest_with_backdrop(&self, parent: &str) -> Option<Item> {
        let r: ItemsResp = self
            .get(
                "/Items",
                &[
                    ("userId", &self.user_id),
                    ("parentId", parent),
                    ("recursive", "true"),
                    ("includeItemTypes", "Movie,Series"),
                    ("imageTypes", "Backdrop"),
                    ("sortBy", "DateCreated"),
                    ("sortOrder", "Descending"),
                    ("limit", "1"),
                ],
            )
            .await
            .ok()?;
        r.items.into_iter().find(|i| i.backdrop_image_tags.as_ref().is_some_and(|t| !t.is_empty()))
    }

    /// Images d'aperçu préparées par le serveur (trickplay) : la plus proche de 320 px de large.
    pub async fn trickplay(&self, id: &str) -> Option<Trickplay> {
        let v: serde_json::Value = self.get(&format!("/Items/{}", seg(id)), &[("userId", &self.user_id), ("Fields", "Trickplay")]).await.ok()?;
        let by_source = v.get("Trickplay")?.as_object()?;
        let widths = by_source.values().next()?.as_object()?;
        let (key, t) = widths.iter().min_by_key(|(k, _)| (k.parse::<i64>().unwrap_or(0) - 320).abs())?;
        let n = |f: &str| t.get(f).and_then(|x| x.as_i64()).filter(|x| *x > 0);
        Some(Trickplay {
            key: key.clone(),
            w: n("Width")? as i32,
            h: n("Height")? as i32,
            cols: n("TileWidth")? as i32,
            rows: n("TileHeight")? as i32,
            count: n("ThumbnailCount")? as i32,
            interval_ms: n("Interval")?,
        })
    }

    /// Planche d'images d'aperçu n° `tile`.
    pub async fn trickplay_tile(&self, id: &str, key: &str, tile: i32) -> Result<Vec<u8>> {
        let resp = self
            .http
            .get(format!("{}/Videos/{}/Trickplay/{}/{}.jpg", self.server, seg(id), seg(key), tile))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// « Reprendre la lecture ».
    pub async fn resume(&self) -> Result<Vec<Item>> {
        let r: ItemsResp = self
            .get(
                "/UserItems/Resume",
                &[("userId", &self.user_id), ("Limit", "16"), ("MediaTypes", "Video"), ("Fields", "Overview")],
            )
            .await?;
        Ok(r.items)
    }

    /// « À suivre ».
    pub async fn next_up(&self) -> Result<Vec<Item>> {
        let r: ItemsResp = self
            .get("/Shows/NextUp", &[("userId", &self.user_id), ("Limit", "16"), ("enableResumable", "false"), ("Fields", "Overview")])
            .await?;
        Ok(r.items)
    }

    /// Derniers ajouts d'une bibliothèque (cet endpoint renvoie un tableau directement).
    pub async fn latest(&self, parent_id: &str) -> Result<Vec<Item>> {
        self.get(
            "/Items/Latest",
            &[("userId", &self.user_id), ("parentId", parent_id), ("Limit", "16"), ("Fields", "Overview")],
        )
        .await
    }

    /// Un élément avec ses métadonnées (résumé, note, durée, état de lecture...).
    pub async fn item(&self, id: &str) -> Result<Item> {
        let id = seg(id);
        match self.get::<Item>(&format!("/Items/{id}"), &[("userId", &self.user_id)]).await {
            Ok(i) => Ok(i),
            Err(e) if e.downcast_ref::<Unauthorized>().is_some() => Err(e),
            // Ancienne route, au cas où.
            Err(_) => self.get(&format!("/Users/{}/Items/{id}", seg(&self.user_id)), &[]).await,
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
                    // Résumé : montré dans la liste des épisodes d'une saison.
                    ("Fields", "Overview"),
                ],
            )
            .await?;
        Ok(r.items)
    }

    /// Configuration du compte (préférences de lecture...).
    pub async fn user_config(&self) -> Result<serde_json::Value> {
        let v: serde_json::Value = self.get(&format!("/Users/{}", seg(&self.user_id)), &[]).await?;
        Ok(v["Configuration"].clone())
    }

    pub async fn set_user_config(&self, cfg: &serde_json::Value) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/Users/{}/Configuration", self.server, seg(&self.user_id)))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(cfg)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&resp.status()])));
        }
        Ok(())
    }

    pub async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        self.get(path, &[]).await
    }

    pub async fn post_json(&self, path: &str, body: &serde_json::Value) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}{}", self.server, path))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {} sur {}", &[&resp.status(), &path])));
        }
        Ok(())
    }

    /// Contenu brut d'une adresse du serveur (avec la connexion).
    pub async fn get_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let resp = self
            .http
            .get(format!("{}{}", self.server, path))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{path} : {}", resp.status()));
        }
        body_limited(resp, MAX_IMAGE).await
    }

    /// Nom de la bibliothèque (dossier de médias du serveur) qui contient un élément.
    pub async fn library_name(&self, id: &str) -> Option<String> {
        let v: serde_json::Value = self.get(&format!("/Items/{}/Ancestors", seg(id)), &[("userId", &self.user_id)]).await.ok()?;
        v.as_array()?
            .iter()
            .find(|a| a["Type"].as_str() == Some("CollectionFolder"))
            .and_then(|a| a["Name"].as_str())
            .filter(|n| !n.trim().is_empty())
            .map(str::to_string)
    }

    /// Fin de session côté serveur : le jeton est révoqué (compte retiré de l'appareil).
    pub async fn logout(&self) -> Result<()> {
        self.post_json("/Sessions/Logout", &serde_json::json!({})).await
    }

    /// Avatar du compte (image Primary de l'utilisateur), si défini.
    /// `original` : fichier tel quel (un GIF reste animé ; réduit par le serveur, il ne l'est plus).
    pub async fn user_avatar(&self, original: bool) -> Option<Vec<u8>> {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let size = if original { "" } else { "maxWidth=200&" };
        self.get_bytes(&format!("/Users/{}/Images/Primary?{size}t={t}", seg(&self.user_id))).await.ok()
    }

    /// Plugin GetAvatar : avatars proposés (id, nom, catégorie ; catégorie vide = ajouté à la main
    /// sur le serveur, « Uncategorized » pour le plugin). Vide si le plugin est absent.
    pub async fn avatars(&self) -> Vec<(String, String, String)> {
        let v: serde_json::Value = match self.get("/GetAvatar/Avatars", &[]).await {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| {
                Some((
                    a["Id"].as_str()?.to_string(),
                    a["Name"].as_str().unwrap_or("").to_string(),
                    a["Category"].as_str().unwrap_or("").to_string(),
                ))
            })
            .collect()
    }

    /// Secours quand le plugin GetAvatar échoue : l'image de l'avatar proposé est envoyée comme
    /// image de profil par l'API de Jellyfin (corps en base64).
    pub async fn upload_avatar(&self, avatar_id: &str) -> Result<()> {
        let resp = self
            .http
            .get(format!("{}/GetAvatar/Image/{}", self.server, seg(avatar_id)))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("image/png").to_string();
        let bytes = body_limited(resp, MAX_IMAGE).await?;
        use base64::Engine;
        let body = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let resp = self
            .http
            .post(format!("{}/UserImage?userId={}", self.server, seg(&self.user_id)))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .header("Content-Type", ct)
            .body(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&resp.status()])));
        }
        Ok(())
    }

    pub async fn set_avatar(&self, avatar_id: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/GetAvatar/SetAvatar", self.server))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(&serde_json::json!({ "AvatarId": avatar_id }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&resp.status()])));
        }
        Ok(())
    }

    /// Recherche dans la bibliothèque (films, séries, épisodes).
    pub async fn search(&self, term: &str) -> Result<Vec<Item>> {
        let r: ItemsResp = self
            .get(
                "/Items",
                &[
                    ("userId", &self.user_id),
                    ("Recursive", "true"),
                    ("searchTerm", term),
                    ("IncludeItemTypes", "Movie,Series,Episode,BoxSet"),
                    ("Limit", "60"),
                ],
            )
            .await?;
        Ok(r.items)
    }

    /// Recherche avancée : genres, tags et studios connus du serveur (triés), proposés dans les filtres.
    pub async fn filter_lists(&self) -> FilterLists {
        // `/Items/Filters2` : l'ancienne route `/Items/Filters` ne rend rien sans dossier parent.
        let f: [(&str, &str); 3] = [("userId", &self.user_id), ("includeItemTypes", "Movie,Series,BoxSet"), ("recursive", "true")];
        let s: [(&str, &str); 3] = [("userId", &self.user_id), ("SortBy", "SortName"), ("Limit", "1000")];
        let (filters, studios) = tokio::join!(self.get::<serde_json::Value>("/Items/Filters2", &f), self.get::<ItemsResp>("/Studios", &s));
        let filters = filters.unwrap_or_default();
        let list = |k: &str| {
            let mut l = filter_names(&filters[k]);
            l.sort_by_key(|a| a.to_lowercase());
            l
        };
        FilterLists {
            genres: list("Genres"),
            tags: list("Tags"),
            studios: studios.map(|r| r.items.into_iter().map(|i| i.name).collect()).unwrap_or_default(),
        }
    }

    /// Recherche avancée dans la bibliothèque : texte (facultatif), types, et pour chaque filtre « l'un
    /// de ceux choisis ». Les genres et tags à exclure sont retirés ici (le serveur n'a pas ce filtre).
    #[allow(clippy::too_many_arguments)]
    pub async fn search_filtered(
        &self,
        term: &str,
        kinds: &str,
        genres: &[String],
        tags: &[String],
        studios: &[String],
        exclude: &[String],
        more: &MoreFilters,
    ) -> Result<Vec<Item>> {
        let (g, t, s) = (genres.join("|"), tags.join("|"), studios.join("|"));
        let mut q: Vec<(&str, &str)> =
            vec![("userId", &self.user_id), ("Recursive", "true"), ("IncludeItemTypes", kinds), ("Fields", "Tags,Genres"), ("Limit", "200")];
        if term.is_empty() {
            q.push(("SortBy", "SortName"));
        } else {
            q.push(("searchTerm", term));
        }
        if !g.is_empty() {
            q.push(("Genres", &g));
        }
        if !t.is_empty() {
            q.push(("Tags", &t));
        }
        if !s.is_empty() {
            q.push(("Studios", &s));
        }
        if !more.years.is_empty() {
            q.push(("Years", &more.years));
        }
        let rating = more.min_rating.to_string();
        if more.min_rating > 0 {
            q.push(("MinCommunityRating", &rating));
        }
        match more.status {
            1 => q.push(("IsPlayed", "false")),
            2 => q.push(("IsPlayed", "true")),
            3 => q.push(("Filters", "IsFavorite")),
            _ => {}
        }
        let r: ItemsResp = self.get("/Items", &q).await?;
        let excluded = |name: &String| exclude.iter().any(|e| e.to_lowercase() == name.to_lowercase());
        Ok(r.items.into_iter().filter(|i| !i.tags.iter().chain(i.genres.iter()).any(excluded)).collect())
    }

    /// Recherche par acteur, studio et tag : (intitulé de la rangée, films et séries), meilleures notes
    /// d'abord. Les trois premiers noms qui correspondent de chaque sorte.
    pub async fn search_people(&self, term: &str) -> Vec<(String, Vec<Item>)> {
        let q: [(&str, &str); 3] = [("userId", &self.user_id), ("searchTerm", term), ("limit", "3")];
        let f: [(&str, &str); 3] = [("userId", &self.user_id), ("includeItemTypes", "Movie,Series"), ("recursive", "true")];
        let (persons, studios, filters) = tokio::join!(
            self.get::<ItemsResp>("/Persons", &q),
            self.get::<ItemsResp>("/Studios", &q),
            self.get::<serde_json::Value>("/Items/Filters2", &f),
        );
        let lower = term.to_lowercase();
        let tags: Vec<String> =
            filter_names(&filters.unwrap_or_default()["Tags"]).into_iter().filter(|t| t.to_lowercase().contains(&lower)).take(3).collect();
        let mut asked: Vec<(String, &str, String)> = Vec::new();
        for p in persons.map(|r| r.items).unwrap_or_default() {
            asked.push((trf("Avec {}", &[&p.name]), "PersonIds", p.id));
        }
        for s in studios.map(|r| r.items).unwrap_or_default() {
            asked.push((trf("Studio : {}", &[&s.name]), "StudioIds", s.id));
        }
        for t in tags {
            asked.push((trf("Tag : {}", &[&t]), "Tags", t));
        }
        let mut out = Vec::new();
        for (title, key, value) in asked {
            let r: Result<ItemsResp> = self
                .get(
                    "/Items",
                    &[
                        ("userId", &self.user_id),
                        ("Recursive", "true"),
                        ("IncludeItemTypes", "Movie,Series"),
                        ("SortBy", "CommunityRating"),
                        ("SortOrder", "Descending"),
                        ("Limit", "24"),
                        (key, &value),
                    ],
                )
                .await;
            if let Ok(r) = r {
                if !r.items.is_empty() {
                    out.push((title, r.items));
                }
            }
        }
        out
    }

    /// Un film ou une série pas encore vu, au hasard.
    pub async fn random_unwatched(&self) -> Result<Option<Item>> {
        let r: ItemsResp = self
            .get(
                "/Items",
                &[
                    ("userId", &self.user_id),
                    ("Recursive", "true"),
                    ("IncludeItemTypes", "Movie,Series"),
                    ("IsPlayed", "false"),
                    ("SortBy", "Random"),
                    ("Limit", "1"),
                ],
            )
            .await?;
        Ok(r.items.into_iter().next())
    }

    /// Seerr : résultats absents du serveur (ceux présents sont déjà dans la recherche Jellyfin).
    /// `kind` : 0 tout · 1 films · 2 séries · 3 collections (sagas de TMDB : « Le Seigneur des anneaux »…).
    pub async fn seerr_search(&self, term: &str, kind: u8) -> Vec<CardInfo> {
        let v: serde_json::Value = match self.get("/JellyfinEnhanced/jellyseerr/search", &[("query", term), ("page", "1")]).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("turtlefin : recherche Seerr « {term} » : {e:#}");
                return Vec::new();
            }
        };
        let mut out = Vec::new();
        // Collections : la recherche de Seerr (recherche « multi » de TMDB) n'en renvoie jamais. On les
        // tire des films trouvés : la fiche d'un film dans Seerr indique sa saga (`collection`).
        if kind == 3 {
            let ids: Vec<i64> = v["results"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["mediaType"].as_str() == Some("movie"))
                .filter_map(|r| r["id"].as_i64())
                .take(6)
                .collect();
            let details = futures_util::future::join_all(
                ids.iter().map(|id| async move { self.get::<serde_json::Value>(&format!("/JellyfinEnhanced/jellyseerr/movie/{id}"), &[]).await }),
            )
            .await;
            let mut seen = std::collections::HashSet::new();
            for d in details.into_iter().flatten() {
                let c = &d["collection"];
                let Some(id) = c["id"].as_i64() else { continue };
                if !seen.insert(id) {
                    continue;
                }
                let mut card = serde_json::json!({ "id": id, "mediaType": "collection", "name": c["name"], "posterPath": c["posterPath"] });
                if card["posterPath"].is_null() {
                    card["posterPath"] = d["posterPath"].clone();
                }
                out.extend(seerr_card(&card));
            }
            return out;
        }
        for r in v["results"].as_array().into_iter().flatten() {
            let wanted = match r["mediaType"].as_str() {
                Some("movie") => kind <= 1,
                Some("tv") => kind == 0 || kind == 2,
                Some("collection") => kind == 0 || kind == 3,
                _ => false,
            };
            if !wanted || r["mediaInfo"]["jellyfinMediaId"].as_str().is_some_and(|s| !s.is_empty()) {
                continue;
            }
            out.extend(seerr_card(r));
        }
        out
    }

    /// Seerr : les films d'une collection (saga), dans l'ordre de sortie. Ceux déjà sur le serveur
    /// ouvrent leur fiche, les autres leur page Seerr.
    pub async fn seerr_collection(&self, id: i64) -> Result<(String, Vec<CardInfo>)> {
        let v: serde_json::Value = self.get(&format!("/JellyfinEnhanced/jellyseerr/collection/{id}"), &[]).await?;
        let mut parts: Vec<&serde_json::Value> = v["parts"].as_array().into_iter().flatten().collect();
        parts.sort_by_key(|p| p["releaseDate"].as_str().unwrap_or("9999").to_string());
        let cards = parts
            .into_iter()
            .filter_map(|p| {
                let mut c = seerr_card(p)?;
                if let Some(jf) = p["mediaInfo"]["jellyfinMediaId"].as_str().filter(|s| !s.is_empty()) {
                    c.id = jf.to_string();
                    c.seerr = false;
                }
                Some(c)
            })
            .collect();
        Ok((v["name"].as_str().unwrap_or("").to_string(), cards))
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
        starts_with: &str,
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
        Self::library_kind(collection_type, &mut q);
        // Saisie au clavier dans la grille : titres dont le nom (de tri) commence par ces lettres.
        if !starts_with.is_empty() {
            q.push(("NameStartsWith", starts_with));
        }
        let r: ItemsResp = self.get("/Items", &q).await?;
        let total = r.total_record_count.unwrap_or(r.items.len() as u32);
        Ok((r.items, total))
    }

    /// Position, dans la grille triée par nom, du premier titre qui commence par `letter` (ou la
    /// suivante) : le nombre de titres classés avant (index A–Z de la bibliothèque).
    pub async fn library_index(&self, parent_id: &str, collection_type: Option<&str>, letter: &str) -> Result<u32> {
        let mut q: Vec<(&str, &str)> = vec![
            ("userId", &self.user_id),
            ("parentId", parent_id),
            ("Limit", "0"),
            ("NameLessThan", letter),
        ];
        Self::library_kind(collection_type, &mut q);
        let r: ItemsResp = self.get("/Items", &q).await?;
        Ok(r.total_record_count.unwrap_or(0))
    }

    /// Films ou séries seulement (pas les dossiers, saisons, épisodes...) selon le type de bibliothèque.
    fn library_kind(collection_type: Option<&str>, q: &mut Vec<(&str, &str)>) {
        match collection_type {
            Some("movies") => q.extend([("Recursive", "true"), ("IncludeItemTypes", "Movie")]),
            Some("tvshows") => q.extend([("Recursive", "true"), ("IncludeItemTypes", "Series")]),
            _ => {}
        }
    }

    /// Segments du média : (début et fin de l'intro, début du générique de fin), en secondes.
    /// Segments Jellyfin 10.10+ (que remplit le plugin Intro Skipper), sinon l'ancienne API du plugin.
    pub async fn segments(&self, id: &str) -> (Option<(f64, f64)>, Option<f64>) {
        let mut intro: Option<(f64, f64)> = None;
        let mut outro: Option<f64> = None;
        let segs = self.get::<serde_json::Value>(&format!("/MediaSegments/{}", seg(id)), &[]).await;
        if std::env::var_os("TURTLEFIN_DEBUG_UP").is_some() {
            eprintln!("turtlefin : segments du serveur : {}", match &segs {
                Ok(v) => v.to_string().chars().take(600).collect::<String>(),
                Err(e) => format!("erreur {e}"),
            });
        }
        if let Ok(v) = segs {
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
            if let Ok(v) = self.get::<serde_json::Value>(&format!("/Episode/{}/IntroSkipperSegments", seg(id)), &[]).await {
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
        let r: ItemsResp = self.get(&format!("/Items/{}/Similar", seg(id)), &[("userId", &self.user_id), ("limit", "16")]).await?;
        Ok(r.items)
    }

    /// Le compte a-t-il le droit de télécharger ?
    /// Droits du compte : (peut télécharger, administrateur du serveur).
    pub async fn rights(&self) -> (bool, bool) {
        let v: serde_json::Value = match self.get(&format!("/Users/{}", seg(&self.user_id)), &[]).await {
            Ok(v) => v,
            Err(_) => return (false, false),
        };
        let p = &v["Policy"];
        (p["EnableContentDownloading"].as_bool().unwrap_or(false), p["IsAdministrator"].as_bool().unwrap_or(false))
    }

    /// Tableau de bord (administrateurs) : réponse JSON d'une adresse du serveur.
    pub async fn admin_get(&self, path: &str, query: &[(&str, &str)]) -> Result<serde_json::Value> {
        self.get(path, query).await
    }

    /// Tableau de bord (administrateurs) : texte brut d'une adresse du serveur (journaux), 4 Mio au plus.
    pub async fn admin_text(&self, path: &str, query: &[(&str, &str)]) -> Result<String> {
        let resp = self
            .http
            .get(format!("{}{}", self.server, path))
            .query(query)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {} sur {}", &[&resp.status(), &path])));
        }
        Ok(String::from_utf8_lossy(&body_limited(resp, 4 << 20).await?).into_owned())
    }

    /// Tableau de bord (administrateurs) : envoie une commande au serveur (lancer une tâche, supprimer un
    /// appareil, redémarrer...), avec un corps JSON si besoin.
    pub async fn admin_send(&self, method: reqwest::Method, path: &str, query: &[(&str, &str)], body: Option<&serde_json::Value>) -> Result<()> {
        let mut req = self
            .http
            .request(method, format!("{}{}", self.server, path))
            .query(query)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)));
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {} sur {}", &[&resp.status(), &path])));
        }
        Ok(())
    }

    /// Administrateurs : relit les informations d'un élément sur Internet (et ses images manquantes),
    /// sans écraser ce qui a été corrigé à la main.
    pub async fn refresh_item(&self, id: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/Items/{}/Refresh", self.server, seg(id)))
            .query(&[
                ("metadataRefreshMode", "FullRefresh"),
                ("imageRefreshMode", "FullRefresh"),
                ("replaceAllMetadata", "false"),
                ("replaceAllImages", "false"),
            ])
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&resp.status()])));
        }
        Ok(())
    }

    /// Administrateurs : titres trouvés sur Internet (TMDB, TVDB...) pour identifier un film ou une série
    /// mal reconnus. `kind` : « Movie » ou « Series ». Chaque résultat est rendu tel quel à `identify_apply`.
    pub async fn identify_search(&self, id: &str, kind: &str, name: &str, year: Option<u32>) -> Result<Vec<serde_json::Value>> {
        let mut info = serde_json::json!({ "Name": name });
        if let Some(y) = year {
            info["Year"] = y.into();
        }
        let body = serde_json::json!({ "SearchInfo": info, "ItemId": id, "IncludeDisabledProviders": false });
        let resp = self
            .http
            .post(format!("{}/Items/RemoteSearch/{}", self.server, seg(kind)))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&resp.status()])));
        }
        let v: serde_json::Value = json_limited(resp).await?;
        Ok(v.as_array().cloned().unwrap_or_default())
    }

    /// Administrateurs : applique le résultat choisi (identifiants TMDB, TVDB...) ; le serveur relit alors
    /// les informations et remplace les images.
    pub async fn identify_apply(&self, id: &str, result: &serde_json::Value) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/Items/RemoteSearch/Apply/{}", self.server, seg(id)))
            .query(&[("replaceAllImages", "true")])
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(result)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&resp.status()])));
        }
        Ok(())
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
            return Err(anyhow!("{}", trf("Le serveur a répondu {} sur {}", &[&resp.status(), &path])));
        }
        Ok(())
    }

    pub async fn set_favorite(&self, id: &str, on: bool) -> Result<()> {
        let m = if on { reqwest::Method::POST } else { reqwest::Method::DELETE };
        self.send_flag(m, &format!("/UserFavoriteItems/{}", seg(id))).await
    }

    pub async fn set_played(&self, id: &str, on: bool) -> Result<()> {
        let m = if on { reqwest::Method::POST } else { reqwest::Method::DELETE };
        self.send_flag(m, &format!("/UserPlayedItems/{}", seg(id))).await
    }

    /// Liste de lecture « À regarder » du compte, créée au besoin (vide, privée).
    pub async fn watchlist_id(&self, create: bool) -> Result<Option<String>> {
        let r: ItemsResp = self
            .get("/Items", &[("userId", &self.user_id), ("Recursive", "true"), ("IncludeItemTypes", "Playlist")])
            .await?;
        if let Some(p) = r.items.into_iter().find(|p| p.name == WATCHLIST) {
            return Ok(Some(p.id));
        }
        if !create {
            return Ok(None);
        }
        let body = serde_json::json!({ "Name": WATCHLIST, "UserId": self.user_id, "Ids": [], "IsPublic": false, "MediaType": "Video" });
        let resp = self
            .http
            .post(format!("{}/Playlists", self.server))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}", trf("Le serveur a répondu {} sur {}", &[&resp.status(), &"/Playlists"])));
        }
        let v: serde_json::Value = resp.json().await?;
        Ok(v["Id"].as_str().map(str::to_string))
    }

    /// Contenu de la liste « À regarder », dans l'ordre d'inscription.
    pub async fn watchlist_items(&self, pid: &str) -> Result<Vec<Item>> {
        let r: ItemsResp = self.get(&format!("/Playlists/{}/Items", seg(pid)), &[("userId", &self.user_id)]).await?;
        Ok(r.items)
    }

    /// Dossiers qui contiennent un élément (bibliothèque comprise), du plus proche au plus lointain.
    pub async fn ancestors(&self, id: &str) -> Result<Vec<Item>> {
        self.get(&format!("/Items/{}/Ancestors", seg(id)), &[("userId", &self.user_id)]).await
    }

    /// Bandes-annonces enregistrées sur le serveur pour un film / une série.
    pub async fn local_trailers(&self, id: &str) -> Result<Vec<Item>> {
        self.get(&format!("/Items/{}/LocalTrailers", seg(id)), &[("userId", &self.user_id)]).await
    }

    /// Éléments par identifiant (séries de la liste « À regarder »).
    pub async fn items_by_ids(&self, ids: &[String]) -> Result<Vec<Item>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = ids.join(",");
        let r: ItemsResp = self.get("/Items", &[("userId", &self.user_id), ("Ids", &ids)]).await?;
        Ok(r.items)
    }

    /// Premier épisode d'une série ou d'une saison (hors épisodes spéciaux s'il y en a d'autres) :
    /// c'est lui qui représente la série dans la liste « À regarder ».
    pub async fn first_episode(&self, parent: &str) -> Result<Option<String>> {
        let r: ItemsResp = self
            .get(
                "/Items",
                &[
                    ("userId", &self.user_id),
                    ("ParentId", parent),
                    ("Recursive", "true"),
                    ("IncludeItemTypes", "Episode"),
                    ("SortBy", "ParentIndexNumber,IndexNumber"),
                    ("Limit", "40"),
                ],
            )
            .await?;
        let first = r.items.iter().find(|e| e.parent_index_number != Some(0)).or(r.items.first());
        Ok(first.map(|e| e.id.clone()))
    }

    pub async fn watchlist_add(&self, pid: &str, id: &str) -> Result<()> {
        self.send_flag(reqwest::Method::POST, &format!("/Playlists/{}/Items?ids={}", seg(pid), seg(id))).await
    }

    pub async fn watchlist_remove(&self, pid: &str, entries: &[String]) -> Result<()> {
        let list: Vec<String> = entries.iter().map(|e| seg(e).into_owned()).collect();
        self.send_flag(reqwest::Method::DELETE, &format!("/Playlists/{}/Items?entryIds={}", seg(pid), list.join(","))).await
    }

    /// Position, « vu » et date de lecture d'un élément (lectures faites hors ligne).
    pub async fn set_user_data(&self, id: &str, body: &serde_json::Value) -> Result<()> {
        let path = format!("/UserItems/{}/UserData?userId={}", seg(id), seg(&self.user_id));
        self.post_json(&path, body).await
    }

    /// Seerr : page d'un film / d'une série.
    pub async fn seerr_details(&self, tv: bool, tmdb: i64) -> Result<SeerrDetails> {
        let path = format!("/JellyfinEnhanced/jellyseerr/{}/{tmdb}", if tv { "tv" } else { "movie" });
        let d: serde_json::Value = self.get(&path, &[]).await?;
        let s = |k: &str| d[k].as_str().unwrap_or("").to_string();
        let date = if tv { s("firstAirDate") } else { s("releaseDate") };
        let names = |k: &str| {
            d[k].as_array().into_iter().flatten().filter_map(|g| g["name"].as_str()).collect::<Vec<_>>().join(", ")
        };
        let mut length: Vec<String> = Vec::new();
        if tv {
            if let Some(r) = d["episodeRunTime"].as_array().and_then(|a| a.first()).and_then(|v| v.as_i64()) {
                length.push(format!("{r} min"));
            }
            if let Some(n) = d["numberOfEpisodes"].as_i64() {
                length.push(trf("{} épisodes", &[&n]));
            }
        } else if let Some(r) = d["runtime"].as_i64().filter(|r| *r > 0) {
            length.push(if r >= 60 { format!("{}h{:02}", r / 60, r % 60) } else { format!("{r} min") });
        }
        let status_txt = match d["status"].as_str().unwrap_or("") {
            "Ended" => tr("Terminée"),
            "Returning Series" => tr("En cours de diffusion"),
            "Canceled" => tr("Annulée"),
            "In Production" => tr("En production"),
            "Released" => tr("Sorti"),
            "Post Production" => tr("Post-production"),
            "Planned" => tr("Prévu"),
            o => o,
        };
        let mut facts: Vec<(String, String)> = Vec::new();
        let mut fact = |k: &str, v: String| {
            if !v.is_empty() {
                facts.push((k.to_string(), v));
            }
        };
        fact("Statut", status_txt.to_string());
        fact(if tv { tr("Première diffusion") } else { tr("Sortie") }, fr_date(&date));
        if tv {
            fact(tr("Dernière diffusion"), fr_date(&s("lastAirDate")));
        }
        fact(tr("Langue originale"), s("originalLanguage").to_uppercase());
        fact(if tv { tr("Diffuseur") } else { tr("Studios") }, names(if tv { "networks" } else { "productionCompanies" }));
        let cast = d["credits"]["cast"]
            .as_array()
            .into_iter()
            .flatten()
            .take(20)
            .map(|c| {
                (
                    c["name"].as_str().unwrap_or("").to_string(),
                    c["character"].as_str().unwrap_or("").to_string(),
                    tmdb_img(c["profilePath"].as_str(), "w185"),
                )
            })
            .collect();
        // Saisons déjà là ou en route : statut média de la saison (2 en attente, 3 en cours,
        // 4 partielle, 5 disponible), ou saison d'une demande qui n'a pas été refusée (3).
        let mut known: Vec<i64> = d["mediaInfo"]["seasons"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|x| matches!(x["status"].as_i64(), Some(2..=5)))
            .filter_map(|x| x["seasonNumber"].as_i64())
            .collect();
        for r in d["mediaInfo"]["requests"].as_array().into_iter().flatten().filter(|r| r["status"].as_i64() != Some(3)) {
            known.extend(r["seasons"].as_array().into_iter().flatten().filter_map(|x| x["seasonNumber"].as_i64()));
        }
        let all_seasons: Vec<i64> =
            d["seasons"].as_array().into_iter().flatten().filter_map(|x| x["seasonNumber"].as_i64()).filter(|n| *n > 0).collect();
        let missing_seasons: Vec<i64> = all_seasons.iter().copied().filter(|n| !known.contains(n)).collect();
        Ok(SeerrDetails {
            tv,
            tmdb,
            missing_seasons,
            title: if tv { s("name") } else { s("title") },
            year: date.chars().take(4).collect(),
            overview: s("overview"),
            poster: tmdb_img(d["posterPath"].as_str(), "w342"),
            backdrop: tmdb_img(d["backdropPath"].as_str(), "w1280"),
            genres: names("genres"),
            length: length.join(" · "),
            rating: d["voteAverage"].as_f64().filter(|v| *v > 0.0).map(|v| format!("{v:.1}")).unwrap_or_default(),
            facts,
            status: d["mediaInfo"]["status"].as_i64().unwrap_or(0),
            jellyfin_id: d["mediaInfo"]["jellyfinMediaId"].as_str().filter(|s| !s.is_empty()).map(str::to_string),
            seasons: d["seasons"].as_array().into_iter().flatten().filter_map(|x| x["seasonNumber"].as_i64()).filter(|n| *n > 0).collect(),
            cast,
        })
    }

    /// Seerr : demande d'un film, ou de toutes les saisons d'une série.
    pub async fn seerr_request(&self, tv: bool, tmdb: i64, seasons: &[i64]) -> Result<()> {
        let mut body = serde_json::json!({ "mediaType": if tv { "tv" } else { "movie" }, "mediaId": tmdb });
        if tv {
            body["seasons"] = serde_json::json!(seasons);
        }
        let resp = self
            .http
            .post(format!("{}/JellyfinEnhanced/jellyseerr/request", self.server))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let code = resp.status();
            let txt = String::from_utf8_lossy(&body_limited(resp, 1 << 20).await.unwrap_or_default()).into_owned();
            let msg = serde_json::from_str::<serde_json::Value>(&txt)
                .ok()
                .and_then(|v| v["message"].as_str().map(str::to_string))
                .unwrap_or_else(|| code.to_string());
            return Err(anyhow!("{msg}"));
        }
        Ok(())
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
            let on_server = jf.is_some();
            let rid = r["id"].as_i64().unwrap_or(0);
            let rtv = r["mediaType"].as_str().map(|m| m == "tv").unwrap_or(tv);
            let mut sub = vec![year];
            if available {
                sub.push("disponible".into());
            }
            out.push(CardInfo {
                group: String::new(),
                id: jf.unwrap_or_else(|| seerr_id(rtv, rid)),
                title,
                subtitle: sub.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "),
                img_id: r["posterPath"].as_str().filter(|p| !p.is_empty()).map(|p| format!("https://image.tmdb.org/t/p/w342{p}")),
                img_tag: None,
                thumbs: Vec::new(),
                progress: 0.0,
                rating: r["voteAverage"].as_f64().filter(|v| *v > 0.0).map(|v| format!("{v:.1}")).unwrap_or_default(),
                seerr: !on_server,
                status: 0,
                count: 0,
                played: false,
                deck: false,
                overview: String::new(),
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
        let r: ItemsResp = self.get(&format!("/Shows/{}/Episodes", seg(series_id)), &q).await?;
        Ok(r.items)
    }

    /// Flux brut (lecture directe) lu par mpv. Le jeton passe par l'en-tête (voir `auth`, posé dans
    /// `http-header-fields` de mpv), jamais dans l'adresse.
    pub fn stream_url(&self, item_id: &str, media_source_id: &str) -> String {
        format!(
            "{}/Videos/{}/stream?static=true&mediaSourceId={}&deviceId={}",
            self.server,
            seg(item_id),
            seg(media_source_id),
            seg(&self.device_id)
        )
    }

    /// Fichier original à télécharger (lecture hors ligne). Le jeton doit passer par l'en-tête
    /// (voir `auth`) : ce point d'accès refuse `api_key` dans l'adresse (401).
    pub fn download_url(&self, item_id: &str) -> String {
        format!("{}/Items/{}/Download", self.server, seg(item_id))
    }

    /// Lecture limitée à `kbps` kbit/s : si le fichier dépasse, le serveur propose un flux HLS converti
    /// (H.264, résolution réduite selon le débit, voir `max_size`). None : le fichier passe tel quel.
    /// `audio` : piste audio à garder (index Jellyfin), le flux converti n'en contient qu'une.
    pub async fn transcode(&self, item_id: &str, media_source_id: &str, kbps: u32, audio: Option<i32>) -> Result<Option<Transcode>> {
        let bps = (kbps as u64 * 1000).to_string();
        let mut q: Vec<(&str, String)> = vec![
            ("userId", self.user_id.clone()),
            ("MaxStreamingBitrate", bps.clone()),
            ("MediaSourceId", media_source_id.to_string()),
        ];
        if let Some(a) = audio {
            q.push(("AudioStreamIndex", a.to_string()));
        }
        // mpv lit tout : lecture directe de n'importe quel fichier sous la limite, sinon H.264 en HLS.
        let profile = serde_json::json!({
            "MaxStreamingBitrate": kbps as u64 * 1000,
            "DirectPlayProfiles": [{ "Type": "Video" }, { "Type": "Audio" }],
            "TranscodingProfiles": [{
                "Container": "ts", "Type": "Video", "VideoCodec": "h264", "AudioCodec": "aac,mp3,ac3,eac3",
                "Context": "Streaming", "Protocol": "hls", "MaxAudioChannels": "6", "MinSegments": 1,
                "BreakOnNonKeyFrames": true
            }],
        });
        let resp = self
            .http
            .post(format!("{}/Items/{}/PlaybackInfo", self.server, seg(item_id)))
            .query(&q)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(&serde_json::json!({ "DeviceProfile": profile }))
            .send()
            .await?;
        if resp.status().as_u16() == 401 {
            return Err(Unauthorized.into());
        }
        let v: serde_json::Value = json_limited(resp.error_for_status()?).await?;
        let sources = v["MediaSources"].as_array().cloned().unwrap_or_default();
        let Some(ms) = sources.iter().find(|m| m["Id"].as_str() == Some(media_source_id)).or(sources.first()) else {
            return Ok(None);
        };
        if ms["SupportsDirectPlay"].as_bool() == Some(true) {
            return Ok(None);
        }
        let Some(path) = ms["TranscodingUrl"].as_str().filter(|p| p.starts_with('/')) else { return Ok(None) };
        let (w, h) = max_size(kbps);
        Ok(Some(Transcode {
            url: format!("{}{}&MaxWidth={w}&MaxHeight={h}", self.server, strip_api_key(path)),
            session: v["PlaySessionId"].as_str().unwrap_or_default().to_string(),
        }))
    }

    /// Flux pour un appareil Google Cast (lecteur par défaut) : le fichier tel quel s'il le lit (MP4 ou
    /// WebM, H.264 / VP8 / VP9, AAC / MP3 / Opus / Vorbis), sinon un flux HLS H.264 + AAC du serveur (qui
    /// recopie ce qui est déjà compatible au lieu de le convertir). Adresse complète, jeton compris :
    /// l'appareil la lit lui-même. Renvoie (adresse, type MIME).
    /// `audio` : piste audio voulue quand ce n'est pas celle par défaut du fichier (le flux est alors
    /// réassemblé par le serveur, l'image recopiée telle quelle). `burn` : sous-titres incrustés dans
    /// la vidéo (conversion complète, lente sur un petit serveur).
    pub async fn cast_stream(&self, item_id: &str, source_id: &str, audio: Option<i32>, burn: Option<i32>) -> Result<(String, String)> {
        let force = audio.is_some() || burn.is_some();
        let mut profile = serde_json::json!({
            "MaxStreamingBitrate": 40_000_000u64,
            "DirectPlayProfiles": [
                { "Type": "Video", "Container": "mp4,m4v", "VideoCodec": "h264", "AudioCodec": "aac,mp3" },
                { "Type": "Video", "Container": "webm", "VideoCodec": "vp8,vp9", "AudioCodec": "opus,vorbis" }
            ],
            "TranscodingProfiles": [{
                "Container": "ts", "Type": "Video", "VideoCodec": "h264", "AudioCodec": "aac,mp3",
                "Context": "Streaming", "Protocol": "hls", "MaxAudioChannels": "2", "MinSegments": 1,
                "BreakOnNonKeyFrames": true
            }],
        });
        if force {
            // Plus de lecture directe : le serveur doit garder la piste voulue (ou incruster les
            // sous-titres) ; l'image H.264 / HEVC est recopiée sans conversion quand c'est possible.
            profile["DirectPlayProfiles"] = serde_json::json!([]);
            profile["SubtitleProfiles"] = serde_json::json!(["srt", "subrip", "ass", "ssa", "vtt", "pgssub", "dvdsub", "dvbsub"]
                .iter()
                .map(|f| serde_json::json!({ "Format": f, "Method": "Encode" }))
                .collect::<Vec<_>>());
        }
        let audio_s = audio.map(|a| a.to_string());
        let sub_s = burn.map(|s| s.to_string()).unwrap_or_else(|| "-1".into());
        // Le serveur ne garde les pistes demandées que si la source est nommée (sinon : celles par
        // défaut du fichier).
        let mut q: Vec<(&str, &str)> =
            vec![("userId", self.user_id.as_str()), ("MediaSourceId", source_id), ("MaxStreamingBitrate", "40000000"), ("SubtitleStreamIndex", &sub_s)];
        if let Some(a) = &audio_s {
            q.push(("AudioStreamIndex", a));
        }
        let mut body = serde_json::json!({
            "DeviceProfile": profile, "UserId": self.user_id, "MediaSourceId": source_id,
            "MaxStreamingBitrate": 40_000_000u64, "SubtitleStreamIndex": burn.unwrap_or(-1),
        });
        if let Some(a) = audio {
            body["AudioStreamIndex"] = serde_json::json!(a);
        }
        let resp = self
            .http
            .post(format!("{}/Items/{}/PlaybackInfo", self.server, seg(item_id)))
            .query(&q)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("PlaybackInfo (diffuser) : {e}"))?;
        if resp.status().as_u16() == 401 {
            return Err(Unauthorized.into());
        }
        let v: serde_json::Value = json_limited(resp.error_for_status()?).await?;
        let ms = v["MediaSources"].as_array().and_then(|a| a.first()).cloned().ok_or_else(|| anyhow!("aucune source"))?;
        let source = ms["Id"].as_str().unwrap_or(item_id);
        if !force && (ms["SupportsDirectPlay"].as_bool() == Some(true) || ms["SupportsDirectStream"].as_bool() == Some(true) && ms["TranscodingUrl"].is_null()) {
            let container = ms["Container"].as_str().unwrap_or("mp4");
            let mime = if container.contains("webm") { "video/webm" } else { "video/mp4" };
            let url = format!(
                "{}/Videos/{}/stream?static=true&MediaSourceId={}&api_key={}",
                self.server,
                seg(item_id),
                seg(source),
                seg(&self.token)
            );
            return Ok((url, mime.to_string()));
        }
        let path = ms["TranscodingUrl"].as_str().filter(|p| p.starts_with('/')).ok_or_else(|| anyhow!("conversion impossible"))?;
        // L'adresse du serveur porte déjà le jeton (ApiKey) ; on le remet sous le nom attendu sinon.
        let mut url = format!("{}{}", self.server, path);
        if !url.to_ascii_lowercase().contains("api_key=") && !url.to_ascii_lowercase().contains("apikey=") {
            url.push_str(&format!("&api_key={}", seg(&self.token)));
        }
        Ok((url, "application/x-mpegURL".to_string()))
    }

    /// Arrête la conversion du serveur pour cette lecture (sinon elle tourne encore un moment pour rien).
    pub async fn stop_transcode(&self, play_session: &str) {
        let _ = self
            .http
            .delete(format!("{}/Videos/ActiveEncodings", self.server))
            .query(&[("deviceId", self.device_id.as_str()), ("playSessionId", play_session)])
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await;
    }

    /// Valeur de l'en-tête Authorization de la session.
    pub fn auth(&self) -> String {
        auth_header(&self.device_id, Some(&self.token))
    }

    /// Adresse du serveur que mpv va lire avec l'en-tête du jeton. mpv (ffmpeg) garde cet en-tête
    /// en suivant une redirection : si le serveur renvoie le flux vers un autre hôte (stockage
    /// externe, CDN, autre site), le jeton y partirait. Les redirections sont donc suivies ici
    /// (requête d'un octet) : `(adresse à lire, jeton à joindre ?)`. Même serveur : rien ne change ;
    /// autre hôte (ou passage de https à http) : l'adresse d'arrivée, sans jeton.
    pub async fn offsite(&self, url: &str) -> (String, bool) {
        static HTTP: std::sync::OnceLock<Option<reqwest::Client>> = std::sync::OnceLock::new();
        let http = HTTP.get_or_init(|| {
            let mut b = http_builder().timeout(Duration::from_secs(10)).connect_timeout(Duration::from_secs(5)).redirect(reqwest::redirect::Policy::none());
            if std::env::var("TURTLEFIN_INSECURE").is_ok() {
                b = b.danger_accept_invalid_certs(true);
            }
            b.build().ok()
        });
        let (Some(http), Ok(home)) = (http, reqwest::Url::parse(&self.server)) else { return (url.to_string(), true) };
        let mut cur = url.to_string();
        for _ in 0..10 {
            let Ok(u) = reqwest::Url::parse(&cur) else { break };
            if u.origin() != home.origin() {
                return (cur, false);
            }
            let resp = http.get(u.clone()).header("Range", "bytes=0-0").header("Authorization", self.auth()).send().await;
            let Ok(resp) = resp else { break };
            if !resp.status().is_redirection() {
                break;
            }
            let Some(next) = resp.headers().get("location").and_then(|l| l.to_str().ok()).and_then(|l| u.join(l).ok()) else { break };
            if !matches!(next.scheme(), "http" | "https") {
                // Ni file:, ni autre protocole que mpv saurait ouvrir.
                return (url.to_string(), false);
            }
            cur = next.to_string();
        }
        (cur, true)
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
            "{}/Videos/{}/{}/Subtitles/{}/Stream.{}",
            self.server,
            seg(item_id),
            seg(media_source_id),
            st.index,
            fmt
        ))
    }

    // -----------------------------------------------------------------------
    // Télécommande : cette appli se déclare pilotable ; une autre appli du compte (un téléphone) liste
    // les appareils pilotables et leur envoie des commandes (le serveur les relaie par WebSocket).
    // -----------------------------------------------------------------------

    /// Cette session accepte d'être pilotée (touches, lecture, volume, messages).
    pub async fn set_capabilities(&self) -> Result<()> {
        let body = serde_json::json!({
            "PlayableMediaTypes": ["Video"],
            "SupportedCommands": ["MoveUp", "MoveDown", "MoveLeft", "MoveRight", "Select", "Back", "GoHome",
                "VolumeUp", "VolumeDown", "SetVolume", "ToggleMute", "DisplayMessage"],
            "SupportsMediaControl": true,
            "SupportsPersistentIdentifier": true,
        });
        let resp = self
            .http
            .post(format!("{}/Sessions/Capabilities/Full", self.server))
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .json(&body)
            .send()
            .await?;
        match resp.status().as_u16() {
            401 => Err(Unauthorized.into()),
            c if !(200..300).contains(&c) => Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&c]))),
            _ => Ok(()),
        }
    }

    /// Appareils du compte que cette appli peut piloter (sauf elle-même) : (session, nom de
    /// l'appareil, appli, titre en cours, en pause).
    pub async fn remote_targets(&self) -> Result<Vec<RemoteTarget>> {
        let v: serde_json::Value = self
            .get("/Sessions", &[("controllableByUserId", &self.user_id)])
            .await?;
        let mut out = Vec::new();
        for s in v.as_array().into_iter().flatten() {
            // Pas de limite d'activité (« activeWithinSeconds ») : une télé restée sur l'accueil ne fait
            // aucune requête, seul son WebSocket vit. Pilotable = connexion ouverte (IsActive).
            if s["DeviceId"].as_str() == Some(self.device_id.as_str())
                || !s["SupportsRemoteControl"].as_bool().unwrap_or(false)
                || s["IsActive"].as_bool() == Some(false)
            {
                continue;
            }
            let (Some(id), Some(device)) = (s["Id"].as_str(), s["DeviceName"].as_str()) else { continue };
            let np = &s["NowPlayingItem"];
            let now = match (np["SeriesName"].as_str(), np["Name"].as_str()) {
                (Some(series), Some(name)) => format!("{series} · {name}"),
                (None, Some(name)) => name.to_string(),
                _ => String::new(),
            };
            out.push(RemoteTarget {
                session: id.to_string(),
                device: device.chars().take(60).collect(),
                client: s["Client"].as_str().unwrap_or("").chars().take(40).collect(),
                now_playing: now.chars().take(120).collect(),
                paused: s["PlayState"]["IsPaused"].as_bool().unwrap_or(false),
                position: s["PlayState"]["PositionTicks"].as_f64().unwrap_or(0.0) / 1e7,
                duration: np["RunTimeTicks"].as_f64().unwrap_or(0.0) / 1e7,
            });
        }
        Ok(out)
    }

    async fn post_empty(&self, path: &str, body: Option<serde_json::Value>) -> Result<()> {
        let mut req = self.http.post(format!("{}{}", self.server, path)).header("Authorization", auth_header(&self.device_id, Some(&self.token)));
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await?;
        match resp.status().as_u16() {
            401 => Err(Unauthorized.into()),
            c if !(200..300).contains(&c) => Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&c]))),
            _ => Ok(()),
        }
    }

    /// Commande générale à un appareil : MoveUp, Select, Back, GoHome, VolumeUp…
    pub async fn remote_command(&self, session: &str, name: &str) -> Result<()> {
        self.post_empty(&format!("/Sessions/{}/Command", seg(session)), Some(serde_json::json!({ "Name": name, "Arguments": {} }))).await
    }

    /// Commande de lecture : PlayPause, Stop, NextTrack, PreviousTrack, Seek (avec la position).
    pub async fn remote_playstate(&self, session: &str, command: &str, seek_secs: Option<f64>) -> Result<()> {
        let mut path = format!("/Sessions/{}/Playing/{}", seg(session), seg(command));
        if let Some(t) = seek_secs {
            path.push_str(&format!("?seekPositionTicks={}", (t.max(0.0) * 1e7) as i64));
        }
        self.post_empty(&path, None).await
    }

    /// Fichier de sous-titres (texte) de l'élément, converti par le serveur au format `ext` (« srt »,
    /// « vtt ») : adresse complète, jeton compris (l'écran de diffusion la lit lui-même).
    pub fn cast_subtitle_url(&self, item_id: &str, source_id: &str, index: i32, ext: &str) -> String {
        format!("{}/Videos/{}/{}/Subtitles/{index}/0/Stream.{ext}?api_key={}", self.server, seg(item_id), seg(source_id), seg(&self.token))
    }

    /// Lire un élément sur l'appareil (à cette position, sinon là où il en était).
    pub async fn remote_play(&self, session: &str, item_id: &str, start_secs: Option<f64>) -> Result<()> {
        let mut path = format!("/Sessions/{}/Playing?playCommand=PlayNow&itemIds={}", seg(session), seg(item_id));
        if let Some(t) = start_secs {
            path.push_str(&format!("&startPositionTicks={}", (t.max(0.0) * 1e7) as i64));
        }
        self.post_empty(&path, None).await
    }

    /// Change le mot de passe du compte. Erreur `WrongPassword` : mot de passe actuel refusé (403).
    /// Serveurs récents : `/Users/Password?userId=` ; plus anciens : `/Users/{id}/Password`.
    pub async fn change_password(&self, current: &str, new: &str) -> Result<()> {
        let body = serde_json::json!({ "CurrentPw": current, "NewPw": new, "ResetPassword": false });
        let mut last = 0;
        for url in [
            format!("{}/Users/Password?userId={}", self.server, seg(&self.user_id)),
            format!("{}/Users/{}/Password", self.server, seg(&self.user_id)),
        ] {
            let resp = self
                .http
                .post(url)
                .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
                .json(&body)
                .send()
                .await?;
            match resp.status().as_u16() {
                c if (200..300).contains(&c) => return Ok(()),
                401 => return Err(Unauthorized.into()),
                403 => return Err(WrongPassword.into()),
                // Adresse inconnue de ce serveur : l'autre forme.
                c @ (404 | 405) => last = c,
                c => return Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&c]))),
            }
        }
        Err(anyhow!("{}", trf("Le serveur a répondu {}", &[&last])))
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
            .get(format!("{}/Items/{}/Images/{}", self.server, seg(item_id), seg(kind)))
            .query(&q)
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("image {} ({}) : {}", item_id, kind, resp.status()));
        }
        let bytes = body_limited(resp, MAX_IMAGE).await?;

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
    /// Image du catalogue d'extensions ou d'un appareil (tableau de bord). Ces adresses viennent des dépôts
    /// choisis par l'administrateur : HTTPS et nom de domaine public seulement, jamais une adresse IP ni
    /// une machine du réseau local.
    pub async fn public_image(&self, url: &str) -> Result<Vec<u8>> {
        let host = url.strip_prefix("https://").and_then(|r| r.split('/').next()).unwrap_or("");
        let local = host.is_empty() || !host.contains('.') || host.ends_with(".local") || host.ends_with(".lan") || host.contains([':', '[', '@', '\\']) || host.chars().all(|c| c.is_ascii_digit() || c == '.');
        if local {
            return Err(anyhow!("image {url} : adresse refusée"));
        }
        // Le nom peut quand même désigner une machine du réseau (« 127.0.0.1.nip.io », entrée DNS
        // piégée) et une redirection mener ailleurs : le nom est résolu ici, chaque adresse doit être
        // publique, la connexion est faite à l'adresse vérifiée et aucune redirection n'est suivie.
        let name = host.split(':').next().unwrap_or(host).to_string();
        let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((name.as_str(), 443)).await?.collect();
        if addrs.is_empty() || !addrs.iter().all(|a| is_public_ip(a.ip())) {
            return Err(anyhow!("image {url} : adresse refusée"));
        }
        let http = http_builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(&name, &addrs)
            .build()?;
        let resp = http.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("image {url} : {}", resp.status()));
        }
        body_limited(resp, MAX_IMAGE).await
    }

    pub async fn image_url(&self, url: &str) -> Result<Vec<u8>> {
        // Affiches TMDB seulement : une adresse venue du serveur ne fait pas envoyer de requête à un
        // autre appareil du réseau (box, imprimante, page d'administration...).
        if !url.starts_with("https://image.tmdb.org/") || url.contains(['@', '\\']) {
            return Err(anyhow!("image {url} : adresse refusée"));
        }
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
        let bytes = body_limited(resp, MAX_IMAGE).await?;
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
    /// Plugin « Turtlefin Trailers » sur le serveur (bandes-annonces des pages Seerr) ?
    pub async fn trailers_plugin(&self) -> bool {
        self.get::<serde_json::Value>("/TurtlefinTrailers/Info", &[]).await.is_ok()
    }

    /// Bande-annonce d'un média absent du serveur, préparée par le plugin (téléchargée au besoin :
    /// quelques secondes, jusqu'à 3 min) : adresse complète du fichier à lire, et la langue trouvée.
    pub async fn prepare_trailer(&self, tv: bool, tmdb: i64, lang: &str, name: &str, year: &str, animated: bool) -> Result<(String, String)> {
        let tmdb = tmdb.to_string();
        let resp = self
            .http
            .get(format!("{}/TurtlefinTrailers/Prepare", self.server))
            .query(&[("type", if tv { "tv" } else { "movie" }), ("tmdbId", tmdb.as_str()), ("lang", lang), ("name", name)])
            // Année : seulement si elle est connue (une valeur vide ferait refuser la requête).
            .query(&[("year", year)][..usize::from(year.parse::<u16>().is_ok())])
            // Animation : le plugin écarte les adaptations en prises de vues réelles.
            .query(&[("animated", "true")][..usize::from(animated)])
            .header("Authorization", auth_header(&self.device_id, Some(&self.token)))
            .timeout(std::time::Duration::from_secs(180))
            .send()
            .await?;
        match resp.status().as_u16() {
            401 => Err(Unauthorized.into()),
            404 => Err(anyhow!("{}", tr("aucune bande-annonce pour ce titre"))),
            c if !(200..300).contains(&c) => Err(anyhow!("{}", trf("Le serveur a répondu {} sur {}", &[&c, &"/TurtlefinTrailers/Prepare"]))),
            _ => {
                let v: serde_json::Value = json_limited(resp).await?;
                let path = v["Path"].as_str().filter(|p| p.starts_with("/TurtlefinTrailers/File/")).ok_or_else(|| anyhow!("{}", tr("aucune bande-annonce pour ce titre")))?;
                Ok((format!("{}{}", self.server, path), v["Lang"].as_str().unwrap_or("").to_string()))
            }
        }
    }

    /// Seerr joignable par Jellyfin Enhanced (de quoi chercher), même si le compte n'y est pas relié
    /// (relié : voir `seerr_user`, nécessaire pour les demandes).
    pub async fn seerr_active(&self) -> bool {
        match self.get::<serde_json::Value>("/JellyfinEnhanced/jellyseerr/user-status", &[]).await {
            Ok(v) => v["active"].as_bool() == Some(true),
            Err(e) => {
                eprintln!("turtlefin : Seerr (Jellyfin Enhanced) : {e:#}");
                false
            }
        }
    }

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
                tmdb,
                available: matches!(media["status"].as_i64(), Some(5)),
                seasons: r["seasons"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|s| s["seasonNumber"].as_i64()).collect())
                    .unwrap_or_default(),
            });
        }
        Ok(out)
    }

    /// Première image disponible parmi plusieurs candidats.
    /// Taille des réponses gardées sur le disque.
    pub fn resp_cache_size() -> u64 {
        let Some(dir) = resp_cache_dir() else { return 0 };
        std::fs::read_dir(dir).map(|d| d.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
    }

    pub async fn image_first(&self, candidates: &[ImageRef], kind: &str, size: Size) -> Option<Vec<u8>> {
        for (id, tag) in candidates {
            if let Ok(b) = self.image(id, kind, tag.as_deref(), size).await {
                return Some(b);
            }
        }
        None
    }
}

/// « 2020-07-08 » -> « 8 juillet 2020 ».
fn fr_date(iso: &str) -> String {
    const MOIS: [&str; 12] =
        ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    let p: Vec<&str> = iso.get(..10).unwrap_or("").split('-').collect();
    match (p.first(), p.get(1).and_then(|m| m.parse::<usize>().ok()), p.get(2).and_then(|d| d.parse::<u32>().ok())) {
        (Some(y), Some(m), Some(d)) if (1..=12).contains(&m) => format!("{d} {} {y}", MOIS[m - 1]),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn path_segments() {
        use super::{seg, seg_dots};
        assert_eq!(seg("abc-1_2"), "abc-1_2");
        assert_eq!(seg("../x"), "%2E%2E%2Fx");
        assert_eq!(seg(""), "");
        for bad in [".", "..", "..."] {
            assert_eq!(seg(bad), "_");
            assert_eq!(seg_dots(bad), "_");
        }
        assert_eq!(seg_dots("1.2.3.0"), "1.2.3.0");
        assert_eq!(seg_dots("a/b?c"), "a%2Fb%3Fc");
        // L'adresse obtenue reste dans le dossier voulu.
        let u = reqwest::Url::parse(&format!("http://h/Items/{}/Images", seg(".."))).unwrap();
        assert_eq!(u.path(), "/Items/_/Images");
    }

    /// Faux serveur : /same redirige sur lui-même, /away vers un autre hôte, /file vers file:,
    /// le reste répond 206. Il note si l'en-tête Authorization lui arrive.
    #[tokio::test]
    async fn token_not_sent_offsite() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = l.accept().await {
                let mut b = vec![0u8; 4096];
                let n = s.read(&mut b).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&b[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("").to_string();
                let loc = match path.as_str() {
                    "/same" => Some("/final".to_string()),
                    "/away" => Some("https://cdn.example.com/v.mkv".to_string()),
                    "/file" => Some("file:///etc/passwd".to_string()),
                    _ => None,
                };
                let r = match loc {
                    Some(l) => format!("HTTP/1.1 302 Found\r\nLocation: {l}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
                    None => "HTTP/1.1 206 Partial\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx".to_string(),
                };
                let _ = s.write_all(r.as_bytes()).await;
            }
        });
        let server = format!("http://127.0.0.1:{port}");
        let c = super::Client::from_saved(&crate::config::Saved { server: server.clone(), ..Default::default() }).unwrap();
        assert_eq!(c.offsite(&format!("{server}/plain")).await, (format!("{server}/plain"), true));
        assert_eq!(c.offsite(&format!("{server}/same")).await, (format!("{server}/final"), true));
        assert_eq!(c.offsite(&format!("{server}/away")).await, ("https://cdn.example.com/v.mkv".to_string(), false));
        assert_eq!(c.offsite(&format!("{server}/file")).await.1, false);
    }

    #[test]
    fn public_ips() {
        use super::is_public_ip;
        for ip in ["1.1.1.1", "140.82.112.3", "2606:4700::1111"] {
            assert!(is_public_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["127.0.0.1", "10.0.0.1", "192.168.1.32", "172.16.0.1", "169.254.1.1", "100.100.1.1", "0.0.0.0", "::1", "fd7a:115c::1", "fe80::1", "::ffff:192.168.1.1", "224.0.0.251"] {
            assert!(!is_public_ip(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn api_key_removed() {
        let p = "/videos/x/master.m3u8?DeviceId=d&ApiKey=secret&VideoBitrate=1&api_key=s2&Tag=t";
        assert_eq!(super::strip_api_key(p), "/videos/x/master.m3u8?DeviceId=d&VideoBitrate=1&Tag=t");
        assert_eq!(super::strip_api_key("/a"), "/a");
    }

    #[test]
    fn sizes_follow_bitrate() {
        assert_eq!(super::max_size(40_000), (3840, 2160));
        assert_eq!(super::max_size(8_000), (1920, 1080));
        assert_eq!(super::max_size(4_000), (1280, 720));
        assert_eq!(super::max_size(1_000), (640, 360));
    }
}
