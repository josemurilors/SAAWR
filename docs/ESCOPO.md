# SA-AWR v2 — Escopo & Documentação Unificada

> **SA-AWR (Add Without Replace)** — utilitário desktop em Rust puro para adicionar
> veículos e armas ao GTA San Andreas **sem substituir** nada do jogo original,
> via ModLoader + fastman92 Limit Adjuster (FLA).
>
> Status: **REESCRITA DO ZERO** decidida. Mesmo repositório (`/home/josemurilors/SAAWR`),
> histórico resetado, código atual vira legado de referência.
> Data: 2026-09-08 · Documento único de escopo + arquitetura + contrato de API + formatos + migração.

---

## 1. Decisão de Reset (fatos, não discussão)

O reset elimina débitos técnicos acumulados: frontend React/Tauri abandonado pela
migração egui já em curso, `commands.rs` IPC órfão, controles de UI sem backend,
persistência de config parcial. A v2 nasce com **backend Rust ultra-robusto** e
**interface minimalista de tela única**.

Princípios inegociáveis da v2:

1. **100% Rust.** Sem Node, Vite, React, Tauri ou WebView. `eframe/egui` nativo.
2. **Stateless por design.** A fonte da verdade é o disco: cada pasta de mod tem
   `meta.json`; o inventário é reconstruído por varredura. Nada de banco paralelo.
3. **Tudo é função pura ou `Result`.** Sem `unwrap()`/`panic!` em caminho de negócio,
   escrita atômica (tmp + rename), backup `.bak` antes de tocar arquivo global.
4. **Core testável sem GUI.** `awr-core` não conhece egui; um CLI de diagnósticos
   exercita o mesmo core.

---

## 2. Stack Técnico

| Camada | Tecnologia | Papel |
| ------ | ---------- | ----- |
| GUI | `eframe` 0.36 + `egui` | Tela única, painel duplo, tema GTA SA dark + dourado |
| Core | `awr-core` (crate de lib) | Toda a lógica de negócio, parsers, I/O, alocação |
| Shell | `awr-app` (bin egui) + `awr-cli` (bin clap) | UI + diagnóstico (`status`, `doctor`) sobre o mesmo core |
| I/O | `encoding_rs` (windows-1252) + `std::fs::read_dir` | Arquivos do jogo usam CP-1252; varredura tem 1 nível (sem `walkdir`), escrita sem regex (sem `regex`) |
| Erros | `thiserror` (`AwrError`) + `anyhow` na casca | Tipos de erro explícitos por domínio |
| Serialização | `serde` + `serde_json` (compacto) | `meta.json` |
| Logging | `tracing` + `tracing-subscriber` | Log em arquivo + console embutido na UI |
| Testes | `tempdir`/`tempfile`, fixtures de `.cfg/.ini/.ide` | Golden tests por parser |
| Release | `cargo-dist` (Windows portable .exe primeiro) | CI GitHub Actions já padronizada |

---

## 3. Arquitetura do Workspace

```text
SAAWR/                      # repo resetado (branch main nova)
├── Cargo.toml              # [workspace] members = crates/*
├── crates/
│   ├── awr-core/           # lógica pura, zero dependência de GUI
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs        # AwrError (thiserror), invariantes de falha
│   │       ├── paths.rs        # detect_install(): Portable | Standalone | raiz
│   │       ├── config.rs       # AppSettings mínimo (gta_path, perfil modloader)
│   │       ├── inventory.rs    # scan() → Vec<InstalledMod> a partir de meta.json
│   │       ├── ids.rs          # ID Allocator determinístico
│   │       ├── fla.rs          # FLA Sync Engine (parser/writer do .ini)
│   │       ├── injector.rs     # ModLoader File Injector (criar/ativar/desativar/apagar)
│   │       ├── meta.rs         # schema e I/O do meta.json
│   │       ├── handling.rs     # parser/serializer handling.cfg (prefixos !, $, %)
│   │       ├── ide.rs          # vehicles.ide / mload collision lines
│   │       ├── cargrp.rs       # grupos de tráfego (cargrp.dat)
│   │       ├── audio.rs        # clonagem/silêncio no gtasa_vehicleAudioSettings.cfg
│   │       ├── weapons.rs      # pipeline de armas + gtasa_weapon_config.dat
│   │       ├── references.rs   # tabelas das 212 refs de veículo + 53 de arma
│   │       ├── validators.rs   # nomes, IDs, unicidade, extensões aceitas
│   │       ├── fsutil.rs       # escrita atômica, .bak, permissões
│   │       └── checker.rs      # Sanity & Health Engine
│   ├── awr-app/            # bin egui: ui.rs, panels/, theme.rs, app.rs
│   └── awr-cli/            # bin: status | doctor (inject/remove só se alguém pedir; testes já exercitam o core)
├── fixtures/               # handling.cfg, FLA .ini, .ide reais p/ golden tests
└── docs/ → este arquivo versionado como docs/ESCOPO.md
```

Grafo de dependência: `awr-app → awr-core`, `awr-cli → awr-core`. Nunca o inverso.

---

## 4. Pilares do Backend (especificação)

### 4.1 ID Allocator (determinístico)

- Faixas padrão (configuráveis; teto rígido = 19999):
  - **veículos: 12500–19999**, pulando 12400–12499 se o floor for baixado;
  - **armas (model ID): 12400–12499** (ver abaixo).
  Segue a convenção do tutorial OFICIAL MixMods: 12000–12499 é historicamente
  ocupado por peças do Tuning Mod, e 12400–12499 é reservado a armas. Assim o
  primeiro carro numa instalação limpa recebe 12500, não 12000. O `killable` é
  ajustado pelo sync (§4.2) para `≥ último ID + folga`.
