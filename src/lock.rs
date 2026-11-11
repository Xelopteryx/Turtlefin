//! Code facultatif d'un compte enregistré : 4 chiffres ou combinaison de 6 flèches, demandé pour
//! ouvrir le compte depuis « Qui regarde ? » (ou le retirer de l'appareil). Pas demandé quand le
//! compte s'ouvre au lancement de l'appli (compte de démarrage, `--user=`).
//!
//! Ce code protège l'accès devant l'écran (comme le code d'un profil), pas le fichier des comptes :
//! 10 000 codes possibles s'essaient en un instant hors de l'appli. Il est gardé haché (sel aléatoire,
//! SHA-256 répété), jamais en clair ; dans l'appli, chaque erreur au-delà de la cinquième impose une
//! attente qui double (30 s, 1 min… 1 h au plus), comptée dans le fichier (relancer l'appli n'y
//! change rien). Code oublié : le mot de passe du compte sur le serveur le retire.

use std::sync::Arc;

use crate::config;
use crate::i18n::{tr, trf};
use crate::{App, AppWindow};

/// Longueur du code selon son genre : « pin » (chiffres) ou « keys » (flèches).
pub fn len(kind: &str) -> usize {
    if kind == "keys" {
        6
    } else {
        4
    }
}

/// Genre d'un code enregistré (vide : aucun code).
pub fn kind(stored: &str) -> &str {
    stored.split(':').next().filter(|k| matches!(*k, "pin" | "keys")).unwrap_or("")
}

const ROUNDS: u32 = 50_000;

