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

fn decode(bytes: &[u8]) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), w, h))
}

/// Télécharge (ou lit en cache) puis décode la première image disponible.
async fn fetch_decoded(
    client: &api::Client,
    candidates: Vec<api::ImageRef>,
    kind: &'static str,
    size: api::Size,
) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let bytes = client.image_first(&candidates, kind, size).await?;
    tokio::task::spawn_blocking(move || decode(&bytes)).await.ok().flatten()
}

// ---------------------------------------------------------------------------
// Images des cartes (accueil et rangée d'enfants)
// ---------------------------------------------------------------------------
#[derive(Clone)]
struct ImageJob {
    a: usize, // rangée (accueil) ; inutilisé pour les enfants
    b: usize, // position dans la rangée
    item_id: String,
    img_id: String,
    tag: Option<String>,
}

type Apply = Arc<dyn Fn(&AppWindow, &ImageJob, SharedPixelBuffer<Rgba8Pixel>) + Send + Sync>;

/// 6 téléchargements à la fois, décodage hors du thread de l'interface.
fn spawn_image_jobs(app: &Arc<App>, client: &api::Client, jobs: Vec<ImageJob>, size: api::Size, apply: Apply) {
    let permits = Arc::new(tokio::sync::Semaphore::new(6));
    for job in jobs {
        let client = client.clone();
        let ui = app.ui();
        let permits = permits.clone();
        let apply = apply.clone();
        app.rt.spawn(async move {
            let _permit = permits.acquire().await.ok();
            let cands = vec![(job.img_id.clone(), job.tag.clone())];
            let Some(buf) = fetch_decoded(&client, cands, "Primary", size).await else { return };
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

async fn load_home(app: Arc<App>, client: api::Client) {
    *app.client.lock().unwrap() = Some(client.clone());
    app.stack.lock().unwrap().clear();
    let ui = app.ui();
    let _ = ui.upgrade_in_event_loop(|u| u.set_screen("loading".into()));

    let (views, resume, next) = tokio::join!(client.views(), client.resume(), client.next_up());
    let views = match views {
        Ok(v) => v,
        Err(e) => {
            handle_error(&ui, e);
            return;
        }
    };

    let mut sections: Vec<SectionData> = Vec::new();
    // « Mes médias » en haut, en vignettes 16:9 (équivalent de l'addon horizontalMyMedia).
    let my_media: Vec<api::Item> = views
        .iter()
        .filter(|v| v.collection_type.as_deref() != Some("livetv"))
        .cloned()
        .collect();
    push_section(&mut sections, "Mes médias", true, Ok(my_media));
    push_section(&mut sections, "Reprendre", false, resume);
    push_section(&mut sections, "À suivre", false, next);

    for v in views.iter().filter(|v| {
        !matches!(v.collection_type.as_deref(), Some("playlists" | "livetv" | "boxsets"))
    }) {
        let latest = client.latest(&v.id).await;
        push_section(&mut sections, &format!("Récemment ajouté · {}", v.name), false, latest);
    }

    let make_jobs = |landscape: bool| -> Vec<ImageJob> {
        sections
            .iter()
            .enumerate()
            .filter(|(_, s)| s.landscape == landscape)
            .flat_map(|(si, s)| {
                s.cards.iter().enumerate().filter_map(move |(ci, c)| {
                    c.img_id.clone().map(|img_id| ImageJob {
                        a: si,
                        b: ci,
                        item_id: c.id.clone(),
                        img_id,
                        tag: c.img_tag.clone(),
                    })
                })
            })
            .collect()
    };
    let poster_jobs = make_jobs(false);
    let thumb_jobs = make_jobs(true);

    let user_name = client.user_name.clone();
    let _ = ui.upgrade_in_event_loop(move |u| {
        // Position verticale cumulée, en « unités de rangée » (une rangée de posters = 1).
        let mut y = 0.0_f32;
        let rows: Vec<Section> = sections
            .iter()
            .map(|s| {
                let rel_h = if s.landscape { 0.72_f32 } else { 1.0_f32 };
                let row = Section {
                    title: s.title.clone().into(),
                    landscape: s.landscape,
                    rel_y: y,
                    rel_h,
                    items: ModelRc::new(VecModel::from(
                        s.cards
                            .iter()
                            .map(|c| CardData {
                                id: c.id.clone().into(),
                                title: c.title.clone().into(),
                                subtitle: c.subtitle.clone().into(),
                                ..Default::default()
                            })
                            .collect::<Vec<_>>(),
                    )),
                };
                y += rel_h;
                row
            })
            .collect();

        u.set_sections(ModelRc::new(VecModel::from(rows)));
        u.set_user_name(user_name.into());
        u.set_sel_section(0);
        u.set_sel_item(0);
        u.set_toast("".into());
        u.set_busy(false);
        u.set_screen("home".into());
    });

    let apply: Apply = Arc::new(
        |u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            set_card_image(u, job.a, job.b, &job.item_id, buf)
        },
    );
    spawn_image_jobs(&app, &client, poster_jobs, api::Size::Fill(270, 405), apply.clone());
    spawn_image_jobs(&app, &client, thumb_jobs, api::Size::Fill(320, 180), apply);
}

// ---------------------------------------------------------------------------
// Navigation : fiche détail
// ---------------------------------------------------------------------------
fn push_detail(app: &Arc<App>, id: String) {
    app.stack.lock().unwrap().push(id.clone());
    start_detail(app, id);
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
    match top {
        Some(id) => start_detail(app, id),
        None => {
            app.gen.fetch_add(1, Ordering::SeqCst);
            let stale = app.home_stale.swap(false, Ordering::SeqCst);
            if let Some(u) = app.ui().upgrade() {
                // On libère les images de la fiche (poster, logo, vignettes).
                u.set_detail(DetailData::default());
                u.set_child_items(ModelRc::default());
                u.set_overview_open(false);
                u.set_screen(if stale { "loading" } else { "home" }.into());
            }
            // Une lecture a eu lieu : on recharge l'accueil (Reprendre / À suivre à jour).
            if stale {
                if let Some(client) = app.client() {
                    let a = app.clone();
                    app.rt.spawn(async move { load_home(a, client).await });
                }
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
                let screen = if prev_empty { "home" } else { "detail" };
                u.set_screen(screen.into());
            });
            return;
        }
    };

    // Séries/saisons : tout. Bibliothèques et collections : 60 premiers seulement
    // (une grille paginée viendra plus tard ; 300 posters en mémoire, c'est trop).
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
        .filter_map(|(ci, c)| {
            c.img_id.clone().map(|img_id| ImageJob {
                a: 0,
                b: ci,
                item_id: c.id.clone(),
                img_id,
                tag: c.img_tag.clone(),
            })
        })
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
        app.rt.spawn(async move {
            if let Some(buf) = fetch_decoded(&client, cands, "Primary", api::Size::Fill(400, 600)).await {
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
            let buf = fetch_decoded(&client, vec![logo_ref], "Logo", api::Size::MaxWidth(700)).await;
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
            app.rt.spawn(async move {
                if let Some(buf) = fetch_decoded(&client, vec![(img_id, tag)], "Primary", api::Size::Fill(320, 180)).await {
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
    let size = if landscape { api::Size::Fill(320, 180) } else { api::Size::Fill(270, 405) };
    let apply: Apply = Arc::new(
        |u: &AppWindow, job: &ImageJob, buf: SharedPixelBuffer<Rgba8Pixel>| {
            set_child_image(u, job.b, &job.item_id, buf)
        },
    );
    spawn_image_jobs(&app, &client, child_jobs, size, apply);
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
            app.stack.lock().unwrap().clear();
            app.gen.fetch_add(1, Ordering::SeqCst);
            if let Some(u) = app.ui().upgrade() {
                u.set_sections(ModelRc::default());
                u.set_detail(DetailData::default());
                u.set_child_items(ModelRc::default());
                u.set_busy(false);
                u.set_screen("login".into());
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
