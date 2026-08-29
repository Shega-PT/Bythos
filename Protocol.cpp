/**
 * =================================================================================
 * PROTOCOL.CPP — IMPLEMENTAÇÃO DO Bythos Protocol V1
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
 * Implementação das funções de serialização, desserialização e validação
 * do Bythos Protocol V1. Este módulo fornece as operações
 * fundamentais de construção e verificação de mensagens transmitidas
 * exclusivamente via UART.
 *
 * =================================================================================
 * ARQUITETURA DE IMPLEMENTAÇÃO
 * =================================================================================
 *
 * 1. CRC8/SMBUS: Tabela de lookup estática de 256 entradas para O(1) por byte.
 *    Polinómio 0x07 — amplamente utilizado em protocolos industriais (I2C, SMBus).
 *    Garante deteção de todos os erros de 1-2 bits e rajadas até 8 bits.
 *
 * 2. Serialização: Campo a campo, SEM memcpy() em structs inteiras.
 *    Isto elimina problemas de padding do compilador e alinhamento em memória,
 *    garantindo que o formato binário no canal é idêntico em qualquer plataforma.
 *
 * 3. Validação: Múltiplas camadas de verificação — estrutura, limites e CRC.
 *    Fail-secure: qualquer invalidade resulta em rejeição imediata da mensagem.
 *
 * =================================================================================
 */

#include "Protocol.h"
#include <string.h>
#include <stddef.h>

/* =================================================================================
 * DEBUG CONFIGURÁVEL
 * =================================================================================
 *
 * O prefixo "[Protocol]" permite filtragem no monitor série.
 * Em produção, manter PROTOCOL_DEBUG ativo é seguro — apenas imprime
 * para Serial/consola e nunca interfere no canal de comunicação.
 */

#define PROTOCOL_DEBUG

#ifdef PROTOCOL_DEBUG
    #ifdef ARDUINO
        #include <Arduino.h>
        #define DEBUG_PRINT(fmt, ...) Serial.printf("[Protocol] " fmt, ##__VA_ARGS__)
    #else
        #include <stdio.h>
        #define DEBUG_PRINT(fmt, ...) printf("[Protocol] " fmt, ##__VA_ARGS__)
    #endif
#else
    #define DEBUG_PRINT(fmt, ...) ((void)0)
#endif

/* =================================================================================
 * TABELA CRC8 — POLINÓMIO 0x07 (CRC-8/SMBUS)
 * =================================================================================
 *
 * Polinómio: x^8 + x^2 + x^1 + x^0 = 0x07
 *
 * A tabela de 256 entradas é pré-calculada e embutida em ROM.
 * Cada entrada corresponde ao resultado do CRC para o valor de um byte
 * quando processado sequencialmente. Isto permite calcular o CRC de
 * qualquer stream comcomplexidade O(n) onde n = número de bytes.
 *
 * O CRC-8/SMBUS é escolhido por:
 *   • Simplicidade de implementação (1 byte de output)
 *   • Boa deteção de erros para tramas curtas (<256 bytes)
 *   • Amplamente suportado em hardware e software
 */

