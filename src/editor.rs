//! Modifier un média (administrateurs) : informations (titre, résumé, dates, notes, genres, tags,
//! studios…) et images (affiche, fond, logo, bannière, vignette), comme « Modifier les métadonnées » et
//! « Modifier les images » de l'interface web de Jellyfin.
//!
//! Bouton « Modifier » (crayon) de la fiche : fenêtre `ed-open`, entièrement au clavier ou à la
//! télécommande (images en haut, informations dessous). Clic droit (ou appui long) sur l'affiche, le
//! logo, le titre ou le résumé : directement la saisie ou le choix d'image concernés (`ed-quick`).
//!
//! Les informations sont relues sur le serveur (fiche entière), modifiées puis renvoyées entières. Un
//! champ corrigé à la main est verrouillé (`LockedFields`) quand le serveur le permet : une actualisation
//! des informations ne le remplace plus.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use slint::{Model, ModelRc, VecModel};

use crate::i18n::{tr, trf};
use crate::{App, AppWindow, EdImg, EdRow};

/// Média en cours de modification.
struct State {
    id: String,
    kind: String,
    dto: Value,
    /// Choix d'une image ouvert : son type et l'adresse de chaque vignette (vide : action).
    pick_kind: String,
    pick: Vec<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Champs modifiables pour ce type de média : (clé du serveur, libellé, sorte).
fn fields(kind: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut v = vec![("Name", tr("Titre"), "text")];
    if matches!(kind, "Movie" | "Series" | "BoxSet") {
        v.push(("OriginalTitle", tr("Titre original"), "text"));
    }
    v.push(("ForcedSortName", tr("Titre pour le tri"), "text"));
    if kind == "Episode" {
        v.push(("ParentIndexNumber", tr("Saison n°"), "int"));
        v.push(("IndexNumber", tr("Épisode n°"), "int"));
    } else if kind == "Season" {
        v.push(("IndexNumber", tr("Saison n°"), "int"));
    }
    v.push(("Overview", tr("Résumé"), "long"));
    if matches!(kind, "Movie" | "Series") {
        v.push(("Taglines", tr("Slogan"), "tagline"));
    }
    v.push(("ProductionYear", tr("Année"), "int"));
    v.push(("PremiereDate", if kind == "Series" { tr("Première diffusion (AAAA-MM-JJ)") } else { tr("Date de sortie (AAAA-MM-JJ)") }, "date"));
    if kind == "Series" {
        v.push(("EndDate", tr("Fin de diffusion (AAAA-MM-JJ)"), "date"));
    }
    v.push(("CommunityRating", tr("Note du public (sur 10)"), "float"));
    if kind == "Movie" {
        v.push(("CriticRating", tr("Note des critiques (sur 100)"), "float"));
    }
    v.push(("OfficialRating", tr("Classification (ex. : FR-12)"), "text"));
    if kind != "Episode" && kind != "Season" {
        v.push(("Genres", tr("Genres"), "list"));
    }
    v.push(("Tags", tr("Tags"), "list"));
    if kind != "Season" {
        v.push(("Studios", tr("Studios"), "studios"));
    }
    v.push(("LockData", tr("Tout verrouiller (le serveur ne change plus rien)"), "toggle"));
    v
}

/// Types d'images proposés pour ce type de média : (type du serveur, libellé, vignette en largeur).
fn image_kinds(kind: &str) -> Vec<(&'static str, &'static str, bool)> {
    match kind {
        "Episode" => vec![("Primary", tr("Image de l'épisode"), true)],
        "Season" => vec![("Primary", tr("Affiche"), false), ("Backdrop", tr("Fond"), true), ("Banner", tr("Bannière"), true), ("Thumb", tr("Vignette"), true)],
        _ => vec![
            ("Primary", tr("Affiche"), false),
            ("Backdrop", tr("Fond"), true),
            ("Logo", tr("Logo"), true),
            ("Banner", tr("Bannière"), true),
            ("Thumb", tr("Vignette"), true),
        ],
    }
}

fn label_of(kind: &str, img: &str) -> String {
    image_kinds(kind).into_iter().find(|k| k.0 == img).map(|k| k.1.to_string()).unwrap_or_else(|| img.to_string())
}

/// Texte affiché (et valeur de départ de la saisie) d'un champ.
fn shown(dto: &Value, key: &str, sort: &str) -> String {
    let v = &dto[key];
    match sort {
        "list" => v.as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default(),
        "studios" => v.as_array().map(|a| a.iter().filter_map(|x| x["Name"].as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default(),
        "tagline" => v.as_array().and_then(|a| a.first()).and_then(|x| x.as_str()).unwrap_or("").to_string(),
        "date" => v.as_str().map(|s| s.chars().take(10).collect()).unwrap_or_default(),
        "toggle" => if v.as_bool().unwrap_or(false) { tr("Oui").to_string() } else { tr("Non").to_string() },
        "int" => v.as_i64().map(|n| n.to_string()).unwrap_or_default(),
        "float" => v.as_f64().map(|n| format!("{}", (n * 10.0).round() / 10.0)).unwrap_or_default(),
        _ => v.as_str().unwrap_or("").to_string(),
    }
}

/// Champs que le serveur sait verrouiller un par un (`LockedFields`).
fn lock_name(key: &str) -> Option<&'static str> {
    match key {
        "Name" => Some("Name"),
        "Overview" => Some("Overview"),
        "Genres" => Some("Genres"),
        "Tags" => Some("Tags"),
        "Studios" => Some("Studios"),
        "OfficialRating" => Some("OfficialRating"),
        _ => None,
    }
}

/// Écrit `text` dans le champ `key` de la fiche. Erreur : message pour l'utilisateur.
fn set_value(dto: &mut Value, key: &str, sort: &str, text: &str) -> Result<(), String> {
    let t = text.trim();
    let list = || t.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>();
    dto[key] = match sort {
        "list" => json!(list()),
        "studios" => json!(list().into_iter().map(|n| json!({ "Name": n })).collect::<Vec<_>>()),
        "tagline" => {
            if t.is_empty() {
                json!([])
            } else {
                json!([t])
            }
        }
        "date" if t.is_empty() => Value::Null,
        "date" => {
            let ok = t.len() == 10 && chrono_like(t);
            if !ok {
                return Err(tr("Date attendue sous la forme AAAA-MM-JJ.").to_string());
            }
            json!(format!("{t}T00:00:00.0000000Z"))
        }
        "int" if t.is_empty() => Value::Null,
        "int" => json!(t.parse::<i64>().map_err(|_| tr("Nombre entier attendu.").to_string())?),
        "float" if t.is_empty() => Value::Null,
        "float" => {
            let n = t.replace(',', ".").parse::<f64>().map_err(|_| tr("Nombre attendu.").to_string())?;
            json!(n)
        }
        _ if key == "Name" && t.is_empty() => return Err(tr("Le titre ne peut pas être vide.").to_string()),
        _ => json!(t),
    };
    if key == "ProductionYear" {
        if let Some(y) = dto[key].as_i64() {
            if !(1800..=2200).contains(&y) {
                return Err(tr("Année invalide.").to_string());
            }
        }
    }
    Ok(())
}

/// AAAA-MM-JJ plausible.
fn chrono_like(t: &str) -> bool {
    let p: Vec<&str> = t.split('-').collect();
    p.len() == 3
        && p[0].len() == 4
        && p.iter().all(|x| x.chars().all(|c| c.is_ascii_digit()))
        && (1..=12).contains(&p[1].parse::<u32>().unwrap_or(0))
        && (1..=31).contains(&p[2].parse::<u32>().unwrap_or(0))
}

fn rows(dto: &Value, kind: &str) -> Vec<EdRow> {
    fields(kind)
        .into_iter()
        .map(|(key, label, sort)| EdRow { key: key.into(), label: label.into(), value: shown(dto, key, sort).replace('\n', " ").into(), kind: sort.into() })
        .collect()
}

/// Étiquette (tag) de l'image actuelle d'un type, s'il y en a une.
fn image_tag(dto: &Value, img: &str) -> Option<String> {
    if img == "Backdrop" {
        return dto["BackdropImageTags"].as_array()?.first()?.as_str().map(String::from);
    }
    dto["ImageTags"][img].as_str().map(String::from)
}

/// Ouvre la modification du média `id`. `focus` : « » (fenêtre complète), « img:<type> » (choix d'une
/// image tout de suite) ou « row:<clé> » (saisie de ce champ tout de suite).
pub fn open(app: &Arc<App>, id: String, focus: &str) {
    if !app.admin.load(Ordering::SeqCst) {
        return;
    }
    let Some(client) = app.client() else { return };
    let full = focus.is_empty();
    if let Some(u) = app.ui().upgrade() {
        if full {
            u.set_ed_title("".into());
            u.set_ed_rows(ModelRc::new(VecModel::<EdRow>::default()));
            u.set_ed_imgs(ModelRc::new(VecModel::<EdImg>::default()));
            u.set_ed_zone(1);
            u.set_ed_rsel(0);
            u.set_ed_isel(0);
            u.set_ed_busy(true);
            u.set_ed_open(true);
        }
    }
    let (app2, focus) = (app.clone(), focus.to_string());
    app.rt.spawn(async move {
        let dto = match client.item_raw(&id).await {
            Ok(d) => d,
            Err(e) => {
                let msg = trf("Action impossible : {}", &[&crate::human_err(&e)]);
                let _ = app2.ui().upgrade_in_event_loop(move |u| {
                    u.set_ed_open(false);
                    u.set_ed_busy(false);
                    u.set_toast(msg.into());
                });
                return;
            }
        };
        let kind = dto["Type"].as_str().unwrap_or("").to_string();
        *STATE.lock().unwrap() = Some(State { id: id.clone(), kind: kind.clone(), dto: dto.clone(), pick_kind: String::new(), pick: Vec::new() });
        let a = app2.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            if full {
                show(&a, &u);
                return;
            }
            if let Some(img) = focus.strip_prefix("img:") {
                if image_kinds(&kind).iter().any(|k| k.0 == img) {
                    open_picker(&a, &u, img);
                }
            } else if let Some(key) = focus.strip_prefix("row:") {
                ask(&u, key);
            }
        });
    });
}

