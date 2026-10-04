mod api;
mod config;
mod discovery;
mod downloads;
mod mpv;
mod player;
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
    tv: bool,
    /// Lecture d'essai sans serveur : --test-video=FICHIER_OU_URL.
    test_video: Option<String>,
    /// Lecture directe d'un élément du serveur (essais) : --play=ID[@SECONDES].
    play: Option<String>,
}

fn parse_cli() -> Cli {
    let mut positional: Vec<String> = Vec::new();
    let mut cli = Cli { user: None, pass: None, server: None, tv: false, test_video: None, play: None };

    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--tv" => cli.tv = true,
            "--desktop" => cli.tv = false,
            s if s.starts_with("--server=") => cli.server = Some(s["--server=".len()..].to_string()),
            s if s.starts_with("--test-video=") => cli.test_video = Some(s["--test-video=".len()..].to_string()),
            s if s.starts_with("--play=") => cli.play = Some(s["--play=".len()..].to_string()),
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
    /// Mode TV (cartes plus grandes : sert au calcul des coins arrondis des images).
    tv: bool,
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
    /// Page Seerr affichée.
    seerr_page: Mutex<Option<api::SeerrDetails>>,
    /// Le compte peut télécharger (bouton Télécharger des fiches).
    can_download: AtomicBool,
    /// File de téléchargement : (id, titre) en attente, et celui en cours (id, titre, avancement).
    dl_queue: Mutex<std::collections::VecDeque<(String, String)>>,
    dl_current: Mutex<Option<(String, String, f32)>>,
    /// Pas de serveur joignable : seuls les téléchargements sont accessibles.
    offline: AtomicBool,
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

fn handle_error(ui: &slint::Weak<AppWindow>, e: anyhow::Error) {
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
    let img = match shape {
        Some(s) => {
            // Recadrage au format exact (comme image-fit: cover), puis coins arrondis.
            let mut img = img.resize_to_fill(s.w, s.h, image::imageops::FilterType::Triangle).to_rgba8();
            round_corners(&mut img, s.radius * s.w as f32, s.top_only);
            img
        }
        None => img.to_rgba8(),
    };
    let (w, h) = img.dimensions();
    Some(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), w, h))
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
fn set_card_image(ui: &AppWindow, si: usize, ci: usize, expected_id: &str, buf: SharedPixelBuffer<Rgba8Pixel>) {
    let sections = ui.get_sections();
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
        Ok(client) => {
            let old = config::load();
            let mut saved = client.to_saved();
            saved.prefer_remote = old.prefer_remote;
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
            config::save(&saved);
            load_home(app, client).await;
        }
        Err(e) => show_login_error(&app.ui(), format!("{e}")),
    }
}

/// Ouvre la session : accueil (onglet affiché) avec ce client.
async fn load_home(app: Arc<App>, client: api::Client) {
    *app.client.lock().unwrap() = Some(client.clone());
    app.stack.lock().unwrap().clear();
    // Essais : --play=ID[@SECONDES] lance directement la lecture.
    if let Some(p) = app.play_arg.lock().unwrap().take() {
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
    app.can_download.store(client.can_download().await, Ordering::SeqCst);
    load_tab(app, client).await;
}

/// Charge l'onglet affiché (« Accueil » ou « Favoris ») et le présente.
async fn load_tab(app: Arc<App>, client: api::Client) {
    let ui = app.ui();
    let _ = ui.upgrade_in_event_loop(|u| u.set_screen("loading".into()));
    let tab = app.tab.lock().unwrap().clone();
    let seerr = *app.seerr_user.lock().unwrap();
    let sections = match (tab.as_str(), seerr) {
        ("favorites", _) => favorite_sections(&client).await,
        ("requests", Some(id)) => request_sections(&client, id).await,
        _ => home_sections(&client).await,
    };
    match sections {
        Ok(s) => {
            app.offline.store(false, Ordering::SeqCst);
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
    let groups: [(&str, &[i64]); 4] =
        [("En attente", &[1]), ("Acceptées", &[2, 5]), ("Refusées", &[3]), ("En échec", &[4])];
    let mut sections: Vec<SectionData> = Vec::new();
    for (title, states) in groups {
        let cards: Vec<api::CardInfo> = reqs.iter().filter(|r| states.contains(&r.status)).map(|r| r.card()).collect();
        if !cards.is_empty() {
            sections.push(SectionData { title: title.to_string(), landscape: false, cards });
        }
    }
    Ok(sections)
}

/// Affiche des rangées de cartes et lance le chargement de leurs images.
fn present_sections(app: &Arc<App>, client: &api::Client, sections: Vec<SectionData>) {
    let ui = app.ui();

    // Dimensions : voir card-w et SectionRow dans app.slint (rangée = 132 px x k + image).
    let k = if app.tv { 1.4_f32 } else { 1.0 };
    let card_w = if app.tv { 230.0_f32 } else { 170.0 };
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
                                ..Default::default()
                            })
                            .collect::<Vec<_>>(),
                    )),
                };
                y += h;
                row
            })
            .collect();

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
        u.set_screen("home".into());
    });

    let apply: Apply = Arc::new(
        |u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            set_card_image(u, job.a, job.b, &job.item_id, buf)
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
        let mut rows: Vec<SectionData> = Vec::new();
        let people = item.people_cards();
        if !people.is_empty() {
            rows.push(SectionData { title: "Casting et équipe".into(), landscape: false, cards: people });
        }
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

        // Dimensions : cartes de 140 px x k (voir detail-rows dans app.slint).
        let k = if app2.tv { 1.4_f32 } else { 1.0 };
        let card_w = 140.0 * k;
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

/// Favori / vu : bascule côté serveur, puis mise à jour du bouton (sans recharger la fiche).
fn toggle_flag(app: &Arc<App>, action: &str) {
    let Some(client) = app.client() else { return };
    let Some(u) = app.ui().upgrade() else { return };
    let d = u.get_detail();
    let id = d.id.to_string();
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
        spawn_image_jobs(&app2, &client, jobs, api::Size::Fill(185, 278), Shape::card(185, 278, 110.0), apply);
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
    let title = lib.name.clone();
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        u.set_lib_title(title.into());
        u.set_lib_items(ModelRc::new(VecModel::<CardData>::default()));
        u.set_lib_total(0);
        u.set_l_sel(0);
        u.set_h_focus(false);
        u.set_screen("library".into());
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
                        ..Default::default()
                    });
                }
            }
            u.set_lib_total(total as i32);
            if let Some(sel) = select {
                u.set_l_sel(sel.min(u.get_lib_items().row_count().saturating_sub(1)) as i32);
            }
        });
        let card_w = if app2.tv { 230.0 } else { 170.0 };
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

