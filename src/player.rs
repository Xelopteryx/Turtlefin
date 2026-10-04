//! Lecture avec libmpv, intégrée à Turtlefin.
//!
//! mpv décode et dessine la vidéo dans une texture affichée par l'interface (voir video.rs) ;
//! les commandes (barre de temps, chapitres, épisodes, pistes) sont dessinées par Slint.
//! Turtlefin rapporte la lecture au serveur (début, progression toutes les 10 s, fin) et
//! enchaîne sur l'épisode suivant en fin de fichier.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use slint::{ModelRc, VecModel};

use crate::api::{Client, Item};
use crate::mpv::{Event, Mpv};
use crate::{video, AppWindow, TrackData};

pub struct PlayRequest {
    /// Élément Jellyfin à lire (None : fichier de test local, voir `test_url`).
    pub item: Option<Item>,
    pub start_secs: f64,
    /// Lecture d'essai sans serveur (option --test-video).
    pub test_url: Option<String>,
}

/// Décodage matériel : variable TURTLEFIN_HWDEC pour forcer une valeur (ex. « auto-safe », « no »).
/// Sur Raspberry Pi (Linux ARM 64 bits), le décodage matériel V4L2 produit des images au format
/// Broadcom « SAND » que mpv ne sait pas importer en OpenGL : résultat, un écran vide. On décode
/// donc en logiciel par défaut. Ailleurs (Windows, PC Linux) : « auto-safe ».
fn hwdec_mode() -> String {
    if let Ok(v) = std::env::var("TURTLEFIN_HWDEC") {
        return v;
    }
    if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "no".to_string()
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
    m.set_option("keep-open", "no");
    m.set_option("config", "no");
    m.set_option("terminal", "no");
    m.set_option("input-default-bindings", "no");
    m.set_option("osc", "no");
    m.set_option("ytdl", "no");
    // Pi : rendu simplifié (mise à l'échelle bilinéaire...) pour ménager le GPU.
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
    for p in ["time-pos", "duration", "pause", "chapter-list", "track-list"] {
        m.observe(p);
    }
    Ok(Arc::new(m))
}

