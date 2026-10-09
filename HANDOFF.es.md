# Turtlefin: traspaso del proyecto (estado a 9 de octubre de 2026, versión 1.0.0)

[Français](HANDOFF.md) · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · **Español** · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · [Português](HANDOFF.pt.md)

Para quien retome el desarrollo (una persona o Claude Code). Léelo entero antes de tocar el código y luego lee
[README.es.md](README.es.md) (uso, instalación, teclas, archivos).
Repositorio: https://github.com/Xelopteryx/Turtlefin · Versión en `Cargo.toml`: 1.0.0.

## 1. Objetivo

Un **cliente de Jellyfin nativo en Rust**, ligero, animado, utilizable con mando a distancia igual que con teclado
y ratón, instalable en cualquier ordenador con Windows o Linux **sin compilar nada**: quien lo usa descarga un
instalador o un paquete (o lanza un comando de instalación), y ya está. Todos los paquetes los fabrica la CI de
GitHub (o el PC de quien mantiene el proyecto), nunca el usuario.

Por qué: Jellyfin Desktop (Qt / QtWebEngine) pierde memoria y acaba cerrándose en las máquinas pequeñas, y la
interfaz web con un tema cargado cae por debajo de 30 imágenes/s en hardware modesto. Regla permanente: mantener la
memoria estable y no volver a introducir nunca desenfoque en tiempo real ni animaciones de filtros.

## 2. Decisiones tomadas

