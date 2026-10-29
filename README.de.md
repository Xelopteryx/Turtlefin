<p align="center"><img src="packaging/icons/turtlefin-aero-256.png" width="96" alt="Turtlefin logo (Frutiger Aero theme)">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin logo">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-nerd-256.png" width="96" alt="Turtlefin logo (Nerd theme)"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · **Deutsch** · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**Ein nativer, leichter und animierter Jellyfin-Client, fürs Wohnzimmer wie für den Schreibtisch.** Geschrieben
in Rust mit Slint (Oberfläche) und libmpv (Wiedergabe): kein Qt, kein eingebetteter Browser. Er läuft auf jedem
Windows- oder Linux-Rechner, vom alten Laptop bis zur kleinen Box am Fernseher, und lässt sich mit der
Fernbedienung genauso gut bedienen wie mit Tastatur und Maus — und auf Android (Smartphone, Tablet, Fernseher) mit dem Finger.

> **Vollständig per Vibecoding entstanden**: Code, Texte und Übersetzungen wurden komplett von einer KI
> (Claude von Anthropic) geschrieben, angeleitet und getestet von einem Menschen, der nicht selbst programmiert hat.

Aktuelle Version: **1.9.8 „Cast Update“** · Sprachen der Oberfläche: Deutsch, English, Français, Español, Italiano, Português,
Polski, Nederlands — und jede selbst hinzugefügte Sprache (siehe [Turtlefin übersetzen](#turtlefin-übersetzen)).

> **Zu dieser Version.** 1.9.7 „Cast Update“ wurde unter Windows, Linux (Raspberry Pi 5 am Fernseher) und
> Android (Smartphone) getestet. Einiges muss noch überarbeitet werden — der Entwickler weiß, dass noch
> Probleme bestehen (siehe die Hinweise zu den [Versionen](https://github.com/Xelopteryx/Turtlefin/releases)
> und die bekannten Probleme in [HANDOFF.de.md](HANDOFF.de.md)) —, aber es war Zeit, eine deutlich weiter
> entwickelte Version als 1.2.0 zu veröffentlichen. Rückmeldungen sind in den
> [Issues](https://github.com/Xelopteryx/Turtlefin/issues) willkommen.

## Was er kann

- **Konten**: Bildschirm „Wer schaut?“ mit Profilbildern (auch animierte GIFs, zwischengespeichert), bis zu
  12 auf dem Gerät gespeicherte Konten (nur das Token, nie das Passwort), Kontowechsel ohne erneute Eingabe,
  „Konten verwalten“ zum Entfernen. Serversuche in allen Netzwerken des Geräts; eine Hauptadresse und eine
  Ersatzadresse, die versucht wird, wenn die Hauptadresse nicht antwortet.
- **Start**: ein animiertes Logo, dessen sieben Punkte echte Prüfungen sind (Sprache, Anzeige, Videoplayer,
  Speicher, Konfiguration, Netzwerk und in der Mitte der Server), danach „Wer schaut?“ — oder direkt das Konto,
  das unter Einstellungen → Konto → **Dieses Konto beim Start öffnen** gewählt wurde.
- **Startseite**: Meine Medien, Weiterschauen, Als Nächstes, Kürzlich hinzugefügt; Reiter Favoriten und Anfragen
  (Seerr). Poster mit verbleibenden Folgen, Häkchen „gesehen“ und Bewertung; Hintergrund aus dem ausgewählten
  Medium.
- **Detailseiten**: Film, Serie, Staffel, Folge; Abspielen, Favorit, gesehen, Download, Audio- / Untertitelwahl
  für die ganze Serie gemerkt; „Mehr davon“ und Seerr-Vorschläge; fehlende Staffeln anfragen.
- **Wiedergabe** (libmpv): Kapitel, Folgen der Staffel, „Intro überspringen“, nächste Folge, Vorschläge am Ende
  einer Serie, eigene Lautstärke von Turtlefin, maximale Bitrate (darüber wandelt der Server das Video um); Position und „gesehen“ werden an den Server gemeldet.
- **Watch Party** (SyncPlay): auf mehreren Geräten gleichzeitig dasselbe schauen.
- **Android** (Smartphone, Tablet, Android TV): dieselbe Anwendung, hoch- wie querformat, mit dem Finger
  (Reihen, Besetzung, Episoden wischen; vom linken Rand wischen: zurück), Lautstärke des Geräts, Fernbedienung
  und Gamepad von Android TV.
- **Übertragen**: ein Video an einen anderen Bildschirm schicken — ein anderes Turtlefin, eine Jellyfin-App des
  Kontos oder einen Chromecast (mit Weiterleitung über das Gerät, wenn der Fernseher den Server nicht
  erreicht) — und es vom Player aus steuern. **Fernbedienung**: ein anderes Turtlefin des Kontos steuern.
- **Seerr-Seiten**: Medien, die nicht auf dem Server sind, öffnen eine eigene Seite (Inhalt, Informationen,
  Besetzung, „Weiterlesen“), mit Anfragen und **Trailer** (vom Plugin Turtlefin Trailers auf dem Server
  vorbereitet, in der Sprache des Kontos, höchstens 24 Stunden aufbewahrt).
- **Trailer**: Symbol auf den Detailseiten; die Trailer des Servers (in der Sprache des Kontos, wenn es mehrere
  gibt), sonst YouTube über yt-dlp, wenn es installiert ist.
- **Designs**: Turtlefin, Frutiger Aero und Nerd, jedes mit eigenem Logo, Start und animiertem Hintergrund;
  ein eigenes Design (siehe Designs unten). Offline wird die ganze Oberfläche bernsteinfarben.
- **Speicher**: Download-Ordner und Platzgrenze; **Passwortprüfung** (Warnung, wenn es zu einfach ist, Ändern in
  der App).
- **Offline**: Downloads ersetzen die Startseite, mit vollständigen Detailseiten ohne Server; offline Gesehenes
  und Favoriten werden beim nächsten Verbinden an das Konto übertragen.
- **Suche** (Bibliothek + Seerr) und Zufallsauswahl.
- **Fernbedienung, Tastatur und Maus überall**: TV-Oberfläche (große Elemente, Bildschirmtastatur) oder
  Computer-Oberfläche (Fenster oder Vollbild, F11), Klick und Mausrad auf allen Seiten, und eine echte Tastatur
  schreibt in beiden Modi direkt in die Textfelder.
- **Einstellungen**: Profilbild (GetAvatar-Avatare nach Kategorie), Sprache der Oberfläche, Audio- und
  Untertitelsprachen, Untertitelgröße, automatische nächste Folge und Intro, TV-Oberfläche, Vollbild,
  Hintergrund, Bewertungen, Uhrzeit, Serveradressen, Bild-Cache, **Update von GitHub**.
- **Animationen** überall (Poster fliegt zur Detailseite, Menü gleitet, Reihen in Kaskade, animierte Anmeldung,
  Sprachwechsel), einzeln einstellbar unter Einstellungen → Animationen, mit Presets (Alle, Leicht, Keine) und
  eigenen, die sich exportieren und importieren lassen.
- **Rundgang**: beim ersten Start angeboten, erneut unter Einstellungen → Über.

## Installieren

Nichts zu kompilieren: herunterladen, installieren, fertig. Alle Dateien stehen auf der Seite
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

Unter Windows wird das neueste Installationsprogramm heruntergeladen und für dein Benutzerkonto ausgeführt
(ohne Administratorrechte). Unter Debian, Ubuntu, Linux Mint und anderen apt-Distributionen wird das
`.deb`-Paket installiert (dein Passwort wird abgefragt); anderswo kommt das AppImage nach `~/.local/bin`, mit
einem Eintrag im Anwendungsmenü.

### Von Hand

| System | Datei | Hinweise |
|---|---|---|
| Windows 64 Bit | `Turtlefin-<version>-windows-x64-setup.exe` | Installationsprogramm: Sprache, Ordner und Modus **installiert** (Startmenü, Deinstallation) oder **portabel** |
| Windows 64 Bit, ohne Installation | `Turtlefin-<version>-windows-x64-portable.zip` | Irgendwo entpacken (USB-Stick…) und `turtlefin.exe` starten |
| Windows 32 Bit | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Für alte PCs |
| Linux, alle Distributionen (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Ausführbar machen (`chmod +x`), dann starten |
| Linux, ARM 64 Bit | `Turtlefin-<version>-linux-aarch64.AppImage` | Ebenso |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Doppelklick oder `sudo apt install ./turtlefin_….deb` |
| Android (Smartphone, Tablet, TV) | `Turtlefin-<version>-android-arm64.apk` | Installation aus dieser Quelle erlauben, dann die Datei öffnen (Android 8 oder neuer) |

Die Windows-Versionen und das AppImage enthalten alles (libmpv-Player inklusive). Das `.deb`-Paket nutzt die
libmpv des Systems (`libmpv2`, von apt automatisch installiert). AppImages und Pakete brauchen eine Distribution
von 2022 oder neuer (Ubuntu 22.04, Debian 12…).

**Portable Version**: eine Datei `portable` neben `turtlefin.exe` sorgt dafür, dass Konfiguration, Konten, Cache
und Downloads im Ordner `data` neben dem Programm bleiben; sonst wird nirgends etwas geschrieben.

### Aktualisieren

**Einstellungen → Über → Nach Updates suchen** vergleicht die installierte Version mit der zuletzt
veröffentlichten, danach erledigt **Aktualisieren** alles je nach Installationsart:

| Installation | Aktualisierung |
|---|---|
| Windows, installiert | das neue Installationsprogramm wird heruntergeladen und still im selben Ordner ausgeführt |
| Windows, portabel | das neue Archiv wird heruntergeladen und seine Dateien ersetzen die alten |
| AppImage | die neue Datei ersetzt die alte |
| .deb-Paket | das Paket wird mit `pkexec` installiert (das Administratorpasswort wird abgefragt) |

**Turtlefin neu starten** startet danach die neue Version.

## Empfohlene Server-Plugins

Turtlefin funktioniert mit einem einfachen Jellyfin-Server (10.11 oder neuer). Diese Plugins, auf dem **Server**
installiert, bringen zusätzliche Funktionen:

| Plugin | Was es Turtlefin bringt |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Reiter Anfragen, Seerr-Ergebnisse in der Suche, Seerr-Vorschläge und -Anfragen auf Detailseiten — über die Jellyfin-Anmeldung, ohne Seerr-Schlüssel im Client |
| [Seerr](https://github.com/seerr-team/seerr) (früher Jellyseerr) | Der Anfrage-Manager selbst, von Jellyfin Enhanced genutzt |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Erkennt Intros und Abspann: Schaltfläche „Intro überspringen“, automatisches Überspringen, „Nächste Folge“ im richtigen Moment |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | Eine Galerie von Profilbildern zur Auswahl unter Einstellungen → Konto |
| Turtlefin Trailers (separat veröffentlicht) | Lädt fehlende Trailer auf den Server, in der Sprache jeder Bibliothek und jedes Kontos, und bereitet die Trailer der Seerr-Seiten vor |

## Starten

```
turtlefin                                  Startanimation, dann „Wer schaut?“ (oder das Startkonto)
turtlefin "Name"                           gespeichertes Konto „Name“
turtlefin "Name" --server=http://…         direkte Anmeldung (Passwort: Variable TURTLEFIN_PASSWORD=…)
turtlefin --tv                             TV-Oberfläche: Vollbild, große Elemente
turtlefin --desktop                        Computer-Oberfläche (hat Vorrang vor der Einstellung „TV-Oberfläche“)
turtlefin --no-intro                       keine Startanimation
turtlefin --sub=fr --audio=ja             Untertitel / Audiosprache für diesen Start (--sub=off: keine)
turtlefin --fullscreen                     Vollbild ohne die TV-Oberfläche
turtlefin --console                        Protokollfenster (Windows)
turtlefin --tutorial                       Rundgang beim Öffnen der Startseite
```

Die Befehlszeile hat immer Vorrang vor den Einstellungen (Startkonto, TV-Oberfläche).

## Tasten und Maus

- **Pfeiltasten** zum Bewegen, **Eingabe** zum Öffnen / Aktivieren, **Esc** oder **Rücktaste** zum Zurückgehen
  (gedrückt gehalten: nur ein Schritt zurück).
- Startseite: **←** auf der ersten Karte (oder Zurück) öffnet das Menü; im Menü schließt **→** oder Esc es.
- **↑** vom oberen Rand einer Seite: obere Leiste (zurück, Start, Menü, Watch Party, Zufall, Suche, Konto).
- Wiedergabe, Steuerung ausgeblendet: **← →** 10 s zurück / vor, **↑ ↓** oder Eingabe zeigen die Steuerung,
  ↓ von den Schaltflächen: Folgen der Staffel. **Leertaste**: Pause · `a` Audio · `s` Untertitel · `f` Vollbild.
- **F11**, überall: Vollbild (auch unter Einstellungen → Anzeige → **Vollbild**, außerhalb der TV-Oberfläche über
  Neustarts hinweg gemerkt).
- **TV-Fernbedienung**: Stopp, nächste / vorherige Episode, Vor- / Zurückspulen, Menü (linkes Menü) und das
  Gamepad (Android, Raspberry Pi über XeLauncher) funktionieren wie die Tastatur.
- **Maus**: Klick zum Öffnen, Mausrad von Reihe zu Reihe (Umschalt + Mausrad: innerhalb der Reihe).
- **Touch und Ziehen**: Tippen öffnet; mit dem Finger wischen (oder mit gedrückter linker Maustaste ziehen) scrollt die Reihe oder die Seite, eine schnelle Geste geht weiter; vom linken Rand: zurück.
- **Text**: ein auf der Tastatur getippter Buchstabe landet direkt im Feld (Suche, Anmeldung, Adresse), auch in
  der TV-Oberfläche; dort öffnet ein Klick auf das Suchfeld die Bildschirmtastatur.

## Dateien

| Wo | Was |
|---|---|
| Konfigurationsordner, `turtlefin/` | `session.json` (aktuelle Sitzung), `accounts.json` (gespeicherte Konten), `prefs.json` (Geräteeinstellungen), `tracks.json` (Spuren pro Serie), `userdata.json` (offline Gesehenes / Favoriten) |
| Datenordner, `turtlefin/downloads/` | Downloads wie eine Bibliothek geordnet: `<Bibliothek>/<Serie>/Season 01/<Serie> S01E01.mkv` (mit `.json`, Bild und Untertiteln), Bilder der Serie geteilt; `queue.json` (Warteschlange) |
| Cache-Ordner, `turtlefin/img/` | Bilder und Profilbilder (unter Über leerbar) |
| **Dokumente** | `Turtlefin Languages` (hinzugefügte Sprachen) und `Turtlefin Presets` (exportierte Animations-Presets) |

Unter Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Unter Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Portable Version: alles in `data\` neben `turtlefin.exe` (`config`, `cache`, `downloads`), die Sprach- und
Preset-Ordner neben dem Programm.

## Bei Problemen

- `turtlefin --console` (Windows) oder Start aus einem Terminal (Linux) zeigt das Protokoll.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: mpv-Protokoll · `TURTLEFIN_MPV_ARGS="…"`: zusätzliche mpv-Optionen.
- `TURTLEFIN_HWDEC=no`: Software-Dekodierung (standardmäßig Hardware, `drm-copy` auf dem Raspberry Pi) · `TURTLEFIN_AO=alsa`: Audioausgabe.
- Linien oder Störungen im Bild (NVIDIA): Einstellungen → Wiedergabe → **Hardware-Dekodierung** aus (alles wird vom Prozessor dekodiert).
- `TURTLEFIN_LIBMPV=Pfad`: anderer Ort für libmpv · `TURTLEFIN_DEBUG_FRAMES=1`: meldet langsame Bilder.
- Die Darstellung muss OpenGL sein (automatisch gewählt): mit `SLINT_BACKEND=winit-software` gibt es kein Video.

## Designs

Einstellungen → Anzeige → **Design**: **Turtlefin** (Standard: Sechsecke, Violett → Blau), **Frutiger Aero**
(im Geist von Windows 7: Himmel und Blasen, bläuliches Glas, glänzende Schaltflächen, Kugel-Logo) und **Nerd**
(eine Platine bei Nacht: der Server ist der Prozessor, Strom fließt über die Leiterbahnen zu jedem Bauteil) —
oder ein eigenes Design. „Animierter Hintergrund“ (Einstellungen → Animationen) belebt jeden Hintergrund; auf
dem Raspberry Pi ist er standardmäßig aus.

- **Design erstellen** öffnet den *Turtlefin Theme Creator* im Browser: jede Farbe, die Rundung, der Glanz, Himmel
  und Blasen, mit Live-Vorschau; er speichert eine `.tftheme`-Datei.
- Diese Datei in den Ordner **Turtlefin Themes** legen (Dokumente, oder neben `turtlefin.exe` in der portablen
  Version), dann **Design importieren**. **Design exportieren** schreibt das aktuelle Design als Ausgangspunkt in
  diesen Ordner.
- Der Editor liegt auch im Repository, `tools/theme-creator.html`: eine einzige Datei, die offline funktioniert.

## Turtlefin übersetzen

Ohne Programmieren, ohne Kompilieren. Hinzugefügte Sprachen liegen in einem einzigen Ordner,
**Turtlefin Languages**: in **Dokumente** (installierte Version) oder neben `turtlefin.exe` (portable Version).

1. **Einstellungen → Anzeige → Sprache hinzufügen** öffnet einen Explorer dieses Ordners; **Vorlage erstellen**
   schreibt dort `modele.po`, mit den Texten auf Englisch und als Notiz dem französischen Original und der
   aktuellen Sprache.
2. Eine Kopie mit dem Namen `<code>.po` anlegen (`sv.po` für Schwedisch, `ja.po` für Japanisch…), in diesem
   Ordner oder einem Unterordner, und jedes `msgstr ""` mit der Übersetzung der englischen `msgid` darüber
   füllen. Die `{}` und `{n}` behalten. Auch `X-Language-Name` (angezeigter Name) und bei Bedarf
   `Plural-Forms` (gettext-Regel der Sprache) ausfüllen. Jeder `.po`-Editor eignet sich, zum Beispiel
   [Poedit](https://poedit.net).
3. Erneut **Sprache hinzufügen**: die Übersetzung erscheint mit ihrem Fortschritt; sie auszuwählen wendet sie an.
   Leer gelassene Texte erscheinen auf Englisch.

Um sie mit allen zu teilen: ein Pull Request, der die Datei zu `lang/` (und zu `BUILTIN` in `src/i18n.rs`)
hinzufügt; `python tools/lang-check.py` prüft, dass nichts fehlt.

## Entwicklung

Siehe [HANDOFF.de.md](HANDOFF.de.md) (auch auf [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): Projektstand, Entscheidungen, Kompilieren, Pakete, Veröffentlichung,
Übersetzungen, bekannte Probleme.

## Lizenz

Turtlefin steht unter der [GNU GPL v3](LICENSE) (oder einer späteren Version): Jeder darf es nutzen, untersuchen,
ändern und weitergeben, solange die Lizenz gleich bleibt und der Quellcode der Änderungen geteilt wird.
