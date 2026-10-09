# Turtlefin: Projektübergabe (Stand 9. Oktober 2026, Version 1.0.0)

[Français](HANDOFF.md) · [English](HANDOFF.en.md) · **Deutsch** · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

Für alle, die die Entwicklung übernehmen (Mensch oder Claude Code). Vollständig lesen, bevor der Code angefasst
wird, danach [README.de.md](README.de.md) lesen (Nutzung, Installation, Tasten, Dateien).
Repository: https://github.com/Xelopteryx/Turtlefin · Version in `Cargo.toml`: 1.0.0.

## 1. Ziel

Ein **nativer Jellyfin-Client in Rust**, leicht, animiert, mit Fernbedienung ebenso bedienbar wie mit Tastatur und
Maus, installierbar auf jedem Windows- oder Linux-Rechner **ohne etwas zu kompilieren**: Wer ihn nutzt, lädt ein
Installationsprogramm oder Paket herunter (oder führt einen Installationsbefehl aus), das ist alles. Alle Pakete
baut die GitHub-CI (oder der PC des Maintainers), nie die Nutzerin oder der Nutzer.

Warum: Jellyfin Desktop (Qt / QtWebEngine) verliert Arbeitsspeicher und stürzt auf kleinen Rechnern irgendwann
ab, und die Weboberfläche mit einem aufwendigen Theme fällt auf bescheidener Hardware unter 30 Bilder/s. Feste
Regel: stabil im Speicher bleiben und nie wieder Echtzeit-Unschärfe oder Filteranimationen einführen.

## 2. Getroffene Entscheidungen

