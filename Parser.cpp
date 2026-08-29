/**
 * =================================================================================
 * PARSER.CPP — IMPLEMENTAÇÃO DA FSM PARA BYTE STREAM (Bythos Protocol V1)
 * =================================================================================
 *
 * AUTOR:      ShegaPT
 * DATA:       2026-04-17
 * VERSÃO:     1.0.0
 *
 * =================================================================================
 * DESCRIÇÃO
 * =================================================================================
 *
 * Implementação completa da Máquina de Estados Finita (FSM) para
 * reconstrução de mensagens TLV a partir de um stream contínuo de bytes
 * recebidos via UART. Inclui proteção contra timeout, buffer overflow
 * e validação estrutural completa com CRC8.
 *
 * =================================================================================
 * NOTAS DE IMPLEMENTAÇÃO
 * =================================================================================
 *
 * 1. DETERMINISMO: A FSM não utiliza recursão nem alocação dinâmica.
 *    Cada chamada de feed() processa exatamente um byte e retorna.
 *    O tempo de processamento é O(1) por byte.
 *
 * 2. TOLERÂNCIA A ERROS: Qualquer invalidade (byte inesperado, timeout,
 *    overflow, CRC inválido) resulta em reset automático para WAIT_START.
 *    O parser fica imediatamente pronto para a próxima mensagem.
 *
 * 3. TIMEOUT: Utiliza esp_timer_get_time() no ESP32 (microssegundos, alta
 *    precisão) ou millis()*1000 noutras plataformas (Arduino, RP2040).
 *    O timeout é verificado antes de processar cada byte (exceto em WAIT_START).
 *
 * 4. VALIDAÇÃO CRC8: A validação é feita no estado WAIT_CHECKSUM após
 *    receber todos os bytes. Se o CRC não corresponder, a mensagem
 *    é descartada e o parser reinicia.
 *
 * =================================================================================
 */

#include "Parser.h"
#include <string.h>

/* =================================================================================
 * DEBUG CONFIGURÁVEL
 * =================================================================================
 *
 * O prefixo "[Parser]" permite filtragem no monitor série.
 * Manter ativo em produção é seguro — o debug apenas imprime para
 * Serial/consola e nunca afeta o canal de comunicação.
 */

#define PARSER_DEBUG

#ifdef PARSER_DEBUG
    #ifdef ARDUINO
        #include <Arduino.h>
        #define DEBUG_PRINT(fmt, ...) Serial.printf("[Parser] " fmt, ##__VA_ARGS__)
    #else
        #include <stdio.h>
        #define DEBUG_PRINT(fmt, ...) printf("[Parser] " fmt, ##__VA_ARGS__)
    #endif
#else
    #define DEBUG_PRINT(fmt, ...) ((void)0)
#endif

/* =================================================================================
 * CONSTRUTOR — INICIALIZAÇÃO DO PARSER
 * =================================================================================
 *
 * Coloca o parser no estado inicial (WAIT_START) e inicializa todas
 * as variáveis de estado com valores seguros. O timeout é definido
 * para 100ms por defeito — suficiente para UART a 115200 baud.
 *
 * O construtor NÃO inicializa o rawBuffer (memset), porque o parser
 * apenas escreve e lê posições válidas controladas por rawOffset.
 */

Parser::Parser() {
    /* Inicializar variáveis de estado para valores seguros */
    rawOffset         = 0;
    state             = PARSER_WAIT_START;
    expectedTLVs      = 0;
    currentTLV        = 0;
    tlvBytesReceived  = 0;
    currentTLVLen     = 0;
    messageReady      = false;
    lastError         = PARSER_OK;
    successCount      = 0;
    errorCount        = 0;
    debugEnabled      = false;

    /* Timeout padrão: 100ms entre bytes — adequado para UART 115200 */
    maxFrameGapMicros = 100000;

    /* Registar timestamp inicial para timeout */
    updateTimestamp();
}

/* =================================================================================
 * RESET — REPORÇÃO DO PARSER AO ESTADO INICIAL
 * =================================================================================
 *
 * Reinicia completamente o parser para aceitar uma nova mensagem.
 * Limpa buffers (por segurança), repõe a FSM em WAIT_START e
 * mantém as estatísticas (successCount/errorCount) intactas.
 *
 * As estatísticas são preservadas porque são úteis para monitorização
 * de longo prazo — permitem detectar padrões de erro.
 */

