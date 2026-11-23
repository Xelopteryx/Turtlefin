//! Tableau de bord des administrateurs du serveur Jellyfin : ce que permet le tableau de bord de
//! l'interface web (sauf la télévision en direct), présenté comme la page Paramètres (mêmes lignes et
//! mêmes touches, `set-dash`), catégories regroupées à gauche.
//!
//! Chaque catégorie est lue sur le serveur à son ouverture (« Chargement… » en attendant), puis gardée
//! jusqu'à une action ; une ligne peut ouvrir une sous-page (`dash_open`, Échap ou ← pour en sortir). Les
//! clés des lignes commencent par « dash: ». Réglages : `Fields` (un champ d'un document de configuration,
//! relu puis renvoyé entier). Les actions qui ne se rattrapent pas demandent un second appui.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use reqwest::Method;
use serde_json::Value;

use crate::i18n::{tr, trf};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{App, DashCard, SettingRow};

fn row(key: &str, label: &str, hint: &str, value: &str, kind: &str, on: bool) -> SettingRow {
    SettingRow { key: key.into(), label: label.into(), hint: hint.into(), value: value.into(), kind: kind.into(), on, img: -1, progress: 0.0, first: 0, count: 0 }
}

fn info(text: &str) -> SettingRow {
    row("info", "", text, "", "info", false)
}

/// Intertitre dans le panneau.
fn head(text: &str) -> SettingRow {
    row("head", text, "", "", "head", false)
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

/// « 2026-10-30T09:12:00.000Z » (heure universelle du serveur) -> « 30/10 10:12 », à l'heure locale.
fn when(iso: &str) -> String {
    if let Ok(t) = chrono::DateTime::parse_from_rfc3339(iso) {
        return t.with_timezone(&chrono::Local).format("%d/%m %H:%M").to_string();
    }
    // Sans fuseau (« 2026-10-30T09:12:00.0000000 ») : tel quel.
    let (d, t) = iso.split_once('T').unwrap_or((iso, ""));
    let p: Vec<&str> = d.split('-').collect();
    match p.as_slice() {
        [_, m, day] => format!("{day}/{m} {}", t.get(..5).unwrap_or("")),
        _ => String::new(),
    }
}

/// Nom pour un chemin d'adresse (espaces, accents...).
fn urlenc(s: &str) -> String {
    if !s.is_empty() && s.trim_matches('.').is_empty() {
        return "_".into();
    }
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Identifiant venu du serveur, placé dans un chemin d'adresse : encodé (pas de « / », « ? », « .. »
/// glissés dans l'adresse ; les points des numéros de version sont gardés).
fn pseg(s: impl AsRef<str>) -> String {
    crate::api::seg_dots(s.as_ref()).into_owned()
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
    if let Some(r) = app.dash_rows.lock().unwrap().get(&cat).cloned() {
        apply_cards(app, cat);
        return r;
    }
    load(app, cat);
    vec![info(tr("Chargement…"))]
}

/// Relit la catégorie `cat` sur le serveur, puis l'affiche si elle l'est toujours.
pub fn load(app: &Arc<App>, cat: i32) {
    let Some(client) = app.client() else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let mut cards = Vec::new();
        let rows = match fetch(&app2, &client, cat, &mut cards).await {
            Ok(r) => r,
            Err(e) => vec![info(&trf("Lecture impossible : {}", &[&crate::human_err(&e)]))],
        };
        app2.dash_cards.lock().unwrap().insert(cat, cards);
        app2.dash_rows.lock().unwrap().insert(cat, rows);
        let a3 = app2.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            if !u.get_set_dash() || u.get_set_cat() != cat {
                return;
            }
            let stats = a3.dash_stats.lock().unwrap().clone();
            u.set_dash_stats(ModelRc::new(VecModel::from(
                stats.into_iter().map(|(l, v)| crate::FactData { label: l.into(), value: v.into(), ..Default::default() }).collect::<Vec<_>>(),
            )));
            u.set_dash_sub(!a3.dash_open.lock().unwrap().is_empty());
            crate::refresh_settings(&a3);
            // Sous-page ouverte ou refermée : la sélection et le défilement repartent du haut.
            if a3.dash_top.swap(false, Ordering::SeqCst) {
                let first = crate::settings_step(&u.get_set_rows(), -1, 1);
                u.set_set_sel(first.max(0));
                u.set_set_top(u.get_set_top() + 1);
            }
        });
    });
}

/// Cartes de la catégorie affichée : mises à jour sur place (la vue d'ensemble est relue sans cesse).
fn apply_cards(app: &Arc<App>, cat: i32) {
    let Some(u) = app.ui().upgrade() else { return };
    let cards = app.dash_cards.lock().unwrap().get(&cat).cloned().unwrap_or_default();
    let cur = u.get_dash_cards();
    if let Some(vm) = cur.as_any().downcast_ref::<VecModel<DashCard>>() {
        if vm.row_count() == cards.len() {
            for (i, c) in cards.into_iter().enumerate() {
                if vm.row_data(i).as_ref() != Some(&c) {
                    vm.set_row_data(i, c);
                }
            }
            return;
        }
    }
    u.set_dash_cards(ModelRc::new(VecModel::from(cards)));
}

/// Grille de cartes dans la liste des lignes. `wide` : image en largeur (sinon portrait rond ou carré).
fn grid(v: &mut Vec<SettingRow>, cards: &mut Vec<DashCard>, wide: bool, list: Vec<DashCard>) {
    if list.is_empty() {
        return;
    }
    let mut r = row(&format!("cards{}", cards.len()), "", "", "", "cards", wide);
    r.first = cards.len() as i32;
    r.count = list.len() as i32;
    v.push(r);
    cards.extend(list);
}

fn card(key: &str, title: &str, sub: &str) -> DashCard {
    DashCard {
        key: key.into(),
        title: title.into(),
        sub: sub.into(),
        letter: title.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default().into(),
        img: -1,
        ..Default::default()
    }
}

/// En-tête d'une page : grande image, nom, détails.
fn hero(title: &str, hint: &str, img: i32, round: bool) -> SettingRow {
    let letter = title.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    let mut r = row("hero", title, hint, &letter, "hero", round);
    r.img = img;
    r
}

/// Image d'une carte : numéro dans `dash-images` (elle arrive ensuite). `src` : adresses à essayer dans
/// l'ordre, séparées par « | » (chemin sur le serveur, ou adresse complète pour le catalogue).
fn image_slot(app: &Arc<App>, c: &crate::api::Client, src: &str) -> i32 {
    let mut known = app.dash_img_ids.lock().unwrap();
    if let Some(i) = known.iter().position(|k| k == src) {
        return i as i32;
    }
    known.push(src.to_string());
    let i = known.len() - 1;
    let (client, src, ui) = (c.clone(), src.to_string(), app.ui());
    app.rt.spawn(async move {
        let (mut buf, mut svg) = (None, None);
        for one in src.split('|') {
            let bytes = if one.starts_with("http") { client.public_image(one).await } else { client.get_bytes(one).await };
            if let Ok(b) = bytes {
                // Logos d'extensions souvent en SVG : dessinés par Slint (sur le fil de l'interface).
                if b.iter().take(400).copied().collect::<Vec<u8>>().windows(4).any(|w| w == b"<svg") {
                    svg = Some(b);
                    break;
                }
                buf = tokio::task::spawn_blocking(move || crate::decode(&b, None)).await.ok().flatten();
                if buf.is_some() {
                    break;
                }
            }
        }
        let _ = ui.upgrade_in_event_loop(move |u| {
            let model = u.get_dash_images();
            let Some(vm) = model.as_any().downcast_ref::<VecModel<slint::Image>>() else { return };
            while vm.row_count() <= i {
                vm.push(slint::Image::default());
            }
            if let Some(b) = buf {
                vm.set_row_data(i, slint::Image::from_rgba8(b));
            } else if let Some(Ok(img)) = svg.map(|b| slint::Image::load_from_svg_data(&b)) {
                vm.set_row_data(i, img);
            }
        });
    });
    i as i32
}

// ---------------------------------------------------------------------------
// Réglages : chaque ligne lit et écrit un champ d'un document de configuration du serveur
// ---------------------------------------------------------------------------

/// Adresses de lecture et d'écriture d'un document de configuration.
fn sec_paths(sec: &str) -> (&'static str, &'static str) {
    match sec {
        "enc" => ("/System/Configuration/encoding", "/System/Configuration/encoding"),
        "net" => ("/System/Configuration/network", "/System/Configuration/network"),
        "nfo" => ("/System/Configuration/xbmcmetadata", "/System/Configuration/xbmcmetadata"),
        "brand" => ("/Branding/Configuration", "/System/Configuration/Branding"),
        _ => ("/System/Configuration", "/System/Configuration"),
    }
}

/// Champ d'un document, éventuellement imbriqué (« TrickplayOptions.Interval »).
fn jget<'a>(v: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(v, |cur, k| &cur[k])
}

fn jset(v: &mut Value, path: &str, val: Value) {
    let mut cur = v;
    let parts: Vec<&str> = path.split('.').collect();
    for k in &parts[..parts.len() - 1] {
        cur = &mut cur[*k];
    }
    cur[parts[parts.len() - 1]] = val;
}

/// Valeur d'un champ, en texte.
fn vtext(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(a) => a.iter().map(vtext).collect::<Vec<_>>().join(", "),
        _ => String::new(),
    }
}

/// Ce qu'un document de configuration donne à afficher.
struct Fields<'a> {
    app: &'a Arc<App>,
    v: &'a mut Vec<SettingRow>,
    sec: &'a str,
    cfg: &'a Value,
}

impl Fields<'_> {
    fn head(&mut self, text: &str) {
        self.v.push(head(text));
    }
    /// Oui / non. Champ absent de cette version du serveur : pas de ligne.
    fn toggle(&mut self, field: &str, label: &str, hint: &str) {
        if let Some(on) = jget(self.cfg, field).as_bool() {
            self.v.push(row(&format!("dash:t:{}:{field}", self.sec), label, hint, "", "toggle", on));
        }
    }
    /// Texte, nombre ou liste (séparée par des virgules) : fenêtre de saisie.
    fn text(&mut self, field: &str, label: &str, hint: &str) {
        let val = jget(self.cfg, field);
        if !val.is_null() || self.sec == "brand" {
            let shown = vtext(val);
            let shown = if shown.is_empty() { tr("Non défini").to_string() } else { shown.chars().take(40).collect() };
            self.v.push(row(&format!("dash:ask:f:{}:{field}", self.sec), label, hint, &shown, "action", false));
        }
    }
    /// Choix dans une liste.
    fn choice(&mut self, field: &str, label: &str, hint: &str, list: &[(&str, &str)]) {
        let val = jget(self.cfg, field);
        if val.is_null() {
            return;
        }
        let cur = vtext(val);
        let shown = list.iter().find(|(k, _)| *k == cur).map_or(cur.clone(), |(_, l)| l.to_string());
        let key = format!("dash:c:{}:{field}", self.sec);
        let owned = list.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect();
        self.app.dash_choices.lock().unwrap().insert(key.clone(), (label.to_string(), owned, cur));
        self.v.push(row(&key, label, hint, &shown, "choice", false));
    }
}

/// Champ d'un document changé : le document entier est relu puis renvoyé, seul ce champ change. La valeur
/// tapée prend le type qu'a le champ sur le serveur.
async fn set_field(c: &crate::api::Client, sec: &str, field: &str, text: &str) -> anyhow::Result<()> {
    let (get, post) = sec_paths(sec);
    let mut cfg = c.admin_get(get, &[]).await?;
    let new = match jget(&cfg, field) {
        Value::Bool(b) => Value::Bool(if text.is_empty() { !b } else { text == "true" }),
        Value::Number(_) => {
            let t = text.trim().replace(',', ".");
            if let Ok(i) = t.parse::<i64>() {
                i.into()
            } else {
                let f: f64 = t.parse().map_err(|_| anyhow::anyhow!("{}", tr("il faut un nombre")))?;
                f.into()
            }
        }
        // Liste : ses éléments gardent leur type (les largeurs des aperçus sont des nombres).
        Value::Array(old) => {
            let numbers = old.first().is_some_and(Value::is_number);
            let mut list = Vec::new();
            for t in text.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                if numbers {
                    list.push(t.parse::<i64>().map_err(|_| anyhow::anyhow!("{}", tr("il faut un nombre")))?.into());
                } else {
                    list.push(Value::String(t.to_string()));
                }
            }
            Value::Array(list)
        }
        _ => Value::String(text.to_string()),
    };
    jset(&mut cfg, field, new);
    c.admin_send(Method::POST, post, &[], Some(&cfg)).await
}