| Tema | Decisión | Motivo |
|---|---|---|
| Lenguaje / UI | Rust + **Slint** `~1.18` (renderizado 100 % Slint), estilo `fluent-dark` impuesto por `build.rs` | Sin navegador; el estilo «native» dependería de Qt |
| Función de Slint `unstable-winit-030` | Filtro de eventos de winit para **F11** (`install_f11`) | Única forma de tener una tecla global; de ahí `~1.18` (API inestable entre versiones menores) |
| Red | `reqwest` 0.13 (rustls, almacén de certificados del sistema), `tokio` | `query` es una feature que hay que activar en 0.13 |
| Reproducción | **libmpv cargada en tiempo de ejecución** (`libloading`, `src/mpv.rs`), renderizado OpenGL en una textura mostrada por Slint (`src/video.rs`), controles de Slint encima (`ui/player.slint`) | Reproductor integrado, sin IPC, funciona en Wayland. El reproductor se recrea en cada reproducción (memoria acotada) |
| Renderizador de Slint | femtovg (OpenGL / GLES) impuesto, salvo si se define `SLINT_BACKEND` | El vídeo pasa por una textura OpenGL |
| Textura de vídeo | Píxeles físicos, origen `TopLeft`, estado GL guardado / restaurado alrededor de mpv; `loadfile` espera al contexto de renderizado | Si no, imagen al revés o «No render context set» |
| Decodificación | `hwdec=no` en Linux ARM de 64 bits, `auto-safe` en el resto (Windows: `d3d11va-copy`) | En las placas ARM probadas (controlador v3d), la decodificación V4L2 produce un formato que el renderizador no sabe importar |
| Audio en Linux | `ao=pipewire,pulse,alsa`, `config=no` | Un `mpv.conf` de usuario que imponía ALSA fallaba cuando PipeWire tiene la salida HDMI |
| Memoria | Caché de mpv limitada (100 / 25 MiB), imágenes pedidas al tamaño justo, 16 elementos por fila | Máquinas pequeñas (4 GB) |
| Idiomas | Francés en el código (idioma de origen), traducción en tiempo de ejecución por `src/i18n.rs` desde `lang/<code>.po` (integrados) y la carpeta `Turtlefin Languages` | Ver la sección 6 |
| Pantalla completa (Windows) | Ventana sin bordes que cubre la pantalla + 1 px (`src/winfull.rs`, `set_tv_window`), no la pantalla completa real; sigue los cambios de resolución (cada 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` para la pantalla completa de Slint | En pantalla completa OpenGL, AMD Software toma la aplicación por un juego: «Pulsa ALT + R» cada vez que vuelve al primer plano |
| Interfaz TV / de ordenador | `tv-mode` solo cambia el tamaño (`k` = 1,4) y la escritura (teclado en pantalla); la pantalla completa va aparte (`full_flag`, `UiPrefs::fullscreen`, F11) | Petición del usuario: pantalla completa sin agrandar la interfaz |
| Ventana (Windows, escritorio) | Reducida y centrada si 1280 x 720 + marco supera el área de trabajo; sigue animada al moverla (`winfull::keep_alive_while_moving`) | Pantallas de 1366 x 768; Windows bloquea el bucle de eventos al mover una ventana |
| Ventana de consola (Windows) | Subsistema «windows» en release; `--console` se engancha a una o abre una; comandos externos sin ventana (`paths::quiet_command`) | Ni consola ni ventana CMD que parpadee |
| Línea de comandos | Siempre tiene prioridad sobre los ajustes (cuenta de inicio, interfaz TV) | Petición del usuario |
| Contraseña | Nunca se guarda (solo el token); en la línea de comandos, mejor `TURTLEFIN_PASSWORD` | Un argumento es visible para los demás procesos |

## 3. Estructura del código

```
build.rs            commit compilado, estilo de Slint, icono del exe (winresource, Windows)
lang/<code>.po      traducciones integradas (origen: el francés del código); tools/lang-check.py las comprueba
ui/theme.slint      tokens del tema, globals Tr (traducción) y Motion (animaciones activadas)
ui/app.slint        AppWindow y todas las pantallas (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo: animación de inicio (7 puntos, unión, zoom), elección del idioma
ui/player.slint     pantalla de reproducción   ui/osk.slint      teclado en pantalla
ui/card.slint       tarjeta de cartel          ui/marquee.slint  texto que se desplaza
ui/typed.slint      texto escrito animado (Str, TypedText)   ui/langx.slint  explorador (idiomas, presets)
ui/dust.slint       barras del cambio de idioma              ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, estado compartido App (Arc), pantallas, navegación (pila + páginas guardadas), ajustes
src/boot.rs         secuencia de inicio (comprobaciones, idioma, elección de la pantalla de llegada)
src/i18n.rs         idioma actual, tr() / trf() / trn(), idiomas añadidos, plantilla de traducción
src/dust.rs         efecto del cambio de idioma (detección de las líneas de texto, morphing)
src/api.rs          cliente REST de Jellyfin (+ relé Seerr de Jellyfin Enhanced, GetAvatar)
src/config.rs       sesión, cuentas (12 como máximo), prefs.json (UiPrefs, AnimFlags, presets), pistas, visto/favoritos sin conexión
src/discovery.rs    búsqueda de servidores (UDP, subredes, ARP, pares VPN)
src/downloads.rs    descargas (reanudación Range, cola, sincronización sin conexión)
src/mpv.rs          enlace con libmpv; src/video.rs textura OpenGL; src/player.rs reproducción, informes, encadenamiento
src/syncplay.rs     watch party (WebSocket /socket)
src/paths.rs        carpetas de configuración / caché / datos; modo portátil; quiet_command
src/update.rs       actualización según la instalación (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows: pantalla / área de trabajo, ventana animada al moverla
src/theme.rs        temas integrados, archivos .tftheme (ThemeDef), aplicación al global Theme
ui/sky.slint        decorado de Frutiger Aero (cielo, colina, burbujas), estático
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logotipo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   compilación y publicación con una etiqueta `v*` (prueba: rama `ci`)
```

Principios:
- Los datos de red pasan por estructuras `Send` y luego `upgrade_in_event_loop` los lleva a los modelos de Slint.
  Imágenes decodificadas fuera del hilo de la UI, 6 descargas en paralelo, aplicadas con una comprobación del id.
- `App.gen` invalida las cargas obsoletas; `App.stack` es la pila de navegación; `PAGES` guarda fichas y
  bibliotecas para volver sin peticiones.
- Navegación con teclado hecha a mano (índices de selección en Rust y en Slint), ya que Slint no gestiona el foco
  de tarjetas dinámicas. Cada pantalla tiene su `FocusScope`; `refocus` devuelve el teclado al sitio correcto.
- Los `changed` de Slint se ejecutan en diferido: no hay que contar con su orden (posiciones en dos tiempos, etc.).
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`, cabecera
  `Authorization: MediaBrowser …, Token=…`. Informes: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` y el WebSocket rechazan `api_key`: token en la cabecera.

## 4. Inicio

`main` aplica el idioma (prefs.json; si no, el archivo `language` escrito por el instalador de Windows) y luego
lanza `boot::run`. La pantalla `boot` muestra 7 puntos: los 6 vértices del hexágono y luego el centro. Cada uno es
una comprobación real (`boot::check`):
1. **idioma** (se pregunta si no se conoce): se cargan las traducciones del idioma elegido (`i18n::check`);
2. **pantalla**: la ventana ha obtenido un contexto OpenGL (`video::gl_info`, versión y tarjeta gráfica en el registro);
3. **reproductor de vídeo**: se crea, inicializa y destruye un reproductor mpv real (`mpv::self_test`);
4. **almacenamiento**: se escribe, se vuelve a leer y se borra un archivo en las carpetas de configuración, datos y caché;
5. **configuración**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` legibles
   (`config::unreadable_files`, llamado al principio de `main`, antes de que se reescriba un archivo dañado);
6. **red**: interfaz activa o ruta hacia el exterior (`discovery::has_network`);
7. **servidor** (el centro): la dirección principal, o si no la de respaldo, responde a `/System/Info/Public` **y**
   es el mismo servidor (identificador comparado con el `server_id` de la sesión); naranja mientras no haya servidor.

Rojo = fallo, con un mensaje y «Continuar» (solo al cabo de 12 s). Todo verde: los vértices se unen, los radios van
hacia el centro y el punto del servidor se convierte en el hexágono relleno — exactamente el logotipo
(`packaging/turtlefin.svg`, misma geometría) —, y luego un zoom hacia el centro. Slint reduce el dibujo de un
`Path` en el grosor de su trazo: los trazados del logotipo se agrandan otro tanto para caer sobre los puntos.

Pantalla de llegada (`boot::route`), en este orden: nombre + contraseña en la línea de comandos → inicio de
sesión; nombre de una cuenta guardada → esa cuenta (nombre desconocido: su formulario de inicio de sesión); cuenta
de inicio (`prefs.autostart_user` / `autostart_server`) → `fly_autostart` (la foto de la cuenta en el centro
durante el inicio de sesión); si no, «¿Quién está viendo?» (o la búsqueda de servidor si no se conoce ninguno).
Una cuenta de inicio que ha desaparecido lleva a «¿Quién está viendo?». `--no-intro` (o la animación «Inicio»
desactivada) se salta la animación; las comprobaciones se hacen igualmente.

## 5. Estado en la versión 1.0.0

Todas las funciones del README están hechas y comprobadas con capturas de pantalla (PC con Windows, pantallas de
1366 x 768 y 1920 x 1080) y en una máquina Linux ARM conectada a una tele, con las cuentas de prueba `test` /
`test2` de un servidor real. Versiones publicadas por la CI: 0.9.0, 0.9.1; la 1.0.0 está lista para etiquetarse
(sección 7).

Mecanismos que no saltan a la vista en el código:
- **Imagen compartida** (`global Hero`): la imagen de una tarjeta vuela hasta el cartel de la ficha y vuelve a la
  tarjeta exacta al regresar (`Hero.want-id`, `hero-card-ok`).
- **Filas** (`global Rows`): desplazamiento propio de cada fila, recordado por clave; al cambiar de fila se llega a
  la tarjeta más cercana en pantalla (`row-to`, `detail-pick-down`).
- **Sin conexión**: `config::Flags` (userdata.json) guarda visto / favoritos / posiciones con una marca «por
  enviar»; `downloads::sync` los envía cuando vuelve el servidor (el dispositivo tiene la última palabra).
- **Direcciones**: `server_main` / `server_backup`; `watch_addresses` (20 s) pasa a la de respaldo y vuelve.
- **Watch party**: una conexión WebSocket por sesión (`sp_conn`), se detiene con 401 / 403, espera creciente.
- **Fotos de perfil**: caché en disco `avatar_<id>_still|anim.bin` + miniatura redonda `avatar_<id>_thumb.png`.
  La miniatura se muestra enseguida, el GIF completo se decodifica fuera del hilo de la UI
  (`avatar_cached_async`) y solo se renueva desde el servidor si ha cambiado (`avatar_fetch`). Animación de los
  GIF: `AnimSlot` (Login, Picker, Header, Fly) y un temporizador común; el avatar de la cabecera recibe todas sus
  imágenes una vez (`avatar-frames`) y solo cambia la imagen visible. GetAvatar `SetAvatar` responde 500 →
  alternativa `POST /UserImage`.
- **Inicio de sesión animado** (`fly-phase` 1 a 4): la foto va al centro, barra de carga, sale volando, luego llega
  la página principal y el avatar de la cabecera aparece (`me-pop`).
- **Cambio de idioma** (`dust.rs`, `ui/dust.slint`): al abrir la lista de idiomas, unas barras cubren cada línea
  de texto, toman el ancho de las nuevas palabras al elegir y luego las revelan. Las líneas se detectan comparando
  una captura de la página (`take_snapshot`) con una captura en la fuente `Turtlefin Blank` (letras vacías, mismos
  anchos, `tools/make-blank-font.py`). El reloj (Montserrat) queda excluido.
- **Visita guiada** (`tour-step` 0 a 10): cada paso deja la interfaz en el estado esperado o se da por hecho si el
  gesto ya está hecho; `tour-ev` se llama desde los `changed`.
- **Animaciones**: global `Motion` (12 interruptores: boot, pages, menu, select, scroll, panels, detail, player,
  search, login, language, tour), `config::AnimFlags`, presets integrados (Todas, Ligeras, Ninguna) y personales,
  exportados / importados en `.json` en `Turtlefin Presets`. Toda duración de animación se escribe
  `duration: Motion.x ? 300ms : 0ms`.
- **Escritura**: en el escritorio, los campos son `TextInput` (texto oculto, dibujado por `TypedText`); en modo TV,
  el teclado en pantalla (`Osk`). En los dos modos, una tecla imprimible que recibe la página va al campo
  (`typing-key`, `erase-key`, `Field.type`); en modo TV, Retroceso solo borra mientras quede texto. En el
  escritorio, un clic al lado de un campo no le quita el teclado (`focus-on-click: root.tv-mode`).
- **Teclas mantenidas**: Intro, Esc y Retroceso no repiten su acción (`event.repeat`), salvo Retroceso al borrar
  texto.
- **Temas** (`src/theme.rs`, global `Theme` de ui/theme.slint): ningún color fijo en la interfaz; las superficies
  translúcidas se escriben `Theme.fg.with-alpha(…)` (blanco en un tema oscuro, tinta en uno claro), el texto sobre el
  degradado de acento usa `Theme.on-accent`. `Gloss` (brillo de gel) y `AeroSky` (ui/sky.slint) solo aparecen con
  `Theme.gloss` / `Theme.bubbles`. Los temas importados se guardan en prefs.json (`theme`, `themes`); el creador
  (`tools/theme-creator.html`) va integrado en el programa (`theme::CREATOR`) y «Crear un tema» lo deja en
  «Turtlefin Themes». Sus temas de partida deben ser idénticos a los de theme.rs.
- **Ratón**: clic en todas partes; rueda en la página principal, la ficha (`d-nav`, compartido con el teclado), la
  búsqueda, las bibliotecas, las descargas y los ajustes; las ventanas en primer plano absorben la rueda.

Sin hacer: Quick Connect; mando de juegos; licencia (la elige quien mantiene el proyecto, antes o después de la
1.0.0); pasarela con XeLauncher (lanzador del centro multimedia de quien mantiene el proyecto, no prioritario).
Previsto a continuación: optimización, versión para Android TV.

## 6. Traducciones

- Se hacen en tiempo de ejecución con `src/i18n.rs`: un mismo catálogo para Rust y para la interfaz.
  Slint: global `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)`; `Tr.k` cambia con cada cambio de idioma, lo que hace recalcular
  los textos. Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- El texto francés **es** la clave: cambiarlo obliga a cambiar el `msgid` de cada `lang/*.po`
  (`tools/lang-check.py` avisa de los textos que faltan o sobran y de los `{}` perdidos). Los 8 idiomas integrados
  (fr, en, es, de, it, pt, pl, nl; `BUILTIN` en i18n.rs) están completos (~530 textos).
- Idiomas añadidos: cualquier `<code>.po` de la carpeta `Turtlefin Languages` (`i18n::lang_dir`: Documentos, junto
  al exe en la versión portátil, dentro de `TURTLEFIN_CONFIG_DIR` en las pruebas; la antigua carpeta `languages` se
  mueve allí), subcarpetas incluidas; nombre leído de `X-Language-Name`; un archivo puede sustituir a un idioma
  integrado. Se recorren con un explorador integrado (`ui/langx.slint`, funciones `lx_*` de main.rs).
- «Crear la plantilla» escribe `modele.po`: msgid en **inglés**, cabecera `X-Source-Language: en`, notas `#.` en
  francés y en el idioma actual; `keyed` devuelve esos msgid al francés mediante `lang/en.po`.
- Textos que faltan en un idioma: inglés. Plurales: regla `Plural-Forms` del archivo, evaluada por i18n.rs.
- Instalador: `[Languages]` y `[CustomMessages]` de `turtlefin.iss`; escribe el código elegido en `language` junto
  al exe, que se usa en el primer inicio.
- Los nombres que vienen del servidor (bibliotecas, medios) no se traducen.

## 7. Compilar, fabricar los paquetes, publicar

Desarrollo:
- Windows: Rust (https://rustup.rs), «Herramientas de compilación de Visual Studio» (C++), git;
  `cargo build --release`; `libmpv-2.dll` (archivo `mpv-dev-x86_64-….7z` de
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) junto al exe.
  La versión debug necesita `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev` y después `cargo build --release`.
- `cargo test --release`: pruebas de i18n, update, etc.

Publicar una versión:
1. Poner el número en `Cargo.toml` (`version = "x.y.z"`), compilar una vez (actualiza `Cargo.lock`) y hacer commit.
2. `git push origin main`, luego `git tag -a vx.y.z -m "Novedades, una por línea"` y `git push origin vx.y.z`.
   El mensaje de la etiqueta se convierte en las notas de la versión, que muestra la actualización integrada.
3. `release.yml` compila Windows x64 / x86 y Linux x86_64 / aarch64, fabrica instaladores, archivos comprimidos,
   AppImage y `.deb`, y los publica en una Release (se sigue en la pestaña Actions del repositorio). Los nombres
   de archivo (cabecera de `release.yml`) los esperan tal cual `update.rs` y los scripts de instalación.
4. Prueba sin publicar: `git push origin main:ci` (se compila todo, no se publica nada).

A mano:
- **Windows**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (o `x86`);
  necesita Inno Setup 6, 7-Zip y NASM (x86). Resultado en `target\dist`.
- **Linux** (en una máquina Linux): `TURTLEFIN_DIST=release cargo build --release`, y luego
  `sh packaging/linux/build-appimage.sh <version>` y `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` al compilar; si no, `update::kind()` cree que es una versión compilada en local.
- x86: las libmpv de 32 bits de shinchiro publicadas desde julio de 2026 se cierran al arrancar (OpenSSL); la del
  10 de junio de 2026 se guarda en la prepublicación `libmpv-i686-20260610` de Turtlefin (no borrarla), que usa
  `build.ps1` (`MPV_TAG`: otra versión de shinchiro). aws-lc necesita NASM en 32 bits.
- Iconos: `packaging/turtlefin.svg` es el logotipo; `packaging/icons/make-icons.py <carpeta>` (Python + Pillow)
  regenera los PNG y el ICO.

Ramas: `main` (única rama de trabajo), `ci` (pruebas de la CI). `interface-lua` y `libmpv` son experimentos
antiguos, ya fusionados en `main`: se pueden borrar. Etiquetas: `v0.9.0`, `v0.9.1` (versiones publicadas) y
`libmpv-i686-20260610` (libmpv de 32 bits, ver arriba).

## 8. Pruebas

- `TURTLEFIN_CONFIG_DIR=<carpeta>`: otra carpeta de configuración (cuentas, prefs, idiomas) sin tocar la real.
- `--open=ID|settings|downloads`, `--play=ID@SEGUNDOS`, `--test-video=archivo` (reproductor sin servidor).
- `TURTLEFIN_DEBUG_FRAMES=1` (imágenes > 25 ms); `TURTLEFIN_DEBUG_GAPS=1` con
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed`: pausas de más de 40 ms entre dos imágenes.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: mensajes de la watch party. Dos instancias en un mismo PC: dos
  `TURTLEFIN_CONFIG_DIR` distintos, `--desktop`.
- Un `prefs.json` escrito por PowerShell 5 lleva BOM: la lectura de los archivos de configuración lo ignora.
- Pruebas automatizadas en Windows: `SetForegroundWindow` solo se acepta tras una tecla simulada; usar F24, no Alt
  (Alt sola pone la ventana en modo menú y se pierde el clic siguiente). Una tecla mantenida se simula con varias
  llamadas seguidas a `keybd_event` de «tecla pulsada» (Windows las marca como repeticiones).

## 9. Problemas conocidos / límites

1. Un fallo de mpv cierra Turtlefin (mismo proceso).
2. La reproducción exige renderizado OpenGL (`SLINT_BACKEND=winit-software` la impide).
3. **mpv 0.40 / 0.41** (corregido en mpv el 23 de enero de 2026, commit f74adc4): una barrera OpenGL por imagen que
   nunca se libera; con el controlador v3d cada una ocupa un descriptor («MESA: error: Export failed» al cabo de
   ~42 s). Solución en `src/mpv.rs` (solo OpenGL ES). Diagnóstico: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l`
   debe mantenerse estable durante la reproducción.
4. Crecimiento de la RAM de mpv (~3 MB/min) con subtítulos ASS: limitado a una reproducción (mpv se recrea en cada
   vídeo).
5. Token en claro en `session.json` / `accounts.json` (0600 en Unix).
6. Tearing en Xorg sin compositor (Openbox solo): usar un compositor (picom `--backend egl --vsync`). Turtlefin
   mantiene 60 imágenes/s.
7. Decodificación por software en Linux ARM: puede costar en 4K / HEVC; pista: `TURTLEFIN_HWDEC=auto-copy`.
8. Micropausas (~40 ms) medidas solo con el renderizado forzado a máxima velocidad, en cada imagen de un GIF de la
   cabecera y en la unión del logotipo; causa no encontrada, invisibles en uso normal.

## 10. Preferencias de trabajo de quien mantiene el proyecto

- Respuestas en francés; poca experiencia con Linux / SSH: explicar los comandos.
- No subir nada a GitHub ni publicar ninguna versión sin su aprobación explícita; las versiones las publica
  quien mantiene el proyecto.
- Pruebas con una copia de la configuración (`TURTLEFIN_CONFIG_DIR`), nunca la real; cuentas de prueba `test` /
  `test2`.
- Tras cada tanda de cambios: un instalador de prueba en su Escritorio
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, borrando el anterior).
