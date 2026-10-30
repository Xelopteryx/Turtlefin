<p align="center"><img src="packaging/icons/turtlefin-aero-256.png" width="96" alt="Turtlefin logo (Frutiger Aero theme)">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin logo">&nbsp;&nbsp;&nbsp;<img src="packaging/icons/turtlefin-nerd-256.png" width="96" alt="Turtlefin logo (Nerd theme)"></p>

# Turtlefin

**English** · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**A native, lightweight, animated Jellyfin client, made for the living room as much as for the desk.** Written in
Rust with Slint (interface) and libmpv (playback): no Qt, no embedded browser. It runs on any Windows or Linux
computer, from an old laptop to a small box plugged into the TV, and works just as well with a remote as with a
keyboard and mouse — and on Android phones, tablets and TVs, with the finger.

> **Fully vibecoded project**: all the code, texts and translations were written by an AI (Claude, by
> Anthropic), guided and tested by a human who did not write any of the code.

Current version: **1.9.9 “Cast Update”** · Interface languages: English, Français, Español, Deutsch, Italiano, Português,
Polski, Nederlands — and any language you add yourself (see [Translate Turtlefin](#translate-turtlefin)).

> **About this version.** 1.9.7 “Cast Update” has been tested on Windows, Linux (Raspberry Pi 5 plugged into a TV)
> and Android (phone). Some things still need work — the developer knows of remaining issues (see the
> [Releases](https://github.com/Xelopteryx/Turtlefin/releases) notes and the known issues in
> [HANDOFF.en.md](HANDOFF.en.md)) — but it was time to publish a version much more advanced than 1.2.0.
> Feedback is welcome in the [issues](https://github.com/Xelopteryx/Turtlefin/issues).

## What it does

- **Accounts**: “Who's watching?” screen with profile pictures (animated GIFs included, cached), up to
  12 accounts saved on the device (the token only, never the password), account switching without typing again,
  “Manage accounts” to remove some. Server discovery on every network of the device; a main address and a backup
  address, tried when the main one does not answer.
- **Startup**: an animated logo whose seven dots are real checks (language, display, video player, storage,
  configuration, network and, in the center, the server), then “Who's watching?” — or straight into the account
  chosen with Settings → Account → **Open this account at startup**.
- **Home**: My media, Continue watching, Next up, Recently added; Favorites and Requests (Seerr) tabs. Posters
  with remaining episodes, “watched” check mark and rating; background taken from the selected media.
- **Detail pages**: movie, series, season, episode; play, favorite, watched, download, audio / subtitle choice
  kept for the whole series; “More like this” and Seerr suggestions; request missing seasons.
- **Playback** (libmpv): chapters, season episodes, “Skip intro”, next episode, suggestions at the end of a
  series, Turtlefin's own volume, maximum bitrate (above it, the server converts the video); position and “watched” reported to the server.
- **Watch party** (SyncPlay): watch the same thing at the same time on several devices.
- **Android** (phone, tablet, Android TV): the same application, portrait or landscape, finger gestures
  (swipe the rows, the cast list, the episodes; swipe from the left edge: back), phone volume, TV remote and
  gamepad.
- **Cast**: send a video to another screen — another Turtlefin, a Jellyfin app of the account, or a Chromecast
  (with a relay through the device when the TV cannot reach the server) — and drive it from the player.
  **Remote**: drive another Turtlefin of the account like with its remote.
- **Seerr pages**: media that are not on the server open on a page of their own (summary, information,
  cast, “Read more”), with Request and **Trailer** (prepared on the server by the Turtlefin Trailers plugin,
  in the account's language, kept 24 hours at most).
- **Trailers**: icon on detail pages; the server's trailers (in the account's language when there are several),
  otherwise YouTube through yt-dlp when it is installed.
- **Themes**: Turtlefin, Frutiger Aero and Nerd, each with its own logo, startup and animated background; a
  theme of your own (see [Themes](#themes)). Offline, the whole interface turns amber.
- **Storage**: download folder and size limit; **password check** (warning when it is too simple, change it
  from the application).
- **Offline**: downloads replace the home screen, with complete detail pages without a server; what was watched
  or favorited offline is sent back to the account on reconnection.
- **Search** (library + Seerr) and random pick.
- **Remote, keyboard and mouse everywhere**: TV interface (large elements, on-screen keyboard) or computer
  interface (window or full screen, F11), click and wheel on every page, and a physical keyboard types straight
  into text fields, in both modes.
- **Settings**: profile picture (GetAvatar avatars by category), interface language, audio and subtitle
  languages, subtitle size, automatic next episode and intro skip, TV interface, full screen, background,
  ratings, clock, server addresses, image cache, **update from GitHub**.
- **Animations** everywhere (poster flying to the detail page, sliding menu, cascading rows, animated sign-in,
  language change), adjustable one by one in Settings → Animations, with presets (All, Light, None) and your
  own, which can be exported and imported.
- **Guided tour**: offered on first launch, available again in Settings → About.

## Install

Nothing to compile: download, install, done. Every file is on the
[Releases](https://github.com/Xelopteryx/Turtlefin/releases) page.

### One command

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

On Windows, the latest installer is downloaded and run for your user account (no administrator rights). On
Debian, Ubuntu, Linux Mint and other apt-based distributions, the `.deb` package is installed (your password is
asked); elsewhere, the AppImage goes to `~/.local/bin` with an entry in the applications menu.

### By hand

| System | File | Notes |
|---|---|---|
| Windows 64-bit | `Turtlefin-<version>-windows-x64-setup.exe` | Installer: language, folder, and **installed** (Start menu, uninstall) or **portable** mode |
| Windows 64-bit, no install | `Turtlefin-<version>-windows-x64-portable.zip` | Unzip anywhere (USB stick…) and run `turtlefin.exe` |
| Windows 32-bit | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | For old PCs |
| Linux, any distribution (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Make it executable (`chmod +x`), then run it |
| Linux with Flatpak (x86_64) | `Turtlefin-<version>-linux-x86_64.flatpak` | `flatpak install --user ./Turtlefin-….flatpak` (the runtime comes from Flathub) |
| Linux, 64-bit ARM | `Turtlefin-<version>-linux-aarch64.AppImage` | Same |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Double-click, or `sudo apt install ./turtlefin_….deb` |
| Android (phone, tablet, TV) | `Turtlefin-<version>-android-arm64.apk` | Allow installing from this source, then open the file (Android 8 or newer) |

The Windows and AppImage builds contain everything (libmpv player included). The `.deb` package uses the
system's libmpv (`libmpv2`, installed automatically by apt). AppImages and packages need a 2022 or newer
distribution (Ubuntu 22.04, Debian 12…).

**Portable version**: a `portable` file next to `turtlefin.exe` keeps the configuration, accounts, cache and
downloads in the `data` folder next to the program; nothing is written anywhere else.

### Update

**Settings → About → Check for updates** compares the installed version with the latest published one, then
**Update** handles everything according to how Turtlefin is installed:

| Installation | Update |
|---|---|
| Windows, installed | the new installer is downloaded and run silently in the same folder |
| Windows, portable | the new archive is downloaded and its files replace the old ones |
| AppImage | the new file replaces the old one |
| Flatpak | not automatic: download the new `.flatpak` file and install it the same way |
| .deb package | the package is installed with `pkexec` (the administrator password is asked) |

**Restart Turtlefin** then starts the new version.

## Recommended server plugins

Turtlefin works with a plain Jellyfin server (10.11 or newer). These plugins, installed on the **server**, add
features:

| Plugin | What it brings to Turtlefin |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Requests tab, Seerr results in search, Seerr suggestions and requests on detail pages — through the Jellyfin sign-in, no Seerr key on the client |
| [Seerr](https://github.com/seerr-team/seerr) (formerly Jellyseerr) | The request manager itself, used by Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Finds intros and credits: “Skip intro” button, automatic skip, “Next episode” at the right time |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | A gallery of profile pictures to choose from in Settings → Account |
| Turtlefin Trailers (published separately) | Downloads missing trailers on the server, in each library's and account's language, and prepares the trailers of the Seerr pages |

## Launch

```
turtlefin                                  startup animation, then “Who's watching?” (or the startup account)
turtlefin "Name"                           saved account “Name”
turtlefin "Name" --server=http://…         direct sign-in (password: variable TURTLEFIN_PASSWORD=…)
turtlefin --tv                             TV interface: full screen, large elements
turtlefin --desktop                        computer interface (overrides the “TV interface” setting)
turtlefin --no-intro                       no startup animation
turtlefin --sub=fr --audio=ja             subtitles / audio language for this launch (--sub=off: none)
turtlefin --fullscreen                     full screen without the TV interface
turtlefin --console                        log window (Windows)
turtlefin --tutorial                       guided tour when the home screen opens
turtlefin --lang=en                        interface language (kept afterwards)
turtlefin --theme=nerd                     theme: turtlefin, aero, nerd (kept afterwards)
turtlefin "Name" --play="Bleach S1E3"      plays an episode (title alone: a movie, or resumes the series)
turtlefin "Name" --open="Dragon Ball"      opens the detail page
turtlefin --help                           all options
```

Command-line options always win over settings (startup account, TV interface).

## Keys and mouse

- **Arrows** to move, **Enter** to open / activate, **Esc** or **Backspace** to go back (held down: one step
  back only).
- Home: **←** on the first card (or Back) opens the menu; in the menu, **→** or Esc closes it.
- **↑** from the top of a page: top bar (back, home, menu, watch party, random, search, account).
- Playback, controls hidden: **← →** rewind / forward 10 s, **↑ ↓** or Enter show the controls, ↓ from the
  buttons: season episodes. **Space**: pause · `a` audio · `s` subtitles · `f` full screen.
- **F11**, anywhere: full screen (also in Settings → Display → **Full screen**, kept between launches outside the
  TV interface).
- **TV remote**: Stop, next / previous episode, fast forward / rewind, Menu (left menu) and the gamepad
  (Android, Raspberry Pi through XeLauncher) work like the keyboard.
- **Mouse**: click to open, wheel to move from row to row (Shift + wheel: within the row).
- **Touch and dragging**: tap to open; swipe a finger (or drag with the left mouse button) to scroll the row or the page, a quick flick goes further; from the left edge: back.
- **Text**: a letter typed on the keyboard goes straight into the field (search, sign-in, address), even in the
  TV interface; in the TV interface, clicking the search field opens the on-screen keyboard.

## Files

| Where | What |
|---|---|
| config folder, `turtlefin/` | `session.json` (current session), `accounts.json` (saved accounts), `prefs.json` (device settings), `tracks.json` (tracks per series), `userdata.json` (watched / favorites done offline) |
| data folder, `turtlefin/downloads/` | downloads laid out like a library: `<library>/<series>/Season 01/<series> S01E01.mkv` (with its `.json`, image and subtitles), series images shared; `queue.json` (pending queue) |
| cache folder, `turtlefin/img/` | images and profile pictures (can be cleared in About) |
| **Documents** | `Turtlefin Languages` (added languages) and `Turtlefin Presets` (exported animation presets) |

On Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
On Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Portable version: everything in `data\` next to `turtlefin.exe` (`config`, `cache`, `downloads`), and the
language and preset folders next to the program.

## Troubleshooting

- `turtlefin --console` (Windows) or launching from a terminal (Linux) shows the log.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: mpv log · `TURTLEFIN_MPV_ARGS="…"`: extra mpv options.
- `TURTLEFIN_HWDEC=no`: software decoding (hardware by default, `drm-copy` on the Raspberry Pi) · `TURTLEFIN_AO=alsa`: audio output.
- Lines or artifacts in the picture (NVIDIA): Settings → Playback → **Hardware decoding** off (everything is decoded by the processor).
- `TURTLEFIN_LIBMPV=path`: other libmpv location · `TURTLEFIN_DEBUG_FRAMES=1`: reports slow frames.
- Rendering must be OpenGL (chosen automatically): with `SLINT_BACKEND=winit-software`, no video.

## Themes

Settings → Display → **Theme**: **Turtlefin** (default: hexagons, purple → blue), **Frutiger Aero** (in the spirit
of Windows 7: sky and bubbles, bluish glass, glossy buttons, orb logo) and **Nerd** (a circuit board at night:
the server is the processor, current runs through the tracks to every component) — or a theme of your own.
“Animated background” (Settings → Animations) brings each background to life; it is off by default on the
Raspberry Pi.

- **Create a theme** opens the *Turtlefin Theme Creator* in the browser: every color, the corner radius, the glossy
  highlight, the sky and bubbles, with a live preview; it saves a `.tftheme` file.
- Put that file in the **Turtlefin Themes** folder (Documents, or next to `turtlefin.exe` for the portable version),
  then **Import a theme**. **Export the theme** writes the current theme to that folder, as a starting point.
- The creator is also in the repository, `tools/theme-creator.html`: a single file that works offline.

## Translate Turtlefin

No programming, no compiling. Added languages live in a single folder, **Turtlefin Languages**: in
**Documents** (installed version) or next to `turtlefin.exe` (portable version).

1. **Settings → Display → Add a language** opens an explorer of that folder; **Create the template** writes
   `modele.po` there, with the texts in English and, as notes, the original French and the current language.
2. Copy it as `<code>.po` (`sv.po` for Swedish, `ja.po` for Japanese…), in that folder or one of its
   subfolders, and fill each `msgstr ""` with the translation of the English `msgid` above it. Keep the `{}`
   and `{n}`. Also fill `X-Language-Name` (displayed name) and, if needed, `Plural-Forms` (the language's
   gettext rule). Any `.po` editor works, for example [Poedit](https://poedit.net).
3. **Add a language** again: the translation appears with how much of it is done; choosing it applies it.
   Texts left empty are shown in English.

To share it with everyone: a pull request that adds the file to `lang/` (and to `BUILTIN` in `src/i18n.rs`);
`python tools/lang-check.py` checks that nothing is missing.

## Development

See [HANDOFF.en.md](HANDOFF.en.md) (also in [Français](HANDOFF.md), [Deutsch](HANDOFF.de.md),
[Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): project state, decisions, building, packages, publishing, translations, known
issues.

## License

Turtlefin is released under the [GNU GPL v3](LICENSE) (or any later version): anyone may use, study, modify
and redistribute it, as long as they keep the same license and share the source code of their changes.