static const uint8_t CRC8_TABLE[256] = {
    0x00, 0x07, 0x0E, 0x09, 0x1C, 0x1B, 0x12, 0x15, 0x38, 0x3F, 0x36, 0x31, 0x24, 0x23, 0x2A, 0x2D,
    0x70, 0x77, 0x7E, 0x79, 0x6C, 0x6B, 0x62, 0x65, 0x48, 0x4F, 0x46, 0x41, 0x54, 0x53, 0x5A, 0x5D,
    0xE0, 0xE7, 0xEE, 0xE9, 0xFC, 0xFB, 0xF2, 0xF5, 0xD8, 0xDF, 0xD6, 0xD1, 0xC4, 0xC3, 0xCA, 0xCD,
    0x90, 0x97, 0x9E, 0x99, 0x8C, 0x8B, 0x82, 0x85, 0xA8, 0xAB, 0xA2, 0xA5, 0xB0, 0xB7, 0xBE, 0xB9,
    0xC7, 0xC0, 0xC9, 0xCE, 0xDB, 0xDC, 0xD5, 0xD2, 0xFF, 0xF8, 0xF1, 0xF6, 0xE3, 0xE4, 0xED, 0xEA,
    0xB7, 0xB0, 0xB9, 0xBE, 0xAB, 0xAC, 0xA5, 0xA2, 0x8F, 0x88, 0x81, 0x86, 0x93, 0x94, 0x9D, 0x9A,
    0x27, 0x20, 0x29, 0x2E, 0x3B, 0x3C, 0x35, 0x32, 0x1F, 0x18, 0x11, 0x16, 0x03, 0x04, 0x0D, 0x0A,
    0x57, 0x50, 0x59, 0x5E, 0x4B, 0x4C, 0x45, 0x42, 0x6F, 0x68, 0x61, 0x66, 0x73, 0x74, 0x7D, 0x7A,
    0x89, 0x8E, 0x87, 0x80, 0x95, 0x92, 0x9B, 0x9C, 0xB1, 0xB6, 0xBF, 0xB8, 0xAD, 0xAA, 0xA3, 0xA4,
    0xF9, 0xFE, 0xF7, 0xF0, 0xE5, 0xE2, 0xEB, 0xEC, 0xC1, 0xC6, 0xCF, 0xC8, 0xDD, 0xDA, 0xD3, 0xD4,
    0x69, 0x6E, 0x67, 0x60, 0x75, 0x72, 0x7B, 0x7C, 0x51, 0x56, 0x5F, 0x58, 0x4D, 0x4A, 0x43, 0x44,
    0x19, 0x1E, 0x17, 0x10, 0x05, 0x02, 0x0B, 0x0C, 0x21, 0x26, 0x2F, 0x28, 0x3D, 0x3A, 0x33, 0x34,
    0x4E, 0x49, 0x40, 0x47, 0x52, 0x55, 0x5C, 0x5B, 0x76, 0x71, 0x78, 0x7F, 0x6A, 0x6D, 0x64, 0x63,
    0x3E, 0x39, 0x30, 0x37, 0x22, 0x25, 0x2C, 0x2B, 0x06, 0x01, 0x08, 0x0F, 0x1A, 0x1D, 0x14, 0x13,
    0xAE, 0xA9, 0xA0, 0xA7, 0xB2, 0xB5, 0xBC, 0xBB, 0x96, 0x91, 0x98, 0x9F, 0x8A, 0x8D, 0x84, 0x83,
    0xDE, 0xD9, 0xD0, 0xD7, 0xC2, 0xC5, 0xCC, 0xCB, 0xE6, 0xE1, 0xE8, 0xEF, 0xFA, 0xFD, 0xF4, 0xF3
};

/* =================================================================================
 * calcCRC8 — CÁLCULO DE CRC8 COM TABELA DE LOOKUP
 * =================================================================================
 *
 * Algoritmo estándar CRC-8/SMBUS:
 *   1. Inicializar registo CRC com 0x00 (valor inicial do SMBUS)
 *   2. Para cada byte do buffer: CRC = TABELA[CRC XOR byte]
 *   3. Retornar CRC final
 *
 * Complexidade temporal: O(n) onde n = len
 * Complexidade espacial: O(1) — tabela estática em ROM
 *
 * @param data  Ponteiro para os dados a processar
 * @param len   Número de bytes a incluir no cálculo
 * @return      CRC8 calculado (intervalo 0x00-0xFF)
 */

uint8_t calcCRC8(const uint8_t* data, size_t len) {
    /* Valor inicial do CRC-8/SMBUS é sempre 0x00 */
    uint8_t crc = 0x00;

    /* Processar cada byte sequencialmente via tabela de lookup */
    for (size_t i = 0; i < len; i++) {
        crc = CRC8_TABLE[crc ^ data[i]];
    }

    return crc;
}