void Parser::reset() {
    /* Limpar buffer de receção para evitar dados residuais */
    memset(rawBuffer, 0, sizeof(rawBuffer));

    /* Reiniciar todas as variáveis de estado da FSM */
    rawOffset         = 0;
    state             = PARSER_WAIT_START;
    expectedTLVs      = 0;
    currentTLV        = 0;
    tlvBytesReceived  = 0;
    currentTLVLen     = 0;
    messageReady      = false;
    lastError         = PARSER_OK;

    /* Repor timestamp para evitar timeout falso imediato */
    updateTimestamp();

    DEBUG_PRINT("Parser resetado\n");
}

/* =================================================================================
 * UPDATE TIMESTAMP — REGISTO DE ATIVIDADE
 * =================================================================================
 *
 * Utiliza a melhor fonte de tempo disponível para a plataforma:
 *   • ESP32: esp_timer_get_time() — microssegundos, alta precisão
 *   • Arduino/Outras: millis() * 1000 — precisão de milissegundos
 *
 * Chamada para cada byte recebido para suporte a deteção de timeout.
 */

void Parser::updateTimestamp() {
#ifdef ESP32
    #include "esp_timer.h"
    lastActivityTime = (uint32_t)esp_timer_get_time();
#else
    /* Fallback para plataformas não-ESP32 */
    lastActivityTime = (uint32_t)millis() * 1000;
#endif
}

/* =================================================================================
 * SET ERROR — TRATAMENTO DE ERRO E RESET
 * =================================================================================
 *
 * Quando um erro é detetado, esta função:
 *   1. Regista o código do erro (para diagnóstico)
 *   2. Incrementa o contador de erros (para estatísticas)
 *   3. Limpa a flag de mensagem pronta
 *   4. Chama reset() para reiniciar o parser
 *
 * Isto garante que após qualquer erro, o parser está imediatamente
 * pronto para processar a próxima mensagem.
 */

void Parser::setError(ParserError error) {
    lastError = error;
    errorCount++;
    messageReady = false;

    DEBUG_PRINT("ERRO: %s (código %d) — total de erros: %lu\n",
                parserErrorToString(error), error, errorCount);

    reset();
}

/* =================================================================================
 * IS TIMED OUT — VERIFICAÇÃO DE TIMEOUT
 * =================================================================================
 *
 * Compara o tempo decorrido desde o último byte recebido com o timeout
 * configurado. Se maxFrameGapMicros == 0, o timeout está desativado.
 *
 * Utiliza aritmética modular (diferença entre timestamps) para lidar
 * corretamente com a reversão do contador em plataformas de 32 bits.
 *
 * @return 1 se timeout ocorreu, 0 caso contrário
 */

uint8_t Parser::isTimedOut() const {
    /* Timeout desativado — retornar imediatamente */
    if (maxFrameGapMicros == 0) return 0;

#ifdef ESP32
    uint32_t now = (uint32_t)esp_timer_get_time();
#else
    uint32_t now = (uint32_t)millis() * 1000;
#endif

    /* Diferença modular — correto mesmo com overflow de uint32 */
    return (now - lastActivityTime) > maxFrameGapMicros;
}

/* =================================================================================
 * FEED — PROCESSAMENTO BYTE-A-BYTE (NÚCLEO DA FSM)
 * =================================================================================
 *
 * Esta é a função mais crítica do parser. Processa um único byte
 * recebido do UART e executa a transição de estado apropriada.
 *
 * Algoritmo:
 *   1. Registar timestamp (para deteção de timeout futuro)
 *   2. Verificar timeout (se não estamos em WAIT_START)
 *   3. Verificar buffer overflow (rawOffset < MAX_MESSAGE_SIZE)
 *   4. Processar o byte conforme o estado atual da FSM
 *   5. Retornar 1 se a mensagem ficou completa e válida
 *
 * @param byte Byte recebido do UART
 * @return uint8_t 1 = mensagem pronta, 0 = aguardando/erro
 */