/// Valeur présente ou non dans un champ liste du document.
async fn toggle_member(c: &crate::api::Client, sec: &str, field: &str, member: &str) -> anyhow::Result<()> {
    let (get, post) = sec_paths(sec);
    let mut cfg = c.admin_get(get, &[]).await?;
    let mut list: Vec<Value> = jget(&cfg, field).as_array().cloned().unwrap_or_default();
    if list.iter().any(|x| x == member) {
        list.retain(|x| x != member);
    } else {
        list.push(member.into());
    }
    jset(&mut cfg, field, Value::Array(list));
    c.admin_send(Method::POST, post, &[], Some(&cfg)).await
}

/// Taille lisible.
fn size(n: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let n = n as f64;
    if n >= 1024.0 * GB { format!("{:.1} To", n / 1024.0 / GB) } else if n >= GB { format!("{:.1} Go", n / GB) } else { format!("{:.0} Mo", n / 1024.0 / 1024.0) }
}

/// Date et heure d'une tâche : « 3:00 » à partir de graduations (dix millions par seconde).
fn ticks_time(t: u64) -> String {
    let s = t / 10_000_000;
    format!("{}:{:02}", s / 3600, s / 60 % 60)
}

/// Image d'un média en cours de lecture : le fond de sa série ou le sien, sinon son affiche.
fn play_image(item: &Value) -> String {
    let id = s(item, "Id");
    let back = item["ParentBackdropItemId"].as_str().map(str::to_string).unwrap_or_else(|| id.clone());
    format!("/Items/{}/Images/Backdrop/0?maxWidth=480|/Items/{}/Images/Primary?maxWidth=480", pseg(&back), pseg(&id))
}