/// Pistes audio / sous-titres pour les menus. Renvoie aussi l'index de la piste active.
fn tracks(list: &Value, kind: &str) -> (Vec<TrackData>, i32) {
    let mut out: Vec<TrackData> = Vec::new();
    if kind == "sub" {
        out.push(TrackData { id: "no".into(), label: "Désactivés".into(), current: false });
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
        let label = if parts.is_empty() { format!("Piste {id}") } else { parts.join(" · ") };
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
struct Current {
    item: Option<Item>,
    ms_id: String,
    session: String,
    pos: f64,
    dur: f64,
    paused: bool,
    chapters: Vec<f64>,
    /// Sous-titres externes à ajouter une fois le fichier chargé : (url, langue).
    subs: Vec<(String, String)>,
    reported_stop: bool,
}

impl Current {
    fn body(&self, pos: f64) -> Option<Value> {
        let item = self.item.as_ref()?;
        Some(json!({
            "ItemId": item.id,
            "MediaSourceId": self.ms_id,
            "PlaySessionId": self.session,
            "PositionTicks": ticks(pos),
            "IsPaused": self.paused,
            "CanSeek": true,
            "PlayMethod": "DirectPlay",
        }))
    }
}

async fn report(client: Option<&Client>, endpoint: &str, body: Option<Value>) {
    if let (Some(c), Some(b)) = (client, body) {
        let _ = c.report(endpoint, &b).await;
    }
}

/// Lance la lecture et rend la main quand l'utilisateur la quitte.
pub async fn play(
    client: Option<Client>,
    req: PlayRequest,
    mut commands: tokio::sync::mpsc::UnboundedReceiver<String>,
    ui: slint::Weak<AppWindow>,
) -> Result<()> {
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

    // Liste des épisodes de la série, pour « épisode précédent / suivant » et l'enchaînement.
    let episodes: Vec<String> = match (client, req.item.as_ref()) {
        (Some(c), Some(it)) if it.kind == "Episode" => match it.series_id.as_deref() {
            Some(sid) => c.episodes(sid, None).await.map(|v| v.into_iter().map(|e| e.id).collect()).unwrap_or_default(),
            None => Vec::new(),
        },
        _ => Vec::new(),
    };

    // mpv doit avoir son rendu (créé par l'interface au prochain affichage) avant d'ouvrir le fichier.
    if tokio::time::timeout(Duration::from_secs(5), render_ready).await.is_err() {
        eprintln!("turtlefin : rendu vidéo pas prêt après 5 s, lecture sans image");
    }

    let mut cur = load(&player, client, req.item, req.start_secs, req.test_url.as_deref(), &episodes, &ui)?;
    report(client, "/Sessions/Playing", cur.body(cur.pos)).await;

    let mut tick = tokio::time::interval(Duration::from_secs(10));
    tick.tick().await; // le premier tick est immédiat : on le consomme
    let mut last_sec: i64 = -1;
    let mut switching = false;
    let mut result: Result<()> = Ok(());

    loop {
        // Changement d'épisode demandé (bouton, ou fin de fichier) : Some(décalage).
        let mut go_episode: Option<i64> = None;

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
                            }
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
                            cur.chapters = v.as_array().into_iter().flatten().filter_map(|c| c["time"].as_f64()).collect();
                            push_chapters(&ui, &cur);
                        }
                        "track-list" => {
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
                    Event::FileLoaded => {
                        for (url, lang) in std::mem::take(&mut cur.subs) {
                            let _ = player.command(&["sub-add", &url, "auto", "", &lang]);
                        }
                        let _ = ui.upgrade_in_event_loop(|u| u.set_p_ready(true));
                    }
                    Event::EndFile { eof, error } => {
                        if switching {
                            // Fin de l'ancien fichier, provoquée par le changement d'épisode.
                            switching = false;
                        } else if let Some(e) = error {
                            result = Err(anyhow!("mpv n'a pas pu lire ce média ({e})"));
                            break;
                        } else if eof {
                            // Fin atteinte : on rapporte la durée complète (le serveur marque « vu »),
                            // puis épisode suivant s'il y en a un.
                            let end = if cur.dur > 0.0 { cur.dur } else { cur.pos };
                            report(client, "/Sessions/Playing/Stopped", cur.body(end)).await;
                            cur.reported_stop = true;
                            if neighbour(&episodes, cur.item.as_ref(), 1).is_some() {
                                go_episode = Some(1);
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
                report(client, "/Sessions/Playing/Progress", cur.body(cur.pos)).await;
            }
            cmd = commands.recv() => {
                let Some(c) = cmd else { break };
                let (verb, arg) = c.split_once(':').unwrap_or((c.as_str(), ""));
                let r = match verb {
                    "pause" => player.command(&["cycle", "pause"]),
                    "seek" => player.command(&["seek", arg, "relative"]),
                    "seek-to" => {
                        let pct = arg.parse::<f64>().unwrap_or(0.0).clamp(0.0, 1.0) * 100.0;
                        player.command(&["seek", &format!("{pct:.3}"), "absolute-percent"])
                    }
                    "chapter" => player.command(&["add", "chapter", arg]),
                    "sub-margin" => player.set_property("sub-margin-y", arg),
                    "aid" => player.set_property("aid", arg),
                    "sid" => player.set_property("sid", arg),
                    "episode" => {
                        go_episode = arg.parse().ok();
                        Ok(())
                    }
                    "stop" => break,
                    _ => Ok(()),
                };
                if let Err(e) = r {
                    eprintln!("turtlefin : {e}");
                }
            }
        }

        if let Some(d) = go_episode {
            let Some(next_id) = neighbour(&episodes, cur.item.as_ref(), d) else { continue };
            let Some(c) = client else { continue };
            if !cur.reported_stop {
                report(client, "/Sessions/Playing/Stopped", cur.body(cur.pos)).await;
            }
            match c.item(&next_id).await {
                Ok(next) => {
                    let start = next.user_data.as_ref().map(|u| u.playback_position_ticks as f64 / 1e7).unwrap_or(0.0);
                    switching = true;
                    match load(&player, client, Some(next), start, None, &episodes, &ui) {
                        Ok(n) => {
                            cur = n;
                            last_sec = -1;
                            report(client, "/Sessions/Playing", cur.body(cur.pos)).await;
                        }
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    result = Err(anyhow!("épisode suivant introuvable ({e})"));
                    break;
                }
            }
        }
    }

    if !cur.reported_stop {
        report(client, "/Sessions/Playing/Stopped", cur.body(cur.pos)).await;
    }
    // Arrêt du lecteur : le thread d'événements se termine, puis l'interface libère le rendu
    // au prochain affichage, ce qui détruit le lecteur (et rend sa mémoire).
    let _ = player.command(&["quit"]);
    let _ = video::attach(&ui, None);
    result
}

/// Épisode voisin (décalage -1 / +1) dans la liste de la série.
fn neighbour(episodes: &[String], item: Option<&Item>, d: i64) -> Option<String> {
    let id = &item?.id;
    let i = episodes.iter().position(|e| e == id)? as i64 + d;
    episodes.get(usize::try_from(i).ok()?).cloned()
}

/// Charge un fichier dans le lecteur et prépare l'interface.
fn load(
    player: &Mpv,
    client: Option<&Client>,
    item: Option<Item>,
    start_secs: f64,
    test_url: Option<&str>,
    episodes: &[String],
    ui: &slint::Weak<AppWindow>,
) -> Result<Current> {
    let (url, ms_id, title, subtitle, subs) = match (&item, client, test_url) {
        (Some(it), Some(c), _) => {
            let source = it.media_sources.as_ref().and_then(|v| v.first());
            let ms_id = source.map(|m| m.id.clone()).unwrap_or_else(|| it.id.clone());
            let subs = source
                .map(|src| {
                    src.media_streams
                        .iter()
                        .filter(|s| s.kind == "Subtitle" && s.is_external)
                        .filter_map(|s| c.subtitle_url(&it.id, &ms_id, s).map(|u| (u, s.language.clone().unwrap_or_default())))
                        .collect()
                })
                .unwrap_or_default();
            let (t, s) = it.titles();
            (c.stream_url(&it.id, &ms_id), ms_id, t, s, subs)
        }
        (_, _, Some(u)) => (u.to_string(), String::new(), "Vidéo de test".to_string(), u.to_string(), Vec::new()),
        _ => return Err(anyhow!("rien à lire")),
    };

    player.set_property("force-media-title", &title)?;
    let start = if start_secs > 1.0 { format!("{start_secs:.1}") } else { "none".to_string() };
    player.set_property("start", &start)?;
    player.command(&["loadfile", &url, "replace"])?;

    let has_prev = neighbour(episodes, item.as_ref(), -1).is_some();
    let has_next = neighbour(episodes, item.as_ref(), 1).is_some();
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_p_title(title.into());
        u.set_p_subtitle(subtitle.into());
        u.set_p_has_prev(has_prev);
        u.set_p_has_next(has_next);
        u.set_p_ready(false);
        u.set_p_progress(0.0);
        u.set_p_pos_text("".into());
        u.set_p_dur_text("".into());
        u.set_p_end_text("".into());
        u.set_p_chapters(ModelRc::default());
        u.set_p_chapter_count(0);
    });

    Ok(Current {
        item,
        ms_id,
        session: uuid::Uuid::new_v4().simple().to_string(),
        pos: start_secs,
        dur: 0.0,
        paused: false,
        chapters: Vec::new(),
        subs,
        reported_stop: false,
    })
}

/// Temps écoulé, durée, progression et heure de fin.
fn push_time(ui: &slint::Weak<AppWindow>, cur: &Current) {
    let (pos, dur) = (cur.pos, cur.dur);
    let progress = if dur > 0.0 { (pos / dur).clamp(0.0, 1.0) as f32 } else { 0.0 };
    let end = if dur > 0.0 {
        let left = chrono::Duration::milliseconds(((dur - pos).max(0.0) * 1000.0) as i64);
        format!("Fin à {}", (chrono::Local::now() + left).format("%H:%M"))
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
