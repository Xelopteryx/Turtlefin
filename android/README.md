# Turtlefin pour Android

Le même code que le bureau (`../src/main.rs` et ses modules, `../ui/`), compilé en bibliothèque
`libturtlefin.so` lancée par `NativeActivity`, avec le moteur Android de Slint (rendu Skia, OpenGL ES).

## Fabriquer l'APK

Il faut : Android Studio (SDK, Java), le NDK (Android Studio > SDK Manager > NDK, ou l'archive r27c
dans `<SDK>\ndk\27.2.12479018`), Rust avec `rustup target add aarch64-linux-android x86_64-linux-android`
et `cargo install cargo-ndk`.

```
powershell -ExecutionPolicy Bypass -File android\build-apk.ps1                 # téléphones + émulateur
powershell -ExecutionPolicy Bypass -File android\build-apk.ps1 -Abi x86_64     # émulateur seul (plus rapide)
```

Résultat : `target\dist\Turtlefin-<version>-android.apk`, signé avec la clé de débogage d'Android Studio
(`-Keystore` pour une vraie clé). La compilation se fait dans `%LOCALAPPDATA%\turtlefin-android`
(place). libmpv vient du projet libmpv-android (Maven Central, `dev.jdtech.mpv:libmpv`, empreinte
vérifiée par le script).

## Essais

```
adb install -r target\dist\Turtlefin-1.2.0-android.apk
adb shell am start -n io.github.xelopteryx.turtlefin/android.app.NativeActivity
adb logcat -s turtlefin                       # messages de Turtlefin (eprintln!, paniques)
adb shell setprop debug.turtlefin.env "TURTLEFIN_HWDEC=no;TURTLEFIN_MPV_ARGS=--gpu-dumb-mode=no"
```

La propriété `debug.turtlefin.env` remplace les variables d'environnement du bureau (92 caractères
au plus) ; elle est lue au lancement. Dossiers : configuration et cache dans la mémoire interne de
l'appli, téléchargements dans `Android/data/io.github.xelopteryx.turtlefin/files/downloads`.

## Ce qui diffère du bureau

- `vendor/i-slint-backend-android-activity` : copie modifiée du moteur Android de Slint 1.18.1
  (`[patch.crates-io]`) : crochet tactile (`set_touch_hook`, défilement au doigt comme au bureau),
  caractère réel des touches d'un clavier physique (Maj, AltGr), bouton Retour = Échap, échelle de
  l'interface (au moins ~1280 x 600 points), `hide_soft_input`. À reporter à chaque mise à jour de Slint.
- HTTPS : certificats d'autorité du système donnés directement à reqwest (`api::http_builder`) ; le
  vérificateur habituel exige un composant Java absent.
- mpv : décodage MediaCodec (`mediacodec-copy`), son AudioTrack, rendu « simple » (`gpu-dumb-mode`) :
  les passes intermédiaires de mpv, mêlées au rendu de Skia, donnaient une image noire en pause.
- Toucher la vidéo montre ou cache les commandes (au lieu de mettre en pause) ; le clavier virtuel fait
  remonter l'écran de saisie (`Kb`, ui/theme.slint) et se range quand son champ disparaît.
- Pas de mise à jour intégrée (installation « inconnue ») ; la watch party en `wss://` n'a pas encore
  de certificats (serveurs `https://` seulement).
