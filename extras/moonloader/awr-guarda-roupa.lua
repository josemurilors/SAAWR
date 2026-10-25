-- SA-AWR Guarda-roupa (MoonLoader)
-- Veste no CJ as roupas instaladas pelo SA-AWR em modloader/awr-roupas, sem passar pelas
-- lojas: nada vai para o shopping.dat, entao nao ha limite de 560 itens nem de 12 por loja.
--
-- Uso: a pe, digite o cheat ROUPA. Setas esquerda/direita trocam (a roupa ja aparece no CJ),
-- seta para cima veste, seta para baixo cancela e volta ao que estava.
-- O jogo salva a roupa que o CJ esta usando; ela continua valendo enquanto os arquivos
-- estiverem instalados.

script_name('AWR Guarda-roupa')
script_author('SA-AWR')
script_description('Veste as roupas do SA-AWR (modloader/awr-roupas) sem as lojas')
script_version('1.0')

require 'lib.moonloader'
local vkeys = require 'vkeys'

local PARTS = {
    [0] = 'torso', [1] = 'cabelo', [2] = 'calca', [3] = 'sapato',
    [13] = 'colar', [14] = 'relogio', [15] = 'oculos', [16] = 'chapeu',
}

-- Each installed item, read from the meta.json the SA-AWR writes in its folder.
local function read_items()
    local items = {}
    local root = getGameDirectory() .. '\\modloader\\awr-roupas\\'
    local handle, name = findFirstFile(root .. '*')
    while handle and name do
        -- ". " in front: switched off in the SA-AWR (ModLoader skips it)
        if name ~= '.' and name ~= '..' and name:sub(1, 2) ~= '. ' then
            local f = io.open(root .. name .. '\\meta.json', 'rb')
            if f then
                local s = f:read('*a')
                f:close()
                local tex = s:match('"texture"%s*:%s*"([^"]+)"')
                local model = s:match('"model"%s*:%s*"([^"]+)"')
                local part = tonumber(s:match('"body_part"%s*:%s*(%d+)'))
                local label = s:match('"name"%s*:%s*"([^"]*)"') or tex
                if tex and model and part then
                    -- the game font has no accents
                    items[#items + 1] = { tex = tex, model = model, part = part, label = label:gsub('[^%w%s%-%.]', '') }
                end
            end
        end
        name = findNextFile(handle)
    end
    if handle then findClose(handle) end
    table.sort(items, function(a, b) return a.label < b.label end)
    return items
end

local function wear(it)
    givePlayerClothesOutsideShop(PLAYER_HANDLE, it.tex, it.model, it.part)
    buildPlayerModel(PLAYER_HANDLE)
end

local function show(items, i)
    local it = items[i]
    printStringNow(string.format('~y~%d/%d~w~ %s (%s)~n~~<~ ~>~ trocar   ~u~ vestir   ~d~ cancelar',
        i, #items, it.label, PARTS[it.part] or it.part), 60000)
end

local function browse()
    local items = read_items()
    if #items == 0 then
        printStringNow('Nenhuma roupa do SA-AWR instalada', 3000)
        return
    end
    -- the clothes worn now, to go back to on cancel or before each preview
    storeClothesState()
    local i = 1
    wear(items[i])
    show(items, i)
    while true do
        wait(0)
        local step = (isKeyJustPressed(vkeys.VK_RIGHT) and 1) or (isKeyJustPressed(vkeys.VK_LEFT) and -1) or 0
        if step ~= 0 then
            i = (i - 1 + step) % #items + 1
            restoreClothesState()
            storeClothesState()
            wear(items[i])
            show(items, i)
        elseif isKeyJustPressed(vkeys.VK_UP) then
            -- keep it: store and restore the current look so the saved state is cleared
            storeClothesState()
            restoreClothesState()
            buildPlayerModel(PLAYER_HANDLE)
            printStringNow('Vestido: ' .. items[i].label, 2000)
            return
        elseif isKeyJustPressed(vkeys.VK_DOWN) or not isCharOnFoot(PLAYER_PED) then
            restoreClothesState()
            buildPlayerModel(PLAYER_HANDLE)
            clearPrints()
            return
        end
    end
end

function main()
    while not isPlayerPlaying(PLAYER_HANDLE) do wait(100) end
    while true do
        wait(0)
        if testCheat('ROUPA') and isCharOnFoot(PLAYER_PED) then
            browse()
        end
    end
end