/// Remplit la fenêtre (titre, lignes, vignettes) avec l'état gardé, puis charge les vignettes.
fn show(app: &Arc<App>, u: &AppWindow) {
    let guard = STATE.lock().unwrap();
    let Some(st) = guard.as_ref() else { return };
    u.set_ed_title(st.dto["Name"].as_str().unwrap_or("").into());
    let new_rows = rows(&st.dto, &st.kind);
    let model = u.get_ed_rows();
    if model.row_count() == new_rows.len() {
        for (i, r) in new_rows.into_iter().enumerate() {
            model.set_row_data(i, r);
        }
    } else {
        u.set_ed_rows(ModelRc::new(VecModel::from(new_rows)));
    }
    let kinds = image_kinds(&st.kind);
    let old = u.get_ed_imgs();
    let old_tags = TAGS.lock().unwrap().clone();
    let tags: Vec<String> = kinds.iter().map(|(img, _, _)| image_tag(&st.dto, img).unwrap_or_default()).collect();
    let mut jobs: Vec<(usize, &'static str, String)> = Vec::new();
    let imgs: Vec<EdImg> = kinds
        .iter()
        .enumerate()
        .map(|(i, (img, label, wide))| {
            // Même image qu'avant (rien changé pour ce type) : la vignette reste.
            let keep = old.row_data(i).filter(|o| o.key.as_str() == *img && o.has_image && old_tags.get(i) == Some(&tags[i]));
            if keep.is_none() && !tags[i].is_empty() {
                jobs.push((i, *img, tags[i].clone()));
            }
            EdImg {
                key: (*img).into(),
                label: (*label).into(),
                image: keep.as_ref().map(|o| o.image.clone()).unwrap_or_default(),
                has_image: keep.is_some(),
                wide: *wide,
                ..Default::default()
            }
        })
        .collect();
    u.set_ed_imgs(ModelRc::new(VecModel::from(imgs)));
    u.set_ed_busy(false);
    let id = st.id.clone();
    drop(guard);
    *TAGS.lock().unwrap() = tags;
    let Some(client) = app.client() else { return };
    for (i, img, tag) in jobs {
        let (client, ui, id) = (client.clone(), app.ui(), id.clone());
        app.rt.spawn(async move {
            let cands = vec![(id, img, Some(tag.clone()))];
            let Some(buf) = crate::fetch_any(&client, cands, crate::api::Size::MaxWidth(420), None).await else { return };
            let _ = ui.upgrade_in_event_loop(move |u| {
                if TAGS.lock().unwrap().get(i).map(String::as_str) != Some(tag.as_str()) {
                    return;
                }
                let model = u.get_ed_imgs();
                let Some(mut e) = model.row_data(i) else { return };
                if e.key.as_str() != img {
                    return;
                }
                e.image = slint::Image::from_rgba8(buf);
                e.has_image = true;
                model.set_row_data(i, e);
            });
        });
    }
}

/// Étiquettes des images montrées dans la fenêtre (une vignette arrivée en retard pour une image
/// remplacée entre-temps est ignorée).
static TAGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Entrée (ou clic) sur une ligne de la fenêtre.
pub fn row(app: &Arc<App>, i: usize) {
    let Some(u) = app.ui().upgrade() else { return };
    let Some(r) = u.get_ed_rows().row_data(i) else { return };
    if r.kind.as_str() == "toggle" {
        let key = r.key.to_string();
        let on = {
            let guard = STATE.lock().unwrap();
            let Some(st) = guard.as_ref() else { return };
            st.dto[key.as_str()].as_bool().unwrap_or(false)
        };
        save(app, &key, if on { "" } else { "1" });
        return;
    }
    ask(&u, r.key.as_str());
}

/// Saisie d'un champ (fenêtre de saisie commune, clé « edit:<champ> »).
fn ask(u: &AppWindow, key: &str) {
    let guard = STATE.lock().unwrap();
    let Some(st) = guard.as_ref() else { return };
    let Some((_, label, sort)) = fields(&st.kind).into_iter().find(|f| f.0 == key) else { return };
    let value = shown(&st.dto, key, sort);
    let title = st.dto["Name"].as_str().unwrap_or("").to_string();
    drop(guard);
    u.set_prompt_title(title.into());
    u.set_prompt_label(match sort {
        "list" | "studios" => trf("{} (séparés par des virgules)", &[&label]),
        _ => label.to_string(),
    }.into());
    u.set_prompt_password(false);
    u.set_prompt_long(sort == "long");
    u.set_prompt_text(value.into());
    u.set_prompt_key(format!("edit:{key}").into());
    u.set_prompt_open(true);
    u.set_prompt_seed(u.get_prompt_seed() + 1);
}

/// Réponse de la saisie (`prompt-ok` avec une clé « edit:<champ> »).
pub fn answer(app: &Arc<App>, key: &str, text: &str) {
    save(app, key, text);
}

/// Écrit le champ, envoie la fiche entière, puis met la fenêtre et la page à jour.
fn save(app: &Arc<App>, key: &str, text: &str) {
    let Some(client) = app.client() else { return };
    let (id, dto) = {
        let mut guard = STATE.lock().unwrap();
        let Some(st) = guard.as_mut() else { return };
        let Some((_, _, sort)) = fields(&st.kind).into_iter().find(|f| f.0 == key) else { return };
        let mut dto = st.dto.clone();
        let r = if sort == "toggle" {
            dto[key] = json!(!text.is_empty());
            Ok(())
        } else {
            set_value(&mut dto, key, sort, text)
        };
        if let Err(msg) = r {
            drop(guard);
            if let Some(u) = app.ui().upgrade() {
                u.set_toast(msg.into());
            }
            return;
        }
        // Corrigé à la main : verrouillé (une actualisation des informations ne le remplace plus).
        if let Some(lock) = lock_name(key) {
            let mut locked: Vec<Value> = dto["LockedFields"].as_array().cloned().unwrap_or_default();
            if !locked.iter().any(|l| l.as_str() == Some(lock)) {
                locked.push(json!(lock));
            }
            dto["LockedFields"] = json!(locked);
        }
        (st.id.clone(), dto)
    };
    let app2 = app.clone();
    app.rt.spawn(async move {
        let r = client.item_save(&id, &dto).await;
        // Relue : le serveur a pu ajuster des valeurs (dates, titre de tri…).
        let fresh = match &r {
            Ok(()) => client.item_raw(&id).await.ok(),
            Err(_) => None,
        };
        if let Some(d) = fresh.clone().or_else(|| r.is_ok().then(|| dto.clone())) {
            if let Some(st) = STATE.lock().unwrap().as_mut().filter(|s| s.id == id) {
                st.dto = d;
            }
        }
        app2.home_stale.store(true, Ordering::SeqCst);
        let a = app2.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            match r {
                Ok(()) => {
                    if u.get_ed_open() {
                        show(&a, &u);
                    }
                    u.set_toast(tr("Enregistré.").into());
                    reload_page(&a, &id);
                }
                Err(e) => u.set_toast(trf("Action impossible : {}", &[&crate::human_err(&e)]).into()),
            }
        });
    });
}

