// The Portuguese text is the key: `t("Importar")` returns it as is in Portuguese and the English
// entry below in English; a missing entry falls back to Portuguese. `{n}` marks a variable.
// Messages that come from the Rust side (errors, findings, traffic group labels) stay in Portuguese.

export type Lang = "pt" | "en";

const KEY = "awr-lang";

function initial(): Lang {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "pt" || saved === "en") return saved;
  } catch {
    // storage can be blocked: just use the browser language
  }
  return navigator.language.toLowerCase().startsWith("pt") ? "pt" : "en";
}

let lang: Lang = initial();

export const getLang = () => lang;

export function setLang(l: Lang) {
  lang = l;
  try {
    localStorage.setItem(KEY, l);
  } catch {
    // not persisted, still applied
  }
}

export function t(pt: string, vars?: Record<string, string | number>): string {
  let s = lang === "en" ? (EN[pt] ?? pt) : pt;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(String(v));
  return s;
}

const EN: Record<string, string> = {
  // app
  "Nada reconhecido: precisa de uma pasta com .dff e .txd.": "Nothing recognized: it needs a folder with a .dff and a .txd.",
  "{n} item(ns) na fila": "{n} item(s) in the queue",
  "GTA SA aberto": "GTA SA opened",
  "{n} injetado(s) no jogo": "{n} injected into the game",
  "{name}: alterações salvas": "{name}: changes saved",
  "desligado": "turned off",
  "ligado": "turned on",
  "{name} removido": "{name} removed",
  "Instalação reparada": "Installation repaired",
  "Apagar TODOS os mods do AWR e restaurar os arquivos do jogo para os originais (.bak)?":
    "Delete ALL AWR mods and restore the game files to the originals (.bak)?",
  "Instalação restaurada": "Installation restored",
  "Copiado: {text}": "Copied: {text}",
  "pronto": "ready",
  "pré-visualização no navegador · dados de exemplo": "browser preview · sample data",
  "Log": "Log",
  "sem mensagens": "no messages",
  "Solte para adicionar à fila": "Drop to add to the queue",
  // top bar
  "SA-AWR: adicionar sem substituir": "SA-AWR: add without replacing",
  "Importar": "Import",
  "Instalados": "Installed",
  "Escolher o GTA SA": "Choose GTA SA",
  "Onde está o GTA San Andreas?": "Where is GTA San Andreas?",
  "Escolher a pasta": "Choose the folder",
  "Pasta do GTA San Andreas": "GTA San Andreas folder",
  "Abrir": "Open",
  "ok": "ok",
  "falta": "missing",
  "ERRO": "ERROR",
  "aviso": "warning",
  "Reparar instalação": "Repair installation",
  "Restaurar originais": "Restore originals",
  "Idioma": "Language",
  // import
  "Pasta do mod (ou de vários)": "Mod folder (or several)",
  "Modelo e textura": "Model and texture",
  "Solte a pasta de um mod, uma pasta com vários, ou os arquivos .dff e .txd. O nome vem da pasta e a referência vem do nome do .dff.":
    "Drop a mod folder, a folder with several, or the .dff and .txd files. The name comes from the folder and the reference from the .dff name.",
  "Solte mais mods aqui": "Drop more mods here",
  "Arraste a pasta do mod para cá": "Drag the mod folder here",
  "Funciona com a pasta de um mod, com uma pasta de vários, ou com .dff + .txd soltos.":
    "Works with one mod folder, a folder of several, or loose .dff + .txd files.",
  "Escolher pasta": "Choose folder",
  "Escolher arquivos": "Choose files",
  "Fila": "Queue",
  "{n} item(ns) · {ready} pronto(s) para injetar": "{n} item(s) · {ready} ready to inject",
  "Limpar fila": "Clear queue",
  "Injetando…": "Injecting…",
  "Injetar {n} no jogo": "Inject {n} into the game",
  "Injetar este item": "Inject this item",
  "Nome no jogo": "In-game name",
  "Nome curto": "Short name",
  "Tirar da fila": "Remove from queue",
  "Tipo": "Type",
  "Veículo": "Vehicle",
  "Arma": "Weapon",
  "Baseado em": "Based on",
  "escolha…": "choose…",
  "Som e comportamento iguais a": "Sound and behavior same as",
  "a referência": "the reference",
  "Comportamento especial": "Special behavior",
  "Aparece no tráfego": "Shows up in traffic",
  "como o veículo de referência": "like the reference vehicle",
  "em nenhum grupo": "in no group",
  "Normal": "Normal",
  "Pistola com silenciador": "Silenced pistol",
  "Herda o comportamento da PISTOL_SILENCED.": "Inherits PISTOL_SILENCED's behavior.",
  "Jetpack": "Jetpack",
  "Clona o JETPACK (a referência muda sozinha).": "Clones JETPACK (the reference changes by itself).",
  "Binóculos (experimental)": "Binoculars (experimental)",
  "Usa NIGHTVISION como base e liga a visão em primeira pessoa. Ainda não foi testado no jogo.":
    "Uses NIGHTVISION as the base and turns on first-person view. Not tested in the game yet.",
  "Usar o handling do pacote": "Use the pack's handling",
  "Usar as linhas de arma do pacote": "Use the pack's weapon lines",
  "Copiar scripts CLEO": "Copy CLEO scripts",
  "Clonar o som do motor": "Clone the engine sound",
  "o ícone do HUD é renomeado sozinho": "the HUD icon is renamed automatically",
  "peças de tuning": "tuning parts",
  "paintjobs": "paintjobs",
  "cores IVF": "IVF colors",
  "só modelo e textura": "model and texture only",
  // installed
  "Clique em um item para editar. O interruptor liga e desliga sem apagar nada; mods de terceiros ficam esmaecidos e não são alterados.":
    "Click an item to edit it. The switch turns it on and off without deleting anything; third-party mods are dimmed and left alone.",
  "Buscar pelo nome": "Search by name",
  "Todos": "All",
  "Nada com esse filtro.": "Nothing matches this filter.",
  "Nada instalado ainda. Vá em Importar e solte a pasta de um mod.": "Nothing installed yet. Go to Import and drop a mod folder.",
  "mod de terceiro · a ferramenta não mexe": "third-party mod · the tool leaves it alone",
  "tuning {n}": "tuning {n}",
  "paintjobs {n}": "paintjobs {n}",
  "handling do pacote": "pack handling",
  "baseado em {ref}": "based on {ref}",
  "Desligar": "Turn off",
  "Ligar": "Turn on",
  "Editar {name}": "Edit {name}",
  "Fechar": "Close",
  "arma · {cls}": "weapon · {cls}",
  "carro": "car",
  "{kind} · baseado em {ref} · arquivo {short}": "{kind} · based on {ref} · file {short}",
  " · desligado": " · turned off",
  "Nome da arma": "Weapon name",
  "Em trainers e scripts use o tipo": "In trainers and scripts use the type",
  ", não o ID do modelo ({id}).": ", not the model ID ({id}).",
  "Copiar o tipo": "Copy the type",
  "Salvando…": "Saving…",
  "Salvar alterações": "Save changes",
  "Desfazer": "Undo",
  "Peças, paintjobs e o modelo não mudam aqui. Para trocar o modelo, exclua e importe de novo.":
    "Parts, paintjobs and the model do not change here. To change the model, delete it and import again.",
  "Excluir de vez": "Delete for good",
  "Manter": "Keep",
  "Excluir a arma": "Delete the weapon",
  "Excluir o veículo": "Delete the vehicle",
  // shop
  "Pickups no mapa (x y z [munição], um por linha)": "Pickups on the map (x y z [ammo], one per line)",
  "Precisa do Create Weapon Pickups instalado.": "Needs Create Weapon Pickups installed.",
  "Vender na Ammu-Nation": "Sell in the Ammu-Nation",
  "Precisa do Custom Ammu-Nation Mod instalado.": "Needs the Custom Ammu-Nation Mod installed.",
  "Categoria": "Category",
  "Desbloqueio": "Unlock",
  "Preço": "Price",
  "Munição": "Ammo",
  "Pistolas": "Pistols",
  "Micro SMGs": "Micro SMGs",
  "Escopetas": "Shotguns",
  "Arremesso": "Thrown",
  "Colete": "Armor",
  "SMGs": "SMGs",
  "Rifles": "Rifles",
  "Assalto": "Assault",
  "Pesadas": "Heavy",
  "Especiais": "Special",
  "Brancas": "Melee",
  "Desde o início": "From the start",
  "Após Robbing Uncle Sam": "After Robbing Uncle Sam",
  "Após Just Business": "After Just Business",
  "Após House Party": "After House Party",
  "Após Made In Heaven": "After Made In Heaven",
  "Após Body Harvest": "After Body Harvest",
  "Após Pier 69": "After Pier 69",
  "Após Mountain Cloud Boys": "After Mountain Cloud Boys",
  "Após Yay Ka-Boom-Boom": "After Yay Ka-Boom-Boom",
  "Após Black Project": "After Black Project",
  "Após You've had your Chips": "After You've had your Chips",
  // plate and HUD preview
  "tipo": "type",
  "Tipo da arma: é este número que vai em trainers e scripts": "Weapon type: this is the number trainers and scripts use",
  "ID do modelo": "Model ID",
  "No jogo aparece: ": "In the game it shows: ",
  "(vazio)": "(empty)",
  " · acentos, símbolos ou tamanho foram ajustados para o jogo": " · accents, symbols or length were adjusted for the game",
  // families and weapon classes
  "Carros": "Cars",
  "Motos": "Bikes",
  "Barcos": "Boats",
  "Aviões": "Aircraft",
  "Armas": "Weapons",
  "corpo a corpo": "melee",
  "pistola": "pistol",
  "submetralhadora": "submachine gun",
  "escopeta": "shotgun",
  "fuzil": "rifle",
  "sniper": "sniper",
  "lançador": "launcher",
  "arremesso": "thrown",
  "arma pesada": "heavy weapon",
  "spray/extintor": "spray/extinguisher",
  "equipamento": "gear",
};
