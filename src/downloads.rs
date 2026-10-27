//! Téléchargements (lecture hors ligne).
//!
//! Rangement dans `<données>/turtlefin/downloads/` (ou le dossier choisi dans Paramètres → Stockage),
//! comme une bibliothèque Jellyfin :
//! ```text
//! <bibliothèque>/                       (ex. « Anime »)
//!   <série>/                            poster.jpg, logo.png, backdrop.jpg : partagés par les épisodes
//!     Season 01/                        season.jpg (« Season 00 » : épisodes spéciaux)
//!       <série> S01E01.mkv              le fichier original (`/Items/{id}/Download`)
//!       <série> S01E01.json             titre, résumé, position… (écrit en dernier : sa présence
//!                                       signifie « téléchargement complet »)
//!       <série> S01E01.jpg              image de l'épisode
//!       <série> S01E01.<n>.<langue>.srt sous-titres externes
//!   <film> (<année>)/                   poster.jpg, logo.png, backdrop.jpg, thumb.jpg
//!     <film> (<année>).mkv, .json, sous-titres
//! ```
//! Les anciens téléchargements (`downloads/<id>/info.json`, `media.<ext>`, une copie des images par
//! épisode) restent lisibles, et sont rangés ainsi dès que le serveur répond (`enrich`).
//!
//! Pendant le transfert, le média s'appelle `<nom>.<ext>.part` ; un transfert interrompu (réseau
//! coupé, appli fermée) reprend là où il s'était arrêté (requête `Range`). La file d'attente est
//! gardée dans `queue.json` et reprend au lancement suivant.
//!
//! Noms et identifiants viennent du serveur : ils sont nettoyés (`fs_name`, `id_ok`) avant de
//! devenir des chemins (pas de « ../ », de séparateur ni de nom réservé de Windows).
//!
//! Le dossier choisi peut contenir d'autres fichiers de l'utilisateur : une suppression n'enlève que
//! les fichiers de Turtlefin, puis les dossiers restés vides.

use crate::i18n::{tr, trf};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
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
    /// Nom du fichier média, dans le dossier de l'élément.
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
    /// Version des métadonnées (4 : rangement par bibliothèque / série / saison).
    pub meta_v: u32,
    pub official_rating: String,
    pub season_overview: String,
    /// Favori (état du compte au téléchargement, puis changements faits sur l'appareil).
    pub favorite: bool,
    /// Bibliothèque du serveur (nom du premier dossier).
    pub library: String,
    /// Fichier de description de l'élément (`<nom>.json`, ou `info.json` à l'ancienne).
    #[serde(skip)]
    pub loc: PathBuf,
}

/// Version actuelle des métadonnées enregistrées.
const META_V: u32 = 4;

impl Entry {
    /// Ancien rangement : un dossier par élément, avec `info.json`.
    fn legacy(&self) -> bool {
        self.loc.file_name().is_some_and(|n| n == "info.json")
    }
    /// Dossier du fichier média (saison pour un épisode).
    pub fn dir(&self) -> PathBuf {
        self.loc.parent().map(Path::to_path_buf).unwrap_or_else(root)
    }
    /// Dossier des images partagées (série, ou film).
    fn meta_dir(&self) -> PathBuf {
        let d = self.dir();
        if !self.legacy() && self.kind == "Episode" {
            return d.parent().map(Path::to_path_buf).unwrap_or(d);
        }
        d
    }
    /// Nom commun des fichiers de l'élément (« Série S01E01 »).
    fn base(&self) -> String {
        self.loc.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }
    pub fn media_path(&self) -> PathBuf {
        self.dir().join(&self.media)
    }
    pub fn poster_path(&self) -> PathBuf {
        self.meta_dir().join("poster.jpg")
    }
    /// Affiche de la série (sinon celle de l'élément).
    pub fn series_poster(&self) -> PathBuf {
        if self.legacy() {
            let p = self.dir().join("series.jpg");
            if p.exists() {
                return p;
            }
        }
        self.poster_path()
    }
    /// Affiche de la saison (sinon celle de la série).
    pub fn season_poster(&self) -> PathBuf {
        let p = self.dir().join("season.jpg");
        if p.exists() { p } else { self.series_poster() }
    }
    /// Logo (série ou film), s'il y en a un.
    pub fn logo_path(&self) -> Option<PathBuf> {
        let p = self.meta_dir().join("logo.png");
        p.exists().then_some(p)
    }
    /// Image de fond (série ou film), s'il y en a une.
    pub fn backdrop_path(&self) -> Option<PathBuf> {
        let p = self.meta_dir().join("backdrop.jpg");
        p.exists().then_some(p)
    }
    pub fn thumb_path(&self) -> PathBuf {
        // Épisode : son image à lui, sinon la vignette.
        let still = if self.legacy() { self.dir().join("still.jpg") } else { self.dir().join(format!("{}.jpg", self.base())) };
        if still.exists() {
            return still;
        }
        let p = self.meta_dir().join("thumb.jpg");
        if p.exists() { p } else { self.poster_path() }
    }
}