async fn fetch(app: &Arc<App>, c: &crate::api::Client, cat: i32, cards: &mut Vec<DashCard>) -> anyhow::Result<Vec<SettingRow>> {
    let armed = app.dash_armed.lock().unwrap().clone();
    // Second appui attendu : la ligne le dit.
    let confirm = |key: &str, label: &str| if armed == key { tr("Confirmer ?").to_string() } else { label.to_string() };
    let open = app.dash_open.lock().unwrap().clone();
    let mut v = Vec::new();
    let back = |title: &str| row("dash:back", &format!("‹  {title}"), tr("Retour à la liste."), tr("Retour"), "action", false);
    match cat {
        // Vue d'ensemble : l'essentiel, d'un coup d'œil.
        0 => {
            if let Some(sid) = open.strip_prefix("sess:") {
                let list = c.admin_get("/Sessions", &[("activeWithinSeconds", "960")]).await?;
                let Some(x) = list.as_array().into_iter().flatten().find(|x| s(x, "Id") == sid) else {
                    v.push(back(tr("Lectures en cours")));
                    v.push(info(tr("Cette lecture est terminée.")));
                    return Ok(v);
                };
                let item = &x["NowPlayingItem"];
                // Appareil sans lecture en cours : son nom.
                let title = if item.is_object() { play_title(item) } else { s(x, "DeviceName") };
                v.push(back(&title));
                if item.is_object() {
                    v.push(hero(&title, &join(&[s(x, "UserName"), s(x, "DeviceName")]), image_slot(app, c, &play_image(item)), false));
                }
                v.push(row("info", &format!("{} · {}", s(x, "UserName"), s(x, "Client")), &join(&[s(x, "DeviceName"), s(x, "ApplicationVersion"), s(x, "RemoteEndPoint")]), "", "info", false));
                if item.is_object() {
                    let t = &x["TranscodingInfo"];
                    if t.is_object() {
                        let reasons = vtext(&t["TranscodeReasons"]);
                        v.push(row("info", tr("Transcodage"), &join(&[s(t, "VideoCodec"), s(t, "AudioCodec"), s(t, "Container"), reasons]), "", "info", false));
                    }
                    let paused = x["PlayState"]["IsPaused"].as_bool().unwrap_or(false);
                    v.push(row(&format!("dash:sess:{sid}:{}", if paused { "Unpause" } else { "Pause" }), if paused { tr("Reprendre") } else { tr("Mettre en pause") }, "", "", "action", false));
                    v.push(row(&format!("dash:sess:{sid}:Stop"), tr("Arrêter la lecture"), "", "", "action", false));
                }
                v.push(row(&format!("dash:ask:msg:{sid}"), tr("Envoyer un message"), tr("Il s'affiche sur l'écran de cet appareil."), "", "action", false));
                return Ok(v);
            }
            let (i, sessions, tasks, libs, users, storage, activity) = tokio::join!(
                c.admin_get("/System/Info", &[]),
                c.admin_get("/Sessions", &[("activeWithinSeconds", "960")]),
                c.admin_get("/ScheduledTasks", &[("isHidden", "false")]),
                c.admin_get("/Library/VirtualFolders", &[]),
                c.admin_get("/Users", &[]),
                c.admin_get("/System/Info/Storage", &[]),
                c.admin_get("/System/ActivityLog/Entries", &[("startIndex", "0"), ("limit", "5")]),
            );
            let i = i?;
            let sessions = sessions.unwrap_or_default();
            let tasks = tasks.unwrap_or_default();
            let all: Vec<&Value> = sessions.as_array().into_iter().flatten().collect();
            let playing: Vec<&&Value> = all.iter().filter(|x| x["NowPlayingItem"].is_object()).collect();
            let running: Vec<&Value> = tasks.as_array().into_iter().flatten().filter(|t| s(t, "State") == "Running").collect();
            *app.dash_stats.lock().unwrap() = vec![
                (tr("Lectures en cours").to_string(), playing.len().to_string()),
                (tr("Appareils actifs").to_string(), all.len().to_string()),
                (tr("Bibliothèques").to_string(), libs.as_ref().ok().and_then(|l| l.as_array()).map_or(0, |a| a.len()).to_string()),
                (tr("Comptes").to_string(), users.as_ref().ok().and_then(|l| l.as_array()).map_or(0, |a| a.len()).to_string()),
                (tr("Tâches en cours").to_string(), running.len().to_string()),
            ];
            v.push(row("stats", "", "", "", "stats", false));
            if i["HasPendingRestart"].as_bool().unwrap_or(false) {
                v.push(info(tr("Un redémarrage est nécessaire pour appliquer des changements.")));
            }
            if i["HasUpdateAvailable"].as_bool().unwrap_or(false) {
                v.push(info(tr("Une mise à jour de Jellyfin est disponible.")));
            }
            v.push(head(tr("Lectures en cours")));
            let mut now = Vec::new();
            for x in &playing {
                let item = &x["NowPlayingItem"];
                let pos = x["PlayState"]["PositionTicks"].as_u64().unwrap_or(0) as f64;
                let len = item["RunTimeTicks"].as_u64().unwrap_or(0) as f64;
                let how = match x["PlayState"]["PlayMethod"].as_str() {
                    Some("Transcode") => tr("transcodage"),
                    Some("DirectStream") => tr("flux direct"),
                    _ => tr("lecture directe"),
                };
                let paused = if x["PlayState"]["IsPaused"].as_bool().unwrap_or(false) { tr("en pause") } else { "" };
                let times = format!("{} / {}", dur(pos as u64 / 10_000_000), dur(len as u64 / 10_000_000));
                let mut k = card(&format!("dash:open:sess:{}", s(x, "Id")), &play_title(item), &join(&[format!("{} · {}", s(x, "UserName"), s(x, "DeviceName")), times, how.to_string()]));
                k.badge = paused.into();
                k.progress = if len > 0.0 { (pos / len) as f32 } else { 0.0 };
                k.img = image_slot(app, c, &play_image(item));
                now.push(k);
            }
            grid(&mut v, cards, true, now);
            if playing.is_empty() {
                v.push(info(tr("Aucune lecture en cours.")));
            }
            let idle: Vec<&&Value> = all.iter().filter(|x| !x["NowPlayingItem"].is_object()).collect();
            if !idle.is_empty() {
                v.push(head(tr("Appareils actifs")));
                let list = idle.iter().map(|x| {
                    let mut k = card(&format!("dash:open:sess:{}", s(x, "Id")), &s(x, "DeviceName"), &join(&[s(x, "UserName"), s(x, "Client")]));
                    let user = s(x, "UserId");
                    if !user.is_empty() && x["UserPrimaryImageTag"].is_string() {
                        k.img = image_slot(app, c, &format!("/Users/{}/Images/Primary?maxWidth=200", pseg(&user)));
                    }
                    k.round = true;
                    k
                }).collect();
                grid(&mut v, cards, false, list);
            }
            if !running.is_empty() {
                v.push(head(tr("Tâches en cours")));
                for t in running {
                    let pct = t["CurrentProgressPercentage"].as_f64().unwrap_or(0.0);
                    let mut r = row(&format!("dash:taskstop:{}", s(t, "Id")), &s(t, "Name"), &format!("{pct:.0} %"), tr("Arrêter"), "bar", false);
                    r.progress = (pct / 100.0) as f32;
                    v.push(r);
                }
            }
            v.push(head(tr("Serveur")));
            let what = join(&[format!("Jellyfin {}", s(&i, "Version")), s(&i, "OperatingSystemDisplayName"), s(&i, "SystemArchitecture")]);
            v.push(hero(&s(&i, "ServerName"), &what, -1, false));
            v.push(row("dash:goto:1", tr("Réglages du serveur"), "", "", "action", false));
            v.push(row("dash:scanall", tr("Analyser toutes les bibliothèques"), tr("Cherche les nouveaux fichiers et ceux qui ont disparu."), tr("Lancer"), "action", false));
            v.push(row("dash:restart", tr("Redémarrer le serveur"), tr("Les lectures en cours s'arrêtent le temps du redémarrage."), &confirm("dash:restart", tr("Redémarrer")), "action", false));
            v.push(row("dash:shutdown", tr("Arrêter le serveur"), tr("Il faudra le rallumer sur la machine elle-même."), &confirm("dash:shutdown", tr("Arrêter")), "action", false));
            if let Ok(st) = storage {
                let mut bars: Vec<(String, &Value)> = Vec::new();
                for (k, l) in [("ProgramDataFolder", tr("Données du serveur")), ("CacheFolder", tr("Cache")), ("TranscodingTempFolder", tr("Transcodage"))] {
                    if st[k].is_object() {
                        bars.push((l.to_string(), &st[k]));
                    }
                }
                for l in st["Libraries"].as_array().into_iter().flatten() {
                    for f in l["Folders"].as_array().into_iter().flatten() {
                        bars.push((s(l, "Name"), f));
                    }
                }
                if !bars.is_empty() {
                    v.push(head(tr("Stockage")));
                }
                for (name, f) in bars {
                    let (used, free) = (f["UsedSpace"].as_u64().unwrap_or(0), f["FreeSpace"].as_u64().unwrap_or(0));
                    let mut r = row("bar", &name, &join(&[s(f, "Path"), trf("{} libres sur {}", &[&size(free), &size(used + free)])]), "", "bar", false);
                    r.progress = if used + free > 0 { used as f32 / (used + free) as f32 } else { 0.0 };
                    v.push(r);
                }
            }
            if let Ok(a) = activity {
                v.push(head(tr("Activité récente")));
                for x in a["Items"].as_array().into_iter().flatten() {
                    v.push(row("line", &s(x, "Name"), &when(&s(x, "Date")), "", "line", false));
                }
                v.push(row("dash:goto:13", tr("Voir toute l'activité"), "", "", "action", false));
            }
        }
        // Général.
        1 => {
            let cfg = c.admin_get("/System/Configuration", &[]).await?;
            let opts = c.admin_get("/Localization/Options", &[]).await.unwrap_or_default();
            let langs: Vec<(String, String)> = opts.as_array().into_iter().flatten().map(|o| (s(o, "Value"), s(o, "Name"))).collect();
            let list: Vec<(&str, &str)> = langs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            let mut f = Fields { app, v: &mut v, sec: "conf", cfg: &cfg };
            f.text("ServerName", tr("Nom du serveur"), tr("Le nom que voient les applis."));
            f.choice("UICulture", tr("Langue préférée"), tr("Langue du serveur : messages, activité, noms de tâches."), &list);
            f.head(tr("Connexion"));
            f.toggle("QuickConnectAvailable", tr("Connexion rapide"), tr("Se connecter sur un nouvel appareil avec un code à six chiffres."));
            f.toggle("AllowClientLogUpload", tr("Journaux envoyés par les applis"), tr("Les applis peuvent envoyer leurs journaux au serveur."));
            f.head(tr("Dossiers"));
            f.text("CachePath", tr("Dossier du cache"), tr("Fichiers temporaires du serveur (redémarrage nécessaire)."));
            f.text("MetadataPath", tr("Dossier des métadonnées"), tr("Images et informations téléchargées (redémarrage nécessaire)."));
            f.head(tr("Performances"));
            f.text("LibraryScanFanoutConcurrency", tr("Analyses en parallèle"), tr("0 : selon le processeur."));
            f.text("LibraryMetadataRefreshConcurrency", tr("Actualisations en parallèle"), tr("0 : selon le processeur."));
            f.text("ParallelImageEncodingLimit", tr("Images encodées en parallèle"), tr("0 : selon le processeur."));
            f.head(tr("Conservation"));
            f.text("ActivityLogRetentionDays", tr("Activité gardée (jours)"), "");
            f.text("LogFileRetentionDays", tr("Journaux gardés (jours)"), "");
            f.toggle("EnableSlowResponseWarning", tr("Signaler les réponses lentes"), tr("Écrit un avertissement dans le journal."));
            f.text("SlowResponseThresholdMs", tr("Réponse lente à partir de (ms)"), "");
        }
        // Apparence : ce que voient les applis web à la connexion.
        2 => {
            let cfg = c.admin_get("/Branding/Configuration", &[]).await?;
            let mut f = Fields { app, v: &mut v, sec: "brand", cfg: &cfg };
            f.text("LoginDisclaimer", tr("Message de connexion"), tr("Affiché sous le formulaire de connexion de l'interface web."));
            f.text("CustomCss", tr("Style personnalisé (CSS)"), tr("Appliqué à l'interface web du serveur."));
            f.toggle("SplashscreenEnabled", tr("Écran d'accueil de l'interface web"), tr("Affiche une image pendant le chargement."));
        }
        // Comptes.
        3 => {
            let me = c.user_id.clone();
            if let Some(id) = open.strip_prefix("user:") {
                let x = c.admin_get(&format!("/Users/{}", pseg(&id)), &[]).await?;
                let p = &x["Policy"];
                let b = |k: &str| p[k].as_bool().unwrap_or(false);
                v.push(back(&s(&x, "Name")));
                let img = if x["PrimaryImageTag"].is_string() { image_slot(app, c, &format!("/Users/{}/Images/Primary?maxWidth=200", pseg(&id))) } else { -1 };
                v.push(hero(&s(&x, "Name"), &if p["IsAdministrator"].as_bool().unwrap_or(false) { tr("Administrateur") } else { tr("Compte") }.to_string(), img, true));
                let mine = id == me;
                v.push(row(&format!("dash:ask:username:{id}"), tr("Nom du compte"), "", &s(&x, "Name"), "action", false));
                v.push(row(&format!("dash:ask:userpw:{id}"), tr("Mot de passe"), tr("Définit un nouveau mot de passe pour ce compte."), tr("Changer"), "action", false));
                if let Some(d) = x["LastActivityDate"].as_str() {
                    v.push(info(&trf("Dernière activité : {}", &[&when(d)])));
                }
                v.push(head(tr("Droits")));
                // Ses propres droits d'administrateur et son activation ne se changent pas d'ici
                // (on se fermerait la porte).
                let rights: [(&str, &str, &str, bool); 16] = [
                    ("IsDisabled", tr("Compte activé"), tr("Un compte désactivé ne peut plus se connecter."), true),
                    ("IsAdministrator", tr("Administrateur"), tr("Gère le serveur (tableau de bord, comptes, bibliothèques)."), false),
                    ("IsHidden", tr("Caché à la connexion"), tr("N'apparaît pas dans la liste de « Qui regarde ? »."), false),
                    ("EnableRemoteAccess", tr("Accès depuis Internet"), tr("Peut se connecter hors du réseau de la maison."), false),
                    ("EnableMediaPlayback", tr("Lecture des médias"), "", false),
                    ("EnableVideoPlaybackTranscoding", tr("Transcodage vidéo"), tr("Le serveur peut convertir la vidéo quand l'appareil ne la lit pas telle quelle."), false),
                    ("EnableAudioPlaybackTranscoding", tr("Transcodage audio"), "", false),
                    ("EnablePlaybackRemuxing", tr("Conversion du conteneur"), tr("Sans réencoder la vidéo ni le son."), false),
                    ("EnableContentDownloading", tr("Téléchargements"), tr("Peut télécharger les médias pour les regarder hors ligne."), false),
                    ("EnableContentDeletion", tr("Suppression de médias"), tr("Peut supprimer des fichiers du serveur."), false),
                    ("EnableCollectionManagement", tr("Gérer les collections"), "", false),
                    ("EnableSubtitleManagement", tr("Gérer les sous-titres"), "", false),
                    ("EnableLyricManagement", tr("Gérer les paroles"), "", false),
                    ("EnableRemoteControlOfOtherUsers", tr("Piloter les autres comptes"), "", false),
                    ("EnableSharedDeviceControl", tr("Piloter les appareils partagés"), "", false),
                    ("EnablePublicSharing", tr("Partage public"), "", false),
                ];
                for (key, label, hint, inverted) in rights {
                    let on = if inverted { !b(key) } else { b(key) };
                    if mine && (key == "IsDisabled" || key == "IsAdministrator") {
                        v.push(row("info", label, &join(&[hint.to_string(), tr("ce compte").to_string()]), if on { tr("Oui") } else { tr("Non") }, "info", false));
                    } else {
                        v.push(row(&format!("dash:policy:{id}:{key}"), label, hint, "", "toggle", on));
                    }
                }
                v.push(row(&format!("dash:ask:maxsess:{id}"), tr("Sessions simultanées au plus"), tr("0 : sans limite."), &vtext(&p["MaxActiveSessions"]), "action", false));
                v.push(row(&format!("dash:ask:bitrate:{id}"), tr("Débit maximal à distance (Mbit/s)"), tr("0 : sans limite."), &(p["RemoteClientBitrateLimit"].as_i64().unwrap_or(0) / 1_000_000).to_string(), "action", false));
                // Bibliothèques visibles.
                v.push(head(tr("Bibliothèques")));
                v.push(row(&format!("dash:policy:{id}:EnableAllFolders"), tr("Toutes les bibliothèques"), tr("Voit toutes les bibliothèques du serveur."), "", "toggle", b("EnableAllFolders")));
                if !b("EnableAllFolders") {
                    let enabled: Vec<String> = p["EnabledFolders"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(|t| t.replace('-', "").to_lowercase())).collect();
                    let list = c.admin_get("/Library/VirtualFolders", &[]).await?;
                    for l in list.as_array().into_iter().flatten() {
                        let lid = s(l, "ItemId");
                        v.push(row(&format!("dash:userlib:{id}:{lid}"), &s(l, "Name"), "", "", "toggle", enabled.contains(&lid.replace('-', "").to_lowercase())));
                    }
                }
                // Contrôle parental.
                v.push(head(tr("Contrôle parental")));
                let ratings = c.admin_get("/Localization/ParentalRatings", &[]).await.unwrap_or_default();
                let mut list: Vec<(String, String)> = vec![("".to_string(), tr("Aucune limite").to_string())];
                let mut seen = Vec::new();
                for r in ratings.as_array().into_iter().flatten() {
                    let score = r["RatingScore"]["score"].as_i64().or_else(|| r["Value"].as_i64());
                    if let Some(sc) = score {
                        if !seen.contains(&sc) {
                            seen.push(sc);
                            list.push((sc.to_string(), s(r, "Name")));
                        }
                    }
                }
                let cur = vtext(&p["MaxParentalRating"]);
                let shown = list.iter().find(|(k, _)| *k == cur).map_or(cur.clone(), |(_, l)| l.clone());
                let key = format!("dash:c:rating:{id}");
                app.dash_choices.lock().unwrap().insert(key.clone(), (tr("Classification maximale").to_string(), list, cur));
                v.push(row(&key, tr("Classification maximale"), tr("Les médias au-dessus sont cachés à ce compte."), &shown, "choice", false));
                v.push(row(&format!("dash:policy:{id}:BlockUnratedItems"), tr("Cacher les médias sans classification"), "", "", "toggle", b("BlockUnratedItems")));
                v.push(row(&format!("dash:ask:tags:{id}:BlockedTags"), tr("Tags bloqués"), tr("Séparés par des virgules."), &vtext(&p["BlockedTags"]), "action", false));
                if !mine {
                    let key = format!("dash:userdel:{id}");
                    v.push(head(tr("Suppression")));
                    v.push(row(&key, tr("Supprimer le compte"), tr("Ses favoris et son avancement sont perdus."), &confirm(&key, tr("Supprimer")), "action", false));
                }
            } else {
                v.push(row("dash:ask:newuser", tr("Nouveau compte"), tr("Crée un compte (sans mot de passe : à définir ensuite)."), tr("Créer"), "action", false));
                let list = c.admin_get("/Users", &[]).await?;
                let mut users = Vec::new();
                for x in list.as_array().into_iter().flatten() {
                    let id = s(x, "Id");
                    let p = &x["Policy"];
                    let seen = x["LastActivityDate"].as_str().map(|d| trf("vu le {}", &[&when(d)])).unwrap_or_default();
                    let mut k = card(&format!("dash:open:user:{id}"), &s(x, "Name"), &seen);
                    k.round = true;
                    if x["PrimaryImageTag"].is_string() {
                        k.img = image_slot(app, c, &format!("/Users/{}/Images/Primary?maxWidth=200", pseg(&id)));
                    }
                    if p["IsDisabled"].as_bool().unwrap_or(false) {
                        k.badge = tr("désactivé").into();
                        k.alert = true;
                    } else if p["IsAdministrator"].as_bool().unwrap_or(false) {
                        k.badge = tr("Administrateur").into();
                    } else if id == me {
                        k.badge = tr("ce compte").into();
                    }
                    users.push(k);
                }
                grid(&mut v, cards, false, users);
            }
        }
        // Bibliothèques : la liste, ou une bibliothèque ouverte.
        4 => {
            let list = c.admin_get("/Library/VirtualFolders", &[]).await?;
            if open == "newlib" {
                let (name, kind, path) = app.dash_newlib.lock().unwrap().clone();
                v.push(back(tr("Nouvelle bibliothèque")));
                v.push(row("dash:ask:nl:name", tr("Nom"), "", if name.is_empty() { tr("Non défini") } else { &name }, "action", false));
                let types = lib_types();
                let shown = types.iter().find(|(k, _)| *k == kind).map_or(String::new(), |(_, l)| l.clone());
                app.dash_choices.lock().unwrap().insert("dash:c:nl:type".to_string(), (tr("Type de contenu").to_string(), types, kind.clone()));
                v.push(row("dash:c:nl:type", tr("Type de contenu"), "", &shown, "choice", false));
                v.push(row("dash:ask:nl:path", tr("Dossier"), tr("Chemin sur le serveur."), if path.is_empty() { tr("Non défini") } else { &path }, "action", false));
                if !name.is_empty() && !path.is_empty() {
                    v.push(row("dash:libcreate", tr("Créer la bibliothèque"), tr("Elle est analysée aussitôt."), "", "action", false));
                }
            } else if let Some(name) = open.strip_prefix("lib:") {
                let Some(x) = list.as_array().into_iter().flatten().find(|x| s(x, "Name") == name) else {
                    v.push(back(tr("Bibliothèques")));
                    return Ok(v);
                };
                let id = s(x, "ItemId");
                v.push(back(name));
                v.push(hero(name, &lib_types().into_iter().find(|(k, _)| *k == s(x, "CollectionType")).map_or(String::new(), |(_, l)| l), image_slot(app, c, &format!("/Items/{}/Images/Primary?maxWidth=480", pseg(&id))), false));
                v.push(row(&format!("dash:scan:{id}"), tr("Analyser"), tr("Cherche les nouveaux fichiers et ceux qui ont disparu."), tr("Lancer"), "action", false));
                v.push(row(&format!("dash:ask:libname:{name}"), tr("Renommer"), "", name, "action", false));
                v.push(head(tr("Dossiers")));
                let paths: Vec<&str> = x["Locations"].as_array().into_iter().flatten().filter_map(|p| p.as_str()).collect();
                for p in &paths {
                    let key = format!("dash:libpathdel:{name}|{p}");
                    v.push(row(&key, p, "", &confirm(&key, tr("Retirer")), "action", false));
                }
                v.push(row(&format!("dash:ask:libpath:{name}"), tr("Ajouter un dossier"), tr("Chemin sur le serveur."), "", "action", false));
                v.push(head(tr("Options")));
                let o = &x["LibraryOptions"];
                for (k, l, h) in [
                    ("EnableRealtimeMonitor", tr("Surveillance en temps réel"), tr("Les changements de fichiers sont vus aussitôt.")),
                    ("EnableInternetProviders", tr("Informations depuis Internet"), ""),
                    ("SaveLocalMetadata", tr("Enregistrer les images avec les médias"), ""),
                    ("EnableEmbeddedTitles", tr("Préférer les titres intégrés aux fichiers"), ""),
                    ("EnableEmbeddedEpisodeInfos", tr("Préférer les informations d'épisode intégrées"), ""),
                    ("EnableAutomaticSeriesGrouping", tr("Regrouper les séries réparties en plusieurs dossiers"), ""),
                    ("EnableChapterImageExtraction", tr("Extraire les images de chapitres"), ""),
                    ("ExtractChapterImagesDuringLibraryScan", tr("Extraire les chapitres pendant l'analyse"), ""),
                    ("EnableTrickplayImageExtraction", tr("Créer les aperçus de la barre de temps"), ""),
                    ("ExtractTrickplayImagesDuringLibraryScan", tr("Créer les aperçus pendant l'analyse"), ""),
                    ("SaveTrickplayWithMedia", tr("Enregistrer les aperçus avec les médias"), ""),
                    ("EnableLUFSScan", tr("Mesurer le volume sonore"), ""),
                    ("SkipSubtitlesIfEmbeddedSubtitlesPresent", tr("Pas de sous-titres si le fichier en contient"), ""),
                    ("SaveSubtitlesWithMedia", tr("Enregistrer les sous-titres avec les médias"), ""),
                ] {
                    if let Some(on) = o[k].as_bool() {
                        v.push(row(&format!("dash:libopt:{id}:{k}"), l, h, "", "toggle", on));
                    }
                }
                let key = format!("dash:c:librefresh:{id}");
                let cur = vtext(&o["AutomaticRefreshIntervalDays"]);
                let days: Vec<(String, String)> = [("0", tr("Jamais")), ("30", tr("Tous les 30 jours")), ("60", tr("Tous les 60 jours")), ("90", tr("Tous les 90 jours"))].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
                let shown = days.iter().find(|(k, _)| *k == cur).map_or(cur.clone(), |(_, l)| l.clone());
                app.dash_choices.lock().unwrap().insert(key.clone(), (tr("Actualisation automatique").to_string(), days, cur));
                v.push(row(&key, tr("Actualisation automatique"), tr("Relit les informations des médias."), &shown, "choice", false));
                v.push(head(tr("Suppression")));
                let key = format!("dash:libdel:{name}");
                v.push(row(&key, tr("Supprimer la bibliothèque"), tr("Les fichiers restent sur le disque."), &confirm(&key, tr("Supprimer")), "action", false));
            } else {
                v.push(row("dash:scanall", tr("Analyser toutes les bibliothèques"), tr("Cherche les nouveaux fichiers et ceux qui ont disparu."), tr("Lancer"), "action", false));
                v.push(row("dash:open:newlib", tr("Nouvelle bibliothèque"), tr("Un nom, un type de contenu et un dossier."), tr("Créer"), "action", false));
                let mut libs = Vec::new();
                for x in list.as_array().into_iter().flatten() {
                    let paths: Vec<&str> = x["Locations"].as_array().into_iter().flatten().filter_map(|p| p.as_str()).collect();
                    let kind = s(x, "CollectionType");
                    let label = lib_types().into_iter().find(|(k, _)| *k == kind).map_or(kind, |(_, l)| l);
                    let mut k = card(&format!("dash:open:lib:{}", s(x, "Name")), &s(x, "Name"), &join(&[label, paths.first().unwrap_or(&"").to_string()]));
                    k.img = image_slot(app, c, &format!("/Items/{}/Images/Primary?maxWidth=480", pseg(&s(x, "ItemId"))));
                    libs.push(k);
                }
                grid(&mut v, cards, true, libs);
            }
        }
        // Affichage des bibliothèques.
        5 => {
            let cfg = c.admin_get("/System/Configuration", &[]).await?;
            let mut f = Fields { app, v: &mut v, sec: "conf", cfg: &cfg };
            f.toggle("EnableFolderView", tr("Vue par dossiers"), tr("Ajoute une bibliothèque qui montre les dossiers tels qu'ils sont sur le disque."));
            f.toggle("DisplaySpecialsWithinSeasons", tr("Épisodes spéciaux dans les saisons"), tr("Les épisodes spéciaux sont rangés dans la saison où ils ont été diffusés."));
            f.toggle("EnableGroupingMoviesIntoCollections", tr("Regrouper les films en collections"), tr("Les films d'une même collection n'apparaissent qu'une fois dans Films."));
            f.toggle("EnableGroupingShowsIntoCollections", tr("Regrouper les séries en collections"), "");
            f.toggle("EnableExternalContentInSuggestions", tr("Contenu externe dans les suggestions"), tr("Télévision en direct, par exemple."));
        }
        // Métadonnées.
        6 => {
            let cfg = c.admin_get("/System/Configuration", &[]).await?;
            let (cu, co) = tokio::join!(c.admin_get("/Localization/Cultures", &[]), c.admin_get("/Localization/Countries", &[]));
            let langs: Vec<(String, String)> = cu.unwrap_or_default().as_array().into_iter().flatten().map(|o| (s(o, "TwoLetterISOLanguageName"), s(o, "DisplayName"))).collect();
            let lands: Vec<(String, String)> = co.unwrap_or_default().as_array().into_iter().flatten().map(|o| (s(o, "TwoLetterISORegionName"), s(o, "DisplayName"))).collect();
            let (l1, l2): (Vec<(&str, &str)>, Vec<(&str, &str)>) = (langs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect(), lands.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect());
            let mut f = Fields { app, v: &mut v, sec: "conf", cfg: &cfg };
            f.choice("PreferredMetadataLanguage", tr("Langue des informations"), tr("Titres et résumés téléchargés."), &l1);
            f.choice("MetadataCountryCode", tr("Pays"), tr("Classifications et dates de sortie."), &l2);
            f.head(tr("Chapitres"));
            f.text("DummyChapterDuration", tr("Chapitres factices toutes les (s)"), tr("0 : aucun."));
            f.choice("ChapterImageResolution", tr("Taille des images de chapitres"), "", &[("MatchSource", tr("Celle de la vidéo")), ("P2160", "2160p"), ("P1440", "1440p"), ("P1080", "1080p"), ("P720", "720p"), ("P480", "480p"), ("P360", "360p"), ("P240", "240p"), ("P144", "144p")]);
        }
        // Fichiers NFO.
        7 => {
            let cfg = c.admin_get("/System/Configuration/xbmcmetadata", &[]).await?;
            let users = c.admin_get("/Users", &[]).await.unwrap_or_default();
            let mut names: Vec<(String, String)> = vec![("".to_string(), tr("Aucun").to_string())];
            names.extend(users.as_array().into_iter().flatten().map(|u| (s(u, "Id"), s(u, "Name"))));
            let list: Vec<(&str, &str)> = names.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            let mut f = Fields { app, v: &mut v, sec: "nfo", cfg: &cfg };
            f.choice("UserId", tr("Compte dont l'avancement est enregistré"), tr("Dates de lecture, vu ou non, favoris."), &list);
            f.choice("ReleaseDateFormat", tr("Format des dates"), "", &[("yyyy-MM-dd", "yyyy-MM-dd")]);
            f.toggle("SaveImagePathsInNfo", tr("Chemins des images dans les NFO"), "");
            f.toggle("EnablePathSubstitution", tr("Remplacer les chemins"), tr("Selon les substitutions du serveur."));
            f.toggle("EnableExtraThumbsDuplication", tr("Dupliquer les vignettes"), tr("Copie extrafanart dans extrathumbs."));
        }
        // Transcodage.
        8 => {
            let cfg = c.admin_get("/System/Configuration/encoding", &[]).await?;
            let mut f = Fields { app, v: &mut v, sec: "enc", cfg: &cfg };
            f.head(tr("Accélération matérielle"));
            f.choice("HardwareAccelerationType", tr("Accélération"), tr("Puce graphique utilisée pour convertir les vidéos."), &[("none", tr("Aucune")), ("amf", "AMD AMF"), ("nvenc", "NVIDIA NVENC"), ("qsv", "Intel Quick Sync"), ("vaapi", "VA-API"), ("rkmpp", "Rockchip MPP"), ("videotoolbox", "Apple VideoToolBox"), ("v4l2m2m", "V4L2")]);
            f.text("VaapiDevice", tr("Périphérique VA-API"), "");
            f.text("QsvDevice", tr("Périphérique Quick Sync"), "");
            f.toggle("EnableHardwareEncoding", tr("Encodage matériel"), "");
            f.toggle("AllowHevcEncoding", tr("Encodage HEVC"), "");
            f.toggle("AllowAv1Encoding", tr("Encodage AV1"), "");
            f.toggle("EnableDecodingColorDepth10Hevc", tr("Décodage HEVC 10 bits"), "");
            f.toggle("EnableDecodingColorDepth10Vp9", tr("Décodage VP9 10 bits"), "");
            f.toggle("PreferSystemNativeHwDecoder", tr("Préférer le décodeur du système"), "");
            f.head(tr("Formats décodés par la puce"));
            let on: Vec<String> = cfg["HardwareDecodingCodecs"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(str::to_string)).collect();
            for (code, label) in [("h264", "H.264"), ("hevc", "HEVC"), ("mpeg2video", "MPEG-2"), ("mpeg4", "MPEG-4"), ("vc1", "VC-1"), ("vp8", "VP8"), ("vp9", "VP9"), ("av1", "AV1")] {
                f.v.push(row(&format!("dash:m:enc:HardwareDecodingCodecs:{code}"), label, "", "", "toggle", on.iter().any(|x| x == code)));
            }
            f.head(tr("Qualité"));
            f.choice("EncoderPreset", tr("Vitesse de l'encodeur"), tr("Plus lent : de meilleures images pour le même débit."), &[("auto", tr("Automatique")), ("veryslow", "veryslow"), ("slower", "slower"), ("slow", "slow"), ("medium", "medium"), ("fast", "fast"), ("faster", "faster"), ("veryfast", "veryfast"), ("superfast", "superfast"), ("ultrafast", "ultrafast")]);
            f.text("H264Crf", tr("Qualité H.264 (CRF)"), tr("Plus bas : meilleure qualité."));
            f.text("H265Crf", tr("Qualité HEVC (CRF)"), tr("Plus bas : meilleure qualité."));
            f.text("EncodingThreadCount", tr("Fils d'encodage"), tr("-1 : automatique."));
            f.choice("DeinterlaceMethod", tr("Désentrelacement"), "", &[("yadif", "YADIF"), ("bwdif", "BWDIF")]);
            f.toggle("DeinterlaceDoubleRate", tr("Doubler la fréquence d'images"), "");
            f.head(tr("Mappage des tons (HDR)"));
            f.toggle("EnableTonemapping", tr("Mappage des tons"), tr("Convertit le HDR pour les écrans SDR."));
            f.choice("TonemappingAlgorithm", tr("Algorithme"), "", &[("none", tr("Aucun")), ("clip", "Clip"), ("linear", "Linear"), ("gamma", "Gamma"), ("reinhard", "Reinhard"), ("hable", "Hable"), ("mobius", "Mobius"), ("bt2390", "BT.2390")]);
            f.choice("TonemappingMode", tr("Mode"), "", &[("auto", tr("Automatique")), ("max", "MAX"), ("rgb", "RGB"), ("lum", "LUM"), ("itp", "ITP")]);
            f.choice("TonemappingRange", tr("Plage"), "", &[("auto", tr("Automatique")), ("tv", "TV"), ("pc", "PC")]);
            f.text("TonemappingDesat", tr("Désaturation"), "");
            f.text("TonemappingPeak", tr("Luminance de crête"), "");
            f.head(tr("Audio"));
            f.toggle("EnableAudioVbr", tr("Débit audio variable"), "");
            f.text("DownMixAudioBoost", tr("Amplification du mixage stéréo"), "");
            f.choice("DownMixStereoAlgorithm", tr("Mixage stéréo"), "", &[("None", tr("Aucun")), ("Dave750", "Dave750"), ("NightmodeDialogue", "Nightmode Dialogue"), ("Rfc7845", "RFC 7845"), ("Ac4", "AC-4")]);
            f.head(tr("Fichiers temporaires"));
            f.text("TranscodingTempPath", tr("Dossier du transcodage"), "");
            f.toggle("EnableThrottling", tr("Limiter la conversion"), tr("S'arrête en avance sur la lecture : moins de processeur."));
            f.text("ThrottleDelaySeconds", tr("Avance avant la limite (s)"), "");
            f.toggle("EnableSegmentDeletion", tr("Supprimer les segments déjà lus"), "");
            f.text("SegmentKeepSeconds", tr("Segments gardés (s)"), "");
            f.head(tr("Sous-titres"));
            f.toggle("EnableSubtitleExtraction", tr("Extraire les sous-titres à la demande"), "");
            f.text("SubtitleExtractionTimeoutMinutes", tr("Délai d'extraction (min)"), "");
            f.toggle("EnableFallbackFont", tr("Police de secours"), "");
            f.text("FallbackFontPath", tr("Dossier de la police de secours"), "");
            f.text("MaxMuxingQueueSize", tr("File d'attente de multiplexage"), "");
        }
        // Reprise de lecture.
        9 => {
            let cfg = c.admin_get("/System/Configuration", &[]).await?;
            let mut f = Fields { app, v: &mut v, sec: "conf", cfg: &cfg };
            f.text("MinResumePct", tr("Reprise à partir de (%)"), tr("Avant : le média est considéré comme non commencé."));
            f.text("MaxResumePct", tr("Reprise jusqu'à (%)"), tr("Après : le média est considéré comme vu."));
            f.text("MinResumeDurationSeconds", tr("Durée minimale pour reprendre (s)"), tr("Plus court : pas de reprise."));
            f.head(tr("Livres audio"));
            f.text("MinAudiobookResume", tr("Reprise à partir de (min)"), "");
            f.text("MaxAudiobookResume", tr("Reste au plus pour reprendre (min)"), "");
        }
        // Débit.
        10 => {
            let cfg = c.admin_get("/System/Configuration", &[]).await?;
            let mut f = Fields { app, v: &mut v, sec: "conf", cfg: &cfg };
            f.text("RemoteClientBitrateLimit", tr("Débit maximal à distance (bit/s)"), tr("Pour tous les appareils hors du réseau. 0 : sans limite."));
            f.text("InactiveSessionThreshold", tr("Session inactive après (min)"), tr("0 : jamais."));
        }
        // Aperçus de la barre de temps.
        11 => {
            let cfg = c.admin_get("/System/Configuration", &[]).await?;
            let mut f = Fields { app, v: &mut v, sec: "conf", cfg: &cfg };
            f.toggle("TrickplayOptions.EnableHwAcceleration", tr("Accélération matérielle"), "");
            f.toggle("TrickplayOptions.EnableHwEncoding", tr("Encodage matériel"), "");
            f.toggle("TrickplayOptions.EnableKeyFrameOnlyExtraction", tr("Seulement les images clés"), tr("Bien plus rapide, moins précis."));
            f.choice("TrickplayOptions.ScanBehavior", tr("Pendant l'analyse"), "", &[("NonBlocking", tr("En arrière-plan")), ("Blocking", tr("Attendre la fin"))]);
            f.choice("TrickplayOptions.ProcessPriority", tr("Priorité"), "", &[("Idle", tr("Très basse")), ("BelowNormal", tr("Basse")), ("Normal", tr("Normale")), ("AboveNormal", tr("Haute")), ("High", tr("Très haute"))]);
            f.text("TrickplayOptions.Interval", tr("Une image toutes les (ms)"), "");
            f.text("TrickplayOptions.WidthResolutions", tr("Largeurs des images"), tr("Séparées par des virgules."));
            f.text("TrickplayOptions.TileWidth", tr("Colonnes par planche"), "");
            f.text("TrickplayOptions.TileHeight", tr("Lignes par planche"), "");
            f.text("TrickplayOptions.Qscale", tr("Qualité (1 à 31)"), tr("Plus bas : meilleure qualité."));
            f.text("TrickplayOptions.JpegQuality", tr("Qualité JPEG (%)"), "");
            f.text("TrickplayOptions.ProcessThreads", tr("Fils utilisés"), tr("0 : automatique."));
        }
        // Appareils.
        12 => {
            let list = c.admin_get("/Devices", &[]).await?;
            if let Some(id) = open.strip_prefix("dev:") {
                let Some(x) = list["Items"].as_array().into_iter().flatten().find(|x| s(x, "Id") == id) else {
                    v.push(back(tr("Appareils")));
                    return Ok(v);
                };
                let name = if s(x, "CustomName").is_empty() { s(x, "Name") } else { s(x, "CustomName") };
                v.push(back(&name));
                v.push(row(&format!("dash:ask:devname:{id}"), tr("Nom affiché"), tr("Vide : le nom donné par l'appareil."), &s(x, "CustomName"), "action", false));
                v.push(row("info", tr("Appli"), &format!("{} {}", s(x, "AppName"), s(x, "AppVersion")), "", "info", false));
                v.push(row("info", tr("Dernier compte"), &join(&[s(x, "LastUserName"), when(&s(x, "DateLastActivity"))]), "", "info", false));
                let key = format!("dash:device:{id}");
                v.push(row(&key, tr("Supprimer l'appareil"), tr("Il devra se reconnecter."), &confirm(&key, tr("Supprimer")), "action", false));
            } else {
                let mut devs = Vec::new();
                for x in list["Items"].as_array().into_iter().flatten() {
                    let id = s(x, "Id");
                    let hint = join(&[s(x, "AppName"), s(x, "LastUserName"), when(&s(x, "DateLastActivity"))]);
                    // Celui-ci : le serveur range parfois les appareils sous son propre identifiant ; la clé
                    // d'accès, elle, est celle de cette connexion.
                    // (à défaut : même nom de machine, appli Turtlefin et dernier compte = celui-ci).
                    let mine = id == c.device_id
                        || (!c.token.is_empty() && s(x, "AccessToken") == c.token)
                        || (s(x, "AppName") == "Turtlefin" && s(x, "Name") == crate::api::device_name() && s(x, "LastUserId") == c.user_id);
                    let name = if s(x, "CustomName").is_empty() { s(x, "Name") } else { s(x, "CustomName") };
                    let mut k = card(&format!("dash:open:dev:{id}"), &name, &hint);
                    k.round = true;
                    if x["IconUrl"].as_str().is_some_and(|u| u.starts_with("http")) {
                        k.img = image_slot(app, c, &s(x, "IconUrl"));
                    }
                    if mine {
                        k.key = "line".into();
                        k.badge = tr("cet appareil").into();
                    }
                    devs.push(k);
                }
                grid(&mut v, cards, false, devs);
            }
        }
        // Activité récente du serveur.
        13 => {
            let list = c.admin_get("/System/ActivityLog/Entries", &[("startIndex", "0"), ("limit", "100")]).await?;
            for x in list["Items"].as_array().into_iter().flatten() {
                let hint = [when(&s(x, "Date")), s(x, "ShortOverview")].into_iter().filter(|t| !t.is_empty()).collect::<Vec<_>>().join(" · ");
                v.push(row("line", &s(x, "Name"), &hint, "", "line", false));
            }
            if v.is_empty() {
                v.push(info(tr("Aucune activité enregistrée.")));
            }
        }
        // Réseau.
        14 => {
            let cfg = c.admin_get("/System/Configuration/network", &[]).await?;
            let mut f = Fields { app, v: &mut v, sec: "net", cfg: &cfg };
            f.v.push(info(tr("Ces réglages prennent effet au redémarrage du serveur. Une erreur peut le rendre injoignable.")));
            f.head(tr("Serveur"));
            f.text("BaseUrl", tr("Préfixe d'adresse"), tr("Par exemple /jellyfin."));
            f.text("LocalNetworkAddresses", tr("Adresses d'écoute"), tr("Séparées par des virgules. Vide : toutes."));
            f.text("LocalNetworkSubnets", tr("Réseau local"), tr("Séparé par des virgules. Vide : celui de la machine."));
            f.text("InternalHttpPort", tr("Port HTTP"), "");
            f.text("PublicHttpPort", tr("Port HTTP public"), "");
            f.head("HTTPS");
            f.toggle("EnableHttps", tr("Activer HTTPS"), "");
            f.toggle("RequireHttps", tr("Exiger HTTPS"), "");
            f.text("InternalHttpsPort", tr("Port HTTPS"), "");
            f.text("PublicHttpsPort", tr("Port HTTPS public"), "");
            f.text("CertificatePath", tr("Certificat"), tr("Chemin du fichier .pfx sur le serveur."));
            f.head(tr("Accès"));
            f.toggle("EnableRemoteAccess", tr("Accès depuis Internet"), "");
            f.toggle("AutoDiscovery", tr("Découverte sur le réseau"), tr("Les applis trouvent le serveur toutes seules."));
            f.toggle("EnableUPnP", tr("Ouverture automatique du port (UPnP)"), "");
            f.toggle("EnableIPv4", "IPv4", "");
            f.toggle("EnableIPv6", "IPv6", "");
            f.text("KnownProxies", tr("Proxys connus"), tr("Séparés par des virgules."));
            f.text("PublishedServerUriBySubnet", tr("Adresses publiées"), tr("Séparées par des virgules."));
            f.head(tr("Filtre d'adresses à distance"));
            f.text("RemoteIPFilter", tr("Adresses"), tr("Séparées par des virgules."));
            f.toggle("IsRemoteIPFilterBlacklist", tr("Liste de blocage"), tr("Activé : ces adresses sont refusées ; sinon, elles seules sont acceptées."));
        }
        // Clés d'API : applications autorisées à parler au serveur sans compte ; création, révocation.
        15 => {
            v.push(row("dash:ask:newkey", tr("Nouvelle clé"), tr("Pour une application qui parle au serveur sans compte."), tr("Créer"), "action", false));
            let list = c.admin_get("/Auth/Keys", &[]).await?;
            for x in list["Items"].as_array().into_iter().flatten() {
                let token = s(x, "AccessToken");
                let shown: String = token.chars().take(6).collect();
                let hint = join(&[format!("{shown}…"), trf("créée le {}", &[&when(&s(x, "DateCreated"))])]);
                let key = format!("dash:keydel:{token}");
                v.push(row(&key, &s(x, "AppName"), &hint, &confirm(&key, tr("Révoquer")), "action", false));
            }
        }
        // Sauvegardes.
        16 => {
            v.push(row("dash:backup", tr("Créer une sauvegarde"), tr("Base de données et réglages, sur le serveur."), tr("Créer"), "action", false));
            let list = c.admin_get("/Backup", &[]).await?;
            let mut items: Vec<&Value> = list.as_array().into_iter().flatten().collect();
            items.sort_by_key(|b| std::cmp::Reverse(s(b, "DateCreated")));
            for x in items {
                let path = s(x, "Path");
                let key = format!("dash:restore:{path}");
                v.push(row(&key, &when(&s(x, "DateCreated")), &join(&[format!("Jellyfin {}", s(x, "ServerVersion")), path.clone()]), &confirm(&key, tr("Restaurer")), "action", false));
            }
        }
        // Journaux du serveur : la liste, ou les dernières lignes d'un journal ouvert.
        17 => {
            if let Some(name) = open.strip_prefix("log:") {
                v.push(back(name));
                let text = c.admin_text("/System/Logs/Log", &[("name", name)]).await?;
                let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
                // Les plus récentes en premier (150 au plus).
                for l in lines.iter().rev().take(150) {
                    let l = l.trim();
                    let (when_, rest) = l.split_once(']').map(|(a, b)| (a.trim_start_matches('['), b.trim())).unwrap_or(("", l));
                    v.push(row("line", &rest.chars().take(160).collect::<String>(), when_, "", "line", false));
                }
            } else {
                let list = c.admin_get("/System/Logs", &[]).await?;
                for x in list.as_array().into_iter().flatten() {
                    let n = x["Size"].as_u64().unwrap_or(0);
                    let hint = join(&[when(&s(x, "DateModified")), format!("{} Ko", n / 1024)]);
                    v.push(row(&format!("dash:open:log:{}", s(x, "Name")), &s(x, "Name"), &hint, tr("Lire"), "action", false));
                }
                if v.is_empty() {
                    v.push(info(tr("Aucun journal.")));
                }
            }
        }
        // Tâches planifiées : état, dernier résultat ; une tâche ouverte : lancer, déclencheurs.
        18 => {
            if let Some(id) = open.strip_prefix("task:") {
                let t = c.admin_get(&format!("/ScheduledTasks/{}", pseg(&id)), &[]).await?;
                v.push(back(&s(&t, "Name")));
                if !s(&t, "Description").is_empty() {
                    v.push(info(&s(&t, "Description")));
                }
                let running = s(&t, "State") == "Running";
                if running {
                    let pct = t["CurrentProgressPercentage"].as_f64().unwrap_or(0.0);
                    let mut r = row(&format!("dash:taskstop:{id}"), tr("Arrêter"), &format!("{pct:.0} %"), "", "bar", false);
                    r.progress = (pct / 100.0) as f32;
                    v.push(r);
                } else {
                    v.push(row(&format!("dash:task:{id}"), tr("Lancer maintenant"), "", "", "action", false));
                }
                let last = &t["LastExecutionResult"];
                if let Some(end) = last["EndTimeUtc"].as_str() {
                    v.push(info(&trf("Dernière fois : {} le {}", &[&task_status(&s(last, "Status")), &when(end)])));
                }
                v.push(head(tr("Déclencheurs")));
                for (i, tr_) in t["Triggers"].as_array().into_iter().flatten().enumerate() {
                    let what = match s(tr_, "Type").as_str() {
                        "DailyTrigger" => trf("Tous les jours à {}", &[&ticks_time(tr_["TimeOfDayTicks"].as_u64().unwrap_or(0))]),
                        "WeeklyTrigger" => trf("Le {} à {}", &[&s(tr_, "DayOfWeek"), &ticks_time(tr_["TimeOfDayTicks"].as_u64().unwrap_or(0))]),
                        "IntervalTrigger" => trf("Toutes les {} h", &[&(tr_["IntervalTicks"].as_u64().unwrap_or(0) / 36_000_000_000)]),
                        "StartupTrigger" => tr("Au démarrage du serveur").to_string(),
                        other => other.to_string(),
                    };
                    let key = format!("dash:trigdel:{id}:{i}");
                    v.push(row(&key, &what, "", &confirm(&key, tr("Retirer")), "action", false));
                }
                v.push(row(&format!("dash:ask:trigday:{id}"), tr("Ajouter : tous les jours"), tr("Heure, par exemple 3:30."), "", "action", false));
                v.push(row(&format!("dash:ask:trighours:{id}"), tr("Ajouter : toutes les…"), tr("Nombre d'heures."), "", "action", false));
                v.push(row(&format!("dash:trigboot:{id}"), tr("Ajouter : au démarrage du serveur"), "", "", "action", false));
            } else {
                let list = c.admin_get("/ScheduledTasks", &[("isHidden", "false")]).await?;
                let mut tasks: Vec<&Value> = list.as_array().into_iter().flatten().collect();
                tasks.sort_by_key(|t| (s(t, "Category"), s(t, "Name")));
                let mut last_cat = String::new();
                for t in tasks {
                    let id = s(t, "Id");
                    if s(t, "Category") != last_cat {
                        last_cat = s(t, "Category");
                        v.push(head(&last_cat));
                    }
                    let running = s(t, "State") == "Running";
                    let hint = if running {
                        trf("en cours ({} %)", &[&(t["CurrentProgressPercentage"].as_f64().unwrap_or(0.0) as i64)])
                    } else {
                        let last = &t["LastExecutionResult"];
                        match last["EndTimeUtc"].as_str() {
                            Some(end) => format!("{} · {}", task_status(&s(last, "Status")), when(end)),
                            None => String::new(),
                        }
                    };
                    v.push(row(&format!("dash:open:task:{id}"), &s(t, "Name"), &hint, tr("Gérer"), "action", false));
                }
            }
        }
        // Catalogue : extensions des dépôts du serveur pas encore installées ; installation.
        20 => {
            let installed: Vec<String> = c.admin_get("/Plugins", &[]).await?
                .as_array().into_iter().flatten().filter_map(|p| p["Id"].as_str().map(|i| i.replace('-', "").to_lowercase())).collect();
            let list = c.admin_get("/Packages", &[]).await?;
            let mut pkgs: Vec<&Value> = list.as_array().into_iter().flatten()
                .filter(|p| !installed.contains(&s(p, "guid").replace('-', "").to_lowercase()))
                .collect();
            pkgs.sort_by_key(|p| (s(p, "category"), s(p, "name")));
            let mut last_cat = String::new();
            let mut group = Vec::new();
            for x in pkgs {
                if s(x, "category") != last_cat {
                    grid(&mut v, cards, true, std::mem::take(&mut group));
                    last_cat = s(x, "category");
                    v.push(head(&last_cat));
                }
                let version = x["versions"].as_array().and_then(|v| v.first()).map(|v| s(v, "version")).unwrap_or_default();
                let key = format!("dash:install:{}:{}:{}", s(x, "guid"), version, s(x, "name"));
                let mut k = card(&key, &s(x, "name"), &join(&[version.clone(), s(x, "overview")]));
                k.contain = true;
                if x["imageUrl"].as_str().is_some_and(|u| u.starts_with("http")) {
                    k.img = image_slot(app, c, &s(x, "imageUrl"));
                }
                if armed == key {
                    k.badge = tr("Confirmer ?").into();
                    k.alert = true;
                }
                group.push(k);
            }
            grid(&mut v, cards, true, group);
            if v.is_empty() {
                v.push(info(tr("Toutes les extensions disponibles sont déjà installées.")));
            }
        }
        // Dépôts d'extensions.
        21 => {
            v.push(row("dash:ask:repo", tr("Ajouter un dépôt"), tr("Son nom, puis son adresse (manifest.json)."), tr("Ajouter"), "action", false));
            let list = c.admin_get("/Repositories", &[]).await?;
            for (i, x) in list.as_array().into_iter().flatten().enumerate() {
                let key = format!("dash:repodel:{i}");
                v.push(row(&key, &s(x, "Name"), &s(x, "Url"), &confirm(&key, tr("Retirer")), "action", false));
            }
        }
        // Extensions : activées ou non (le serveur doit redémarrer pour en tenir compte).
        _ => {
            let list = c.admin_get("/Plugins", &[]).await?;
            let pending = app.dash_plugins.lock().unwrap().clone();
            // Mise à jour en attente du redémarrage : le serveur annonce aussi l'ancienne version
            // (« remplacée »). Une seule carte, une seule page : la version à venir.
            let all: Vec<&Value> = list.as_array().into_iter().flatten().collect();
            let list: Vec<&Value> = all.iter().copied().filter(|x| s(x, "Status") != "Superseded" || !all.iter().any(|o| s(o, "Id") == s(x, "Id") && s(o, "Status") != "Superseded")).collect();
            if let Some(id) = open.strip_prefix("plugin:") {
                let Some(x) = list.iter().copied().find(|x| s(x, "Id") == id) else {
                    v.push(back(tr("Extensions")));
                    return Ok(v);
                };
                let version = s(x, "Version");
                v.push(back(&s(x, "Name")));
                let img = if x["HasImage"].as_bool().unwrap_or(false) { image_slot(app, c, &format!("/Plugins/{}/{}/Image", pseg(&id), pseg(&version))) } else { -1 };
                v.push(hero(&s(x, "Name"), &s(x, "Description"), img, false));
                let status = s(x, "Status");
                let on = pending.get(id).copied().unwrap_or(status == "Active");
                v.push(row("info", &format!("{} {version}", tr("Version")), &plugin_status(&status), "", "info", false));
                v.push(row(&format!("dash:plugin:{id}:{version}"), tr("Activée"), tr("Pris en compte au prochain redémarrage du serveur."), "", "toggle", on));
                let key = format!("dash:plugdel:{id}:{version}");
                v.push(row(&key, tr("Désinstaller"), tr("Supprimée au prochain redémarrage du serveur."), &confirm(&key, tr("Désinstaller")), "action", false));
                return Ok(v);
            }
            let mut plugins = Vec::new();
            for x in list.iter().copied() {
                let (id, status) = (s(x, "Id"), s(x, "Status"));
                // Changement demandé pendant cette session : il ne compte qu'au redémarrage du serveur
                // (le serveur continue d'annoncer l'ancien état d'ici là).
                let state = match pending.get(&id) {
                    Some(true) => tr("activée au prochain redémarrage").to_string(),
                    Some(false) => tr("désactivée au prochain redémarrage").to_string(),
                    None => plugin_status(&status),
                };
                let mut k = card(&format!("dash:open:plugin:{id}"), &s(x, "Name"), &join(&[s(x, "Version"), s(x, "Description")]));
                k.contain = true;
                if x["HasImage"].as_bool().unwrap_or(false) {
                    k.img = image_slot(app, c, &format!("/Plugins/{}/{}/Image", pseg(&id), pseg(&s(x, "Version"))));
                }
                k.alert = status != "Active" && !pending.contains_key(&id);
                // Extension fournie avec Jellyfin : ni désactivable ni désinstallable.
                if x["CanUninstall"].as_bool() == Some(false) {
                    k.key = "line".into();
                    k.badge = tr("intégrée à Jellyfin").into();
                    k.alert = false;
                } else if status != "Active" || pending.contains_key(&id) {
                    k.badge = state.into();
                }
                plugins.push(k);
            }
            grid(&mut v, cards, true, plugins);
            if v.is_empty() {
                v.push(info(tr("Aucune extension installée.")));
            }
        }
    }
    Ok(v)
}

