# Plano de desenvolvimento — SA-AWR v2

## Context

A spec `E:\downloads\SA-AWR-V2-ESCOPO.md` foi revisada e alinhada aos tutoriais MixMods
(carros + armas, OLA obrigatório, `Vehicle/Weapon Models` sempre com `#`, sync remove `#`
só das chaves que o tutorial manda editar). Agora começa a implementação.
Decisões do dono nesta sessão:
- **Do zero**, sem código legado disponível (o "portar" da §10 vira "escrever").
- Repo novo em `E:\projetos\SAAWR` (pasta vazia; `git init` lá).
- Instalação real de teste: `D:\Games\GTA San Andreas`.
- Detalhar Fases 0 e 1; Fases 2–4 só em resumo.

## O que a instalação real mostrou (muda a spec)

- FLA instalado **dentro do modloader**: `modloader\$fastman92 limit adjuster\`
  (`.asi` + `data\gtasa_vehicleAudioSettings.cfg`, `data\gtasa_weapon_config.dat`).
- **Não existe `fastman92limitAdjuster_GTASA.ini`** em lugar nenhum da instalação.
  → Pré-requisito: o usuário coloca o `.ini` oficial (do download do FLA) ao lado do
  `.asi`. A ferramenta **não gera** o `.ini`; ausente = `Error` no checker (já previsto).
  Esse `.ini` oficial também é a fixture `fla_original.ini`.
- OLA em `modloader\Open Limit Adjuster\III.VC.SA.LimitAdjuster.asi`, com
  `VehicleModels/WeaponModels/VehicleStructs = unlimited`. Detecção: procurar o `.asi`
  na raiz e em `modloader\*\`.
- `gtasa_vehicleAudioSettings.cfg` termina em `;the end` **sem newline final** → inserir
  antes do sentinela sem criar/remover newline no fim (caso de golden test).
- Arquivos originais em `data\` (CRLF, ASCII; `weapon.dat` tem byte CP-1252 do `£`).
  Formatos confirmados: handling `LANDSTAL  1700.0 …` (tabs+espaços misturados),
  IDE `400,	landstal, 	landstal, 	car, …`, cargrp `linerun, packer, … # POPCYCLE_GROUP_WORKERS`.

## Desvios da spec (simplificações, a registrar no ESCOPO.md)

1. **Referências lidas dos arquivos originais do jogo**, não de tabela escrita à mão:
   `data\vehicles.ide` (seção `cars`: nome, tipo car/bike/bmx/boat/plane/heli/quad/…)
   + `data\handling.cfg` (linha a clonar). Armas: `data\default.ide` (seção `weap`) +
   `data\weapon.dat`. Elimina ~270 entradas manuais e fica sempre correto.
   O `VehicleKind` sai da coluna de tipo do IDE.
2. **CLI sem `clap`**: `std::env::args` com `status | doctor | inject | remove`.
   `inject/remove` entram porque o critério da Fase 1 (sem GUI) precisa deles.
3. Dependências da Fase 0–1: `thiserror`, `serde`, `serde_json`, `encoding_rs`,
   `directories`, `tracing` (+ `tempfile` em dev). `eframe` só na Fase 2.

## Estrutura

```
E:\projetos\SAAWR\
  Cargo.toml                 # [workspace] members = ["crates/*"]
  .gitattributes             # *.ini *.cfg *.ide *.dat *.expected -text
  crates/awr-core/src/       # módulos abaixo, um por pilar da spec
  crates/awr-cli/src/main.rs
  fixtures/                  # cópias (nunca editar D:\ nos testes)
```

## Fase 0 — Fundação

Ordem (cada passo com teste antes do próximo):

1. `git init`, workspace, `.gitattributes`, CI GitHub Actions `cargo fmt --check`,
   `clippy -D warnings`, `test` (Windows runner).