uint8_t Parser::feed(uint8_t byte) {
    /* ================================================================
     * FASE 1: REGISTO DE ATIVIDADE
     * ================================================================
     * Atualizar timestamp para suporte a deteção de timeout.
     */
    updateTimestamp();

    /* ================================================================
     * FASE 2: VERIFICAÇÃO DE TIMEOUT
     * ================================================================
     * Se não estamos à espera de START_BYTE e o timeout está ativo,
     * verificar se o gap entre bytes excedeu o limite.
     * Se excedeu, registar erro (o reset automático é feito por setError).
     */
    if (state != PARSER_WAIT_START && maxFrameGapMicros > 0) {
        if (isTimedOut()) {
            DEBUG_PRINT("Timeout — gap entre bytes excedeu %lu us\n", maxFrameGapMicros);
            setError(PARSER_ERR_TIMEOUT);
            /* Após erro, o parser está em WAIT_START. */
            /* Continuar para processar o byte atual como início de nova mensagem. */
        }
    }

    /* ================================================================
     * FASE 3: PROTEÇÃO CONTRA BUFFER OVERFLOW
     * ================================================================
     * Se o buffer de receção está cheio, é impossível continuar.
     * Registar erro e retornar — o parser é reiniciado por setError().
     */
    if (rawOffset >= MAX_MESSAGE_SIZE) {
        DEBUG_PRINT("Buffer overflow — rawOffset=%zu, MAX=%d\n", rawOffset, MAX_MESSAGE_SIZE);
        setError(PARSER_ERR_OVERFLOW);
        return 0;
    }

    /* ================================================================
     * FASE 4: MÁQUINA DE ESTADOS (FSM)
     * ================================================================
     * Cada estado define como o byte atual é processado.
     * Transições determinísticas — sem comportamento aleatório.
     */

    switch (state) {

        /* ===========================================================
         * ESTADO: PARSER_WAIT_START
         * ===========================================================
         * Estado inicial e de ressincronização.
         * Procura ativamente o byte de sincronização (0xAA).
         * Todos os outros bytes são descartados silenciosamente.
         *
         * Transição: 0xAA → PARSER_WAIT_MSGID
         * =========================================================== */
        case PARSER_WAIT_START:
            if (byte == START_BYTE) {
                rawBuffer[rawOffset++] = byte;
                state = PARSER_WAIT_MSGID;
                DEBUG_PRINT("START_BYTE recebido (0x%02X) → WAIT_MSGID\n", byte);
            }
            return 0;  /* Sempre retorna 0 — aguardando mais bytes */

        /* ===========================================================
         * ESTADO: PARSER_WAIT_MSGID
         * ===========================================================
         * Aguarda o byte de msgID e valida-o imediatamente.
         * Se inválido, a trama é rejeitada (erro e reset).
         *
         * Transição válida: → PARSER_WAIT_TLVCOUNT
         * Transição erro:   → PARSER_WAIT_START (via setError)
         * =========================================================== */
        case PARSER_WAIT_MSGID:
            rawBuffer[rawOffset++] = byte;

            if (!isValidMsgID(byte)) {
                DEBUG_PRINT("msgID inválido: 0x%02X — erro\n", byte);
                setError(PARSER_ERR_INVALID_START);
                return 0;
            }

            state = PARSER_WAIT_TLVCOUNT;
            DEBUG_PRINT("msgID=0x%02X válido → WAIT_TLVCOUNT\n", byte);
            return 0;

        /* ===========================================================
         * ESTADO: PARSER_WAIT_TLVCOUNT
         * ===========================================================
         * Aguarda o número de campos TLV (0-8).
         * Se tlvCount == 0, saltamos diretamente para CHECKSUM
         * (mensagem sem campos de dados — apenas cabeçalho).
         *
         * Transição tlvCount==0: → PARSER_WAIT_CHECKSUM
         * Transição tlvCount>0:  → PARSER_WAIT_TLV_ID
         * Transição erro:        → PARSER_WAIT_START (via setError)
         * =========================================================== */
        case PARSER_WAIT_TLVCOUNT:
            rawBuffer[rawOffset++] = byte;

            if (byte > MAX_TLV_FIELDS) {
                DEBUG_PRINT("tlvCount=%d > MAX_TLV_FIELDS=%d — erro\n", byte, MAX_TLV_FIELDS);
                setError(PARSER_ERR_TLV_COUNT);
                return 0;
            }

            expectedTLVs = byte;
            currentTLV = 0;

            if (expectedTLVs == 0) {
                /* Sem TLVs — ir diretamente buscar o CRC */
                state = PARSER_WAIT_CHECKSUM;
                DEBUG_PRINT("tlvCount=0 → WAIT_CHECKSUM\n");
            } else {
                /* Começar a receção do primeiro TLV */
                state = PARSER_WAIT_TLV_ID;
                DEBUG_PRINT("tlvCount=%d → WAIT_TLV_ID\n", expectedTLVs);
            }
            return 0;

        /* ===========================================================
         * ESTADO: PARSER_WAIT_TLV_ID
         * ===========================================================
         * Aguarda o byte de ID do campo TLV atual.
         * NOTA: O ID NÃO é validado aqui — IDs desconhecidos são
         * aceites e ignorados a nível superior.
         *
         * Transição: → PARSER_WAIT_TLV_LEN
         * =========================================================== */
        case PARSER_WAIT_TLV_ID:
            rawBuffer[rawOffset++] = byte;
            state = PARSER_WAIT_TLV_LEN;
            DEBUG_PRINT("TLV[%d] ID=0x%02X → WAIT_TLV_LEN\n", currentTLV, byte);
            return 0;

        /* ===========================================================
         * ESTADO: PARSER_WAIT_TLV_LEN
         * ===========================================================
         * Aguarda o comprimento do payload do TLV atual.
         * Se len == 0, o campo não tem dados — avançar para o próximo.
         * Se len > MAX_TLV_DATA, erro (payload demasiado grande).
         *
         * Transição len==0: → PARSER_WAIT_TLV_ID (próximo) ou CHECKSUM
         * Transição len>0:  → PARSER_WAIT_TLV_DATA
         * Transição erro:   → PARSER_WAIT_START (via setError)
         * =========================================================== */
        case PARSER_WAIT_TLV_LEN:
            rawBuffer[rawOffset++] = byte;

            if (byte > MAX_TLV_DATA) {
                DEBUG_PRINT("TLV[%d] len=%d > MAX_TLV_DATA=%d — erro\n",
                            currentTLV, byte, MAX_TLV_DATA);
                setError(PARSER_ERR_TLV_LEN);
                return 0;
            }

            currentTLVLen = byte;
            tlvBytesReceived = 0;

            if (currentTLVLen == 0) {
                /* Campo vazio — avançar para o próximo TLV ou CHECKSUM */
                currentTLV++;
                if (currentTLV >= expectedTLVs) {
                    state = PARSER_WAIT_CHECKSUM;
                    DEBUG_PRINT("Último TLV (vazio) → WAIT_CHECKSUM\n");
                } else {
                    state = PARSER_WAIT_TLV_ID;
                    DEBUG_PRINT("TLV[%d] vazio → próximo TLV\n", currentTLV - 1);
                }
            } else {
                /* Campo com dados — receber payload */
                state = PARSER_WAIT_TLV_DATA;
                DEBUG_PRINT("TLV[%d] len=%d → WAIT_TLV_DATA\n", currentTLV, currentTLVLen);
            }
            return 0;

        /* ===========================================================
         * ESTADO: PARSER_WAIT_TLV_DATA
         * ===========================================================
         * Acumula os bytes do payload do TLV atual.
         * Conta os bytes recebidos e quando atinge o comprimento
         * esperado, avança para o próximo TLV ou para CHECKSUM.
         *
         * Transição: → PARSER_WAIT_TLV_ID (próximo) ou CHECKSUM
         * =========================================================== */
        case PARSER_WAIT_TLV_DATA:
            rawBuffer[rawOffset++] = byte;
            tlvBytesReceived++;

            if (tlvBytesReceived >= currentTLVLen) {
                currentTLV++;
                if (currentTLV >= expectedTLVs) {
                    /* Todos os TLVs recebidos — buscar CRC */
                    state = PARSER_WAIT_CHECKSUM;
                    DEBUG_PRINT("TLV[%d] completo → WAIT_CHECKSUM\n", currentTLV - 1);
                } else {
                    /* Ainda há TLVs por receber */
                    state = PARSER_WAIT_TLV_ID;
                    DEBUG_PRINT("TLV[%d] completo → próximo TLV\n", currentTLV - 1);
                }
            }
            return 0;

        /* ===========================================================
         * ESTADO: PARSER_WAIT_CHECKSUM
         * ===========================================================
         * Estado final — recebe o byte de CRC8 e valida a trama
         * completa chamando validateMessage().
         *
         * Se a validação for bem-sucedida:
         *   1. Preenche a struct msg com os dados recebidos
         *   2. Desserializa os TLVs para a struct
         *   3. Incrementa successCount
         *   4. Retorna 1 (mensagem pronta)
         *
         * Se a validação falhar:
         *   1. Regista o erro (CHECKSUM)
         *   2. O parser é reiniciado
         *   3. Retorna 0
         *
         * Transição sucesso: → PARSER_WAIT_START (via acknowledge)
         * Transição erro:    → PARSER_WAIT_START (via setError)
         * =========================================================== */
        case PARSER_WAIT_CHECKSUM:
            rawBuffer[rawOffset++] = byte;  /* Este é o byte do CRC */

            /* Validar toda a trama: estrutura + CRC8 */
            if (validateMessage(rawBuffer, rawOffset)) {
                /* Preencher cabeçalho da mensagem estruturada */
                msg.startByte = rawBuffer[0];
                msg.msgID     = rawBuffer[1];
                msg.tlvCount  = rawBuffer[2];
                msg.checksum  = rawBuffer[rawOffset - 1];

                /* Desserializar os TLVs para a struct */
                size_t parsedCount = 0;
                parseTLV(rawBuffer + 3, rawOffset - 4, msg.tlvs, &parsedCount);
                msg.tlvCount = (uint8_t)parsedCount;

                DEBUG_PRINT("Mensagem validada com sucesso — msgID=0x%02X, %d TLVs\n",
                            msg.msgID, msg.tlvCount);

                successCount++;
                messageReady = true;
                lastError = PARSER_OK;

                return 1;  /* Mensagem pronta para processamento */
            } else {
                DEBUG_PRINT("Validação falhou — CRC ou estrutura inválida\n");
                setError(PARSER_ERR_CHECKSUM);
                return 0;
            }

        /* ===========================================================
         * ESTADO DESCONHECIDO (nunca deveria ocorrer)
         * ===========================================================
         * Se por alguma razão a FSM chegar a um estado inválido,
         * registar erro e reiniciar. Garante recuperação.
         * =========================================================== */
        default:
            DEBUG_PRINT("Estado desconhecido: %d — reset\n", state);
            setError(PARSER_ERR_OVERFLOW);
            return 0;
    }
}

