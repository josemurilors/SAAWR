// SA-AWR: catalogo de roupas nas lojas e guarda-roupa (script CLEO).
//
// Roupas, cabelos e acessorios instalados pelo SA-AWR NAO entram no shopping.dat do jogo (que
// mostra no maximo 12 itens por parte do corpo, aceita 560 precos e perde linhas quando outro
// mod mexe nele). O app escreve a lista em CLEO\AWR-Roupas\awr-roupas.ini e este script faz a
// loja: um menu com paginas, previa no CJ, compra e guarda-roupa.
//
//   - Dentro de uma loja de roupas ou barbearia: tecla N ou digite LOJA   (catalogo da loja)
//   - Em qualquer lugar, a pe: digite ROUPA                              (guarda-roupa)
//   - Dentro de uma loja: digite AWRCAL e escolha a loja                 (calibrar o interior)
//   - Digite AWRINFO para ver o interior e as coordenadas do CJ
//
// O que o jogador ja comprou fica em CLEO\AWR-Roupas\estado.ini (o app nunca escreve nele).
// Compilar: scripts/build-cleo.sh  (gta3sc, --cs --guesser)

SCRIPT_START
{
SCRIPT_NAME AWRROUP

// 32 variaveis locais no maximo; as strings gastam 2 (8 bytes) ou 4 (16 bytes) cada
LVAR_INT area shop list total page menu sel last_sel i g t1 t2 rows plr
LVAR_FLOAT px py pz fx fy
LVAR_TEXT_LABEL16 tex model
LVAR_TEXT_LABEL gxt sec

area = 0
shop = -1
page = 0

main_loop:
WAIT 0
IF IS_PLAYER_PLAYING 0
    GET_PLAYER_CHAR 0 plr
    IF IS_CHAR_ON_FOOT plr
        IF IS_PLAYER_CONTROL_ON 0
            GOSUB check_triggers
        ENDIF
    ENDIF
ENDIF
GOTO main_loop

// ---------------------------------------------------------------------------------------
// gatilhos: troca de interior, cheats e tecla
// ---------------------------------------------------------------------------------------
check_triggers:
GET_AREA_VISIBLE t2
IF NOT t2 = area
    area = t2
    // o interior mudou: espera o CJ chegar ao lugar antes de descobrir a loja (page conta quadros)
    shop = -2
    page = 0
ENDIF
IF shop = -2
    page++
    IF page > 25
        GOSUB detect_shop
        IF shop >= 0
            GOSUB show_hint
        ENDIF
    ENDIF
ENDIF
IF TEST_CHEAT "ROUPA"
    GOSUB clear_cheat
    list = 9
    GOSUB run_menu
    RETURN
ENDIF
IF TEST_CHEAT "AWRCAL"
    GOSUB clear_cheat
    GOSUB calibrate
    RETURN
ENDIF
IF TEST_CHEAT "AWRINFO"
    GOSUB clear_cheat
    GET_PLAYER_CHAR 0 plr
    GET_CHAR_COORDINATES plr px py pz
    PRINT_FORMATTED_NOW "Area %d  X %.1f  Y %.1f" 6000 area px py
    RETURN
ENDIF
IF shop >= 0
    IF IS_KEY_PRESSED VK_KEY_N
    OR TEST_CHEAT "LOJA"
        GOSUB clear_cheat
        list = shop
        GOSUB run_menu
    ENDIF
ENDIF
RETURN

// zera o texto digitado (CCheat::m_CheatString[0] = 0), para o cheat nao disparar de novo
clear_cheat:
WRITE_MEMORY 0x969110 1 0 FALSE
RETURN

// ---------------------------------------------------------------------------------------
// em qual loja o CJ esta: interior + distancia ate o ponto da loja (o estado.ini, escrito pela
// calibracao, vale mais que o ini do app)
// ---------------------------------------------------------------------------------------
detect_shop:
shop = -1
GET_PLAYER_CHAR 0 plr
GET_CHAR_COORDINATES plr px py pz
i = 0
WHILE i < 9
    STRING_FORMAT sec "Shop%d" i
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" $sec "Set" t1
    IF t1 = 1
        READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" $sec "Area" t1
        READ_FLOAT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" $sec "X" fx
        READ_FLOAT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" $sec "Y" fy
    ELSE
        READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Area" t1
        READ_FLOAT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "X" fx
        READ_FLOAT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Y" fy
    ENDIF
    IF t1 = area
        GET_DISTANCE_BETWEEN_COORDS_2D px py fx fy pz
        IF 70.0 > pz
            shop = i
            i = 9
        ENDIF
    ENDIF
    i++
ENDWHILE
RETURN

show_hint:
STRING_FORMAT sec "List%d" shop
READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Count" t1
IF t1 > 0
    PRINT_HELP AWRSHP
ENDIF
RETURN

// ---------------------------------------------------------------------------------------
// menu: lista `list` (0..8 = loja; 9 = guarda-roupa; 10 = guarda-roupa com tudo)
// ---------------------------------------------------------------------------------------
run_menu:
IF list = 9
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" "Config" "WardrobeAll" t1
    IF t1 = 1
        list = 10
    ENDIF
ENDIF
IF list = 9
    // itens sem loja (de graca) e depois os comprados
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" "List9" "Count" total
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" "OwnedList" "Count" t2
    total += t2
ELSE
    STRING_FORMAT sec "List%d" list
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Count" total
ENDIF
IF 1 > total
    PRINT_STRING_NOW "Nenhum item AWR aqui." 2500
    RETURN
ENDIF
page = 0
last_sel = -1
SET_PLAYER_CONTROL 0 FALSE
STORE_CLOTHES_STATE
PRINT_HELP_FOREVER AWRHLP
GOSUB build_menu

menu_loop:
WAIT 0
GET_PLAYER_CHAR 0 plr
IF NOT IS_CHAR_ON_FOOT plr
    GOTO menu_exit
ENDIF
IF IS_BUTTON_PRESSED PAD1 TRIANGLE
    GOTO menu_exit
ENDIF
GET_MENU_ITEM_SELECTED menu sel
IF NOT sel = last_sel
    last_sel = sel
    GOSUB preview
ENDIF
GET_MENU_ITEM_ACCEPTED menu i
IF i >= 0
    GOSUB accept
    WAIT 250
ENDIF
GOTO menu_loop

menu_exit:
DELETE_MENU menu
RESTORE_CLOTHES_STATE
BUILD_PLAYER_MODEL 0
CLEAR_HELP
SET_PLAYER_CONTROL 0 TRUE
WAIT 300
RETURN

// ---------------------------------------------------------------------------------------
// monta a pagina `page`: ate 11 itens (linhas 0..10) e, se houver mais de uma pagina, a linha
// de "proxima pagina" logo depois do ultimo item (o menu pula linhas vazias)
// ---------------------------------------------------------------------------------------
build_menu:
IF list >= 9
    CREATE_MENU AWRWRD 40.0 130.0 190.0 2 TRUE TRUE 1 menu
ELSE
    CREATE_MENU AWRTIT 40.0 130.0 190.0 2 TRUE TRUE 1 menu
ENDIF
SET_MENU_COLUMN_WIDTH menu 0 190
SET_MENU_COLUMN_WIDTH menu 1 80
SET_MENU_COLUMN_ORIENTATION menu 1 2
SET_MENU_COLUMN menu 0 AWRHD1 DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY
SET_MENU_COLUMN menu 1 AWRHD2 DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY
rows = total
t1 = page
t1 *= 11
rows -= t1
IF rows > 11
    rows = 11
ENDIF
i = 0
WHILE i < rows
    g = page
    g *= 11
    g += i
    GOSUB lookup
    GOSUB fill_row
    i++
ENDWHILE
IF total > 11
    t1 = total
    t1 += 10
    t1 /= 11
    t2 = page
    t2 += 1
    SET_MENU_ITEM_WITH_2_NUMBERS menu 0 rows AWRNXT t2 t1
ENDIF
RETURN

// g = indice global na lista; devolve em g o indice do item (secao [Item<g>] do ini)
lookup:
IF list = 9
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" "List9" "Count" t1
    IF t1 > g
        STRING_FORMAT model "I%d" g
        READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" "List9" $model g
    ELSE
        // comprados: o estado.ini guarda o nome da textura; [Index] a leva ate o item
        g -= t1
        STRING_FORMAT model "T%d" g
        READ_STRING_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" "OwnedList" $model tex
        READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" "Index" $tex g
    ENDIF
ELSE
    STRING_FORMAT sec "List%d" list
    STRING_FORMAT model "I%d" g
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec $model g
ENDIF
RETURN

// linha i do menu para o item g
fill_row:
STRING_FORMAT sec "Item%d" g
READ_STRING_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Gxt" gxt
SET_MENU_ITEM_WITH_NUMBER menu 0 i $gxt 0
IF list >= 9
    SET_MENU_ITEM_WITH_NUMBER menu 1 i AWRFRE 0
ELSE
    READ_STRING_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Tex" tex
    READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" "Owned" $tex t1
    IF t1 = 1
        SET_MENU_ITEM_WITH_NUMBER menu 1 i AWROWN 0
        HIGHLIGHT_MENU_ITEM menu i TRUE
    ELSE
        READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Price" t1
        SET_MENU_ITEM_WITH_NUMBER menu 1 i AWRPRC t1
    ENDIF
ENDIF
RETURN

// ---------------------------------------------------------------------------------------
// a linha selecionada vira previa no CJ (sempre sobre a roupa guardada no abrir/comprar)
// ---------------------------------------------------------------------------------------
preview:
IF sel >= 0
    IF rows > sel
        g = page
        g *= 11
        g += sel
        GOSUB lookup
        STRING_FORMAT sec "Item%d" g
        READ_STRING_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Tex" tex
        READ_STRING_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Model" model
        READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Part" t2
        RESTORE_CLOTHES_STATE
        STORE_CLOTHES_STATE
        GIVE_PLAYER_CLOTHES_OUTSIDE_SHOP 0 $tex $model t2
        BUILD_PLAYER_MODEL 0
    ENDIF
ENDIF
RETURN

// ---------------------------------------------------------------------------------------
// linha `i` aceita: proxima pagina, vestir (gratis ou ja comprado) ou comprar
// ---------------------------------------------------------------------------------------
accept:
IF i = rows
    IF total > 11
        page++
        t1 = total
        t1 += 10
        t1 /= 11
        IF page >= t1
            page = 0
        ENDIF
        DELETE_MENU menu
        GOSUB build_menu
        last_sel = -1
    ENDIF
    RETURN
ENDIF
IF i > rows
    RETURN
ENDIF
g = page
g *= 11
g += i
GOSUB lookup
STRING_FORMAT sec "Item%d" g
READ_STRING_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Tex" tex
IF list >= 9
    STORE_CLOTHES_STATE
    RETURN
ENDIF
READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" "Owned" $tex t1
IF t1 = 1
    STORE_CLOTHES_STATE
    RETURN
ENDIF
READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\awr-roupas.ini" $sec "Price" t1
STORE_SCORE 0 t2
IF t1 > t2
    PRINT_STRING_NOW "~r~Dinheiro insuficiente." 2500
    RETURN
ENDIF
t2 = 0
t2 -= t1
ADD_SCORE 0 t2
WRITE_INT_TO_INI_FILE 1 "CLEO\AWR-Roupas\estado.ini" "Owned" $tex
READ_INT_FROM_INI_FILE "CLEO\AWR-Roupas\estado.ini" "OwnedList" "Count" t2
STRING_FORMAT model "T%d" t2
WRITE_STRING_TO_INI_FILE $tex "CLEO\AWR-Roupas\estado.ini" "OwnedList" $model
t2++
WRITE_INT_TO_INI_FILE t2 "CLEO\AWR-Roupas\estado.ini" "OwnedList" "Count"
SET_MENU_ITEM_WITH_NUMBER menu 1 i AWROWN 0
HIGHLIGHT_MENU_ITEM menu i TRUE
STORE_CLOTHES_STATE
PRINT_STRING_NOW "~g~Item comprado!" 2500
RETURN

// ---------------------------------------------------------------------------------------
// calibracao: o jogador diz em qual loja esta e o script grava o interior e as coordenadas
// ---------------------------------------------------------------------------------------
calibrate:
IF 1 > area
    PRINT_STRING_NOW "Entre em uma loja antes de calibrar." 3000
    RETURN
ENDIF
GET_PLAYER_CHAR 0 plr
GET_CHAR_COORDINATES plr px py pz
SET_PLAYER_CONTROL 0 FALSE
CREATE_MENU AWRCAL 40.0 130.0 220.0 1 TRUE TRUE 1 menu
SET_MENU_COLUMN menu 0 AWRCAL DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY DUMMY
SET_MENU_ITEM_WITH_NUMBER menu 0 0 AWRS0 0
SET_MENU_ITEM_WITH_NUMBER menu 0 1 AWRS1 0
SET_MENU_ITEM_WITH_NUMBER menu 0 2 AWRS2 0
SET_MENU_ITEM_WITH_NUMBER menu 0 3 AWRS3 0
SET_MENU_ITEM_WITH_NUMBER menu 0 4 AWRS4 0
SET_MENU_ITEM_WITH_NUMBER menu 0 5 AWRS5 0
SET_MENU_ITEM_WITH_NUMBER menu 0 6 AWRS6 0
SET_MENU_ITEM_WITH_NUMBER menu 0 7 AWRS7 0
SET_MENU_ITEM_WITH_NUMBER menu 0 8 AWRS8 0
PRINT_HELP_FOREVER AWRHLP

cal_loop:
WAIT 0
IF IS_BUTTON_PRESSED PAD1 TRIANGLE
    GOTO cal_exit
ENDIF
GET_MENU_ITEM_ACCEPTED menu i
IF i >= 0
    STRING_FORMAT sec "Shop%d" i
    WRITE_INT_TO_INI_FILE 1 "CLEO\AWR-Roupas\estado.ini" $sec "Set"
    WRITE_INT_TO_INI_FILE area "CLEO\AWR-Roupas\estado.ini" $sec "Area"
    WRITE_FLOAT_TO_INI_FILE px "CLEO\AWR-Roupas\estado.ini" $sec "X"
    WRITE_FLOAT_TO_INI_FILE py "CLEO\AWR-Roupas\estado.ini" $sec "Y"
    PRINT_STRING_NOW "~g~Loja calibrada." 3000
    GOTO cal_exit
ENDIF
GOTO cal_loop

cal_exit:
DELETE_MENU menu
CLEAR_HELP
SET_PLAYER_CONTROL 0 TRUE
shop = -2
page = 0
WAIT 300
RETURN
}
SCRIPT_END
