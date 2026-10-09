# Turtlefin: projectoverdracht (stand op 9 oktober 2026, versie 1.0.0)

[Français](HANDOFF.md) · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · **Nederlands** · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

Voor wie de ontwikkeling overneemt (een mens of Claude Code). Lees het helemaal voordat je de code aanraakt, en
lees daarna [README.nl.md](README.nl.md) (gebruik, installatie, toetsen, bestanden).
Repository: https://github.com/Xelopteryx/Turtlefin · Versie in `Cargo.toml`: 1.0.0.

## 1. Doel

Een **native Jellyfin-client in Rust**, licht, geanimeerd, even goed te bedienen met een afstandsbediening als met
toetsenbord en muis, te installeren op elke Windows- of Linux-computer **zonder iets te compileren**: wie hem
gebruikt, downloadt een installatieprogramma of pakket (of voert een installatieopdracht uit), meer niet. Alle
pakketten worden gebouwd door de GitHub-CI (of de pc van wie het project onderhoudt), nooit door de gebruiker.

Waarom: Jellyfin Desktop (Qt / QtWebEngine) lekt geheugen en loopt op kleine machines uiteindelijk vast, en de
webinterface met een zwaar thema zakt op bescheiden hardware onder 30 beelden/s. Vaste regel: stabiel blijven in
het geheugen en nooit meer realtime vervaging of filteranimaties invoeren.

## 2. Genomen beslissingen

