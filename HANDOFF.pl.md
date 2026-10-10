# Turtlefin: przekazanie projektu (stan na 9 października 2026, wersja 1.1.0)

[Français](HANDOFF.md) · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · **Polski** · [Português](HANDOFF.pt.md)

Dla osoby, która przejmuje rozwój (człowieka lub Claude Code). Przeczytaj całość przed dotknięciem kodu, a potem
[README.pl.md](README.pl.md) (użytkowanie, instalacja, klawisze, pliki).
Repozytorium: https://github.com/Xelopteryx/Turtlefin · Wersja w `Cargo.toml`: 1.1.0.

## 1. Cel

**Natywny klient Jellyfin w Rust**, lekki, animowany, obsługiwany pilotem równie dobrze jak klawiaturą i myszą,
instalowany na dowolnym komputerze z Windows lub Linuksem **bez kompilowania czegokolwiek**: użytkownik pobiera
instalator lub pakiet (albo uruchamia polecenie instalacji) i to wszystko. Wszystkie pakiety buduje CI GitHuba
(lub komputer osoby utrzymującej projekt), nigdy użytkownik.

Dlaczego: Jellyfin Desktop (Qt / QtWebEngine) gubi pamięć i w końcu pada na małych komputerach, a interfejs
webowy z ciężkim motywem spada poniżej 30 klatek/s na skromnym sprzęcie. Stała zasada: zachować stabilne zużycie
pamięci i nigdy nie wprowadzać ponownie rozmycia w czasie rzeczywistym ani animacji filtrów.

## 2. Podjęte decyzje

