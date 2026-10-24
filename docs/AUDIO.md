# Áudio de armas: o que existe e o que falta

Resultado do spike (pesquisa em documentação pública; **nada foi testado no jogo**).

## Fase A (feita): herdar som e comportamento de outra arma

No `gtasa_weapon_config.dat` do FLA, a última coluna (`parentWeaponType`) diz de qual arma a nova herda. O som de tiro do jogo é por tipo de arma, então trocar o pai troca o som. Mas o pai também define animação e tipo de tiro: uma pistola com pai de fuzil pode ficar com a pose errada. Por isso:

- Interface: "Som e comportamento iguais a" (padrão: a referência). CLI: `--sound-like <ARMA>`.
- Vale só para armas sem comportamento especial (silenciador, jetpack e binóculos têm pai fixo).
- A arma precisa existir na lista lida do jogo; nome desconhecido dá erro.
- A escolha fica gravada em `weapon_config` do `meta.json`, como o resto da linha.

## Fase B (adiada): sons próprios

O áudio do jogo fica em `audio/SFX` (bancos `.raw` ligados por `BankLkup.dat`, `BankSlot.dat`, `PakFiles.dat`). Editar ou acrescentar bancos é um formato binário sem suporte no ModLoader nem no FLA que eu tenha encontrado, e mexer nele **substitui** arquivos, contra o princípio do projeto.

O caminho viável é o plugin **EarShot** (ASI, versões OpenAL e miniaudio):

- Cada arma tem uma pasta com `{nome no weapon.dat}.earshot` e WAVs com nomes fixos (`shoot.wav`, `reload.wav`, `after.wav`, `distant.wav`, `dryfire.wav`; variações com número no fim, `shoot0.wav`).
- O `.earshot` guarda só atenuação e pitch (`shoot.maxDist = 120.0`, `shoot.pitch = 1.0`...).
- O nome que ele procura é o do `weapon.dat`, o mesmo que o AWR grava em maiúsculas a partir do nome curto. Isso casa com armas novas.
- O `.asi`, o `.ini` e a pasta `EarShot` podem ficar no ModLoader; `OpenAL32.dll` e `alsoft.ini` precisam ficar na raiz do jogo.
- O EarShot não está instalado na máquina de desenvolvimento.

### Perguntas em aberto (testar no jogo antes de implementar)

1. O EarShot lê a pasta de sons de dentro de `modloader/awr-weapons/{id}-{nome}/EarShot/`, ou só da pasta `EarShot` ao lado do `.asi`? Se só da segunda, o AWR teria de gerenciar uma pasta própria dentro dela, como já faz com `awr-transito`.
2. O nome da arma é comparado sem diferenciar maiúsculas? E funciona com os tipos novos do FLA (70+)? A documentação não diz.
3. Um `.earshot` vazio é válido?

### Plano se as respostas forem boas

Opção "Sons (pasta com WAVs)" no import de arma: copiar os WAVs com os nomes padrão, gerar `{SHORT}.earshot` com os valores padrão, ligar/desligar junto com a arma (as pastas de mods desligados saem do caminho) e remover sem resíduo. Avisar na interface se o EarShot não for encontrado.
