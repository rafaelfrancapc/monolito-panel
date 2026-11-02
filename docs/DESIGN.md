# Integração visual da sessão

O painel usa tokens semânticos do crate `monolito-ui`: `canvas`, `primary`,
`secondary`, `subtle`, `text`, `muted`, `border`, `accent` e `on_accent`.
As cores de texto e dos controles escuros e claros têm verificações de contraste.
Barra de 48 px, botões de 40 px, ícones de tarefas de 24 px e bandeja de 16 px
mantêm seus centros alinhados; o indicador de execução é uma camada independente.
Iniciar e configurações rápidas compartilham padding de 24 px, cantos de 8 px,
controles de 4 px e espaçamento múltiplo de 4 px. Em HiDPI, o GTK escala essas
medidas lógicas e o backend X11 reserva a altura física correspondente.

O [Win11 Icon Theme](https://github.com/yeyushengfan258/win11-icon-theme)
instalado com nome `Win11` fornece ícones de aplicativos e símbolos do sistema.
Os símbolos exclusivos da barra são recursos GResource próprios. O painel não
copia os arquivos de outro tema. O nome do tema pode ser alterado em
`appearance.toml` para uma instalação personalizada.

O projeto Settings já usa nomes semânticos de tokens GTK4; o painel procura seu
arquivo `org.monolito.Settings.desktop` primeiro. Uma integração posterior pode
compartilhar valores de preferência e exportação de paleta sem atrelar o processo
do painel ao processo Settings. O Greeter é GTK3 sob LightDM e roda antes do login;
um adaptador de CSS/ícones e configuração do LightDM deve receber os mesmos
valores visuais sem depender do diretório pessoal do usuário autenticado. Nenhum
controle de sessão ou autenticação é transferido ao painel.
