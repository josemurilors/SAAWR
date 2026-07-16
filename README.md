# SA - AWR (Add Without Replace)

Ferramentas para gerenciar veículos e armas adicionados ao GTA San Andreas sem substituir,
usando **modloader** + **fastman92 limit adjuster**.

Método baseado nos tutoriais da MixMods:
- [Adicionar carros sem substituir](https://www.mixmods.com.br/2020/02/tutorial-adicionar-carros-sem-substituir/)
- [Adicionar armas sem substituir](https://www.mixmods.com.br/2019/02/tutorial-adicionar-armas-sem-substituir/)

---

## Configuração

Antes de usar, copie `config.ini.example` para `config.ini` e ajuste os caminhos:

```bash
cp config.ini.example config.ini
```

Edite `config.ini` com os caminhos corretos da sua instalação:

```ini
[paths]
game_dir = /mnt/sda1/games/GTA San Andreas
modloader_vehicles = modloader/novos-carros
modloader_weapons = modloader/novas-armas
fastman92_dir = modloader/$fastman92 limit adjuster
game_data = data

[weapon_config]
weapon_id_start = 12400
vehicle_id_start = 12501
weapon_config_file = data/gtasa_weapon_config.dat
```

---

## Scripts disponíveis

| Script | Função |
|--------|--------|
| `add_car.py` | Adicionar veículos novos automaticamente |
| `clean_car.py` | Listar instalados e limpar resíduos de veículos deletados |
| `add_weapon.py` | Adicionar armas novas automaticamente |
| `clean_weapon.py` | Listar instalados e limpar resíduos de armas deletadas |
| `gta_utils.py` | Utilitários compartilhados (não executa sozinho) |

> `gta_utils.py` é um **módulo de suporte** importado pelos outros scripts.
> Ele fornece escrita segura com backup (`.bak`), leitura de arquivos com encoding
> `cp1252`, carregamento de configuração e os marcadores `BEGIN/END` que delimitam
> veículos/armas gerados nos arquivos globais.

---

## Como o método funciona

### Veículos

O GTA SA original suporta **212 veículos** (IDs 400–611). Para adicionar novos sem substituir:

1. **fastman92 limit adjuster** expande os limites do jogo (handling, IDE, áudio, IDs)
2. **Modloader** lê arquivos `.txt` em pastas e mescla com os dados originais
3. Cada veículo novo fica numa pasta com `.dff` + `.txd` + `linhas.txt` + `.fxt`

### Armas

O GTA SA original suporta **53 armas** (IDs 321–373). Para adicionar novas sem substituir:

1. **fastman92 limit adjuster** expande os limites de tipos de arma
2. **Modloader** lê arquivos `.txt` em pastas e mescla com os dados originais
3. Cada arma nova fica numa pasta com `.dff` + `.txd` + `linhas_arma.txt` + `.fxt`

### Dependências

| Componente | Função |
|---|---|
| [Silent's ASI Loader](https://www.mixmods.com.br/2013/02/silent-asi-loader.html) | Carregar .asi plugins |
| [ModLoader](https://www.mixmods.com.br/2015/01/SA-Modloader.html) | Gerenciar mods por pastas |
| [fastman92 limit adjuster](https://www.mixmods.com.br/2015/09/fastman92-limit-adjuster.html) | Expandir limites do jogo |

> ⚠ **Importante:** O uso do **fastman92 limit adjuster** em conjunto com o
> **[Open Limit Adjuster (OLA)](https://www.mixmods.com.br/2014/09/iiivcsa-open-limit-adjuster.html)**
> é **altamente recomendado**. O OLA gerencia os limites de forma automática.

### Configuração do fastman92

No `$fastman92 limit adjuster/fastman92limitAdjuster_GTASA.ini`:

```ini
Apply handling.cfg patch = 1
Number of standard lines = 500
Number of bike lines = 30
Count of killable model IDs = 19601
Make paintjobs work for any ID = 1
Enable vehicle audio loader = 1

; Para armas:
Weapon Models = 51
Enable weapon type loader = 1
Weapon type loader, number of type IDs = 80
```

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

### 3. (Opcional) Nome curto personalizado

Para forçar um nome específico, use **formato de 2 linhas** no `.txt`:

```
lambo
infernus
```

### 4. Roda

```bash
python ferramentas/add_car.py
```

---

## add_weapon.py — Adicionar arma

### 1. Cria a pasta

```
modloader/novas-armas/
└── Desert Eagle Neon/
    ├── deagle.dff               ← modelo da arma
    ├── deagle.txd               ← textura da arma
    ├── deagleicon.txd           ← ícone da arma (opcional, renomeado automaticamente)
    └── "Desert Eagle Neon.txt"  ← ver passo 2
```

### 2. Cria o `.txt`

O **nome do arquivo** vira o nome no jogo. O **conteúdo** é a arma referência
(comportamento serão clonados dela).

```
"Desert Eagle Neon.txt"  →  display: "Desert Eagle Neon"
  (conteúdo: "desert_eagle")  →  clona do Desert Eagle
```

### 3. (Opcional) Nome curto personalizado

Para forçar um nome específico, use **formato de 2 linhas** no `.txt`:

```
deaglneon
desert_eagle
```

### 4. Roda

```bash
python ferramentas/add_weapon.py
```

```
Armas pendentes encontradas: 1

──────────────────────────────────────────────────
Pasta: Desert Eagle Neon
──────────────────────────────────────────────────
  → .txt: 'Desert Eagle Neon' → display: 'Desert Eagle Neon'
  → Clone de: desert_eagle
  ✓ Referência encontrada no default.ide (ID: 348)
  ✓ Linha de comportamento encontrada no weapon.dat
  ✓ Próximo ID: 12400
  ✓ Nome curto: deaglneon (8 chars)
  ✓ deagle.dff → deaglneon.dff
  ✓ deagle.txd → deaglneon.txd
  ✓ deagleicon.txd → deaglneonicon.txd
  ✓ linhas_arma.txt criado
  ✓ deaglneon.fxt criado ('Desert Eagle Neon')

  ✅ Desert Eagle Neon (ID 12400, deaglneon) pronto!
  Pasta: 12400-desert-eagle-neon
  Próximo passo: Execute o aplicativo 'Gerar Free IDs List' do fastman92
```

### 5. Configuração do fastman92 para armas

Após adicionar armas, execute o **"Gerar Free IDs List"** do fastman92
para gerar a lista de IDs livres. Isso atualiza o arquivo
`gtasa_weapon_config.dat` automaticamente.

---

## clean_car.py — Listar e limpar resíduos de veículos

```bash
python ferramentas/clean_car.py
```

Lista todos os veículos instalados e verifica resíduos no áudio e trânsito.

---

## clean_weapon.py — Listar e limpar resíduos de armas

```bash
python ferramentas/clean_weapon.py
```

Lista todas as armas instaladas e verifica resíduos no
`gtasa_weapon_config.dat`, `WeaponLoader.txt` e `Weapons.ide`.

---

## Estrutura de pastas

```
GTA San Andreas/
├── ferramentas/
│   ├── config.ini              ← configuração de caminhos
│   ├── add_car.py
│   ├── clean_car.py
│   ├── add_weapon.py
│   ├── clean_weapon.py
│   └── README.md
│
├── data/
│   ├── default.ide             ← armas originais (seção weap)
│   ├── vehicleIDE              ← veículos originais
│   └── weapon.dat              ← comportamento de armas
│
└── modloader/
    ├── $fastman92 limit adjuster/
    │   └── data/
    │       ├── gtasa_vehicleAudioSettings.cfg  ← áudio (add_car.py edita)
    │       └── gtasa_weapon_config.dat         ← config armas (add_weapon.py edita)
    │
    ├── novos-carros/
    │   ├── data/               ← dados ORIGINAIS (NÃO EDITAR)
    │   ├── add-transito/       ← cargrp.dat
    │   └── {ID}-{nome}/        ← veículos instalados
    │
    └── novas-armas/            ← NOVO
        ├── data/               ← dados ORIGINAIS (NÃO EDITAR)
        ├── WeaponLoader.txt    ← "IDE DATA\MAPS\Weapons.ide"
        ├── Weapons.ide         ← seção weap/end com todas as armas
        └── {ID}-{nome}/        ← armas instaladas
            ├── {short}.dff
            ├── {short}.txd
            ├── {short}icon.txd
            ├── {short}.fxt
            ├── linhas_arma.txt
            └── "Nome Arma.txt"
```

---

## Segurança: Backups automáticos

Sempre que o script altera um arquivo global, ele **cria um `.bak` automaticamente**:

```
gtasa_vehicleAudioSettings.cfg.bak
gtasa_weapon_config.dat.bak
cargrp.dat.bak
```

---

## Dicas

**Spawnar veículos**: [Djjr Car Spawner](https://www.mixmods.com.br/2015/12/djjr-car-spawner.html)

**Paintjobs**: `Make paintjobs work for any ID = 1` no .ini do fastman92

**Peças tuning**: [Tutorial Daniel69](https://forum.mixmods.com.br/f37-tutoriais/t1714-como-adicionar-pecas-tuning-sem-substituir)

---

## Referências

- Tutorial carros: [MixMods — Adicionar carros sem substituir](https://www.mixmods.com.br/2020/02/tutorial-adicionar-carros-sem-substituir/)
- Tutorial armas: [MixMods — Adicionar armas sem substituir](https://www.mixmods.com.br/2019/02/tutorial-adicionar-armas-sem-substituir/)
- Fórum MixMods: [forum.mixmods.com.br](https://forum.mixmods.com.br)
- Lista de IDs livres: [tuningmodparts.blogspot.com](http://tuningmodparts.blogspot.com.br/p/ids.html)

---

## Compartilhar

Basta zipar as pastas `modloader/novos-carros/` ou `modloader/novas-armas/`.
Cada item é auto-contido.

Quem receber precisa de:
1. **Modloader** instalado
2. **fastman92 limit adjuster** instalado (com a configuração acima)
3. Os arquivos de configuração do fastman92 com as entradas de áudio/armas