| Thema | Entscheidung | Grund |
|---|---|---|
| Sprache / UI | Rust + **Slint** `~1.18` (Darstellung zu 100 % durch Slint), Stil `fluent-dark` per `build.rs` erzwungen | Kein Browser; der Stil „native“ hinge von Qt ab |
| Slint-Feature `unstable-winit-030` | winit-Ereignisfilter für **F11** (`install_f11`) | Einzige Möglichkeit für eine globale Taste; daher `~1.18` (API zwischen Minor-Versionen instabil) |
| Netzwerk | `reqwest` 0.13 (rustls, Zertifikatsspeicher des Systems), `tokio` | `query` ist in 0.13 ein zu aktivierendes Feature |
| Wiedergabe | **libmpv zur Laufzeit geladen** (`libloading`, `src/mpv.rs`), OpenGL-Darstellung in eine von Slint angezeigte Textur (`src/video.rs`), Slint-Steuerung darüber (`ui/player.slint`) | Eingebetteter Player, kein IPC, funktioniert unter Wayland. Der Player wird bei jeder Wiedergabe neu erstellt (begrenzter Speicher) |
| Slint-Renderer | femtovg (OpenGL / GLES) erzwungen, außer `SLINT_BACKEND` ist gesetzt | Das Video läuft über eine OpenGL-Textur |
| Videotextur | Physische Pixel, Ursprung `TopLeft`, GL-Zustand rund um mpv gesichert / wiederhergestellt; `loadfile` wartet auf den Render-Kontext | Sonst Bild auf dem Kopf oder „No render context set“ |
| Dekodierung | `hwdec=no` unter Linux ARM 64 Bit, sonst `auto-safe` (Windows: `d3d11va-copy`) | Auf den getesteten ARM-Boards (Treiber v3d) liefert die V4L2-Dekodierung ein Format, das der Renderer nicht importieren kann |
| Audio unter Linux | `ao=pipewire,pulse,alsa`, `config=no` | Eine Benutzer-`mpv.conf`, die ALSA erzwang, scheiterte, wenn PipeWire den HDMI-Ausgang hält |
| Speicher | mpv-Cache begrenzt (100 / 25 MiB), Bilder in passender Größe angefordert, 16 Einträge pro Reihe | Kleine Rechner (4 GB) |
| Sprachen | Französisch im Code (Ausgangssprache), Übersetzung zur Laufzeit durch `src/i18n.rs` aus `lang/<code>.po` (eingebaut) und dem Ordner `Turtlefin Languages` | Siehe Abschnitt 6 |
| Vollbild (Windows) | Randloses Fenster, das den Bildschirm + 1 px bedeckt (`src/winfull.rs`, `set_tv_window`), kein echtes Vollbild; folgt Auflösungswechseln (alle 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` für das Vollbild von Slint | Im OpenGL-Vollbild hält AMD Software die App für ein Spiel: „ALT + R drücken“ bei jeder Rückkehr in den Vordergrund |
| TV- / Computer-Oberfläche | `tv-mode` ändert nur die Größe (`k` = 1,4) und die Eingabe (Bildschirmtastatur); das Vollbild ist getrennt (`full_flag`, `UiPrefs::fullscreen`, F11) | Wunsch des Nutzers: Vollbild, ohne die Oberfläche zu vergrößern |
| Fenster (Windows, Desktop) | Verkleinert und zentriert, wenn 1280 x 720 + Rahmen den Arbeitsbereich übersteigt; animiert beim Verschieben weiter (`winfull::keep_alive_while_moving`) | Bildschirme 1366 x 768; Windows blockiert die Ereignisschleife beim Verschieben |
| Konsolenfenster (Windows) | Subsystem „windows“ im Release; `--console` hängt eine an / öffnet eine; externe Befehle ohne Fenster (`paths::quiet_command`) | Keine Konsole und kein aufblitzendes CMD-Fenster |
| Befehlszeile | Hat immer Vorrang vor den Einstellungen (Startkonto, TV-Oberfläche) | Wunsch des Nutzers |
| Passwort | Nie gespeichert (nur das Token); in der Befehlszeile lieber `TURTLEFIN_PASSWORD` | Ein Argument ist für andere Prozesse sichtbar |

## 3. Aufbau des Codes

```
build.rs            kompilierter Commit, Slint-Stil, Icon der Exe (winresource, Windows)
lang/<code>.po      eingebaute Übersetzungen (Quelle: das Französisch im Code); tools/lang-check.py prüft sie
ui/theme.slint      Theme-Tokens, Globals Tr (Übersetzung) und Motion (aktive Animationen)
ui/app.slint        AppWindow und alle Bildschirme (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo: Startanimation (7 Punkte, Verbindung, Zoom), Sprachwahl
ui/player.slint     Wiedergabebildschirm     ui/osk.slint      Bildschirmtastatur
ui/card.slint       Posterkarte              ui/marquee.slint  Lauftext
ui/typed.slint      animierter getippter Text (Str, TypedText)   ui/langx.slint  Explorer (Sprachen, Presets)
ui/dust.slint       Balken des Sprachwechsels                   ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, gemeinsamer Zustand App (Arc), Bildschirme, Navigation (Stapel + gemerkte Seiten), Einstellungen
src/boot.rs         Startablauf (Prüfungen, Sprache, Wahl des Zielbildschirms)
src/i18n.rs         aktuelle Sprache, tr() / trf() / trn(), hinzugefügte Sprachen, Übersetzungsvorlage
src/dust.rs         Effekt des Sprachwechsels (Erkennen der Textzeilen, Morphing)
src/api.rs          Jellyfin-REST-Client (+ Seerr-Relay von Jellyfin Enhanced, GetAvatar)
src/config.rs       Sitzung, Konten (höchstens 12), prefs.json (UiPrefs, AnimFlags, Presets), Spuren, offline gesehen/Favoriten
src/discovery.rs    Serversuche (UDP, Subnetze, ARP, VPN-Peers)
src/downloads.rs    Downloads (Range-Fortsetzung, Warteschlange, Offline-Synchronisierung)
src/mpv.rs          libmpv-Anbindung; src/video.rs OpenGL-Textur; src/player.rs Wiedergabe, Meldungen, Verkettung
src/syncplay.rs     Watch Party (WebSocket /socket)
src/paths.rs        Ordner für Konfiguration / Cache / Daten; portabler Modus; quiet_command
src/update.rs       Update je nach Installation (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows: Bildschirm / Arbeitsbereich, Fenster beim Verschieben animiert
src/theme.rs        eingebaute Designs, .tftheme-Dateien (ThemeDef), Anwendung auf das Global Theme
ui/sky.slint        Kulissen (Himmel und Blasen · „Harmony“ von Windows 7), statisch, zwischengespeichert
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (Logo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   Kompilieren und Veröffentlichen bei einem Tag `v*` (Test: Branch `ci`)
```

Grundsätze:
- Netzwerkdaten laufen über `Send`-Strukturen, dann schiebt `upgrade_in_event_loop` sie in die Slint-Modelle.
  Bilder außerhalb des UI-Threads dekodiert, 6 parallele Downloads, mit einer Prüfung der ID angewendet.
- `App.gen` macht veraltete Ladevorgänge ungültig; `App.stack` ist der Navigationsstapel; `PAGES` behält Detail-
  und Bibliotheksseiten für Rückkehr ohne Anfrage.
- Tastaturnavigation von Hand (Auswahlindizes in Rust und Slint), da Slint den Fokus dynamischer Karten nicht
  verwaltet. Jeder Bildschirm hat seinen `FocusScope`; `refocus` gibt die Tastatur an die richtige Stelle zurück.
- `changed`-Handler von Slint werden verzögert: nicht auf ihre Reihenfolge verlassen (Positionen in zwei Schritten
  usw.).
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`, Header
  `Authorization: MediaBrowser …, Token=…`. Meldungen: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` und der WebSocket lehnen `api_key` ab: Token im Header.

## 4. Start

`main` wendet die Sprache an (prefs.json, sonst die vom Windows-Installationsprogramm geschriebene Datei
`language`) und startet dann `boot::run`. Der Bildschirm `boot` zeigt 7 Punkte: die 6 Ecken des Sechsecks, dann
die Mitte. Jeder ist eine echte Prüfung (`boot::check`):
1. **Sprache** (abgefragt, falls unbekannt): die Übersetzungen der gewählten Sprache werden geladen (`i18n::check`);
2. **Anzeige**: das Fenster hat einen OpenGL-Kontext erhalten (`video::gl_info`, Version und Grafikkarte im Protokoll);
3. **Videoplayer**: ein echter mpv-Player wird erstellt, initialisiert und wieder zerstört (`mpv::self_test`);
4. **Speicher**: eine Datei wird in den Ordnern für Konfiguration, Daten und Cache geschrieben, gelesen und gelöscht;
5. **Konfiguration**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` lesbar
   (`config::unreadable_files`, ganz am Anfang von `main` aufgerufen, bevor eine beschädigte Datei überschrieben wird);
6. **Netzwerk**: aktive Schnittstelle oder Route nach außen (`discovery::has_network`);
7. **Server** (die Mitte): die Hauptadresse, sonst die Ersatzadresse, antwortet auf `/System/Info/Public` **und** es
   ist derselbe Server (ID mit `server_id` der Sitzung verglichen); orange, solange kein Server eingerichtet ist.

Rot = Fehler, mit einer Meldung und „Weiter“ (von selbst nach 12 s). Alles grün: die Ecken verbinden sich, die
Strahlen laufen zur Mitte, und der Serverpunkt wird zum gefüllten Sechseck — genau das Logo
(`packaging/turtlefin.svg`, gleiche Geometrie) —, dann ein Zoom in die Mitte. Slint verkleinert die Zeichnung
eines `Path` um seine Strichstärke: die Pfade des Logos sind um so viel vergrößert, damit sie auf den Punkten liegen.

Zielbildschirm (`boot::route`), in dieser Reihenfolge: Name + Passwort in der Befehlszeile → Anmeldung; Name
eines gespeicherten Kontos → dieses Konto (unbekannter Name: sein Anmeldeformular); Startkonto
(`prefs.autostart_user` / `autostart_server`) → `fly_autostart` (das Bild des Kontos in der Mitte während der
Anmeldung); sonst „Wer schaut?“ (oder die Serversuche, falls keiner bekannt ist). Ein verschwundenes Startkonto
führt zu „Wer schaut?“. `--no-intro` (oder die ausgeschaltete Animation „Start“) überspringt die Animation; die
Prüfungen finden trotzdem statt.

## 5. Stand bei Version 1.0.0

Alle Funktionen der README sind fertig und auf Bildschirmfotos geprüft (Windows-PC, Bildschirme 1366 x 768 und
1920 x 1080) sowie auf einem Linux-ARM-Rechner am Fernseher, mit den Testkonten `test` / `test2` eines echten
Servers. Von der CI veröffentlichte Versionen: 0.9.0, 0.9.1; 1.0.0 ist bereit für das Tag (Abschnitt 7).

Mechanismen, die im Code nicht offensichtlich sind:
- **Geteiltes Bild** (`global Hero`): das Bild einer Karte fliegt zum Poster der Detailseite und bei der Rückkehr
  genau auf die Karte zurück (`Hero.want-id`, `hero-card-ok`).
- **Reihen** (`global Rows`): eigenes Scrollen pro Reihe, nach Schlüssel gemerkt; beim Reihenwechsel landet die
  Auswahl auf der nächstgelegenen Karte (`row-to`, `detail-pick-down`).
- **Offline**: `config::Flags` (userdata.json) speichert gesehen / Favoriten / Positionen mit einem Flag „zu
  senden“; `downloads::sync` sendet sie, wenn der Server zurück ist (das Gerät hat das letzte Wort).
- **Adressen**: `server_main` / `server_backup`; `watch_addresses` (20 s) wechselt auf die Ersatzadresse und zurück.
- **Watch Party**: eine WebSocket-Verbindung pro Sitzung (`sp_conn`), Abbruch bei 401 / 403, wachsende Wartezeit.
- **Profilbilder**: Festplatten-Cache `avatar_<id>_still|anim.bin` + rundes Vorschaubild `avatar_<id>_thumb.png`.
  Das Vorschaubild wird sofort gezeigt, das vollständige GIF außerhalb des UI-Threads dekodiert
  (`avatar_cached_async`) und nur dann vom Server erneuert, wenn es sich geändert hat (`avatar_fetch`).
  GIF-Animation: `AnimSlot` (Login, Picker, Header, Fly) und ein gemeinsamer Timer; der Avatar der Kopfzeile
  erhält alle Bilder einmal (`avatar-frames`), nur das sichtbare Bild wechselt. GetAvatar `SetAvatar` antwortet
  mit 500 → Ausweichweg `POST /UserImage`.
- **Animierte Anmeldung** (`fly-phase` 1 bis 4): das Bild wandert in die Mitte, Ladebalken, fliegt davon, dann
  erscheint die Startseite und der Avatar der Kopfzeile springt hervor (`me-pop`).
- **Sprachwechsel** (`dust.rs`, `ui/dust.slint`): beim Öffnen der Sprachliste bedecken Balken jede Textzeile,
  nehmen nach der Wahl die Breite der neuen Wörter an und decken sie dann auf. Die Zeilen werden gefunden, indem
  ein Abbild der Seite (`take_snapshot`) mit einem Abbild in der Schrift `Turtlefin Blank` (leere Zeichen, gleiche
  Breiten, `tools/make-blank-font.py`) verglichen wird. Die Uhr (Montserrat) ist ausgenommen.
- **Rundgang** (`tour-step` 0 bis 10): jeder Schritt bringt die Oberfläche in den erwarteten Zustand oder gilt als
  erledigt, wenn die Geste schon gemacht ist; `tour-ev` wird aus `changed`-Handlern aufgerufen.
- **Animationen**: Global `Motion` (12 Schalter: boot, pages, menu, select, scroll, panels, detail, player,
  search, login, language, tour), `config::AnimFlags`, eingebaute Presets (Alle, Leicht, Keine) und eigene, als
  `.json` in `Turtlefin Presets` exportiert / importiert. Jede Animationsdauer wird
  `duration: Motion.x ? 300ms : 0ms` geschrieben.
- **Eingabe**: am Computer sind die Felder `TextInput`s (Text verborgen, von `TypedText` gezeichnet); im TV-Modus
  die Bildschirmtastatur (`Osk`). In beiden Modi geht eine druckbare Taste, die die Seite erhält, ins Feld
  (`typing-key`, `erase-key`, `Field.type`); im TV-Modus löscht die Rücktaste nur, solange Text übrig ist. Am
  Computer nimmt ein Klick neben ein Feld diesem nicht die Tastatur weg (`focus-on-click: root.tv-mode`).
- **Gehaltene Tasten**: Eingabe, Esc und Rücktaste wiederholen ihre Aktion nicht (`event.repeat`), außer der
  Rücktaste beim Löschen von Text.
- **Designs** (`src/theme.rs`, Global `Theme` in ui/theme.slint): keine fest eingetragene Farbe in der Oberfläche;
  durchscheinende Flächen werden `Theme.fg.with-alpha(…)` geschrieben (weiß auf dunklem, Tinte auf hellem Design),
  Text auf dem Akzentverlauf nutzt `Theme.on-accent`. `Gloss` (Gel-Glanz) und `AeroSky` (ui/sky.slint) erscheinen
  nur bei `Theme.gloss` / `Theme.bubbles`. Importierte Designs liegen in prefs.json (`theme`, `themes`); der Editor
  (`tools/theme-creator.html`) ist ins Programm eingebettet (`theme::CREATOR`) und wird von „Design erstellen“ in
  „Turtlefin Themes“ abgelegt. Seine Ausgangsdesigns müssen mit denen in theme.rs übereinstimmen.
- **Maus**: Klick überall; Mausrad auf Startseite, Detailseite (`d-nav`, mit der Tastatur geteilt), Suche,
  Bibliotheken, Downloads, Einstellungen; Fenster im Vordergrund schlucken das Mausrad.

Nicht gemacht: Quick Connect; Gamepad; Lizenz (vom Maintainer zu wählen, vor oder nach 1.0.0); Anbindung an
XeLauncher (Mediacenter-Starter des Maintainers, keine Priorität). Als Nächstes geplant: Optimierung,
Version für Android TV.

## 6. Übersetzungen

- Zur Laufzeit durch `src/i18n.rs`: ein Katalog für Rust und für die Oberfläche.
  Slint: Global `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)`; `Tr.k` ändert sich bei jedem Sprachwechsel, wodurch die Texte
  neu berechnet werden. Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- Der französische Text **ist** der Schlüssel: ihn zu ändern heißt, die `msgid` in jeder `lang/*.po` zu ändern
  (`tools/lang-check.py` meldet fehlende und überzählige Texte und verlorene `{}`). Die 8 eingebauten Sprachen
  (fr, en, es, de, it, pt, pl, nl; `BUILTIN` in i18n.rs) sind vollständig (~530 Texte).
- Hinzugefügte Sprachen: jede `<code>.po` im Ordner `Turtlefin Languages` (`i18n::lang_dir`: Dokumente, neben der
  Exe in der portablen Version, unter `TURTLEFIN_CONFIG_DIR` bei Tests; der alte Ordner `languages` wird dorthin
  verschoben), Unterordner eingeschlossen; Name aus `X-Language-Name`; eine Datei kann eine eingebaute Sprache
  ersetzen. Durchsucht mit einem eingebauten Explorer (`ui/langx.slint`, Funktionen `lx_*` in main.rs).
- „Vorlage erstellen“ schreibt `modele.po`: msgid auf **Englisch**, Header `X-Source-Language: en`, Notizen `#.`
  auf Französisch und in der aktuellen Sprache; `keyed` führt diese msgid über `lang/en.po` auf das Französische
  zurück.
- In einer Sprache fehlende Texte: Englisch. Plural: Regel `Plural-Forms` der Datei, von i18n.rs ausgewertet.
- Installationsprogramm: `[Languages]` und `[CustomMessages]` in `turtlefin.iss`; es schreibt den gewählten Code
  in `language` neben der Exe, der beim ersten Start übernommen wird.
- Namen vom Server (Bibliotheken, Medien) werden nicht übersetzt.

## 7. Kompilieren, Pakete bauen, veröffentlichen

Entwicklung:
- Windows: Rust (https://rustup.rs), „Visual Studio Build Tools“ (C++), git; `cargo build --release`;
  `libmpv-2.dll` (Archiv `mpv-dev-x86_64-….7z` von
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) neben der Exe.
  Die Debug-Version braucht `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev`, dann `cargo build --release`.
- `cargo test --release`: Tests für i18n, update usw.

Eine Version veröffentlichen:
1. Die Nummer in `Cargo.toml` eintragen (`version = "x.y.z"`), einmal kompilieren (aktualisiert `Cargo.lock`),
   committen.
2. `git push origin main`, dann `git tag -a vx.y.z -m "Neuerungen, eine pro Zeile"` und `git push origin vx.y.z`.
   Die Nachricht des Tags wird zu den Versionshinweisen, die das eingebaute Update anzeigt.
3. `release.yml` kompiliert Windows x64 / x86 und Linux x86_64 / aarch64, baut Installationsprogramme, Archive,
   AppImages und `.deb`-Pakete und veröffentlicht sie in einem Release (im Reiter Actions des Repositorys zu
   verfolgen). Die Dateinamen (Kopf von `release.yml`) erwarten `update.rs` und die Installationsskripte genau so.
4. Test ohne Veröffentlichung: `git push origin main:ci` (alles wird kompiliert, nichts veröffentlicht).

Von Hand:
- **Windows**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (oder `x86`);
  braucht Inno Setup 6, 7-Zip und NASM (x86). Ergebnis in `target\dist`.
- **Linux** (auf einem Linux-Rechner): `TURTLEFIN_DIST=release cargo build --release`, dann
  `sh packaging/linux/build-appimage.sh <version>` und `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` beim Kompilieren, sonst hält `update::kind()` es für eine lokal kompilierte Version.
- x86: die seit Juli 2026 veröffentlichten 32-Bit-libmpv von shinchiro stürzen beim Start ab (OpenSSL); die vom
  10. Juni 2026 liegt in der Vorabversion `libmpv-i686-20260610` von Turtlefin (nicht löschen) und wird von
  `build.ps1` genutzt (`MPV_TAG`: eine andere Version von shinchiro). aws-lc braucht NASM in 32 Bit.
- Icons: `packaging/turtlefin.svg` ist das Logo; `packaging/icons/make-icons.py <ordner>` (Python + Pillow)
  erzeugt PNGs und ICO neu.

Update einer lokal kompilierten Kopie (`Kind::Source`): geänderte oder nicht verfolgte Dateien werden vor
`git pull --ff-only` beiseitegelegt (`git stash -u`, zurückholbar mit `git stash pop`). Die Neuerungen erscheinen in
einem eigenen Fenster (Einstellungen → Über → Neuerungen ansehen); `TURTLEFIN_TEST_UPDATE="Version x|Notiz|Notiz"`
simuliert ein Update zum Ausprobieren. Verknüpfungen in den Farben des Designs: `theme::apply_shortcuts` (.ico auf
den .lnk unter Windows, Icons `turtlefin` in ~/.local/share/icons unter Linux), bei Tests (`TURTLEFIN_CONFIG_DIR`)
übersprungen, außer mit `TURTLEFIN_TEST_SHORTCUTS=1`.

Branches: `main` (einziger Arbeitsbranch), `ci` (CI-Tests). `interface-lua` und `libmpv` sind alte, bereits in
`main` gemergte Experimente: sie können gelöscht werden. Tags: `v0.9.0`, `v0.9.1` (veröffentlichte Versionen) und
`libmpv-i686-20260610` (32-Bit-libmpv, siehe oben).

## 8. Tests

- `TURTLEFIN_CONFIG_DIR=<ordner>`: anderer Konfigurationsordner (Konten, Prefs, Sprachen), ohne den echten zu berühren.
- `--open=ID|settings|downloads`, `--play=ID@SEKUNDEN`, `--test-video=datei` (Player ohne Server).
- `TURTLEFIN_DEBUG_FRAMES=1` (Bilder > 25 ms); `TURTLEFIN_DEBUG_GAPS=1` mit
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed`: Pausen über 40 ms zwischen zwei Bildern.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: Nachrichten der Watch Party. Zwei Instanzen auf einem PC: zwei verschiedene
  `TURTLEFIN_CONFIG_DIR`, `--desktop`.
- Eine von PowerShell 5 geschriebene `prefs.json` hat ein BOM: beim Lesen der Konfigurationsdateien wird es ignoriert.
- Automatisierte Tests unter Windows: `SetForegroundWindow` wird erst nach einer simulierten Taste akzeptiert;
  F24 verwenden, nicht Alt (Alt allein versetzt das Fenster in den Menümodus und der nächste Klick geht verloren).
  Eine gehaltene Taste wird mit mehreren aufeinanderfolgenden „Taste gedrückt“-Aufrufen von `keybd_event`
  simuliert (Windows markiert sie als Wiederholungen).

## 9. Bekannte Probleme / Grenzen

1. Ein Absturz von mpv lässt Turtlefin abstürzen (gleicher Prozess).
2. Die Wiedergabe braucht OpenGL-Darstellung (`SLINT_BACKEND=winit-software` verhindert sie).
3. **mpv 0.40 / 0.41** (in mpv am 23. Januar 2026 behoben, Commit f74adc4): pro Bild eine nie freigegebene
   OpenGL-Fence; mit dem Treiber v3d belegt jede einen Deskriptor („MESA: error: Export failed“ nach ~42 s).
   Umgehung in `src/mpv.rs` (nur OpenGL ES). Diagnose: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` muss während
   der Wiedergabe stabil bleiben.
4. Speicherwachstum von mpv (~3 MB/min) bei ASS-Untertiteln: auf eine Wiedergabe begrenzt (mpv für jedes Video
   neu erstellt).
5. Token im Klartext in `session.json` / `accounts.json` (0600 unter Unix).
6. Tearing unter Xorg ohne Compositor (nur Openbox): einen Compositor verwenden (picom `--backend egl --vsync`).
   Turtlefin hält 60 Bilder/s.
7. Software-Dekodierung unter Linux ARM: kann bei 4K / HEVC an Grenzen stoßen; Ansatz: `TURTLEFIN_HWDEC=auto-copy`.
8. Mikropausen (~40 ms), nur bei auf volle Geschwindigkeit erzwungener Darstellung gemessen, bei jedem Bild eines
   GIFs in der Kopfzeile und beim Verbinden des Logos; Ursache nicht gefunden, im normalen Betrieb unsichtbar.

## 10. Arbeitsweise des Maintainers

- Antworten auf Französisch; wenig Erfahrung mit Linux / SSH: die Befehle erklären.
- Nichts auf GitHub pushen und keine Version veröffentlichen ohne ausdrückliche Zustimmung des Maintainers, der die
  Versionen selbst veröffentlicht.
- Tests mit einer Kopie der Konfiguration (`TURTLEFIN_CONFIG_DIR`), nie mit der echten; Testkonten `test` / `test2`.
- Nach jeder Änderungsrunde: ein Test-Installationsprogramm auf dem Desktop des Maintainers
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, das alte gelöscht).