/* =================================================================================
 * HAS MESSAGE — VERIFICAÇÃO DE DISPONIBILIDADE
 * =================================================================================
 *
 * Retorna se existe uma mensagem completa e validada disponível
 * para processamento. Chamada tipicamente antes de getMessage().
 *
 * @return uint8_t 1 = mensagem disponível, 0 = nenhuma
 */

uint8_t Parser::hasMessage() const {
    return messageReady ? 1 : 0;
}

/* =================================================================================
 * GET MESSAGE — OBTER PONTEIRO PARA A MENSAGEM
 * =================================================================================
 *
 * Retorna um ponteiro para a struct TLVMessage contendo a última
 * mensagem recebida e validada.
 *
 * ATENÇÃO: O ponteiro aponta para memória interna do parser.
 *          O conteúdo pode ser sobrescrito na próxima chamada de feed().
 *          Para uso seguro, utilizar copyMessage().
 *
 * @return TLVMessage* Ponteiro para a mensagem, ou nullptr se indisponível
 */

TLVMessage* Parser::getMessage() {
    return hasMessage() ? &msg : nullptr;
}

/* =================================================================================
 * COPY MESSAGE — CÓPIA SEGURA DA MENSAGEM
 * =================================================================================
 *
 * Copia a mensagem atual para um buffer externo, eliminando a
 * dependência da persistência do buffer interno do parser.
 *
 * Esta é a forma RECOMENDADA de obter a mensagem pois é thread-safe
 * (desenquanto chamada de um único contexto) e não sofre de
 * sobrescrita por chamadas futuras de feed().
 *
 * @param output Ponteiro para TLVMessage de destino (deve ter espaço)
 * @return uint8_t 1 = sucesso, 0 = falha (output nullptr ou sem mensagem)
 */

