<p align="center"><img src="docs/logos.png" width="360" alt="Logo di Turtlefin · Logo di Turtlefin (tema Frutiger Aero) · Logo di Turtlefin (tema Nerd)"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · **Italiano** · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**Un client Jellyfin nativo, leggero e animato, pensato per il salotto come per la scrivania.** Scritto in Rust con
Slint (interfaccia) e libmpv (riproduzione): niente Qt, nessun browser integrato. Funziona su qualsiasi computer Windows
o Linux, dal vecchio portatile al piccolo box collegato alla TV, e si usa bene sia con il telecomando sia con tastiera e
mouse — e su Android (telefono, tablet, TV), con il dito.

> **Progetto interamente «vibecodato»**: tutto il codice, i testi e le traduzioni sono stati scritti da un'IA
> (Claude, di Anthropic), guidata e testata da una persona che non ha scritto il codice.

Versione attuale: **2.0.0** · Lingue dell'interfaccia: Français, English, Español, Deutsch, Italiano, Português,
Polski, Nederlands — e qualsiasi lingua che Lei aggiunga (vedere [Tradurre Turtlefin](#tradurre-turtlefin)).

> **Su questa versione.** La 2.0.0 è stata provata su Windows, su un Raspberry Pi 5 collegato a una TV, su Linux e su
> Android (telefono, emulatore di TV). Restano cose da rivedere (vedere le note delle
> [versioni](https://github.com/Xelopteryx/Turtlefin/releases) e i problemi noti di [HANDOFF.it.md](HANDOFF.it.md)).
> I Suoi riscontri sono benvenuti nelle [segnalazioni](https://github.com/Xelopteryx/Turtlefin/issues).

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="100%" alt="Home di Turtlefin, tema Turtlefin"></p>

## Immagini

I tre temi, ognuno con il suo sfondo animato di giorno e di notte (qui su un server di prova con film liberi della
Blender Foundation):

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="32%" alt="Tema Turtlefin">&nbsp;<img src="docs/screenshots/en-home-aero.jpg" width="32%" alt="Tema Frutiger Aero">&nbsp;<img src="docs/screenshots/en-home-nerd.jpg" width="32%" alt="Tema Nerd"></p>

La scheda di un film, la schermata finale (ciò che ha appena finito e cosa guardare dopo) e «Chi sta guardando?»:

<p align="center"><img src="docs/screenshots/en-detail.jpg" width="32%" alt="Scheda di un film">&nbsp;<img src="docs/screenshots/en-end-screen.jpg" width="32%" alt="Schermata finale">&nbsp;<img src="docs/screenshots/en-who-is-watching.jpg" width="32%" alt="Chi sta guardando?"></p>

La stessa app su un telefono:

<p align="center"><img src="docs/screenshots/en-phone-home.jpg" width="24%" alt="Home su un telefono">&nbsp;<img src="docs/screenshots/en-phone-detail.jpg" width="24%" alt="Scheda su un telefono">&nbsp;<img src="docs/screenshots/en-phone-settings.jpg" width="24%" alt="Impostazioni su un telefono"></p>

## Cosa sa fare

- **Account**: schermata «Chi sta guardando?» con foto del profilo (GIF animate comprese), fino a 12 account salvati
  sul dispositivo (solo il token, mai la password), codice facoltativo per account, cambio di account senza
  riscrivere nulla. Ricerca dei server su tutte le reti del dispositivo; un indirizzo principale e uno di riserva,
  provato quando il principale non risponde.
- **Avvio**: un logo animato i cui sette punti sono veri controlli (lingua, schermo, lettore video, archiviazione,
  configurazione, rete e, al centro, il server), poi «Chi sta guardando?» — o direttamente l'account scelto con
  Impostazioni → Account → **Apri questo account all'avvio**.
- **Home**: banner, I miei media, Continua a guardare, Prossimi, Aggiunti di recente; schede Preferiti, Da guardare e
  Richieste (Seerr). Locandine con episodi rimanenti, spunta «visto» e voto; lo sfondo viene dal media selezionato.
- **Schede**: film, serie, stagione, episodio; riproduzione, preferito, visto, download, scelta audio / sottotitoli
  ricordata per tutta la serie; voto, durata, uscita e, per una serie o una stagione, gli **episodi visti** («8 / 24»
  e la percentuale); scheda Dettagli (generi, studi, persone, lingue audio con la loro bandiera); «Altri simili» e
  suggerimenti di Seerr; richiesta delle stagioni mancanti.
- **Riproduzione** (libmpv): capitoli, episodi della stagione, «Salta l'intro», episodio successivo, bitrate massimo
  (oltre, il server converte il video), volume proprio; una **schermata finale** animata: ciò che ha appena finito e tre
  suggerimenti, con il riassunto di quello scelto; posizione e «visto» inviati al server.
- **Ricerca** nella libreria e in Seerr: titoli, persone (con la loro filmografia assente dal server) e **studi o
  reti** («disney», «pixar», «netflix»: i loro film e serie da richiedere); filtri, ricerca avanzata, media a caso.
- **Librerie**: griglia con indice A–Z e barra di scorrimento, filtro digitando le prime lettere di un titolo; un menu
  su ogni scheda (clic destro, pressione lunga, OK tenuto premuto): visto / non visto, Scarica, Richiedi e, per un
  amministratore, Identifica, Aggiorna, Modifica. I contenuti già richiesti mostrano lo stato della loro richiesta.
- **Selezionare più schede** nelle griglie (pressione lunga, trascinamento con il mouse, OK tenuto premuto sul
  telecomando), poi richiederle o scaricarle tutte insieme.
- **Watch party** (SyncPlay): guardare la stessa cosa nello stesso momento su più dispositivi.
- **Android** (telefono, tablet, Android TV): la stessa app, in verticale e in orizzontale, con il dito (scorrere dal
  bordo sinistro: indietro), volume del telefono, telecomando e gamepad di Android TV.
- **Trasmetti**: mandare un video su un altro schermo — un altro Turtlefin, un'app Jellyfin dell'account o un
  Chromecast — e controllarlo dal lettore. **Telecomando**: controllare un altro Turtlefin dell'account.
- **Pagine Seerr**: un media assente dal server si apre sulla sua pagina (riassunto, informazioni, cast), con Richiedi
  e **Trailer**.
- **Temi**: Turtlefin, Frutiger Aero e Nerd, ognuno con il suo logo, il suo avvio e il suo sfondo animato che segue
  l'ora; un tema personale (vedere [Temi](#temi)). Offline, tutta l'interfaccia diventa ambra.
- **Offline**: i download sostituiscono la home, con schede complete senza server; ciò che è stato visto o messo tra i
  preferiti offline viene inviato all'account alla riconnessione.
- **Telecomando, tastiera, mouse e dito ovunque**: interfaccia TV (elementi grandi, tastiera su schermo) o interfaccia
  computer (finestra o schermo intero, F11).
- **Impostazioni** ordinate per categorie, con ricerca: foto del profilo (GetAvatar), lingue, sottotitoli,
  riproduzione, schermo, dimensione del testo, rete, archiviazione, animazioni, mod, **aggiornamento da GitHub**.
- **Animazioni** ovunque (locandina che vola verso la sua scheda, banner che si apre, file a cascata, transizioni tra
  pagine), regolabili una per una, con preset (Tutte, Leggere, Nessuno).
- **Fluidità**: Turtlefin misura ciò che il dispositivo può fare e alleggerisce da solo lo sfondo per mantenere 60
  immagini al secondo.
- **Visita guidata** e **demo** integrate («Scopri Turtlefin» al primo avvio), con veri film liberi; non viene salvato
  nulla.
- **Stato Discord** (disattivato per impostazione predefinita); **amministratori**: modificare un media dalla sua
  scheda, pannello di controllo.

## Requisiti di sistema

Misurati su un Raspberry Pi 5, un telefono di fascia bassa (Galaxy A12s) e un emulatore Android limitato (un solo
core limitato al 30 %: utilizzabile, animazioni a 15–50 immagini al secondo; lo stesso core senza limite: animazioni a
60 immagini al secondo).

**Minimi** — tutto funziona, animazioni disattivate (Impostazioni → Animazioni → **Nessuno**); lo sfondo si ferma da
solo se il dispositivo non tiene il passo:

- processore a 64 bit con 4 core intorno a 1,5 GHz (Cortex-A53 / A55 di una TV o di un telefono di fascia bassa, Intel
  Celeron o Atom recente);
- 2 GB di memoria (Turtlefin ne usa da 200 a 500 MB);
- accelerazione grafica OpenGL (Windows, Linux su PC) o OpenGL ES (Android, Raspberry Pi); decodifica video hardware
  consigliata;
- Windows 10 o 11, Linux del 2022 o più recente (Ubuntu 22.04, Debian 12…), Android 8 o più recente.

**Consigliati** — tutte le animazioni a 60 immagini al secondo:

- processore recente a 4 core intorno a 2 GHz o più (Cortex-A76 del Raspberry Pi 5, Intel Core i3 di 8ª generazione,
  AMD Ryzen 3, o meglio);
- 4 GB di memoria;
- decodifica hardware HEVC a 10 bit per il 4K.

## Installare

Niente da compilare: si scarica, si installa, fatto. Tutti i file sono nella pagina
[Releases](https://github.com/Xelopteryx/Turtlefin/releases).

### Con un solo comando

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminale):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

Su Windows, l'ultimo programma di installazione viene scaricato ed eseguito per il Suo account utente (senza diritti di
amministratore). Su Debian, Ubuntu, Linux Mint e le altre distribuzioni con apt viene installato il pacchetto `.deb`
(Le viene chiesta la password); altrove, l'AppImage va in `~/.local/bin` con una voce nel menu delle applicazioni.

### A mano

- **Windows 64 bit**: `Turtlefin-<version>-windows-x64-setup.exe` — programma di installazione: lingua, cartella e
  modalità **installata** (menu Start, disinstallazione) o **portatile**.
- **Windows 64 bit, senza installare**: `Turtlefin-<version>-windows-x64-portable.zip` — da decomprimere dove vuole
  (chiavetta USB…), poi avviare `turtlefin.exe`.
- **Windows 32 bit** (vecchi PC): `…-windows-x86-setup.exe` o `…-windows-x86-portable.zip`.
- **Linux, tutte le distribuzioni (x86_64)**: `Turtlefin-<version>-linux-x86_64.AppImage` — renderla eseguibile
  (`chmod +x`), poi avviarla. ARM 64 bit: `…-linux-aarch64.AppImage`.
- **Linux con Flatpak (x86_64)**: `Turtlefin-<version>-linux-x86_64.flatpak` —
  `flatpak install --user ./Turtlefin-….flatpak` (il runtime viene da Flathub).
- **Debian, Ubuntu, Mint…**: `turtlefin_<version>_amd64.deb` o `_arm64.deb` — doppio clic, oppure
  `sudo apt install ./turtlefin_….deb`.
- **Android** (telefono, tablet, TV): `Turtlefin-<version>-android-arm64.apk` (TV a 32 bit:
  `…-android-androidtv-armv7.apk`) — consentire l'installazione da questa origine, poi aprire il file.

Le versioni Windows e AppImage contengono tutto (lettore libmpv compreso). Il pacchetto `.deb` usa la libmpv del
sistema (`libmpv2`, installata automaticamente da apt).

**Versione portatile**: un file chiamato `portable` accanto a `turtlefin.exe` fa conservare configurazione, account,
cache e download nella cartella `data` accanto al programma; non viene scritto nulla altrove.

### Aggiornare

**Impostazioni → Informazioni → Cerca aggiornamenti** confronta la versione installata con l'ultima pubblicata, poi
**Aggiorna** si occupa di tutto secondo il modo in cui Turtlefin è installato:

- **Windows, installato**: il nuovo programma di installazione viene scaricato ed eseguito in silenzio nella stessa
  cartella;
- **Windows, portatile**: il nuovo archivio viene scaricato e i suoi file sostituiscono i vecchi;
- **AppImage**: il nuovo file sostituisce il vecchio;
- **Pacchetto .deb**: il pacchetto viene installato con `pkexec` (viene chiesta la password di amministratore);
- **Flatpak**: non automatico — scarichi il nuovo file `.flatpak` e lo installi allo stesso modo.

**Riavvia Turtlefin** avvia poi la nuova versione.

## Plugin del server consigliati

Turtlefin funziona con un server Jellyfin semplice (10.11 o più recente). Questi plugin, da installare sul **server**,
aggiungono funzioni:

- [**Jellyfin Enhanced**](https://github.com/n00bcodr/Jellyfin-Enhanced): scheda Richieste, risultati Seerr nella
  ricerca (titoli, persone, studi), suggerimenti e richieste Seerr sulle schede — tramite l'accesso Jellyfin, senza
  chiave Seerr sul client.
- [**Seerr**](https://github.com/seerr-team/seerr) (già Jellyseerr): il gestore delle richieste vero e proprio, usato
  da Jellyfin Enhanced.
- [**Intro Skipper**](https://github.com/intro-skipper/intro-skipper): individua intro e titoli di coda — pulsante
  «Salta l'intro», salto automatico, «Episodio successivo» al momento giusto.
- [**GetAvatar**](https://github.com/cedev-1/jellyfin-plugin-GetAvatar): una galleria di foto del profilo da scegliere
  in Impostazioni → Account.
- **Turtlefin Trailers** (in sviluppo, non ancora pubblicato): scarica sul server i trailer mancanti, nella lingua di
  ogni libreria e di ogni account, e prepara quelli delle pagine Seerr.

## Avviare

- `turtlefin`: animazione di avvio, poi «Chi sta guardando?» (o l'account di avvio).
- `turtlefin "Nome"`: account salvato «Nome».
- `turtlefin "Nome" --server=http://…`: accesso diretto (password nella variabile `TURTLEFIN_PASSWORD`).
- `turtlefin --tv`: interfaccia TV (schermo intero, elementi grandi); `--desktop`: interfaccia computer.
- `turtlefin --fullscreen`: schermo intero senza l'interfaccia TV; `--no-intro`: senza animazione di avvio.
- `turtlefin --sub=fr --audio=ja`: sottotitoli e lingua audio di questo avvio (`--sub=off`: nessuno).
- `turtlefin --lang=en`, `--theme=nerd`: lingua e tema (mantenuti poi).
- `turtlefin "Nome" --play="Bleach S1E3"`: riproduce un episodio («Titolo» da solo: un film, o riprende la serie);
  `--open="Dragon Ball"`: apre la scheda.
- `turtlefin --demo`: demo integrata; `--theme-creator`: Turtlefin Theme Creator; `--console`: finestra del registro
  (Windows); `--help`: tutte le opzioni.

La riga di comando prevale sempre sulle impostazioni (account di avvio, interfaccia TV).

## Tasti e mouse

- **Frecce** per spostarsi, **Invio** per aprire / attivare, **Esc** o **Backspace** per tornare indietro.
- Home: **←** sulla prima scheda (o Indietro) apre il menu; **↑** dall'alto di una pagina: barra superiore.
- Riproduzione, comandi nascosti: **← →** indietro / avanti di 10 s, **↑ ↓** o Invio mostrano i comandi; **Spazio**:
  pausa · `a` audio · `s` sottotitoli · `f` schermo intero.
- **F11**, ovunque: schermo intero (anche in Impostazioni → Schermo, mantenuto da un avvio all'altro).
- **Telecomando**: Stop, episodio successivo / precedente, avanti / indietro veloce, Menu; OK tenuto premuto su una
  scheda: il suo menu (o una selezione di più schede in una griglia).
- **Mouse**: clic per aprire, clic destro per il menu di una scheda, rotella per passare da una fila all'altra (Maiusc +
  rotella: dentro la fila).
- **Dito**: tocco per aprire, scorrimento per spostarsi, dal bordo sinistro: indietro; in una griglia, una pressione
  lunga su una scheda avvia una selezione di più schede.
- **Testo**: una lettera digitata sulla tastiera va direttamente nel campo (ricerca, accesso), anche nell'interfaccia TV.

## File

- **Cartella di configurazione** (`turtlefin/`): `session.json` (sessione attuale), `accounts.json` (account salvati),
  `prefs.json` (impostazioni del dispositivo), `tracks.json` (tracce per serie), `userdata.json` (visti / preferiti
  offline).
- **Cartella dei dati** (`turtlefin/downloads/`): download ordinati come una libreria
  (`<libreria>/<serie>/Season 01/<serie> S01E01.mkv`, con immagine e sottotitoli); `queue.json` (coda d'attesa).
- **Cartella della cache** (`turtlefin/img/`): immagini e foto del profilo (svuotabile in Informazioni).
- **Documenti**: `Turtlefin Languages` (lingue aggiunte), `Turtlefin Themes` (temi), `Turtlefin Mods` (mod) e
  `Turtlefin Presets` (preset di animazione esportati).

Su Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`. Su Windows:
`%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`. Versione portatile: tutto in
`data\` accanto a `turtlefin.exe`.

## In caso di problemi

- `turtlefin --console` (Windows) o un avvio da un terminale (Linux) mostra il registro.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: registro di mpv · `TURTLEFIN_MPV_ARGS="…"`: opzioni mpv aggiuntive.
- `TURTLEFIN_HWDEC=no`: decodifica software · `TURTLEFIN_AO=alsa`: uscita audio.
- Righe o disturbi nell'immagine (NVIDIA): Impostazioni → Riproduzione → **Decodifica hardware** disattivata.
- Dispositivo lento: Impostazioni → Animazioni → **Leggere** o **Nessuno**, e Impostazioni → Schermo → Tema →
  **Dettaglio dello sfondo** → Fermo.
- Il rendering deve essere OpenGL (scelto automaticamente): con `SLINT_BACKEND=winit-software`, niente video.

## Temi

Impostazioni → Schermo → **Tema**: **Turtlefin** (predefinito: esagoni, viola → blu, macchie di colore che fluttuano),
**Frutiger Aero** (nello spirito di Windows 7: vetro azzurrato, pulsanti lucidi, una laguna attraverso la cui acqua si
vede l'immagine del media) e **Nerd** (una scheda elettronica: il server è il processore, la corrente percorre le
piste) — o un tema personale. Ogni sfondo segue l'ora (giorno, tramonto, notte, alba), o resta sempre di giorno o di
notte.

- **Crea un tema** apre il *Turtlefin Theme Creator* (finestra a parte, anche nel menu Start): ogni colore,
  l'arrotondamento, i riflessi, lo sfondo (immagine, GIF o video), il logo e una musica, con anteprima dal vivo.
  **Salva** scrive il tema nella cartella **Turtlefin Themes**; **Installa** lo aggiunge e lo sceglie.
- **Importa un tema** aggiunge un tema ricevuto (la sua cartella o il suo file `.tftheme`); **Esporta il tema** scrive
  il tema attuale in quella cartella, come punto di partenza.

## Tradurre Turtlefin

Senza programmare né compilare. Le lingue aggiunte stanno in un'unica cartella, **Turtlefin Languages**: in
**Documenti** (versione installata) o accanto a `turtlefin.exe` (versione portatile).

1. **Impostazioni → Schermo → Aggiungi una lingua** apre un esploratore di quella cartella; **Crea il modello** vi
   scrive `modele.po`, con i testi in inglese e, come nota, il francese originale.
2. Ne faccia una copia chiamata `<codice>.po` (`sv.po` per lo svedese, `ja.po` per il giapponese…) e riempia ogni
   `msgstr ""` con la traduzione del `msgid` inglese sopra. Mantenga `{}` e `{n}`. Compili anche `X-Language-Name`
   (nome mostrato) e, se serve, `Plural-Forms`. Va bene qualsiasi editor di `.po`, per esempio
   [Poedit](https://poedit.net).
3. Di nuovo **Aggiungi una lingua**: la traduzione compare con la parte già tradotta; sceglierla la applica. I testi
   lasciati vuoti appaiono in inglese.

Per condividerla con tutti: una pull request che aggiunge il file a `lang/` (e a `BUILTIN` in `src/i18n.rs`);
`python tools/lang-check.py` controlla che non manchi nulla.

## Sviluppo

Vedere [HANDOFF.it.md](HANDOFF.it.md) (anche in [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Español](HANDOFF.es.md), [Nederlands](HANDOFF.nl.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): stato del progetto, decisioni, compilazione, pacchetti, pubblicazione, traduzioni,
problemi noti.

## Ringraziamenti

Grazie a [LainX84](https://github.com/LainX84): i suoi suggerimenti dettagliati
([issue n. 2](https://github.com/Xelopteryx/Turtlefin/issues/2)) hanno dato forma a buona parte della 2.0 — un indice
A–Z e una barra di scorrimento ben visibile per le librerie grandi, il filtro digitando le prime lettere di un titolo,
lo scorrimento elastico, le schede distribuite una sola volta per sessione (con un'impostazione per disattivare
l'animazione) e il menu delle schede (visto / non visto per ricominciare, Identifica, Aggiorna, Modifica il titolo).
Grazie per aver dedicato il tempo a scriverli!

## Licenza

Turtlefin è pubblicato con licenza [GNU GPL v3](LICENSE) (o versione successiva): chiunque può usarlo, studiarlo,
modificarlo e ridistribuirlo, a condizione di mantenere la stessa licenza e di condividere il codice delle proprie
modifiche. Le immagini mostrano film liberi della Blender Foundation (CC BY). Il carattere Turtlefin Symbols deriva da
DejaVu Sans (licenza in `ui/fonts/DejaVu-LICENSE.txt`).
