# Turtlefin: passagem do projeto (estado a 9 de outubro de 2026, versão 1.1.0)

[Français](HANDOFF.md) · [English](HANDOFF.en.md) · [Deutsch](HANDOFF.de.md) · [Español](HANDOFF.es.md) · [Italiano](HANDOFF.it.md) · [Nederlands](HANDOFF.nl.md) · [Polski](HANDOFF.pl.md) · **Português**

Para quem retomar o desenvolvimento (uma pessoa ou o Claude Code). Lê-o todo antes de mexer no código e depois lê
[README.pt.md](README.pt.md) (utilização, instalação, teclas, ficheiros).
Repositório: https://github.com/Xelopteryx/Turtlefin · Versão em `Cargo.toml`: 1.1.0.

## 1. Objetivo

Um **cliente Jellyfin nativo em Rust**, leve, animado, utilizável com um comando tal como com teclado e rato,
instalável em qualquer computador Windows ou Linux **sem compilar nada**: quem o usa transfere um instalador ou
um pacote (ou executa um comando de instalação), e pronto. Todos os pacotes são feitos pela CI do GitHub (ou pelo
PC de quem mantém o projeto), nunca pelo utilizador.

Porquê: o Jellyfin Desktop (Qt / QtWebEngine) perde memória e acaba por falhar nas máquinas pequenas, e a interface
web com um tema pesado cai abaixo das 30 imagens/s em hardware modesto. Regra permanente: manter a memória estável
e nunca voltar a introduzir desfocagem em tempo real nem animações de filtros.

## 2. Decisões tomadas

| Assunto | Decisão | Motivo |
|---|---|---|
| Linguagem / UI | Rust + **Slint** `~1.18` (renderização 100 % Slint), estilo `fluent-dark` imposto por `build.rs` | Sem navegador; o estilo «native» dependeria do Qt |
| Funcionalidade Slint `unstable-winit-030` | Filtro de eventos winit para **F11** (`install_f11`) | Única forma de ter uma tecla global; daí `~1.18` (API instável entre versões menores) |
| Rede | `reqwest` 0.13 (rustls, arquivo de certificados do sistema), `tokio` | `query` é uma funcionalidade a ativar na 0.13 |
| Reprodução | **libmpv carregada em tempo de execução** (`libloading`, `src/mpv.rs`), renderização OpenGL numa textura mostrada pelo Slint (`src/video.rs`), comandos Slint por cima (`ui/player.slint`) | Leitor integrado, sem IPC, funciona em Wayland. O leitor é recriado em cada reprodução (memória limitada) |
| Renderizador Slint | femtovg (OpenGL / GLES) imposto, salvo se `SLINT_BACKEND` estiver definido | O vídeo passa por uma textura OpenGL |
| Textura de vídeo | Píxeis físicos, origem `TopLeft`, estado GL guardado / reposto à volta do mpv; `loadfile` espera pelo contexto de renderização | Senão, imagem ao contrário ou «No render context set» |
| Descodificação | `hwdec=no` no Linux ARM de 64 bits, `auto-safe` no resto (Windows: `d3d11va-copy`) | Nas placas ARM testadas (controlador v3d), a descodificação V4L2 produz um formato que o renderizador não sabe importar |
| Áudio no Linux | `ao=pipewire,pulse,alsa`, `config=no` | Um `mpv.conf` do utilizador que impunha ALSA falhava quando o PipeWire tem a saída HDMI |
| Memória | Cache do mpv limitada (100 / 25 MiB), imagens pedidas no tamanho certo, 16 elementos por fila | Máquinas pequenas (4 GB) |
| Idiomas | Francês no código (idioma de origem), tradução em tempo de execução por `src/i18n.rs` a partir de `lang/<code>.po` (integrados) e da pasta `Turtlefin Languages` | Ver a secção 6 |
| Ecrã inteiro (Windows) | Janela sem margens que cobre o ecrã + 1 px (`src/winfull.rs`, `set_tv_window`), não o verdadeiro ecrã inteiro; segue as mudanças de resolução (a cada 3 s). `TURTLEFIN_TRUE_FULLSCREEN=1` para o ecrã inteiro do Slint | Em ecrã inteiro OpenGL, o AMD Software toma a aplicação por um jogo: «Prima ALT + R» sempre que volta ao primeiro plano |
| Interface TV / de computador | `tv-mode` só muda o tamanho (`k` = 1,4) e a escrita (teclado no ecrã); o ecrã inteiro é à parte (`full_flag`, `UiPrefs::fullscreen`, F11) | Pedido do utilizador: ecrã inteiro sem aumentar a interface |
| Janela (Windows, ambiente de trabalho) | Reduzida e centrada se 1280 x 720 + moldura ultrapassar a área de trabalho; continua animada ao ser movida (`winfull::keep_alive_while_moving`) | Ecrãs de 1366 x 768; o Windows bloqueia o ciclo de eventos enquanto uma janela é movida |
| Janela de consola (Windows) | Subsistema «windows» em release; `--console` liga-se a uma consola ou abre uma; comandos externos sem janela (`paths::quiet_command`) | Sem consola nem janela CMD a piscar |
| Linha de comandos | Tem sempre prioridade sobre as definições (conta de arranque, interface TV) | Pedido do utilizador |
| Palavra-passe | Nunca guardada (só o token); na linha de comandos, preferir `TURTLEFIN_PASSWORD` | Um argumento é visível para os outros processos |

