//! Démarrage : le logo (ui/boot.slint) fait les vérifications, puis ouvre la première page.
//! Sept points : les six sommets de l'hexagone, depuis le haut dans le sens horaire (langue ·
//! affichage · lecteur vidéo · stockage · configuration · réseau), puis le centre (serveur).
//! Chaque point est une vraie vérification (voir `check`).
//!
//! Première page : ligne de commande (compte et mot de passe, ou compte enregistré nommé) >
//! compte choisi pour le démarrage (Paramètres > Compte) > « Qui regarde ? » (ou la recherche
//! des serveurs s'il n'y en a encore aucun).

use std::sync::Arc;
use std::time::Duration;

use crate::{api, config, discovery, i18n::{tr, trf}, App, AppWindow, TrackData};
use slint::{ModelRc, VecModel};

/// Nombre de points : six sommets + le centre (serveur, vérifié en dernier, check : _).
const POINTS: usize = 7;


/// Ce que la ligne de commande demande au démarrage, et ce que `main` a déjà constaté.
#[derive(Clone, Default)]
pub struct Start {
    pub user: Option<String>,
    pub pass: Option<String>,
    pub server: String,
    /// L'adresse vient de `--server=` (sinon c'est celle de la session enregistrée).
    pub server_from_cli: bool,
    /// Pas d'animation (`--no-intro`) : vérifications sans attendre, page suivante tout de suite.
    pub no_intro: bool,
    /// Le rendu de la vidéo a pu être branché sur la fenêtre (`video::install`).
    pub video_ok: bool,
    /// Fichiers de configuration présents mais illisibles (`config::unreadable_files`, vérifiés
    /// par `main` avant toute lecture).
    pub config_issues: Vec<&'static str>,
}

/// Réponse attendue de l'interface (langue choisie, « continuer » après un échec, fin de la liaison).
pub type Waiter = std::sync::Mutex<Option<tokio::sync::oneshot::Sender<String>>>;

async fn wait(app: &Arc<App>, timeout: Option<Duration>) -> String {
    let (tx, rx) = tokio::sync::oneshot::channel();
    *app.boot_wait.lock().unwrap() = Some(tx);
    match timeout {
        Some(t) => tokio::time::timeout(t, rx).await.ok().and_then(|r| r.ok()).unwrap_or_default(),
        None => rx.await.unwrap_or_default(),
    }
}

/// Réponse de l'interface (rappels boot-lang, boot-continue, boot-linked).
pub fn answer(app: &Arc<App>, v: String) {
    if let Some(tx) = app.boot_wait.lock().unwrap().take() {
        let _ = tx.send(v);
    }
}

fn ui(app: &Arc<App>, f: impl FnOnce(&AppWindow) + Send + 'static) {
    let _ = app.ui().upgrade_in_event_loop(move |u| f(&u));
}

fn set_state(app: &Arc<App>, i: usize, s: i32) {
    ui(app, move |u| {
        let mut v: Vec<i32> = (0..POINTS).map(|j| slint::Model::row_data(&u.get_boot_state(), j).unwrap_or(0)).collect();
        v[i] = s;
        u.set_boot_state(ModelRc::new(VecModel::from(v)));
    });
}

const NAMES: [&str; POINTS] = ["Langue", "Affichage", "Lecteur vidéo", "Stockage", "Configuration", "Réseau", "Serveur"];

/// Libellés des sept points (dans la langue choisie).
fn labels() -> Vec<slint::SharedString> {
    NAMES.into_iter().map(|n| tr(n).into()).collect()
}

/// Textes du logo pendant le choix de la langue (étiquettes, « Langue… »), tels qu'affichés
/// pendant le repérage des lettres de l'effet (i18n::pseudo).
fn boot_texts(u: &AppWindow) {
    let l: Vec<slint::SharedString> = labels().iter().map(|s| crate::i18n::pseudo(s).into()).collect();
    u.set_boot_labels(ModelRc::new(VecModel::from(l)));
    u.set_boot_text(crate::i18n::pseudo(&format!("{}…", tr(NAMES[0]))).into());
}