/* =================================================================================
 * buildTLV — SERIALIZAÇÃO DE UM CAMPO TLV INDIVIDUAL
 * =================================================================================
 *
 * Serializa um campo TLV para um buffer de saída no formato compacto:
 *   [ID (1 byte)] [LEN (1 byte)] [DATA (LEN bytes)]
 *
 * Validações internas:
 *   • output não pode ser NULL
 *   • data não pode ser NULL quando len > 0
 *   • len é truncado silenciosamente para MAX_TLV_DATA se excedido
 *
 * @param id     Identificador do campo (ex: FLD_DATA_0)
 * @param data   Ponteiro para os dados a serializar
 * @param len    Número de bytes dos dados (máx MAX_TLV_DATA = 16)
 * @param output Buffer de saída (mínimo len+2 bytes)
 * @return       Número de bytes escritos (0 em caso de erro)
 */

size_t buildTLV(uint8_t id, const uint8_t* data, uint8_t len, uint8_t* output) {
    /* Validar ponteiro de saída — retorno imediato se inválido */
    if (output == nullptr) {
        DEBUG_PRINT("ERRO: buildTLV — output é NULL\n");
        return 0;
    }

    /* Validar coerência entre ponteiro e comprimento */
    if (data == nullptr && len > 0) {
        DEBUG_PRINT("ERRO: buildTLV — data é NULL mas len=%d\n", len);
        return 0;
    }

    /* Truncamento silencioso para evitar buffer overflow */
    if (len > MAX_TLV_DATA) {
        DEBUG_PRINT("AVISO: buildTLV id=0x%02X len=%d > MAX=%d → truncado\n",
                    id, len, MAX_TLV_DATA);
        len = MAX_TLV_DATA;
    }

    /* Serialização compacta: ID + LEN + DATA */
    output[0] = id;
    output[1] = len;

    if (len > 0) {
        memcpy(&output[2], data, len);
    }

    return (size_t)(len + 2);  /* Retornar total: ID + LEN + DATA */
}

/* =================================================================================
 * buildMessage — SERIALIZAÇÃO DE UMA MENSAGEM TLV COMPLETA
 * =================================================================================
 *
 * Monta uma trama completa pronta para transmissão via UART:
 *   [START_BYTE (0xAA)] [MSGID] [TLV_COUNT] [TLV0] [TLV1]...[TLVn] [CRC8]
 *
 * O CRC8 é calculado sobre TODOS os bytes anteriores ao CRC (START até
 * ao último byte do último TLV), garantindo verificação de integridade.
 *
 * Validações:
 *   • buffer e msg não podem ser NULL
 *   • bufferSize deve ser >= MIN_MESSAGE_SIZE (4 bytes)
 *   • tlvCount é limitado a MAX_TLV_FIELDS
 *   • Cada TLV é verificado individualmente contra overflow
 *
 * @param msg        Ponteiro para a mensagem estruturada em memória
 * @param msgID      ID da mensagem (sobrescreve msg->msgID se != 0)
 * @param buffer     Buffer de saída para a trama serializada
 * @param bufferSize Capacidade total do buffer de saída
 * @return           Número de bytes escritos (0 = erro)
 */