2. **Fixtures**: copiar de `D:\Games\GTA San Andreas\data\` `handling.cfg`,
   `vehicles.ide`, `carcols.dat`, `cargrp.dat`, `default.ide`, `weapon.dat`,
   `object.dat`, e do FLA `gtasa_vehicleAudioSettings.cfg`, `gtasa_weapon_config.dat`,
   `fla_original.ini` (o oficial). Mais a fixture envenenada `; Acentuação ção é` em
   CP-1252. Teste `fixtures_are_crlf` verifica `\r\n` em todas (substitui `harvest.sh`).
3. `error.rs` — `AwrError` da §8 (com `AlreadyExists`, sem `IdRace`).
4. `fsutil.rs` — `read(path) -> Vec<u8>`, `write_atomic(path, &[u8])` (tmp no mesmo
   dir + rename), `backup_once(path)` (`.bak` se não existir), `cp1252_to_string` só
   para exibição. Teste: round-trip byte-exato da fixture envenenada.
5. `paths.rs` — `detect_install(hint)`: raiz = pasta com `gta_sa.exe`; `modloader\`;
   FLA = `*fastman92limitAdjuster*.asi` na raiz ou `modloader\*\` (+ `.ini` e `data\`
   ao lado); OLA idem. Teste com árvore fake em `tempfile`.
6. `config.rs` — `AppSettings { gta_path, id_floor_vehicle, id_floor_weapon }` em
   `directories::ProjectDirs::config_dir()/sa-awr.json`.
7. `meta.rs` + `inventory.rs` — schema §7 sem `enabled/foreign`; `scan()` lê
   `modloader\awr-cars\*` e `awr-weapons\*`; `enabled` = nome sem prefixo `. `;
   pasta `{id}-*` sem `meta.json` = `foreign`. `ModKey = PathBuf`.
8. `awr-cli status` imprime instalação detectada + inventário.

**Pronto quando:** `awr-cli status --gta "D:\Games\GTA San Andreas"` lista FLA/OLA/ModLoader
e inventário vazio; CI verde.

## Fase 1 — Pipeline de veículo

0. **Spike manual no jogo (antes de codar o injector, ~30 min, numa CÓPIA do jogo):**
   pasta de teste no modloader com um `linhas.txt` contendo handling + IDE + carcols
   misturados, e uma cópia de `cargrp.dat`. Confirma no jogo (Djjr Car Spawner) que o
   ModLoader aceita linhas mistas e o cargrp por pasta. Se não aceitar: um `.txt` por
   tipo de arquivo (`handling.txt`, `vehicles.txt`, `carcols.txt`). Resultado entra
   no ESCOPO.md.
   **Resultado (2026-10-23, `GTA SA - teste`, IDs 12500/12501):** linhas mistas em um
   `linhas.txt` e um `.txt` por tipo foram aceitos (spawn, colisão e entrada OK). Só com OLA
   o carro spawna voando (handling.cfg vanilla = 210 linhas): precisa do FLA com
   `Apply handling.cfg patch = 1`. Áudio e `Apply ID limit patch` passam; `Enable model
   special feature loader` crasha no load (access violation, EIP inválido) e fica fora do `plan()`.
   **Pendente:** cargrp por pasta e áudio clonado (linha própria) não testados no jogo.
1. `references.rs` — parse de `vehicles.ide` (seção `cars`) + `handling.cfg` do jogo →
   `Reference { name, kind, handling_line: Vec<u8>, ide_line, carcols_line }`.
   Teste: 212 veículos; `PCJ600` → Bike com linha `!` existente.
2. `validators.rs` — `short_name()` (minúsculo, `[a-z0-9]`, ≤ 7, único no inventário).
3. `ids.rs` — `collect_used_ids()` (400–611, IDs dos IDE originais, pastas `{id}-*`
   ativas e `. `-desativadas em todo o modloader até 2 níveis) e `allocate(floor)` =
   menor livre, pulando 12400–12499 para veículos. Testes da §9.
4. `fla.rs` — parser/writer sobre `Vec<u8>` (§4.2): seções, match de chave por nome
   completo, troca só o token numérico, remove `#` só nas chaves geridas, nunca toca
   `Vehicle/Weapon Models`, insere chave ausente na seção certa, monótono, pisos do
   tutorial. Golden: `fla_original.ini` → `fla_expected_sync.ini` byte-a-byte.