/// Un dossier accepte-t-il l'écriture (fichier créé, relu, supprimé) ?
fn writable(dir: Option<std::path::PathBuf>, what: &str) -> Result<(), String> {
    let dir = dir.ok_or_else(|| trf("Dossier {} introuvable.", &[&what]))?;
    let probe = dir.join(".turtlefin-ecriture");
    std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::write(&probe, b"ok"))
        .and_then(|_| std::fs::read(&probe))
        .and_then(|b| if b == b"ok" { Ok(()) } else { Err(std::io::Error::other("relecture différente")) })
        .and_then(|_| std::fs::remove_file(&probe))
        .map_err(|e| {
            eprintln!("turtlefin : écriture impossible dans {} ({e})", dir.display());
            trf("Impossible d'écrire dans le dossier {} : {}", &[&what, &dir.display()])
        })
}

/// Résultat d'une vérification : Ok(true) tout va bien · Ok(false) à configurer · Err(explication).
async fn check(i: usize, start: &Start) -> Result<bool, String> {
    match i {
        // Langue : celle choisie a ses traductions (sinon l'interface resterait à moitié traduite).
        0 => {
            let code = config::ui_prefs().language;
            match crate::i18n::check(&code) {
                Ok(n) => {
                    eprintln!("turtlefin : langue « {code} » ({n} textes traduits)");
                    Ok(true)
                }
                Err(e) => {
                    eprintln!("turtlefin : {e}");
                    crate::i18n::set_language("fr");
                    Err(tr("Traductions introuvables : l'interface reste en français.").to_string())
                }
            }
        }
        // Affichage : la fenêtre a bien obtenu un contexte OpenGL (nécessaire à la vidéo).
        1 => {
            if !start.video_ok {
                return Err(tr("Affichage OpenGL indisponible : l'interface marche, mais pas la vidéo.").to_string());
            }
            for _ in 0..30 {
                if let Some(info) = crate::video::gl_info() {
                    eprintln!("turtlefin : OpenGL {info}");
                    return Ok(true);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(tr("Affichage OpenGL indisponible : l'interface marche, mais pas la vidéo.").to_string())
        }
        // Lecteur vidéo : un vrai lecteur mpv est créé, initialisé puis détruit.
        2 => match tokio::task::spawn_blocking(crate::mpv::self_test).await {
            Ok(Ok(())) => Ok(true),
            Ok(Err(e)) => {
                eprintln!("turtlefin : {e}");
                Err(tr("Lecteur vidéo (libmpv) introuvable : la lecture ne fonctionnera pas.").to_string())
            }
            Err(_) => Err(tr("Lecteur vidéo (libmpv) introuvable : la lecture ne fonctionnera pas.").to_string()),
        },
        // Stockage : écriture et relecture dans les dossiers de configuration, de données et de cache.
        3 => {
            writable(crate::paths::config_dir(), tr("de configuration"))?;
            writable(crate::paths::data_dir(), tr("des téléchargements"))?;
            writable(crate::paths::cache_dir(), tr("du cache"))?;
            Ok(true)
        }
        // Configuration : session, comptes, réglages... lisibles (vérifiés avant la première lecture).
        4 => {
            if start.config_issues.is_empty() {
                Ok(true)
            } else {
                Err(trf("Fichier de configuration illisible ({}) : réglages par défaut utilisés.", &[&start.config_issues.join(", ")]))
            }
        }
        // Réseau : une interface active, ou une route vers l'extérieur.
        5 => {
            if tokio::task::spawn_blocking(discovery::has_network).await.unwrap_or(false) {
                Ok(true)
            } else {
                Err(tr("Aucun réseau : seuls les téléchargements seront disponibles.").to_string())
            }
        }
        // Serveur : il répond (adresse principale, sinon de secours) et c'est bien le même serveur.
        _ => {
            let s = config::load();
            if !start.server_from_cli && s.server.is_empty() && s.server_main.is_empty() && s.server_backup.is_empty() {
                return Ok(false); // premier lancement : la recherche des serveurs suivra
            }
            let (main, backup) = if start.server_from_cli {
                (start.server.clone(), String::new())
            } else {
                (if s.server_main.is_empty() { s.server.clone() } else { s.server_main.clone() }, s.server_backup.clone())
            };
            let best = discovery::pick(&main, &backup).await;
            match discovery::server_id(&best).await {
                None => Err(tr("Serveur injoignable : nouvel essai automatique ensuite.").to_string()),
                Some(id) if !start.server_from_cli && !s.server_id.is_empty() && id != s.server_id => {
                    eprintln!("turtlefin : {best} répond avec l'identifiant {id}, attendu {}", s.server_id);
                    Err(tr("Un autre serveur Jellyfin répond à cette adresse.").to_string())
                }
                Some(_) => Ok(true),
            }
        }
    }
}

/// Déroulement complet (dans le runtime tokio).
pub async fn run(app: Arc<App>, start: Start) {
    let animate = !start.no_intro;
    let step = Duration::from_millis(if animate { 200 } else { 0 });
    let labels = labels();
    ui(&app, move |u| {
        u.set_boot_labels(ModelRc::new(VecModel::from(labels)));
        u.set_boot_on(animate);
    });

    for i in 0..POINTS {
        set_state(&app, i, 1);
        let txt = format!("{}…", tr(NAMES[i]));
        ui(&app, move |u| u.set_boot_text(txt.into()));
        tokio::time::sleep(step).await;

        // Langue : demandée au premier lancement (choix gardé dans prefs.json).
        if i == 0 && config::ui_prefs().language.is_empty() {
            let all = crate::i18n::languages();
            let sys = crate::i18n::system_language();
            let sel = all.iter().position(|(id, _)| *id == sys).unwrap_or(0) as i32;
            let langs: Vec<TrackData> =
                all.into_iter().map(|(id, label)| TrackData { id: id.into(), label: label.into(), current: false }).collect();
            ui(&app, move |u| {
                u.set_boot_on(true);
                u.set_boot_lang_title("Langue · Language".into());
                u.set_boot_langs(ModelRc::new(VecModel::from(langs)));
                u.set_boot_lang_sel(sel);
                // Les textes du logo partent en nuage pendant le choix (dust.rs).
                crate::dust::dissolve(u, 2, std::rc::Rc::new(boot_texts));
                u.set_boot_lang_open(true);
            });
            let code = wait(&app, None).await;
            let mut p = config::ui_prefs();
            p.language = if code.is_empty() { "fr".into() } else { code };
            config::save_ui_prefs(&p);
            // La liste se ferme, puis les grains du nuage réécrivent les textes dans la langue
            // choisie (dust.rs) ; la suite attend la fin de l'effet.
            ui(&app, |u| u.set_boot_lang_open(false));
            tokio::time::sleep(Duration::from_millis(380)).await;
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            let lang = p.language.clone();
            ui(&app, move |u| {
                let apply: crate::dust::Apply = std::rc::Rc::new(|u: &AppWindow, code: &str| {
                    crate::i18n::set_language(code);
                    boot_texts(u);
                });
                crate::dust::reform(u, Some((lang, apply)), move |u| {
                    u.set_boot_on(animate);
                    let _ = tx.send(());
                });
            });
            let _ = rx.await;
        }

        // Visite guidée : proposée une fois, juste après la langue (pas sans animation : --no-intro).
        if i == 0 && animate && !config::ui_prefs().tutorial_offered {
            let items = vec![
                TrackData { id: "yes".into(), label: tr("Oui, montre-moi").into(), current: false },
                TrackData { id: "no".into(), label: tr("Non merci").into(), current: false },
            ];
            let title = tr("Visite guidée ?");
            ui(&app, move |u| {
                u.set_boot_lang_title(title.into());
                u.set_boot_langs(ModelRc::new(VecModel::from(items)));
                u.set_boot_lang_sel(0);
                u.set_boot_lang_open(true);
            });
            let answer = wait(&app, None).await;
            let mut p = config::ui_prefs();
            p.tutorial_offered = true;
            p.tutorial_pending = answer == "yes";
            config::save_ui_prefs(&p);
            ui(&app, |u| u.set_boot_lang_open(false));
        }

        let result = check(i, &start).await;
        match result {
            Ok(true) => set_state(&app, i, 2),
            Ok(false) => {
                // Pas encore de serveur : point orange et explication, la recherche suit.
                set_state(&app, i, 4);
                let txt = tr("Aucun serveur choisi : la recherche des serveurs va suivre.");
                ui(&app, move |u| u.set_boot_text(txt.into()));
                if animate {
                    tokio::time::sleep(Duration::from_millis(1400)).await;
                }
            }
            Err(msg) => {
                eprintln!("turtlefin : démarrage : {msg}");
                set_state(&app, i, 3);
                if animate {
                    let hint = tr("Continuer");
                    ui(&app, move |u| {
                        u.set_boot_fail(msg.into());
                        u.set_boot_fail_hint(hint.into());
                    });
                    // Sans réponse (télé sans clavier) : on continue tout seul après 12 s.
                    wait(&app, Some(Duration::from_secs(12))).await;
                    ui(&app, |u| u.set_boot_fail("".into()));
                }
            }
        }
    }
    ui(&app, |u| u.set_boot_text("".into()));

    if animate {
        // Les points se relient (liaison animée), puis on zoome dans le centre vers la page suivante.
        // Petite pause d'abord : le dernier point (le serveur) a le temps de passer au vert.
        tokio::time::sleep(Duration::from_millis(500)).await;
        ui(&app, |u| u.set_boot_phase(1));
        wait(&app, Some(Duration::from_secs(3))).await;
        let a = app.clone();
        ui(&app, move |u| {
            route(&a, &start, u);
            u.set_boot_phase(2);
        });
    } else {
        let a = app.clone();
        ui(&app, move |u| {
            route(&a, &start, u);
            u.set_boot_on(false);
        });
    }
}

/// Première page après les vérifications.
fn route(app: &Arc<App>, start: &Start, u: &AppWindow) {
    let saved = config::load();
    // 1. Ligne de commande : compte et mot de passe.
    if let (Some(user), Some(pw)) = (start.user.clone(), start.pass.clone()) {
        if start.server.is_empty() {
            u.set_screen("login".into());
            u.set_error_text(tr("Indique le serveur avec --server=URL (ou choisis-le ci-dessous).").into());
        } else {
            u.set_screen("login".into());
            u.set_busy(true);
            let (a, s) = (app.clone(), start.server.clone());
            app.rt.spawn(async move { crate::login_flow(a, s, user, pw).await });
        }
        return;
    }
    // 2. Ligne de commande : nom d'un compte enregistré (sans mot de passe).
    if let Some(name) = &start.user {
        if let Some(acc) = config::accounts().into_iter().find(|a| a.user_name.eq_ignore_ascii_case(name)) {
            u.set_screen("login".into());
            crate::login_pick(app, format!("acc:{}", acc.user_id));
            return;
        }
        // Pas enregistré sur cet appareil : sa connexion (mot de passe), pas un autre compte.
        if !saved.server.is_empty() || !start.server.is_empty() {
            u.set_screen("login".into());
            u.set_login_title(crate::i18n::trf("Connexion de {}", &[name]).into());
            u.set_login_user(name.as_str().into());
            u.set_login_field(1);
            u.set_login_mode("form".into());
            return;
        }
    }
    // 3. Compte choisi pour le démarrage (s'il est toujours enregistré sur cet appareil).
    let p = config::ui_prefs();
    let acc = config::accounts().into_iter().find(|a| a.user_id == p.autostart_user && a.server_id == p.autostart_server);
    if let Some(acc) = acc.filter(|_| !p.autostart_user.is_empty() && !saved.server.is_empty()) {
        let mut s = saved.clone();
        s.user_id = acc.user_id;
        s.user_name = acc.user_name;
        s.token = acc.token;
        s.server_id = acc.server_id;
        config::save(&s);
        u.set_screen("loading".into());
        let a = app.clone();
        app.rt.spawn(async move { crate::open_saved_session(a, s).await });
        return;
    }
    // 4. « Qui regarde ? » (la recherche des serveurs s'il n'y en a aucun).
    u.set_screen("login".into());
    let _ = api::normalize_server(&start.server);
}
