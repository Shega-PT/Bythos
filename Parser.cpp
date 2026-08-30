/**
 * =================================================================================
 * PARSER.CPP — IMPLEMENTAÇÃO DO PARSER BYTE-A-BYTE
 * =================================================================================
 * 
 * AUTOR:      ShegaPT
 * DATA:       2026-04-17
 * VERSÃO:     2.0.0
 * 
 * =================================================================================
 * DESCRIÇÃO
 * =================================================================================
 * 
 * Implementação da Máquina de Estados Finita (FSM) para reconstrução de mensagens
 * TLV a partir de um stream de bytes. Inclui proteção contra timeout, buffer
 * overflow e validação estrutural completa.
 * 
 * =================================================================================
 * NOTAS DE IMPLEMENTAÇÃO
 * =================================================================================
 * 
 * 1. A FSM é determinística e não utiliza recursão.
 * 2. Todas as validações são feitas em tempo real, byte a byte.
 * 3. Timeout: utiliza esp_timer_get_time() no ESP32, millis()*1000 noutras plataformas.
 * 4. Em caso de erro, o parser faz reset automático e descarta a mensagem.
 * 5. O buffer rawBuffer acumula a mensagem para validação CRC8 final.
 * 
 * =================================================================================
 */

#include "Parser.h"
#include <string.h>

/* =================================================================================
 * INCLUDES CONDICIONAIS POR PLATAFORMA
 * =================================================================================
 * 
 * esp_timer.h: Fornece esp_timer_get_time() no ESP32 — alta precisão
 * em microssegundos. Apenas incluído quando a plataforma é ESP32.
 * Noutras plataformas, utiliza millis() como fallback.
 */

#ifdef ESP32
    #include "esp_timer.h"
#endif

/* =================================================================================
 * DEBUG CONFIGURÁVEL
 * =================================================================================
 * 
 * Prefixo "[Parser]" para fácil filtragem no monitor série.
 * Seguro manter ativo em produção — não afeta nenhum canal de voo.
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
 * CONSTRUTOR
 * =================================================================================
 * 
 * Inicializa todas as variáveis e coloca o parser no estado WAIT_START.
 * As estatísticas (successCount, errorCount) são inicializadas a zero.
 */

Parser::Parser() {
    /* Garantir que todos os campos têm valores definidos */
    memset(rawBuffer, 0, sizeof(rawBuffer));
    memset(&msg, 0, sizeof(TLVMessage));
    
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
    maxFrameGapMicros = 100000;  /* 100ms por defeito — suficiente para UART 115200 */
    
    updateTimestamp();  /* Inicializar timestamp para timeout */
}

/* =================================================================================
 * RESET — REPÕE O PARSER AO ESTADO INICIAL
 * =================================================================================
 * 
 * Limpa buffers, reinicia a FSM e mantém as estatísticas (successCount/errorCount)
 * para permitir monitorização de longo prazo.
 */

void Parser::reset() {
    /* Limpar buffers (segurança — evita lixo residual) */
    memset(rawBuffer, 0, sizeof(rawBuffer));
    memset(&msg, 0, sizeof(TLVMessage));
    
    /* Reiniciar todas as variáveis de estado */
    rawOffset         = 0;
    state             = PARSER_WAIT_START;
    expectedTLVs      = 0;
    currentTLV        = 0;
    tlvBytesReceived  = 0;
    currentTLVLen     = 0;
    messageReady      = false;
    lastError         = PARSER_OK;
    
    /* Reiniciar timestamp para evitar timeout falso */
    updateTimestamp();
    
    DEBUG_PRINT("Parser resetado\n");
}

/* =================================================================================
 * UPDATE TIMESTAMP — ATUALIZA O TEMPO DA ÚLTIMA ATIVIDADE
 * =================================================================================
 * 
 * Utiliza a melhor fonte de tempo disponível para a plataforma.
 * No ESP32: esp_timer_get_time() (microssegundos, alta precisão)
 * Noutras: millis() * 1000 (precisão de milissegundos)
 */

