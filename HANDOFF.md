# Turtlefin : passation de projet (état au 13 novembre 2026, version 1.9.9, correctif de la 1.9.8)

**Français** · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

Document destiné à qui reprend le développement (humain ou Claude Code). Lis-le en entier avant de toucher au
code, puis lis [README.fr.md](README.fr.md) (usage, installation, touches, fichiers).
Dépôt : https://github.com/Xelopteryx/Turtlefin · Version dans `Cargo.toml` : 1.9.9 (correctifs de la 1.9.7 « Cast Update » :
1.9.8 et 1.9.7 le 29 octobre 2026, 1.9.9 le 30 ; versions publiées avant : 0.9.0, 0.9.1, 1.0.0, 1.1.0, 1.2.0).
Détail : 1.9.8 en section 6 quater (« Lignes sur NVIDIA », « Journal de mpv sans jeton »), 1.9.9 en 6 septies.
L'APK Android reste en 1.9.7 (les correctifs ne concernent pas Android).

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
| Rendu Slint | femtovg (OpenGL / GLES) imposé, sauf si `SLINT_BACKEND` est défini ; Windows et Linux PC (x86) : OpenGL classique exigé (`require_opengl`, `TURTLEFIN_GLES=1` pour revenir à GLES) ; le Pi et Android restent en GLES | La vidéo passe par une texture OpenGL. Slint demande d'abord GLES 2.0, que les pilotes NVIDIA acceptent (Windows et Linux), et ce pilote barrait de lignes les images `nv12` / `p010` du décodage matériel (ticket n° 1) |
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
src/lock.rs         code facultatif d'un compte (4 chiffres ou 6 flèches), demandé sur « Qui regarde ? »
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
  `TURTLEFIN_MPV_LOG` (diagnostic) : jeton masqué depuis la 1.9.8 (`mpv::redact`).
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
- **Quatrième passe (6 novembre 2026)** : dépendances (`cargo audit`, PC et `android/`) : rustls 0.23.46
  (4 failles), rustls-webpki 0.103.16, webpki-root-certs 1.0.11, pulldown-cmark 0.13.6, tokio 1.54 ;
  restent seulement des crates « non maintenues » tirées par Slint. Relais : un appareil qui n'est pas
  l'écran d'une diffusion est coupé dès la connexion (il ne prend plus une des 32 places), en-têtes en
  10 s au total (avant : 10 s par octet, une place gardée des heures), chemin relayé lu dans l'adresse
  analysée ; essai `relay::tests::only_the_screen_is_served`. `api::seg` : « . » et « .. » deviennent
  « _ » (même encodés en `%2E%2E`, l'analyse de l'adresse les prenait pour un retour au dossier
  parent) ; tableau de bord : tous les identifiants des chemins encodés (`pseg`, points des versions
  gardés). Images du catalogue d'extensions (`public_image`) : nom résolu, adresses publiques seulement
  (`api::is_public_ip`, Tailscale 100.64/10 compris), connexion à l'adresse vérifiée, sans redirection
  (avant : un dépôt pouvait faire envoyer des requêtes aux machines du réseau local). Cast : longueur
  de message sans débordement. Export d'un thème nommé « CON », « NUL », « COM1 »… : préfixé de « _ ».
  Cache des réponses en 0700 sous Unix.
  Points « non corrigés » de la troisième passe, réglés : `build-appimage.sh` télécharge linuxdeploy
  `1-alpha-20251107-1` (version épinglée, empreinte SHA-256 vérifiée par architecture) ; flux redirigé
  par le serveur vers un autre hôte : `Client::offsite` suit les redirections avant mpv (requête d'un
  octet) et mpv lit l'adresse d'arrivée sans l'en-tête du jeton (sous-titres redirigés ailleurs
  écartés ; flux HLS non concernés) ; adresse `http://` qui passe par Internet : si le même serveur
  (même identifiant) répond en `https://` sur cet hôte (port saisi, 443, 8920), elle est remplacée
  (`discovery::https_twin`, à la connexion et au démarrage pour les adresses enregistrées) ; Linux :
  jeton rangé dans le trousseau de la session (Secret Service via `secret-tool` : GNOME Keyring,
  KWallet), le fichier ne garde que `secret-service:<empreinte>` ; sans trousseau (Pi en kiosque,
  Flatpak, version portable, `TURTLEFIN_NO_KEYRING=1`) : fichier 0600 comme avant. Trousseau
  verrouillé au lancement : la référence est gardée, la connexion redemandée.

Reste : un serveur joignable seulement en `http://` par Internet transmet le jeton en clair (rien à
chiffrer côté client : HTTPS à configurer sur le serveur, ou VPN ; avertissement à la connexion) ; jeton
en fichier 0600 sur les machines Linux sans trousseau.

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
  `UiPrefs::no_hwdec` (Paramètres → Lecture → Décodage matériel) : tout au processeur. Ce n'était pas la
  cause : voir plus bas « Lignes sur NVIDIA ».
- **Lignes sur NVIDIA, cause réelle (ticket n° 1, RTX 2070 Super, pilote 617.42)** : pas d'ANGLE ; Slint
  (glutin) demande d'abord un contexte OpenGL ES 2.0, que les pilotes NVIDIA acceptent sous Windows
  (`OpenGL ES 3.2 NVIDIA`), AMD / Intel non (OpenGL classique). Sur ce GLES, les images à deux plans
  (`nv12` / `p010`, décodage matériel) étaient barrées de lignes ; décodeur hors de cause (d3d11va et nvdec
  identiques, image propre avec `--vf=format=yuv420p`). Correction : `require_opengl()` sous Windows
  (main.rs, `BackendSelector`) ; `TURTLEFIN_GLES=1` pour revenir à GLES. Confirmé par l'auteur du ticket
  (DVD, Full HD, 4K, décodage matériel actif).
- **Journal de mpv sans jeton** : `TURTLEFIN_MPV_LOG` est écrit par Turtlefin (`Mpv::log_to`,
  `mpv_request_log_messages` niveau debug, même présentation que mpv) au lieu de l'option `log-file` :
  mpv y recopiait la commande `change-list http-header-fields` avec l'en-tête Authorization. `mpv::redact`
  masque `Token=`, `api_key=`, `ApiKey=`.
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