// ---------------------------------------------------------------------------
// Noms de fichiers
// ---------------------------------------------------------------------------

/// Identifiant Jellyfin utilisable tel quel dans un chemin (lettres et chiffres seulement).
fn id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Nom de dossier / de fichier tiré d'un texte du serveur : caractères interdits (Windows compris)
/// remplacés, pas de « . » ni « .. », pas de nom réservé (CON, NUL…), longueur bornée.
pub fn fs_name(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        let c = match c {
            '<' | '>' | '"' | '|' | '?' | '*' => continue,
            ':' => ' ',
            '/' | '\\' => '-',
            c if c.is_control() => ' ',
            c => c,
        };
        if c == ' ' && (out.is_empty() || out.ends_with(' ')) {
            continue;
        }
        out.push(c);
    }
    // Au plus 80 caractères (chemins Windows), sans couper une lettre.
    if let Some((i, _)) = out.char_indices().nth(80) {
        out.truncate(i);
    }
    let out = out.trim_matches(|c: char| c == ' ' || c == '.').to_string();
    if out.is_empty() {
        return tr("Sans titre").to_string();
    }
    let stem = out.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && stem.as_bytes()[3].is_ascii_digit());
    if reserved { format!("{out}_") } else { out }
}

/// Dossier de la saison : « Season 01 », « Season 00 » pour les épisodes spéciaux.
fn season_dir(index: Option<u32>) -> String {
    format!("Season {:02}", index.unwrap_or(0))
}

/// Emplacement d'un élément : (dossier du média, nom commun de ses fichiers).
fn place(library: &str, item: &crate::api::Item) -> (PathBuf, String) {
    let lib = root().join(fs_name(library));
    if item.kind == "Episode" {
        let series = fs_name(item.series_name.as_deref().unwrap_or(&item.name));
        let base = match (item.parent_index_number, item.index_number) {
            (Some(s), Some(e)) => format!("{series} S{s:02}E{e:02}"),
            (None, Some(e)) => format!("{series} E{e:02}"),
            _ => format!("{series} - {}", fs_name(&item.name)),
        };
        return (lib.join(&series).join(season_dir(item.parent_index_number)), fs_name(&base));
    }
    let name = match item.production_year {
        Some(y) => format!("{} ({y})", item.name),
        None => item.name.clone(),
    };
    let name = fs_name(&name);
    (lib.join(&name), name)
}

/// Bibliothèque d'un élément (nom du dossier du serveur), sinon un nom selon son type.
async fn library_of(client: &Client, item: &crate::api::Item) -> String {
    match client.library_name(&item.id).await {
        Some(n) => n,
        None if item.kind == "Episode" => tr("Séries").to_string(),
        None => tr("Films").to_string(),
    }
}

// ---------------------------------------------------------------------------
// Index : identifiant -> fichier de description, gardé en mémoire (les écrans demandent souvent
// « cet élément est-il téléchargé ? »). Refait quand un fichier attendu a disparu.
// ---------------------------------------------------------------------------

static INDEX: Mutex<Option<HashMap<String, PathBuf>>> = Mutex::new(None);

fn invalidate() {
    *INDEX.lock().unwrap() = None;
}

/// Fichiers de description trouvés sous `dir` (sur `depth` niveaux), hors ceux de la racine `top`
/// (file d'attente).
fn scan_in(dir: &Path, top: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for d in rd.flatten() {
        let p = d.path();
        let Ok(ft) = d.file_type() else { continue };
        if ft.is_dir() {
            if depth > 0 {
                scan_in(&p, top, depth - 1, out);
            }
        } else if p.extension().is_some_and(|x| x == "json") && dir != top {
            out.push(p);
        }
    }
}

fn scan(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    scan_in(dir, &root(), depth, out);
}