fn open_settings(app: &Arc<App>) {
    let saved = config::load();
    let current = app.client().map(|c| c.server).unwrap_or_default();
    if let Some(u) = app.ui().upgrade() {
        u.set_prefer_remote(saved.prefer_remote);
        u.set_addr_local(saved.server_local.into());
        u.set_addr_remote(saved.server_remote.into());
        u.set_addr_current(current.into());
        u.set_set_sel(0);
        u.set_h_focus(false);
        u.set_screen("settings".into());
    }
}

/// Préférence réseau : enregistrée, puis la meilleure adresse est choisie tout de suite.
fn set_prefer_remote(app: &Arc<App>, prefer: bool) {
    let mut saved = config::load();
    saved.prefer_remote = prefer;
    config::save(&saved);
    if let Some(u) = app.ui().upgrade() {
        u.set_prefer_remote(prefer);
    }
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
        let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_addr_current(best.into()));
    });
}

// ---------------------------------------------------------------------------
// Menu latéral : bibliothèques, demandes, compte
// ---------------------------------------------------------------------------
fn set_menu(app: &Arc<App>, views: &[api::Item]) {
    let has_requests = app.seerr_user.lock().unwrap().is_some();
    let mut e: Vec<(String, String, bool)> = vec![
        ("Navigation".into(), String::new(), true),
        ("Accueil".into(), "home".into(), false),
    ];
    if has_requests {
        e.push(("Demandes".into(), "requests".into(), false));
    }
    e.push(("Téléchargements".into(), "downloads".into(), false));
    e.push(("Bibliothèques".into(), String::new(), true));
    for v in views.iter().filter(|v| v.collection_type.as_deref() != Some("livetv")) {
        e.push((v.name.clone(), format!("lib:{}", v.id), false));
    }
    e.push(("Compte".into(), String::new(), true));
    for (label, action) in [
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
    let can = !app.stack.lock().unwrap().is_empty();
    if let Some(u) = app.ui().upgrade() {
        u.set_can_back(can);
    }
}

/// Retour direct à l'accueil (bouton Accueil, menu).
fn go_home(app: &Arc<App>) {
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
        u.set_screen(if stale { "loading" } else { "home" }.into());
    }
    if stale {
        if let Some(client) = app.client() {
            let a = app.clone();
            app.rt.spawn(async move { load_tab(a, client).await });
        }
    }
}

fn start_detail(app: &Arc<App>, id: String) {
    let Some(client) = app.client() else { return };
    let my_gen = app.gen.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(u) = app.ui().upgrade() {
        u.set_overview_open(false);
        u.set_toast("".into());
        u.set_screen("loading".into());
    }
    let app2 = app.clone();
    app.rt.spawn(async move {
        load_detail(app2, client, id, my_gen).await;
    });
}

