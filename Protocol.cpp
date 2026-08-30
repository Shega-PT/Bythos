/**
 * =================================================================================
 * PROTOCOL.CPP — IMPLEMENTAÇÃO DO PROTOCOLO TLV v2.0.0
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
 * Implementação das funções de serialização, desserialização, validação CRC8
 * e manipulação de campos TLV para o protocolo Bythos Protocol V2.
 * 
 * =================================================================================
 * NOTAS DE IMPLEMENTAÇÃO
 * =================================================================================
 * 
 * 1. CRC8: Utiliza tabela de lookup para máxima velocidade (O(1) por byte).
 *         Polinómio 0x07 (CRC-8/SMBUS) — usado em I2C e muitos protocolos.
 * 
 * 2. Serialização: Campo a campo, SEM usar memcpy() em structs inteiras
 *    para evitar problemas de padding e alinhamento.
 * 
 * 3. Debug: Compilação condicional — desativar definindo NDEBUG ou removendo
 *    a macro PROTOCOL_DEBUG. Em produção, manter ativo pois apenas imprime
 *    para Serial/console, nunca interfere no canal de comunicação.
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
 * Define PROTOCOL_DEBUG para ativar mensagens de diagnóstico.
 * O prefixo "[Protocol]" permite filtrar no monitor série.
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
 * TABELA CRC8 (POLINÓMIO 0x07 — CRC-8/SMBUS)
 * =================================================================================
 * 
 * Polinómio: x^8 + x^2 + x^1 + x^0 = 0x07
 * 
 * Porque usar tabela?
 *   - Velocidade: O(1) por byte vs O(8) por byte do cálculo bit-a-bit
 *   - Simplicidade: 256 bytes de ROM são insignificantes no ESP32
 *   - Deteção de erros: Detecta todos os erros de 1-2 bits, rajadas ≤8 bits
 * 
 * A tabela foi pré-calculada e está embutida em ROM para máxima performance.
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
 * Algoritmo:
 *   1. Inicializar CRC com 0x00
 *   2. Para cada byte: CRC = TABELA[CRC XOR byte]
 *   3. Retornar CRC final
 * 
 * Complexidade: O(n) onde n é o número de bytes
 * 
 * @param data  Ponteiro para os dados
 * @param len   Número de bytes
 * @return      CRC8 calculado
 */

uint8_t calcCRC8(const uint8_t* data, size_t len) {
    uint8_t crc = 0x00;  /* Valor inicial do CRC-8/SMBUS é 0x00 */
    
    for (size_t i = 0; i < len; i++) {
        crc = CRC8_TABLE[crc ^ data[i]];
    }
    
    return crc;
}

/* =================================================================================
 * buildTLV — SERIALIZA UM CAMPO TLV INDIVIDUAL
 * =================================================================================
 * 
 * Formato de saída:
 *   [ID (1 byte)] [LEN (1 byte)] [DATA (LEN bytes)]
 * 
 * @param id     Identificador do campo (ex: FLD_ROLL)
 * @param data   Ponteiro para os dados a serializar
 * @param len    Número de bytes dos dados (máx MAX_TLV_DATA)
 * @param output Buffer de saída (deve ter pelo menos len + 2 bytes)
 * @return       Número de bytes escritos (0 = erro)
 */

size_t buildTLV(uint8_t id, const uint8_t* data, uint8_t len, uint8_t* output) {
    /* Validação de parâmetros — segurança em primeiro lugar */
    if (output == nullptr) {
        DEBUG_PRINT("ERRO: buildTLV — output é NULL\n");
        return 0;
    }
    
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
    
    /* Serialização compacta */
    output[0] = id;
    output[1] = len;
    
    if (len > 0) {
        memcpy(&output[2], data, len);
    }
    
    return (size_t)(len + 2);  /* ID + LEN + DATA */
}

