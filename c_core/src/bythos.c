/**
 * @file bythos.c
 * @brief Bythos Protocol v3.0.0 — C Wrapper Implementation
 *
 * Implementação do wrapper C para o protocolo Bythos.
 * Este ficheiro contém a lógica de negócio para construção,
 * validação e parsing de mensagens Bythos.
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 3.0.0
 */

#include "bythos.h"
#include <string.h>

// ============================================================================
// TABELAS CRC (CCITT)
// ============================================================================

static const uint16_t crc16_table[256] = {
    0x0000, 0x1021, 0x2042, 0x3063, 0x4084, 0x50A5, 0x60C6, 0x70E7,
    0x8108, 0x9129, 0xA14A, 0xB16B, 0xC18C, 0xD1AD, 0xE1CE, 0xF1EF,
    0x1231, 0x0210, 0x3273, 0x2252, 0x52B5, 0x4294, 0x72F7, 0x62D6,
    0x9339, 0x8318, 0xB37B, 0xA35A, 0xD3BD, 0xC39C, 0xF3FF, 0xE3DE,
    0x2462, 0x3443, 0x0420, 0x1401, 0x64E6, 0x74C7, 0x44A4, 0x5485,
    0xA56A, 0xB54B, 0x8528, 0x9509, 0xE5EE, 0xF5CF, 0xC5AC, 0xD58D,
    0x3653, 0x2672, 0x1611, 0x0630, 0x76D7, 0x66F6, 0x5695, 0x46B4,
    0xB75B, 0xA77A, 0x9719, 0x8738, 0xF7DF, 0xE7FE, 0xD79D, 0xC7BC,
    0x48C4, 0x58E5, 0x6886, 0x78A7, 0x0840, 0x1861, 0x2802, 0x3823,
    0xC9CC, 0xD9ED, 0xE98E, 0xF9AF, 0x8948, 0x9969, 0xA90A, 0xB92B,
    0x5AF5, 0x4AD4, 0x7AB7, 0x6A96, 0x1A71, 0x0A50, 0x3A33, 0x2A12,
    0xDBFD, 0xCBDC, 0xFBBF, 0xEB9E, 0x9B79, 0x8B58, 0xBB3B, 0xAB1A,
    0x6CA6, 0x7C87, 0x4CE4, 0x5CC5, 0x2C22, 0x3C03, 0x0C60, 0x1C41,
    0xEDAE, 0xFD8F, 0xCDEC, 0xDDCD, 0xAD2A, 0xBD0B, 0x8D68, 0x9D49,
    0x7E97, 0x6EB6, 0x5ED5, 0x4EF4, 0x3E13, 0x2E32, 0x1E51, 0x0E70,
    0xFF9F, 0xEFBE, 0xDFDD, 0xCFFC, 0xBF1B, 0xAF3A, 0x9F59, 0x8F78,
    0x9188, 0x81A9, 0xB1CA, 0xA1EB, 0xD10C, 0xC12D, 0xF14E, 0xE16F,
    0x1080, 0x00A1, 0x30C2, 0x20E3, 0x5004, 0x4025, 0x7046, 0x6067,
    0x83B9, 0x9398, 0xA3FB, 0xB3DA, 0xC33D, 0xD31C, 0xE37F, 0xF35E,
    0x02B1, 0x1290, 0x22F3, 0x32D2, 0x4235, 0x5214, 0x6277, 0x7256,
    0xB5EA, 0xA5CB, 0x95A8, 0x8589, 0xF56E, 0xE54F, 0xD52C, 0xC50D,
    0x34E2, 0x24C3, 0x14A0, 0x0481, 0x7466, 0x6447, 0x5424, 0x4405,
    0xA7DB, 0xB7FA, 0x8799, 0x97B8, 0xE75F, 0xF77E, 0xC71D, 0xD73C,
    0x26D3, 0x36F2, 0x0691, 0x16B0, 0x6657, 0x7676, 0x4615, 0x5634,
    0xD94C, 0xC96D, 0xF90E, 0xE92F, 0x99C8, 0x89E9, 0xB98A, 0xA9AB,
    0x5844, 0x4865, 0x7806, 0x6827, 0x18C0, 0x08E1, 0x3882, 0x28A3,
    0xCB7D, 0xDB5C, 0xEB3F, 0xFB1E, 0x8BF9, 0x9BD8, 0xABBB, 0xBB9A,
    0x4A75, 0x5A54, 0x6A37, 0x7A16, 0x0AF1, 0x1AD0, 0x2AB3, 0x3A92,
    0xFD2E, 0xED0F, 0xDD6C, 0xCD4D, 0xBDAA, 0xAD8B, 0x9DE8, 0x8DC9,
    0x7C26, 0x6C07, 0x5C64, 0x4C45, 0x3CA2, 0x2C83, 0x1CE0, 0x0CC1,
    0xEF1F, 0xFF3E, 0xCF5D, 0xDF7C, 0xAF9B, 0xBFBA, 0x8FD9, 0x9FF8,
    0x6E17, 0x7E36, 0x4E55, 0x5E74, 0x2E93, 0x3EB2, 0x0ED1, 0x1EF0
};

