# Turtlefin : passation de projet (état au 7 octobre 2026, version 0.9.1)

**Français** · [English](HANDOFF.en.md)

Document destiné à qui reprend le développement (humain ou Claude Code). Lis-le en entier avant de toucher au
code, puis lis [README.fr.md](README.fr.md) (usage, installation, touches, fichiers).
Dépôt : https://github.com/Xelopteryx/Turtlefin · Version dans `Cargo.toml` : 0.9.1.

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
| Interface TV (Windows) | Fenêtre sans bordure qui couvre l'écran + 1 px (`src/winfull.rs`, `set_tv_window`), pas le vrai plein écran ; suit les changements de définition (toutes les 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` pour le plein écran de Slint | En plein écran OpenGL, AMD Software prend l'appli pour un jeu : « Appuyez sur ALT + R » à chaque retour au premier plan |
| Fenêtre (Windows, bureau) | Réduite et centrée si 1280 x 720 + cadre dépasse la zone de travail | Écrans 1366 x 768 : le bas passait sous la barre des tâches |
| Fenêtre console (Windows) | Sous-système « windows » en release ; `--console` en rattache / ouvre une | Demande de l'utilisateur : pas de console sans option |
| Ligne de commande | Prime toujours sur les réglages (compte de démarrage, interface TV) | Demande de l'utilisateur |
| Mot de passe | Jamais enregistré (jeton seulement) ; en ligne de commande, préférer `TURTLEFIN_PASSWORD` | Un argument est visible des autres processus |

## 3. Structure du code

```
build.rs          commit compilé, style Slint, traductions intégrées, icône de l'exe (winresource, Windows)
lang/<code>.po    traductions (source : le français du code) ; tools/lang-check.py les vérifie
ui/theme.slint    jetons de thème, global Prefs
ui/app.slint      AppWindow et tous les écrans (boot, login, loading, home, detail, library, settings…)
ui/boot.slint     BootLogo : animation de démarrage (7 points, liaison, zoom), choix de la langue
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
`boot::run`. L'écran `boot` montre 7 points : les 6 sommets de l'hexagone puis le centre. Chacun est une vraie
vérification (`boot::check`) :
1. **langue** (demandée si inconnue) : les traductions de la langue choisie se chargent (`i18n::check`) ;
2. **affichage** : la fenêtre a obtenu un contexte OpenGL (`video::gl_info`, version et carte graphique au journal) ;
3. **lecteur vidéo** : un vrai lecteur mpv est créé, initialisé puis détruit (`mpv::self_test`) ;
4. **stockage** : écriture, relecture et suppression d'un fichier dans les dossiers de config, de données et de cache ;
5. **configuration** : `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` lisibles
   (`config::unreadable_files`, appelé au tout début de `main`, avant qu'un fichier abîmé soit réécrit) ;
6. **réseau** : interface active ou route vers l'extérieur (`discovery::has_network`) ;
7. **serveur** (le centre) : l'adresse principale, sinon de secours, répond à `/System/Info/Public` **et** c'est
   le même serveur (identifiant comparé à `server_id` de la session) ; « à configurer » au premier lancement.

Rouge = échec, avec un message et « Continuer » (seul au bout de 12 s). Tout vert : les sommets se relient, les
rayons partent vers le centre et le point du serveur devient l'hexagone plein — exactement le logo
(`packaging/turtlefin.svg`, même géométrie) —, puis zoom dans le centre. Attention : Slint réduit le dessin d'un
`Path` de l'épaisseur de son trait ; les chemins du logo sont agrandis d'autant pour tomber sur les points.

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

- **7 octobre 2026 (soir)** : lecture à la fréquence de l'écran (`Render::render` : mpv ne dessine que les
  nouvelles images, `BLOCK_FOR_TARGET_TIME=0` ; avant, l'interface était bridée à celle de la vidéo, 26 i/s) ;
  volume (bouton + barre, `prefs.volume`), icônes Audio / Sous-titres, appuis et panneaux animés ; fiche qui
  s'écarte au lancement de la lecture (`play-go`, `po`) ; fragmentation des textes pendant le choix de la langue
  (`Tr.fx`, `Tr.k`, `i18n::fragment`) ; icônes du menu et des catégories (`NavIcon`) ; visite guidée (`tour-step`,
  proposée au démarrage, `--tutorial`, Paramètres → À propos) ; point serveur orange tant qu'aucun serveur.

- **8 octobre 2026** : changement de langue (`dust.rs`, `ui/dust.slint`) — à l'ouverture de la liste des
  langues, une barre lumineuse parcourt chaque ligne de texte et la recouvre de blanc ; au choix, les blocs
  prennent la largeur des nouveaux mots et la barre les dévoile (à l'annulation, les anciens). Les lignes sont
  repérées en capturant la page (`take_snapshot`) telle quelle puis avec une police aux lettres vides et aux
  largeurs identiques (`Turtlefin Blank`, `tools/make-blank-font.py`, `dust-blank`), regroupées en lignes ; l'animation est faite par Slint
  (quelques rectangles, rien pendant le choix). Blocs sous la liste (`dust-layer`), liste masquée pendant les
  captures (`dust-snap`) ; pastilles de valeur à largeur animée. Remplace la fragmentation (`Tr.fx`) et un
  premier essai en nuage de grains, trop coûteux (image de toute la fenêtre à chaque image). Visite refaite :
  elle montre l'interface (sélection qui se promène, menu ouvert, `tour-tick`), parle télécommande
  (schéma flèches / OK / Retour), cadres calés sur les vraies positions (`tabs-x`, `right-w`), boutons
  Retour / Passer / Suivant au clavier (← →, OK). Accueil affiché tout de suite au retour (plus l'ancienne page pendant le rechargement).
  Roue crantée du menu redessinée. Corrigés en testant tout (deux instances, hors ligne...) : menu vide hors
  ligne, compte retiré de « Qui regarde ? » quand son jeton est refusé, champ du mot de passe sans le clavier
  (bureau), `turtlefin "Nom"` non enregistré qui ouvrait un autre compte, Retour sur « Qui regarde ? » après
  « Changer de compte », « Retenu pour toute la série » sur un film, mise à jour d'une version locale non
  publiée affichée en échec (404 GitHub reconnu sans dépendre de la langue). Watch party vérifiée à deux.

Pas fait : Quick Connect ; manette ; fond flouté en option avec logos transparents ; licence (à choisir par le
mainteneur) ; passerelle XeLauncher (lanceur du média center du mainteneur, pas prioritaire).

## 6. Traductions

- Faites à l'exécution par `src/i18n.rs` (plus par Slint) : un même catalogue pour Rust et pour l'interface.
  Slint : global `Tr` (ui/theme.slint) — `Tr.t(Tr.l, "…")`, `Tr.f(Tr.l, "… {} …", a, b)`,
  `Tr.p(Tr.l, "{n} serveur", "{n} serveurs", n)` ; `Tr.l` change à chaque changement de langue, ce qui fait
  recalculer les textes (branché dans `main`). Rust : `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- Le texte français **est** la clé : le modifier demande de modifier le `msgid` de chaque `lang/*.po`
  (`tools/lang-check.py` signale les textes manquants et les `{}` perdus).
