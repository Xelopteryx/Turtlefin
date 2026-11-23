<p align="center"><img src="packaging/icons/turtlefin-aero-256.png" width="96" alt="Logo Turtlefin (motyw Frutiger Aero)">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-256.png" width="128" alt="Logo Turtlefin">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-nerd-256.png" width="96" alt="Logo Turtlefin (motyw Nerd)"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · **Polski** · [Português](README.pt.md)

**Natywny, lekki i animowany klient Jellyfin, stworzony zarówno do salonu, jak i do biurka.** Napisany w Ruście ze
Slint (interfejs) i libmpv (odtwarzanie): bez Qt, bez wbudowanej przeglądarki. Działa na każdym komputerze z Windows
lub Linuksem, od starego laptopa po małe pudełko podłączone do telewizora, i obsługuje się go równie dobrze pilotem,
jak klawiaturą i myszą — a na Androidzie (telefon, tablet, telewizor) palcem.

> **Projekt w całości „vibecodowany”**: cały kod, teksty i tłumaczenia napisała sztuczna inteligencja (Claude, od
> Anthropic), prowadzona i testowana przez człowieka, który sam nie napisał kodu.

Obecna wersja: **2.0.0** · Języki interfejsu: Français, English, Español, Deutsch, Italiano, Português, Polski,
Nederlands — oraz każdy język, który dodasz Ty sam (zobacz [Tłumaczenie Turtlefin](#tłumaczenie-turtlefin)).

> **O tej wersji.** Wersja 2.0.0 była testowana w Windows, na Raspberry Pi 5 podłączonym do telewizora, w Linuksie i na
> Androidzie (telefon, emulator telewizora). Niektóre rzeczy wymagają jeszcze poprawek (zobacz notatki
> [wydań](https://github.com/Xelopteryx/Turtlefin/releases) i znane problemy w [HANDOFF.pl.md](HANDOFF.pl.md)). Twoje
> uwagi są mile widziane w [zgłoszeniach](https://github.com/Xelopteryx/Turtlefin/issues).

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="100%" alt="Ekran główny Turtlefin, motyw Turtlefin"></p>

## Zrzuty ekranu

Trzy motywy, każdy z własną animowaną scenerią dzienną i nocną (tu na serwerze testowym z wolnymi filmami Blender
Foundation):

<p align="center"><img src="docs/screenshots/en-home-turtlefin.jpg" width="32%" alt="Motyw Turtlefin">&nbsp;<img src="docs/screenshots/en-home-aero.jpg" width="32%" alt="Motyw Frutiger Aero">&nbsp;<img src="docs/screenshots/en-home-nerd.jpg" width="32%" alt="Motyw Nerd"></p>

Strona filmu, ekran końcowy (to, co właśnie skończyłeś oglądać, i co obejrzeć dalej) oraz „Kto ogląda?”:

<p align="center"><img src="docs/screenshots/en-detail.jpg" width="32%" alt="Strona filmu">&nbsp;<img src="docs/screenshots/en-end-screen.jpg" width="32%" alt="Ekran końcowy">&nbsp;<img src="docs/screenshots/en-who-is-watching.jpg" width="32%" alt="Kto ogląda?"></p>

Ta sama aplikacja na telefonie:

<p align="center"><img src="docs/screenshots/en-phone-home.jpg" width="24%" alt="Ekran główny na telefonie">&nbsp;<img src="docs/screenshots/en-phone-detail.jpg" width="24%" alt="Strona filmu na telefonie">&nbsp;<img src="docs/screenshots/en-phone-settings.jpg" width="24%" alt="Ustawienia na telefonie"></p>

## Co potrafi

- **Konta**: ekran „Kto ogląda?” ze zdjęciami profilowymi (także animowane GIF-y), do 12 kont zapisanych na urządzeniu
  (tylko token, nigdy hasło), opcjonalny kod dla konta, zmiana konta bez ponownego wpisywania. Wyszukiwanie serwerów
  we wszystkich sieciach urządzenia; adres główny i zapasowy, próbowany, gdy główny nie odpowiada.
- **Uruchamianie**: animowane logo, którego siedem punktów to prawdziwe sprawdzenia (język, ekran, odtwarzacz wideo,
  pamięć, konfiguracja, sieć i w środku serwer), potem „Kto ogląda?” — albo od razu wybrane konto przez Ustawienia →
  Konto → **Otwieraj to konto przy starcie**.
- **Ekran główny**: baner, Moje media, Oglądaj dalej, Następne, Ostatnio dodane; karty Ulubione, Do obejrzenia i Prośby
  (Seerr). Plakaty z liczbą pozostałych odcinków, znacznikiem „obejrzane” i oceną; tło pochodzi z wybranego elementu.
- **Strony**: film, serial, sezon, odcinek; odtwarzanie, ulubione, obejrzane, pobieranie, wybór dźwięku / napisów
  zapamiętany dla całego serialu; ocena, czas trwania, premiera, a dla serialu lub sezonu **obejrzane odcinki**
  („8 / 24” i procent); karta Szczegóły (gatunki, studia, osoby, języki dźwięku z flagą); „Więcej podobnych” i
  propozycje Seerr; prośba o brakujące sezony.
- **Odtwarzanie** (libmpv): rozdziały, odcinki sezonu, „Pomiń intro”, następny odcinek, maksymalna przepływność
  (powyżej niej serwer konwertuje wideo), własna głośność; animowany **ekran końcowy**: to, co właśnie skończyłeś, i
  trzy propozycje ze streszczeniem wybranej; pozycja i „obejrzane” odsyłane do serwera.
- **Wyszukiwanie** w bibliotece i w Seerr: tytuły, osoby (z ich filmografią, której brak na serwerze) oraz **studia lub
  stacje** („disney”, „pixar”, „netflix”: ich filmy i seriale do poproszenia); filtry, wyszukiwanie zaawansowane,
  losowy element.
- **Biblioteki**: siatka z indeksem A–Z i paskiem przewijania, filtrowanie przez wpisanie pierwszych liter tytułu;
  menu każdej karty (prawy przycisk, długie przytrzymanie, przytrzymane OK): obejrzane / nieobejrzane, Pobierz, Poproś
  oraz, dla administratora, Identyfikuj, Odśwież, Edytuj. Media, o które już poproszono, pokazują stan prośby.
- **Zaznaczanie wielu kart** w siatkach (długie przytrzymanie, przeciągnięcie myszą, przytrzymanie OK na pilocie),
  a potem poproszenie o nie lub pobranie wszystkich naraz.
- **Watch party** (SyncPlay): oglądanie tego samego w tym samym czasie na kilku urządzeniach.
- **Android** (telefon, tablet, Android TV): ta sama aplikacja, w pionie i w poziomie, palcem (przesunięcie od lewej
  krawędzi: wstecz), głośność telefonu, pilot i pad Android TV.
- **Przesyłanie**: wysłanie wideo na inny ekran — inny Turtlefin, aplikację Jellyfin konta lub Chromecast — i
  sterowanie nim z odtwarzacza. **Pilot**: sterowanie innym Turtlefin konta.
- **Strony Seerr**: element, którego brak na serwerze, otwiera się na własnej stronie (streszczenie, informacje,
  obsada), z przyciskami Poproś i **Zwiastun**.
- **Motywy**: Turtlefin, Frutiger Aero i Nerd, każdy z własnym logo, uruchamianiem i animowaną scenerią zależną od pory
  dnia; własny motyw (zobacz [Motywy](#motywy)). Offline cały interfejs zmienia kolor na bursztynowy.
- **Offline**: pobrane pliki zastępują ekran główny, z pełnymi stronami bez serwera; to, co obejrzano lub dodano do
  ulubionych offline, trafia na konto po ponownym połączeniu.
- **Pilot, klawiatura, mysz i palec wszędzie**: interfejs TV (duże elementy, klawiatura ekranowa) lub interfejs
  komputerowy (okno lub pełny ekran, F11).
- **Ustawienia** według kategorii, z wyszukiwaniem: zdjęcie profilowe (GetAvatar), języki, napisy, odtwarzanie,
  wyświetlanie, rozmiar tekstu, sieć, pamięć, animacje, mody, **aktualizacja z GitHuba**.
- **Animacje** wszędzie (plakat lecący do swojej strony, otwierający się baner, rzędy kaskadą, przejścia między
  stronami), regulowane pojedynczo, z ustawieniami (Wszystkie, Lekkie, Brak).
- **Płynność**: Turtlefin mierzy możliwości urządzenia i sam upraszcza scenerię, aby utrzymać 60 klatek na sekundę.
- Wbudowane **oprowadzanie** i **demo** („Poznaj Turtlefin” przy pierwszym uruchomieniu), z prawdziwymi wolnymi
  filmami; nic nie jest zapisywane.
- **Status Discord** (domyślnie wyłączony); **administratorzy**: edycja elementu z jego strony, panel.

## Wymagania systemowe

Zmierzone na Raspberry Pi 5, tanim telefonie (Galaxy A12s) i spowolnionym emulatorze Androida (jeden rdzeń ograniczony
do 30 %: da się używać, animacje 15–50 klatek na sekundę; ten sam rdzeń bez ograniczenia: animacje 60 klatek na sekundę).

**Minimalne** — wszystko działa, animacje wyłączone (Ustawienia → Animacje → **Brak**); sceneria sama się zatrzymuje,
jeśli urządzenie nie nadąża:

- 64-bitowy procesor 4-rdzeniowy około 1,5 GHz (Cortex-A53 / A55 telewizora lub taniego telefonu, nowszy Intel Celeron
  lub Atom);
- 2 GB pamięci (Turtlefin używa od 200 do 500 MB);
- akceleracja grafiki OpenGL (Windows, Linux na PC) lub OpenGL ES (Android, Raspberry Pi); zalecane sprzętowe
  dekodowanie wideo;
- Windows 10 lub 11, Linux z 2022 roku lub nowszy (Ubuntu 22.04, Debian 12…), Android 8 lub nowszy.

**Zalecane** — wszystkie animacje w 60 klatkach na sekundę:

- nowszy procesor 4-rdzeniowy około 2 GHz lub szybszy (Cortex-A76 z Raspberry Pi 5, Intel Core i3 8. generacji,
  AMD Ryzen 3 lub lepszy);
- 4 GB pamięci;
- sprzętowe dekodowanie HEVC 10-bit dla 4K.

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

W Windows najnowszy instalator jest pobierany i uruchamiany dla Twojego konta użytkownika (bez uprawnień
administratora). W Debianie, Ubuntu, Linux Mint i innych dystrybucjach z apt instalowany jest pakiet `.deb` (zostaniesz
poproszony o hasło); w pozostałych AppImage trafia do `~/.local/bin` z wpisem w menu aplikacji.

### Ręcznie

- **Windows 64-bit**: `Turtlefin-<version>-windows-x64-setup.exe` — instalator: język, folder i tryb **zainstalowany**
  (menu Start, odinstalowanie) lub **przenośny**.
- **Windows 64-bit, bez instalacji**: `Turtlefin-<version>-windows-x64-portable.zip` — rozpakuj, gdzie chcesz
  (pendrive…), potem uruchom `turtlefin.exe`.
- **Windows 32-bit** (stare komputery): `…-windows-x86-setup.exe` lub `…-windows-x86-portable.zip`.
- **Linux, każda dystrybucja (x86_64)**: `Turtlefin-<version>-linux-x86_64.AppImage` — nadaj prawo wykonywania
  (`chmod +x`), potem uruchom. ARM 64-bit: `…-linux-aarch64.AppImage`.
- **Linux z Flatpakiem (x86_64)**: `Turtlefin-<version>-linux-x86_64.flatpak` —
  `flatpak install --user ./Turtlefin-….flatpak` (środowisko pochodzi z Flathub).
- **Debian, Ubuntu, Mint…**: `turtlefin_<version>_amd64.deb` lub `_arm64.deb` — dwuklik lub
  `sudo apt install ./turtlefin_….deb`.
- **Android** (telefon, tablet, telewizor): `Turtlefin-<version>-android-arm64.apk` (telewizor 32-bit:
  `…-android-androidtv-armv7.apk`) — zezwól na instalację z tego źródła, potem otwórz plik.

Wersje dla Windows i AppImage zawierają wszystko (łącznie z odtwarzaczem libmpv). Pakiet `.deb` korzysta z systemowej
libmpv (`libmpv2`, instalowanej automatycznie przez apt).

**Wersja przenośna**: plik o nazwie `portable` obok `turtlefin.exe` sprawia, że konfiguracja, konta, pamięć podręczna
i pobrane pliki zostają w folderze `data` obok programu; nic nie jest zapisywane gdzie indziej.

### Aktualizacja

**Ustawienia → Informacje → Sprawdź aktualizacje** porównuje zainstalowaną wersję z najnowszą, a potem **Aktualizuj**
zajmuje się wszystkim w zależności od sposobu instalacji Turtlefin:

- **Windows, zainstalowany**: nowy instalator jest pobierany i uruchamiany po cichu w tym samym folderze;
- **Windows, przenośny**: nowe archiwum jest pobierane, a jego pliki zastępują stare;
- **AppImage**: nowy plik zastępuje stary;
- **Pakiet .deb**: pakiet jest instalowany przez `pkexec` (pojawi się prośba o hasło administratora);
- **Flatpak**: nie automatycznie — pobierz nowy plik `.flatpak` i zainstaluj go w ten sam sposób.

**Uruchom ponownie Turtlefin** uruchamia potem nową wersję.

## Zalecane wtyczki serwera

Turtlefin działa ze zwykłym serwerem Jellyfin (10.11 lub nowszym). Te wtyczki, instalowane na **serwerze**, dodają
funkcje:

- [**Jellyfin Enhanced**](https://github.com/n00bcodr/Jellyfin-Enhanced): karta Prośby, wyniki Seerr w wyszukiwaniu
  (tytuły, osoby, studia), propozycje i prośby Seerr na stronach — przez logowanie Jellyfin, bez klucza Seerr w
  kliencie.
- [**Seerr**](https://github.com/seerr-team/seerr) (dawniej Jellyseerr): sam menedżer próśb, używany przez Jellyfin
  Enhanced.
- [**Intro Skipper**](https://github.com/intro-skipper/intro-skipper): wykrywa intro i napisy końcowe — przycisk „Pomiń
  intro”, automatyczne pomijanie, „Następny odcinek” we właściwym momencie.
- [**GetAvatar**](https://github.com/cedev-1/jellyfin-plugin-GetAvatar): galeria zdjęć profilowych do wyboru w
  Ustawienia → Konto.
- **Turtlefin Trailers** (w trakcie tworzenia, jeszcze niewydany): pobiera na serwer brakujące zwiastuny, w języku
  każdej biblioteki i każdego konta, i przygotowuje te dla stron Seerr.

## Uruchamianie

- `turtlefin`: animacja startowa, potem „Kto ogląda?” (lub konto startowe).
- `turtlefin "Nazwa"`: zapisane konto „Nazwa”.
- `turtlefin "Nazwa" --server=http://…`: bezpośrednie logowanie (hasło w zmiennej `TURTLEFIN_PASSWORD`).
- `turtlefin --tv`: interfejs TV (pełny ekran, duże elementy); `--desktop`: interfejs komputerowy.
- `turtlefin --fullscreen`: pełny ekran bez interfejsu TV; `--no-intro`: bez animacji startowej.
- `turtlefin --sub=fr --audio=ja`: napisy i język dźwięku dla tego uruchomienia (`--sub=off`: bez napisów).
- `turtlefin --lang=en`, `--theme=nerd`: język i motyw (zachowywane potem).
- `turtlefin "Nazwa" --play="Bleach S1E3"`: odtwarza odcinek (sam „Tytuł”: film lub wznowienie serialu);
  `--open="Dragon Ball"`: otwiera stronę.
- `turtlefin --demo`: wbudowane demo; `--theme-creator`: Turtlefin Theme Creator; `--console`: okno dziennika
  (Windows); `--help`: wszystkie opcje.

Wiersz poleceń zawsze ma pierwszeństwo przed ustawieniami (konto startowe, interfejs TV).

## Klawisze i mysz

- **Strzałki** do poruszania się, **Enter** do otwierania / aktywowania, **Esc** lub **Backspace**, aby wrócić.
- Ekran główny: **←** na pierwszej karcie (lub Wstecz) otwiera menu; **↑** z góry strony: górny pasek.
- Odtwarzanie, ukryte sterowanie: **← →** 10 s do tyłu / do przodu, **↑ ↓** lub Enter pokazują sterowanie; **Spacja**:
  pauza · `a` dźwięk · `s` napisy · `f` pełny ekran.
- **F11**, wszędzie: pełny ekran (także w Ustawienia → Wyświetlanie, zachowywany między uruchomieniami).
- **Pilot**: Stop, następny / poprzedni odcinek, przewijanie do przodu / do tyłu, Menu; przytrzymanie OK na karcie: jej
  menu (lub zaznaczenie wielu kart w siatce).
- **Mysz**: kliknięcie otwiera, prawy przycisk to menu karty, kółko przechodzi między rzędami (Shift + kółko: w obrębie
  rzędu).
- **Palec**: dotknięcie otwiera, przesunięcie przewija, od lewej krawędzi: wstecz; w siatce długie przytrzymanie karty
  rozpoczyna zaznaczanie wielu kart.
- **Tekst**: litera wpisana na klawiaturze trafia od razu do pola (wyszukiwanie, logowanie), także w interfejsie TV.

## Pliki

- **Folder konfiguracji** (`turtlefin/`): `session.json` (bieżąca sesja), `accounts.json` (zapisane konta),
  `prefs.json` (ustawienia urządzenia), `tracks.json` (ścieżki dla seriali), `userdata.json` (obejrzane / ulubione
  offline).
- **Folder danych** (`turtlefin/downloads/`): pobrane pliki ułożone jak biblioteka
  (`<biblioteka>/<serial>/Season 01/<serial> S01E01.mkv`, z obrazem i napisami); `queue.json` (kolejka).
- **Folder pamięci podręcznej** (`turtlefin/img/`): obrazy i zdjęcia profilowe (można wyczyścić w Informacje).
- **Dokumenty**: `Turtlefin Languages` (dodane języki), `Turtlefin Themes` (motywy), `Turtlefin Mods` (mody) i
  `Turtlefin Presets` (wyeksportowane ustawienia animacji).

W Linuksie: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`. W Windows:
`%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`. Wersja przenośna: wszystko
w `data\` obok `turtlefin.exe`.

## Rozwiązywanie problemów

- `turtlefin --console` (Windows) lub uruchomienie z terminala (Linux) pokazuje dziennik.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: dziennik mpv · `TURTLEFIN_MPV_ARGS="…"`: dodatkowe opcje mpv.
- `TURTLEFIN_HWDEC=no`: dekodowanie programowe · `TURTLEFIN_AO=alsa`: wyjście dźwięku.
- Linie lub zakłócenia w obrazie (NVIDIA): Ustawienia → Odtwarzanie → **Dekodowanie sprzętowe** wyłączone.
- Wolne urządzenie: Ustawienia → Animacje → **Lekkie** lub **Brak**, oraz Ustawienia → Wyświetlanie → Motyw →
  **Szczegółowość tła** → Nieruchome.
- Renderowanie musi używać OpenGL (wybierane automatycznie): z `SLINT_BACKEND=winit-software` nie ma wideo.

## Motywy

Ustawienia → Wyświetlanie → **Motyw**: **Turtlefin** (domyślny: sześciokąty, fiolet → niebieski, dryfujące plamy
koloru), **Frutiger Aero** (w duchu Windows 7: niebieskawe szkło, błyszczące przyciski, laguna, przez której wodę widać
obraz elementu) i **Nerd** (płytka drukowana: serwer jest procesorem, prąd płynie ścieżkami) — lub własny motyw. Każda
sceneria podąża za porą dnia (dzień, zmierzch, noc, świt) albo zawsze zostaje dzienna lub nocna.

- **Utwórz motyw** otwiera *Turtlefin Theme Creator* (osobne okno, także w menu Start): każdy kolor, zaokrąglenie,
  odblaski, tło (obraz, GIF lub wideo), logo i muzyka, z podglądem na żywo. **Zapisz** zapisuje motyw w folderze
  **Turtlefin Themes**; **Zainstaluj** dodaje go i wybiera.
- **Importuj motyw** dodaje otrzymany motyw (jego folder lub plik `.tftheme`); **Eksportuj motyw** zapisuje bieżący
  motyw w tym folderze jako punkt wyjścia.

## Tłumaczenie Turtlefin

Bez programowania i kompilowania. Dodane języki znajdują się w jednym folderze, **Turtlefin Languages**: w
**Dokumentach** (wersja zainstalowana) lub obok `turtlefin.exe` (wersja przenośna).

1. **Ustawienia → Wyświetlanie → Dodaj język** otwiera eksplorator tego folderu; **Utwórz szablon** zapisuje w nim
   `modele.po`, z tekstami po angielsku i oryginałem francuskim w notatce.
2. Zrób kopię o nazwie `<kod>.po` (`sv.po` dla szwedzkiego, `ja.po` dla japońskiego…) i wypełnij każde `msgstr ""`
   tłumaczeniem angielskiego `msgid` powyżej. Zachowaj `{}` i `{n}`. Wypełnij też `X-Language-Name` (wyświetlana
   nazwa) i w razie potrzeby `Plural-Forms`. Nada się każdy edytor `.po`, na przykład [Poedit](https://poedit.net).
3. Ponownie **Dodaj język**: tłumaczenie pojawi się z przetłumaczoną częścią; wybranie go je stosuje. Teksty
   pozostawione puste wyświetlają się po angielsku.

Aby udostępnić je wszystkim: pull request dodający plik do `lang/` (i do `BUILTIN` w `src/i18n.rs`);
`python tools/lang-check.py` sprawdza, czy niczego nie brakuje.

## Rozwój

Zobacz [HANDOFF.pl.md](HANDOFF.pl.md) (także po [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md),
[Português](HANDOFF.pt.md)): stan projektu, decyzje, kompilacja, pakiety, publikacja, tłumaczenia, znane problemy.

## Podziękowania

Dziękujemy [LainX84](https://github.com/LainX84): szczegółowe sugestie
([zgłoszenie nr 2](https://github.com/Xelopteryx/Turtlefin/issues/2)) ukształtowały dużą część wersji 2.0 — indeks A–Z
i dobrze widoczny pasek przewijania dla dużych bibliotek, filtrowanie przez wpisanie pierwszych liter tytułu,
sprężyste przewijanie, karty rozdawane tylko raz na sesję (z ustawieniem wyłączającym animację) oraz menu kart
(obejrzane / nieobejrzane, by zacząć od nowa, Identyfikuj, Odśwież, Edytuj tytuł). Dziękujemy za poświęcony czas!

## Licencja

Turtlefin jest udostępniony na licencji [GNU GPL v3](LICENSE) (lub późniejszej wersji): każdy może go używać, badać,
modyfikować i rozpowszechniać, pod warunkiem zachowania tej samej licencji i udostępnienia kodu swoich zmian. Zrzuty
ekranu pokazują wolne filmy Blender Foundation (CC BY). Czcionka Turtlefin Symbols pochodzi z DejaVu Sans (licencja w
`ui/fonts/DejaVu-LICENSE.txt`).
