mod api;
mod config;
mod discovery;
mod downloads;
mod mpv;
mod player;
mod syncplay;
mod video;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use slint::{ComponentHandle, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};

slint::include_modules!();

// ---------------------------------------------------------------------------
// Ligne de commande :
//   turtlefin                              -> écran de connexion (ou session sauvegardée)
//   turtlefin "Cody" "mot de passe" --tv   -> connexion automatique, plein écran
//   options : --tv | --desktop | --server=URL
//   le mot de passe peut aussi venir de la variable TURTLEFIN_PASSWORD
//   (un argument de ligne de commande est visible par les autres processus).
// ---------------------------------------------------------------------------
struct Cli {
    user: Option<String>,
    pass: Option<String>,
    server: Option<String>,
    tv: Option<bool>,
    /// Lecture d'essai sans serveur : --test-video=FICHIER_OU_URL.
    test_video: Option<String>,
    /// Lecture directe d'un élément du serveur (essais) : --play=ID[@SECONDES].
    play: Option<String>,
}

fn parse_cli() -> Cli {
    let mut positional: Vec<String> = Vec::new();
    let mut cli = Cli { user: None, pass: None, server: None, tv: None, test_video: None, play: None };

    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--tv" => cli.tv = Some(true),
            "--desktop" => cli.tv = Some(false),
            s if s.starts_with("--server=") => cli.server = Some(s["--server=".len()..].to_string()),
            s if s.starts_with("--test-video=") => cli.test_video = Some(s["--test-video=".len()..].to_string()),
            s if s.starts_with("--play=") => cli.play = Some(s["--play=".len()..].to_string()),
            s if s.starts_with("--open=") => cli.play = Some(format!("open:{}", &s["--open=".len()..])),
            s if s.starts_with("--") => eprintln!("Option inconnue : {s}"),
            _ => positional.push(a.clone()),
        }
    }

    cli.user = positional.first().cloned();
    cli.pass = positional
        .get(1)
        .cloned()
        .or_else(|| std::env::var("TURTLEFIN_PASSWORD").ok());
    cli
}

// ---------------------------------------------------------------------------
// État partagé entre l'interface et les tâches réseau
// ---------------------------------------------------------------------------
struct App {
    rt: tokio::runtime::Handle,
    // Dans un Mutex pour que App reste Send + Sync.
    ui: Mutex<slint::Weak<AppWindow>>,
    client: Mutex<Option<api::Client>>,
    /// Pile de navigation des fiches (id des éléments ouverts).
    stack: Mutex<Vec<String>>,
    /// Numéro de la dernière fiche demandée : ignore les réponses périmées.
    gen: AtomicU64,
    device_id: String,
    /// Mode TV (cartes plus grandes : sert au calcul des coins arrondis des images). Réglable.
    tv_flag: AtomicBool,
    /// Onglet de l'accueil affiché : "home", "favorites" ou "requests".
    tab: Mutex<String>,
    /// Identifiant Seerr de l'utilisateur (onglet Demandes), si Seerr est disponible.
    seerr_user: Mutex<Option<i64>>,
    /// Bibliothèque affichée en grille : (id, type de collection, éléments chargés, total).
    library: Mutex<Option<(String, Option<String>, u32, u32)>>,
    /// Une page de bibliothèque est en cours de chargement.
    lib_loading: AtomicBool,
    /// Poster sélectionné dans une bibliothèque au moment d'ouvrir une fiche : (bibliothèque, index),
    /// pour y revenir au retour.
    lib_return: Mutex<Option<(String, usize)>>,
    /// Fiche affichée : clé de préférences et pistes (langue, libellé) audio / sous-titres.
    detail_streams: Mutex<(String, Vec<(String, String)>, Vec<(String, String)>)>,
    /// Fiche d'une série incomplète : (id Jellyfin, id TMDB, saisons manquantes à demander).
    missing_seasons: Mutex<(String, i64, Vec<i64>)>,
    /// Watch party (SyncPlay) : état du groupe, connexion déjà ouverte.
    sp: syncplay::Shared,
    sp_connected: AtomicBool,
    /// Configuration du compte Jellyfin (Paramètres > Lecture).
    user_cfg: Mutex<serde_json::Value>,
    /// Avatars GetAvatar proposés : (id, nom).
    avatars: Mutex<Vec<(String, String)>>,
    /// Dernière recherche lancée (les réponses plus anciennes sont ignorées).
    search_gen: AtomicU64,
    /// Page Seerr affichée.
    seerr_page: Mutex<Option<api::SeerrDetails>>,
    /// Le compte peut télécharger (bouton Télécharger des fiches).
    can_download: AtomicBool,
    /// File de téléchargement : (id, titre) en attente, et celui en cours (id, titre, avancement).
    dl_queue: Mutex<std::collections::VecDeque<(String, String)>>,
    dl_current: Mutex<Option<(String, String, f32)>>,
    /// Pas de serveur joignable : seuls les téléchargements sont accessibles.
    offline: AtomicBool,
    /// D'où part la pile de pages : "downloads" (fiches des téléchargements) ou l'accueil.
    stack_base: Mutex<String>,
    /// Élément dont les rangées du bas de la fiche sont affichées (série d'un épisode...).
    rows_base: Mutex<String>,
    /// Empreinte de l'image de fond affichée : la même image ne refait pas de fondu.
    bg_hash: AtomicU64,
    /// Bibliothèques du compte (entrées du menu).
    views: Mutex<Vec<api::Item>>,
    /// Fond d'écran : média affiché et génération (une demande plus récente annule les autres).
    bg_id: Mutex<String>,
    bg_gen: AtomicU64,
    /// Fin de la dernière lecture : (position, durée, fin atteinte), pour les lectures hors ligne.
    last_play: Mutex<Option<(f64, f64, bool)>>,
    /// Essais : élément à lancer au démarrage (--play) et position de départ forcée.
    play_arg: Mutex<Option<String>>,
    play_start: Mutex<Option<f64>>,
    /// Serveurs trouvés par la dernière recherche.
    found: Mutex<Vec<discovery::Found>>,
    /// Serveur choisi, en attente de connexion : (adresse locale, adresse distante).
    pending_server: Mutex<Option<(String, String)>>,
    /// Saisie manuelle en cours : adresses déjà validées et celle qui attend un choix http/https.
    manual: Mutex<(Option<String>, Option<String>, Vec<String>)>,
    /// Une lecture est en cours (évite les doubles lancements).
    playing: AtomicBool,
    /// L'accueil doit être rechargé au retour (état « Reprendre » modifié par une lecture).
    home_stale: AtomicBool,
    /// Canal vers la lecture en cours (touches clavier -> commandes mpv).
    player_tx: Mutex<Option<tokio::sync::mpsc::UnboundedSender<String>>>,
}

impl App {
    fn tv(&self) -> bool {
        self.tv_flag.load(Ordering::Relaxed)
    }

    fn ui(&self) -> slint::Weak<AppWindow> {
        self.ui.lock().unwrap().clone()
    }
    fn client(&self) -> Option<api::Client> {
        self.client.lock().unwrap().clone()
    }
}

// ---------------------------------------------------------------------------
// Utilitaires
// ---------------------------------------------------------------------------
fn show_login_error(ui: &slint::Weak<AppWindow>, msg: String) {
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_busy(false);
        u.set_error_text(msg.into());
        u.set_screen("login".into());
    });
}

/// Chargement d'une page : l'écran affiché reste visible sous une barre de progression (sauf depuis
/// la connexion, où l'écran « Chargement » prend le relais).
fn begin_loading(u: &AppWindow) {
    match u.get_screen().as_str() {
        "login" => u.set_screen("loading".into()),
        // D'une fiche à une autre : la fiche affichée reste jusqu'à l'échange (rien ne s'efface).
        "detail" => {}
        _ => u.set_loading(true),
    }
}

/// Affiche une page avec son animation d'arrivée, même si c'est le même écran (fiche -> fiche).
fn show_screen(u: &AppWindow, name: &str) {
    u.set_loading(false);
    if u.get_screen().as_str() == name && name == "detail" {
        u.invoke_detail_swap();
    } else if u.get_screen().as_str() == name {
        u.invoke_page_enter();
    } else {
        u.set_screen(name.into());
    }
}

/// Pose une fiche en gardant ce qui ne change pas : logo et affiche identiques (même élément
/// d'origine) sont repris tels quels ; une affiche différente fait un fondu enchaîné avec l'ancienne.
fn set_detail_smooth(u: &AppWindow, mut new: DetailData) {
    let old = u.get_detail();
    let on_detail = u.get_screen().as_str() == "detail";
    if on_detail && old.has_logo && !new.logo_key.is_empty() && old.logo_key == new.logo_key && !new.has_logo {
        new.logo = old.logo.clone();
        new.has_logo = true;
        new.expect_logo = true;
    }
    if on_detail && old.has_poster && !new.poster_key.is_empty() && old.poster_key == new.poster_key && !new.has_poster {
        new.poster = old.poster.clone();
        new.has_poster = true;
    } else if on_detail && old.has_poster && old.poster_key != new.poster_key {
        u.set_d_poster_old(old.poster.clone());
        u.set_d_poster_swap(!new.has_poster);
    }
    u.set_detail(new);
}

/// Signature d'une liste de boutons (seuls les changements réels s'animent).
fn buttons_sig(b: &[(String, String, String, bool)], icons: bool) -> String {
    b.iter().filter(|x| x.2.is_empty() != icons).map(|x| format!("{}|{}|{}", x.0, x.1, x.2)).collect::<Vec<_>>().join(";")
}

fn handle_error(ui: &slint::Weak<AppWindow>, e: anyhow::Error) {
    let _ = ui.upgrade_in_event_loop(|u| u.set_loading(false));
    if e.downcast_ref::<api::Unauthorized>().is_some() {
        config::clear_token();
    }
    show_login_error(ui, format!("{e}"));
}

/// Forme finale d'une image de carte : taille exacte et coins arrondis intégrés aux pixels.
///
/// Sur le moteur OpenGL de Slint (femtovg), un élément arrondi qui rogne son contenu est dessiné
/// hors écran puis recollé, à chaque image : avec des dizaines de cartes, le défilement saccade
/// sur le Pi. Les images sont donc préparées une fois pour toutes au décodage.
#[derive(Clone, Copy)]
struct Shape {
    w: u32,
    h: u32,
    /// Rayon des coins, en fraction de la largeur (rayon affiché / largeur affichée).
    radius: f32,
    /// Seulement les coins du haut (image en tête d'une carte, le bas est l'encadré du titre).
    top_only: bool,
}

impl Shape {
    /// Image affichée sur `display_w` pixels logiques avec des coins de 10 px (Theme.radius).
    fn card(w: u32, h: u32, display_w: f32) -> Shape {
        Shape { w, h, radius: 10.0 / display_w, top_only: false }
    }

    /// Image en tête de carte : coins du haut arrondis, ceux du bas droits.
    fn card_top(w: u32, h: u32, display_w: f32) -> Shape {
        Shape { top_only: true, ..Shape::card(w, h, display_w) }
    }
}

/// Rend transparents (avec lissage) les coins arrondis de rayon `r` pixels (les deux du haut seulement
/// si `top_only`).
fn round_corners(img: &mut image::RgbaImage, r: f32, top_only: bool) {
    let (w, h) = img.dimensions();
    let n = (r.ceil() as u32).min(w / 2).min(h / 2);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (r - (x as f32 + 0.5), r - (y as f32 + 0.5));
            if dx <= 0.0 || dy <= 0.0 {
                continue;
            }
            let coverage = (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            if coverage >= 1.0 {
                continue;
            }
            let corners = [(x, y), (w - 1 - x, y), (x, h - 1 - y), (w - 1 - x, h - 1 - y)];
            for &(px, py) in &corners[..if top_only { 2 } else { 4 }] {
                let p = img.get_pixel_mut(px, py);
                p[3] = (p[3] as f32 * coverage).round() as u8;
            }
        }
    }
}

/// Fond d'écran : image réduite, floutée et assombrie une seule fois au décodage.
fn decode_backdrop(bytes: &[u8]) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let img = image::load_from_memory(bytes).ok()?.resize_to_fill(480, 270, image::imageops::FilterType::Triangle);
    let mut img = image::imageops::blur(&img.to_rgba8(), 6.0);
    for p in img.pixels_mut() {
        for c in 0..3 {
            p[c] = (p[c] as f32 * 0.45) as u8;
        }
    }
    let (w, h) = img.dimensions();
    Some(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), w, h))
}

fn decode(bytes: &[u8], shape: Option<Shape>) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let img = image::load_from_memory(bytes).ok()?;
    Some(match shape {
        Some(s) => shaped(img, s),
        None => {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), w, h)
        }
    })
}

/// Recadrage au format exact (comme image-fit: cover), puis coins arrondis.
fn shaped(img: image::DynamicImage, s: Shape) -> SharedPixelBuffer<Rgba8Pixel> {
    let mut img = img.resize_to_fill(s.w, s.h, image::imageops::FilterType::Triangle).to_rgba8();
    round_corners(&mut img, s.radius * s.w as f32, s.top_only);
    let (w, h) = img.dimensions();
    SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), w, h)
}

// ---------------------------------------------------------------------------
// Avatars animés (GIF) : toutes les images sont décodées une fois, puis une minuterie les fait
// défiler. Réglage « Affichage » : GIF figés sur leur première image.
// ---------------------------------------------------------------------------
static STILL_GIFS: AtomicBool = AtomicBool::new(false);
/// Réglage « Affichage » : pas de fond d'écran tiré du média sélectionné.
static NO_BACKDROP: AtomicBool = AtomicBool::new(false);

/// Nouvelle image de fond ? (false : la même que celle affichée, rien à faire).
fn same_backdrop(app: &Arc<App>, bytes: &[u8]) -> bool {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    let v = h.finish();
    app.bg_hash.swap(v, Ordering::SeqCst) != v
}

/// Fond d'écran du média sélectionné : chargé après une courte pause (pas à chaque carte survolée),
/// flouté et assombri au décodage, puis affiché en fondu enchaîné.
fn set_backdrop(app: &Arc<App>, id: String) {
    if id.is_empty() || NO_BACKDROP.load(Ordering::Relaxed) {
        return;
    }
    if id.starts_with("dl:") {
        let rid = real_id(&id).to_string();
        let path = downloads::list()
            .into_iter()
            .find(|e| e.id == rid || e.series_id == rid || e.season_id == rid)
            .and_then(|e| e.backdrop_path());
        if let Some(p) = path {
            set_local_backdrop(app, id, p);
        }
        return;
    }
    {
        let mut cur = app.bg_id.lock().unwrap();
        if *cur == id {
            return;
        }
        *cur = id.clone();
    }
    let my = app.bg_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let Some(client) = app.client() else { return };
    let a = app.clone();
    app.rt.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        if a.bg_gen.load(Ordering::SeqCst) != my {
            return;
        }
        let Ok(item) = client.item(&id).await else { return };
        let Some(bytes) = client.backdrop(&item).await else { return };
        if !same_backdrop(&a, &bytes) {
            return;
        }
        let Some(buf) = tokio::task::spawn_blocking(move || decode_backdrop(&bytes)).await.ok().flatten() else { return };
        if a.bg_gen.load(Ordering::SeqCst) != my {
            return;
        }
        let _ = a.ui().upgrade_in_event_loop(move |u| {
            // L'image va dans le calque caché, puis les deux calques s'échangent en fondu.
            let img = slint::Image::from_rgba8(buf);
            if u.get_bg_flip() {
                u.set_bg_a(img);
                u.set_bg_flip(false);
            } else {
                u.set_bg_b(img);
                u.set_bg_flip(true);
            }
            u.set_bg_show(true);
        });
    });
}

/// Images d'une animation et durée de chacune (ms). Une seule image : image fixe.
type Frames = Vec<(SharedPixelBuffer<Rgba8Pixel>, u32)>;

/// Au plus 150 images (avatar de 160 px : 100 Ko par image, 15 Mo au pire).
const MAX_FRAMES: usize = 150;

/// Comme `decode`, en gardant toutes les images d'un GIF animé (sauf si les GIF sont figés).
fn decode_frames(bytes: &[u8], shape: Shape) -> Option<Frames> {
    use image::AnimationDecoder;
    if bytes.starts_with(b"GIF8") && !STILL_GIFS.load(Ordering::Relaxed) {
        if let Ok(dec) = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes)) {
            let mut out = Frames::new();
            for f in dec.into_frames().take(MAX_FRAMES) {
                let Ok(f) = f else { break };
                let (n, d) = f.delay().numer_denom_ms();
                let ms = if d == 0 { 0 } else { n / d };
                // Comme les navigateurs : une durée de moins de 20 ms vaut 100 ms.
                let ms = if ms < 20 { 100 } else { ms };
                out.push((shaped(image::DynamicImage::ImageRgba8(f.into_buffer()), shape), ms));
            }
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    decode(bytes, Some(shape)).map(|b| vec![(b, 0)])
}

/// Place d'une image animée.
#[derive(Clone, Copy, PartialEq)]
enum AnimSlot {
    /// Tuile de l'écran de connexion.
    Login(usize),
    /// Avatar proposé dans les paramètres.
    Picker(usize),
    /// Avatar du compte (en-tête, paramètres).
    Header,
}

struct Anim {
    slot: AnimSlot,
    /// Identifiant de la carte : si la liste a changé, l'animation s'arrête.
    key: String,
    frames: Vec<(slint::Image, u32)>,
    idx: usize,
    due: std::time::Instant,
}

thread_local! {
    static ANIMS: std::cell::RefCell<Vec<Anim>> = const { std::cell::RefCell::new(Vec::new()) };
    static ANIM_TIMER: slint::Timer = slint::Timer::default();
}

/// Pose une image à sa place ; false si la place n'existe plus (liste remplacée).
fn anim_set(u: &AppWindow, slot: AnimSlot, key: &str, img: slint::Image) -> bool {
    let card = |m: ModelRc<CardData>, i: usize| {
        let Some(mut c) = m.row_data(i) else { return false };
        if c.id.as_str() != key {
            return false;
        }
        c.image = img.clone();
        c.has_image = true;
        m.set_row_data(i, c);
        true
    };
    match slot {
        AnimSlot::Login(i) => card(u.get_login_tiles(), i),
        AnimSlot::Picker(i) => card(u.get_avatar_items(), i),
        AnimSlot::Header => {
            u.set_avatar(img);
            u.set_has_avatar(true);
            true
        }
    }
}

/// Affiche une image (fixe ou animée) à sa place, en remplaçant l'animation qui s'y trouvait.
fn show_frames(u: &AppWindow, slot: AnimSlot, key: &str, frames: Frames) {
    stop_anims(|s| s == slot);
    let frames: Vec<(slint::Image, u32)> = frames.into_iter().map(|(b, ms)| (slint::Image::from_rgba8(b), ms)).collect();
    let Some((first, ms)) = frames.first().cloned() else { return };
    if !anim_set(u, slot, key, first) || frames.len() < 2 {
        return;
    }
    let due = std::time::Instant::now() + std::time::Duration::from_millis(ms as u64);
    ANIMS.with_borrow_mut(|a| a.push(Anim { slot, key: key.to_string(), frames, idx: 0, due }));
    let ui = u.as_weak();
    ANIM_TIMER.with(|t| {
        if t.running() {
            return;
        }
        t.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(20), move || {
            let Some(u) = ui.upgrade() else { return };
            // Pendant la lecture, aucun écran n'est affiché sous la vidéo.
            if u.get_playing() {
                return;
            }
            let screen = u.get_screen();
            let now = std::time::Instant::now();
            ANIMS.with_borrow_mut(|list| {
                list.retain_mut(|a| {
                    let shown = match a.slot {
                        AnimSlot::Login(_) => screen == "login",
                        AnimSlot::Picker(_) => screen == "settings",
                        AnimSlot::Header => screen != "login",
                    };
                    if !shown || now < a.due {
                        return true;
                    }
                    a.idx = (a.idx + 1) % a.frames.len();
                    let (img, ms) = a.frames[a.idx].clone();
                    // Animation restée cachée : elle repart d'ici plutôt que de rattraper son retard.
                    let base = if now.duration_since(a.due).as_millis() > 500 { now } else { a.due };
                    a.due = base + std::time::Duration::from_millis(ms as u64);
                    anim_set(&u, a.slot, &a.key, img)
                });
                if list.is_empty() {
                    ANIM_TIMER.with(|t| t.stop());
                }
            });
        });
    });
}

/// Arrête les animations d'un type de place (fin de session, nouvelle liste).
fn stop_anims(f: impl Fn(AnimSlot) -> bool) {
    ANIMS.with_borrow_mut(|a| a.retain(|x| !f(x.slot)));
}

/// Comme fetch_decoded, pour des candidats de types différents (Thumb, Backdrop, Primary).
async fn fetch_any(
    client: &api::Client,
    candidates: Vec<api::ImgCand>,
    size: api::Size,
    shape: Option<Shape>,
) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let bytes = client.image_any(&candidates, size).await?;
    tokio::task::spawn_blocking(move || decode(&bytes, shape)).await.ok().flatten()
}