/// Titre d'une lecture : « Série · Épisode ».
fn play_title(item: &Value) -> String {
    match item["SeriesName"].as_str() {
        Some(series) => format!("{series} · {}", s(item, "Name")),
        None => s(item, "Name"),
    }
}

/// Types de contenu d'une bibliothèque.
fn lib_types() -> Vec<(String, String)> {
    [("movies", tr("Films")), ("tvshows", tr("Séries")), ("music", tr("Musique")), ("musicvideos", tr("Clips")), ("homevideos", tr("Vidéos et photos")), ("books", tr("Livres")), ("mixed", tr("Films et séries mélangés"))]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

/// Ligne du tableau de bord activée (clé « dash:… »).
pub fn activate(app: &Arc<App>, key: &str) {
    let Some(client) = app.client() else { return };
    // Lignes sans action.
    if !key.starts_with("dash:") {
        return;
    }
    // Actions qui ne se rattrapent pas : un premier appui arme, le second exécute.
    let risky = matches!(key, "dash:restart" | "dash:shutdown")
        || ["dash:device:", "dash:userdel:", "dash:keydel:", "dash:install:", "dash:libdel:", "dash:libpathdel:", "dash:plugdel:", "dash:restore:", "dash:repodel:", "dash:trigdel:"].iter().any(|p| key.starts_with(p));
    // Sous-page : rien à envoyer au serveur.
    if let Some(what) = key.strip_prefix("dash:open:") {
        *app.dash_open.lock().unwrap() = what.to_string();
        app.dash_armed.lock().unwrap().clear();
        app.dash_top.store(true, Ordering::SeqCst);
        if what == "newlib" {
            *app.dash_newlib.lock().unwrap() = (String::new(), "movies".to_string(), String::new());
        }
        reload_current(app);
        return;
    }
    if key == "dash:back" {
        app.dash_open.lock().unwrap().clear();
        app.dash_armed.lock().unwrap().clear();
        app.dash_top.store(true, Ordering::SeqCst);
        reload_current(app);
        return;
    }
    if let Some(cat) = key.strip_prefix("dash:goto:") {
        if let Some(u) = app.ui().upgrade() {
            app.dash_open.lock().unwrap().clear();
            app.dash_top.store(true, Ordering::SeqCst);
            u.set_set_cat(cat.parse().unwrap_or(0));
            u.set_set_in(false);
            reload_current(app);
            crate::refresh_settings(app);
        }
        return;
    }
    if key.starts_with("dash:c:") {
        open_choice(app, key);
        return;
    }
    if let Some(what) = key.strip_prefix("dash:ask:") {
        ask(app, what);
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
        let saved = tr("Réglage du serveur enregistré.");
        let r: anyhow::Result<&'static str> = match parts.as_slice() {
            ["dash", "scanall"] => client.admin_send(Method::POST, "/Library/Refresh", &[], None).await.map(|_| tr("Analyse des bibliothèques lancée.")),
            ["dash", "restart"] => client.admin_send(Method::POST, "/System/Restart", &[], None).await.map(|_| tr("Le serveur redémarre.")),
            ["dash", "shutdown"] => client.admin_send(Method::POST, "/System/Shutdown", &[], None).await.map(|_| tr("Le serveur s'arrête.")),
            ["dash", "sess", id, cmd] => {
                let r = client.admin_send(Method::POST, &format!("/Sessions/{}/Playing/{}", pseg(&id), pseg(&cmd)), &[], None).await;
                r.map(|_| if *cmd == "Stop" { tr("Lecture arrêtée.") } else { tr("Commande envoyée.") })
            }
            ["dash", "scan", id] => client
                .admin_send(Method::POST, &format!("/Items/{}/Refresh", pseg(&id)), &[("Recursive", "true"), ("MetadataRefreshMode", "Default"), ("ImageRefreshMode", "Default")], None)
                .await
                .map(|_| tr("Analyse de la bibliothèque lancée.")),
            ["dash", "task", id] => client.admin_send(Method::POST, &format!("/ScheduledTasks/Running/{}", pseg(&id)), &[], None).await.map(|_| tr("Tâche lancée.")),
            ["dash", "taskstop", id] => client.admin_send(Method::DELETE, &format!("/ScheduledTasks/Running/{}", pseg(&id)), &[], None).await.map(|_| tr("Tâche arrêtée.")),
            ["dash", "device", id] => {
                let r = client.admin_send(Method::DELETE, "/Devices", &[("id", id)], None).await;
                if r.is_ok() {
                    app2.dash_open.lock().unwrap().clear();
                    app2.dash_top.store(true, Ordering::SeqCst);
                }
                r.map(|_| tr("Appareil supprimé."))
            }
            ["dash", "policy", id, field] => toggle_policy(&client, id, field).await,
            ["dash", "userlib", id, lib] => toggle_user_lib(&client, id, lib).await,
            ["dash", "t", sec, field] => set_field(&client, sec, field, "").await.map(|_| saved),
            ["dash", "m", sec, field, member] => toggle_member(&client, sec, field, member).await.map(|_| saved),
            ["dash", "libopt", id, field] => toggle_lib_opt(&client, id, field).await.map(|_| saved),
            ["dash", "install", guid, version, name @ ..] => {
                let name = name.join(":");
                client
                    .admin_send(Method::POST, &format!("/Packages/Installed/{}", urlenc(&name)), &[("assemblyGuid", guid), ("version", version)], None)
                    .await
                    .map(|_| tr("Installation lancée : l'extension sera active au prochain redémarrage du serveur."))
            }
            ["dash", "keydel", token] => client.admin_send(Method::DELETE, &format!("/Auth/Keys/{}", pseg(&token)), &[], None).await.map(|_| tr("Clé révoquée.")),
            ["dash", "userdel", id] => {
                let r = client.admin_send(Method::DELETE, &format!("/Users/{}", pseg(&id)), &[], None).await;
                if r.is_ok() {
                    app2.dash_open.lock().unwrap().clear();
                    app2.dash_top.store(true, Ordering::SeqCst);
                }
                r.map(|_| tr("Compte supprimé."))
            }
            ["dash", "plugin", id, version] => toggle_plugin(&app2, &client, id, version).await,
            ["dash", "plugdel", id, version] => {
                let r = client.admin_send(Method::DELETE, &format!("/Plugins/{}/{}", pseg(&id), pseg(&version)), &[], None).await;
                if r.is_ok() {
                    app2.dash_open.lock().unwrap().clear();
                    app2.dash_top.store(true, Ordering::SeqCst);
                }
                r.map(|_| tr("Extension désinstallée (au prochain redémarrage)."))
            }
            ["dash", "libdel", name] => {
                let r = client.admin_send(Method::DELETE, "/Library/VirtualFolders", &[("name", name), ("refreshLibrary", "true")], None).await;
                if r.is_ok() {
                    app2.dash_open.lock().unwrap().clear();
                    app2.dash_top.store(true, Ordering::SeqCst);
                    app2.home_stale.store(true, Ordering::SeqCst);
                }
                r.map(|_| tr("Bibliothèque supprimée."))
            }
            ["dash", "libcreate"] => {
                let (name, kind, path) = app2.dash_newlib.lock().unwrap().clone();
                let kind = if kind == "mixed" { String::new() } else { kind };
                let mut q = vec![("name", name.as_str()), ("paths", path.as_str()), ("refreshLibrary", "true")];
                if !kind.is_empty() {
                    q.push(("collectionType", kind.as_str()));
                }
                let r = client.admin_send(Method::POST, "/Library/VirtualFolders", &q, Some(&serde_json::json!({ "LibraryOptions": {} }))).await;
                if r.is_ok() {
                    app2.dash_open.lock().unwrap().clear();
                    app2.dash_top.store(true, Ordering::SeqCst);
                    app2.home_stale.store(true, Ordering::SeqCst);
                }
                r.map(|_| tr("Bibliothèque créée."))
            }
            ["dash", "backup"] => client
                .admin_send(Method::POST, "/Backup/Create", &[], Some(&serde_json::json!({ "Metadata": false, "Trickplay": false, "Subtitles": false, "Database": true })))
                .await
                .map(|_| tr("Sauvegarde créée.")),
            ["dash", "trigboot", id] => add_trigger(&client, id, serde_json::json!({ "Type": "StartupTrigger" })).await,
            ["dash", "trigdel", id, i] => del_trigger(&client, id, i.parse().unwrap_or(usize::MAX)).await,
            ["dash", "repodel", i] => del_repo(&client, i.parse().unwrap_or(usize::MAX)).await,
            _ => {
                // Clés dont la valeur peut contenir « : » (chemins).
                if let Some(path) = key.strip_prefix("dash:restore:") {
                    client.admin_send(Method::POST, "/Backup/Restore", &[], Some(&serde_json::json!({ "ArchiveFileName": path }))).await.map(|_| tr("Restauration lancée : le serveur redémarre."))
                } else if let Some((name, path)) = key.strip_prefix("dash:libpathdel:").and_then(|r| r.split_once('|')) {
                    client.admin_send(Method::DELETE, "/Library/VirtualFolders/Paths", &[("name", name), ("path", path), ("refreshLibrary", "true")], None).await.map(|_| tr("Dossier retiré."))
                } else {
                    return;
                }
            }
        };
        let msg = match r {
            Ok(m) => m.to_string(),
            Err(e) => trf("Action impossible : {}", &[&crate::human_err(&e)]),
        };
        // État relu (tâche lancée, appareil supprimé...) ; un instant pour que le serveur s'y mette.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        load(&app2, cat);
        let _ = app2.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
    });
}

