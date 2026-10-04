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
- **Interface de lecture** dessinée dans la vidéo, aux couleurs du thème (`src/turtlefin_ui.lua`, chargé dans mpv) :
  barre avec titre, temps, barre de progression cliquable, boutons Lecture/Pause, -10 s, +10 s, Audio, Sous-titres,
  Arrêter ; menus des pistes audio / sous-titres (langue, titre, codec, canaux ; point blanc = piste active).
  Elle apparaît au mouvement de la souris ou à une touche, et se masque après 3 s (sauf en pause ou menu ouvert).
- Souris : survol + clic sur les boutons et les pistes, clic sur la vidéo = pause, molette = volume (ou défilement du menu).
- Touches (reçues par Turtlefin ou par mpv, donc valables aussi avec une télécommande) :
  - barre masquée : Entrée affiche la barre et sélectionne les boutons · Espace pause · ← → saut de 10 s ·
    ↑ ↓ volume · `a` menu audio · `s` menu sous-titres · Échap/Retour arrière arrête la lecture ;
  - boutons sélectionnés : ← → changer de bouton · Entrée activer · Échap masquer la barre ;
  - menu de pistes : ↑ ↓ choisir · Entrée valider · Échap ou ← fermer.
  - `f` ou F11 plein écran de la fenêtre · `q` quitter tout de suite.
- Limite du mode intégré : rien de Slint ne peut s'afficher PAR-DESSUS la vidéo, d'où l'interface dessinée par mpv.
- Les sous-titres externes (srt/ass/vtt) sont ajoutés automatiquement.
- Le jeton d'accès figure dans la ligne de commande de mpv (visible des autres processus de la machine).

## Diagnostic de la lecture

- `TURTLEFIN_MPV_LOG=/tmp/mpv.log` : enregistre le journal détaillé de mpv (à lire avec `tail -n 80 /tmp/mpv.log`).
- `TURTLEFIN_MPV_ARGS="--vo=x11 --hwdec=no"` : ajoute des options à mpv sans recompiler.
- `TURTLEFIN_EMBED=0` : mpv dans sa propre fenêtre (pour savoir si le souci vient de l'intégration).
- `TURTLEFIN_HWDEC=auto-safe` (ou autre valeur de `--hwdec`) force le décodage matériel. Par défaut : logiciel sur
  Raspberry Pi / Linux ARM 64 bits (le décodage matériel V4L2 y donne un écran vide avec Vulkan), `auto-safe` ailleurs.
- Son sous Linux : Turtlefin demande à mpv d'utiliser PipeWire/PulseAudio (puis ALSA) avec le périphérique « auto »,
  ce qui prime sur un `ao=alsa` de `~/.config/mpv/mpv.conf`. `TURTLEFIN_AO=alsa` choisit un autre pilote ;
  `TURTLEFIN_AO=` (vide) laisse la config de mpv décider.
- Le cache réseau de mpv est plafonné (100 Mo en avant, 25 Mo en arrière) : sa RAM monte puis se stabilise.

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