fn read_entry(loc: &Path) -> Option<Entry> {
    let text = std::fs::read_to_string(loc).ok()?;
    let mut e: Entry = serde_json::from_str(&text).ok()?;
    if !id_ok(&e.id) || e.media.is_empty() || e.media.contains(['/', '\\']) || e.media.starts_with('.') {
        return None;
    }
    e.loc = loc.to_path_buf();
    e.media_path().exists().then_some(e)
}

/// Tous les éléments complets, avec la date de leur description.
fn all() -> Vec<(std::time::SystemTime, Entry)> {
    let mut files = Vec::new();
    let top = root();
    scan_in(&top, &top, 3, &mut files);
    let mut out = Vec::new();
    let mut index = HashMap::new();
    for f in files {
        let Some(e) = read_entry(&f) else { continue };
        index.insert(e.id.clone(), f.clone());
        let t = std::fs::metadata(&f).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        out.push((t, e));
    }
    *INDEX.lock().unwrap() = Some(index);
    out
}

/// Réécrit la description de l'élément (position, « vu », métadonnées complétées).
pub fn save(e: &Entry) {
    if let Ok(t) = serde_json::to_string_pretty(e) {
        let _ = std::fs::write(&e.loc, t);
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

/// Image enregistrée seulement si elle manque (images partagées par les épisodes d'une série).
async fn save_missing(path: &Path, fetch: impl std::future::Future<Output = Option<Vec<u8>>>) {
    if !path.exists() {
        if let Some(b) = fetch.await {
            save_bytes(path, &b).await;
        }
    }
}

/// Métadonnées et images : `item` est l'élément tel que donné par le serveur. Les images de la
/// série (ou du film) et de la saison ne sont demandées qu'une fois, pour tous les épisodes.
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
    let (dir, meta) = (e.dir(), e.meta_dir());
    let _ = tokio::fs::create_dir_all(&meta).await;
    if item.kind == "Episode" {
        if let Some(t) = item.image_tags.as_ref().and_then(|m| m.get("Primary")) {
            if let Some(b) = client.image_first(&[(item.id.clone(), Some(t.clone()))], "Primary", Size::Fill(400, 225)).await {
                save_bytes(&dir.join(format!("{}.jpg", e.base())), &b).await;
            }
        }
    } else {
        save_missing(&meta.join("thumb.jpg"), client.image_any(&item.card().thumbs, Size::Fill(400, 225))).await;
    }
    // Logo et fond : ceux de la série pour un épisode.
    let mut look = item.clone();
    if id_ok(&e.series_id) {
        if let Ok(series) = client.item(&e.series_id).await {
            e.series_overview = series.overview.clone().unwrap_or_default();
            if e.year.is_none() {
                e.year = series.production_year;
            }
            look = series;
        }
        let series_ref = [(e.series_id.clone(), item.series_primary_image_tag.clone())];
        save_missing(&meta.join("poster.jpg"), client.image_first(&series_ref, "Primary", Size::Fill(400, 600))).await;
    }
    save_missing(&meta.join("poster.jpg"), client.image_first(&item.poster_candidates(), "Primary", Size::Fill(400, 600))).await;
    if id_ok(&e.season_id) {
        save_missing(&dir.join("season.jpg"), client.image_first(&[(e.season_id.clone(), None)], "Primary", Size::Fill(400, 600))).await;
        if let Ok(season) = client.item(&e.season_id).await {
            e.season_overview = season.overview.clone().unwrap_or_default();
        }
    }
    if let Some(logo) = look.logo_candidate().or_else(|| item.logo_candidate()) {
        save_missing(&meta.join("logo.png"), client.image_first(&[logo], "Logo", Size::MaxWidth(500))).await;
    }
    save_missing(&meta.join("backdrop.jpg"), client.backdrop(&look)).await;
}

/// Téléchargement plus ancien : métadonnées complétées et fichiers rangés par bibliothèque /
/// série / saison (une fois, quand le serveur répond).
pub async fn enrich(client: &Client, id: &str) -> bool {
    // Un seul rangement à la fois (l'écran Téléchargements et la reconnexion peuvent le demander).
    static BUSY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _busy = BUSY.lock().await;
    let Some(old) = get(id) else { return false };
    if old.meta_v >= META_V {
        return false;
    }
    let Ok(item) = client.item(id).await else { return false };
    let mut e = old.clone();
    if old.legacy() {
        let library = library_of(client, &item).await;
        let Some(moved) = relocate(&old, &library, &item) else { return false };
        e = moved;
    }
    fill_meta(client, &mut e, &item).await;
    save(&e);
    invalidate();
    true
}

/// Métadonnées à compléter ou fichiers à ranger (`enrich`).
pub fn outdated(e: &Entry) -> bool {
    e.meta_v < META_V
}

/// Tous les anciens téléchargements rangés (à la connexion) ; rend le nombre d'éléments rangés.
pub async fn migrate(client: &Client) -> usize {
    let mut n = 0;
    for e in list().into_iter().filter(outdated) {
        if enrich(client, &e.id).await {
            n += 1;
        }
    }
    n
}

/// Range un ancien téléchargement (`downloads/<id>/`) à sa place, puis supprime l'ancien dossier.
fn relocate(old: &Entry, library: &str, item: &crate::api::Item) -> Option<Entry> {
    let (dir, mut base) = place(library, item);
    std::fs::create_dir_all(&dir).ok()?;
    if let Some(other) = read_entry(&dir.join(format!("{base}.json"))).filter(|o| o.id != old.id) {
        let _ = other;
        base = format!("{base} [{}]", old.id.chars().take(8).collect::<String>());
    }
    let src = old.dir();
    let ext = Path::new(&old.media).extension().map(|x| x.to_string_lossy().into_owned()).unwrap_or_else(|| "mkv".into());
    let media = format!("{base}.{ext}");
    move_file(&old.media_path(), &dir.join(&media)).ok()?;
    let mut e = old.clone();
    e.media = media;
    e.library = library.to_string();
    e.subs = old
        .subs
        .iter()
        .enumerate()
        .filter_map(|(n, (f, lang))| {
            let ext = Path::new(f).extension()?.to_string_lossy().into_owned();
            let name = sub_name(&base, n, lang, &ext);
            move_file(&src.join(f), &dir.join(&name)).ok()?;
            Some((name, lang.clone()))
        })
        .collect();
    e.loc = dir.join(format!("{base}.json"));
    // Images : celles de l'épisode à côté de lui, celles de la série / du film partagées.
    let meta = e.meta_dir();
    let keep = |from: &str, to: PathBuf| {
        let f = src.join(from);
        if f.exists() && !to.exists() {
            let _ = move_file(&f, &to);
        }
    };
    if item.kind == "Episode" {
        keep("still.jpg", dir.join(format!("{base}.jpg")));
        keep("season.jpg", dir.join("season.jpg"));
        keep("series.jpg", meta.join("poster.jpg"));
    } else {
        keep("thumb.jpg", meta.join("thumb.jpg"));
        keep("poster.jpg", meta.join("poster.jpg"));
    }
    keep("logo.png", meta.join("logo.png"));
    keep("backdrop.jpg", meta.join("backdrop.jpg"));
    save(&e);
    if src.starts_with(root()) && src != root() {
        let _ = std::fs::remove_dir_all(&src);
    }
    Some(e)
}

/// Déplace un fichier (copie puis suppression si le renommage est impossible).
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)
}

