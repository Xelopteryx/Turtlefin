<p align="center"><img src="packaging/icons/turtlefin-aero-256.png" width="96" alt="Turtlefin-logo (thema Frutiger Aero)">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin-logo">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-nerd-256.png" width="96" alt="Turtlefin-logo (thema Nerd)"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · **Nederlands** · [Polski](README.pl.md) · [Português](README.pt.md)

**Een native, lichte, geanimeerde Jellyfin-client, gemaakt voor de woonkamer én voor het bureau.** Geschreven in Rust
met Slint (interface) en libmpv (afspelen): geen Qt, geen ingebouwde browser. Hij draait op elke Windows- of
Linux-computer, van de oude laptop tot het kleine kastje bij de tv, en werkt even goed met een afstandsbediening als
met toetsenbord en muis — en op Android (telefoon, tablet, tv), met de vinger.

> **Volledig „gevibecodeerd” project**: alle code, teksten en vertalingen zijn geschreven door een AI (Claude, van
> Anthropic), begeleid en getest door een mens die zelf geen code heeft geschreven.

Huidige versie: **2.0.0** · Talen van de interface: Français, English, Español, Deutsch, Italiano, Português, Polski,
Nederlands — en elke taal die u zelf toevoegt (zie [Turtlefin vertalen](#turtlefin-vertalen)).

> **Over deze versie.** 2.0.0 is getest onder Windows, op een Raspberry Pi 5 aan een tv, onder Linux en op Android
> (telefoon, tv-emulator). Er moet nog het een en ander verbeterd worden (zie de notities van de
> [releases](https://github.com/Xelopteryx/Turtlefin/releases) en de bekende problemen in
> [HANDOFF.nl.md](HANDOFF.nl.md)). Uw feedback is welkom in de [issues](https://github.com/Xelopteryx/Turtlefin/issues).

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="100%" alt="Startpagina van Turtlefin, thema Turtlefin"></p>

## Schermafbeeldingen

De drie thema's, elk met een eigen geanimeerd decor voor dag en nacht (hier op een testserver met vrije films van de
Blender Foundation):

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="32%" alt="Thema Turtlefin">&nbsp;<img src="docs/screenshots/en-home-aero.jpg" width="32%" alt="Thema Frutiger Aero">&nbsp;<img src="docs/screenshots/en-home-nerd.jpg" width="32%" alt="Thema Nerd"></p>

De pagina van een film, het eindscherm (wat u net hebt uitgekeken en wat u daarna kunt kijken) en „Wie kijkt er?”:

<p align="center"><img src="docs/screenshots/en-detail.jpg" width="32%" alt="Pagina van een film">&nbsp;<img src="docs/screenshots/en-end-screen.jpg" width="32%" alt="Eindscherm">&nbsp;<img src="docs/screenshots/en-who-is-watching.jpg" width="32%" alt="Wie kijkt er?"></p>

Dezelfde app op een telefoon:

<p align="center"><img src="docs/screenshots/en-phone-home.jpg" width="24%" alt="Startpagina op een telefoon">&nbsp;<img src="docs/screenshots/en-phone-detail.jpg" width="24%" alt="Filmpagina op een telefoon">&nbsp;<img src="docs/screenshots/en-phone-settings.jpg" width="24%" alt="Instellingen op een telefoon"></p>

## Wat het kan

- **Accounts**: scherm „Wie kijkt er?” met profielfoto's (geanimeerde GIF's inbegrepen), tot 12 accounts opgeslagen op
  het apparaat (alleen het token, nooit het wachtwoord), optionele code per account, van account wisselen zonder
  opnieuw te typen. Servers zoeken op alle netwerken van het apparaat; een hoofd- en een reserveadres, dat geprobeerd
  wordt als het hoofdadres niet antwoordt.
- **Opstarten**: een geanimeerd logo waarvan de zeven punten echte controles zijn (taal, weergave, videospeler, opslag,
  configuratie, netwerk en in het midden de server), daarna „Wie kijkt er?” — of meteen het gekozen account met
  Instellingen → Account → **Dit account openen bij het starten**.
- **Startpagina**: banner, Mijn media, Verder kijken, Volgende, Recent toegevoegd; tabbladen Favorieten, Kijklijst en
  Verzoeken (Seerr). Posters met resterende afleveringen, vinkje „bekeken” en beoordeling; de achtergrond komt van het
  geselecteerde item.
- **Pagina's**: film, serie, seizoen, aflevering; afspelen, favoriet, bekeken, downloaden, keuze van audio /
  ondertitels onthouden voor de hele serie; beoordeling, duur, verschijningsdatum en, voor een serie of seizoen, de
  **bekeken afleveringen** („8 / 24” en het percentage); tabblad Details (genres, studio's, personen, audiotalen met
  hun vlag); „Meer zoals dit” en Seerr-suggesties; ontbrekende seizoenen aanvragen.
- **Afspelen** (libmpv): hoofdstukken, afleveringen van het seizoen, „Intro overslaan”, volgende aflevering, maximale
  bitrate (daarboven zet de server de video om), eigen volume; een geanimeerd **eindscherm**: wat u net hebt
  uitgekeken en drie suggesties, met de samenvatting van de gekozen suggestie; positie en „bekeken” teruggestuurd naar
  de server.
- **Zoeken** in de bibliotheek en in Seerr: titels, personen (met hun filmografie die op de server ontbreekt) en
  **studio's of zenders** („disney”, „pixar”, „netflix”: hun films en series om aan te vragen); filters, uitgebreid
  zoeken, willekeurig item.
- **Meerdere kaarten selecteren** in rasters (lang drukken, slepen met de muis, OK ingedrukt houden op de
  afstandsbediening), en ze daarna in één keer aanvragen of downloaden.
- **Watch party** (SyncPlay): hetzelfde tegelijk kijken op meerdere apparaten.
- **Android** (telefoon, tablet, Android TV): dezelfde app, staand en liggend, met de vinger (vegen vanaf de linkerrand:
  terug), volume van de telefoon, afstandsbediening en gamepad van Android TV.
- **Casten**: een video naar een ander scherm sturen — een andere Turtlefin, een Jellyfin-app van het account of een
  Chromecast — en hem vanuit de speler bedienen. **Afstandsbediening**: een andere Turtlefin van het account bedienen.
- **Seerr-pagina's**: een item dat niet op de server staat, opent op een eigen pagina (samenvatting, informatie,
  cast), met Aanvragen en **Trailer**.
- **Thema's**: Turtlefin, Frutiger Aero en Nerd, elk met een eigen logo, opstart en geanimeerd decor dat het tijdstip
  volgt; een eigen thema (zie [Thema's](#themas)). Offline wordt de hele interface amberkleurig.
- **Offline**: downloads vervangen de startpagina, met volledige pagina's zonder server; wat offline is bekeken of als
  favoriet gemarkeerd, gaat bij het opnieuw verbinden naar het account.
- **Afstandsbediening, toetsenbord, muis en vinger overal**: tv-interface (grote elementen, schermtoetsenbord) of
  computerinterface (venster of volledig scherm, F11).
- **Instellingen** per categorie, met zoekfunctie: profielfoto (GetAvatar), talen, ondertitels, afspelen, weergave,
  tekstgrootte, netwerk, opslag, animaties, mods, **update vanaf GitHub**.
- **Animaties** overal (poster die naar zijn pagina vliegt, banner die opengaat, rijen als een waterval, overgangen
  tussen pagina's), één voor één instelbaar, met voorinstellingen (Alle, Licht, Geen).
- **Vloeiendheid**: Turtlefin meet wat het apparaat aankan en maakt zijn decor vanzelf lichter om 60 beelden per
  seconde te halen.
- Ingebouwde **rondleiding** en **demo** („Turtlefin ontdekken” bij de eerste start), met echte vrije films; er wordt
  niets opgeslagen.
- **Discord-status** (standaard uit); **beheerders**: een item bewerken vanaf zijn pagina, dashboard.

## Systeemvereisten

Gemeten op een Raspberry Pi 5, een instaptelefoon (Galaxy A12s) en een afgeremde Android-emulator (één kern beperkt tot
30 %: bruikbaar, animaties op 15–50 beelden per seconde; dezelfde kern zonder beperking: animaties op 60 beelden per
seconde).

**Minimaal** — alles werkt, animaties uit (Instellingen → Animaties → **Geen**); het decor staat vanzelf stil als het
apparaat het niet bijhoudt:

- 64-bits processor met 4 kernen rond 1,5 GHz (Cortex-A53 / A55 van een tv of een instaptelefoon, recente Intel
  Celeron of Atom);
- 2 GB geheugen (Turtlefin gebruikt 200 tot 500 MB);
- grafische versnelling OpenGL (Windows, Linux op pc) of OpenGL ES (Android, Raspberry Pi); hardwarematige
  videodecodering aanbevolen;
- Windows 10 of 11, Linux uit 2022 of nieuwer (Ubuntu 22.04, Debian 12…), Android 8 of nieuwer.

**Aanbevolen** — alle animaties op 60 beelden per seconde:

- recente processor met 4 kernen rond 2 GHz of meer (Cortex-A76 van de Raspberry Pi 5, Intel Core i3 van de 8e
  generatie, AMD Ryzen 3, of beter);
- 4 GB geheugen;
- hardwarematige HEVC 10-bits decodering voor 4K.

## Installeren

Niets te compileren: downloaden, installeren, klaar. Alle bestanden staan op de pagina
[Releases](https://github.com/Xelopteryx/Turtlefin/releases).

### Met één opdracht

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

Onder Windows wordt het nieuwste installatieprogramma gedownload en voor uw gebruikersaccount uitgevoerd (zonder
beheerdersrechten). Onder Debian, Ubuntu, Linux Mint en de andere apt-distributies wordt het `.deb`-pakket
geïnstalleerd (uw wachtwoord wordt gevraagd); elders komt de AppImage in `~/.local/bin`, met een item in het
toepassingenmenu.

### Met de hand

- **Windows 64-bits**: `Turtlefin-<version>-windows-x64-setup.exe` — installatieprogramma: taal, map en modus
  **geïnstalleerd** (Startmenu, verwijderen) of **draagbaar**.
- **Windows 64-bits, zonder installatie**: `Turtlefin-<version>-windows-x64-portable.zip` — uitpakken waar u wilt
  (USB-stick…), daarna `turtlefin.exe` starten.
- **Windows 32-bits** (oude pc's): `…-windows-x86-setup.exe` of `…-windows-x86-portable.zip`.
- **Linux, alle distributies (x86_64)**: `Turtlefin-<version>-linux-x86_64.AppImage` — uitvoerbaar maken (`chmod +x`),
  daarna starten. ARM 64-bits: `…-linux-aarch64.AppImage`.
- **Linux met Flatpak (x86_64)**: `Turtlefin-<version>-linux-x86_64.flatpak` —
  `flatpak install --user ./Turtlefin-….flatpak` (de runtime komt van Flathub).
- **Debian, Ubuntu, Mint…**: `turtlefin_<version>_amd64.deb` of `_arm64.deb` — dubbelklikken, of
  `sudo apt install ./turtlefin_….deb`.
- **Android** (telefoon, tablet, tv): `Turtlefin-<version>-android-arm64.apk` (32-bits tv:
  `…-android-androidtv-armv7.apk`) — installeren uit deze bron toestaan, daarna het bestand openen.

De Windows- en AppImage-versies bevatten alles (libmpv-speler inbegrepen). Het `.deb`-pakket gebruikt de libmpv van het
systeem (`libmpv2`, automatisch geïnstalleerd door apt).

**Draagbare versie**: een bestand met de naam `portable` naast `turtlefin.exe` bewaart configuratie, accounts, cache en
downloads in de map `data` naast het programma; er wordt nergens anders iets geschreven.

### Bijwerken

**Instellingen → Over → Naar updates zoeken** vergelijkt de geïnstalleerde versie met de nieuwste, daarna regelt
**Bijwerken** alles, afhankelijk van hoe Turtlefin is geïnstalleerd:

- **Windows, geïnstalleerd**: het nieuwe installatieprogramma wordt gedownload en stil in dezelfde map uitgevoerd;
- **Windows, draagbaar**: het nieuwe archief wordt gedownload en de bestanden vervangen de oude;
- **AppImage**: het nieuwe bestand vervangt het oude;
- **.deb-pakket**: het pakket wordt geïnstalleerd met `pkexec` (het beheerderswachtwoord wordt gevraagd);
- **Flatpak**: niet automatisch — download het nieuwe `.flatpak`-bestand en installeer het op dezelfde manier.

**Turtlefin herstarten** start daarna de nieuwe versie.

## Aanbevolen serverplug-ins

Turtlefin werkt met een gewone Jellyfin-server (10.11 of nieuwer). Deze plug-ins, op de **server** geïnstalleerd,
voegen functies toe:

- [**Jellyfin Enhanced**](https://github.com/n00bcodr/Jellyfin-Enhanced): tabblad Verzoeken, Seerr-resultaten in het
  zoeken (titels, personen, studio's), Seerr-suggesties en -verzoeken op de pagina's — via de Jellyfin-aanmelding,
  zonder Seerr-sleutel in de client.
- [**Seerr**](https://github.com/seerr-team/seerr) (voorheen Jellyseerr): de verzoekbeheerder zelf, gebruikt door
  Jellyfin Enhanced.
- [**Intro Skipper**](https://github.com/intro-skipper/intro-skipper): vindt intro's en aftitelingen — knop „Intro
  overslaan”, automatisch overslaan, „Volgende aflevering” op het juiste moment.
- [**GetAvatar**](https://github.com/cedev-1/jellyfin-plugin-GetAvatar): een galerij met profielfoto's om uit te kiezen
  in Instellingen → Account.
- **Turtlefin Trailers** (in ontwikkeling, nog niet uitgebracht): downloadt ontbrekende trailers naar de server, in de
  taal van elke bibliotheek en elk account, en bereidt die van de Seerr-pagina's voor.

## Starten

- `turtlefin`: opstartanimatie, daarna „Wie kijkt er?” (of het opstartaccount).
- `turtlefin "Naam"`: opgeslagen account „Naam”.
- `turtlefin "Naam" --server=http://…`: rechtstreekse aanmelding (wachtwoord in de variabele `TURTLEFIN_PASSWORD`).
- `turtlefin --tv`: tv-interface (volledig scherm, grote elementen); `--desktop`: computerinterface.
- `turtlefin --fullscreen`: volledig scherm zonder tv-interface; `--no-intro`: zonder opstartanimatie.
- `turtlefin --sub=fr --audio=ja`: ondertitels en audiotaal voor deze start (`--sub=off`: geen).
- `turtlefin --lang=en`, `--theme=nerd`: taal en thema (daarna bewaard).
- `turtlefin "Naam" --play="Bleach S1E3"`: speelt een aflevering af („Titel” alleen: een film, of de serie hervatten);
  `--open="Dragon Ball"`: opent de pagina.
- `turtlefin --demo`: ingebouwde demo; `--theme-creator`: Turtlefin Theme Creator; `--console`: logvenster
  (Windows); `--help`: alle opties.

De opdrachtregel gaat altijd voor op de instellingen (opstartaccount, tv-interface).

## Toetsen en muis

- **Pijltjes** om te bewegen, **Enter** om te openen / activeren, **Esc** of **Backspace** om terug te gaan.
- Startpagina: **←** op de eerste kaart (of Terug) opent het menu; **↑** vanaf de bovenkant van een pagina: bovenbalk.
- Afspelen, bediening verborgen: **← →** 10 s terug / vooruit, **↑ ↓** of Enter tonen de bediening; **Spatie**:
  pauze · `a` audio · `s` ondertitels · `f` volledig scherm.
- **F11**, overal: volledig scherm (ook in Instellingen → Weergave, bewaard van de ene start tot de volgende).
- **Afstandsbediening**: Stop, volgende / vorige aflevering, vooruit / terug spoelen, Menu; OK ingedrukt houden op een
  kaart: het menu ervan (of een selectie van meerdere kaarten in een raster).
- **Muis**: klikken om te openen, rechtsklikken voor het menu van een kaart, scrollwiel om van rij naar rij te gaan
  (Shift + scrollwiel: binnen de rij).
- **Vinger**: tikken opent, vegen scrollt, vanaf de linkerrand: terug; in een raster begint lang drukken op een kaart
  een selectie van meerdere kaarten.
- **Tekst**: een letter getypt op het toetsenbord gaat meteen in het veld (zoeken, aanmelden), ook in de tv-interface.

## Bestanden

- **Configuratiemap** (`turtlefin/`): `session.json` (huidige sessie), `accounts.json` (opgeslagen accounts),
  `prefs.json` (instellingen van het apparaat), `tracks.json` (sporen per serie), `userdata.json` (offline bekeken /
  favorieten).
- **Gegevensmap** (`turtlefin/downloads/`): downloads geordend als een bibliotheek
  (`<bibliotheek>/<serie>/Season 01/<serie> S01E01.mkv`, met afbeelding en ondertitels); `queue.json` (wachtrij).
- **Cachemap** (`turtlefin/img/`): afbeeldingen en profielfoto's (te legen in Over).
- **Documenten**: `Turtlefin Languages` (toegevoegde talen), `Turtlefin Themes` (thema's), `Turtlefin Mods` (mods) en
  `Turtlefin Presets` (geëxporteerde animatie-voorinstellingen).

Onder Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`. Onder Windows:
`%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`. Draagbare versie: alles in
`data\` naast `turtlefin.exe`.

## Problemen oplossen

- `turtlefin --console` (Windows) of een start vanuit een terminal (Linux) toont het logboek.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: logboek van mpv · `TURTLEFIN_MPV_ARGS="…"`: extra mpv-opties.
- `TURTLEFIN_HWDEC=no`: softwarematige decodering · `TURTLEFIN_AO=alsa`: geluidsuitvoer.
- Lijnen of storingen in het beeld (NVIDIA): Instellingen → Afspelen → **Hardwaredecodering** uit.
- Traag apparaat: Instellingen → Animaties → **Licht** of **Geen**, en Instellingen → Weergave → Thema →
  **Detail van het decor** → Stilstaand.
- De weergave moet OpenGL zijn (automatisch gekozen): met `SLINT_BACKEND=winit-software` geen video.

## Thema's

Instellingen → Weergave → **Thema**: **Turtlefin** (standaard: zeshoeken, paars → blauw, drijvende kleurvlekken),
**Frutiger Aero** (in de geest van Windows 7: blauwig glas, glanzende knoppen, een lagune waar het beeld van het item
door het water heen te zien is) en **Nerd** (een printplaat: de server is de processor, de stroom loopt door de sporen)
— of een eigen thema. Elk decor volgt het tijdstip (dag, schemering, nacht, dageraad), of blijft altijd dag of nacht.

- **Thema maken** opent de *Turtlefin Theme Creator* (apart venster, ook in het Startmenu): elke kleur, de afronding, de
  glansranden, de achtergrond (afbeelding, GIF of video), het logo en een muziekje, met live voorbeeld. **Opslaan**
  schrijft het thema naar de map **Turtlefin Themes**; **Installeren** voegt het toe en kiest het.
- **Thema importeren** voegt een ontvangen thema toe (de map of het `.tftheme`-bestand); **Thema exporteren** schrijft
  het huidige thema als vertrekpunt naar die map.

## Turtlefin vertalen

Zonder programmeren of compileren. Toegevoegde talen staan in één map, **Turtlefin Languages**: in **Documenten**
(geïnstalleerde versie) of naast `turtlefin.exe` (draagbare versie).

1. **Instellingen → Weergave → Taal toevoegen** opent een verkenner van die map; **Sjabloon maken** schrijft daar
   `modele.po`, met de teksten in het Engels en als notitie het oorspronkelijke Frans.
2. Maak er een kopie van met de naam `<code>.po` (`sv.po` voor Zweeds, `ja.po` voor Japans…) en vul elke `msgstr ""`
   in met de vertaling van de Engelse `msgid` erboven. Behoud de `{}` en `{n}`. Vul ook `X-Language-Name` (getoonde
   naam) in en zo nodig `Plural-Forms`. Elke `.po`-editor volstaat, bijvoorbeeld [Poedit](https://poedit.net).
3. Opnieuw **Taal toevoegen**: de vertaling verschijnt met het vertaalde deel; kiezen past haar toe. Leeg gelaten
   teksten verschijnen in het Engels.

Om haar met iedereen te delen: een pull request die het bestand toevoegt aan `lang/` (en aan `BUILTIN` in
`src/i18n.rs`); `python tools/lang-check.py` controleert dat er niets ontbreekt.

## Ontwikkeling

Zie [HANDOFF.nl.md](HANDOFF.nl.md) (ook in het [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): stand van het project, beslissingen, compileren, pakketten, publiceren, vertalingen,
bekende problemen.

## Dankwoord

Dank aan [LainX84](https://github.com/LainX84): de uitgebreide suggesties
([issue #2](https://github.com/Xelopteryx/Turtlefin/issues/2)) hebben een groot deel van 2.0 gevormd — een A–Z-index
en een goed zichtbare schuifbalk voor grote bibliotheken, filteren door de eerste letters van een titel te typen,
verend scrollen, kaarten die maar één keer per sessie worden uitgedeeld (met een instelling om de animatie uit te
zetten) en het kaartmenu (bekeken / niet bekeken om opnieuw te beginnen, Identificeren, Vernieuwen, Titel bewerken).
Bedankt dat u de tijd nam om ze op te schrijven!

## Licentie

Turtlefin wordt uitgebracht onder de [GNU GPL v3](LICENSE) (of een latere versie): iedereen mag het gebruiken,
bestuderen, aanpassen en verspreiden, zolang dezelfde licentie behouden blijft en de code van de wijzigingen gedeeld
wordt. De schermafbeeldingen tonen vrije films van de Blender Foundation (CC BY). Het lettertype Turtlefin Symbols is
afgeleid van DejaVu Sans (licentie in `ui/fonts/DejaVu-LICENSE.txt`).