/// Télécharge (ou lit en cache) puis décode la première image disponible.
async fn fetch_decoded(
    client: &api::Client,
    candidates: Vec<api::ImageRef>,
    kind: &'static str,
    size: api::Size,
    shape: Option<Shape>,
) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let bytes = client.image_first(&candidates, kind, size).await?;
    tokio::task::spawn_blocking(move || decode(&bytes, shape)).await.ok().flatten()
}

// ---------------------------------------------------------------------------
// Images des cartes (accueil et rangée d'enfants)
// ---------------------------------------------------------------------------
#[derive(Clone)]
struct ImageJob {
    a: usize, // rangée (accueil) ; inutilisé pour les enfants
    b: usize, // position dans la rangée
    item_id: String,
    /// Images possibles, par ordre de préférence.
    cands: Vec<api::ImgCand>,
}

impl ImageJob {
    /// Carte paysage : vignettes 16:9 ; sinon poster (Primary).
    fn for_card(a: usize, b: usize, c: &api::CardInfo, landscape: bool) -> Option<ImageJob> {
        let cands: Vec<api::ImgCand> = if landscape {
            c.thumbs.clone()
        } else {
            c.img_id
                .clone()
                .map(|id| {
                    let kind = if id.starts_with("http") { "Url" } else { "Primary" };
                    (id, kind, c.img_tag.clone())
                })
                .into_iter()
                .collect()
        };
        (!cands.is_empty()).then(|| ImageJob { a, b, item_id: c.id.clone(), cands })
    }
}

type Apply = Arc<dyn Fn(&AppWindow, &ImageJob, SharedPixelBuffer<Rgba8Pixel>) + Send + Sync>;

/// 6 téléchargements à la fois, décodage hors du thread de l'interface.
fn spawn_image_jobs(app: &Arc<App>, client: &api::Client, jobs: Vec<ImageJob>, size: api::Size, shape: Shape, apply: Apply) {
    let permits = Arc::new(tokio::sync::Semaphore::new(6));
    for job in jobs {
        let client = client.clone();
        let ui = app.ui();
        let permits = permits.clone();
        let apply = apply.clone();
        app.rt.spawn(async move {
            let _permit = permits.acquire().await.ok();
            let Some(buf) = fetch_any(&client, job.cands.clone(), size, Some(shape)).await else { return };
            let _ = ui.upgrade_in_event_loop(move |u| (*apply)(&u, &job, buf));
        });
    }
}

/// Accueil. À appeler depuis le thread de l'interface uniquement.
/// Image d'une carte dans des rangées (accueil, recherche).
fn set_row_image(sections: &ModelRc<Section>, si: usize, ci: usize, expected_id: &str, buf: SharedPixelBuffer<Rgba8Pixel>) {
    let Some(section) = sections.row_data(si) else { return };
    let Some(mut card) = section.items.row_data(ci) else { return };
    // Si l'accueil a été rechargé entre-temps, on n'écrit pas sur la mauvaise carte.
    if card.id.as_str() != expected_id {
        return;
    }
    card.image = slint::Image::from_rgba8(buf);
    card.has_image = true;
    section.items.set_row_data(ci, card);
}

/// Rangée d'enfants de la fiche.
fn set_child_image(ui: &AppWindow, ci: usize, expected_id: &str, buf: SharedPixelBuffer<Rgba8Pixel>) {
    let model = ui.get_child_items();
    let Some(mut card) = model.row_data(ci) else { return };
    if card.id.as_str() != expected_id {
        return;
    }
    card.image = slint::Image::from_rgba8(buf);
    card.has_image = true;
    model.set_row_data(ci, card);
}

// ---------------------------------------------------------------------------
// Connexion puis accueil
// ---------------------------------------------------------------------------
struct SectionData {
    title: String,
    landscape: bool,
    cards: Vec<api::CardInfo>,
}

fn push_section(out: &mut Vec<SectionData>, title: &str, landscape: bool, items: anyhow::Result<Vec<api::Item>>) {
    if let Ok(items) = items {
        if !items.is_empty() {
            out.push(SectionData {
                title: title.to_string(),
                landscape,
                cards: items.iter().map(|i| i.card()).collect(),
            });
        }
    }
}

async fn login_flow(app: Arc<App>, server: String, user: String, pw: String) {
    match api::Client::login(&server, &user, &pw, &app.device_id).await {
        Ok(client) => start_session(app, client).await,
        Err(e) => show_login_error(&app.ui(), format!("{e}")),
    }
}

/// Session ouverte (mot de passe ou compte enregistré) : session et compte enregistrés, accueil.
async fn start_session(app: Arc<App>, client: api::Client) {
    {
        {
            let old = config::load();
            let mut saved = client.to_saved();
            saved.prefer_remote = old.prefer_remote;
            saved.still_gifs = old.still_gifs;
            saved.no_backdrop = old.no_backdrop;
            // Adresses du serveur : celles choisies à l'écran des serveurs, sinon déduites de l'adresse saisie.
            match app.pending_server.lock().unwrap().take() {
                Some((local, remote)) => {
                    saved.server_local = local;
                    saved.server_remote = remote;
                }
                None => {
                    if discovery::is_local_url(&client.server) {
                        saved.server_local = client.server.clone();
                    } else {
                        saved.server_remote = client.server.clone();
                    }
                }
            }
            // Le même serveur (même identifiant) garde ses adresses connues.
            let server_id = discovery::probe(&reqwest::Client::new(), &client.server).await.map(|i| i.0).unwrap_or_default();
            if !server_id.is_empty() && server_id == old.server_id {
                if saved.server_local.is_empty() {
                    saved.server_local = old.server_local.clone();
                }
                if saved.server_remote.is_empty() {
                    saved.server_remote = old.server_remote.clone();
                }
            }
            saved.server_id = server_id.clone();
            config::save(&saved);
            config::save_account(config::Account {
                server_id,
                user_id: client.user_id.clone(),
                user_name: client.user_name.clone(),
                token: client.token.clone(),
            });
            load_home(app, client).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Écran de connexion : comptes enregistrés, comptes du serveur, autre compte
// ---------------------------------------------------------------------------
fn login_opened(app: &Arc<App>) {
    let Some(u) = app.ui().upgrade() else { return };
    let server = api::normalize_server(&u.get_server());
    u.set_login_mode("pick".into());
    u.set_login_sel(0);
    let app2 = app.clone();
    app.rt.spawn(async move {
        let http = reqwest::Client::new();
        let info = if server.ends_with("//") || server.is_empty() { None } else { discovery::probe(&http, &server).await };
        let (sid, sname) = info.map(|i| (i.0, i.1)).unwrap_or_default();
        // (action, nom, sous-titre, identifiant d'utilisateur pour l'avatar)
        let mut tiles: Vec<(String, String, String, String)> = Vec::new();
        if !sid.is_empty() {
            for a in config::accounts().into_iter().filter(|a| a.server_id == sid) {
                tiles.push((format!("acc:{}", a.user_id), a.user_name, "Enregistré".into(), a.user_id));
            }
            let public: serde_json::Value = match http.get(format!("{server}/Users/Public")).send().await {
                Ok(r) => r.json().await.unwrap_or_default(),
                Err(_) => serde_json::Value::Null,
            };
            for p in public.as_array().into_iter().flatten() {
                let (Some(id), Some(name)) = (p["Id"].as_str(), p["Name"].as_str()) else { continue };
                if tiles.iter().any(|t| t.3 == id) {
                    continue;
                }
                tiles.push((format!("pub:{name}"), name.to_string(), String::new(), id.to_string()));
            }
        }
        tiles.push(("other".into(), "Autre compte".into(), String::new(), String::new()));
        let reachable = !sid.is_empty();
        let rows = tiles.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            let cards: Vec<CardData> = rows
                .iter()
                .map(|(id, name, sub, _)| CardData {
                    id: id.clone().into(),
                    title: name.clone().into(),
                    subtitle: sub.clone().into(),
                    // Initiale affichée tant que l'avatar n'est pas chargé (Slint n'a pas de sous-chaîne).
                    rating: name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into()).into(),
                    ..Default::default()
                })
                .collect();
            u.set_login_tiles(ModelRc::new(VecModel::from(cards)));
            // L'écran de connexion reprend le clavier (après le menu, par exemple).
            u.set_refocus(u.get_refocus() + 1);
            u.set_server_name(if reachable { sname.into() } else { "Serveur injoignable".into() });
        });
        // Avatars (publics, sans connexion), en cercle.
        for (i, (key, _, _, uid)) in tiles.into_iter().enumerate() {
            if uid.is_empty() {
                continue;
            }
            let (http, server, ui) = (http.clone(), server.clone(), app2.ui());
            tokio::spawn(async move {
                // Fichier d'origine si les GIF sont animés (réduit par le serveur, un GIF ne l'est plus).
                let size = if STILL_GIFS.load(Ordering::Relaxed) { "?maxWidth=200" } else { "" };
                let Ok(r) = http.get(format!("{server}/Users/{uid}/Images/Primary{size}")).send().await else { return };
                if !r.status().is_success() {
                    return;
                }
                let Ok(bytes) = r.bytes().await else { return };
                let Some(frames) = tokio::task::spawn_blocking(move || decode_frames(&bytes, Shape { w: 200, h: 200, radius: 0.5, top_only: false }))
                    .await
                    .ok()
                    .flatten()
                else {
                    return;
                };
                let _ = ui.upgrade_in_event_loop(move |u| show_frames(&u, AnimSlot::Login(i), &key, frames));
            });
        }
    });
}

/// Tuile choisie : compte enregistré (connexion directe), compte du serveur ou autre compte.
fn login_pick(app: &Arc<App>, action: String) {
    let Some(u) = app.ui().upgrade() else { return };
    u.set_error_text("".into());
    u.set_login_pass("".into());
    if action == "other" {
        u.set_login_user("".into());
        u.set_login_field(0);
        u.set_login_mode("form".into());
        return;
    }
    if let Some(name) = action.strip_prefix("pub:") {
        u.set_login_user(name.into());
        u.set_login_field(1);
        u.set_login_mode("form".into());
        return;
    }
    let Some(uid) = action.strip_prefix("acc:") else { return };
    let Some(acc) = config::accounts().into_iter().find(|a| a.user_id == uid) else { return };
    let server = u.get_server().to_string();
    u.set_busy(true);
    let app2 = app.clone();
    app.rt.spawn(async move {
        let client = match api::Client::from_token(&server, &acc.user_id, &acc.user_name, &acc.token, &app2.device_id) {
            Ok(c) => c,
            Err(e) => return show_login_error(&app2.ui(), format!("{e}")),
        };
        match client.user_config().await {
            Ok(_) => start_session(app2, client).await,
            Err(_) => {
                // Jeton expiré ou révoqué : on demande le mot de passe.
                config::forget_account(&acc.user_id);
                let name = acc.user_name.clone();
                let _ = app2.ui().upgrade_in_event_loop(move |u| {
                    u.set_busy(false);
                    u.set_login_user(name.into());
                    u.set_login_field(1);
                    u.set_login_mode("form".into());
                    u.set_error_text("Session expirée : entre le mot de passe.".into());
                });
            }
        }
    });
}

/// Fin de session (déconnexion ou changement de compte) puis écran de connexion.
/// `forget` : le compte est retiré des comptes enregistrés.
fn end_session(app: &Arc<App>, forget: bool) {
    if app.sp.lock().unwrap().group.is_some() {
        if let Some(c) = app.client() {
            app.rt.spawn(async move {
                let _ = syncplay::leave(&c).await;
            });
        }
        *app.sp.lock().unwrap() = syncplay::State::default();
        refresh_party(app);
    }
    if forget {
        if let Some(c) = app.client() {
            config::forget_account(&c.user_id);
        }
    }
    config::clear_token();
    *app.client.lock().unwrap() = None;
    app.bg_id.lock().unwrap().clear();
    *app.tab.lock().unwrap() = "home".to_string();
    app.stack.lock().unwrap().clear();
    *app.library.lock().unwrap() = None;
    app.gen.fetch_add(1, Ordering::SeqCst);
    if let Some(u) = app.ui().upgrade() {
        u.set_tab("home".into());
        u.set_h_focus(false);
        u.set_can_back(false);
        u.set_has_requests(false);
        stop_anims(|s| s == AnimSlot::Header);
        u.set_has_avatar(false);
        u.set_bg_show(false);
        u.set_menu_open(false);
        u.set_lib_items(ModelRc::default());
        u.set_sections(ModelRc::default());
        u.set_detail(DetailData::default());
        u.set_child_items(ModelRc::default());
        u.set_busy(false);
        u.set_error_text("".into());
        u.set_screen("login".into());
    }
}

/// Ouvre la session : accueil (onglet affiché) avec ce client.
async fn load_home(app: Arc<App>, client: api::Client) {
    *app.client.lock().unwrap() = Some(client.clone());
    app.offline.store(false, Ordering::SeqCst);
    let _ = app.ui().upgrade_in_event_loop(|u| u.set_offline(false));
    sync_offline_plays(&app, &client);
    app.stack.lock().unwrap().clear();
    // Essais : --play=ID[@SECONDES] lance directement la lecture.
    let arg = app.play_arg.lock().unwrap().take();
    if let Some(id) = arg.as_deref().and_then(|p| p.strip_prefix("open:")) {
        // Essais : --open=ID ouvre directement une fiche (après l'accueil), --open=downloads les téléchargements.
        let (a, id) = (app.clone(), id.to_string());
        app.rt.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let a2 = a.clone();
            let _ = a.ui().upgrade_in_event_loop(move |_| if id == "downloads" { open_downloads(&a2) } else { push_detail(&a2, id) });
        });
    } else if let Some(p) = arg {
        let (id, at) = p.split_once('@').map(|(i, t)| (i.to_string(), t.parse::<f64>().ok())).unwrap_or((p.clone(), None));
        *app.play_start.lock().unwrap() = at;
        let a = app.clone();
        app.rt.spawn(async move { play_flow(a, Some(id), None).await });
    }
    // Onglet Demandes : seulement si Seerr est joignable et relié au compte (via Jellyfin Enhanced).
    let seerr = client.seerr_user().await;
    *app.seerr_user.lock().unwrap() = seerr;
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        u.set_has_requests(seerr.is_some());
        u.set_can_back(false);
    });
    // Menu latéral : bibliothèques de l'utilisateur.
    if let Ok(views) = client.views().await {
        set_menu(&app, &views);
    }
    complete_addresses(&app, &client);
    if let Ok(cfg) = client.user_config().await {
        apply_user_defaults(&cfg);
        *app.user_cfg.lock().unwrap() = cfg;
    }
    load_header_avatar(&app, &client);
    start_syncplay(&app, &client);
    app.can_download.store(client.can_download().await, Ordering::SeqCst);
    load_tab(app, client).await;
}

/// Charge l'onglet affiché (« Accueil » ou « Favoris ») et le présente.
async fn load_tab(app: Arc<App>, client: api::Client) {
    let ui = app.ui();
    let _ = ui.upgrade_in_event_loop(|u| begin_loading(&u));
    let tab = app.tab.lock().unwrap().clone();
    let seerr = *app.seerr_user.lock().unwrap();
    let sections = match (tab.as_str(), seerr) {
        ("favorites", _) => favorite_sections(&client).await,
        ("requests", Some(id)) => request_sections(&client, id).await,
        _ => home_sections(&client).await,
    };
    match sections {
        Ok(s) => {
            if app.offline.swap(false, Ordering::SeqCst) {
                let _ = app.ui().upgrade_in_event_loop(|u| u.set_offline(false));
                sync_offline_plays(&app, &client);
            }
            present_sections(&app, &client, s)
        }
        // Serveur injoignable (pas un refus d'accès) : on bascule sur les téléchargements.
        Err(e) if e.downcast_ref::<api::Unauthorized>().is_none() && !downloads::list().is_empty() => go_offline(&app),
        Err(e) => handle_error(&ui, e),
    }
}

async fn home_sections(client: &api::Client) -> anyhow::Result<Vec<SectionData>> {
    let (views, resume, next) = tokio::join!(client.views(), client.resume(), client.next_up());
    let views = views?;

    let mut sections: Vec<SectionData> = Vec::new();
    // « Mes médias » en haut, en vignettes 16:9 (équivalent de l'addon horizontalMyMedia).
    let my_media: Vec<api::Item> = views
        .iter()
        .filter(|v| v.collection_type.as_deref() != Some("livetv"))
        .cloned()
        .collect();
    push_section(&mut sections, "Mes médias", true, Ok(my_media));
    // « Reprendre » et « À suivre » en vignettes 16:9 avec avancement, comme JellySkin.
    let resume_ids: Vec<String> = resume.as_ref().map(|r| r.iter().map(|i| i.id.clone()).collect()).unwrap_or_default();
    let next = next.map(|n| n.into_iter().filter(|i| !resume_ids.contains(&i.id)).collect());
    push_section(&mut sections, "Reprendre", true, resume);
    push_section(&mut sections, "À suivre", true, next);

    for v in views.iter().filter(|v| {
        !matches!(v.collection_type.as_deref(), Some("playlists" | "livetv" | "boxsets"))
    }) {
        let latest = client.latest(&v.id).await;
        push_section(&mut sections, &format!("Récemment ajouté · {}", v.name), false, latest);
    }
    Ok(sections)
}

async fn favorite_sections(client: &api::Client) -> anyhow::Result<Vec<SectionData>> {
    let items = client.favorites().await?;
    let groups: [(&str, &[&str], bool); 5] = [
        ("Films", &["Movie"], false),
        ("Séries", &["Series"], false),
        ("Saisons", &["Season"], false),
        ("Épisodes", &["Episode"], true),
        ("Autres", &["BoxSet", "Video", "MusicVideo"], false),
    ];
    let mut sections: Vec<SectionData> = Vec::new();
    for (title, kinds, landscape) in groups {
        let of_kind: Vec<api::Item> = items.iter().filter(|i| kinds.contains(&i.kind.as_str())).cloned().collect();
        push_section(&mut sections, title, landscape, Ok(of_kind));
    }
    Ok(sections)
}

/// Demandes Seerr de l'utilisateur, rangées par état.
async fn request_sections(client: &api::Client, seerr_user: i64) -> anyhow::Result<Vec<SectionData>> {
    let mut reqs = client.seerr_requests(seerr_user).await?;
    reqs.sort_by(|a, b| b.created.cmp(&a.created)); // plus récentes d'abord
    // Rangées par état de carte (voir SeerrRequest::card) : disponibles d'abord.
    let groups: [(&str, &[i32]); 5] =
        [("Disponibles", &[4]), ("En attente", &[1]), ("Acceptées", &[2]), ("Refusées", &[3]), ("En échec", &[5])];
    let mut sections: Vec<SectionData> = Vec::new();
    for (title, states) in groups {
        let cards: Vec<api::CardInfo> = reqs.iter().map(|r| r.card()).filter(|c| states.contains(&c.status)).collect();
        if !cards.is_empty() {
            sections.push(SectionData { title: title.to_string(), landscape: false, cards });
        }
    }
    Ok(sections)
}

/// Affiche des rangées de cartes et lance le chargement de leurs images.
fn present_sections(app: &Arc<App>, client: &api::Client, sections: Vec<SectionData>) {
    present_rows(app, client, sections, false);
}

/// Rangées de cartes de l'accueil (`search` = false) ou des résultats de recherche.
fn present_rows(app: &Arc<App>, client: &api::Client, sections: Vec<SectionData>, search: bool) {
    let ui = app.ui();

    // Dimensions : voir card-w et SectionRow dans app.slint (rangée = 132 px x k + image).
    let k = if app.tv() { 1.4_f32 } else { 1.0 };
    let card_w = if app.tv() { 230.0_f32 } else { 170.0 };
    let row_h = move |landscape: bool| 132.0 * k + if landscape { card_w * 1.5 * 0.5625 } else { card_w * 1.5 };

    let mut poster_jobs: Vec<ImageJob> = Vec::new();
    let mut thumb_jobs: Vec<ImageJob> = Vec::new();
    for (si, s) in sections.iter().enumerate() {
        for (ci, c) in s.cards.iter().enumerate() {
            let job = ImageJob::for_card(si, ci, c, s.landscape);
            if s.landscape { thumb_jobs.extend(job) } else { poster_jobs.extend(job) }
        }
    }

    let user_name = client.user_name.clone();
    let _ = ui.upgrade_in_event_loop(move |u| {
        let mut y = 0.0_f32;
        let rows: Vec<Section> = sections
            .iter()
            .map(|s| {
                let h = row_h(s.landscape);
                let row = Section {
                    title: s.title.clone().into(),
                    landscape: s.landscape,
                    y_px: y,
                    h_px: h,
                    items: ModelRc::new(VecModel::from(
                        s.cards
                            .iter()
                            .map(|c| CardData {
                                id: c.id.clone().into(),
                                title: c.title.clone().into(),
                                subtitle: c.subtitle.clone().into(),
                                progress: c.progress,
                                rating: c.rating.clone().into(),
                                seerr: c.seerr,
                                status: c.status,
                                count: c.count,
                                played: c.played,
                                ..Default::default()
                            })
                            .collect::<Vec<_>>(),
                    )),
                };
                y += h;
                row
            })
            .collect();

        if search {
            u.set_search_sections(ModelRc::new(VecModel::from(rows)));
            u.set_s_section(0);
            u.set_s_item(0);
            u.set_search_busy(false);
            return;
        }
        let empty = rows.is_empty();
        u.set_sections(ModelRc::new(VecModel::from(rows)));
        u.set_user_name(user_name.into());
        u.set_sel_section(0);
        u.set_sel_item(0);
        // Rien à afficher (aucun favori...) : la sélection reste dans l'en-tête.
        if empty {
            u.set_h_focus(true);
        }
        u.set_toast("".into());
        u.set_busy(false);
        show_screen(&u, "home");
    });

    let apply: Apply = Arc::new(
        move |u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            let rows = if search { u.get_search_sections() } else { u.get_sections() };
            set_row_image(&rows, job.a, job.b, &job.item_id, buf)
        },
    );
    spawn_image_jobs(app, client, poster_jobs, api::Size::Fill(270, 405), Shape::card_top(270, 405, card_w), apply.clone());
    spawn_image_jobs(app, client, thumb_jobs, api::Size::Fill(400, 225), Shape::card_top(400, 225, card_w * 1.5), apply);
}

