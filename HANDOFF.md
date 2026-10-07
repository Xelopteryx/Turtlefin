# Turtlefin : passation de projet (état au 7 octobre 2026, version 0.9.0)

**Français** · [English](HANDOFF.en.md)

Document destiné à qui reprend le développement (humain ou Claude Code). Lis-le en entier avant de toucher au
code, puis lis [README.fr.md](README.fr.md) (usage, installation, touches, fichiers).
Dépôt : https://github.com/Xelopteryx/Turtlefin · Version dans `Cargo.toml` : 0.9.0.

## 1. Objectif

Un **client Jellyfin natif en Rust**, léger, animé et utilisable entièrement au clavier / à la télécommande,
installable sur n'importe quel ordinateur Windows ou Linux **sans rien compiler** : l'utilisateur télécharge un
installeur ou un paquet (ou lance une commande d'installation), c'est tout. Tous les paquets sont fabriqués par le
mainteneur (CI GitHub ou son PC), jamais par l'utilisateur.

Pourquoi : Jellyfin Desktop (Qt / QtWebEngine) fuit de la RAM et finit par planter sur les petites machines, et
l'interface web avec un thème chargé tombe sous 30 images/s sur du matériel modeste. Règle permanente : rester
stable en mémoire et ne jamais réintroduire de flou temps réel ni d'animation de filtre.

## 2. Décisions prises

| Sujet | Décision | Raison |
|---|---|---|
| Langage / UI | Rust + **Slint** 1.18 (rendu 100 % Slint), style `fluent-dark` imposé par `build.rs` | Pas de navigateur ; le style « native » dépendrait de Qt |
| Réseau | `reqwest` 0.13 (rustls, magasin de certificats du système), `tokio` | `query` est une feature à activer en 0.13 |
| Lecture | **libmpv chargée à l'exécution** (`libloading`, `src/mpv.rs`), rendu OpenGL dans une texture affichée par Slint (`src/video.rs`), commandes Slint par-dessus (`ui/player.slint`) | Lecteur intégré, pas d'IPC, marche sous Wayland. Le lecteur est recréé à chaque lecture (mémoire bornée) |
| Rendu Slint | femtovg (OpenGL / GLES) imposé, sauf si `SLINT_BACKEND` est défini | La vidéo passe par une texture OpenGL |
| Texture vidéo | Pixels physiques, origine `TopLeft`, état GL sauvegardé / rétabli autour de mpv ; `loadfile` attend le contexte de rendu | Sinon image à l'envers ou « No render context set » |
| Décodage | `hwdec=no` sous Linux ARM 64 bits, `auto-safe` ailleurs (Windows : `d3d11va-copy`) | Sur les cartes ARM testées (pilote v3d), le décodage V4L2 sort un format que le rendu ne sait pas importer |
| Audio Linux | `ao=pipewire,pulse,alsa`, `config=no` | Un `mpv.conf` utilisateur imposant ALSA échouait quand PipeWire tient la sortie HDMI |
| Mémoire | Cache mpv plafonné (100 / 25 MiB), images demandées à la bonne taille, 16 éléments par rangée | Petites machines (4 Go) |
| Langues | Textes écrits en français dans le code (langue source), traductions gettext dans `lang/<code>/LC_MESSAGES/turtlefin.po`, intégrées à la compilation | Voir section 6 |
| Fenêtre console (Windows) | Sous-système « windows » en release ; `--console` en rattache / ouvre une | Demande de l'utilisateur : pas de console sans option |
| Ligne de commande | Prime toujours sur les réglages (compte de démarrage, interface TV) | Demande de l'utilisateur |
| Mot de passe | Jamais enregistré (jeton seulement) ; en ligne de commande, préférer `TURTLEFIN_PASSWORD` | Un argument est visible des autres processus |

## 3. Structure du code

```
build.rs          commit compilé, style Slint, traductions intégrées, icône de l'exe (winresource, Windows)
lang/en/…/turtlefin.po   traductions anglaises (source : le français du code)
ui/theme.slint    jetons de thème, global Prefs
ui/app.slint      AppWindow et tous les écrans (boot, login, loading, home, detail, library, settings…)
ui/boot.slint     BootLogo : animation de démarrage (6 points, liaison, zoom), choix de la langue
ui/player.slint   écran de lecture ; ui/osk.slint clavier à l'écran ; ui/card.slint, ui/marquee.slint
src/main.rs       CLI, état partagé App (Arc), écrans, navigation (pile + pages gardées), paramètres
src/boot.rs       séquence de démarrage (vérifications, langue, choix de l'écran d'arrivée)
src/i18n.rs       langue courante, tr() / trf() côté Rust, langue du système / de l'installeur
src/api.rs        client Jellyfin REST (+ relais Seerr de Jellyfin Enhanced, GetAvatar)
src/config.rs     session, comptes (12 au plus), prefs.json (réglages de l'appareil), pistes, vu/favoris hors ligne
src/discovery.rs  recherche des serveurs (UDP, sous-réseaux, ARP, pairs VPN)
src/downloads.rs  téléchargements (reprise Range, file, synchronisation hors ligne)
src/mpv.rs        liaison libmpv ; src/video.rs texture OpenGL ; src/player.rs lecture, rapports, enchaînement
src/syncplay.rs   watch party (WebSocket /socket)
src/paths.rs      dossiers config / cache / données ; mode portable (fichier `portable` à côté de l'exe)
src/update.rs     mise à jour selon l'installation (Kind : Source, WinInstalled, WinPortable, AppImage, Deb)
packaging/        windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                  icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh (une commande)
.github/workflows/release.yml   compilation et publication sur étiquette `v*`
```

Principes :
- Les données réseau passent par des structures `Send`, puis `upgrade_in_event_loop` les pousse dans les modèles
  Slint. Images décodées hors du thread UI, 6 téléchargements en parallèle, appliquées avec une garde sur l'id.
- `App.gen` invalide les chargements périmés ; `App.stack` est la pile de navigation ; `PAGES` garde fiches et
  bibliothèques pour des retours sans requête.
- Navigation clavier faite à la main (indices de sélection en Rust), Slint ne gérant pas le focus de cartes
  dynamiques. `refocus` rend le clavier au bon `FocusScope`.
- Jellyfin 10.11 : `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`,
  en-tête `Authorization: MediaBrowser …, Token=…`. Rapports : `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` et le WebSocket refusent `api_key` : jeton dans l'en-tête.

## 4. Démarrage

`main` applique la langue (prefs.json, sinon fichier `language` écrit par l'installeur Windows), puis lance
`boot::run`. L'écran `boot` (logo hexagonal) montre 6 points, un par vérification : **langue** (demandée si
inconnue), **affichage** (OpenGL), **lecteur vidéo** (libmpv chargeable), **stockage** (écriture dans le dossier de
config), **réseau**, **serveur** (« à configurer » au premier lancement). Rouge = échec, avec un message et
« Continuer ». Tout vert : les points se relient aux couleurs du thème, puis zoom dans le point central.

Écran d'arrivée (`boot::route`), dans l'ordre : nom + mot de passe en ligne de commande → connexion ; nom d'un
compte enregistré → ce compte ; compte de démarrage (`prefs.autostart_user` / `autostart_server`, réglage
Compte → « Ouvrir ce compte au démarrage ») → `open_saved_session` ; sinon « Qui regarde ? » (ou la recherche de
serveur si aucun n'est connu). Un compte de démarrage disparu (oublié localement ou jeton refusé par le serveur)
mène à « Qui regarde ? ». `--no-intro` saute l'animation (les vérifications ont lieu quand même).

## 5. État

Toutes les fonctions listées dans le README sont faites et ont été vérifiées sur captures d'écran (PC Windows) et
sur une machine Linux ARM branchée à une télé, avec les comptes de test `test` / `test2` d'un vrai serveur.
Points notables, non évidents dans le code :
- **Image partagée** (`global Hero`) : l'image d'une carte vole vers l'affiche de la fiche et revient sur la carte
  exacte au retour (`Hero.want-id`, `hero-card-ok`). Les `changed` de Slint sont différés : positions en deux temps.
- **Rangées** (`global Rows`) : défilement propre à chaque rangée, mémorisé par clé ; changement de rangée vers la
  carte la plus proche à l'écran.
- **Hors ligne** : `config::Flags` (userdata.json) garde vu / favoris / positions avec un drapeau « à envoyer » ;
  `downloads::sync` les renvoie au retour du serveur (l'appareil a le dernier mot).
- **Adresses** : `server_main` / `server_backup` ; `watch_addresses` (20 s) bascule sur le secours et revient.
- **Watch party** : une connexion WebSocket par session (`sp_conn`), arrêt sur 401 / 403, délai croissant.
- **Avatars** : GIF décodés une fois, seul l'avatar sélectionné est animé ; images rondes fixes en cache disque.
  GetAvatar `SetAvatar` répond 500 → repli `POST /UserImage`.
- **Version 0.9.0** : paquets Windows / Linux et mise à jour par GitHub Releases. Vérifié en local : installeur x64
  (installé et portable, désinstallation), x86, AppImage aarch64, `.deb` arm64 (contenu). **La CI n'a jamais
  tourné** (rien de poussé) et la mise à jour par Releases n'a pas pu être essayée sans publication.
- **7 octobre 2026** : animation de démarrage, langues (français / anglais, ~400 textes), compte de démarrage,
  console seulement avec `--console`, logo = icône (exe, fenêtre, installeur, paquets Linux), installeur bilingue
  qui transmet sa langue, scripts d'installation en une commande, README / HANDOFF en deux langues.

Pas fait : Quick Connect ; manette ; fond flouté en option avec logos transparents ; licence (à choisir par le
mainteneur) ; passerelle XeLauncher (lanceur du média center du mainteneur, pas prioritaire).

## 6. Traductions

- Slint : `@tr("…")`, pluriels `@tr("{n} serveur" | "{n} serveurs" % n)`. Rust : `tr("…")` (renvoie
  `&'static str`) et `trf("… {} …", &[&x])`. Le texte français **est** la clé : le modifier demande de modifier le
  `msgid` du `.po`.
- Changer de langue : `i18n::set_language` (passe par `invoke_from_event_loop`, `select_bundled_translation`
  devant tourner sur le thread UI ; `""` = français).
- Ajouter une langue : copier `lang/en`, traduire les `msgstr`, ajouter le code à `i18n::LANGUAGES` et à
  `i18n::catalog`, et un `[Languages]` à `turtlefin.iss` si Inno Setup a la traduction.
- Les noms venant du serveur (bibliothèques, médias) ne sont pas traduits.

## 7. Compiler et fabriquer les paquets (mainteneur seulement)

Développement :
- Windows : Rust (https://rustup.rs), « Outils de build Visual Studio » (C++), git ; `cargo build --release` ;
  `libmpv-2.dll` (archive `mpv-dev-x86_64-….7z` de
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) à côté de l'exe.
  La version debug a besoin de `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu) : `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev` puis `cargo build --release`.

Publication :
- **Automatique** : `git tag v0.9.0 && git push origin v0.9.0`. `release.yml` compile Windows x64 / x86 et Linux
  x86_64 / aarch64, fabrique installeurs, archives, AppImage et `.deb`, et les publie dans une Release. Les noms de
  fichiers (en-tête de `release.yml`) sont attendus tels quels par `update.rs` et les scripts d'installation.
- **Windows à la main** : `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (ou
  `x86`) ; il faut Inno Setup 6, 7-Zip et NASM (x86). Résultat dans `target\dist`.
- **Linux à la main** (sur une machine Linux) : `TURTLEFIN_DIST=release cargo build --release`, puis
  `sh packaging/linux/build-appimage.sh <version>` et `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` à la compilation, sinon `update::kind()` croit à une version compilée sur place.
- x86 : les libmpv 32 bits de shinchiro publiées depuis juillet 2026 plantent au démarrage (OpenSSL) ;
  `build.ps1` fige celle du 10 juin 2026 (`MPV_TAG`). shinchiro ne garde qu'une trentaine de versions : la copier
  ailleurs avant qu'elle disparaisse. aws-lc demande NASM en 32 bits.
- Icônes : `packaging/turtlefin.svg` est le logo ; `packaging/icons/make-icons.py <dossier>` (Python + Pillow)
  régénère les PNG et l'ICO.

## 8. Essais

- `TURTLEFIN_CONFIG_DIR=<dossier>` : autre dossier de config (comptes, prefs) sans toucher au vrai.
- `--open=settings|downloads`, `--play=ID@SECONDES`, `--test-video=fichier` (lecteur sans serveur).
- `TURTLEFIN_DEBUG_FRAMES=1` (images > 25 ms), `SLINT_DEBUG_PERFORMANCE=refresh_full_speed,console`.
- Un `prefs.json` écrit par PowerShell 5 a un BOM : la lecture des fichiers de config l'ignore.

## 9. Problèmes connus / limites

1. Un plantage de mpv fait planter Turtlefin (même processus).
2. La lecture exige le rendu OpenGL (`SLINT_BACKEND=winit-software` l'empêche).
3. **mpv 0.40 / 0.41** (corrigé dans mpv le 23 janvier 2026, commit f74adc4) : une barrière OpenGL par image
   jamais libérée ; avec le pilote v3d chacune occupe un descripteur (« MESA: error: Export failed » après ~42 s).
   Contournement dans `src/mpv.rs` (OpenGL ES uniquement). Diagnostic : `ls /proc/$(pgrep -x turtlefin)/fd | wc -l`
   doit rester stable pendant la lecture.
4. Croissance RAM de mpv (~3 Mo/min) sur des sous-titres ASS : bornée à une lecture (mpv recréé à chaque vidéo).
5. Jeton en clair dans `session.json` / `accounts.json` (0600 sous Unix).
6. Déchirement d'image sous Xorg sans compositeur (Openbox seul) : utiliser un compositeur (picom
   `--backend egl --vsync`). Turtlefin tient 60 images/s.
7. Décodage logiciel sous Linux ARM : peut peiner en 4K / HEVC ; piste : `TURTLEFIN_HWDEC=auto-copy`.

## 10. Préférences de travail du mainteneur

- Réponses en français ; il connaît peu Linux / SSH : expliquer les commandes.
- Ne rien pousser sur GitHub avant sa validation explicite.
- Essais avec une copie de la config (`TURTLEFIN_CONFIG_DIR`), jamais la vraie ; comptes de test `test` / `test2`.