uint8_t Parser::copyMessage(TLVMessage* output) {
    if (output == nullptr || !hasMessage()) {
        return 0;
    }
    memcpy(output, &msg, sizeof(TLVMessage));
    return 1;
}

/* =================================================================================
 * ACKNOWLEDGE — CONFIRMAÇÃO E LIBERTAÇÃO DO PARSER
 * =================================================================================
 *
 * Deve ser chamada APÓS processar completamente a mensagem recebida
 * (e após qualquer validação adicional da camada superior).
 *
 * Esta função:
 *   1. Limpa a flag messageReady
 *   2. Reinicia o parser (limpa buffer e estado)
 *
 * Após esta chamada, o parser está pronto para aceitar uma nova trama.
 */

void Parser::acknowledge() {
    messageReady = false;
    reset();  /* Limpar buffer e repor FSM para a próxima mensagem */
    DEBUG_PRINT("Mensagem acknowledge — parser pronto para próxima\n");
}

/* =================================================================================
 * SET MAX FRAME GAP — CONFIGURAÇÃO DO TIMEOUT
 * =================================================================================
 *
 * Define o intervalo máximo permitido entre bytes consecutivos.
 * Se o gap exceder este valor, a mensagem em curso é descartada.
 *
 * Valores típicos:
 *   • 100000 (100ms) — padrão, adequado para UART 115200
 *   • 50000  (50ms)  — agressivo, para ligações rápidas
 *   • 0              — timeout desativado (aceitar qualquer gap)
 *
 * @param micros Timeout em microssegundos (0 = desativado)
 */

