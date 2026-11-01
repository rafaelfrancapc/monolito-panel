# Monolito Panel

Painel de shell **Rust/GTK4 para XFCE/X11**, independente do `xfce4-panel`.
Primeira entrega incremental: barra inferior, Iniciar à esquerda, aplicativos e
relógio à direita, com popups GTK nativos. Não é uma interface web.

## Compilar no Ubuntu Server ou WSL

Referência: **Ubuntu 24.04 LTS**, GTK ≥ 4.12 e Rust estável via rustup.
Ubuntu 22.04 fornece GTK antigo demais; use uma VM/distribuição WSL 24.04.

```bash
./scripts/setup-ubuntu.sh
cargo fmt --all --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo run --locked -- --dev
```

O script usa `sudo apt-get` apenas para dependências de compilação e fontes.
Não instala um desktop completo, não altera sessão, autostart ou painel XFCE.
A instalação de Rust usa o instalador oficial por HTTPS, sem desativar verificações.
`Cargo.lock` fixa as dependências resolvidas. O primeiro build precisa de Internet.

**Compilar não exige uma sessão gráfica.** Executar o shell exige um servidor X11
com um gerenciador de janelas EWMH, como xfwm4. Em Ubuntu Server, use uma sessão
XFCE/X11 em VM para o teste interativo, ou o teste isolado abaixo. Uma janela
GTK aberta pelo WSLg não comprova suporte de shell: o compositor do Windows não
fornece os contratos de posicionamento/reserva EWMH do XFCE. No WSL, compile e
rode o teste Xvfb; valide a integração final numa sessão XFCE/X11 Linux.
Não use `xhost +`, não exponha X11 na rede e não altere `DISPLAY` para um endereço
arbitrário como solução para permissões.

### Teste X11 isolado, inclusive no WSL sem desktop

```bash
sudo apt-get install --no-install-recommends xvfb xauth xfwm4 dbus-x11 \
  x11-utils x11-xserver-utils xdotool python3
./scripts/smoke-x11.sh
```

O teste inicia um Xvfb e um xfwm4 próprios, verifica propriedades EWMH,
geometria, área de trabalho reservada, popups, Escape, busca/lançamento XDG,
minimização/restauração e redimensionamento RandR, depois
reinicia em `--dev`. Não encerra nem substitui o painel da sua sessão.
Os testes unitários não precisam de display.

## Executar na sessão XFCE/X11

```bash
cargo run --locked -- --dev          # coexiste sem reservar espaço
cargo run --locked                    # reserva 48 px na borda inferior
cargo run --locked -- --theme=light
cargo run --locked -- --theme=dark
```

Feche o processo no terminal com Ctrl+C. O X11 remove a reserva quando a janela
é destruída. O painel original permanece instalado e em execução. Se ambos
ocuparem a borda inferior, haverá sobreposição; prefira `--dev` ou uma sessão
isolada para desenvolver. Não há autostart instalado nem substituição automática.
Se você tiver encerrado manualmente o painel original em um teste, restaure com
`xfce4-panel` no terminal da mesma sessão. A aplicação não executa essa ação.

## Estado desta entrega

| Área | Implementado | Limitações / próximo marco |
|---|---|---|
| Shell (A) | GTK4/X11, dock sem decoração, 48 px, struts, relógio, claro/escuro | Seleção de um monitor; barras em vários monitores e auto-ocultação pendentes |
| Iniciar (B parcial) | Popover à esquerda, busca de nomes localizados XDG, launch por GIO, favoritos fixar/desafixar persistidos | Reordenação por UI e ações de sessão pendentes; ordem configurável no TOML |
| Taskbar (C experimental) | Eventos EWMH, janela ativa, workspaces, ativação/minimização, agrupamento por metadados e lista de janelas | Sem miniaturas, urgência, fechamento/context menu completo ou ícones `_NET_WM_ICON`; apps não identificados usam ícone genérico |
| Quick Settings (D parcial) | Popup à direita, leitura NetworkManager e toggle Wi-Fi, volume/mudo reais por `wpctl`, bateria UPower se presente | Atualiza ao abrir ou após ação local; não acompanha mudanças externas continuamente; sem SSID/lista de redes, seleção de saída, Bluetooth, brilho ou perfis de energia |
| Calendário (E parcial) | Calendário GTK e aviso explícito sobre histórico | Bandeja SNI, histórico e ações de notificações pendentes; não toma o nome `org.freedesktop.Notifications` |

Os controles ficam indisponíveis quando os serviços necessários não existem.
Não há valores fictícios. Wi-Fi depende do NetworkManager e da política de
permissões da sessão; falhas são apresentadas ao usuário. Áudio depende de
PipeWire/WirePlumber e `wpctl` no PATH. Estes serviços não são necessários para
compilar e não são instalados/ativados pelo script. Em servidor, contêiner ou WSL
é normal não ter acesso a esses serviços/hardware.

