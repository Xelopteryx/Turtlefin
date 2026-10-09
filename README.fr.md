<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Logo de Turtlefin"></p>

# Turtlefin

[English](README.md) · **Français** · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**Client Jellyfin natif, léger et animé, pensé pour le salon comme pour le bureau.** Écrit en Rust avec Slint
(interface) et libmpv (lecture) : pas de Qt, pas de navigateur embarqué. Il tourne sur n'importe quel ordinateur
Windows ou Linux, du vieux portable au petit boîtier branché à la télé, et s'utilise aussi bien à la télécommande
qu'au clavier et à la souris.

Version actuelle : **1.0.0** · Langues de l'interface : Français, English, Español, Deutsch, Italiano, Português,
Polski, Nederlands — et toute langue ajoutée soi-même (voir [Traduire Turtlefin](#traduire-turtlefin)).

## Ce qu'il sait faire

- **Comptes** : écran « Qui regarde ? » avec photos de profil (GIF animés compris, gardées en cache), jusqu'à
  12 comptes enregistrés sur l'appareil (le jeton seulement, jamais le mot de passe), changement de compte sans
  ressaisie, « Gérer les comptes » pour en retirer. Recherche des serveurs sur tous les réseaux de l'appareil ;
  une adresse principale et une adresse de secours, essayée quand la principale ne répond pas.
- **Démarrage** : logo animé dont les sept points sont de vraies vérifications (langue, affichage, lecteur vidéo,
  stockage, configuration, réseau et, au centre, le serveur), puis « Qui regarde ? » — ou directement le compte
  choisi avec Paramètres → Compte → **Ouvrir ce compte au démarrage**.
- **Accueil** : Mes médias, Reprendre, À suivre, Récemment ajouté ; onglets Favoris et Demandes (Seerr).
  Affiches avec épisodes restants, coche « vu » et note ; fond d'écran tiré du média sélectionné.
- **Fiches** : film, série, saison, épisode ; lecture, favori, vu, téléchargement, choix audio / sous-titres
  retenu pour toute la série ; « Plus de ce genre » et suggestions Seerr ; demande des saisons manquantes.
- **Lecture** (libmpv) : chapitres, épisodes de la saison, « Passer l'intro », épisode suivant, suggestions en
  fin de série, volume propre à Turtlefin ; position et « vu » renvoyés au serveur.
- **Watch party** (SyncPlay) : regarder la même chose en même temps sur plusieurs appareils.
- **Hors ligne** : les téléchargements remplacent l'accueil, avec des fiches complètes sans serveur ; ce qui a
  été vu ou mis en favori hors ligne est renvoyé au compte à la reconnexion.
- **Recherche** (bibliothèque + Seerr) et média au hasard.
- **Télécommande, clavier et souris partout** : interface TV (grands éléments, clavier à l'écran) ou interface
  ordinateur (fenêtre ou plein écran, F11), clic et molette sur toutes les pages, et le vrai clavier écrit
  directement dans les champs, dans les deux modes.
- **Paramètres** : photo de profil (avatars GetAvatar par catégorie), langue de l'interface, langues audio et
  sous-titres, taille des sous-titres, épisode suivant et intro automatiques, interface TV, plein écran, fond
  d'écran, notes, heure, adresses du serveur, cache d'images, **mise à jour depuis GitHub**.
- **Animations** partout (affiche qui vole vers la fiche, menu qui glisse, rangées en cascade, connexion animée,
  changement de langue), réglables une par une dans Paramètres → Animations, avec des presets (Toutes, Légères,
  Aucune) et les siens, exportables et importables.
- **Visite guidée** : proposée au premier lancement, à revoir dans Paramètres → À propos.

## Installer

Rien à compiler : on télécharge, on installe, c'est fini. Tous les fichiers sont sur la page
[Releases](https://github.com/Xelopteryx/Turtlefin/releases).

### En une commande

**Windows** (PowerShell) :

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminal) :

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

Sous Windows, le dernier installeur est téléchargé et lancé pour ton compte utilisateur (sans droits
administrateur). Sous Debian, Ubuntu, Linux Mint et les autres distributions à apt, c'est le paquet `.deb` qui est
installé (ton mot de passe est demandé) ; ailleurs, l'AppImage va dans `~/.local/bin` avec une entrée dans le menu
des applications.

### À la main

| Système | Fichier | Remarques |
|---|---|---|
| Windows 64 bits | `Turtlefin-<version>-windows-x64-setup.exe` | Installeur : langue, dossier, et mode **installé** (menu Démarrer, désinstallation) ou **portable** |
| Windows 64 bits, sans installer | `Turtlefin-<version>-windows-x64-portable.zip` | Décompresser où l'on veut (clé USB…) et lancer `turtlefin.exe` |
| Windows 32 bits | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Pour les vieux PC |
| Linux, toutes distributions (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Le rendre exécutable (`chmod +x`), puis le lancer |
| Linux, ARM 64 bits | `Turtlefin-<version>-linux-aarch64.AppImage` | Idem |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Double-clic, ou `sudo apt install ./turtlefin_….deb` |

Les versions Windows et AppImage contiennent tout (lecteur libmpv compris). Le paquet `.deb` utilise la libmpv
du système (`libmpv2`, installée automatiquement par apt). Les AppImage et paquets demandent une distribution de
2022 ou plus récente (Ubuntu 22.04, Debian 12…).

**Version portable** : un fichier `portable` à côté de `turtlefin.exe` fait garder la configuration, les comptes,
le cache et les téléchargements dans le dossier `data` à côté du programme ; rien n'est écrit ailleurs.

### Mettre à jour

**Paramètres → À propos → Rechercher une mise à jour** compare la version installée à la dernière publiée,
puis **Mettre à jour** s'occupe de tout selon la façon dont Turtlefin est installé :

| Installation | Mise à jour |
|---|---|
| Windows, installé | le nouvel installeur est téléchargé puis relancé en silence dans le même dossier |
| Windows, portable | la nouvelle archive est téléchargée et ses fichiers remplacent les anciens |
| AppImage | le nouveau fichier remplace l'ancien |
| Paquet .deb | le paquet est installé avec `pkexec` (le mot de passe administrateur est demandé) |

**Redémarrer Turtlefin** lance ensuite la nouvelle version.

## Plugins serveur recommandés

Turtlefin fonctionne avec un serveur Jellyfin simple (10.11 ou plus récent). Ces plugins, à installer sur le
**serveur**, ajoutent des fonctions :

| Plugin | Ce qu'il apporte à Turtlefin |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Onglet Demandes, résultats Seerr dans la recherche, suggestions et demandes Seerr sur les fiches — avec la connexion Jellyfin, sans clé Seerr côté client |
| [Seerr](https://github.com/seerr-team/seerr) (anciennement Jellyseerr) | Le gestionnaire de demandes lui-même, utilisé par Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Repère intros et génériques : bouton « Passer l'intro », saut automatique, « Épisode suivant » au bon moment |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | Une galerie de photos de profil à choisir dans Paramètres → Compte |

## Lancer

```
turtlefin                                  animation de démarrage, puis « Qui regarde ? » (ou le compte de démarrage)
turtlefin "Nom"                            compte enregistré « Nom »
turtlefin "Nom" --server=http://…          connexion directe (mot de passe : variable TURTLEFIN_PASSWORD=…)
turtlefin --tv                             interface TV : plein écran, grands éléments
turtlefin --desktop                        interface ordinateur (prime sur le réglage « Interface TV »)
turtlefin --no-intro                       pas d'animation de démarrage
turtlefin --console                        fenêtre de journal (Windows)
turtlefin --tutorial                       visite guidée à l'arrivée sur l'accueil
```

La ligne de commande l'emporte toujours sur les réglages (compte de démarrage, interface TV).

## Touches et souris

- **Flèches** pour se déplacer, **Entrée** pour ouvrir / activer, **Échap** ou **Retour arrière** pour revenir
  (touche maintenue : un seul retour).
- Accueil : **←** sur la première carte (ou Retour) ouvre le menu ; dans le menu, **→** ou Échap le referme.
- **↑** depuis le haut d'une page : barre du haut (retour, accueil, menu, watch party, hasard, recherche, compte).
- Lecture, commandes masquées : **← →** reculer / avancer de 10 s, **↑ ↓** ou Entrée affichent les commandes,
  ↓ depuis les boutons : épisodes de la saison. **Espace** : pause · `a` audio · `s` sous-titres · `f` plein écran.
- **F11**, partout : plein écran (aussi dans Paramètres → Affichage → **Plein écran**, gardé d'un lancement à
  l'autre hors interface TV).
- **Souris** : clic pour ouvrir, molette pour passer d'une rangée à l'autre (Maj + molette : dans la rangée).
- **Texte** : une lettre tapée au clavier va directement dans le champ (recherche, connexion, adresse), même en
  interface TV ; un clic sur le champ de recherche en interface TV ouvre le clavier à l'écran.

## Fichiers

| Où | Quoi |
|---|---|
| dossier de config, `turtlefin/` | `session.json` (session en cours), `accounts.json` (comptes enregistrés), `prefs.json` (réglages de l'appareil), `tracks.json` (pistes par série), `userdata.json` (vu / favoris faits hors ligne) |
| dossier de données, `turtlefin/downloads/` | téléchargements (média, affiches, fond, logo, `info.json`), `queue.json` (file en attente) |
| dossier de cache, `turtlefin/img/` | images et photos de profil (vidable dans À propos) |
| **Documents** | `Turtlefin Languages` (langues ajoutées) et `Turtlefin Presets` (presets d'animations exportés) |

Sous Linux : `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Sous Windows : `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Version portable : tout dans `data\` à côté de `turtlefin.exe` (`config`, `cache`, `downloads`), et les dossiers
de langues et de presets à côté du programme.

## En cas de problème

- `turtlefin --console` (Windows) ou un lancement depuis un terminal (Linux) pour voir le journal.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log` : journal de mpv · `TURTLEFIN_MPV_ARGS="…"` : options mpv en plus.
- `TURTLEFIN_HWDEC=auto-copy` : décodage matériel (logiciel par défaut sous Linux ARM) · `TURTLEFIN_AO=alsa` : sortie son.
- `TURTLEFIN_LIBMPV=chemin` : autre emplacement de libmpv · `TURTLEFIN_DEBUG_FRAMES=1` : signale les images lentes.
- Le rendu doit être OpenGL (choisi automatiquement) : avec `SLINT_BACKEND=winit-software`, pas de vidéo.

## Thèmes

Paramètres → Affichage → **Thème** : Turtlefin (par défaut), Sombre, Clair, Frutiger Aero (dans l'esprit de
Windows 7 : fond « Harmony », verre bleuté, boutons en gel brillant, lecteur à orbe, démarrage en billes de verre), Turtlefin vert — ou un thème à soi.

- **Créer un thème** ouvre le *Turtlefin Theme Creator* dans le navigateur : chaque couleur, l'arrondi, le reflet
  brillant, le ciel et les bulles, avec un aperçu en direct ; il enregistre un fichier `.tftheme`.
- Mettre ce fichier dans le dossier **Turtlefin Themes** (Documents, ou à côté de `turtlefin.exe` en version
  portable), puis **Importer un thème**. **Exporter le thème** écrit le thème actuel dans ce dossier, comme point de
  départ.
- Le créateur est aussi dans le dépôt, `tools/theme-creator.html` : un seul fichier, qui s'ouvre hors ligne.

## Traduire Turtlefin

Sans programmer ni compiler. Les langues ajoutées vivent dans un seul dossier, **Turtlefin Languages** :
dans **Documents** (version installée) ou à côté de `turtlefin.exe` (version portable).

1. **Paramètres → Affichage → Ajouter une langue** ouvre l'explorateur de ce dossier ; **Créer le modèle**
   y écrit `modele.po`, dont les textes sont en anglais, avec en note le français d'origine et la langue en cours.
2. En faire une copie nommée `<code>.po` (`sv.po` pour le suédois, `ja.po` pour le japonais…), dans ce dossier
   ou un de ses sous-dossiers, et remplir chaque `msgstr ""` avec la traduction du `msgid` anglais au-dessus.
   Garder les `{}` et `{n}`. Remplir aussi `X-Language-Name` (nom affiché) et, si besoin, `Plural-Forms`
   (règle gettext de la langue). N'importe quel éditeur de `.po` convient, par exemple [Poedit](https://poedit.net).
3. De nouveau **Ajouter une langue** : la traduction apparaît avec la part déjà traduite ; la choisir
   l'applique. Les textes laissés vides s'affichent en anglais.

Pour la partager avec tout le monde : une pull request qui ajoute le fichier à `lang/` (et à `BUILTIN` dans
`src/i18n.rs`) ; `python tools/lang-check.py` vérifie qu'il ne manque rien.

## Développement

Voir [HANDOFF.md](HANDOFF.md) (aussi en [English](HANDOFF.en.md), [Deutsch](HANDOFF.de.md),
[Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)) : état du projet, décisions, compilation, paquets, publication, traductions,
problèmes connus.
