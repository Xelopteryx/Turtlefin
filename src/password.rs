//! Solidité d'un mot de passe, sans rien envoyer nulle part : « 12345 » (le mot de passe donné par
//! défaut aux nouveaux comptes), le nom du compte, une suite (« abcdef », « 987654 ») ou un mot de
//! passe trop court sont « trop simples ». Le mot de passe n'est jamais gardé : seul le verdict
//! (trop simple ou non) est retenu pour le compte (voir config::Account::weak_pw).

/// Mots de passe parmi les plus courants (en minuscules). Les variantes avec un chiffre à la fin
/// (« azerty1 ») sont reconnues aussi (voir `common`).
const COMMON: &[&str] = &[
    "0000", "1111", "1234", "12345", "123456", "1234567", "12345678", "123456789", "1234567890",
    "000000", "111111", "121212", "123123", "654321", "666666", "696969", "112233", "159753",
    "password", "passw0rd", "motdepasse", "mdp", "azerty", "azertyuiop", "qwerty", "qwertyuiop",
    "qwertz", "abc123", "admin", "administrateur", "root", "user", "guest", "test", "jellyfin",
    "turtlefin", "changeme", "welcome", "bienvenue", "bonjour", "soleil", "doudou", "loulou",
    "chouchou", "marseille", "paris", "iloveyou", "jetaime", "football", "dragon", "monkey",
    "letmein", "master", "secret", "princesse", "nicolas", "thomas", "superman", "batman",
    "pokemon", "naruto", "starwars", "netflix", "film", "films", "cinema", "famille",
];

/// Solidité de 0 à 4 : 0 trop simple · 1 faible · 2 correct · 3 bon · 4 solide.
pub fn score(user: &str, pw: &str) -> u8 {
    let n = pw.chars().count();
    if n == 0 {
        return 0;
    }
    let low = pw.to_lowercase();
    let name = user.trim().to_lowercase();
    if common(&low) {
        return 0;
    }
    // Le nom du compte (ou l'inverse : le mot de passe dans le nom).
    if !name.is_empty() && (low == name || (name.chars().count() >= 3 && low.contains(&name)) || (n >= 3 && name.contains(&low))) {
        return 0;
    }
    let mut distinct: Vec<char> = low.chars().collect();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() <= 2 || (distinct.len() <= 3 && n < 12) || sequence(&low) {
        return 0;
    }
    let classes = [
        pw.chars().any(|c| c.is_lowercase()),
        pw.chars().any(|c| c.is_uppercase()),
        pw.chars().any(|c| c.is_ascii_digit()),
        pw.chars().any(|c| !c.is_alphanumeric()),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    let digits_only = pw.chars().all(|c| c.is_ascii_digit());
    let s = if n < 6 {
        0
    } else if n < 8 || (digits_only && n < 10) {
        1
    } else if n >= 16 || (n >= 12 && classes >= 3) {
        4
    } else if (n >= 12 && classes >= 2) || (n >= 10 && classes >= 3) {
        3
    } else if classes >= 2 || n >= 10 {
        2
    } else {
        1
    };
    s
}

/// Trop simple pour être gardé (l'interface demande de le changer).
pub fn is_weak(user: &str, pw: &str) -> bool {
    score(user, pw) <= 1
}

/// Mot de passe de la liste, éventuellement suivi de chiffres ou d'un « ! » (« azerty123 », « soleil! »).
fn common(low: &str) -> bool {
    let core = low.trim_end_matches(|c: char| c.is_ascii_digit() || c == '!' || c == '.');
    COMMON.contains(&low) || (!core.is_empty() && COMMON.contains(&core) && low.len() - core.len() <= 4)
}

/// Suite régulière : « abcdef », « 13579 », « 987654 », « aaaa »...
fn sequence(low: &str) -> bool {
    let c: Vec<i64> = low.chars().map(|c| c as i64).collect();
    if c.len() < 4 {
        return false;
    }
    let d = c[1] - c[0];
    d.abs() <= 2 && c.windows(2).all(|w| w[1] - w[0] == d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_ones() {
        for pw in ["", "12345", "123456", "azerty", "Azerty123", "password", "test", "aaaaaaa", "abcdefgh", "98765432", "13579135", "soleil!", "12121212"] {
            assert!(is_weak("marie", pw), "{pw} devrait être trop simple");
        }
        assert!(is_weak("marie", "marie"));
        assert!(is_weak("marie", "Marie2024"));
        assert!(is_weak("marie", "8462913")); // chiffres seuls, trop court
    }

    #[test]
    fn strong_enough() {
        for pw in ["Tortue-Volante", "chat noir 42", "K7#mq9Lp", "pluie-sur-la-mer", "Gr4ndB1sc0tte"] {
            assert!(!is_weak("marie", pw), "{pw} devrait passer");
        }
        assert_eq!(score("marie", "pluie-sur-la-mer"), 4);
        assert!(score("marie", "K7#mq9Lp") >= 2);
    }
}
