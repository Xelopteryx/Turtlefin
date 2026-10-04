# Turtlefin (M0 → M3b)

Client Jellyfin natif en Rust + Slint. Pas de Qt, pas de navigateur embarqué.

## Lancer

    cargo run --release

Premier build (ou après changement de dépendances) : plusieurs minutes.
Prérequis Windows : Rust + « Outils de build Visual Studio » (charge de travail C++).
Prérequis Linux : Rust, un compilateur C, `pkg-config`, `libfontconfig1-dev`, `libxkbcommon-dev`
(Debian/Raspberry Pi OS : `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev`).

> reqwest 0.13 utilise rustls avec aws-lc-rs. Si ce build échoue sur ta machine (outil C/cmake manquant),
> remets dans Cargo.toml :
>     reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls-native-roots"] }
>     directories = "5"
> (la 0.12 n'a pas besoin de la feature "query" : elle est incluse.)

## Lecture (M3) : mpv requis

Turtlefin lance **mpv** dans un processus séparé (le flux est lu en lecture directe) et le pilote par IPC.
Il rapporte au serveur le début, la progression (toutes les 10 s) et la fin de la lecture :
« Reprendre » et « vu » sont donc à jour partout.

- Windows : télécharge mpv (mpv.io), puis mets `mpv.exe` à côté de `turtlefin.exe` ou dans le PATH.
- Linux / Pi : `sudo apt install mpv`.
- Autre emplacement : variable `TURTLEFIN_MPV=chemin/vers/mpv`.
- Le bouton Lecture sur une série lance le prochain épisode ; sur une saison, le premier épisode non vu.
- **Vidéo intégrée** : mpv dessine dans la fenêtre de Turtlefin (HWND sous Windows, XID sous X11).
  Sous Wayland ce n'est pas possible : mpv s'ouvre alors dans sa propre fenêtre (un message l'indique).
  `TURTLEFIN_EMBED=0` force l'ancien comportement (fenêtre mpv séparée).
- Touches pendant la lecture (reçues par Turtlefin, donc valables aussi avec une télécommande) :
  Espace/Entrée pause · ← → saut de 10 s · ↑ ↓ volume · `a` piste audio · `s` sous-titres ·
  `f` ou F11 plein écran de la fenêtre Turtlefin · Échap/Retour arrière/`q` arrêter.
  Si tu cliques sur la vidéo, mpv reçoit le clavier directement (ses touches habituelles marchent aussi).
- Limite du mode intégré : rien de Turtlefin ne peut s'afficher PAR-DESSUS la vidéo (la barre de mpv fait office de commandes).
- Les sous-titres externes (srt/ass/vtt) sont ajoutés automatiquement.
- Le jeton d'accès figure dans la ligne de commande de mpv (visible des autres processus de la machine).

## Ligne de commande

    turtlefin                                    # écran de connexion, ou session sauvegardée
    turtlefin "Cody" "mot de passe" --server=https://mon.serveur --tv

    --tv         plein écran, tailles agrandies
    --desktop    mode fenêtre (défaut)
    --server=URL adresse du serveur
    TURTLEFIN_PASSWORD=...   alternative au mot de passe en argument
                             (un argument est visible dans la liste des processus)

## Touches

Accueil : flèches pour naviguer · Entrée ou clic : ouvrir la fiche.

Fiche :
- ← → : changer de bouton / de carte · ↑ ↓ : changer de zone (boutons, résumé, saisons/épisodes)
- Entrée : activer (bouton, ouvrir le résumé complet, ouvrir une saison/un épisode)
- Échap ou Retour arrière : revenir en arrière (pile de navigation)
- Résumé complet ouvert : ↑ ↓ PageHaut PageBas pour défiler, Échap/Entrée pour fermer

## Nouveautés de la fiche et de l'accueil

- Accueil : rangée « Mes médias » (bibliothèques, vignettes 16:9) en haut ; ouvrir une bibliothèque affiche ses 60 premiers éléments.
- Série : bloc « À suivre » en colonne à droite (→ depuis les boutons ou le résumé pour l'atteindre).
- Fiche film/épisode : langues audio dans la ligne d'infos.

## Fichiers écrits

- Session (serveur + jeton, jamais le mot de passe) : dossier de config de l'OS, `turtlefin/session.json`
- Cache d'images : dossier de cache de l'OS, `turtlefin/img/` (pas encore de purge automatique)

## Variables utiles

- `TURTLEFIN_INSECURE=1` : accepte n'importe quel certificat (test uniquement)
- `SLINT_BACKEND=winit-software` : force le rendu logiciel si le GPU pose problème

## Pas encore là

Enchaînement automatique des épisodes · grille paginée des bibliothèques · recherche et réglages (M4) · fond (backdrop) de la fiche, en option.