- `collect_used_ids()` varre: (a) IDs dos arquivos originais do jogo (400–611,
  321–373 e vizinhança), (b) IDs ocupados por pastas `{id}-*` (ativas ou `. `-desativadas) em qualquer
  perfil do modloader, (c) IDs presentes em `vehicles.ide`/`cargrp`/áudio órfãos.
- **Reaproveitamento:** menor ID livre dentro da faixa vence (determinístico,
  testável com fixture).
- **Semântica das pastas desativadas:** convenção **oficial do ModLoader** para
  não-carregar uma pasta é prefixar o nome com **ponto + espaço** (`. 12500-gtr34`)
  — NÃO `!` (esse é o prefixo de *linha* do handling.cfg, namespace separado).
  Pastas `. `-prefixadas ficam FORA do inventário ativo exibido, MAS entram em
  `collect_used_ids()` — IDs de mods desativados continuam reservados, então
  reativar nunca colide com um mod injetado no ínterim.
- **Concorrência:** app de um usuário, um processo; o `allocate()` lê o disco na
  hora da injeção e basta. Sem re-scan extra nem erro de corrida próprio (um
  re-scan não impediria dois processos de colidirem). O rename final (§8) nunca
  sobrescreve pasta existente.
- Armas: aloca DOIS números com convenções oficiais distintas (tutorial MixMods
  de armas, verificado 2026-09-08):
  - **model ID** do `.ide`/`weapon.dat`: faixa reservada oficial **12400–12499**
    (continua o floor `12400` que o legado já usa), subindo pelo configurável.
  - **type ID** do `gtasa_weapon_config.dat`: floor oficial **70** (71, 72, …).
    O `Weapon type loader, number of type IDs` do FLA é o TAMANHO DO ARRAY de
    tipos — usar type ID alto (o legado v1 atribuía `weapon_type_id = 12400`!)
    forçaria o FLA a alocar 12.401 slots de tipo, estourando RAM à toa (o
    tutorial alerta "tudo custa memória"). **Bug legado a não portar**: type ID
    e model ID são espaços separados.

### 4.2 FLA Sync Engine

- **Parser: loop manual byte-a-byte com máquina de estados de seção.** Proibido
  `read_to_string`, crates genéricas de INI (`rust-ini`, `serde_ini`) e regex como
  caminho de escrita: decodificam para `String` (bytes CP-1252 em comentários `;`
  do fastman92 polonês quebram ou sofrem replacement) e re-serializam com
  normalização de espaçamento/CRLF. O writer opera sobre `Vec<u8>`:
  split em `\n` preservando `\r` e o newline final; `current_section` atualizado
  em linhas `[...]`; match de chave por **igualdade do nome completo** (não
  `starts_with`), indentação e o bloco de espaços ao redor do `=` copiados
  verbatim; **apenas o token numérico** após o `=` é substituído.
- **OLA é obrigatório (decisão do dono, 2026-10-23, seguindo os tutoriais
  MixMods de carros e armas, §14).** Não existe modo "FLA-manual": sem OLA a
  injeção é bloqueada (§4.4). Os tutoriais dizem, para `Vehicle Models` e
  `Weapon Models`:
  > "Importante: Se você usa Open Limit Adjuster, NÃO EDITE ESTA LINHA! O Open
  > Limit Adjuster configura esse número automaticamente para você de modo
  > ilimitado. Se você o usa, pule esta etapa! E deixe “#” no início dessa
  > linha para ela não ser carregada!"