size_t buildMessage(TLVMessage* msg, uint8_t msgID, uint8_t* buffer, size_t bufferSize) {
    /* ================================================================
     * FASE 1: VALIDAÇÃO DE PARÂMETROS DE ENTRADA
     * ================================================================
     * Verificar todos os ponteiros e limites antes de qualquer escrita.
     * Qualquer invalidade resulta em retorno imediato de 0.
     */
    if (buffer == nullptr) {
        DEBUG_PRINT("ERRO: buildMessage — buffer é NULL\n");
        return 0;
    }

    if (msg == nullptr) {
        DEBUG_PRINT("ERRO: buildMessage — msg é NULL\n");
        return 0;
    }

    if (bufferSize < MIN_MESSAGE_SIZE) {
        DEBUG_PRINT("ERRO: buildMessage — buffer demasiado pequeno (%zu < %d)\n",
                    bufferSize, MIN_MESSAGE_SIZE);
        return 0;
    }

    /* ================================================================
     * FASE 2: NORMALIZAÇÃO DO NÚMERO DE TLVs
     * ================================================================
     * Garantir que o número de campos não excede o máximo permitido.
     * Se exceder, é truncado silenciosamente com aviso.
     */
    if (msg->tlvCount > MAX_TLV_FIELDS) {
        DEBUG_PRINT("AVISO: buildMessage tlvCount=%d > MAX=%d → truncado\n",
                    msg->tlvCount, MAX_TLV_FIELDS);
        msg->tlvCount = MAX_TLV_FIELDS;
    }

    /* ================================================================
     * FASE 3: ESCRITA DO CABEÇALHO (3 bytes)
     * ================================================================
     * Os três primeiros bytes são固定: START_BYTE, MSGID, TLV_COUNT.
     */
    buffer[0] = START_BYTE;
    buffer[1] = msgID;
    buffer[2] = msg->tlvCount;

    size_t offset = 3;  /* Próxima posição livre no buffer */

    /* ================================================================
     * FASE 4: SERIALIZAÇÃO SEQUENCIAL DOS TLVs
     * ================================================================
     * Cada campo TLV é serializado individualmente via buildTLV().
     * É verificado espaço suficiente antes de cada escrita.
     */
    for (size_t i = 0; i < msg->tlvCount; i++) {
        /* Verificar espaço para ID + LEN + payload do TLV atual */
        if (offset + 2 + msg->tlvs[i].len > bufferSize - 1) {
            DEBUG_PRINT("ERRO: buildMessage — buffer overflow no TLV %zu\n", i);
            return 0;
        }

        size_t written = buildTLV(msg->tlvs[i].id,
                                   msg->tlvs[i].data,
                                   msg->tlvs[i].len,
                                   &buffer[offset]);

        if (written == 0) {
            DEBUG_PRINT("ERRO: buildMessage — buildTLV falhou para TLV %zu\n", i);
            return 0;
        }

        offset += written;
    }

    /* ================================================================
     * FASE 5: CÁLCULO E ESCRITA DO CRC8
     * ================================================================
     * O CRC é calculado sobre todo o conteúdo já escrito (START até
     * ao último byte do último TLV) e escrito na posição seguinte.
     */
    if (offset + 1 > bufferSize) {
        DEBUG_PRINT("ERRO: buildMessage — sem espaço para CRC8\n");
        return 0;
    }

    buffer[offset] = calcCRC8(buffer, offset);
    offset++;

    DEBUG_PRINT("buildMessage: msgID=0x%02X, %d TLVs, tamanho=%zu bytes\n",
                msgID, msg->tlvCount, offset);

    return offset;
}

/* =================================================================================
 * validateMessage — VALIDAÇÃO ESTRUTURAL E DE INTEGRIDADE
 * =================================================================================
 *
 * Valida uma mensagem recebida verificando múltiplas camadas:
 *
 *   CAMADA 1 — Parâmetros básicos:
 *     • Buffer não é NULL
 *     • Comprimento >= MIN_MESSAGE_SIZE (4 bytes)
 *
 *   CAMADA 2 — Sincronização:
 *     • Primeiro byte deve ser START_BYTE (0xAA)
 *
 *   CAMADA 3 — Cabeçalho:
 *     • msgID deve estar no intervalo válido (0x10-0x15)
 *     • tlvCount não pode exceder MAX_TLV_FIELDS (8)
 *
 *   CAMADA 4 — Estrutura TLV:
 *     • Cada TLV tem pelo menos 2 bytes (ID + LEN)
 *     • Payload de cada TLV não excede o buffer restante
 *
 *   CAMADA 5 — Integridade:
 *     • CRC8 calculado corresponde ao CRC8 recebido
 *
 * Esta validação NÃO verifica autenticação (HMAC) nem sequência
 * anti-replay. A integridade depende exclusivamente do CRC8.
 *
 * @param buffer Buffer contendo a mensagem recebida
 * @param length Comprimento efetivo do buffer
 * @return       1 = mensagem válida, 0 = inválida
 */

