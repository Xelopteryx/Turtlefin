mod api;
mod config;
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
}

fn parse_cli() -> Cli {
    let mut positional: Vec<String> = Vec::new();
    let mut cli = Cli { user: None, pass: None, server: None, tv: false, test_video: None };

    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--tv" => cli.tv = true,
            "--desktop" => cli.tv = false,
            s if s.starts_with("--server=") => cli.server = Some(s["--server=".len()..].to_string()),
            s if s.starts_with("--test-video=") => cli.test_video = Some(s["--test-video=".len()..].to_string()),
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
            config::save(&client.to_saved());
            load_home(app, client).await;
        }
        Err(e) => show_login_error(&app.ui(), format!("{e}")),
    }
}

/// Ouvre la session : accueil (onglet affiché) avec ce client.
async fn load_home(app: Arc<App>, client: api::Client) {
    *app.client.lock().unwrap() = Some(client.clone());
    app.stack.lock().unwrap().clear();
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
        Ok(s) => present_sections(&app, &client, s),
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
    if id.is_empty() {
        if let Some(u) = app.ui().upgrade() {
            u.set_toast("Pas encore disponible dans Jellyfin.".into());
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
    let buttons = item.buttons();
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
                    .map(|(label, action)| ButtonData { label: label.into(), action: action.into() })
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
                ..Default::default()
            })
            .collect();

        u.set_detail(detail);
        u.set_child_items(ModelRc::new(VecModel::from(cards)));
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

async fn play_flow(app: Arc<App>, id: Option<String>, test_url: Option<String>) {
    let client = app.client();
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

    let result: anyhow::Result<()> = async {
        let (item, start_secs) = match (&client, id) {
            (Some(c), Some(id)) => {
                let target = resolve_playable(c, c.item(&id).await?).await?;
                let start = target
                    .user_data
                    .as_ref()
                    .map(|u| u.playback_position_ticks as f64 / 10_000_000.0)
                    .unwrap_or(0.0);
                (Some(target), start)
            }
            _ => (None, 0.0),
        };
        player::play(client.clone(), player::PlayRequest { item, start_secs, test_url }, rx, ui.clone()).await
    }
    .await;

    *app.player_tx.lock().unwrap() = None;
    app.playing.store(false, Ordering::SeqCst);
    app.home_stale.store(true, Ordering::SeqCst);

    let msg = result.err().map(|e| format!("Lecture impossible : {e}"));
    let top = app.stack.lock().unwrap().last().cloned();
    let app2 = app.clone();
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_playing(false);
        // Recharge la fiche : l'état « Reprendre » / « vu » a pu changer.
        if let Some(id) = top {
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
                if let Some(u) = app.ui().upgrade() {
                    u.set_soon_title("Paramètres".into());
                    u.set_screen("soon".into());
                }
            } else if a == "server" || a == "logout" {
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
        match api::Client::from_saved(&saved) {
            Ok(client) => {
                let a = app.clone();
                rt.spawn(async move { load_home(a, client).await });
            }
            Err(e) => ui.set_error_text(format!("{e}").into()),
        }
    }

    // Essai du lecteur sans serveur : turtlefin --test-video=chemin/vers/video.mkv
    if let Some(url) = cli.test_video.clone() {
        let a = app.clone();
        rt.spawn(async move { play_flow(a, None, Some(url)).await });
    }

    ui.run()?;
    Ok(())
}
