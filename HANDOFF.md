# Turtlefin : passation de projet (état au 4 octobre 2026)

Document destiné à Claude Code. Lis-le en entier avant de toucher au code, puis lis `README.md`.
Dépôt : https://github.com/Xelopteryx/Turtlefin · Version dans `Cargo.toml` : 0.3.1 (le menu des pistes a été ajouté depuis sans changer le numéro).

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
| Lecture | **mpv en processus séparé**, piloté par IPC JSON | Isolation : une fuite ou un crash de mpv ne touche pas l'UI. Flux en lecture directe (`/Videos/{id}/stream?static=true`) |
| Intégration vidéo | mpv dessine **dans la fenêtre Turtlefin** via `--wid` (HWND Windows, XID X11) | Demande explicite de l'utilisateur. Impossible sous Wayland → repli sur une fenêtre mpv séparée (message affiché) |
| Contrôles pendant la lecture | Turtlefin reçoit les touches et les envoie à mpv par IPC | Valable aussi avec une télécommande qui émule le clavier |
| Interface de lecture | Script Lua `src/turtlefin_ui.lua` (inclus dans le binaire, écrit dans le dossier temporaire, chargé par `--script`, `--osc=no`) : barre de contrôle + menus des pistes en ASS, couleurs de `Style.css` | Rien de Slint ne peut s'afficher par-dessus la vidéo intégrée (fenêtre enfant). Turtlefin transmet les touches par `script-message-to turtlefin_ui key <nom>` ; le script gère aussi souris et touches reçues directement par mpv |
| Décodage | `--hwdec=no` sur Linux ARM 64 bits ; `auto-safe` ailleurs | Pi 5 : le décodage matériel V4L2 sort du format Broadcom SAND que Vulkan ne sait pas importer (« Mapping hardware decoded surface failed », écran bleu) |
| Audio Linux | `--ao=pipewire,pulse,alsa --audio-device=auto` | `~/.config/mpv/mpv.conf` impose `ao=alsa` / `plughw:1,0` qui échoue quand PipeWire occupe la sortie HDMI |
| Mémoire | Cache réseau mpv plafonné (100 MiB avant / 25 MiB arrière) ; images demandées à la bonne taille ; 16 éléments par rangée d'accueil | Le Pi a 4 Go |
| Style | Pas de flou temps réel (backdrop-filter), pas d'ombres portées animées | C'était la cause principale des fps bas du thème web |
| Fond (backdrop) de fiche | **Repoussé**, à faire plus tard comme **option**, avec logos transparents | Demande de l'utilisateur |
| Ligne de commande | `turtlefin [USER [PASS]] [--tv\|--desktop] [--server=URL]` + `TURTLEFIN_PASSWORD` | Pour remplacer Jellyfin Desktop dans le kiosque. Un argument est visible des autres processus : préférer la variable d'environnement |