| Temat | Decyzja | Powód |
|---|---|---|
| Język / UI | Rust + **Slint** `~1.18` (renderowanie w 100 % przez Slint), styl `fluent-dark` wymuszony przez `build.rs` | Bez przeglądarki; styl „native” zależałby od Qt |
| Funkcja Slint `unstable-winit-030` | Filtr zdarzeń winit dla **F11** (`install_f11`) | Jedyny sposób na globalny klawisz; stąd `~1.18` (API niestabilne między wersjami pomocniczymi) |
| Sieć | `reqwest` 0.13 (rustls, systemowy magazyn certyfikatów), `tokio` | `query` to w 0.13 funkcja do włączenia |
| Odtwarzanie | **libmpv ładowana w czasie działania** (`libloading`, `src/mpv.rs`), renderowanie OpenGL do tekstury wyświetlanej przez Slint (`src/video.rs`), sterowanie Slint na wierzchu (`ui/player.slint`) | Wbudowany odtwarzacz, bez IPC, działa pod Waylandem. Odtwarzacz jest tworzony od nowa przy każdym odtwarzaniu (ograniczona pamięć) |
| Renderer Slint | femtovg (OpenGL / GLES) wymuszony, chyba że ustawiono `SLINT_BACKEND` | Wideo przechodzi przez teksturę OpenGL |
| Tekstura wideo | Fizyczne piksele, początek `TopLeft`, stan GL zapisywany / przywracany wokół mpv; `loadfile` czeka na kontekst renderowania | Inaczej obraz do góry nogami albo „No render context set” |
| Dekodowanie | `hwdec=no` w Linuksie ARM 64-bit, gdzie indziej `auto-safe` (Windows: `d3d11va-copy`) | Na testowanych płytkach ARM (sterownik v3d) dekodowanie V4L2 daje format, którego renderer nie umie zaimportować |
| Dźwięk w Linuksie | `ao=pipewire,pulse,alsa`, `config=no` | Plik `mpv.conf` użytkownika wymuszający ALSA zawodził, gdy PipeWire trzyma wyjście HDMI |
| Pamięć | Pamięć podręczna mpv ograniczona (100 / 25 MiB), obrazy pobierane we właściwym rozmiarze, 16 elementów w rzędzie | Małe komputery (4 GB) |
| Języki | Francuski w kodzie (język źródłowy), tłumaczenie w czasie działania przez `src/i18n.rs` z `lang/<code>.po` (wbudowane) i z folderu `Turtlefin Languages` | Zob. rozdział 6 |
| Pełny ekran (Windows) | Okno bez ramki pokrywające ekran + 1 px (`src/winfull.rs`, `set_tv_window`), nie prawdziwy pełny ekran; śledzi zmiany rozdzielczości (co 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` dla pełnego ekranu Slint | W pełnym ekranie OpenGL AMD Software bierze aplikację za grę: „Naciśnij ALT + R” przy każdym powrocie na pierwszy plan |
| Interfejs TV / komputerowy | `tv-mode` zmienia tylko rozmiar (`k` = 1,4) i wprowadzanie tekstu (klawiatura ekranowa); pełny ekran jest osobno (`full_flag`, `UiPrefs::fullscreen`, F11) | Prośba użytkownika: pełny ekran bez powiększania interfejsu |
| Okno (Windows, pulpit) | Zmniejszone i wyśrodkowane, gdy 1280 x 720 + ramka przekracza obszar roboczy; animowane podczas przesuwania (`winfull::keep_alive_while_moving`) | Ekrany 1366 x 768; Windows blokuje pętlę zdarzeń podczas przesuwania okna |
| Okno konsoli (Windows) | Podsystem „windows” w wersji release; `--console` dołącza / otwiera konsolę; polecenia zewnętrzne bez okna (`paths::quiet_command`) | Bez konsoli i bez migającego okna CMD |
| Wiersz poleceń | Zawsze ma pierwszeństwo przed ustawieniami (konto startowe, interfejs TV) | Prośba użytkownika |
| Hasło | Nigdy nie jest zapisywane (tylko token); w wierszu poleceń lepiej `TURTLEFIN_PASSWORD` | Argument jest widoczny dla innych procesów |

## 3. Struktura kodu

```
build.rs            skompilowany commit, styl Slint, ikona pliku exe (winresource, Windows)
lang/<code>.po      wbudowane tłumaczenia (źródło: francuski w kodzie); tools/lang-check.py je sprawdza
ui/theme.slint      tokeny motywu, globale Tr (tłumaczenie) i Motion (włączone animacje)
ui/app.slint        AppWindow i wszystkie ekrany (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo: animacja startowa (7 punktów, łączenie, zoom), wybór języka
ui/player.slint     ekran odtwarzania           ui/osk.slint      klawiatura ekranowa
ui/card.slint       karta z plakatem            ui/marquee.slint  przewijany tekst
ui/typed.slint      animowany wpisywany tekst (Str, TypedText)   ui/langx.slint  eksplorator (języki, presety)
ui/dust.slint       paski zmiany języka                         ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, wspólny stan App (Arc), ekrany, nawigacja (stos + zachowane strony), ustawienia
src/boot.rs         sekwencja startowa (kontrole, język, wybór ekranu docelowego)
src/i18n.rs         bieżący język, tr() / trf() / trn(), dodane języki, szablon tłumaczenia
src/dust.rs         efekt zmiany języka (znajdowanie wierszy tekstu, morfing)
src/api.rs          klient REST Jellyfin (+ przekaźnik Seerr z Jellyfin Enhanced, GetAvatar)
src/config.rs       sesja, konta (najwyżej 12), prefs.json (UiPrefs, AnimFlags, presety), ścieżki, obejrzane/ulubione offline
src/discovery.rs    wyszukiwanie serwerów (UDP, podsieci, ARP, węzły VPN)
src/downloads.rs    pobieranie (wznawianie Range, kolejka, synchronizacja offline)
src/mpv.rs          powiązanie z libmpv; src/video.rs tekstura OpenGL; src/player.rs odtwarzanie, raporty, kolejne odcinki
src/syncplay.rs     watch party (WebSocket /socket)
src/paths.rs        foldery konfiguracji / pamięci podręcznej / danych; tryb przenośny; quiet_command
src/update.rs       aktualizacja zależnie od instalacji (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows: ekran / obszar roboczy, okno animowane podczas przesuwania
src/theme.rs        wbudowane motywy, pliki .tftheme (ThemeDef), nakładanie na globalny Theme
ui/sky.slint        scenerie (niebo i bańki · „Harmony” z Windows 7), statyczne, w pamięci podręcznej
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   kompilacja i publikacja po tagu `v*` (próba: gałąź `ci`)
```

Zasady:
- Dane z sieci przechodzą przez struktury `Send`, a potem `upgrade_in_event_loop` wstawia je do modeli Slint.
  Obrazy dekodowane poza wątkiem UI, 6 pobrań równolegle, nakładane z kontrolą identyfikatora.
- `App.gen` unieważnia przestarzałe ładowania; `App.stack` to stos nawigacji; `PAGES` przechowuje strony
  szczegółów i biblioteki, aby wracać bez zapytań.
- Nawigacja klawiaturą zrobiona ręcznie (indeksy zaznaczenia w Rust i Slint), bo Slint nie obsługuje fokusu
  dynamicznych kart. Każdy ekran ma swój `FocusScope`; `refocus` oddaje klawiaturę we właściwe miejsce.
- Handlery `changed` w Slint są odroczone: nie polegać na ich kolejności (pozycje w dwóch krokach itd.).
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`, nagłówek
  `Authorization: MediaBrowser …, Token=…`. Raporty: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` i WebSocket odrzucają `api_key`: token w nagłówku.

## 4. Uruchamianie

`main` ustawia język (prefs.json, a w przeciwnym razie plik `language` zapisany przez instalator Windows), potem
uruchamia `boot::run`. Ekran `boot` pokazuje 7 punktów: 6 wierzchołków sześciokąta, potem środek. Każdy to
prawdziwa kontrola (`boot::check`):
1. **język** (pytanie, jeśli nieznany): ładują się tłumaczenia wybranego języka (`i18n::check`);
2. **wyświetlanie**: okno dostało kontekst OpenGL (`video::gl_info`, wersja i karta graficzna w dzienniku);
3. **odtwarzacz wideo**: prawdziwy odtwarzacz mpv jest tworzony, inicjalizowany i niszczony (`mpv::self_test`);
4. **pamięć**: plik jest zapisywany, odczytywany i usuwany w folderach konfiguracji, danych i pamięci podręcznej;
5. **konfiguracja**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` czytelne
   (`config::unreadable_files`, wywoływane na samym początku `main`, zanim uszkodzony plik zostanie nadpisany);
6. **sieć**: aktywny interfejs lub trasa na zewnątrz (`discovery::has_network`);
7. **serwer** (środek): adres główny, a w przeciwnym razie zapasowy, odpowiada na `/System/Info/Public` **i** jest
   to ten sam serwer (identyfikator porównany z `server_id` sesji); pomarańczowy, dopóki nie ustawiono serwera.

Czerwony = błąd, z komunikatem i przyciskiem „Dalej” (sam po 12 s). Wszystko zielone: wierzchołki się łączą,
promienie biegną do środka, a punkt serwera staje się wypełnionym sześciokątem — dokładnie logo
(`packaging/turtlefin.svg`, ta sama geometria) —, potem zoom w środek. Slint pomniejsza rysunek `Path` o grubość
linii: ścieżki logo są o tyle powiększone, by trafić w punkty.

Ekran docelowy (`boot::route`), w tej kolejności: nazwa + hasło w wierszu poleceń → logowanie; nazwa zapisanego
konta → to konto (nieznana nazwa: jego formularz logowania); konto startowe (`prefs.autostart_user` /
`autostart_server`) → `fly_autostart` (zdjęcie konta w środku podczas logowania); w przeciwnym razie „Kto ogląda?”
(albo wyszukiwanie serwera, jeśli żaden nie jest znany). Konto startowe, które zniknęło, prowadzi do „Kto
ogląda?”. `--no-intro` (lub wyłączona animacja „Uruchamianie”) pomija animację; kontrole i tak się odbywają.

## 5. Stan w wersji 1.0.0

Wszystkie funkcje z README są gotowe i sprawdzone na zrzutach ekranu (komputer z Windows, ekrany 1366 x 768 i
1920 x 1080) oraz na komputerze z Linuksem ARM podłączonym do telewizora, z kontami testowymi `test` / `test2`
prawdziwego serwera. Wersje opublikowane przez CI: 0.9.0, 0.9.1; wersja 1.0.0 jest gotowa do otagowania
(rozdział 7).

Mechanizmy nieoczywiste w kodzie:
- **Wspólny obraz** (`global Hero`): obraz karty leci do plakatu strony szczegółów i wraca dokładnie na tę kartę
  przy powrocie (`Hero.want-id`, `hero-card-ok`).
- **Rzędy** (`global Rows`): osobne przewijanie każdego rzędu, zapamiętywane po kluczu; przy zmianie rzędu
  zaznaczenie trafia na najbliższą kartę na ekranie (`row-to`, `detail-pick-down`).
- **Offline**: `config::Flags` (userdata.json) przechowuje obejrzane / ulubione / pozycje z flagą „do wysłania”;
  `downloads::sync` wysyła je po powrocie serwera (urządzenie ma ostatnie słowo).
- **Adresy**: `server_main` / `server_backup`; `watch_addresses` (20 s) przełącza na zapasowy i z powrotem.
- **Watch party**: jedno połączenie WebSocket na sesję (`sp_conn`), zatrzymanie przy 401 / 403, rosnące opóźnienie.
- **Zdjęcia profilowe**: pamięć podręczna na dysku `avatar_<id>_still|anim.bin` + okrągła miniatura
  `avatar_<id>_thumb.png`. Miniatura jest pokazywana od razu, pełny GIF dekodowany poza wątkiem UI
  (`avatar_cached_async`), a odświeżany z serwera tylko wtedy, gdy się zmienił (`avatar_fetch`). Animacja GIF-ów:
  `AnimSlot` (Login, Picker, Header, Fly) i jeden wspólny zegar; awatar w nagłówku dostaje wszystkie klatki raz
  (`avatar-frames`) i zmienia się tylko widoczna klatka. GetAvatar `SetAvatar` odpowiada 500 → zastępczo
  `POST /UserImage`.
- **Animowane logowanie** (`fly-phase` od 1 do 4): zdjęcie przesuwa się do środka, pasek ładowania, odlatuje, potem
  pojawia się strona główna, a awatar w nagłówku wyskakuje (`me-pop`).
- **Zmiana języka** (`dust.rs`, `ui/dust.slint`): po otwarciu listy języków paski zakrywają każdy wiersz tekstu, po
  wyborze przyjmują szerokość nowych słów i je odsłaniają. Wiersze są znajdowane przez porównanie zrzutu strony
  (`take_snapshot`) ze zrzutem w czcionce `Turtlefin Blank` (puste znaki, te same szerokości,
  `tools/make-blank-font.py`). Zegar (Montserrat) jest pomijany.
- **Przewodnik** (`tour-step` od 0 do 10): każdy krok przywraca interfejs do oczekiwanego stanu albo zalicza się,
  jeśli gest już wykonano; `tour-ev` jest wywoływane z handlerów `changed`.
- **Animacje**: globalny `Motion` (12 przełączników: boot, pages, menu, select, scroll, panels, detail, player,
  search, login, language, tour), `config::AnimFlags`, presety wbudowane (Wszystkie, Lekkie, Brak) i własne,
  eksportowane / importowane jako `.json` w `Turtlefin Presets`. Każdy czas animacji zapisuje się jako
  `duration: Motion.x ? 300ms : 0ms`.
- **Wprowadzanie tekstu**: na komputerze pola to `TextInput` (tekst ukryty, rysowany przez `TypedText`); w trybie
  TV klawiatura ekranowa (`Osk`). W obu trybach drukowalny klawisz odebrany przez stronę trafia do pola
  (`typing-key`, `erase-key`, `Field.type`); w trybie TV Backspace kasuje tylko, dopóki zostaje tekst. Na
  komputerze kliknięcie obok pola nie odbiera mu klawiatury (`focus-on-click: root.tv-mode`).
- **Przytrzymane klawisze**: Enter, Esc i Backspace nie powtarzają swojej akcji (`event.repeat`), z wyjątkiem
  Backspace przy kasowaniu tekstu.
- **Motywy** (`src/theme.rs`, globalny `Theme` z ui/theme.slint): żadnego koloru wpisanego na stałe w interfejsie;
  półprzezroczyste powierzchnie zapisuje się jako `Theme.fg.with-alpha(…)` (biel w motywie ciemnym, atrament w
  jasnym), tekst na gradiencie akcentu używa `Theme.on-accent`. `Gloss` (żelowy połysk) i `AeroSky` (ui/sky.slint)
  pojawiają się tylko przy `Theme.gloss` / `Theme.bubbles`. Zaimportowane motywy są w prefs.json (`theme`,
  `themes`); kreator (`tools/theme-creator.html`) jest wbudowany w program (`theme::CREATOR`), a „Utwórz motyw”
  zapisuje go w „Turtlefin Themes”. Jego motywy startowe muszą być identyczne z tymi w theme.rs.
- **Mysz**: kliknięcie wszędzie; kółko na stronie głównej, stronie szczegółów (`d-nav`, wspólne z klawiaturą),
  w wyszukiwaniu, bibliotekach, pobranych, ustawieniach; okna na pierwszym planie przechwytują kółko.

Niezrobione: Quick Connect; pad do gier; licencja (do wyboru przez osobę utrzymującą projekt, przed 1.0.0 lub po
niej); połączenie z XeLauncher (programem startowym centrum multimedialnego osoby utrzymującej projekt, bez
priorytetu). Plan na dalej: optymalizacja, wersja na Android TV.

## 6. Tłumaczenia

- Wykonywane w czasie działania przez `src/i18n.rs`: jeden katalog dla Rust i dla interfejsu.
  Slint: globalny `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)`; `Tr.k` zmienia się przy każdej zmianie języka, co przelicza
  teksty. Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- Francuski tekst **jest** kluczem: jego zmiana wymaga zmiany `msgid` w każdym `lang/*.po`
  (`tools/lang-check.py` zgłasza brakujące i nadmiarowe teksty oraz zgubione `{}`). 8 wbudowanych języków (fr, en,
  es, de, it, pt, pl, nl; `BUILTIN` w i18n.rs) jest kompletnych (~530 tekstów).
- Dodane języki: każdy `<code>.po` w folderze `Turtlefin Languages` (`i18n::lang_dir`: Dokumenty, obok pliku exe
  w wersji przenośnej, wewnątrz `TURTLEFIN_CONFIG_DIR` podczas testów; stary folder `languages` jest tam
  przenoszony), łącznie z podfolderami; nazwa czytana z `X-Language-Name`; plik może zastąpić wbudowany język.
  Przeglądane wbudowanym eksploratorem (`ui/langx.slint`, funkcje `lx_*` w main.rs).
- „Utwórz szablon” zapisuje `modele.po`: msgid po **angielsku**, nagłówek `X-Source-Language: en`, uwagi `#.` po
  francusku i w bieżącym języku; `keyed` sprowadza te msgid z powrotem do francuskiego przez `lang/en.po`.
- Teksty brakujące w danym języku: angielski. Liczba mnoga: reguła `Plural-Forms` pliku, obliczana przez i18n.rs.
- Instalator: `[Languages]` i `[CustomMessages]` w `turtlefin.iss`; zapisuje wybrany kod w pliku `language` obok
  pliku exe, odczytywanym przy pierwszym uruchomieniu.
- Nazwy pochodzące z serwera (biblioteki, materiały) nie są tłumaczone.

## 7. Kompilacja, budowanie pakietów, publikacja

Rozwój:
- Windows: Rust (https://rustup.rs), „Build Tools for Visual Studio” (C++), git; `cargo build --release`;
  `libmpv-2.dll` (archiwum `mpv-dev-x86_64-….7z` z
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) obok pliku exe.
  Wersja debug potrzebuje `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev`, potem `cargo build --release`.
- `cargo test --release`: testy i18n, update itd.

Publikacja wersji:
1. Wpisać numer w `Cargo.toml` (`version = "x.y.z"`), raz skompilować (aktualizuje `Cargo.lock`), zrobić commit.
2. `git push origin main`, potem `git tag -a vx.y.z -m "Nowości, jedna w wierszu"` i `git push origin vx.y.z`.
   Opis tagu staje się informacjami o wersji, pokazywanymi przez wbudowaną aktualizację.
3. `release.yml` kompiluje Windows x64 / x86 i Linux x86_64 / aarch64, buduje instalatory, archiwa, AppImage i
   pakiety `.deb` i publikuje je w Release (postęp na karcie Actions repozytorium). Nazwy plików (nagłówek
   `release.yml`) są oczekiwane dokładnie w tej postaci przez `update.rs` i skrypty instalacyjne.
4. Próba bez publikacji: `git push origin main:ci` (wszystko się kompiluje, nic nie jest publikowane).

Ręcznie:
- **Windows**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (lub `x86`);
  potrzebne Inno Setup 6, 7-Zip i NASM (x86). Wynik w `target\dist`.
- **Linux** (na komputerze z Linuksem): `TURTLEFIN_DIST=release cargo build --release`, potem
  `sh packaging/linux/build-appimage.sh <version>` i `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` przy kompilacji, w przeciwnym razie `update::kind()` uzna, że to wersja skompilowana
  lokalnie.
- x86: 32-bitowe libmpv od shinchiro publikowane od lipca 2026 padają przy starcie (OpenSSL); ta z 10 czerwca 2026
  jest przechowywana w wersji wstępnej `libmpv-i686-20260610` Turtlefin (nie usuwać), używanej przez `build.ps1`
  (`MPV_TAG`: inna wersja od shinchiro). aws-lc wymaga NASM w wersji 32-bitowej.
- Ikony: `packaging/turtlefin.svg` to logo; `packaging/icons/make-icons.py <folder>` (Python + Pillow) generuje
  ponownie PNG i ICO.

Aktualizacja kopii skompilowanej lokalnie (`Kind::Source`): zmienione lub nieśledzone pliki są odkładane
(`git stash -u`, do odzyskania przez `git stash pop`) przed `git pull --ff-only`. Nowości pokazują się w osobnym
oknie (Ustawienia → Informacje → Zobacz nowości); `TURTLEFIN_TEST_UPDATE="Wersja x|uwaga|uwaga"` symuluje
aktualizację do wypróbowania. Skróty w kolorach motywu: `theme::apply_shortcuts` (.ico na plikach .lnk w Windows,
ikony `turtlefin` w ~/.local/share/icons w Linuksie), pomijane podczas testów (`TURTLEFIN_CONFIG_DIR`), chyba że
ustawiono `TURTLEFIN_TEST_SHORTCUTS=1`.

Gałęzie: `main` (jedyna gałąź robocza), `ci` (próby CI). `interface-lua` i `libmpv` to stare eksperymenty, już
scalone z `main`: można je usunąć. Tagi: `v0.9.0`, `v0.9.1` (opublikowane wersje) i `libmpv-i686-20260610`
(32-bitowa libmpv, zob. wyżej).

## 8. Testy

- `TURTLEFIN_CONFIG_DIR=<folder>`: inny folder konfiguracji (konta, prefs, języki), bez ruszania prawdziwego.
- `--open=ID|settings|downloads`, `--play=ID@SEKUNDY`, `--test-video=plik` (odtwarzacz bez serwera).
- `TURTLEFIN_DEBUG_FRAMES=1` (klatki > 25 ms); `TURTLEFIN_DEBUG_GAPS=1` z
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed`: przerwy dłuższe niż 40 ms między dwiema klatkami.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: komunikaty watch party. Dwie instancje na jednym komputerze: dwa różne
  `TURTLEFIN_CONFIG_DIR`, `--desktop`.
- `prefs.json` zapisany przez PowerShell 5 ma BOM: odczyt plików konfiguracji go pomija.
- Testy automatyczne w Windows: `SetForegroundWindow` jest akceptowane dopiero po symulowanym klawiszu; używać F24,
  nie Alt (sam Alt przełącza okno w tryb menu i następne kliknięcie przepada). Przytrzymany klawisz symuluje się
  kilkoma kolejnymi wywołaniami `keybd_event` „klawisz wciśnięty” (Windows oznacza je jako powtórzenia).

## 9. Znane problemy / ograniczenia

1. Awaria mpv powoduje awarię Turtlefin (ten sam proces).
2. Odtwarzanie wymaga renderowania OpenGL (`SLINT_BACKEND=winit-software` je uniemożliwia).
3. **mpv 0.40 / 0.41** (naprawione w mpv 23 stycznia 2026, commit f74adc4): na każdą klatkę bariera OpenGL, która
   nigdy nie jest zwalniana; ze sterownikiem v3d każda zajmuje deskryptor („MESA: error: Export failed” po ~42 s).
   Obejście w `src/mpv.rs` (tylko OpenGL ES). Diagnostyka: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` musi
   pozostawać stałe podczas odtwarzania.
4. Wzrost pamięci RAM mpv (~3 MB/min) przy napisach ASS: ograniczony do jednego odtwarzania (mpv tworzony od nowa
   dla każdego wideo).
5. Token jawnym tekstem w `session.json` / `accounts.json` (0600 w systemach Unix).
6. Rozrywanie obrazu pod Xorg bez kompozytora (sam Openbox): użyć kompozytora (picom `--backend egl --vsync`).
   Turtlefin utrzymuje 60 klatek/s.
7. Dekodowanie programowe w Linuksie ARM: może nie nadążać przy 4K / HEVC; trop: `TURTLEFIN_HWDEC=auto-copy`.
8. Mikroprzerwy (~40 ms) mierzone tylko przy renderowaniu wymuszonym na pełną szybkość, przy każdej klatce GIF-a w
   nagłówku i przy łączeniu logo; przyczyny nie znaleziono, niewidoczne podczas normalnego użytkowania.

## 10. Preferencje pracy osoby utrzymującej projekt

- Odpowiedzi po francusku; niewielkie doświadczenie z Linuksem / SSH: tłumaczyć polecenia.
- Nic nie wysyłać na GitHuba i nie publikować wersji bez wyraźnej zgody; wersje publikuje osoba utrzymująca projekt.
- Testy na kopii konfiguracji (`TURTLEFIN_CONFIG_DIR`), nigdy na prawdziwej; konta testowe `test` / `test2`.
- Po każdej serii zmian: testowy instalator na Pulpicie osoby utrzymującej projekt
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, poprzedni usunięty).
