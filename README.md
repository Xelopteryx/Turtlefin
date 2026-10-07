<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Turtlefin logo"></p>

# Turtlefin

**English** · [Français](README.fr.md)

**A lightweight, animated native Jellyfin client, made for the couch.** Written in Rust with Slint (interface) and
libmpv (playback): no Qt, no embedded web browser. It runs on any Windows or Linux computer, from an old laptop to
a small box plugged into the TV, and is fully usable with a keyboard or a remote.

Current version: **0.9.1** · Interface languages: English, Français, Español, Deutsch, Italiano, Português, Polski, Nederlands — and any language you add yourself (see [Translate Turtlefin](#translate-turtlefin)).

## Features

- **Accounts**: “Who's watching?” screen with avatars (animated GIFs too), up to 12 accounts saved on the device
  (only the access token, never the password), switching accounts without typing again, “Manage accounts” to
  remove some. Server search on every network of the device (local and VPN); a main address and a backup address,
  used when the main one does not answer (Settings → Network).
- **Startup**: animated logo whose seven dots are real checks (language, display, video player, storage,
  configuration, network, and the server in the centre); they then link up into the logo and it opens
  “Who's watching?” — or directly the account chosen in Settings → Account → **Open this account at startup**.
- **Home**: My media, Continue watching, Next up, Recently added; Favorites and Requests (Seerr) tabs.
  Posters with remaining episodes, “watched” check mark, rating; background taken from the selected item.
- **Detail pages**: movie, series, season, episode; play, favorite, watched, download, audio / subtitle choice
  remembered per series; “More like this” and Seerr suggestions; request the missing seasons of a series.
- **Playback** (libmpv): remote-friendly controls, chapters, season episodes, “Skip intro”, next episode,
  suggestions at the end of a series; position and “watched” sent back to the server.
- **Watch party** (SyncPlay): watch the same thing at the same time on several devices.
- **Offline**: downloads shown like the home screen, full detail pages without a server; what you watch,
  mark as watched or favorite offline is sent back to your account when the server is reachable again.
- **Search** (library + Seerr), random pick, on-screen keyboard for TV use.
- **Settings**: profile picture (GetAvatar avatars, sorted by category), interface language, audio and subtitle
  languages, subtitle size, autoplay next episode, automatic intro skip, TV interface, background, ratings, clock,
  server addresses, image cache, **updates from GitHub**.
- Animated transitions everywhere (poster flying to its page, sliding menu, cascading rows).

## Install

Nothing to build: download, install, done. Every file is on the
[Releases](https://github.com/Xelopteryx/Turtlefin/releases) page.

### In one command

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

On Windows, the latest installer is downloaded and run for your user account (no administrator rights).
On Debian, Ubuntu, Linux Mint and other apt-based distributions, the `.deb` package is installed (your
password is asked); elsewhere, the AppImage goes to `~/.local/bin` with an entry in the applications menu.

### By hand

| System | File | Notes |
|---|---|---|
| Windows 64-bit | `Turtlefin-<version>-windows-x64-setup.exe` | Installer: language, folder, and **installed** (Start menu, uninstaller) or **portable** mode |
| Windows 64-bit, no install | `Turtlefin-<version>-windows-x64-portable.zip` | Unzip anywhere (USB stick…) and run `turtlefin.exe` |
| Windows 32-bit | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | For old PCs |
| Linux, any distribution (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Make it executable (`chmod +x`), then run it |
| Linux, ARM 64-bit | `Turtlefin-<version>-linux-aarch64.AppImage` | Same |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Double-click it, or `sudo apt install ./turtlefin_….deb` |

Windows and AppImage builds contain everything (libmpv player included). The `.deb` package uses the system
libmpv (`libmpv2`, installed automatically by apt). AppImages and packages need a 2022 or newer distribution
(Ubuntu 22.04, Debian 12…).

**Portable mode**: a `portable` file next to `turtlefin.exe` keeps settings, accounts, cache and downloads in the
`data` folder next to the program; nothing is written anywhere else.

### Updates

**Settings → About → Check for updates** compares the installed version with the latest release, then
“Update” does the right thing for the way Turtlefin is installed:

| Installed as | Update |
|---|---|
| Windows, installed | the new installer is downloaded and run silently in the same folder |
| Windows, portable | the new archive is downloaded and its files replace the old ones |
| AppImage | the new file replaces the old one |
| .deb package | the package is installed with `pkexec` (administrator password asked) |

“Restart Turtlefin” then starts the new version.

## Recommended server plugins

Turtlefin works with a plain Jellyfin server (10.11 or newer). These plugins, installed on the **server**, unlock
extra features:

| Plugin | What it brings to Turtlefin |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Requests tab, Seerr results in search, Seerr suggestions and requests on detail pages — through your Jellyfin login, no Seerr key on the client |
| [Seerr](https://github.com/seerr-team/seerr) (formerly Jellyseerr) | The request manager itself, used by Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Detects intros and credits: “Skip intro” button, automatic skip, “Next episode” at the right moment |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | A gallery of profile pictures to choose from in Settings → Account |

## Launch options

```
turtlefin                                  startup animation, then “Who's watching?” (or the startup account)
turtlefin "Name"                           saved account “Name”
turtlefin "Name" --server=http://…         direct sign-in (password: TURTLEFIN_PASSWORD=… environment variable)
turtlefin --tv                             full screen, large elements (TV)
turtlefin --desktop                        window (overrides the “TV interface” setting)
turtlefin --no-intro                       no startup animation
turtlefin --console                        show the log window (Windows)
```

Command-line options always win over settings (startup account, TV interface).

## Keys (remote or keyboard)

- **Arrows** to move, **Enter** to open / activate, **Esc** or **Backspace** to go back.
- Home: **←** on the first card (or Back) opens the menu; in the menu, **→** or Esc closes it.
- ↑ from the top of a page: top bar (back, home, menu, watch party, random, search, account).
- Playback, controls hidden: ← → rewind / forward 10 s, ↑ ↓ or Enter show the controls,
  ↓ from the buttons: season episodes. Space: pause · `a` audio · `s` subtitles · `f` full screen.

## Files

| Where | What |
|---|---|
| config folder, `turtlefin/` | `session.json` (current session), `accounts.json` (saved accounts), `prefs.json` (device settings), `tracks.json` (tracks per series), `userdata.json` (watched / favorites made offline) |
| data folder, `turtlefin/downloads/` | downloads (media, posters, background, logo, `info.json`), `queue.json` (pending queue) |
| cache folder, `turtlefin/img/` | images (can be emptied in About) |

Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Portable: everything in `data\` next to `turtlefin.exe` (`config`, `cache`, `downloads`).

## Troubleshooting

- `turtlefin --console` (Windows) or start it from a terminal (Linux) to see the log.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: mpv log · `TURTLEFIN_MPV_ARGS="…"`: extra mpv options.
- `TURTLEFIN_HWDEC=auto-copy`: hardware decoding (software by default on ARM Linux) · `TURTLEFIN_AO=alsa`: audio output.
- `TURTLEFIN_LIBMPV=path`: other libmpv location · `TURTLEFIN_DEBUG_FRAMES=1`: reports slow frames.
- Rendering must be OpenGL (chosen automatically): with `SLINT_BACKEND=winit-software`, no video.

## Translate Turtlefin

No programming and no compiling needed:

1. **Settings → Display → Add a language**: Turtlefin writes a template, `modele.po`, and opens its folder
   (`languages` in Turtlefin's configuration folder).
2. Copy it as `<code>.po` (`sv.po` for Swedish, `ja.po` for Japanese…) and fill in each `msgstr ""` with the
   translation of the `msgid` above it (French). Keep the `{}` and `{n}`. Also fill in `X-Language-Name` (the name
   shown in the list) and, if needed, `Plural-Forms` (standard gettext rule for your language).
   Any `.po` editor works, for example [Poedit](https://poedit.net).
3. Restart Turtlefin and pick the language in **Settings → Display → Interface language**. Texts left empty are
   shown in English.

To share it with everyone, open a pull request adding the file to `lang/` (and to `BUILTIN` in `src/i18n.rs`);
`python tools/lang-check.py` checks that nothing is missing.

## Development

See [HANDOFF.en.md](HANDOFF.en.md) ([français](HANDOFF.md)): project state, decisions, building, packaging,
translations, known issues.