/// Choix d'une ligne : la liste habituelle des Paramètres.
fn open_choice(app: &Arc<App>, key: &str) {
    let Some((title, list, cur)) = app.dash_choices.lock().unwrap().get(key).cloned() else { return };
    let Some(u) = app.ui().upgrade() else { return };
    let sel = list.iter().position(|(k, _)| *k == cur).unwrap_or(0) as i32;
    let rows: Vec<crate::TrackData> = list.iter().map(|(k, l)| crate::TrackData { id: k.clone().into(), label: l.clone().into(), current: *k == cur }).collect();
    u.set_ch_items(ModelRc::new(VecModel::from(rows)));
    u.set_ch_title(title.into());
    u.set_ch_sel(sel);
    u.set_ch_key(key.into());
}

/// Valeur choisie dans la liste d'une ligne.
pub fn chosen(app: &Arc<App>, key: &str, value: &str) {
    let parts: Vec<&str> = key.splitn(4, ':').collect();
    // Nouvelle bibliothèque : gardé jusqu'à la création.
    if key == "dash:c:nl:type" {
        app.dash_newlib.lock().unwrap().1 = value.to_string();
        reload_current(app);
        return;
    }
    let Some(client) = app.client() else { return };
    let (app2, value) = (app.clone(), value.to_string());
    let (sec, field) = (parts.get(2).unwrap_or(&"").to_string(), parts.get(3).unwrap_or(&"").to_string());
    let cat = app.ui().upgrade().map(|u| u.get_set_cat()).unwrap_or(0);
    app.rt.spawn(async move {
        let r = match sec.as_str() {
            "rating" => set_policy_value(&client, &field, "MaxParentalRating", if value.is_empty() { Value::Null } else { value.parse::<i64>().map_or(Value::Null, Value::from) }).await,
            "librefresh" => set_lib_opt(&client, &field, "AutomaticRefreshIntervalDays", value.parse::<i64>().unwrap_or(0).into()).await,
            _ => set_field(&client, &sec, &field, &value).await,
        };
        finish(&app2, cat, r.map(|_| tr("Réglage du serveur enregistré."))).await;
    });
}

