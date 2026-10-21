//! Lecture avec libmpv, intégrée à Turtlefin.
//!
//! mpv décode et dessine la vidéo dans une texture affichée par l'interface (voir video.rs) ;
//! les commandes (barre de temps, chapitres, épisodes, pistes) sont dessinées par Slint.
//! Turtlefin rapporte la lecture au serveur (début, progression toutes les 10 s, fin) et
//! enchaîne sur l'épisode suivant en fin de fichier.

use crate::i18n::{tr, trf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use slint::{Model, ModelRc, VecModel};

use crate::api::{Client, Item};
use crate::mpv::{Event, Mpv};
use crate::{api, video, App, AppWindow, CardData, TrackData};

/// Comment la lecture s'est terminée.
pub enum Exit {
    /// Retour à l'écran précédent (fiche).
    Back,
    /// « Retour à l'accueil » demandé depuis l'écran de fin.
    Home,
}

pub struct PlayRequest {
    /// Élément Jellyfin à lire (None : fichier de test local, voir `test_url`).
    pub item: Option<Item>,
    pub start_secs: f64,
    /// Lecture sans serveur : fichier local (téléchargement, ou option --test-video).
    pub test_url: Option<String>,
    /// Titre et sous-titre d'un fichier local.
    pub local_title: Option<(String, String)>,
    /// Sous-titres externes d'un fichier local : (chemin, langue).
    pub local_subs: Vec<(String, String)>,
    /// Watch party : lecture synchronisée (pause, reprise et sauts passent par le serveur).
    pub sync: bool,
}

/// Décodage matériel : variable TURTLEFIN_HWDEC pour forcer une valeur (ex. « auto-safe », « no »).
/// Sous Linux ARM 64 bits (Raspberry Pi), les images du décodeur matériel arrivent au format Broadcom
/// « SAND » que mpv ne sait pas importer en OpenGL (écran vide) : « drm-copy » les recopie en mémoire
/// au format habituel. Mesuré sur un Pi 5 (4K HEVC 10 bits, profil « fast ») : aucune image perdue au
/// lieu de 17 %, ~1 cœur au lieu de 4 ; image identique au pixel près. Le H.264 (pas de décodeur
/// matériel sur le Pi 5) et les machines sans DRM retombent seuls sur le logiciel.
/// Ailleurs (Windows, PC Linux) : « auto-safe ».
fn hwdec_mode() -> String {
    if let Ok(v) = std::env::var("TURTLEFIN_HWDEC") {
        return v;
    }
    if cfg!(target_os = "android") {
        // Android : décodeurs matériels du téléphone (MediaCodec), images recopiées pour OpenGL.
        "mediacodec-copy".to_string()
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "drm-copy".to_string()
    } else {
        "auto-safe".to_string()
    }
}

fn ticks(secs: f64) -> i64 {
    (secs.max(0.0) * 10_000_000.0) as i64
}

/// « 1:02:03 » ou « 2:03 ».
fn fmt_time(secs: f64) -> String {
    let t = secs.max(0.0) as u64;
    let (h, m, s) = (t / 3600, t % 3600 / 60, t % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Crée et configure le lecteur.
fn new_player() -> Result<Arc<Mpv>> {
    let m = Mpv::new()?;
    // Rendu dans la fenêtre de Turtlefin (API de rendu de libmpv).
    m.set_option("vo", "libmpv");
    m.set_option("hwdec", &hwdec_mode());
    // Le lecteur reste ouvert entre deux épisodes ; on ne lit ni mpv.conf ni les scripts de l'utilisateur.
    m.set_option("idle", "yes");
    // Volume de Turtlefin seulement (gardé d'une vidéo à l'autre). Android : mpv à 100 %, la barre du
    // lecteur règle le volume « média » du téléphone (un seul volume, celui des touches).
    m.set_option("volume", &app_volume().to_string());
    m.set_option("keep-open", "no");
    m.set_option("config", "no");
    m.set_option("terminal", "no");
    m.set_option("input-default-bindings", "no");
    m.set_option("osc", "no");
    m.set_option("ytdl", "no");
    // Scripts intégrés de mpv (console, statistiques, profils automatiques) : inutiles ici (pas de
    // raccourcis de mpv) ; non chargés, ils ne coûtent ni mémoire ni code exposé.
    m.set_option("load-stats-overlay", "no");
    m.set_option("load-console", "no");
    m.set_option("load-auto-profiles", "no");
    m.set_option("load-scripts", "no");
    // Linux ARM : rendu simplifié (mise à l'échelle bilinéaire...) pour ménager le GPU.
    if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        m.set_option("profile", "fast");
    }
    // Linux : sortie audio via PipeWire/PulseAudio, ALSA en dernier recours.
    // TURTLEFIN_AO=alsa (par ex.) pour choisir autre chose, TURTLEFIN_AO= (vide) pour ne rien imposer.
    #[cfg(target_os = "linux")]
    match std::env::var("TURTLEFIN_AO") {
        Ok(v) if v.is_empty() => {}
        Ok(v) => {
            m.set_option("ao", &v);
            m.set_option("audio-device", "auto");
        }
        Err(_) => {
            m.set_option("ao", "pipewire,pulse,alsa");
            m.set_option("audio-device", "auto");
        }
    }
    // Android : AudioTrack (machine virtuelle Java donnée à FFmpeg, voir mpv.rs), OpenSL ES en secours.
    #[cfg(target_os = "android")]
    m.set_option("ao", "audiotrack,opensles");
    // Android : rendu « simple » de mpv (sans passes intermédiaires). Mêlées au rendu de Skia (moteur de
    // Slint sous Android), celles-ci redonnaient une image noire quand mpv redessine l'image en pause.
    #[cfg(target_os = "android")]
    m.set_option("gpu-dumb-mode", "yes");
    // Coupure courte du réseau : ffmpeg se reconnecte tout seul et reprend au même octet. Réseau muet :
    // abandon au bout de 20 s (la lecture reprend alors quand le serveur répond, voir `play`).
    m.set_option("stream-lavf-o", "reconnect=1,reconnect_streamed=1,reconnect_on_network_error=1,reconnect_delay_max=10");
    m.set_option("network-timeout", "20");
    // Plafonne le cache réseau.
    m.set_option("demuxer-max-bytes", "100MiB");
    m.set_option("demuxer-max-back-bytes", "25MiB");
    // Diagnostic : TURTLEFIN_MPV_LOG=/chemin/mpv.log enregistre le journal détaillé de mpv.
    if let Ok(path) = std::env::var("TURTLEFIN_MPV_LOG") {
        m.set_option("log-file", &path);
        m.set_option("msg-level", "all=v");
    }
    // Essais : TURTLEFIN_MPV_ARGS="--hwdec=no --profile=fast" ajoute des options sans recompiler.
    if let Ok(extra) = std::env::var("TURTLEFIN_MPV_ARGS") {
        for a in extra.split_whitespace() {
            let a = a.trim_start_matches("--");
            match a.split_once('=') {
                Some((k, v)) => m.set_option(k, v),
                None => m.set_option(a, "yes"),
            }
        }
    }
    m.initialize()?;
    for p in ["time-pos", "duration", "pause", "chapter-list", "track-list", "paused-for-cache"] {
        m.observe(p);
    }
    Ok(Arc::new(m))
}

/// Pistes audio / sous-titres pour les menus. Renvoie aussi l'index de la piste active.
fn tracks(list: &Value, kind: &str) -> (Vec<TrackData>, i32) {
    let mut out: Vec<TrackData> = Vec::new();
    if kind == "sub" {
        out.push(TrackData { id: "no".into(), label: tr("Désactivés").into(), current: false });
    }
    for t in list.as_array().into_iter().flatten().filter(|t| t["type"] == kind) {
        let id = t["id"].as_i64().unwrap_or(0);
        let mut parts: Vec<String> = Vec::new();
        if let Some(l) = t["lang"].as_str().filter(|s| !s.is_empty()) {
            parts.push(l.to_uppercase());
        }
        // Sous-titres externes : le « titre » est souvent un bout d'URL, inutile à afficher.
        if let Some(s) = t["title"].as_str().filter(|s| !s.is_empty() && !s.contains(['/', '?'])) {
            parts.push(s.to_string());
        }
        if let Some(c) = t["codec"].as_str().filter(|s| !s.is_empty()) {
            parts.push(c.to_uppercase());
        }
        if kind == "audio" {
            if let Some(c) = t["demux-channel-count"].as_i64() {
                parts.push(format!("{c} canaux"));
            }
        }
        if t["external"].as_bool().unwrap_or(false) {
            parts.push("externe".to_string());
        }
        let label = if parts.is_empty() { trf("Piste {}", &[&id]) } else { parts.join(" · ") };
        out.push(TrackData {
            id: id.to_string().into(),
            label: label.into(),
            current: t["selected"].as_bool().unwrap_or(false),
        });
    }
    if kind == "sub" {
        let any = out.iter().skip(1).any(|t| t.current);
        out[0].current = !any;
    }
    let current = out.iter().position(|t| t.current).unwrap_or(0) as i32;
    (out, current)
}

/// Un fichier en cours de lecture et son rapport au serveur.
/// Vidéo passée à un écran de diffusion depuis le lecteur : (élément, position en secondes).
pub static HANDOFF: std::sync::Mutex<Option<(String, f64)>> = std::sync::Mutex::new(None);

struct Current {
    item: Option<Item>,
    ms_id: String,
    session: String,
    pos: f64,
    dur: f64,
    paused: bool,
    chapters: Vec<f64>,
    /// Titres des chapitres (repérage du générique de fin à défaut de segment).
    chapter_titles: Vec<(f64, String)>,
    /// Début du générique de fin (segment Jellyfin), si connu.
    outro: Option<f64>,
    /// Intro (début, fin) : bouton « Passer l'intro ».
    intro: Option<(f64, f64)>,
    intro_shown: bool,
    /// Propositions de fin déjà affichées pour ce fichier.
    up_shown: bool,
    /// Fin du fichier atteinte (mpv est au repos, en attente d'un choix).
    ended: bool,
    /// Sous-titres externes à ajouter une fois le fichier chargé : (url, langue).
    subs: Vec<(String, String)>,
    reported_stop: bool,
    /// Flux converti par le serveur (débit maximal) : rapports « Transcode », conversion arrêtée à la fin.
    transcode: bool,
}

impl Current {
    /// Moment où proposer la suite : segment « Outro », sinon chapitre de générique (par son nom, sinon
    /// par sa place), sinon les 3 dernières % de la durée (au moins 40 s).
    fn outro_at(&self) -> f64 {
        if let Some(o) = self.outro {
            return o;
        }
        let credits = ["ending", "credit", "générique", "generique", "outro", "end title"];
        if let Some((t, _)) = self.chapter_titles.iter().find(|(t, name)| {
            let n = name.to_lowercase();
            *t > self.dur * 0.5 && (credits.iter().any(|c| n.contains(c)) || n == "ed" || n.starts_with("ed "))
        }) {
            return *t;
        }
        // Chapitres sans nom (« (05)00:21:50:040 ») : le dernier qui commence dans les 5 dernières minutes,
        // au moins 45 s avant la fin, est le générique (l'aperçu de l'épisode suivant, plus court, vient
        // après ; la dernière scène, avant). Le plus sûr reste le plugin Intro Skipper du serveur (segments).
        if let Some(t) = self.chapters.iter().rev().find(|&&t| t > self.dur * 0.6 && self.dur - t <= 300.0 && self.dur - t >= 45.0) {
            return *t;
        }
        self.dur - (self.dur * 0.03).max(40.0)
    }

    fn body(&self, pos: f64) -> Option<Value> {
        let item = self.item.as_ref()?;
        Some(json!({
            "ItemId": item.id,
            "MediaSourceId": self.ms_id,
            "PlaySessionId": self.session,
            "PositionTicks": ticks(pos),
            "IsPaused": self.paused,
            "CanSeek": true,
            "PlayMethod": if self.transcode { "Transcode" } else { "DirectPlay" },
        }))
    }
}

async fn report(client: Option<&Client>, endpoint: &str, body: Option<Value>) {
    if let (Some(c), Some(b)) = (client, body) {
        let _ = c.report(endpoint, &b).await;
    }
}

/// Fin de lecture d'un fichier (rapport « Stopped » à la position `pos`) ; flux converti : le
/// serveur arrête aussi sa conversion (sur un petit serveur, elle occuperait le processeur pour rien).
async fn stopped(client: Option<&Client>, cur: &Current, pos: f64) {
    report(client, "/Sessions/Playing/Stopped", cur.body(pos)).await;
    if let (Some(c), true) = (client, cur.transcode) {
        c.stop_transcode(&cur.session).await;
    }
}

/// Liste « Qualité » du lecteur : débits proposés, le réglage en cours marqué.
pub fn quality_tracks() -> (Vec<TrackData>, i32) {
    let cur = crate::config::ui_prefs().max_bitrate;
    let list: Vec<TrackData> = crate::api::BITRATES
        .iter()
        .map(|&k| TrackData { id: k.to_string().into(), label: bitrate_label(k).into(), current: k == cur })
        .collect();
    let i = crate::api::BITRATES.iter().position(|&k| k == cur).unwrap_or(0) as i32;
    (list, i)
}

/// « Originale (sans limite) », « 8 Mbit/s · 1080p »...
pub fn bitrate_label(kbps: u32) -> String {
    if kbps == 0 {
        return tr("Originale (sans limite)").to_string();
    }
    let rate = if kbps % 1000 == 0 { format!("{}", kbps / 1000) } else { format!("{:.1}", kbps as f64 / 1000.0) };
    let (_, h) = crate::api::max_size(kbps);
    format!("{} · {}p", trf("{} Mbit/s", &[&rate]), h)
}

/// Liste des épisodes de la série (pour précédent / suivant et l'enchaînement).
pub async fn series_episodes(client: Option<&Client>, item: Option<&Item>) -> Vec<String> {
    match (client, item) {
        (Some(c), Some(it)) if it.kind == "Episode" => match it.series_id.as_deref() {
            Some(sid) => c.episodes(sid, None).await.map(|v| v.into_iter().map(|e| e.id).collect()).unwrap_or_default(),
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Lance la lecture et rend la main quand l'utilisateur la quitte.
pub async fn play(
    app: Arc<App>,
    client: Option<Client>,
    req: PlayRequest,
    mut commands: tokio::sync::mpsc::UnboundedReceiver<String>,
    ui: slint::Weak<AppWindow>,
) -> Result<Exit> {
    let player = new_player()?;
    let client = client.as_ref();

    // Événements de mpv : un thread dédié les attend et les transmet à la boucle ci-dessous.
    let (ev_tx, mut events) = tokio::sync::mpsc::unbounded_channel::<Event>();
    {
        let p = player.clone();
        std::thread::spawn(move || loop {
            match p.wait_event(-1.0) {
                Some(Event::Shutdown) => {
                    let _ = ev_tx.send(Event::Shutdown);
                    break;
                }
                Some(e) => {
                    let _ = ev_tx.send(e);
                }
                None => {}
            }
        });
    }
    let render_ready = video::attach(&ui, Some(player.clone()));

    let mut episodes = series_episodes(client, req.item.as_ref()).await;

    // mpv doit avoir son rendu (créé par l'interface au prochain affichage) avant d'ouvrir le fichier.
    if tokio::time::timeout(Duration::from_secs(5), render_ready).await.is_err() {
        eprintln!("turtlefin : rendu vidéo pas prêt après 5 s, lecture sans image");
    }

    let sync = req.sync;
    let sp = app.sp.clone();
    // Watch party : on démarre en pause et on attend la reprise commune.
    if sync {
        let _ = player.set_property("pause", "yes");
    }
    // Prêt à signaler au groupe (après le chargement ou un saut).
    let mut sp_wait_ready = sync;
    // Lecture d'un flux du serveur (pas d'un fichier local) : une fin prématurée est une coupure.
    let streaming = req.test_url.is_none() && client.is_some();
    // Connexion perdue : position où reprendre quand le serveur répondra de nouveau.
    let mut lost_at: Option<f64> = None;
    let mut cur = load(&player, client, req.item, req.start_secs, req.test_url.as_deref(), &episodes, &ui).await?;
    if let Some((t, s)) = req.local_title {
        let _ = ui.upgrade_in_event_loop(move |u| {
            u.set_p_title(t.into());
            u.set_p_subtitle(s.into());
        });
    }
    cur.subs.extend(req.local_subs);
    prepare_extras(&app, client, &mut cur).await;
    report(client, "/Sessions/Playing", cur.body(cur.pos)).await;

    let mut tick = tokio::time::interval(Duration::from_secs(10));
    tick.tick().await; // le premier tick est immédiat : on le consomme
    let mut last_sec: i64 = -1;
    let mut last_tracks = Value::Null;
    let mut switching = false;
    let mut result: Result<Exit> = Ok(Exit::Back);

    loop {
        // Changement d'épisode demandé (bouton, ou fin de fichier) : Some(décalage).
        let mut go_episode: Option<i64> = None;
        // Autre élément à lire (épisode choisi dans la saison, suggestion de fin).
        let mut go_item: Option<String> = None;
        // Position de départ imposée (watch party).
        let mut sp_start: Option<f64> = None;

        tokio::select! {
            ev = events.recv() => {
                let Some(ev) = ev else { break };
                match ev {
                    Event::Property(name, v) => match name.as_str() {
                        "time-pos" => {
                            cur.pos = v.as_f64().unwrap_or(cur.pos);
                            let sec = cur.pos as i64;
                            if sec != last_sec {
                                last_sec = sec;
                                push_time(&ui, &cur);
                                // Intro : bouton « Passer l'intro » 5 s au plus, dès qu'elle commence.
                                if let Some((s, e)) = cur.intro {
                                    if !cur.intro_shown && cur.pos >= s && cur.pos < e - 2.0 && crate::config::ui_prefs().auto_skip_intro && !sync {
                                        // Réglage « Passer l'intro automatiquement ».
                                        cur.intro_shown = true;
                                        let _ = player.command(&["seek", &format!("{e:.1}"), "absolute"]);
                                        let _ = ui.upgrade_in_event_loop(|u| u.set_toast(tr("Intro passée").into()));
                                    }
                                    if !cur.intro_shown && cur.pos >= s && cur.pos < e - 2.0 {
                                        cur.intro_shown = true;
                                        let _ = ui.upgrade_in_event_loop(|u| u.set_p_skip_intro(true));
                                        let ui2 = ui.clone();
                                        let hide_at = (5.0_f64).min(e - cur.pos);
                                        tokio::spawn(async move {
                                            tokio::time::sleep(Duration::from_secs_f64(hide_at.max(0.5))).await;
                                            let _ = ui2.upgrade_in_event_loop(|u| u.set_p_skip_intro(false));
                                        });
                                    }
                                }
                                if std::env::var_os("TURTLEFIN_DEBUG_UP").is_some() && sec % 5 == 0 {
                                    eprintln!("turtlefin : pos {:.0} / {:.0}, générique à {:.0}, déjà proposé {}, fini {}", cur.pos, cur.dur, cur.outro_at(), cur.up_shown, cur.ended);
                                }
                                // Générique de fin : on propose la suite (une fois par fichier).
                                if !cur.up_shown && !cur.ended && cur.dur > 120.0 && cur.pos >= cur.outro_at() && cur.pos < cur.dur - 2.0 {
                                    cur.up_shown = true;
                                    let next = neighbour(&episodes, cur.item.as_ref(), 1);
                                    show_up_next(&app, client, cur.item.as_ref(), next, false);
                                }
                            }
                        }
                        "paused-for-cache" => {
                            // Réseau lent : l'image s'arrête le temps de remplir le cache (indicateur).
                            let b = v.as_bool().unwrap_or(false);
                            let _ = ui.upgrade_in_event_loop(move |u| u.set_p_buffering(b));
                        }
                        "duration" => {
                            cur.dur = v.as_f64().unwrap_or(0.0);
                            push_time(&ui, &cur);
                            push_chapters(&ui, &cur);
                        }
                        "pause" => {
                            let p = v.as_bool().unwrap_or(false);
                            if p != cur.paused {
                                cur.paused = p;
                                report(client, "/Sessions/Playing/Progress", cur.body(cur.pos)).await;
                            }
                            let _ = ui.upgrade_in_event_loop(move |u| u.set_p_paused(p));
                        }
                        "chapter-list" => {
                            let list: Vec<(f64, String)> = v
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|c| Some((c["time"].as_f64()?, c["title"].as_str().unwrap_or("").to_string())))
                                .collect();
                            if std::env::var_os("TURTLEFIN_DEBUG_UP").is_some() {
                                eprintln!("turtlefin : chapitres {list:?}");
                            }
                            cur.chapters = list.iter().map(|(t, _)| *t).collect();
                            cur.chapter_titles = list;
                            push_chapters(&ui, &cur);
                        }
                        "track-list" => {
                            last_tracks = v.clone();
                            let (audio, audio_cur) = tracks(&v, "audio");
                            let (subs, sub_cur) = tracks(&v, "sub");
                            let _ = ui.upgrade_in_event_loop(move |u| {
                                u.set_audio_tracks(ModelRc::new(VecModel::from(audio)));
                                u.set_audio_current(audio_cur);
                                u.set_sub_tracks(ModelRc::new(VecModel::from(subs)));
                                u.set_sub_current(sub_cur);
                            });
                        }
                        _ => {}
                    },
                    Event::Restart => {
                        if sync && sp_wait_ready {
                            sp_wait_ready = false;
                            if let Some(c) = client {
                                let (c, sp, pos, playing) = (c.clone(), sp.clone(), cur.pos, !cur.paused);
                                tokio::spawn(async move {
                                    let _ = crate::syncplay::ready(&c, &sp, pos, playing).await;
                                });
                            }
                        }
                    }
                    Event::FileLoaded => {
                        for (url, lang) in std::mem::take(&mut cur.subs) {
                            let _ = player.command(&["sub-add", &url, "auto", "", &lang]);
                        }
                        let _ = ui.upgrade_in_event_loop(|u| u.set_p_ready(true));
                    }
                    Event::EndFile { eof, error } => {
                        // Fin avant la durée annoncée : coupure du réseau, ou durée du fichier fausse (souvent
                        // plus longue que la vidéo). Le serveur répond et le générique est atteint : c'est
                        // la vraie fin, pas une coupure.
                        let early = streaming && eof && lost_at.is_none() && cur.dur > 60.0 && cur.pos < cur.dur - 15.0;
                        let natural = early
                            && (cur.up_shown || cur.pos >= cur.outro_at() - 5.0 || cur.pos >= cur.dur * 0.9)
                            && match client {
                                Some(c) => tokio::time::timeout(Duration::from_secs(4), crate::discovery::reachable(&c.server)).await.unwrap_or(false),
                                None => false,
                            };
                        if switching {
                            // Fin de l'ancien fichier, provoquée par le changement d'élément.
                            switching = false;
                        } else if streaming && (lost_at.is_some() || (early && !natural)) {
                            // Le flux s'arrête bien avant la fin (ou la reprise a échoué) : c'est le réseau,
                            // pas la fin de l'épisode. Rien n'est marqué « vu » ; on reprend au même
                            // endroit dès que le serveur répond.
                            let at = lost_at.unwrap_or(cur.pos);
                            lost_at = Some(at);
                            eprintln!("turtlefin : flux interrompu à {at:.0} s ({error:?}), reprise dès le retour du serveur");
                            let _ = ui.upgrade_in_event_loop(|u| {
                                u.set_p_lost(true);
                                u.set_p_buffering(false);
                            });
                            if let Some(tx) = app.player_tx.lock().unwrap().clone() {
                                let a = app.clone();
                                tokio::spawn(async move {
                                    loop {
                                        tokio::time::sleep(Duration::from_secs(3)).await;
                                        if tx.is_closed() {
                                            return;
                                        }
                                        // L'adresse en vigueur, ou l'autre (bascule sur la secours).
                                        let ok = match a.client() {
                                            Some(c) => crate::discovery::reachable(&c.server).await || crate::check_address(&a).await,
                                            None => false,
                                        };
                                        if ok {
                                            let _ = tx.send("reload".to_string());
                                            return;
                                        }
                                    }
                                });
                            }
                        } else if let Some(e) = error {
                            result = Err(anyhow!("{}", trf("mpv n'a pas pu lire ce média ({})", &[&e])));
                            break;
                        } else if eof {
                            // Fin atteinte : on rapporte la durée complète (le serveur marque « vu »).
                            let end = if cur.dur > 0.0 { cur.dur } else { cur.pos };
                            stopped(client, &cur, end).await;
                            cur.reported_stop = true;
                            cur.ended = true;
                            if sync {
                                // Watch party : on laisse la suite au groupe (écran de fin pour choisir).
                                show_up_next(&app, client, cur.item.as_ref(), None, true);
                            } else if neighbour(&episodes, cur.item.as_ref(), 1).is_some() && autonext(&app) {
                                // Épisode suivant (ou premier de la saison suivante).
                                go_episode = Some(1);
                            } else if neighbour(&episodes, cur.item.as_ref(), 1).is_some() {
                                // Enchaînement coupé (réglage) : la carte « Épisode suivant » attend un choix.
                                let next = neighbour(&episodes, cur.item.as_ref(), 1);
                                if !cur.up_shown {
                                    cur.up_shown = true;
                                    show_up_next(&app, client, cur.item.as_ref(), next, false);
                                } else {
                                    let _ = ui.upgrade_in_event_loop(|u| u.set_p_up_mode("next".into()));
                                }
                            } else if cur.up_shown {
                                // Suggestions déjà chargées pendant le générique : écran de fin tout de suite.
                                let _ = ui.upgrade_in_event_loop(|u| u.set_p_up_mode("end".into()));
                            } else if client.is_some() && cur.item.is_some() {
                                // Fin de série ou de film : écran de fin (suggestions, retour à l'accueil).
                                show_up_next(&app, client, cur.item.as_ref(), None, true);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    Event::Shutdown => break,
                }
            }
            _ = tick.tick() => {
                if !cur.ended {
                    report(client, "/Sessions/Playing/Progress", cur.body(cur.pos)).await;
                }
            }
            cmd = commands.recv() => {
                let Some(c) = cmd else { break };
                let (verb, arg) = c.split_once(':').unwrap_or((c.as_str(), ""));
                // Watch party : pause, reprise et sauts passent par le serveur, qui les renvoie à tous.
                if sync && matches!(verb, "pause" | "seek" | "seek-to" | "chapter") {
                    if let Some(cl) = client {
                        let target = match verb {
                            "seek" => cur.pos + arg.parse::<f64>().unwrap_or(0.0),
                            "seek-to" => cur.dur * arg.parse::<f64>().unwrap_or(0.0).clamp(0.0, 1.0),
                            "chapter" => {
                                let d = arg.parse::<i64>().unwrap_or(1);
                                let i = cur.chapters.iter().rposition(|t| *t <= cur.pos + 0.5).map(|i| i as i64).unwrap_or(-1) + d;
                                usize::try_from(i).ok().and_then(|i| cur.chapters.get(i).copied()).unwrap_or(cur.pos)
                            }
                            _ => cur.pos,
                        };
                        let (cl, paused) = (cl.clone(), cur.paused);
                        let is_pause = verb == "pause";
                        tokio::spawn(async move {
                            let _ = if is_pause {
                                if paused { crate::syncplay::unpause(&cl).await } else { crate::syncplay::pause(&cl).await }
                            } else {
                                crate::syncplay::seek(&cl, target.max(0.0)).await
                            };
                        });
                    }
                    continue;
                }
                let r = match verb {
                    // Commandes du groupe (watch party), reçues du serveur.
                    "sp-unpause" => {
                        let (pos, delay) = arg.split_once(':').unwrap_or((arg, "0"));
                        let (pos, delay) = (pos.parse::<f64>().unwrap_or(cur.pos), delay.parse::<f64>().unwrap_or(0.0));
                        // Valeurs venues du serveur : bornées (« inf » ou « NaN » feraient planter l'attente).
                        let pos = if pos.is_finite() { pos.max(0.0) } else { cur.pos };
                        let delay = if delay.is_finite() { delay.clamp(-30.0, 10.0) } else { 0.0 };
                        // En retard : on rattrape ; en avance : on attend l'instant commun.
                        let at = pos + (-delay).max(0.0);
                        if (at - cur.pos).abs() > 0.5 {
                            let _ = player.command(&["seek", &format!("{at:.3}"), "absolute", "exact"]);
                        }
                        let p = player.clone();
                        tokio::spawn(async move {
                            if delay > 0.0 {
                                tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                            }
                            let _ = p.set_property("pause", "no");
                        });
                        Ok(())
                    }
                    "sp-pause" => {
                        let _ = player.set_property("pause", "yes");
                        match arg.parse::<f64>() {
                            Ok(p) if (p - cur.pos).abs() > 0.3 => player.command(&["seek", &format!("{p:.3}"), "absolute", "exact"]),
                            _ => Ok(()),
                        }
                    }
                    "sp-seek" => {
                        let _ = player.set_property("pause", "yes");
                        sp_wait_ready = true;
                        if let Some(cl) = client {
                            let (cl, sp, pos) = (cl.clone(), sp.clone(), cur.pos);
                            tokio::spawn(async move {
                                let _ = crate::syncplay::buffering(&cl, &sp, pos).await;
                            });
                        }
                        let p = arg.parse::<f64>().unwrap_or(cur.pos);
                        player.command(&["seek", &format!("{p:.3}"), "absolute", "exact"])
                    }
                    "sp-load" => {
                        let (id, start) = arg.split_once('@').unwrap_or((arg, "0"));
                        go_item = Some(id.to_string());
                        sp_start = start.parse().ok();
                        Ok(())
                    }
                    // Serveur de nouveau joignable après une coupure : le flux reprend où il s'était arrêté.
                    "reload" => match lost_at {
                        Some(at) => {
                            let keep = (cur.up_shown, cur.intro_shown);
                            // Adresse peut-être changée entre-temps (secours) : le client à jour.
                            let fresh = app.client();
                            let c = fresh.as_ref().or(client);
                            match load(&player, c, cur.item.clone(), at, None, &episodes, &ui).await {
                                Ok(n) => {
                                    cur = n;
                                    (cur.up_shown, cur.intro_shown) = keep;
                                    last_sec = -1;
                                    lost_at = None;
                                    prepare_extras(&app, client, &mut cur).await;
                                    report(client, "/Sessions/Playing", cur.body(cur.pos)).await;
                                    let _ = ui.upgrade_in_event_loop(|u| {
                                        u.set_p_lost(false);
                                        u.set_toast(tr("Connexion rétablie : la lecture reprend.").into());
                                    });
                                    Ok(())
                                }
                                Err(e) => Err(e),
                            }
                        }
                        None => Ok(()),
                    },
                    "pause" => player.command(&["cycle", "pause"]),
                    "seek" => player.command(&["seek", arg, "relative"]),
                    "seek-to" => {
                        let pct = arg.parse::<f64>().unwrap_or(0.0).clamp(0.0, 1.0) * 100.0;
                        player.command(&["seek", &format!("{pct:.3}"), "absolute-percent"])
                    }
                    "chapter" => player.command(&["add", "chapter", arg]),
                    "skip-intro" => {
                        let _ = ui.upgrade_in_event_loop(|u| u.set_p_skip_intro(false));
                        match cur.intro {
                            Some((_, e)) => player.command(&["seek", &format!("{e:.2}"), "absolute"]),
                            None => Ok(()),
                        }
                    }
                    "sub-margin" => player.set_property("sub-margin-y", arg),
                    "volume" => apply_volume(&player, arg.parse::<u32>().unwrap_or(100).min(100)),
                    // Télécommande : volume par pas, ou coupé / rétabli.
                    "volume-step" | "volume-mute" => {
                        let now = volume_shown();
                        let v = if verb == "volume-mute" { if now > 0 { 0 } else { 50 } } else { (now + arg.parse::<i32>().unwrap_or(0)).clamp(0, 100) };
                        let _ = ui.upgrade_in_event_loop(move |u| u.set_p_volume(v));
                        apply_volume(&player, v as u32)
                    }
                    // Télécommande : pause ou reprise imposée, saut à une position.
                    "pause-set" => player.set_property("pause", if arg == "yes" { "yes" } else { "no" }),
                    "seek-abs" => {
                        let t = arg.parse::<f64>().ok().filter(|t| t.is_finite()).unwrap_or(cur.pos).max(0.0);
                        player.command(&["seek", &format!("{t:.3}"), "absolute"])
                    }
                    // Barre du volume ouverte : Android relit le volume du téléphone (touches de volume).
                    "volume-poll" => {
                        #[cfg(target_os = "android")]
                        if let Some(v) = crate::android_entry::system_volume() {
                            let _ = ui.upgrade_in_event_loop(move |u| {
                                if u.get_p_volume() != v {
                                    u.set_p_volume(v);
                                }
                            });
                        }
                        Ok(())
                    }
                    // Changement de piste pendant la lecture : retenu pour la série / le film.
                    "aid" | "sid" => {
                        let r = player.set_property(verb, arg);
                        if let Some(it) = &cur.item {
                            let lang = if arg == "no" {
                                "off".to_string()
                            } else {
                                track_lang(&last_tracks, arg)
                            };
                            if verb == "aid" {
                                crate::config::set_track_pref(&it.pref_key(), Some(&lang), None);
                            } else {
                                crate::config::set_track_pref(&it.pref_key(), None, Some(&lang));
                            }
                        }
                        r
                    }
                    "episode" => {
                        go_episode = arg.parse().ok();
                        Ok(())
                    }
                    // Débit maximal changé pendant la lecture : enregistré, puis la vidéo repart au même endroit.
                    "quality" => {
                        let k = arg.parse::<u32>().unwrap_or(0);
                        let mut p = crate::config::ui_prefs();
                        if p.max_bitrate == k || cur.item.is_none() || cur.ended {
                            Ok(())
                        } else {
                            p.max_bitrate = k;
                            crate::config::save_ui_prefs(&p);
                            let keep = (cur.up_shown, cur.intro_shown, cur.paused);
                            let at = cur.pos;
                            stopped(client, &cur, at).await;
                            switching = true;
                            match load(&player, client, cur.item.clone(), at, None, &episodes, &ui).await {
                                Ok(n) => {
                                    cur = n;
                                    (cur.up_shown, cur.intro_shown) = (keep.0, keep.1);
                                    last_sec = -1;
                                    let _ = player.set_property("pause", if keep.2 { "yes" } else { "no" });
                                    prepare_extras(&app, client, &mut cur).await;
                                    report(client, "/Sessions/Playing", cur.body(cur.pos)).await;
                                    let label = bitrate_label(k);
                                    let _ = ui.upgrade_in_event_loop(move |u| u.set_toast(trf("Qualité : {}", &[&label]).into()));
                                    Ok(())
                                }
                                Err(e) => Err(e),
                            }
                        }
                    }
                    // Propositions de fin : lire la suite, ou continuer à regarder (générique).
                    "up" if arg == "play" => {
                        go_episode = Some(1);
                        Ok(())
                    }
                    "up" => {
                        let _ = ui.upgrade_in_event_loop(|u| u.set_p_up_mode("".into()));
                        if cur.ended {
                            // Plus rien à regarder : on quitte.
                            break;
                        }
                        Ok(())
                    }
                    "goto" | "pick" => {
                        go_item = Some(arg.to_string());
                        Ok(())
                    }
                    "home" => {
                        result = Ok(Exit::Home);
                        break;
                    }
                    "stop" => break,
                    // Diffuser : la vidéo passe à l'écran choisi, au même endroit ; la lecture ici s'arrête.
                    "handoff" => {
                        if let Some(it) = &cur.item {
                            *HANDOFF.lock().unwrap() = Some((it.id.clone(), cur.pos));
                        }
                        break;
                    }
                    _ => Ok(()),
                };
                if let Err(e) = r {
                    eprintln!("turtlefin : {e}");
                }
            }
        }

        // Watch party : un changement d'élément décidé ici (épisode suivant, suggestion) est
        // envoyé au groupe ; tout le monde (nous compris) le recevra par « sp-load ».
        if sync && sp_start.is_none() {
            let target = match (go_episode, &go_item) {
                (Some(d), _) => neighbour(&episodes, cur.item.as_ref(), d),
                (None, Some(id)) => Some(id.clone()),
                _ => None,
            };
            if let (Some(id), Some(cl)) = (target, client) {
                let cl = cl.clone();
                tokio::spawn(async move {
                    let it = match cl.item(&id).await {
                        Ok(it) => crate::resolve_playable(&cl, it).await.map(|t| t.id).unwrap_or(id),
                        Err(_) => id,
                    };
                    let _ = crate::syncplay::play(&cl, &it, 0.0).await;
                });
                continue;
            }
        }
        // Élément suivant à lire : épisode voisin, ou élément choisi.
        let next: Option<Result<Item>> = match (go_episode, go_item, client) {
            (Some(d), _, Some(c)) => match neighbour(&episodes, cur.item.as_ref(), d) {
                Some(id) => Some(c.item(&id).await.map_err(|e| anyhow!("{}", trf("épisode suivant introuvable ({})", &[&e])))),
                None => None,
            },
            (None, Some(id), Some(c)) => Some(match c.item(&id).await {
                Ok(it) => crate::resolve_playable(c, it).await,
                Err(e) => Err(anyhow!("{}", trf("élément introuvable ({})", &[&e]))),
            }),
            _ => None,
        };
        if let Some(next) = next {
            let next = match next {
                Ok(n) => n,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            };
            if !cur.reported_stop {
                stopped(client, &cur, cur.pos).await;
            }
            // Autre série (suggestion) : nouvelle liste d'épisodes.
            if !episodes.contains(&next.id) {
                episodes = series_episodes(client, Some(&next)).await;
            }
            let start = sp_start.unwrap_or_else(|| next.user_data.as_ref().map(|u| u.playback_position_ticks as f64 / 1e7).unwrap_or(0.0));
            if sync {
                let _ = player.set_property("pause", "yes");
                sp_wait_ready = true;
            } else {
                // Épisode choisi (bandeau, suivant, suggestion) : il démarre, même si l'on était en pause.
                let _ = player.set_property("pause", "no");
            }
            // L'ancien fichier se terminera (« stop ») sauf s'il était déjà fini.
            switching = !cur.ended;
            match load(&player, client, Some(next), start, None, &episodes, &ui).await {
                Ok(n) => {
                    cur = n;
                    last_sec = -1;
                    prepare_extras(&app, client, &mut cur).await;
                    report(client, "/Sessions/Playing", cur.body(cur.pos)).await;
                }
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
    }

    if !cur.reported_stop {
        stopped(client, &cur, cur.pos).await;
    }
    if let Some(at) = lost_at {
        cur.pos = at;
    }
    *app.last_play.lock().unwrap() = Some((cur.pos, cur.dur, cur.ended));
    let _ = ui.upgrade_in_event_loop(|u| {
        u.set_p_lost(false);
        u.set_p_buffering(false);
    });
    // Arrêt du lecteur : le thread d'événements se termine, puis l'interface libère le rendu
    // au prochain affichage, ce qui détruit le lecteur (et rend sa mémoire).
    let _ = player.command(&["quit"]);
    // Rendu détaché tout de suite ; l'avis de fin (Receiver) ne sert pas ici.
    drop(video::attach(&ui, None));
    result
}

/// Par fichier : début du générique (segments Jellyfin) et épisodes de la saison (bandeau ↓).
async fn prepare_extras(app: &Arc<App>, client: Option<&Client>, cur: &mut Current) {
    let (Some(c), Some(it)) = (client, cur.item.clone()) else { return };
    let (intro, outro) = c.segments(&it.id).await;
    cur.intro = intro;
    cur.outro = outro;

    let ui = app.ui();
    let _ = ui.upgrade_in_event_loop(|u| {
        u.set_p_episodes(ModelRc::default());
        u.set_p_up_mode("".into());
        u.set_p_skip_intro(false);
        u.set_p_has_bg(false);
    });
    // Fond de l'écran de fin : image du média, floutée et assombrie une fois.
    {
        let (ui2, c2, it2) = (ui.clone(), c.clone(), it.clone());
        app.rt.spawn(async move {
            let Some(bytes) = c2.backdrop(&it2).await else { return };
            let Some(buf) = tokio::task::spawn_blocking(move || crate::decode_backdrop(&bytes)).await.ok().flatten() else { return };
            let _ = ui2.upgrade_in_event_loop(move |u| {
                u.set_p_bg(slint::Image::from_rgba8(buf));
                u.set_p_has_bg(true);
            });
        });
    }
    if it.kind != "Episode" {
        return;
    }
    let (Some(sid), Some(season)) = (it.series_id.clone(), it.season_id.clone()) else { return };
    let (app2, c2) = (app.clone(), c.clone());
    app.rt.spawn(async move {
        let eps = c2.episodes(&sid, Some(&season)).await.unwrap_or_default();
        let cur_idx = eps.iter().position(|e| e.id == it.id).unwrap_or(0) as i32;
        let cards: Vec<api::CardInfo> = eps.iter().map(|e| e.child_card()).collect();
        let jobs: Vec<crate::ImageJob> =
            cards.iter().enumerate().filter_map(|(i, c)| crate::ImageJob::for_card(0, i, c, true)).collect();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            u.set_p_episodes(ModelRc::new(VecModel::from(cards.iter().map(card_data).collect::<Vec<_>>())));
            u.set_p_ep_current(cur_idx);
        });
        let k = if app2.tv() { 1.4 } else { 1.0 };
        let apply: crate::Apply = Arc::new(|u: &AppWindow, job: &crate::ImageJob, buf: slint::SharedPixelBuffer<slint::Rgba8Pixel>| {
            let model = u.get_p_episodes();
            if let Some(mut card) = model.row_data(job.b) {
                if card.id.as_str() == job.item_id {
                    card.image = slint::Image::from_rgba8(buf);
                    card.has_image = true;
                    model.set_row_data(job.b, card);
                }
            }
        });
        crate::spawn_image_jobs(&app2, &c2, jobs, api::Size::Fill(400, 225), crate::Shape::card_top(400, 225, 220.0 * k), apply);
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
        ..Default::default()
    }
}

/// Réglage du compte « Épisode suivant automatique » (activé par défaut).
fn autonext(app: &Arc<App>) -> bool {
    app.user_cfg.lock().unwrap()["EnableNextEpisodeAutoPlay"].as_bool().unwrap_or(true)
}

/// Propositions de fin :
/// - un épisode suit : « Épisode suivant » (ou « Saison suivante » s'il ouvre une autre saison) ;
/// - sinon (fin de série, film) : 3 titres au hasard parmi « Plus de ce genre », la vidéo réduite
///   en haut à gauche ; à la fin du fichier (`ended`), écran de fin avec « Retour à l'accueil ».
fn show_up_next(app: &Arc<App>, client: Option<&Client>, item: Option<&Item>, next: Option<String>, ended: bool) {
    let (Some(c), Some(it)) = (client.cloned(), item.cloned()) else { return };
    let app2 = app.clone();
    app.rt.spawn(async move {
        if let Some(next_id) = next {
            let Ok(n) = c.item(&next_id).await else { return };
            let new_season = n.parent_index_number != it.parent_index_number;
            let title = if new_season { tr("Saison suivante") } else { tr("Épisode suivant") };
            let button = if new_season { tr("Passer à la saison suivante") } else { tr("Passer à l'épisode suivant") };
            let card = n.child_card();
            let sub = match (n.parent_index_number, n.index_number) {
                (Some(s), Some(e)) => format!("S{s}E{e} · {}", n.name),
                _ => n.name.clone(),
            };
            let _ = app2.ui().upgrade_in_event_loop(move |u| {
                u.set_p_up_title(title.into());
                u.set_p_up_button(button.into());
                u.set_p_up_sub(sub.into());
                u.set_p_has_up_image(false);
                u.set_p_up_mode("next".into());
            });
            if let Some(job) = crate::ImageJob::for_card(0, 0, &card, true) {
                let apply: crate::Apply = Arc::new(|u: &AppWindow, _job: &crate::ImageJob, buf: slint::SharedPixelBuffer<slint::Rgba8Pixel>| {
                    u.set_p_up_image(slint::Image::from_rgba8(buf));
                    u.set_p_has_up_image(true);
                });
                let k = if app2.tv() { 1.4 } else { 1.0 };
                crate::spawn_image_jobs(&app2, &c, vec![job], api::Size::Fill(400, 225), crate::Shape::card(400, 225, 260.0 * k), apply);
            }
            return;
        }

        // Suggestions : 3 au hasard parmi « Plus de ce genre » (de la série pour un épisode).
        let base = match (&*it.kind, &it.series_id) {
            ("Episode" | "Season", Some(sid)) => sid.clone(),
            _ => it.id.clone(),
        };
        let mut sim = c.similar(&base).await.unwrap_or_default();
        shuffle(&mut sim);
        sim.truncate(3);
        if sim.is_empty() {
            if ended {
                let _ = app2.ui().upgrade_in_event_loop(|u| u.set_p_up_mode("end".into()));
            }
            return;
        }
        let cards: Vec<api::CardInfo> = sim.iter().map(|i| i.card()).collect();
        let jobs: Vec<crate::ImageJob> =
            cards.iter().enumerate().filter_map(|(i, c)| crate::ImageJob::for_card(0, i, c, false)).collect();
        let _ = app2.ui().upgrade_in_event_loop(move |u| {
            u.set_p_picks(ModelRc::new(VecModel::from(cards.iter().map(card_data).collect::<Vec<_>>())));
            u.set_p_up_mode(if ended { "end" } else { "pick" }.into());
        });
        let k = if app2.tv() { 1.4 } else { 1.0 };
        let apply: crate::Apply = Arc::new(|u: &AppWindow, job: &crate::ImageJob, buf: slint::SharedPixelBuffer<slint::Rgba8Pixel>| {
            let model = u.get_p_picks();
            if let Some(mut card) = model.row_data(job.b) {
                if card.id.as_str() == job.item_id {
                    card.image = slint::Image::from_rgba8(buf);
                    card.has_image = true;
                    model.set_row_data(job.b, card);
                }
            }
        });
        crate::spawn_image_jobs(&app2, &c, jobs, api::Size::Fill(270, 405), crate::Shape::card_top(270, 405, 200.0 * k), apply);
    });
}

/// Mélange simple (pas besoin d'un générateur aléatoire de qualité pour choisir 3 suggestions).
fn shuffle<T>(v: &mut [T]) {
    let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
    for i in (1..v.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = ((seed >> 33) as usize) % (i + 1);
        v.swap(i, j);
    }
}

/// Langue d'une piste mpv (identifiant `id`) d'après la dernière liste reçue.
fn track_lang(list: &Value, id: &str) -> String {
    list.as_array()
        .into_iter()
        .flatten()
        .find(|t| t["id"].as_i64().map(|i| i.to_string()).as_deref() == Some(id))
        .and_then(|t| t["lang"].as_str())
        .unwrap_or("")
        .to_string()
}

/// Épisode voisin (décalage -1 / +1) dans la liste de la série.
fn neighbour(episodes: &[String], item: Option<&Item>, d: i64) -> Option<String> {
    let id = &item?.id;
    let i = episodes.iter().position(|e| e == id)? as i64 + d;
    episodes.get(usize::try_from(i).ok()?).cloned()
}

/// Charge un fichier dans le lecteur et prépare l'interface.
async fn load(
    player: &Mpv,
    client: Option<&Client>,
    item: Option<Item>,
    start_secs: f64,
    test_url: Option<&str>,
    episodes: &[String],
    ui: &slint::Weak<AppWindow>,
) -> Result<Current> {
    let mut tc: Option<crate::api::Transcode> = None;
    let (url, ms_id, title, subtitle, subs) = match (&item, client, test_url) {
        (Some(it), Some(c), _) => {
            let source = it.media_sources.as_ref().and_then(|v| v.first());
            let ms_id = source.map(|m| m.id.clone()).unwrap_or_else(|| it.id.clone());
            // Débit maximal (réglage) : le serveur dit s'il faut convertir. Échec de la demande : on
            // tente le fichier original plutôt que de ne rien lire.
            let kbps = crate::config::ui_prefs().max_bitrate;
            if kbps > 0 && source.and_then(|m| m.remote_url()).is_none() {
                match c.transcode(&it.id, &ms_id, kbps, preferred_audio(it)).await {
                    Ok(t) => tc = t,
                    Err(e) => eprintln!("turtlefin : débit maximal ignoré ({e})"),
                }
            }
            // Flux converti : les sous-titres intégrés n'y sont plus ; le serveur les fournit à part
            // (texte seulement, comme les sous-titres externes).
            let subs = source
                .map(|src| {
                    src.media_streams
                        .iter()
                        .filter(|s| s.kind == "Subtitle" && (s.is_external || tc.is_some()))
                        .filter_map(|s| c.subtitle_url(&it.id, &ms_id, s).map(|u| (u, s.language.clone().unwrap_or_default())))
                        .collect()
                })
                .unwrap_or_default();
            let (t, s) = it.titles();
            let url = match (&tc, source.and_then(|m| m.remote_url())) {
                (Some(t), _) => t.url.clone(),
                (None, Some(remote)) => remote.to_string(),
                (None, None) => c.stream_url(&it.id, &ms_id),
            };
            (url, ms_id, t, s, subs)
        }
        (_, _, Some(u)) => (u.to_string(), String::new(), tr("Vidéo de test").to_string(), u.to_string(), Vec::new()),
        _ => return Err(anyhow!("{}", tr("rien à lire"))),
    };

    // Jeton dans l'en-tête des requêtes de mpv, pas dans l'adresse : il n'apparaît ni dans les
    // journaux (mpv, serveur, proxy) ni dans les messages d'erreur. `append` garde la valeur entière
    // (elle contient des virgules, séparateur des listes de mpv).
    let _ = player.command(&["change-list", "http-header-fields", "clr", ""]);
    // Seulement vers le serveur Jellyfin lui-même (pas vers une source distante).
    if let (Some(c), Some(_), true) = (client, &item, client.is_some_and(|c| url.starts_with(&c.server))) {
        player.command(&["change-list", "http-header-fields", "append", &format!("Authorization: {}", c.auth())])?;
    }
    player.set_property("force-media-title", &title)?;
    // Paramètres > Sous-titres > Taille.
    let _ = player.set_property("sub-scale", &format!("{:.2}", crate::config::ui_prefs().sub_scale));
    // Pistes préférées de la série / du film (langue audio, sous-titres).
    if let Some(it) = &item {
        let pref = crate::config::track_pref(&it.pref_key());
        // Rien de choisi pour la série : préférences du compte Jellyfin.
        let (d_audio, d_sub, d_mode) = crate::config::user_defaults();
        let audio = if pref.audio.is_empty() { d_audio } else { pref.audio.clone() };
        let sub = if !pref.sub.is_empty() {
            pref.sub.clone()
        } else if d_mode == "None" {
            "off".to_string()
        } else {
            d_sub
        };
        let _ = player.set_property("alang", &audio);
        match sub.as_str() {
            "off" => {
                let _ = player.set_property("sid", "no");
            }
            lang => {
                let _ = player.set_property("sid", "auto");
                let _ = player.set_property("slang", lang);
            }
        }
    }
    let start = if start_secs > 1.0 { format!("{start_secs:.1}") } else { "none".to_string() };
    player.set_property("start", &start)?;
    player.command(&["loadfile", &url, "replace"])?;

    let has_prev = neighbour(episodes, item.as_ref(), -1).is_some();
    let has_next = neighbour(episodes, item.as_ref(), 1).is_some();
    // Bouton Qualité : seulement pour un flux du serveur (pas un fichier téléchargé ni une source distante).
    let quality = item.is_some() && client.is_some_and(|c| url.starts_with(&c.server));
    let shown_volume = volume_shown();
    let _ = ui.upgrade_in_event_loop(move |u| {
        let (list, i) = if quality { quality_tracks() } else { (Vec::new(), 0) };
        u.set_p_quality(ModelRc::new(VecModel::from(list)));
        u.set_p_quality_current(i);
        u.set_p_title(title.into());
        u.set_p_subtitle(subtitle.into());
        u.set_p_has_prev(has_prev);
        u.set_p_has_next(has_next);
        u.set_p_ready(false);
        u.set_p_progress(0.0);
        u.set_p_pos_text("".into());
        u.set_p_dur_text("".into());
        u.set_p_end_text("".into());
        u.set_p_volume(shown_volume);
        u.set_p_chapters(ModelRc::default());
        u.set_p_chapter_count(0);
    });

    let transcode = tc.is_some();
    Ok(Current {
        item,
        ms_id,
        // Flux converti : la session est celle que le serveur a ouverte pour la conversion.
        session: tc.map(|t| t.session).filter(|s| !s.is_empty()).unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string()),
        pos: start_secs,
        dur: 0.0,
        paused: false,
        chapters: Vec::new(),
        chapter_titles: Vec::new(),
        outro: None,
        intro: None,
        intro_shown: false,
        up_shown: false,
        ended: false,
        subs,
        reported_stop: false,
        transcode,
    })
}

/// Volume choisi (0 à 100) : celui du téléphone sur Android, sinon celui de Turtlefin (gardé).
fn apply_volume(player: &Mpv, v: u32) -> Result<()> {
    #[cfg(target_os = "android")]
    {
        let _ = player;
        crate::android_entry::set_system_volume(v as i32);
        Ok(())
    }
    #[cfg(not(target_os = "android"))]
    {
        let mut p = crate::config::ui_prefs();
        p.volume = v;
        crate::config::save_ui_prefs(&p);
        player.set_property("volume", &v.to_string())
    }
}

/// Volume donné à mpv : celui de Turtlefin (bureau), 100 % sur Android (voir « volume »).
fn app_volume() -> u32 {
    if cfg!(target_os = "android") {
        100
    } else {
        crate::config::ui_prefs().volume.min(100)
    }
}

/// Volume montré par la barre du lecteur : celui du téléphone sur Android.
fn volume_shown() -> i32 {
    #[cfg(target_os = "android")]
    if let Some(v) = crate::android_entry::system_volume() {
        return v;
    }
    app_volume() as i32
}

/// Piste audio à garder dans un flux converti (il n'en contient qu'une) : langue préférée de la
/// série / du film, sinon celle du compte. None : le serveur prend la piste par défaut.
fn preferred_audio(it: &Item) -> Option<i32> {
    let pref = crate::config::track_pref(&it.pref_key()).audio;
    let want = if pref.is_empty() { crate::config::user_defaults().0 } else { pref };
    let langs: Vec<String> = want.split(',').map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect();
    let streams = &it.media_sources.as_ref()?.first()?.media_streams;
    streams
        .iter()
        .filter(|s| s.kind == "Audio")
        .find(|s| s.language.as_deref().is_some_and(|l| langs.contains(&l.to_lowercase())))
        .map(|s| s.index)
}

/// Temps écoulé, durée, progression et heure de fin.
fn push_time(ui: &slint::Weak<AppWindow>, cur: &Current) {
    let (pos, dur) = (cur.pos, cur.dur);
    let progress = if dur > 0.0 { (pos / dur).clamp(0.0, 1.0) as f32 } else { 0.0 };
    let end = if dur > 0.0 {
        // Durée absurde (fichier abîmé ou piégé) : pas d'heure de fin plutôt qu'un débordement.
        let left = chrono::Duration::milliseconds(((dur - pos).clamp(0.0, 1e9) * 1000.0) as i64);
        match chrono::Local::now().checked_add_signed(left) {
            Some(t) => trf("Fin à {}", &[&t.format("%H:%M")]),
            None => String::new(),
        }
    } else {
        String::new()
    };
    let (pos_t, dur_t) = (fmt_time(pos), if dur > 0.0 { fmt_time(dur) } else { String::new() });
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_p_progress(progress);
        u.set_p_pos_text(pos_t.into());
        u.set_p_dur_text(dur_t.into());
        u.set_p_end_text(end.into());
    });
}

/// Repères de chapitres sur la barre de temps (fractions de la durée).
fn push_chapters(ui: &slint::Weak<AppWindow>, cur: &Current) {
    let marks: Vec<f32> = if cur.dur > 0.0 {
        cur.chapters.iter().filter(|t| **t > 0.5).map(|t| (t / cur.dur).clamp(0.0, 1.0) as f32).collect()
    } else {
        Vec::new()
    };
    let count = cur.chapters.len() as i32;
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_p_chapters(ModelRc::new(VecModel::from(marks)));
        u.set_p_chapter_count(count);
    });
}