void Parser::updateTimestamp() {
#ifdef ESP32
    lastActivityTime = (uint32_t)esp_timer_get_time();
#else
    /* Fallback para plataformas não-ESP32 (Arduino, Raspberry Pi Pico, etc.) */
    lastActivityTime = (uint32_t)millis() * 1000;
#endif
}

/* =================================================================================
 * SET ERROR — REGISTA UM ERRO E FAZ RESET
 * =================================================================================
 * 
 * Incrementa o contador de erros, guarda o código do erro e reinicia o parser.
 * Isto garante que após um erro, o parser está pronto para a próxima mensagem.
 */

void Parser::setError(ParserError error) {
    /* =====================================================================
     * CRITICAL FIX: Atribuir lastError DEPOIS de reset()
     * =====================================================================
     * BUG ANTERIOR: lastError era definido antes de reset(), mas reset()
     * repunha lastError = PARSER_OK, silenciando todos os erros.
     * getLastError() retornava sempre PARSER_OK independentemente do
     * erro ocorrido — diagnosticamento completamente inoperacional.
     *
     * CORREÇÃO: Chamar reset() primeiro, e só então atribuir lastError.
     * Assim o valor do erro é preservado e acessível via getLastError().
     * ===================================================================== */
    errorCount++;
    messageReady = false;
    
    reset();  /* Repõe o parser ao estado inicial — incluindo lastError = OK */
    
    /* Agora sim, registar o erro. Sobrescreve o PARSER_OK do reset. */
    lastError = error;
    
    DEBUG_PRINT("ERRO: %s (código %d) — total de erros: %lu\n", 
                parserErrorToString(error), error, errorCount);
}

/* =================================================================================
 * IS TIMED OUT — VERIFICA SE OCORREU TIMEOUT NA RECEÇÃO
 * =================================================================================
 * 
 * Compara o tempo decorrido desde o último byte com o timeout configurado.
 * Se maxFrameGapMicros == 0, o timeout está desativado.
 * 
 * @return 1 se timeout ocorreu, 0 caso contrário
 */

uint8_t Parser::isTimedOut() const {
    if (maxFrameGapMicros == 0) return 0;  /* Timeout desativado */
    
#ifdef ESP32
    uint32_t now = (uint32_t)esp_timer_get_time();
    return (now - lastActivityTime) > maxFrameGapMicros;
#else
    uint32_t now = (uint32_t)millis() * 1000;
    return (now - lastActivityTime) > maxFrameGapMicros;
#endif
}

/* =================================================================================
 * FEED — ALIMENTA UM BYTE AO PARSER (NÚCLEO DA FSM)
 * =================================================================================
 * 
 * Esta é a função mais importante do parser. Processa cada byte recebido
 * e avança a máquina de estados conforme necessário.
 * 
 * Algoritmo:
 *   1. Atualiza timestamp (para deteção de timeout)
 *   2. Verifica timeout (se aplicável)
 *   3. Protege contra buffer overflow
 *   4. Acumula o byte (exceto em WAIT_START, onde só acumula após START_BYTE)
 *   5. Executa a transição de estado apropriada
 *   6. Quando a mensagem está completa, valida e retorna 1
 * 
 * @param byte  Byte recebido do canal físico
 * @return uint8_t  1 = mensagem completa e válida, 0 = continuação/erro
 */