/// Tabela CRC-8/SMBUS (polinómio 0x07) — 256 entradas.
/// Marcação `const` permite ao compilador colocar em flash (ROM)
/// em microcontroladores, poupando memória RAM valiosa.
static const uint8_t crc8_table[256] = {
    0x00, 0x07, 0x0E, 0x09, 0x1C, 0x1B, 0x12, 0x15,
    0x38, 0x3F, 0x36, 0x31, 0x24, 0x23, 0x2A, 0x2D,
    0x70, 0x77, 0x7E, 0x79, 0x6C, 0x6B, 0x62, 0x65,
    0x48, 0x4F, 0x46, 0x41, 0x54, 0x53, 0x5A, 0x5D,
    0xE0, 0xE7, 0xEE, 0xE9, 0xFC, 0xFB, 0xF2, 0xF5,
    0xD8, 0xDF, 0xD6, 0xD1, 0xC4, 0xC3, 0xCA, 0xCD,
    0x90, 0x97, 0x9E, 0x99, 0x8C, 0x8B, 0x82, 0x85,
    0xA8, 0xAF, 0xA6, 0xA1, 0xB4, 0xB3, 0xBA, 0xBD,
    0xC7, 0xC0, 0xC9, 0xCE, 0xDB, 0xDC, 0xD5, 0xD2,
    0xFF, 0xF8, 0xF1, 0xF6, 0xE3, 0xE4, 0xED, 0xEA,
    0xB7, 0xB0, 0xB9, 0xBE, 0xAB, 0xAC, 0xA5, 0xA2,
    0x8F, 0x88, 0x81, 0x86, 0x93, 0x94, 0x9D, 0x9A,
    0x27, 0x20, 0x29, 0x2E, 0x3B, 0x3C, 0x35, 0x32,
    0x1F, 0x18, 0x11, 0x16, 0x03, 0x04, 0x0D, 0x0A,
    0x57, 0x50, 0x59, 0x5E, 0x4B, 0x4C, 0x45, 0x42,
    0x6F, 0x68, 0x61, 0x66, 0x73, 0x74, 0x7D, 0x7A,
    0x89, 0x8E, 0x87, 0x80, 0x95, 0x92, 0x9B, 0x9C,
    0xB1, 0xB6, 0xBF, 0xB8, 0xAD, 0xAA, 0xA3, 0xA4,
    0xF9, 0xFE, 0xF7, 0xF0, 0xE5, 0xE2, 0xEB, 0xEC,
    0xC1, 0xC6, 0xCF, 0xC8, 0xDD, 0xDA, 0xD3, 0xD4,
    0x69, 0x6E, 0x67, 0x60, 0x75, 0x72, 0x7B, 0x7C,
    0x51, 0x56, 0x5F, 0x58, 0x4D, 0x4A, 0x43, 0x44,
    0x19, 0x1E, 0x17, 0x10, 0x05, 0x02, 0x0B, 0x0C,
    0x21, 0x26, 0x2F, 0x28, 0x3D, 0x3A, 0x33, 0x34,
    0x4E, 0x49, 0x40, 0x47, 0x52, 0x55, 0x5C, 0x5B,
    0x76, 0x71, 0x78, 0x7F, 0x6A, 0x6D, 0x64, 0x63,
    0x3E, 0x39, 0x30, 0x37, 0x22, 0x25, 0x2C, 0x2B,
    0x06, 0x01, 0x08, 0x0F, 0x1A, 0x1D, 0x14, 0x13,
    0xAE, 0xA9, 0xA0, 0xA7, 0xB2, 0xB5, 0xBC, 0xBB,
    0x96, 0x91, 0x98, 0x9F, 0x8A, 0x8D, 0x84, 0x83,
    0xDE, 0xD9, 0xD0, 0xD7, 0xC2, 0xC5, 0xCC, 0xCB,
    0xE6, 0xE1, 0xE8, 0xEF, 0xFA, 0xFD, 0xF4, 0xF3
};

