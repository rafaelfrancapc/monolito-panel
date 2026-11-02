# Validação executada

Ambiente: Ubuntu 24.04 em contêiner, GTK 4.14.5, Rust estável, Xvfb e xfwm4.
O alvo de produção é Linux Mint 22.3 em sessão XFCE ou Cinnamon X11.

- `cargo fmt --all --check`, `cargo check --workspace --locked` e Clippy com
  `-D warnings`: passaram.
- `cargo test --workspace --locked`: 23 testes passaram. O teste de recursos GTK
  foi executado separadamente com `xvfb-run` e passou; verifica os nomes de
  bateria e recursos de ícones para todos os estados.
- `scripts/smoke-x11.sh`: passou. Verifica tipo dock, área reservada, popups,
  Escape, tecla Windows, atualização de apps instalados, watcher SNI, busca/lançamento
  XDG, minimização/restauração e mudança RandR.
- `scripts/visual-check.py`: capturas reais de Start e configurações rápidas nos
  temas claro e escuro, com geometria dentro da tela em escalas 100% e 200%.
- Os seis catálogos gettext (`de`, `es`, `fr`, `it`, `pt` e `pt_BR`) passaram
  em `msgfmt --check` e são incluídos no pacote.
- `scripts/test-session.sh`: passou com substituição e restauração simuladas dos
  painéis nativos XFCE e Cinnamon, sem alterar a sessão do host.
- `scripts/build-deb.sh`: gera `dist/monolito-panel_0.1.3_amd64.deb` com binário,
  tradução, serviço do usuário e autostart XDG. O conteúdo e o controle foram
  inspecionados com `dpkg-deb`.

O Xvfb não fornece hardware de bateria, Wi-Fi, Bluetooth ou brilho, nem reproduz
uma sessão Mint completa. A reação dos serviços reais, o Win11 Icon Theme
instalado e a substituição do painel no desktop Mint ainda exigem validação nessa
sessão. O tema de ícones ausente foi testado como fallback; a GUI não deve mostrar
símbolos quebrados por essa ausência.
O teste da bandeja confirma o watcher DBus, mas ainda não exercita um ícone SNI
de aplicativo real nem ícones XEmbed. O fallback de brilho por XRandR passou na
seleção de saída e na alteração de gama em Xvfb; precisa de validação visual em
monitor físico.
O clima foi validado com uma resposta determinística nas capturas; a consulta
externa e a localização aproximada por IP dependem do provedor wttr.in.
