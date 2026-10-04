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

## Lecture : libmpv requise

Turtlefin lit les vidéos avec **libmpv** (le moteur de mpv, sous forme de bibliothèque), chargée au moment de
la lecture. La vidéo est dessinée dans la fenêtre de Turtlefin et les commandes Slint s'affichent par-dessus.
Turtlefin rapporte au serveur le début, la progression (toutes les 10 s) et la fin de la lecture :
« Reprendre » et « vu » sont donc à jour partout.

- Windows : télécharge `mpv-dev-x86_64-….7z` sur https://github.com/shinchiro/mpv-winbuild-cmake/releases,
  et place `libmpv-2.dll` (dans l'archive) à côté de `turtlefin.exe` (dossier `target/release`).
- Linux / Pi : `sudo apt install libmpv2` (le programme `mpv` n'est plus nécessaire).
- Autre emplacement : variable `TURTLEFIN_LIBMPV=chemin/vers/libmpv`.
- Le rendu de Slint doit être OpenGL (femtovg, choisi automatiquement) : avec `SLINT_BACKEND=winit-software`,
  la lecture est impossible.
- Le bouton Lecture sur une série lance le prochain épisode ; sur une saison, le premier épisode non vu.
  En fin d'épisode, la lecture enchaîne sur le suivant.
- Les sous-titres externes (srt/ass/vtt) sont ajoutés automatiquement.

### Commandes pendant la lecture

En haut à gauche : bouton **Retour** (quitte la vidéo), titre et épisode.
En bas : temps écoulé et durée, barre de temps (repères de chapitres, cliquable et déplaçable à la souris),
⏮ épisode précédent · chapitre précédent · Lecture/Pause · chapitre suivant · ⏭ épisode suivant,
**Audio** et **Sous-titres** à droite (menus des pistes : point blanc = piste active), heure de fin au centre.
Les commandes se masquent après 3 s sans activité (sauf en pause ou menu ouvert).

- Commandes masquées : ← → reculer / avancer de 10 s (sans rien afficher) · ↑ ↓ affichent les commandes,
  barre de temps sélectionnée · Entrée les affiche sur Lecture/Pause · Échap/Retour arrière quitte la vidéo.
- Barre de temps : ← → déplacer le curseur de 10 s · ↓ boutons (Lecture/Pause) · ↑ bouton Retour.
- Boutons : ← → changer de bouton · Entrée activer · ↑ barre de temps · Échap masquer les commandes.
- Menu des pistes : ↑ ↓ choisir · Entrée valider · Échap ou ← fermer.
- Partout : Espace pause · `a` menu audio · `s` menu sous-titres · `f` ou F11 plein écran · `q` quitter la vidéo.
- Souris : bouger affiche les commandes · clic sur la vidéo = pause · double-clic = plein écran.

## Diagnostic de la lecture

- `turtlefin --test-video=chemin/vers/video.mkv` : essai du lecteur sans serveur.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log` : enregistre le journal détaillé de mpv (à lire avec `tail -n 80 /tmp/mpv.log`).
- `TURTLEFIN_MPV_ARGS="--hwdec=no --profile=fast"` : ajoute des options à mpv sans recompiler.
- `TURTLEFIN_HWDEC=auto-copy` (ou autre valeur de `--hwdec`) force le décodage matériel. Par défaut : logiciel sur
  Raspberry Pi / Linux ARM 64 bits (le décodage matériel V4L2 y donnait un écran vide), `auto-safe` ailleurs.
- Son sous Linux : PipeWire/PulseAudio (puis ALSA) avec le périphérique « auto ». `TURTLEFIN_AO=alsa` choisit un
  autre pilote ; `TURTLEFIN_AO=` (vide) laisse mpv décider. Le `mpv.conf` de l'utilisateur n'est pas lu.
- Le cache réseau de mpv est plafonné (100 Mo en avant, 25 Mo en arrière).

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
- `SLINT_BACKEND=winit-software` : force le rendu logiciel si le GPU pose problème (la lecture vidéo devient alors impossible)

## Pas encore là

Enchaînement automatique des épisodes · grille paginée des bibliothèques · recherche et réglages (M4) · fond (backdrop) de la fiche, en option.