## 3. Estrutura do código

```
build.rs            commit compilado, estilo Slint, ícone do exe (winresource, Windows)
lang/<code>.po      traduções integradas (origem: o francês do código); tools/lang-check.py verifica-as
ui/theme.slint      tokens do tema, globais Tr (tradução) e Motion (animações ativas)
ui/app.slint        AppWindow e todos os ecrãs (login, loading, home, detail, library, search, settings…)
ui/boot.slint       BootLogo: animação de arranque (7 pontos, ligação, zoom), escolha do idioma
ui/player.slint     ecrã de reprodução         ui/osk.slint      teclado no ecrã
ui/card.slint       cartão de cartaz           ui/marquee.slint  texto em deslocação
ui/typed.slint      texto escrito animado (Str, TypedText)   ui/langx.slint  explorador (idiomas, presets)
ui/dust.slint       barras da mudança de idioma              ui/fonts/       Montserrat, Turtlefin Blank
src/main.rs         CLI, estado partilhado App (Arc), ecrãs, navegação (pilha + páginas guardadas), definições
src/boot.rs         sequência de arranque (verificações, idioma, escolha do ecrã de chegada)
src/i18n.rs         idioma atual, tr() / trf() / trn(), idiomas acrescentados, modelo de tradução
src/dust.rs         efeito da mudança de idioma (deteção das linhas de texto, morphing)
src/api.rs          cliente REST do Jellyfin (+ retransmissão Seerr do Jellyfin Enhanced, GetAvatar)
src/config.rs       sessão, contas (12 no máximo), prefs.json (UiPrefs, AnimFlags, presets), faixas, vistos/favoritos offline
src/discovery.rs    procura de servidores (UDP, sub-redes, ARP, pares VPN)
src/downloads.rs    transferências (retoma Range, fila, sincronização offline)
src/mpv.rs          ligação à libmpv; src/video.rs textura OpenGL; src/player.rs reprodução, relatórios, encadeamento
src/syncplay.rs     watch party (WebSocket /socket)
src/paths.rs        pastas de configuração / cache / dados; modo portátil; quiet_command
src/update.rs       atualização conforme a instalação (Kind: Source, WinInstalled, WinPortable, AppImage, Deb)
src/winfull.rs      Windows: ecrã / área de trabalho, janela animada ao ser movida
src/theme.rs        temas integrados, ficheiros .tftheme (ThemeDef), aplicação ao global Theme
ui/sky.slint        cenários (céu e bolhas · «Harmony» do Windows 7), estáticos, em cache
packaging/          windows/ (turtlefin.iss, build.ps1), linux/ (build-appimage.sh, .desktop),
                    icons/ (ICO, PNG, make-icons.py), turtlefin.svg (logótipo), install.ps1 / install.sh
tools/              lang-check.py, make-blank-font.py, theme-creator.html (Turtlefin Theme Creator)
.github/workflows/release.yml   compilação e publicação com uma etiqueta `v*` (ensaio: ramo `ci`)
```

Princípios:
- Os dados de rede passam por estruturas `Send` e depois `upgrade_in_event_loop` põe-nos nos modelos do Slint.
  Imagens descodificadas fora da thread da UI, 6 transferências em paralelo, aplicadas com uma verificação do id.
- `App.gen` invalida os carregamentos desatualizados; `App.stack` é a pilha de navegação; `PAGES` guarda fichas e
  bibliotecas para voltar sem pedidos.
