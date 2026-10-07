//! Watch party (SyncPlay de Jellyfin) : plusieurs comptes regardent la même chose en même temps.
//!
//! - Une connexion WebSocket (`/socket`) reçoit les messages du serveur : état du groupe
//!   (`SyncPlayGroupUpdate`) et commandes de lecture (`SyncPlayCommand` : pause, reprise, saut).
//! - Les actions locales (lancer un média, pause, saut) sont envoyées au serveur au lieu d'être
//!   appliquées directement ; le serveur les renvoie à tout le groupe au même instant (`When`).
//! - Le lecteur signale qu'il est prêt (`Ready`) une fois le média chargé ou le saut terminé.

use crate::i18n::trf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};

use crate::api::Client;

/// Commande à appliquer par le lecteur.
#[derive(Clone, Debug)]
pub enum Cmd {
    /// Reprendre à `pos` secondes, `delay` secondes plus tard.
    Unpause { pos: f64, delay: f64 },
    Pause { pos: f64 },
    Seek { pos: f64 },
    Stop,
}

/// État de la watch party.
#[derive(Default)]
pub struct State {
    pub group: Option<(String, String)>, // (identifiant, nom)
    pub participants: Vec<String>,
    /// Élément de la file en cours (pour Ready / Buffering).
    pub playlist_item: String,
    /// État du groupe : Idle, Waiting, Paused, Playing.
    pub state: String,
    /// Élément Jellyfin que le groupe regarde (file en cours).
    pub item_id: String,
}

pub type Shared = Arc<Mutex<State>>;

/// Ce que la connexion transmet à l'application.
pub enum Event {
    /// Le groupe a changé (rejoint, quitté, participants) : rafraîchir l'affichage.
    Group,
    /// Message court à afficher.
    Toast(String),
    /// Lancer ce média (file du groupe), à cette position, en attente de la reprise commune.
    Play { item_id: String, start: f64 },
    Command(Cmd),
}

fn ticks(secs: f64) -> i64 {
    (secs.max(0.0) * 1e7) as i64
}

/// Délai jusqu'à un instant du serveur (« 2026-10-04T20:33:12.1234567Z »), en secondes (peut être négatif).
fn until(when: &str) -> f64 {
    match chrono::DateTime::parse_from_rfc3339(when) {
        Ok(t) => (t.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_milliseconds() as f64 / 1000.0,
        Err(_) => 0.0,
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Connexion WebSocket au serveur, avec reconnexion automatique. Une par session : la tâche est
/// arrêtée à la fin de la session. Jeton refusé (401 / 403 : compte déconnecté ou retiré) : on
/// arrête, réessayer ne servirait à rien.
pub async fn connect(client: Client, state: Shared, tx: tokio::sync::mpsc::UnboundedSender<Event>) {
    // Serveur injoignable : 5 s, puis 10, 20, 40, 60 s au plus entre deux essais.
    let mut wait = 5;
    loop {
        let start = std::time::Instant::now();
        if let Err(e) = run(&client, &state, &tx).await {
            let msg = e.to_string();
            if msg.contains("401") || msg.contains("403") {
                eprintln!("turtlefin : watch party, connexion refusée ({msg}) : jeton plus valable, arrêt");
                return;
            }
            // Connexion qui a tenu un moment : on repart du délai le plus court.
            if start.elapsed() > Duration::from_secs(60) {
                wait = 5;
            }
            eprintln!("turtlefin : watch party, connexion perdue ({msg}), nouvel essai dans {wait} s");
        }
        if tx.is_closed() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(wait)).await;
        wait = (wait * 2).min(60);
    }
}

async fn run(client: &Client, state: &Shared, tx: &tokio::sync::mpsc::UnboundedSender<Event>) -> Result<()> {
    let base = client.server.replacen("http", "ws", 1);
    // Le jeton passe par l'en-tête (le serveur refuse `api_key` dans l'adresse : 403).
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut req = format!("{base}/socket?deviceId={}", client.device_id).into_client_request()?;
    req.headers_mut().insert("Authorization", client.auth().parse()?);
    let (ws, _) = tokio_tungstenite::connect_async(req).await?;
    let (mut write, mut read) = ws.split();
    let mut keepalive = tokio::time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            msg = read.next() => {
                let Some(msg) = msg else { return Err(anyhow!("fermée")) };
                let msg = msg?;
                let Ok(text) = msg.to_text() else { continue };
                let Ok(v) = serde_json::from_str::<Value>(text) else { continue };
                match v["MessageType"].as_str().unwrap_or("") {
                    "ForceKeepAlive" => {
                        let secs = v["Data"].as_u64().unwrap_or(60).max(10);
                        keepalive = tokio::time::interval(Duration::from_secs(secs / 2));
                    }
                    "SyncPlayGroupUpdate" => group_update(state, tx, &v["Data"]),
                    "SyncPlayCommand" => command(state, tx, &v["Data"]),
                    _ => {}
                }
            }
            _ = keepalive.tick() => {
                write.send(tokio_tungstenite::tungstenite::Message::Text(json!({ "MessageType": "KeepAlive" }).to_string().into())).await?;
            }
        }
    }
}

