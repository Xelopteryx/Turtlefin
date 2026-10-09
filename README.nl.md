<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Logo van Turtlefin"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · **Nederlands** · [Polski](README.pl.md) · [Português](README.pt.md)

**Een native, lichte en geanimeerde Jellyfin-client, gemaakt voor de woonkamer én het bureau.** Geschreven in Rust
met Slint (interface) en libmpv (afspelen): geen Qt, geen ingebouwde browser. Hij draait op elke Windows- of
Linux-computer, van een oude laptop tot een klein kastje aan de tv, en werkt even goed met een afstandsbediening
als met toetsenbord en muis.

Huidige versie: **1.0.0** · Talen van de interface: Nederlands, English, Français, Español, Deutsch, Italiano,
Português, Polski — en elke taal die je zelf toevoegt (zie [Turtlefin vertalen](#turtlefin-vertalen)).

## Wat hij kan

- **Accounts**: scherm „Wie kijkt er?” met profielfoto's (ook geanimeerde GIF's, in de cache), tot
  12 accounts opgeslagen op het apparaat (alleen het token, nooit het wachtwoord), van account wisselen zonder
  opnieuw te typen, „Accounts beheren” om er te verwijderen. Servers zoeken op alle netwerken van het apparaat;
  een hoofdadres en een reserveadres, dat wordt geprobeerd als het hoofdadres niet antwoordt.
- **Opstarten**: een geanimeerd logo waarvan de zeven punten echte controles zijn (taal, weergave,
  videospeler, opslag, configuratie, netwerk en in het midden de server), daarna „Wie kijkt er?” — of meteen het
  account dat gekozen is via Instellingen → Account → **Dit account openen bij het starten**.
- **Startpagina**: Mijn media, Verder kijken, Volgende, Recent toegevoegd; tabbladen Favorieten en Verzoeken
  (Seerr). Posters met resterende afleveringen, vinkje „gezien” en beoordeling; achtergrond van het geselecteerde
  item.
- **Detailpagina's**: film, serie, seizoen, aflevering; afspelen, favoriet, gezien, downloaden, keuze van audio /
  ondertitels onthouden voor de hele serie; „Meer zoals dit” en Seerr-suggesties; ontbrekende seizoenen
  aanvragen.
- **Afspelen** (libmpv): hoofdstukken, afleveringen van het seizoen, „Intro overslaan”, volgende aflevering,
  suggesties aan het einde van een serie, eigen volume van Turtlefin; positie en „gezien” worden naar de server
  gestuurd.
- **Watch party** (SyncPlay): op meerdere apparaten tegelijk hetzelfde kijken.
- **Offline**: downloads vervangen de startpagina, met volledige detailpagina's zonder server; wat offline is
  gekeken of als favoriet is gemarkeerd, gaat bij het opnieuw verbinden naar het account.
- **Zoeken** (bibliotheek + Seerr) en een willekeurig item.
- **Afstandsbediening, toetsenbord en muis overal**: tv-interface (grote elementen, schermtoetsenbord) of
  computerinterface (venster of volledig scherm, F11), klikken en scrollen op elke pagina, en een echt
  toetsenbord typt in beide modi direct in de tekstvelden.
- **Instellingen**: profielfoto (GetAvatar-avatars per categorie), taal van de interface, audio- en
  ondertiteltalen, grootte van de ondertitels, automatische volgende aflevering en intro, tv-interface, volledig
  scherm, achtergrond, beoordelingen, klok, serveradressen, afbeeldingscache, **update via GitHub**.
- **Animaties** overal (poster die naar de detailpagina vliegt, schuivend menu, rijen in cascade, geanimeerd
  aanmelden, taalwissel), één voor één in te stellen via Instellingen → Animaties, met presets (Alle, Licht, Geen)
  en eigen presets, die je kunt exporteren en importeren.
- **Rondleiding**: aangeboden bij de eerste start, opnieuw te bekijken via Instellingen → Over.

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

Op Windows wordt het nieuwste installatieprogramma gedownload en voor jouw gebruikersaccount uitgevoerd (zonder
beheerdersrechten). Op Debian, Ubuntu, Linux Mint en andere distributies met apt wordt het `.deb`-pakket
geïnstalleerd (je wachtwoord wordt gevraagd); elders komt de AppImage in `~/.local/bin`, met een item in het
toepassingenmenu.

### Met de hand

| Systeem | Bestand | Opmerkingen |
|---|---|---|
| Windows 64-bit | `Turtlefin-<version>-windows-x64-setup.exe` | Installatieprogramma: taal, map en modus **geïnstalleerd** (Startmenu, verwijderen) of **draagbaar** |
| Windows 64-bit, zonder installatie | `Turtlefin-<version>-windows-x64-portable.zip` | Uitpakken waar je wilt (usb-stick…) en `turtlefin.exe` starten |
| Windows 32-bit | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Voor oude pc's |
| Linux, alle distributies (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Uitvoerbaar maken (`chmod +x`) en starten |
| Linux, ARM 64-bit | `Turtlefin-<version>-linux-aarch64.AppImage` | Idem |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Dubbelklikken, of `sudo apt install ./turtlefin_….deb` |

De Windows-versies en de AppImage bevatten alles (libmpv-speler inbegrepen). Het `.deb`-pakket gebruikt de libmpv
van het systeem (`libmpv2`, automatisch geïnstalleerd door apt). AppImages en pakketten vereisen een distributie
uit 2022 of nieuwer (Ubuntu 22.04, Debian 12…).

**Draagbare versie**: een bestand `portable` naast `turtlefin.exe` zorgt dat configuratie, accounts, cache en
downloads in de map `data` naast het programma blijven; er wordt nergens anders iets geschreven.

### Bijwerken

**Instellingen → Over → Naar updates zoeken** vergelijkt de geïnstalleerde versie met de laatst gepubliceerde;
daarna regelt **Bijwerken** alles, afhankelijk van hoe Turtlefin is geïnstalleerd:

| Installatie | Bijwerken |
|---|---|
| Windows, geïnstalleerd | het nieuwe installatieprogramma wordt gedownload en stil in dezelfde map uitgevoerd |
| Windows, draagbaar | het nieuwe archief wordt gedownload en de bestanden vervangen de oude |
| AppImage | het nieuwe bestand vervangt het oude |
| .deb-pakket | het pakket wordt geïnstalleerd met `pkexec` (het beheerderswachtwoord wordt gevraagd) |

**Turtlefin herstarten** start daarna de nieuwe versie.

## Aanbevolen serverplugins

Turtlefin werkt met een gewone Jellyfin-server (10.11 of nieuwer). Deze plugins, te installeren op de **server**,
voegen functies toe:

| Plugin | Wat het Turtlefin oplevert |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Tabblad Verzoeken, Seerr-resultaten bij het zoeken, Seerr-suggesties en -verzoeken op detailpagina's — via de Jellyfin-aanmelding, zonder Seerr-sleutel in de client |
| [Seerr](https://github.com/seerr-team/seerr) (voorheen Jellyseerr) | De verzoekbeheerder zelf, gebruikt door Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Herkent intro's en aftiteling: knop „Intro overslaan”, automatisch overslaan, „Volgende aflevering” op het juiste moment |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | Een galerij met profielfoto's om uit te kiezen via Instellingen → Account |

## Starten

```
turtlefin                                  startanimatie, dan „Wie kijkt er?” (of het startaccount)
turtlefin "Naam"                           opgeslagen account „Naam”
turtlefin "Naam" --server=http://…         direct aanmelden (wachtwoord: variabele TURTLEFIN_PASSWORD=…)
turtlefin --tv                             tv-interface: volledig scherm, grote elementen
turtlefin --desktop                        computerinterface (gaat voor de instelling „Tv-interface”)
turtlefin --no-intro                       geen startanimatie
turtlefin --console                        logvenster (Windows)
turtlefin --tutorial                       rondleiding bij het openen van de startpagina
```

De opdrachtregel gaat altijd voor de instellingen (startaccount, tv-interface).

## Toetsen en muis

- **Pijltjes** om te bewegen, **Enter** om te openen / activeren, **Esc** of **Backspace** om terug te gaan
  (ingedrukt gehouden: maar één stap terug).
- Startpagina: **←** op de eerste kaart (of Terug) opent het menu; in het menu sluit **→** of Esc het.
- **↑** vanaf de bovenkant van een pagina: bovenbalk (terug, start, menu, watch party, willekeurig, zoeken,
  account).
- Afspelen, bediening verborgen: **← →** 10 s terug / vooruit, **↑ ↓** of Enter tonen de bediening, ↓ vanaf de
  knoppen: afleveringen van het seizoen. **Spatie**: pauze · `a` audio · `s` ondertitels · `f` volledig scherm.
- **F11**, overal: volledig scherm (ook via Instellingen → Weergave → **Volledig scherm**, buiten de tv-interface
  onthouden tussen starts).
- **Muis**: klikken om te openen, scrollwiel om van rij naar rij te gaan (Shift + scrollwiel: binnen de rij).
- **Tekst**: een letter die je op het toetsenbord typt, komt direct in het veld (zoeken, aanmelden, adres), ook in
  de tv-interface; daar opent een klik op het zoekveld het schermtoetsenbord.

## Bestanden

| Waar | Wat |
|---|---|
| configuratiemap, `turtlefin/` | `session.json` (huidige sessie), `accounts.json` (opgeslagen accounts), `prefs.json` (apparaatinstellingen), `tracks.json` (sporen per serie), `userdata.json` (offline gezien / favorieten) |
| gegevensmap, `turtlefin/downloads/` | downloads (media, posters, achtergrond, logo, `info.json`), `queue.json` (wachtrij) |
| cachemap, `turtlefin/img/` | afbeeldingen en profielfoto's (te legen via Over) |
| **Documenten** | `Turtlefin Languages` (toegevoegde talen) en `Turtlefin Presets` (geëxporteerde animatiepresets) |

Op Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Op Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Draagbare versie: alles in `data\` naast `turtlefin.exe` (`config`, `cache`, `downloads`), en de mappen voor talen
en presets naast het programma.

## Bij problemen

- `turtlefin --console` (Windows) of starten vanuit een terminal (Linux) toont het logboek.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: logboek van mpv · `TURTLEFIN_MPV_ARGS="…"`: extra mpv-opties.
- `TURTLEFIN_HWDEC=auto-copy`: hardwaredecodering (standaard software op Linux ARM) · `TURTLEFIN_AO=alsa`: geluidsuitvoer.
- `TURTLEFIN_LIBMPV=pad`: andere locatie van libmpv · `TURTLEFIN_DEBUG_FRAMES=1`: meldt trage beelden.
- De weergave moet OpenGL zijn (automatisch gekozen): met `SLINT_BACKEND=winit-software` is er geen video.

## Thema's

Instellingen → Weergave → **Thema**: Turtlefin (standaard), Donker, Licht, Frutiger Aero (in de geest van Windows 7:
achtergrond „Harmony”, blauwig glas, glanzende gelknoppen, speler met bol, opstarten met glazen bollen), Turtlefin groen — of een eigen thema.

- **Thema maken** opent de *Turtlefin Theme Creator* in de browser: elke kleur, de afronding, de glans, de lucht en
  de bellen, met een live voorbeeld; hij bewaart een `.tftheme`-bestand.
- Zet dat bestand in de map **Turtlefin Themes** (Documenten, of naast `turtlefin.exe` in de draagbare versie) en
  kies dan **Thema importeren**. **Thema exporteren** schrijft het huidige thema in die map, als vertrekpunt.
- De themamaker staat ook in de repository, `tools/theme-creator.html`: één bestand dat offline werkt.

## Turtlefin vertalen

Zonder programmeren of compileren. Toegevoegde talen staan in één map, **Turtlefin Languages**: in
**Documenten** (geïnstalleerde versie) of naast `turtlefin.exe` (draagbare versie).

1. **Instellingen → Weergave → Taal toevoegen** opent een verkenner van die map; **Sjabloon maken** schrijft er
   `modele.po` in, met de teksten in het Engels en als notitie het oorspronkelijke Frans en de huidige taal.
2. Maak er een kopie van met de naam `<code>.po` (`sv.po` voor Zweeds, `ja.po` voor Japans…), in die map of een
   submap, en vul elke `msgstr ""` met de vertaling van de Engelse `msgid` erboven. Behoud de `{}` en `{n}`. Vul
   ook `X-Language-Name` (weergegeven naam) in en zo nodig `Plural-Forms` (gettext-regel van de taal). Elke
   `.po`-editor is geschikt, bijvoorbeeld [Poedit](https://poedit.net).
3. Opnieuw **Taal toevoegen**: de vertaling verschijnt met het deel dat al vertaald is; kiezen past haar toe.
   Leeg gelaten teksten worden in het Engels getoond.

Om haar met iedereen te delen: een pull request die het bestand toevoegt aan `lang/` (en aan `BUILTIN` in
`src/i18n.rs`); `python tools/lang-check.py` controleert dat er niets ontbreekt.

## Ontwikkeling

Zie [HANDOFF.nl.md](HANDOFF.nl.md) (ook in het [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): stand van het project, beslissingen, compileren, pakketten, publiceren,
vertalingen, bekende problemen.