/// « Série S01E01.0.fre.srt » : numéro (plusieurs pistes d'une même langue), langue, format.
fn sub_name(base: &str, n: usize, lang: &str, ext: &str) -> String {
    let lang: String = lang.chars().filter(|c| c.is_ascii_alphanumeric()).take(12).collect();
    let ext: String = ext.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    let ext = if ext.is_empty() { "srt".to_string() } else { ext };
    if lang.is_empty() { format!("{base}.{n}.{ext}") } else { format!("{base}.{n}.{lang}.{ext}") }
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
    std::fs::read_to_string(root().join("queue.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<(String, String)>>(&t).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|(id, _)| id_ok(id))
        .collect()
}

/// Dossier des téléchargements choisi (Paramètres → Stockage), lu une fois dans les préférences.
static ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Dossier par défaut : `downloads` dans le dossier de données de Turtlefin.
pub fn default_root() -> PathBuf {
    crate::paths::data_dir()
        .map(|d| d.join("downloads"))
        .unwrap_or_else(|| PathBuf::from("downloads"))
}

pub fn root() -> PathBuf {
    let mut r = ROOT.lock().unwrap();
    if let Some(p) = r.as_ref() {
        return p.clone();
    }
    let custom = crate::config::ui_prefs().download_dir;
    let p = if custom.is_empty() { default_root() } else { PathBuf::from(custom) };
    *r = Some(p.clone());
    p
}

/// Nouveau dossier des téléchargements (vide : celui par défaut), gardé dans les préférences.
pub fn set_root(dir: Option<&Path>) {
    let mut p = crate::config::ui_prefs();
    p.download_dir = dir.map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
    crate::config::save_ui_prefs(&p);
    *ROOT.lock().unwrap() = None;
    invalidate();
}

/// « 1.4 Go », « 350 Mo ».
pub fn fmt_size(b: u64) -> String {
    if b >= 1 << 30 {
        format!("{:.1} Go", b as f64 / (1u64 << 30) as f64)
    } else {
        format!("{} Mo", b >> 20)
    }
}

/// Place sur le disque qui contient `p` (ou son plus proche dossier existant) : (libre, total), en
/// octets. Libre : ce que ce compte peut encore écrire (quotas compris).
pub fn disk_space(p: &Path) -> Option<(u64, u64)> {
    let mut at = p;
    while !at.exists() {
        at = at.parent()?;
    }
    disk_space_of(at)
}

#[cfg(windows)]
fn disk_space_of(p: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, free_to_caller: *mut u64, total: *mut u64, total_free: *mut u64) -> i32;
    }
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain([0]).collect();
    let (mut free, mut total, mut all_free) = (0u64, 0u64, 0u64);
    // SAFETY : chaîne terminée par 0, pointeurs vers des variables locales.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut all_free) };
    (ok != 0).then_some((free, total))
}

