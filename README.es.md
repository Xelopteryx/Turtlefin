<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Logotipo de Turtlefin"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · **Español** · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md)

**Un cliente de Jellyfin nativo, ligero y animado, pensado tanto para el salón como para el escritorio.** Escrito
en Rust con Slint (interfaz) y libmpv (reproducción): sin Qt y sin navegador integrado. Funciona en cualquier
ordenador con Windows o Linux, desde un portátil viejo hasta una pequeña caja conectada a la tele, y se usa igual
de bien con un mando a distancia que con teclado y ratón.

> **Proyecto totalmente «vibecodeado»**: todo el código, los textos y las traducciones los escribió una IA
> (Claude, de Anthropic), guiada y probada por un humano que no lo programó él mismo.

Versión actual: **1.0.0** · Idiomas de la interfaz: Español, English, Français, Deutsch, Italiano, Português,
Polski, Nederlands — y cualquier idioma que añadas tú mismo (ver [Traducir Turtlefin](#traducir-turtlefin)).

## Qué hace

- **Cuentas**: pantalla «¿Quién está viendo?» con fotos de perfil (GIF animados incluidos, en caché), hasta
  12 cuentas guardadas en el dispositivo (solo el token, nunca la contraseña), cambio de cuenta sin volver a
  escribir nada, «Gestionar cuentas» para quitar alguna. Búsqueda de servidores en todas las redes del
  dispositivo; una dirección principal y una dirección de respaldo, que se prueba cuando la principal no responde.
- **Inicio**: un logotipo animado cuyos siete puntos son comprobaciones reales (idioma, pantalla, reproductor de
  vídeo, almacenamiento, configuración, red y, en el centro, el servidor), y luego «¿Quién está viendo?» — o
  directamente la cuenta elegida en Ajustes → Cuenta → **Abrir esta cuenta al iniciar**.
- **Página principal**: Mis medios, Continuar viendo, A continuación, Añadido recientemente; pestañas Favoritos y
  Solicitudes (Seerr). Carteles con episodios pendientes, marca de «visto» y nota; fondo tomado del contenido
  seleccionado.
- **Fichas**: película, serie, temporada, episodio; reproducir, favorito, visto, descarga, elección de audio /
  subtítulos recordada para toda la serie; «Más como esto» y sugerencias de Seerr; solicitud de temporadas que
  faltan.
- **Reproducción** (libmpv): capítulos, episodios de la temporada, «Saltar intro», episodio siguiente,
  sugerencias al final de una serie, volumen propio de Turtlefin; posición y «visto» enviados al servidor.
- **Watch party** (SyncPlay): ver lo mismo a la vez en varios dispositivos.
- **Sin conexión**: las descargas sustituyen a la página principal, con fichas completas sin servidor; lo visto o
  marcado como favorito sin conexión se envía a la cuenta al reconectar.
- **Búsqueda** (biblioteca + Seerr) y contenido al azar.
- **Mando, teclado y ratón en todas partes**: interfaz TV (elementos grandes, teclado en pantalla) o interfaz de
  ordenador (ventana o pantalla completa, F11), clic y rueda en todas las páginas, y un teclado físico escribe
  directamente en los campos de texto, en los dos modos.
- **Ajustes**: foto de perfil (avatares de GetAvatar por categoría), idioma de la interfaz, idiomas de audio y
  subtítulos, tamaño de los subtítulos, episodio siguiente e intro automáticos, interfaz TV, pantalla completa,
  fondo, notas, hora, direcciones del servidor, caché de imágenes, **actualización desde GitHub**.
- **Animaciones** en todas partes (cartel que vuela hacia la ficha, menú que se desliza, filas en cascada, inicio
  de sesión animado, cambio de idioma), ajustables una a una en Ajustes → Animaciones, con presets (Todas,
  Ligeras, Ninguna) y los tuyos, que se pueden exportar e importar.
- **Visita guiada**: se ofrece en el primer inicio y se puede volver a ver en Ajustes → Acerca de.

## Instalar

Nada que compilar: se descarga, se instala y listo. Todos los archivos están en la página
[Releases](https://github.com/Xelopteryx/Turtlefin/releases).

### Con un solo comando

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

En Windows se descarga el último instalador y se ejecuta para tu cuenta de usuario (sin permisos de
administrador). En Debian, Ubuntu, Linux Mint y las demás distribuciones con apt se instala el paquete `.deb` (se
pide tu contraseña); en las demás, el AppImage va a `~/.local/bin` con una entrada en el menú de aplicaciones.

### A mano

| Sistema | Archivo | Notas |
|---|---|---|
| Windows 64 bits | `Turtlefin-<version>-windows-x64-setup.exe` | Instalador: idioma, carpeta y modo **instalado** (menú Inicio, desinstalación) o **portátil** |
| Windows 64 bits, sin instalar | `Turtlefin-<version>-windows-x64-portable.zip` | Descomprimir donde se quiera (memoria USB…) y abrir `turtlefin.exe` |
| Windows 32 bits | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Para PC antiguos |
| Linux, cualquier distribución (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Hacerlo ejecutable (`chmod +x`) y abrirlo |
| Linux, ARM 64 bits | `Turtlefin-<version>-linux-aarch64.AppImage` | Igual |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Doble clic, o `sudo apt install ./turtlefin_….deb` |

Las versiones de Windows y el AppImage lo contienen todo (reproductor libmpv incluido). El paquete `.deb` usa la
libmpv del sistema (`libmpv2`, instalada automáticamente por apt). Los AppImage y paquetes requieren una
distribución de 2022 o más reciente (Ubuntu 22.04, Debian 12…).

**Versión portátil**: un archivo `portable` junto a `turtlefin.exe` hace que la configuración, las cuentas, la
caché y las descargas se guarden en la carpeta `data` junto al programa; no se escribe nada en ningún otro sitio.

### Actualizar

**Ajustes → Acerca de → Buscar actualizaciones** compara la versión instalada con la última publicada; luego
**Actualizar** se encarga de todo según cómo esté instalado Turtlefin:

| Instalación | Actualización |
|---|---|
| Windows, instalado | se descarga el nuevo instalador y se ejecuta en silencio en la misma carpeta |
| Windows, portátil | se descarga el nuevo archivo comprimido y sus archivos sustituyen a los antiguos |
| AppImage | el nuevo archivo sustituye al antiguo |
| Paquete .deb | el paquete se instala con `pkexec` (se pide la contraseña de administrador) |

**Reiniciar Turtlefin** abre después la nueva versión.

## Plugins de servidor recomendados

Turtlefin funciona con un servidor Jellyfin sencillo (10.11 o más reciente). Estos plugins, instalados en el
**servidor**, añaden funciones:

| Plugin | Qué aporta a Turtlefin |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Pestaña Solicitudes, resultados de Seerr en la búsqueda, sugerencias y solicitudes de Seerr en las fichas — con el inicio de sesión de Jellyfin, sin clave de Seerr en el cliente |
| [Seerr](https://github.com/seerr-team/seerr) (antes Jellyseerr) | El propio gestor de solicitudes, usado por Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Detecta intros y créditos: botón «Saltar intro», salto automático, «Episodio siguiente» en el momento justo |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | Una galería de fotos de perfil para elegir en Ajustes → Cuenta |

## Abrir

```
turtlefin                                  animación de inicio y luego «¿Quién está viendo?» (o la cuenta de inicio)
turtlefin "Nombre"                         cuenta guardada «Nombre»
turtlefin "Nombre" --server=http://…       inicio de sesión directo (contraseña: variable TURTLEFIN_PASSWORD=…)
turtlefin --tv                             interfaz TV: pantalla completa, elementos grandes
turtlefin --desktop                        interfaz de ordenador (tiene prioridad sobre el ajuste «Interfaz TV»)
turtlefin --no-intro                       sin animación de inicio
turtlefin --console                        ventana de registro (Windows)
turtlefin --tutorial                       visita guiada al llegar a la página principal
```

La línea de comandos siempre tiene prioridad sobre los ajustes (cuenta de inicio, interfaz TV).

## Teclas y ratón

- **Flechas** para moverse, **Intro** para abrir / activar, **Esc** o **Retroceso** para volver (mantenida
  pulsada: un solo paso atrás).
- Página principal: **←** en la primera tarjeta (o Atrás) abre el menú; en el menú, **→** o Esc lo cierra.
- **↑** desde la parte superior de una página: barra superior (atrás, inicio, menú, watch party, azar, búsqueda,
  cuenta).
- Reproducción, controles ocultos: **← →** retroceder / avanzar 10 s, **↑ ↓** o Intro muestran los controles,
  ↓ desde los botones: episodios de la temporada. **Espacio**: pausa · `a` audio · `s` subtítulos · `f` pantalla
  completa.
- **F11**, en todas partes: pantalla completa (también en Ajustes → Pantalla → **Pantalla completa**, que se
  recuerda entre inicios fuera de la interfaz TV).
- **Ratón**: clic para abrir, rueda para pasar de una fila a otra (Mayús + rueda: dentro de la fila).
- **Texto**: una letra escrita en el teclado va directamente al campo (búsqueda, inicio de sesión, dirección),
  incluso en la interfaz TV; en ella, un clic en el campo de búsqueda abre el teclado en pantalla.

## Archivos

| Dónde | Qué |
|---|---|
| carpeta de configuración, `turtlefin/` | `session.json` (sesión actual), `accounts.json` (cuentas guardadas), `prefs.json` (ajustes del dispositivo), `tracks.json` (pistas por serie), `userdata.json` (visto / favoritos sin conexión) |
| carpeta de datos, `turtlefin/downloads/` | descargas (medio, carteles, fondo, logotipo, `info.json`), `queue.json` (cola pendiente) |
| carpeta de caché, `turtlefin/img/` | imágenes y fotos de perfil (se puede vaciar en Acerca de) |
| **Documentos** | `Turtlefin Languages` (idiomas añadidos) y `Turtlefin Presets` (presets de animaciones exportados) |

En Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
En Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Versión portátil: todo en `data\` junto a `turtlefin.exe` (`config`, `cache`, `downloads`), y las carpetas de
idiomas y de presets junto al programa.

## Si algo falla

- `turtlefin --console` (Windows) o abrirlo desde un terminal (Linux) muestra el registro.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: registro de mpv · `TURTLEFIN_MPV_ARGS="…"`: opciones de mpv adicionales.
- `TURTLEFIN_HWDEC=auto-copy`: decodificación por hardware (por software de forma predeterminada en Linux ARM) · `TURTLEFIN_AO=alsa`: salida de sonido.
- `TURTLEFIN_LIBMPV=ruta`: otra ubicación de libmpv · `TURTLEFIN_DEBUG_FRAMES=1`: avisa de las imágenes lentas.
- El renderizado debe ser OpenGL (se elige automáticamente): con `SLINT_BACKEND=winit-software` no hay vídeo.

## Temas

Ajustes → Pantalla → **Tema**: Turtlefin (predeterminado), Oscuro, Claro, Frutiger Aero (al estilo de Windows 7:
fondo «Harmony», cristal azulado, botones de gel brillantes, reproductor con orbe, inicio con esferas de cristal), Turtlefin verde — o un tema propio.

- **Crear un tema** abre *Turtlefin Theme Creator* en el navegador: cada color, el redondeo, el brillo, el cielo y
  las burbujas, con vista previa en directo; guarda un archivo `.tftheme`.
- Poner ese archivo en la carpeta **Turtlefin Themes** (Documentos, o junto a `turtlefin.exe` en la versión
  portátil) y luego **Importar un tema**. **Exportar el tema** escribe el tema actual en esa carpeta, como punto de
  partida.
- El creador también está en el repositorio, `tools/theme-creator.html`: un solo archivo que funciona sin conexión.

## Traducir Turtlefin

Sin programar ni compilar. Los idiomas añadidos están en una sola carpeta, **Turtlefin Languages**: en
**Documentos** (versión instalada) o junto a `turtlefin.exe` (versión portátil).

1. **Ajustes → Pantalla → Añadir un idioma** abre un explorador de esa carpeta; **Crear la plantilla** escribe en
   ella `modele.po`, con los textos en inglés y, como nota, el francés original y el idioma actual.
2. Hacer una copia llamada `<código>.po` (`sv.po` para el sueco, `ja.po` para el japonés…), en esa carpeta o en
   una subcarpeta, y rellenar cada `msgstr ""` con la traducción del `msgid` inglés que tiene encima. Conservar
   los `{}` y `{n}`. Rellenar también `X-Language-Name` (nombre mostrado) y, si hace falta, `Plural-Forms` (regla
   gettext del idioma). Sirve cualquier editor de `.po`, por ejemplo [Poedit](https://poedit.net).
3. De nuevo **Añadir un idioma**: la traducción aparece con la parte ya traducida; al elegirla se aplica. Los
   textos que se dejan vacíos se muestran en inglés.

Para compartirla con todo el mundo: una pull request que añada el archivo a `lang/` (y a `BUILTIN` en
`src/i18n.rs`); `python tools/lang-check.py` comprueba que no falte nada.

## Desarrollo

Ver [HANDOFF.es.md](HANDOFF.es.md) (también en [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md), [Polski](HANDOFF.pl.md),
[Português](HANDOFF.pt.md)): estado del proyecto, decisiones, compilación, paquetes, publicación, traducciones,
problemas conocidos.

## Licencia

Turtlefin se publica bajo la [GNU GPL v3](LICENSE) (o una versión posterior): cualquiera puede usarlo, estudiarlo,
modificarlo y redistribuirlo, siempre que conserve la misma licencia y comparta el código de sus cambios.
