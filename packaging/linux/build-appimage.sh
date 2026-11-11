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

# Outil linuxdeploy (assemble l'AppImage et y copie les bibliothèques nécessaires). Version épinglée
# et empreinte SHA-256 vérifiée (celle publiée par GitHub pour cette version) : la version
# « continuous » change sans cesse et n'a pas d'empreinte stable. Pour passer à une version plus
# récente : changer LD_TAG et les quatre empreintes (champ « digest » de l'API GitHub des versions).
LD_TAG="1-alpha-20251107-1"
case "$ARCH" in
    x86_64)  LD_SHA="c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d" ;;
    aarch64) LD_SHA="620095110d693282b8ebeb244a95b5e911cf8f65f76c88b4b47d16ae6346fcff" ;;
    armhf)   LD_SHA="e359161979fa4bee50b92ce7102fb510299caebf34f711d983fba7a8f4bb1c2e" ;;
    i386)    LD_SHA="a2a88d142aac42db779483ca07c10dbf318b27f514691107fc88a202faae17b5" ;;
    *) echo "linuxdeploy : architecture $ARCH non prise en charge"; exit 1 ;;
esac
LD="$TOOLS/linuxdeploy-$LD_TAG-$ARCH.AppImage"
[ -f "$LD" ] || wget -q -O "$LD" "https://github.com/linuxdeploy/linuxdeploy/releases/download/$LD_TAG/linuxdeploy-$ARCH.AppImage"
if [ "$(sha256sum "$LD" | cut -d' ' -f1)" != "$LD_SHA" ]; then
    rm -f "$LD"
    echo "linuxdeploy : empreinte SHA-256 inattendue, fichier supprimé"
    exit 1
fi
chmod +x "$LD"

# libmpv est chargée à l'exécution (dlopen) : linuxdeploy ne la verrait pas toute seule.
# (ldconfig est dans /sbin, hors du PATH d'un utilisateur ordinaire sur Debian.)
LDCONFIG="$(command -v ldconfig || echo /sbin/ldconfig)"
LIBMPV="$("$LDCONFIG" -p | awk '/libmpv\.so\.2 /{print $NF; exit}')"
[ -n "$LIBMPV" ] || LIBMPV="$("$LDCONFIG" -p | awk '/libmpv\.so\.1 /{print $NF; exit}')"
[ -n "$LIBMPV" ] || { echo "libmpv introuvable (installe libmpv-dev)"; exit 1; }
# Clavier sous X : libxkbcommon-x11 est elle aussi chargée à l'exécution (winit). Absente d'une Debian
# minimale : Turtlefin s'arrêtait aussitôt lancé. Embarquée si la machine de compilation l'a.
XKBX11="$("$LDCONFIG" -p | awk '/libxkbcommon-x11\.so\.0 /{print $NF; exit}')"
# (chemins système, sans espace : passés tels quels à linuxdeploy)
EXTRA=""
[ -n "$XKBX11" ] && EXTRA="--library $XKBX11"

# Pas de FUSE dans les conteneurs d'intégration continue : on extrait l'outil à la volée.
export APPIMAGE_EXTRACT_AND_RUN=1
export LDAI_OUTPUT="Turtlefin-$VERSION-linux-$ARCH.AppImage"

"$LD" --appdir "$WORK/AppDir" \
  --executable "$BIN" \
  --library "$LIBMPV"   $EXTRA \
  --desktop-file packaging/linux/turtlefin.desktop \
  --icon-file packaging/turtlefin.svg \
  --output appimage

mv "$LDAI_OUTPUT" target/
echo "AppImage : target/$LDAI_OUTPUT"
