//! Tableau de bord des administrateurs du serveur Jellyfin : ce que permet l'administration de
//! l'interface web (serveur, lectures en cours, bibliothèques, comptes, tâches, appareils, activité,
//! extensions), présenté comme la page Paramètres (mêmes catégories et lignes, `set-dash`).
//!
//! Chaque catégorie est lue sur le serveur à son ouverture (« Chargement… » en attendant), puis gardée
//! jusqu'à la suivante ou jusqu'à une action. Les clés des lignes commencent par « dash: » ; les actions
//! qui ne se rattrapent pas (redémarrer, arrêter, supprimer) demandent un second appui.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use reqwest::Method;
use serde_json::Value;

use crate::i18n::{tr, trf};
use crate::{App, SettingRow};

fn row(key: &str, label: &str, hint: &str, value: &str, kind: &str, on: bool) -> SettingRow {
    SettingRow { key: key.into(), label: label.into(), hint: hint.into(), value: value.into(), kind: kind.into(), on }
}

fn info(text: &str) -> SettingRow {
    row("info", "", text, "", "info", false)
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

/// « 2026-10-30T09:12:00.000Z » -> « 30/10 09:12 » (heure du serveur, UTC).
fn when(iso: &str) -> String {
    let (d, t) = iso.split_once('T').unwrap_or((iso, ""));
    let p: Vec<&str> = d.split('-').collect();
    match p.as_slice() {
        [_, m, day] => format!("{day}/{m} {}", t.get(..5).unwrap_or("")),
        _ => String::new(),
    }
}

/// Morceaux non vides, séparés par « · ».
fn join(parts: &[String]) -> String {
    parts.iter().map(|p| p.trim()).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" · ")
}

/// Dernier résultat d'une tâche planifiée, en clair.
fn task_status(s: &str) -> String {
    match s {
        "Completed" => tr("terminée"),
        "Failed" => tr("échouée"),
        "Cancelled" => tr("annulée"),
        "Aborted" => tr("interrompue"),
        other => return other.to_string(),
    }
    .to_string()
}

/// État d'une extension, en clair.
fn plugin_status(s: &str) -> String {
    match s {
        "Active" => tr("active"),
        "Disabled" => tr("désactivée"),
        "Restart" => tr("redémarrage du serveur nécessaire"),
        "Malfunctioned" => tr("en panne"),
        "NotSupported" => tr("incompatible avec cette version"),
        "Superseded" => tr("remplacée par une version plus récente"),
        "Deleted" => tr("supprimée au prochain redémarrage"),
        other => return other.to_string(),
    }
    .to_string()
}

/// Durée en secondes -> « 1:02:03 » ou « 2:03 ».
fn dur(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// Lignes de la catégorie `cat` : celles déjà lues, sinon « Chargement… » (et la lecture est lancée).
pub fn rows(app: &Arc<App>, cat: i32) -> Vec<SettingRow> {
    if let Some(r) = app.dash_rows.lock().unwrap().get(&cat) {
        return r.clone();
    }
    load(app, cat);
    vec![info(tr("Chargement…"))]
}

/// Relit la catégorie `cat` sur le serveur, puis l'affiche si elle l'est toujours.
pub fn load(app: &Arc<App>, cat: i32) {
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let rows = match fetch(&app2, &client, cat).await {
            Ok(r) => r,
            Err(e) => vec![info(&trf("Lecture impossible : {}", &[&crate::human_err(&e)]))],
        };
        app2.dash_rows.lock().unwrap().insert(cat, rows);
        let a3 = app2.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            if u.get_set_dash() && u.get_set_cat() == cat {
                crate::refresh_settings(&a3);
            }
        });
    });
}