uint8_t Parser::feed(uint8_t byte) {
    /* =====================================================================
     * 1. VERIFICAR TIMEOUT ANTES DE ATUALIZAR O TIMESTAMP
     * =====================================================================
     * CRITICAL FIX: A verificação de timeout deve ocorrer ANTES de
     * atualizar lastActivityTime. Caso contrário, a diferença entre
     * "now" e "lastActivityTime" será sempre ~0, tornando a deteção
     * de gaps entre bytes completamente inoperacional.
     *
     * Fluxo correto:
     *   (a) Verificar se o tempo desde o último byte excedeu o limite
     *   (b) Se sim → erro e reset
     *   (c) Só depois atualizar o timestamp para o momento atual
     * ===================================================================== */
    if (state != PARSER_WAIT_START && maxFrameGapMicros > 0) {
        if (isTimedOut()) {
            DEBUG_PRINT("Timeout — gap entre bytes excedeu %lu us\n", maxFrameGapMicros);
            setError(PARSER_ERR_TIMEOUT);
            /* Após erro, o parser repousa em WAIT_START.
             * O byte atual pode ser o início de uma nova mensagem → processar. */
        }
    }
    
    /* 2. Atualizar timestamp da última atividade (só após validação de timeout) */
    updateTimestamp();
    
    /* 3. Proteção contra buffer overflow */
    if (rawOffset >= MAX_MESSAGE_SIZE) {
        DEBUG_PRINT("Buffer overflow — rawOffset=%zu, MAX=%d\n", rawOffset, MAX_MESSAGE_SIZE);
        setError(PARSER_ERR_OVERFLOW);
        return 0;
    }
    
    /* 4. Acumular byte no buffer (exceto em WAIT_START, onde acumulamos após validação) */
    /* NOTA: Em WAIT_START, só acumulamos se o byte for START_BYTE */
    
    /* 5. Máquina de Estados */
    switch (state) {
        
        /* ----------------------------------------------------------------------
         * ESTADO: WAIT_START
         * 
         * Aguarda o byte de sincronização (0xAA). Descartamos silenciosamente
         * qualquer byte diferente. Este é o mecanismo de ressincronização.
         * ---------------------------------------------------------------------- */
        case PARSER_WAIT_START:
            if (byte == START_BYTE) {
                rawBuffer[rawOffset++] = byte;
                state = PARSER_WAIT_MSGID;
                DEBUG_PRINT("START_BYTE recebido (0x%02X) → WAIT_MSGID\n", byte);
            }
            return 0;
        
        /* ----------------------------------------------------------------------
         * ESTADO: WAIT_MSGID
         * 
         * Aguarda o ID da mensagem. Valida imediatamente usando a função
         * constexpr isValidMsgID(). Se inválido, erro e reset.
         * ---------------------------------------------------------------------- */
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
        
        /* ----------------------------------------------------------------------
         * ESTADO: WAIT_TLVCOUNT
         * 
         * Aguarda o número de campos TLV. Valida contra MAX_TLV_FIELDS.
         * Se tlvCount == 0, saltamos diretamente para CHECKSUM.
         * ---------------------------------------------------------------------- */
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
                state = PARSER_WAIT_CHECKSUM;
                DEBUG_PRINT("tlvCount=0 → WAIT_CHECKSUM\n");
            } else {
                state = PARSER_WAIT_TLV_ID;
                DEBUG_PRINT("tlvCount=%d → WAIT_TLV_ID\n", expectedTLVs);
            }
            return 0;
        
        /* ----------------------------------------------------------------------
         * ESTADO: WAIT_TLV_ID
         * 
         * Aguarda o ID do TLV atual.
         * NOTA: Não validamos o ID aqui. IDs desconhecidos são aceites e
         *       ignorados a nível superior (pelo módulo que processa a mensagem).
         * ---------------------------------------------------------------------- */
        case PARSER_WAIT_TLV_ID:
            rawBuffer[rawOffset++] = byte;
            state = PARSER_WAIT_TLV_LEN;
            DEBUG_PRINT("TLV[%d] ID=0x%02X → WAIT_TLV_LEN\n", currentTLV, byte);
            return 0;
        
        /* ----------------------------------------------------------------------
         * ESTADO: WAIT_TLV_LEN
         * 
         * Aguarda o comprimento do payload do TLV atual.
         * Valida contra MAX_TLV_VIDEO_DATA (limite máximo do sistema).
         * Se len == 0, avançamos para o próximo TLV sem dados.
         * ---------------------------------------------------------------------- */
        case PARSER_WAIT_TLV_LEN:
            rawBuffer[rawOffset++] = byte;
            
            if (byte > MAX_TLV_VIDEO_DATA) {
                DEBUG_PRINT("TLV[%d] len=%d > MAX_TLV_VIDEO_DATA=%d — erro\n", 
                            currentTLV, byte, MAX_TLV_VIDEO_DATA);
                setError(PARSER_ERR_TLV_LEN);
                return 0;
            }
            
            currentTLVLen = byte;
            tlvBytesReceived = 0;
            
            if (currentTLVLen == 0) {
                /* Campo sem payload → avançar para próximo TLV */
                currentTLV++;
                if (currentTLV >= expectedTLVs) {
                    state = PARSER_WAIT_CHECKSUM;
                    DEBUG_PRINT("Último TLV (vazio) → WAIT_CHECKSUM\n");
                } else {
                    state = PARSER_WAIT_TLV_ID;
                    DEBUG_PRINT("TLV[%d] vazio → próximo TLV\n", currentTLV - 1);
                }
            } else {
                state = PARSER_WAIT_TLV_DATA;
                DEBUG_PRINT("TLV[%d] len=%d → WAIT_TLV_DATA\n", currentTLV, currentTLVLen);
            }
            return 0;
        
        /* ----------------------------------------------------------------------
         * ESTADO: WAIT_TLV_DATA
         * 
         * Aguarda os bytes do payload. Conta os bytes recebidos e quando
         * atinge o comprimento esperado, avança para o próximo TLV ou CHECKSUM.
         * ---------------------------------------------------------------------- */
        case PARSER_WAIT_TLV_DATA:
            rawBuffer[rawOffset++] = byte;
            tlvBytesReceived++;
            
            if (tlvBytesReceived >= currentTLVLen) {
                currentTLV++;
                if (currentTLV >= expectedTLVs) {
                    state = PARSER_WAIT_CHECKSUM;
                    DEBUG_PRINT("TLV[%d] completo → WAIT_CHECKSUM\n", currentTLV - 1);
                } else {
                    state = PARSER_WAIT_TLV_ID;
                    DEBUG_PRINT("TLV[%d] completo → próximo TLV\n", currentTLV - 1);
                }
            }
            return 0;
        
        /* ----------------------------------------------------------------------
         * ESTADO: WAIT_CHECKSUM
         * 
         * Recebe o byte de CRC8 e finaliza a mensagem.
         * Chama validateMessage() para verificar integridade.
         * Se válida, preenche a struct msg e retorna 1.
         * ---------------------------------------------------------------------- */
        case PARSER_WAIT_CHECKSUM:
            rawBuffer[rawOffset++] = byte;  /* Este é o byte do CRC */
            
            /* Validar a mensagem completa */
            if (validateMessage(rawBuffer, rawOffset)) {
                /* Preencher cabeçalho da struct msg */
                msg.startByte = rawBuffer[0];
                msg.msgID     = rawBuffer[1];
                msg.tlvCount  = rawBuffer[2];
                msg.checksum  = rawBuffer[rawOffset - 1];
                
                /* Desserializar os TLVs para a struct */
                size_t parsedCount = 0;
                parseTLV(rawBuffer + 3, rawOffset - 4, msg.tlvs, &parsedCount);
                msg.tlvCount = (uint8_t)parsedCount;
                
                DEBUG_PRINT("✅ Mensagem validada! msgID=0x%02X, %d TLVs\n", 
                            msg.msgID, msg.tlvCount);
                
                successCount++;
                messageReady = true;
                lastError = PARSER_OK;
                
                return 1;  /* Mensagem pronta — chamar Security::verifyPacket() */
            } else {
                DEBUG_PRINT("❌ Validação falhou (CRC ou estrutura inválida)\n");
                setError(PARSER_ERR_CHECKSUM);
                return 0;
            }
        
        /* ----------------------------------------------------------------------
         * ESTADO DESCONHECIDO (nunca deveria acontecer)
         * ---------------------------------------------------------------------- */
        default:
            DEBUG_PRINT("Estado desconhecido: %d — reset\n", state);
            setError(PARSER_ERR_OVERFLOW);
            return 0;
    }
}