#[cfg(unix)]
fn disk_space_of(p: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(p.as_os_str().as_bytes()).ok()?;
    // SAFETY : chemin terminé par 0 ; statvfs remplit la structure ou rend une erreur.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let unit = st.f_frsize as u64;
    Some((st.f_bavail as u64 * unit, st.f_blocks as u64 * unit))
}

#[cfg(not(any(windows, unix)))]
fn disk_space_of(_: &Path) -> Option<(u64, u64)> {
    None
}

/// Place prise par les téléchargements : médias, sous-titres et transferts en cours.
pub fn used_bytes() -> u64 {
    let len = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    let mut n: u64 = list().iter().map(|e| len(&e.media_path()) + e.subs.iter().map(|(f, _)| len(&e.dir().join(f))).sum::<u64>()).sum();
    fn parts(dir: &Path, depth: u32, n: &mut u64) {
        for d in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = d.path();
            match d.file_type() {
                Ok(t) if t.is_dir() && depth > 0 => parts(&p, depth - 1, n),
                Ok(t) if t.is_file() && p.extension().is_some_and(|x| x == "part") => *n += d.metadata().map(|m| m.len()).unwrap_or(0),
                _ => {}
            }
        }
    }
    parts(&root(), 3, &mut n);
    n
}

/// Limite de place choisie (Paramètres → Stockage), en octets ; None : sans limite.
pub fn limit_bytes() -> Option<u64> {
    let gb = crate::config::ui_prefs().download_limit_gb;
    (gb > 0).then(|| u64::from(gb) << 30)
}

/// Assez de place pour `need` octets de plus ? (disque, et limite choisie). Sinon : la raison.
fn check_room(dir: &Path, need: u64) -> Result<()> {
    // Marge laissée au système (et aux images, sous-titres) : 256 Mo.
    const MARGIN: u64 = 256 << 20;
    if let Some((free, _)) = disk_space(dir) {
        if need + MARGIN > free {
            return Err(Refused(trf("pas assez de place sur le disque (il faut {}, {} libres)", &[&fmt_size(need), &fmt_size(free)])).into());
        }
    }
    if let Some(lim) = limit_bytes() {
        let used = used_bytes();
        if used + need > lim {
            return Err(Refused(trf("limite de place atteinte ({} sur {} ; Paramètres → Stockage)", &[&fmt_size(used), &fmt_size(lim)])).into());
        }
    }
    Ok(())
}

/// Fichiers partagés que Turtlefin range à côté des médias (images de la série, de la saison…).
const SHARED_FILES: [&str; 6] = ["poster.jpg", "logo.png", "backdrop.jpg", "thumb.jpg", "season.jpg", "series.jpg"];

/// `dir` ne contient plus aucun téléchargement : ses images partagées sont retirées, puis le dossier
/// s'il est vide (d'autres fichiers de l'utilisateur le gardent). Rend vrai s'il a disparu.
fn prune(dir: &Path) -> bool {
    for f in SHARED_FILES {
        let _ = std::fs::remove_file(dir.join(f));
    }
    std::fs::remove_dir(dir).is_ok()
}

