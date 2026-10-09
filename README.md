<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin logo"></p>

# Turtlefin

**English** · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**A native, lightweight, animated Jellyfin client, made for the living room as much as for the desk.** Written in
Rust with Slint (interface) and libmpv (playback): no Qt, no embedded browser. It runs on any Windows or Linux
computer, from an old laptop to a small box plugged into the TV, and works just as well with a remote as with a
keyboard and mouse.

Current version: **1.0.0** · Interface languages: English, Français, Español, Deutsch, Italiano, Português,
Polski, Nederlands — and any language you add yourself (see [Translate Turtlefin](#translate-turtlefin)).

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
  series, Turtlefin's own volume; position and “watched” reported to the server.
- **Watch party** (SyncPlay): watch the same thing at the same time on several devices.
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
| Linux, 64-bit ARM | `Turtlefin-<version>-linux-aarch64.AppImage` | Same |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Double-click, or `sudo apt install ./turtlefin_….deb` |

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

## Launch

```
turtlefin                                  startup animation, then “Who's watching?” (or the startup account)
turtlefin "Name"                           saved account “Name”
turtlefin "Name" --server=http://…         direct sign-in (password: variable TURTLEFIN_PASSWORD=…)
turtlefin --tv                             TV interface: full screen, large elements
turtlefin --desktop                        computer interface (overrides the “TV interface” setting)
turtlefin --no-intro                       no startup animation
turtlefin --console                        log window (Windows)
turtlefin --tutorial                       guided tour when the home screen opens
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
- **Mouse**: click to open, wheel to move from row to row (Shift + wheel: within the row).
- **Text**: a letter typed on the keyboard goes straight into the field (search, sign-in, address), even in the
  TV interface; in the TV interface, clicking the search field opens the on-screen keyboard.

## Files

| Where | What |
|---|---|
| config folder, `turtlefin/` | `session.json` (current session), `accounts.json` (saved accounts), `prefs.json` (device settings), `tracks.json` (tracks per series), `userdata.json` (watched / favorites done offline) |
| data folder, `turtlefin/downloads/` | downloads (media, posters, background, logo, `info.json`), `queue.json` (pending queue) |
| cache folder, `turtlefin/img/` | images and profile pictures (can be cleared in About) |
| **Documents** | `Turtlefin Languages` (added languages) and `Turtlefin Presets` (exported animation presets) |

On Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
On Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Portable version: everything in `data\` next to `turtlefin.exe` (`config`, `cache`, `downloads`), and the
language and preset folders next to the program.

## Troubleshooting

- `turtlefin --console` (Windows) or launching from a terminal (Linux) shows the log.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: mpv log · `TURTLEFIN_MPV_ARGS="…"`: extra mpv options.
- `TURTLEFIN_HWDEC=auto-copy`: hardware decoding (software by default on Linux ARM) · `TURTLEFIN_AO=alsa`: audio output.
- `TURTLEFIN_LIBMPV=path`: other libmpv location · `TURTLEFIN_DEBUG_FRAMES=1`: reports slow frames.
- Rendering must be OpenGL (chosen automatically): with `SLINT_BACKEND=winit-software`, no video.

## Themes

Settings → Display → **Theme**: Turtlefin (default), Dark, Light, Frutiger Aero (sky, grass, bubbles, glossy gel
buttons), Turtlefin green — or a theme of your own.

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