/* =================================================================================
 * HAS MESSAGE — VERIFICA SE HÁ MENSAGEM DISPONÍVEL
 * =================================================================================
 */

uint8_t Parser::hasMessage() const {
    return messageReady ? 1 : 0;
}

/* =================================================================================
 * GET MESSAGE — RETORNA PONTEIRO PARA A MENSAGEM
 * =================================================================================
 * 
 * NOTA: O ponteiro aponta para buffer interno. O conteúdo pode ser sobrescrito
 *       na próxima chamada de feed(). Para uso seguro, use copyMessage().
 */

TLVMessage* Parser::getMessage() {
    return hasMessage() ? &msg : nullptr;
}

/* =================================================================================
 * COPY MESSAGE — COPIA A MENSAGEM PARA UM BUFFER EXTERNO
 * =================================================================================
 * 
 * Esta é a forma RECOMENDADA de obter a mensagem, pois é thread-safe
 * e não depende da persistência do buffer interno.
 * 
 * @param output  Ponteiro para TLVMessage de destino
 * @return uint8_t  1 = sucesso, 0 = falha
 */

uint8_t Parser::copyMessage(TLVMessage* output) {
    if (output == nullptr || !hasMessage()) {
        return 0;
    }
    memcpy(output, &msg, sizeof(TLVMessage));
    return 1;
}