/* =================================================================================
 * buildTLVVideo — SERIALIZA UM CAMPO TLV DE VÍDEO
 * =================================================================================
 * 
 * Específico para payloads maiores (até MAX_TLV_VIDEO_DATA = 128 bytes)
 * Usa o ID fixo FLD_VIDEO_PAYLOAD (0xB3)
 * 
 * @param data   Ponteiro para os dados de vídeo
 * @param len    Comprimento do chunk (máx MAX_TLV_VIDEO_DATA)
 * @param output Buffer de saída
 * @return       Número de bytes escritos
 */

size_t buildTLVVideo(const uint8_t* data, uint8_t len, uint8_t* output) {
    if (output == nullptr) {
        DEBUG_PRINT("ERRO: buildTLVVideo — output é NULL\n");
        return 0;
    }
    
    if (len > MAX_TLV_VIDEO_DATA) {
        DEBUG_PRINT("AVISO: buildTLVVideo len=%d > MAX=%d → truncado\n", 
                    len, MAX_TLV_VIDEO_DATA);
        len = MAX_TLV_VIDEO_DATA;
    }
    
    output[0] = FLD_VIDEO_PAYLOAD;  /* ID fixo para payload de vídeo */
    output[1] = len;
    
    if (len > 0) {
        memcpy(&output[2], data, len);
    }
    
    return (size_t)(len + 2);
}

/* =================================================================================
 * buildMessage — SERIALIZA UMA MENSAGEM TLV COMPLETA
 * =================================================================================
 * 
 * Formato de saída:
 *   [START_BYTE][MSGID][TLV_COUNT][TLV0][TLV1]...[TLVn][CRC8]
 * 
 * O CRC8 é calculado sobre TODO o conteúdo (START_BYTE até ao último byte
 * do último TLV), excluindo o próprio byte de CRC.
 * 
 * @param msg        Ponteiro para a mensagem estruturada
 * @param msgID      ID da mensagem (sobrescreve msg->msgID se !=0)
 * @param buffer     Buffer de saída
 * @param bufferSize Tamanho do buffer (deve ser >= MIN_MESSAGE_SIZE)
 * @return           Número de bytes escritos (0 = erro)
 */