5. `handling.rs` / `ide.rs` / `carcols` (dentro de `ide.rs` se for 1 função) — clonar
   linha trocando só os spans dos tokens (nome em MAIÚSCULAS, model ID no IDE).
   Golden: `handling_expected_*.cfg`. Linhas especiais `!`/`$`/`%` clonadas junto.
6. `cargrp.rs` — acrescenta `, short` antes do ` # POPCYCLE_GROUP_*` da classe; remove
   só o próprio short. `audio.rs` — clona linha da referência renomeada, insere antes de
   `;the end` (teste: arquivo sem newline final continua sem).
7. `injector.rs` — `inject_vehicle`: staging `modloader\.awr-staging\` → globais
   (cargrp, áudio, FLA) com `.bak` → rename da pasta (`AlreadyExists` se existir).
   `remove`: pasta → globais. `set_enabled`: rename `. ` + tira/repõe linhas globais.
   `repair`: apaga staging e linhas globais cujo short não tem pasta AWR ativa.
   Teste de ida e volta (§9) numa cópia `tempfile` da árvore de fixtures.
8. `checker.rs` mínimo: instalação, ModLoader, FLA `.asi`+`.ini`, **OLA (Error)**,
   `Vehicle/Weapon Models` sem `#` (Warn), outro `cargrp.dat` (Warn), jogo rodando
   (`gta_sa.exe` travado para escrita → Warn/`LockedByGame`). `inject_*` recusa com
   `Error` presente.
9. CLI: `doctor`, `inject --name --dff --txd --ref`, `remove <pasta>`.

**Pronto quando:** numa cópia do jogo, injetar 3 carros reais (um carro, uma moto, um
barco), spawnar no jogo, removê-los; `repair` não acha resíduo; `diff` dos arquivos
originais em `data\` = zero; `.ini` do FLA só mudou nas chaves geridas.

## Resultado da Fase 3 (armas)

Confirmado no jogo (2026-10-23): o ModLoader aceita `IDE DATA\MAPS\{short}.ide` no `Loader.txt` e as
linhas `$`/`£` do `weapon.dat` no mesmo arquivo; a arma nova usa o tipo de arma a partir de 70 no
`gtasa_weapon_config.dat`. Armadilha: o RZL Trainer pede o **tipo** (70), não o ID do modelo (12400).

## Fases 2–4 (resumo; detalhar ao fim da Fase 1)

- **2. UI egui** tela única §6 sobre `AwrCore` (drop, dropdown de referência, lista,
  toggle/excluir, rodapé com `health()`).
- **3. Armas**: `weapons.rs` (Loader.txt com IDE + linhas `$`/`£` do weapon.dat +
  object.dat; Weapons.ide; cópia da linha no `gtasa_weapon_config.dat`; type ID ≥ 70;
  aviso de textura `{short}icon` ausente).
- **4. Release**: `cargo-dist`, `.exe` portable, beta tester.

## Verificação

- `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` (local + CI).
- Golden tests comparam `Vec<u8>` com `fs::read("fixtures/*.expected")`.
- Testes de escrita só em `tempfile`; **nunca** escrevem em `D:\Games\GTA San Andreas`.
- Aceite real: cópia do jogo (ex.: `D:\Games\GTA SA - teste`), `awr-cli` + spawner no jogo.

## Riscos

- Spike do passo 1.0 pode mudar o layout de pasta do injector.
- O `.ini` oficial do FLA precisa vir do usuário (não está na instalação).