// ============================================================================
// TABELA DE VALIDAÇÃO DE MSG_ID
// ============================================================================

static const uint8_t valid_msg_ids[] = {
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x1B
};
#define NUM_VALID_MSG_IDS (sizeof(valid_msg_ids) / sizeof(valid_msg_ids[0]))

// ============================================================================
// FUNÇÕES AUXILIARES
// ============================================================================

/**
 * Verifica se um msg_id é válido.
 */
static int is_valid_msg_id(uint8_t id) {
    for (size_t i = 0; i < NUM_VALID_MSG_IDS; i++) {
        if (valid_msg_ids[i] == id) return 1;
    }
    return 0;
}

/**
 * Verifica se um FieldID tem tipo válido (bits 7-5 = 0-7).
 */
/**
 * Verifica se um field_id tem um tipo válido (bits 7-5).
 *
 * Nota: como os bits 7-5 formam apenas 3 bits (valores 0-7), esta
 * validação é trivialmente verdadeira para qualquer byte. A função
 * é mantida por consistência com a API Rust (is_valid_field_id) e
 * para futura expansão caso o protocolo alargue o espaço de tipos.
 *
 * A validação específica do domínio (ex: GPS, IMU) é responsibility
 * da aplicação, não do protocolo.
 *
 * @param field_id ID do campo (8 bits)
 * @return         1 se válido (sempre true para qualquer byte)
 */
static int is_valid_field_id(uint8_t field_id) {
    uint8_t field_type = (field_id >> 5) & 0x07;
    return field_type <= 7;
}

// ============================================================================
// CRC
// ============================================================================

uint16_t bythos_calc_crc16(const uint8_t* data, size_t len) {
    if (data == NULL || len == 0) return 0xFFFF;

    uint16_t crc = 0xFFFF;
    for (size_t i = 0; i < len; i++) {
        crc = (crc << 8) ^ crc16_table[((crc >> 8) ^ data[i]) & 0xFF];
    }
    return crc;
}

uint8_t bythos_calc_crc8(const uint8_t* data, size_t len) {
    if (data == NULL || len == 0) return 0x00;

    uint8_t crc = 0x00;
    for (size_t i = 0; i < len; i++) {
        crc = crc8_table[crc ^ data[i]];
    }
    return crc;
}

// ============================================================================
// ASSINATURA
// ============================================================================

uint8_t bythos_signature_compute(uint8_t key, uint8_t msg_id, uint8_t seq_lo, uint8_t seq_hi) {
    return key ^ msg_id ^ seq_lo ^ seq_hi;
}