async fn fetch(app: &Arc<App>, c: &crate::api::Client, cat: i32) -> anyhow::Result<Vec<SettingRow>> {
    let armed = app.dash_armed.lock().unwrap().clone();
    // Second appui attendu : la ligne le dit.
    let confirm = |key: &str, label: &str| if armed == key { tr("Confirmer ?").to_string() } else { label.to_string() };
    let mut v = Vec::new();
    match cat {
        // Serveur : identité, état, commandes générales.
        0 => {
            let i = c.admin_get("/System/Info", &[]).await?;
            let what = join(&[format!("Jellyfin {}", s(&i, "Version")), s(&i, "OperatingSystemDisplayName")]);
            v.push(row("info", &s(&i, "ServerName"), &what, "", "info", false));
            if i["HasPendingRestart"].as_bool().unwrap_or(false) {
                v.push(info(tr("Un redémarrage est nécessaire pour appliquer des changements.")));
            }
            if i["HasUpdateAvailable"].as_bool().unwrap_or(false) {
                v.push(info(tr("Une mise à jour de Jellyfin est disponible.")));
            }
            v.push(row("dash:scanall", tr("Analyser toutes les bibliothèques"), tr("Cherche les nouveaux fichiers et ceux qui ont disparu."), tr("Lancer"), "action", false));
            // Réglages du serveur (ceux qui se résument à oui / non) ; langue des informations en clair.
            let cfg = c.admin_get("/System/Configuration", &[]).await?;
            let lang = join(&[s(&cfg, "PreferredMetadataLanguage"), s(&cfg, "MetadataCountryCode")]);
            if !lang.is_empty() {
                v.push(row("info", tr("Langue des informations"), &lang, "", "info", false));
            }
            for (key, label, hint) in [
                ("QuickConnectAvailable", tr("Connexion rapide"), tr("Se connecter sur un nouvel appareil avec un code à six chiffres.")),
                ("EnableGroupingIntoCollections", tr("Regrouper en collections"), tr("Les films d'une même collection n'apparaissent qu'une fois dans Films.")),
                ("DisplaySpecialsWithinSeasons", tr("Épisodes spéciaux dans les saisons"), tr("Les épisodes spéciaux sont rangés dans la saison où ils ont été diffusés.")),
                ("EnableFolderView", tr("Vue par dossiers"), tr("Ajoute une bibliothèque qui montre les dossiers tels qu'ils sont sur le disque.")),
            ] {
                if let Some(on) = cfg[key].as_bool() {
                    v.push(row(&format!("dash:conf:{key}"), label, hint, "", "toggle", on));
                }
            }
            v.push(row("dash:restart", tr("Redémarrer le serveur"), tr("Les lectures en cours s'arrêtent le temps du redémarrage."), &confirm("dash:restart", tr("Redémarrer")), "action", false));
            v.push(row("dash:shutdown", tr("Arrêter le serveur"), tr("Il faudra le rallumer sur la machine elle-même."), &confirm("dash:shutdown", tr("Arrêter")), "action", false));
        }
        // Lectures en cours (et appareils connectés).
        1 => {
            let list = c.admin_get("/Sessions", &[("activeWithinSeconds", "960")]).await?;
            let mut n = 0;
            for x in list.as_array().into_iter().flatten() {
                let who = format!("{} · {}", s(x, "UserName"), s(x, "Client"));
                let dev = s(x, "DeviceName");
                if let Some(item) = x.get("NowPlayingItem").filter(|i| i.is_object()) {
                    n += 1;
                    let mut title = s(item, "Name");
                    if let Some(series) = item["SeriesName"].as_str() {
                        title = format!("{series} · {title}");
                    }
                    let pos = x["PlayState"]["PositionTicks"].as_u64().unwrap_or(0) / 10_000_000;
                    let len = item["RunTimeTicks"].as_u64().unwrap_or(0) / 10_000_000;
                    let how = match x["PlayState"]["PlayMethod"].as_str() {
                        Some("Transcode") => tr("transcodage"),
                        Some("DirectStream") => tr("flux direct"),
                        _ => tr("lecture directe"),
                    };
                    let paused = if x["PlayState"]["IsPaused"].as_bool().unwrap_or(false) { tr(" · en pause") } else { "" };
                    let hint = format!("{title} · {} / {} · {how}{paused} · {dev}", dur(pos), dur(len));
                    v.push(row(&format!("dash:stop:{}", s(x, "Id")), &who, &hint, tr("Arrêter"), "action", false));
                } else {
                    v.push(row("info", &who, &trf("Connecté · {}", &[&dev]), "", "info", false));
                }
            }
            if v.is_empty() {
                v.push(info(tr("Personne n'est connecté en ce moment.")));
            } else if n == 0 {
                v.insert(0, info(tr("Aucune lecture en cours.")));
            }
        }
        // Bibliothèques : dossiers, analyse.
        2 => {
            v.push(row("dash:scanall", tr("Analyser toutes les bibliothèques"), tr("Cherche les nouveaux fichiers et ceux qui ont disparu."), tr("Lancer"), "action", false));
            let list = c.admin_get("/Library/VirtualFolders", &[]).await?;
            for x in list.as_array().into_iter().flatten() {
                let paths: Vec<&str> = x["Locations"].as_array().into_iter().flatten().filter_map(|p| p.as_str()).collect();
                let hint = format!("{} · {}", s(x, "CollectionType"), paths.join(" · "));
                v.push(row(&format!("dash:scan:{}", s(x, "ItemId")), &s(x, "Name"), &hint, tr("Analyser"), "action", false));
            }
        }
        // Comptes : la liste, ou le détail d'un compte ouvert (ses droits, sa suppression).
        3 => {
            let me = c.user_id.clone();
            let open = app.dash_user.lock().unwrap().clone();
            if !open.is_empty() {
                let x = c.admin_get(&format!("/Users/{open}"), &[]).await?;
                let p = &x["Policy"];
                let b = |k: &str| p[k].as_bool().unwrap_or(false);
                v.push(row("dash:userback", &format!("‹  {}", s(&x, "Name")), tr("Retour à la liste des comptes."), tr("Retour"), "action", false));
                let mine = open == me;
                // Ses propres droits d'administrateur et son activation ne se changent pas d'ici
                // (on se fermerait la porte).
                let rights: [(&str, &str, &str, bool); 7] = [
                    ("IsDisabled", tr("Compte activé"), tr("Un compte désactivé ne peut plus se connecter."), true),
                    ("IsAdministrator", tr("Administrateur"), tr("Gère le serveur (tableau de bord, comptes, bibliothèques)."), false),
                    ("EnableRemoteAccess", tr("Accès depuis Internet"), tr("Peut se connecter hors du réseau de la maison."), false),
                    ("EnableContentDownloading", tr("Téléchargements"), tr("Peut télécharger les médias pour les regarder hors ligne."), false),
                    ("EnableVideoPlaybackTranscoding", tr("Transcodage vidéo"), tr("Le serveur peut convertir la vidéo quand l'appareil ne la lit pas telle quelle."), false),
                    ("EnableAllFolders", tr("Toutes les bibliothèques"), tr("Voit toutes les bibliothèques du serveur."), false),
                    ("EnableContentDeletion", tr("Suppression de médias"), tr("Peut supprimer des fichiers du serveur."), false),
                ];
                for (key, label, hint, inverted) in rights {
                    let on = if inverted { !b(key) } else { b(key) };
                    if mine && (key == "IsDisabled" || key == "IsAdministrator") {
                        v.push(row("info", label, &format!("{hint} · {}", tr("ce compte")), if on { tr("Oui") } else { tr("Non") }, "info", false));
                    } else {
                        v.push(row(&format!("dash:policy:{open}:{key}"), label, hint, "", "toggle", on));
                    }
                }
                if let Some(d) = x["LastActivityDate"].as_str() {
                    v.push(info(&trf("Dernière activité : {}", &[&when(d)])));
                }
                if !mine {
                    let key = format!("dash:userdel:{open}");
                    v.push(row(&key, tr("Supprimer le compte"), tr("Ses favoris et son avancement sont perdus."), &confirm(&key, tr("Supprimer")), "action", false));
                }
            } else {
                let list = c.admin_get("/Users", &[]).await?;
                for x in list.as_array().into_iter().flatten() {
                    let id = s(x, "Id");
                    let p = &x["Policy"];
                    let mut hint = Vec::new();
                    if p["IsAdministrator"].as_bool().unwrap_or(false) {
                        hint.push(tr("Administrateur").to_string());
                    }
                    if p["IsDisabled"].as_bool().unwrap_or(false) {
                        hint.push(tr("désactivé").to_string());
                    }
                    if let Some(d) = x["LastActivityDate"].as_str() {
                        hint.push(trf("vu le {}", &[&when(d)]));
                    }
                    if id == me {
                        hint.push(tr("ce compte").to_string());
                    }
                    v.push(row(&format!("dash:useropen:{id}"), &s(x, "Name"), &hint.join(" · "), tr("Gérer"), "action", false));
                }
            }
        }
        // Tâches planifiées : état, dernier résultat, lancer ou arrêter.
        4 => {
            let list = c.admin_get("/ScheduledTasks", &[("isHidden", "false")]).await?;
            let mut tasks: Vec<&Value> = list.as_array().into_iter().flatten().collect();
            tasks.sort_by_key(|t| (s(t, "Category"), s(t, "Name")));
            for t in tasks {
                let id = s(t, "Id");
                let running = s(t, "State") == "Running";
                let hint = if running {
                    trf("{} · en cours ({} %)", &[&s(t, "Category"), &(t["CurrentProgressPercentage"].as_f64().unwrap_or(0.0) as i64)])
                } else {
                    let last = &t["LastExecutionResult"];
                    match last["EndTimeUtc"].as_str() {
                        Some(end) => trf("{} · {} le {}", &[&s(t, "Category"), &task_status(&s(last, "Status")), &when(end)]),
                        None => s(t, "Category"),
                    }
                };
                let (key, label) = if running { (format!("dash:taskstop:{id}"), tr("Arrêter")) } else { (format!("dash:task:{id}"), tr("Lancer")) };
                v.push(row(&key, &s(t, "Name"), &hint, label, "action", false));
            }
        }
        // Appareils : suppression (déconnecte l'appareil), sauf celui-ci.
        5 => {
            let list = c.admin_get("/Devices", &[]).await?;
            for x in list["Items"].as_array().into_iter().flatten() {
                let id = s(x, "Id");
                let hint = join(&[format!("{} {}", s(x, "AppName"), s(x, "AppVersion")), s(x, "LastUserName"), when(&s(x, "DateLastActivity"))]);
                // Celui-ci : le serveur range parfois les appareils sous son propre identifiant ; la clé
                // d'accès, elle, est celle de cette connexion.
                // (à défaut : même nom de machine, appli Turtlefin et dernier compte = celui-ci).
                let mine = id == c.device_id
                    || (!c.token.is_empty() && s(x, "AccessToken") == c.token)
                    || (s(x, "AppName") == "Turtlefin" && s(x, "Name") == crate::api::device_name() && s(x, "LastUserId") == c.user_id);
                if mine {
                    v.push(row("info", &s(x, "Name"), &format!("{hint} · {}", tr("cet appareil")), "", "info", false));
                } else {
                    let key = format!("dash:device:{id}");
                    v.push(row(&key, &s(x, "Name"), &hint, &confirm(&key, tr("Supprimer")), "action", false));
                }
            }
        }
        // Activité récente du serveur.
        6 => {
            let list = c.admin_get("/System/ActivityLog/Entries", &[("startIndex", "0"), ("limit", "40")]).await?;
            for x in list["Items"].as_array().into_iter().flatten() {
                let hint = [when(&s(x, "Date")), s(x, "ShortOverview")].into_iter().filter(|t| !t.is_empty()).collect::<Vec<_>>().join(" · ");
                v.push(row("info", &s(x, "Name"), &hint, "", "info", false));
            }
            if v.is_empty() {
                v.push(info(tr("Aucune activité enregistrée.")));
            }
        }
        // Extensions : activées ou non (le serveur doit redémarrer pour en tenir compte).
        _ => {
            let list = c.admin_get("/Plugins", &[]).await?;
            let pending = app.dash_plugins.lock().unwrap().clone();
            for x in list.as_array().into_iter().flatten() {
                let (id, status) = (s(x, "Id"), s(x, "Status"));
                // Changement demandé pendant cette session : il ne compte qu'au redémarrage du serveur
                // (le serveur continue d'annoncer l'ancien état d'ici là).
                let on = pending.get(&id).copied().unwrap_or(status == "Active");
                let state = match pending.get(&id) {
                    Some(true) => tr("activée au prochain redémarrage").to_string(),
                    Some(false) => tr("désactivée au prochain redémarrage").to_string(),
                    None => plugin_status(&status),
                };
                let hint = join(&[s(x, "Version"), state, s(x, "Description").chars().take(80).collect::<String>()]);
                if x["CanUninstall"].as_bool() == Some(false) {
                    // Extension fournie avec Jellyfin : ni désactivable ni désinstallable.
                    v.push(row("info", &s(x, "Name"), &format!("{hint} · {}", tr("intégrée à Jellyfin")), "", "info", false));
                } else {
                    let key = format!("dash:plugin:{id}:{}", s(x, "Version"));
                    v.push(row(&key, &s(x, "Name"), &hint, "", "toggle", on));
                }
            }
            if v.is_empty() {
                v.push(info(tr("Aucune extension installée.")));
            }
        }
    }
    Ok(v)
}

