# Ferramentas para GTA SA — Adicionar/Remover veículos novos

Conjunto de scripts para gerenciar veículos adicionados sem substituir,
usando **modloader** + **fastman92 limit adjuster**.

Método baseado no tutorial da [MixMods: Adicionar carros sem substituir](https://www.mixmods.com.br/2020/02/tutorial-adicionar-carros-sem-substituir/).

---

## Scripts disponíveis

| Script | Função |
|--------|--------|
| `add_car.py` | Adicionar veículos novos automaticamente |
| `clean_car.py` | Listar instalados e limpar resíduos de veículos deletados |

---

## Como o método funciona

O GTA SA original suporta **212 veículos** (IDs 400–611). Para adicionar novos sem substituir:

1. **fastman92 limit adjuster** expande os limites do jogo (handling, IDE, áudio, IDs)
2. **Modloader** lê arquivos `.txt` em pastas e mescla com os dados originais
3. Cada veículo novo fica numa pasta com `.dff` + `.txd` + `linhas.txt` + `.fxt`

### Dependências

| Componente | Função |
|---|---|
| [Silent's ASI Loader](https://www.mixmods.com.br/2013/02/silent-asi-loader.html) | Carregar .asi plugins |
| [ModLoader](https://www.mixmods.com.br/2015/01/SA-Modloader.html) | Gerenciar mods por pastas |
| [fastman92 limit adjuster](https://www.mixmods.com.br/2015/09/fastman92-limit-adjuster.html) | Expandir limites do jogo |

> ⚠ **Importante:** O uso do **fastman92 limit adjuster** em conjunto com o
> **[Open Limit Adjuster (OLA)](https://www.mixmods.com.br/2014/09/iiivcsa-open-limit-adjuster.html)**
> é **altamente recomendado**. O OLA gerencia os limites de forma automática
> (`Vehicle Models`, `VehicleStructs`, `Cargrp cars per group`, etc).
> Se você usa OLA, **não edite** a linha `Vehicle Models` no .ini do fastman92.

### Configuração do fastman92

No `$fastman92 limit adjuster/fastman92limitAdjuster_GTASA.ini`:

```ini
Apply handling.cfg patch = 1
Number of standard lines = 500
Number of bike lines = 30
Count of killable model IDs = 19601
Make paintjobs work for any ID = 1
Enable vehicle audio loader = 1
```

---

### Sincronizar trânsito (`--sync`)

Se você tem veículos instalados que não aparecem no trânsito (só por trainer),
roda o `--sync` que ele adiciona todos ao `cargrp.dat` automaticamente:

```bash
python ferramentas/add_car.py --sync
```

```
✓ 8 entrada(s) adicionada(s) ao cargrp.dat
```

Útil quando:
- Você instalou veículos manualmente antes do script existir
- Algo corrompeu o `cargrp.dat`
- Quer garantir que tudo está circulando

---

## add_car.py — Adicionar veículo

### 1. Cria a pasta

```
modloader/novos-carros/
└── lamborghini-gallardo-2015/    ← nome livre (sem ID)
    ├── qualquer.dff              ← qualquer nome
    ├── qualquer.txd              ← qualquer nome
    └── "Lamborghini Gallardo 2015.txt"  ← ver passo 2
```

### 2. Cria o `.txt`

O **nome do arquivo** vira o nome no jogo. O **conteúdo** é o veículo referência
(handling, áudio, classe de trânsito serão clonados dele).

```
"Nome do Veiculo 2015.txt"  →  display: "Nome do Veiculo '15"
  (conteúdo: "infernus")    →  clona do Infernus
```

Anos de 4 dígitos no final do nome são convertidos automaticamente (`2015` → `'15`).

### 3. (Opcional) Nome curto personalizado

Por padrão, o script gera um nome curto automático de até 7 caracteres.
Para forçar um nome específico, use **formato de 2 linhas** no `.txt`:

```
lambo
infernus
```

| Linha | Conteúdo |
|-------|----------|
| 1ª | Nome curto desejado (≤7 chars, único) |
| 2ª | Veículo referência para clonar |

Sem a 1ª linha, o comportamento é o mesmo de antes (só a referência).

### 4. Roda

```bash
python ferramentas/add_car.py
```

```
Veículos pendentes encontrados: 1

──────────────────────────────────────────────────
Pasta: lamborghini-gallardo-2015
──────────────────────────────────────────────────
  → .txt: 'Lamborghini Gallardo 2015' → display: 'Lamborghini Gallardo '15'
  → Clone de: infernus
  ✓ Referência: infernus (tipo: car, classe: executive)
  ✓ Handling de referência encontrado
  ✓ Próximo ID: 12508
  ✓ Nome curto: lambo (5 chars)
  ✓ qualquer.dff → lambo.dff
  ✓ qualquer.txd → lambo.txd
  ✓ linhas.txt criado
  ✓ lambo.fxt criado ('Lamborghini Gallardo '15')
  ✓ Áudio adicionado (clonado de infernus)
  ✓ Adicionado a CASUAL_RICH, BUSINESS no cargrp.dat
  ✓ Pasta renomeada: lamborghini-gallardo-2015 → 12508-lamborghini-gallardo-2015

  ✅ Lamborghini Gallardo '15 (ID 12508, lambo) pronto!
```

A pasta é **renomeada automaticamente** para `{ID}-{nome-normalizado}`.

Pode deixar várias pastas pendentes de uma vez — o script processa todas em ordem.

---

## Segurança: Backups automáticos

Sempre que o script altera um arquivo global (`cargrp.dat`,
`gtasa_vehicleAudioSettings.cfg`), ele **cria um `.bak` automaticamente**
antes de escrever:

```
modloader/add-transito/cargrp.bak
modloader/$fastman92 limit adjuster/data/gtasa_vehicleAudioSettings.cfg.bak
```

Se algo der errado (encoding incorreto, interrupção), é só restaurar o `.bak`.

---

## Gerenciamento de áudio: Marcadores `BEGIN / END`

O `add_car.py` agora delimita os veículos gerados com marcadores fixos:

```ini
# BEGIN GENERATED VEHICLES
lambo   0   38  37  1  0.9 ...
audib6  0   38  37  1  0.9 ...
# END GENERATED VEHICLES
;the end
```

Isso permite que o `clean_car.py` identifique resíduos **de forma cirúrgica**:
apenas o que está dentro dos marcadores é escaneado, sem tocar em entradas
originais ou comentários do fastman92.

---

## clean_car.py — Listar e limpar resíduos

```bash
python ferramentas/clean_car.py
```

Lista todos os veículos instalados e verifica se há entradas fantasmas no
**áudio** (`gtasa_vehicleAudioSettings.cfg`) e no **trânsito** (`cargrp.dat`)
de veículos que foram deletados.

```
Veículos instalados:
  ID       Modelo    Display                           Pasta
  ──────── ───────── ───────────────────────────────── ─────────────────────────
  12501    hfr92     Honda CBR900RR Fireblade '92      12501-honda-fireblade-92
  12507    audib6    Audi A4                           12507-audi-a4-b6-sline

Nenhum resíduo encontrado. Tudo limpo!
```

Se encontrar resíduos, pergunta se quer removê-los:

```
Resíduos encontrados (2):

  Áudio (gtasa_vehicleAudioSettings.cfg):
    xxxxxxxx  → linha 286

  Trânsito (cargrp.dat):
    xxxxxxxx  → POPCYCLE_GROUP_CASUAL_RICH

Remover resíduos? (s/N):
```

---

## O que o `add_car.py` faz

| # | Ação | Detalhe |
|---|------|---------|
| 1 | Identifica pendentes | Pastas com `.dff` + `.txd` + `.txt` sem `linhas.txt` |
| 2 | Lê o `.txt` | Nome → display name. Conteúdo → veículo p/ clonar |
| 3 | Define o ID | **primeiro gap** a partir de 12501 (reusa IDs de veículos deletados) |
| 4 | Gera nome curto | ≤7 chars, sem colisão |
| 5 | Renomeia `.dff`/`.txd` | Evita conflito com veículos originais |
| 6 | Clona handling | Copia linha de handling do veículo referência |
| 7 | Gera `linhas.txt` | 4 seções: IDE, Handling, Carcols, Carmods |
| 8 | Gera `.fxt` | Nome que aparece ao entrar no veículo |
| 9 | Adiciona áudio | Insere antes de `;the end` no arquivo do fastman92 |
| 10 | Adiciona ao trânsito | Insere no `cargrp.dat` no grupo da classe |
| 11 | Renomeia a pasta | `{ID}-{nome-normalizado}` |
| 12 | Valida | IDs, nomes, arquivos necessários |

### Clonagem do veículo referência

| Item | Origem |
|------|--------|
| Tipo (`car`, `bike`, `boat`...) | vehicles.ide → coluna `Type` |
| Classe (`executive`, `normal`...) | vehicles.ide → coluna `Class` |
| Grupos de trânsito | Mapeado da classe para `cargrp.dat` |
| Anims | vehicles.ide → coluna `Anims` |
| Flags, CompRules, Rodas | vehicles.ide → colunas finais |
| Handling | `data/handling.cfg` |
| Áudio | `gtasa_vehicleAudioSettings.cfg` |

---

## Estrutura de pastas

```
GTA San Andreas/
├── ferramentas/
│   ├── add_car.py
│   ├── clean_car.py
│   └── README.md
│
└── modloader/
    ├── $fastman92 limit adjuster/
    │   └── data/
    │       └── gtasa_vehicleAudioSettings.cfg  ← áudio (add_car.py edita)
    │
    └── novos-carros/
        ├── data/                         ← dados ORIGINAIS (NÃO EDITAR)
        │   ├── vehicles.ide
        │   ├── handling.cfg
        │   ├── carcols.dat
        │   └── carmods.dat
        │
        ├── add-transito/
        │   └── cargrp.dat                ← trânsito (add_car.py edita)
        │
        ├── 12501-honda-fireblade-92/     ← instalado
        ├── 12502-suzuki-gsx-r750rr/      ← instalado
        └── 12508-lamborghini-gallardo-2015/  ← NOVO (pasta renomeada)
            ├── lambo.dff
            ├── lambo.txd
            ├── lambo.fxt
            ├── linhas.txt
            └── "Lamborghini Gallardo 2015.txt"
```

---

## Mapeamento classe → trânsito

| Classe | Grupos no `cargrp.dat` |
|---|---|
| `richfamily` | CASUAL_RICH, BUSINESS |
| `normal` | CASUAL_AVERAGE |
| `poorfamily` | CASUAL_POOR |
| `worker` | WORKERS |
| `executive` | CASUAL_RICH, BUSINESS |
| `motorbike` | BEACHFOLK, PARKFOLK, CASUAL_AVERAGE, CASUAL_RICH |
| `moped` | BEACHFOLK, PARKFOLK |
| `taxi` | WORKERS, BUSINESS, CLUBBERS |
| `bicycle` | BEACHFOLK, PARKFOLK |

---

## Dicas

**Spawnar**: [Djjr Car Spawner](https://www.mixmods.com.br/2015/12/djjr-car-spawner.html)
ou [Car Spawner by fastman92](https://www.mixmods.com.br/2017/01/car-spawner-by-fastman92.html)

**Paintjobs**: `Make paintjobs work for any ID = 1` no .ini do fastman92

**Peças tuning**: [Tutorial Daniel69](https://forum.mixmods.com.br/f37-tutoriais/t1714-como-adicionar-pecas-tuning-sem-substituir)

**Funcionalidades especiais** (suspensão, faróis pop-up):
`model_special_features.dat` com `Enable model special feature loader = 1`

---

## Referências

- Tutorial original: [MixMods — Adicionar carros sem substituir](https://www.mixmods.com.br/2020/02/tutorial-adicionar-carros-sem-substituir/)
- Fórum MixMods: [forum.mixmods.com.br](https://forum.mixmods.com.br)
- Lista de IDs livres: [tuningmodparts.blogspot.com](http://tuningmodparts.blogspot.com.br/p/ids.html)

---

## Compartilhar

Basta zipar a pasta `modloader/novos-carros/`. Cada veículo é auto-contido.

Quem receber precisa de:
1. **Modloader** instalado
2. **fastman92 limit adjuster** instalado (com a [configuração](#configuração-do-fastman92) acima)
3. O arquivo `gtasa_vehicleAudioSettings.cfg` com as entradas de áudio dos veículos