uint8_t bythos_signature_validate(uint8_t signature, uint8_t key, uint8_t msg_id, uint8_t seq_lo, uint8_t seq_hi) {
    return (signature == bythos_signature_compute(key, msg_id, seq_lo, seq_hi)) ? 1 : 0;
}

// ============================================================================
// FIELDID COM TIPO
// ============================================================================

uint8_t bythos_field_id_encode(uint8_t field_type, uint8_t field_id) {
    if (field_type > 7 || field_id > 31) return 0xFF;
    return (field_type << 5) | (field_id & 0x1F);
}

void bythos_field_id_decode(uint8_t field_id, uint8_t* type_out, uint8_t* id_out) {
    if (type_out == NULL || id_out == NULL) return;
    *type_out = (field_id >> 5) & 0x07;
    *id_out = field_id & 0x1F;
}

uint8_t bythos_field_id_valid(uint8_t field_id) {
    return is_valid_field_id(field_id) ? 1 : 0;
}

// ============================================================================
// CAN ID
// ============================================================================

/**
 * Constrói um CAN ID extended (29-bit) a partir dos seus componentes.
 *
 * Formato do CAN ID:
 *   Bits 28-26: Prioridade (3 bits, 0-4)
 *   Bits 25-22: Grupo de origem (4 bits, 0x0-0xF)
 *   Bits 21-18: Grupo de destino (4 bits, 0x0-0xF)
 *   Bits 17-14: Tipo de mensagem (4 bits, 0x0-0x7)
 *   Bits 13-0:  Reservado (14 bits)
 *
 * @param priority   Nível de prioridade (0-4)
 * @param src_group  Grupo de origem (0x0-0xF)
 * @param dst_group  Grupo de destino (0x0=broadcast)
 * @param msg_type   Tipo de mensagem (0x0-0x7)
 * @return           CAN ID de 29 bits
 */
uint32_t bythos_can_id_make(uint8_t priority, uint8_t src_group, uint8_t dst_group, uint8_t msg_type) {
    if (priority > 4 || src_group > 0xF || dst_group > 0xF || msg_type > 0x7) {
        return 0;
    }
    return ((uint32_t)priority << 26) | ((uint32_t)src_group << 22) |
           ((uint32_t)dst_group << 18) | (((uint32_t)msg_type & 0x0F) << 14);
}

/**
 * Extrai a prioridade de um CAN ID extended.
 * Bits 28-26 → valor 0-4
 */
uint8_t bythos_can_id_priority(uint32_t can_id) {
    return (can_id >> 26) & 0x07;
}

/**
 * Extrai o grupo de origem de um CAN ID extended.
 * Bits 25-22 → valor 0x0-0xF
 */
uint8_t bythos_can_id_src(uint32_t can_id) {
    return (can_id >> 22) & 0x0F;
}

/**
 * Extrai o grupo de destino de um CAN ID extended.
 * Bits 21-18 → valor 0x0-0xF
 */
uint8_t bythos_can_id_dst(uint32_t can_id) {
    return (can_id >> 18) & 0x0F;
}

/**
 * Extrai o tipo de mensagem de um CAN ID extended.
 * Bits 17-14 → valor 0x0-0x7
 */
uint8_t bythos_can_id_type(uint32_t can_id) {
    return (can_id >> 14) & 0x0F;
}

/**
 * Verifica se o CAN ID pertence ao bus de segurança (msg_type == Safety = 0x7).
 */
uint8_t bythos_is_safety_bus(uint32_t can_id) {
    return (bythos_can_id_type(can_id) == BYTHOS_CAN_SAFETY) ? 1 : 0;
}

// ============================================================================
// SERIALIZAÇÃO
// ============================================================================

