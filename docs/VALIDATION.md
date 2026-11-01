# Validação executada

Ambiente: **Ubuntu 24.04 LTS em contêiner**, GTK **4.14.5**, Rust estável
**1.99.1**, Xvfb e **xfwm4 4.18.0**, renderizador Cairo e compositor desligado.
O host de onboarding é Debian; a compilação foi feita no Ubuntu para
corresponder à VM/WSL indicada pelo usuário.

## Resultados

- `scripts/cloud-ubuntu.sh setup`: passou, incluindo repetição com pacotes
  instalados, build com lockfile e compilação/verificação do catálogo gettext.
- `cargo fmt --all --check`: passou.
- `cargo check --workspace --locked`: passou.
- `cargo test --workspace --locked`: **7 testes passaram**, sem falhas ou skips.
  Há 6 testes no painel e 1 em `monolito-ui`; não existem doctests executáveis.
- `cargo build --locked`: passou.
- `scripts/cloud-ubuntu.sh smoke` / `scripts/smoke-x11.sh`: passou numa sessão
  X11/D-Bus privada. Confirma:
  - tipo dock e `SKIP_TASKBAR` no XID real do GTK;
  - geometria `1280×48` em `(0,752)` e strut com intervalo inclusivo `0..1279`;
  - workarea do xfwm4 reduzida para `1280×752`;
  - popups GTK acima da barra, Iniciar à esquerda e configurações à direita;
  - Escape fecha o popup mesmo quando o campo de pesquisa possui foco;
  - pesquisa e lançamento por Enter de um `.desktop` real de teste;
  - cliente X11 ativado, minimizado e restaurado pela taskbar;
  - redimensionamento do framebuffer para `1024×768` e retorno para `1280×800`,
    com atualização da posição, largura e strut;
  - segundo início em `--dev` com todos os valores de strut zerados.

Capturas reais do menu Iniciar e configurações rápidas foram inspecionadas na
sessão Xvfb. Isso revelou e permitiu corrigir a dependência do carregador SVG e
os artefatos de sombra sem compositor. Os testes também detectaram perda de
cliques causada por recriar os botões ao mudar o foco; a taskbar agora atualiza
os widgets existentes enquanto a estrutura de grupos permanece igual.

O teste Xvfb emite avisos de DRI3/EGL indisponível, limite de descritores D-Bus e
ausência de session manager. As verificações terminaram com status zero usando
Cairo. Não é um teste de aceleração gráfica nem de uma sessão XFCE completa.

## Limites da evidência

Não há acesso ao Monolito Settings, ao compositor WSLg ou a hardware/serviços de
rede, áudio e bateria nesta tarefa. No contêiner, os estados indisponíveis foram
verificados; não foi validada uma alteração real de Wi-Fi, volume ou bateria.
Os backends desses controles continuam experimentais. Não há testes de SNI,
Bluetooth, brilho ou histórico: esses recursos estão explicitamente pendentes.

O resize virtual testa eventos RandR e o fallback quando não há saída ativa;
não comprova hotplug ou posicionamento em vários monitores físicos. Os testes
unitários cobrem o cálculo de struts inclusive em monitores interiores.

Checklist complementar na VM XFCE/X11:

- várias aplicações e grupos, ícones, alternância de workspace;
- resolução com saída ativa, hotplug e monitores de alturas/posições diferentes;
- temas, composição, tradução, fontes, pequenos monitores e escala HiDPI;
- NetworkManager/Polkit, PipeWire/WirePlumber e UPower reais;
- coexistência com xfce4-panel e integração futura com Monolito Settings;
- auditoria de acessibilidade e leitores de tela.

A configuração reutilizável é salva como rascunho nas configurações do ambiente.
Salvar o rascunho não publica o snapshot nem valida a restauração em outra tarefa.
