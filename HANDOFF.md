# Turtlefin : passation de projet (état au 5 octobre 2026)

Document destiné à Claude Code. Lis-le en entier avant de toucher au code, puis lis `README.md`.
Dépôt : https://github.com/Xelopteryx/Turtlefin · Version dans `Cargo.toml` : 0.3.1.

**Branches** : `main` (état d'origine), `interface-lua` (mpv en processus séparé + interface de lecture en script Lua), `libmpv` (lecteur intégré, voir ci-dessous). Le choix entre `interface-lua` et `libmpv` dépend de la mesure RAM/CPU sur le Pi (section 9).

## 1. Objectif

Remplacer Jellyfin Desktop / jellyfin-web par un **client Jellyfin natif en Rust**, sans Qt ni navigateur embarqué.

Pourquoi :
- Le client Qt/QtWebEngine fuit de la RAM et plante après ~40 min sur le Raspberry Pi (fuite de descripteurs GPU, bug amont non corrigeable).
- L'interface web avec le thème perso (JellySkin + beaucoup de CSS/JS) tombait à ≤ 30 fps sur vieux PC / Pi.
- Beaucoup du CSS du thème ne sert qu'à **cacher** des éléments du client officiel, et il manque des fonctions voulues.

Cibles : Windows (développement et tests) et Linux (Raspberry Pi 5 « Prometheus », kiosque télé). Une passerelle avec **XeLauncher** (lanceur Electron du média center) viendra plus tard, ce n'est pas une priorité.

## 2. Décisions prises

| Sujet | Décision | Raison |
|---|---|---|
| Langage / UI | Rust + **Slint** (rendu 100 % Slint) | Pas de navigateur, léger, Windows + Linux. `build.rs` force le style `fluent-dark` pour ne jamais retomber sur le style « native » (dépend de Qt) |
| Réseau | `reqwest` 0.13 (rustls, magasin de certificats du système), `tokio` | Un certificat mkcert installé sur la machine est accepté. `query` est une feature à activer en 0.13 |
| Lecture (branche `libmpv`) | **libmpv chargée à l'exécution** (`libloading`, `src/mpv.rs`), API de rendu OpenGL : mpv dessine dans une texture que Slint affiche (`src/video.rs`, `BorrowedOpenGLTexture`), commandes en Slint par-dessus (`ui/player.slint`) | Demande de l'utilisateur : un lecteur « natif », pas une app qui en pilote une autre. Plus d'IPC ni de `--wid` (marche aussi sous Wayland). Contrepartie : plus d'isolation de processus ; le lecteur est détruit à chaque fin de lecture, donc sa mémoire reste bornée à une lecture. Flux en lecture directe (`/Videos/{id}/stream?static=true`) |
| Rendu Slint | femtovg (OpenGL / GLES) imposé au démarrage par `BackendSelector`, sauf si `SLINT_BACKEND` est défini | La vidéo passe par une texture OpenGL ; le rendu logiciel ne peut pas l'afficher |
| Création du rendu mpv | `video::attach` renvoie un signal ; la lecture attend que l'interface ait créé le contexte de rendu avant `loadfile` | Sinon mpv ouvre le fichier sans sortie vidéo (« No render context set ») |
| Texture vidéo | Taille de la fenêtre en pixels physiques, origine `TopLeft`, état OpenGL sauvegardé/rétabli autour du rendu mpv | Constaté sur Windows : `BottomLeft` donne une image à l'envers |
| Écrans pendant la lecture | Aucun écran (connexion, accueil, fiche) n'est instancié sous la vidéo | Rien à redessiner à chaque image, et le clavier va au `FocusScope` du lecteur (sinon l'accueil le lui prend) |
| Navigation du lecteur | Commandes masquées : ← → ±10 s sans rien afficher, ↑ ↓ → barre de temps ; barre : ↓ boutons (Lecture/Pause), ↑ Retour ; boutons : ⏮ ép. · chapitre · ⏯ · chapitre · ⏭ ép., Audio / Sous-titres à droite ; heure de fin au centre | Spécification de l'utilisateur (4 octobre 2026) |
| Décodage | `hwdec=no` sur Linux ARM 64 bits ; `auto-safe` ailleurs | Pi 5 : le décodage matériel V4L2 sort du format Broadcom SAND que Vulkan ne savait pas importer (écran bleu). Avec libmpv/OpenGL, `TURTLEFIN_HWDEC=auto-copy` est à essayer. Windows : `d3d11va-copy` (pas d'interop directe avec le GL de Slint), ~5 % d'un cœur de plus que mpv séparé |
| Audio Linux | `ao=pipewire,pulse,alsa`, `audio-device=auto` ; libmpv ne lit pas `mpv.conf` (`config=no`) | `~/.config/mpv/mpv.conf` impose `ao=alsa` / `plughw:1,0` qui échoue quand PipeWire occupe la sortie HDMI |
| Mémoire | Cache réseau mpv plafonné (100 MiB avant / 25 MiB arrière) ; images demandées à la bonne taille ; 16 éléments par rangée d'accueil | Le Pi a 4 Go |
| Style | Pas de flou temps réel (backdrop-filter), pas d'ombres portées animées | C'était la cause principale des fps bas du thème web |
| Fond (backdrop) de fiche | **Repoussé**, à faire plus tard comme **option**, avec logos transparents | Demande de l'utilisateur |
| Ligne de commande | `turtlefin [USER [PASS]] [--tv\|--desktop] [--server=URL]` + `TURTLEFIN_PASSWORD` | Pour remplacer Jellyfin Desktop dans le kiosque. Un argument est visible des autres processus : préférer la variable d'environnement |

Préférences de travail de l'utilisateur :
- Il compile et teste lui-même (`cargo run --release`) sur son PC Windows, puis sur le Pi, et renvoie les erreurs telles quelles. (Cette consigne « ne pas compiler » valait pour l'ancien assistant en ligne ; Claude Code peut compiler sur sa machine.)
- Réponses en français. Il connaît peu les commandes Linux/SSH : explique-les.

## 3. Environnement de test

- **PC Windows** : développement, compilation rapide. Branche `libmpv` : `libmpv-2.dll` (build shinchiro, `mpv-dev-x86_64-*.7z`) à côté de `turtlefin.exe`. Branche `interface-lua` : `mpv.exe` dans le PATH (`C:\mpv`).
- **Raspberry Pi 5 4 Go**, Raspberry Pi OS, hostname `Prometheus`, utilisateur `xelopteryx`, projet dans `/home/xelopteryx/turtlefin`. Affichage X11 (kiosque `.xinitrc`), son PipeWire (sortie HDMI). Lancement depuis SSH :
  `DISPLAY=:0 XAUTHORITY=/home/xelopteryx/.Xauthority ./target/release/turtlefin <user> --server=http://... --tv` (mot de passe via `TURTLEFIN_PASSWORD`).
- **Serveur Jellyfin 10.11.11**, joint via Tailscale depuis le Pi (tailscaled monte à 100 % d'un cœur pendant la lecture : préférer l'adresse LAN quand c'est possible). Compte de test : `test`.
- Dépendances Pi : `build-essential cmake pkg-config libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libxcb1-dev libgl1-mesa-dev libegl1-mesa-dev`, plus `libmpv2` (branche `libmpv`) ou `mpv` (branche `interface-lua`).

## 4. Structure du code

```
Cargo.toml      deps : slint, tokio, reqwest 0.13, serde, anyhow, directories 6, uuid, image, libloading, glow, chrono
build.rs        compile ui/app.slint avec le style fluent-dark
ui/theme.slint  jetons de thème (Theme : accents, verre sombre de Style.css)
ui/app.slint    Card, SectionRow, ActionButton, OverviewPanel, AppWindow (écrans : login, loading, home, detail, lecture)
ui/player.slint écran de lecture : vidéo, barre de temps + chapitres, boutons, menus des pistes, navigation clavier
src/main.rs     CLI (+ --test-video), état partagé App (Arc), flux login/accueil/fiche, navigation (pile), lecture, images
src/api.rs      client Jellyfin REST : login, vues, reprise, à suivre, derniers ajouts, fiche, enfants, images (cache disque), rapports de lecture
src/mpv.rs      liaison minimale libmpv (chargement dynamique) : lecteur, propriétés observées, événements, rendu OpenGL
src/video.rs    texture OpenGL de la vidéo, branchée sur le rappel de rendu de Slint
src/player.rs   lecture : options mpv, rapports au serveur, pistes, chapitres, épisode précédent/suivant, enchaînement
src/config.rs   session sauvegardée (serveur + jeton, jamais le mot de passe), chmod 0600 sous Unix
README.md       usage, touches, variables d'environnement, diagnostics
```

Principes :
- Les données réseau passent par des structures `Send` (`SectionData`, `CardInfo`), puis `upgrade_in_event_loop` les pousse dans les modèles Slint. Les images sont décodées hors du thread UI (`spawn_blocking`), 6 téléchargements en parallèle (sémaphore), appliquées par `set_card_image` / `set_child_image` avec une **garde sur l'id** (évite d'écrire sur une carte rechargée entre-temps).
- `App.gen` (AtomicU64) invalide les chargements de fiche périmés. `App.stack` est la pile de navigation. `home_stale` force le rechargement de l'accueil après une lecture.
- Navigation clavier faite à la main (un seul `FocusScope`, indices `sel-section/sel-item`, `d-zone/d-button/d-child`) car Slint ne gère pas ça pour des cartes dynamiques.
- Jellyfin 10.11 : `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest` (renvoie un tableau), `/Items/{id}`, `/Items?parentId=`, en-tête `Authorization: MediaBrowser Client=..., Token=...`. Rapports : `POST /Sessions/Playing`, `/Progress` (toutes les 10 s et à chaque pause), `/Stopped`.

## 5. État par jalon

### Terminé et vu fonctionner par l'utilisateur
- **M0** squelette, thème (jetons dans `Theme`), connexion serveur + utilisateur, jeton stocké.
- **M1** accueil : rangée « Mes médias » (vignettes 16:9), « Reprendre », « À suivre », « Récemment ajouté » par bibliothèque, posters chargés en asynchrone, navigation clavier. (Validé sur capture d'écran Windows.)
- **M3** lecture mpv (processus séparé) : démarre, reprise, son et image OK sur le Pi après les correctifs hwdec/ao. Ligne de commande utilisée sur le Pi.
- Test d'endurance Pi (14 min) : **Turtlefin stable à ≈ 112 Mo** ; mpv 395 → 440 Mo (voir « Problèmes connus »).
- Menu des pistes au clavier (`a` / `s`, première version dessinée par mpv) : le changement de piste fonctionne.

### Branche `libmpv` : vérifié par Claude sur le PC Windows le 4 octobre 2026 (captures d'écran), pas encore par l'utilisateur
- **M2** fiche détail (vue sur capture : poster, logo, sous-titre, boutons, infos, résumé).
- Lecture intégrée avec libmpv sur le vrai serveur (FMA S1E2, compte `test`) : image (4:3 avec bandes), chapitres réels sur la
  barre, reprise, menus audio (FRE AC3 / JPN TrueHD) et sous-titres (Forced / Complet ASS), épisode suivant (S1E2 → S1E3 sans
  quitter le lecteur), retour à la fiche après Échap, deux cycles lecture/arrêt sans fuite (≈ 240 Mo après arrêt).
- Navigation clavier du lecteur (↓ barre, ↓ boutons, → chapitre suivant, Entrée = saut de chapitre ; `s` menu ; Échap).
- Vidéo d'essai sans serveur : `turtlefin --test-video=fichier` (+ `TURTLEFIN_MPV_ARGS="--chapters-file=... --audio-files=... --sub-files=..."`).
- Mesure sur le PC (même épisode, 20 s) : mpv séparé 420 Mo (Turtlefin 200 + mpv 224), 0,8 % d'un cœur ;
  libmpv 399 Mo, 4,5 à 5,6 % d'un cœur (décodage `d3d11va-copy`). **Reste à mesurer sur le Pi**, où les deux décodent en logiciel.
- Vérifié par l'utilisateur sur le Pi : compilation, lecture fluide, mode TV. Pas testé : souris réelle (clic, glisser la barre), Wayland,
  enchaînement automatique en fin d'épisode, « Reprendre » à jour dans Jellyfin Web.

### Menus au style JellySkin (4 octobre 2026) : vérifié sur captures (PC et télé du Pi)
- Police Montserrat intégrée (`ui/fonts`, OFL), fond #010e18, titres de section précédés d'un trait.
- En-tête : Accueil / Favoris / Demandes (si Seerr), recherche et profil (écrans d'attente ; Profil = déconnexion),
  horloge. Navigation : ↑ depuis la première rangée monte dans l'en-tête.
- Cartes : encadré translucide, image aux coins du haut arrondis au décodage (`Shape::card_top`), titre centré,
  sélection = encadré plus sombre + `transform-scale` 1,06. Reprendre / À suivre en 16:9 (vignette Thumb de la série,
  comme le client web) avec avancement et note. Rangées positionnées en pixels calculés en Rust (`y-px`, `h-px`).
- **Onglet Demandes** : via le plugin Jellyfin Enhanced (relais Seerr avec la connexion Jellyfin, sans clé côté client).
  `GET /JellyfinEnhanced/jellyseerr/user-status` (actif + compte relié → onglet affiché),
  `GET /JellyfinEnhanced/jellyseerr/request?take=100&filter=all` filtré sur `requestedBy.id`, titre/affiche par
  `/JellyfinEnhanced/jellyseerr/{movie|tv}/{tmdbId}`, affiches TMDB (`image.tmdb.org`, cache disque). Rangées :
  En attente (1), Acceptées (2, 5), Refusées (3), En échec (4). Une demande disponible ouvre sa fiche Jellyfin.
  Vérifié avec de vraies demandes (compte `test`).
- Backdrop flouté : repoussé, en option (décision de l'utilisateur).
- **Navigation commune** (`NavBar`) : ☰ menu sur chaque écran, ← Retour seulement si la pile n'est pas vide
  (`can-back`), ⌂ Accueil hors accueil. ↑ depuis le haut d'un écran monte dans la barre (sur Retour s'il existe).
  Le bouton « Retour » des fiches a été retiré. Les FocusScope des écrans reprennent le clavier via `refocus`.
- **Menu latéral** : Navigation (Accueil, Demandes), Bibliothèques, Compte (Sélectionner un serveur = déconnexion
  vers l'écran de connexion, Paramètres = écran « bientôt », Se déconnecter, Fermer l'application).
- **Bibliothèques / collections / dossiers** : grille centrée (écran `library`) au lieu d'une fiche, pages de 60,
  page suivante chargée à l'approche de la fin ; films et séries en recherche récursive par type comme le client web.
  Retour depuis une fiche : la sélection est retrouvée.
- « À suivre » n'affiche plus les épisodes commencés (`enableResumable=false` + exclusion de ceux de « Reprendre »).
- Onglet Demandes vérifié avec de vraies demandes (compte `test`).

### Serveurs, fiche enrichie, lecteur, téléchargements (4 octobre 2026, vérifié sur le PC, compte `test`)
- **Serveurs** (`src/discovery.rs`) : découverte UDP Jellyfin + sondage du /24 local et des appareils Tailscale en
  ligne (`tailscale status --json`), ports 8096 / 8097 / 8920, regroupement par identifiant de serveur. Le serveur
  de l'utilisateur (Chulak, Jellyfin 12.1, conteneur) ne répond pas en UDP mais est trouvé par sondage :
  local http://192.168.1.32:8097, Tailscale http://100.111.157.87:8097. Saisie manuelle (locale + distante) :
  sans schéma, http et https essayés, choix demandé si les deux répondent. `Saved` : `server_local`,
  `server_remote`, `prefer_remote` (Paramètres > Réseau) ; adresse manquante complétée en arrière-plan.
- **Fiche** : boutons ▶ ♥ ✓ ⬇ (⬇ si `Policy.EnableContentDownloading`), rangées Casting et équipe, Plus de ce
  genre (`/Items/{id}/Similar`), Similaires / Recommandés de Seerr (`/JellyfinEnhanced/jellyseerr/{movie|tv}/{tmdb}/…`).
- **Lecteur** : ↑↓ → Pause sélectionné ; ↓ depuis les boutons → épisodes de la saison ; au générique
  (`/MediaSegments/{id}` Outro, sinon chapitre « Ending… », sinon 3 % de la durée, min 40 s) : « Épisode suivant »
  / « Saison suivante », ou 3 suggestions au hasard de « Plus de ce genre » avec la vidéo réduite (sélectionnable) ;
  écran de fin avec « Retour à l'accueil » (`player::Exit::Home`).
- **Téléchargements** (`src/downloads.rs`) : `/Items/{id}/Download` dans `<données>/turtlefin/downloads/<id>/`
  (+ affiche, vignette, sous-titres externes, `info.json`), file d'attente un par un, écran Téléchargements
  (Lire / Supprimer), lecture locale sans rapport au serveur, démarrage hors ligne si le serveur est injoignable.
  **Pas testé** : le transfert réel (le compte `test` n'a pas le droit de télécharger) et le démarrage hors ligne.
  Pas fait : synchroniser la progression / « vu » des lectures hors ligne au retour en ligne.

### Lots du 4 octobre 2026 (soir) : vérifiés sur le PC (compte `test`), watch party entre PC et Pi
1. Téléchargements : jeton dans l'en-tête (`/Items/{id}/Download` refuse `api_key`, 401) — transfert réel vérifié,
   démarrage hors ligne vérifié. « Passer l'intro » (segments Intro Skipper), carte « épisode suivant » en bas à
   gauche, écran de fin (fond = image floutée, animations), barre ← ⌂ ☰, séparateurs du menu. Option `--play=ID@SECONDES`.
2. Navigation entre rangées : carte visuellement au-dessus / en dessous (`row-pick` / `row-seen` en Rust).
3. Seerr : badge SEERR (seulement si absent du serveur), page Seerr au premier plan, bouton Demander
   (`POST /JellyfinEnhanced/jellyseerr/request`, **non testé** : pas de vraie demande créée).
4. Fiche : boutons Audio / Sous-titres, préférence par série (`config/tracks.json`), appliquée via alang/slang/sid.
5. Recherche (bibliothèque + Seerr), dé « au hasard » (non vu), clavier à l'écran en mode TV (`ui/osk.slint`).
6. Paramètres : Profil (avatars GetAvatar : `/GetAvatar/Avatars`, `/GetAvatar/Image/{id}`, `POST /GetAvatar/SetAvatar`),
   Lecture (préférences du compte Jellyfin), Réseau, Compte. Avatar dans l'en-tête (GIF de 2 Mo : lent à charger).
7. Watch party (`src/syncplay.rs`, WebSocket `/socket` avec le jeton dans l'en-tête — `api_key` refusé, 403) :
   créer / rejoindre / quitter vérifiés entre `test` (Pi) et `test2` (PC) ; lancement synchronisé et pause / reprise
   communes vérifiés (même image des deux côtés). **Saut synchronisé non vérifié** (tests clavier ratés).

### Lots du 5 octobre 2026 : vérifiés sur le PC (captures), pas encore sur le Pi
1. Barre du haut sur toutes les pages (watch party, dé, recherche, avatar, heure), fiche à deux lignes de boutons
   (icônes / pastilles), pastilles d'épisodes restants et de statut Seerr sur les affiches, page Seerr plein écran TV.
   Avatar : GetAvatar `SetAvatar` répond 500 pour tout le monde (plugin) → repli `POST /UserImage` (base64).
2. **Clavier à l'écran** refait (`ui/osk.slint`, façon téléphone : lettres AZERTY / chiffres-symboles / accents),
   réutilisé par la recherche et la connexion.
3. **Écran de connexion** « Qui regarde ? » : comptes enregistrés sur l'appareil (`accounts.json` : jeton, jamais le
   mot de passe), comptes publics du serveur (`/Users/Public`), « Autre compte » (formulaire). Menu « Changer de
   compte ». Connexion par tuile enregistrée vérifiée (session `test` ouverte sans mot de passe). Jeton refusé → le
   compte est oublié et le mot de passe demandé.
4. **Avatars GIF animés** (tuiles de connexion, en-tête, choix d'avatar) : toutes les images décodées une fois
   (150 au plus, ~100 Ko chacune), une minuterie Slint les fait défiler, en pause pendant la lecture et quand
   l'avatar n'est pas affiché. Réglage **Paramètres → Affichage → GIF figés** (`still_gifs` dans `session.json`) :
   première image seulement, image réduite par le serveur. Le réglage lui-même n'a pas été essayé à l'écran.
5. **Téléchargements** : liste générale (séries, films) → saisons → épisodes (vignettes 16:9), bandeau d'infos de la
   carte sélectionnée (année, durée, note, reprise / vu, taille, résumé). `info.json` garde série, saison, numéros,
   année, note, durée, et `series.jpg` / `season.jpg` ; les anciens téléchargements sont complétés dès que le
   serveur répond (`downloads::enrich`, champ `meta`).
6. **Hors ligne** : pastille verte « Hors ligne » dans l'en-tête, titre de la barre en vert. Une lecture locale note sa
   position et « vu » (règles Jellyfin : vu après 90 %, pas de reprise sous 5 %) avec `dirty: true` ; le serveur est
   réessayé toutes les 30 s, et au retour : `POST /UserItems/{id}/UserData` (position, vu, date), puis l'accueil.
   Vérifié : reprise à 5 min envoyée, l'épisode est apparu dans « Reprendre ».
7. Messages (toast) : bulle centrée en bas, effacée après 6 s.

Essais : `TURTLEFIN_CONFIG_DIR=<dossier>` (autre session / comptes, sans toucher à la vraie), `--open=downloads`.
Sur Windows, la version debug a besoin de `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.

### Pas fait
- Manette / télécommande ; défilement à la molette, survol souris qui déplace le focus.
- Quick Connect sur l'écran de connexion ; clavier à l'écran pour la saisie manuelle d'un serveur.
- Favoris faits hors ligne (rien ne permet d'en ajouter hors ligne pour l'instant).
- Fond (backdrop) en option, avec logos transparents.
- Passerelle XeLauncher ; démarrage automatique sur le Pi ; compilation/paquetage.

## 6. Reprise du CSS/JS du thème (correspondances)

Reproduit : boutons de fiche à dégradé, résumé en bloc translucide cliquable + panneau complet, poster agrandi, poster de saison sur les épisodes (`Portrait_Poster.js`), saisons sur une rangée horizontale, colonne « À suivre » resserrée, « Mes médias » horizontal, logo en haut de fiche, langues audio dans la ligne d'infos, bouton Déconnexion au dégradé.
Volontairement absent : tout ce que le CSS masquait (genres, studios, tags, liens externes, titre original, réalisateurs, sélecteurs de pistes) ; flous et ombres animées.
Pas encore : cartes de suggestions de recherche, picker d'avatar, backdrop.

## 7. Problèmes connus / limites

1. Branche `libmpv` : un plantage de mpv fait planter Turtlefin (même processus). Le lecteur est recréé à chaque lecture.
2. Branche `libmpv` : la lecture exige le rendu OpenGL de Slint (`SLINT_BACKEND=winit-software` l'empêche).
2b. **Bug de mpv 0.40 / 0.41** (corrigé dans mpv le 23 janvier 2026, commit f74adc4, pas encore publié) : une barrière OpenGL
   (glFenceSync) par image jamais libérée avec libmpv. Sur le Pi (mpv 0.40/0.41, pilote v3d) chacune occupe un fichier :
   « MESA: error: Export failed » après ~42 s (limite de 1024). Contournement dans `src/mpv.rs` (OpenGL ES uniquement) :
   tampons persistants cachés à mpv, barrières créées pendant le dessin d'une image notées et libérées si mpv ne l'a pas fait.
   Diagnostic : `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` doit rester stable (~50) pendant la lecture.
   **Vérifié sur le Pi le 4 octobre 2026** : 46 à 48 fichiers ouverts stables sur 2 min de lecture, plus aucun message ;
   Turtlefin ≈ 99 % CPU (contre 120 % avec la fuite) et 411 à 452 Mo (ancienne version : Turtlefin 112 + mpv 395-440 Mo).
3. **Croissance RAM de mpv** (~3 Mo/min, par paliers, sur un épisode de FMA aux sous-titres ASS) : cause non élucidée (polices/glyphes libass probable). Test à faire avec `TURTLEFIN_MPV_ARGS="--sid=no"`. Borné à la durée d'une lecture puisque mpv est relancé à chaque vidéo.
4. Cache d'images disque sans purge (`<cache>/turtlefin/img`).
5. Jeton d'accès en clair dans `session.json` (0600 sous Unix) ; avec libmpv, il n'apparaît plus dans une ligne de commande.
6. Le multi-ligne avec points de suspension du résumé dépend de la version de Slint (au pire coupure nette).
7. Le `FocusScope` de taille nulle de la fiche : si les flèches ne répondent pas, regarder là.
8. Le défilement mémorisé des rangées de l'accueil est perdu quand on revient de la fiche (l'écran est recréé).
10. **Saccades au défilement des menus sur le Pi** : c'était du **déchirement d'image** (Xorg modesetting + Openbox sans
   compositeur, pas d'option TearFree : le haut de l'écran montrait l'image précédente). Turtlefin tient 59 images/s
   (écran 59,8 Hz). Corrigé par le compositeur **picom** (vérifié par l'utilisateur le 4 octobre 2026) :
   `MESA_GL_VERSION_OVERRIDE=3.3 MESA_GLSL_VERSION_OVERRIDE=330 picom --backend egl --vsync` (le moteur GLX ne démarre pas
   sur le v3d, picom 12 exige GLSL 3.30 alors que le pilote annonce 3.1). Coût : picom ~2,5 % d'un cœur, lecture inchangée.
   Avant ça : cartes arrondies avec `clip: true` remplacées par des images préformées (évite les couches hors écran).
   Mesures : `SLINT_DEBUG_PERFORMANCE=refresh_full_speed,console` + `TURTLEFIN_DEBUG_FRAMES=1` (images > 25 ms, hors vidéo).
9. Décodage logiciel sur le Pi : peut peiner en 4K/HEVC lourd ; piste future : `TURTLEFIN_HWDEC=auto-copy` avec libmpv (copie en mémoire, évite l'import SAND), sinon v4l2request / drm-copy.

## 8. Variables d'environnement utiles

`TURTLEFIN_PASSWORD`, `TURTLEFIN_LIBMPV` (chemin de libmpv), `TURTLEFIN_MPV_ARGS`, `TURTLEFIN_MPV_LOG`, `TURTLEFIN_HWDEC`, `TURTLEFIN_AO`, `TURTLEFIN_INSECURE=1` (test uniquement), `TURTLEFIN_CONFIG_DIR` (autre dossier de session, comptes et pistes : essais), `SLINT_BACKEND=winit-software`.

## 9. Ordre de travail proposé

1. L'utilisateur teste la branche `libmpv` sur Windows (souris, mode TV, ressenti).
2. Pousser `interface-lua` et `libmpv` sur GitHub ; sur le Pi, compiler les deux (`CARGO_TARGET_DIR` différent) et mesurer RAM + CPU
   sur le même épisode. Garder `libmpv` si elle est au moins aussi légère, sinon `interface-lua`.
3. Fusionner la branche retenue dans `main`, puis test d'endurance de 1 à 2 h sur le Pi.
4. **M4a** : langues audio / sous-titres préférées lues dans le profil Jellyfin.
5. **M4b** : recherche avec cartes à poster, puis grille de bibliothèque paginée.
6. **M5** : profils / Quick Connect / clavier à l'écran, backdrop optionnel.
7. Passerelle XeLauncher, démarrage automatique, paquetage.

À chaque étape : garder Turtlefin stable en mémoire (c'est la raison d'être du projet) et ne jamais réintroduire de flou temps réel ou d'animation de filtre.