void Parser::setMaxFrameGap(uint32_t micros) {
    maxFrameGapMicros = micros;
    DEBUG_PRINT("Timeout máximo entre bytes definido para %lu us\n", micros);
}

/* =================================================================================
 * SET DEBUG — CONTROLO DE MENSAGENS DE DIAGNÓSTICO
 * =================================================================================
 *
 * Ativa ou desativa a flag de debug interna. Na prática, as mensagens
 * de debug são controladas pela macro PARSER_DEBUG em tempo de compilação.
 * Esta função permite controlo adicional em runtime.
 *
 * @param enable true = ativar debug, false = desativar
 */

void Parser::setDebug(bool enable) {
    debugEnabled = enable;
    if (enable) {
        DEBUG_PRINT("Debug do Parser ATIVADO\n");
    }
}

/* =================================================================================
 * FUNÇÕES DE DIAGNÓSTICO (GETTERS PÚBLICOS)
 * =================================================================================
 *
 * Permitem aceder ao estado interno do parser para monitorização
 * e diagnóstico sem expor a estrutura interna.
 */

/**
 * Retorna o último código de erro registado.
 * Útil para determinar a causa de uma falha após feed() retornar 0.
 */
ParserError Parser::getLastError() const {
    return lastError;
}

/**
 * Retorna o número total de mensagens processadas com sucesso.
 * O contador não é resetado — permite monitorização de longo prazo.
 */
uint32_t Parser::getSuccessCount() const {
    return successCount;
}

/**
 * Retorna o número total de erros ocorridos desde o arranque.
 * Um número elevado de erros pode indicar problemas no canal UART.
 */
uint32_t Parser::getErrorCount() const {
    return errorCount;
}

/**
 * Retorna o estado atual da FSM.
 * Útil para debug e diagnóstico — permite ver onde o parser "travou".
 */
ParserState Parser::getCurrentState() const {
    return state;
}

/* =================================================================================
 * FUNÇÕES AUXILIARES — CONVERSÃO DE ENUMS PARA STRINGS
 * =================================================================================
 *
 * Estas funções mapeiam cada valor de enum para a sua representação
 * textual. Essenciais para logging legível e diagnóstico remoto.
 */

const char* parserStateToString(ParserState state) {
    switch (state) {
        case PARSER_WAIT_START:    return "WAIT_START";
        case PARSER_WAIT_MSGID:    return "WAIT_MSGID";
        case PARSER_WAIT_TLVCOUNT: return "WAIT_TLVCOUNT";
        case PARSER_WAIT_TLV_ID:   return "WAIT_TLV_ID";
        case PARSER_WAIT_TLV_LEN:  return "WAIT_TLV_LEN";
        case PARSER_WAIT_TLV_DATA: return "WAIT_TLV_DATA";
        case PARSER_WAIT_CHECKSUM: return "WAIT_CHECKSUM";
        default:                   return "UNKNOWN_STATE";
    }
}

const char* parserErrorToString(ParserError error) {
    switch (error) {
        case PARSER_OK:                return "OK";
        case PARSER_ERR_OVERFLOW:      return "OVERFLOW";
        case PARSER_ERR_TIMEOUT:       return "TIMEOUT";
        case PARSER_ERR_INVALID_START: return "INVALID_START";
        case PARSER_ERR_CHECKSUM:      return "CHECKSUM";
        case PARSER_ERR_TLV_COUNT:     return "TLV_COUNT";
        case PARSER_ERR_TLV_LEN:       return "TLV_LEN";
        default:                       return "UNKNOWN_ERROR";
    }
}