/// Fiche affichée de ce média : rechargée sans repartir du haut.
fn reload_page(app: &Arc<App>, id: &str) {
    if app.stack.lock().unwrap().last().map(String::as_str) != Some(id) {
        return;
    }
    if let Some(u) = app.ui().upgrade() {
        u.set_keep_scroll(true);
    }
    crate::start_detail(app, id.to_string());
}

/// Entrée (ou clic) sur une vignette de la fenêtre : choix d'une image de ce type.
pub fn img(app: &Arc<App>, i: usize) {
    let Some(u) = app.ui().upgrade() else { return };
    let Some(e) = u.get_ed_imgs().row_data(i) else { return };
    open_picker(app, &u, e.key.as_str());
}

/// Ouvre le choix d'une image : « Depuis un fichier… » (bureau), « Retirer » (s'il y en a une), puis
/// celles des fournisseurs, vignettes chargées au fur et à mesure.
fn open_picker(app: &Arc<App>, u: &AppWindow, img: &str) {
    let (id, kind, has) = {
        let mut guard = STATE.lock().unwrap();
        let Some(st) = guard.as_mut() else { return };
        st.pick_kind = img.to_string();
        st.pick.clear();
        (st.id.clone(), st.kind.clone(), image_tag(&st.dto, img).is_some())
    };
    let wide = image_kinds(&kind).iter().find(|k| k.0 == img).is_some_and(|k| k.2);
    let mut items = Vec::new();
    let mut urls = Vec::new();
    if cfg!(not(target_os = "android")) {
        items.push(EdImg { key: "file".into(), label: tr("Depuis un fichier…").into(), wide, ..Default::default() });
        urls.push(String::new());
    }
    if has {
        items.push(EdImg { key: "remove".into(), label: tr("Retirer l'image").into(), wide, ..Default::default() });
        urls.push(String::new());
    }
    if let Some(st) = STATE.lock().unwrap().as_mut() {
        st.pick = urls;
    }
    let first = items.len();
    let name = STATE.lock().unwrap().as_ref().map(|s| s.dto["Name"].as_str().unwrap_or("").to_string()).unwrap_or_default();
    u.set_pk_title(trf("{} · {}", &[&label_of(&kind, img), &name]).into());
    u.set_pk_wide(wide);
    u.set_pk_items(ModelRc::new(VecModel::from(items)));
    u.set_pk_sel(0);
    u.set_pk_busy(true);
    u.set_pk_open(true);
    let Some(client) = app.client() else { return };
    let (app2, img) = (app.clone(), img.to_string());
    app.rt.spawn(async move {
        let list = client.remote_images(&id, &img).await.unwrap_or_default();
        let mut found: Vec<(String, String, String)> = Vec::new();
        for r in list.iter().take(60) {
            let Some(url) = r["Url"].as_str().filter(|u| u.starts_with("https://")) else { continue };
            let provider = r["ProviderName"].as_str().unwrap_or("");
            let lang = r["Language"].as_str().unwrap_or("");
            let label = [provider, lang].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
            let size = match (r["Width"].as_i64(), r["Height"].as_i64()) {
                (Some(w), Some(h)) if w > 0 && h > 0 => format!("{w} × {h}"),
                _ => String::new(),
            };
            found.push((url.to_string(), label, size));
        }
        let same = |st: &State| st.id == id && st.pick_kind == img;
        {
            let mut guard = STATE.lock().unwrap();
            let Some(st) = guard.as_mut().filter(|s| same(s)) else { return };
            st.pick.extend(found.iter().map(|f| f.0.clone()));
        }
        let previews: Vec<(usize, String)> = found.iter().enumerate().map(|(i, f)| (first + i, preview_url(&f.0))).collect();
        let none = found.is_empty();
        let ui = app2.ui();
        let _ = ui.upgrade_in_event_loop(move |u| {
            u.set_pk_busy(false);
            let model = u.get_pk_items();
            let Some(vm) = model.as_any().downcast_ref::<VecModel<EdImg>>() else { return };
            for (_, label, size) in found {
                vm.push(EdImg { key: "url".into(), label: label.into(), sub: size.into(), wide, ..Default::default() });
            }
            if none {
                u.set_toast(tr("Aucune image proposée par les fournisseurs du serveur.").into());
            }
        });
        // Vignettes : 6 à la fois.
        let permits = Arc::new(tokio::sync::Semaphore::new(6));
        for (i, url) in previews {
            let (client, ui, permits) = (client.clone(), app2.ui(), permits.clone());
            app2.rt.spawn(async move {
                let _p = permits.acquire().await.ok();
                let kind = if url.starts_with("https://image.tmdb.org/") { "Url" } else { "Pub" };
                let Some(buf) = crate::fetch_any(&client, vec![(url.clone(), kind, None)], crate::api::Size::MaxWidth(400), None).await else { return };
                let _ = ui.upgrade_in_event_loop(move |u| {
                    let model = u.get_pk_items();
                    let Some(mut e) = model.row_data(i) else { return };
                    if e.key.as_str() != "url" {
                        return;
                    }
                    e.image = slint::Image::from_rgba8(buf);
                    e.has_image = true;
                    model.set_row_data(i, e);
                });
            });
        }
    });
}