bythos_ssize_t bythos_build(const BythosMessage* msg, uint8_t msg_id, uint8_t signature_key,
                           uint8_t* buffer, size_t buffer_size) {
    if (msg == NULL || buffer == NULL) return -1;
    if (!is_valid_msg_id(msg_id)) return -1;
    if (buffer_size < BYTHOS_OVERHEAD) return -1;

    size_t offset = 0;

    // --- Cabeçalho Bythos (7 bytes) ---
    buffer[offset++] = BYTHOS_START_BYTE;
    buffer[offset++] = BYTHOS_VERSION;
    buffer[offset++] = msg->node_id;
    buffer[offset++] = msg_id;

    // SEQ_NUM (2 bytes, little-endian)
    buffer[offset++] = (uint8_t)(msg->seq_num & 0xFF);
    buffer[offset++] = (uint8_t)((msg->seq_num >> 8) & 0xFF);

    buffer[offset++] = msg->tlv_count;

    // --- Campos TLV ---
    for (uint8_t i = 0; i < msg->tlv_count && i < BYTHOS_MAX_TLV_FIELDS; i++) {
        const BythosTLVField* tlv = &msg->tlvs[i];
        if (offset + 2 + tlv->len > buffer_size) return -1;
        buffer[offset++] = tlv->id;
        buffer[offset++] = tlv->len;
        memcpy(&buffer[offset], tlv->data, tlv->len);
        offset += tlv->len;
    }

    // --- Assinatura ---
    uint8_t signature = bythos_signature_compute(
        signature_key, msg_id,
        (uint8_t)(msg->seq_num & 0xFF),
        (uint8_t)((msg->seq_num >> 8) & 0xFF)
    );
    buffer[offset++] = signature;

    // --- CRC16 ---
    uint16_t crc = bythos_calc_crc16(buffer, offset);
    buffer[offset++] = (uint8_t)(crc & 0xFF);
    buffer[offset++] = (uint8_t)((crc >> 8) & 0xFF);

    return (bythos_ssize_t)offset;
}

/**
 * Valida uma mensagem Bythos completa num buffer.
 *
 * Verifica: byte de início, versão, msg_id, tlv_count,
 * integridade dos campos TLV (bounds e tamanhos) e CRC16.
 * Alinhado com a implementação Rust validate_message().
 *
 * Nota: o CRC16 é validado sobre o tamanho real da mensagem calculado
 * a partir dos TLVs, NÃO sobre o tamanho do buffer fornecido. Isto
 * permite que o buffer contenha dados adicionais sem afetar a validação.
 *
 * @param buffer  Buffer contendo a mensagem
 * @param length  Tamanho do buffer (mínimo BYTHOS_OVERHEAD)
 * @return        tlv_count se válido, 0xFF em erro
 */
uint8_t bythos_validate(const uint8_t* buffer, size_t length) {
    if (buffer == NULL || length < BYTHOS_OVERHEAD) return 0xFF;

    // Validar byte de início
    if (buffer[0] != BYTHOS_START_BYTE) return 0xFF;

    // Validar versão
    if (buffer[1] != BYTHOS_VERSION) return 0xFF;

    // Validar msg_id
    if (!is_valid_msg_id(buffer[3])) return 0xFF;

    // Validar tlv_count
    uint8_t tlv_count = buffer[6];
    if (tlv_count > BYTHOS_MAX_TLV_FIELDS) return 0xFF;

    // Percorrer campos TLV validando bounds e calculando tamanho real
    size_t offset = BYTHOS_HEADER_SIZE;
    for (uint8_t i = 0; i < tlv_count; i++) {
        if (offset + BYTHOS_TLV_HEADER_SIZE > length) return 0xFF;
        uint8_t field_len = buffer[offset + 1];
        if (field_len > BYTHOS_MAX_TLV_DATA) return 0xFF;
        offset += BYTHOS_TLV_HEADER_SIZE + field_len;
    }

    // offset contém agora BYTHOS_HEADER_SIZE + soma dos TLVs
    // Tamanho total da mensagem = offset + signature(1) + crc16(2)
    size_t msg_size = offset + BYTHOS_SIGNATURE_SIZE + BYTHOS_CRC16_SIZE;
    if (length < msg_size) return 0xFF;

    // Validar CRC16 sobre o tamanho real da mensagem (não o buffer inteiro)
    size_t crc_offset = msg_size - BYTHOS_CRC16_SIZE;
    uint16_t received_crc = (uint16_t)buffer[crc_offset] | ((uint16_t)buffer[crc_offset + 1] << 8);
    uint16_t computed_crc = bythos_calc_crc16(buffer, crc_offset);
    if (received_crc != computed_crc) return 0xFF;

    return tlv_count;
}

