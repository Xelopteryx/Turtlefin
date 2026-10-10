<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin-Logo"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · **Deutsch** · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**Ein nativer, leichter und animierter Jellyfin-Client, fürs Wohnzimmer wie für den Schreibtisch.** Geschrieben
in Rust mit Slint (Oberfläche) und libmpv (Wiedergabe): kein Qt, kein eingebetteter Browser. Er läuft auf jedem
Windows- oder Linux-Rechner, vom alten Laptop bis zur kleinen Box am Fernseher, und lässt sich mit der
Fernbedienung genauso gut bedienen wie mit Tastatur und Maus.

> **Vollständig per Vibecoding entstanden**: Code, Texte und Übersetzungen wurden komplett von einer KI
> (Claude von Anthropic) geschrieben, angeleitet und getestet von einem Menschen, der nicht selbst programmiert hat.

Aktuelle Version: **1.0.0** · Sprachen der Oberfläche: Deutsch, English, Français, Español, Italiano, Português,
Polski, Nederlands — und jede selbst hinzugefügte Sprache (siehe [Turtlefin übersetzen](#turtlefin-übersetzen)).

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
  einer Serie, eigene Lautstärke von Turtlefin; Position und „gesehen“ werden an den Server gemeldet.
- **Watch Party** (SyncPlay): auf mehreren Geräten gleichzeitig dasselbe schauen.
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

## Starten

```
turtlefin                                  Startanimation, dann „Wer schaut?“ (oder das Startkonto)
turtlefin "Name"                           gespeichertes Konto „Name“
turtlefin "Name" --server=http://…         direkte Anmeldung (Passwort: Variable TURTLEFIN_PASSWORD=…)
turtlefin --tv                             TV-Oberfläche: Vollbild, große Elemente
turtlefin --desktop                        Computer-Oberfläche (hat Vorrang vor der Einstellung „TV-Oberfläche“)
turtlefin --no-intro                       keine Startanimation
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
- **Maus**: Klick zum Öffnen, Mausrad von Reihe zu Reihe (Umschalt + Mausrad: innerhalb der Reihe).
- **Text**: ein auf der Tastatur getippter Buchstabe landet direkt im Feld (Suche, Anmeldung, Adresse), auch in
  der TV-Oberfläche; dort öffnet ein Klick auf das Suchfeld die Bildschirmtastatur.

## Dateien

| Wo | Was |
|---|---|
| Konfigurationsordner, `turtlefin/` | `session.json` (aktuelle Sitzung), `accounts.json` (gespeicherte Konten), `prefs.json` (Geräteeinstellungen), `tracks.json` (Spuren pro Serie), `userdata.json` (offline Gesehenes / Favoriten) |
| Datenordner, `turtlefin/downloads/` | Downloads (Medium, Poster, Hintergrund, Logo, `info.json`), `queue.json` (Warteschlange) |
| Cache-Ordner, `turtlefin/img/` | Bilder und Profilbilder (unter Über leerbar) |
| **Dokumente** | `Turtlefin Languages` (hinzugefügte Sprachen) und `Turtlefin Presets` (exportierte Animations-Presets) |

Unter Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Unter Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Portable Version: alles in `data\` neben `turtlefin.exe` (`config`, `cache`, `downloads`), die Sprach- und
Preset-Ordner neben dem Programm.

## Bei Problemen

- `turtlefin --console` (Windows) oder Start aus einem Terminal (Linux) zeigt das Protokoll.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: mpv-Protokoll · `TURTLEFIN_MPV_ARGS="…"`: zusätzliche mpv-Optionen.
- `TURTLEFIN_HWDEC=auto-copy`: Hardware-Dekodierung (unter Linux ARM standardmäßig Software) · `TURTLEFIN_AO=alsa`: Audioausgabe.
- `TURTLEFIN_LIBMPV=Pfad`: anderer Ort für libmpv · `TURTLEFIN_DEBUG_FRAMES=1`: meldet langsame Bilder.
- Die Darstellung muss OpenGL sein (automatisch gewählt): mit `SLINT_BACKEND=winit-software` gibt es kein Video.

## Designs

Einstellungen → Anzeige → **Design**: Turtlefin (Standard), Dunkel, Hell, Frutiger Aero (im Geist von Windows 7:
Hintergrund „Harmony“, bläuliches Glas, glänzende Gel-Schaltflächen, Player mit Kugel, Start mit Glaskugeln), Turtlefin Grün — oder ein eigenes Design.

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