/// Ligne du tableau de bord activée (clé « dash:… »).
pub fn activate(app: &Arc<App>, key: &str) {
    let Some(client) = app.client() else { return };
    // Actions qui ne se rattrapent pas : un premier appui arme, le second exécute.
    let risky = matches!(key, "dash:restart" | "dash:shutdown") || key.starts_with("dash:device:") || key.starts_with("dash:userdel:");
    // Ouvrir / fermer le détail d'un compte : rien à envoyer au serveur.
    if let Some(id) = key.strip_prefix("dash:useropen:") {
        *app.dash_user.lock().unwrap() = id.to_string();
        reload_current(app);
        return;
    }
    if key == "dash:userback" {
        app.dash_user.lock().unwrap().clear();
        reload_current(app);
        return;
    }
    if risky {
        let mut armed = app.dash_armed.lock().unwrap();
        if *armed != key {
            *armed = key.to_string();
            drop(armed);
            reload_current(app);
            return;
        }
        armed.clear();
    }
    let cat = app.ui().upgrade().map(|u| u.get_set_cat()).unwrap_or(0);
    let key = key.to_string();
    let app2 = app.clone();
    app.rt.spawn(async move {
        let parts: Vec<&str> = key.split(':').collect();
        let r = match parts.as_slice() {
            ["dash", "scanall"] => client.admin_send(Method::POST, "/Library/Refresh", &[], None).await.map(|_| tr("Analyse des bibliothèques lancée.")),
            ["dash", "restart"] => client.admin_send(Method::POST, "/System/Restart", &[], None).await.map(|_| tr("Le serveur redémarre.")),
            ["dash", "shutdown"] => client.admin_send(Method::POST, "/System/Shutdown", &[], None).await.map(|_| tr("Le serveur s'arrête.")),
            ["dash", "stop", id] => client.admin_send(Method::POST, &format!("/Sessions/{id}/Playing/Stop"), &[], None).await.map(|_| tr("Lecture arrêtée.")),
            ["dash", "scan", id] => client
                .admin_send(Method::POST, &format!("/Items/{id}/Refresh"), &[("Recursive", "true"), ("MetadataRefreshMode", "Default"), ("ImageRefreshMode", "Default")], None)
                .await
                .map(|_| tr("Analyse de la bibliothèque lancée.")),
            ["dash", "task", id] => client.admin_send(Method::POST, &format!("/ScheduledTasks/Running/{id}"), &[], None).await.map(|_| tr("Tâche lancée.")),
            ["dash", "taskstop", id] => client.admin_send(Method::DELETE, &format!("/ScheduledTasks/Running/{id}"), &[], None).await.map(|_| tr("Tâche arrêtée.")),
            ["dash", "device", id] => client.admin_send(Method::DELETE, "/Devices", &[("id", id)], None).await.map(|_| tr("Appareil supprimé.")),
            ["dash", "policy", id, field] => toggle_policy(&client, id, field).await,
            ["dash", "conf", field] => toggle_conf(&client, field).await,
            ["dash", "userdel", id] => {
                let r = client.admin_send(Method::DELETE, &format!("/Users/{id}"), &[], None).await;
                if r.is_ok() {
                    app2.dash_user.lock().unwrap().clear();
                }
                r.map(|_| tr("Compte supprimé."))
            }
            ["dash", "plugin", id, version] => toggle_plugin(&app2, &client, id, version).await,
            _ => return,
        };
        let msg = match r {
            Ok(m) => m.to_string(),
            Err(e) => trf("Action impossible : {}", &[&crate::human_err(&e)]),
        };
        // État relu (tâche lancée, appareil supprimé...) ; un instant pour que le serveur s'y mette.
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        app2.dash_rows.lock().unwrap().remove(&cat);
        load(&app2, cat);
        let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
    });
}