fn go_back(app: &Arc<App>) {
    let top = {
        let mut s = app.stack.lock().unwrap();
        s.pop();
        s.last().cloned()
    };
    sync_can_back(app);
    match top {
        Some(id) => start_detail(app, id),
        // Pile vide : accueil (rechargé si une lecture a eu lieu : Reprendre / À suivre à jour).
        None => go_home(app),
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
    let _ = ui.upgrade_in_event_loop(move |u| {
        let detail = DetailData {
            id: id_for_ui.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            misc: misc.into(),
            overview: overview.into(),
            expect_logo,
            children_title: children_title.into(),
            children_landscape: landscape,
            has_next,
            next_id: next_id.into(),
            next_title: next_title.into(),
            next_subtitle: next_sub.into(),
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
                ..Default::default()
            })
            .collect();

        u.set_detail(detail);
        u.set_child_items(ModelRc::new(VecModel::from(cards)));
        u.set_detail_rows(ModelRc::default());
        u.set_d_x(0);
        u.set_d_zone(0);
        u.set_d_button(0);
        u.set_d_child(0);
        u.set_overview_open(false);
        u.set_screen("detail".into());
    });

    // Poster (400x600) : saison pour un épisode, sinon élément ou série.
    {
        let client = client.clone();
        let ui = app.ui();
        let id = item_id.clone();
        let cands = item.poster_candidates();
        // poster-w dans app.slint : 200 px x k (k = 1,4 en mode TV).
        let shape = Shape::card(400, 600, if app.tv { 280.0 } else { 200.0 });
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
            let shape = Shape::card(320, 180, if app.tv { 364.0 } else { 260.0 });
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

    spawn_detail_rows(&app, &client, &item);

    // Vignettes des enfants : 16:9 pour les épisodes, posters sinon.
    let size = if landscape { api::Size::Fill(400, 225) } else { api::Size::Fill(270, 405) };
    // child-card-w dans app.slint : (240 px en 16:9, 120 sinon) x k.
    let k = if app.tv { 1.4 } else { 1.0 };
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
    play_flow_with(app, None, Some(url), Some((e.title.clone(), e.subtitle.clone())), subs).await;
}

async fn play_flow(app: Arc<App>, id: Option<String>, test_url: Option<String>) {
    play_flow_with(app, id, test_url, None, Vec::new()).await;
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
            _ => (None, 0.0),
        };
        let req = player::PlayRequest { item, start_secs, test_url, local_title, local_subs };
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
    let ui = AppWindow::new()?;
    let video_ok = match video::install(&ui) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("turtlefin : affichage vidéo indisponible ({e:?})");
            false
        }
    };
    ui.set_tv_mode(cli.tv);
    if cli.tv {
        ui.window().set_fullscreen(true);
    }

    let app = Arc::new(App {
        rt: rt.handle().clone(),
        ui: Mutex::new(ui.as_weak()),
        client: Mutex::new(None),
        stack: Mutex::new(Vec::new()),
        gen: AtomicU64::new(0),
        device_id: saved.device_id.clone(),
        tv: cli.tv,
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
        dl_queue: Mutex::new(std::collections::VecDeque::new()),
        dl_current: Mutex::new(None),
        offline: AtomicBool::new(false),
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
        move || {
            config::clear_token();
            *app.client.lock().unwrap() = None;
            *app.tab.lock().unwrap() = "home".to_string();
            app.stack.lock().unwrap().clear();
            *app.library.lock().unwrap() = None;
            app.gen.fetch_add(1, Ordering::SeqCst);
            if let Some(u) = app.ui().upgrade() {
                u.set_tab("home".into());
                u.set_h_focus(false);
                u.set_can_back(false);
                u.set_has_requests(false);
                u.set_menu_open(false);
                u.set_lib_items(ModelRc::default());
                u.set_sections(ModelRc::default());
                u.set_detail(DetailData::default());
                u.set_child_items(ModelRc::default());
                u.set_busy(false);
                u.set_screen("login".into());
            }
        }
    });

    ui.on_select_tab({
        let app = app.clone();
        move |tab| {
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

    ui.on_seerr_action({
        let app = app.clone();
        move || seerr_action(&app)
    });

    ui.on_dl_play({
        let app = app.clone();
        move |id| play_download(&app, &id)
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

    ui.on_set_prefer_remote({
        let app = app.clone();
        move |p| set_prefer_remote(&app, p)
    });

    // Navigation entre rangées : carte de la rangée cible dont le centre, avec le défilement
    // actuel de cette rangée (celui de sa dernière sélection, comme SectionRow), est le plus
    // proche de `x` : on arrive sur la carte visuellement au-dessus / en dessous.
    let row_mem: Arc<Mutex<std::collections::HashMap<i32, i32>>> = Arc::default();
    ui.on_row_seen({
        let m = row_mem.clone();
        move |key, idx| {
            m.lock().unwrap().insert(key, idx);
        }
    });
    ui.on_row_pick({
        let m = row_mem.clone();
        move |x, rw, n, w, gap, pad, key| {
            if n <= 0 {
                return 0;
            }
            let rem = m.lock().unwrap().get(&key).copied().unwrap_or(0).clamp(0, n - 1);
            let strip = n as f32 * (w + gap) + 2.0 * pad;
            let base = |i: i32| pad + i as f32 * (w + gap) + w / 2.0;
            let offset = (rw - strip).max(rw / 2.0 - base(rem)).min(0.0);
            (0..n).min_by(|a, b| (base(*a) + offset - x).abs().total_cmp(&(base(*b) + offset - x).abs())).unwrap_or(0)
        }
    });
    // Nouvelles rangées (accueil rechargé, autre fiche) : défilements oubliés.
    {
        let m = row_mem.clone();
        let weak = ui.as_weak();
        let last: Arc<Mutex<(usize, String)>> = Arc::default();
        let t = slint::Timer::default();
        t.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(300), move || {
            let Some(u) = weak.upgrade() else { return };
            let now = (u.get_sections().row_count(), u.get_detail().id.to_string());
            let mut l = last.lock().unwrap();
            if *l != now {
                *l = now;
                m.lock().unwrap().clear();
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
                push_detail(&app, id.to_string());
            } else if a == "settings" {
                open_settings(&app);
            } else if a == "downloads" {
                open_downloads(&app);
            } else if a == "server" {
                open_servers(&app);
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
        move |id| push_detail(&app, id.to_string())
    });

    ui.on_run_action({
        let app = app.clone();
        move |action| {
            let a = action.as_str();
            if a == "back" {
                go_back(&app);
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
            } else if a == "fav" || a == "played" {
                toggle_flag(&app, a);
            } else if a == "download" {
                start_download(&app);
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

    // Essai du lecteur sans serveur : turtlefin --test-video=chemin/vers/video.mkv
    if let Some(url) = cli.test_video.clone() {
        let a = app.clone();
        rt.spawn(async move { play_flow(a, None, Some(url)).await });
    }

    ui.run()?;
    Ok(())
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

/// Grille de l'écran Téléchargements (affiches lues sur le disque).
fn refresh_downloads(app: &Arc<App>) {
    let entries = downloads::list();
    let k = if app.tv { 1.4 } else { 1.0 };
    let card_w = if app.tv { 230.0 } else { 170.0 };
    let _ = k;
    let ui = app.ui();
    let cards: Vec<(String, String, String)> = entries.iter().map(|e| (e.id.clone(), e.title.clone(), e.subtitle.clone())).collect();
    let _ = ui.upgrade_in_event_loop(move |u| {
        let rows: Vec<CardData> = cards
            .iter()
            .map(|(id, t, s)| CardData { id: id.clone().into(), title: t.clone().into(), subtitle: s.clone().into(), ..Default::default() })
            .collect();
        u.set_dl_items(ModelRc::new(VecModel::from(rows)));
        let n = u.get_dl_items().row_count() as i32;
        if u.get_dl_sel() >= n {
            u.set_dl_sel((n - 1).max(0));
        }
    });
    for (i, e) in entries.into_iter().enumerate() {
        let ui = app.ui();
        app.rt.spawn(async move {
            let path = e.poster_path();
            let buf = tokio::task::spawn_blocking(move || {
                let bytes = std::fs::read(path).ok()?;
                decode(&bytes, Some(Shape::card_top(270, 405, card_w)))
            })
            .await
            .ok()
            .flatten();
            let Some(buf) = buf else { return };
            let id = e.id.clone();
            let _ = ui.upgrade_in_event_loop(move |u| {
                let model = u.get_dl_items();
                if let Some(mut c) = model.row_data(i) {
                    if c.id.as_str() == id {
                        c.image = slint::Image::from_rgba8(buf);
                        c.has_image = true;
                        model.set_row_data(i, c);
                    }
                }
            });
        });
    }
}

/// Mode hors ligne : écran Téléchargements, avec un message.
fn go_offline(app: &Arc<App>) {
    app.offline.store(true, Ordering::SeqCst);
    let a = app.clone();
    let _ = app.ui().upgrade_in_event_loop(move |u| {
        open_downloads(&a);
        u.set_toast("Serveur injoignable : mode hors ligne (téléchargements).".into());
    });
}

fn open_downloads(app: &Arc<App>) {
    refresh_downloads(app);
    push_dl_status(app);
    if let Some(u) = app.ui().upgrade() {
        u.set_dl_sel(0);
        u.set_dl_dialog(false);
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