- Langues intégrées : `lang/<code>.po` + `BUILTIN` dans i18n.rs (fr, en, es, de, it, pt, pl, nl). Langues
  ajoutées : tout `<code>.po` du dossier `languages` (config ou à côté de l'exe), nom lu dans
  `X-Language-Name` ; un fichier peut remplacer une langue intégrée. « Ajouter une langue » écrit `modele.po`
  (msgid en **anglais**, en-tête `X-Source-Language: en`, notes `#.` en français et dans la langue en cours ;
  `keyed` ramène ces msgid au français par `lang/en.po`) et copie les `.po` de Turtlefin trouvés dans
  Téléchargements / Bureau (`import_languages`) : pas de gestionnaire de fichiers nécessaire.
- Textes absents d'une langue : anglais. Pluriels : règle `Plural-Forms` du fichier, évaluée par i18n.rs.
- Installeur : `[Languages]` et `[CustomMessages]` de `turtlefin.iss` (langues d'Inno Setup) ; il écrit le code
  choisi dans `language` à côté de l'exe, repris au premier lancement.
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
- **Automatique** : `git tag -a v0.9.2 -m "Nouveautés…" && git push origin v0.9.2` (le message de l'étiquette devient les notes de version, montrées par la mise à jour intégrée). `release.yml` compile Windows x64 / x86 et Linux
  x86_64 / aarch64, fabrique installeurs, archives, AppImage et `.deb`, et les publie dans une Release. Les noms de
  fichiers (en-tête de `release.yml`) sont attendus tels quels par `update.rs` et les scripts d'installation.
- **Windows à la main** : `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (ou
  `x86`) ; il faut Inno Setup 6, 7-Zip et NASM (x86). Résultat dans `target\dist`.
- **Linux à la main** (sur une machine Linux) : `TURTLEFIN_DIST=release cargo build --release`, puis
  `sh packaging/linux/build-appimage.sh <version>` et `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` à la compilation, sinon `update::kind()` croit à une version compilée sur place.
- x86 : les libmpv 32 bits de shinchiro publiées depuis juillet 2026 plantent au démarrage (OpenSSL) ;
  celle du 10 juin 2026 (retirée par shinchiro, encore cassées en octobre) est gardée dans la pré-version
  `libmpv-i686-20260610` de Turtlefin, que `build.ps1` utilise (`MPV_TAG` : une autre version de shinchiro). aws-lc demande NASM en 32 bits.
- Icônes : `packaging/turtlefin.svg` est le logo ; `packaging/icons/make-icons.py <dossier>` (Python + Pillow)
  régénère les PNG et l'ICO.

## 8. Essais

- `TURTLEFIN_CONFIG_DIR=<dossier>` : autre dossier de config (comptes, prefs) sans toucher au vrai.
- `--open=settings|downloads`, `--play=ID@SECONDES`, `--test-video=fichier` (lecteur sans serveur).
- `TURTLEFIN_DEBUG_FRAMES=1` (images > 25 ms), `SLINT_DEBUG_PERFORMANCE=refresh_full_speed,console`.
- Un `prefs.json` écrit par PowerShell 5 a un BOM : la lecture des fichiers de config l'ignore.
- `TURTLEFIN_DEBUG_SYNCPLAY=1` : messages de la watch party reçus et requêtes envoyées (les échecs sont
  toujours écrits). Deux instances sur un même PC : deux `TURTLEFIN_CONFIG_DIR` différents, `--desktop`.
- Essais automatisés sous Windows : `SetForegroundWindow` n'est accepté qu'après un appui clavier (Alt) ;
  sans ça, les touches simulées partent dans une autre fenêtre (faux « clavier bloqué »).

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