/// Vignette plus légère que l'image d'origine (TMDB : 300 px de large ; Fanart : son aperçu).
fn preview_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://image.tmdb.org/t/p/") {
        if let Some((_, path)) = rest.split_once('/') {
            return format!("https://image.tmdb.org/t/p/w300/{path}");
        }
    }
    url.replacen("https://assets.fanart.tv/fanart/", "https://assets.fanart.tv/preview/", 1)
}

/// Entrée (ou clic) sur une vignette du choix d'une image.
pub fn choose(app: &Arc<App>, i: usize) {
    let Some(u) = app.ui().upgrade() else { return };
    let Some(e) = u.get_pk_items().row_data(i) else { return };
    let Some(client) = app.client() else { return };
    let (id, img, url) = {
        let guard = STATE.lock().unwrap();
        let Some(st) = guard.as_ref() else { return };
        (st.id.clone(), st.pick_kind.clone(), st.pick.get(i).cloned().unwrap_or_default())
    };
    // Fichier de cette machine : choisi ici (fenêtre du système), envoyé ensuite.
    let file = if e.key.as_str() == "file" {
        match pick_file() {
            Some(f) => Some(f),
            None => return,
        }
    } else {
        None
    };
    u.set_pk_open(false);
    u.set_refocus(u.get_refocus() + 1);
    u.set_toast(tr("Envoi au serveur…").into());
    let key = e.key.to_string();
    let app2 = app.clone();
    app.rt.spawn(async move {
        let r: anyhow::Result<&'static str> = match key.as_str() {
            "remove" => client.image_delete(&id, &img, 0).await.map(|_| tr("Image retirée.")),
            "file" => {
                let path = file.unwrap_or_default();
                match tokio::fs::read(&path).await {
                    Ok(bytes) if bytes.len() <= 20 << 20 => {
                        let mime = mime_of(&path, &bytes);
                        match mime {
                            Some(m) => client.image_upload(&id, &img, &bytes, m).await.map(|_| tr("Image remplacée.")),
                            None => Err(anyhow::anyhow!("{}", tr("format d'image non reconnu (JPEG, PNG, WebP ou GIF)"))),
                        }
                    }
                    Ok(_) => Err(anyhow::anyhow!("{}", tr("image trop lourde (20 Mo au plus)"))),
                    Err(e) => Err(e.into()),
                }
            }
            _ if !url.is_empty() => client.remote_image_apply(&id, &img, &url).await.map(|_| tr("Image remplacée.")),
            _ => return,
        };
        let fresh = client.item_raw(&id).await.ok();
        if let Some(d) = fresh {
            if let Some(st) = STATE.lock().unwrap().as_mut().filter(|s| s.id == id) {
                st.dto = d;
            }
        }
        app2.home_stale.store(true, Ordering::SeqCst);
        let a = app2.clone();
        let _ = app2.ui().upgrade_in_event_loop(move |u| match r {
            Ok(msg) => {
                u.set_toast(msg.into());
                if u.get_ed_open() {
                    show(&a, &u);
                }
                reload_page(&a, &id);
            }
            Err(e) => u.set_toast(trf("Action impossible : {}", &[&crate::human_err(&e)]).into()),
        });
    });
}

/// Type d'image d'après les premiers octets (l'extension peut mentir).
fn mime_of(path: &str, b: &[u8]) -> Option<&'static str> {
    let _ = path;
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if b.starts_with(b"\x89PNG") {
        Some("image/png")
    } else if b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else if b.starts_with(b"GIF8") {
        Some("image/gif")
    } else {
        None
    }
}

#[cfg(not(target_os = "android"))]
fn pick_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter(tr("Images"), &["jpg", "jpeg", "png", "webp", "gif"])
        .pick_file()
        .map(|p| p.display().to_string())
}

#[cfg(target_os = "android")]
fn pick_file() -> Option<String> {
    None
}