- Navegação por teclado feita à mão (índices de seleção em Rust e em Slint), porque o Slint não gere o foco de
  cartões dinâmicos. Cada ecrã tem o seu `FocusScope`; `refocus` devolve o teclado ao sítio certo.
- Os `changed` do Slint são diferidos: não contar com a sua ordem (posições em dois tempos, etc.).
- Jellyfin 10.11: `/UserViews`, `/UserItems/Resume`, `/Shows/NextUp`, `/Items/Latest`, `/Items/{id}`, cabeçalho
  `Authorization: MediaBrowser …, Token=…`. Relatórios: `/Sessions/Playing`, `/Progress`, `/Stopped`.
  `/Items/{id}/Download` e o WebSocket recusam `api_key`: token no cabeçalho.

## 4. Arranque

`main` aplica o idioma (prefs.json; senão, o ficheiro `language` escrito pelo instalador do Windows) e depois lança
`boot::run`. O ecrã `boot` mostra 7 pontos: os 6 vértices do hexágono e depois o centro. Cada um é uma verificação
real (`boot::check`):
1. **idioma** (perguntado se for desconhecido): carregam-se as traduções do idioma escolhido (`i18n::check`);
2. **ecrã**: a janela obteve um contexto OpenGL (`video::gl_info`, versão e placa gráfica no registo);
3. **leitor de vídeo**: um verdadeiro leitor mpv é criado, inicializado e destruído (`mpv::self_test`);
4. **armazenamento**: um ficheiro é escrito, relido e apagado nas pastas de configuração, dados e cache;
5. **configuração**: `session.json`, `accounts.json`, `prefs.json`, `tracks.json`, `userdata.json` legíveis
   (`config::unreadable_files`, chamado logo no início de `main`, antes de um ficheiro danificado ser reescrito);
6. **rede**: interface ativa ou rota para o exterior (`discovery::has_network`);
7. **servidor** (o centro): o endereço principal, ou senão o alternativo, responde a `/System/Info/Public` **e** é o
   mesmo servidor (identificador comparado com o `server_id` da sessão); laranja enquanto não houver servidor.

Vermelho = falha, com uma mensagem e «Continuar» (sozinho ao fim de 12 s). Tudo verde: os vértices ligam-se, os
raios vão para o centro e o ponto do servidor torna-se o hexágono cheio — exatamente o logótipo
(`packaging/turtlefin.svg`, a mesma geometria) —, depois um zoom para o centro. O Slint reduz o desenho de um
`Path` na espessura do traço: os caminhos do logótipo estão aumentados outro tanto para caírem sobre os pontos.

Ecrã de chegada (`boot::route`), por esta ordem: nome + palavra-passe na linha de comandos → início de sessão; nome
de uma conta guardada → essa conta (nome desconhecido: o seu formulário de início de sessão); conta de arranque
(`prefs.autostart_user` / `autostart_server`) → `fly_autostart` (a foto da conta ao centro durante o início de
sessão); senão «Quem está a ver?» (ou a procura de servidor se não se conhecer nenhum). Uma conta de arranque que
desapareceu leva a «Quem está a ver?». `--no-intro` (ou a animação «Arranque» desligada) salta a animação; as
verificações fazem-se na mesma.

## 5. Estado na versão 1.0.0

Todas as funções do README estão feitas e verificadas em capturas de ecrã (PC Windows, ecrãs de 1366 x 768 e
1920 x 1080) e numa máquina Linux ARM ligada a uma televisão, com as contas de teste `test` / `test2` de um
servidor real. Versões publicadas pela CI: 0.9.0, 0.9.1; a 1.0.0 está pronta para ser etiquetada (secção 7).

Mecanismos que não são evidentes no código:
- **Imagem partilhada** (`global Hero`): a imagem de um cartão voa até ao cartaz da ficha e volta ao cartão exato
  no regresso (`Hero.want-id`, `hero-card-ok`).
- **Filas** (`global Rows`): deslocação própria de cada fila, memorizada por chave; ao mudar de fila chega-se ao
  cartão mais próximo no ecrã (`row-to`, `detail-pick-down`).
- **Offline**: `config::Flags` (userdata.json) guarda vistos / favoritos / posições com uma marca «a enviar»;
  `downloads::sync` envia-os quando o servidor volta (o dispositivo tem a última palavra).