/**
 * Valida a assinatura de uma mensagem Bythos.
 * Primeiro valida CRC16 e estrutura, depois verifica a assinatura XOR.
 *
 * Utiliza bythos_validate() para obter o tamanho real da mensagem
 * antes de extrair o byte de assinatura, evitando problemas com
 * buffers maiores que a mensagem.
 *
 * @param buffer         Buffer contendo a mensagem
 * @param length         Tamanho do buffer (mínimo BYTHOS_OVERHEAD)
 * @param signature_key  Chave de assinatura
 * @return               1 se válida, 0 caso contrário
 */
uint8_t bythos_validate_signature(const uint8_t* buffer, size_t length, uint8_t signature_key) {
    if (buffer == NULL || length < BYTHOS_OVERHEAD) return 0;

    // Validar estrutura e CRC primeiro — se falhar, assinatura é irrelevante
    if (bythos_validate(buffer, length) == 0xFF) return 0;

    // Calcular tamanho real da mensagem a partir dos TLVs
    uint8_t tlv_count = buffer[6];
    size_t offset = BYTHOS_HEADER_SIZE;
    for (uint8_t i = 0; i < tlv_count; i++) {
        uint8_t field_len = buffer[offset + 1];
        offset += BYTHOS_TLV_HEADER_SIZE + field_len;
    }
    // offset = BYTHOS_HEADER_SIZE + soma dos TLVs
    size_t sig_offset = offset;  // Assinatura está logo após os TLVs

    uint8_t signature = buffer[sig_offset];
    uint8_t msg_id = buffer[3];
    uint8_t seq_lo = buffer[4];
    uint8_t seq_hi = buffer[5];

    return bythos_signature_validate(signature, signature_key, msg_id, seq_lo, seq_hi);
}

void bythos_parse_tlv(const uint8_t* data, size_t length, BythosTLVField* output, size_t* count) {
    if (data == NULL || output == NULL || count == NULL) return;

    size_t capacity = *count;
    size_t offset = 0;
    size_t parsed = 0;

    while (offset + BYTHOS_TLV_HEADER_SIZE <= length && parsed < capacity) {
        uint8_t id = data[offset++];
        uint8_t len = data[offset++];

        if (!is_valid_field_id(id)) break;
        if (len > BYTHOS_MAX_TLV_DATA) break;
        if (offset + len > length) break;

        output[parsed].id = id;
        output[parsed].len = len;
        memcpy(output[parsed].data, &data[offset], len);
        offset += len;
        parsed++;
    }

    *count = parsed;
}

// ============================================================================
// ADICIONAR CAMPOS TLV
// ============================================================================

int8_t bythos_tlv_add(BythosMessage* msg, uint8_t id, const uint8_t* data, uint8_t len) {
    if (msg == NULL || data == NULL) return -1;
    if (!is_valid_field_id(id)) return -1;
    if (len > BYTHOS_MAX_TLV_DATA) return -1;
    if (msg->tlv_count >= BYTHOS_MAX_TLV_FIELDS) return -1;

    BythosTLVField* tlv = &msg->tlvs[msg->tlv_count];
    tlv->id = id;
    tlv->len = len;
    memcpy(tlv->data, data, len);
    msg->tlv_count++;
    return 0;
}

int8_t bythos_tlv_add_f32(BythosMessage* msg, uint8_t id, float value) {
    uint8_t bytes[4];
    bythos_f32_to_bytes(value, bytes);
    return bythos_tlv_add(msg, id, bytes, 4);
}