/// Déplace tous les téléchargements de `from` vers `to` (mêmes sous-dossiers), transferts en cours
/// et file d'attente compris. Rend (éléments déplacés, éléments restés en place). À lancer hors
/// du fil de l'interface (copie d'un disque à l'autre : plusieurs minutes).
pub fn move_all(from: &Path, to: &Path) -> (usize, usize) {
    let (mut moved, mut failed) = (0, 0);
    let mut files = Vec::new();
    scan_in(from, from, 3, &mut files);
    let mut touched: Vec<PathBuf> = Vec::new();
    for f in files {
        let Some(e) = read_entry(&f) else { continue };
        let Ok(rel) = e.dir().strip_prefix(from).map(Path::to_path_buf) else { continue };
        // Le dossier de l'élément et ceux au-dessus de lui, jusqu'à la racine (bibliothèque, série).
        let mut d = e.dir();
        while d.starts_with(from) && d != from {
            touched.push(d.clone());
            match d.parent() {
                Some(p) => d = p.to_path_buf(),
                None => break,
            }
        }
        let dest = to.join(&rel);
        let ok = std::fs::create_dir_all(&dest).is_ok() && {
            let base = e.base();
            let mut names: Vec<String> = vec![e.media.clone()];
            names.extend(e.subs.iter().map(|(n, _)| n.clone()));
            if e.legacy() {
                names.extend(["still.jpg", "series.jpg", "season.jpg", "poster.jpg", "thumb.jpg", "logo.png", "backdrop.jpg"].map(String::from));
            } else {
                names.push(format!("{base}.jpg"));
            }
            // Le média et les sous-titres d'abord, la description en dernier (sa présence signifie
            // « complet »).
            let all_ok = names.iter().all(|n| {
                let src = e.dir().join(n);
                !src.exists() || move_file(&src, &dest.join(n)).is_ok()
            });
            all_ok && move_file(&e.loc, &dest.join(e.loc.file_name().unwrap_or_default())).is_ok()
        };
        if ok { moved += 1 } else { failed += 1 }
    }
    // Restes des dossiers qui avaient un téléchargement (images partagées, transferts en cours),
    // puis ces dossiers s'ils sont vides, du plus profond au moins profond ; la file d'attente.
    touched.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    touched.dedup();
    for d in &touched {
        let Ok(rel) = d.strip_prefix(from) else { continue };
        for f in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let name = f.file_name().to_string_lossy().into_owned();
            let p = f.path();
            if p.is_file() && (SHARED_FILES.contains(&name.as_str()) || name.ends_with(".part")) {
                let target = to.join(rel).join(&name);
                if !target.exists() && std::fs::create_dir_all(to.join(rel)).is_ok() {
                    let _ = move_file(&p, &target);
                }
            }
        }
        let _ = std::fs::remove_dir(d);
    }
    let q = from.join("queue.json");
    if q.exists() && !to.join("queue.json").exists() {
        let _ = move_file(&q, &to.join("queue.json"));
    }
    let _ = std::fs::remove_dir(from);
    invalidate();
    (moved, failed)
}

/// Éléments complets rangés sous `dir` (autre dossier que celui en cours).
pub fn list_in(dir: &Path) -> Vec<Entry> {
    let mut files = Vec::new();
    scan_in(dir, dir, 3, &mut files);
    files.iter().filter_map(|f| read_entry(f)).collect()
}

/// Éléments complètement téléchargés, du plus récent au plus ancien.
pub fn list() -> Vec<Entry> {
    let mut out = all();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, e)| e).collect()
}

pub fn get(id: &str) -> Option<Entry> {
    if !id_ok(id) {
        return None;
    }
    let cached = INDEX.lock().unwrap().as_ref().map(|m| m.get(id).cloned());
    let loc = match cached {
        Some(loc) => loc,
        None => {
            all();
            INDEX.lock().unwrap().as_ref().and_then(|m| m.get(id).cloned())
        }
    }?;
    match read_entry(&loc) {
        Some(e) if e.id == id => Some(e),
        // Fichier déplacé ou supprimé entre-temps : l'index est refait.
        _ => {
            invalidate();
            None
        }
    }
}

pub fn exists(id: &str) -> bool {
    get(id).is_some()
}