fn group_update(state: &Shared, tx: &tokio::sync::mpsc::UnboundedSender<Event>, d: &Value) {
    let data = &d["Data"];
    match d["Type"].as_str().unwrap_or("") {
        "GroupJoined" => {
            let mut s = state.lock().unwrap();
            let name = data["GroupName"].as_str().unwrap_or("Watch party").to_string();
            s.group = Some((data["GroupId"].as_str().unwrap_or("").to_string(), name.clone()));
            s.participants = data["Participants"].as_array().into_iter().flatten().filter_map(|p| p.as_str().map(str::to_string)).collect();
            drop(s);
            let _ = tx.send(Event::Toast(trf("Watch party « {} » rejointe.", &[&name])));
            let _ = tx.send(Event::Group);
        }
        "UserJoined" | "UserLeft" => {
            let who = data.as_str().unwrap_or("Quelqu'un").to_string();
            let joined = d["Type"] == "UserJoined";
            {
                let mut s = state.lock().unwrap();
                if joined {
                    if !s.participants.contains(&who) {
                        s.participants.push(who.clone());
                    }
                } else {
                    s.participants.retain(|p| *p != who);
                }
            }
            let _ = tx.send(Event::Toast(if joined { trf("{} a rejoint la watch party.", &[&who]) } else { trf("{} a quitté la watch party.", &[&who]) }));
            let _ = tx.send(Event::Group);
        }
        "GroupLeft" | "NotInGroup" | "GroupDoesNotExist" => {
            *state.lock().unwrap() = State::default();
            let _ = tx.send(Event::Group);
        }
        "StateUpdate" => {
            state.lock().unwrap().state = data["State"].as_str().unwrap_or("").to_string();
            let _ = tx.send(Event::Group);
        }
        "PlayQueue" => {
            let reason = data["Reason"].as_str().unwrap_or("");
            let idx = data["PlayingItemIndex"].as_i64().unwrap_or(0).max(0) as usize;
            let Some(entry) = data["Playlist"].as_array().and_then(|p| p.get(idx)) else { return };
            let item_id = entry["ItemId"].as_str().unwrap_or("").to_string();
            {
                let mut st = state.lock().unwrap();
                st.playlist_item = entry["PlaylistItemId"].as_str().unwrap_or("").to_string();
                st.item_id = item_id.clone();
            }
            let _ = tx.send(Event::Group);
            if matches!(reason, "NewPlaylist" | "SetCurrentItem" | "NextItem" | "PreviousItem") && !item_id.is_empty() {
                let start = data["StartPositionTicks"].as_f64().unwrap_or(0.0) / 1e7;
                let _ = tx.send(Event::Play { item_id, start });
            }
        }
        _ => {}
    }
}

fn command(state: &Shared, tx: &tokio::sync::mpsc::UnboundedSender<Event>, d: &Value) {
    let pos = d["PositionTicks"].as_f64().unwrap_or(0.0) / 1e7;
    let when = d["When"].as_str().unwrap_or("");
    if let Some(pid) = d["PlaylistItemId"].as_str().filter(|s| !s.is_empty()) {
        state.lock().unwrap().playlist_item = pid.to_string();
    }
    let cmd = match d["Command"].as_str().unwrap_or("") {
        "Unpause" => Cmd::Unpause { pos, delay: until(when) },
        "Pause" => Cmd::Pause { pos },
        "Seek" => Cmd::Seek { pos },
        "Stop" => Cmd::Stop,
        _ => return,
    };
    let _ = tx.send(Event::Command(cmd));
}

// ---------------------------------------------------------------------------
// Requêtes au serveur
// ---------------------------------------------------------------------------
async fn post(client: &Client, path: &str, body: Value) -> Result<()> {
    client.post_json(&format!("/SyncPlay/{path}"), &body).await
}

/// Groupes existants : (identifiant, nom, participants).
pub async fn list(client: &Client) -> Vec<(String, String, Vec<String>)> {
    let v: Value = client.get_json("/SyncPlay/List").await.unwrap_or_default();
    v.as_array()
        .into_iter()
        .flatten()
        .map(|g| {
            (
                g["GroupId"].as_str().unwrap_or("").to_string(),
                g["GroupName"].as_str().unwrap_or("").to_string(),
                g["Participants"].as_array().into_iter().flatten().filter_map(|p| p.as_str().map(str::to_string)).collect(),
            )
        })
        .collect()
}

pub async fn create(client: &Client, name: &str) -> Result<()> {
    post(client, "New", json!({ "GroupName": name })).await
}

pub async fn join(client: &Client, id: &str) -> Result<()> {
    post(client, "Join", json!({ "GroupId": id })).await
}

pub async fn leave(client: &Client) -> Result<()> {
    post(client, "Leave", json!({})).await
}

/// Lance un média pour tout le groupe.
pub async fn play(client: &Client, item_id: &str, start: f64) -> Result<()> {
    post(client, "SetNewQueue", json!({ "PlayingQueue": [item_id], "PlayingItemPosition": 0, "StartPositionTicks": ticks(start) })).await
}

pub async fn pause(client: &Client) -> Result<()> {
    post(client, "Pause", json!({})).await
}

pub async fn unpause(client: &Client) -> Result<()> {
    post(client, "Unpause", json!({})).await
}

pub async fn seek(client: &Client, pos: f64) -> Result<()> {
    post(client, "Seek", json!({ "PositionTicks": ticks(pos) })).await
}

/// Prêt à cette position (média chargé, saut terminé).
pub async fn ready(client: &Client, state: &Shared, pos: f64, playing: bool) -> Result<()> {
    let pid = state.lock().unwrap().playlist_item.clone();
    post(client, "Ready", json!({ "When": now_iso(), "PositionTicks": ticks(pos), "IsPlaying": playing, "PlaylistItemId": pid })).await
}

pub async fn buffering(client: &Client, state: &Shared, pos: f64) -> Result<()> {
    let pid = state.lock().unwrap().playlist_item.clone();
    post(client, "Buffering", json!({ "When": now_iso(), "PositionTicks": ticks(pos), "IsPlaying": false, "PlaylistItemId": pid })).await
}