## 6 sexies. Version 1.9.7 « Cast Update » (29 octobre 2026)

Première version publiée depuis la 1.2.0 : elle réunit tout ce qui est décrit en 6 ter, 6 quater, les retours
du 28 octobre et 6 quinquies (Android, Diffuser et relais, télécommande, thèmes Aero et Nerd et leurs fonds
animés, pages Seerr, bandes-annonces, hors ligne en ambre, stockage, mot de passe, audit de sécurité). Nom, dans
la lignée des versions GitHub (« Interface Update », « Touch Update ») : « Diffuser » (cast), la principale
nouveauté : envoyer la lecture d'un appareil à l'autre (téléphone, PC, Pi, télés), avec relais au besoin. Le README montre les trois logos (Turtlefin, Aero, Nerd,
`packaging/icons/turtlefin-*-256.png`, tirés de `theme::icon` par le test `icon_dump`) et dit franchement
qu'il reste des choses à revoir (voir la section 9). Publiée le 29 octobre 2026 (étiquette `v1.9.7`, APK jointe
à la main).

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
- **Curseur sur la vidéo** : il restait affiché quand la souris ne bougeait plus (Slint n'applique
  `mouse-cursor: none` qu'au mouvement suivant, qui réaffiche les commandes). Le lecteur envoie maintenant
  `cursor:hide` / `cursor:show` avec les commandes, et `cursor_visible` cache le pointeur au niveau de la
  fenêtre (winit) ; il est réaffiché à la fin de la lecture. Vérifié avec `GetCursorInfo`.

## 6 septies. Version 1.9.9, correctif de la 1.9.8 (30 octobre 2026)

- **Lignes sur NVIDIA sous Linux aussi** : reproduit sur Chulak (Ubuntu 26.04, GTX 960, pilote 580) dans le
  Flatpak, contexte `OpenGL ES 3.2 NVIDIA`. OpenGL classique exigé aussi sous Linux x86 / x86_64 (main.rs, `pc`) ;
  vérifié : `3.3.0 NVIDIA`, image propre, version compilée sur le PC puis Flatpak de la CI. Le Pi reste en GLES.
- **Windows 32 bits** : ne démarrait plus depuis le 8 octobre (`SetWindowLongPtrW` n'existe pas dans le user32
  32 bits ; message « point d'entrée introuvable »). `winfull.rs` : `link_name = "SetWindowLongW"` en 32 bits.
  Vérifié sur le PC : démarrage, lecture, déplacement de la fenêtre.
- **Flatpak** : premier `.flatpak` publié (job `flatpak`, voir section 7). Le manifeste remplace les options C du
  SDK pour Turtlefin (aws-lc-sys refuse un jitterentropy optimisé). Installé depuis un fichier, Flatpak prend
  seul le runtime, les pilotes GL (NVIDIA compris) et `codecs-extra` (HEVC 10 bits lu) ; `ffmpeg-full` n'est
  pas nécessaire. Fiche AppStream : versions 1.9.7 à 1.9.9.
- **Numérotation** : quatre nombres (1.9.8.1) impossibles : Cargo n'accepte que x.y.z et `update::newer` ne compare
  que trois nombres (une 1.9.8.1 ne serait pas proposée aux 1.9.8).
- **Compilation Linux x86_64 sur le PC** (comme pour le Pi) : `%LOCALAPPDATA%/turtlefin-x64-sys` (libfontconfig de
  Chulak + faux pkg-config), `CARGO_TARGET_DIR=target/linux-x64 PKG_CONFIG=…/turtlefin-x64-sys/pkg-config.bat
  PKG_CONFIG_ALLOW_CROSS=1 RUSTC_BOOTSTRAP=1 RUSTFLAGS="-Zthreads=16 -C link-arg=-Wl,--allow-shlib-undefined"
  cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.35` (`-Zthreads` : analyse parallèle de rustc,
  essais locaux seulement).

## 6 octies. Après la 1.9.9 : bibliothèques, menu des cartes, tableau de bord (30 octobre 2026, pas publié)

Demandes du ticket n° 2 (LainX84) et du mainteneur. Commits locaux 8ba7ef1 … 506893e.
- **Bibliothèque** : index A–Z à droite (`az`, `lib-jump` → `api::library_index`, `NameLessThan`) avec barre
  de défilement ; la bande de l'index est exclue du geste « glisser pour défiler » (`az-band`, `Swipe`).
  Lettres tapées = filtre (`lib-key`, `NameStartsWith`, `lib_filter`, `lib_gen` contre les réponses dans le
  désordre). Distribution des cartes une fois par session (`lib_dealt`, `deal-allowed`, réglage
  `AnimFlags::deal`).
- **Menu des cartes** (`cm-open`) : clic droit, appui long au doigt (`Card.menu-requested`), Entrée / OK
  maintenu (la fiche s'ouvre au relâchement d'un appui court, `enter-down`). Vu / non vu (`card_played`,
  état relu sur le serveur). Administrateurs (`is-admin`, `Client::rights`) : Identifier
  (`identify_search` / `identify_apply`, `RemoteSearch`), Actualiser (`refresh_item`), Modifier le titre
  (champ `Name` verrouillé dans `LockedFields`).
- **Tableau de bord** (`src/dashboard.rs`) : même page que les Paramètres (`set-dash`), 11 catégories ;
  lignes lues à l'ouverture d'une catégorie (`dash_rows`), clés « dash:… ». Actions irréversibles : second
  appui (`dash_armed`, « Confirmer ? »). Extensions : le serveur annonce l'ancien état jusqu'à son
  redémarrage, d'où `dash_plugins`. « Cet appareil » : Id, clé d'accès, ou nom + appli + compte (le serveur
  range parfois l'appareil sous un autre identifiant). Saisie de texte commune : `prompt-*`,
  `dashboard::ask` / `answer`. Dates à l'heure locale (`when`).
- Compilation sur le PC : `RUSTC_BOOTSTRAP=1 RUSTFLAGS=-Zthreads=16`, `CARGO_TARGET_DIR=target/fast`
  (analyse parallèle de rustc ; ~3 min 30 à 6 min après un changement).
- Reste : transcodage (choix), bibliothèques par compte, ajout/suppression de bibliothèques, fiche
  complète d'un média, création de clé d'API, désinstallation d'extension ; APK à refaire.

## 6 nonies. Liste « problèmes vue.txt » : diffusion, thèmes, lecteur (4 novembre 2026, pas publié)

- **Diffusion** : plus de bouton « Lire sur … » : un écran choisi, Lecture y envoie (`a == "play"` avec
  `app.cast` → `cast_play_item`). Retour ou accueil depuis le lecteur-télécommande : la vidéo s'arrête et
  l'écran reste choisi (`cast_stop_video`). Fermeture normale : `cast_stop_on_exit` (3 s au plus). Appli tuée :
  `Saved.cast_last` noté à chaque envoi, `cast_restore` s'y rebranche à la connexion (`to_saved` le garde :
  la connexion réécrit la session). Android tue souvent l'appli sans passer par la sortie : c'est ce chemin-là.
- **Thèmes** : aperçu dans la liste de choix (`theme-preview`, `theme::preview`, sans recalculer l'icône de la
  fenêtre ; Échap revient au thème enregistré). Les thèmes Anime et Cartoon ajoutés ici ont été retirés depuis (section 6 undecies).
- **Android** : changer de thème ne change plus l'icône tout de suite (`PENDING_ICON`, appliqué sur
  `MainEvent::Stop`) : c'est `setComponentEnabledSetting` qui fermait l'appli. PAS ENCORE ESSAYÉ sur téléphone.
- **Lecteur** : commandes en fondu et glissement (`osd-p`) ; souris sur le panneau ou réglage en cours : elles
  restent (`idle` négatif, `vid-area.has-hover`) ; souris qui va vers « Passer l'intro » / la carte « Épisode
  suivant » commandes cachées : le pointeur revient sans les commandes (`aim`), qui ne reviennent que s'il
  s'arrête 0,9 s sur la vidéo ; bandeau des épisodes navigable à la souris, bouton de fermeture.
- **Divers** : fondu du formulaire de connexion ; sélection visible dans les champs (`TypedText` masqué pendant
  une sélection) ; recherche par acteur, studio, tag (`api::search_people`) ; bloc résumé et carte « À suivre »
  aux couleurs du thème avec survol et sélection.
- Propositions graphiques (8 pistes, A à H, au choix du mainteneur) : https://claude.ai/artifact/F9MwLJaZWLZbXroqgDFKJs

## 6 decies. Propositions graphiques A à H (4 novembre 2026, pas publié)

- **A, bannière** de l'accueil (`Banner`, `present_banner`, `banner-h`, `banner-sel`) : à reprendre, sinon à suivre,
  sinon dernier ajouté ; onglet Accueil seulement, jamais pendant la visite guidée. Les rangées gardent leurs `y-px` :
  c'est `banner-h` qui les décale.
- **B** : carte choisie à 1,08, voisines sous un voile (`dim`, un rectangle : moins coûteux qu'une opacité), résumé sur
  l'image après 0,5 s (`info-a`, réglage `card_info`, pas au doigt).
- **C** : `play_label` (« Reprendre à 7:13 »), bouton Lecture coloré avec texte (icône seule en portrait), boutons
  alignés à gauche. **D** : tuiles « Mes médias » sans légende (`tiles`, `bare`).
- **E** : `media_tint` (teinte vive dominante de l'affiche, réglage `media_color`) : haut de la fiche et bouton Lecture.
- **F** : formes d'attente sur l'écran « loading » seulement. **H** existait déjà (liste d'épisodes avec résumé).
- **G** : au survol de la barre de temps, temps et chapitre ; vignette si le serveur a des images trickplay
  (`Client::trickplay`, `seek_hover`). LA VIGNETTE N'A PAS ÉTÉ VUE : pas d'images préparées sur le serveur d'essai.
- Aperçu des thèmes au doigt : première touche = essai, seconde = choix.
- Android : l'icône du thème change en quittant l'appli, sans la fermer (essayé sur le téléphone, appli gardée en
  vie) ; ces propositions A à H n'ont pas été compilées ni essayées sur Android.

## 6 undecies. Décors à tous les formats ; thèmes Anime et Cartoon retirés (4 novembre 2026, pas publié)

- **Formats** : plus aucun décor étiré. `HarmonyScene` et `SkyScene` gardent la même échelle en largeur et en hauteur
  (rognés sur les côtés en portrait). Nerd : `CircuitEchoes` répète la carte autour de la carte centrale quand la
  fenêtre est plus large ou plus haute qu'elle (seulement en fond des pages, pas au démarrage).
- **Anime et Cartoon retirés** à la demande du mainteneur : perfectionner l'appli d'abord, les thèmes ensuite. Restent
  Turtlefin, Frutiger Aero et Nerd ; un `prefs.json` qui nomme `anime` ou `cartoon` retombe sur Turtlefin. Le dernier état
  est sauvegardé hors du dépôt, dans `Turtlefin-themes-sauvegarde` à côté du dossier du projet (correctif et fichiers) ;
  une version antérieure reste dans le commit 24f4064. Son image `anime-library.jpg` vient de NoobAI-XL 1.1, dont la licence
  interdit la commercialisation, images générées comprises : à ne pas reprendre dans une version vendue.
- Essais : `target/fast` a sa `libmpv-2.dll` (posée par `build.rs`, voir section 7) ; avant : `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`, sinon la lecture revient
  aussitôt à la fiche.
- **Tournée des 3 thèmes** : panneaux des fenêtres opaques sur Nerd (`Theme.term` les rendait transparents, seul Aero a un
  `WinFrame` qui dessine le fond) ; voile sous les réglages sur Nerd ; coche de la liste de choix visible sur Nerd. Bandeau
  d'épisodes du lecteur : bouton de fermeture dans sa propre colonne, flèches à la souris (`arrows`), un cran de molette = une
  demi-carte, un clic sur un épisode referme le bandeau comme Entrée.
- **Décor Nerd en retrait** (`decor-dim` à 0,45, sauf sur l'écran « loading ») : `Backdrop.decor`, opacité élément par élément dans
  `CircuitDrift` (`dim`), pas sur tout le calque (trop coûteux sur le Pi). PAS ESSAYÉ sur le Pi.
- **Menu des cartes sur les rangées** (accueil, recherche, fiche) : `SectionRow.card-menu`, `row-card-menu`. Pas sur les tuiles, ni
  sur les cartes Seerr ou les personnes (`card-menu-ok`), ni sur la page Téléchargements. `card_played` met à jour la carte partout.
- **Demandes et À regarder en cartes** : une carte par état (`request_sections`, affiche de la plus récente) ou par bibliothèque
  (`watch_sections`, `watch_groups`, bibliothèque lue par `/Items/{id}/Ancestors`, gardée dans `App.item_lib`). Identifiants
  `group:req:<état>`, `group:watch:<bibliothèque>` : la grille des bibliothèques les ouvre (`open_group`, `group_cards`, `lib-list` :
  pas d'index A-Z). Groupe vide : `empty:…`, `status` négatif = code affiché (`EmptyTile`, propre à chaque thème).
  Carte de demandes : `status` 10 + état = liseré de couleur.
- **Compte à rebours** sur la carte « Épisode suivant » (`up-auto`, `up-left`) : seulement si l'enchaînement se fera (réglage du
  compte, hors watch party). **Squelettes de chargement** (`LoadSkeleton`, `load-shape`) : pages après 0,35 s, connexion après 0,9 s.
- **Délai d'attente** : `load_home` 40 s en tout (les requêtes s'enchaînent, chacune limitée à 30 s), `load_tab` 25 s, `root.loading` 25 s
  (`load_timed_out`, `load-timeout`, `tab-failed`). **Chargement qui traîne** : `LoadMark`, propre à chaque thème, à la place de la bulle.
- **Lecteur aux couleurs du thème** : `PlayerButton` (TermGlass sur Nerd ; Win7Glass sur Aero, sauf l'orbe de Lecture), `panel-r`, barres
  carrées sur Nerd. **Collections et Playlists** : onglets de la barre du haut (`coll-id`, `play-id`, posés par `set_menu`), retirés de
  « Mes médias », gardés dans le menu. Essai d'un serveur lent : relais local devant Jellyfin, avec une copie de la config qui le vise.
  Pas de distribution sur eux ni sur les cartes de groupe (`deal`). À leur racine, ce sont des onglets (`lib-tab-root`) : même barre que
  l'accueil, sans retour (Échap ramène à l'accueil). Ouverts par leur onglet, tuiles de groupe (`is-group`, Demandes, À regarder) et
  collection ou bibliothèque déjà distribuée : pas d'image partagée, cartes en vague (`lib-wave`). Paquet : miniature de la grille
  (`pc-cols` = colonnes de la grille), départs du haut vers le bas (`deal-delay`) ; au retour, `deck-holding` garde les affiches dans la
  carte jusqu'à la page précédente (`deck-left`).
- **Barre du haut** : `tabs-lo` identique sur toutes les pages (largeur mesurée du nom), donc une ou deux lignes, textes ou icônes, partout
  pareil ; onglet en icône sélectionné : son nom dessous. Deux lignes : `hdr-arrow` (flèches par ligne, ↑ ↓ pour en changer) ; `nav-enter`
  monte sur l'onglet affiché. **États des demandes** : `Theme.req-color` (vert disponible, orange en attente, bleu acceptée, rouge refusée,
  violet échec), matière propre au thème (pastille, verre, étiquette à équerres et diode). **Fond** : pas de fond pour une tuile (bibliothèque,
  groupe, dossier vide) ni un onglet vide. Rangées et grille : place pour la carte agrandie (`room`, `top-pad`).
- **Accueil** : bannière moins haute au bureau (`banner-full`) ; la page ne défile pas pour une rangée déjà entièrement visible (`home-y`) ;
  l'onglet Accueil, ou l'onglet déjà affiché, ramène tout en haut. **Recherche** : sélection sur le champ = aucun résultat sélectionné.
  **Paramètres** : nom de la page dans la barre du haut ; sur une colonne, celui de la catégorie ouverte reste dans la page.
- **Cache des réponses** (`api.rs` : `RESP`, `cache_only`, `live`, `changes`, dossier `cache/api`, 64 Mo, vidé avec le cache d'images) : onglets, première page des grilles et fiches déjà vues s'affichent tout de suite, puis sont remplacés sur place si le serveur répond autre chose (`refresh_sections`). Liste blanche `resp_keep` : ni comptes ni tableau de bord. Pas encore : pages Seerr, recherche ; le démarrage attend encore quelques réponses du serveur avant le premier onglet.
- **À regarder, Demandes** : onglet et entrée de menu seulement s'ils ont un contenu (`sync_tabs`, `has-watch`, `has-requests`), jamais de tuile vide. **Page Seerr** : état de la demande du compte (`seerr_reqs`, `SeerrPage.state`) ; demande refusée ou en échec : on peut redemander (bouton non essayé, il enverrait une vraie demande).
- **Connexion** : tuile déplacée au lieu de dupliquée quand l'ordre change ; taille des tuiles selon le nombre de comptes (`pk-fits`), zone qui défile sous le titre ; limite de 64 comptes. **Thèmes** : appliqués au choix seulement (`theme::preview` gardé pour un futur choix au premier démarrage) ; liste fermée en quittant la page. **Touche maintenue** (menu, Paramètres) : un pas toutes les 130 ms, surbrillance sans glissement (`rep-wait`, `rep-on`).
- **Tableau de bord refait** (`src/dashboard.rs`, 22 catégories en 6 groupes, `cat-top`, `cat-heads`) : vue d'ensemble (chiffres, lectures avec affiche et avancement, tâches, stockage, activité ; relue toutes les 4 s, `dash-tick`), Général, Apparence, Comptes, Bibliothèques (création, dossiers, options), Affichage, Métadonnées, NFO, Transcodage, Reprise, Débit, Aperçus, Appareils, Activité, Réseau, Clés d'API, Sauvegardes, Journaux, Tâches (déclencheurs), Extensions, Catalogue, Dépôts. Sous-pages : `dash_open`, Échap ou ← pour en sortir. Réglages : `Fields` (toggle, text, choice) sur un document du serveur relu puis renvoyé entier (`set_field`). Types de lignes en plus : head, stats, bar, line, hero (en-tête d'une page), cards (grille : `dash-cards`, cartes d'une catégorie ; `dash-card-sel`, flèches dans la grille, sortie à ses bords ; images `image_slot`, chemin serveur ou HTTPS public, PNG/JPEG/SVG). Grilles : lectures, appareils, comptes, bibliothèques, extensions, catalogue. **Écritures essayées sur ABYDOS** et vérifiées par l'API : oui/non, nombre, liste, choix, création, droit, renommage et suppression de compte, clé d'API, sauvegarde, déclencheur, option de bibliothèque, message à un appareil. **Pas essayées** : pause et arrêt d'une lecture (Jormungand ne se lit pas sur ABYDOS), création de bibliothèque, dossiers, dépôts, extensions, réseau, redémarrage, restauration. Pas repris du web : TV en direct, pages de réglages propres aux extensions, envoi de l'image d'accueil.
- **Tout mettre en cache** (Paramètres > À propos, `precache`) : onglets, bibliothèques page par page, fiches et enfants, images aux tailles affichées ; quatre à la fois ; limites `IMG_CACHE_MAX` 2 Gio et `RESP_CACHE_MAX` 256 Mio.
- **Pi (5 novembre)** : `/usr/bin/turtlefin` remplacé (ancien : `turtlefin.avant-5nov`) ; `~/xelauncher/src/JSs/ipc-turtlefin.js` repris du dépôt Xelauncher (`--kiosk`, CEC ; ancien : `.avant-5nov`), lu au prochain lancement d'Electron.
- **Clavier seul (tournée du 5 novembre sur le Pi)** : index A–Z au clavier (`az-on`, `az-sel`, `az-index`) ; ← → restent sur la ligne de la grille (→ au bout : lettres, ← au bout : menu). « Passer l'intro » caché sous une liste du lecteur. Lecteur : Diffuser atteignable (`zone` 4, → depuis Retour) ; le clavier revient au lecteur à la fermeture du panneau (`refocus` de `PlayerView`). Après une lecture : pas de cache pour l'onglet et la fiche suivants (`no_cache_once`, `home_stale`). Mesures sur le Pi (Aero, TV) : repos 18 %, navigation 25 à 58 % d'un cœur, 9 images de plus de 25 ms en 1 min, mémoire 470 à 740 Mo.
- **Retirer un compte** : deux appuis (`ask-forget`, `arm-id`, 5 s), le titre et la tuile le disent.
- **À traduire** : 297 textes manquent dans chaque langue (`tools/lang-check.py`), l'appli affiche le français à la place.
- **Essais** : une copie de la config contient les vrais jetons ; « retirer un compte » révoque le jeton sur le serveur. Effacer les jetons de la copie pour tester ces écrans.
- **Correctifs du 5 novembre au soir** : « Échap pour annuler » seulement quand une page s'ouvre depuis une autre (`LoadMark.can-cancel`, pas à l'accueil) ; lecture impossible : le message reste affiché après le retour à la page (`toast_hold`, vérifié sur ABYDOS, Y: absent) ; « Chargement… » du lecteur coupé d'un coup quand la vidéo est prête ; retour de lecture sans écran d'attente (`keep_page`, les rangées restent, la bannière se met à jour) ; recherche : ↑ depuis le champ va à son icône, le menu ne marque plus « Accueil », résultats du serveur d'abord, personnes et Seerr ajoutés dessous sans bouger la sélection, Échap ferme le clavier à l'écran ; page Seerr : distribution personne par personne (`sr-cast-i`, `cast-go`), Entrée cherche cette personne.
- **Processeur au repos (Pi, 19 %)** : le décor ne bouge pas ; seuls l'avatar GIF et l'alerte de mot de passe du compte `test` font redessiner (`still_gifs` les fige pour le GIF). Pas de changement de code.
- **Vérifié sur ABYDOS** : redémarrage et arrêt du serveur depuis le tableau de bord. **Pas encore essayés** : pause et arrêt d'une lecture depuis le tableau de bord, création de bibliothèque et chemins, dépôts, activation et installation d'extensions, restauration d'une sauvegarde, mode portrait, thèmes Turtlefin et Nerd sur le tableau de bord. Formulaire « Autre compte » au bureau : ↑ ↓ ne changent pas de champ (Entrée et Tab, oui) ; en mode TV, oui.
- **Toutes les animations coupées** : pas de voile ni d'attente entre deux pages (`nav-go`, `veil`, `skel-go` à 0,9 s), pas d'image partagée
  (`open-hero`), fond préparé avec la page (`page_backdrop`, grille et fiche ; 0,7 s au plus), attentes immobiles.

## 6 duodecies. Téléphone en portrait (5 novembre 2026, pas publié)

Essayé sur un Galaxy A12s (SM-A127F, 720 x 1600, Android 13 ; modeste), APK arm64 `-Release`.

- **Distribution d'une bibliothèque** : en portrait le paquet se pose en haut, sur la première ligne de la grille.
  Les lignes du haut partaient d'abord et se posaient sous le reste du paquet. Maintenant `deal-delay` part du bas
  en portrait (toujours la ligne la plus loin du paquet d'abord) : le paquet se replie vers le haut. Les morceaux
  disparaissent sans fondu en portrait (pas de calque). Au retour, une carte au-delà de `deal-n` rentre dans le
  morceau de sa colonne sur la dernière ligne (`pc-j`) ; avant, elle visait une place sous le paquet.
- **À regarder, Demandes** (une seule rangée de tuiles) : en portrait, la rangée devient une grille (`home-wrap`,
  `wrap-cols`, `sec-h` ; `SectionRow.wrap-cols`). Largeur de carte habituelle, cartes élargies pour remplir la ligne
  (sur le téléphone : vignettes 16:9 sur 1 colonne, affiches sur 2). ↑ ↓ changent de ligne.
- **Menu de gauche** : la barre de navigation d'Android recouvrait « Paramètres » (pas touchable). Le menu réserve
  maintenant `bot` en bas (même règle que les messages : 48 px sur un téléphone en portrait).
- Vu correct en portrait : accueil, fiche, bibliothèque, favoris, collections, listes, téléchargements, Watch party,
  Télécommande.

## 6 terdecies. Code d'un compte (6 novembre 2026, pas publié)

Paramètres → Compte → « Code pour ouvrir ce compte » : Aucun, code à 4 chiffres, ou combinaison de 6 flèches
(clavier, télécommande, manette). Demandé sur « Qui regarde ? » pour ouvrir le compte (y compris Retour après
« Changer de compte ») et pour le retirer de l'appareil ; **pas** quand le compte s'ouvre au lancement (compte de
démarrage, `--user=`). Changer ou retirer le code demande le code actuel. « Code oublié ? » : le mot de passe
du compte sur le serveur le retire (`App.lock_reset`, appliqué dans `start_session`).
- `src/lock.rs` : code haché (`genre:sel:empreinte`, 50 000 tours de SHA-256), comparaison en temps constant ;
  après 5 erreurs, attente de 30 s qui double (1 h au plus), comptée dans `accounts.json` (`lock_fails`,
  `lock_until`) : relancer l'appli n'y change rien. `save_account` garde le code d'une reconnexion à l'autre.
- Interface : `lock-*` dans `ui/app.slint` (pavé de chiffres ou croix de flèches, points, secousse). La
  fenêtre garde le clavier (`changed has-focus`) ; la tuile d'un compte protégé ne lance pas l'envol de la
  photo (`login-locked`). Clavier AZERTY : la rangée du haut sans Maj (& é " ' ( …) compte comme chiffres
  (`lock::typed_digit`), comme le pavé numérique.
- Animations : croix de flèches, boule au milieu qui s'étire en pointe vers la flèche appuyée (`pt-q`,
  `pt-ang`, tourne par le plus court chemin) ; bouton allumé tant que sa touche est tenue (`down`,
  `key-released`) ou 160 ms après un clic (`tap`) ; points qui gonflent en se remplissant.
- Attente : `lock-wait` fait tourner un minuteur d'une seconde (lock-key « tick ») qui recompte le temps
  restant ; `Flow.wrong` garde « Code incorrect » jusqu'à la fin de l'attente.
- Télécommande : en combinaison, OK passe aux boutons du bas (`bar` : ⌫, Annuler, « Code oublié ? »),
  ← → entre eux, ↑ revient au code (rappel « OK : boutons du bas » sous la croix) ; en chiffres, ↓ sous la
  dernière rangée mène à « Code oublié ? » (`lock-sel` 12). Retour/Échap ferme toujours.
- Limite (dite à l'utilisateur) : protège l'accès devant l'écran, pas le fichier (10 000 codes possibles).
- Essayé sur le PC (config à part, faux compte) : mauvais code, bon code, flèches, attente, « Code oublié ».
  Pas essayé à l'écran : le choix du code dans les Paramètres (il faut une vraie session).

## 6 quattuordecies. Choix de l'avatar au doigt et en portrait (7 novembre 2026, pas publié)

Signalé : sur le téléphone, la grille des photos de profil défilait beaucoup trop vite, et rien n'était visible
en portrait.
- **Défilement** : un glissement envoie des dizaines d'événements (`scroll_at`, puis le lancer) et chacun
  avançait d'une rangée entière. Au doigt (`Nav.drag`), la grille et la liste des catégories défilent maintenant
  au pixel (`av-off`, `g-off`, bornés par `av-max`, `g-max`) ; `av-top` et `g-top` suivent la rangée du haut. La
  molette et le clavier gardent une rangée par cran ; le décalage se réaligne sur eux (`changed avt` / `gt`).
  Seules les flèches ramènent la grille sur l'avatar choisi (une autre touche défaisait le défilement).
- **Portrait** (`av-one` : `set-one` et plusieurs catégories) : comme les Paramètres, la liste des catégories en
  pleine largeur, puis la catégorie touchée en plein écran (`av-zone` 1, son nom et son nombre en sous-titre) ;
  Retour, Échap ou ← en première colonne reviennent à la liste. Avant, la colonne des catégories (250 px) ne
  laissait à la grille qu'une vingtaine de pixels. « Actuelle » est caché sur une colonne ; titres coupés (`elide`).
- Au doigt, le sous-titre n'affiche plus l'aide au clavier (« {} avatars en {} catégories », « {} avatars » :
  traduits dans les 7 langues).
- Essayé sur le PC (fenêtre 404 x 721, glissement à la souris : même chemin que le doigt) et en large. Pas encore
  sur le téléphone (pas branché ce jour-là).
- Chulak (serveur Ubuntu, `chulak-1` sur Tailscale) : version Linux x86_64 compilée sur le PC (cargo-zigbuild,
  libfontconfig de chulak-1 dans `%LOCALAPPDATA%/turtlefin-x64-sys`), lancée sur un écran virtuel (Xvfb dans un
  conteneur Docker, `--ipc=host`, écran :99) : langue, visite, thème, recherche (Chulak trouvé), « Qui regarde ? »
  corrects. Pas plus loin sans mot de passe. Remarque : sous Linux, l'effet de changement de langue est sauté
  (« take_snapshot is not supported by this FemtoVG backend »).

## 6 quindecies. Seconde liste « problèmes vue.txt » (11 novembre 2026, pas publié)

- **Couleur du média retirée** partout (`media_tint`, réglage `media_color`) ; bouton Lecture aux couleurs du thème, en
  retrait au repos, plein une fois choisi, sans contour ; plus de contour sur la carte « À suivre ».
- **Défilement** (molette, doigt, souris glissée) : recherche et téléchargements défilent au pixel comme l'accueil
  (`s-free`, `dl-free`, `rows-section-at`) ; choix du compte et listes de choix : une ligne par cran, plus une par événement.
- **Page Seerr** : la barre du haut dessinée par le thème reste visible (`Theme.header-glass`) ; « au hasard », recherche et
  paramètres referment la page (`seerr-drop`). « Passer l'intro » : fond de la même forme que le bouton du thème.
- **Barre du haut** : `Theme.header-style` (1 verre d'Aero · 2 trait d'accent de Turtlefin · 3 bandeau de terminal de Nerd),
  champ `header` d'un thème (« glass », « abyss », « term »). **Logo Aero** : quatre volets aux couleurs de Windows 7
  (`Win7Panes` dans logo.slint, `WIN7_PANES` dans theme.rs pour l'icône, mêmes tracés).
- **Recherche avancée** : ligne de filtres sous le champ (`s-adv`, `s-bar`, `sf-*`, `SearchFilter`) : type (tout, films,
  séries, collections), genres, tags, studios, tags à exclure (retirés ici, le serveur n'a pas ce filtre). Listes par
  `/Items/Filters2` (`/Items/Filters` ne rend plus rien sans dossier parent). Collections de Seerr : carte `seerrcol:<id>`,
  ses films remplacent les résultats (`seerr_collection`, route `/JellyfinEnhanced/jellyseerr/collection/{id}` PAS ESSAYÉE).
- **Fiche** : bloc d'informations à droite du titre (`detail_facts`, `DetailData.facts` : genres, studio, réalisation, tags)
  et rangée « Distribution » (cartes `jfperson:<nom>` : la recherche s'ouvre sur ce nom ; `is-plain` : pas d'image partagée).
- **Effets sonores** (`src/sfx.rs`, Paramètres → Affichage, coupés par défaut) : sons synthétisés par jeu (« abyss » gouttes,
  « aero » verre, « nerd » bips), écrits en WAV dans le cache, joués par un mpv audio à part créé au premier son. Événements :
  move, ok, back, tap, type, toggle, error, start (touches et clics pris dans le filtre d'événements winit : PAS sur Android).
  Un thème choisit son jeu (`sounds`) et peut remplacer chaque son par un fichier (`sound_files`).
- **Thèmes complets** : `background` (image, GIF ou vidéo ; `background_dim`), `logo_file` et `logo_anim`, `dir` (dossier du
  thème, posé à l'import). GIF : images décodées hors du fil de l'interface, 96 Mo au plus (`theme::load_assets`). Vidéo :
  un mpv muet en boucle qui prend la texture vidéo quand rien n'est lu (`video::set_background`). GIF et vidéo ne bougent
  que si « Décor du thème qui bouge » est activé. `theme::apply_tokens` pose un thème sur n'importe quelle fenêtre.
- **Turtlefin Theme Creator** : fenêtre à part du même programme (`turtlefin --theme-creator`, `src/creator.rs`,
  `ui/creator.slint`, raccourci du menu Démarrer) ; l'ancien `tools/theme-creator.html` est supprimé. « Enregistrer » écrit
  `Turtlefin Themes/<nom>/<nom>.tftheme` et copie les fichiers du thème à côté (`theme::export_package`) ; « Installer »
  l'ajoute à prefs.json et le choisit. « Parcourir… » : crate `rfd` (portail XDG sous Linux).
- **Mods** (`src/mods.rs`, Paramètres → Mods) : scripts Lua (crate `mlua`, Lua 5.4 compilé avec l'appli) du dossier
  « Turtlefin Mods », à côté de « Turtlefin Themes ». Non isolés (Lua complet) : l'avertissement est en tête de la page.
  Activation et ordre : `UiPrefs::mods` ; un changement demande « Sauvegarder » puis propose de redémarrer. Journal
  `mods.log` (dossier de configuration), créé seulement si un mod est activé. Fonctions offertes : voir l'en-tête de mods.rs.
- **Abydos** : bibliothèque Animes retirée (fichiers gardés) ; bibliothèques Films et Séries sur
  `D:\Fjolvar\Documents\Abydos Media` (films ouverts de Blender, CC BY, pris sur la chaîne YouTube de Blender par yt-dlp).
- **À refaire après ce lot** : `python tools/flatpak-cargo-sources.py` (Cargo.lock a changé : mlua, rfd) ; traductions (les
  nouveaux textes s'affichent en français) ; APK Android (pas compilée avec ce lot).

## 6 sexdecies. Troisième liste (13 novembre 2026, pas publié)

- **Effets sonores retirés** (pas au goût du mainteneur) : plus de `src/sfx.rs`, du dossier `sounds/` ni de `embed_sounds`
  (build.rs) ; réglages `sfx`, champs `sounds` / `sound_files` des thèmes oubliés à la lecture.
- **Musique de fond d'un thème** (`src/music.rs`, champ `music` du `.tftheme`, copié par `export_package`) : un mpv audio à
  part la joue en boucle. `video::attach` appelle `music::block` : coupée dès qu'un média occupe le lecteur, même en pause,
  reprise au même endroit à la sortie du lecteur. Réglages `UiPrefs::music` (oui par défaut) et `music_volume` (40 %).
  La fonction `sound(fichier)` des mods passe par `music::sound`.
- **Essai d'un thème** (`src/demo.rs`) : bouton « Essayer dans Turtlefin » du créateur → `turtlefin --theme-demo=<fichier>` :
  le vrai Turtlefin, sur un faux serveur Jellyfin interne (127.0.0.1, port libre : bibliothèques, films, séries, saisons,
  épisodes, collections, recherche et filtres, favoris, vu, lecture d'une mire YUV4MPEG fabriquée à la demande), dossiers
  de config / données / cache à part (`<cache>/theme-demo`, vidés à chaque essai), connexion automatique (compte « Démo »),
  fond animé et musique activés. Le créateur réécrit `essai.tftheme` à chaque changement, l'essai le relit (`demo::watch`,
  500 ms). « Créer un thème » reste masqué en interface TV, en borne et sur Android (`external_ok`).
- **Écran de pause** (`PlayerView`, `pause-*`) : après `UiPrefs::pause_info` secondes en pause (5 à 30, ou jamais ; 10 par
  défaut, Paramètres → Lecture), les commandes s'effacent ; logo du média à gauche, « S01E02 - Titre » à sa droite (un
  film : l'année), résumé dessous. Un geste le retire (`show` remet `pause-t` à 0). Logo chargé par `prepare_extras`.
- **Fiche** : bulle du résumé jusqu'au bas de l'affiche, un style par famille (citation pour Turtlefin, volet de verre de
  l'Explorateur pour Aero, terminal pour Nerd) ; affiches des saisons à la taille des autres cartes (`child-card-w`) ;
  contenu des onglets remonté (`tab-lift`) ; hauteur des rangées recalculée dans `rows_for_tab` selon l'interface en cours
  (une rangée gardée d'avant un passage TV ↔ bureau était trop basse : noms des cartes coupés, titres sur les affiches).
- **Recherche** : types en boutons directs (Tout, Films, Séries, Collections) ; listes Genres, Tags, Studios, Exclus
  filtrées par le texte tapé (`sf-query`, `FILTER_ALL`) avec index A–Z à droite (`sf-az`, « # » pour le reste) ;
  suggestions sous le champ (`suggestions`, `sg-items`) : genres, tags, studios qui finissent les derniers mots tapés
  (noms à plusieurs mots compris), « -mot » pour exclure un genre ou un tag ; Tab ou OK l'ajoute aux filtres.
- **Défilement à ressort** (ticket n° 2) : rangées, accueil, recherche, fiche, grille, téléchargements (260 ms, léger
  dépassement).
- Plus aucune mention du Pi dans les textes de l'appli.
- **Recherche en interface TV (retours du 13 novembre au soir)** : suggestions juste sous le champ, au-dessus du clavier à
  l'écran ; ↑ depuis sa première ligne → ligne du haut (`s-sug` : suggestions puis « Recherche avancée »), OK ajoute le
  filtre et rend la main au clavier. Filtres choisis rappelés dans le champ (`sf-tokens`). « Recherche avancée » à droite du
  champ ; filtres : Type, Année (décennies), Note minimale, État (vus, pas vus, favoris) en menus à choix unique
  (`sf-single`, `PICK_*` de main.rs, `api::MoreFilters` : `Years`, `MinCommunityRating`, `IsPlayed`, `Filters=IsFavorite`),
  Genres, Tags, Studios, Exclus en listes à cases (cochées en tête). Résultats choisis au clavier : le haut de la page glisse
  (`lift`), une rangée d'affiches tient en entier. Clavier à l'écran dans la matière du thème (Win7Glass, TermGlass).
- **Fiche** : défilement par `d-fit` (la rangée choisie, ou les onglets et la première rangée de leur contenu, entière si elle
  tient) ; molette bornée par `d-max` (onglet Détails compris ; avant, elle remontait la page). PAS VÉRIFIÉ À LA MOLETTE :
  la molette simulée (mouse_event, WM_MOUSEWHEEL) n'agit pas dans les essais automatisés.
- **Recherche, rangées (14 novembre)** : de haut en bas, champ et « Recherche avancée » (→ depuis le champ), filtres,
  suggestions (4 au plus, noms coupés), clavier à l'écran, résultats ; ↑ ↓ passent à la rangée voisine qui existe
  (`row-now`, `go`, `move` de la page). Filtres rappelés dans le champ : Retour arrière sur un champ vide ou clic sur la
  pastille les retire (`remove_token`, `filter_tokens`). Clavier à l'écran : ‹ › déplacent le curseur (`Osk.back`,
  `cursor-keys`, `Str.tail`), deux appuis rapides sur Maj verrouillent les majuscules (`caps`, trait sous la flèche).
- **Bibliothèque** : affiches réduites pour deux rangées entières (`lib.cw`). **Fiche** : saisons et distribution en grille
  (`d-cols`, `d-pitch`, `kids-h`, `dcast-h`, ↑ ↓ de ligne en ligne), onglet Détails au moins aussi haut qu'une rangée ;
  « Plus de ce genre » : chaque rangée a son titre quand il y en a plusieurs (« Dans ta bibliothèque », Similaires,
  Recommandés).

## 7. Compiler, fabriquer les paquets, publier

Développement :
- Windows : Rust (https://rustup.rs), « Outils de build Visual Studio » (C++), git ; `cargo build --release` ;
  `libmpv-2.dll` est dans le dépôt (`vendor/libmpv/windows-x86_64` et `windows-i686`, compressée en gzip :
  GitHub refuse les fichiers de plus de 100 Mo) ; `build.rs` (`libmpv_next_to_exe`) la décompresse, vérifie
  son SHA-256 et la pose à côté de l'exe, debug comme release, même après un `cargo clean`, sans réseau.
  `build.ps1` la reprend de là. Changer de version : `tools/update-libmpv.ps1 -Download` (ou le chemin
  d'une DLL ; empreinte dans `libmpv-2.dll.sha256`, provenance dans `SOURCE.txt`), voir `vendor/libmpv/README.md`. La DLL ne se
  recrée que quand le script de compilation tourne (après `cargo clean`, ou un changement de `build.rs`,
  de l'interface ou du `.gz`) : DLL supprimée à la main, `touch build.rs`.
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
  Cargo.lock). Essai : `flatpak-builder --user --install --force-clean build-dir <manifeste>`.
  **Pas sur Flathub** : leurs règles (docs.flathub.org, « Generative AI policy ») interdisent un manifeste écrit
  ou aidé par une IA, interdisent qu'une IA ouvre la demande ou y réponde, et refusent les traductions
  automatiques ; le reste est laissé aux relecteurs. Choix du mainteneur : un fichier
  `Turtlefin-<v>-linux-x86_64.flatpak` joint à chaque version par `release.yml` (job `flatpak`, conteneur
  `flatpak-github-actions:freedesktop-25.08` ; le runtime vient de Flathub à l'installation). Installé ainsi,
  rien ne le met à jour : `update::Kind::Flatpak` renvoie vers GitHub, comme une installation inconnue.
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