uint8_t validateMessage(const uint8_t* buffer, size_t length) {
    /* ================================================================
     * CAMADA 1: VALIDAÇÃO DE PARÂMETROS BÁSICOS
     * ================================================================ */
    if (buffer == nullptr) {
        DEBUG_PRINT("ERRO: validateMessage — buffer é NULL\n");
        return 0;
    }

    if (length < MIN_MESSAGE_SIZE) {
        DEBUG_PRINT("ERRO: validateMessage — buffer demasiado curto (%zu < %d)\n",
                    length, MIN_MESSAGE_SIZE);
        return 0;
    }

    /* ================================================================
     * CAMADA 2: VERIFICAÇÃO DE SINCRONIZAÇÃO
     * ================================================================
     * O primeiro byte deve ser sempre 0xAA (START_BYTE).
     * Se não for, a mensagem é descartada imediatamente.
     */
    if (buffer[0] != START_BYTE) {
        DEBUG_PRINT("ERRO: validateMessage — START_BYTE inválido (0x%02X != 0xAA)\n", buffer[0]);
        return 0;
    }

    /* ================================================================
     * CAMADA 3: VALIDAÇÃO DO CABEÇALHO
     * ================================================================
     * Verificar msgID e tlvCount contra os limites definidos.
     */
    uint8_t msgID = buffer[1];
    if (!isValidMsgID(msgID)) {
        DEBUG_PRINT("ERRO: validateMessage — msgID desconhecido (0x%02X)\n", msgID);
        return 0;
    }

    uint8_t tlvCount = buffer[2];
    if (tlvCount > MAX_TLV_FIELDS) {
        DEBUG_PRINT("ERRO: validateMessage — tlvCount (%d) > MAX_TLV_FIELDS (%d)\n",
                    tlvCount, MAX_TLV_FIELDS);
        return 0;
    }

    /* ================================================================
     * CAMADA 4: VALIDAÇÃO DA ESTRUTURA TLV
     * ================================================================
     * Percorrer sequencialmente todos os TLVs verificando:
     *   • Existem pelo menos 2 bytes (ID + LEN)
     *   • O payload cabe no buffer restante
     *   • O LEN não excede o máximo permitido
     */
    size_t offset = 3;  /* Posição inicial: após START + MSGID + COUNT */

    for (uint8_t i = 0; i < tlvCount; i++) {
        /* Verificar se existem pelo menos 2 bytes para ID + LEN */
        if (offset + 2 > length - 1) {
            DEBUG_PRINT("ERRO: validateMessage — TLV[%d] header truncado\n", i);
            return 0;
        }

        uint8_t tlvLen = buffer[offset + 1];

        /* Verificar se o payload cabe no buffer restante */
        if (offset + 2 + tlvLen > length - 1) {
            DEBUG_PRINT("ERRO: validateMessage — payload TLV[%d] truncado (len=%d)\n", i, tlvLen);
            return 0;
        }

        /* Aviso para payloads que excedem o máximo (não crítico) */
        if (tlvLen > MAX_TLV_DATA) {
            DEBUG_PRINT("AVISO: validateMessage — TLV[%d] len=%d > MAX_TLV_DATA\n", i, tlvLen);
        }

        offset += 2 + tlvLen;  /* Avançar para o próximo TLV */
    }

    /* ================================================================
     * CAMADA 5: VERIFICAÇÃO DE INTEGRIDADE (CRC8)
     * ================================================================
     * O CRC8 deve ocupar exatamente a última posição do buffer.
     * Se o offset não coincide com length - 1, há incoerência.
     */
    if (offset + 1 != length) {
        DEBUG_PRINT("ERRO: validateMessage — tamanho incoerente (offset=%zu, length=%zu)\n",
                    offset, length);
        return 0;
    }

    /* Calcular CRC8 sobre todo o conteúdo (excluindo o próprio CRC) */
    uint8_t crcCalculado = calcCRC8(buffer, offset);
    uint8_t crcRecebido = buffer[length - 1];

    if (crcCalculado != crcRecebido) {
        DEBUG_PRINT("ERRO: validateMessage — CRC8 falhou (calc=0x%02X, recv=0x%02X)\n",
                    crcCalculado, crcRecebido);
        return 0;
    }

    DEBUG_PRINT("validateMessage: mensagem válida — msgID=0x%02X, %d TLVs\n", msgID, tlvCount);
    return 1;
}