Préférences de travail de l'utilisateur :
- Il compile et teste lui-même (`cargo run --release`) sur son PC Windows, puis sur le Pi, et renvoie les erreurs telles quelles. (Cette consigne « ne pas compiler » valait pour l'ancien assistant en ligne ; Claude Code peut compiler sur sa machine.)
- Réponses en français. Il connaît peu les commandes Linux/SSH : explique-les.

## 3. Environnement de test

- **PC Windows** : développement, compilation rapide. `mpv.exe` doit être à côté de `turtlefin.exe` ou dans le PATH.
- **Raspberry Pi 5 4 Go**, Raspberry Pi OS, hostname `Prometheus`, utilisateur `xelopteryx`, projet dans `/home/xelopteryx/turtlefin`. Affichage X11 (kiosque `.xinitrc`), son PipeWire (sortie HDMI). Lancement depuis SSH :
  `DISPLAY=:0 XAUTHORITY=/home/xelopteryx/.Xauthority ./target/release/turtlefin <user> --server=http://... --tv` (mot de passe via `TURTLEFIN_PASSWORD`).
- **Serveur Jellyfin 10.11.11**, joint via Tailscale depuis le Pi (tailscaled monte à 100 % d'un cœur pendant la lecture : préférer l'adresse LAN quand c'est possible). Compte de test : `test`.
- Dépendances Pi : `build-essential cmake pkg-config libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libxcb1-dev libgl1-mesa-dev libegl1-mesa-dev mpv`.

## 4. Structure du code

```
Cargo.toml      deps : slint (+raw-window-handle-06), tokio, reqwest 0.13, serde, anyhow, directories 6, uuid, image
build.rs        compile ui/app.slint avec le style fluent-dark
ui/app.slint    tout l'UI : Theme, Card, SectionRow, ActionButton, OverviewPanel, AppWindow
                (écrans : login, loading, home, detail ; overlay « Lecture en cours »)
src/main.rs     CLI, état partagé App (Arc), flux login/accueil/fiche, navigation (pile), lecture, images
src/api.rs      client Jellyfin REST : login, vues, reprise, à suivre, derniers ajouts, fiche, enfants, images (cache disque), rapports de lecture
src/player.rs   lancement de mpv, IPC (socket Unix / pipe nommé Windows), rapports de lecture, menu des pistes
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
- **M3** lecture mpv : démarre, reprise, son et image OK sur le Pi après les correctifs hwdec/ao. Ligne de commande utilisée sur le Pi.
- Test d'endurance Pi (14 min) : **Turtlefin stable à ≈ 112 Mo** ; mpv 395 → 440 Mo (voir « Problèmes connus »).

### Codé, compile (l'utilisateur dit « aucune erreur »), mais jamais confirmé visuellement
- **M2** fiche détail série / saison / épisode / film : poster (épisode → poster de saison), logo (sinon titre), boutons dégradé violet→bleu (Lecture/Reprendre, Voir la saison, Voir la série, Retour), ligne année · classification · durée · note · langues audio, résumé dans un bloc translucide cliquable + panneau « voir plus » défilable, rangée saisons/épisodes/contenu, bloc « À suivre » à droite pour les séries, pile de navigation avec Échap/Retour arrière.
- Bouton Lecture intelligent : série → prochain épisode (`NextUp`, sinon premier), saison → premier non vu, film/épisode → reprise.
- Rapport de progression au serveur (à vérifier : après un arrêt en cours d'épisode, « Reprendre » doit être à jour sur l'accueil et dans Jellyfin Web).
- Sous-titres externes (srt/ass/vtt) ajoutés via `--sub-file`.
- Vidéo intégrée dans la fenêtre : testée sur le Pi (X11) avec image + son ; comportement Windows (scintillement au redimensionnement ?) à confirmer.

### Validé : menu des pistes au clavier (première version, `a` / `s`)
Le changement de piste audio / sous-titres fonctionne (confirmé par l'utilisateur le 4 octobre 2026).

### Codé le 4 octobre 2026 : compile (`cargo check`), rendu vérifié dans mpv sur une vidéo de test, **pas testé dans Turtlefin**
- **Interface de lecture** (`src/turtlefin_ui.lua`) qui remplace le menu clavier : barre en verre sombre (titre, temps,
  progression cliquable à dégradé, boutons pilule Lecture/Pause, -10 s, +10 s, Audio, Sous-titres, Arrêter), panneaux de
  pistes (élément actif en dégradé #a95bc2 → #00a4db, point blanc = piste en cours). Souris (survol, clic, molette),
  clavier/télécommande (voir README). Masquage après 3 s ; sous-titres remontés pendant que la barre est visible.
  `player.rs` ne fait plus que transmettre les touches au script (`q` = quitter directement).
  À vérifier : souris dans la vidéo intégrée sous Windows et sur le Pi (X11), lisibilité en mode TV, Échap/Retour arrière.

### Pas fait
- **M4** : recherche (cartes à poster comme le JS `search_suggestion_poster.js`), réglages, manette/télécommande.
- Enchaînement automatique des épisodes.
- Langues audio / sous-titres préférées (lire `GET /Users/{id}` → `Configuration`: `AudioLanguagePreference`, `SubtitleLanguagePreference`, `SubtitleMode`, `PlayDefaultAudioTrack` et passer `--alang` / `--slang` à mpv).
- Grille de bibliothèque paginée (aujourd'hui : 60 premiers éléments d'une bibliothèque/collection).
- Défilement à la molette, survol souris qui déplace le focus.
- Écran de connexion « vrai » (sélecteur de profils avec avatars via `/Users/Public`, Quick Connect, clavier à l'écran pour la télé). La connexion en ligne de commande couvre le kiosque en attendant.
- Fond (backdrop) en option, avec logos transparents.
- Interface Slint par-dessus la vidéo = libmpv (gros chantier, seulement si nécessaire).
- Passer l'intro (segments média Jellyfin 10.10+), avatar utilisateur dans l'en-tête, picker d'avatar (`Avatar_picker.js` du thème).
- Passerelle XeLauncher ; démarrage automatique sur le Pi ; compilation/paquetage.

## 6. Reprise du CSS/JS du thème (correspondances)

Reproduit : boutons de fiche à dégradé, résumé en bloc translucide cliquable + panneau complet, poster agrandi, poster de saison sur les épisodes (`Portrait_Poster.js`), saisons sur une rangée horizontale, colonne « À suivre » resserrée, « Mes médias » horizontal, logo en haut de fiche, langues audio dans la ligne d'infos, bouton Déconnexion au dégradé.
Volontairement absent : tout ce que le CSS masquait (genres, studios, tags, liens externes, titre original, réalisateurs, sélecteurs de pistes) ; flous et ombres animées.
Pas encore : cartes de suggestions de recherche, picker d'avatar, backdrop.

## 7. Problèmes connus / limites

1. **Wayland** : pas de `--wid`, mpv s'ouvre dans sa propre fenêtre (toast explicatif). `TURTLEFIN_EMBED=0` force ce mode partout.
2. **Rien de Slint au-dessus de la vidéo intégrée** (fenêtre enfant) : d'où le menu de pistes dessiné par mpv.
3. **Croissance RAM de mpv** (~3 Mo/min, par paliers, sur un épisode de FMA aux sous-titres ASS) : cause non élucidée (polices/glyphes libass probable). Test à faire avec `TURTLEFIN_MPV_ARGS="--sid=no"`. Borné à la durée d'une lecture puisque mpv est relancé à chaque vidéo.
4. Cache d'images disque sans purge (`<cache>/turtlefin/img`).
5. Jeton d'accès visible dans la ligne de commande de mpv et en clair dans `session.json` (0600 sous Unix).
6. Le multi-ligne avec points de suspension du résumé dépend de la version de Slint (au pire coupure nette).
7. Le `FocusScope` de taille nulle de la fiche : si les flèches ne répondent pas, regarder là.
8. Le défilement mémorisé des rangées de l'accueil est perdu quand on revient de la fiche (l'écran est recréé).
9. Décodage logiciel sur le Pi : peut peiner en 4K/HEVC lourd ; piste future : v4l2request / drm-copy ou un mpv/driver adapté au SAND de Broadcom.

## 8. Variables d'environnement utiles

`TURTLEFIN_PASSWORD`, `TURTLEFIN_MPV` (chemin de mpv), `TURTLEFIN_MPV_ARGS`, `TURTLEFIN_MPV_LOG`, `TURTLEFIN_EMBED=0`, `TURTLEFIN_HWDEC`, `TURTLEFIN_AO`, `TURTLEFIN_INSECURE=1` (test uniquement), `SLINT_BACKEND=winit-software`.

## 9. Ordre de travail proposé

1. Compiler la version actuelle sur Windows, corriger les éventuelles erreurs du menu de pistes, tester : lecture, `a` / `s`, reprise, retour à l'accueil (« Reprendre » à jour), boutons de la fiche, bloc « À suivre ».
2. Pousser sur GitHub, `git pull` + `cargo build --release` sur le Pi ; refaire un test d'endurance de 1 à 2 h (`ps -o rss` toutes les minutes).
3. **M4a** : enchaînement automatique + langues préférées lues dans le profil Jellyfin.
4. **M4b** : recherche avec cartes à poster, puis grille de bibliothèque paginée.
5. **M5** : profils / Quick Connect / clavier à l'écran, backdrop optionnel.
6. Passerelle XeLauncher, démarrage automatique, paquetage.

À chaque étape : garder Turtlefin stable en mémoire (c'est la raison d'être du projet) et ne jamais réintroduire de flou temps réel ou d'animation de filtre.