- **Endereços**: `server_main` / `server_backup`; `watch_addresses` (20 s) passa para o alternativo e volta.
- **Watch party**: uma ligação WebSocket por sessão (`sp_conn`), para com 401 / 403, espera crescente.
- **Fotos de perfil**: cache em disco `avatar_<id>_still|anim.bin` + miniatura redonda `avatar_<id>_thumb.png`. A
  miniatura é mostrada de imediato, o GIF completo é descodificado fora da thread da UI (`avatar_cached_async`) e
  só é renovado a partir do servidor se tiver mudado (`avatar_fetch`). Animação dos GIF: `AnimSlot` (Login,
  Picker, Header, Fly) e um temporizador comum; o avatar do cabeçalho recebe todas as imagens uma vez
  (`avatar-frames`) e só muda a imagem visível. GetAvatar `SetAvatar` responde 500 → alternativa `POST /UserImage`.
- **Início de sessão animado** (`fly-phase` de 1 a 4): a foto vai para o centro, barra de carregamento, voa para
  longe, depois chega a página inicial e o avatar do cabeçalho aparece (`me-pop`).
- **Mudança de idioma** (`dust.rs`, `ui/dust.slint`): ao abrir a lista de idiomas, barras cobrem cada linha de
  texto, ganham a largura das novas palavras na escolha e depois revelam-nas. As linhas são detetadas comparando
  uma captura da página (`take_snapshot`) com uma captura no tipo de letra `Turtlefin Blank` (letras vazias, as
  mesmas larguras, `tools/make-blank-font.py`). O relógio (Montserrat) fica de fora.
- **Visita guiada** (`tour-step` de 0 a 10): cada passo repõe a interface no estado esperado ou dá-se por feito se
  o gesto já tiver sido feito; `tour-ev` é chamado a partir dos `changed`.
- **Animações**: global `Motion` (12 interruptores: boot, pages, menu, select, scroll, panels, detail, player,
  search, login, language, tour), `config::AnimFlags`, presets integrados (Todas, Leves, Nenhum) e pessoais,
  exportados / importados em `.json` em `Turtlefin Presets`. Cada duração de animação escreve-se
  `duration: Motion.x ? 300ms : 0ms`.
- **Escrita**: no computador, os campos são `TextInput` (texto escondido, desenhado por `TypedText`); no modo TV,
  o teclado no ecrã (`Osk`). Nos dois modos, uma tecla imprimível recebida pela página vai para o campo
  (`typing-key`, `erase-key`, `Field.type`); no modo TV, Retrocesso só apaga enquanto houver texto. No computador,
  um clique ao lado de um campo não lhe tira o teclado (`focus-on-click: root.tv-mode`).
- **Teclas mantidas premidas**: Enter, Esc e Retrocesso não repetem a ação (`event.repeat`), exceto Retrocesso
  ao apagar texto.
- **Temas** (`src/theme.rs`, global `Theme` de ui/theme.slint): nenhuma cor fixa na interface; as superfícies
  translúcidas escrevem-se `Theme.fg.with-alpha(…)` (branco num tema escuro, tinta num claro), o texto sobre o
  gradiente de acento usa `Theme.on-accent`. `Gloss` (brilho de gel) e `AeroSky` (ui/sky.slint) só aparecem com
  `Theme.gloss` / `Theme.bubbles`. Os temas importados ficam em prefs.json (`theme`, `themes`); o criador
  (`tools/theme-creator.html`) está integrado no programa (`theme::CREATOR`) e «Criar um tema» põe-no em
  «Turtlefin Themes». Os seus temas de partida têm de ficar iguais aos de theme.rs.
- **Rato**: clique em todo o lado; roda na página inicial, na ficha (`d-nav`, partilhado com o teclado), na
  pesquisa, nas bibliotecas, nas transferências e nas definições; as janelas em primeiro plano absorvem a roda.

Por fazer: Quick Connect; comando de jogos; licença (a escolher por quem mantém o projeto, antes ou depois da
1.0.0); ligação ao XeLauncher (lançador do centro multimédia de quem mantém o projeto, sem prioridade). Previsto a
seguir: otimização, versão para Android TV.

## 6. Traduções

- Feitas em tempo de execução por `src/i18n.rs`: um só catálogo para Rust e para a interface.
  Slint: global `Tr` (ui/theme.slint) — `Tr.t(Tr.k, "…")`, `Tr.f(Tr.k, "… {} …", a, b)`,
  `Tr.p(Tr.k, "{n} serveur", "{n} serveurs", n)`; `Tr.k` muda a cada mudança de idioma, o que faz recalcular os
  textos. Rust: `tr("…")` (`&'static str`), `trf("… {} …", &[&x])`, `trn`.