/// Supprime un élément ; les images de la saison, de la série (ou du film) partent avec le dernier
/// élément qui les utilisait.
pub fn remove(id: &str) {
    let Some(e) = get(id) else { return };
    invalidate();
    if e.legacy() {
        let dir = e.dir();
        if dir.starts_with(root()) && dir != root() {
            let _ = std::fs::remove_dir_all(dir);
        }
        return;
    }
    let dir = e.dir();
    let base = e.base();
    let _ = std::fs::remove_file(e.media_path());
    for (f, _) in &e.subs {
        let _ = std::fs::remove_file(dir.join(f));
    }
    let _ = std::fs::remove_file(dir.join(format!("{base}.jpg")));
    let _ = std::fs::remove_file(&e.loc);
    // Dossiers devenus inutiles (plus aucun élément dedans), jusqu'à la bibliothèque : seuls les
    // fichiers de Turtlefin en sont retirés, un dossier qui garde autre chose reste.
    let mut d = dir;
    while d.starts_with(root()) && d != root() {
        let mut files = Vec::new();
        scan(&d, 2, &mut files);
        if files.iter().any(|f| read_entry(f).is_some()) || has_partial(&d) || !prune(&d) {
            break;
        }
        match d.parent() {
            Some(p) => d = p.to_path_buf(),
            None => break,
        }
    }
}

/// Un transfert en cours (fichier `.part`) dans ce dossier ou en dessous.
fn has_partial(dir: &Path) -> bool {
    std::fs::read_dir(dir).into_iter().flatten().flatten().any(|d| {
        let p = d.path();
        if p.is_dir() { has_partial(&p) } else { p.extension().is_some_and(|x| x == "part") }
    })
}

async fn save_bytes(path: &Path, bytes: &[u8]) {
    let _ = tokio::fs::write(path, bytes).await;
}