/// Réglage oui / non du serveur inversé : la configuration entière est renvoyée, seul `field` change.
async fn toggle_conf(c: &crate::api::Client, field: &str) -> anyhow::Result<&'static str> {
    let mut cfg = c.admin_get("/System/Configuration", &[]).await?;
    let now = !cfg[field].as_bool().unwrap_or(false);
    cfg[field] = now.into();
    c.admin_send(Method::POST, "/System/Configuration", &[], Some(&cfg)).await?;
    Ok(tr("Réglage du serveur enregistré."))
}

/// Un droit d'un compte inversé : la politique entière est renvoyée, seul `field` change.
async fn toggle_policy(c: &crate::api::Client, id: &str, field: &str) -> anyhow::Result<&'static str> {
    let user = c.admin_get(&format!("/Users/{id}"), &[]).await?;
    let mut policy = user["Policy"].clone();
    let now = !policy[field].as_bool().unwrap_or(false);
    policy[field] = now.into();
    c.admin_send(Method::POST, &format!("/Users/{id}/Policy"), &[], Some(&policy)).await?;
    Ok(match (field, now) {
        ("IsDisabled", true) => tr("Compte désactivé."),
        ("IsDisabled", false) => tr("Compte activé."),
        _ => tr("Droit modifié."),
    })
}

