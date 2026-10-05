# Turtlefin

**Client Jellyfin natif, léger et animé, pensé pour la télé.** Écrit en Rust avec Slint (interface) et libmpv
(lecture) : pas de Qt, pas de navigateur embarqué. Il tourne sur Windows et sur Linux, en particulier sur un
Raspberry Pi 5 branché à une télé, où il remplace Jellyfin Desktop (trop gourmand, il finissait par planter).

Version actuelle : **0.4.0**.

## Ce qu'il sait faire

- **Comptes** : écran « Qui regarde ? » avec avatars (GIF animés compris), jusqu'à 12 comptes enregistrés sur
  l'appareil (le jeton seulement, jamais le mot de passe), changement de compte sans ressaisie, « Gérer les comptes »
  pour en retirer. Recherche des serveurs sur le réseau local et Tailscale, adresse locale et distante du même serveur.
- **Accueil** : Mes médias, Reprendre, À suivre, Récemment ajouté ; onglets Favoris et Demandes (Seerr).
  Affiches avec épisodes restants, coche « vu », note ; fond d'écran tiré du média sélectionné.
- **Fiches** : film, série, saison, épisode ; lecture, favori, vu, téléchargement, choix audio / sous-titres retenu
  par série ; « Plus de ce genre » et suggestions Seerr ; demande des saisons manquantes d'une série.
- **Lecture** (libmpv) : commandes à la télécommande, chapitres, épisodes de la saison, « Passer l'intro »,
  épisode suivant, suggestions en fin de série ; position et « vu » renvoyés au serveur.
- **Watch party** (SyncPlay) : regarder la même chose en même temps sur plusieurs appareils.
- **Hors ligne** : téléchargements présentés comme l'accueil, fiches complètes sans serveur ; les lectures,
  « vu » et favoris faits hors ligne sont renvoyés au compte à la reconnexion.
- **Recherche** (bibliothèque + Seerr), média au hasard, clavier à l'écran pour la télé.
- **Paramètres** : langues audio et sous-titres, taille des sous-titres, épisode suivant automatique, intro
  passée automatiquement, interface TV, fond d'écran, notes, heure, cache d'images, **mise à jour depuis GitHub**.
- Transitions animées partout (affiche qui vole vers la fiche, menu qui glisse, rangées en cascade).

## Installer

Turtlefin se compile sur l'appareil (le bouton de mise à jour se sert ensuite de ce même dossier).

### Raspberry Pi / Debian / Ubuntu

```sh
sudo apt install git build-essential pkg-config libfontconfig1-dev libxkbcommon-dev libmpv2 libmpv-dev
curl https://sh.rustup.rs -sSf | sh          # Rust (puis ouvrir un nouveau terminal)
git clone https://github.com/Xelopteryx/Turtlefin.git ~/turtlefin
cd ~/turtlefin && cargo build --release      # premier build : une dizaine de minutes sur un Pi 5
./target/release/turtlefin --tv
```

### Windows

1. Installer Rust (https://rustup.rs) et les « Outils de build Visual Studio » (charge de travail C++), puis git.
2. `git clone https://github.com/Xelopteryx/Turtlefin.git` puis `cargo build --release` dans le dossier.
3. Télécharger `mpv-dev-x86_64-….7z` sur https://github.com/shinchiro/mpv-winbuild-cmake/releases et placer
   `libmpv-2.dll` à côté de `target/release/turtlefin.exe`.

### Mettre à jour

**Paramètres → À propos → Rechercher une mise à jour** compare la version installée à GitHub, puis
« Mettre à jour » télécharge et recompile (quelques minutes sur un Pi ; l'appli reste utilisable), et
« Redémarrer Turtlefin » lance la nouvelle version. À la main : `git pull && cargo build --release`.

## Lancer

```
turtlefin                                  écran de connexion, ou dernière session
turtlefin --tv                             plein écran, grands éléments (télé)
turtlefin --desktop                        fenêtre (prime sur le réglage « Interface TV »)
turtlefin "Nom" --server=http://… --tv      connexion directe (mot de passe : TURTLEFIN_PASSWORD=…)
```

## Touches (télécommande ou clavier)

- **Flèches** pour se déplacer, **Entrée** pour ouvrir / activer, **Échap** ou **Retour arrière** pour revenir.
- Accueil : **←** sur la première carte (ou Retour) ouvre le menu ; dans le menu, **→** ou Échap le referme.
- ↑ depuis le haut d'une page : barre du haut (retour, accueil, menu, watch party, hasard, recherche, compte).
- Lecture, commandes masquées : ← → reculer / avancer de 10 s, ↑ ↓ ou Entrée affichent les commandes,
  ↓ depuis les boutons : épisodes de la saison. Espace : pause · `a` audio · `s` sous-titres · `f` plein écran.

## Fichiers

| Où | Quoi |
|---|---|
| dossier de config de l'OS, `turtlefin/` | `session.json` (session en cours), `accounts.json` (comptes enregistrés), `prefs.json` (réglages de l'appareil), `tracks.json` (pistes par série), `userdata.json` (vu / favoris faits hors ligne) |
| dossier de données, `turtlefin/downloads/` | téléchargements (média, affiches, fond, logo, `info.json`) |
| dossier de cache, `turtlefin/img/` | images (vidable dans À propos) |

Sous Linux : `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Sous Windows : `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.

## Diagnostic

- `TURTLEFIN_DEBUG_FRAMES=1` : signale les images longues à dessiner et le temps de chargement des fiches.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log` : journal de mpv · `TURTLEFIN_MPV_ARGS="…"` : options mpv en plus.
- `TURTLEFIN_HWDEC=auto-copy` : décodage matériel (logiciel par défaut sur le Pi) · `TURTLEFIN_AO=alsa` : sortie son.
- `TURTLEFIN_LIBMPV=chemin` : autre emplacement de libmpv · `TURTLEFIN_CONFIG_DIR=dossier` : autre dossier de
  session (essais) · `turtlefin --test-video=fichier` : lecteur sans serveur.
- Le rendu doit être OpenGL (choisi automatiquement) : avec `SLINT_BACKEND=winit-software`, pas de vidéo.

## Développement

Voir `HANDOFF.md` (état détaillé du projet, décisions, problèmes connus).
