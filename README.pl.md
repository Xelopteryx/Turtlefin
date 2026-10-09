<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Logo Turtlefin"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · **Polski** · [Português](README.pt.md)

**Natywny, lekki i animowany klient Jellyfin, stworzony zarówno do salonu, jak i do biurka.** Napisany w Rust ze
Slint (interfejs) i libmpv (odtwarzanie): bez Qt i bez wbudowanej przeglądarki. Działa na każdym komputerze z
Windows lub Linuksem, od starego laptopa po małe pudełko podłączone do telewizora, i równie dobrze obsługuje się
go pilotem, jak klawiaturą i myszą.

Aktualna wersja: **1.0.0** · Języki interfejsu: Polski, English, Français, Español, Deutsch, Italiano, Português,
Nederlands — oraz każdy język dodany samodzielnie (zob. [Tłumaczenie Turtlefin](#tłumaczenie-turtlefin)).

## Co potrafi

- **Konta**: ekran „Kto ogląda?” ze zdjęciami profilowymi (także animowanymi GIF-ami, w pamięci podręcznej), do
  12 kont zapisanych na urządzeniu (tylko token, nigdy hasło), zmiana konta bez ponownego wpisywania,
  „Zarządzaj kontami”, aby je usuwać. Wyszukiwanie serwerów we wszystkich sieciach urządzenia; adres główny i
  adres zapasowy, próbowany, gdy główny nie odpowiada.
- **Uruchamianie**: animowane logo, którego siedem punktów to prawdziwe kontrole (język, wyświetlanie,
  odtwarzacz wideo, pamięć, konfiguracja, sieć i — w środku — serwer), a potem „Kto ogląda?” — albo od razu konto
  wybrane w Ustawienia → Konto → **Otwieraj to konto przy starcie**.
- **Strona główna**: Moje media, Oglądaj dalej, Następne, Ostatnio dodane; karty Ulubione i Prośby (Seerr).
  Plakaty z liczbą pozostałych odcinków, znacznikiem „obejrzane” i oceną; tło z wybranego materiału.
- **Strony szczegółów**: film, serial, sezon, odcinek; odtwarzanie, ulubione, obejrzane, pobieranie, wybór
  dźwięku / napisów zapamiętany dla całego serialu; „Więcej podobnych” i propozycje Seerr; prośba o brakujące
  sezony.
- **Odtwarzanie** (libmpv): rozdziały, odcinki sezonu, „Pomiń intro”, następny odcinek, propozycje na koniec
  serialu, własna głośność Turtlefin; pozycja i „obejrzane” wysyłane do serwera.
- **Watch party** (SyncPlay): oglądanie tego samego w tym samym czasie na kilku urządzeniach.
- **Offline**: pobrane materiały zastępują stronę główną, z pełnymi stronami szczegółów bez serwera; to, co
  obejrzano lub dodano do ulubionych offline, trafia do konta po ponownym połączeniu.
- **Wyszukiwanie** (biblioteka + Seerr) i losowy materiał.
- **Pilot, klawiatura i mysz wszędzie**: interfejs TV (duże elementy, klawiatura ekranowa) lub interfejs
  komputerowy (okno lub pełny ekran, F11), kliknięcia i kółko myszy na każdej stronie, a fizyczna klawiatura
  pisze bezpośrednio w polach tekstowych w obu trybach.
- **Ustawienia**: zdjęcie profilowe (awatary GetAvatar według kategorii), język interfejsu, języki dźwięku i
  napisów, rozmiar napisów, automatyczny następny odcinek i pomijanie intro, interfejs TV, pełny ekran, tło,
  oceny, godzina, adresy serwera, pamięć podręczna obrazów, **aktualizacja z GitHuba**.
- **Animacje** wszędzie (plakat lecący do strony szczegółów, wysuwane menu, rzędy kaskadą, animowane logowanie,
  zmiana języka), regulowane pojedynczo w Ustawienia → Animacje, z presetami (Wszystkie, Lekkie, Brak) i
  własnymi, które można eksportować i importować.
- **Przewodnik**: proponowany przy pierwszym uruchomieniu, do obejrzenia ponownie w Ustawienia → Informacje.

## Instalacja

Nic do kompilowania: pobierasz, instalujesz, gotowe. Wszystkie pliki są na stronie
[Releases](https://github.com/Xelopteryx/Turtlefin/releases).

### Jednym poleceniem

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

W Windows najnowszy instalator jest pobierany i uruchamiany dla twojego konta użytkownika (bez uprawnień
administratora). W Debianie, Ubuntu, Linux Mint i innych dystrybucjach z apt instalowany jest pakiet `.deb`
(pojawi się prośba o hasło); w pozostałych AppImage trafia do `~/.local/bin`, z wpisem w menu aplikacji.

### Ręcznie

| System | Plik | Uwagi |
|---|---|---|
| Windows 64-bit | `Turtlefin-<version>-windows-x64-setup.exe` | Instalator: język, folder i tryb **zainstalowany** (menu Start, odinstalowanie) lub **przenośny** |
| Windows 64-bit, bez instalacji | `Turtlefin-<version>-windows-x64-portable.zip` | Rozpakować w dowolnym miejscu (pendrive…) i uruchomić `turtlefin.exe` |
| Windows 32-bit | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Dla starszych komputerów |
| Linux, wszystkie dystrybucje (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Nadać prawo wykonywania (`chmod +x`) i uruchomić |
| Linux, ARM 64-bit | `Turtlefin-<version>-linux-aarch64.AppImage` | Tak samo |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Dwuklik albo `sudo apt install ./turtlefin_….deb` |

Wersje dla Windows i AppImage zawierają wszystko (łącznie z odtwarzaczem libmpv). Pakiet `.deb` korzysta z libmpv
systemu (`libmpv2`, instalowanej automatycznie przez apt). AppImage i pakiety wymagają dystrybucji z 2022 roku lub
nowszej (Ubuntu 22.04, Debian 12…).

**Wersja przenośna**: plik `portable` obok `turtlefin.exe` sprawia, że konfiguracja, konta, pamięć podręczna i
pobrane pliki zostają w folderze `data` obok programu; nic nie jest zapisywane gdzie indziej.

### Aktualizacja

**Ustawienia → Informacje → Sprawdź aktualizacje** porównuje zainstalowaną wersję z najnowszą opublikowaną, a
potem **Aktualizuj** robi wszystko, zależnie od sposobu instalacji Turtlefin:

| Instalacja | Aktualizacja |
|---|---|
| Windows, zainstalowany | nowy instalator jest pobierany i uruchamiany po cichu w tym samym folderze |
| Windows, przenośny | nowe archiwum jest pobierane, a jego pliki zastępują stare |
| AppImage | nowy plik zastępuje stary |
| Pakiet .deb | pakiet jest instalowany przez `pkexec` (pojawi się prośba o hasło administratora) |

**Uruchom ponownie Turtlefin** otwiera potem nową wersję.

## Zalecane wtyczki serwera

Turtlefin działa ze zwykłym serwerem Jellyfin (10.11 lub nowszym). Te wtyczki, instalowane na **serwerze**,
dodają funkcje:

| Wtyczka | Co daje Turtlefin |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Karta Prośby, wyniki Seerr w wyszukiwaniu, propozycje i prośby Seerr na stronach szczegółów — przez logowanie Jellyfin, bez klucza Seerr w kliencie |
| [Seerr](https://github.com/seerr-team/seerr) (dawniej Jellyseerr) | Sam menedżer próśb, używany przez Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Wykrywa czołówki i napisy końcowe: przycisk „Pomiń intro”, automatyczne pomijanie, „Następny odcinek” we właściwej chwili |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | Galeria zdjęć profilowych do wyboru w Ustawienia → Konto |

## Uruchamianie

```
turtlefin                                  animacja startowa, potem „Kto ogląda?” (lub konto startowe)
turtlefin "Nazwa"                          zapisane konto „Nazwa”
turtlefin "Nazwa" --server=http://…        bezpośrednie logowanie (hasło: zmienna TURTLEFIN_PASSWORD=…)
turtlefin --tv                             interfejs TV: pełny ekran, duże elementy
turtlefin --desktop                        interfejs komputerowy (ma pierwszeństwo przed ustawieniem „Interfejs TV”)
turtlefin --no-intro                       bez animacji startowej
turtlefin --console                        okno dziennika (Windows)
turtlefin --tutorial                       przewodnik po otwarciu strony głównej
```

Wiersz poleceń zawsze ma pierwszeństwo przed ustawieniami (konto startowe, interfejs TV).

## Klawisze i mysz

- **Strzałki** do poruszania się, **Enter**, aby otworzyć / aktywować, **Esc** lub **Backspace**, aby wrócić
  (przytrzymany: tylko jeden krok wstecz).
- Strona główna: **←** na pierwszej karcie (lub Wstecz) otwiera menu; w menu **→** lub Esc je zamyka.
- **↑** z górnej części strony: górny pasek (wstecz, start, menu, watch party, losowo, wyszukiwanie, konto).
- Odtwarzanie, ukryte sterowanie: **← →** cofnij / przewiń o 10 s, **↑ ↓** lub Enter pokazują sterowanie, ↓ z
  przycisków: odcinki sezonu. **Spacja**: pauza · `a` dźwięk · `s` napisy · `f` pełny ekran.
- **F11**, wszędzie: pełny ekran (także w Ustawienia → Wyświetlanie → **Pełny ekran**, zapamiętywany między
  uruchomieniami poza interfejsem TV).
- **Mysz**: kliknięcie otwiera, kółko przechodzi z rzędu do rzędu (Shift + kółko: w obrębie rzędu).
- **Tekst**: litera wpisana na klawiaturze trafia od razu do pola (wyszukiwanie, logowanie, adres), także w
  interfejsie TV; tam kliknięcie pola wyszukiwania otwiera klawiaturę ekranową.

## Pliki

| Gdzie | Co |
|---|---|
| folder konfiguracji, `turtlefin/` | `session.json` (bieżąca sesja), `accounts.json` (zapisane konta), `prefs.json` (ustawienia urządzenia), `tracks.json` (ścieżki dla seriali), `userdata.json` (obejrzane / ulubione offline) |
| folder danych, `turtlefin/downloads/` | pobrane pliki (materiał, plakaty, tło, logo, `info.json`), `queue.json` (kolejka) |
| folder pamięci podręcznej, `turtlefin/img/` | obrazy i zdjęcia profilowe (do wyczyszczenia w Informacje) |
| **Dokumenty** | `Turtlefin Languages` (dodane języki) i `Turtlefin Presets` (wyeksportowane presety animacji) |

W Linuksie: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
W Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Wersja przenośna: wszystko w `data\` obok `turtlefin.exe` (`config`, `cache`, `downloads`), a foldery języków i
presetów obok programu.

## W razie problemów

- `turtlefin --console` (Windows) lub uruchomienie z terminala (Linux) pokazuje dziennik.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: dziennik mpv · `TURTLEFIN_MPV_ARGS="…"`: dodatkowe opcje mpv.
- `TURTLEFIN_HWDEC=auto-copy`: dekodowanie sprzętowe (w Linuksie ARM domyślnie programowe) · `TURTLEFIN_AO=alsa`: wyjście dźwięku.
- `TURTLEFIN_LIBMPV=ścieżka`: inne położenie libmpv · `TURTLEFIN_DEBUG_FRAMES=1`: zgłasza wolne klatki.
- Renderowanie musi używać OpenGL (wybierane automatycznie): z `SLINT_BACKEND=winit-software` nie ma obrazu wideo.

## Motywy

Ustawienia → Wyświetlanie → **Motyw**: Turtlefin (domyślny), Ciemny, Jasny, Frutiger Aero (niebo, trawa, bańki,
błyszczące żelowe przyciski), Turtlefin zielony — albo własny motyw.

- **Utwórz motyw** otwiera *Turtlefin Theme Creator* w przeglądarce: każdy kolor, zaokrąglenie, połysk, niebo i
  bańki, z podglądem na żywo; zapisuje plik `.tftheme`.
- Ten plik należy umieścić w folderze **Turtlefin Themes** (Dokumenty, lub obok `turtlefin.exe` w wersji
  przenośnej), a potem wybrać **Importuj motyw**. **Eksportuj motyw** zapisuje bieżący motyw w tym folderze jako
  punkt wyjścia.
- Kreator jest też w repozytorium, `tools/theme-creator.html`: jeden plik, który działa offline.

## Tłumaczenie Turtlefin

Bez programowania i kompilowania. Dodane języki znajdują się w jednym folderze, **Turtlefin Languages**: w
**Dokumentach** (wersja zainstalowana) lub obok `turtlefin.exe` (wersja przenośna).

1. **Ustawienia → Wyświetlanie → Dodaj język** otwiera eksplorator tego folderu; **Utwórz szablon** zapisuje w nim
   `modele.po`, z tekstami po angielsku oraz, jako uwagi, oryginałem francuskim i bieżącym językiem.
2. Zrobić jego kopię o nazwie `<kod>.po` (`sv.po` dla szwedzkiego, `ja.po` dla japońskiego…), w tym folderze lub
   podfolderze, i wypełnić każde `msgstr ""` tłumaczeniem angielskiego `msgid` powyżej. Zachować `{}` i `{n}`.
   Wypełnić też `X-Language-Name` (wyświetlana nazwa) i w razie potrzeby `Plural-Forms` (reguła gettext danego
   języka). Nada się każdy edytor `.po`, na przykład [Poedit](https://poedit.net).
3. Ponownie **Dodaj język**: tłumaczenie pojawi się z informacją, ile już przetłumaczono; wybranie go je
   zastosuje. Teksty pozostawione puste są wyświetlane po angielsku.

Aby udostępnić je wszystkim: pull request dodający plik do `lang/` (i do `BUILTIN` w `src/i18n.rs`);
`python tools/lang-check.py` sprawdza, czy niczego nie brakuje.

## Rozwój

Zob. [HANDOFF.pl.md](HANDOFF.pl.md) (także po [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md),
[Português](HANDOFF.pt.md)): stan projektu, decyzje, kompilacja, pakiety, publikacja, tłumaczenia, znane
problemy.
