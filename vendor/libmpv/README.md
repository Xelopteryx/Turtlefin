# libmpv pour Windows

`libmpv-2.dll`, compressée en gzip (GitHub refuse les fichiers de plus de 100 Mo). `build.rs` la
décompresse, vérifie son empreinte SHA-256 et la pose à côté de `turtlefin.exe` à chaque compilation
Windows : rien à télécharger, même après un `cargo clean`. Sous Linux, libmpv vient du système
(`libmpv-dev` / `libmpv2`).

Chaque dossier (`windows-x86_64`, `windows-i686`) contient :
- `libmpv-2.dll.gz` : la DLL compressée ;
- `libmpv-2.dll.sha256` : l'empreinte de la DLL décompressée (vérifiée par `build.rs`) ;
- `SOURCE.txt` : d'où elle vient.

## Changer de version

Une commande, depuis la racine du dépôt, puis `cargo build` :

```
powershell -ExecutionPolicy Bypass -File tools\update-libmpv.ps1 -Download
powershell -ExecutionPolicy Bypass -File tools\update-libmpv.ps1 C:\chemin\libmpv-2.dll
powershell -ExecutionPolicy Bypass -File tools\update-libmpv.ps1 C:\chemin\libmpv-2.dll -Arch x86
```

`-Download` prend la dernière version de shinchiro/mpv-winbuild-cmake (empreinte de l'archive vérifiée
avec celle publiée par GitHub ; 7-Zip nécessaire). Le script vérifie que la DLL est de la bonne
architecture, la compresse, écrit l'empreinte et la provenance.

32 bits : les versions publiées après le 10 juin 2026 plantent au démarrage (« OpenSSL internal error:
assertion failed: lock != NULL ») ; celle du dépôt est la dernière bonne (copie gardée dans la
pré-version `libmpv-i686-20260610` de Turtlefin). Le script refuse donc `-Download -Arch x86` (sauf
`-Force`).

Chaque version ajoutée reste dans l'historique du dépôt (environ 45 Mo).

## Licence et sources

libmpv (mpv) et FFmpeg, compilés par shinchiro, sont distribués sous GPL. Sources :
- mpv : https://github.com/mpv-player/mpv
- FFmpeg et les autres bibliothèques incluses : https://github.com/shinchiro/mpv-winbuild-cmake (recettes
  de compilation et versions exactes utilisées)