- O texto francês **é** a chave: alterá-lo obriga a alterar o `msgid` em cada `lang/*.po`
  (`tools/lang-check.py` assinala textos em falta e a mais e `{}` perdidos). Os 8 idiomas integrados (fr, en, es,
  de, it, pt, pl, nl; `BUILTIN` em i18n.rs) estão completos (~530 textos).
- Idiomas acrescentados: qualquer `<code>.po` da pasta `Turtlefin Languages` (`i18n::lang_dir`: Documentos, ao lado
  do exe na versão portátil, dentro de `TURTLEFIN_CONFIG_DIR` nos testes; a antiga pasta `languages` é movida para
  lá), subpastas incluídas; nome lido de `X-Language-Name`; um ficheiro pode substituir um idioma integrado.
  Percorridos por um explorador integrado (`ui/langx.slint`, funções `lx_*` de main.rs).
- «Criar o modelo» escreve `modele.po`: msgid em **inglês**, cabeçalho `X-Source-Language: en`, notas `#.` em
  francês e no idioma atual; `keyed` devolve esses msgid ao francês através de `lang/en.po`.
- Textos em falta num idioma: inglês. Plurais: regra `Plural-Forms` do ficheiro, avaliada por i18n.rs.
- Instalador: `[Languages]` e `[CustomMessages]` de `turtlefin.iss`; escreve o código escolhido em `language` ao
  lado do exe, usado no primeiro arranque.
- Os nomes que vêm do servidor (bibliotecas, conteúdos) não são traduzidos.

## 7. Compilar, fazer os pacotes, publicar