/// Extension activée <-> désactivée (prise en compte au prochain redémarrage du serveur) : l'état de
/// départ est celui affiché (changement déjà demandé dans la session, sinon celui du serveur).
async fn toggle_plugin(app: &Arc<App>, c: &crate::api::Client, id: &str, version: &str) -> anyhow::Result<&'static str> {
    let cached = app.dash_plugins.lock().unwrap().get(id).copied();
    let shown = match cached {
        Some(on) => on,
        None => {
            let list = c.admin_get("/Plugins", &[]).await?;
            list.as_array().into_iter().flatten().find(|p| p["Id"].as_str() == Some(id)).map(|p| p["Status"].as_str() == Some("Active")).unwrap_or(false)
        }
    };
    let what = if shown { "Disable" } else { "Enable" };
    c.admin_send(Method::POST, &format!("/Plugins/{id}/{version}/{what}"), &[], None).await?;
    app.dash_plugins.lock().unwrap().insert(id.to_string(), !shown);
    Ok(if shown { tr("Extension désactivée (au prochain redémarrage).") } else { tr("Extension activée (au prochain redémarrage).") })
}

/// La catégorie affichée est relue (après avoir armé une action risquée, ou au retour sur la page).
pub fn reload_current(app: &Arc<App>) {
    let Some(u) = app.ui().upgrade() else { return };
    let cat = u.get_set_cat();
    app.dash_rows.lock().unwrap().remove(&cat);
    load(app, cat);
}

/// Ouvre le tableau de bord (menu de gauche, administrateurs seulement).
pub fn open(app: &Arc<App>) {
    if !app.admin.load(Ordering::SeqCst) {
        return;
    }
    app.dash_rows.lock().unwrap().clear();
    app.dash_armed.lock().unwrap().clear();
    app.dash_user.lock().unwrap().clear();
    if let Some(u) = app.ui().upgrade() {
        u.set_set_dash(true);
        u.set_set_cat(0);
        u.set_set_sel(0);
        u.set_set_in(false);
        u.set_av_open(false);
        u.set_h_focus(false);
        u.set_screen("settings".into());
    }
    crate::refresh_settings(app);
}