async fn finish(app: &Arc<App>, cat: i32, r: anyhow::Result<&'static str>) {
    let msg = match r {
        Ok(m) => m.to_string(),
        Err(e) => trf("Action impossible : {}", &[&crate::human_err(&e)]),
    };
    load(app, cat);
    let _ = app.ui().upgrade_in_event_loop(move |u| u.set_toast(msg.into()));
}

/// Un droit d'un compte inversé : la politique entière est renvoyée, seul `field` change.
async fn toggle_policy(c: &crate::api::Client, id: &str, field: &str) -> anyhow::Result<&'static str> {
    let user = c.admin_get(&format!("/Users/{}", pseg(&id)), &[]).await?;
    let mut policy = user["Policy"].clone();
    let now = !policy[field].as_bool().unwrap_or(false);
    policy[field] = now.into();
    c.admin_send(Method::POST, &format!("/Users/{}/Policy", pseg(&id)), &[], Some(&policy)).await?;
    Ok(match (field, now) {
        ("IsDisabled", true) => tr("Compte désactivé."),
        ("IsDisabled", false) => tr("Compte activé."),
        _ => tr("Droit modifié."),
    })
}

async fn set_policy_value(c: &crate::api::Client, id: &str, field: &str, val: Value) -> anyhow::Result<()> {
    let user = c.admin_get(&format!("/Users/{}", pseg(&id)), &[]).await?;
    let mut policy = user["Policy"].clone();
    policy[field] = val;
    c.admin_send(Method::POST, &format!("/Users/{}/Policy", pseg(&id)), &[], Some(&policy)).await
}