// ---------------------------------------------------------------------------
// Fiche : rangées du bas (casting, plus de ce genre, Seerr) et boutons à bascule
// ---------------------------------------------------------------------------
fn spawn_detail_rows(app: &Arc<App>, client: &api::Client, item: &api::Item) {
    let (app2, client2, item) = (app.clone(), client.clone(), item.clone());
    let seerr = *app.seerr_user.lock().unwrap();
    app.rt.spawn(async move {
        // (Pas de « Casting et équipe » : les noms affichés ne sont pas ceux des voix françaises.)
        let mut rows: Vec<SectionData> = Vec::new();
        // Épisode / saison : suggestions de la série.
        let base = match item.kind.as_str() {
            "Episode" | "Season" => match &item.series_id {
                Some(sid) => client2.item(sid).await.unwrap_or_else(|_| item.clone()),
                None => item.clone(),
            },
            _ => item.clone(),
        };
        if let Ok(sim) = client2.similar(&base.id).await {
            push_section(&mut rows, "Plus de ce genre", false, Ok(sim));
        }
        if let (Some(_), Some(tmdb)) = (seerr, base.tmdb()) {
            let tv = base.kind == "Series";
            for (title, kind) in [("Similaires", "similar"), ("Recommandés", "recommendations")] {
                let cards = client2.seerr_related(tv, tmdb, kind).await;
                if !cards.is_empty() {
                    rows.push(SectionData { title: title.into(), landscape: false, cards });
                }
            }
        }
        if rows.is_empty() {
            return;
        }

        // Dimensions : mêmes cartes que l'accueil (card-w dans app.slint).
        let k = if app2.tv() { 1.4_f32 } else { 1.0 };
        let card_w = if app2.tv() { 230.0_f32 } else { 170.0 };
        let row_h = 132.0 * k + card_w * 1.5;
        let item_id = item.id.clone();
        let mut jobs: Vec<ImageJob> = Vec::new();
        for (ri, r) in rows.iter().enumerate() {
            jobs.extend(r.cards.iter().enumerate().filter_map(|(ci, c)| ImageJob::for_card(ri, ci, c, false)));
        }
        let id_check = item_id.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            if u.get_detail().id.as_str() != id_check {
                return;
            }
            let sections: Vec<Section> = rows
                .iter()
                .enumerate()
                .map(|(i, s)| Section {
                    title: s.title.clone().into(),
                    landscape: false,
                    y_px: i as f32 * row_h,
                    h_px: row_h,
                    items: ModelRc::new(VecModel::from(
                        s.cards
                            .iter()
                            .map(|c| CardData {
                                id: c.id.clone().into(),
                                title: c.title.clone().into(),
                                subtitle: c.subtitle.clone().into(),
                                progress: c.progress,
                                rating: c.rating.clone().into(),
                                seerr: c.seerr,
                                status: c.status,
                                count: c.count,
                                played: c.played,
                                ..Default::default()
                            })
                            .collect::<Vec<_>>(),
                    )),
                })
                .collect();
            u.set_detail_rows(ModelRc::new(VecModel::from(sections)));
        });
        let apply: Apply = Arc::new(move |u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            let rows = u.get_detail_rows();
            let Some(row) = rows.row_data(job.a) else { return };
            let Some(mut card) = row.items.row_data(job.b) else { return };
            if card.id.as_str() != job.item_id {
                return;
            }
            card.image = slint::Image::from_rgba8(buf);
            card.has_image = true;
            row.items.set_row_data(job.b, card);
        });
        spawn_image_jobs(&app2, &client2, jobs, api::Size::Fill(270, 405), Shape::card_top(270, 405, card_w), apply);
    });
}

/// « Japanese - AAC - Stereo - Default » -> « Japanese ».
fn short_label(l: &str) -> String {
    l.split(" - ").next().unwrap_or(l).trim().to_string()
}

/// Nom d'une langue (code ISO 639-2 de Jellyfin), sinon le libellé court de la piste.
fn lang_name(code: &str, label: &str) -> String {
    let n = match code {
        "fre" | "fra" => "Français",
        "eng" => "Anglais",
        "jpn" => "Japonais",
        "ger" | "deu" => "Allemand",
        "spa" => "Espagnol",
        "ita" => "Italien",
        "por" => "Portugais",
        "kor" => "Coréen",
        "chi" | "zho" => "Chinois",
        "rus" => "Russe",
        "ara" => "Arabe",
        "dut" | "nld" => "Néerlandais",
        _ => "",
    };
    if n.is_empty() { short_label(label) } else { n.to_string() }
}

/// Ouvre la liste des pistes de la fiche (« audio » ou « sub »).
fn open_track_picker(app: &Arc<App>, kind: &str) {
    let (key, audio, subs) = app.detail_streams.lock().unwrap().clone();
    let pref = config::track_pref(&key);
    let mut rows: Vec<TrackData> = Vec::new();
    let cur;
    if kind == "audio" {
        cur = pref.audio.clone();
        rows.push(TrackData { id: "".into(), label: "Par défaut (fichier)".into(), current: cur.is_empty() });
        for (l, t) in &audio {
            rows.push(TrackData { id: l.clone().into(), label: t.clone().into(), current: *l == cur });
        }
    } else {
        cur = pref.sub.clone();
        rows.push(TrackData { id: "".into(), label: "Par défaut (fichier)".into(), current: cur.is_empty() });
        rows.push(TrackData { id: "off".into(), label: "Désactivés".into(), current: cur == "off" });
        for (l, t) in &subs {
            rows.push(TrackData { id: l.clone().into(), label: t.clone().into(), current: *l == cur });
        }
    }
    let sel = rows.iter().position(|r| r.current).unwrap_or(0) as i32;
    if let Some(u) = app.ui().upgrade() {
        u.set_tp_items(ModelRc::new(VecModel::from(rows)));
        u.set_tp_sel(sel);
        u.set_tp_kind(kind.into());
    }
}

/// Piste choisie dans la liste : enregistrée pour la série / le film, bouton mis à jour.
fn pick_track(app: &Arc<App>, kind: &str, lang: &str, label: &str) {
    let key = app.detail_streams.lock().unwrap().0.clone();
    if kind == "audio" {
        config::set_track_pref(&key, Some(lang), None);
    } else {
        config::set_track_pref(&key, None, Some(lang));
    }
    let Some(u) = app.ui().upgrade() else { return };
    let d = u.get_detail();
    let action = if kind == "audio" { "pick-audio" } else { "pick-sub" };
    let shown = if lang.is_empty() { "par défaut".to_string() } else if lang == "off" { "désactivés".to_string() } else { lang_name(lang, label) };
    for i in 0..d.buttons.row_count() {
        if let Some(mut b) = d.buttons.row_data(i) {
            if b.action == action {
                b.label = format!("{} : {shown}", if kind == "audio" { "Audio" } else { "Sous-titres" }).into();
                d.buttons.set_row_data(i, b);
            }
        }
    }
    u.set_tp_kind("".into());
}

/// Favori / vu : bascule côté serveur, puis mise à jour du bouton (sans recharger la fiche).
fn toggle_flag(app: &Arc<App>, action: &str) {
    let Some(u) = app.ui().upgrade() else { return };
    let d = u.get_detail();
    let key = d.id.to_string();
    let id = real_id(&key).to_string();
    let fav = action == "fav";
    // Fiche d'un téléchargement ou pas de serveur : gardé sur l'appareil, renvoyé à la reconnexion.
    let client = match app.client() {
        Some(c) if !key.starts_with("dl:") && !app.offline.load(Ordering::SeqCst) => c,
        _ => {
            let Some(idx) = (0..d.buttons.row_count()).find(|i| d.buttons.row_data(*i).is_some_and(|b| b.action == action)) else { return };
            let on = !d.buttons.row_data(idx).is_some_and(|b| b.active);
            config::set_flag(&id, fav, on, true);
            // « Vu » sur une série ou une saison : ses épisodes téléchargés aussi (affichage).
            if !fav && (key.starts_with("dl:series:") || key.starts_with("dl:season:")) {
                for e in downloads::list().into_iter().filter(|e| e.series_id == id || e.season_id == id) {
                    config::set_flag(&e.id, false, on, false);
                }
            }
            if let Some(mut b) = d.buttons.row_data(idx) {
                b.active = on;
                d.buttons.set_row_data(idx, b);
            }
            if key.starts_with("dl:") {
                // Enfants (coches) et téléchargements à jour ; la sélection reste sur le bouton.
                PAGES.with_borrow_mut(|p| p.retain(|k, _| !k.starts_with("dl:")));
                refresh_downloads(app);
                show_local_detail(app, &key);
            }
            // En ligne : envoyé tout de suite (sinon à la reconnexion).
            if let (Some(c), false) = (app.client(), app.offline.load(Ordering::SeqCst)) {
                sync_offline_plays(app, &c);
            }
            return;
        }
    };
    let Some(idx) = (0..d.buttons.row_count()).find(|i| d.buttons.row_data(*i).map(|b| b.action == action).unwrap_or(false)) else {
        return;
    };
    let on = !d.buttons.row_data(idx).map(|b| b.active).unwrap_or(false);
    // Affichage immédiat ; on revient en arrière si le serveur refuse.
    let set = move |u: &AppWindow, v: bool| {
        let d = u.get_detail();
        if let Some(mut b) = d.buttons.row_data(idx) {
            b.active = v;
            d.buttons.set_row_data(idx, b);
        }
    };
    set(&u, on);
    let (app2, action) = (app.clone(), action.to_string());
    app.rt.spawn(async move {
        let r = if action == "fav" { client.set_favorite(&id, on).await } else { client.set_played(&id, on).await };
        app2.home_stale.store(true, Ordering::SeqCst);
        if r.is_ok() && downloads::exists(&id) {
            // Téléchargé : l'état est aussi gardé pour l'affichage hors ligne (déjà sur le compte).
            config::set_flag(&id, action == "fav", on, false);
        }
        if let Err(e) = r {
            let msg = format!("Action impossible : {e}");
            let _ = app2.ui().upgrade_in_event_loop(move |u| {
                set(&u, !on);
                u.set_toast(msg.into());
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Watch party (SyncPlay)
// ---------------------------------------------------------------------------
fn start_syncplay(app: &Arc<App>, client: &api::Client) {
    if app.sp_connected.swap(true, Ordering::SeqCst) {
        return;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<syncplay::Event>();
    let (c, sp) = (client.clone(), app.sp.clone());
    app.rt.spawn(async move { syncplay::connect(c, sp, tx) });
    let app2 = app.clone();
    app.rt.spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                syncplay::Event::Group => refresh_party(&app2),
                syncplay::Event::Toast(m) => {
                    let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(m.into()));
                }
                syncplay::Event::Play { item_id, start } => {
                    // Lecture en cours : le lecteur change de média ; sinon on le lance.
                    let tx = app2.player_tx.lock().unwrap().clone();
                    match tx {
                        Some(tx) => {
                            let _ = tx.send(format!("sp-load:{item_id}@{start}"));
                        }
                        None => {
                            let a = app2.clone();
                            app2.rt.spawn(async move { play_flow_group(a, item_id, start).await });
                        }
                    }
                }
                syncplay::Event::Command(cmd) => {
                    let msg = match cmd {
                        syncplay::Cmd::Unpause { pos, delay } => format!("sp-unpause:{pos}:{delay}"),
                        syncplay::Cmd::Pause { pos } => format!("sp-pause:{pos}"),
                        syncplay::Cmd::Seek { pos } => format!("sp-seek:{pos}"),
                        syncplay::Cmd::Stop => "stop".to_string(),
                    };
                    if let Some(tx) = app2.player_tx.lock().unwrap().as_ref() {
                        let _ = tx.send(msg);
                    }
                }
            }
        }
    });
}

/// Écran Watch party : groupe actuel ou liste des groupes.
fn refresh_party(app: &Arc<App>) {
    let group = {
        let s = app.sp.lock().unwrap();
        let st = match s.state.as_str() {
            "Playing" => "En lecture",
            "Paused" => "En pause",
            "Waiting" => "En attente des participants",
            _ => "Prêt : lance un film ou un épisode",
        };
        (s.group.clone(), s.participants.clone(), st.to_string(), s.item_id.clone())
    };
    let (group, people, state, now_id) = (group.0, group.1, group.2, group.3);
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let groups = if group.is_none() { syncplay::list(&client).await } else { Vec::new() };
        // Ce que le groupe regarde : « Série — S1E2 · titre », ou le film.
        let now = if group.is_some() && !now_id.is_empty() {
            match client.item(&now_id).await {
                Ok(it) => {
                    let (t, sub) = it.titles();
                    if sub.is_empty() { t } else { format!("{t} — {sub}") }
                }
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            u.set_sp_active(group.is_some());
            u.set_sp_name(group.clone().map(|g| g.1).unwrap_or_default().into());
            u.set_sp_people(people.join(", ").into());
            u.set_sp_count(people.len() as i32);
            u.set_sp_list(ModelRc::new(VecModel::from(people.iter().map(|p| slint::SharedString::from(p.as_str())).collect::<Vec<_>>())));
            u.set_sp_now(now.into());
            u.set_sp_state(state.into());
            // Menu : la watch party en cours y est signalée (nombre de participants).
            let m = u.get_menu_entries();
            if let Some(i) = (0..m.row_count()).find(|&i| m.row_data(i).is_some_and(|e| e.action == "party")) {
                if let Some(mut e) = m.row_data(i) {
                    e.label = if group.is_some() { format!("Watch party · {} en ligne", people.len()).into() } else { "Watch party".into() };
                    m.set_row_data(i, e);
                }
            }
            let rows: Vec<MenuEntry> = groups
                .into_iter()
                .map(|(id, name, p)| MenuEntry { label: format!("{name}  ·  {}", p.join(", ")).into(), action: id.into(), header: false })
                .collect();
            u.set_sp_groups(ModelRc::new(VecModel::from(rows)));
        });
    });
}

fn open_party(app: &Arc<App>) {
    if let Some(u) = app.ui().upgrade() {
        u.set_sp_sel(0);
        u.set_h_focus(false);
        u.set_screen("party".into());
    }
    refresh_party(app);
}

/// Action de l'écran Watch party : "create", "leave", ou l'identifiant d'un groupe à rejoindre.
fn party_action(app: &Arc<App>, action: String) {
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let r = match action.as_str() {
            "create" => syncplay::create(&client, &format!("Watch party de {}", client.user_name)).await,
            "leave" => syncplay::leave(&client).await,
            id => syncplay::join(&client, id).await,
        };
        if let Err(e) = r {
            let msg = format!("Watch party : {e}");
            let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        refresh_party(&app2);
    });
}

// ---------------------------------------------------------------------------
// Recherche (bibliothèque + Seerr) et média au hasard
// ---------------------------------------------------------------------------
fn open_search(app: &Arc<App>) {
    if let Some(u) = app.ui().upgrade() {
        u.set_h_focus(false);
        u.set_s_osk(u.get_tv_mode());
        u.set_screen("search".into());
    }
}

/// Texte de recherche modifié : recherche après une courte pause de frappe.
fn search_changed(app: &Arc<App>, text: String) {
    let my = app.search_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let Some(client) = app.client() else { return };
    let term = text.trim().to_string();
    if term.chars().count() < 2 {
        if let Some(u) = app.ui().upgrade() {
            u.set_search_sections(ModelRc::default());
            u.set_search_busy(false);
        }
        return;
    }
    if let Some(u) = app.ui().upgrade() {
        u.set_search_busy(true);
    }
    let app2 = app.clone();
    let seerr = app.seerr_user.lock().unwrap().is_some();
    app.rt.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        if app2.search_gen.load(Ordering::SeqCst) != my {
            return;
        }
        let (found, from_seerr) = tokio::join!(client.search(&term), async {
            if seerr { client.seerr_search(&term).await } else { Vec::new() }
        });
        if app2.search_gen.load(Ordering::SeqCst) != my {
            return;
        }
        let items = found.unwrap_or_default();
        let mut sections: Vec<SectionData> = Vec::new();
        for (title, kinds, landscape) in [
            ("Films", &["Movie", "BoxSet"][..], false),
            ("Séries", &["Series"][..], false),
            ("Épisodes", &["Episode"][..], true),
        ] {
            let of: Vec<api::Item> = items.iter().filter(|i| kinds.contains(&i.kind.as_str())).cloned().collect();
            push_section(&mut sections, title, landscape, Ok(of));
        }
        if !from_seerr.is_empty() {
            sections.push(SectionData { title: "À demander (Seerr)".into(), landscape: false, cards: from_seerr });
        }
        present_rows(&app2, &client, sections, true);
    });
}

fn random_pick(app: &Arc<App>) {
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        match client.random_unwatched().await {
            Ok(Some(it)) => {
                let a = app2.clone();
                let _ = app2.ui().upgrade_in_event_loop(move |_| push_detail(&a, it.id));
            }
            _ => {
                let _ = app2.ui().upgrade_in_event_loop(|u| u.set_toast("Plus rien à découvrir : tout a été vu !".into()));
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Page Seerr (média absent du serveur) : informations et bouton Demander
// ---------------------------------------------------------------------------
fn open_seerr(app: &Arc<App>, tv: bool, tmdb: i64) {
    let Some(client) = app.client() else { return };
    if let Some(u) = app.ui().upgrade() {
        u.set_sr(SeerrPage { loading: true, ..Default::default() });
        u.set_sr_cast(ModelRc::default());
        u.set_sr_sel(0);
        u.set_sr_scroll(0.0);
        u.set_sr_open(true);
    }
    let app2 = app.clone();
    app.rt.spawn(async move {
        let d = match client.seerr_details(tv, tmdb).await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("Seerr : {e}");
                let _ = app2.ui().upgrade_in_event_loop(move |u| {
                    u.set_sr_open(false);
                    u.set_toast(msg.into());
                });
                return;
            }
        };
        *app2.seerr_page.lock().unwrap() = Some(d.clone());
        let (status_text, can_request) = match d.status {
            5 => ("Disponible sur le serveur", false),
            4 => ("Partiellement disponible", true),
            3 => ("En cours de traitement", false),
            2 => ("Demande en attente", false),
            _ => ("", true),
        };
        let cast: Vec<api::CardInfo> = d
            .cast
            .iter()
            .enumerate()
            .map(|(i, (name, role, photo))| api::CardInfo {
                id: format!("person:{i}"),
                title: name.clone(),
                subtitle: role.clone(),
                img_id: photo.clone(),
                img_tag: None,
                thumbs: Vec::new(),
                progress: 0.0,
                rating: String::new(),
                seerr: false,
                status: 0,
                count: 0,
                played: false,
            })
            .collect();
        let jobs: Vec<ImageJob> = cast.iter().enumerate().filter_map(|(i, c)| ImageJob::for_card(0, i, c, false)).collect();
        let d2 = d.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            let d = d2;
            let page = SeerrPage {
                loading: false,
                title: d.title.clone().into(),
                year: d.year.clone().into(),
                overview: d.overview.clone().into(),
                genres: d.genres.clone().into(),
                length: d.length.clone().into(),
                rating: d.rating.clone().into(),
                facts: ModelRc::new(VecModel::from(
                    d.facts.iter().map(|(k, v)| FactData { label: k.clone().into(), value: v.clone().into() }).collect::<Vec<_>>(),
                )),
                status: status_text.into(),
                can_request,
                on_server: d.jellyfin_id.is_some(),
                ..Default::default()
            };
            u.set_sr(page);
            u.set_sr_cast(ModelRc::new(VecModel::from(cast.iter().map(card_data).collect::<Vec<_>>())));
        });
        // Affiche et fond.
        for (url, is_bg) in [(d.poster.clone(), false), (d.backdrop.clone().or(d.poster.clone()), true)] {
            let Some(url) = url else { continue };
            let (c2, ui) = (client.clone(), app2.ui());
            app2.rt.spawn(async move {
                let Some(bytes) = c2.image_any(&[(url, "Url", None)], api::Size::Fill(0, 0)).await else { return };
                let buf = tokio::task::spawn_blocking(move || {
                    if is_bg { decode_backdrop(&bytes) } else { decode(&bytes, Some(Shape::card(270, 405, 150.0))) }
                })
                .await
                .ok()
                .flatten();
                let Some(buf) = buf else { return };
                let _ = ui.upgrade_in_event_loop(move |u| {
                    let mut p = u.get_sr();
                    if is_bg {
                        p.backdrop = slint::Image::from_rgba8(buf);
                        p.has_backdrop = true;
                    } else {
                        p.poster = slint::Image::from_rgba8(buf);
                        p.has_poster = true;
                    }
                    u.set_sr(p);
                });
            });
        }
        let apply: Apply = Arc::new(|u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            let m = u.get_sr_cast();
            if let Some(mut c) = m.row_data(job.b) {
                if c.id.as_str() == job.item_id {
                    c.image = slint::Image::from_rgba8(buf);
                    c.has_image = true;
                    m.set_row_data(job.b, c);
                }
            }
        });
        spawn_image_jobs(&app2, &client, jobs, api::Size::Fill(160, 160), Shape { w: 160, h: 160, radius: 0.5, top_only: false }, apply);
    });
}