/* =================================================================================
 * parseTLV — DESSERIALIZAÇÃO DE CAMPOS TLV
 * =================================================================================
 *
 * Extrai sequencialmente os campos TLV de um buffer bruto que contém
 * apenas a sequência de TLVs (sem START_BYTE, MSGID, COUNT, CRC).
 *
 * Cada TLV é lido como:
 *   [ID (1B)] [LEN (1B)] [DATA (LEN bytes)]
 *
 * Os campos são copiados para o array de saída até atingir:
 *   • O fim do buffer de entrada
 *   • O limite MAX_TLV_FIELDS
 *   • Um TLV truncado (LEN maior que o espaço restante)
 *
 * @param data   Buffer com a sequência de TLVs
 * @param length Comprimento do buffer de entrada
 * @param output Array de TLVField de saída (mínimo MAX_TLV_FIELDS)
 * @param count  Número de campos efetivamente extraídos (output)
 */

void parseTLV(const uint8_t* data, size_t length, TLVField* output, size_t* count) {
    /* Validar todos os parâmetros de entrada */
    if (data == nullptr || output == nullptr || count == nullptr) {
        if (count != nullptr) *count = 0;
        DEBUG_PRINT("ERRO: parseTLV — parâmetros inválidos\n");
        return;
    }

    size_t offset = 0;   /* Posição atual no buffer de entrada */
    size_t idx = 0;      /* Índice atual no array de saída */

    /* Limite máximo de campos a extrair */
    const size_t maxFields = MAX_TLV_FIELDS;

    /* Extrair TLVs sequencialmente até atingir o fim ou o limite */
    while (offset + 2 <= length && idx < maxFields) {
        uint8_t id = data[offset];
        uint8_t len = data[offset + 1];

        /* Verificar se o payload completo cabe no buffer restante */
        if (offset + 2 + len > length) {
            DEBUG_PRINT("AVISO: parseTLV — TLV truncado no índice %zu\n", idx);
            break;
        }

        /* Preencher o campo de saída */
        output[idx].id = id;
        output[idx].len = len;

        /* Copiar payload limitado ao máximo permitido */
        size_t copyLen = (len <= MAX_TLV_DATA) ? len : MAX_TLV_DATA;
        memcpy(output[idx].data, &data[offset + 2], copyLen);

        /* Se o payload excedeu o máximo, preencher o resto com zeros */
        if (len > MAX_TLV_DATA) {
            memset(&output[idx].data[MAX_TLV_DATA], 0, len - MAX_TLV_DATA);
        }

        offset += 2 + len;  /* Avançar para o próximo TLV no buffer */
        idx++;              /* Avançar para a próxima posição de saída */
    }

    *count = idx;  /* Retornar o número total de campos extraídos */

    /* Aviso se sobraram dados não processados no buffer */
    if (idx < maxFields && offset + 2 <= length) {
        DEBUG_PRINT("AVISO: parseTLV — buffer com dados extra (%zu bytes não processados)\n",
                    length - offset);
    }
}
