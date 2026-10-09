<p align="center"><img src="packaging/icons/turtlefin-256.png" width="128" alt="Logótipo do Turtlefin"></p>

# Turtlefin

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · **Português**

**Um cliente Jellyfin nativo, leve e animado, pensado tanto para a sala como para a secretária.** Escrito em Rust
com Slint (interface) e libmpv (reprodução): sem Qt e sem navegador incorporado. Funciona em qualquer computador
Windows ou Linux, do portátil antigo à pequena caixa ligada à televisão, e usa-se tão bem com um comando como com
teclado e rato.

Versão atual: **1.0.0** · Idiomas da interface: Português, English, Français, Español, Deutsch, Italiano, Polski,
Nederlands — e qualquer idioma que acrescentes tu mesmo (ver [Traduzir o Turtlefin](#traduzir-o-turtlefin)).

## O que faz

- **Contas**: ecrã «Quem está a ver?» com fotos de perfil (GIF animados incluídos, em cache), até 12 contas
  guardadas no dispositivo (só o token, nunca a palavra-passe), mudança de conta sem voltar a escrever nada,
  «Gerir contas» para remover algumas. Procura de servidores em todas as redes do dispositivo; um endereço
  principal e um endereço alternativo, experimentado quando o principal não responde.
- **Arranque**: um logótipo animado cujos sete pontos são verificações reais (idioma, ecrã, leitor de vídeo,
  armazenamento, configuração, rede e, ao centro, o servidor), depois «Quem está a ver?» — ou diretamente a conta
  escolhida em Definições → Conta → **Abrir esta conta ao iniciar**.
- **Página inicial**: Os meus conteúdos, Continuar a ver, A seguir, Adicionado recentemente; separadores
  Favoritos e Pedidos (Seerr). Cartazes com episódios por ver, visto e classificação; fundo retirado do conteúdo
  selecionado.
- **Fichas**: filme, série, temporada, episódio; reproduzir, favorito, visto, transferência, escolha de áudio /
  legendas guardada para a série inteira; «Mais deste género» e sugestões do Seerr; pedido das temporadas em
  falta.
- **Reprodução** (libmpv): capítulos, episódios da temporada, «Saltar intro», episódio seguinte, sugestões no fim
  de uma série, volume próprio do Turtlefin; posição e «visto» enviados ao servidor.
- **Watch party** (SyncPlay): ver a mesma coisa ao mesmo tempo em vários dispositivos.
- **Offline**: as transferências substituem a página inicial, com fichas completas sem servidor; o que foi visto
  ou marcado como favorito offline é enviado para a conta ao voltar a ligar.
- **Pesquisa** (biblioteca + Seerr) e conteúdo ao acaso.
- **Comando, teclado e rato em todo o lado**: interface TV (elementos grandes, teclado no ecrã) ou interface de
  computador (janela ou ecrã inteiro, F11), clique e roda em todas as páginas, e um teclado físico escreve
  diretamente nos campos de texto, nos dois modos.
- **Definições**: foto de perfil (avatares GetAvatar por categoria), idioma da interface, idiomas de áudio e
  legendas, tamanho das legendas, episódio seguinte e intro automáticos, interface TV, ecrã inteiro, fundo,
  classificações, hora, endereços do servidor, cache de imagens, **atualização a partir do GitHub**.
- **Animações** em todo o lado (cartaz que voa até à ficha, menu que desliza, filas em cascata, início de sessão
  animado, mudança de idioma), ajustáveis uma a uma em Definições → Animações, com presets (Todas, Leves, Nenhum)
  e os teus, que se podem exportar e importar.
- **Visita guiada**: proposta no primeiro arranque, para rever em Definições → Acerca.

## Instalar

Nada para compilar: transfere-se, instala-se e pronto. Todos os ficheiros estão na página
[Releases](https://github.com/Xelopteryx/Turtlefin/releases).

### Com um só comando

**Windows** (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.ps1 | iex
```

**Linux** (terminal):

```sh
curl -fsSL https://raw.githubusercontent.com/Xelopteryx/Turtlefin/main/packaging/install.sh | sh
```

No Windows, o instalador mais recente é transferido e executado para a tua conta de utilizador (sem direitos de
administrador). No Debian, Ubuntu, Linux Mint e outras distribuições com apt é instalado o pacote `.deb` (é pedida
a tua palavra-passe); nas restantes, a AppImage vai para `~/.local/bin` com uma entrada no menu de aplicações.

### À mão

| Sistema | Ficheiro | Notas |
|---|---|---|
| Windows 64 bits | `Turtlefin-<version>-windows-x64-setup.exe` | Instalador: idioma, pasta e modo **instalado** (menu Iniciar, desinstalação) ou **portátil** |
| Windows 64 bits, sem instalar | `Turtlefin-<version>-windows-x64-portable.zip` | Descompactar onde se quiser (pen USB…) e abrir `turtlefin.exe` |
| Windows 32 bits | `…-windows-x86-setup.exe` / `…-windows-x86-portable.zip` | Para PC antigos |
| Linux, todas as distribuições (x86_64) | `Turtlefin-<version>-linux-x86_64.AppImage` | Torná-la executável (`chmod +x`) e abri-la |
| Linux, ARM 64 bits | `Turtlefin-<version>-linux-aarch64.AppImage` | Igual |
| Debian, Ubuntu, Mint… | `turtlefin_<version>_amd64.deb` / `_arm64.deb` | Duplo clique, ou `sudo apt install ./turtlefin_….deb` |

As versões Windows e a AppImage contêm tudo (leitor libmpv incluído). O pacote `.deb` usa a libmpv do sistema
(`libmpv2`, instalada automaticamente pelo apt). As AppImage e os pacotes exigem uma distribuição de 2022 ou mais
recente (Ubuntu 22.04, Debian 12…).

**Versão portátil**: um ficheiro `portable` ao lado de `turtlefin.exe` faz com que a configuração, as contas, a
cache e as transferências fiquem na pasta `data` ao lado do programa; nada é escrito em mais lado nenhum.

### Atualizar

**Definições → Acerca → Procurar atualizações** compara a versão instalada com a última publicada; depois
**Atualizar** trata de tudo conforme a forma como o Turtlefin está instalado:

| Instalação | Atualização |
|---|---|
| Windows, instalado | o novo instalador é transferido e executado em silêncio na mesma pasta |
| Windows, portátil | o novo arquivo é transferido e os seus ficheiros substituem os antigos |
| AppImage | o novo ficheiro substitui o antigo |
| Pacote .deb | o pacote é instalado com `pkexec` (é pedida a palavra-passe de administrador) |

**Reiniciar o Turtlefin** abre depois a nova versão.

## Plugins de servidor recomendados

O Turtlefin funciona com um servidor Jellyfin simples (10.11 ou mais recente). Estes plugins, a instalar no
**servidor**, acrescentam funções:

| Plugin | O que traz ao Turtlefin |
|---|---|
| [Jellyfin Enhanced](https://github.com/n00bcodr/Jellyfin-Enhanced) | Separador Pedidos, resultados do Seerr na pesquisa, sugestões e pedidos do Seerr nas fichas — com o início de sessão do Jellyfin, sem chave do Seerr no cliente |
| [Seerr](https://github.com/seerr-team/seerr) (antes Jellyseerr) | O próprio gestor de pedidos, usado pelo Jellyfin Enhanced |
| [Intro Skipper](https://github.com/intro-skipper/intro-skipper) | Deteta intros e genéricos: botão «Saltar intro», salto automático, «Episódio seguinte» no momento certo |
| [GetAvatar](https://github.com/cedev-1/jellyfin-plugin-GetAvatar) | Uma galeria de fotos de perfil para escolher em Definições → Conta |

## Abrir

```
turtlefin                                  animação de arranque, depois «Quem está a ver?» (ou a conta de arranque)
turtlefin "Nome"                           conta guardada «Nome»
turtlefin "Nome" --server=http://…         início de sessão direto (palavra-passe: variável TURTLEFIN_PASSWORD=…)
turtlefin --tv                             interface TV: ecrã inteiro, elementos grandes
turtlefin --desktop                        interface de computador (tem prioridade sobre a definição «Interface TV»)
turtlefin --no-intro                       sem animação de arranque
turtlefin --console                        janela de registo (Windows)
turtlefin --tutorial                       visita guiada ao chegar à página inicial
```

A linha de comandos tem sempre prioridade sobre as definições (conta de arranque, interface TV).

## Teclas e rato

- **Setas** para mover, **Enter** para abrir / ativar, **Esc** ou **Retrocesso** para voltar (mantida premida:
  só um passo atrás).
- Página inicial: **←** no primeiro cartão (ou Voltar) abre o menu; no menu, **→** ou Esc fecha-o.
- **↑** a partir do topo de uma página: barra superior (voltar, início, menu, watch party, ao acaso, pesquisa,
  conta).
- Reprodução, comandos escondidos: **← →** recuar / avançar 10 s, **↑ ↓** ou Enter mostram os comandos, ↓ a
  partir dos botões: episódios da temporada. **Espaço**: pausa · `a` áudio · `s` legendas · `f` ecrã inteiro.
- **F11**, em todo o lado: ecrã inteiro (também em Definições → Ecrã → **Ecrã inteiro**, guardado entre
  arranques fora da interface TV).
- **Rato**: clique para abrir, roda para passar de uma fila para outra (Shift + roda: dentro da fila).
- **Texto**: uma letra escrita no teclado vai diretamente para o campo (pesquisa, início de sessão, endereço),
  mesmo na interface TV; aí, um clique no campo de pesquisa abre o teclado no ecrã.

## Ficheiros

| Onde | O quê |
|---|---|
| pasta de configuração, `turtlefin/` | `session.json` (sessão atual), `accounts.json` (contas guardadas), `prefs.json` (definições do dispositivo), `tracks.json` (faixas por série), `userdata.json` (vistos / favoritos offline) |
| pasta de dados, `turtlefin/downloads/` | transferências (conteúdo, cartazes, fundo, logótipo, `info.json`), `queue.json` (fila pendente) |
| pasta de cache, `turtlefin/img/` | imagens e fotos de perfil (pode ser limpa em Acerca) |
| **Documentos** | `Turtlefin Languages` (idiomas acrescentados) e `Turtlefin Presets` (presets de animações exportados) |

No Linux: `~/.config/turtlefin`, `~/.local/share/turtlefin`, `~/.cache/turtlefin`.
No Windows: `%APPDATA%\turtlefin\config`, `%APPDATA%\turtlefin\data`, `%LOCALAPPDATA%\turtlefin\cache`.
Versão portátil: tudo em `data\` ao lado de `turtlefin.exe` (`config`, `cache`, `downloads`), e as pastas de
idiomas e de presets ao lado do programa.

## Em caso de problema

- `turtlefin --console` (Windows) ou abrir a partir de um terminal (Linux) mostra o registo.
- `TURTLEFIN_MPV_LOG=/tmp/mpv.log`: registo do mpv · `TURTLEFIN_MPV_ARGS="…"`: opções do mpv adicionais.
- `TURTLEFIN_HWDEC=auto-copy`: descodificação por hardware (por software, por omissão, no Linux ARM) · `TURTLEFIN_AO=alsa`: saída de som.
- `TURTLEFIN_LIBMPV=caminho`: outra localização da libmpv · `TURTLEFIN_DEBUG_FRAMES=1`: assinala as imagens lentas.
- A renderização tem de ser OpenGL (escolhida automaticamente): com `SLINT_BACKEND=winit-software` não há vídeo.

## Temas

Definições → Ecrã → **Tema**: Turtlefin (por omissão), Escuro, Claro, Frutiger Aero (céu, relva, bolhas, botões de
gel brilhantes), Turtlefin verde — ou um tema teu.

- **Criar um tema** abre o *Turtlefin Theme Creator* no navegador: cada cor, o arredondamento, o brilho, o céu e as
  bolhas, com pré-visualização em direto; guarda um ficheiro `.tftheme`.
- Pôr esse ficheiro na pasta **Turtlefin Themes** (Documentos, ou ao lado de `turtlefin.exe` na versão portátil) e
  depois **Importar um tema**. **Exportar o tema** escreve o tema atual nessa pasta, como ponto de partida.
- O criador também está no repositório, `tools/theme-creator.html`: um só ficheiro, que funciona offline.

## Traduzir o Turtlefin

Sem programar nem compilar. Os idiomas acrescentados ficam numa única pasta, **Turtlefin Languages**: em
**Documentos** (versão instalada) ou ao lado de `turtlefin.exe` (versão portátil).

1. **Definições → Ecrã → Adicionar um idioma** abre um explorador dessa pasta; **Criar o modelo** escreve lá
   `modele.po`, com os textos em inglês e, como nota, o francês original e o idioma atual.
2. Fazer uma cópia chamada `<código>.po` (`sv.po` para sueco, `ja.po` para japonês…), nessa pasta ou numa
   subpasta, e preencher cada `msgstr ""` com a tradução do `msgid` inglês que está por cima. Manter os `{}` e
   `{n}`. Preencher também `X-Language-Name` (nome mostrado) e, se for preciso, `Plural-Forms` (regra gettext do
   idioma). Serve qualquer editor de `.po`, por exemplo o [Poedit](https://poedit.net).
3. De novo **Adicionar um idioma**: a tradução aparece com a parte já traduzida; escolhê-la aplica-a. Os textos
   deixados vazios aparecem em inglês.

Para a partilhar com toda a gente: um pull request que acrescente o ficheiro a `lang/` (e a `BUILTIN` em
`src/i18n.rs`); `python tools/lang-check.py` verifica que não falta nada.

## Desenvolvimento

Ver [HANDOFF.pt.md](HANDOFF.pt.md) (também em [Français](HANDOFF.md), [English](HANDOFF.en.md),
[Deutsch](HANDOFF.de.md), [Español](HANDOFF.es.md), [Italiano](HANDOFF.it.md), [Nederlands](HANDOFF.nl.md),
[Polski](HANDOFF.pl.md)): estado do projeto, decisões, compilação, pacotes, publicação, traduções, problemas
conhecidos.
