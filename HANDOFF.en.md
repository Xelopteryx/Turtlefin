# Turtlefin: project handoff (state on October 9, 2026, version 1.0.0)

[Français](HANDOFF.md) · **English** · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

For whoever takes over development (a person or Claude Code). Read it entirely before touching the code, then
read [README.md](README.md) (usage, installation, keys, files).
Repository: https://github.com/Xelopteryx/Turtlefin · Version in `Cargo.toml`: 1.0.0.

## 1. Goal

A **native Jellyfin client in Rust**, lightweight, animated, usable with a remote as well as with a keyboard and
mouse, installable on any Windows or Linux computer **without compiling anything**: the user downloads an
installer or a package (or runs an install command), that's all. Every package is built by GitHub CI (or the
maintainer's PC), never by the user.

Why: Jellyfin Desktop (Qt / QtWebEngine) leaks RAM and ends up crashing on small machines, and the web interface
with a heavy theme drops below 30 fps on modest hardware. Permanent rule: stay stable in memory and never bring
back real-time blur or filter animations.

## 2. Decisions

| Topic | Decision | Reason |
|---|---|---|
| Language / UI | Rust + **Slint** `~1.18` (100 % Slint rendering), `fluent-dark` style forced by `build.rs` | No browser; the “native” style would depend on Qt |
| Slint feature `unstable-winit-030` | winit event filter for **F11** (`install_f11`) | The only way to get a global key; hence `~1.18` (unstable API between minor versions) |
| Network | `reqwest` 0.13 (rustls, system certificate store), `tokio` | `query` is a feature to enable in 0.13 |
| Playback | **libmpv loaded at runtime** (`libloading`, `src/mpv.rs`), OpenGL rendering into a texture shown by Slint (`src/video.rs`), Slint controls on top (`ui/player.slint`) | Embedded player, no IPC, works on Wayland. The player is recreated for each playback (bounded memory) |
| Slint renderer | femtovg (OpenGL / GLES) forced unless `SLINT_BACKEND` is set | Video goes through an OpenGL texture |
| Video texture | Physical pixels, `TopLeft` origin, GL state saved / restored around mpv; `loadfile` waits for the render context | Otherwise upside-down image or “No render context set” |
| Decoding | `hwdec=no` on 64-bit Linux ARM, `auto-safe` elsewhere (Windows: `d3d11va-copy`) | On the tested ARM boards (v3d driver), V4L2 decoding outputs a format the renderer cannot import |
| Linux audio | `ao=pipewire,pulse,alsa`, `config=no` | A user `mpv.conf` forcing ALSA failed when PipeWire holds the HDMI output |
| Memory | mpv cache capped (100 / 25 MiB), images requested at the right size, 16 items per row | Small machines (4 GB) |
| Languages | French in the code (source language), runtime translation by `src/i18n.rs` from `lang/<code>.po` (built in) and the `Turtlefin Languages` folder | See section 6 |
| Full screen (Windows) | Borderless window covering the screen + 1 px (`src/winfull.rs`, `set_tv_window`), not true full screen; follows resolution changes (every 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` for Slint's full screen | In OpenGL full screen, AMD Software treats the app as a game: “Press ALT + R” every time it comes back to the foreground |
| TV / computer interface | `tv-mode` only changes the size (`k` = 1.4) and the input (on-screen keyboard); full screen is separate (`full_flag`, `UiPrefs::fullscreen`, F11) | User request: full screen without enlarging the interface |
| Window (Windows, desktop) | Shrunk and centered when 1280 x 720 + frame exceeds the work area; keeps animating while moved (`winfull::keep_alive_while_moving`) | 1366 x 768 screens; Windows blocks the event loop while a window is moved |
| Console window (Windows) | “windows” subsystem in release; `--console` attaches / opens one; external commands without a window (`paths::quiet_command`) | No console and no flashing CMD window |
| Command line | Always overrides settings (startup account, TV interface) | User request |
| Password | Never saved (token only); on the command line, prefer `TURTLEFIN_PASSWORD` | An argument is visible to other processes |

## 3. Code layout

```
build.rs            compiled commit, Slint style, exe icon (winresource, Windows)
lang/<code>.po      built-in translations (source: the French in the code); tools/lang-check.py checks them
ui/theme.slint      theme tokens, globals Tr (translation) and Motion (enabled animations)
ui/app.slint        AppWindow and every screen (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo: startup animation (7 dots, joining, zoom), language choice
ui/player.slint     playback screen        ui/osk.slint      on-screen keyboard
ui/card.slint       poster card            ui/marquee.slint  scrolling text
ui/typed.slint      animated typed text (Str, TypedText)   ui/langx.slint  explorer (languages, presets)
ui/dust.slint       language-change bars                   ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, shared App state (Arc), screens, navigation (stack + kept pages), settings
src/boot.rs         startup sequence (checks, language, landing screen)
src/i18n.rs         current language, tr() / trf() / trn(), added languages, translation template
src/dust.rs         language-change effect (finding text lines, morphing)
src/api.rs          Jellyfin REST client (+ Seerr relay of Jellyfin Enhanced, GetAvatar)
src/config.rs       session, accounts (12 max), prefs.json (UiPrefs, AnimFlags, presets), tracks, offline watched/favorites
src/discovery.rs    server discovery (UDP, subnets, ARP, VPN peers)
src/downloads.rs    downloads (Range resume, queue, offline sync)
src/mpv.rs          libmpv binding; src/video.rs OpenGL texture; src/player.rs playback, reports, chaining
src/syncplay.rs     watch party (WebSocket /socket)
src/paths.rs        config / cache / data folders; portable mode; quiet_command
src/update.rs       update by installation kind (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows: screen / work area, window animated while moved
src/theme.rs        built-in themes, .tftheme files (ThemeDef), applied to the Theme global
ui/sky.slint        Frutiger Aero scenery (sky, hill, bubbles), static
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   build and publish on a `v*` tag (trial: `ci` branch)
```

Principles:
- Network data goes through `Send` structures, then `upgrade_in_event_loop` pushes it into the Slint models.
  Images decoded off the UI thread, 6 parallel downloads, applied with an id guard.
- `App.gen` invalidates stale loads; `App.stack` is the navigation stack; `PAGES` keeps detail and library pages
  for request-free returns.
- Keyboard navigation is hand-made (selection indices in Rust and Slint), since Slint does not handle focus for
  dynamic cards. Each screen has its `FocusScope`; `refocus` gives the keyboard back to the right place.
- Slint `changed` handlers are deferred: do not rely on their order (two-step positions, etc.).
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`, header
  `Authorization: MediaBrowser …, Token=…`. Reports: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` and the WebSocket refuse `api_key`: token in the header.

## 4. Startup

`main` applies the language (prefs.json, otherwise the `language` file written by the Windows installer), then
runs `boot::run`. The `boot` screen shows 7 dots: the 6 corners of the hexagon, then the center. Each is a real
check (`boot::check`):
1. **language** (asked if unknown): the chosen language's translations load (`i18n::check`);
2. **display**: the window got an OpenGL context (`video::gl_info`, version and graphics card in the log);
3. **video player**: a real mpv player is created, initialized, then destroyed (`mpv::self_test`);
4. **storage**: a file is written, read back and deleted in the config, data and cache folders;
5. **configuration**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` readable
   (`config::unreadable_files`, called at the very start of `main`, before a damaged file is rewritten);
6. **network**: an active interface or a route outside (`discovery::has_network`);
7. **server** (the center): the main address, otherwise the backup one, answers `/System/Info/Public` **and** it
   is the same server (id compared with the session's `server_id`); orange while no server is configured.

Red = failure, with a message and “Continue” (on its own after 12 s). All green: the corners join, the spokes go
to the center and the server dot becomes the filled hexagon — exactly the logo (`packaging/turtlefin.svg`, same
geometry) —, then a zoom into the center. Slint shrinks a `Path` drawing by its stroke width: the logo paths are
enlarged by that much to land on the dots.

Landing screen (`boot::route`), in order: name + password on the command line → sign-in; a saved account's name
→ that account (unknown name: its sign-in form); startup account (`prefs.autostart_user` / `autostart_server`)
→ `fly_autostart` (the account's picture in the center while signing in); otherwise “Who's watching?” (or server
discovery if none is known). A startup account that disappeared leads to “Who's watching?”. `--no-intro` (or the
“Startup” animation turned off) skips the animation; the checks still run.

## 5. State at version 1.0.0

Every feature in the README is done and checked on screenshots (Windows PC, 1366 x 768 and 1920 x 1080 screens)
and on a Linux ARM machine plugged into a TV, with the `test` / `test2` test accounts of a real server. Versions
published by the CI: 0.9.0, 0.9.1; 1.0.0 is ready to be tagged (section 7).

Mechanisms that are not obvious in the code:
- **Shared image** (`global Hero`): a card's image flies to the detail page's poster and back onto the exact card
  on return (`Hero.want-id`, `hero-card-ok`).
- **Rows** (`global Rows`): per-row scrolling, remembered by key; moving to another row lands on the closest card
  on screen (`row-to`, `detail-pick-down`).
- **Offline**: `config::Flags` (userdata.json) keeps watched / favorites / positions with a “to send” flag;
  `downloads::sync` sends them when the server is back (the device has the last word).
- **Addresses**: `server_main` / `server_backup`; `watch_addresses` (20 s) switches to the backup and back.
- **Watch party**: one WebSocket connection per session (`sp_conn`), stops on 401 / 403, growing delay.
- **Profile pictures**: disk cache `avatar_<id>_still|anim.bin` + round thumbnail `avatar_<id>_thumb.png`. The
  thumbnail is shown at once, the full GIF is decoded off the UI thread (`avatar_cached_async`), then refreshed
  from the server only if it changed (`avatar_fetch`). GIF animation: `AnimSlot` (Login, Picker, Header, Fly) and
  one shared timer; the header avatar receives all its frames once (`avatar-frames`) and only the visible frame
  changes. GetAvatar `SetAvatar` answers 500 → fallback `POST /UserImage`.
- **Animated sign-in** (`fly-phase` 1 to 4): the picture moves to the center, loading bar, flies away, then the
  home screen arrives and the header avatar pops in (`me-pop`).
- **Language change** (`dust.rs`, `ui/dust.slint`): when the language list opens, bars cover every line of text,
  take the width of the new words once one is chosen, then reveal them. Lines are found by comparing a snapshot
  of the page (`take_snapshot`) with a snapshot in the `Turtlefin Blank` font (empty glyphs, same widths,
  `tools/make-blank-font.py`). The clock (Montserrat) is excluded.
- **Guided tour** (`tour-step` 0 to 10): each step puts the interface back in the expected state or completes if
  the gesture is already done; `tour-ev` is called from `changed` handlers.
- **Animations**: global `Motion` (12 switches: boot, pages, menu, select, scroll, panels, detail, player, search,
  login, language, tour), `config::AnimFlags`, built-in presets (All, Light, None) and personal ones, exported /
  imported as `.json` in `Turtlefin Presets`. Every animation duration is written
  `duration: Motion.x ? 300ms : 0ms`.
- **Input**: on the desktop, fields are `TextInput`s (hidden text, drawn by `TypedText`); in TV mode, the
  on-screen keyboard (`Osk`). In both modes, a printable key received by the page goes to the field
  (`typing-key`, `erase-key`, `Field.type`); in TV mode Backspace only erases while text remains. On the desktop,
  clicking next to a field does not take the keyboard away from it (`focus-on-click: root.tv-mode`).
- **Held keys**: Enter, Esc and Backspace do not repeat their action (`event.repeat`), except Backspace erasing
  text.
- **Themes** (`src/theme.rs`, global `Theme` in ui/theme.slint): no hard-coded color in the interface; translucent
  surfaces are written `Theme.fg.with-alpha(…)` (white on a dark theme, ink on a light one), text on the accent
  gradient uses `Theme.on-accent`. `Gloss` (gel highlight) and `AeroSky` (ui/sky.slint) only show when
  `Theme.gloss` / `Theme.bubbles` are set. Imported themes are kept in prefs.json (`theme`, `themes`); the creator
  (`tools/theme-creator.html`) is embedded in the program (`theme::CREATOR`) and dropped into “Turtlefin Themes” by
  “Create a theme”. Its starting themes must stay identical to those in theme.rs.
- **Mouse**: click everywhere; wheel on home, detail page (`d-nav`, shared with the keyboard), search, libraries,
  downloads, settings; foreground windows swallow the wheel.

Not done: Quick Connect; gamepad; license (to be chosen by the maintainer, before or after 1.0.0); XeLauncher
bridge (the maintainer's media center launcher, low priority). Planned next: optimization, Android TV version.

## 6. Translations

- Done at runtime by `src/i18n.rs`: one catalog for Rust and for the interface.
  Slint: global `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)`; `Tr.k` changes at every language change, which recomputes the
  texts. Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- The French text **is** the key: changing it means changing the `msgid` in every `lang/*.po`
  (`tools/lang-check.py` reports missing and extra texts and lost `{}`). The 8 built-in languages (fr, en, es,
  de, it, pt, pl, nl; `BUILTIN` in i18n.rs) are complete (~530 texts).
- Added languages: any `<code>.po` in the `Turtlefin Languages` folder (`i18n::lang_dir`: Documents, next to the
  exe when portable, under `TURTLEFIN_CONFIG_DIR` during tests; the old `languages` folder is moved there),
  subfolders included; name read from `X-Language-Name`; a file can replace a built-in language. Browsed by a
  built-in explorer (`ui/langx.slint`, the `lx_*` functions in main.rs).
- “Create the template” writes `modele.po`: msgids in **English**, header `X-Source-Language: en`, `#.` notes in
  French and in the current language; `keyed` maps those msgids back to French through `lang/en.po`.
- Texts missing from a language: English. Plurals: the file's `Plural-Forms` rule, evaluated by i18n.rs.
- Installer: `[Languages]` and `[CustomMessages]` in `turtlefin.iss`; it writes the chosen code to `language` next
  to the exe, picked up on first launch.
- Names coming from the server (libraries, media) are not translated.

## 7. Building, packaging, publishing

Development:
- Windows: Rust (https://rustup.rs), “Visual Studio Build Tools” (C++), git; `cargo build --release`;
  `libmpv-2.dll` (archive `mpv-dev-x86_64-….7z` from
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) next to the exe.
  The debug build needs `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev`, then `cargo build --release`.
- `cargo test --release`: tests for i18n, update, etc.

Publishing a version:
1. Put the number in `Cargo.toml` (`version = "x.y.z"`), build once (updates `Cargo.lock`), commit.
2. `git push origin main`, then `git tag -a vx.y.z -m "What's new, one per line"` and `git push origin vx.y.z`.
   The tag message becomes the release notes, shown by the built-in updater.
3. `release.yml` builds Windows x64 / x86 and Linux x86_64 / aarch64, makes installers, archives, AppImages and
   `.deb` packages, and publishes them in a Release (follow it in the repository's Actions tab). File names
   (header of `release.yml`) are expected as is by `update.rs` and the install scripts.
4. Trial without publishing: `git push origin main:ci` (everything is built, nothing is published).

By hand:
- **Windows**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (or `x86`);
  needs Inno Setup 6, 7-Zip and NASM (x86). Output in `target\dist`.
- **Linux** (on a Linux machine): `TURTLEFIN_DIST=release cargo build --release`, then
  `sh packaging/linux/build-appimage.sh <version>` and `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` at build time, otherwise `update::kind()` thinks it is a locally built version.
- x86: shinchiro's 32-bit libmpv builds published since July 2026 crash on startup (OpenSSL); the June 10, 2026
  one is kept in Turtlefin's `libmpv-i686-20260610` pre-release (do not delete it), used by `build.ps1`
  (`MPV_TAG`: another shinchiro version). aws-lc needs NASM in 32-bit.
- Icons: `packaging/turtlefin.svg` is the logo; `packaging/icons/make-icons.py <folder>` (Python + Pillow)
  regenerates the PNGs and the ICO.

Branches: `main` (the only working branch), `ci` (CI trials). `interface-lua` and `libmpv` are old experiments,
already merged into `main`: they can be deleted. Tags: `v0.9.0`, `v0.9.1` (published versions) and
`libmpv-i686-20260610` (32-bit libmpv, see above).

## 8. Testing

- `TURTLEFIN_CONFIG_DIR=<folder>`: another config folder (accounts, prefs, languages) without touching the real one.
- `--open=ID|settings|downloads`, `--play=ID@SECONDS`, `--test-video=file` (player without a server).
- `TURTLEFIN_DEBUG_FRAMES=1` (frames > 25 ms); `TURTLEFIN_DEBUG_GAPS=1` with
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed`: pauses over 40 ms between two frames.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: watch party messages. Two instances on one PC: two different
  `TURTLEFIN_CONFIG_DIR`, `--desktop`.
- A `prefs.json` written by PowerShell 5 has a BOM: reading the config files ignores it.
- Automated tests on Windows: `SetForegroundWindow` is only accepted after a simulated key; use F24, not Alt (Alt
  alone puts the window in menu mode and the next click is lost). A held key is simulated with several
  successive “key down” `keybd_event` calls (Windows marks them as repeats).

## 9. Known issues / limits

1. An mpv crash crashes Turtlefin (same process).
2. Playback needs OpenGL rendering (`SLINT_BACKEND=winit-software` prevents it).
3. **mpv 0.40 / 0.41** (fixed in mpv on January 23, 2026, commit f74adc4): one OpenGL fence per frame never
   released; with the v3d driver each one holds a descriptor (“MESA: error: Export failed” after ~42 s).
   Workaround in `src/mpv.rs` (OpenGL ES only). Diagnosis: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` must
   stay stable during playback.
4. mpv RAM growth (~3 MB/min) with ASS subtitles: bounded to one playback (mpv recreated for each video).
5. Token in plain text in `session.json` / `accounts.json` (0600 on Unix).
6. Tearing under Xorg without a compositor (bare Openbox): use a compositor (picom `--backend egl --vsync`).
   Turtlefin holds 60 fps.
7. Software decoding on Linux ARM: may struggle with 4K / HEVC; lead: `TURTLEFIN_HWDEC=auto-copy`.
8. Micro-pauses (~40 ms) measured only with rendering forced to full speed, on each frame of a header GIF and when
   the logo joins; cause not found, invisible in normal use.

## 10. Maintainer's working preferences

- Answers in French; little experience with Linux / SSH: explain the commands.
- Never push to GitHub or publish a version without the maintainer's explicit approval; the maintainer publishes
  the versions.
- Test with a copy of the config (`TURTLEFIN_CONFIG_DIR`), never the real one; test accounts `test` / `test2`.
- After each batch of changes: a test installer on the maintainer's Desktop
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, the old one deleted).