/// Bouton de la page Seerr : demander, ou ouvrir la fiche si le média est sur le serveur.
fn seerr_action(app: &Arc<App>) {
    let Some(d) = app.seerr_page.lock().unwrap().clone() else { return };
    if let Some(id) = d.jellyfin_id.clone() {
        if let Some(u) = app.ui().upgrade() {
            u.set_sr_open(false);
        }
        push_detail(app, id);
        return;
    }
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let r = client.seerr_request(d.tv, d.tmdb, &d.seasons).await;
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            let mut p = u.get_sr();
            match r {
                Ok(()) => {
                    p.status = "Demande envoyée".into();
                    p.can_request = false;
                }
                Err(e) => p.status = format!("Demande impossible : {e}").into(),
            }
            u.set_sr(p);
        });
        app2.home_stale.store(true, Ordering::SeqCst);
    });
}

fn card_data(c: &api::CardInfo) -> CardData {
    CardData {
        id: c.id.clone().into(),
        title: c.title.clone().into(),
        subtitle: c.subtitle.clone().into(),
        progress: c.progress,
        rating: c.rating.clone().into(),
        seerr: c.seerr,
        status: c.status,
        count: c.count,
        played: c.played,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Bibliothèque : grille de posters, chargée par pages de 60
// ---------------------------------------------------------------------------
const LIB_PAGE: u32 = 60;

fn open_library(app: &Arc<App>, client: &api::Client, lib: api::Item) {
    // Retour depuis une fiche : on recharge assez de posters pour retrouver la sélection.
    let back_to = match app.lib_return.lock().unwrap().take() {
        Some((id, sel)) if id == lib.id => Some(sel),
        _ => None,
    };
    *app.library.lock().unwrap() = Some((lib.id.clone(), lib.collection_type.clone(), 0, 0));
    let at_root = app.stack.lock().unwrap().len() <= 1;
    let title = lib.name.clone();
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        u.set_lib_title(title.into());
        u.set_lib_items(ModelRc::new(VecModel::<CardData>::default()));
        u.set_lib_total(0);
        u.set_l_sel(0);
        u.set_h_focus(false);
        u.set_at_lib_root(at_root);
        // Retour depuis une fiche : la page n'apparaît qu'une fois la carte d'origine rechargée
        // (sinon la grille défilerait depuis le haut jusqu'à elle).
        if back_to.is_some() {
            begin_loading(&u);
        } else {
            show_screen(&u, "library");
        }
    });
    load_library_page(app, client, back_to);
}

/// Charge la page suivante de la bibliothèque affichée (rien si tout est chargé ou déjà en cours).
/// `select` : poster à resélectionner (retour depuis une fiche) ; la page est agrandie pour l'inclure.
fn load_library_page(app: &Arc<App>, client: &api::Client, select: Option<usize>) {
    let Some((id, ctype, loaded, total)) = app.library.lock().unwrap().clone() else { return };
    if (loaded > 0 && loaded >= total) || app.lib_loading.swap(true, Ordering::SeqCst) {
        return;
    }
    let (app2, client2) = (app.clone(), client.clone());
    app.rt.spawn(async move {
        let limit = select.map(|s| (s as u32 + LIB_PAGE / 2).max(LIB_PAGE)).unwrap_or(LIB_PAGE);
        let page = client2.library_page(&id, ctype.as_deref(), loaded, limit).await;
        app2.lib_loading.store(false, Ordering::SeqCst);
        let (items, total) = match page {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Bibliothèque illisible : {e}");
                let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
                return;
            }
        };
        // La bibliothèque a pu changer entre-temps (retour, autre bibliothèque).
        {
            let mut lib = app2.library.lock().unwrap();
            match lib.as_mut() {
                Some((cur, _, l, t)) if *cur == id => {
                    *l += items.len() as u32;
                    *t = total;
                }
                _ => return,
            }
        }
        let cards: Vec<api::CardInfo> = items.iter().map(|i| i.card()).collect();
        let jobs: Vec<ImageJob> = cards
            .iter()
            .enumerate()
            .filter_map(|(i, c)| ImageJob::for_card(0, loaded as usize + i, c, false))
            .collect();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            let model = u.get_lib_items();
            if let Some(vm) = model.as_any().downcast_ref::<VecModel<CardData>>() {
                for c in &cards {
                    vm.push(CardData {
                        id: c.id.clone().into(),
                        title: c.title.clone().into(),
                        subtitle: c.subtitle.clone().into(),
                        progress: c.progress,
                        rating: c.rating.clone().into(),
                                seerr: c.seerr,
                                status: c.status,
                                count: c.count,
                                played: c.played,
                        ..Default::default()
                    });
                }
            }
            u.set_lib_total(total as i32);
            // Première page : les lignes arrivent en cascade.
            if loaded == 0 && select.is_none() {
                u.invoke_page_enter();
            }
            if let Some(sel) = select {
                u.set_l_sel(sel.min(u.get_lib_items().row_count().saturating_sub(1)) as i32);
                show_screen(&u, "library");
            }
        });
        let card_w = if app2.tv() { 230.0 } else { 170.0 };
        let apply: Apply = Arc::new(|u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            let model = u.get_lib_items();
            let Some(mut card) = model.row_data(job.b) else { return };
            if card.id.as_str() != job.item_id {
                return;
            }
            card.image = slint::Image::from_rgba8(buf);
            card.has_image = true;
            model.set_row_data(job.b, card);
        });
        spawn_image_jobs(&app2, &client2, jobs, api::Size::Fill(270, 405), Shape::card_top(270, 405, card_w), apply);
    });
}

// ---------------------------------------------------------------------------
// Serveurs : recherche (local + Tailscale), saisie manuelle, paramètres réseau
// ---------------------------------------------------------------------------
fn open_servers(app: &Arc<App>) {
    if let Some(u) = app.ui().upgrade() {
        u.set_srv_sel(0);
        u.set_manual_open(false);
        u.set_choice_a("".into());
        u.set_choice_b("".into());
        u.set_h_focus(false);
        u.set_screen("servers".into());
    }
    search_servers(app);
}

fn search_servers(app: &Arc<App>) {
    let ui = app.ui();
    let _ = ui.upgrade_in_event_loop(|u| u.set_srv_searching(true));
    let app2 = app.clone();
    app.rt.spawn(async move {
        let found = discovery::discover().await;
        *app2.found.lock().unwrap() = found.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            let rows: Vec<ServerData> = found
                .iter()
                .map(|f| ServerData {
                    id: f.id.clone().into(),
                    name: f.name.clone().into(),
                    version: f.version.clone().into(),
                    local: f.local.clone().unwrap_or_default().into(),
                    remote: f.remote.clone().unwrap_or_default().into(),
                })
                .collect();
            let n = rows.len() as i32;
            u.set_servers(ModelRc::new(VecModel::from(rows)));
            u.set_srv_searching(false);
            // Rien trouvé : sélection sur « Saisir une adresse ».
            u.set_srv_sel(if n == 0 { 1 } else { 0 });
        });
    });
}

/// Serveur choisi : écran de connexion sur la meilleure adresse.
fn choose_server(app: &Arc<App>, local: String, remote: String) {
    let prefer_remote = config::load().prefer_remote;
    *app.pending_server.lock().unwrap() = Some((local.clone(), remote.clone()));
    let app2 = app.clone();
    app.rt.spawn(async move {
        let best = discovery::pick(&local, &remote, prefer_remote).await;
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            u.set_server(best.into());
            u.set_manual_open(false);
            u.set_choice_a("".into());
            u.set_error_text("".into());
            u.set_busy(false);
            u.set_screen("login".into());
            u.invoke_login_opened();
        });
    });
}

/// Saisie manuelle : chaque adresse est vérifiée (http et https si non précisé). Si les deux
/// répondent, l'utilisateur choisit ; sinon l'adresse qui répond est retenue.
fn manual_submit(app: &Arc<App>, local: String, remote: String) {
    let app2 = app.clone();
    let _ = app.ui().upgrade_in_event_loop(|u| u.set_manual_busy(true));
    app.rt.spawn(async move {
        let mut chosen: Vec<Option<String>> = Vec::new();
        let mut ids: Vec<String> = Vec::new();
        for input in [&local, &remote] {
            if input.trim().is_empty() {
                chosen.push(None);
                continue;
            }
            let hits = discovery::resolve(input).await;
            match hits.len() {
                0 => {
                    let msg = format!("Aucun serveur Jellyfin ne répond à « {} ».", input.trim());
                    let _ = app2.ui().upgrade_in_event_loop(move |u| {
                        u.set_manual_busy(false);
                        u.set_manual_error(msg.into());
                    });
                    return;
                }
                1 => {
                    ids.push(hits[0].1.clone());
                    chosen.push(Some(hits[0].0.clone()));
                }
                _ => {
                    // Deux réponses (http et https) : on demande, puis on reprendra là.
                    ids.push(hits[0].1.clone());
                    chosen.push(None);
                    let (a, b) = (hits[0].0.clone(), hits[1].0.clone());
                    *app2.manual.lock().unwrap() = (
                        chosen.first().cloned().flatten(),
                        None,
                        vec![local.clone(), remote.clone(), (chosen.len() - 1).to_string()],
                    );
                    let _ = app2.ui().upgrade_in_event_loop(move |u| {
                        u.set_manual_busy(false);
                        u.set_choice_a(a.into());
                        u.set_choice_b(b.into());
                    });
                    return;
                }
            }
        }
        if ids.len() == 2 && ids[0] != ids[1] {
            let _ = app2.ui().upgrade_in_event_loop(|u| {
                u.set_manual_busy(false);
                u.set_manual_error("Les deux adresses mènent à deux serveurs différents.".into());
            });
            return;
        }
        let (l, r) = (chosen[0].clone().unwrap_or_default(), chosen[1].clone().unwrap_or_default());
        if l.is_empty() && r.is_empty() {
            let _ = app2.ui().upgrade_in_event_loop(|u| {
                u.set_manual_busy(false);
                u.set_manual_error("Indique au moins une adresse.".into());
            });
            return;
        }
        let _ = app2.ui().upgrade_in_event_loop(|u| u.set_manual_busy(false));
        choose_server(&app2, l, r);
    });
}

/// Réponse au choix http / https : on fixe l'adresse concernée et on termine la saisie.
fn manual_choice(app: &Arc<App>, url: String) {
    let (first, _, ctx) = app.manual.lock().unwrap().clone();
    if let Some(u) = app.ui().upgrade() {
        u.set_choice_a("".into());
        u.set_choice_b("".into());
    }
    if url.is_empty() || ctx.len() < 3 {
        return;
    }
    let (local, remote, which) = (ctx[0].clone(), ctx[1].clone(), ctx[2].clone());
    if which == "0" {
        // L'adresse locale était ambiguë : on la remplace par le choix et on vérifie la distante.
        manual_submit(app, url, remote);
    } else {
        // La distante était ambiguë ; la locale était déjà validée.
        manual_submit(app, first.unwrap_or(local), url);
    }
}

/// Une des deux adresses du serveur est inconnue : recherche discrète en arrière-plan du même
/// serveur (même identifiant) pour la compléter, puis passage sur l'adresse préférée.
fn complete_addresses(app: &Arc<App>, client: &api::Client) {
    let saved = config::load();
    if !saved.server_local.is_empty() && !saved.server_remote.is_empty() {
        return;
    }
    let (app2, base) = (app.clone(), client.server.clone());
    app.rt.spawn(async move {
        let http = reqwest::Client::new();
        let Some((id, ..)) = discovery::probe(&http, &base).await else { return };
        let Some(f) = discovery::discover().await.into_iter().find(|f| f.id == id) else { return };
        let mut s = config::load();
        if s.server_local.is_empty() {
            s.server_local = f.local.unwrap_or_default();
        }
        if s.server_remote.is_empty() {
            s.server_remote = f.remote.unwrap_or_default();
        }
        let best = discovery::pick(&s.server_local, &s.server_remote, s.prefer_remote).await;
        if !best.is_empty() && best != base {
            s.server = best.clone();
            if let Some(c) = app2.client.lock().unwrap().as_mut() {
                c.set_server(&best);
            }
        }
        config::save(&s);
    });
}

// ---------------------------------------------------------------------------
// Paramètres : Profil (avatar), Lecture, Réseau, Affichage, Compte
// ---------------------------------------------------------------------------
const LANGS: [(&str, &str); 9] = [
    ("", "Aucune préférence"),
    ("fre", "Français"),
    ("eng", "Anglais"),
    ("jpn", "Japonais"),
    ("ger", "Allemand"),
    ("spa", "Espagnol"),
    ("ita", "Italien"),
    ("kor", "Coréen"),
    ("chi", "Chinois"),
];
const SUB_MODES: [(&str, &str); 5] = [
    ("Default", "Par défaut"),
    ("Smart", "Intelligent"),
    ("OnlyForced", "Forcés uniquement"),
    ("Always", "Toujours"),
    ("None", "Aucun"),
];

fn apply_user_defaults(cfg: &serde_json::Value) {
    config::set_user_defaults(
        cfg["AudioLanguagePreference"].as_str().unwrap_or(""),
        cfg["SubtitleLanguagePreference"].as_str().unwrap_or(""),
        cfg["SubtitleMode"].as_str().unwrap_or("Default"),
    );
}

fn label_of<'a>(list: &[(&'a str, &'a str)], v: &str) -> &'a str {
    list.iter().find(|(k, _)| *k == v).map(|(_, l)| *l).unwrap_or(list[0].1)
}

/// Lignes de la catégorie affichée : (clé, libellé, valeur, type « toggle » / « choice » / « action » / « info »).
/// Tailles de sous-titres proposées (mpv `sub-scale`).
const SUB_SIZES: [(&str, &str); 5] = [("0.8", "Petite"), ("1", "Normale"), ("1.2", "Grande"), ("1.45", "Très grande"), ("1.7", "Énorme")];

fn sub_size_label(v: f64) -> &'static str {
    SUB_SIZES.iter().min_by(|a, b| (a.0.parse::<f64>().unwrap_or(1.0) - v).abs().total_cmp(&(b.0.parse::<f64>().unwrap_or(1.0) - v).abs())).map(|x| x.1).unwrap_or("Normale")
}

/// Réglages de l'appareil appliqués à l'interface (global Prefs).
fn apply_ui_prefs(u: &AppWindow, p: &config::UiPrefs) {
    let g = u.global::<Prefs>();
    g.set_ratings(p.show_ratings);
    g.set_marquee(p.marquee);
    g.set_clock(p.show_clock);
}

