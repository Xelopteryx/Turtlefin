# Turtlefin : passation de projet (état au 9 octobre 2026, version 1.1.0)

**Français** · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

Document destiné à qui reprend le développement (humain ou Claude Code). Lis-le en entier avant de toucher au
code, puis lis [README.fr.md](README.fr.md) (usage, installation, touches, fichiers).
Dépôt : https://github.com/Xelopteryx/Turtlefin · Version dans `Cargo.toml` : 1.1.0.

## 1. Objectif

Un **client Jellyfin natif en Rust**, léger, animé, utilisable à la télécommande comme au clavier et à la souris,
installable sur n'importe quel ordinateur Windows ou Linux **sans rien compiler** : l'utilisateur télécharge un
installeur ou un paquet (ou lance une commande d'installation), c'est tout. Tous les paquets sont fabriqués par la
CI GitHub (ou le PC du mainteneur), jamais par l'utilisateur.

Pourquoi : Jellyfin Desktop (Qt / QtWebEngine) fuit de la RAM et finit par planter sur les petites machines, et
l'interface web avec un thème chargé tombe sous 30 images/s sur du matériel modeste. Règle permanente : rester
stable en mémoire et ne jamais réintroduire de flou temps réel ni d'animation de filtre.

## 2. Décisions prises

| Sujet | Décision | Raison |
|---|---|---|
| Langage / UI | Rust + **Slint** `~1.18` (rendu 100 % Slint), style `fluent-dark` imposé par `build.rs` | Pas de navigateur ; le style « native » dépendrait de Qt |
| Fonction Slint `unstable-winit-030` | Filtre d'événements winit pour **F11** (`install_f11`) | Seul moyen d'avoir une touche globale ; d'où `~1.18` (API instable d'une version mineure à l'autre) |
| Réseau | `reqwest` 0.13 (rustls, magasin de certificats du système), `tokio` | `query` est une feature à activer en 0.13 |
| Lecture | **libmpv chargée à l'exécution** (`libloading`, `src/mpv.rs`), rendu OpenGL dans une texture affichée par Slint (`src/video.rs`), commandes Slint par-dessus (`ui/player.slint`) | Lecteur intégré, pas d'IPC, marche sous Wayland. Le lecteur est recréé à chaque lecture (mémoire bornée) |
| Rendu Slint | femtovg (OpenGL / GLES) imposé, sauf si `SLINT_BACKEND` est défini | La vidéo passe par une texture OpenGL |
| Texture vidéo | Pixels physiques, origine `TopLeft`, état GL sauvegardé / rétabli autour de mpv ; `loadfile` attend le contexte de rendu | Sinon image à l'envers ou « No render context set » |
| Décodage | `hwdec=no` sous Linux ARM 64 bits, `auto-safe` ailleurs (Windows : `d3d11va-copy`) | Sur les cartes ARM testées (pilote v3d), le décodage V4L2 sort un format que le rendu ne sait pas importer |
| Audio Linux | `ao=pipewire,pulse,alsa`, `config=no` | Un `mpv.conf` utilisateur imposant ALSA échouait quand PipeWire tient la sortie HDMI |
| Mémoire | Cache mpv plafonné (100 / 25 MiB), images demandées à la bonne taille, 16 éléments par rangée | Petites machines (4 Go) |
| Langues | Français dans le code (langue source), traduction à l'exécution par `src/i18n.rs` depuis `lang/<code>.po` (intégrés) et le dossier `Turtlefin Languages` | Voir section 6 |
| Plein écran (Windows) | Fenêtre sans bordure qui couvre l'écran + 1 px (`src/winfull.rs`, `set_tv_window`), pas le vrai plein écran ; suit les changements de définition (toutes les 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` pour le plein écran de Slint | En plein écran OpenGL, AMD Software prend l'appli pour un jeu : « Appuyez sur ALT + R » à chaque retour au premier plan |
| Interface TV / ordinateur | `tv-mode` ne change que la taille (`k` = 1,4) et la saisie (clavier à l'écran) ; le plein écran est à part (`full_flag`, `UiPrefs::fullscreen`, F11) | Demande de l'utilisateur : plein écran sans grossir l'interface |
| Fenêtre (Windows, bureau) | Réduite et centrée si 1280 x 720 + cadre dépasse la zone de travail ; animée pendant un déplacement (`winfull::keep_alive_while_moving`) | Écrans 1366 x 768 ; Windows bloque la boucle d'événements pendant un déplacement |
| Fenêtre console (Windows) | Sous-système « windows » en release ; `--console` en rattache / ouvre une ; commandes externes sans fenêtre (`paths::quiet_command`) | Pas de console ni de fenêtre CMD qui clignote |
| Ligne de commande | Prime toujours sur les réglages (compte de démarrage, interface TV) | Demande de l'utilisateur |
| Mot de passe | Jamais enregistré (jeton seulement) ; en ligne de commande, préférer `TURTLEFIN_PASSWORD` | Un argument est visible des autres processus |

## 3. Structure du code

```
build.rs            commit compilé, style Slint, icône de l'exe (winresource, Windows)
lang/<code>.po      traductions intégrées (source : le français du code) ; tools/lang-check.py les vérifie
ui/theme.slint      jetons de thème, globals Tr (traduction) et Motion (animations activées)
ui/app.slint        AppWindow et tous les écrans (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo : animation de démarrage (7 points, liaison, zoom), choix de la langue
ui/player.slint     écran de lecture          ui/osk.slint      clavier à l'écran
ui/card.slint       carte d'affiche            ui/marquee.slint  texte qui défile
ui/typed.slint      texte tapé animé (Str, TypedText)   ui/langx.slint  explorateur (langues, presets)
ui/dust.slint       barres du changement de langue      ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, état partagé App (Arc), écrans, navigation (pile + pages gardées), paramètres
src/boot.rs         séquence de démarrage (vérifications, langue, choix de l'écran d'arrivée)
src/i18n.rs         langue courante, tr() / trf() / trn(), langues ajoutées, modèle de traduction
src/dust.rs         effet du changement de langue (repérage des lignes de texte, morphing)
src/api.rs          client Jellyfin REST (+ relais Seerr de Jellyfin Enhanced, GetAvatar)
src/config.rs       session, comptes (12 au plus), prefs.json (UiPrefs, AnimFlags, presets), pistes, vu/favoris hors ligne
src/discovery.rs    recherche des serveurs (UDP, sous-réseaux, ARP, pairs VPN)
src/downloads.rs    téléchargements (reprise Range, file, synchronisation hors ligne)
src/mpv.rs          liaison libmpv ; src/video.rs texture OpenGL ; src/player.rs lecture, rapports, enchaînement
src/syncplay.rs     watch party (WebSocket /socket)
src/paths.rs        dossiers config / cache / données ; mode portable ; quiet_command
src/update.rs       mise à jour selon l'installation (Kind : Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows : écran / zone de travail, fenêtre animée pendant un déplacement
src/theme.rs        thèmes intégrés, fichiers .tftheme (ThemeDef), application au global Theme
ui/sky.slint        décors (ciel et bulles · « Harmony » de Windows 7), statiques, gardés en cache
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   compilation et publication sur étiquette `v*` (essai : branche `ci`)
```

Principes :
- Les données réseau passent par des structures `Send`, puis `upgrade_in_event_loop` les pousse dans les modèles
  Slint. Images décodées hors du thread UI, 6 téléchargements en parallèle, appliquées avec une garde sur l'id.
- `App.gen` invalide les chargements périmés ; `App.stack` est la pile de navigation ; `PAGES` garde fiches et
  bibliothèques pour des retours sans requête.
- Navigation clavier faite à la main (indices de sélection en Rust et en Slint), Slint ne gérant pas le focus de
  cartes dynamiques. Chaque écran a son `FocusScope` ; `refocus` rend le clavier au bon endroit.
- Les `changed` de Slint sont différés : ne pas compter sur leur ordre (positions en deux temps, etc.).
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
   le même serveur (identifiant comparé à `server_id` de la session) ; orange tant qu'aucun serveur n'est configuré.

Rouge = échec, avec un message et « Continuer » (seul au bout de 12 s). Tout vert : les sommets se relient, les
rayons partent vers le centre et le point du serveur devient l'hexagone plein — exactement le logo
(`packaging/turtlefin.svg`, même géométrie) —, puis zoom dans le centre. Slint réduit le dessin d'un `Path` de
l'épaisseur de son trait : les chemins du logo sont agrandis d'autant pour tomber sur les points.

Écran d'arrivée (`boot::route`), dans l'ordre : nom + mot de passe en ligne de commande → connexion ; nom d'un
compte enregistré → ce compte (nom inconnu : son formulaire de connexion) ; compte de démarrage
(`prefs.autostart_user` / `autostart_server`) → `fly_autostart` (la photo du compte au centre pendant la
connexion) ; sinon « Qui regarde ? » (ou la recherche de serveur si aucun n'est connu). Un compte de démarrage
disparu mène à « Qui regarde ? ». `--no-intro` (ou l'animation « Démarrage » coupée) saute l'animation ; les
vérifications ont lieu quand même.

## 5. État à la version 1.0.0

Toutes les fonctions du README sont faites et vérifiées sur captures d'écran (PC Windows, écrans 1366 x 768 et
1920 x 1080) et sur une machine Linux ARM branchée à une télé, avec les comptes de test `test` / `test2` d'un vrai
serveur. Versions publiées par la CI : 0.9.0, 0.9.1 ; la 1.0.0 est prête à être étiquetée (section 7).

Mécanismes non évidents dans le code :
- **Image partagée** (`global Hero`) : l'image d'une carte vole vers l'affiche de la fiche et revient sur la carte
  exacte au retour (`Hero.want-id`, `hero-card-ok`).
- **Rangées** (`global Rows`) : défilement propre à chaque rangée, mémorisé par clé ; changement de rangée vers la
  carte la plus proche à l'écran (`row-to`, `detail-pick-down`).
- **Hors ligne** : `config::Flags` (userdata.json) garde vu / favoris / positions avec un drapeau « à envoyer » ;
  `downloads::sync` les renvoie au retour du serveur (l'appareil a le dernier mot).
- **Adresses** : `server_main` / `server_backup` ; `watch_addresses` (20 s) bascule sur le secours et revient.
- **Watch party** : une connexion WebSocket par session (`sp_conn`), arrêt sur 401 / 403, délai croissant.
- **Photos de profil** : cache disque `avatar_<id>_still|anim.bin` + vignette ronde `avatar_<id>_thumb.png`. La
  vignette est posée tout de suite, le GIF complet est décodé hors du fil de l'interface (`avatar_cached_async`),
  puis rafraîchi depuis le serveur seulement s'il a changé (`avatar_fetch`). Animation des GIF : `AnimSlot`
  (Login, Picker, Header, Fly) et une minuterie commune ; l'avatar de l'en-tête reçoit toutes ses images une fois
  (`avatar-frames`) et seule l'image visible change. GetAvatar `SetAvatar` répond 500 → repli `POST /UserImage`.
- **Connexion animée** (`fly-phase` 1 à 4) : la photo va au centre, barre de chargement, s'envole, puis
  l'accueil arrive et l'avatar de l'en-tête apparaît (`me-pop`).
- **Changement de langue** (`dust.rs`, `ui/dust.slint`) : à l'ouverture de la liste des langues, des barres
  couvrent chaque ligne de texte, prennent la largeur des nouveaux mots au choix, puis les dévoilent. Les lignes
  sont repérées en comparant une capture de la page (`take_snapshot`) avec une capture en police
  `Turtlefin Blank` (lettres vides, mêmes largeurs, `tools/make-blank-font.py`). L'horloge (Montserrat) est exclue.
- **Visite guidée** (`tour-step` 0 à 10) : chaque étape remet l'interface dans l'état attendu ou se valide si le
  geste est déjà fait ; `tour-ev` est appelé depuis les `changed`.
- **Animations** : global `Motion` (12 interrupteurs : boot, pages, menu, select, scroll, panels, detail, player,
  search, login, language, tour), `config::AnimFlags`, presets intégrés (Toutes, Légères, Aucune) et personnels,
  exportés / importés en `.json` dans `Turtlefin Presets`. Toute durée d'animation s'écrit
  `duration: Motion.x ? 300ms : 0ms`.
- **Saisie** : au bureau, les champs sont des `TextInput` (texte masqué, dessiné par `TypedText`) ; en mode TV,
  le clavier à l'écran (`Osk`). Dans les deux modes, une touche imprimable reçue par la page va au champ
  (`typing-key`, `erase-key`, `Field.type`) ; Retour arrière n'efface en mode TV que s'il reste du texte. Un clic
  à côté d'un champ ne lui retire pas le clavier au bureau (`focus-on-click: root.tv-mode`).
- **Touches maintenues** : Entrée, Échap et Retour arrière ne répètent pas leur action (`event.repeat`), sauf
  Retour arrière pour effacer du texte.
- **Thèmes** (`src/theme.rs`, global `Theme` de ui/theme.slint) : aucune couleur en dur dans l'interface ; les
  surfaces translucides s'écrivent `Theme.fg.with-alpha(…)` (blanc sur un thème sombre, encre sur un clair), le
  texte posé sur le dégradé d'accent prend `Theme.on-accent`. `Gloss` (reflet de gel) et `AeroSky` (ui/sky.slint)
  ne s'affichent que si `Theme.gloss` / `Theme.bubbles`. Thèmes importés gardés dans prefs.json (`theme`,
  `themes`) ; le créateur (`tools/theme-creator.html`) est intégré au programme (`theme::CREATOR`) et déposé dans
  « Turtlefin Themes » par « Créer un thème ». Ses thèmes de départ doivent rester identiques à ceux de theme.rs.
  Jetons de style au-delà des couleurs : `card-border`, `glow`, `sheen` (cartes), `player-bar`, `player-ink`,
  `player-orb` (lecteur), `scenery`, `header-glass`, `edge`, `streaks`, `boot-style` (démarrage en billes, Aero).
  L'icône de la fenêtre est dessinée en Rust aux couleurs du thème (`theme::icon`) et posée par winit
  (`theme::window_icon`) : Slint ne transmet pas une icône sans clé de cache.
- **Souris** : clic partout ; molette sur l'accueil, la fiche (`d-nav`, partagé avec le clavier), la recherche,
  les bibliothèques, les téléchargements, les paramètres ; les fenêtres au premier plan absorbent la molette.

Pas fait : Quick Connect ; manette ; licence (à choisir par le mainteneur avant ou après la 1.0.0) ; passerelle
XeLauncher (lanceur du média center du mainteneur, pas prioritaire). Prévu ensuite : optimisation, version
Android TV.

## 6. Traductions

- Faites à l'exécution par `src/i18n.rs` : un même catalogue pour Rust et pour l'interface.
  Slint : global `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)` ; `Tr.k` change à chaque changement de langue, ce qui fait
  recalculer les textes. Rust : `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- Le texte français **est** la clé : le modifier demande de modifier le `msgid` de chaque `lang/*.po`
  (`tools/lang-check.py` signale les textes manquants, en trop et les `{}` perdus). Les 8 langues intégrées
  (fr, en, es, de, it, pt, pl, nl ; `BUILTIN` dans i18n.rs) sont complètes (~530 textes).
- Langues ajoutées : tout `<code>.po` du dossier `Turtlefin Languages` (`i18n::lang_dir` : Documents, à côté de
  l'exe en portable, sous `TURTLEFIN_CONFIG_DIR` pendant les essais ; l'ancien dossier `languages` y est
  déplacé), sous-dossiers compris ; nom lu dans `X-Language-Name` ; un fichier peut remplacer une langue
  intégrée. Parcourues par un explorateur intégré (`ui/langx.slint`, fonctions `lx_*` de main.rs).
- « Créer le modèle » écrit `modele.po` : msgid en **anglais**, en-tête `X-Source-Language: en`, notes `#.` en
  français et dans la langue en cours ; `keyed` ramène ces msgid au français par `lang/en.po`.
- Textes absents d'une langue : anglais. Pluriels : règle `Plural-Forms` du fichier, évaluée par i18n.rs.
- Installeur : `[Languages]` et `[CustomMessages]` de `turtlefin.iss` ; il écrit le code choisi dans `language`
  à côté de l'exe, repris au premier lancement.
- Les noms venant du serveur (bibliothèques, médias) ne sont pas traduits.

## 7. Compiler, fabriquer les paquets, publier

Développement :
- Windows : Rust (https://rustup.rs), « Outils de build Visual Studio » (C++), git ; `cargo build --release` ;
  `libmpv-2.dll` (archive `mpv-dev-x86_64-….7z` de
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) à côté de l'exe.
  La version debug a besoin de `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu) : `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev` puis `cargo build --release`.
- `cargo test --release` : tests de i18n, update, etc.

Publier une version :
1. Mettre le numéro dans `Cargo.toml` (`version = "x.y.z"`), compiler une fois (met `Cargo.lock` à jour),
   valider (commit).
2. `git push origin main`, puis `git tag -a vx.y.z -m "Nouveautés, une par ligne"` et `git push origin vx.y.z`.
   Le message de l'étiquette devient les notes de version, montrées par la mise à jour intégrée.
3. `release.yml` compile Windows x64 / x86 et Linux x86_64 / aarch64, fabrique installeurs, archives, AppImage
   et `.deb`, et les publie dans une Release (suivi dans l'onglet Actions du dépôt). Les noms de fichiers (en-tête de
   `release.yml`) sont attendus tels quels par `update.rs` et les scripts d'installation.
4. Essai sans publier : `git push origin main:ci` (tout est compilé, rien n'est publié).

À la main :
- **Windows** : `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (ou `x86`) ;
  il faut Inno Setup 6, 7-Zip et NASM (x86). Résultat dans `target\dist`.
- **Linux** (sur une machine Linux) : `TURTLEFIN_DIST=release cargo build --release`, puis
  `sh packaging/linux/build-appimage.sh <version>` et `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` à la compilation, sinon `update::kind()` croit à une version compilée sur place.
- x86 : les libmpv 32 bits de shinchiro publiées depuis juillet 2026 plantent au démarrage (OpenSSL) ; celle du
  10 juin 2026 est gardée dans la pré-version `libmpv-i686-20260610` de Turtlefin (ne pas la supprimer), que
  `build.ps1` utilise (`MPV_TAG` : une autre version de shinchiro). aws-lc demande NASM en 32 bits.
- Icônes : `packaging/turtlefin.svg` est le logo ; `packaging/icons/make-icons.py <dossier>` (Python + Pillow)
  régénère les PNG et l'ICO.

Mise à jour d'une copie compilée sur place (`Kind::Source`) : fichiers modifiés ou non suivis mis de côté
(`git stash -u`, récupérables avec `git stash pop`) avant `git pull --ff-only`. Les nouveautés s'affichent dans
une fenêtre à part (Paramètres → À propos → Voir les nouveautés) ; `TURTLEFIN_TEST_UPDATE="Version x|note|note"`
simule une mise à jour pour l'essayer. Raccourcis aux couleurs du thème : `theme::apply_shortcuts` (.ico sur les
.lnk sous Windows, icônes `turtlefin` dans ~/.local/share/icons sous Linux), sautés pendant les essais
(`TURTLEFIN_CONFIG_DIR`) sauf avec `TURTLEFIN_TEST_SHORTCUTS=1`.

Branches : `main` (seule branche de travail), `ci` (essais de la CI). `interface-lua` et `libmpv` sont
d'anciennes expériences, déjà fusionnées dans `main` : elles peuvent être supprimées. Étiquettes : `v0.9.0`,
`v0.9.1` (versions publiées) et `libmpv-i686-20260610` (libmpv 32 bits, voir plus haut).

## 8. Essais

- `TURTLEFIN_CONFIG_DIR=<dossier>` : autre dossier de config (comptes, prefs, langues) sans toucher au vrai.
- `--open=ID|settings|downloads`, `--play=ID@SECONDES`, `--test-video=fichier` (lecteur sans serveur).
- `TURTLEFIN_DEBUG_FRAMES=1` (images > 25 ms) ; `TURTLEFIN_DEBUG_GAPS=1` avec
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed` : pauses de plus de 40 ms entre deux images.
- `TURTLEFIN_DEBUG_SYNCPLAY=1` : messages de la watch party. Deux instances sur un même PC : deux
  `TURTLEFIN_CONFIG_DIR` différents, `--desktop`.
- Un `prefs.json` écrit par PowerShell 5 a un BOM : la lecture des fichiers de config l'ignore.
- Essais automatisés sous Windows : `SetForegroundWindow` n'est accepté qu'après une touche simulée ; utiliser
  F24, pas Alt (Alt seul fait entrer la fenêtre en mode menu et le clic suivant est perdu). Une touche maintenue
  se simule par plusieurs `keybd_event` « enfoncée » successifs (Windows les marque comme répétitions).

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
8. Micro-pauses (~40 ms) mesurées seulement en rendu forcé à pleine vitesse, à chaque image d'un GIF de
   l'en-tête et à la liaison du logo ; cause non trouvée, invisibles en usage normal.

## 10. Préférences de travail du mainteneur

- Réponses en français ; peu d'expérience de Linux / SSH : expliquer les commandes.
- Ne rien pousser sur GitHub ni publier de version sans validation explicite du mainteneur, qui publie les versions.
- Essais avec une copie de la config (`TURTLEFIN_CONFIG_DIR`), jamais la vraie ; comptes de test `test` / `test2`.
- Après chaque lot de changements : installeur de test sur le Bureau du mainteneur
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, l'ancien supprimé).