- **Semântica do `#`:** no FLA, linha `#Chave = valor` é **setting
  desativado** (o oficial vem com ~3/4 das chaves assim; `;` sim é doc).
  Política v2, igual aos tutoriais:
  - **Chaves do OLA — `Vehicle Models` e `Weapon Models`:** o sync **nunca
    toca**. Devem ficar `#`-desativadas. Se o jogador as deixou ativas, vira
    `Finding::Warn` ("com OLA, esta linha deve ficar com `#`; ativa, ela
    limita o OLA") — a ferramenta não recoloca o `#` sozinha.
  - **Chaves que o tutorial manda ativar** (lista abaixo): se estão `#Chave`,
    o sync remove **só aquele `#`** e ajusta o valor — é o passo do tutorial
    ("Remova esse “#” da tal linha e só assim altere o valor").
  - **Todas as outras chaves:** nunca tocadas, com ou sem `#`.
  Chaves ausentes são **inseridas na `[SECTION]` correta** (nunca em EOF; bug
  legado `lines.push`).
- Chaves geridas pelo sync (as que os tutoriais mandam editar, menos as do OLA):
  `Apply handling.cfg patch`, `Number of standard lines`, `Number of bike lines`,
  `Number of boat lines`, `Number of flying lines`,
  `Count of killable model IDs`, `Enable vehicle audio loader`,
  `Enable weapon type loader`, `Weapon type loader, number of type IDs`,
  `Object info entries` (armas pedem linha no object.dat → subir este limite),
  `Cargrp cars per group`, `Make paintjobs work for any ID`,
  (`Apply ID limit patch` e `Enable model special feature loader` NÃO são geridas: o
  special feature loader crashou junto do OLA no spike). `VehicleStructs` não é gerida: o OLA ajusta sozinho.
- **`Count of killable model IDs` NÃO é do OLA:** os tutoriais mandam editá-la
  mesmo com OLA (≈ 12600 para IDs até 12599). O sync sempre a gerencia.
- Algoritmo **por tipo de chave** (são grandezas diferentes):
  - chave de **teto de ID** (`Count of killable model IDs`):
    `limit = máx(id_usado) + headroom(16)` — é um teto absoluto, não largura
    de faixa (ex.: ID 12500 → ≥ 12516 → 12550). Sem valores extremos: o
    tutorial avisa que isso gasta RAM;
  - chaves de **contagem** (`Number of standard/bike/boat/flying lines`,
    `Object info entries`, `Cargrp cars per group`,
    `Weapon type loader, number of type IDs`):
    `limit = quantidade_original + mods_AWR_daquele_tipo + headroom(16)`
    (para type IDs: `máx(type_id) + 1 + headroom`, pois é tamanho de array),
    **nunca abaixo do piso do tutorial**: standard lines 300, bike lines 20,
    type IDs 80;
  - flags (`Apply …`, `Enable …`, `Make …`): valor `1`.
  Valores numéricos arredondados **para cima** a múltiplo de 50, **nunca abaixo
  do que já está instalado** (monótono crescente; redução só via `cleanup`).
- Toda injeção/remoção chama `sync()` no mesmo ciclo (o inventário é a fonte).
- Detecção cobre os 3 modos: modloader + cache `.fla_cache`, e **instalação na
  raiz do GTA** (trabalho já iniciado na árvore suja atual — porta para a v2).
- **Bugs legados a não portar** (descobertos no `apply_golden_config` v1):
  (1) `content.lines()` + `writeln!` converte o arquivo inteiro CRLF→LF
  (diff massivo em 717 linhas); (2) `trim_start_matches('#')` ao reescrever
  **reativa silenciosamente todas as settings desativadas**, inclusive
  `Vehicle Models`/`Weapon Models`, que com OLA têm de ficar com `#`. Na v2 só
  as chaves geridas da lista acima perdem o `#`. Os golden tests da §9 falham
  para os dois se regressarem.

### 4.3 ModLoader File Injector

- Pastas auto-contidas, isoladas por entidade:
  ```text
  modloader/
  ├── awr-cars/{ID}-{short}/        # ex.: 12500-gtr34/
  │   ├── {short}.dff              # modelo (copiado, nunca movido da origem)
  │   ├── {short}.txd              # textura
  │   ├── {short}.fxt              # texto do jogo (nome no HUD)
  │   ├── linhas.txt               # linhas handling (padrão + especial !/%/$, nome
  │   │                            # em MAIÚSCULAS), vehicles.ide e carcols.dat, via
  │   │                            # mecanismo ".txt" do ModLoader (o tutorial: um .txt
  │   │                            # de qualquer nome com as linhas é detectado).
  │   │                            # NUNCA misturar com arquivos data do mesmo tipo
  │   │                            # (regra oficial: handling.cfg arquivo + linhas em
  │   │                            # .txt não coexistem no ModLoader)
  │   ├── audio_ref.txt ou cfg     # áudio clonado/silenciado (via FLA audio loader;
  │   │                            # ao tocar gtasa_vehicleAudioSettings.cfg global:
  │   │                            # inserir SEMPRE acima do sentinela ";the end")
  │   └── meta.json                # estado da ferramenta (schema §7)
  └── awr-weapons/{ID}-{short}/
      ├── {short}.dff / {short}.fxt
      ├── {short}.txd              # textura; o ícone é a textura "{short}icon"
      │                            # DENTRO deste .txd (sniper/binóculo: também
      │                            # "{short}crosshair"), não um arquivo separado
      ├── Loader.txt               # linha "IDE DATA\MAPSWeapons.ide" (carrega o .ide
      │                            # novo) + linhas weapon.dat clonadas da referência
      │                            # ($ arma de fogo / £ branca; ID 348 → model ID) +
      │                            # linha object.dat (tutorial: "aparentemente
      │                            # necessário", evita crash). O weapon.dat do jogo
      │                            # nunca é editado.
      ├── Weapons.ide              # bloco "weap"/"end" com a linha da arma
      │                            # (ex.: "12400, revolver, revolver, python, 1, 30, 0")
      └── meta.json
  ```
  - **Tipo de arma ≠ tipo de veículo no `weapon.dat`:** `$ ` = arma de fogo,
    `£ ` = arma branca. O `£` é byte **0xA3 em CP-1252** (confirmado no `weapon.dat` original; 2 bytes `C2 A3` se UTF-8) e o
    tutorial avisa que "pode bugar conforme a codificação" → entra como fixture
    envenenada obrigatória do parser de armas.
  - cargrp: uma cópia de `cargrp.dat` no perfil AWR (método v1, `add-transito/`;
    método do tutorial), com o short name acrescentado ao fim da linha
    `POPCYCLE_GROUP_*` da classe, após vírgula. O tutorial avisa que só funciona
    se **nenhum outro `cargrp.dat`** estiver no modloader → o checker acusa (§4.4).
    `Cargrp cars per group` é gerida pelo sync; `VehicleStructs` o OLA ajusta.
  - Áudio e `gtasa_weapon_config.dat` ficam na pasta `data` do fastman92, fora
    do modloader: são editados no lugar (com `.bak`), como mandam os tutoriais.
- **Ativar/Desativar** = renomear a pasta prefixando/removendo **`. `** (ponto +
  espaço) — método OFICIAL documentado no tutorial do ModLoader ("ponto e espaço
  no início do nome da pasta → não será carregada"). Excluir = deletar a pasta.
  (Corrigido em 2026-09-08 contra a fonte: `!` NÃO é convenção ModLoader.)
  **Desativar também retira as linhas do mod dos arquivos globais** (`cargrp.dat`
  compartilhado, `gtasa_vehicleAudioSettings.cfg`, `gtasa_weapon_config.dat`);
  ativar as **regenera a partir do `meta.json`**. Sem isso o tráfego/áudio citaria
  um modelo não carregado → crash. Desativar → reativar deve deixar os globais
  byte-idênticos ao estado anterior (teste obrigatório, §9).
- Escritas: arquivos de origem são copiados, nunca movidos (sem hash de
  duplicados — fora do MVP).
- Nome de pasta sempre `{id}-{short}` (ex.: `12500-gtr34`); `validators::short_name()` único
  no repo (já existe no legado). **Regra oficial: `{short}` ≤ 7 caracteres**
  (limite do tutorial para nome de modelo/handling; validador v2 bloqueia > 7).

### 4.4 Sanity & Health Checker

Executa na inicialização e sob demanda (`doctor`). Produz `Vec<Finding>` com
severidade (`Info | Warn | Error | Fixable`):

| Checagem | Comportamento |
| -------- | ------------- |
| Instalação GTA detectada (portable/standalone/raiz) | Error se ausente |
| ModLoader presente + perfil ativo legível | Error |
| FLA `.asi` + `.ini` localizados | Error |
| **OLA presente (`III.VC.SA.LimitAdjuster.asi` / `OpenLimitAdjuster.asi`)** | **Obrigatório (decisão 2026-10-23).** Ausente → `Error` e injeção bloqueada, com orientação para instalar. A ferramenta nunca renomeia/remove o `.asi` do OLA |
| `Vehicle Models` / `Weapon Models` ativas (sem `#`) no `.ini` do FLA | Warn: com OLA devem ficar com `#` (citação do tutorial, §4.2); o jogador corrige |
| Outro `cargrp.dat` no modloader além da cópia AWR | Warn: o tutorial diz que a cópia só funciona sem outro `cargrp.dat` instalado |
| **Mods incompatíveis com add-without-replace** (Tuning Mod < v1.5, Real Traffic Fix < v1.2, mods antigos que controlam handling por modelo) | Warn com versões mínimas do tutorial oficial |
| Permissões de escrita na pasta do jogo / modloader | Error com caminho exato |
| Pastas `{id}-*` sem `meta.json` (mods de terceiros) | Info → inventário as lista como `estrangeiro`, **não toca** |
| IDs duplicados entre pastas, linhas órfãs em cargrp/áudio/FLA | Fixable via cleanup |
| Jogo em execução (arquivos travados) | Warn |
| Limites FLA insuficientes para o inventário atual | Fixable → sync |

---

## 5. Escopo MVP (enxuto)

| Módulo | Escopo de execução |
| ------ | ------------------ |
| **Importador Rápido** | Drag & drop de `.dff` + `.txd` (arma: o `.txd` precisa da textura `{short}icon`; a ferramenta avisa se faltar) e `.fxt`/txt opcional; formulário mínimo: Nome e Veículo/Arma de Referência (dropdown com busca). **Não há campo Tipo:** o tipo (Carro/Moto/Avião/Barco/Heli; arma: Pistol/Melee/etc.) vem de `references.rs` a partir da referência — um campo separado permitiria Tipo=Carro com Ref=PCJ600 (linha `!` + IDE `car` → crash). Botão **Injetar no Jogo**. |
| **Gerenciador de Estado** | Lista única dos instalados: tipo, ID, nome, chave de tráfego, origem (AWR/estrangeiro) e ação direta por linha: **Ativar / Desativar / Excluir**. |
| **Auto-Patching Engine** | Clonagem transparente da linha de handling da referência (prefixos `!` moto, `%` barco, `$` avião/heli respeitados), geração da linha IDE, entrada cargrp (faixa de tráfego por classe), e alocação/clone de áudio no FLA. Usuário não vê nada disso; só "Injetar". |
| **Limpeza & Restauração** | Excluir mod = apagar pasta + varrer resíduos (linhas órfãs em cargrp/áudio/weapon_config), depois **re-sync do `.ini` do FLA**. Botão global "Reparar instalação" roda a mesma pipeline sem apagar nada. |

Entrou na Fase 1 (confirmado no jogo em 2026-10-23): peças de tuning (`*.dff` renomeadas
`{peça}_{short}`, IDs próprios, linha `carmods` e IDE de cada peça no `linhas.txt`),
paintjobs `{ref}N.txd` -> `{short}N.txd` e `.ivfc` do ImVehFt com `vehicle_id` trocado. Exige
ImVehFt/VehFuncs (rodas, luzes) no jogo. O GXT original não tem nome por peça: sem `.fxt` de peça.

Fora do MVP (roadmap futuro): editor avançado de
handling campo-a-campo, perfis múltiplos de modloader, importação de mods de
terceiros (criar meta.json retroativo), tradução EN da UI.

---

## 6. Interface (tela única minimalista)

Janela 1100×720 (mín. 880×560), tema GTA SA: fundo escuro, dourado `#D4AF37` como
accent, cantos retos, densidade alta, sem abas/rotas/navegação lateral.

```text
┌─────────────────────────────┬──────────────────────────────────────┐
│ PAINEL ESQUERDO             │ PAINEL DIREITO — Inventário          │
│  [ Novo ]                   │  ⛟ 12500  GT R34       AUTO  [🗑]    │
│  Arraste .dff + .txd        │  ⛟ 12501  Sanchez MX   AUTO  [🗑]    │
│  Ref:     ( LANDSTAL ▾ )    │  ⛩ 12502  Dodo Fix     OFF   [🗑]    │
│  Nome:    [____________]    │  ⛁ 12400  Desert Eagle AUTO  [🗑]    │
│                             │  …                                   │
│  Trânsito: ( auto ▾ )       │                                      │
│                             │                                      │
│  [ ⚡ INJETAR NO JOGO ]     │                                      │
├─────────────────────────────┴──────────────────────────────────────┤
│ ⛭ GTA SA: /run/media/.../GTA SA   ✓FLA Sincronizado  ⓘ OLA: modo  │
│ ✓ ModLoader   ● 12 veículos ativos · 2 armas   [Log ▾] [Config ▾]  │
└─────────────────────────────────────────────────────────────────────┘
```

- Drop zone aceita arrastar arquivos **ou** pasta do mod; preenchimento automático
  de nome/short a partir do arquivo.
- Dropdown de referência com busca fuzzy (212 veículos / 53 armas de `references.rs`).
- Ação por linha: um clique (toggle liga/desliga; lixeira pede confirmação inline).
- Rodapé = `HealthSummary` do checker, sempre vivo (re-check pós-operação).
- Configuração fica num popover do rodapé (3 campos: caminho GTA, pasta ModLoader,
  faixa de ID). Nada de página de settings.
- Log do `tracing` num drawer do rodapé (substitui console antigo).

---

## 7. Estado & Persistência

### `meta.json` (por pasta de mod — schema versionado)

```json
{
  "v": 1,
  "kind": "vehicle",
  "id": 12500,
  "weapon_type_id": null,
  "name": "GT R34",
  "short": "gtr34",
  "vehicle_type": "car",
  "reference": "LANDSTAL",
  "traffic_group": "common_car",
  "audio": "clone:LANDSTAL",
  "created": "2026-09-08T10:00:00Z",
  "files": { "dff": "gtr34.dff", "txd": "gtr34.txd", "fxt": "gtr34.fxt" }
}
```

- `inventory::scan()` lê todos os `meta.json` do perfil; pasta sem `meta.json` vira
  entrada `foreign` (exibida, nunca alterada).
- **Nada no `meta.json` repete o que o disco já diz:** ativo/desativado vem do
  prefixo `. ` da pasta, estrangeiro = pasta sem `meta.json`. Assim, renomear a
  pasta no Explorer nunca deixa dois estados discordando.
- Config global `AppSettings` (~4 campos) em `directories::config_dir()/sa-awr.json`.
  Fim da persistência "meia-boca" de encoding/backup/sound da v1.

---

## 8. Contrato da API do Core

`awr-app` e `awr-cli` só enxergam isto (todos os erros são `AwrError`; `Result` em
tudo; functions sem I/O global escondido):

```rust
// ── bootstrap ─────────────────────────────────────────────
pub fn detect_install(root_hint: Option<&Path>) -> Result<InstallInfo, AwrError>;
impl InstallInfo { pub fn core(&self) -> Result<AwrCore, AwrError>; }

// ── fachada principal ─────────────────────────────────────
pub struct AwrCore { /* paths + settings, Send+Sync */ }

impl AwrCore {
    // leitura
    pub fn inventory(&self) -> Result<Vec<InstalledMod>, AwrError>;
    pub fn health(&self) -> Vec<Finding>;                       // nunca falha
    pub fn next_ids(&self) -> Result<IdPreview, AwrError>;      // p/ exibir no form

    // escrita (cada arquivo é atômico; o conjunto é recuperável pelo repair(), ver regras abaixo)
    pub fn inject_vehicle(&mut self, req: InjectVehicle<'_>) -> Result<InjectedMod, AwrError>;
    pub fn inject_weapon(&mut self, req: InjectWeapon<'_>) -> Result<InjectedMod, AwrError>;
    // mod identificado pela PASTA (ModKey = caminho vindo de inventory()), nunca pelo
    // ID: IDs duplicados entre pastas existem (§4.4) e remove(id) apagaria a errada
    pub fn set_enabled(&mut self, key: &ModKey, enabled: bool) -> Result<ModState, AwrError>; // rename + tira/repõe linhas globais + sync FLA
    pub fn remove(&mut self, key: &ModKey) -> Result<RemovalReport, AwrError>;   // pasta + resíduos + sync FLA
    pub fn sync_fla(&mut self) -> Result<FlaSyncReport, AwrError>;
    pub fn repair(&mut self) -> Result<RepairReport, AwrError>; // limpa órfãos, resync, mantém estrangeiros
    pub fn apply_fix(&mut self, finding: FindingId) -> Result<(), AwrError>; // ex.: ajustar chave gerida do FLA, limpar órfão (NUNCA tocar o .asi do OLA nem Vehicle/Weapon Models)

    pub fn settings(&self) -> &AppSettings;
    pub fn save_settings(&mut self, s: AppSettings) -> Result<(), AwrError>;
}

// ── tipos de domínio ──────────────────────────────────────
pub struct InjectVehicle<'a> {
    pub name: &'a str,
    pub dff: &'a Path, pub txd: &'a Path,
    pub fxt: Option<&'a Path>,
    pub reference: RefId,                 // validado contra references::VEHICLES; define o VehicleKind
    pub traffic: TrafficChoice,           // Auto | Group(u8) | None
    pub audio: AudioChoice,               // CloneFromRef | Silent
}

pub enum AwrError {
    NotFound(NotFoundKind), Permission { path: PathBuf }, Io { path: PathBuf, src: io::Error },
    Encoding { path: PathBuf }, InvalidName(String), DuplicateShort { short: String },
    IdExhausted { range: IdRange, used: usize }, AlreadyExists { path: PathBuf },
    Fla(FlaError), Parse { path: PathBuf, line: usize, expected: &'static str },
    LockedByGame, ForeignMod { path: PathBuf },
}
```

Regras do contrato:

- **Encoding é centralizado no `fsutil`.** Toda leitura/escrita de arquivo do jogo
  (handling.cfg, vehicles.ide, cargrp.dat, .fxt, FLA .ini) passa por
  `encoding_rs::WINDOWS_1252`. Nunca escrever UTF-8 nesses formatos: isso corrompe
  acentos no HUD do GTA SA. O parser mantém CRLF e terminadores originais.

- Métodos de escrita retornam **relatórios** (`RemovalReport`, `FlaSyncReport`)
  com a lista exata de arquivos tocados → a UI renderiza "o que aconteceu" do
  retorno, nunca adivinha.
- `inject_*` grava primeiro em staging **dentro do mesmo filesystem do alvo**
  (`modloader/.awr-staging/`, nunca `/tmp`), e promove com renomeações atômicas.
  Cross-device rename falha com `EXDEV`; como o staging fica dentro de `modloader/`,
  staging e destino estão sempre no mesmo filesystem (sem fallback `copy + delete`).
- **Ordem fixa de escrita** (arquivos diferentes não têm commit conjunto):
  1. montar a pasta completa no staging;
  2. editar os arquivos globais (cargrp, áudio, weapon_config, FLA `.ini`), cada um
     com `.bak` + tmp + rename;
  3. promover a pasta com um rename (falha com `AlreadyExists` se a pasta de
     destino já existe; nunca sobrescreve).
  Queda entre 2 e 3 deixa só linhas globais órfãs (sem pasta) + staging, que o
  `repair()` já remove. Na remoção, ordem inversa: pasta primeiro, globais depois.
- Golden tests: cada parser tem fixture de entrada + saída esperada byte-a-byte
  (respeitando CRLF e CP-1252 dos arquivos do jogo).

---

## 9. Especificação dos Formatos de Arquivo

### `handling.cfg` (ModLoader merge por linha)

- Colunas: `nome_modelo` + ~34 campos (anim_group, model_id, class, mass, …).
- **O original da Rockstar mistura tabs e espaços e desalinha** — por isso a
  clonagem troca **somente os tokens** (span de byte), preservando os separadores
  da linha de referência. `split_whitespace().join(" ")` no caminho de escrita é
  proibido (achata o alinhamento e gera diff massivo).
- Prefixos de categoria (preservados na clonagem — código legado `tuning.rs` já
  implementa, v2 mantém com testes):
  - sem prefixo → **carro** (padrão)
  - `! ` → **moto** (bike lines)
  - `% ` → **barco** (boat lines)
  - `$ ` → **avião/heli** (plane/heli lines)
- Clonagem: copia a linha da referência e troca **só o nome** (campo 1; na linha
  especial `!`/`%`/`$`, o campo 2). O `handling.cfg` do SA **não tem coluna de ID**
  (o campo 2 da linha padrão é a massa) — o ID novo vai só no IDE. Modificações de
  massa/tração não são MVP.
- Tokenização igual à do jogo: vírgula e espaço/tab são o mesmo separador (o
  `vehicles.ide` original tem linhas sem vírgula, ex. `585,	emperor		emperor,`).

### `vehicles.ide` (linha de veículo)

`ID, nome, short, car|bike|boat|plane|..., MODEL_NAME, TXD_NAME, null, ignore, phys_ctrl_level, ?, ?, ?`
(model_id = mesmo ID handling; `TXD_NAME` = `{short}`). Arquivos collision `.col`
são opcionais: sem `.col` usa a colisão da referência via FLA.

### `cargrp.dat` (tráfego)

**Uma linha por grupo**, na ordem fixa dos grupos: `nome, nome, … \t# POPCYCLE_GROUP_*`
(ou rótulo de gangue/`Boats`/`Cheat*`). Não há blocos `$cargrp … $end`.
Tráfego "Auto" = o clone entra **nos mesmos grupos em que a referência aparece**
(referência sem grupo, ex. trem/avião, fica sem tráfego). Remoção tira só o
`{short}` e seu separador; nunca apaga uma linha (mudaria a ordem dos grupos).

### `fastman92limitAdjuster_GTASA.ini`

Verificada contra o oficial de 717 linhas (fixture):

- **Sem BOM, 100% CRLF, SEM newline final** (716 linhas; a última,
  `#Use a different INI = `, não termina em `\r\n` — o writer preserva isso).
  Chave com espaço duplo antes do `=` (`#Enable weapon type loader  = 0`) →
  comparar o nome da chave sem os espaços das pontas.
- `[SECTION]` × ~30; cada chave é precedida de blocos de doc `; Chave (default)`.
- **`;` = doc; linha `#Chave = valor` = setting desativado** (o oficial vem com
  ~166 desativadas vs 54 ativas). Nunca decodificar isso como "comentário".
- Edição por chave preservando todo o resto (ver §4.2): só o token numérico muda
  (e, nas chaves geridas, o `#` inicial é removido); espaçamento em torno do `=`
  permanece byte-exato; `Vehicle Models`/`Weapon Models` e chaves não geridas saem
  byte-idênticas; chave nova entra na `[SECTION]` certa, nunca em EOF.
- Confirmado: **não há inline comment** após o valor → o token vai até o whitespace
  final / EOL. Ainda assim o parser defende parando em `;`/`#` por segurança.

### Golden tests & fixtures (Fase 0-a)

- Assert **sempre** `assert_eq!(&Vec<u8> resultado, &fs::read("fixtures/x.expected"))`
  — nunca comparar `String` (esconderia mudança de CRLF e de CP-1252).
- Fixtures: `fla_original.ini`, `fla_expected_sync.ini`, `handling_original.cfg`,
  `handling_expected_infernus_cloned.cfg`, **+ fixture envenenada** com acentos
  CP-1252 (`; Acentuação ção é`) num bloco `;` para provar round-trip byte-exato
  de bytes que a ferramenta ignora.
- Testes obrigatórios fora dos parsers:
  - **allocator:** fixture de pastas com uma `. `-desativada → o ID dela não é
    reutilizado; com buraco na faixa → o menor ID livre vence; 12400–12499 nunca
    vai para veículo;
  - **ida e volta:** injetar → desativar (globais sem as linhas do mod) →
    reativar → globais byte-idênticos aos de antes de desativar;
  - **sync FLA:** com um ID 12500, `killable` sai ≥ 12516; nunca diminui;
    `#Vehicle Models` e `#Weapon Models` saem byte-idênticas (continuam com `#`);
    `#Enable weapon type loader = 0` vira `Enable weapon type loader = 1`.
- `harvest.sh` valida com `file`/`xxd` que as fixtures estão CRLF + Windows-1252
  antes do commit. **Trava duradoura:** `.gitattributes` com `*.ini -text`,
  `*.cfg -text`, `*.ide -text`, `*.fxt -text`, `*.expected -text` para o Git
  jamais normalizar as fixtures (não confiar só no script).

### `gtasa_weapon_config.dat` (armas) — arquivo GLOBAL do FLA/data

Conforme o tutorial oficial, a arma nova é definida **editando este arquivo
global** (não há merge por pasta aqui): **copia-se** a linha da referência (a
linha original **nunca é apagada nem alterada** — add without replace) e cria-se
`{type_id} {NOVA_ARMA} ... {ARMA_REFERENCIA_NO_FIM}` (o token final é a arma
cuja balística/sons herdar — ex.: `70 REVOLVER ... DESERT_EAGLE`). As linhas de
exemplo que vêm no arquivo usam prefixo `#` (ignoradas pelo FLA). Casos
especiais: lança-mísseis/explosivos exigem linha de associação (`MEU_RLAUNCHER
ROCKET`) **depois** de definir a arma; silenciador genérico = apendar
`PISTOL_SILENCED`; jetpack/binóculos = `JETPACK_TYPE`/`BINOCULARS_TYPE`.
Portanto `inject_weapon`/`remove` fazem **edição atômica deste arquivo global**
(backup `.bak`, inserção antes do fim, remoção só de linhas cujo short name é
do próprio mod) e o scan usa `collect_used_weapon_types()` (herdado do legado
`ids.rs`) para achar type IDs livres a partir de **70**.

### `.fxt` (texto)

Arquivo `{short}.fxt` dentro da própria pasta: linhas
`{NOME_MODELO} {nome amigável}` — ModLoader faz merge do `fxt` da pasta.

---

## 10. Plano de Migração do Código Atual

> **Atualização 2026-10-23:** o código legado não está disponível. A v2 é escrita
> do zero em `E:\projetos\SAAWR`; a tabela abaixo serve só como lista do que cada
> módulo precisa fazer. Desvios decididos no plano de desenvolvimento:
> - **Referências lidas dos arquivos originais do jogo** (`data\vehicles.ide` +
>   `handling.cfg`; armas: `default.ide` + `weapon.dat`), não de uma tabela escrita
>   à mão em `references.rs`. O tipo do veículo vem da coluna de tipo do IDE.
> - **CLI sem `clap`** (`std::env::args`), com `status | doctor | inject | remove`:
>   `inject/remove` voltam porque o critério da Fase 1 (sem GUI) precisa deles.
> - Instalação de referência (`D:\Games\GTA San Andreas`): FLA e OLA ficam **dentro
>   do modloader** (`modloader\$fastman92 limit adjuster\`, `modloader\Open Limit
>   Adjuster\`). O `fastman92limitAdjuster_GTASA.ini` **não veio na instalação**: o
>   jogador coloca o oficial ao lado do `.asi` (a ferramenta não o gera).
> - `gtasa_vehicleAudioSettings.cfg` termina em `;the end` **sem newline final**.

Reset no mesmo repo: `git branch -M legacy/v1` (inclui a árvore suja atual, que
tem FLA root-detection e `is_ola_installed` em andamento → portar), depois branch
`main` órfão com a estrutura de workspace da §3.

| Módulo legado (src-tauri/src) | LOC | Destino v2 |
| ----------------------------- | --- | ---------- |
| `ids.rs` | 487 | **Portar** p/ `core::ids`; generalizar faixa p/ 12000–19999 + reaproveitamento explícito |
| `fla.rs` | 356 | **Portar com refactor** (inclui diff não commitado): separar parser/writer do `sync()` |
| `tuning.rs` | 443 | **Dividir**: handling/ide/cargrp/audio viram módulos próprios do core |
| `cargrp.rs` | 345 | **Portar** quase 1:1 |
| `audio.rs` | 233 | **Portar** quase 1:1 |
| `weapons.rs` | 426 | **Portar**; extrair weapon_config p/ `fla.rs` |
| `cleanup.rs` | 291 | **Portar** p/ dentro de `remove()`/`repair()` |
| `checker.rs` | 197 | **Portar + expandir**: permissões, jogo aberto, estrangeiros |
| `references.rs` | 274 | **Portar** (tabelas 212/53 + testes) |
| `validators.rs` | 196 | **Portar** |
| `encoding.rs` | 100 | **Portar** → `fsutil` |
| `config.rs` + `types.rs` | 296 | **Reescrever** mínimo (§7) |
| `edit.rs` | 114 | **Absorver** em `inventory` (edição campo-a-campo sai do MVP) |
| `commands.rs` | 228 | **Descartar** (IPC Tauri morto) |
| `gui.rs` | 1107 | **Refactor** → `crates/awr-app` na tela única da §6 |
| `main.rs`/`lib.rs` | 38 | Recriar pelo workspace |
| `src/` (React), `index.html`, `package.json`, `vite.config.ts`, `tsconfig*`, `node_modules`, `src-tauri/Cargo.toml` (tauri), `capabilities/`, `build.rs`, `public/`, `scripts/` JS | — | **Descartar do repo** |
| Docs `.github/`, `CONTRIBUTING`, `SECURITY`, LICENSE | — | **Manter**; CI vira `cargo fmt/clippy/test` + `cargo-dist` (maturix: deletar jobs npm) |

Ordem de port: `fsutil → paths/config → meta/inventory → ids → fla → handling/ide/cargrp/audio
→ injector → weapons → cleanup → checker → app egui → cli`. Cada passo com testes
de fixture antes do próximo (TDD por módulo, sem GUI no caminho).

---

## 11. Roadmap

| Fase | Entrega | Critério de pronto |
| ---- | ------- | ------------------ |
| **0. Fundação** | Workspace, `AwrError`, fsutil atômico, detect_install, meta.json + scan, CI Rust | `awr-cli status` lista inventário de uma instalação real |
| **1. Pipeline Veículo** | ids + fla sync + injector + cleanup de veículos | injetar/remove 3 carros reais num GTA de teste sem tocar arquivo original; `repair()` zera resíduos |
| **2. UI** | Tela única egui (§6), toggle/remove, rodapé health com status do OLA | fluxo completo sem terminal: drop → inject → listar → desativar → excluir |
| **3. Armas** | weapons pipeline + weapon_config + textura `{short}icon` no `.txd` | idem fase 1 para armas; aviso "regerar Free IDs" funcional |
| **4. Release** | cargo-dist, .exe portable, docs/ESCOPO público | instalação em GTA SA "limpo + modloader + FLA" por um beta tester |

---

## 12. Pré-requisitos do Ambiente do Jogador (a ferramenta detecta, não instala)

- GTA SA 1.0 US (Steam/RGL: fazer downgrade antes), ModLoader, fastman92 Limit
  Adjuster (Silent's ASI Loader) **e Open Limit Adjuster — obrigatório**
  (decisão 2026-10-23). O tutorial de carros diz que, instalando via ModLoader,
  `Vehicle Models` só funciona com OLA; os dois tutoriais mandam deixar
  `Vehicle Models`/`Weapon Models` com `#` quando se usa OLA (citação em §4.2).
  Sem OLA o checker dá `Error` e a injeção fica bloqueada. A ferramenta
  **nunca** renomeia/remove o `.asi` do OLA.
- Jogo fechado durante operações (checker detecta processo/arquivos travados).

## 13. Não-Metas

Sem suporte a Android/CS; sem gerenciamento de mods de terceiros alheio ao AWR;
sem edição visual de handling; sem telemetria, update automático ou rede de
qualquer tipo (100% offline).

## 14. Referências oficiais verificadas (fontes da spec)

Facts desta spec ancorados em 2026-09-08 nas fontes primárias; tutoriais de
carros e armas re-verificados em 2026-10-23:

- **ModLoader (LINK/2012), tutorial completo** — https://www.mixmods.com.br/2015/07/tutorial-dicas-tudo-sobre-mod-loader.html
  - Desativar pasta = **ponto + espaço** no início do nome (NÃO `!`).
  - **Regra de fusão:** `.cfg/.ide/.dat` arquivo e linhas em `.txt` do MESMO
    tipo **não coexistem** (arquivo data vence); por isso AWR usa só `.txt`.
  - `merge` de gta.dat etc; áudio Mono PCM 16-bit; `.col` via `COLFILE`.
- **Adicionar CARROS sem substituir** — https://www.mixmods.com.br/2020/02/tutorial-adicionar-carros-sem-substituir/
  - Convenção de ID **12500+** (12000–12499 = Tuning Mod/peças). `killable ≈ 12600`
    para IDs até 12599 (editar mesmo com OLA; valor extremo gasta RAM).
  - `Vehicle Models`: **não editar com OLA**, deixar `#`; via ModLoader só
    funciona com OLA. `VehicleStructs` o OLA ajusta sozinho.
  - Chaves editadas: remover o `#` e só então mudar o valor. Pisos sugeridos:
    `Number of standard lines` ≥ 300, `Number of bike lines` ≈ 20–30.
  - Linhas handling (nome em MAIÚSCULAS; especiais `!`/`$`/`%`), vehicles.ide e
    carcols.dat podem ir num `.txt` de qualquer nome no ModLoader.
  - `;the end` é sentinela do `gtasa_vehicleAudioSettings.cfg` (nada abaixo é lido).
  - cargrp: cópia no ModLoader só funciona sem outro `cargrp.dat` instalado lá.
  - Compatibilidade: Tuning Mod ≥ v1.5, Real Traffic Fix ≥ v1.2, First Person Mod
    não-oficial. Nome curto **≤ 7 letras**.
- **Adicionar ARMAS sem substituir** — https://www.mixmods.com.br/2019/02/tutorial-adicionar-armas-sem-substituir/
  - Reservar **12400–12499** para model ID; type ID a partir de **70** (não usar o
    `-1` automático, para poder rastrear os IDs).
  - `Weapon Models` → **não editar se usa OLA**, deixar `#`. `£` especial: salvar em
    ANSI (encoding).
  - Arma exige `Weapons.ide` (bloco weap/end) + `Loader.txt` (`IDE DATA\MAPS...` e
    as linhas `$`/`£` do weapon.dat — o arquivo do jogo não é editado);
    `gtasa_weapon_config.dat` global: **copiar** a linha da referência, nunca
    apagar; `object.dat` + `Object info entries`; `Enable weapon type loader = 1`.
  - Ícone = textura `{dff}icon` dentro do `.txd` da arma; sniper também `{dff}crosshair`.
- **fastman92 Limit Adjuster** — https://www.mixmods.com.br/2022/01/fastman92-limit-adjuster/
  (chave `#`-prefixada = linha NÃO carregada; `;` = doc).
- **Open Limit Adjuster** — https://www.mixmods.com.br/2022/10/open-limit-adjuster/
  (auto-gerencia limites "de modo ilimitado"; **obrigatório na v2**).
- **ModLoader atual** — https://www.mixmods.com.br/2026/05/modloader/ (v0.3.10,
  "melhorada compatibilidade com ID limit adjusters").
- Lista de IDs livres: https://tuningmodparts.blogspot.com.br/p/ids.html
- Spawners de teste: Djjr Car Spawner; Name Weapon Spawner.
