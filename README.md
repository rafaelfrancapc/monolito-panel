# Desktop Panel

Painel de tarefas em Rust e GTK4 para sessões **XFCE ou Cinnamon em X11**. Reúne
Iniciar com busca de aplicativos, tarefas e janelas abertas, configurações rápidas,
relógio e calendário. O visual segue as referências do Windows 11 e os tokens de
contraste do Fluent Design; a interface usa widgets GTK nativos.

## Plataformas

O alvo é **Linux Mint 22.3**, baseado no Ubuntu 24.04. A compilação funciona no
Ubuntu Server 24.04 e no Ubuntu 24.04 sob WSL. A execução como painel exige uma
sessão gráfica X11 com um gerenciador de janelas EWMH. O WSL é adequado para
compilar e testar com Xvfb; valide a sessão final numa VM ou máquina Mint.

O [Win11 Icon Theme](https://github.com/yeyushengfan258/win11-icon-theme) deve
estar instalado com o nome padrão `Win11`. O painel seleciona esse tema de ícones
por padrão. Os ícones próprios da barra estão embutidos no binário; aplicativos
sem ícone no tema usam o ícone publicado pela janela X11 ou um ícone genérico.
O [Win11 GTK Theme](https://github.com/yeyushengfan258/Win11-gtk-theme) pode ser
usado para outros programas da sessão; o painel aplica seu próprio CSS para
manter dimensões e contraste estáveis. Nenhuma imagem da Microsoft ou desses
temas é distribuída neste repositório.

## Compilar no Ubuntu 24.04 ou Mint 22.3

```bash
./scripts/setup-ubuntu.sh
cargo fmt --all --check
cargo test --workspace --locked
cargo run --locked -- --dev
```

`setup-ubuntu.sh` instala dependências de compilação e Rust estável. Não altera
a sessão gráfica. Para executar o teste X11 isolado:

```bash
sudo apt-get install --no-install-recommends xvfb xauth xfwm4 dbus-x11 \
  x11-utils x11-xserver-utils xdotool python3
./scripts/smoke-x11.sh
```

## Criar e instalar o pacote `.deb`

```bash
./scripts/build-deb.sh
sudo apt install ./dist/monolito-panel_0.1.0_$(dpkg --print-architecture).deb
```

O script compila com `--release --locked` e inclui binário, traduções, unidade
`systemd --user` e autostart XDG. O pacote instala tudo sob `/usr` e
`/etc/xdg/autostart`; não inicia processos gráficos durante a instalação.
Confirme a presença do Win11 Icon Theme na conta que utilizará o painel.

Ao entrar numa sessão XFCE ou Cinnamon **X11**, o autostart importa as variáveis
da sessão para o gerenciador de serviços do usuário e inicia
`monolito-panel.service`. O serviço reserva a faixa inferior da tela. No XFCE,
ele encerra o processo `xfce4-panel` quando está ativo; no Cinnamon, desativa
temporariamente os painéis com `org.cinnamon panels-enabled=[]`. Ao parar o
serviço, restaura o painel nativo que estava ativo. A configuração de painéis do
Cinnamon é lida da sessão atual e restaurada ao sair; não há desinstalação do
desktop. O gerenciador de arquivos e a área de trabalho continuam sob controle
do ambiente original.

Para testar sem substituir o painel nativo, use `cargo run -- --dev` numa sessão
X11. Para parar a versão instalada e restaurar o painel nativo na sessão atual:

```bash
systemctl --user stop monolito-panel.service
```

Para desativar o início automático nas próximas sessões, crie uma sobreposição
XDG do usuário e pare o serviço atual:

```bash
mkdir -p ~/.config/autostart
printf '[Desktop Entry]\nType=Application\nHidden=true\n' > ~/.config/autostart/monolito-panel.desktop
systemctl --user stop monolito-panel.service
```

Para reativar, remova essa sobreposição e entre novamente na sessão. A remoção
do pacote com `sudo apt remove monolito-panel` retira a unidade e o autostart;
pare primeiro o serviço do usuário se ainda estiver executando.

## Configuração

Arquivos opcionais em `$XDG_CONFIG_HOME/monolito/` ou `~/.config/monolito/`:

```toml
# panel.toml
version = 1
height = 48
alignment = "center" # ou "left"
pinned = ["org.xfce.Thunar.desktop", "firefox.desktop"]
```

```toml
# appearance.toml
version = 1
theme = "dark"       # ou "light"
accent = "#0078d4"
animations = true
icon_theme = "Win11"
```

O painel carrega essas opções ao iniciar. O botão de configurações prioriza
`org.monolito.Settings.desktop` quando instalado e usa os gerenciadores XFCE ou
Cinnamon como alternativas. Os tokens de aparência estão em `monolito-ui`;
Settings e [GTK Greeter](https://github.com/rafaelfrancapc/monolito-gtk-greeter)
continuam processos separados. O Greeter usa GTK3/LightDM e exige configuração
de tema na tela de login pelo administrador; o painel não altera LightDM.

## Estado e validação

Busca, lançamento, fixação, janela ativa, agrupamento, rede, Bluetooth, áudio,
brilho, bateria, perfis de energia e calendário estão implementados. Os controles
dependem dos serviços reais da sessão; sem eles aparecem indisponíveis.
Histórico de notificações e bandeja SNI ainda não estão disponíveis. A execução
em Wayland requer outro backend.

Os testes automatizados cobrem lógica e uma sessão privada Xvfb/xfwm4. Veja
[VALIDATION.md](docs/VALIDATION.md) para resultados e limites da validação.
