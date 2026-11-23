<p align="center"><img src="packaging/icons/turtlefin-aero-256.png" width="96" alt="Turtlefin-Logo (Design Frutiger Aero)">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin-Logo">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-nerd-256.png" width="96" alt="Turtlefin-Logo (Design Nerd)"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · **Deutsch** · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**Ein nativer, schlanker, animierter Jellyfin-Client, gemacht für das Wohnzimmer wie für den Schreibtisch.**
Geschrieben in Rust mit Slint (Oberfläche) und libmpv (Wiedergabe): kein Qt, kein eingebetteter Browser. Er läuft auf
jedem Windows- oder Linux-Rechner, vom alten Laptop bis zur kleinen Box am Fernseher, und lässt sich mit der
Fernbedienung ebenso bedienen wie mit Tastatur und Maus — und auf Android (Telefon, Tablet, Fernseher) mit dem Finger.

> **Vollständig „vibecodiertes“ Projekt**: Der gesamte Code, die Texte und die Übersetzungen wurden von einer KI
> (Claude, von Anthropic) geschrieben, angeleitet und getestet von einem Menschen, der selbst keinen Code geschrieben hat.

Aktuelle Version: **2.0.0** · Sprachen der Oberfläche: Français, English, Español, Deutsch, Italiano, Português,
Polski, Nederlands — und jede Sprache, die Sie selbst hinzufügen (siehe [Turtlefin übersetzen](#turtlefin-übersetzen)).

> **Zu dieser Version.** 2.0.0 wurde unter Windows, auf einem Raspberry Pi 5 am Fernseher, unter Linux und auf
> Android (Telefon, TV-Emulator) getestet. Einiges ist noch zu verbessern (siehe die Hinweise der
> [Releases](https://github.com/Xelopteryx/Turtlefin/releases) und die bekannten Probleme in
> [HANDOFF.de.md](HANDOFF.de.md)). Rückmeldungen sind in den [Issues](https://github.com/Xelopteryx/Turtlefin/issues)
> willkommen.

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="100%" alt="Startseite von Turtlefin, Design Turtlefin"></p>

## Bildschirmfotos

Die drei Designs, jedes mit seiner eigenen animierten Kulisse für Tag und Nacht (hier auf einem Testserver mit freien
Filmen der Blender Foundation):

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="32%" alt="Design Turtlefin">&nbsp;<img src="docs/screenshots/en-home-aero.jpg" width="32%" alt="Design Frutiger Aero">&nbsp;<img src="docs/screenshots/en-home-nerd.jpg" width="32%" alt="Design Nerd"></p>

Die Seite eines Films, der Abspann-Bildschirm (was Sie gerade beendet haben und was als Nächstes kommt) und
„Wer schaut?“:

<p align="center"><img src="docs/screenshots/en-detail.jpg" width="32%" alt="Seite eines Films">&nbsp;<img src="docs/screenshots/en-end-screen.jpg" width="32%" alt="Abspann-Bildschirm">&nbsp;<img src="docs/screenshots/en-who-is-watching.jpg" width="32%" alt="Wer schaut?"></p>

Dieselbe App auf einem Telefon:

<p align="center"><img src="docs/screenshots/en-phone-home.jpg" width="24%" alt="Startseite auf einem Telefon">&nbsp;<img src="docs/screenshots/en-phone-detail.jpg" width="24%" alt="Filmseite auf einem Telefon">&nbsp;<img src="docs/screenshots/en-phone-settings.jpg" width="24%" alt="Einstellungen auf einem Telefon"></p>

## Was es kann

- **Konten**: Bildschirm „Wer schaut?“ mit Profilbildern (animierte GIFs inklusive), bis zu 12 auf dem Gerät
  gespeicherte Konten (nur das Token, nie das Passwort), optionaler Code pro Konto, Kontowechsel ohne erneute
  Eingabe. Serversuche in allen Netzen des Geräts; eine Haupt- und eine Ersatzadresse, die versucht wird, wenn die
  Hauptadresse nicht antwortet.
- **Start**: ein animiertes Logo, dessen sieben Punkte echte Prüfungen sind (Sprache, Anzeige, Videoplayer,
  Speicher, Konfiguration, Netzwerk und in der Mitte der Server), dann „Wer schaut?“ — oder direkt das gewählte
  Konto mit Einstellungen → Konto → **Dieses Konto beim Start öffnen**.
- **Startseite**: Banner, Meine Medien, Weiterschauen, Als Nächstes, Zuletzt hinzugefügt; Reiter Favoriten,
  Merkliste und Anfragen (Seerr). Poster mit verbleibenden Folgen, Häkchen „gesehen“ und Bewertung; der Hintergrund
  stammt vom ausgewählten Medium.
- **Medienseiten**: Film, Serie, Staffel, Folge; Wiedergabe, Favorit, gesehen, Download, Audio- und
  Untertitelwahl für die ganze Serie gemerkt; Bewertung, Dauer, Erscheinungsdatum und bei einer Serie oder Staffel
  die **gesehenen Folgen** („8 / 24“ und der Prozentsatz); Reiter Details (Genres, Studios, Personen, Audiosprachen
  mit Flagge); „Mehr davon“ und Seerr-Vorschläge; fehlende Staffeln anfragen.
- **Wiedergabe** (libmpv): Kapitel, Folgen der Staffel, „Intro überspringen“, nächste Folge, maximale Bitrate
  (darüber wandelt der Server das Video um), eigene Lautstärke; ein animierter **Abspann-Bildschirm**: was Sie
  gerade beendet haben und drei Vorschläge mit der Zusammenfassung des gewählten; Position und „gesehen“ gehen an
  den Server zurück.
- **Suche** in der Bibliothek und in Seerr: Titel, Personen (mit ihrer Filmografie, die auf dem Server fehlt) und
  **Studios oder Sender** („disney“, „pixar“, „netflix“: ihre Filme und Serien zum Anfragen); Filter, erweiterte
  Suche, Zufallsmedium.
- **Mehrere Karten auswählen** in Rastern (langes Drücken, Ziehen mit der Maus, OK gedrückt halten auf der
  Fernbedienung), dann alle auf einmal anfragen oder herunterladen.
- **Watch Party** (SyncPlay): auf mehreren Geräten gleichzeitig dasselbe schauen.
- **Android** (Telefon, Tablet, Android TV): dieselbe App, hoch und quer, mit dem Finger (Wischen vom linken Rand:
  zurück), Lautstärke des Telefons, Fernbedienung und Gamepad von Android TV.
- **Übertragen**: ein Video an einen anderen Bildschirm senden — ein anderes Turtlefin, eine Jellyfin-App des Kontos
  oder ein Chromecast — und es vom Player aus steuern. **Fernbedienung**: ein anderes Turtlefin des Kontos steuern.
- **Seerr-Seiten**: ein Medium, das nicht auf dem Server ist, öffnet sich auf einer eigenen Seite (Zusammenfassung,
  Informationen, Besetzung), mit Anfragen und **Trailer**.
- **Designs**: Turtlefin, Frutiger Aero und Nerd, jedes mit eigenem Logo, Start und animierter Kulisse, die der
  Tageszeit folgt; ein eigenes Design (siehe [Designs](#designs)). Offline wird die ganze Oberfläche bernsteinfarben.
- **Offline**: Downloads ersetzen die Startseite, mit vollständigen Medienseiten ohne Server; was offline gesehen
  oder favorisiert wurde, geht beim erneuten Verbinden an das Konto.
- **Fernbedienung, Tastatur, Maus und Finger überall**: TV-Oberfläche (große Elemente, Bildschirmtastatur) oder
  Computer-Oberfläche (Fenster oder Vollbild, F11).
- **Einstellungen** nach Kategorien, mit Suche: Profilbild (GetAvatar), Sprachen, Untertitel, Wiedergabe, Anzeige,
  Textgröße, Netzwerk, Speicher, Animationen, Mods, **Update von GitHub**.
- **Animationen** überall (Poster fliegt zu seiner Seite, Banner öffnet sich, Reihen im Kaskadeneffekt, Übergänge
  zwischen Seiten), einzeln einstellbar, mit Voreinstellungen (Alle, Leicht, Keine).
- **Flüssigkeit**: Turtlefin misst, was das Gerät leistet, und vereinfacht seine Kulisse von selbst, um 60 Bilder
  pro Sekunde zu halten.
- Eingebaute **Tour** und **Demo** („Turtlefin entdecken“ beim ersten Start), mit echten freien Filmen; nichts wird
  gespeichert.
- **Discord-Status** (standardmäßig aus); **Administratoren**: ein Medium auf seiner Seite bearbeiten, Dashboard.

## Systemanforderungen

Gemessen auf einem Raspberry Pi 5, einem Einsteigertelefon (Galaxy A12s) und einem gedrosselten Android-Emulator
(ein einziger Kern, auf 30 % begrenzt: benutzbar, Animationen mit 15–50 Bildern pro Sekunde; derselbe Kern ohne
Begrenzung: Animationen mit 60 Bildern pro Sekunde).

**Mindestens** — alles funktioniert, Animationen aus (Einstellungen → Animationen → **Keine**); die Kulisse bleibt
von selbst stehen, wenn das Gerät nicht mitkommt:

- 64-Bit-Prozessor mit 4 Kernen um 1,5 GHz (Cortex-A53 / A55 eines Fernsehers oder Einsteigertelefons, aktueller
  Intel Celeron oder Atom);
- 2 GB Arbeitsspeicher (Turtlefin belegt 200 bis 500 MB);
- Grafikbeschleunigung OpenGL (Windows, Linux auf dem PC) oder OpenGL ES (Android, Raspberry Pi);
  Hardware-Videodekodierung empfohlen;
- Windows 10 oder 11, Linux ab 2022 (Ubuntu 22.04, Debian 12…), Android 8 oder neuer.

**Empfohlen** — alle Animationen mit 60 Bildern pro Sekunde:

- aktueller Prozessor mit 4 Kernen um 2 GHz oder mehr (Cortex-A76 des Raspberry Pi 5, Intel Core i3 der
  8. Generation, AMD Ryzen 3 oder besser);
- 4 GB Arbeitsspeicher;
- Hardware-Dekodierung HEVC 10 Bit für 4K.

## Installieren

Nichts zu kompilieren: herunterladen, installieren, fertig. Alle Dateien sind auf der Seite
[Releases](https://github.com/Xelopteryx/Turtlefin/releases).

### Mit einem Befehl

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (Terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

Unter Windows wird das neueste Installationsprogramm heruntergeladen und für Ihr Benutzerkonto ausgeführt (ohne
Administratorrechte). Unter Debian, Ubuntu, Linux Mint und den anderen apt-Distributionen wird das `.deb`-Paket
installiert (Ihr Passwort wird abgefragt); anderswo kommt das AppImage nach `~/.local/bin`, mit einem Eintrag im
Anwendungsmenü.

### Von Hand

- **Windows 64 Bit**: `Turtlefin-<version>-windows-x64-setup.exe` — Installationsprogramm: Sprache, Ordner und Modus
  **installiert** (Startmenü, Deinstallation) oder **portabel**.
- **Windows 64 Bit, ohne Installation**: `Turtlefin-<version>-windows-x64-portable.zip` — beliebig entpacken
  (USB-Stick…), dann `turtlefin.exe` starten.
- **Windows 32 Bit** (alte PCs): `…-windows-x86-setup.exe` oder `…-windows-x86-portable.zip`.
- **Linux, alle Distributionen (x86_64)**: `Turtlefin-<version>-linux-x86_64.AppImage` — ausführbar machen
  (`chmod +x`), dann starten. ARM 64 Bit: `…-linux-aarch64.AppImage`.
- **Linux mit Flatpak (x86_64)**: `Turtlefin-<version>-linux-x86_64.flatpak` —
  `flatpak install --user ./Turtlefin-….flatpak` (die Laufzeit kommt von Flathub).
- **Debian, Ubuntu, Mint…**: `turtlefin_<version>_amd64.deb` oder `_arm64.deb` — Doppelklick oder
  `sudo apt install ./turtlefin_….deb`.
- **Android** (Telefon, Tablet, Fernseher): `Turtlefin-<version>-android-arm64.apk` (32-Bit-Fernseher:
  `…-android-androidtv-armv7.apk`) — Installation aus dieser Quelle erlauben, dann die Datei öffnen.

Die Windows- und AppImage-Versionen enthalten alles (libmpv-Player inklusive). Das `.deb`-Paket nutzt die libmpv des
Systems (`libmpv2`, von apt automatisch installiert).

**Portable Version**: Eine Datei namens `portable` neben `turtlefin.exe` hält Konfiguration, Konten, Cache und
Downloads im Ordner `data` neben dem Programm; nirgends sonst wird etwas geschrieben.

### Aktualisieren

**Einstellungen → Über → Nach Updates suchen** vergleicht die installierte Version mit der neuesten, dann erledigt
**Aktualisieren** alles je nach Art der Installation:

- **Windows, installiert**: das neue Installationsprogramm wird heruntergeladen und still im selben Ordner ausgeführt;
- **Windows, portabel**: das neue Archiv wird heruntergeladen und seine Dateien ersetzen die alten;
- **AppImage**: die neue Datei ersetzt die alte;
- **.deb-Paket**: das Paket wird mit `pkexec` installiert (das Administratorpasswort wird abgefragt);
- **Flatpak**: nicht automatisch — laden Sie die neue `.flatpak`-Datei herunter und installieren Sie sie ebenso.

**Turtlefin neu starten** startet dann die neue Version.

## Empfohlene Server-Plugins

Turtlefin funktioniert mit einem einfachen Jellyfin-Server (10.11 oder neuer). Diese Plugins, auf dem **Server**
installiert, fügen Funktionen hinzu:

- [**Jellyfin Enhanced**](https://github.com/n00bcodr/Jellyfin-Enhanced): Reiter Anfragen, Seerr-Ergebnisse in der
  Suche (Titel, Personen, Studios), Seerr-Vorschläge und -Anfragen auf den Medienseiten — über die Jellyfin-Anmeldung,
  ohne Seerr-Schlüssel im Client.
- [**Seerr**](https://github.com/seerr-team/seerr) (früher Jellyseerr): die Anfrageverwaltung selbst, von Jellyfin
  Enhanced genutzt.
- [**Intro Skipper**](https://github.com/intro-skipper/intro-skipper): erkennt Intros und Abspann — Knopf „Intro
  überspringen“, automatisches Überspringen, „Nächste Folge“ im richtigen Moment.
- [**GetAvatar**](https://github.com/cedev-1/jellyfin-plugin-GetAvatar): eine Galerie von Profilbildern zur Auswahl
  in Einstellungen → Konto.
- **Turtlefin Trailers** (in Entwicklung, noch nicht veröffentlicht): lädt fehlende Trailer auf den Server herunter,
  in der Sprache jeder Bibliothek und jedes Kontos, und bereitet die der Seerr-Seiten vor.

## Starten

- `turtlefin`: Startanimation, dann „Wer schaut?“ (oder das Startkonto).
- `turtlefin "Name"`: gespeichertes Konto „Name“.
- `turtlefin "Name" --server=http://…`: direkte Anmeldung (Passwort in der Variable `TURTLEFIN_PASSWORD`).
- `turtlefin --tv`: TV-Oberfläche (Vollbild, große Elemente); `--desktop`: Computer-Oberfläche.
- `turtlefin --fullscreen`: Vollbild ohne TV-Oberfläche; `--no-intro`: ohne Startanimation.
- `turtlefin --sub=fr --audio=ja`: Untertitel und Audiosprache für diesen Start (`--sub=off`: keine).
- `turtlefin --lang=en`, `--theme=nerd`: Sprache und Design (danach beibehalten).
- `turtlefin "Name" --play="Bleach S1E3"`: spielt eine Folge ab („Titel“ allein: ein Film oder die Serie
  fortsetzen); `--open="Dragon Ball"`: öffnet die Seite.
- `turtlefin --demo`: eingebaute Demo; `--theme-creator`: Turtlefin Theme Creator; `--console`: Protokollfenster
  (Windows); `--help`: alle Optionen.

Die Befehlszeile hat immer Vorrang vor den Einstellungen (Startkonto, TV-Oberfläche).

## Tasten und Maus

- **Pfeile** zum Bewegen, **Enter** zum Öffnen / Aktivieren, **Esc** oder **Rücktaste** für zurück.
- Startseite: **←** auf der ersten Karte (oder Zurück) öffnet das Menü; **↑** vom oberen Rand einer Seite: obere Leiste.
- Wiedergabe, Steuerung ausgeblendet: **← →** 10 s zurück / vor, **↑ ↓** oder Enter zeigen die Steuerung;
  **Leertaste**: Pause · `a` Audio · `s` Untertitel · `f` Vollbild.
- **F11**, überall: Vollbild (auch in Einstellungen → Anzeige, bleibt von einem Start zum nächsten).
- **Fernbedienung**: Stopp, nächste / vorherige Folge, Vor- / Rückspulen, Menü; OK gedrückt halten auf einer Karte:
  ihr Menü (oder eine Auswahl mehrerer Karten in einem Raster).
- **Maus**: Klick zum Öffnen, Rechtsklick für das Menü einer Karte, Mausrad von Reihe zu Reihe (Umschalt + Mausrad:
  innerhalb der Reihe).
- **Finger**: Tippen öffnet, Wischen scrollt, vom linken Rand: zurück; in einem Raster beginnt langes Drücken auf
  einer Karte eine Auswahl mehrerer Karten.
- **Text**: ein auf der Tastatur getippter Buchstabe geht direkt ins Feld (Suche, Anmeldung), auch in der TV-Oberfläche.

## Dateien

- **Konfigurationsordner** (`turtlefin/`): `session.json` (aktuelle Sitzung), `accounts.json` (gespeicherte Konten),
  `prefs.json` (Geräteeinstellungen), `tracks.json` (Spuren pro Serie), `userdata.json` (offline gesehen / favorisiert).
- **Datenordner** (`turtlefin/downloads/`): Downloads wie eine Bibliothek geordnet
  (`<Bibliothek>/<Serie>/Season 01/<Serie> S01E01.mkv`, mit Bild und Untertiteln); `queue.json` (Warteschlange).
- **Cache-Ordner** (`turtlefin/img/`): Bilder und Profilbilder (in Über leerbar).
- **Dokumente**: `Turtlefin Languages` (hinzugefügte Sprachen), `Turtlefin Themes` (Designs), `Turtlefin Mods` (Mods)
  und `Turtlefin Presets` (exportierte Animations-Voreinstellungen).

Unter Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`. Unter Windows:
`%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`. Portable Version: alles in
`data\` neben `turtlefin.exe`.

## Fehlerbehebung

- `turtlefin --console` (Windows) oder ein Start aus einem Terminal (Linux) zeigt das Protokoll.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: mpv-Protokoll · `TURTLEFIN_MPV_ARGS="…"`: zusätzliche mpv-Optionen.
- `TURTLEFIN_HWDEC=no`: Software-Dekodierung · `TURTLEFIN_AO=alsa`: Audioausgabe.
- Linien oder Störungen im Bild (NVIDIA): Einstellungen → Wiedergabe → **Hardware-Dekodierung** aus.
- Langsames Gerät: Einstellungen → Animationen → **Leicht** oder **Keine**, und Einstellungen → Anzeige → Design →
  **Detail der Kulisse** → Unbewegt.
- Das Rendering muss OpenGL sein (automatisch gewählt): mit `SLINT_BACKEND=winit-software` kein Video.

## Designs

Einstellungen → Anzeige → **Design**: **Turtlefin** (Standard: Sechsecke, Violett → Blau, treibende Farbflecken),
**Frutiger Aero** (im Geist von Windows 7: bläuliches Glas, glänzende Knöpfe, eine Lagune, durch deren Wasser das Bild
des Mediums zu sehen ist) und **Nerd** (eine Platine: der Server ist der Prozessor, Strom läuft durch die Leiterbahnen)
— oder ein eigenes Design. Jede Kulisse folgt der Tageszeit (Tag, Abenddämmerung, Nacht, Morgendämmerung) oder bleibt
immer Tag oder Nacht.

- **Design erstellen** öffnet den *Turtlefin Theme Creator* (eigenes Fenster, auch im Startmenü): jede Farbe, die
  Rundung, die Glanzlichter, der Hintergrund (Bild, GIF oder Video), das Logo und eine Musik, mit Live-Vorschau.
  **Speichern** schreibt das Design in den Ordner **Turtlefin Themes**; **Installieren** fügt es hinzu und wählt es.
- **Design importieren** fügt ein erhaltenes Design hinzu (seinen Ordner oder seine `.tftheme`-Datei);
  **Design exportieren** schreibt das aktuelle Design als Ausgangspunkt in diesen Ordner.

## Turtlefin übersetzen

Ohne Programmieren, ohne Kompilieren. Hinzugefügte Sprachen liegen in einem einzigen Ordner, **Turtlefin Languages**:
in **Dokumente** (installierte Version) oder neben `turtlefin.exe` (portable Version).

1. **Einstellungen → Anzeige → Sprache hinzufügen** öffnet einen Explorer dieses Ordners; **Vorlage erstellen**
   schreibt dort `modele.po`, mit den Texten auf Englisch und dem französischen Original als Notiz.
2. Kopieren Sie sie als `<code>.po` (`sv.po` für Schwedisch, `ja.po` für Japanisch…) und füllen Sie jedes
   `msgstr ""` mit der Übersetzung des englischen `msgid` darüber. Behalten Sie die `{}` und `{n}`. Füllen Sie auch
   `X-Language-Name` (angezeigter Name) und bei Bedarf `Plural-Forms` aus. Jeder `.po`-Editor eignet sich, zum
   Beispiel [Poedit](https://poedit.net).
3. Erneut **Sprache hinzufügen**: die Übersetzung erscheint mit ihrem Fortschritt; sie auszuwählen wendet sie an.
   Leer gelassene Texte erscheinen auf Englisch.

Um sie mit allen zu teilen: ein Pull Request, der die Datei zu `lang/` hinzufügt (und zu `BUILTIN` in `src/i18n.rs`);
`python tools/lang-check.py` prüft, dass nichts fehlt.

## Entwicklung

Siehe [HANDOFF.de.md](HANDOFF.de.md) (auch auf [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): Projektstand, Entscheidungen, Kompilieren, Pakete, Veröffentlichung, Übersetzungen,
bekannte Probleme.

## Danksagung

Danke an [LainX84](https://github.com/LainX84): die ausführlichen Vorschläge
([Issue #2](https://github.com/Xelopteryx/Turtlefin/issues/2)) haben einen großen Teil von 2.0 geprägt — ein A–Z-Index
und eine gut sichtbare Bildlaufleiste für große Bibliotheken, Filtern durch Eintippen der ersten Buchstaben eines
Titels, federndes Scrollen, Karten, die nur einmal pro Sitzung ausgeteilt werden (mit einer Einstellung zum Abschalten
der Animation), und das Kartenmenü (gesehen / nicht gesehen zum Neustart, Identifizieren, Aktualisieren, Titel
bearbeiten). Vielen Dank, dass Sie sich die Zeit dafür genommen haben!

## Lizenz

Turtlefin steht unter der [GNU GPL v3](LICENSE) (oder einer späteren Version): Jeder darf es nutzen, untersuchen,
ändern und weitergeben, sofern dieselbe Lizenz beibehalten und der Code der Änderungen geteilt wird. Die
Bildschirmfotos zeigen freie Filme der Blender Foundation (CC BY). Die Schrift Turtlefin Symbols stammt aus
DejaVu Sans (Lizenz in `ui/fonts/DejaVu-LICENSE.txt`).