int8_t bythos_tlv_add_i32(BythosMessage* msg, uint8_t id, int32_t value) {
    uint8_t bytes[4];
    bythos_i32_to_bytes(value, bytes);
    return bythos_tlv_add(msg, id, bytes, 4);
}

int8_t bythos_tlv_add_u32(BythosMessage* msg, uint8_t id, uint32_t value) {
    uint8_t bytes[4];
    bythos_u32_to_bytes(value, bytes);
    return bythos_tlv_add(msg, id, bytes, 4);
}

int8_t bythos_tlv_add_u16(BythosMessage* msg, uint8_t id, uint16_t value) {
    uint8_t bytes[2];
    bythos_u16_to_bytes(value, bytes);
    return bythos_tlv_add(msg, id, bytes, 2);
}

int8_t bythos_tlv_add_u8(BythosMessage* msg, uint8_t id, uint8_t value) {
    return bythos_tlv_add(msg, id, &value, 1);
}

// ============================================================================
// INICIALIZAÇÃO DE MENSAGEM
// ============================================================================

/**
 * Inicializa uma estrutura BythosMessage com os valores por omissão.
 * Zera toda a estrutura antes de definir os campos para evitar dados residuais.
 *
 * @param msg     Apontador para a estrutura a inicializar
 * @param node_id ID do nó (0x0-0xF)
 * @param msg_id  ID da mensagem (0x10-0x1B)
 */
void bythos_init(BythosMessage* msg, uint8_t node_id, uint8_t msg_id) {
    if (msg == NULL) return;
    if (node_id > 0xF) return;
    if (!is_valid_msg_id(msg_id)) return;

    // Zerar toda a estrutura para evitar dados residuais
    memset(msg, 0, sizeof(BythosMessage));

    msg->start_byte = BYTHOS_START_BYTE;
    msg->version = BYTHOS_VERSION;
    msg->node_id = node_id;
    msg->msg_id = msg_id;
}

void bythos_set_seq(BythosMessage* msg, uint16_t seq) {
    if (msg == NULL) return;
    msg->seq_num = seq;
}

void bythos_clear(BythosMessage* msg) {
    if (msg == NULL) return;
    memset(msg, 0, sizeof(BythosMessage));
}

// ============================================================================
// CONVERSÃO DE BYTES
// ============================================================================

void bythos_f32_to_bytes(float value, uint8_t* bytes) {
    if (bytes == NULL) return;
    // memcpy evita type-punning via union (undefined behavior em C)
    memcpy(bytes, &value, sizeof(float));
}

float bythos_bytes_to_f32(const uint8_t* bytes) {
    if (bytes == NULL) return 0.0f;
    // memcpy evita type-punning via union (undefined behavior em C)
    float result;
    memcpy(&result, bytes, sizeof(float));
    return result;
}

void bythos_i32_to_bytes(int32_t value, uint8_t* bytes) {
    if (bytes == NULL) return;
    bytes[0] = (uint8_t)(value & 0xFF);
    bytes[1] = (uint8_t)((value >> 8) & 0xFF);
    bytes[2] = (uint8_t)((value >> 16) & 0xFF);
    bytes[3] = (uint8_t)((value >> 24) & 0xFF);
}

int32_t bythos_bytes_to_i32(const uint8_t* bytes) {
    if (bytes == NULL) return 0;
    return (int32_t)((uint32_t)bytes[0] | ((uint32_t)bytes[1] << 8) |
                     ((uint32_t)bytes[2] << 16) | ((uint32_t)bytes[3] << 24));
}

void bythos_u32_to_bytes(uint32_t value, uint8_t* bytes) {
    if (bytes == NULL) return;
    bytes[0] = (uint8_t)(value & 0xFF);
    bytes[1] = (uint8_t)((value >> 8) & 0xFF);
    bytes[2] = (uint8_t)((value >> 16) & 0xFF);
    bytes[3] = (uint8_t)((value >> 24) & 0xFF);
}

