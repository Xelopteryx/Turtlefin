# Turtlefin: passaggio di consegne (stato al 9 ottobre 2026, versione 1.0.0)

[Français](HANDOFF.md) · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · **Italiano** · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

Per chi riprende lo sviluppo (una persona o Claude Code). Leggilo tutto prima di toccare il codice, poi leggi
[README.it.md](README.it.md) (uso, installazione, tasti, file).
Repository: https://github.com/Xelopteryx/Turtlefin · Versione in `Cargo.toml`: 1.0.0.

## 1. Obiettivo

Un **client Jellyfin nativo in Rust**, leggero, animato, utilizzabile con il telecomando come con tastiera e
mouse, installabile su qualsiasi computer Windows o Linux **senza compilare nulla**: chi lo usa scarica un
programma di installazione o un pacchetto (oppure lancia un comando di installazione), e basta. Tutti i pacchetti
li produce la CI di GitHub (o il PC di chi mantiene il progetto), mai l'utente.

Perché: Jellyfin Desktop (Qt / QtWebEngine) perde memoria e finisce per bloccarsi sulle macchine piccole, e
l'interfaccia web con un tema pesante scende sotto i 30 fotogrammi/s su hardware modesto. Regola permanente:
restare stabili in memoria e non reintrodurre mai sfocature in tempo reale né animazioni di filtri.

## 2. Decisioni prese

