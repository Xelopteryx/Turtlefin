# Turtlefin : passation de projet (état au 29 octobre 2026, version 1.9.7 « Retrouvailles »)

**Français** · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

Document destiné à qui reprend le développement (humain ou Claude Code). Lis-le en entier avant de toucher au
code, puis lis [README.fr.md](README.fr.md) (usage, installation, touches, fichiers).
Dépôt : https://github.com/Xelopteryx/Turtlefin · Version dans `Cargo.toml` : 1.9.7 (« Retrouvailles », prête mais
pas encore publiée au 29 octobre 2026 ; versions publiées avant elle : 0.9.0, 0.9.1, 1.0.0, 1.1.0, 1.2.0).

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
| Décodage | `hwdec=drm-copy` sous Linux ARM 64 bits, `auto-safe` ailleurs (Windows : `d3d11va-copy`) | Sur le Pi (pilote v3d), le décodeur matériel sort un format (« SAND ») que le rendu ne sait pas importer ; la copie le remet au format habituel. Pi 5, 4K HEVC 10 bits : 0 image perdue au lieu de 17 %, ~1 cœur au lieu de 4 |
| Audio Linux | `ao=pipewire,pulse,alsa`, `config=no` | Un `mpv.conf` utilisateur imposant ALSA échouait quand PipeWire tient la sortie HDMI |
| Mémoire | Cache mpv plafonné (100 / 25 MiB), images demandées à la bonne taille, 16 éléments par rangée | Petites machines (4 Go) |
| Langues | Français dans le code (langue source), traduction à l'exécution par `src/i18n.rs` depuis `lang/<code>.po` (intégrés) et le dossier `Turtlefin Languages` | Voir section 6 |
| Plein écran (Windows) | Fenêtre sans bordure qui couvre l'écran + 1 px (`src/winfull.rs`, `set_tv_window`), pas le vrai plein écran ; suit les changements de définition (toutes les 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` pour le plein écran de Slint | En plein écran OpenGL, AMD Software prend l'appli pour un jeu : « Appuyez sur ALT + R » à chaque retour au premier plan |
| Interface TV / ordinateur | `tv-mode` ne change que la taille (`k` = 1,4) et la saisie (clavier à l'écran) ; le plein écran est à part (`full_flag`, `UiPrefs::fullscreen`, F11) | Demande de l'utilisateur : plein écran sans grossir l'interface |
| Fenêtre (Windows, bureau) | Réduite et centrée si 1280 x 720 + cadre dépasse la zone de travail ; animée pendant un déplacement (`winfull::keep_alive_while_moving`) | Écrans 1366 x 768 ; Windows bloque la boucle d'événements pendant un déplacement |
| Fenêtre console (Windows) | Sous-système « windows » en release ; `--console` en rattache / ouvre une ; commandes externes sans fenêtre (`paths::quiet_command`) | Pas de console ni de fenêtre CMD qui clignote |
| Ligne de commande | Prime toujours sur les réglages (compte de démarrage, interface TV) | Demande de l'utilisateur |
| Mot de passe | Jamais enregistré (jeton seulement) ; en ligne de commande, préférer `TURTLEFIN_PASSWORD` | Un argument est visible des autres processus |
| Jeton sur le disque | Chiffré par DPAPI sous Windows (`config::secret`, préfixe `dpapi:`), fichier 0600 ailleurs ; jamais dans une adresse (mpv le reçoit par `http-header-fields`) ; révoqué (`/Sessions/Logout`) quand un compte est retiré | Audit de sécurité d'octobre 2026 (section 6 bis) |
| Téléchargements | `<bibliothèque>/<série>/Season 01/<série> S01E01.mkv` (+ `.json`, `.jpg`, sous-titres) ; images de la série et de la saison partagées | Demande du mainteneur : moins de place, rangement lisible |
| Sélection visible | Exclusive : au clavier / télécommande (`Nav.kb`) seule la sélection est mise en avant, à la souris seul l'élément survolé | Sur le Pi, le pointeur immobile mettait une carte en avant en plus de la sélection |
| Tactile | Glissements traduits en flèches (`Swipe`, main.rs) ; le glisser de la souris aussi | Toutes les pages savent déjà se servir des flèches |

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
src/chromecast.rs   Diffuser : appareils Google Cast ; src/relay.rs relais HTTP ; src/lan.rs réseau local
src/password.rs     solidité d'un mot de passe (alerte « trop simple »)
src/paths.rs        dossiers config / cache / données ; mode portable ; quiet_command
src/update.rs       mise à jour selon l'installation (Kind : Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows : écran / zone de travail, fenêtre animée pendant un déplacement
src/theme.rs        thèmes intégrés, fichiers .tftheme (ThemeDef), application au global Theme
ui/sky.slint        décors (ciel et bulles · « Harmony » de Windows 7), statiques, gardés en cache
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    flatpak/ (manifeste, fiche AppStream, cargo-sources.json),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator),
                    flatpak-cargo-sources.py (sources Cargo de Flatpak, à relancer après chaque Cargo.lock)
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
  Au bureau, la souris est le sélecteur : rien n'est mis en avant sans survol, le survol anime les cartes,
  boutons, onglets, listes et le menu (qui choisit l'entrée survolée). Partout, la mise en avant s'écrit
  `Nav.kb ? focused : survol` (jamais les deux à la fois).
- **Tactile et glisser** (`Swipe`, main.rs, dans le filtre d'événements winit de `install_f11`) : un glissement
  du doigt (ou de la souris, bouton gauche) devient des pas de flèches (une demi-carte en largeur, une rangée en
  hauteur), avec un lancer à la fin ; depuis le bord gauche : Échap. Dès que le geste est reconnu, Slint reçoit
  `PointerExited` : la carte touchée ne s'ouvre pas. Une carte touchée (ou tirée à la souris) rend sa rangée
  active (`touched`). `TURTLEFIN_DEBUG_TOUCH=1` affiche les gestes. Glisser à la souris vérifié ; pas encore
  essayé sur un vrai écran tactile (l'injection tactile de Windows arrive à l'appli comme une souris).
- **Fond** (`Backdrop`, app.slint) : dégradé du thème, image du média (deux calques en fondu), décor, barre de
  verre. Dessiné une fois sous les pages et une seconde fois comme voile des changements de page : le contenu
  s'efface, le fond ne bouge pas (avant, tout passait au noir sauf l'affiche qui vole).
- **Retour de l'image partagée** : la carte du même élément, sinon celle d'un autre épisode de la même série
  (`CardData.group`, « À suivre » passé à l'épisode suivant après « vu ») ; l'accueil rechargé resélectionne
  cette carte (`home_keep`). Carte introuvable : l'image s'efface en fondu (`hero-out`). Fiche descendue
  (affiche hors de l'écran) : pas de vol.
- **Changement de thème** : balayage de gauche à droite avec une bande de lumière aux couleurs du nouveau thème
  (une seule capture découpée), à la place des carreaux.
- **Effet de langue** : la police vide garde les boîtes englobantes de Montserrat (`recalcBBoxes = False` dans
  make-blank-font.py) ; sinon, en interface TV (k = 1,4), la page bougeait d'une fraction de pixel entre les
  deux captures et les icônes étaient prises pour du texte.
- **Fin de lecture plus tôt que la durée annoncée** : vraie fin si le générique est atteint et que le serveur
  répond (`natural`, player.rs) ; sinon coupure du réseau (reprise automatique).
- **Proposition « Épisode suivant »** (`outro_at`) : segment Outro / Credits du serveur (plugin Intro Skipper,
  conseillé), sinon chapitre nommé « ending », « credits »…, sinon chapitre sans nom : le dernier qui commence
  dans les 5 dernières minutes et au moins 45 s avant la fin ; à défaut, 3 % avant la fin (40 s au moins).
  `TURTLEFIN_DEBUG_UP=1` affiche position, générique et chapitres.

Pas fait : Quick Connect ; manette native (XeLauncher la traduit en touches : `xe_tf_input.py`, répétition
~6 pas/s et appuis en double ignorés depuis le 14 octobre 2026).

Android (commencé le 15 octobre 2026, voir `android/README.md`) : APK du même code (`android/`, racine
`../src/main.rs`, point d'entrée `android/src/entry.rs`), moteur Android de Slint (Skia) dont une copie
modifiée est dans `android/vendor/` (crochet tactile, clavier physique, Retour, échelle). Vérifié dans
l'émulateur (Android 13, x86_64) : connexion, accueil, défilement au doigt, fiches, lecture (MediaCodec),
pause, arrière-plan. Reste : essais sur un vrai téléphone et le Meta Quest (arm64), watch party en `wss://`,
mise en page en portrait (l'appli est en paysage), publication (clé de signature, CI).

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

## 6 bis. Sécurité (audit d'octobre 2026)

Fait après des remarques sur Reddit ; l'application a été « attaquée » point par point :
- **Serveur malveillant ou défaillant** : identifiants et étiquettes passés dans les adresses encodés
  (`api::seg`), noms de fichiers du cache et des téléchargements nettoyés (`api::file_key`,
  `downloads::fs_name`, `id_ok` : plus de `../`, de séparateur ni de nom réservé Windows) ; réponses JSON
  (64 Mio) et images (24 Mio) bornées (`api::body_limited`) ; décodage d'image limité à 8192 px et 256 Mio
  (`load_image`, GIF à 2048 px) contre les bombes de décompression.
- **Jeton** : voir la section 2 (DPAPI, en-tête, révocation). Fichiers de config écrits d'un coup
  (`config::write_atomic`, fichier provisoire puis renommage) et créés directement en 0600 sous Unix.
- **Mise à jour** : seulement depuis `https://github.com/Xelopteryx/Turtlefin/releases/download/`, empreinte
  SHA-256 comparée à celle publiée par GitHub (`digest`), dossier dans le cache de l'utilisateur (plus
  `/tmp/turtlefin-update`, où un autre compte pouvait remplacer le .deb avant son installation en root), numéro
  de version vérifié, relance du .deb sans interprétation shell (paramètres `$1` et `$@`).
- **Fichiers .po étrangers** : règle de pluriel bornée (division de i64::MIN par -1, imbrication sans fin).
- **Windows** : `open_url` met l'adresse entre guillemets (un `&` dans un nom de dossier n'ouvre plus une commande).
- **Deuxième passe** : découverte UDP limitée aux réponses du réseau local / VPN et adresse annoncée vérifiée
  (même identifiant) : une machine d'Internet ne peut plus se faire passer pour le serveur ; avertissement avant
  d'envoyer un mot de passe en `http://` par Internet (`discovery::plain_over_internet`, second essai = connexion) ;
  source distante (`.strm`, `Protocol: Http`) lue directement, sans jeton (une redirection l'aurait envoyé à
  l'autre hôte) ; tous les identifiants des chemins encodés ; thème piégé (couleur avec un caractère accentué)
  qui faisait planter ; langues, thèmes, préréglages bornés en taille (`paths::read_text_limited`) ; watch party :
  délai de reprise borné, messages WebSocket de 2 Mio au plus ; scripts intégrés de mpv non chargés.
  `TURTLEFIN_MPV_LOG` (diagnostic) écrit encore le jeton dans le journal de mpv : à ne pas partager tel quel.
- **Troisième passe** : redirections de `https` vers `http` refusées par tous les clients HTTP
  (`api::redirect_policy` ; une 307/308 aurait renvoyé le mot de passe en clair) ; dossier de configuration en
  0700 sous Unix ; Windows : `SetDllDirectoryW("")` au démarrage (`mpv::harden_dll_search`) et libmpv cherchée
  seulement à côté de l'exécutable (une DLL piégée dans le dossier courant n'est plus chargée) ; scripts
  d'installation (`packaging/install.sh`, `install.ps1`) et `packaging/windows/build.ps1` (libmpv de shinchiro)
  vérifient l'empreinte SHA-256 publiée par GitHub ; CI : `permissions: contents: read` (écriture seulement pour
  la publication), actions épinglées par commit (à mettre à jour à la main). Xelauncher : sortie audio,
  rotation et volume vérifiés avant d'être passés au shell.
  Non corrigé : `build-appimage.sh` télécharge linuxdeploy « continuous » sans empreinte (pas d'empreinte
  stable publiée) ; mpv/ffmpeg renvoient l'en-tête du jeton si le serveur Jellyfin lui-même redirige un flux.

Reste : un serveur en `http://` transmet le jeton en clair (choix de l'utilisateur) ; jeton en clair (0600)
sous Linux, faute de trousseau commun à toutes les machines visées.

## 6 ter. Travaux d'octobre 2026 (après la 1.2.0 ; publiés avec la 1.9.7)

- **Mot de passe** (`src/password.rs`) : `score()` 0 à 4, `is_weak()` (liste de mots de passe courants, nom du
  compte, suites, longueur). Seul le verdict est gardé (`Account.weak_pw`), jamais le mot de passe. Alerte
  (global `Alert` de `ui/theme.slint`) : anneau rouge autour de la photo, pastilles « ! » sur le chemin
  (Paramètres → Compte → Changer le mot de passe, menu de gauche). Changement : `Client::change_password`
  (`/Users/Password`, 403 = mot de passe actuel faux), fenêtre en trois étapes (modes 4 à 6 du dialogue `addr`).
- **Diffuser** (icône en haut à droite) : choisit l'écran où envoyer les vidéos (« Lire sur … » sur les fiches) :
  autre Turtlefin ou appli Jellyfin du compte (`/Sessions?controllableByUserId`, commande `Play` du serveur ;
  pilotables = `SupportsRemoteControl` et `IsActive`, sans limite d'activité : une télé restée sur l'accueil ne
  fait aucune requête) ou télé DLNA du réseau (retirée en 1.9.6, voir 6 quater ; ancien `src/cast.rs` : SSDP, puis SOAP AVTransport / RenderingControl ;
  adresses privées seulement, l'hôte de LOCATION doit être celui qui a répondu ; la télé lit
  `/Videos/{id}/stream?static=true&api_key=…`). Le panneau montre ensuite ce qui y passe (relu toutes les 2 s)
  avec lecture, avance, arrêt et volume (`cast_cmd`).
- **Télécommande** (menu de gauche) : liste les autres Turtlefin du compte (`Client == "Turtlefin"`), puis pilote
  celui choisi (`remote_cmd` : `GeneralCommand` MoveUp…/Select/Back/GoHome/Volume, `Playstate`). Côté piloté,
  `syncplay.rs` reçoit ces messages par le WebSocket (`Event::Remote`, `remote_received` dans main.rs) ; le
  WebSocket annonce les capacités (`Client::set_capabilities`) à chaque connexion. Nom de l'appareil
  (`api::device_name`) : `TURTLEFIN_DEVICE_NAME`, sinon nom de l'ordinateur ; Android : nom choisi dans les
  réglages (Settings.Global `device_name`), sinon le modèle ; accents encodés dans l'en-tête (`header_value`).
- **Télécommande de la télé** : touches Select / Back / Exit / Lecture traduites (main.rs, `install_f11`, et
  `android/vendor/…/androidwindowadapter.rs`) ; sur le Pi, HDMI-CEC par Xelauncher (`cec-ctl --playback`).
- **Frutiger Aero** : police Selawik, cadre de Windows 7 dessiné par Turtlefin en fenêtre (`WinCaption`,
  boutons `CaptionButton` d'après 7.css, MIT, `ui/icons/win7`) ; verre flouté par
  `winfull::set_blur` (SetWindowCompositionAttribute) et coins découpés par `winfull::set_round`
  (SetWindowRgn, refait sur WM_SIZE, enlevé agrandie ou en plein écran) ; `chrome-top` / `chrome-side` décalent
  l'interface sous la barre. Piège trouvé : dans Slint, un élément sans `x` / `y` est **centré** dans son parent ;
  la zone de la barre de titre (sans `y`) faisait une bande invisible au milieu de la fenêtre qui prenait les
  clics. Toujours donner `x` / `y` aux `TouchArea` dimensionnées.
- **Plein écran animé** (Windows) : la fenêtre grandit jusqu'à l'écran (`animate_window`, 280 ms).
  **Interface TV ↔ bureau** : `relayout_rows` recalcule les rangées déjà affichées (plus de trous).
- **Panneaux du haut** (Diffuser, watch party, télécommande) : tout chemin de fermeture rend le clavier à la page
  (`changed cast-pop / party-pop / remote-open` → `refocus`).
- **Android** : volume du lecteur = volume média du téléphone (JNI AudioManager) ; zones de toucher des barres
  de temps et de volume (`hit-*` de `PlayerView`, posées dans `init` : une valeur constante ne passe jamais par
  `changed`) ; en paysage, marge de 48 px des deux côtés du lecteur (la barre de navigation passe par-dessus
  l'appli sans être signalée, `Nav.phone`) ; toast au-dessus de cette barre ; « Mode tablette » ; réglages
  inutiles retirés ; Android TV : interface TV imposée.
- **Collections** : l'affiche va au centre puis les films sont distribués (`deal-*` dans app.slint).
- **Publication préparée** : Linux 32 bits (armhf, i386) dans `release.yml` (job `linux32`, conteneur Debian 12) ;
  APK armeabi-v7a / x86 possibles (`build-apk.ps1 -Abi`), signature de publication `-Release` (clé
  `%USERPROFILE%\.android\turtlefin-release.jks`, mot de passe protégé par DPAPI).

## 6 quater. Version 1.9.6 (27 octobre 2026)

- **Décodage (ticket GitHub n° 1)** : Windows : `hwdec=d3d11va-copy,nvdec-copy,dxva2-copy` (`player::hwdec_mode`).
  Le rendu passe par ANGLE (OpenGL ES sur Direct3D) ; `auto-safe` partageait l'image décodée avec ANGLE
  (d3d11-egl, cuda) et donnait des lignes de parasites sur NVIDIA. `UiPrefs::no_hwdec` (Paramètres → Lecture
  → Décodage matériel) : tout au processeur.
- **DLNA retiré** : plus de `cast.rs` ; `lan.rs` garde `lan_ipv4s`, `multicast_socket`, `lan_reachable` ;
  `api::cast_stream` ne sert plus que Chromecast ; relais sans réécriture MPEG-TS ; `ui_prefs()` oublie les
  écrans `dlna:`.
- **Stockage** (catégorie 7 des Paramètres, À propos passe en 8) : `UiPrefs::download_dir` (vide : dossier par
  défaut), `download_limit_gb` (0 : sans limite). `downloads::root()` lu une fois (`ROOT`), `set_root`,
  `move_all` (déplace médias, sous-titres, descriptions, images partagées, `.part`, file d'attente ; seulement
  les dossiers qui avaient un téléchargement), `disk_space` (GetDiskFreeSpaceExW / statvfs), `used_bytes`,
  `check_room` avant chaque transfert (taille de `MediaSource::size`, marge 256 Mo). Suppression : `prune` ne
  retire que les fichiers de Turtlefin puis les dossiers vides (avant : `remove_dir_all`). Choix du dossier :
  explorateur intégré en mode `dldir` (`dldir_show`, bouton « pick » de ui/langx.slint) ; caché en borne et
  sur Android.
- **Démarrage hors ligne** (`boot.rs`, `Outcome`) : réseau absent ou serveur muet → point orange (état 5, le
  logo se forme normalement ; 4 reste « aucun serveur », logo sans cœur), `Start::unreachable`. `route` :
  téléchargements + compte de démarrage ou compte unique → hors ligne ; sinon « Qui regarde ? » avec message
  (`watch_server_back` l'efface au retour du serveur) ; `login_pick` hors ligne avec téléchargements → ce compte
  devient la session, `go_offline`.
- **Ligne de commande** : `--sub=LANGUE|off`, `--audio=LANGUE` (`player::cli_tracks`, `lang_codes` : fr → fre,fra,fr),
  valables pour tout le lancement.
- **Télécommande** : bureau (winit) et Android (vendor) donnent Stop → `Key.Stop`, suivant / précédent et
  Chaîne ± → PageDown / PageUp, avance / retour rapides → F18 / F17 (`REMOTE_FF`, `REMOTE_REW`), Menu / Info /
  Guide → `Key.Menu` (menu de gauche, dans `nav-key`). Manette Android : X, Y, L1, R1, L2, R2, Start, Select.
  Android : `is_tv` regarde aussi `Configuration.uiMode`.

- **Cœur du logo** (`global Srv` de ui/logo.slint, `set_core` dans main.rs) : 0 joint · 1 injoignable
  (cœur fendu, ressoudé au retour) · 2 hors ligne (maison ; Nerd : pistes ambre, `CircuitScene.offline`).
  Chaque `ThemeLogo` joue le changement, et le joue aussi à son apparition (`start-from`) ; passage hors
  ligne : grand logo au centre (`off-show`, app.slint). Hors ligne au démarrage : bouton « Regarder hors
  ligne » (`offline-ok`, action `offline`). Téléchargements : `Entry.server_id` / `user_id`,
  `downloads::belongs` filtre `sync` et `migrate`.
- **Animations** : `deck-rise` (bibliothèque sans carte de départ), `Card.moving` (textes cachés pendant
  le vol : femtovg épaissit les lettres d'un élément tourné ou agrandi), `swap-wait` (fiche suivante
  cachée jusqu'à l'arrivée de l'affiche), `restore_page` rend aussi la zone clavier (≥ 2).
- **Décor Turtlefin** : `HexScene` (fixe, en cache) + `HexDrift` (lueurs et onde, minuterie de 50 ms,
  phase commune `Drift`), réglage `AnimFlags::scenery` (« Fond animé », coupé par défaut sur le Pi).
  Aero : `AeroDrift` (bulles, reflet) ; Nerd : `CircuitDrift` (paquets, horloge, balayage), même phase.

Retours du 28 octobre (après la 1.9.6, même numéro) :
- **Srv.core 3 « perdu »** (serveur muet, rien de téléchargé) : `gone`, `slot`, `plug` dans ThemeLogo.
  `unreachable_start` (main.rs, appelé par `boot::route` et `open_saved_session`) : téléchargements →
  « Qui regarde ? » avec « Regarder hors ligne » (core 1, plus de passage automatique) ; sinon recherche de
  serveur (core 3) et `watch_server_found` (5 s ; compte de démarrage : session rouverte). Grand logo au
  centre aussi au retour (core 0 depuis 1, 2 ou 3 ; `off-show`).
- **Démarrage** : `handoff` de BootLogo (le ThemeLogo prend le relais du dessin du démarrage, identique) ;
  Nerd : `CircuitScene.morph` (puces → plots, processeur → puce centrale) et `gone` (effacement au départ
  du logo) ; pistes tracées de plot en plot. Serveur muet : 1,3 s de plus avant le départ (boot.rs).
- **Bibliothèques** : la fin de distribution n'éteint plus l'image partagée que depuis le minuteur (plus de
  `changed deal-anim`) ; `hero-ready` ne relance rien pendant le ramassage ; paquet borné en hauteur
  (`deck-bh`, `deck-w16`). Portrait (`narrow`) : bannière en haut (`deck-top`), morceaux = colonnes de la
  grille (`lib-cols`), départs rangée par rangée (`deal-delay`).
- **Fiche** : `d-snap` / `d-jump` (affiche de la page hors de l'écran : saut au début sans défilement).
- **Seerr** : `sr-pad`, `sr-poster-w` (destination de l'image qui vole), mise en page `tall` ; `blk(b)`,
  `cast-e(i)`, `open-e`, `out-c` / `out-e`.
- **Lecteur** : `seek-chain` (touchers suivants : +10 s chacun, `seek-total`).
- **Explorateur** : bouton `mkdir` (mode 9 de la fenêtre de saisie ; l'explorateur s'efface pendant la saisie).
- **Hors ligne** : `theme::set_offline` (appelé par `set_core`) : `apply` passe fonds, accents, verre, cartes
  à l'ambre (`amber`, teinte 36°, clarté gardée), via `switch_theme`. Téléchargements au démarrage : core 2
  directement ; plus de bouton « Regarder hors ligne » (tuiles « Hors ligne », `login_pick` n'attend le
  serveur que 3 s). `check_offline_empty` : plus rien hors ligne → core 3 + recherche de serveur.
- **Lecteur** : `pos-secs` / `dur-secs`, `seek-base`, `seek-step` (sauts bornés). **Bibliothèque portrait** :
  `hero-arc` (courbe par le centre, `hero-xp`… remplacent les interpolations linéaires).
- **Seerr** : `sr-more` (fenêtre « Lire la suite »), `sr-cast-off` (distribution qui défile), `sr-sel` 2 et 3.
- **Barre du haut** : plus de bouton maison (`show-home` toujours faux).
- **Paquet en vol** : ses morceaux suivent `deck-p` (entiers tant que la distribution n'a pas commencé),
  pas `deal-p` : `deal-wait` exclut `loading` (pour la grille), la carte disparaissait pendant le
  chargement de la bibliothèque.
- **Fonds animés** (ui/sky.slint) : l'énergie part du centre (le serveur). `Drift.ph` en secondes, tiré de
  `animation-tick()` par une minuterie de 33 ms (une minuterie en retard ne ralentit plus le décor).
  Nerd : `CircuitDrift` (processeur → puces → toutes les pistes, `Spark`), pistes partagées par le global
  `Board` ; Turtlefin : `HexDrift` (onde qui allume chaque anneau, `HexRing`) ; Aero : `AeroDrift` (bulles
  nées au centre, anneau de lumière). Rien sans serveur (`no-server`, core 3).
- **Logo du démarrage** au centre exact (`mid-cy` de boot.slint, `cy` de CircuitScene et CircuitDrift :
  plus de décalage de 30 px).
- **Seerr** : la page garde le clavier tant qu'elle est ouverte (les `FocusScope` des pages ont
  `!root.sr-open`, celui de Seerr reprend le clavier à chaque `refocus`) : Retour faisait reculer la page
  cachée dessous. Affiche = image de la carte ouverte (`sr-card-on`, `sr-card-img`, posées au départ du
  vol) : Seerr donne parfois une autre affiche à sa page qu'à ses listes. Au doigt : la distribution suit
  le doigt (`sr-cast-off` en colonnes, réel), pas de chevrons (`Nav.touch`) ; glisser vers le haut
  (doigt ou molette, `pull`) ouvre « Lire la suite », tirer la fenêtre vers le bas depuis son début la
  referme.
- **Image partagée** : `hero-out` remis à faux au départ d'un vol et minuterie de sécurité comptée depuis
  `hero-safe-t0` (une image encore là d'un retour pouvait rendre la nouvelle invisible ou couper son vol).
- **Bibliothèque en portrait** : plus de passage par le centre (`hero-arc` faux) ; cartes distribuées sans
  fondu (une carte semi-transparente est dessinée dans un calque : une quinzaine à la fois ramaient).
- **Lecteur** : « Chargement… » seulement après 700 ms d'attente, en fondu (`load-slow`, `load-a`).
- **Nerd** : capot du processeur (`CircuitScene.lid`, posé à la fin du démarrage et dans le fond des
  pages, seulement processeur branché) : les sept puces dessous, logo gravé dessus. Fond animé
  (`CircuitDrift`, cycle de 6 s) : le logo du capot s'allume (cœur, rayons, plots), puis le courant suit
  un vrai calendrier (`TraceData.t0` / `td`, généré : vitesse constante ; une piste qui part d'un
  composant attend qu'il soit atteint), chaque composant atteint s'allume et éclaire autour de lui
  (`Board.parts`, `PartData`) ; tout s'éteint ensemble à la fin du cycle. Étincelles cachées sous le capot
  (`Spark.under-lid`).
- **Découverte** : « who is JellyfinServer? » aussi envoyé à l'adresse de diffusion de chaque interface
  (255.255.255.255 ne sort que par celle de la route par défaut sous Windows).
- **Seerr sous la barre du haut** : la page est dessinée avant la barre (et le menu), `y: header-h`, son
  fond remonte derrière la barre ; `back-anim` et `nav-out` la referment d'abord. `hero-ty` en tient compte.
- **Arrivée sur une fiche** : l'image de la carte reste posée sur l'affiche de la fiche 250 ms puis
  s'efface (`landed-hold`, `landed-a`, au temps) : l'affiche de la fiche pouvait mettre quelques images
  à s'afficher (vu sur le téléphone).
- **Nerd** : pas de courant au démarrage sans processeur branché ni maison (`powered` de boot.slint) ;
  broches étamées (couche à part dans CircuitScene), une à chaque arrivée de piste, aucune dans le vide
  (régénérées par script depuis Board.traces, dans CircuitScene et Board.parts).
- **ABYDOS** : Jellyfin 12.4 portable dans `%LOCALAPPDATA%\jellyfin-test\server-12.4` (même `--datadir` ;
  sauvegarde de la 12.2 : `data.avant-12.4`) ; Seerr 3.5 en Docker (`seerr-abydos`, port 5055, réglages
  dans `jellyfin-test\seerr`), connecté avec le compte test ; dépôt du plugin Jellyfin Enhanced ajouté.
- **Bande-annonce** : bouton à icône (clap, `icon == "trailer"`) juste à droite de Télécharger. Plusieurs
  bandes-annonces locales : celle dont le nom porte la langue audio du compte (« … [fr+en] », plugin
  Turtlefin Trailers, `trailer_rank`), sinon celle de l'interface. Plugin : dépôt à part
  `GitHub/Turtlefin-Trailers` (une par langue voulue, versions doublées réunies, clé TMDB nécessaire
  pour les langues).
- **Bande-annonce des pages Seerr** (média absent du serveur) : bouton « Bande-annonce » (`sr-sel` 4) si le
  plugin Turtlefin Trailers répond (`/TurtlefinTrailers/Info`, `sr-trailers`) ; `seerr_trailer` demande
  `/TurtlefinTrailers/Prepare` (langue audio du compte, titre, année ; jusqu'à 3 min) puis lit le fichier
  rendu (nom aléatoire, cache du plugin effacé après 24 h au plus). Onglet / menu choisi avec Seerr
  ouverte : l'affiche part avec la page (`nav-out`), pas de vol vers une carte absente.
- **Pi** : Xelauncher lance `turtlefin` du PATH, c'est-à-dire `/usr/bin/turtlefin` du paquet .deb ;
  une version compilée sur le PC doit y être installée (l'ancienne gardée à côté), pas seulement dans
  `~/turtlefin/target/release`.

## 6 quinquies. Audit « sortir de l'appli » (28 octobre 2026, au soir)

Vérifié sûr : commandes externes sans shell (yt-dlp `--ignore-config`, `--`, identifiant validé) ;
`open_url` et ouverture de dossier au bureau seulement (`external_ok`) ; explorateur des langues /
thèmes / presets confiné à son dossier ; création de dossier au nom nettoyé ; mpv sans config, sans
scripts, sans ytdl ; relais : jeton aléatoire, IP de l'écran, hôte fixe, GET/HEAD/OPTIONS.
Corrigé :
- sélecteur du dossier des téléchargements (tout le disque, création de dossiers) : retiré dès que
  `!external_ok` (kiosk, interface TV, Android) et refusé dans `lx_open_mode` ; avant, seulement avec
  `--kiosk`, or le Pi du salon tourne en `--tv` ;
- relais : 32 connexions à la fois au plus ;
- watch party : identifiant d'élément du groupe validé (lettres, chiffres, tirets), position finie ;
- journaux et messages d'erreur : jetons masqués (`redact` : api_key, ApiKey, token, X-Emby-Token ;
  sur le Pi, la sortie est recopiée dans un fichier par Xelauncher) ; relais : erreur sans l'adresse ;
- Android 12+ : `dataExtractionRules` (res/xml) : rien n'est copié vers le nuage ni vers un autre
  appareil (allowBackup=false ne suffit plus pour le transfert d'appareil à appareil).
Plugin Turtlefin Trailers : 400 Mo au plus par vidéo, cache plafonné à 5 Go, entrées bornées, yt-dlp
sans configuration utilisateur. Reste (accepté) : un utilisateur connecté peut faire télécharger au
serveur des vidéos YouTube (borné : 2 à la fois, 400 Mo, cache de 24 h et 5 Go).

Essais d'utilisateur (PC) corrigés : menu de gauche qui défilait sous la souris (l'entrée visée fuyait
au survol, clic raté) : au survol, le défilement est figé (`menu-free`) ; messages de liste vide
(`EmptyNote` : panneau, fondu) qui tombaient sur le capot Nerd ; capot assombri dans le fond des pages,
composants allumés plus discrets ; broches jamais au coin d'un composant (pistes de la ROM et de la
carte graphique décalées, tracés fixes et points d'arrivée refaits depuis `Board.traces`).

## 6 sexies. Version 1.9.7 « Retrouvailles » (29 octobre 2026)

Première version publiée depuis la 1.2.0 : elle réunit tout ce qui est décrit en 6 ter, 6 quater, les retours
du 28 octobre et 6 quinquies (Android, Diffuser et relais, télécommande, thèmes Aero et Nerd et leurs fonds
animés, pages Seerr, bandes-annonces, hors ligne en ambre, stockage, mot de passe, audit de sécurité). Nom :
l'appli retrouve d'elle-même son serveur (et le perd proprement), dans les deux sens et sur toutes les pages
(voir plus bas). Le README montre les trois logos (Turtlefin, Aero, Nerd,
`packaging/icons/turtlefin-*-256.png`, tirés de `theme::icon` par le test `icon_dump`) et dit franchement
qu'il reste des choses à revoir (voir la section 9). Pas encore publiée : le développeur veut d'abord juger
le reste (pousser, étiquette `v1.9.7`, APK jointe à la main).

Ajouts de la fin de session :
- **Diffuser** : `ActionButton` coupe son libellé (« … ») au lieu de déborder (« Voir la lecture · <titre> »,
  « Ne plus diffuser sur <nom> »). Noms de comptes : déjà coupés (`Marquee`).
- **Téléchargements** : « Aucun téléchargement » seulement s'il n'y a rien de fini ni rien en cours
  (`dl-status`).
- **Compilation** : `codegen-units = 16` et `incremental = true` dans les deux `Cargo.toml` (avant :
  `codegen-units = 1`, un seul cœur pendant toute la génération de code). Mesuré sur le Ryzen 7 5800X3D :
  modification d'un `.rs` ~1 min au lieu de ~8 min 30 ; d'un `.slint` ~6 min (tout le code de l'interface est
  régénéré) ; compilation complète ~7 min 20. La vérification du crate reste mono-cœur (Rust stable).
- **Plugin Turtlefin Trailers** (dépôt à part, `GitHub/Turtlefin-Trailers`, pas encore publié) : tâche
  planifiée (une bande-annonce par langue voulue : bibliothèque, langue audio de chaque compte, réglages ;
  versions doublées réunies, sinon l'officielle sous-titrée), points d'accès `/TurtlefinTrailers/Info`,
  `/Prepare` (authentifié), `/File/<nom aléatoire>` (cache de 24 h au plus et 5 Go). Il faut yt-dlp et deno
  (moteur JavaScript de yt-dlp, sinon ni tous les formats ni les sous-titres) sur le serveur, et une clé TMDB
  pour les langues. La recherche YouTube est coupée par défaut (peu fiable).
- **APK** : pas dans la CI ; fabriquée à la main (`android/build-apk.ps1 -Release`) et jointe à la version.

Les cinq derniers problèmes avant la sortie (vidéos du 29 octobre et retours) :
- **Seerr depuis une fiche** : en bas d'une fiche (Similaires, Distribution), ouvrir une page Seerr ne
  remonte plus la fiche ni ne la fond (`open-hero`, `up` faux pour une page Seerr) : la page Seerr s'ouvre
  par-dessus, le poster vole à sa place, et le retour retrouve la fiche là où elle était.
- **Passage en ambre** : fondu de tout l'écran (`theme-fade`, image de la fenêtre au-dessus de tout, qui
  s'efface en 0,9 s), au démarrage compris ; avant, le balayage du changement de thème passait sous le logo
  de démarrage et l'ambre arrivait d'un coup.
- **Logo hors ligne** : l'orbe passe à un ambre foncé et la lueur ambre est sous la carapace (avant : par-
  dessus, l'hexagone blanc devenait illisible).
- **Hors ligne, Téléchargements = Accueil** : barre du haut comme sur l'accueil (« Turtlefin », onglet Accueil
  allumé, pas de retour) tant que le serveur n'est pas revenu (`off-home` dans `NavBar`).
- **Suivi du serveur** (`watch_server`, toutes les 6 s, remplace l'ancien essai de 30 s du mode hors ligne) :
  perdu (deux essais ratés) → sur « Qui regarde ? », comptes « Hors ligne » et message ; ailleurs, hors ligne
  s'il y a des téléchargements, sinon cœur fendu et message ; revenu → retour en ligne, l'accueil remplace
  les téléchargements une fois chargé (plus de page vide ni de grand logo de chargement entre les deux). Une
  fiche qui ne se charge plus faute de serveur passe aussi hors ligne, sans message d'échec avant.
  **Pendant une lecture** : un seul message, rien d'autre ; le thème (`theme_pending`) et la page changent à
  la fin de la lecture. Essayé sur le PC avec un relais local coupé et rétabli (config de test,
  `TURTLEFIN_CONFIG_DIR`) et `TURTLEFIN_NO_DISCOVERY=1` (sinon l'appli retrouve le serveur sur le réseau
  local, ce qui est voulu).

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
- **Flatpak** (Flathub) : `packaging/flatpak/io.github.Xelopteryx.Turtlefin.yml` compile libass, libplacebo et
  libmpv, prend FFmpeg complet dans l'extension `org.freedesktop.Platform.ffmpeg-full`, puis Turtlefin hors ligne
  à partir de `cargo-sources.json` (`python tools/flatpak-cargo-sources.py` après chaque changement de
  Cargo.lock). Essai : `flatpak-builder --user --install --force-clean build-dir <manifeste>`. Pour Flathub, la
  source `dir` est à remplacer par le dépôt git (étiquette + commit). Jamais compilé pour de vrai à ce jour.
  Dans un Flatpak, la mise à jour intégrée renvoie vers Flatpak (`update::Kind::Flatpak`).
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

Android : `powershell -ExecutionPolicy Bypass -File android\build-apk.ps1` (NDK, cibles Rust Android et
cargo-ndk : voir `android/README.md`) ; compilation dans `%LOCALAPPDATA%\turtlefin-android`, APK dans
`target\dist`. Pas encore dans la CI.

## 8. Essais

- `TURTLEFIN_CONFIG_DIR=<dossier>` : autre dossier de config (comptes, prefs, langues) sans toucher au vrai ;
  `TURTLEFIN_DATA_DIR=<dossier>` : autre dossier de téléchargements.
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
5. Jeton en clair dans `session.json` / `accounts.json` sous Linux (0600) ; chiffré par DPAPI sous Windows.
   Un `session.json` copié d'un autre PC Windows n'est plus lisible : le compte redemande son mot de passe.
6. Déchirement d'image sous Xorg sans compositeur (Openbox seul) : utiliser un compositeur (picom
   `--backend egl --vsync`). Turtlefin tient 60 images/s.
7. Linux ARM : HEVC décodé par la puce du Pi (`drm-copy`, vérifié dans Turtlefin le 15 octobre 2026 sur un
   épisode 4K HEVC du serveur : ~0,15 cœur au lieu de ~0,8, image correcte) ; le H.264 reste en logiciel (pas
   de décodeur sur le Pi 5), suffisant en 1440p. `TURTLEFIN_HWDEC=no` pour revenir au logiciel.
   Le Pi (Wi-Fi) ne reçoit que 3 à 5 Mbit/s du serveur : la 4K (~22 Mbit/s) s'arrête pour charger.
10. Débit maximal (Paramètres → Lecture, bouton Qualité du lecteur ; `UiPrefs::max_bitrate`, kbit/s, 0 = original) :
   `api::Client::transcode` (PlaybackInfo avec un profil « tout en lecture directe sous la limite, sinon H.264
   en HLS »), `ApiKey` retiré de l'adresse (le jeton passe par l'en-tête, qui marche aussi pour les morceaux HLS),
   résolution plafonnée selon le débit (`api::max_size`), une seule piste audio (langue préférée,
   `preferred_audio`), sous-titres texte servis à part, conversion arrêtée à la fin (`stop_transcode`). Changer de
   qualité en lecture recharge la vidéo au même endroit. Le serveur du mainteneur (Raspberry Pi) convertit à
   ~0,3× le temps réel (4K HEVC comme 1440p H.264, mesuré le 15 octobre 2026) : inutilisable chez lui, utile
   avec un serveur plus puissant.
8. Micro-pauses (~40 ms) mesurées seulement en rendu forcé à pleine vitesse, à chaque image d'un GIF de
   l'en-tête et à la liaison du logo ; cause non trouvée, invisibles en usage normal.
9. Mémoire (mesurée sur le Pi le 14 octobre 2026, film de 3 h 48) : ~215 Mo à l'écran de connexion, ~465 Mo
   après une première lecture (palier gardé, aucune croissance d'une lecture à l'autre), +20 Mo/h pendant une
   lecture, rendus à la fin ; descripteurs stables (46 en lecture, 23-25 après).
11. Reste à revoir à la 1.9.7 (connu, pas encore corrigé) :
   - Diffuser vers une télé hors de chez soi (relais du téléphone, Chromecast) : pas essayé (aucune télé
     disponible pour des essais) ; la télécommande d'Android TV non plus.
   - Bandes-annonces : YouTube limite fort ses sous-titres traduits (erreur 429) ; beaucoup d'animés n'ont pas
     de bande-annonce française chez TMDB ; une bande-annonce sans voix est étiquetée par la langue de son texte.
   - Thème Nerd : le fond animé coûte ~35-40 % d'un cœur sur le Pi 5 (60 images/s tenues) ; il reste coupé par
     défaut sur le Pi.
   - Distribution d'une bibliothèque en portrait : allégée, encore lourde sur un téléphone modeste.
   - Trailarr (bandes-annonces par Radarr / Sonarr) : pas retenu ; le plugin Turtlefin Trailers le remplace.

## 10. Préférences de travail du mainteneur

- Réponses en français ; peu d'expérience de Linux / SSH : expliquer les commandes.
- Ne rien pousser sur GitHub ni publier de version sans validation explicite du mainteneur, qui publie les versions.
- Essais avec une copie de la config (`TURTLEFIN_CONFIG_DIR`), jamais la vraie ; comptes de test `test` / `test2`.
- Après chaque lot de changements : installeur de test sur le Bureau du mainteneur
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, l'ancien supprimé).