size_t buildMessage(TLVMessage* msg, uint8_t msgID, uint8_t* buffer, size_t bufferSize) {
    /* Validações iniciais */
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
    
    /* Garantir que o número de TLVs não excede o máximo */
    if (msg->tlvCount > MAX_TLV_FIELDS) {
        DEBUG_PRINT("AVISO: buildMessage tlvCount=%d > MAX=%d → truncado\n", 
                    msg->tlvCount, MAX_TLV_FIELDS);
        msg->tlvCount = MAX_TLV_FIELDS;
    }
    
    /* Escrever cabeçalho */
    buffer[0] = START_BYTE;
    buffer[1] = msgID;
    buffer[2] = msg->tlvCount;
    
    size_t offset = 3;  /* Próxima posição livre no buffer */
    
    /* Serializar cada campo TLV */
    for (size_t i = 0; i < msg->tlvCount; i++) {
        /* Verificar espaço suficiente para ID + LEN + DATA */
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
    
    /* Verificar espaço para o CRC8 */
    if (offset + 1 > bufferSize) {
        DEBUG_PRINT("ERRO: buildMessage — sem espaço para CRC8\n");
        return 0;
    }
    
    /* Calcular e escrever CRC8 sobre todo o conteúdo */
    buffer[offset] = calcCRC8(buffer, offset);
    offset++;
    
    DEBUG_PRINT("buildMessage: msgID=0x%02X, %d TLVs, tamanho=%zu bytes\n", 
                msgID, msg->tlvCount, offset);
    
    return offset;
}

/* =================================================================================
 * validateMessage — VALIDA UMA MENSAGEM RECEBIDA
 * =================================================================================
 * 
 * Verificações realizadas:
 *   1. START_BYTE correto (0xAA)
 *   2. msgID válido (0x10-0x18)
 *   3. tlvCount dentro dos limites (≤ MAX_TLV_FIELDS)
 *   4. Estrutura dos TLVs (sem truncamentos)
 *   5. CRC8 correspondente
 * 
 * NOTA: Esta função NÃO verifica HMAC ou sequência anti-replay.
 *       Essas validações são responsabilidade do módulo Security.
 * 
 * @param buffer Buffer com a mensagem recebida
 * @param length Comprimento do buffer
 * @return       1 = válida, 0 = inválida
 */

uint8_t validateMessage(const uint8_t* buffer, size_t length) {
    /* Validação básica de parâmetros */
    if (buffer == nullptr) {
        DEBUG_PRINT("ERRO: validateMessage — buffer é NULL\n");
        return 0;
    }
    
    if (length < MIN_MESSAGE_SIZE) {
        DEBUG_PRINT("ERRO: validateMessage — buffer demasiado curto (%zu < %d)\n", 
                    length, MIN_MESSAGE_SIZE);
        return 0;
    }
    
    /* 1. Verificar START_BYTE */
    if (buffer[0] != START_BYTE) {
        DEBUG_PRINT("ERRO: validateMessage — START_BYTE inválido (0x%02X != 0xAA)\n", buffer[0]);
        return 0;
    }
    
    /* 2. Verificar msgID */
    uint8_t msgID = buffer[1];
    if (!isValidMsgID(msgID)) {
        DEBUG_PRINT("ERRO: validateMessage — msgID desconhecido (0x%02X)\n", msgID);
        return 0;
    }
    
    /* 3. Verificar tlvCount */
    uint8_t tlvCount = buffer[2];
    if (tlvCount > MAX_TLV_FIELDS) {
        DEBUG_PRINT("ERRO: validateMessage — tlvCount (%d) > MAX_TLV_FIELDS (%d)\n", 
                    tlvCount, MAX_TLV_FIELDS);
        return 0;
    }
    
    /* 4. Percorrer TLVs para validar estrutura */
    size_t offset = 3;  /* Começa após START_BYTE + MSGID + COUNT */
    
    for (uint8_t i = 0; i < tlvCount; i++) {
        /* Verificar se cabe pelo menos ID + LEN */
        if (offset + 2 > length - 1) {
            DEBUG_PRINT("ERRO: validateMessage — TLV[%d] header truncado\n", i);
            return 0;
        }
        
        uint8_t tlvLen = buffer[offset + 1];
        
        /* Verificar se o payload cabe no buffer */
        if (offset + 2 + tlvLen > length - 1) {
            DEBUG_PRINT("ERRO: validateMessage — payload TLV[%d] truncado (len=%d)\n", i, tlvLen);
            return 0;
        }
        
        /* Aviso para depuração (não crítico) */
        if (tlvLen > MAX_TLV_DATA) {
            DEBUG_PRINT("AVISO: validateMessage — TLV[%d] len=%d > MAX_TLV_DATA\n", i, tlvLen);
        }
        
        offset += 2 + tlvLen;
    }
    
    /* 5. Verificar se o CRC está na posição esperada */
    if (offset + 1 != length) {
        DEBUG_PRINT("ERRO: validateMessage — tamanho incoerente (offset=%zu, length=%zu)\n", 
                    offset, length);
        return 0;
    }
    
    /* 6. Verificar CRC8 */
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
 * parseTLV — EXTRAI CAMPOS TLV DE UM BUFFER BRUTO
 * =================================================================================
 * 
 * Esta função percorre um buffer que contém apenas a sequência de TLVs
 * (sem START_BYTE, MSGID, COUNT, CRC) e preenche um array de TLVField.
 * 
 * @param data   Buffer com dados TLV
 * @param length Comprimento do buffer
 * @param output Array de saída (deve ter capacidade para MAX_TLV_FIELDS)
 * @param count  Número de campos extraídos (output)
 */

void parseTLV(const uint8_t* data, size_t length, TLVField* output, size_t* count) {
    /* Validação de parâmetros */
    if (data == nullptr || output == nullptr || count == nullptr) {
        if (count != nullptr) *count = 0;
        DEBUG_PRINT("ERRO: parseTLV — parâmetros inválidos\n");
        return;
    }
    
    size_t offset = 0;
    size_t idx = 0;
    
    /* Limitar ao máximo de campos */
    const size_t maxFields = MAX_TLV_FIELDS;
    
    while (offset + 2 <= length && idx < maxFields) {
        uint8_t id = data[offset];
        uint8_t len = data[offset + 1];
        
        /* Verificar se o payload cabe no buffer */
        if (offset + 2 + len > length) {
            DEBUG_PRINT("AVISO: parseTLV — TLV truncado no índice %zu\n", idx);
            break;
        }
        
        /* Preencher campo de saída */
        output[idx].id = id;
        output[idx].len = len;
        
        /* Copiar payload para o campo de saída — limitado a MAX_TLV_DATA
         * para evitar overflow. Se o payload exceder, é truncado silenciosamente. */
        size_t copyLen = (len <= MAX_TLV_DATA) ? len : MAX_TLV_DATA;
        memcpy(output[idx].data, &data[offset + 2], copyLen);
        
        /* =================================================================
         * CRITICAL FIX: Zero-padding CORRETO dentro dos limites do array
         * =================================================================
         * BUG ANTERIOR: O memset escrevia para além de output[idx].data[],
         * corrompendo TLVFields adjacentes no array de saída. O cálculo
         * usava "len - MAX_TLV_DATA" como tamanho, mas escrevia a partir
         * de "data[MAX_TLV_DATA]" — para além do fim do array de 32 bytes.
         *
         * CORREÇÃO: Preencher com zeros APENAS os bytes restantes dentro
         * do array data[MAX_TLV_DATA], desde copyLen até MAX_TLV_DATA.
         * ================================================================= */
        if (copyLen < MAX_TLV_DATA) {
            memset(&output[idx].data[copyLen], 0, MAX_TLV_DATA - copyLen);
        }
        
        offset += 2 + len;
        idx++;
    }
    
    *count = idx;
    
    if (idx < maxFields && offset + 2 <= length) {
        DEBUG_PRINT("AVISO: parseTLV — buffer com dados extra (%zu bytes não processados)\n", 
                    length - offset);
    }
}

/* =================================================================================
 * FUNÇÕES DE DEBUG
 * =================================================================================
 * 
 * Estas funções são úteis para diagnóstico e desenvolvimento.
 * Em produção, podem ser desativadas removendo PROTOCOL_DEBUG.
 */

void printTLVField(const TLVField* field) {
    if (field == nullptr) {
        DEBUG_PRINT("TLV: NULL\n");
        return;
    }
    
    DEBUG_PRINT("  TLV ID:0x%02X LEN:%d DATA:", field->id, field->len);
    
    for (uint8_t i = 0; i < field->len && i < 16; i++) {  /* Limitar a 16 bytes para legibilidade */
        DEBUG_PRINT(" %02X", field->data[i]);
    }
    
    if (field->len > 16) {
        DEBUG_PRINT(" ... (%d bytes restantes)", field->len - 16);
    }
    
    DEBUG_PRINT("\n");
}

void printMessage(const TLVMessage* msg) {
    if (msg == nullptr) {
        DEBUG_PRINT("Message: NULL\n");
        return;
    }
    
    DEBUG_PRINT("=== MENSAGEM TLV ===\n");
    DEBUG_PRINT("  START_BYTE: 0x%02X\n", msg->startByte);
    DEBUG_PRINT("  MSG ID:     0x%02X\n", msg->msgID);
    DEBUG_PRINT("  TLV COUNT:  %d\n", msg->tlvCount);
    DEBUG_PRINT("  CHECKSUM:   0x%02X\n", msg->checksum);
    
    for (uint8_t i = 0; i < msg->tlvCount; i++) {
        printTLVField(&msg->tlvs[i]);
    }
    
    DEBUG_PRINT("========================\n");
}