#!/bin/sh
# AppImage de Turtlefin (version portable Linux) : l'exécutable, libmpv et toutes ses
# dépendances (ffmpeg...) dans un seul fichier. Les pilotes graphiques (OpenGL / EGL) et la
# bibliothèque C restent ceux du système, comme le veut le format AppImage.
#
# Usage : packaging/linux/build-appimage.sh <version> [binaire]
# Prérequis : libmpv installée (libmpv-dev / libmpv2), wget ; compilé avec `cargo build --release`.
set -eu

VERSION="$1"
BIN="${2:-target/release/turtlefin}"
ARCH="$(uname -m)"            # x86_64 ou aarch64
WORK="target/appimage"
TOOLS="target/appimage-tools"

rm -rf "$WORK"
mkdir -p "$WORK" "$TOOLS"

# Outil linuxdeploy (assemble l'AppImage et y copie les bibliothèques nécessaires).
LD="$TOOLS/linuxdeploy-$ARCH.AppImage"
[ -f "$LD" ] || wget -q -O "$LD" "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$ARCH.AppImage"
chmod +x "$LD"

# libmpv est chargée à l'exécution (dlopen) : linuxdeploy ne la verrait pas toute seule.
# (ldconfig est dans /sbin, hors du PATH d'un utilisateur ordinaire sur Debian.)
LDCONFIG="$(command -v ldconfig || echo /sbin/ldconfig)"
LIBMPV="$("$LDCONFIG" -p | awk '/libmpv\.so\.2 /{print $NF; exit}')"
[ -n "$LIBMPV" ] || LIBMPV="$("$LDCONFIG" -p | awk '/libmpv\.so\.1 /{print $NF; exit}')"
[ -n "$LIBMPV" ] || { echo "libmpv introuvable (installe libmpv-dev)"; exit 1; }

# Pas de FUSE dans les conteneurs d'intégration continue : on extrait l'outil à la volée.
export APPIMAGE_EXTRACT_AND_RUN=1
export LDAI_OUTPUT="Turtlefin-$VERSION-linux-$ARCH.AppImage"

"$LD" --appdir "$WORK/AppDir" \
  --executable "$BIN" \
  --library "$LIBMPV" \
  --desktop-file packaging/linux/turtlefin.desktop \
  --icon-file packaging/turtlefin.svg \
  --output appimage

mv "$LDAI_OUTPUT" target/
echo "AppImage : target/$LDAI_OUTPUT"