/// Taille du cache d'images sur le disque (octets).
fn cache_size() -> u64 {
    let Some(dir) = api::image_cache_dir() else { return 0 };
    std::fs::read_dir(dir).map(|d| d.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
}

/// Lignes de la catégorie affichée : (clé, libellé, aide, valeur, type « toggle » / « choice » / « action » / « info »).
/// Catégories : 0 Profil · 1 Lecture · 2 Sous-titres · 3 Affichage · 4 Réseau · 5 Compte · 6 À propos.
fn settings_rows(app: &Arc<App>, cat: i32) -> Vec<SettingRow> {
    let row = |key: &str, label: &str, hint: &str, value: String, kind: &str, on: bool| SettingRow {
        key: key.into(),
        label: label.into(),
        hint: hint.into(),
        value: value.into(),
        kind: kind.into(),
        on,
    };
    let prefs = config::ui_prefs();
    let c = app.user_cfg.lock().unwrap().clone();
    let s = |k: &str| c[k].as_str().unwrap_or("").to_string();
    match cat {
        1 => vec![
            row("alang", "Langue audio préférée", "Choisie à l'ouverture d'un film ou d'un épisode, si elle existe.", label_of(&LANGS, &s("AudioLanguagePreference")).into(), "choice", false),
            row("defaudio", "Piste audio par défaut du fichier", "Sans langue préférée, la piste marquée « par défaut » est lue.", String::new(), "toggle", c["PlayDefaultAudioTrack"].as_bool().unwrap_or(true)),
            row("autonext", "Épisode suivant automatique", "À la fin d'un épisode, le suivant démarre tout seul.", String::new(), "toggle", c["EnableNextEpisodeAutoPlay"].as_bool().unwrap_or(true)),
            row("autoskip", "Passer l'intro automatiquement", "Quand le serveur connaît l'intro (segments), elle est sautée sans demander.", String::new(), "toggle", prefs.auto_skip_intro),
            row("info", "", "Un choix fait sur la fiche d'un film ou d'une série reste prioritaire.", String::new(), "info", false),
        ],
        2 => vec![
            row("slang", "Langue des sous-titres préférée", "Utilisée selon le mode ci-dessous.", label_of(&LANGS, &s("SubtitleLanguagePreference")).into(), "choice", false),
            row("submode", "Quand afficher les sous-titres", "Par défaut : selon le fichier · Intelligent : si l'audio n'est pas dans ta langue.", label_of(&SUB_MODES, &s("SubtitleMode")).into(), "choice", false),
            row("subsize", "Taille des sous-titres", "Appliquée à la prochaine vidéo.", sub_size_label(prefs.sub_scale).into(), "choice", false),
        ],
        3 => vec![
            row("tvmode", "Interface TV", "Grands éléments et plein écran, pour la télé (--tv et --desktop priment).", String::new(), "toggle", app.tv()),
            row("backdrop", "Fond d'écran du média sélectionné", "Image floutée derrière les pages. À couper si l'appareil est lent.", String::new(), "toggle", !NO_BACKDROP.load(Ordering::Relaxed)),
            row("ratings", "Notes sur les affiches", "La note de la communauté (★) en bas à droite des affiches.", String::new(), "toggle", prefs.show_ratings),
            row("marquee", "Faire défiler les noms trop longs", "Sur l'élément sélectionné seulement.", String::new(), "toggle", prefs.marquee),
            row("clock", "Afficher l'heure", "En haut à droite de l'écran.", String::new(), "toggle", prefs.show_clock),
            row("still_gifs", "Avatars animés figés", "Les avatars GIF restent sur leur première image (moins de calcul).", String::new(), "toggle", STILL_GIFS.load(Ordering::Relaxed)),
        ],
        4 => {
            let saved = config::load();
            let current = app.client().map(|c| c.server).unwrap_or_default();
            vec![
                row("prefer_local", "Privilégier l'adresse locale", "Si elle répond, sinon l'adresse distante (Tailscale...).", String::new(), "toggle", !saved.prefer_remote),
                row("info", "", &format!("Adresse locale : {}", if saved.server_local.is_empty() { "inconnue" } else { &saved.server_local }), String::new(), "info", false),
                row("info", "", &format!("Adresse distante : {}", if saved.server_remote.is_empty() { "inconnue" } else { &saved.server_remote }), String::new(), "info", false),
                row("info", "", &format!("Utilisée en ce moment : {current}"), String::new(), "info", false),
                row("server", "Sélectionner un serveur", "", String::new(), "action", false),
            ]
        }
        5 => {
            let who = app.client().map(|c| c.user_name).unwrap_or_default();
            vec![
                row("info", "", &format!("Connecté en tant que {who}"), String::new(), "info", false),
                row("switch", "Changer de compte", "Les comptes enregistrés restent disponibles.", String::new(), "action", false),
                row("logout", "Se déconnecter", "Le compte est retiré de cet appareil.", String::new(), "action", false),
                row("quit", "Fermer l'application", "", String::new(), "action", false),
            ]
        }
        6 => {
            let size = cache_size();
            let srv = app.client().map(|c| c.server).unwrap_or_default();
            vec![
                row("info", "", &format!("Turtlefin {} · client Jellyfin natif (Rust + Slint + mpv)", env!("CARGO_PKG_VERSION")), String::new(), "info", false),
                row("info", "", &format!("Serveur : {srv}"), String::new(), "info", false),
                row("info", "", &format!("Appareil : {}", app.device_id), String::new(), "info", false),
                row("clearcache", "Vider le cache d'images", "Affiches et vignettes gardées sur le disque ; elles seront retéléchargées.", format!("{} Mo", size >> 20), "action", false),
            ]
        }
        _ => Vec::new(),
    }
}

/// Choix proposés pour un réglage « choice » : (titre, [(valeur, libellé)], valeur actuelle).
fn setting_choices(app: &Arc<App>, key: &str) -> Option<(String, Vec<(String, String)>, String)> {
    let c = app.user_cfg.lock().unwrap().clone();
    let s = |k: &str| c[k].as_str().unwrap_or("").to_string();
    let own = |l: &[(&str, &str)]| l.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<Vec<_>>();
    match key {
        "alang" => Some(("Langue audio préférée".into(), own(&LANGS), s("AudioLanguagePreference"))),
        "slang" => Some(("Langue des sous-titres".into(), own(&LANGS), s("SubtitleLanguagePreference"))),
        "submode" => Some(("Quand afficher les sous-titres".into(), own(&SUB_MODES), {
            let m = s("SubtitleMode");
            if m.is_empty() { "Default".into() } else { m }
        })),
        "subsize" => {
            let cur = config::ui_prefs().sub_scale;
            let v = SUB_SIZES.iter().find(|x| sub_size_label(cur) == x.1).map(|x| x.0).unwrap_or("1");
            Some(("Taille des sous-titres".into(), own(&SUB_SIZES), v.into()))
        }
        _ => None,
    }
}

/// Ouvre la liste de choix d'un réglage (← → / ↑ ↓ pour choisir, Entrée pour valider).
fn open_choice(app: &Arc<App>, key: &str) {
    let Some((title, list, cur)) = setting_choices(app, key) else { return };
    let sel = list.iter().position(|(k, _)| *k == cur).unwrap_or(0) as i32;
    if let Some(u) = app.ui().upgrade() {
        let rows: Vec<TrackData> = list.iter().map(|(k, l)| TrackData { id: k.clone().into(), label: l.clone().into(), current: *k == cur }).collect();
        u.set_ch_items(ModelRc::new(VecModel::from(rows)));
        u.set_ch_title(title.into());
        u.set_ch_sel(sel);
        u.set_ch_key(key.into());
    }
}

/// Valeur choisie dans la liste d'un réglage.
fn choose_setting(app: &Arc<App>, key: &str, value: &str) {
    if key == "subsize" {
        let mut p = config::ui_prefs();
        p.sub_scale = value.parse().unwrap_or(1.0);
        config::save_ui_prefs(&p);
        refresh_settings(app);
        return;
    }
    let field = match key {
        "alang" => "AudioLanguagePreference",
        "slang" => "SubtitleLanguagePreference",
        "submode" => "SubtitleMode",
        _ => return,
    };
    let mut cfg = app.user_cfg.lock().unwrap().clone();
    if !cfg.is_object() {
        return;
    }
    cfg[field] = value.into();
    save_user_cfg(app, cfg);
}

/// Préférences du compte Jellyfin : appliquées, affichées, puis envoyées au serveur.
fn save_user_cfg(app: &Arc<App>, cfg: serde_json::Value) {
    apply_user_defaults(&cfg);
    *app.user_cfg.lock().unwrap() = cfg.clone();
    refresh_settings(app);
    if let Some(client) = app.client() {
        let app2 = app.clone();
        app.rt.spawn(async move {
            if let Err(e) = client.set_user_config(&cfg).await {
                let msg = format!("Réglage non enregistré : {e}");
                let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
            }
        });
    }
}

/// Réglage de l'appareil à bascule (prefs.json).
fn toggle_ui_pref(app: &Arc<App>, f: impl Fn(&mut config::UiPrefs)) {
    let mut p = config::ui_prefs();
    f(&mut p);
    config::save_ui_prefs(&p);
    if let Some(u) = app.ui().upgrade() {
        apply_ui_prefs(&u, &p);
    }
    refresh_settings(app);
}

fn refresh_settings(app: &Arc<App>) {
    let Some(u) = app.ui().upgrade() else { return };
    let rows = settings_rows(app, u.get_set_cat());
    u.set_set_rows(ModelRc::new(VecModel::from(rows)));
}

fn open_settings(app: &Arc<App>) {
    if let Some(u) = app.ui().upgrade() {
        u.set_set_cat(0);
        u.set_set_sel(0);
        u.set_set_in(false);
        u.set_h_focus(false);
        u.set_screen("settings".into());
    }
    refresh_settings(app);
    load_avatars(app);
}

/// Avatars proposés par GetAvatar (images décodées une fois, GIF compris : première image).
fn load_avatars(app: &Arc<App>) {
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let list = client.avatars().await;
        *app2.avatars.lock().unwrap() = list.clone();
        let names: Vec<(String, String)> = list.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            let rows: Vec<CardData> = names.iter().map(|(id, n)| CardData { id: id.clone().into(), title: n.clone().into(), ..Default::default() }).collect();
            u.set_avatar_items(ModelRc::new(VecModel::from(rows)));
            u.set_has_getavatar(!names.is_empty());
        });
        for (i, (id, _)) in list.into_iter().enumerate() {
            let (c, ui) = (client.clone(), app2.ui());
            app2.rt.spawn(async move {
                let Ok(bytes) = c.get_bytes(&format!("/GetAvatar/Image/{id}")).await else { return };
                let Some(frames) = tokio::task::spawn_blocking(move || decode_frames(&bytes, Shape { w: 160, h: 160, radius: 0.5, top_only: false }))
                    .await
                    .ok()
                    .flatten()
                else {
                    return;
                };
                let _ = ui.upgrade_in_event_loop(move |u| show_frames(&u, AnimSlot::Picker(i), &id, frames));
            });
        }
    });
}

/// Avatar du compte, rond, dans l'en-tête et les paramètres.
fn load_header_avatar(app: &Arc<App>, client: &api::Client) {
    let (c, ui) = (client.clone(), app.ui());
    app.rt.spawn(async move {
        let Some(bytes) = c.user_avatar(!STILL_GIFS.load(Ordering::Relaxed)).await else {
            let _ = ui.upgrade_in_event_loop(|u| {
                stop_anims(|s| s == AnimSlot::Header);
                u.set_has_avatar(false);
            });
            return;
        };
        let Some(frames) = tokio::task::spawn_blocking(move || decode_frames(&bytes, Shape { w: 160, h: 160, radius: 0.5, top_only: false }))
            .await
            .ok()
            .flatten()
        else {
            return;
        };
        let _ = ui.upgrade_in_event_loop(move |u| show_frames(&u, AnimSlot::Header, "", frames));
    });
}

fn set_avatar(app: &Arc<App>, id: String) {
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let r = match client.set_avatar(&id).await {
            Ok(()) => Ok(()),
            Err(_) => client.upload_avatar(&id).await,
        };
        let msg = match r {
            Ok(()) => {
                load_header_avatar(&app2, &client);
                "Avatar modifié.".to_string()
            }
            Err(e) => format!("Avatar impossible : {e}"),
        };
        let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
    });
}

/// Ligne de paramètre activée (Entrée / clic).
fn settings_activate(app: &Arc<App>, key: &str) {
    match key {
        "alang" | "slang" | "submode" | "subsize" => open_choice(app, key),
        "defaudio" | "autonext" => {
            let mut cfg = app.user_cfg.lock().unwrap().clone();
            if !cfg.is_object() {
                return;
            }
            let field = if key == "defaudio" { "PlayDefaultAudioTrack" } else { "EnableNextEpisodeAutoPlay" };
            let v = cfg[field].as_bool().unwrap_or(true);
            cfg[field] = (!v).into();
            save_user_cfg(app, cfg);
        }
        "autoskip" => toggle_ui_pref(app, |p| p.auto_skip_intro = !p.auto_skip_intro),
        "ratings" => toggle_ui_pref(app, |p| p.show_ratings = !p.show_ratings),
        "marquee" => toggle_ui_pref(app, |p| p.marquee = !p.marquee),
        "clock" => toggle_ui_pref(app, |p| p.show_clock = !p.show_clock),
        "tvmode" => {
            let on = !app.tv();
            toggle_ui_pref(app, |p| p.tv = on);
            app.tv_flag.store(on, Ordering::Relaxed);
            if let Some(u) = app.ui().upgrade() {
                u.set_tv_mode(on);
                u.window().set_fullscreen(on);
            }
            refresh_settings(app);
        }
        "clearcache" => {
            if let Some(dir) = api::image_cache_dir() {
                for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                    let _ = std::fs::remove_file(e.path());
                }
            }
            if let Some(u) = app.ui().upgrade() {
                u.set_toast("Cache d'images vidé.".into());
            }
            refresh_settings(app);
        }
        "switch" => end_session(app, false),
        "prefer_local" => {
            let now_remote = config::load().prefer_remote;
            set_prefer_remote(app, !now_remote);
        }
        "backdrop" => {
            let mut saved = config::load();
            saved.no_backdrop = !saved.no_backdrop;
            config::save(&saved);
            NO_BACKDROP.store(saved.no_backdrop, Ordering::Relaxed);
            app.bg_id.lock().unwrap().clear();
            if let Some(u) = app.ui().upgrade() {
                u.set_bg_show(!saved.no_backdrop);
            }
            refresh_settings(app);
        }
        "still_gifs" => {
            let mut saved = config::load();
            saved.still_gifs = !saved.still_gifs;
            config::save(&saved);
            STILL_GIFS.store(saved.still_gifs, Ordering::Relaxed);
            refresh_settings(app);
            // Avatars rechargés : animés (fichier d'origine) ou figés.
            if let Some(client) = app.client() {
                load_header_avatar(app, &client);
            }
            load_avatars(app);
        }
        "server" => open_servers(app),
        "logout" => {
            if let Some(u) = app.ui().upgrade() {
                u.invoke_logout();
            }
        }
        "quit" => {
            let _ = slint::quit_event_loop();
        }
        _ => {}
    }
}

/// Préférence réseau : enregistrée, puis la meilleure adresse est choisie tout de suite.
fn set_prefer_remote(app: &Arc<App>, prefer: bool) {
    let mut saved = config::load();
    saved.prefer_remote = prefer;
    config::save(&saved);
    refresh_settings(app);
    let app2 = app.clone();
    app.rt.spawn(async move {
        let best = discovery::pick(&saved.server_local, &saved.server_remote, prefer).await;
        if best.is_empty() {
            return;
        }
        if let Some(c) = app2.client.lock().unwrap().as_mut() {
            c.set_server(&best);
        }
        let mut s = config::load();
        s.server = best.clone();
        config::save(&s);
        let a3 = app2.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |_| refresh_settings(&a3));
        let _ = best;
    });
}

// ---------------------------------------------------------------------------
// Menu latéral : bibliothèques, demandes, compte
// ---------------------------------------------------------------------------
fn set_menu(app: &Arc<App>, views: &[api::Item]) {
    *app.views.lock().unwrap() = views.to_vec();
    let has_requests = app.seerr_user.lock().unwrap().is_some();
    let mut e: Vec<(String, String, bool)> = vec![
        ("Navigation".into(), String::new(), true),
        ("Accueil".into(), "home".into(), false),
    ];
    if has_requests {
        e.push(("Demandes".into(), "requests".into(), false));
    }
    e.push(("Téléchargements".into(), "downloads".into(), false));
    e.push(("Watch party".into(), "party".into(), false));
    e.push(("Bibliothèques".into(), String::new(), true));
    for v in views.iter().filter(|v| v.collection_type.as_deref() != Some("livetv")) {
        e.push((v.name.clone(), format!("lib:{}", v.id), false));
    }
    e.push(("Compte".into(), String::new(), true));
    for (label, action) in [
        ("Changer de compte", "switch"),
        ("Sélectionner un serveur", "server"),
        ("Paramètres", "settings"),
        ("Se déconnecter", "logout"),
        ("Fermer l'application", "quit"),
    ] {
        e.push((label.into(), action.into(), false));
    }
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        let entries: Vec<MenuEntry> = e
            .into_iter()
            .map(|(label, action, header)| MenuEntry { label: label.into(), action: action.into(), header })
            .collect();
        u.set_menu_entries(ModelRc::new(VecModel::from(entries)));
    });
}

// ---------------------------------------------------------------------------
// Navigation : fiche détail
// ---------------------------------------------------------------------------
fn push_detail(app: &Arc<App>, id: String) {
    if id.starts_with("person:") {
        return;
    }
    if let Some(rest) = id.strip_prefix("seerr:") {
        let (kind, tmdb) = rest.split_once(':').unwrap_or(("movie", "0"));
        open_seerr(app, kind == "tv", tmdb.parse().unwrap_or(0));
        return;
    }
    if id.is_empty() {
        if let Some(u) = app.ui().upgrade() {
            u.set_toast("Pas (encore) dans ta bibliothèque Jellyfin.".into());
        }
        return;
    }
    if let Some(u) = app.ui().upgrade() {
        cache_current_page(app, &u);
        if app.stack.lock().unwrap().is_empty() {
            let from_dl = u.get_screen().as_str() == "downloads";
            *app.stack_base.lock().unwrap() = if from_dl { "downloads".into() } else { String::new() };
            if from_dl {
                u.set_here_lib("downloads".into());
            }
        }
    }
    // Depuis une bibliothèque : on retient le poster sélectionné pour le retour.
    if let (Some(u), Some((lib, ..))) = (app.ui().upgrade(), app.library.lock().unwrap().clone()) {
        if u.get_screen().as_str() == "library" {
            *app.lib_return.lock().unwrap() = Some((lib, u.get_l_sel().max(0) as usize));
        }
    }
    app.stack.lock().unwrap().push(id.clone());
    sync_can_back(app);
    start_detail(app, id);
}

/// Le bouton Retour n'a de sens que si l'on a navigué depuis l'accueil.
fn sync_can_back(app: &Arc<App>) {
    let (can, prev) = {
        let st = app.stack.lock().unwrap();
        (!st.is_empty(), if st.len() >= 2 { st.get(st.len() - 2).cloned() } else { None })
    };
    // Retour vers une autre fiche (pas une bibliothèque) : échange en douceur, sans voile.
    let soft = prev.is_some_and(|id| {
        let lib = PAGES.with_borrow(|p| matches!(p.get(&id), Some(CachedPage::Library { .. })))
            || app.views.lock().unwrap().iter().any(|v| v.id == id);
        !lib
    });
    if let Some(u) = app.ui().upgrade() {
        u.set_can_back(can);
        u.set_back_soft(soft);
    }
}

/// Retour direct à l'accueil (bouton Accueil, menu).
fn go_home(app: &Arc<App>) {
    // Hors ligne : les téléchargements tiennent lieu d'accueil.
    if app.offline.load(Ordering::SeqCst) {
        open_downloads(app);
        return;
    }
    if app.client().is_none() {
        if let Some(u) = app.ui().upgrade() {
            u.set_screen("login".into());
        }
        return;
    }
    app.stack.lock().unwrap().clear();
    sync_can_back(app);
    app.gen.fetch_add(1, Ordering::SeqCst);
    *app.library.lock().unwrap() = None;
    let stale = app.home_stale.swap(false, Ordering::SeqCst);
    if let Some(u) = app.ui().upgrade() {
        u.set_detail(DetailData::default());
        u.set_child_items(ModelRc::default());
        u.set_lib_items(ModelRc::default());
        u.set_overview_open(false);
        u.set_h_focus(false);
        if stale {
            begin_loading(&u);
        } else {
            show_screen(&u, "home");
        }
    }
    if stale {
        if let Some(client) = app.client() {
            let a = app.clone();
            app.rt.spawn(async move { load_tab(a, client).await });
        }
    }
}

fn start_detail(app: &Arc<App>, id: String) {
    if id.starts_with("dl:") {
        show_local_detail(app, &id);
        return;
    }
    let Some(client) = app.client() else { return };
    let my_gen = app.gen.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(u) = app.ui().upgrade() {
        u.set_overview_open(false);
        u.set_toast("".into());
        begin_loading(&u);
    }
    let app2 = app.clone();
    app.rt.spawn(async move {
        load_detail(app2, client, id, my_gen).await;
    });
}

/// Fiche d'une série : saisons absentes du serveur et pas encore demandées -> bouton « Demander ».
async fn add_missing_seasons_button(app: Arc<App>, client: api::Client, id: String, tmdb: i64) {
    let Ok(d) = client.seerr_details(true, tmdb).await else { return };
    if d.missing_seasons.is_empty() {
        return;
    }
    let n = d.missing_seasons.len();
    let label = if n == 1 {
        format!("Demander la saison {}", d.missing_seasons[0])
    } else {
        format!("Demander les {n} saisons manquantes")
    };
    *app.missing_seasons.lock().unwrap() = (id.clone(), tmdb, d.missing_seasons);
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        let detail = u.get_detail();
        if detail.id.as_str() != id {
            return;
        }
        if let Some(m) = detail.buttons.as_any().downcast_ref::<VecModel<ButtonData>>() {
            if !(0..m.row_count()).any(|i| m.row_data(i).is_some_and(|b| b.action.as_str() == "seerr-missing")) {
                m.push(ButtonData { label: label.into(), action: "seerr-missing".into(), icon: "".into(), active: false });
            }
        }
    });
}

