# Turtlefin

**Client Jellyfin natif, léger et animé, pensé pour la télé.** Écrit en Rust avec Slint (interface) et libmpv
(lecture) : pas de Qt, pas de navigateur embarqué. Il tourne sur Windows et sur Linux, en particulier sur un
Raspberry Pi 5 branché à une télé, où il remplace Jellyfin Desktop (trop gourmand, il finissait par planter).

Version actuelle : **0.9.0**.

## Ce qu'il sait faire

- **Comptes** : écran « Qui regarde ? » avec avatars (GIF animés compris), jusqu'à 12 comptes enregistrés sur
  l'appareil (le jeton seulement, jamais le mot de passe), changement de compte sans ressaisie, « Gérer les comptes »
  pour en retirer. Recherche des serveurs sur tous les réseaux de l'appareil (local et VPN) ; une adresse
  principale et une adresse de secours, essayée quand la principale ne répond pas (Paramètres → Réseau).
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
- **Paramètres** : photo de profil (avatars du plugin GetAvatar, rangés par catégorie), langues audio et
  sous-titres, taille des sous-titres, épisode suivant automatique, intro passée automatiquement, interface TV,
  fond d'écran, notes, heure, adresses du serveur, cache d'images, **mise à jour depuis GitHub**.
- Transitions animées partout (affiche qui vole vers la fiche, menu qui glisse, rangées en cascade).

## Installer

Les versions prêtes à l'emploi sont sur la page
[Releases](https://github.com/Xelopteryx/Turtlefin/releases) : rien à compiler.

| Système | Fichier | Remarques |
|---|---|---|
| Windows 64 bits | `Turtlefin-<version>-windows-x64-setup.exe` | Installeur : choix du dossier, et du mode **installé** (menu Démarrer, désinstallation) ou **portable** |
| Windows 64 bits, sans installer | `Turtlefin-<version>-windows-x64-portable.zip` | Décompresser où l'on veut (clé USB...) et lancer `turtlefin.exe` |
| Windows 32 bits | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Pour les vieux PC |
| Linux, toutes distributions | `Turtlefin-<version>-linux-x86_64.AppImage` | Version portable : `chmod +x` puis lancer le fichier |
| Raspberry Pi (64 bits), Linux ARM | `Turtlefin-<version>-linux-aarch64.AppImage` | Idem |
| Debian, Ubuntu, Raspberry Pi OS | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Version installée : `sudo apt install ./turtlefin_….deb` |

Les versions Windows et AppImage contiennent tout (lecteur libmpv compris). Le paquet `.deb` utilise la libmpv
du système (`libmpv2`, installée automatiquement par apt). Les AppImage et paquets demandent une distribution de
2022 ou plus récente (Ubuntu 22.04, Debian 12, Raspberry Pi OS Bookworm…).

**Version portable** : un fichier `portable` à côté de `turtlefin.exe` fait garder la configuration, les comptes,
le cache et les téléchargements dans le dossier `data` à côté du programme ; rien n'est écrit ailleurs.

### Mettre à jour

**Paramètres → À propos → Rechercher une mise à jour** compare la version installée à la dernière publiée,
puis « Mettre à jour » s'occupe de tout selon la façon dont Turtlefin est installé :

| Installation | Mise à jour |
|---|---|
| Windows, installé | le nouvel installeur est téléchargé puis relancé en silence dans le même dossier |
| Windows, portable | la nouvelle archive est téléchargée et ses fichiers remplacent les anciens |
| AppImage | le nouveau fichier remplace l'ancien |
| Paquet .deb | le paquet est installé avec `pkexec` (le mot de passe administrateur est demandé) |
| Compilé depuis les sources | `git pull` puis recompilation dans le même dossier |

« Redémarrer Turtlefin » lance ensuite la nouvelle version.

### Compiler soi-même

Linux (Debian, Ubuntu, Raspberry Pi OS) :

```sh
sudo apt install git build-essential pkg-config libfontconfig1-dev libxkbcommon-dev libmpv2 libmpv-dev
curl https://sh.rustup.rs -sSf | sh          # Rust (puis ouvrir un nouveau terminal)
git clone https://github.com/Xelopteryx/Turtlefin.git ~/turtlefin
cd ~/turtlefin && cargo build --release      # premier build : une dizaine de minutes sur un Pi 5
./target/release/turtlefin --tv
```

Windows : installer Rust (https://rustup.rs), les « Outils de build Visual Studio » (charge de travail C++) et git,
puis `cargo build --release` ; placer `libmpv-2.dll` (archive `mpv-dev-x86_64-….7z` de
[shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) à côté de
`target/release/turtlefin.exe`.

### Fabriquer les paquets

- **Automatiquement** : pousser une étiquette de version (`git tag v0.9.0 && git push origin v0.9.0`). GitHub
  Actions (`.github/workflows/release.yml`) compile pour Windows x64 / x86 et Linux x86_64 / aarch64, fabrique
  tous les fichiers ci-dessus et les publie dans une Release.
- **À la main, Windows** : `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64`
  (ou `x86`). Il faut Inno Setup 6, 7-Zip et, pour x86, NASM. Résultat dans `target\dist`.
- **À la main, Linux** : `TURTLEFIN_DIST=release cargo build --release`, puis
  `sh packaging/linux/build-appimage.sh <version>` (AppImage) et `cargo deb --no-build` (paquet, avec cargo-deb).

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
| dossier de config, `turtlefin/` | `session.json` (session en cours), `accounts.json` (comptes enregistrés), `prefs.json` (réglages de l'appareil), `tracks.json` (pistes par série), `userdata.json` (vu / favoris faits hors ligne) |
| dossier de données, `turtlefin/downloads/` | téléchargements (média, affiches, fond, logo, `info.json`), `queue.json` (file en attente) |
| dossier de cache, `turtlefin/img/` | images (vidable dans À propos) |

Sous Linux : `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Sous Windows : `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Version portable : tout dans `data\` à côté de `turtlefin.exe` (`config`, `cache`, `downloads`).

## Diagnostic

- `TURTLEFIN_DEBUG_FRAMES=1` : signale les images longues à dessiner et le temps de chargement des fiches.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log` : journal de mpv · `TURTLEFIN_MPV_ARGS="…"` : options mpv en plus.
- `TURTLEFIN_HWDEC=auto-copy` : décodage matériel (logiciel par défaut sur le Pi) · `TURTLEFIN_AO=alsa` : sortie son.
- `TURTLEFIN_LIBMPV=chemin` : autre emplacement de libmpv · `TURTLEFIN_CONFIG_DIR=dossier` : autre dossier de
  session (essais) · `turtlefin --test-video=fichier` : lecteur sans serveur.
- Le rendu doit être OpenGL (choisi automatiquement) : avec `SLINT_BACKEND=winit-software`, pas de vidéo.

## Développement

Voir `HANDOFF.md` (état détaillé du projet, décisions, problèmes connus).