## Configuração

Arquivos opcionais sob `$XDG_CONFIG_HOME/monolito/` ou `~/.config/monolito/`:

`panel.toml` (gravado atomicamente ao fixar/desafixar):

```toml
version = 1
height = 48
# monitor = 0      # índice RandR; omitido = primário ou primeiro disponível
pinned = ["org.xfce.Thunar.desktop", "firefox.desktop"]
```

`appearance.toml` (contrato proposto compartilhado com Settings):

```toml
version = 1
theme = "dark"    # light ou dark
accent = "#49b9ed"
animations = true
```

As preferências são carregadas ao iniciar. Não há monitoramento de arquivos
nem promessa de compatibilidade com o Settings existente, que não foi fornecido.
`monolito-ui` reúne tokens/CSS e o modelo de aparência, sem depender do processo
Settings. O acesso ao Settings usa o desktop ID `org.monolito.Settings.desktop`;
se não estiver instalado, aparece uma mensagem, sem fallback arbitrário.

Os nomes de apps vêm de GIO/DesktopAppInfo: diretórios XDG, localização,
`Hidden`/`NoDisplay`/restrições de desktop e lançamento de `.desktop` ficam a
cargo da implementação GIO. A associação com janelas usa `StartupWMClass` ou
ID `.desktop` compatível com `WM_CLASS`, nunca o título da janela. A heurística
é conservadora e alguns aplicativos precisarão de identificação adicional.

## Arquitetura

- `src/platform`: interface `DesktopShellBackend`, comandos e eventos. `x11rb`
  usa uma conexão separada numa thread para EWMH/RandR, sem bloquear a thread GTK.
- `gdk4-x11`: identifica o XID real de `GdkX11Surface`; a dependência fica isolada
  no ponto de anexação da janela. Não há ponte GTK3 nem plugin de painel.
- `src/app.rs`: widgets da barra e atualização dos estados publicados pelo backend.
- `src/popups`: Start, configurações rápidas e calendário, todos GTK4 nativos.
- `src/services`: aplicativos XDG, D-Bus via GIO e subprocessos assíncronos.
  GIO já fornece o transporte integrado com o main loop GLib; adicionar zbus
  neste marco duplicaria esse mecanismo sem necessidade.
- `crates/monolito-ui`: contrato inicial de aparência e CSS reutilizável.

O worker X11 reage a eventos do servidor. O timer GTK só drena a fila local;
não consulta o servidor repetidamente. O relógio atualiza uma vez por segundo.
As consultas D-Bus e `wpctl` são assíncronas, com escrita de volume agrupada
por debounce. Os resultados de hardware ainda não têm assinaturas contínuas.

O GTK realiza a janela, o worker define tipo/struts antes do mapeamento, e só
então a interface a apresenta. Ao mapear ou mudar a geometria RandR, o backend
reaplica posição e reserva. Uma reserva inferior só é publicada quando o monitor
toca a borda inferior da tela virtual: em monitores interiores a barra funciona
sem strut, evitando uma reserva global errada. O fallback sem RandR usa a tela
inteira. Wayland exige outro backend/protocolos; não é suportado nesta entrega.

## Localização e visual

Texto fonte em inglês, gettext com catálogo inicial `po/pt_BR.po`.
`./scripts/translations.sh` gera catálogos locais (requer `gettext`).
Execute com `LANGUAGE=pt_BR cargo run -- --dev` numa locale UTF-8.
`MONOLITO_LOCALE_DIR` permite apontar para catálogos instalados.
O estado de Wi-Fi usa por enquanto rótulos técnicos `on/off`.

CSS próprio, Inter e Montserrat Alternates com fallback. O tema base Adwaita
é selecionado apenas neste processo para evitar herança acidental de temas
antigos. Ícones simbólicos livres do tema instalado; nenhum recurso Microsoft.
Controles GTK fornecem foco, tooltips e navegação por teclado; popups fecham por
Escape. Auditoria de acessibilidade, composição/transparência, pequenos monitores,
escalas HiDPI e múltiplos monitores reais ainda precisam de validação.

No ambiente gerenciado desta tarefa, o helper `scripts/cloud-ubuntu.sh` usa
um contêiner Ubuntu 24.04 (`setup`, `check`, `smoke`), preservando o checkout em
`/work`. O host não possui GTK4 nem permissões para instalação global. O helper
é específico desse ambiente; na VM ou no WSL use `setup-ubuntu.sh` diretamente.

Veja [VALIDATION.md](docs/VALIDATION.md) para os resultados efetivamente obtidos.