/// Bibliothèque visible ou non d'un compte.
async fn toggle_user_lib(c: &crate::api::Client, id: &str, lib: &str) -> anyhow::Result<&'static str> {
    let user = c.admin_get(&format!("/Users/{}", pseg(&id)), &[]).await?;
    let mut policy = user["Policy"].clone();
    let norm = |t: &str| t.replace('-', "").to_lowercase();
    let mut list: Vec<Value> = policy["EnabledFolders"].as_array().cloned().unwrap_or_default();
    if list.iter().any(|x| x.as_str().is_some_and(|t| norm(t) == norm(lib))) {
        list.retain(|x| !x.as_str().is_some_and(|t| norm(t) == norm(lib)));
    } else {
        list.push(lib.into());
    }
    policy["EnabledFolders"] = Value::Array(list);
    c.admin_send(Method::POST, &format!("/Users/{}/Policy", pseg(&id)), &[], Some(&policy)).await?;
    Ok(tr("Droit modifié."))
}

/// Options d'une bibliothèque : relues, puis renvoyées avec ce champ changé.
async fn set_lib_opt(c: &crate::api::Client, id: &str, field: &str, val: Value) -> anyhow::Result<()> {
    let list = c.admin_get("/Library/VirtualFolders", &[]).await?;
    let x = list.as_array().into_iter().flatten().find(|x| s(x, "ItemId") == id).ok_or_else(|| anyhow::anyhow!("{}", tr("Bibliothèque introuvable.")))?;
    let mut o = x["LibraryOptions"].clone();
    o[field] = val;
    c.admin_send(Method::POST, "/Library/VirtualFolders/LibraryOptions", &[], Some(&serde_json::json!({ "Id": id, "LibraryOptions": o }))).await
}

async fn toggle_lib_opt(c: &crate::api::Client, id: &str, field: &str) -> anyhow::Result<()> {
    let list = c.admin_get("/Library/VirtualFolders", &[]).await?;
    let now = list.as_array().into_iter().flatten().find(|x| s(x, "ItemId") == id).and_then(|x| x["LibraryOptions"][field].as_bool()).unwrap_or(false);
    set_lib_opt(c, id, field, (!now).into()).await
}

