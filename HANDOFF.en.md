# Turtlefin: project handoff (state on October 7, 2026, version 0.9.1)

[Français](HANDOFF.md) · **English**

For whoever takes over development (human or Claude Code). Read it fully before touching the code, then read
[README.md](README.md) (usage, install, keys, files). The French version is the reference if they ever differ.
Repository: https://github.com/Xelopteryx/Turtlefin · Version in `Cargo.toml`: 0.9.1.

## 1. Goal

A **native Jellyfin client in Rust**, lightweight, animated and fully usable with a keyboard / remote, installable
on any Windows or Linux computer **without building anything**: users download an installer or a package (or run
one install command), that's all. Every package is built by the maintainer (GitHub CI or their PC), never by users.

Why: Jellyfin Desktop (Qt / QtWebEngine) leaks RAM and eventually crashes on small machines, and the web interface
with a heavy theme drops below 30 fps on modest hardware. Standing rule: stay memory-stable and never bring back
real-time blur or filter animations.

## 2. Decisions

| Topic | Decision | Reason |
|---|---|---|
| Language / UI | Rust + **Slint** 1.18 (100 % Slint rendering), `fluent-dark` style forced by `build.rs` | No browser; the “native” style would depend on Qt |
| Network | `reqwest` 0.13 (rustls, system certificate store), `tokio` | `query` is an opt-in feature in 0.13 |
| Playback | **libmpv loaded at runtime** (`libloading`, `src/mpv.rs`), OpenGL rendering into a texture shown by Slint (`src/video.rs`), Slint controls on top (`ui/player.slint`) | Built-in player, no IPC, works on Wayland. The player is recreated for each playback (bounded memory) |
| Slint renderer | femtovg (OpenGL / GLES) forced unless `SLINT_BACKEND` is set | Video goes through an OpenGL texture |
| Video texture | Physical pixels, `TopLeft` origin, GL state saved / restored around mpv; `loadfile` waits for the render context | Otherwise upside-down image or “No render context set” |
| Decoding | `hwdec=no` on 64-bit ARM Linux, `auto-safe` elsewhere (Windows: `d3d11va-copy`) | On the ARM boards tested (v3d driver), V4L2 decoding outputs a format the renderer cannot import |
| Linux audio | `ao=pipewire,pulse,alsa`, `config=no` | A user `mpv.conf` forcing ALSA failed while PipeWire held the HDMI output |
| Memory | mpv cache capped (100 / 25 MiB), images requested at the right size, 16 items per row | Small machines (4 GB) |
| Languages | Texts written in French in the code (source language), gettext translations in `lang/<code>/LC_MESSAGES/turtlefin.po`, built in | See section 6 |
| TV interface (Windows) | Borderless window covering the screen + 1 px (`src/winfull.rs`, `set_tv_window`), not true full screen; follows resolution changes (every 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` for Slint's full screen | In OpenGL full screen, AMD Software treats the app as a game: “Press ALT + R” every time it comes back to the foreground |
| Window (Windows, desktop) | Shrunk and centred when 1280 x 720 + frame exceeds the work area | 1366 x 768 screens: the bottom went under the taskbar |
| Console window (Windows) | “windows” subsystem in release; `--console` attaches / opens one | User request: no console unless asked |
| Command line | Always wins over settings (startup account, TV interface) | User request |
| Password | Never stored (token only); on the command line, prefer `TURTLEFIN_PASSWORD` | Arguments are visible to other processes |

## 3. Code layout

```
build.rs          built commit, Slint style, bundled translations, exe icon (winresource, Windows)
lang/<code>.po    translations (source: the French in the code); tools/lang-check.py checks them
ui/theme.slint    theme tokens, global Prefs
ui/app.slint      AppWindow and every screen (boot, login, loading, home, detail, library, settings…)
ui/boot.slint     BootLogo: startup animation (7 dots, linking, zoom), language picker
ui/player.slint   playback screen; ui/osk.slint on-screen keyboard; ui/card.slint, ui/marquee.slint
src/main.rs       CLI, shared App state (Arc), screens, navigation (stack + kept pages), settings
src/boot.rs       startup sequence (checks, language, landing screen)
src/i18n.rs       current language, Rust-side tr() / trf(), system / installer language
src/api.rs        Jellyfin REST client (+ Jellyfin Enhanced Seerr relay, GetAvatar)
src/config.rs     session, accounts (12 max), prefs.json (device settings), tracks, offline watched/favorites
src/discovery.rs  server search (UDP, subnets, ARP, VPN peers)
src/downloads.rs  downloads (Range resume, queue, offline sync)
src/mpv.rs        libmpv binding; src/video.rs OpenGL texture; src/player.rs playback, reports, chaining
src/syncplay.rs   watch party (WebSocket /socket)
src/paths.rs      config / cache / data folders; portable mode (`portable` file next to the exe)
src/update.rs     update per install kind (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
packaging/        windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                  icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh (one command)
.github/workflows/release.yml   build and publish on a `v*` tag
```

Principles:
- Network data goes through `Send` structs, then `upgrade_in_event_loop` pushes it into Slint models. Images are
  decoded off the UI thread, 6 downloads in parallel, applied with an id guard.
- `App.gen` invalidates stale loads; `App.stack` is the navigation stack; `PAGES` keeps detail and library pages
  so going back needs no request.
- Keyboard navigation is hand-made (selection indices in Rust), since Slint does not handle focus across dynamic
  cards. `refocus` gives the keyboard back to the right `FocusScope`.
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`,
  `Authorization: MediaBrowser …, Token=…` header. Reports: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` and the WebSocket refuse `api_key`: token in the header.

## 4. Startup

`main` applies the language (prefs.json, otherwise the `language` file written by the Windows installer), then
starts `boot::run`. The `boot` screen shows 7 dots: the 6 corners of the hexagon, then the centre. Each one is a
real check (`boot::check`):
1. **language** (asked if unknown): the chosen language's translations load (`i18n::check`);
2. **display**: the window got an OpenGL context (`video::gl_info`, version and graphics card in the log);
3. **video player**: a real mpv player is created, initialized and destroyed (`mpv::self_test`);
4. **storage**: a file is written, read back and deleted in the config, data and cache folders;
5. **configuration**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` are readable
   (`config::unreadable_files`, called at the very start of `main`, before a damaged file gets rewritten);
6. **network**: an active interface or a route to the outside (`discovery::has_network`);
7. **server** (the centre): the main address, otherwise the backup, answers `/System/Info/Public` **and** it is
   the same server (id compared with the session's `server_id`); “to configure” on first run.

Red = failure, with a message and “Continue” (automatic after 12 s). All green: the corners link up, the spokes
grow towards the centre and the server dot becomes the filled hexagon — exactly the logo
(`packaging/turtlefin.svg`, same geometry) —, then the view zooms into the centre. Beware: Slint shrinks a `Path`'s
drawing by its stroke width; the logo paths are enlarged by that much so they land on the dots.

Landing screen (`boot::route`), in order: name + password on the command line → sign in; name of a saved account →
that account; startup account (`prefs.autostart_user` / `autostart_server`, setting Account → “Open this account
at startup”) → `open_saved_session`; otherwise “Who's watching?” (or the server search if none is known). A
startup account that is gone (forgotten locally or token refused by the server) leads to “Who's watching?”.
`--no-intro` skips the animation (the checks still run).

## 5. State

Every feature listed in the README is done and was checked on screenshots (Windows PC) and on an ARM Linux machine
plugged into a TV, with the `test` / `test2` accounts of a real server. Notable points, not obvious from the code:
- **Shared image** (`global Hero`): a card's image flies to the detail poster and back onto the exact card
  (`Hero.want-id`, `hero-card-ok`). Slint `changed` handlers are deferred: positions are taken in two steps.
- **Rows** (`global Rows`): per-row scrolling, remembered by key; moving between rows picks the closest card on
  screen.
- **Offline**: `config::Flags` (userdata.json) keeps watched / favorites / positions with a “to send” flag;
  `downloads::sync` sends them back when the server returns (the device wins).
- **Addresses**: `server_main` / `server_backup`; `watch_addresses` (20 s) switches to the backup and back.
- **Watch party**: one WebSocket connection per session (`sp_conn`), stops on 401 / 403, growing retry delay.
- **Avatars**: GIFs decoded once, only the selected avatar is animated; round still images cached on disk.
  GetAvatar `SetAvatar` answers 500 → fallback `POST /UserImage`.
- **Version 0.9.0**: Windows / Linux packages and updates through GitHub Releases. Checked locally: x64 installer
  (installed and portable, uninstall), x86, aarch64 AppImage, arm64 `.deb` (contents). **CI has never run**
  (nothing pushed) and Release-based updates could not be tried without a release.
- **October 7, 2026**: startup animation, languages (French / English, ~400 strings), startup account, console
  only with `--console`, logo = icon (exe, window, installer, Linux packages), bilingual installer passing its
  language on, one-command install scripts, README / HANDOFF in two languages.

- **October 7, 2026 (evening)**: playback at the screen's refresh rate (`Render::render`: mpv draws only new
  frames, `BLOCK_FOR_TARGET_TIME=0`; before, the UI was capped at the video's rate, 26 fps); volume (button + bar,
  `prefs.volume`), Audio / Subtitles icons, animated presses and panels; detail page moving aside when playback
  starts (`play-go`, `po`); text fragmentation while choosing the language (`Tr.fx`, `Tr.k`, `i18n::fragment`);
  menu and category icons (`NavIcon`); guided tour (`tour-step`, offered at startup, `--tutorial`, Settings →
  About); orange server dot while no server is chosen.

- **October 8, 2026**: language change (`dust.rs`, `ui/dust.slint`) — when the language list opens, a glowing bar
  runs along each text line and covers it in white; on choosing, the blocks take the width of the new words and
  the bar reveals them (on cancel, the old ones). Lines are found by capturing the page (`take_snapshot`) as is,
  then with a font with empty glyphs and identical widths (`Turtlefin Blank`, `tools/make-blank-font.py`,
  `dust-blank`), grouped into lines;
  Slint does the animation (a few rectangles, nothing while choosing). Blocks under the list (`dust-layer`), list
  hidden during captures (`dust-snap`); value pills with animated width. Replaces the fragmentation (`Tr.fx`) and a
  first grain-cloud attempt, too costly (a whole-window image every frame). Tour redone: it shows
  the UI (selection moving by itself, menu opened, `tour-tick`), speaks remote (arrows / OK / Back drawing),
  highlights follow the real positions (`tabs-x`, `right-w`), Back / Skip / Next buttons (← →, OK). Home shown at once when going
  back (no longer the previous page during the reload). Menu cogwheel redrawn. Fixed while testing everything
  (two instances, offline...): empty menu offline, account removed from “Who's watching?” when its token is
  refused, password field without keyboard focus (desktop), `turtlefin "Name"` not saved opening another
  account, Back on “Who's watching?” after “Switch account”, “Remembered for the whole series” on a movie,
  update check of an unpublished local build shown as a failure (GitHub 404 detected regardless of language).
  Watch party checked with two clients.

Not done: Quick Connect; gamepad; optional blurred background with transparent logos; licence (maintainer's
choice); XeLauncher bridge (the maintainer's media-center launcher, low priority).

## 6. Translations

- Done at runtime by `src/i18n.rs` (no longer by Slint): one catalog for Rust and the interface. Slint: global
  `Tr` (ui/theme.slint) — `Tr.t(Tr.l, "…")`, `Tr.f(Tr.l, "… {} …", a, b)`, `Tr.p(Tr.l, "{n} serveur",
  "{n} serveurs", n)`; `Tr.l` changes on every language switch, which re-evaluates the texts (wired in `main`).
  Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- The French text **is** the key: changing it means changing the `msgid` in every `lang/*.po`
  (`tools/lang-check.py` reports missing texts and lost `{}`).
- Built-in languages: `lang/<code>.po` + `BUILTIN` in i18n.rs (fr, en, es, de, it, pt, pl, nl). Added languages:
  any `<code>.po` in the `languages` folder (config, or next to the exe), name read from `X-Language-Name`; a
  file can override a built-in language. “Add a language” writes `modele.po` (**English** msgids,
  `X-Source-Language: en` header, `#.` notes in French and in the current language; `keyed` maps those msgids
  back to French through `lang/en.po`) and copies Turtlefin `.po` files found in Downloads / Desktop
  (`import_languages`): no file manager needed.
- Texts missing from a language: English. Plurals: the file's `Plural-Forms` rule, evaluated by i18n.rs.
- Installer: `[Languages]` and `[CustomMessages]` of `turtlefin.iss` (Inno Setup languages); it writes the chosen
  code to `language` next to the exe, picked up on first launch.
- Names coming from the server (libraries, media) are not translated.

## 7. Building and packaging (maintainer only)

Development:
- Windows: Rust (https://rustup.rs), Visual Studio Build Tools (C++), git; `cargo build --release`;
  `libmpv-2.dll` (archive `mpv-dev-x86_64-….7z` from
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) next to the exe.
  Debug builds need `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev`, then `cargo build --release`.

Publishing:
- **Automatic**: `git tag -a v0.9.2 -m "What's new…" && git push origin v0.9.2` (the tag message becomes the release notes, shown by the built-in updater). `release.yml` builds Windows x64 / x86 and Linux
  x86_64 / aarch64, makes installers, archives, AppImages and `.deb` packages, and publishes them in a Release.
  File names (header of `release.yml`) are expected as-is by `update.rs` and the install scripts.
- **Windows by hand**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (or
  `x86`); needs Inno Setup 6, 7-Zip and NASM (x86). Output in `target\dist`.
- **Linux by hand** (on a Linux machine): `TURTLEFIN_DIST=release cargo build --release`, then
  `sh packaging/linux/build-appimage.sh <version>` and `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` at build time, otherwise `update::kind()` assumes a build from source.
- x86: shinchiro's 32-bit libmpv builds published since July 2026 crash at startup (OpenSSL); `build.ps1` pins the
  June 10, 2026 one, removed by shinchiro (later ones still broken in October), is kept in Turtlefin's
  `libmpv-i686-20260610` pre-release, used by `build.ps1` (`MPV_TAG`: another shinchiro version). aws-lc needs NASM for 32-bit builds.
- Icons: `packaging/turtlefin.svg` is the logo; `packaging/icons/make-icons.py <folder>` (Python + Pillow)
  regenerates the PNGs and the ICO.

## 8. Testing

- `TURTLEFIN_CONFIG_DIR=<folder>`: another config folder (accounts, prefs) without touching the real one.
- `--open=settings|downloads`, `--play=ID@SECONDS`, `--test-video=file` (player without a server).
- `TURTLEFIN_DEBUG_FRAMES=1` (frames > 25 ms), `SLINT_DEBUG_PERFORMANCE=refresh_full_speed,console`.
- A `prefs.json` written by PowerShell 5 has a BOM: config reading ignores it.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: watch-party messages received and requests sent (failures are always logged).
  Two instances on one PC: two different `TURTLEFIN_CONFIG_DIR`, `--desktop`.
- Automated tests on Windows: `SetForegroundWindow` is only allowed after a key press (Alt); without it,
  simulated keys go to another window (fake “frozen keyboard”).

## 9. Known issues / limits

1. An mpv crash brings Turtlefin down (same process).
2. Playback needs OpenGL rendering (`SLINT_BACKEND=winit-software` prevents it).
3. **mpv 0.40 / 0.41** (fixed in mpv on January 23, 2026, commit f74adc4): one OpenGL fence per frame never
   released; with the v3d driver each one holds a file descriptor (“MESA: error: Export failed” after ~42 s).
   Workaround in `src/mpv.rs` (OpenGL ES only). Check: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` must stay stable
   during playback.
4. mpv RAM growth (~3 MB/min) with ASS subtitles: bounded to one playback (mpv recreated for each video).
5. Token stored in clear in `session.json` / `accounts.json` (0600 on Unix).
6. Tearing under Xorg without a compositor (bare Openbox): use a compositor (picom `--backend egl --vsync`).
   Turtlefin holds 60 fps.
7. Software decoding on ARM Linux: may struggle with 4K / HEVC; lead: `TURTLEFIN_HWDEC=auto-copy`.

## 10. Maintainer's working preferences

- Answers in French; little Linux / SSH experience: explain commands.
- Never push to GitHub before their explicit approval.
- Test with a copy of the config (`TURTLEFIN_CONFIG_DIR`), never the real one; test accounts `test` / `test2`.
