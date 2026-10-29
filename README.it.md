<p align="center"><img src="packaging/icons/turtlefin-aero-256.png" width="96" alt="Turtlefin logo (Frutiger Aero theme)">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin logo">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-nerd-256.png" width="96" alt="Turtlefin logo (Nerd theme)"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · **Italiano** · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**Un client Jellyfin nativo, leggero e animato, pensato sia per il salotto sia per la scrivania.** Scritto in Rust
con Slint (interfaccia) e libmpv (riproduzione): niente Qt, niente browser integrato. Funziona su qualsiasi
computer Windows o Linux, dal vecchio portatile al piccolo box collegato alla TV, e si usa altrettanto bene con il
telecomando come con tastiera e mouse — e su Android (telefono, tablet, TV), col dito.

> **Progetto interamente «vibecodato»**: tutto il codice, i testi e le traduzioni sono stati scritti da un'IA
> (Claude, di Anthropic), guidata e provata da un umano che non l'ha programmato di persona.

Versione attuale: **1.9.7 «Ritrovo»** · Lingue dell'interfaccia: Italiano, English, Français, Español, Deutsch, Português,
Polski, Nederlands — e qualsiasi lingua aggiunta da te (vedi [Tradurre Turtlefin](#tradurre-turtlefin)).

> **Su questa versione.** La 1.9.7 «Ritrovo» è stata provata su Windows, Linux (Raspberry Pi 5 collegato a
> un televisore) e Android (telefono). Ci sono ancora cose da rivedere — lo sviluppatore sa che restano dei
> problemi (vedi le note delle [versioni](https://github.com/Xelopteryx/Turtlefin/releases) e i problemi
> noti in [HANDOFF.it.md](HANDOFF.it.md)) —, ma bisognava pur pubblicare una versione molto più avanzata della
> 1.2.0. I riscontri sono benvenuti nelle [segnalazioni](https://github.com/Xelopteryx/Turtlefin/issues).

## Cosa sa fare

- **Account**: schermata «Chi sta guardando?» con immagini del profilo (GIF animate comprese, in cache), fino a
  12 account salvati sul dispositivo (solo il token, mai la password), cambio di account senza riscrivere nulla,
  «Gestisci account» per rimuoverne. Ricerca dei server su tutte le reti del dispositivo; un indirizzo principale
  e un indirizzo di riserva, provato quando il principale non risponde.
- **Avvio**: un logo animato i cui sette punti sono veri controlli (lingua, schermo, lettore video, archiviazione,
  configurazione, rete e, al centro, il server), poi «Chi sta guardando?» — oppure direttamente l'account scelto
  in Impostazioni → Account → **Apri questo account all'avvio**.
- **Home**: I miei media, Continua a guardare, Prossimi, Aggiunti di recente; schede Preferiti e Richieste
  (Seerr). Locandine con episodi rimanenti, segno di «visto» e voto; sfondo preso dal media selezionato.
- **Schede**: film, serie, stagione, episodio; riproduci, preferito, visto, download, scelta audio / sottotitoli
  ricordata per tutta la serie; «Altri simili» e suggerimenti Seerr; richiesta delle stagioni mancanti.
- **Riproduzione** (libmpv): capitoli, episodi della stagione, «Salta l'intro», episodio successivo, suggerimenti
  a fine serie, volume proprio di Turtlefin, bitrate massimo (oltre, il server converte il video); posizione e «visto» inviati al server.
- **Watch party** (SyncPlay): guardare la stessa cosa nello stesso momento su più dispositivi.
- **Android** (telefono, tablet, Android TV): la stessa applicazione, in verticale e in orizzontale, col dito
  (righe, cast, episodi che scorrono; dal bordo sinistro: indietro), volume del telefono, telecomando e gamepad
  di Android TV.
- **Trasmetti**: mandare un video su un altro schermo — un altro Turtlefin, un'app Jellyfin dell'account o un
  Chromecast (con un relè attraverso il dispositivo quando la TV non raggiunge il server) — e pilotarlo dal
  lettore. **Telecomando**: pilotare un altro Turtlefin dell'account come col suo telecomando.
- **Pagine Seerr**: un contenuto assente dal server si apre in una pagina sua (trama, informazioni, cast,
  «Leggi tutto»), con Richiedi e **Trailer** (preparato sul server dal plugin Turtlefin Trailers, nella lingua
  dell'account, conservato al massimo 24 ore).
- **Trailer**: icona nelle schede; quelli del server (nella lingua dell'account se ce ne sono più d'uno),
  altrimenti YouTube tramite yt-dlp se è installato.
- **Temi**: Turtlefin, Frutiger Aero e Nerd, ognuno col suo logo, il suo avvio e il suo sfondo animato; un tema
  proprio (vedi Temi più sotto). Offline, tutta l'interfaccia diventa ambra.
- **Archiviazione**: cartella dei download e limite di spazio; **controllo della password** (avviso se è troppo
  semplice, modifica dall'applicazione).
- **Offline**: i download sostituiscono la home, con schede complete senza server; ciò che è stato visto o messo
  tra i preferiti offline viene inviato all'account alla riconnessione.
- **Ricerca** (libreria + Seerr) e media a caso.
- **Telecomando, tastiera e mouse ovunque**: interfaccia TV (elementi grandi, tastiera a schermo) o interfaccia
  computer (finestra o schermo intero, F11), clic e rotellina su tutte le pagine, e una tastiera fisica scrive
  direttamente nei campi di testo, in entrambe le modalità.
- **Impostazioni**: immagine del profilo (avatar GetAvatar per categoria), lingua dell'interfaccia, lingue audio e
  sottotitoli, dimensione dei sottotitoli, episodio successivo e intro automatici, interfaccia TV, schermo intero,
  sfondo, voti, ora, indirizzi del server, cache delle immagini, **aggiornamento da GitHub**.
- **Animazioni** ovunque (locandina che vola verso la scheda, menu che scorre, righe a cascata, accesso animato,
  cambio di lingua), regolabili una per una in Impostazioni → Animazioni, con preset (Tutte, Leggere, Nessuno) e
  i propri, esportabili e importabili.
- **Visita guidata**: proposta al primo avvio, da rivedere in Impostazioni → Informazioni.

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

Su Windows viene scaricato l'ultimo programma di installazione ed eseguito per il tuo account utente (senza
diritti di amministratore). Su Debian, Ubuntu, Linux Mint e le altre distribuzioni con apt viene installato il
pacchetto `.deb` (viene chiesta la tua password); altrove l'AppImage va in `~/.local/bin` con una voce nel menu
delle applicazioni.

### A mano

| Sistema | File | Note |
|---|---|---|
| Windows 64 bit | `Turtlefin-<version>-windows-x64-setup.exe` | Programma di installazione: lingua, cartella e modalità **installata** (menu Start, disinstallazione) o **portatile** |
| Windows 64 bit, senza installare | `Turtlefin-<version>-windows-x64-portable.zip` | Estrarre dove si vuole (chiavetta USB…) e avviare `turtlefin.exe` |
| Windows 32 bit | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Per i PC vecchi |
| Linux, tutte le distribuzioni (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Renderlo eseguibile (`chmod +x`), poi avviarlo |
| Linux, ARM 64 bit | `Turtlefin-<version>-linux-aarch64.AppImage` | Idem |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Doppio clic, oppure `sudo apt install ./turtlefin_….deb` |
| Android (telefono, tablet, TV) | `Turtlefin-<version>-android-arm64.apk` | Consentire l'installazione da questa fonte, poi aprire il file (Android 8 o successivo) |

Le versioni Windows e l'AppImage contengono tutto (lettore libmpv compreso). Il pacchetto `.deb` usa la libmpv
del sistema (`libmpv2`, installata automaticamente da apt). AppImage e pacchetti richiedono una distribuzione del
2022 o più recente (Ubuntu 22.04, Debian 12…).

**Versione portatile**: un file `portable` accanto a `turtlefin.exe` fa tenere configurazione, account, cache e
download nella cartella `data` accanto al programma; nient'altro viene scritto altrove.

### Aggiornare

**Impostazioni → Informazioni → Cerca aggiornamenti** confronta la versione installata con l'ultima pubblicata,
poi **Aggiorna** si occupa di tutto a seconda di come è installato Turtlefin:

| Installazione | Aggiornamento |
|---|---|
| Windows, installato | il nuovo programma di installazione viene scaricato ed eseguito in silenzio nella stessa cartella |
| Windows, portatile | il nuovo archivio viene scaricato e i suoi file sostituiscono i vecchi |
| AppImage | il nuovo file sostituisce il vecchio |
| Pacchetto .deb | il pacchetto viene installato con `pkexec` (viene chiesta la password di amministratore) |

**Riavvia Turtlefin** avvia poi la nuova versione.

## Plugin del server consigliati

Turtlefin funziona con un semplice server Jellyfin (10.11 o più recente). Questi plugin, da installare sul
**server**, aggiungono funzioni:

| Plugin | Cosa porta a Turtlefin |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Scheda Richieste, risultati Seerr nella ricerca, suggerimenti e richieste Seerr nelle schede — con l'accesso Jellyfin, senza chiave Seerr nel client |
| [Seerr](https://github.com/seerr-team/seerr) (prima Jellyseerr) | Il gestore delle richieste vero e proprio, usato da Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Individua intro e titoli di coda: pulsante «Salta l'intro», salto automatico, «Episodio successivo» al momento giusto |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | Una galleria di immagini del profilo da scegliere in Impostazioni → Account |
| Turtlefin Trailers (pubblicato a parte) | Scarica sul server i trailer mancanti, nella lingua di ogni libreria e di ogni account, e prepara quelli delle pagine Seerr |

## Avviare

```
turtlefin                                  animazione di avvio, poi «Chi sta guardando?» (o l'account di avvio)
turtlefin "Nome"                           account salvato «Nome»
turtlefin "Nome" --server=http://…         accesso diretto (password: variabile TURTLEFIN_PASSWORD=…)
turtlefin --tv                             interfaccia TV: schermo intero, elementi grandi
turtlefin --desktop                        interfaccia computer (prevale sull'impostazione «Interfaccia TV»)
turtlefin --no-intro                       nessuna animazione di avvio
turtlefin --sub=fr --audio=ja             sottotitoli / lingua audio di questo avvio (--sub=off: nessuno)
turtlefin --fullscreen                     schermo intero senza l'interfaccia TV
turtlefin --console                        finestra del registro (Windows)
turtlefin --tutorial                       visita guidata all'arrivo sulla home
```

La riga di comando prevale sempre sulle impostazioni (account di avvio, interfaccia TV).

## Tasti e mouse

- **Frecce** per muoversi, **Invio** per aprire / attivare, **Esc** o **Backspace** per tornare indietro (tenuto
  premuto: un solo passo indietro).
- Home: **←** sulla prima scheda (o Indietro) apre il menu; nel menu, **→** o Esc lo chiude.
- **↑** dalla parte alta di una pagina: barra superiore (indietro, home, menu, watch party, a caso, ricerca,
  account).
- Riproduzione, comandi nascosti: **← →** indietro / avanti di 10 s, **↑ ↓** o Invio mostrano i comandi, ↓ dai
  pulsanti: episodi della stagione. **Spazio**: pausa · `a` audio · `s` sottotitoli · `f` schermo intero.
- **F11**, ovunque: schermo intero (anche in Impostazioni → Schermo → **Schermo intero**, ricordato tra un avvio
  e l'altro fuori dall'interfaccia TV).
- **Telecomando della TV**: Stop, episodio successivo / precedente, avanti / indietro veloce, Menu (menu a
  sinistra) e il gamepad (Android, Raspberry Pi con XeLauncher) funzionano come la tastiera.
- **Mouse**: clic per aprire, rotellina per passare da una riga all'altra (Maiusc + rotellina: dentro la riga).
- **Tocco e trascinamento**: toccare apre; far scorrere il dito (o trascinare col tasto sinistro del mouse) fa scorrere la riga o la pagina, un gesto rapido va più lontano; dal bordo sinistro: indietro.
- **Testo**: una lettera digitata sulla tastiera va direttamente nel campo (ricerca, accesso, indirizzo), anche
  nell'interfaccia TV; lì, un clic sul campo di ricerca apre la tastiera a schermo.

## File

| Dove | Cosa |
|---|---|
| cartella di configurazione, `turtlefin/` | `session.json` (sessione attuale), `accounts.json` (account salvati), `prefs.json` (impostazioni del dispositivo), `tracks.json` (tracce per serie), `userdata.json` (visti / preferiti offline) |
| cartella dei dati, `turtlefin/downloads/` | download ordinati come una libreria: `<libreria>/<serie>/Season 01/<serie> S01E01.mkv` (con il suo `.json`, l'immagine e i sottotitoli), immagini della serie condivise; `queue.json` (coda in attesa) |
| cartella della cache, `turtlefin/img/` | immagini e immagini del profilo (svuotabile in Informazioni) |
| **Documenti** | `Turtlefin Languages` (lingue aggiunte) e `Turtlefin Presets` (preset di animazioni esportati) |

Su Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Su Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Versione portatile: tutto in `data\` accanto a `turtlefin.exe` (`config`, `cache`, `downloads`), e le cartelle
delle lingue e dei preset accanto al programma.

## In caso di problemi

- `turtlefin --console` (Windows) o un avvio da terminale (Linux) mostra il registro.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: registro di mpv · `TURTLEFIN_MPV_ARGS="…"`: opzioni mpv aggiuntive.
- `TURTLEFIN_HWDEC=no`: decodifica software (hardware di default, `drm-copy` sul Raspberry Pi) · `TURTLEFIN_AO=alsa`: uscita audio.
- Righe o disturbi nell'immagine (NVIDIA): Impostazioni → Riproduzione → **Decodifica hardware** spenta (tutto è decodificato dal processore).
- `TURTLEFIN_LIBMPV=percorso`: altra posizione di libmpv · `TURTLEFIN_DEBUG_FRAMES=1`: segnala i fotogrammi lenti.
- Il rendering deve essere OpenGL (scelto automaticamente): con `SLINT_BACKEND=winit-software` niente video.

## Temi

Impostazioni → Schermo → **Tema**: **Turtlefin** (predefinito: esagoni, viola → blu), **Frutiger Aero** (nello
spirito di Windows 7: cielo e bolle, vetro azzurrato, pulsanti lucidi, logo a sfera) e **Nerd** (una scheda madre
di notte: il server è il processore, la corrente percorre le piste fino a ogni componente) — o un tema proprio.
«Sfondo animato» (Impostazioni → Animazioni) anima ogni sfondo; sul Raspberry Pi è spento per impostazione
predefinita.

- **Crea un tema** apre *Turtlefin Theme Creator* nel browser: ogni colore, l'arrotondamento, il riflesso lucido,
  il cielo e le bolle, con anteprima in tempo reale; salva un file `.tftheme`.
- Mettere quel file nella cartella **Turtlefin Themes** (Documenti, o accanto a `turtlefin.exe` nella versione
  portatile), poi **Importa un tema**. **Esporta il tema** scrive il tema attuale in quella cartella, come punto di
  partenza.
- Il creatore è anche nel repository, `tools/theme-creator.html`: un unico file che funziona offline.

## Tradurre Turtlefin

Senza programmare né compilare. Le lingue aggiunte stanno in un'unica cartella, **Turtlefin Languages**: in
**Documenti** (versione installata) o accanto a `turtlefin.exe` (versione portatile).

1. **Impostazioni → Schermo → Aggiungi una lingua** apre un esploratore di quella cartella; **Crea il modello**
   vi scrive `modele.po`, con i testi in inglese e, come nota, il francese originale e la lingua attuale.
2. Farne una copia chiamata `<codice>.po` (`sv.po` per lo svedese, `ja.po` per il giapponese…), in quella
   cartella o in una sottocartella, e riempire ogni `msgstr ""` con la traduzione del `msgid` inglese sopra.
   Mantenere i `{}` e `{n}`. Riempire anche `X-Language-Name` (nome mostrato) e, se serve, `Plural-Forms`
   (regola gettext della lingua). Va bene qualsiasi editor di `.po`, per esempio [Poedit](https://poedit.net).
3. Di nuovo **Aggiungi una lingua**: la traduzione compare con la parte già tradotta; sceglierla la applica. I
   testi lasciati vuoti appaiono in inglese.

Per condividerla con tutti: una pull request che aggiunge il file a `lang/` (e a `BUILTIN` in `src/i18n.rs`);
`python tools/lang-check.py` verifica che non manchi nulla.

## Sviluppo

Vedi [HANDOFF.it.md](HANDOFF.it.md) (anche in [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Español](HANDOFF.es.md), [Nederlands](HANDOFF.nl.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): stato del progetto, decisioni, compilazione, pacchetti, pubblicazione, traduzioni,
problemi noti.

## Licenza

Turtlefin è distribuito con licenza [GNU GPL v3](LICENSE) (o successiva): chiunque può usarlo, studiarlo,
modificarlo e ridistribuirlo, purché mantenga la stessa licenza e condivida il codice delle proprie modifiche.