Desenvolvimento:
- Windows: Rust (https://rustup.rs), «Ferramentas de Compilação do Visual Studio» (C++), git;
  `cargo build --release`; `libmpv-2.dll` (arquivo `mpv-dev-x86_64-….7z` de
  [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake/releases)) ao lado do exe.
  A versão debug precisa de `TURTLEFIN_LIBMPV=target/release/libmpv-2.dll`.
- Linux (Debian / Ubuntu): `sudo apt install build-essential pkg-config libfontconfig1-dev libxkbcommon-dev
  libmpv-dev` e depois `cargo build --release`.
- `cargo test --release`: testes de i18n, update, etc.

Publicar uma versão:
1. Pôr o número em `Cargo.toml` (`version = "x.y.z"`), compilar uma vez (atualiza `Cargo.lock`) e fazer commit.
2. `git push origin main`, depois `git tag -a vx.y.z -m "Novidades, uma por linha"` e `git push origin vx.y.z`.
   A mensagem da etiqueta passa a ser as notas da versão, mostradas pela atualização integrada.
3. `release.yml` compila Windows x64 / x86 e Linux x86_64 / aarch64, faz instaladores, arquivos, AppImage e
   pacotes `.deb`, e publica-os numa Release (acompanhar no separador Actions do repositório). Os nomes dos
   ficheiros (cabeçalho de `release.yml`) são esperados tal e qual por `update.rs` e pelos scripts de instalação.
4. Ensaio sem publicar: `git push origin main:ci` (compila-se tudo, não se publica nada).

À mão:
- **Windows**: `powershell -ExecutionPolicy Bypass -File packaging\windows\build.ps1 -Arch x64` (ou `x86`);
  são precisos Inno Setup 6, 7-Zip e NASM (x86). Resultado em `target\dist`.
- **Linux** (numa máquina Linux): `TURTLEFIN_DIST=release cargo build --release`, depois
  `sh packaging/linux/build-appimage.sh <version>` e `cargo deb --no-build`.
- `TURTLEFIN_DIST=release` na compilação; senão, `update::kind()` julga que é uma versão compilada localmente.
- x86: as libmpv de 32 bits do shinchiro publicadas desde julho de 2026 falham no arranque (OpenSSL); a de 10 de
  junho de 2026 está guardada na pré-versão `libmpv-i686-20260610` do Turtlefin (não apagar), usada por
  `build.ps1` (`MPV_TAG`: outra versão do shinchiro). O aws-lc precisa de NASM em 32 bits.
- Ícones: `packaging/turtlefin.svg` é o logótipo; `packaging/icons/make-icons.py <pasta>` (Python + Pillow) volta
  a gerar os PNG e o ICO.

Atualização de uma cópia compilada localmente (`Kind::Source`): os ficheiros modificados ou não seguidos são
postos de lado (`git stash -u`, recuperáveis com `git stash pop`) antes de `git pull --ff-only`. As novidades
aparecem numa janela própria (Definições → Acerca → Ver as novidades); `TURTLEFIN_TEST_UPDATE="Versão x|nota|nota"`
simula uma atualização para a experimentar. Atalhos nas cores do tema: `theme::apply_shortcuts` (.ico nos .lnk no
Windows, ícones `turtlefin` em ~/.local/share/icons no Linux), ignorados durante os testes (`TURTLEFIN_CONFIG_DIR`)
exceto com `TURTLEFIN_TEST_SHORTCUTS=1`.

Ramos: `main` (único ramo de trabalho), `ci` (ensaios da CI). `interface-lua` e `libmpv` são experiências antigas,
já integradas em `main`: podem ser apagados. Etiquetas: `v0.9.0`, `v0.9.1` (versões publicadas) e
`libmpv-i686-20260610` (libmpv de 32 bits, ver acima).

## 8. Testes

- `TURTLEFIN_CONFIG_DIR=<pasta>`: outra pasta de configuração (contas, prefs, idiomas) sem tocar na verdadeira.
- `--open=ID|settings|downloads`, `--play=ID@SEGUNDOS`, `--test-video=ficheiro` (leitor sem servidor).
- `TURTLEFIN_DEBUG_FRAMES=1` (imagens > 25 ms); `TURTLEFIN_DEBUG_GAPS=1` com
  `SLINT_DEBUG_PERFORMANCE=refresh_full_speed`: pausas de mais de 40 ms entre duas imagens.
- `TURTLEFIN_DEBUG_SYNCPLAY=1`: mensagens da watch party. Duas instâncias no mesmo PC: dois
  `TURTLEFIN_CONFIG_DIR` diferentes, `--desktop`.
- Um `prefs.json` escrito pelo PowerShell 5 tem BOM: a leitura dos ficheiros de configuração ignora-o.
- Testes automatizados no Windows: `SetForegroundWindow` só é aceite depois de uma tecla simulada; usar F24, não
  Alt (Alt sozinha põe a janela em modo de menu e o clique seguinte perde-se). Uma tecla mantida premida simula-se
  com várias chamadas seguidas a `keybd_event` de «tecla premida» (o Windows marca-as como repetições).

## 9. Problemas conhecidos / limites

1. Uma falha do mpv faz falhar o Turtlefin (mesmo processo).
2. A reprodução exige renderização OpenGL (`SLINT_BACKEND=winit-software` impede-a).
3. **mpv 0.40 / 0.41** (corrigido no mpv a 23 de janeiro de 2026, commit f74adc4): uma barreira OpenGL por imagem
   nunca libertada; com o controlador v3d cada uma ocupa um descritor («MESA: error: Export failed» ao fim de
   ~42 s). Contorno em `src/mpv.rs` (só OpenGL ES). Diagnóstico: `ls /proc/$(pgrep -x turtlefin)/fd | wc -l` deve
   manter-se estável durante a reprodução.
4. Crescimento da RAM do mpv (~3 MB/min) com legendas ASS: limitado a uma reprodução (mpv recriado em cada vídeo).
5. Token em texto simples em `session.json` / `accounts.json` (0600 em Unix).
6. Tearing no Xorg sem compositor (só Openbox): usar um compositor (picom `--backend egl --vsync`). O Turtlefin
   mantém 60 imagens/s.
7. Descodificação por software no Linux ARM: pode ter dificuldades em 4K / HEVC; pista: `TURTLEFIN_HWDEC=auto-copy`.
8. Micropausas (~40 ms) medidas só com a renderização forçada à velocidade máxima, em cada imagem de um GIF do
   cabeçalho e na ligação do logótipo; causa não encontrada, invisíveis em utilização normal.

## 10. Preferências de trabalho de quem mantém o projeto

- Respostas em francês; pouca experiência com Linux / SSH: explicar os comandos.
- Não enviar nada para o GitHub nem publicar versões sem aprovação explícita; as versões são publicadas por quem
  mantém o projeto.
- Testes com uma cópia da configuração (`TURTLEFIN_CONFIG_DIR`), nunca a verdadeira; contas de teste `test` /
  `test2`.
- Depois de cada conjunto de alterações: um instalador de teste no Ambiente de Trabalho de quem mantém o projeto
  (`Turtlefin-test-<commit>-windows-x64-setup.exe`, apagando o anterior).
