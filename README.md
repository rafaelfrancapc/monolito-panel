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
sudo apt install ./dist/monolito-panel_0.1.3_$(dpkg --print-architecture).deb
/usr/libexec/monolito-panel/session start
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
start_pinned = ["org.gnome.Calculator.desktop"]
weather_enabled = true
# vazio = localização aproximada pelo IP; também aceita cidade, estado ou país
weather_location = "São Paulo, SP"
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

`pinned` controla apenas a barra; `start_pinned` controla apenas o Iniciar. Fixar
um aplicativo em um local remove sua fixação no outro. O clique direito no
Iniciar ou no ícone da barra oferece fixação e atalho na área de trabalho.
A lista de apps instalados atualiza
automaticamente pelo monitor de aplicativos do GIO. A tecla Windows abre e fecha
o Iniciar numa sessão X11 com XInput2. O nome de perfil usa o nome real do sistema,
e a foto vem de `~/.face`, `~/.face.icon` ou AccountsService. Clicar no perfil
oferece logout pela sessão XFCE ou Cinnamon.

O clima aparece no canto esquerdo e atualiza ao iniciar e a cada 30 minutos.
Sem `weather_location`, o serviço wttr.in estima a localização pelo endereço IP;
defina uma cidade para evitar essa estimativa. O clique direito oferece **Ocultar
clima** e grava `weather_enabled = false`. Para reativar, altere o valor para
`true` no `panel.toml` e reinicie o serviço. Se a rede ou o provedor estiverem
indisponíveis, o painel mantém o último resultado e informa o erro no tooltip.

A lupa ao lado do Iniciar abre o App Finder no XFCE. No Cinnamon, abre diretamente
a pesquisa do Iniciar do painel, que substitui o menu do painel nativo.

## Estado e validação

Busca, lançamento, fixação, janela ativa, agrupamento, rede, Bluetooth, áudio,
brilho, bateria, perfis de energia e calendário estão implementados. Os controles
dependem dos serviços reais da sessão; sem eles aparecem indisponíveis.
A bandeja StatusNotifier hospeda os ícones de serviços que usam esse protocolo,
com ativação e menu de contexto. Ícones XEmbed antigos ainda precisam de uma ponte
separada. O brilho tenta `brightnessctl`, `xbacklight`, sysfs e, por fim, escurece
por software com XRandR. Histórico de notificações ainda não está disponível.
A execução em Wayland requer outro backend.

Os testes automatizados cobrem lógica e uma sessão privada Xvfb/xfwm4. Veja
[VALIDATION.md](docs/VALIDATION.md) para resultados e limites da validação.

## Idiomas

O texto fonte é inglês. O pacote inclui catálogos gettext para português do Brasil
(`pt_BR`), português (`pt`), espanhol, francês, alemão e italiano. O idioma segue
a locale da sessão. `LANGUAGE=pt_BR monolito-panel --dev` permite testar um
catálogo específico.