/// Demande à Seerr les saisons manquantes de la série affichée.
fn request_missing_seasons(app: &Arc<App>) {
    let (id, tmdb, seasons) = app.missing_seasons.lock().unwrap().clone();
    let Some(client) = app.client() else { return };
    if seasons.is_empty() {
        return;
    }
    let a = app.clone();
    app.rt.spawn(async move {
        let r = client.seerr_request(true, tmdb, &seasons).await;
        let list = seasons.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ");
        let ok = r.is_ok();
        let msg = match r {
            Ok(()) => format!("Demande envoyée à Seerr : saison(s) {list}."),
            Err(e) => format!("Demande refusée par Seerr : {e}"),
        };
        if ok {
            a.missing_seasons.lock().unwrap().2.clear();
        }
        let _ = a.ui().upgrade_in_event_loop(move |u| {
            u.set_toast(msg.into());
            let detail = u.get_detail();
            if !ok || detail.id.as_str() != id {
                return;
            }
            if let Some(m) = detail.buttons.as_any().downcast_ref::<VecModel<ButtonData>>() {
                if let Some(i) = (0..m.row_count()).find(|&i| m.row_data(i).is_some_and(|b| b.action.as_str() == "seerr-missing")) {
                    m.set_row_data(i, ButtonData { label: "Saisons demandées".into(), action: "".into(), icon: "".into(), active: true });
                }
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Pages quittées (fiches, bibliothèques) gardées telles quelles : le retour les réaffiche tout de
// suite, sans redemander quoi que ce soit au serveur (les images sont déjà décodées).
// ---------------------------------------------------------------------------
enum CachedPage {
    Detail {
        detail: DetailData,
        child: ModelRc<CardData>,
        rows: ModelRc<Section>,
        zone: (i32, i32, i32, i32),
    },
    Library {
        title: slint::SharedString,
        items: ModelRc<CardData>,
        total: i32,
        sel: i32,
        at_root: bool,
        state: (String, Option<String>, u32, u32),
    },
}

thread_local! {
    static PAGES: std::cell::RefCell<std::collections::HashMap<String, CachedPage>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Garde la page affichée (sommet de la pile) avant d'en ouvrir une autre.
fn cache_current_page(app: &Arc<App>, u: &AppWindow) {
    let Some(top) = app.stack.lock().unwrap().last().cloned() else { return };
    let page = match u.get_screen().as_str() {
        "detail" if u.get_detail().id.as_str() == top => CachedPage::Detail {
            detail: u.get_detail(),
            child: u.get_child_items(),
            rows: u.get_detail_rows(),
            zone: (u.get_d_zone(), u.get_d_x(), u.get_d_child(), u.get_d_button()),
        },
        "library" => match app.library.lock().unwrap().clone() {
            Some(state) if state.0 == top => CachedPage::Library {
                title: u.get_lib_title(),
                items: u.get_lib_items(),
                total: u.get_lib_total(),
                sel: u.get_l_sel(),
                at_root: u.get_at_lib_root(),
                state,
            },
            _ => return,
        },
        _ => return,
    };
    PAGES.with_borrow_mut(|p| {
        p.insert(top, page);
    });
}

/// Réaffiche une page gardée ; false si elle n'est pas (ou plus) en mémoire.
fn restore_page(app: &Arc<App>, id: &str) -> bool {
    let Some(page) = PAGES.with_borrow_mut(|p| p.remove(id)) else { return false };
    let Some(u) = app.ui().upgrade() else { return false };
    app.gen.fetch_add(1, Ordering::SeqCst);
    match page {
        CachedPage::Detail { detail, child, rows, zone } => {
            set_detail_smooth(&u, detail);
            u.set_child_items(child);
            u.set_detail_rows(rows);
            *app.rows_base.lock().unwrap() = String::new();
            // Retour : la sélection revient sur Lecture (comme en ouvrant la fiche).
            u.set_d_zone(0);
            u.set_d_x(zone.1);
            u.set_d_child(zone.2);
            u.set_d_button(0);
            u.set_overview_open(false);
            show_screen(&u, "detail");
        }
        CachedPage::Library { title, items, total, sel, at_root, state } => {
            *app.library.lock().unwrap() = Some(state);
            u.set_lib_title(title);
            u.set_lib_items(items);
            u.set_lib_total(total);
            u.set_l_sel(sel);
            u.set_at_lib_root(at_root);
            u.set_h_focus(false);
            show_screen(&u, "library");
        }
    }
    true
}

fn go_back(app: &Arc<App>) {
    let top = {
        let mut s = app.stack.lock().unwrap();
        s.pop();
        s.last().cloned()
    };
    sync_can_back(app);
    match top {
        // Page gardée : tout de suite (sauf après une lecture : état « vu » / reprise à jour).
        Some(id) if !app.home_stale.load(Ordering::SeqCst) && restore_page(app, &id) => {}
        Some(id) => start_detail(app, id),
        // Pile vide : accueil (rechargé si une lecture a eu lieu : Reprendre / À suivre à jour).
        None => {
            PAGES.with_borrow_mut(|p| p.clear());
            if std::mem::take(&mut *app.stack_base.lock().unwrap()) == "downloads" {
                open_downloads(app);
            } else {
                go_home(app)
            }
        }
    }
}

async fn load_detail(app: Arc<App>, client: api::Client, id: String, my_gen: u64) {
    let ui = app.ui();

    let item = match client.item(&id).await {
        Ok(i) => i,
        Err(e) => {
            if e.downcast_ref::<api::Unauthorized>().is_some() {
                handle_error(&ui, e);
                return;
            }
            if app.gen.load(Ordering::SeqCst) != my_gen {
                return;
            }
            // On annule cette ouverture et on revient à l'écran précédent.
            let prev_empty = {
                let mut s = app.stack.lock().unwrap();
                s.pop();
                s.is_empty()
            };
            let msg = format!("Impossible d'ouvrir la fiche : {e}");
            let _ = ui.upgrade_in_event_loop(move |u| {
                u.set_toast(msg.into());
                u.set_loading(false);
                let screen = if prev_empty { "home" } else { "detail" };
                u.set_screen(screen.into());
            });
            return;
        }
    };

    if matches!(item.kind.as_str(), "CollectionFolder" | "UserView" | "Folder" | "BoxSet") {
        if app.gen.load(Ordering::SeqCst) == my_gen {
            open_library(&app, &client, item);
        }
        return;
    }

    // Séries/saisons : tout. Autres dossiers : 60 premiers seulement.
    let is_season_like = matches!(item.kind.as_str(), "Series" | "Season");
    let limit: u32 = if is_season_like { 300 } else { 60 };
    let children: Vec<api::Item> = if item.is_folder {
        let sort = if is_season_like { "IndexNumber,SortName" } else { "SortName" };
        client.children(&id, sort, limit).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let truncated = !is_season_like && children.len() as u32 >= limit;

    // Bloc « À suivre » (séries uniquement)
    let next: Option<api::Item> = if item.kind == "Series" {
        client.next_up_for(&id).await.ok().flatten()
    } else {
        None
    };

    if app.gen.load(Ordering::SeqCst) != my_gen {
        return; // l'utilisateur a navigué ailleurs entre-temps
    }

    let (title, subtitle) = item.titles();
    let landscape = !children.is_empty() && children.iter().all(|c| c.kind == "Episode");
    let child_cards: Vec<api::CardInfo> = children.iter().map(|c| c.child_card()).collect();
    let children_title = match item.kind.as_str() {
        "Series" => "Saisons".to_string(),
        "Season" => "Épisodes".to_string(),
        _ if truncated => format!("Contenu · {limit} premiers"),
        _ => "Contenu".to_string(),
    };
    let next_card = next.as_ref().map(|n| (n.id.clone(), n.titles().1, n.child_card()));
    let (next_id, next_title, next_sub) = match &next_card {
        Some((nid, t, c)) => (nid.clone(), t.clone(), c.subtitle.clone()),
        None => Default::default(),
    };
    let has_next = next_card.is_some();
    let mut buttons = item.buttons(app.can_download.load(Ordering::SeqCst));
    // Film / épisode : choix des pistes pour la lecture (retenu pour toute la série).
    let audio_streams = item.streams("Audio");
    let sub_streams = item.streams("Subtitle");
    if matches!(item.kind.as_str(), "Movie" | "Episode") {
        let pref = config::track_pref(&item.pref_key());
        let lang_label = |list: &[(String, String)], lang: &str| -> Option<String> {
            list.iter().find(|(l, _)| l == lang).map(|(_, t)| t.clone())
        };
        if audio_streams.len() > 1 {
            let label = lang_label(&audio_streams, &pref.audio).map(|l| lang_name(&pref.audio, &l)).unwrap_or_else(|| "par défaut".into());
            buttons.push((format!("Audio : {label}"), "pick-audio".into(), String::new(), false));
        }
        if !sub_streams.is_empty() {
            let label = match pref.sub.as_str() {
                "off" => "désactivés".to_string(),
                l => lang_label(&sub_streams, l).map(|t| lang_name(l, &t)).unwrap_or_else(|| "par défaut".into()),
            };
            buttons.push((format!("Sous-titres : {label}"), "pick-sub".into(), String::new(), false));
        }
    }
    *app.detail_streams.lock().unwrap() = (item.pref_key(), audio_streams, sub_streams);
    if downloads::exists(&item.id) {
        for b in buttons.iter_mut().filter(|b| b.1 == "download") {
            b.3 = true;
        }
    }
    let misc = item.misc_line();
    let overview = item.overview.clone().unwrap_or_default();
    let logo = item.logo_candidate();
    let item_id = item.id.clone();

    let child_jobs: Vec<ImageJob> = child_cards
        .iter()
        .enumerate()
        .filter_map(|(ci, c)| ImageJob::for_card(0, ci, c, landscape))
        .collect();

    let expect_logo = logo.is_some();
    let id_for_ui = item_id.clone();
    let logo_key = logo.as_ref().map(|l| l.0.clone()).unwrap_or_default();
    let poster_key = item.poster_candidates().first().map(|c| c.0.clone()).unwrap_or_default();
    let icons_sig = buttons_sig(&buttons, true);
    let chips_sig = buttons_sig(&buttons, false);
    let children_sig = child_cards.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(",");
    // Rangées du bas (plus de ce genre...) : celles de la série pour un épisode / une saison ;
    // identiques d'une fiche à l'autre de la même série, elles restent.
    let rows_base = match item.kind.as_str() {
        "Episode" | "Season" => item.series_id.clone().unwrap_or_else(|| item.id.clone()),
        _ => item.id.clone(),
    };
    let keep_rows = std::mem::replace(&mut *app.rows_base.lock().unwrap(), rows_base.clone()) == rows_base;
    // Série incomplète (Seerr) : bouton pour demander les saisons manquantes, ajouté une fois connu.
    if item.kind == "Series" && app.seerr_user.lock().unwrap().is_some() {
        if let Some(tmdb) = item.tmdb() {
            let (a, c, id) = (app.clone(), client.clone(), item.id.clone());
            app.rt.spawn(async move { add_missing_seasons_button(a, c, id, tmdb).await });
        }
    }
    let _ = ui.upgrade_in_event_loop(move |u| {
        let detail = DetailData {
            id: id_for_ui.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            misc: misc.into(),
            overview: overview.into(),
            expect_logo,
            logo_key: logo_key.into(),
            poster_key: poster_key.into(),
            icons_sig: icons_sig.into(),
            chips_sig: chips_sig.into(),
            children_sig: children_sig.into(),
            children_title: children_title.into(),
            children_landscape: landscape,
            has_next,
            next_id: next_id.into(),
            next_title: next_title.into(),
            next_subtitle: next_sub.into(),
            icon_count: buttons.iter().filter(|b| !b.2.is_empty()).count() as i32,
            buttons: ModelRc::new(VecModel::from(
                buttons
                    .into_iter()
                    .map(|(label, action, icon, active)| ButtonData {
                        label: label.into(),
                        action: action.into(),
                        icon: icon.into(),
                        active,
                    })
                    .collect::<Vec<_>>(),
            )),
            ..Default::default()
        };
        let cards: Vec<CardData> = child_cards
            .iter()
            .map(|c| CardData {
                id: c.id.clone().into(),
                title: c.title.clone().into(),
                subtitle: c.subtitle.clone().into(),
                progress: c.progress,
                rating: c.rating.clone().into(),
                                seerr: c.seerr,
                                status: c.status,
                                count: c.count,
                                played: c.played,
                ..Default::default()
            })
            .collect();

        set_detail_smooth(&u, detail);
        u.set_child_items(ModelRc::new(VecModel::from(cards)));
        if !keep_rows {
            u.set_detail_rows(ModelRc::default());
        }
        u.set_d_x(0);
        u.set_d_zone(0);
        u.set_d_button(0);
        u.set_d_child(0);
        u.set_overview_open(false);
        show_screen(&u, "detail");
    });

    // Poster (400x600) : saison pour un épisode, sinon élément ou série.
    {
        let client = client.clone();
        let ui = app.ui();
        let id = item_id.clone();
        let cands = item.poster_candidates();
        // poster-w dans app.slint : 200 px x k (k = 1,4 en mode TV).
        let shape = Shape::card(400, 600, if app.tv() { 280.0 } else { 200.0 });
        app.rt.spawn(async move {
            if let Some(buf) = fetch_decoded(&client, cands, "Primary", api::Size::Fill(400, 600), Some(shape)).await {
                let _ = ui.upgrade_in_event_loop(move |u| {
                    let mut d = u.get_detail();
                    if d.id.as_str() != id {
                        return;
                    }
                    d.poster = slint::Image::from_rgba8(buf);
                    d.has_poster = true;
                    u.set_detail(d);
                });
            }
        });
    }

    // Logo : si absent ou en échec, on retombe sur le titre en texte.
    if let Some(logo_ref) = logo {
        let client = client.clone();
        let ui = app.ui();
        let id = item_id.clone();
        app.rt.spawn(async move {
            let buf = fetch_decoded(&client, vec![logo_ref], "Logo", api::Size::MaxWidth(700), None).await;
            let _ = ui.upgrade_in_event_loop(move |u| {
                let mut d = u.get_detail();
                if d.id.as_str() != id {
                    return;
                }
                match buf {
                    Some(b) => {
                        d.logo = slint::Image::from_rgba8(b);
                        d.has_logo = true;
                    }
                    None => d.expect_logo = false,
                }
                u.set_detail(d);
            });
        });
    }

    // Vignette du bloc « À suivre ».
    if let Some((_, _, card)) = next_card {
        if let Some(img_id) = card.img_id {
            let client = client.clone();
            let ui = app.ui();
            let id = item_id.clone();
            let tag = card.img_tag;
            // next-w dans app.slint : 260 px x k.
            let shape = Shape::card(320, 180, if app.tv() { 364.0 } else { 260.0 });
            app.rt.spawn(async move {
                if let Some(buf) = fetch_decoded(&client, vec![(img_id, tag)], "Primary", api::Size::Fill(320, 180), Some(shape)).await {
                    let _ = ui.upgrade_in_event_loop(move |u| {
                        let mut d = u.get_detail();
                        if d.id.as_str() != id {
                            return;
                        }
                        d.next_image = slint::Image::from_rgba8(buf);
                        d.has_next_image = true;
                        u.set_detail(d);
                    });
                }
            });
        }
    }

    if !keep_rows {
        spawn_detail_rows(&app, &client, &item);
    }

    // Vignettes des enfants : 16:9 pour les épisodes, posters sinon.
    let size = if landscape { api::Size::Fill(400, 225) } else { api::Size::Fill(270, 405) };
    // child-card-w dans app.slint : (240 px en 16:9, 120 sinon) x k.
    let k = if app.tv() { 1.4 } else { 1.0 };
    let shape = if landscape { Shape::card_top(400, 225, 240.0 * k) } else { Shape::card_top(270, 405, 120.0 * k) };
    let apply: Apply = Arc::new(
        |u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            set_child_image(u, job.b, &job.item_id, buf)
        },
    );
    spawn_image_jobs(&app, &client, child_jobs, size, shape, apply);
}

// ---------------------------------------------------------------------------
// Lecture (M3)
// ---------------------------------------------------------------------------

/// Série -> prochain épisode ; saison -> premier épisode non vu ; film/épisode -> lui-même.
async fn resolve_playable(client: &api::Client, item: api::Item) -> anyhow::Result<api::Item> {
    match item.kind.as_str() {
        "Series" => {
            if let Some(n) = client.next_up_for(&item.id).await? {
                return client.item(&n.id).await;
            }
            let eps = client.episodes(&item.id, None).await?;
            let first = eps
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("Cette série n'a aucun épisode"))?;
            client.item(&first.id).await
        }
        "Season" => {
            let series_id = item
                .series_id
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Saison sans série associée"))?;
            let eps = client.episodes(&series_id, Some(&item.id)).await?;
            let pick = eps
                .iter()
                .find(|e| !e.user_data.as_ref().map(|u| u.played).unwrap_or(false))
                .or(eps.first())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Cette saison n'a aucun épisode"))?;
            client.item(&pick.id).await
        }
        "Movie" | "Episode" | "Video" | "MusicVideo" => Ok(item),
        other => Err(anyhow::anyhow!("Impossible de lancer la lecture d'un élément de type {other}")),
    }
}

async fn play_local(app: Arc<App>, e: downloads::Entry) {
    let subs = e.subs.iter().map(|(f, l)| (e.dir().join(f).to_string_lossy().into_owned(), l.clone())).collect();
    let url = e.media_path().to_string_lossy().into_owned();
    if e.position > 0.0 && !e.played {
        *app.play_start.lock().unwrap() = Some(e.position);
    }
    *app.last_play.lock().unwrap() = None;
    play_flow_with(app.clone(), None, Some(url), Some((e.title.clone(), e.subtitle.clone())), subs).await;
    // Position et « vu » gardés sur le disque, renvoyés au serveur dès qu'il répond.
    if let Some((pos, dur, ended)) = app.last_play.lock().unwrap().take() {
        downloads::record_play(&e.id, pos, dur, ended);
    }
    if let Some(client) = app.client() {
        if !app.offline.load(Ordering::SeqCst) {
            sync_offline_plays(&app, &client);
        }
    }
    let a = app.clone();
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        if u.get_screen().as_str() == "downloads" {
            refresh_downloads(&a);
        }
    });
}

/// Lectures faites hors ligne renvoyées au serveur (position, « vu »).
fn sync_offline_plays(app: &Arc<App>, client: &api::Client) {
    let (c, ui) = (client.clone(), app.ui());
    app.rt.spawn(async move {
        let n = downloads::sync(&c).await;
        if n > 0 {
            let msg = if n == 1 { "1 changement fait hors ligne envoyé au serveur.".to_string() } else { format!("{n} changements faits hors ligne envoyés au serveur.") };
            let _ = ui.upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
        }
    });
}

async fn play_flow(app: Arc<App>, id: Option<String>, test_url: Option<String>) {
    // Watch party : le média est lancé pour tout le groupe (le serveur renvoie la file à tous).
    let in_group = app.sp.lock().unwrap().group.is_some();
    if let (Some(id), Some(client), true) = (&id, app.client(), in_group) {
        let target = match client.item(id).await {
            Ok(it) => resolve_playable(&client, it).await,
            Err(e) => Err(e),
        };
        if let Ok(t) = target {
            let start = t.user_data.as_ref().map(|u| u.playback_position_ticks as f64 / 1e7).unwrap_or(0.0);
            if let Err(e) = syncplay::play(&client, &t.id, start).await {
                let msg = format!("Watch party : {e}");
                let _ = app.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
            }
        }
        return;
    }
    play_flow_with(app, id, test_url, None, Vec::new()).await;
}

/// Lecture lancée par le groupe (watch party) à une position imposée.
async fn play_flow_group(app: Arc<App>, id: String, start: f64) {
    *app.play_start.lock().unwrap() = Some(start);
    play_flow_with(app, Some(id), None, None, Vec::new()).await;
}

async fn play_flow_with(
    app: Arc<App>,
    id: Option<String>,
    test_url: Option<String>,
    local_title: Option<(String, String)>,
    local_subs: Vec<(String, String)>,
) {
    // Lecture locale (téléchargement) : pas de rapport au serveur, même connecté.
    let client = if test_url.is_some() { None } else { app.client() };
    if client.is_none() && test_url.is_none() {
        return;
    }
    if app.playing.swap(true, Ordering::SeqCst) {
        return; // déjà en lecture
    }
    let ui = app.ui();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    *app.player_tx.lock().unwrap() = Some(tx);
    let _ = ui.upgrade_in_event_loop(|u| {
        u.set_toast("".into());
        u.set_playing(true);
    });

    let result: anyhow::Result<player::Exit> = async {
        let (item, start_secs) = match (&client, id) {
            (Some(c), Some(id)) => {
                let target = resolve_playable(c, c.item(&id).await?).await?;
                let start = app.play_start.lock().unwrap().take().unwrap_or_else(|| {
                    target.user_data.as_ref().map(|u| u.playback_position_ticks as f64 / 10_000_000.0).unwrap_or(0.0)
                });
                (Some(target), start)
            }
            // Fichier local : reprise éventuelle (téléchargement déjà commencé).
            _ => (None, app.play_start.lock().unwrap().take().unwrap_or(0.0)),
        };
        let sync = app.sp.lock().unwrap().group.is_some() && test_url.is_none();
        let req = player::PlayRequest { item, start_secs, test_url, local_title, local_subs, sync };
        player::play(app.clone(), client.clone(), req, rx, ui.clone()).await
    }
    .await;

    *app.player_tx.lock().unwrap() = None;
    app.playing.store(false, Ordering::SeqCst);
    app.home_stale.store(true, Ordering::SeqCst);

    let go_home_after = matches!(result, Ok(player::Exit::Home));
    let msg = result.err().map(|e| format!("Lecture impossible : {e}"));
    let top = app.stack.lock().unwrap().last().cloned();
    let app2 = app.clone();
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_playing(false);
        u.set_p_up_mode("".into());
        if go_home_after {
            go_home(&app2);
        } else if let Some(id) = top {
            // Recharge la fiche : l'état « Reprendre » / « vu » a pu changer.
            start_detail(&app2, id);
        }
        if let Some(m) = msg {
            u.set_toast(m.into());
        }
    });
}

// ---------------------------------------------------------------------------
fn main() -> anyhow::Result<()> {
    let cli = parse_cli();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;

    // La vidéo passe par OpenGL : on demande le moteur de rendu femtovg (OpenGL / OpenGL ES),
    // sauf si SLINT_BACKEND impose autre chose.
    if std::env::var_os("SLINT_BACKEND").is_none() {
        if let Err(e) = slint::BackendSelector::new().renderer_name("femtovg".into()).select() {
            eprintln!("turtlefin : rendu OpenGL indisponible ({e}), lecture vidéo impossible");
        }
    }

    let saved = config::load();
    STILL_GIFS.store(saved.still_gifs, Ordering::Relaxed);
    NO_BACKDROP.store(saved.no_backdrop, Ordering::Relaxed);
    let ui = AppWindow::new()?;
    let video_ok = match video::install(&ui) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("turtlefin : affichage vidéo indisponible ({e:?})");
            false
        }
    };
    let prefs = config::ui_prefs();
    let tv = cli.tv.unwrap_or(prefs.tv);
    ui.set_tv_mode(tv);
    if tv {
        ui.window().set_fullscreen(true);
    }
    apply_ui_prefs(&ui, &prefs);

    let app = Arc::new(App {
        rt: rt.handle().clone(),
        ui: Mutex::new(ui.as_weak()),
        client: Mutex::new(None),
        stack: Mutex::new(Vec::new()),
        gen: AtomicU64::new(0),
        device_id: saved.device_id.clone(),
        tv_flag: AtomicBool::new(tv),
        tab: Mutex::new("home".to_string()),
        seerr_user: Mutex::new(None),
        library: Mutex::new(None),
        lib_loading: AtomicBool::new(false),
        lib_return: Mutex::new(None),
        found: Mutex::new(Vec::new()),
        play_arg: Mutex::new(cli.play.clone()),
        play_start: Mutex::new(None),
        can_download: AtomicBool::new(false),
        seerr_page: Mutex::new(None),
        search_gen: AtomicU64::new(0),
        user_cfg: Mutex::new(serde_json::Value::Null),
        sp: Arc::default(),
        sp_connected: AtomicBool::new(false),
        avatars: Mutex::new(Vec::new()),
        detail_streams: Mutex::new(Default::default()),
        missing_seasons: Mutex::new(Default::default()),
        dl_queue: Mutex::new(std::collections::VecDeque::new()),
        dl_current: Mutex::new(None),
        offline: AtomicBool::new(false),
        last_play: Mutex::new(None),
        bg_id: Mutex::new(String::new()),
        views: Mutex::new(Vec::new()),
        rows_base: Mutex::new(String::new()),
        bg_hash: AtomicU64::new(0),
        stack_base: Mutex::new(String::new()),
        bg_gen: AtomicU64::new(0),
        pending_server: Mutex::new(None),
        manual: Mutex::new((None, None, Vec::new())),
        playing: AtomicBool::new(false),
        home_stale: AtomicBool::new(false),
        player_tx: Mutex::new(None),
    });

    let server_hint = cli.server.clone().unwrap_or_else(|| saved.server.clone());
    ui.set_server(server_hint.clone().into());

    ui.on_login({
        let app = app.clone();
        move |server, user, pw| {
            if let Some(u) = app.ui().upgrade() {
                u.set_busy(true);
                u.set_error_text("".into());
            }
            let a = app.clone();
            let (s, us, p) = (server.to_string(), user.to_string(), pw.to_string());
            app.rt.spawn(async move { login_flow(a, s, us, p).await });
        }
    });

    ui.on_logout({
        let app = app.clone();
        move || end_session(&app, true)
    });

    ui.on_switch_account({
        let app = app.clone();
        move || end_session(&app, false)
    });

    ui.on_login_opened({
        let app = app.clone();
        move || login_opened(&app)
    });

    ui.on_login_pick({
        let app = app.clone();
        move |a| login_pick(&app, a.to_string())
    });

    // Mot de passe affiché en points.
    ui.on_mask(|t| "•".repeat(t.chars().count()).into());

    ui.on_select_tab({
        let app = app.clone();
        move |tab| {
            // Onglet déjà affiché (et à jour) : rien à recharger, la sélection redescend aux rangées.
            let same = *app.tab.lock().unwrap() == tab.as_str();
            if let Some(u) = app.ui().upgrade() {
                if same && u.get_screen().as_str() == "home" && !app.home_stale.load(Ordering::SeqCst) {
                    if u.get_sections().row_count() > 0 {
                        u.set_h_focus(false);
                    }
                    return;
                }
            }
            *app.tab.lock().unwrap() = tab.to_string();
            if let Some(u) = app.ui().upgrade() {
                u.set_tab(tab.clone());
                u.set_h_focus(false);
            }
            if let Some(client) = app.client() {
                let a = app.clone();
                app.rt.spawn(async move { load_tab(a, client).await });
            }
        }
    });

    // Horloge de l'en-tête (« 16:52 »), mise à jour toutes les 10 s.
    let clock = slint::Timer::default();
    {
        let tick = {
            let ui = ui.as_weak();
            move || {
                if let Some(u) = ui.upgrade() {
                    u.set_clock(chrono::Local::now().format("%H:%M").to_string().into());
                }
            }
        };
        tick();
        clock.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(10), tick);
    }

    ui.on_party_action({
        let app = app.clone();
        move |a| party_action(&app, a.to_string())
    });

    ui.on_search_open({
        let app = app.clone();
        move || open_search(&app)
    });

    ui.on_search_changed({
        let app = app.clone();
        move |t| search_changed(&app, t.to_string())
    });

    // Effacement du dernier caractère (clavier à l'écran : "\u{8}" + texte).
    ui.on_text_backspace(|t| {
        let mut s = t.trim_start_matches('\u{8}').to_string();
        s.pop();
        s.into()
    });

    ui.on_random_pick({
        let app = app.clone();
        move || random_pick(&app)
    });

    ui.on_track_pick({
        let app = app.clone();
        move |kind, lang, label| pick_track(&app, &kind, &lang, &label)
    });

    ui.on_seerr_action({
        let app = app.clone();
        move || seerr_action(&app)
    });

    ui.on_dl_play({
        let app = app.clone();
        move |id| play_download(&app, &id)
    });

    ui.on_backdrop_for({
        let app = app.clone();
        move |id| set_backdrop(&app, id.to_string())
    });


    ui.on_dl_delete({
        let app = app.clone();
        move |id| {
            downloads::remove(&id);
            refresh_downloads(&app);
        }
    });

    ui.on_servers_search({
        let app = app.clone();
        move || search_servers(&app)
    });

    ui.on_server_pick({
        let app = app.clone();
        move |i| {
            let f = app.found.lock().unwrap().get(i as usize).cloned();
            if let Some(f) = f {
                choose_server(&app, f.local.unwrap_or_default(), f.remote.unwrap_or_default());
            }
        }
    });

    ui.on_manual_submit({
        let app = app.clone();
        move |local, remote| manual_submit(&app, local.to_string(), remote.to_string())
    });

    ui.on_manual_choice({
        let app = app.clone();
        move |url| manual_choice(&app, url.to_string())
    });

    ui.on_settings_activate({
        let app = app.clone();
        move |k| settings_activate(&app, &k)
    });

    ui.on_settings_category({
        let app = app.clone();
        move || refresh_settings(&app)
    });

    ui.on_avatar_pick({
        let app = app.clone();
        move |id| set_avatar(&app, id.to_string())
    });

    // Rangées horizontales (voir `global Rows`) : défilement propre à chaque rangée, qui n'avance
    // que lorsque la sélection atteint l'avant-dernière carte visible ; changement de rangée vers la
    // carte la plus proche à l'écran. Mémoire : clé -> décalage (px logiques, <= 0).
    let row_off: Arc<Mutex<std::collections::HashMap<i32, f32>>> = Arc::default();
    ui.on_row_seen(|_, _| {});
    ui.global::<Rows>().on_scroll({
        let m = row_off.clone();
        move |key, idx, n, w, gap, pad, rw| {
            if n <= 0 || rw <= 0.0 {
                return 0.0;
            }
            let idx = idx.clamp(0, n - 1);
            let left = |i: i32| pad + i as f32 * (w + gap);
            let strip = n as f32 * (w + gap) + 2.0 * pad;
            let min_off = (rw - strip).min(0.0);
            let mut map = m.lock().unwrap();
            let mut off = map.get(&key).copied().unwrap_or(0.0);
            // La voisine de droite (ou de gauche) reste visible : on voit ce qui suit.
            let ahead = (idx + 1).min(n - 1);
            if left(ahead) + w + off > rw - pad {
                off = rw - pad - w - left(ahead);
            }
            let behind = (idx - 1).max(0);
            if left(behind) + off < pad {
                off = pad - left(behind);
            }
            // Cartes très larges : la sélectionnée en entier, au moins.
            if left(idx) + w + off > rw {
                off = rw - pad - w - left(idx);
            }
            let off = off.clamp(min_off, 0.0);
            map.insert(key, off);
            off
        }
    });
    ui.global::<Rows>().on_pick({
        let m = row_off.clone();
        move |from, from_idx, from_w, to, to_n, to_w, gap, pad, _rw| {
            if to_n <= 0 {
                return 0;
            }
            let map = m.lock().unwrap();
            let x = pad + from_idx as f32 * (from_w + gap) + from_w / 2.0 + map.get(&from).copied().unwrap_or(0.0);
            let off = map.get(&to).copied().unwrap_or(0.0);
            let left = |i: i32| pad + i as f32 * (to_w + gap) + off;
            let center = |i: i32| left(i) + to_w / 2.0;
            // Cartes où l'on peut arriver sans faire défiler la rangée (voisines visibles aussi) :
            // la rangée ne bouge pas au changement de rangée.
            let still = |i: i32| {
                let ahead = (i + 1).min(to_n - 1);
                let behind = (i - 1).max(0);
                left(ahead) + to_w <= _rw - pad + 0.5 && left(behind) >= pad - 0.5
            };
            let dist = |i: &i32| (center(*i) - x).abs();
            (0..to_n)
                .filter(|i| still(*i))
                .min_by(|a, b| dist(a).total_cmp(&dist(b)))
                .or_else(|| (0..to_n).min_by(|a, b| dist(a).total_cmp(&dist(b))))
                .unwrap_or(0)
        }
    });
    // Nouvelles rangées : défilements oubliés (accueil rechargé : clés < 1000 ; autre fiche : 1000+).
    {
        let m = row_off.clone();
        let weak = ui.as_weak();
        let last: Arc<Mutex<(usize, String, usize)>> = Arc::default();
        let t = slint::Timer::default();
        t.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(300), move || {
            let Some(u) = weak.upgrade() else { return };
            let now = (u.get_sections().row_count(), u.get_detail().id.to_string(), u.get_search_sections().row_count());
            let mut l = last.lock().unwrap();
            if *l != now {
                let mut map = m.lock().unwrap();
                if l.0 != now.0 {
                    map.retain(|k, _| *k >= 1000);
                }
                if l.1 != now.1 {
                    map.retain(|k, _| !(1000..2000).contains(k));
                }
                if l.2 != now.2 {
                    map.retain(|k, _| *k < 2000);
                }
                *l = now;
            }
        });
        std::mem::forget(t);
    }

    ui.on_go_home({
        let app = app.clone();
        move || go_home(&app)
    });

    ui.on_lib_more({
        let app = app.clone();
        move || {
            if let Some(client) = app.client() {
                load_library_page(&app, &client, None);
            }
        }
    });

    ui.on_menu_action({
        let app = app.clone();
        move |action| {
            let a = action.as_str();
            if a == "home" {
                // Bouton maison et « Accueil » du menu : toujours l'onglet Accueil (pas Favoris
                // ni Demandes), rechargé s'il n'était pas affiché.
                if std::mem::replace(&mut *app.tab.lock().unwrap(), "home".to_string()) != "home" {
                    app.home_stale.store(true, Ordering::SeqCst);
                }
                if let Some(u) = app.ui().upgrade() {
                    u.set_tab("home".into());
                }
                go_home(&app);
            } else if a == "requests" {
                *app.tab.lock().unwrap() = "requests".to_string();
                if let Some(u) = app.ui().upgrade() {
                    u.set_tab("requests".into());
                }
                app.home_stale.store(true, Ordering::SeqCst);
                go_home(&app);
            } else if let Some(id) = a.strip_prefix("lib:") {
                // Une bibliothèque ouverte depuis le menu repart de l'accueil.
                app.stack.lock().unwrap().clear();
                if let Some(u) = app.ui().upgrade() {
                    u.set_here_lib(a.into());
                }
                push_detail(&app, id.to_string());
            } else if a == "settings" {
                open_settings(&app);
            } else if a == "downloads" {
                open_downloads(&app);
            } else if a == "party" {
                open_party(&app);
            } else if a == "server" {
                open_servers(&app);
            } else if a == "switch" {
                end_session(&app, false);
            } else if a == "logout" {
                if let Some(u) = app.ui().upgrade() {
                    u.invoke_logout();
                }
            } else if a == "quit" {
                let _ = slint::quit_event_loop();
            }
        }
    });

    ui.on_open_item({
        let app = app.clone();
        move |id| {
            // Depuis l'accueil : une bibliothèque (« Mes médias ») devient l'endroit où l'on est ;
            // autre chose reste une branche de l'accueil.
            if app.stack.lock().unwrap().is_empty() {
                let is_view = app.views.lock().unwrap().iter().any(|v| v.id == id.as_str());
                if let Some(u) = app.ui().upgrade() {
                    u.set_here_lib(if is_view { format!("lib:{id}").into() } else { "".into() });
                }
            }
            push_detail(&app, id.to_string())
        }
    });

    ui.on_is_seerr(|id| id.starts_with("seerr:"));

    ui.on_settings_choose({
        let app = app.clone();
        move |key, value| choose_setting(&app, &key, &value)
    });

    ui.on_menu_find({
        let weak = ui.as_weak();
        move |action| {
            let Some(u) = weak.upgrade() else { return -1 };
            let m = u.get_menu_entries();
            (0..m.row_count()).find(|&i| m.row_data(i).is_some_and(|e| !e.header && e.action == action)).map(|i| i as i32).unwrap_or(-1)
        }
    });

    ui.on_run_action({
        let app = app.clone();
        move |action| {
            let a = action.as_str();
            if a == "back" {
                go_back(&app);
            } else if a == "play" && app.stack.lock().unwrap().last().is_some_and(|t| t.starts_with("dl:")) {
                let top = app.stack.lock().unwrap().last().cloned().unwrap_or_default();
                play_local_key(&app, &top);
            } else if a == "dl-delete" {
                let top = app.stack.lock().unwrap().last().cloned().unwrap_or_default();
                delete_local_key(&app, &top);
            } else if a == "play" {
                let top = app.stack.lock().unwrap().last().cloned();
                if let Some(id) = top {
                    if !video_ok {
                        if let Some(u) = app.ui().upgrade() {
                            u.set_toast("Lecture impossible : le rendu OpenGL n'est pas disponible (SLINT_BACKEND ?).".into());
                        }
                        return;
                    }
                    let a2 = app.clone();
                    app.rt.spawn(async move { play_flow(a2, Some(id), None).await });
                }
            } else if a == "pick-audio" || a == "pick-sub" {
                open_track_picker(&app, if a == "pick-audio" { "audio" } else { "sub" });
            } else if a == "fav" || a == "played" {
                toggle_flag(&app, a);
            } else if a == "download" {
                start_download(&app);
            } else if a == "seerr-missing" {
                request_missing_seasons(&app);
            } else if let Some(id) = a.strip_prefix("open:") {
                push_detail(&app, id.to_string());
            }
        }
    });

    ui.on_player_cmd({
        let app = app.clone();
        move |cmd| {
            if let Some(tx) = app.player_tx.lock().unwrap().as_ref() {
                let _ = tx.send(cmd.to_string());
            }
        }
    });

    // Démarrage automatique : arguments > session sauvegardée > écran de connexion.
    if let (Some(user), Some(pw)) = (cli.user.clone(), cli.pass.clone()) {
        if server_hint.is_empty() {
            ui.set_error_text("Indique le serveur avec --server=URL (ou saisis-le ci-dessous).".into());
        } else {
            ui.set_busy(true);
            let a = app.clone();
            let s = server_hint.clone();
            rt.spawn(async move { login_flow(a, s, user, pw).await });
        }
    } else if !saved.token.is_empty() && !saved.server.is_empty() {
        ui.set_screen("loading".into());
        let a = app.clone();
        let mut saved = saved.clone();
        rt.spawn(async move {
            // Ancienne session (une seule adresse) : on la range côté local ou distant.
            if saved.server_local.is_empty() && saved.server_remote.is_empty() {
                if discovery::is_local_url(&saved.server) {
                    saved.server_local = saved.server.clone();
                } else {
                    saved.server_remote = saved.server.clone();
                }
            }
            let best = discovery::pick(&saved.server_local, &saved.server_remote, saved.prefer_remote).await;
            if !best.is_empty() {
                saved.server = best;
            }
            config::save(&saved);
            // Serveur injoignable : téléchargements seulement (s'il y en a).
            if !discovery::reachable(&saved.server).await && !downloads::list().is_empty() {
                go_offline(&a);
                return;
            }
            match api::Client::from_saved(&saved) {
                Ok(client) => load_home(a, client).await,
                Err(e) => show_login_error(&a.ui(), format!("{e}")),
            }
        });
    }

    if ui.get_screen().as_str() == "login" {
        login_opened(&app);
    }

    // Essai du lecteur sans serveur : turtlefin --test-video=chemin/vers/video.mkv
    if let Some(url) = cli.test_video.clone() {
        let a = app.clone();
        rt.spawn(async move { play_flow(a, None, Some(url)).await });
    }

    ui.run()?;
    // Fermeture : on quitte la watch party (sinon le serveur garde une session fantôme dans le groupe).
    leave_party_blocking(&app, &rt);
    Ok(())
}