/// Télécharge un film / un épisode. `progress(fraction)` est appelé au fil du transfert.
pub async fn download(client: &Client, id: &str, progress: impl Fn(f32)) -> Result<()> {
    if !id_ok(id) {
        return Err(Refused(tr("élément inconnu").to_string()).into());
    }
    if exists(id) {
        return Ok(());
    }
    let item = client.item(id).await?;
    if item.id != id {
        return Err(Refused(tr("élément inconnu").to_string()).into());
    }
    let library = library_of(client, &item).await;
    let (dir, mut base) = place(&library, &item);
    tokio::fs::create_dir_all(&dir).await?;
    // Deux éléments du même nom (même série en double sur le serveur…) : le second est distingué.
    if read_entry(&dir.join(format!("{base}.json"))).is_some_and(|o| o.id != item.id) {
        base = format!("{base} [{}]", item.id.chars().take(8).collect::<String>());
    }

    // Sous-titres externes (jeton dans l'en-tête, comme pour le média).
    let source = item.media_sources.as_ref().and_then(|v| v.first());
    let ms_id = source.map(|m| m.id.clone()).unwrap_or_else(|| item.id.clone());
    let mut subs: Vec<(String, String)> = Vec::new();
    let http = crate::api::http_builder().redirect(crate::api::redirect_policy()).connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(60)).build()?;
    for (n, st) in source.into_iter().flat_map(|s| s.media_streams.iter()).filter(|s| s.kind == "Subtitle" && s.is_external).enumerate() {
        let Some(url) = client.subtitle_url(&item.id, &ms_id, st) else { continue };
        let ext = url.rsplit('.').next().unwrap_or("srt");
        let Ok(r) = http.get(&url).header("Authorization", client.auth()).send().await else { continue };
        if !r.status().is_success() {
            continue;
        }
        if let Ok(b) = crate::api::body_limited(r, 16 << 20).await {
            let lang = st.language.clone().unwrap_or_default();
            let name = sub_name(&base, n, &lang, ext);
            save_bytes(&dir.join(&name), &b).await;
            subs.push((name, lang));
        }
    }

    // Le média, par morceaux, dans un fichier provisoire.
    let ext = source
        .and_then(|s| s.container.clone())
        .and_then(|c| c.split(',').next().map(str::to_string))
        .filter(|c| !c.is_empty() && c.len() <= 8 && c.chars().all(|ch| ch.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "mkv".to_string());
    let media = format!("{base}.{ext}");
    let part = dir.join(format!("{media}.part"));
    // Transfert interrompu : on reprend après ce qui est déjà sur le disque.
    let have = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
    // Assez de place (disque, limite choisie) pour ce qui reste à recevoir ?
    let size = source.and_then(|s| s.size).unwrap_or(0);
    {
        let (dir, need) = (dir.clone(), size.saturating_sub(have));
        tokio::task::spawn_blocking(move || check_room(&dir, need)).await??;
    }
    let mut req = http.get(client.download_url(&item.id)).header("Authorization", client.auth());
    if have > 0 {
        req = req.header("Range", format!("bytes={have}-"));
    }
    // Pas de limite de durée pour le média lui-même (plusieurs gigaoctets) : seulement le silence.
    let req = req.timeout(Duration::from_secs(24 * 3600));
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
        library,
        loc: dir.join(format!("{base}.json")),
        ..Default::default()
    };
    if let Some(ud) = &item.user_data {
        entry.position = ud.playback_position_ticks as f64 / 1e7;
        entry.played = ud.played;
    }
    fill_meta(client, &mut entry, &item).await;
    tokio::fs::write(&entry.loc, serde_json::to_string_pretty(&entry)?).await?;
    invalidate();
    progress(1.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(fs_name("Cowboy Bebop"), "Cowboy Bebop");
        assert_eq!(fs_name("../../etc/passwd"), "-..-etc-passwd");
        assert_eq!(fs_name("Re:Zero"), "Re Zero");
        assert_eq!(fs_name("What?*<>|\""), "What");
        assert_eq!(fs_name("CON"), "CON_");
        assert_eq!(fs_name("com1.txt"), "com1.txt_");
        assert_eq!(fs_name("  ..  "), tr("Sans titre"));
        assert_eq!(fs_name("x".repeat(300).as_str()).chars().count(), 80);
        assert_eq!(season_dir(Some(1)), "Season 01");
        assert_eq!(season_dir(Some(0)), "Season 00");
        assert_eq!(season_dir(Some(12)), "Season 12");
        assert!(!id_ok("../x"));
        assert!(id_ok("13562385d903c96cf4c7f20f40768987"));
        assert_eq!(sub_name("B S01E01", 0, "fre", "srt"), "B S01E01.0.fre.srt");
        assert_eq!(sub_name("B", 1, "../", "s/rt?x"), "B.1.srtx");
    }

    #[test]
    fn move_and_prune() {
        // Un film déplacé d'un dossier à l'autre ; un fichier de l'utilisateur reste où il est.
        let base = std::env::temp_dir().join(format!("tf-dl-{}", std::process::id()));
        let (from, to) = (base.join("a"), base.join("b"));
        let dir = from.join("Films").join("Akira (1988)");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Akira (1988).mkv"), b"film").unwrap();
        std::fs::write(dir.join("poster.jpg"), b"img").unwrap();
        std::fs::write(from.join("Films").join("perso.txt"), b"a moi").unwrap();
        let e = Entry { id: "a3".into(), kind: "Movie".into(), media: "Akira (1988).mkv".into(), ..Default::default() };
        std::fs::write(dir.join("Akira (1988).json"), serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(move_all(&from, &to), (1, 0));
        let moved = to.join("Films").join("Akira (1988)");
        assert!(moved.join("Akira (1988).mkv").exists() && moved.join("Akira (1988).json").exists() && moved.join("poster.jpg").exists());
        assert!(!dir.exists());
        assert!(from.join("Films").join("perso.txt").exists());
        // Un dossier qui garde un fichier de l'utilisateur n'est pas supprimé.
        std::fs::write(moved.join("notes.txt"), b"a moi").unwrap();
        assert!(!prune(&moved));
        assert!(moved.join("notes.txt").exists() && !moved.join("poster.jpg").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn places() {
        let ep: crate::api::Item = serde_json::from_value(serde_json::json!({
            "Id": "a1", "Name": "Session 1", "Type": "Episode", "IsFolder": false,
            "SeriesName": "Cowboy Bebop", "ParentIndexNumber": 1, "IndexNumber": 1
        }))
        .unwrap();
        let (dir, base) = place("Anime", &ep);
        assert!(dir.ends_with(Path::new("Anime").join("Cowboy Bebop").join("Season 01")));
        assert_eq!(base, "Cowboy Bebop S01E01");
        let sp: crate::api::Item = serde_json::from_value(serde_json::json!({
            "Id": "a2", "Name": "Special", "Type": "Episode", "IsFolder": false,
            "SeriesName": "Cowboy Bebop", "ParentIndexNumber": 0, "IndexNumber": 3
        }))
        .unwrap();
        let (dir, base) = place("Anime", &sp);
        assert!(dir.ends_with("Season 00"));
        assert_eq!(base, "Cowboy Bebop S00E03");
        let mv: crate::api::Item = serde_json::from_value(serde_json::json!({
            "Id": "a3", "Name": "Akira", "Type": "Movie", "IsFolder": false, "ProductionYear": 1988
        }))
        .unwrap();
        let (dir, base) = place("Films", &mv);
        assert!(dir.ends_with(Path::new("Films").join("Akira (1988)")));
        assert_eq!(base, "Akira (1988)");
    }
}
