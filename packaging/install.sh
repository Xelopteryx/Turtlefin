#!/bin/sh
# Installation de Turtlefin en une commande (Linux) :
#   curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
#
# Debian, Ubuntu, Mint... (apt présent) : paquet .deb, installé avec apt (mot de passe
# administrateur demandé), mises à jour par Turtlefin lui-même.
# Autres distributions : AppImage dans ~/.local/bin, avec un raccourci dans le menu des applications.
# Messages en anglais : le script sert à tout le monde.
set -eu

REPO=Xelopteryx/Turtlefin
case "$(uname -m)" in
  x86_64 | amd64) ARCH=x86_64; DEB=amd64 ;;
  aarch64 | arm64) ARCH=aarch64; DEB=arm64 ;;
  *) echo "Turtlefin: unsupported processor $(uname -m) (x86_64 or aarch64 only)." >&2; exit 1 ;;
esac

echo "Turtlefin: looking for the latest version ($ARCH)..."
JSON=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")
url_of() { printf '%s\n' "$JSON" | grep -o "\"browser_download_url\": *\"[^\"]*$1\"" | head -n 1 | sed 's/.*"\(https[^"]*\)"/\1/'; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

if command -v apt-get >/dev/null 2>&1 && command -v dpkg >/dev/null 2>&1; then
  URL=$(url_of "_$DEB.deb")
  [ -n "$URL" ] || { echo "No $DEB .deb package in the latest release." >&2; exit 1; }
  echo "Downloading $(basename "$URL")..."
  curl -fL --progress-bar -o "$TMP/turtlefin.deb" "$URL"
  chmod 644 "$TMP/turtlefin.deb"
  chmod 755 "$TMP"
  echo "Installing (administrator password)..."
  if [ "$(id -u)" -eq 0 ]; then apt-get install -y "$TMP/turtlefin.deb"; else sudo apt-get install -y "$TMP/turtlefin.deb"; fi
  echo "Turtlefin is installed: applications menu, or the \"turtlefin\" command."
else
  URL=$(url_of "-linux-$ARCH.AppImage")
  [ -n "$URL" ] || { echo "No $ARCH AppImage in the latest release." >&2; exit 1; }
  BIN="$HOME/.local/bin"
  APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
  ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
  mkdir -p "$BIN" "$APPS" "$ICONS"
  echo "Downloading $(basename "$URL")..."
  curl -fL --progress-bar -o "$BIN/Turtlefin.AppImage" "$URL"
  chmod +x "$BIN/Turtlefin.AppImage"
  curl -fsSL -o "$ICONS/turtlefin.svg" "https://raw.githubusercontent.com/$REPO/main/packaging/turtlefin.svg" || true
  cat > "$APPS/turtlefin.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Turtlefin
Comment=Jellyfin client
Exec=$BIN/Turtlefin.AppImage
Icon=turtlefin
Categories=AudioVideo;Video;Player;
Terminal=false
EOF
  echo "Turtlefin is installed: applications menu, or $BIN/Turtlefin.AppImage."
  echo "(If the AppImage does not start, install FUSE 2: package \"fuse\" or \"libfuse2\".)"
fi