uint32_t bythos_bytes_to_u32(const uint8_t* bytes) {
    if (bytes == NULL) return 0;
    return (uint32_t)bytes[0] | ((uint32_t)bytes[1] << 8) |
           ((uint32_t)bytes[2] << 16) | ((uint32_t)bytes[3] << 24);
}

void bythos_u16_to_bytes(uint16_t value, uint8_t* bytes) {
    if (bytes == NULL) return;
    bytes[0] = (uint8_t)(value & 0xFF);
    bytes[1] = (uint8_t)((value >> 8) & 0xFF);
}

uint16_t bythos_bytes_to_u16(const uint8_t* bytes) {
    if (bytes == NULL) return 0;
    return (uint16_t)bytes[0] | ((uint16_t)bytes[1] << 8);
}

// ============================================================================
// VALIDAÇÃO
// ============================================================================

uint8_t bythos_msg_id_valid(uint8_t id) {
    return is_valid_msg_id(id) ? 1 : 0;
}

/**
 * Retorna a prioridade de uma mensagem com base no seu ID e estado de failsafe.
 *
 * Alinhado com a implementação Rust: quando failsafe está ativo, todas as
 * mensagens (exceto Debug) recebem prioridade SUPER_CRITICAL.
 *
 * @param msg_id          ID da mensagem (0x10-0x1B)
 * @param failsafe_active 1 se failsafe ativo, 0 caso contrário
 * @return                Nível de prioridade (0=SuperCritical..4=Low)
 */
uint8_t bythos_msg_priority(uint8_t msg_id, uint8_t failsafe_active) {
    if (!is_valid_msg_id(msg_id)) return BYTHOS_PRIORITY_LOW;

    // Quando failsafe ativo: Debug=LOW, todos os outros=SUPER_CRITICAL
    if (failsafe_active) {
        if (msg_id == BYTHOS_MSG_DEBUG) return BYTHOS_PRIORITY_LOW;
        return BYTHOS_PRIORITY_SUPER_CRITICAL;
    }

    // Prioridade padrão por tipo de mensagem
    switch (msg_id) {
        case BYTHOS_MSG_HEARTBEAT:  return BYTHOS_PRIORITY_MEDIUM;
        case BYTHOS_MSG_TELEMETRY:  return BYTHOS_PRIORITY_MEDIUM;
        case BYTHOS_MSG_COMMAND:    return BYTHOS_PRIORITY_HIGH;
        case BYTHOS_MSG_ACK:        return BYTHOS_PRIORITY_HIGH;
        case BYTHOS_MSG_FAILSAFE:   return BYTHOS_PRIORITY_SUPER_CRITICAL;
        case BYTHOS_MSG_DEBUG:      return BYTHOS_PRIORITY_LOW;
        case BYTHOS_MSG_VIDEO:      return BYTHOS_PRIORITY_LOW;
        case BYTHOS_MSG_SHELL:      return BYTHOS_PRIORITY_MEDIUM;
        case BYTHOS_MSG_SIDATA:     return BYTHOS_PRIORITY_MEDIUM;
        case BYTHOS_MSG_WATCHDOG:   return BYTHOS_PRIORITY_MEDIUM;
        case BYTHOS_MSG_PING:       return BYTHOS_PRIORITY_MEDIUM;
        case BYTHOS_MSG_CLOCK:      return BYTHOS_PRIORITY_HIGH;
        default:                    return BYTHOS_PRIORITY_LOW;
    }
}

// ============================================================================
// UTILITÁRIOS
// ============================================================================

const char* bythos_version(void) {
    return "3.0.0";
}

size_t bythos_overhead(void) {
    return BYTHOS_OVERHEAD;
}

size_t bythos_max_message_size(void) {
    return BYTHOS_MAX_MESSAGE_SIZE;
}