/// Quitte la watch party en cours, en attendant la réponse du serveur (2 s au plus).
fn leave_party_blocking(app: &Arc<App>, rt: &tokio::runtime::Runtime) {
    if app.sp.lock().unwrap().group.is_none() {
        return;
    }
    if let Some(c) = app.client() {
        rt.block_on(async {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), syncplay::leave(&c)).await;
        });
    }
    *app.sp.lock().unwrap() = syncplay::State::default();
}

// ---------------------------------------------------------------------------
// Téléchargements (lecture hors ligne)
// ---------------------------------------------------------------------------

/// Bouton Télécharger de la fiche : film / épisode, ou tous les épisodes d'une saison / série.
fn start_download(app: &Arc<App>) {
    let Some(client) = app.client() else { return };
    let Some(id) = app.ui().upgrade().map(|u| u.get_detail().id.to_string()) else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let Ok(item) = client.item(&id).await else { return };
        let list: Vec<api::Item> = match item.kind.as_str() {
            "Series" => client.episodes(&item.id, None).await.unwrap_or_default(),
            "Season" => match &item.series_id {
                Some(sid) => client.episodes(sid, Some(&item.id)).await.unwrap_or_default(),
                None => Vec::new(),
            },
            _ => vec![item],
        };
        let mut added = 0;
        {
            let mut q = app2.dl_queue.lock().unwrap();
            let current = app2.dl_current.lock().unwrap().as_ref().map(|c| c.0.clone());
            for it in list {
                let already = downloads::exists(&it.id) || q.iter().any(|(i, _)| *i == it.id) || current.as_deref() == Some(it.id.as_str());
                if !already {
                    let (t, s) = it.titles();
                    q.push_back((it.id.clone(), if s.is_empty() { t } else { format!("{t} · {s}") }));
                    added += 1;
                }
            }
        }
        let msg = if added == 0 { "Déjà téléchargé.".to_string() } else { format!("Ajouté aux téléchargements ({added}).") };
        let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
        run_downloads(&app2, &client);
    });
}

/// Traite la file, un élément à la fois (rien si un transfert est déjà en cours).
fn run_downloads(app: &Arc<App>, client: &api::Client) {
    if app.dl_current.lock().unwrap().is_some() {
        return;
    }
    let Some((id, title)) = app.dl_queue.lock().unwrap().pop_front() else {
        push_dl_status(app);
        return;
    };
    *app.dl_current.lock().unwrap() = Some((id.clone(), title.clone(), 0.0));
    push_dl_status(app);
    let (app2, client2) = (app.clone(), client.clone());
    app.rt.spawn(async move {
        let app3 = app2.clone();
        let r = downloads::download(&client2, &id, move |p| {
            if let Some(c) = app3.dl_current.lock().unwrap().as_mut() {
                c.2 = p;
            }
            push_dl_status(&app3);
        })
        .await;
        *app2.dl_current.lock().unwrap() = None;
        if let Err(e) = r {
            let msg = format!("Téléchargement impossible ({title}) : {e}");
            let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
        }
        refresh_downloads(&app2);
        run_downloads(&app2, &client2);
    });
}

fn push_dl_status(app: &Arc<App>) {
    let waiting = app.dl_queue.lock().unwrap().len();
    let status = match app.dl_current.lock().unwrap().clone() {
        Some((_, title, p)) => {
            let mut s = format!("Téléchargement : {title} — {:.0} %", p * 100.0);
            if waiting > 0 {
                s.push_str(&format!(" · {waiting} en attente"));
            }
            s
        }
        None => String::new(),
    };
    let _ = app.ui().upgrade_in_event_loop(move |u| u.set_dl_status(status.into()));
}

fn fmt_size(b: u64) -> String {
    if b >= 1 << 30 {
        format!("{:.1} Go", b as f64 / (1u64 << 30) as f64)
    } else {
        format!("{} Mo", b >> 20)
    }
}

fn fmt_mins(secs: f64) -> String {
    let m = (secs / 60.0).round() as i64;
    if m >= 60 { format!("{} h {:02}", m / 60, m % 60) } else { format!("{m} min") }
}

/// Ligne de détails d'un élément : année, durée, note, état de lecture, taille.
fn dl_details(e: &downloads::Entry) -> String {
    let mut v: Vec<String> = Vec::new();
    if let Some(y) = e.year {
        v.push(y.to_string());
    }
    if e.runtime_secs > 0.0 {
        v.push(fmt_mins(e.runtime_secs));
    }
    if let Some(r) = e.rating {
        v.push(format!("★ {r:.1}"));
    }
    if e.played {
        v.push("Vu".into());
    } else if e.position > 0.0 {
        v.push(format!("Reprise à {}", fmt_mins(e.position)));
    }
    v.push(fmt_size(e.size));
    if e.dirty {
        v.push("à synchroniser".into());
    }
    v.join("  ·  ")
}