/* =================================================================================
 * ACKNOWLEDGE — CONFIRMA A MENSAGEM E PREPARA PARA A PRÓXIMA
 * =================================================================================
 * 
 * Deve ser chamada APÓS processar a mensagem (e após a validação Security).
 * Liberta o parser para aceitar a próxima mensagem.
 */

void Parser::acknowledge() {
    messageReady = false;
    reset();  /* Limpar buffer e estado após acknowledge */
    DEBUG_PRINT("Mensagem acknowledge — parser pronto para próxima\n");
}

/* =================================================================================
 * SET MAX FRAME GAP — CONFIGURA TIMEOUT ENTRE BYTES
 * =================================================================================
 * 
 * @param micros  Timeout em microssegundos (0 = desativado)
 */

void Parser::setMaxFrameGap(uint32_t micros) {
    maxFrameGapMicros = micros;
    DEBUG_PRINT("Timeout máximo entre bytes definido para %lu us\n", micros);
}

/* =================================================================================
 * FUNÇÕES DE DIAGNÓSTICO (GETTERS)
 * =================================================================================
 */

ParserError Parser::getLastError() const {
    return lastError;
}

uint32_t Parser::getSuccessCount() const {
    return successCount;
}

uint32_t Parser::getErrorCount() const {
    return errorCount;
}

ParserState Parser::getCurrentState() const {
    return state;
}

/* =================================================================================
 * FUNÇÕES AUXILIARES PARA DEBUG (CONVERSÃO PARA STRING)
 * =================================================================================
 */

const char* parserStateToString(ParserState state) {
    switch (state) {
        case PARSER_WAIT_START:     return "WAIT_START";
        case PARSER_WAIT_MSGID:     return "WAIT_MSGID";
        case PARSER_WAIT_TLVCOUNT:  return "WAIT_TLVCOUNT";
        case PARSER_WAIT_TLV_ID:    return "WAIT_TLV_ID";
        case PARSER_WAIT_TLV_LEN:   return "WAIT_TLV_LEN";
        case PARSER_WAIT_TLV_DATA:  return "WAIT_TLV_DATA";
        case PARSER_WAIT_CHECKSUM:  return "WAIT_CHECKSUM";
        default:                    return "UNKNOWN_STATE";
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