/// Déclencheurs d'une tâche : la liste entière est renvoyée.
async fn triggers(c: &crate::api::Client, id: &str) -> anyhow::Result<Vec<Value>> {
    Ok(c.admin_get(&format!("/ScheduledTasks/{}", pseg(&id)), &[]).await?["Triggers"].as_array().cloned().unwrap_or_default())
}

async fn add_trigger(c: &crate::api::Client, id: &str, t: Value) -> anyhow::Result<&'static str> {
    let mut list = triggers(c, id).await?;
    list.push(t);
    c.admin_send(Method::POST, &format!("/ScheduledTasks/{}/Triggers", pseg(&id)), &[], Some(&Value::Array(list))).await?;
    Ok(tr("Déclencheur ajouté."))
}

async fn del_trigger(c: &crate::api::Client, id: &str, i: usize) -> anyhow::Result<&'static str> {
    let mut list = triggers(c, id).await?;
    if i < list.len() {
        list.remove(i);
    }
    c.admin_send(Method::POST, &format!("/ScheduledTasks/{}/Triggers", pseg(&id)), &[], Some(&Value::Array(list))).await?;
    Ok(tr("Déclencheur retiré."))
}

async fn del_repo(c: &crate::api::Client, i: usize) -> anyhow::Result<&'static str> {
    let mut list = c.admin_get("/Repositories", &[]).await?.as_array().cloned().unwrap_or_default();
    if i < list.len() {
        list.remove(i);
    }
    c.admin_send(Method::POST, "/Repositories", &[], Some(&Value::Array(list))).await?;
    Ok(tr("Dépôt retiré."))
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
    c.admin_send(Method::POST, &format!("/Plugins/{}/{}/{}", pseg(&id), pseg(&version), pseg(&what)), &[], None).await?;
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
    app.dash_open.lock().unwrap().clear();
    if let Some(u) = app.ui().upgrade() {
        u.set_dash_sub(false);
        u.set_dash_images(ModelRc::new(VecModel::<slint::Image>::default()));
        app.dash_img_ids.lock().unwrap().clear();
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

/// Ouvre la saisie d'un texte pour `what` (« newuser », « userpw:<id> », « servername », « title:<id> »).
pub fn ask(app: &Arc<App>, what: &str) {
    let Some(u) = app.ui().upgrade() else { return };
    let (title, label, password) = match what.split(':').next().unwrap_or("") {
        "newuser" => (tr("Nouveau compte"), tr("Nom du compte"), false),
        "userpw" => (tr("Nouveau mot de passe"), tr("Mot de passe (vide : aucun)"), true),
        "title" => (tr("Modifier le titre"), tr("Titre"), false),
        "msg" => (tr("Envoyer un message"), tr("Message"), false),
        "newkey" => (tr("Nouvelle clé"), tr("Nom de l'application"), false),
        "repo" => (tr("Ajouter un dépôt"), tr("Nom du dépôt"), false),
        "repourl" => (tr("Ajouter un dépôt"), tr("Adresse (manifest.json)"), false),
        "trigday" => (tr("Ajouter : tous les jours"), tr("Heure (3:30)"), false),
        "trighours" => (tr("Ajouter : toutes les…"), tr("Nombre d'heures"), false),
        "libpath" => (tr("Ajouter un dossier"), tr("Chemin sur le serveur"), false),
        "libname" => (tr("Renommer"), tr("Nom"), false),
        "username" => (tr("Nom du compte"), tr("Nom"), false),
        "devname" => (tr("Nom affiché"), tr("Nom"), false),
        "maxsess" => (tr("Sessions simultanées au plus"), tr("Nombre"), false),
        "bitrate" => (tr("Débit maximal à distance (Mbit/s)"), tr("Nombre"), false),
        "tags" => (tr("Tags bloqués"), tr("Séparés par des virgules."), false),
        "nl" => (tr("Nouvelle bibliothèque"), if what == "nl:path" { tr("Chemin sur le serveur") } else { tr("Nom") }, false),
        // Réglage d'un document : le libellé de sa ligne.
        "f" => {
            let label = u.get_set_rows().iter().find(|r| r.key.as_str() == format!("dash:ask:{what}")).map(|r| r.label.to_string()).unwrap_or_default();
            (tr("Modifier"), Box::leak(label.into_boxed_str()) as &'static str, false)
        }
        _ => return,
    };
    u.set_prompt_title(title.into());
    u.set_prompt_label(label.into());
    u.set_prompt_password(password);
    u.set_prompt_text("".into());
    u.set_prompt_key(what.into());
    u.set_prompt_open(true);
    // Valeur de départ : le nom actuel (serveur, média).
    let Some(client) = app.client() else { return };
    let (what, weak) = (what.to_string(), u.as_weak());
    app.rt.spawn(async move {
        let cur = match what.split_once(':') {
            Some(("title", id)) => client.admin_get(&format!("/Items/{}", pseg(&id)), &[("userId", &client.user_id)]).await.ok().map(|v| s(&v, "Name")),
            Some(("f", rest)) => match rest.split_once(':') {
                Some((sec, field)) => client.admin_get(sec_paths(sec).0, &[]).await.ok().map(|v| vtext(jget(&v, field))),
                None => None,
            },
            Some(("username", id)) => client.admin_get(&format!("/Users/{}", pseg(&id)), &[]).await.ok().map(|v| s(&v, "Name")),
            Some(("libname", name)) => Some(name.to_string()),
            _ => None,
        };
        if let Some(cur) = cur {
            let _ = weak.upgrade_in_event_loop(move |u| {
                if u.get_prompt_open() && u.get_prompt_text().is_empty() {
                    u.set_prompt_text(cur.into());
                    u.set_prompt_seed(u.get_prompt_seed() + 1);
                }
            });
        }
    });
}

/// Saisie validée : `what` comme pour `ask`, `text` le texte tapé.
pub fn answer(app: &Arc<App>, what: String, text: String) {
    let Some(client) = app.client() else { return };
    let text = text.trim().to_string();
    let app2 = app.clone();
    app.rt.spawn(async move {
        let r: anyhow::Result<&'static str> = match what.split_once(':').unwrap_or((what.as_str(), "")) {
            ("newuser", _) if !text.is_empty() => client
                .admin_send(Method::POST, "/Users/New", &[], Some(&serde_json::json!({ "Name": text })))
                .await
                .map(|_| tr("Compte créé.")),
            ("userpw", id) => client
                .admin_send(Method::POST, &format!("/Users/{}/Password", pseg(&id)), &[], Some(&serde_json::json!({ "NewPw": text, "ResetPassword": false })))
                .await
                .map(|_| tr("Mot de passe changé.")),
            ("f", rest) => match rest.split_once(':') {
                Some((sec, field)) => set_field(&client, sec, field, &text).await.map(|_| tr("Réglage du serveur enregistré.")),
                None => return,
            },
            ("msg", id) if !text.is_empty() => client
                .admin_send(Method::POST, &format!("/Sessions/{}/Message", pseg(&id)), &[], Some(&serde_json::json!({ "Header": "Turtlefin", "Text": text, "TimeoutMs": 10000 })))
                .await
                .map(|_| tr("Message envoyé.")),
            ("newkey", _) if !text.is_empty() => client.admin_send(Method::POST, "/Auth/Keys", &[("app", &text)], None).await.map(|_| tr("Clé créée.")),
            // Le nom d'abord : la seconde saisie demande l'adresse.
            ("repo", _) if !text.is_empty() => {
                let a3 = app2.clone();
                let _ = app2.ui().upgrade_in_event_loop(move |_| ask(&a3, &format!("repourl:{text}")));
                return;
            }
            ("repourl", name) if !text.is_empty() => async {
                let mut list = client.admin_get("/Repositories", &[]).await?.as_array().cloned().unwrap_or_default();
                list.push(serde_json::json!({ "Name": name, "Url": text, "Enabled": true }));
                client.admin_send(Method::POST, "/Repositories", &[], Some(&Value::Array(list))).await?;
                Ok(tr("Dépôt ajouté."))
            }
            .await,
            ("trigday", id) => match text.split_once(':').and_then(|(h, m)| Some((h.trim().parse::<u64>().ok()?, m.trim().parse::<u64>().ok()?))) {
                Some((h, m)) if h < 24 && m < 60 => add_trigger(&client, id, serde_json::json!({ "Type": "DailyTrigger", "TimeOfDayTicks": (h * 3600 + m * 60) * 10_000_000 })).await,
                _ => Err(anyhow::anyhow!("{}", tr("écrivez une heure, par exemple 3:30"))),
            },
            ("trighours", id) => match text.parse::<u64>() {
                Ok(h) if h > 0 => add_trigger(&client, id, serde_json::json!({ "Type": "IntervalTrigger", "IntervalTicks": h * 36_000_000_000u64 })).await,
                _ => Err(anyhow::anyhow!("{}", tr("il faut un nombre"))),
            },
            ("libpath", name) if !text.is_empty() => client
                .admin_send(Method::POST, "/Library/VirtualFolders/Paths", &[("refreshLibrary", "true")], Some(&serde_json::json!({ "Name": name, "Path": text })))
                .await
                .map(|_| tr("Dossier ajouté.")),
            ("libname", name) if !text.is_empty() => {
                let r = client.admin_send(Method::POST, "/Library/VirtualFolders/Name", &[("name", name), ("newName", &text), ("refreshLibrary", "true")], None).await;
                if r.is_ok() {
                    *app2.dash_open.lock().unwrap() = format!("lib:{text}");
                }
                r.map(|_| tr("Bibliothèque renommée."))
            }
            ("username", id) if !text.is_empty() => async {
                let mut user = client.admin_get(&format!("/Users/{}", pseg(&id)), &[]).await?;
                user["Name"] = text.clone().into();
                client.admin_send(Method::POST, "/Users", &[("userId", id)], Some(&user)).await?;
                Ok(tr("Compte renommé."))
            }
            .await,
            ("devname", id) => client.admin_send(Method::POST, "/Devices/Options", &[("id", id)], Some(&serde_json::json!({ "CustomName": text }))).await.map(|_| tr("Appareil renommé.")),
            ("maxsess", id) => match text.parse::<i64>() {
                Ok(n) => set_policy_value(&client, id, "MaxActiveSessions", n.into()).await.map(|_| tr("Droit modifié.")),
                Err(_) => Err(anyhow::anyhow!("{}", tr("il faut un nombre"))),
            },
            ("bitrate", id) => match text.replace(',', ".").parse::<f64>() {
                Ok(n) => set_policy_value(&client, id, "RemoteClientBitrateLimit", ((n * 1_000_000.0) as i64).into()).await.map(|_| tr("Droit modifié.")),
                Err(_) => Err(anyhow::anyhow!("{}", tr("il faut un nombre"))),
            },
            ("tags", rest) => match rest.split_once(':') {
                Some((id, field)) => {
                    let list: Vec<Value> = text.split(',').map(str::trim).filter(|t| !t.is_empty()).map(|t| Value::String(t.to_string())).collect();
                    set_policy_value(&client, id, field, Value::Array(list)).await.map(|_| tr("Droit modifié."))
                }
                None => return,
            },
            ("nl", part) => {
                let mut n = app2.dash_newlib.lock().unwrap();
                if part == "path" {
                    n.2 = text.clone();
                } else {
                    n.0 = text.clone();
                }
                drop(n);
                let a3 = app2.clone();
                let _ = app2.ui().upgrade_in_event_loop(move |_| reload_current(&a3));
                return;
            }
            ("title", id) if !text.is_empty() => async {
                // Fiche complète renvoyée, seul le titre change ; il est verrouillé pour qu'une
                // actualisation ne le remplace pas.
                let mut item = client.admin_get(&format!("/Items/{}", pseg(&id)), &[("userId", &client.user_id)]).await?;
                item["Name"] = text.clone().into();
                let mut locked: Vec<Value> = item["LockedFields"].as_array().cloned().unwrap_or_default();
                if !locked.iter().any(|f| f == "Name") {
                    locked.push("Name".into());
                }
                item["LockedFields"] = locked.into();
                client.admin_send(Method::POST, &format!("/Items/{}", pseg(&id)), &[], Some(&item)).await?;
                Ok(tr("Titre modifié."))
            }
            .await,
            _ => return,
        };
        let msg = match r {
            Ok(m) => m.to_string(),
            Err(e) => trf("Action impossible : {}", &[&crate::human_err(&e)]),
        };
        app2.home_stale.store(true, Ordering::SeqCst);
        let a3 = app2.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            u.set_toast(msg.into());
            if u.get_set_dash() {
                reload_current(&a3);
            }
        });
    });
}