| Argomento | Decisione | Motivo |
|---|---|---|
| Linguaggio / UI | Rust + **Slint** `~1.18` (rendering 100 % Slint), stile `fluent-dark` imposto da `build.rs` | Niente browser; lo stile «native» dipenderebbe da Qt |
| Funzione di Slint `unstable-winit-030` | Filtro degli eventi winit per **F11** (`install_f11`) | Unico modo per avere un tasto globale; da qui `~1.18` (API instabile tra versioni minori) |
| Rete | `reqwest` 0.13 (rustls, archivio dei certificati del sistema), `tokio` | `query` è una feature da attivare in 0.13 |
| Riproduzione | **libmpv caricata a runtime** (`libloading`, `src/mpv.rs`), rendering OpenGL in una texture mostrata da Slint (`src/video.rs`), comandi Slint sopra (`ui/player.slint`) | Lettore integrato, niente IPC, funziona sotto Wayland. Il lettore viene ricreato a ogni riproduzione (memoria limitata) |
| Renderer di Slint | femtovg (OpenGL / GLES) imposto, salvo se `SLINT_BACKEND` è definito | Il video passa da una texture OpenGL |
| Texture video | Pixel fisici, origine `TopLeft`, stato GL salvato / ripristinato attorno a mpv; `loadfile` aspetta il contesto di rendering | Altrimenti immagine capovolta o «No render context set» |
| Decodifica | `hwdec=no` su Linux ARM a 64 bit, `auto-safe` altrove (Windows: `d3d11va-copy`) | Sulle schede ARM provate (driver v3d), la decodifica V4L2 produce un formato che il renderer non sa importare |
| Audio su Linux | `ao=pipewire,pulse,alsa`, `config=no` | Un `mpv.conf` utente che imponeva ALSA falliva quando PipeWire tiene l'uscita HDMI |
| Memoria | Cache di mpv limitata (100 / 25 MiB), immagini chieste alla giusta dimensione, 16 elementi per riga | Macchine piccole (4 GB) |
| Lingue | Francese nel codice (lingua sorgente), traduzione a runtime da `src/i18n.rs` a partire da `lang/<code>.po` (integrati) e dalla cartella `Turtlefin Languages` | Vedi la sezione 6 |
| Schermo intero (Windows) | Finestra senza bordi che copre lo schermo + 1 px (`src/winfull.rs`, `set_tv_window`), non il vero schermo intero; segue i cambi di risoluzione (ogni 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` per lo schermo intero di Slint | In schermo intero OpenGL, AMD Software scambia l'app per un gioco: «Premi ALT + R» a ogni ritorno in primo piano |
| Interfaccia TV / computer | `tv-mode` cambia solo la dimensione (`k` = 1,4) e l'inserimento del testo (tastiera a schermo); lo schermo intero è a parte (`full_flag`, `UiPrefs::fullscreen`, F11) | Richiesta dell'utente: schermo intero senza ingrandire l'interfaccia |
| Finestra (Windows, desktop) | Ridotta e centrata se 1280 x 720 + cornice supera l'area di lavoro; resta animata durante lo spostamento (`winfull::keep_alive_while_moving`) | Schermi 1366 x 768; Windows blocca il ciclo degli eventi durante lo spostamento di una finestra |
| Finestra di console (Windows) | Sottosistema «windows» in release; `--console` si aggancia a una console o ne apre una; comandi esterni senza finestra (`paths::quiet_command`) | Niente console né finestre CMD che lampeggiano |
| Riga di comando | Prevale sempre sulle impostazioni (account di avvio, interfaccia TV) | Richiesta dell'utente |
| Password | Mai salvata (solo il token); da riga di comando, meglio `TURTLEFIN_PASSWORD` | Un argomento è visibile agli altri processi |

## 3. Struttura del codice

```
build.rs            commit compilato, stile Slint, icona dell'exe (winresource, Windows)
lang/<code>.po      traduzioni integrate (sorgente: il francese del codice); tools/lang-check.py le verifica
ui/theme.slint      token del tema, global Tr (traduzione) e Motion (animazioni attive)
ui/app.slint        AppWindow e tutte le schermate (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo: animazione di avvio (7 punti, unione, zoom), scelta della lingua
ui/player.slint     schermata di riproduzione   ui/osk.slint      tastiera a schermo
ui/card.slint       scheda locandina            ui/marquee.slint  testo che scorre
ui/typed.slint      testo digitato animato (Str, TypedText)   ui/langx.slint  esploratore (lingue, preset)
ui/dust.slint       barre del cambio di lingua                ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, stato condiviso App (Arc), schermate, navigazione (pila + pagine conservate), impostazioni
src/boot.rs         sequenza di avvio (controlli, lingua, scelta della schermata di arrivo)
src/i18n.rs         lingua corrente, tr() / trf() / trn(), lingue aggiunte, modello di traduzione
src/dust.rs         effetto del cambio di lingua (individuazione delle righe di testo, morphing)
src/api.rs          client REST di Jellyfin (+ relay Seerr di Jellyfin Enhanced, GetAvatar)
src/config.rs       sessione, account (12 al massimo), prefs.json (UiPrefs, AnimFlags, preset), tracce, visti/preferiti offline
src/discovery.rs    ricerca dei server (UDP, sottoreti, ARP, peer VPN)
src/downloads.rs    download (ripresa Range, coda, sincronizzazione offline)
src/mpv.rs          collegamento a libmpv; src/video.rs texture OpenGL; src/player.rs riproduzione, rapporti, concatenazione
src/syncplay.rs     watch party (WebSocket /socket)
src/paths.rs        cartelle di configurazione / cache / dati; modalità portatile; quiet_command
src/update.rs       aggiornamento secondo l'installazione (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows: schermo / area di lavoro, finestra animata durante lo spostamento
src/theme.rs        temi integrati, file .tftheme (ThemeDef), applicazione al global Theme
ui/sky.slint        scenari (cielo e bolle · «Harmony» di Windows 7), statici, in cache
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   compilazione e pubblicazione con un'etichetta `v*` (prova: ramo `ci`)
```

Principi:
- I dati di rete passano per strutture `Send`, poi `upgrade_in_event_loop` li porta nei modelli Slint. Immagini
  decodificate fuori dal thread della UI, 6 download in parallelo, applicate con un controllo sull'id.
- `App.gen` invalida i caricamenti superati; `App.stack` è la pila di navigazione; `PAGES` conserva schede e
  librerie per tornare indietro senza richieste.
- Navigazione da tastiera fatta a mano (indici di selezione in Rust e in Slint), perché Slint non gestisce il focus
  di schede dinamiche. Ogni schermata ha il suo `FocusScope`; `refocus` restituisce la tastiera al posto giusto.
- I `changed` di Slint sono differiti: non contare sul loro ordine (posizioni in due tempi, ecc.).
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`, intestazione
  `Authorization: MediaBrowser …, Token=…`. Rapporti: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` e il WebSocket rifiutano `api_key`: token nell'intestazione.

## 4. Avvio

`main` applica la lingua (prefs.json, altrimenti il file `language` scritto dal programma di installazione di
Windows), poi lancia `boot::run`. La schermata `boot` mostra 7 punti: i 6 vertici dell'esagono, poi il centro.
Ognuno è un controllo vero (`boot::check`):
1. **lingua** (chiesta se sconosciuta): si caricano le traduzioni della lingua scelta (`i18n::check`);
2. **schermo**: la finestra ha ottenuto un contesto OpenGL (`video::gl_info`, versione e scheda grafica nel registro);
3. **lettore video**: un vero lettore mpv viene creato, inizializzato e poi distrutto (`mpv::self_test`);
4. **archiviazione**: un file viene scritto, riletto e cancellato nelle cartelle di configurazione, dati e cache;
5. **configurazione**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` leggibili
   (`config::unreadable_files`, chiamato all'inizio di `main`, prima che un file rovinato venga riscritto);
6. **rete**: interfaccia attiva o percorso verso l'esterno (`discovery::has_network`);
7. **server** (il centro): l'indirizzo principale, altrimenti quello di riserva, risponde a `/System/Info/Public`
   **ed** è lo stesso server (identificativo confrontato con il `server_id` della sessione); arancione finché non
   c'è un server configurato.

Rosso = errore, con un messaggio e «Continua» (da solo dopo 12 s). Tutto verde: i vertici si uniscono, i raggi
vanno verso il centro e il punto del server diventa l'esagono pieno — esattamente il logo
(`packaging/turtlefin.svg`, stessa geometria) —, poi uno zoom nel centro. Slint riduce il disegno di un `Path`
dello spessore del suo tratto: i tracciati del logo sono ingranditi altrettanto per cadere sui punti.

Schermata di arrivo (`boot::route`), in quest'ordine: nome + password da riga di comando → accesso; nome di un
account salvato → quell'account (nome sconosciuto: il suo modulo di accesso); account di avvio
(`prefs.autostart_user` / `autostart_server`) → `fly_autostart` (l'immagine dell'account al centro durante
l'accesso); altrimenti «Chi sta guardando?» (o la ricerca del server se non se ne conosce nessuno). Un account di
avvio scomparso porta a «Chi sta guardando?». `--no-intro` (o l'animazione «Avvio» disattivata) salta
l'animazione; i controlli vengono fatti comunque.

## 5. Stato alla versione 1.0.0

Tutte le funzioni del README sono fatte e verificate su screenshot (PC Windows, schermi 1366 x 768 e 1920 x 1080)
e su una macchina Linux ARM collegata a una TV, con gli account di prova `test` / `test2` di un server reale.
Versioni pubblicate dalla CI: 0.9.0, 0.9.1; la 1.0.0 è pronta per l'etichetta (sezione 7).

Meccanismi non evidenti nel codice:
- **Immagine condivisa** (`global Hero`): l'immagine di una scheda vola verso la locandina della pagina di
  dettaglio e torna sulla scheda esatta al ritorno (`Hero.want-id`, `hero-card-ok`).
- **Righe** (`global Rows`): scorrimento proprio di ogni riga, ricordato per chiave; cambiando riga si arriva alla
  scheda più vicina sullo schermo (`row-to`, `detail-pick-down`).
- **Offline**: `config::Flags` (userdata.json) conserva visti / preferiti / posizioni con un indicatore «da
  inviare»; `downloads::sync` li invia al ritorno del server (il dispositivo ha l'ultima parola).
- **Indirizzi**: `server_main` / `server_backup`; `watch_addresses` (20 s) passa a quello di riserva e torna.
- **Watch party**: una connessione WebSocket per sessione (`sp_conn`), si ferma su 401 / 403, attesa crescente.
- **Immagini del profilo**: cache su disco `avatar_<id>_still|anim.bin` + miniatura rotonda
  `avatar_<id>_thumb.png`. La miniatura viene mostrata subito, la GIF completa è decodificata fuori dal thread
  della UI (`avatar_cached_async`) e rinnovata dal server solo se è cambiata (`avatar_fetch`). Animazione delle
  GIF: `AnimSlot` (Login, Picker, Header, Fly) e un timer comune; l'avatar dell'intestazione riceve tutti i suoi
  fotogrammi una volta (`avatar-frames`) e cambia solo il fotogramma visibile. GetAvatar `SetAvatar` risponde 500
  → ripiego `POST /UserImage`.
- **Accesso animato** (`fly-phase` da 1 a 4): l'immagine va al centro, barra di caricamento, vola via, poi arriva
  la home e l'avatar dell'intestazione compare (`me-pop`).
- **Cambio di lingua** (`dust.rs`, `ui/dust.slint`): all'apertura dell'elenco delle lingue, delle barre coprono
  ogni riga di testo, prendono la larghezza delle nuove parole alla scelta, poi le svelano. Le righe vengono
  individuate confrontando un'istantanea della pagina (`take_snapshot`) con un'istantanea nel carattere
  `Turtlefin Blank` (lettere vuote, stesse larghezze, `tools/make-blank-font.py`). L'orologio (Montserrat) è escluso.
- **Visita guidata** (`tour-step` da 0 a 10): ogni passo rimette l'interfaccia nello stato atteso o si convalida
  se il gesto è già fatto; `tour-ev` viene chiamato dai `changed`.
- **Animazioni**: global `Motion` (12 interruttori: boot, pages, menu, select, scroll, panels, detail, player,
  search, login, language, tour), `config::AnimFlags`, preset integrati (Tutte, Leggere, Nessuno) e personali,
  esportati / importati in `.json` in `Turtlefin Presets`. Ogni durata di animazione si scrive
  `duration: Motion.x ? 300ms : 0ms`.
- **Inserimento del testo**: sul desktop i campi sono `TextInput` (testo nascosto, disegnato da `TypedText`); in
  modalità TV, la tastiera a schermo (`Osk`). In entrambe le modalità un tasto stampabile ricevuto dalla pagina va
  nel campo (`typing-key`, `erase-key`, `Field.type`); in modalità TV, Backspace cancella solo finché resta testo.
  Sul desktop, un clic accanto a un campo non gli toglie la tastiera (`focus-on-click: root.tv-mode`).
- **Tasti tenuti premuti**: Invio, Esc e Backspace non ripetono la loro azione (`event.repeat`), tranne Backspace
  per cancellare testo.
- **Temi** (`src/theme.rs`, global `Theme` di ui/theme.slint): nessun colore fisso nell'interfaccia; le superfici
  traslucide si scrivono `Theme.fg.with-alpha(…)` (bianco su un tema scuro, inchiostro su uno chiaro), il testo sul
  gradiente d'accento usa `Theme.on-accent`. `Gloss` (riflesso in gel) e `AeroSky` (ui/sky.slint) compaiono solo
  con `Theme.gloss` / `Theme.bubbles`. I temi importati restano in prefs.json (`theme`, `themes`); il creatore
  (`tools/theme-creator.html`) è integrato nel programma (`theme::CREATOR`) e «Crea un tema» lo mette in
  «Turtlefin Themes». I suoi temi di partenza devono restare identici a quelli di theme.rs.
- **Mouse**: clic ovunque; rotellina su home, pagina di dettaglio (`d-nav`, condiviso con la tastiera), ricerca,
  librerie, download, impostazioni; le finestre in primo piano assorbono la rotellina.

Non fatto: Quick Connect; gamepad; licenza (da scegliere da chi mantiene il progetto, prima o dopo la 1.0.0);
collegamento con XeLauncher (launcher del media center di chi mantiene il progetto, non prioritario). Previsto in
seguito: ottimizzazione, versione per Android TV.

## 6. Traduzioni

- Fatte a runtime da `src/i18n.rs`: uno stesso catalogo per Rust e per l'interfaccia.
  Slint: global `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)`; `Tr.k` cambia a ogni cambio di lingua, il che fa ricalcolare i
  testi. Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- Il testo francese **è** la chiave: modificarlo richiede di modificare il `msgid` in ogni `lang/*.po`
  (`tools/lang-check.py` segnala testi mancanti, in più e `{}` persi). Le 8 lingue integrate (fr, en, es, de, it,
  pt, pl, nl; `BUILTIN` in i18n.rs) sono complete (~530 testi).
- Lingue aggiunte: qualsiasi `<code>.po` della cartella `Turtlefin Languages` (`i18n::lang_dir`: Documenti, accanto
  all'exe nella versione portatile, dentro `TURTLEFIN_CONFIG_DIR` durante le prove; la vecchia cartella
  `languages` viene spostata lì), sottocartelle comprese; nome letto da `X-Language-Name`; un file può sostituire
  una lingua integrata. Sfogliate con un esploratore integrato (`ui/langx.slint`, funzioni `lx_*` di main.rs).
- «Crea il modello» scrive `modele.po`: msgid in **inglese**, intestazione `X-Source-Language: en`, note `#.` in
  francese e nella lingua corrente; `keyed` riporta quei msgid al francese tramite `lang/en.po`.
- Testi assenti in una lingua: inglese. Plurali: regola `Plural-Forms` del file, valutata da i18n.rs.
- Programma di installazione: `[Languages]` e `[CustomMessages]` di `turtlefin.iss`; scrive il codice scelto in
  `language` accanto all'exe, ripreso al primo avvio.
- I nomi che arrivano dal server (librerie, media) non vengono tradotti.

## 7. Compilare, produrre i pacchetti, pubblicare

Sviluppo:
- Windows: Rust (https://rustup.rs), «Build Tools di Visual Studio» (C++), git; `cargo build --release`;
  `libmpv-2.dll` (archivio `mpv-dev-x86_64-….7z` di
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) accanto all'exe.
  La versione debug ha bisogno di `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev`, poi `cargo build --release`.
- `cargo test --release`: test di i18n, update, ecc.

Pubblicare una versione:
1. Mettere il numero in `Cargo.toml` (`version = "x.y.z"`), compilare una volta (aggiorna `Cargo.lock`), fare il
   commit.
2. `git push origin main`, poi `git tag -a vx.y.z -m "Novità, una per riga"` e `git push origin vx.y.z`.
   Il messaggio dell'etichetta diventa le note di versione, mostrate dall'aggiornamento integrato.
3. `release.yml` compila Windows x64 / x86 e Linux x86_64 / aarch64, produce programmi di installazione, archivi,
   AppImage e `.deb`, e li pubblica in una Release (da seguire nella scheda Actions del repository). I nomi dei
   file (intestazione di `release.yml`) sono attesi così come sono da `update.rs` e dagli script di installazione.
4. Prova senza pubblicare: `git push origin main:ci` (si compila tutto, non si pubblica nulla).

A mano:
- **Windows**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (o `x86`);
  servono Inno Setup 6, 7-Zip e NASM (x86). Risultato in `target\dist`.
- **Linux** (su una macchina Linux): `TURTLEFIN_DIST=release cargo build --release`, poi
  `sh packaging/linux/build-appimage.sh <version>` e `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` in compilazione, altrimenti `update::kind()` crede che sia una versione compilata in
  locale.
- x86: le libmpv a 32 bit di shinchiro pubblicate da luglio 2026 si bloccano all'avvio (OpenSSL); quella del
  10 giugno 2026 è conservata nella pre-release `libmpv-i686-20260610` di Turtlefin (non cancellarla), usata da
  `build.ps1` (`MPV_TAG`: un'altra versione di shinchiro). aws-lc richiede NASM a 32 bit.
- Icone: `packaging/turtlefin.svg` è il logo; `packaging/icons/make-icons.py <cartella>` (Python + Pillow)
  rigenera i PNG e l'ICO.

Aggiornamento di una copia compilata in locale (`Kind::Source`): i file modificati o non tracciati vengono messi
da parte (`git stash -u`, recuperabili con `git stash pop`) prima di `git pull --ff-only`. Le novità compaiono in una
finestra a sé (Impostazioni → Informazioni → Vedi le novità); `TURTLEFIN_TEST_UPDATE="Versione x|nota|nota"` simula
un aggiornamento per provarla. Collegamenti con i colori del tema: `theme::apply_shortcuts` (.ico sui .lnk su
Windows, icone `turtlefin` in ~/.local/share/icons su Linux), saltati durante le prove (`TURTLEFIN_CONFIG_DIR`)
tranne con `TURTLEFIN_TEST_SHORTCUTS=1`.

Rami: `main` (unico ramo di lavoro), `ci` (prove della CI). `interface-lua` e `libmpv` sono vecchi esperimenti,
già uniti in `main`: si possono cancellare. Etichette: `v0.9.0`, `v0.9.1` (versioni pubblicate) e
`libmpv-i686-20260610` (libmpv a 32 bit, vedi sopra).

## 8. Prove

- `TURTLEFIN_CONFIG_DIR=<cartella>`: un'altra cartella di configurazione (account, prefs, lingue) senza toccare
  quella vera.
- `--open=ID|settings|downloads`, `--play=ID@SECONDI`, `--test-video=file` (lettore senza server).
- `TURTLEFIN_DEBUG_FRAMES=1` (fotogrammi > 25 ms); `TURTLEFIN_DEBUG_GAPS=1` con
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed`: pause di oltre 40 ms tra due fotogrammi.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: messaggi della watch party. Due istanze sullo stesso PC: due
  `TURTLEFIN_CONFIG_DIR` diversi, `--desktop`.
- Un `prefs.json` scritto da PowerShell 5 ha un BOM: la lettura dei file di configurazione lo ignora.
- Prove automatizzate su Windows: `SetForegroundWindow` viene accettato solo dopo un tasto simulato; usare F24, non
  Alt (Alt da solo mette la finestra in modalità menu e il clic successivo va perso). Un tasto tenuto premuto si
  simula con più chiamate consecutive a `keybd_event` di «tasto premuto» (Windows le segna come ripetizioni).

## 9. Problemi noti / limiti

1. Un crash di mpv fa chiudere Turtlefin (stesso processo).
2. La riproduzione richiede il rendering OpenGL (`SLINT_BACKEND=winit-software` la impedisce).
3. **mpv 0.40 / 0.41** (corretto in mpv il 23 gennaio 2026, commit f74adc4): una barriera OpenGL per fotogramma mai
   liberata; con il driver v3d ognuna occupa un descrittore («MESA: error: Export failed» dopo ~42 s).
   Aggiramento in `src/mpv.rs` (solo OpenGL ES). Diagnosi: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` deve
   restare stabile durante la riproduzione.
4. Crescita della RAM di mpv (~3 MB/min) con sottotitoli ASS: limitata a una riproduzione (mpv ricreato a ogni
   video).
5. Token in chiaro in `session.json` / `accounts.json` (0600 su Unix).
6. Tearing sotto Xorg senza compositor (solo Openbox): usare un compositor (picom `--backend egl --vsync`).
   Turtlefin regge 60 fotogrammi/s.
7. Decodifica software su Linux ARM: può faticare in 4K / HEVC; pista: `TURTLEFIN_HWDEC=auto-copy`.
8. Micropause (~40 ms) misurate solo con il rendering forzato alla massima velocità, a ogni fotogramma di una GIF
   dell'intestazione e all'unione del logo; causa non trovata, invisibili nell'uso normale.

## 10. Preferenze di lavoro di chi mantiene il progetto

- Risposte in francese; poca esperienza con Linux / SSH: spiegare i comandi.
- Non inviare nulla a GitHub né pubblicare versioni senza la sua approvazione esplicita; le versioni le pubblica
  chi mantiene il progetto.
- Prove con una copia della configurazione (`TURTLEFIN_CONFIG_DIR`), mai quella vera; account di prova `test` /
  `test2`.
- Dopo ogni gruppo di modifiche: un programma di installazione di prova sul suo Desktop
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, cancellando il precedente).