| Onderwerp | Beslissing | Reden |
|---|---|---|
| Taal / UI | Rust + **Slint** `~1.18` (weergave 100 % Slint), stijl `fluent-dark` opgelegd door `build.rs` | Geen browser; de stijl „native” zou van Qt afhangen |
| Slint-feature `unstable-winit-030` | winit-gebeurtenisfilter voor **F11** (`install_f11`) | De enige manier om een globale toets te krijgen; vandaar `~1.18` (API instabiel tussen kleine versies) |
| Netwerk | `reqwest` 0.13 (rustls, certificatenopslag van het systeem), `tokio` | `query` is in 0.13 een feature die je moet inschakelen |
| Afspelen | **libmpv tijdens het draaien geladen** (`libloading`, `src/mpv.rs`), OpenGL-weergave in een textuur die Slint toont (`src/video.rs`), Slint-bediening erbovenop (`ui/player.slint`) | Ingebouwde speler, geen IPC, werkt onder Wayland. De speler wordt bij elke weergave opnieuw gemaakt (begrensd geheugen) |
| Slint-renderer | femtovg (OpenGL / GLES) opgelegd, tenzij `SLINT_BACKEND` is ingesteld | De video loopt via een OpenGL-textuur |
| Videotextuur | Fysieke pixels, oorsprong `TopLeft`, GL-toestand rond mpv bewaard / hersteld; `loadfile` wacht op de rendercontext | Anders een omgekeerd beeld of „No render context set” |
| Decodering | `hwdec=no` op Linux ARM 64-bit, elders `auto-safe` (Windows: `d3d11va-copy`) | Op de geteste ARM-borden (driver v3d) levert V4L2-decodering een formaat dat de renderer niet kan importeren |
| Geluid op Linux | `ao=pipewire,pulse,alsa`, `config=no` | Een gebruikers-`mpv.conf` die ALSA oplegde, faalde als PipeWire de HDMI-uitgang vasthoudt |
| Geheugen | mpv-cache begrensd (100 / 25 MiB), afbeeldingen op de juiste grootte opgevraagd, 16 items per rij | Kleine machines (4 GB) |
| Talen | Frans in de code (brontaal), vertaling tijdens het draaien door `src/i18n.rs` uit `lang/<code>.po` (ingebouwd) en de map `Turtlefin Languages` | Zie hoofdstuk 6 |
| Volledig scherm (Windows) | Randloos venster dat het scherm + 1 px bedekt (`src/winfull.rs`, `set_tv_window`), geen echt volledig scherm; volgt resolutiewijzigingen (elke 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` voor het volledige scherm van Slint | In OpenGL-volledig scherm ziet AMD Software de app als een spel: „Druk op ALT + R” bij elke terugkeer naar de voorgrond |
| Tv- / computerinterface | `tv-mode` verandert alleen de grootte (`k` = 1,4) en de invoer (schermtoetsenbord); volledig scherm staat los (`full_flag`, `UiPrefs::fullscreen`, F11) | Verzoek van de gebruiker: volledig scherm zonder de interface te vergroten |
| Venster (Windows, desktop) | Verkleind en gecentreerd als 1280 x 720 + rand groter is dan het werkgebied; blijft geanimeerd tijdens het verplaatsen (`winfull::keep_alive_while_moving`) | Schermen van 1366 x 768; Windows blokkeert de gebeurtenislus tijdens het verplaatsen |
| Consolevenster (Windows) | Subsysteem „windows” in release; `--console` koppelt er een aan / opent er een; externe opdrachten zonder venster (`paths::quiet_command`) | Geen console en geen flitsend CMD-venster |
| Opdrachtregel | Gaat altijd voor de instellingen (startaccount, tv-interface) | Verzoek van de gebruiker |
| Wachtwoord | Nooit opgeslagen (alleen het token); op de opdrachtregel liever `TURTLEFIN_PASSWORD` | Een argument is zichtbaar voor andere processen |

## 3. Opbouw van de code

```
build.rs            gecompileerde commit, Slint-stijl, pictogram van de exe (winresource, Windows)
lang/<code>.po      ingebouwde vertalingen (bron: het Frans in de code); tools/lang-check.py controleert ze
ui/theme.slint      thematokens, globals Tr (vertaling) en Motion (ingeschakelde animaties)
ui/app.slint        AppWindow en alle schermen (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo: startanimatie (7 punten, verbinding, zoom), taalkeuze
ui/player.slint     afspeelscherm          ui/osk.slint      schermtoetsenbord
ui/card.slint       posterkaart            ui/marquee.slint  scrollende tekst
ui/typed.slint      geanimeerde getypte tekst (Str, TypedText)   ui/langx.slint  verkenner (talen, presets)
ui/dust.slint       balken van de taalwissel                     ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, gedeelde toestand App (Arc), schermen, navigatie (stapel + bewaarde pagina's), instellingen
src/boot.rs         opstartvolgorde (controles, taal, keuze van het beginscherm)
src/i18n.rs         huidige taal, tr() / trf() / trn(), toegevoegde talen, vertaalsjabloon
src/dust.rs         effect van de taalwissel (tekstregels vinden, morphing)
src/api.rs          Jellyfin-REST-client (+ Seerr-relay van Jellyfin Enhanced, GetAvatar)
src/config.rs       sessie, accounts (hoogstens 12), prefs.json (UiPrefs, AnimFlags, presets), sporen, offline gezien/favorieten
src/discovery.rs    servers zoeken (UDP, subnetten, ARP, VPN-peers)
src/downloads.rs    downloads (hervatten met Range, wachtrij, offline synchronisatie)
src/mpv.rs          koppeling met libmpv; src/video.rs OpenGL-textuur; src/player.rs afspelen, meldingen, doorschakelen
src/syncplay.rs     watch party (WebSocket /socket)
src/paths.rs        mappen voor configuratie / cache / gegevens; draagbare modus; quiet_command
src/update.rs       bijwerken volgens de installatie (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows: scherm / werkgebied, venster geanimeerd tijdens het verplaatsen
src/theme.rs        ingebouwde thema's, .tftheme-bestanden (ThemeDef), toegepast op het global Theme
ui/sky.slint        decor van Frutiger Aero (lucht, heuvel, bellen), statisch
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   compileren en publiceren bij een tag `v*` (proef: branch `ci`)
```

Principes:
- Netwerkgegevens gaan via `Send`-structuren, daarna zet `upgrade_in_event_loop` ze in de Slint-modellen.
  Afbeeldingen buiten de UI-thread gedecodeerd, 6 parallelle downloads, toegepast met een controle op de id.
- `App.gen` maakt verouderde laadacties ongeldig; `App.stack` is de navigatiestapel; `PAGES` bewaart detail- en
  bibliotheekpagina's om zonder verzoek terug te gaan.
- Toetsenbordnavigatie met de hand (selectie-indexen in Rust en Slint), omdat Slint de focus van dynamische kaarten
  niet beheert. Elk scherm heeft zijn `FocusScope`; `refocus` geeft het toetsenbord terug aan de juiste plek.
- `changed`-handlers van Slint worden uitgesteld: reken niet op hun volgorde (posities in twee stappen enz.).
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`, header
  `Authorization: MediaBrowser …, Token=…`. Meldingen: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` en de WebSocket weigeren `api_key`: token in de header.

## 4. Opstarten

`main` past de taal toe (prefs.json, anders het bestand `language` dat het Windows-installatieprogramma schrijft) en
start dan `boot::run`. Het scherm `boot` toont 7 punten: de 6 hoeken van de zeshoek en dan het midden. Elk punt is
een echte controle (`boot::check`):
1. **taal** (gevraagd als ze onbekend is): de vertalingen van de gekozen taal worden geladen (`i18n::check`);
2. **weergave**: het venster heeft een OpenGL-context gekregen (`video::gl_info`, versie en videokaart in het logboek);
3. **videospeler**: een echte mpv-speler wordt gemaakt, geïnitialiseerd en weer vernietigd (`mpv::self_test`);
4. **opslag**: een bestand wordt geschreven, teruggelezen en verwijderd in de mappen voor configuratie, gegevens en cache;
5. **configuratie**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` leesbaar
   (`config::unreadable_files`, helemaal aan het begin van `main` aangeroepen, voordat een beschadigd bestand
   wordt overschreven);
6. **netwerk**: actieve interface of route naar buiten (`discovery::has_network`);
7. **server** (het midden): het hoofdadres, anders het reserveadres, antwoordt op `/System/Info/Public` **en** het is
   dezelfde server (id vergeleken met `server_id` van de sessie); oranje zolang er geen server is ingesteld.

Rood = fout, met een bericht en „Doorgaan” (vanzelf na 12 s). Alles groen: de hoeken verbinden zich, de stralen
gaan naar het midden en het serverpunt wordt de gevulde zeshoek — precies het logo (`packaging/turtlefin.svg`,
dezelfde geometrie) —, daarna een zoom naar het midden. Slint verkleint de tekening van een `Path` met de dikte van
de lijn: de paden van het logo zijn evenveel vergroot om op de punten te vallen.

Beginscherm (`boot::route`), in deze volgorde: naam + wachtwoord op de opdrachtregel → aanmelden; naam van een
opgeslagen account → dat account (onbekende naam: het aanmeldformulier); startaccount (`prefs.autostart_user` /
`autostart_server`) → `fly_autostart` (de foto van het account in het midden tijdens het aanmelden); anders „Wie
kijkt er?” (of het zoeken naar een server als er geen bekend is). Een verdwenen startaccount leidt naar „Wie kijkt
er?”. `--no-intro` (of de uitgeschakelde animatie „Opstarten”) slaat de animatie over; de controles gebeuren toch.

## 5. Stand bij versie 1.0.0

Alle functies uit de README zijn klaar en gecontroleerd op schermafbeeldingen (Windows-pc, schermen van 1366 x 768
en 1920 x 1080) en op een Linux-ARM-machine aan een tv, met de testaccounts `test` / `test2` van een echte server.
Door de CI gepubliceerde versies: 0.9.0, 0.9.1; 1.0.0 is klaar om getagd te worden (hoofdstuk 7).

Mechanismen die in de code niet vanzelf spreken:
- **Gedeelde afbeelding** (`global Hero`): de afbeelding van een kaart vliegt naar de poster van de detailpagina en
  bij terugkeer precies terug op de kaart (`Hero.want-id`, `hero-card-ok`).
- **Rijen** (`global Rows`): eigen scrollpositie per rij, onthouden per sleutel; bij het wisselen van rij kom je
  op de dichtstbijzijnde kaart uit (`row-to`, `detail-pick-down`).
- **Offline**: `config::Flags` (userdata.json) bewaart gezien / favorieten / posities met een vlag „te
  versturen”; `downloads::sync` stuurt ze als de server terug is (het apparaat heeft het laatste woord).
- **Adressen**: `server_main` / `server_backup`; `watch_addresses` (20 s) schakelt over naar het reserveadres en
  terug.
- **Watch party**: één WebSocket-verbinding per sessie (`sp_conn`), stopt bij 401 / 403, oplopende wachttijd.
- **Profielfoto's**: schijfcache `avatar_<id>_still|anim.bin` + ronde miniatuur `avatar_<id>_thumb.png`. De
  miniatuur wordt meteen getoond, de volledige GIF buiten de UI-thread gedecodeerd (`avatar_cached_async`) en alleen
  van de server vernieuwd als hij veranderd is (`avatar_fetch`). GIF-animatie: `AnimSlot` (Login, Picker, Header,
  Fly) en één gedeelde timer; de avatar in de kopregel krijgt al zijn beelden één keer (`avatar-frames`) en alleen
  het zichtbare beeld verandert. GetAvatar `SetAvatar` antwoordt 500 → terugval `POST /UserImage`.
- **Geanimeerd aanmelden** (`fly-phase` 1 tot 4): de foto gaat naar het midden, laadbalk, vliegt weg, dan komt de
  startpagina en springt de avatar in de kopregel tevoorschijn (`me-pop`).
- **Taalwissel** (`dust.rs`, `ui/dust.slint`): bij het openen van de talenlijst bedekken balken elke tekstregel,
  nemen na de keuze de breedte van de nieuwe woorden aan en onthullen ze dan. De regels worden gevonden door een
  momentopname van de pagina (`take_snapshot`) te vergelijken met een momentopname in het lettertype
  `Turtlefin Blank` (lege tekens, dezelfde breedtes, `tools/make-blank-font.py`). De klok (Montserrat) valt erbuiten.
- **Rondleiding** (`tour-step` 0 tot 10): elke stap zet de interface in de verwachte toestand of telt als gedaan
  als het gebaar al is gemaakt; `tour-ev` wordt aangeroepen vanuit `changed`-handlers.
- **Animaties**: global `Motion` (12 schakelaars: boot, pages, menu, select, scroll, panels, detail, player,
  search, login, language, tour), `config::AnimFlags`, ingebouwde presets (Alle, Licht, Geen) en eigen presets,
  geëxporteerd / geïmporteerd als `.json` in `Turtlefin Presets`. Elke animatieduur wordt geschreven als
  `duration: Motion.x ? 300ms : 0ms`.
- **Invoer**: op de desktop zijn de velden `TextInput`s (tekst verborgen, getekend door `TypedText`); in tv-modus
  het schermtoetsenbord (`Osk`). In beide modi gaat een afdrukbare toets die de pagina ontvangt naar het veld
  (`typing-key`, `erase-key`, `Field.type`); in tv-modus wist Backspace alleen zolang er tekst over is. Op de
  desktop neemt een klik naast een veld het toetsenbord niet af (`focus-on-click: root.tv-mode`).
- **Ingedrukt gehouden toetsen**: Enter, Esc en Backspace herhalen hun actie niet (`event.repeat`), behalve
  Backspace bij het wissen van tekst.
- **Thema's** (`src/theme.rs`, global `Theme` in ui/theme.slint): geen vaste kleur in de interface; doorschijnende
  vlakken worden `Theme.fg.with-alpha(…)` geschreven (wit op een donker thema, inkt op een licht), tekst op het
  accentverloop gebruikt `Theme.on-accent`. `Gloss` (gelglans) en `AeroSky` (ui/sky.slint) verschijnen alleen bij
  `Theme.gloss` / `Theme.bubbles`. Geïmporteerde thema's staan in prefs.json (`theme`, `themes`); de themamaker
  (`tools/theme-creator.html`) zit in het programma (`theme::CREATOR`) en „Thema maken” zet hem in
  „Turtlefin Themes”. Zijn startthema's moeten gelijk blijven aan die in theme.rs.
- **Muis**: overal klikken; scrollwiel op de startpagina, de detailpagina (`d-nav`, gedeeld met het toetsenbord),
  zoeken, bibliotheken, downloads, instellingen; vensters op de voorgrond vangen het scrollwiel op.

Niet gedaan: Quick Connect; gamepad; licentie (te kiezen door wie het project onderhoudt, voor of na 1.0.0);
koppeling met XeLauncher (starter van het mediacenter van wie het project onderhoudt, geen prioriteit). Hierna
gepland: optimalisatie, versie voor Android TV.

## 6. Vertalingen

- Tijdens het draaien gedaan door `src/i18n.rs`: één catalogus voor Rust en voor de interface.
  Slint: global `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)`; `Tr.k` verandert bij elke taalwissel, waardoor de teksten
  opnieuw worden berekend. Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- De Franse tekst **is** de sleutel: als je die wijzigt, moet de `msgid` in elke `lang/*.po` mee veranderen
  (`tools/lang-check.py` meldt ontbrekende en overtollige teksten en verloren `{}`). De 8 ingebouwde talen (fr, en,
  es, de, it, pt, pl, nl; `BUILTIN` in i18n.rs) zijn volledig (~530 teksten).
- Toegevoegde talen: elke `<code>.po` in de map `Turtlefin Languages` (`i18n::lang_dir`: Documenten, naast de exe
  in de draagbare versie, onder `TURTLEFIN_CONFIG_DIR` tijdens tests; de oude map `languages` wordt daarheen
  verplaatst), submappen inbegrepen; naam gelezen uit `X-Language-Name`; een bestand kan een ingebouwde taal
  vervangen. Doorbladerd met een ingebouwde verkenner (`ui/langx.slint`, functies `lx_*` in main.rs).
- „Sjabloon maken” schrijft `modele.po`: msgid in het **Engels**, header `X-Source-Language: en`, notities `#.` in
  het Frans en in de huidige taal; `keyed` zet die msgid via `lang/en.po` terug naar het Frans.
- Teksten die in een taal ontbreken: Engels. Meervouden: regel `Plural-Forms` van het bestand, geëvalueerd door
  i18n.rs.
- Installatieprogramma: `[Languages]` en `[CustomMessages]` in `turtlefin.iss`; het schrijft de gekozen code naar
  `language` naast de exe, die bij de eerste start wordt overgenomen.
- Namen die van de server komen (bibliotheken, media) worden niet vertaald.

## 7. Compileren, pakketten bouwen, publiceren

Ontwikkeling:
- Windows: Rust (https://rustup.rs), „Build Tools voor Visual Studio” (C++), git; `cargo build --release`;
  `libmpv-2.dll` (archief `mpv-dev-x86_64-….7z` van
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) naast de exe.
  De debugversie heeft `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll` nodig.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev`, daarna `cargo build --release`.
- `cargo test --release`: tests voor i18n, update enz.

Een versie publiceren:
1. Het nummer in `Cargo.toml` zetten (`version = "x.y.z"`), één keer compileren (werkt `Cargo.lock` bij),
   committen.
2. `git push origin main`, daarna `git tag -a vx.y.z -m "Nieuwigheden, één per regel"` en `git push origin vx.y.z`.
   Het bericht van de tag wordt de versie-opmerkingen, die de ingebouwde update toont.
3. `release.yml` compileert Windows x64 / x86 en Linux x86_64 / aarch64, bouwt installatieprogramma's, archieven,
   AppImages en `.deb`-pakketten en publiceert ze in een Release (te volgen in het tabblad Actions van de
   repository). `update.rs` en de installatiescripts verwachten de bestandsnamen (kop van `release.yml`) precies zo.
4. Proef zonder publiceren: `git push origin main:ci` (alles wordt gecompileerd, niets gepubliceerd).

Met de hand:
- **Windows**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (of `x86`);
  vereist Inno Setup 6, 7-Zip en NASM (x86). Resultaat in `target\dist`.
- **Linux** (op een Linux-machine): `TURTLEFIN_DIST=release cargo build --release`, daarna
  `sh packaging/linux/build-appimage.sh <version>` en `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` bij het compileren, anders denkt `update::kind()` dat het een lokaal gecompileerde
  versie is.
- x86: de 32-bit-libmpv's van shinchiro die sinds juli 2026 zijn gepubliceerd, crashen bij het starten (OpenSSL);
  die van 10 juni 2026 wordt bewaard in de voorlopige release `libmpv-i686-20260610` van Turtlefin (niet
  verwijderen), die `build.ps1` gebruikt (`MPV_TAG`: een andere versie van shinchiro). aws-lc vereist NASM in 32-bit.
- Pictogrammen: `packaging/turtlefin.svg` is het logo; `packaging/icons/make-icons.py <map>` (Python + Pillow)
  maakt de PNG's en het ICO opnieuw.

Branches: `main` (de enige werkbranch), `ci` (CI-proeven). `interface-lua` en `libmpv` zijn oude experimenten, al
samengevoegd in `main`: ze mogen weg. Tags: `v0.9.0`, `v0.9.1` (gepubliceerde versies) en `libmpv-i686-20260610`
(32-bit-libmpv, zie hierboven).

## 8. Testen

- `TURTLEFIN_CONFIG_DIR=<map>`: een andere configuratiemap (accounts, prefs, talen) zonder de echte aan te raken.
- `--open=ID|settings|downloads`, `--play=ID@SECONDEN`, `--test-video=bestand` (speler zonder server).
- `TURTLEFIN_DEBUG_FRAMES=1` (beelden > 25 ms); `TURTLEFIN_DEBUG_GAPS=1` met
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed`: pauzes van meer dan 40 ms tussen twee beelden.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: berichten van de watch party. Twee instanties op één pc: twee verschillende
  `TURTLEFIN_CONFIG_DIR`, `--desktop`.
- Een `prefs.json` die PowerShell 5 heeft geschreven, heeft een BOM: bij het lezen van de configuratiebestanden
  wordt die genegeerd.
- Geautomatiseerde tests op Windows: `SetForegroundWindow` wordt pas na een gesimuleerde toets geaccepteerd;
  gebruik F24, niet Alt (Alt alleen zet het venster in menumodus en de volgende klik gaat verloren). Een
  ingedrukt gehouden toets simuleer je met meerdere opeenvolgende `keybd_event`-aanroepen „toets omlaag” (Windows
  markeert ze als herhalingen).

## 9. Bekende problemen / grenzen

1. Een crash van mpv laat Turtlefin crashen (hetzelfde proces).
2. Afspelen vereist OpenGL-weergave (`SLINT_BACKEND=winit-software` verhindert het).
3. **mpv 0.40 / 0.41** (in mpv opgelost op 23 januari 2026, commit f74adc4): per beeld een OpenGL-fence die nooit
   wordt vrijgegeven; met de driver v3d neemt elke fence een descriptor in beslag („MESA: error: Export failed” na
   ~42 s). Omweg in `src/mpv.rs` (alleen OpenGL ES). Diagnose: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` moet
   tijdens het afspelen stabiel blijven.
4. Geheugengroei van mpv (~3 MB/min) bij ASS-ondertitels: beperkt tot één weergave (mpv voor elke video opnieuw
   gemaakt).
5. Token in leesbare tekst in `session.json` / `accounts.json` (0600 op Unix).
6. Tearing onder Xorg zonder compositor (alleen Openbox): gebruik een compositor (picom `--backend egl --vsync`).
   Turtlefin haalt 60 beelden/s.
7. Softwaredecodering op Linux ARM: kan moeite hebben met 4K / HEVC; spoor: `TURTLEFIN_HWDEC=auto-copy`.
8. Micropauzes (~40 ms), alleen gemeten met weergave op volle snelheid geforceerd, bij elk beeld van een GIF in de
   kopregel en bij het verbinden van het logo; oorzaak niet gevonden, onzichtbaar bij normaal gebruik.

## 10. Werkvoorkeuren van wie het project onderhoudt

- Antwoorden in het Frans; weinig ervaring met Linux / SSH: de opdrachten uitleggen.
- Niets naar GitHub pushen en geen versie publiceren zonder uitdrukkelijke toestemming; de versies publiceert wie
  het project onderhoudt.
- Testen met een kopie van de configuratie (`TURTLEFIN_CONFIG_DIR`), nooit de echte; testaccounts `test` / `test2`.
- Na elke reeks wijzigingen: een testinstallatieprogramma op het Bureaublad van wie het project onderhoudt
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, het oude verwijderd).
