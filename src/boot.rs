//! Démarrage : le logo à six points (ui/boot.slint) fait les vérifications, puis ouvre la première
//! page. Ordre des points (sommets de l'hexagone, depuis le haut, sens horaire) :
//! langue · affichage · lecteur vidéo · stockage · réseau · serveur.
//!
//! Première page : ligne de commande (compte et mot de passe, ou compte enregistré nommé) >
//! compte choisi pour le démarrage (Paramètres > Compte) > « Qui regarde ? » (ou la recherche
//! des serveurs s'il n'y en a encore aucun).

use std::sync::Arc;
use std::time::Duration;

use crate::{api, config, discovery, i18n::tr, App, AppWindow, TrackData};
use slint::{ModelRc, VecModel};

/// Ce que la ligne de commande demande au démarrage.
#[derive(Clone, Default)]
pub struct Start {
    pub user: Option<String>,
    pub pass: Option<String>,
    pub server: String,
    /// Pas d'animation (`--no-intro`) : vérifications sans attendre, page suivante tout de suite.
    pub no_intro: bool,
    pub video_ok: bool,
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
        let mut v: Vec<i32> = (0..6).map(|j| slint::Model::row_data(&u.get_boot_state(), j).unwrap_or(0)).collect();
        v[i] = s;
        u.set_boot_state(ModelRc::new(VecModel::from(v)));
    });
}

/// Labels des six points (dans la langue choisie).
fn labels() -> Vec<slint::SharedString> {
    [tr("Langue"), tr("Affichage"), tr("Lecteur vidéo"), tr("Stockage"), tr("Réseau"), tr("Serveur")]
        .into_iter()
        .map(Into::into)
        .collect()
}

/// Résultat d'une vérification : Ok(true) tout va bien · Ok(false) à configurer · Err(explication).
async fn check(app: &Arc<App>, i: usize, start: &Start) -> Result<bool, String> {
    match i {
        0 => Ok(true), // langue : voir `run`
        1 => {
            if start.video_ok {
                Ok(true)
            } else {
                Err(tr("Affichage OpenGL indisponible : l'interface marche, mais pas la vidéo.").to_string())
            }
        }
        2 => crate::mpv::available().map(|_| true).map_err(|e| {
            eprintln!("turtlefin : {e}");
            tr("Lecteur vidéo (libmpv) introuvable : la lecture ne fonctionnera pas.").to_string()
        }),
        3 => {
            let dir = crate::paths::config_dir().ok_or_else(|| tr("Dossier de configuration introuvable.").to_string())?;
            let probe = dir.join(".ecriture");
            std::fs::create_dir_all(&dir)
                .and_then(|_| std::fs::write(&probe, b"ok"))
                .and_then(|_| std::fs::remove_file(&probe))
                .map(|_| true)
                .map_err(|e| format!("{} ({e})", tr("Impossible d'écrire la configuration")))
        }
        4 => {
            if tokio::task::spawn_blocking(discovery::has_network).await.unwrap_or(false) {
                Ok(true)
            } else {
                Err(tr("Aucun réseau : seuls les téléchargements seront disponibles.").to_string())
            }
        }
        _ => {
            let s = config::load();
            if s.server_main.is_empty() && s.server_backup.is_empty() && start.server.is_empty() {
                return Ok(false); // premier lancement : la recherche des serveurs suivra
            }
            let main = if start.server.is_empty() { s.server_main.clone() } else { start.server.clone() };
            let best = discovery::pick(&main, &s.server_backup).await;
            if !best.is_empty() && discovery::reachable(&best).await {
                Ok(true)
            } else {
                let _ = app;
                Err(tr("Serveur injoignable : nouvel essai automatique ensuite.").to_string())
            }
        }
    }
}

/// Déroulement complet (dans le runtime tokio).
pub async fn run(app: Arc<App>, start: Start) {
    let animate = !start.no_intro;
    let step = Duration::from_millis(if animate { 230 } else { 0 });
    let labels = labels();
    ui(&app, move |u| {
        u.set_boot_labels(ModelRc::new(VecModel::from(labels)));
        u.set_boot_on(animate);
    });

    for i in 0..6 {
        set_state(&app, i, 1);
        let txt = format!("{}…", tr(["Langue", "Affichage", "Lecteur vidéo", "Stockage", "Réseau", "Serveur"][i]));
        ui(&app, move |u| u.set_boot_text(txt.into()));
        tokio::time::sleep(step).await;

        // Langue : demandée au premier lancement (choix gardé dans prefs.json).
        if i == 0 && config::ui_prefs().language.is_empty() {
            let langs: Vec<TrackData> = crate::i18n::LANGUAGES
                .iter()
                .map(|(id, label)| TrackData { id: (*id).into(), label: (*label).into(), current: false })
                .collect();
            let sel = crate::i18n::LANGUAGES.iter().position(|(id, _)| *id == crate::i18n::system_language()).unwrap_or(0) as i32;
            ui(&app, move |u| {
                u.set_boot_on(true);
                u.set_boot_lang_title("Langue · Language".into());
                u.set_boot_langs(ModelRc::new(VecModel::from(langs)));
                u.set_boot_lang_sel(sel);
                u.set_boot_lang_open(true);
            });
            let code = wait(&app, None).await;
            let mut p = config::ui_prefs();
            p.language = if code.is_empty() { "fr".into() } else { code };
            config::save_ui_prefs(&p);
            crate::i18n::set_language(&p.language);
            let labels = self::labels();
            ui(&app, move |u| {
                u.set_boot_lang_open(false);
                u.set_boot_labels(ModelRc::new(VecModel::from(labels)));
                u.set_boot_on(animate);
            });
        }

        match check(&app, i, &start).await {
            Ok(ok) => set_state(&app, i, if ok { 2 } else { 4 }),
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
        tokio::time::sleep(Duration::from_millis(150)).await;
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
