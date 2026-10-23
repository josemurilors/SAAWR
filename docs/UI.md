# Plano de redesign da interface (Fase 2b)

Estado: **proposta, aguardando aprovação**. A interface atual funciona (importar, fila em
lote, tráfego por grupo, inventário), mas é a versão "formulário": tudo em texto e botões
padrão. Este plano a torna mais moderna e intuitiva **sem mudar o core**.

## O que incomoda hoje

- O fluxo não é óbvio: há dois painéis iguais, sem dizer o que fazer primeiro.
- Tudo parece formulário: textos e botões do mesmo peso, sem hierarquia.
- O inventário é uma lista de texto; não dá para ver, num relance, o que está ligado,
  o ID, o tipo (carro, moto, barco, avião) e se tem tuning e paintjobs.
- Erros e avisos aparecem só como linhas no rodapé.
- A configuração (caminho do GTA) ocupa um lugar nobre da tela.

## Princípios

1. **Um caminho principal:** soltar → conferir → injetar. Tudo o mais é secundário.
2. **Mostrar o estado, não pedir:** nome, nome curto e referência já vêm sugeridos; o
   usuário só corrige o que estiver errado (já funciona no lote).
3. **Feedback imediato:** toda ação mostra o resultado na tela (toast), não só no log.
4. **Densidade alta, tema GTA SA:** fundo escuro, dourado `#D4AF37` como acento, cantos
   retos (já definido no ESCOPO §6), mas com mais respiro e hierarquia.

## Estrutura da tela (1100×720, mínimo 880×560)

```text
┌──────────────────────────────────────────────────────────────────────┐
│ SA-AWR            [ Importar ]  [ Instalados (12) ]        ●  GTA SA │  <- barra superior
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐         │
│   │   Solte aqui a pasta do mod, ou uma pasta com vários   │         │  <- zona de drop grande
│   │   [ Procurar pasta ]   [ Procurar arquivos ]            │         │
│   └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │
│                                                                      │
│   ☑ [ícone] VW New Beetle        ref: uranus ▾   tráfego: auto ▾     │  <- cartões da fila
│        beetle · tuning · 4 paintjobs · cores IVF            ✕        │
│   ☑ [ícone] bmw800gs             ref: sanchez ▾  tráfego: auto ▾     │
│        ⚠ nome curto repetido                                         │
│                                                                      │
│                              [ ⚡ INJETAR 2 NO JOGO ]                 │  <- botão fixo, dourado
├──────────────────────────────────────────────────────────────────────┤
│ ✓ ModLoader  ✓ FLA  ✓ OLA    Último: injetado 12517-bmw800g   [Log]  │  <- rodapé fino
└──────────────────────────────────────────────────────────────────────┘
```

Duas abas simples na barra superior (não é navegação lateral):

- **Importar:** a zona de drop, a fila e o botão de injetar.
- **Instalados:** o inventário, em cartões (ver abaixo).

O indicador "GTA SA" abre um popover com o caminho da instalação, a faixa de ID e o
botão "Reparar instalação". Sai do rodapé.

### Cartão do veículo instalado

```text
┌────────────────────────────────────────────────────────────┐
│ [ícone do tipo]  VW New Beetle                  12502  ON ◉│
│ ref: uranus · tuning (14) · paintjobs (4) · IVF            │
│ tráfego: como a referência                    [ Excluir ]  │
└────────────────────────────────────────────────────────────┘
```

- Interruptor ON/OFF no lugar do botão de texto.
- Mod desligado fica esmaecido. Mod de terceiros aparece com um selo "terceiro" e sem ações.
- Excluir pede confirmação **no próprio cartão** (já é assim hoje).
- Filtro por texto e por tipo (carro, moto, barco, avião) no topo da aba.

## Interações

- **Soltar em qualquer lugar da janela** funciona (já funciona); a janela inteira escurece
  com a mensagem "Solte para adicionar à fila".
- Ao soltar, a aba muda para **Importar** sozinha.
- **Toast** no canto por 4 segundos: "3 veículos injetados", "erro: …" (vermelho, fica até
  fechar). O Log continua no rodapé para o histórico.
- Barra de progresso no botão durante a injeção (a cópia de `.dff` de 10 MB leva alguns
  segundos hoje e a janela congela). A injeção passa a rodar numa thread.
- Atalhos: `Ctrl+O` abre pasta, `Ctrl+Enter` injeta, `Esc` limpa a fila.
- Referência com **busca fuzzy** de verdade (hoje é "contém").

## Visual

- Fonte: a padrão do egui, com tamanhos em escala (título 20, corpo 14, detalhe 12).
- Ícones: pequenos SVGs próprios por tipo (carro, moto, barco, avião), dourados.
- Cartões: fundo `#1B1B1B`, borda `#2A2A2A`, borda dourada ao passar o mouse.
- Estados: dourado = ação principal, vermelho `#E05A4F` = erro, cinza = secundário.

## Etapas (cada uma entrega algo usável)

1. **Estrutura:** abas Importar e Instalados, barra superior, rodapé fino, popover do GTA SA.
2. **Cartões:** fila e inventário em cartões, ícones por tipo, filtros.
3. **Feedback:** toasts, injeção em thread com progresso, busca fuzzy.
4. **Acabamento:** atalhos, estados vazios ilustrados, ajuste de contraste.

## Perguntas em aberto

- Quer **duas abas** (Importar / Instalados) ou manter tudo numa tela só, com o inventário
  sempre visível ao lado? A tela única é mais fiel ao ESCOPO §6; as abas dão mais espaço
  aos cartões.
- Os **ícones por tipo** devem ser desenhados por mim (SVG simples) ou você prefere outro
  estilo?
- A janela deve lembrar tamanho e posição entre execuções?