fn digest(salt: &str, code: &str) -> String {
    use sha2::Digest;
    let mut h: [u8; 32] = sha2::Sha256::digest(format!("turtlefin-lock:{salt}:{code}").as_bytes()).into();
    for _ in 0..ROUNDS {
        let mut s = sha2::Sha256::new();
        s.update(h);
        s.update(salt.as_bytes());
        h = s.finalize().into();
    }
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// Code haché à enregistrer : « genre:sel:empreinte ».
pub fn make(kind: &str, code: &str) -> String {
    let salt = uuid::Uuid::new_v4().simple().to_string();
    format!("{kind}:{salt}:{}", digest(&salt, code))
}

/// Le code tapé est-il le bon ? (comparaison en temps constant)
pub fn verify(stored: &str, code: &str) -> bool {
    let mut parts = stored.splitn(3, ':');
    let (Some(_), Some(salt), Some(want)) = (parts.next(), parts.next(), parts.next()) else { return false };
    let got = digest(salt, code);
    got.len() == want.len() && got.bytes().zip(want.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// Attente imposée après `fails` erreurs de suite (secondes).
pub fn delay(fails: u32) -> u64 {
    if fails < 5 {
        0
    } else {
        (30u64 << (fails - 5).min(7)).min(3600)
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Ce que le code débloque.
#[derive(Clone, Debug, PartialEq)]
pub enum Goal {
    /// Ouvrir le compte (« Qui regarde ? »).
    Open,
    /// Le retirer de cet appareil.
    Forget,
    /// Paramètres : nouveau code de ce genre (vide : retirer le code).
    Set(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Step {
    /// Code actuel du compte.
    Check,
    /// Nouveau code.
    New,
    /// Nouveau code, une seconde fois.
    Confirm(String),
}

pub struct Flow {
    goal: Goal,
    uid: String,
    name: String,
    step: Step,
    buf: String,
    /// Dernier code tapé faux : l'attente qui suit le dit (« Code incorrect. Réessaie dans… »).
    wrong: bool,
}

impl Flow {
    /// Genre du code en cours de saisie.
    fn kind(&self) -> String {
        match (&self.step, &self.goal) {
            (Step::Check, _) => kind(&stored(&self.uid)).to_string(),
            (_, Goal::Set(k)) => k.clone(),
            _ => String::new(),
        }
    }
}

fn stored(uid: &str) -> String {
    config::accounts().into_iter().find(|a| a.user_id == uid).map(|a| a.lock).unwrap_or_default()
}

/// Le compte a-t-il un code ?
pub fn locked(uid: &str) -> bool {
    !kind(&stored(uid)).is_empty()
}

/// Demande le code du compte `uid` avant `goal` (Ouvrir, Retirer). `false` : pas de code, l'action
/// peut se faire tout de suite ; `true` : la fenêtre du code est ouverte, l'action suivra.
pub fn guard(app: &Arc<App>, uid: &str, goal: Goal) -> bool {
    if !locked(uid) {
        return false;
    }
    let name = config::accounts().into_iter().find(|a| a.user_id == uid).map(|a| a.user_name).unwrap_or_default();
    *app.lock_flow.lock().unwrap() = Some(Flow { goal, uid: uid.to_string(), name, step: Step::Check, buf: String::new(), wrong: false });
    show(app, "", false);
    true
}

/// Paramètres : choisir un nouveau code (`new_kind` : « pin », « keys ») ou le retirer (vide), pour le
/// compte `uid`. Le code actuel est demandé d'abord, s'il y en a un.
pub fn change(app: &Arc<App>, uid: &str, new_kind: &str) {
    // Plus de nouveau code à chiffres (voir choices).
    if uid.is_empty() || new_kind == "pin" || (new_kind.is_empty() && !locked(uid)) {
        return;
    }
    let name = config::accounts().into_iter().find(|a| a.user_id == uid).map(|a| a.user_name).unwrap_or_default();
    let step = if locked(uid) { Step::Check } else { Step::New };
    *app.lock_flow.lock().unwrap() = Some(Flow { goal: Goal::Set(new_kind.to_string()), uid: uid.to_string(), name, step, buf: String::new(), wrong: false });
    show(app, "", false);
}

/// Titre de la fenêtre selon l'étape.
fn title(f: &Flow) -> String {
    match (&f.step, &f.goal) {
        (Step::Check, Goal::Open) => trf("Code de {}", &[&f.name]),
        (Step::Check, Goal::Forget) => trf("Retirer {} : code du compte", &[&f.name]),
        (Step::Check, Goal::Set(_)) => tr("Code actuel du compte").to_string(),
        (Step::New, _) if f.kind() == "keys" => tr("Nouvelle combinaison : 6 flèches").to_string(),
        (Step::New, _) => tr("Nouveau code : 4 chiffres").to_string(),
        (Step::Confirm(_), _) => tr("Encore une fois, pour confirmer").to_string(),
    }
}

/// Met la fenêtre à jour (ouverte s'il y a une saisie en cours, fermée sinon).
fn show(app: &Arc<App>, msg: &str, error: bool) {
    let Some(u) = app.ui().upgrade() else { return };
    let flow = app.lock_flow.lock().unwrap();
    let Some(f) = flow.as_ref() else {
        u.set_lock_open(false);
        u.set_refocus(u.get_refocus() + 1);
        return;
    };
    let kind = f.kind();
    let mut msg = msg.to_string();
    let mut error = error;
    // Attente en cours (trop d'erreurs) : dite dès l'ouverture, recomptée chaque seconde (« tick »).
    let until = if f.step == Step::Check { config::accounts().into_iter().find(|a| a.user_id == f.uid).map(|a| a.lock_until).unwrap_or(0) } else { 0 };
    let waiting = until > now();
    if waiting && msg.is_empty() {
        let left = until - now();
        msg = if f.wrong { trf("Code incorrect. Réessaie dans {} s.", &[&left]) } else { trf("Trop d'essais : réessaie dans {} s.", &[&left]) };
        error |= f.wrong;
    }
    if msg.is_empty() && f.step == Step::New && kind == "keys" {
        msg = tr("Avec les flèches du clavier, de la télécommande ou ci-dessous.").to_string();
    }
    let was_open = u.get_lock_open();
    u.set_lock_title(title(f).into());
    u.set_lock_kind(kind.as_str().into());
    u.set_lock_len(len(&kind) as i32);
    u.set_lock_count(f.buf.split(',').filter(|k| !k.is_empty()).count() as i32);
    u.set_lock_msg(msg.into());
    u.set_lock_error(error);
    u.set_lock_wait(waiting);
    u.set_lock_forgot(f.step == Step::Check && f.goal != Goal::Forget);
    if !was_open {
        u.set_lock_sel(4);
    }
    u.set_lock_open(true);
}

/// Touche de la fenêtre : chiffre, « up » / « down » / « left » / « right », « back », « cancel »,
/// « forgot ».
pub fn key(app: &Arc<App>, k: &str) {
    // Caractère tapé au clavier : le chiffre qu'il désigne, sinon rien.
    let k = match k.strip_prefix("type:") {
        Some(t) => match typed_digit(t) {
            Some(d) => d,
            None => return,
        },
        None => k,
    };
    let mut flow = app.lock_flow.lock().unwrap();
    let Some(f) = flow.as_mut() else { return };
    match k {
        "cancel" => {
            *flow = None;
            drop(flow);
            return show(app, "", false);
        }
        "forgot" => {
            let (uid, name) = (f.uid.clone(), f.name.clone());
            *flow = None;
            drop(flow);
            show(app, "", false);
            forgot(app, &uid, &name);
            return;
        }
        // Chaque seconde pendant une attente : le temps restant ; l'attente finie, le message s'en va.
        "tick" => {
            let until = config::accounts().into_iter().find(|a| a.user_id == f.uid).map(|a| a.lock_until).unwrap_or(0);
            if until <= now() {
                f.wrong = false;
            }
            drop(flow);
            return show(app, "", false);
        }
        "back" => {
            let mut keys: Vec<&str> = f.buf.split(',').filter(|k| !k.is_empty()).collect();
            keys.pop();
            f.buf = keys.join(",");
            drop(flow);
            return show(app, "", false);
        }
        _ => {}
    }
    let kind = f.kind();
    let ok = if kind == "keys" { matches!(k, "up" | "down" | "left" | "right") } else { k.len() == 1 && k.as_bytes()[0].is_ascii_digit() };
    if !ok {
        return;
    }
    // Attente imposée : rien n'est pris.
    if f.step == Step::Check {
        let until = config::accounts().into_iter().find(|a| a.user_id == f.uid).map(|a| a.lock_until).unwrap_or(0);
        if until > now() {
            drop(flow);
            return show(app, "", true);
        }
    }
    if !f.buf.is_empty() {
        f.buf.push(',');
    }
    f.buf.push_str(k);
    if f.buf.split(',').count() < len(&kind) {
        drop(flow);
        return show(app, "", false);
    }
    // Code complet.
    let code = std::mem::take(&mut f.buf);
    match f.step.clone() {
        Step::Check => {
            if verify(&stored(&f.uid), &code) {
                config::update_account(&f.uid, |a| {
                    a.lock_fails = 0;
                    a.lock_until = 0;
                });
                match f.goal.clone() {
                    Goal::Set(new_kind) if new_kind.is_empty() => {
                        config::set_lock(&f.uid, "");
                        *flow = None;
                        drop(flow);
                        show(app, "", false);
                        return done(app, tr("Code du compte retiré."));
                    }
                    Goal::Set(_) => {
                        f.step = Step::New;
                        drop(flow);
                        return show(app, "", false);
                    }
                    goal => {
                        let uid = f.uid.clone();
                        *flow = None;
                        drop(flow);
                        show(app, "", false);
                        return unlocked(app, &uid, goal);
                    }
                }
            }
            f.wrong = true;
            let mut fails = 0;
            config::update_account(&f.uid, |a| {
                a.lock_fails += 1;
                fails = a.lock_fails;
                a.lock_until = now() + delay(a.lock_fails);
            });
            drop(flow);
            // Attente : `show` écrit le message et le recompte.
            let msg = if delay(fails) > 0 { String::new() } else { tr("Code incorrect.").to_string() };
            shake(app);
            show(app, &msg, true);
        }
        Step::New => {
            f.step = Step::Confirm(code);
            drop(flow);
            show(app, "", false);
        }
        Step::Confirm(first) => {
            if first != code {
                f.step = Step::New;
                drop(flow);
                shake(app);
                return show(app, tr("Les deux codes ne sont pas les mêmes. Recommence."), true);
            }
            let kind = f.kind();
            config::set_lock(&f.uid, &make(&kind, &code));
            *flow = None;
            drop(flow);
            show(app, "", false);
            done(app, if kind == "keys" { tr("Combinaison enregistrée.") } else { tr("Code enregistré.") });
        }
    }
}

/// Chiffre désigné par un caractère tapé : chiffre (rangée du haut avec Maj, pavé numérique) ou
/// rangée du haut d'un clavier AZERTY sans Maj (français : & é " ' ( - è _ ç à ; belge : § et !).
pub fn typed_digit(t: &str) -> Option<&'static str> {
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    let d = match t {
        "&" => 1,
        "é" => 2,
        "\"" => 3,
        "'" => 4,
        "(" => 5,
        "-" | "§" => 6,
        "è" => 7,
        "_" | "!" => 8,
        "ç" => 9,
        "à" => 0,
        _ => return DIGITS.iter().find(|d| **d == t).copied(),
    };
    Some(DIGITS[d])
}

fn shake(app: &Arc<App>) {
    if let Some(u) = app.ui().upgrade() {
        u.set_lock_shake(u.get_lock_shake() + 1);
    }
}

/// Code changé dans les paramètres : message, ligne des paramètres à jour.
fn done(app: &Arc<App>, msg: &str) {
    if let Some(u) = app.ui().upgrade() {
        u.set_toast(msg.into());
    }
    crate::refresh_settings(app);
}

/// Bon code : l'action demandée.
fn unlocked(app: &Arc<App>, uid: &str, goal: Goal) {
    match goal {
        Goal::Open => {
            *app.lock_pass.lock().unwrap() = Some(uid.to_string());
            crate::login_pick(app, format!("acc:{uid}"));
        }
        Goal::Forget => crate::forget_saved_account(app, uid),
        Goal::Set(_) => {}
    }
}

/// « Code oublié ? » : le mot de passe du compte sur le serveur ; s'il est bon, le code est retiré
/// et le compte s'ouvre (voir start_session).
fn forgot(app: &Arc<App>, uid: &str, name: &str) {
    *app.lock_reset.lock().unwrap() = Some(uid.to_string());
    let Some(u) = app.ui().upgrade() else { return };
    u.set_error_text("".into());
    u.set_login_pass("".into());
    u.set_login_title(trf("Code oublié : mot de passe de {}", &[&name]).into());
    u.set_login_user(name.into());
    u.set_login_field(1);
    u.set_login_mode("form".into());
}

/// Passe-droit d'une seule fois : le compte `uid` s'ouvre sans code (bon code tapé, ou ouverture au
/// lancement de l'appli).
pub fn take_pass(app: &Arc<App>, uid: &str) -> bool {
    let mut p = app.lock_pass.lock().unwrap();
    if p.as_deref() == Some(uid) {
        *p = None;
        return true;
    }
    false
}

pub fn give_pass(app: &Arc<App>, uid: &str) {
    *app.lock_pass.lock().unwrap() = Some(uid.to_string());
}

/// Ligne des paramètres : genre du code actuel, en clair.
pub fn label(uid: &str) -> String {
    match kind(&stored(uid)) {
        "pin" => tr("Code à 4 chiffres").to_string(),
        "keys" => tr("Combinaison de 6 flèches").to_string(),
        _ => tr("Aucun").to_string(),
    }
}

/// Choix proposés dans les paramètres (genre, libellé) et le genre actuel. Plus de nouveau code à
/// chiffres : à la télécommande, la touche choisie du pavé se voit à l'écran (quelqu'un dans la pièce
/// lirait le code). Un code à chiffres déjà enregistré reste valable et reste dans la liste tant
/// qu'il n'est pas changé.
pub fn choices(uid: &str) -> (Vec<(String, String)>, String) {
    let cur = kind(&stored(uid)).to_string();
    let mut v = vec![(String::new(), tr("Aucun").to_string())];
    if cur == "pin" {
        v.push(("pin".into(), tr("Code à 4 chiffres").to_string()));
    }
    v.push(("keys".into(), tr("Combinaison de 6 flèches").to_string()));
    (v, cur)
}

/// Interface : la touche de la fenêtre arrive ici.
pub fn wire(app: &Arc<App>, ui: &AppWindow) {
    ui.on_login_locked(|id| id.strip_prefix("acc:").is_some_and(locked));
    let app = app.clone();
    ui.on_lock_key(move |k| key(&app, k.as_str()));
}

#[cfg(test)]
mod tests {
    #[test]
    fn hash_and_delay() {
        let s = super::make("pin", "1,2,3,4");
        assert_eq!(super::kind(&s), "pin");
        assert!(!s.contains("1,2,3,4"));
        assert!(super::verify(&s, "1,2,3,4"));
        assert!(!super::verify(&s, "1,2,3,5"));
        assert!(!super::verify("", "1,2,3,4"));
        assert_ne!(super::make("pin", "1,2,3,4"), s, "sel aléatoire");
        assert_eq!(super::kind(""), "");
        assert_eq!(super::kind("garbage"), "");
        assert_eq!((super::delay(4), super::delay(5), super::delay(6), super::delay(40)), (0, 30, 60, 3600));
        assert_eq!(super::typed_digit("&"), Some("1"));
        assert_eq!(super::typed_digit("à"), Some("0"));
        assert_eq!(super::typed_digit("7"), Some("7"));
        assert_eq!(super::typed_digit("a"), None);
    }
}