fn season_label(e: &downloads::Entry) -> String {
    if !e.season_name.is_empty() {
        e.season_name.clone()
    } else if let Some(n) = e.season_index {
        format!("Saison {n}")
    } else {
        "Épisodes".into()
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Épisodes et films téléchargés, avec « vu » et favori à jour (décisions prises sur l'appareil comprises).
fn local_entries() -> Vec<downloads::Entry> {
    let flags = config::all_flags();
    downloads::list()
        .into_iter()
        .map(|mut e| {
            if let Some(f) = flags.get(&e.id) {
                if let Some(v) = f.played {
                    e.played = v;
                }
                if let Some(v) = f.favorite {
                    e.favorite = v;
                }
            }
            e
        })
        .collect()
}

/// « Vu » / favori d'une série ou d'une saison : décision prise sur l'appareil, sinon déduit.
fn local_flag(id: &str, favorite: bool, default: bool) -> bool {
    let f = config::flags(id);
    (if favorite { f.favorite } else { f.played }).unwrap_or(default)
}

/// Élément Jellyfin derrière une clé de page locale (« dl:series:<id> », « dl:season:<id> », « dl:item:<id> »).
fn real_id(key: &str) -> &str {
    key.strip_prefix("dl:series:").or_else(|| key.strip_prefix("dl:season:")).or_else(|| key.strip_prefix("dl:item:")).unwrap_or(key)
}

fn sort_episodes(eps: &mut [&downloads::Entry]) {
    eps.sort_by_key(|e| (e.season_index.unwrap_or(u32::MAX), e.episode_index.unwrap_or(u32::MAX)));
}

/// Carte locale : (clé, titre, sous-titre, image sur le disque, épisodes restants, avancement, vu).
type LocalCard = (String, String, String, std::path::PathBuf, i32, f32, bool);

fn entry_progress(e: &downloads::Entry) -> f32 {
    if e.played || e.runtime_secs <= 0.0 { 0.0 } else { (e.position / e.runtime_secs) as f32 }
}

/// Écran Téléchargements, présenté comme l'accueil : Reprendre, Séries, Films (données du disque).
fn refresh_downloads(app: &Arc<App>) {
    let entries = local_entries();
    let mut resume: Vec<LocalCard> = Vec::new();
    let mut series: Vec<LocalCard> = Vec::new();
    let mut movies: Vec<LocalCard> = Vec::new();
    for e in &entries {
        if e.position > 0.0 && !e.played {
            let (t, sub) = if e.kind == "Episode" { (e.series_name.clone(), e.subtitle.clone()) } else { (e.title.clone(), e.year.map(|y| y.to_string()).unwrap_or_default()) };
            resume.push((format!("dl:item:{}", e.id), t, sub, e.thumb_path(), 0, entry_progress(e), false));
        }
        if e.kind == "Episode" && !e.series_id.is_empty() {
            let key = format!("dl:series:{}", e.series_id);
            if series.iter().any(|c| c.0 == key) {
                continue;
            }
            let eps: Vec<&downloads::Entry> = entries.iter().filter(|x| x.series_id == e.series_id).collect();
            let left = eps.iter().filter(|x| !x.played).count();
            let all_played = local_flag(&e.series_id, false, left == 0);
            series.push((key, e.series_name.clone(), plural(eps.len(), "épisode", "épisodes"), e.series_poster(), if all_played { 0 } else { left as i32 }, 0.0, all_played));
        } else if e.kind != "Episode" {
            movies.push((format!("dl:item:{}", e.id), e.title.clone(), e.year.map(|y| y.to_string()).unwrap_or_default(), e.poster_path(), 0, entry_progress(e), e.played));
        }
    }
    let mut sections: Vec<(String, bool, Vec<LocalCard>)> = Vec::new();
    for (t, l, c) in [("Reprendre", true, resume), ("Séries", false, series), ("Films", false, movies)] {
        if !c.is_empty() {
            sections.push((t.to_string(), l, c));
        }
    }
    let k = if app.tv() { 1.4_f32 } else { 1.0 };
    let card_w = if app.tv() { 230.0_f32 } else { 170.0 };
    let row_h = move |landscape: bool| 132.0 * k + if landscape { card_w * 1.5 * 0.5625 } else { card_w * 1.5 };
    let rows: Vec<(String, bool, Vec<LocalCard>)> = sections.clone();
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        let mut y = 0.0_f32;
        let model: Vec<Section> = rows
            .iter()
            .map(|(title, landscape, cards)| {
                let h = row_h(*landscape);
                let s = Section {
                    title: title.clone().into(),
                    landscape: *landscape,
                    y_px: y,
                    h_px: h,
                    items: ModelRc::new(VecModel::from(
                        cards
                            .iter()
                            .map(|c| CardData {
                                id: c.0.clone().into(),
                                title: c.1.clone().into(),
                                subtitle: c.2.clone().into(),
                                count: c.4,
                                progress: c.5,
                                played: c.6,
                                ..Default::default()
                            })
                            .collect::<Vec<_>>(),
                    )),
                };
                y += h;
                s
            })
            .collect();
        let n = model.len() as i32;
        u.set_dl_sections(ModelRc::new(VecModel::from(model)));
        u.set_dl_level(0);
        if u.get_dl_sec() >= n {
            u.set_dl_sec((n - 1).max(0));
            u.set_dl_item(0);
        }
    });
    // Images lues sur le disque, mises à la forme des cartes.
    for (si, (_, landscape, cards)) in sections.into_iter().enumerate() {
        let shape = if landscape { Shape::card_top(400, 225, card_w * 1.5) } else { Shape::card_top(270, 405, card_w) };
        for (ci, c) in cards.into_iter().enumerate() {
            let ui = app.ui();
            app.rt.spawn(async move {
                let path = c.3.clone();
                let Some(buf) = tokio::task::spawn_blocking(move || decode(&std::fs::read(path).ok()?, Some(shape))).await.ok().flatten() else { return };
                let _ = ui.upgrade_in_event_loop(move |u| {
                    let secs = u.get_dl_sections();
                    let Some(sec) = secs.row_data(si) else { return };
                    if let Some(mut card) = sec.items.row_data(ci) {
                        if card.id.as_str() == c.0 {
                            card.image = slint::Image::from_rgba8(buf);
                            card.has_image = true;
                            sec.items.set_row_data(ci, card);
                        }
                    }
                });
            });
        }
    }
    // Anciens téléchargements (métadonnées incomplètes) : complétés si le serveur répond.
    if let (Some(client), false) = (app.client(), app.offline.load(Ordering::SeqCst)) {
        let old: Vec<String> = entries.iter().filter(|e| e.meta_v < 2).map(|e| e.id.clone()).collect();
        if !old.is_empty() {
            let a = app.clone();
            app.rt.spawn(async move {
                let mut changed = false;
                for id in old {
                    changed |= downloads::enrich(&client, &id).await;
                }
                if changed {
                    let a2 = a.clone();
                    let _ = a.ui().upgrade_in_event_loop(move |_| refresh_downloads(&a2));
                }
            });
        }
    }
}

/// Fiche d'un téléchargement (série, saison, épisode ou film), construite avec les seules données
/// du disque : elle marche sans serveur.
fn show_local_detail(app: &Arc<App>, key: &str) {
    let entries = local_entries();
    let sid = real_id(key).to_string();
    // (titre, sous-titre, ligne d'infos, résumé, affiche, logo, fond, boutons, titre des enfants, enfants 16:9, enfants)
    let mut buttons: Vec<(String, String, String, bool)> = Vec::new();
    let children: Vec<LocalCard>;
    let (title, subtitle, misc, overview, poster, logo, backdrop, children_title, landscape);
    if let Some(series_id) = key.strip_prefix("dl:series:") {
        let mut eps: Vec<&downloads::Entry> = entries.iter().filter(|e| e.series_id == series_id).collect();
        if eps.is_empty() {
            return;
        }
        sort_episodes(&mut eps);
        let first = eps[0];
        let mut seasons: Vec<&downloads::Entry> = Vec::new();
        for e in &eps {
            if !seasons.iter().any(|s| s.season_id == e.season_id) {
                seasons.push(e);
            }
        }
        let left = eps.iter().filter(|e| !e.played).count();
        title = first.series_name.clone();
        subtitle = String::new();
        let mut m = Vec::new();
        if let Some(y) = first.year {
            m.push(y.to_string());
        }
        if !first.official_rating.is_empty() {
            m.push(first.official_rating.clone());
        }
        m.push(plural(seasons.len(), "saison", "saisons"));
        m.push(plural(eps.len(), "épisode téléchargé", "épisodes téléchargés"));
        m.push(fmt_size(eps.iter().map(|e| e.size).sum()));
        misc = m.join("  ·  ");
        overview = first.series_overview.clone();
        poster = first.series_poster();
        logo = first.logo_path();
        backdrop = first.backdrop_path();
        children_title = "Saisons".to_string();
        landscape = false;
        buttons.push((String::new(), "play".into(), "play".into(), false));
        buttons.push((String::new(), "fav".into(), "heart".into(), local_flag(series_id, true, false)));
        buttons.push((String::new(), "played".into(), "check".into(), local_flag(series_id, false, left == 0)));
        buttons.push((String::new(), "dl-delete".into(), "trash".into(), false));
        children = seasons
            .iter()
            .map(|s| {
                let n = eps.iter().filter(|e| e.season_id == s.season_id).count();
                let l = eps.iter().filter(|e| e.season_id == s.season_id && !e.played).count();
                let played = local_flag(&s.season_id, false, l == 0);
                (format!("dl:season:{}", s.season_id), season_label(s), plural(n, "épisode", "épisodes"), s.season_poster(), if played { 0 } else { l as i32 }, 0.0, played)
            })
            .collect();
    } else if let Some(season_id) = key.strip_prefix("dl:season:") {
        let mut eps: Vec<&downloads::Entry> = entries.iter().filter(|e| e.season_id == season_id).collect();
        if eps.is_empty() {
            return;
        }
        sort_episodes(&mut eps);
        let first = eps[0];
        let left = eps.iter().filter(|e| !e.played).count();
        title = first.series_name.clone();
        subtitle = season_label(first);
        misc = format!("{}  ·  {}", plural(eps.len(), "épisode téléchargé", "épisodes téléchargés"), fmt_size(eps.iter().map(|e| e.size).sum()));
        overview = if first.season_overview.is_empty() { first.series_overview.clone() } else { first.season_overview.clone() };
        poster = first.season_poster();
        logo = first.logo_path();
        backdrop = first.backdrop_path();
        children_title = "Épisodes".to_string();
        landscape = true;
        buttons.push((String::new(), "play".into(), "play".into(), false));
        buttons.push((String::new(), "fav".into(), "heart".into(), local_flag(season_id, true, false)));
        buttons.push((String::new(), "played".into(), "check".into(), local_flag(season_id, false, left == 0)));
        buttons.push((String::new(), "dl-delete".into(), "trash".into(), false));
        buttons.push(("Voir la série".into(), format!("open:dl:series:{}", first.series_id), "label".into(), false));
        children = eps
            .iter()
            .map(|e| {
                let name = e.subtitle.split(" · ").last().unwrap_or(&e.subtitle).to_string();
                let t = match e.episode_index {
                    Some(n) => format!("{n}. {name}"),
                    None => name,
                };
                (format!("dl:item:{}", e.id), t, dl_details(e), e.thumb_path(), 0, entry_progress(e), e.played)
            })
            .collect();
    } else {
        let Some(e) = entries.iter().find(|e| e.id == sid) else { return };
        title = e.title.clone();
        subtitle = if e.kind == "Episode" { e.subtitle.clone() } else { String::new() };
        misc = dl_details(e);
        overview = e.overview.clone();
        poster = if e.kind == "Episode" { e.season_poster() } else { e.poster_path() };
        logo = e.logo_path();
        backdrop = e.backdrop_path();
        children_title = String::new();
        landscape = false;
        buttons.push((String::new(), "play".into(), "play".into(), false));
        buttons.push((String::new(), "fav".into(), "heart".into(), e.favorite));
        buttons.push((String::new(), "played".into(), "check".into(), e.played));
        buttons.push((String::new(), "dl-delete".into(), "trash".into(), false));
        if e.kind == "Episode" {
            buttons.push(("Voir la série".into(), format!("open:dl:series:{}", e.series_id), String::new(), false));
            buttons.push(("Voir la saison".into(), format!("open:dl:season:{}", e.season_id), String::new(), false));
        }
        children = Vec::new();
    }
    let key_s = key.to_string();
    let has_logo_file = logo.is_some();
    let rows: Vec<LocalCard> = children.clone();
    // Clés d'images : empreinte du fichier (chaque épisode garde sa copie de l'affiche de la série).
    let file_key = |p: &std::path::Path| -> String {
        use std::hash::{Hash, Hasher};
        let Ok(b) = std::fs::read(p) else { return String::new() };
        let mut h = std::collections::hash_map::DefaultHasher::new();
        b.hash(&mut h);
        format!("{:x}", h.finish())
    };
    let poster_key = file_key(&poster);
    let logo_key = logo.as_deref().map(file_key).unwrap_or_default();
    let icons_sig = buttons_sig(&buttons, true);
    let chips_sig = buttons_sig(&buttons, false);
    let children_sig = children.iter().map(|c| c.0.as_str()).collect::<Vec<_>>().join(",");
    *app.rows_base.lock().unwrap() = String::new();
    if let Some(u) = app.ui().upgrade() {
        // Même fiche réaffichée (après « vu » / favori) : pas d'animation, la sélection reste.
        let same = u.get_screen().as_str() == "detail" && u.get_detail().id.as_str() == key_s;
        let detail = DetailData {
            id: key_s.clone().into(),
            title: title.into(),
            subtitle: subtitle.into(),
            misc: misc.into(),
            overview: overview.into(),
            expect_logo: has_logo_file,
            logo_key: logo_key.into(),
            poster_key: poster_key.into(),
            icons_sig: icons_sig.into(),
            chips_sig: chips_sig.into(),
            children_sig: children_sig.into(),
            children_title: children_title.into(),
            children_landscape: landscape,
            icon_count: buttons.iter().filter(|b| !b.2.is_empty()).count() as i32,
            buttons: ModelRc::new(VecModel::from(
                buttons
                    .into_iter()
                    .map(|(label, action, icon, active)| ButtonData { label: label.into(), action: action.into(), icon: icon.into(), active })
                    .collect::<Vec<_>>(),
            )),
            ..Default::default()
        };
        set_detail_smooth(&u, detail);
        u.set_child_items(ModelRc::new(VecModel::from(
            rows.iter()
                .map(|c| CardData { id: c.0.clone().into(), title: c.1.clone().into(), subtitle: c.2.clone().into(), count: c.4, progress: c.5, played: c.6, ..Default::default() })
                .collect::<Vec<_>>(),
        )));
        u.set_detail_rows(ModelRc::default());
        if !same {
            u.set_d_x(0);
            u.set_d_zone(0);
            u.set_d_button(0);
            u.set_d_child(0);
            u.set_overview_open(false);
            show_screen(&u, "detail");
        }
    }
    // Images du disque : affiche, logo, enfants ; fond d'écran.
    let read = |p: std::path::PathBuf, shape: Option<Shape>| async move {
        tokio::task::spawn_blocking(move || decode(&std::fs::read(p).ok()?, shape)).await.ok().flatten()
    };
    let (ui, k2) = (app.ui(), key_s.clone());
    let poster_shape = Shape::card(400, 600, if app.tv() { 280.0 } else { 200.0 });
    app.rt.spawn(async move {
        let p = read(poster, Some(poster_shape)).await;
        let l = match logo {
            Some(path) => read(path, None).await,
            None => None,
        };
        let _ = ui.upgrade_in_event_loop(move |u| {
            let mut d = u.get_detail();
            if d.id.as_str() != k2 {
                return;
            }
            if let Some(b) = p {
                d.poster = slint::Image::from_rgba8(b);
                d.has_poster = true;
            }
            match l {
                Some(b) => {
                    d.logo = slint::Image::from_rgba8(b);
                    d.has_logo = true;
                }
                None => d.expect_logo = false,
            }
            u.set_detail(d);
        });
    });
    let k = if app.tv() { 1.4 } else { 1.0 };
    let shape = if landscape { Shape::card_top(400, 225, 240.0 * k) } else { Shape::card_top(270, 405, 120.0 * k) };
    for (i, c) in children.into_iter().enumerate() {
        let ui = app.ui();
        app.rt.spawn(async move {
            let path = c.3.clone();
            let Some(buf) = tokio::task::spawn_blocking(move || decode(&std::fs::read(path).ok()?, Some(shape))).await.ok().flatten() else { return };
            let _ = ui.upgrade_in_event_loop(move |u| set_child_image(&u, i, &c.0, buf));
        });
    }
    if let Some(bd) = backdrop {
        set_local_backdrop(app, key_s, bd);
    }
}

/// Fond d'écran lu sur le disque (téléchargements).
fn set_local_backdrop(app: &Arc<App>, key: String, path: std::path::PathBuf) {
    if NO_BACKDROP.load(Ordering::Relaxed) {
        return;
    }
    {
        let mut cur = app.bg_id.lock().unwrap();
        if *cur == key {
            return;
        }
        *cur = key;
    }
    let my = app.bg_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let a = app.clone();
    app.rt.spawn(async move {
        let Ok(bytes) = std::fs::read(&path) else { return };
        if !same_backdrop(&a, &bytes) {
            return;
        }
        let Some(buf) = tokio::task::spawn_blocking(move || decode_backdrop(&bytes)).await.ok().flatten() else { return };
        if a.bg_gen.load(Ordering::SeqCst) != my {
            return;
        }
        let _ = a.ui().upgrade_in_event_loop(move |u| {
            let img = slint::Image::from_rgba8(buf);
            if u.get_bg_flip() {
                u.set_bg_a(img);
                u.set_bg_flip(false);
            } else {
                u.set_bg_b(img);
                u.set_bg_flip(true);
            }
            u.set_bg_show(true);
        });
    });
}

/// Lecture depuis une fiche locale : l'élément, ou le premier épisode pas encore vu de la série / saison.
fn play_local_key(app: &Arc<App>, key: &str) {
    let entries = local_entries();
    let id = real_id(key);
    let mut eps: Vec<&downloads::Entry> = if key.starts_with("dl:series:") {
        entries.iter().filter(|e| e.series_id == id).collect()
    } else if key.starts_with("dl:season:") {
        entries.iter().filter(|e| e.season_id == id).collect()
    } else {
        entries.iter().filter(|e| e.id == id).collect()
    };
    sort_episodes(&mut eps);
    let pick = eps.iter().find(|e| !e.played).or(eps.first()).map(|e| e.id.clone());
    if let Some(id) = pick {
        play_download(app, &id);
    }
}

/// Suppression depuis une fiche locale (l'élément, ou toute la série / saison), puis retour.
fn delete_local_key(app: &Arc<App>, key: &str) {
    let id = real_id(key).to_string();
    for e in downloads::list() {
        let hit = if key.starts_with("dl:series:") {
            e.series_id == id
        } else if key.starts_with("dl:season:") {
            e.season_id == id
        } else {
            e.id == id
        };
        if hit {
            downloads::remove(&e.id);
        }
    }
    PAGES.with_borrow_mut(|p| p.retain(|k, _| !k.starts_with("dl:")));
    // Retour à la page la plus proche qui existe encore (saison, série...), sinon à la liste des
    // téléchargements (série entièrement supprimée).
    let top = {
        let mut st = app.stack.lock().unwrap();
        st.pop();
        while st.last().is_some_and(|t| t.starts_with("dl:") && !local_key_exists(t)) {
            st.pop();
        }
        st.last().cloned()
    };
    sync_can_back(app);
    refresh_downloads(app);
    if let Some(u) = app.ui().upgrade() {
        u.set_toast("Téléchargement supprimé.".into());
    }
    match top {
        Some(id) => start_detail(app, id),
        None => open_downloads(app),
    }
}

/// La page locale (série, saison, élément) a-t-elle encore des téléchargements ?
fn local_key_exists(key: &str) -> bool {
    let id = real_id(key);
    downloads::list().iter().any(|e| {
        if key.starts_with("dl:series:") {
            e.series_id == id
        } else if key.starts_with("dl:season:") {
            e.season_id == id
        } else {
            e.id == id
        }
    })
}

/// Mode hors ligne : écran Téléchargements, avec un message.
fn go_offline(app: &Arc<App>) {
    let was = app.offline.swap(true, Ordering::SeqCst);
    let a = app.clone();
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        u.set_offline(true);
        open_downloads(&a);
        u.set_toast("Serveur injoignable : mode hors ligne (téléchargements).".into());
    });
    if !was {
        watch_reconnect(app);
    }
}

/// Hors ligne : le serveur est réessayé toutes les 30 s. Dès qu'il répond (et hors lecture), les
/// lectures faites hors ligne lui sont envoyées et l'accueil revient.
fn watch_reconnect(app: &Arc<App>) {
    let a = app.clone();
    app.rt.spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            if !a.offline.load(Ordering::SeqCst) {
                return;
            }
            if a.playing.load(Ordering::SeqCst) {
                continue;
            }
            let mut saved = config::load();
            if saved.token.is_empty() {
                return;
            }
            let best = discovery::pick(&saved.server_local, &saved.server_remote, saved.prefer_remote).await;
            if !best.is_empty() {
                saved.server = best;
            }
            if !discovery::reachable(&saved.server).await {
                continue;
            }
            let Ok(client) = api::Client::from_saved(&saved) else { return };
            config::save(&saved);
            let _ = a.ui().upgrade_in_event_loop(|u| u.set_toast("Serveur de nouveau joignable : retour en ligne.".into()));
            load_home(a.clone(), client).await;
            return;
        }
    });
}

fn open_downloads(app: &Arc<App>) {
    app.stack.lock().unwrap().clear();
    sync_can_back(app);
    refresh_downloads(app);
    push_dl_status(app);
    if let Some(u) = app.ui().upgrade() {
        u.set_h_focus(false);
        u.set_screen("downloads".into());
    }
}

/// Lecture d'un téléchargement (fonctionne sans serveur).
fn play_download(app: &Arc<App>, id: &str) {
    let Some(e) = downloads::get(id) else { return };
    let a = app.clone();
    app.rt.spawn(async move { play_local(a, e).await });
}
