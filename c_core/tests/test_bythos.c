/**
 * @file test_bythos.c
 * @brief Bythos Protocol v3.0.0 — Unit Tests
 *
 * Testes unitários abrangentes para o protocolo Bythos v3.0.0.
 * Testa construção, validação, parsing e casos extremos.
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 3.0.0
 */

#include "bythos.h"
#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <math.h>

// ============================================================================
// CONSTANTES DE TESTE
// ============================================================================

#define TEST_KEY     0x42
#define TEST_NODE    0x06
#define TEST_SEQ     42

// ============================================================================
// TESTES — CONSTANTES
// ============================================================================

void test_constants(void) {
    printf("  [1/8] Constantes... ");
    assert(BYTHOS_START_BYTE == 0xAA);
    assert(BYTHOS_VERSION == 0x03);
    assert(BYTHOS_HEADER_SIZE == 7);
    assert(BYTHOS_SIGNATURE_SIZE == 1);
    assert(BYTHOS_CRC16_SIZE == 2);
    assert(BYTHOS_OVERHEAD == 10);
    assert(BYTHOS_MAX_TLV_FIELDS == 32);
    assert(BYTHOS_MAX_TLV_DATA == 32);
    assert(BYTHOS_MAX_MESSAGE_SIZE == 1098);
    printf("OK\n");
}

// ============================================================================
// TESTES — FIELDID
// ============================================================================

void test_field_id_encode_decode(void) {
    printf("  [2/8] FieldID encode/decode... ");
    for (int t = 0; t <= 7; t++) {
        for (int id = 0; id <= 31; id++) {
            uint8_t encoded = bythos_field_id_encode(t, id);
            uint8_t decoded_type, decoded_id;
            bythos_field_id_decode(encoded, &decoded_type, &decoded_id);
            assert(decoded_type == t);
            assert(decoded_id == id);
        }
    }
    printf("OK\n");
}

void test_field_id_invalid(void) {
    printf("  [2/8] FieldID inválido... ");
    assert(bythos_field_id_encode(8, 0) == 0xFF);
    assert(bythos_field_id_encode(0, 32) == 0xFF);
    assert(bythos_field_id_valid(0x00) == 1);
    assert(bythos_field_id_valid(0xE0) == 1);
    printf("OK\n");
}

// ============================================================================
// TESTES — CAN ID
// ============================================================================

void test_can_id(void) {
    printf("  [3/8] CAN ID... ");
    uint32_t can_id = bythos_can_id_make(2, 0x6, 0x0, 0x0);
    assert(bythos_can_id_priority(can_id) == 2);
    assert(bythos_can_id_src(can_id) == 0x6);
    assert(bythos_can_id_dst(can_id) == 0x0);
    assert(bythos_can_id_type(can_id) == 0x0);
    assert(bythos_is_safety_bus(can_id) == 0);

    uint32_t safety_id = bythos_can_id_make(0, 0x3, 0x4, 0x7);
    assert(bythos_is_safety_bus(safety_id) == 1);

    assert(bythos_can_id_make(5, 0, 0, 0) == 0);
    printf("OK\n");
}

// ============================================================================
// TESTES — ASSINATURA
// ============================================================================

void test_signature(void) {
    printf("  [4/8] Assinatura... ");
    uint8_t sig = bythos_signature_compute(0x42, 0x11, 0x2A, 0x00);
    assert(sig == (0x42 ^ 0x11 ^ 0x2A ^ 0x00));

    assert(bythos_signature_validate(sig, 0x42, 0x11, 0x2A, 0x00) == 1);
    assert(bythos_signature_validate(sig, 0x43, 0x11, 0x2A, 0x00) == 0);
    assert(bythos_signature_validate(sig, 0x42, 0x12, 0x2A, 0x00) == 0);

    uint8_t sig_zero = bythos_signature_compute(0x00, 0x10, 0x00, 0x00);
    assert(sig_zero == 0x10);
    printf("OK\n");
}

// ============================================================================
// TESTES — CRC
// ============================================================================

void test_crc16(void) {
    printf("  [5/8] CRC-16... ");
    const uint8_t data[] = "123456789";
    assert(bythos_calc_crc16(data, 9) == 0x29B1);

    uint8_t data1[] = {0x01, 0x02, 0x03};
    uint8_t data2[] = {0x01, 0x02, 0x04};
    assert(bythos_calc_crc16(data1, 3) != bythos_calc_crc16(data2, 3));
    printf("OK\n");
}

// ============================================================================
// TESTES — BUILDER + VALIDATE
// ============================================================================

void test_builder_basic(void) {
    printf("  [6/8] Builder básico... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xC0, 2);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));

    assert(size > 0);
    assert(buffer[0] == BYTHOS_START_BYTE);
    assert(buffer[1] == BYTHOS_VERSION);
    assert(buffer[2] == TEST_NODE);
    assert(buffer[3] == BYTHOS_MSG_TELEMETRY);

    assert(bythos_validate(buffer, size) == 1);
    assert(bythos_validate_signature(buffer, size, TEST_KEY) == 1);
    printf("OK\n");
}

void test_builder_multiple_tlvs(void) {
    printf("  [6/8] Builder múltiplos TLVs... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, TEST_SEQ);

    bythos_tlv_add_f32(&msg, 0x26, 40.0f);    // Latitude
    bythos_tlv_add_f32(&msg, 0x27, -8.0f);    // Longitude
    bythos_tlv_add_u8(&msg, 0xC0, 4);         // State
    bythos_tlv_add_u8(&msg, 0xC1, 3);         // Mode
    bythos_tlv_add_u32(&msg, 0xC2, 3600);     // Uptime
    bythos_tlv_add_u16(&msg, 0xA0, 42);       // FrameId

    assert(msg.tlv_count == 6);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    assert(size > 0);
    assert(bythos_validate(buffer, size) != 0xFF);
    printf("OK\n");
}

void test_builder_overflow(void) {
    printf("  [6/8] Builder overflow... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);

    for (int i = 0; i < BYTHOS_MAX_TLV_FIELDS; i++) {
        bythos_tlv_add_u8(&msg, i, 0);
    }

    assert(bythos_tlv_add_u8(&msg, 0xFF, 0) == -1);
    printf("OK\n");
}

void test_builder_invalid_msg_id(void) {
    printf("  [6/8] Builder msg_id inválido... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, 0xFF, TEST_KEY, buffer, sizeof(buffer));
    assert(size == -1);
    printf("OK\n");
}

// ============================================================================
// TESTES — PARSER
// ============================================================================

void test_parser_full_message(void) {
    printf("  [7/8] Parser mensagem completa... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xC0, 2);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    assert(size > 0);

    // Simular parser byte a byte
    BythosMessage parsed;
    bythos_clear(&parsed);
    size_t tlv_capacity = BYTHOS_MAX_TLV_FIELDS;

    // Extrair campos TLV do buffer
    size_t tlv_start = BYTHOS_HEADER_SIZE;
    size_t tlv_end = size - BYTHOS_SIGNATURE_SIZE - BYTHOS_CRC16_SIZE;
    bythos_parse_tlv(&buffer[tlv_start], tlv_end - tlv_start, parsed.tlvs, &tlv_capacity);

    assert(tlv_capacity == 1);
    assert(parsed.tlvs[0].id == 0xC0);
    assert(parsed.tlvs[0].len == 1);
    assert(parsed.tlvs[0].data[0] == 2);
    printf("OK\n");
}

void test_parser_invalid_crc(void) {
    printf("  [7/8] Parser CRC inválido... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xC0, 2);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    buffer[size - 1]++; // Corromper CRC

    assert(bythos_validate(buffer, size) == 0xFF);
    assert(bythos_validate_signature(buffer, size, TEST_KEY) == 0);
    printf("OK\n");
}

void test_parser_invalid_signature(void) {
    printf("  [7/8] Parser assinatura inválida... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xC0, 2);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    size_t sig_idx = size - BYTHOS_CRC16_SIZE - BYTHOS_SIGNATURE_SIZE;
    buffer[sig_idx]++; // Corromper assinatura

    assert(bythos_validate_signature(buffer, size, TEST_KEY) == 0);
    printf("OK\n");
}

// ============================================================================
// TESTES — ROUNDTRIP
// ============================================================================

void test_roundtrip(void) {
    printf("  [8/8] Roundtrip completo... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, TEST_SEQ);

    bythos_tlv_add_f32(&msg, 0x26, 40.0f);
    bythos_tlv_add_f32(&msg, 0x27, -8.0f);
    bythos_tlv_add_u8(&msg, 0xC0, 4);
    bythos_tlv_add_u8(&msg, 0xC1, 3);
    bythos_tlv_add_u32(&msg, 0xC2, 3600);
    bythos_tlv_add_u16(&msg, 0xA0, 42);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    assert(size > 0);

    assert(bythos_validate(buffer, size) != 0xFF);
    assert(bythos_validate_signature(buffer, size, TEST_KEY) == 1);

    // Parse TLVs
    size_t tlv_start = BYTHOS_HEADER_SIZE;
    size_t tlv_end = size - BYTHOS_SIGNATURE_SIZE - BYTHOS_CRC16_SIZE;
    BythosTLVField tlvs[BYTHOS_MAX_TLV_FIELDS];
    size_t tlv_count = BYTHOS_MAX_TLV_FIELDS;
    bythos_parse_tlv(&buffer[tlv_start], tlv_end - tlv_start, tlvs, &tlv_count);

    assert(tlv_count == 6);
    assert(tlvs[0].id == 0x26);
    assert(tlvs[2].id == 0xC0);
    assert(tlvs[2].data[0] == 4);

    // Verificar float roundtrip
    float lat = bythos_bytes_to_f32(tlvs[0].data);
    assert(fabs(lat - 40.0f) < 0.001f);

    printf("OK\n");
}

// ============================================================================
// TESTES — VALIDAÇÃO ADICIONAL
// ============================================================================

/**
 * Testa validação com tlv_count inválido (> 32).
 * Deve retornar 0xFF (erro).
 */
void test_validate_tlv_count_overflow(void) {
    printf("  [9/9] Validate tlv_count overflow... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xC0, 2);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    assert(size > 0);

    // Corromper tlv_count para 33 (> BYTHOS_MAX_TLV_FIELDS)
    buffer[6] = 33;
    // Recalcular CRC16 para que a validação de CRC passe até ao check TLV
    uint16_t crc = bythos_calc_crc16(buffer, size - BYTHOS_CRC16_SIZE);
    buffer[size - 2] = (uint8_t)(crc & 0xFF);
    buffer[size - 1] = (uint8_t)((crc >> 8) & 0xFF);

    assert(bythos_validate(buffer, size) == 0xFF);
    printf("OK\n");
}

/**
 * Testa validação com dados TLV corrompidos.
 * Deve retornar 0xFF (erro) quando o parser encontra bounds inválidos.
 */
void test_validate_corrupt_tlv_data(void) {
    printf("  [9/9] Validate TLV data corrupto... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xC0, 2);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    assert(size > 0);

    // Corromper o tamanho do TLV para um valor absurdo
    buffer[8] = 0xFF; // field_len = 255
    // Recalcular CRC
    uint16_t crc = bythos_calc_crc16(buffer, size - BYTHOS_CRC16_SIZE);
    buffer[size - 2] = (uint8_t)(crc & 0xFF);
    buffer[size - 1] = (uint8_t)((crc >> 8) & 0xFF);

    assert(bythos_validate(buffer, size) == 0xFF);
    printf("OK\n");
}

/**
 * Testa validação de assinatura com chave errada.
 * Deve retornar 0 (falso).
 */
void test_validate_signature_wrong_key(void) {
    printf("  [9/9] Validate signature chave errada... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xC0, 2);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, TEST_KEY, buffer, sizeof(buffer));
    assert(size > 0);

    // Chave errada
    assert(bythos_validate_signature(buffer, size, TEST_KEY + 1) == 0);
    printf("OK\n");
}

/**
 * Testa inicialização com parâmetros inválidos.
 * Deve manter a estrutura inalterada.
 */
void test_init_invalid_params(void) {
    printf("  [9/9] Init parâmetros inválidos... ");
    BythosMessage msg;

    // NULL pointer
    bythos_init(NULL, TEST_NODE, BYTHOS_MSG_TELEMETRY);

    // node_id > 0xF
    bythos_init(&msg, 0x10, BYTHOS_MSG_TELEMETRY);
    assert(msg.start_byte != BYTHOS_START_BYTE);

    // msg_id inválido
    bythos_init(&msg, TEST_NODE, 0x0F);
    assert(msg.start_byte != BYTHOS_START_BYTE);

    // Valores válidos
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    assert(msg.start_byte == BYTHOS_START_BYTE);
    assert(msg.node_id == TEST_NODE);
    assert(msg.msg_id == BYTHOS_MSG_TELEMETRY);
    assert(msg.tlv_count == 0);
    printf("OK\n");
}

/**
 * Testa clear() — verifica que a estrutura é resetada.
 */
void test_clear_message(void) {
    printf("  [9/9] Clear mensagem... ");
    BythosMessage msg;
    bythos_init(&msg, TEST_NODE, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, TEST_SEQ);
    bythos_tlv_add_u8(&msg, 0xC0, 2);
    assert(msg.tlv_count == 1);

    bythos_clear(&msg);
    assert(msg.tlv_count == 0);
    assert(msg.seq_num == 0);
    assert(msg.signature == 0);
    assert(msg.checksum == 0);
    printf("OK\n");
}

/**
 * Testa prioridade de mensagens alinhada com Rust.
 * Verifica que os valores coincidem com get_msg_priority() em Rust.
 */
void test_msg_priority_alignment(void) {
    printf("  [9/9] MsgPriority alinhamento... ");
    // Sem failsafe
    assert(bythos_msg_priority(BYTHOS_MSG_HEARTBEAT, 0) == BYTHOS_PRIORITY_MEDIUM);
    assert(bythos_msg_priority(BYTHOS_MSG_TELEMETRY, 0) == BYTHOS_PRIORITY_MEDIUM);
    assert(bythos_msg_priority(BYTHOS_MSG_COMMAND, 0) == BYTHOS_PRIORITY_HIGH);
    assert(bythos_msg_priority(BYTHOS_MSG_ACK, 0) == BYTHOS_PRIORITY_HIGH);
    assert(bythos_msg_priority(BYTHOS_MSG_FAILSAFE, 0) == BYTHOS_PRIORITY_SUPER_CRITICAL);
    assert(bythos_msg_priority(BYTHOS_MSG_DEBUG, 0) == BYTHOS_PRIORITY_LOW);
    assert(bythos_msg_priority(BYTHOS_MSG_VIDEO, 0) == BYTHOS_PRIORITY_LOW);
    assert(bythos_msg_priority(BYTHOS_MSG_SHELL, 0) == BYTHOS_PRIORITY_MEDIUM);
    assert(bythos_msg_priority(BYTHOS_MSG_SIDATA, 0) == BYTHOS_PRIORITY_MEDIUM);
    assert(bythos_msg_priority(BYTHOS_MSG_WATCHDOG, 0) == BYTHOS_PRIORITY_MEDIUM);
    assert(bythos_msg_priority(BYTHOS_MSG_PING, 0) == BYTHOS_PRIORITY_MEDIUM);
    assert(bythos_msg_priority(BYTHOS_MSG_CLOCK, 0) == BYTHOS_PRIORITY_HIGH);

    // Com failsafe: Debug=LOW, todos os outros=SUPER_CRITICAL
    assert(bythos_msg_priority(BYTHOS_MSG_DEBUG, 1) == BYTHOS_PRIORITY_LOW);
    assert(bythos_msg_priority(BYTHOS_MSG_HEARTBEAT, 1) == BYTHOS_PRIORITY_SUPER_CRITICAL);
    assert(bythos_msg_priority(BYTHOS_MSG_TELEMETRY, 1) == BYTHOS_PRIORITY_SUPER_CRITICAL);
    assert(bythos_msg_priority(BYTHOS_MSG_COMMAND, 1) == BYTHOS_PRIORITY_SUPER_CRITICAL);
    assert(bythos_msg_priority(BYTHOS_MSG_ACK, 1) == BYTHOS_PRIORITY_SUPER_CRITICAL);

    // msg_id inválido
    assert(bythos_msg_priority(0x0F, 0) == BYTHOS_PRIORITY_LOW);
    printf("OK\n");
}

/**
 * Testa CAN ID com msg_type de 4 bits (bits 17-14).
 * Verifica alinhamento com o layout Rust.
 */
void test_can_id_4bit_msg_type(void) {
    printf("  [9/9] CAN ID 4-bit msg_type... ");
    // Testar valores de msg_type que usam 4 bits
    uint32_t can_id0 = bythos_can_id_make(1, 0x2, 0x3, 0x0);
    assert(bythos_can_id_type(can_id0) == 0x0);

    uint32_t can_id7 = bythos_can_id_make(1, 0x2, 0x3, 0x7);
    assert(bythos_can_id_type(can_id7) == 0x7);

    // Verificar que bits 17-14 são usados (não 19-16)
    uint32_t can_id_test = bythos_can_id_make(0, 0, 0, 0x5);
    uint32_t expected = 0x5 << 14;
    assert((can_id_test & 0x0007C000) == expected);

    // Verificar roundtrip completo
    uint8_t p = bythos_can_id_priority(can_id7);
    uint8_t s = bythos_can_id_src(can_id7);
    uint8_t d = bythos_can_id_dst(can_id7);
    uint8_t t = bythos_can_id_type(can_id7);
    assert(p == 1);
    assert(s == 0x2);
    assert(d == 0x3);
    assert(t == 0x7);
    printf("OK\n");
}

/**
 * Testa constantes FieldId unificadas — correspondência com Rust.
 * Verifica que cada #define BYTHOS_FIELD_* coincide com o enum Rust FieldId.
 */
void test_field_id_constants_alignment(void) {
    printf("  [9/9] FieldId constants alinhamento... ");
    // GPS
    assert(BYTHOS_FIELD_GPS_LATITUDE == 0x26);
    assert(BYTHOS_FIELD_GPS_LONGITUDE == 0x27);
    assert(BYTHOS_FIELD_GPS_ALTITUDE == 0x28);
    assert(BYTHOS_FIELD_GPS_SPEED == 0x29);
    assert(BYTHOS_FIELD_GPS_COURSE == 0x2A);
    assert(BYTHOS_FIELD_GPS_SATELLITES == 0xC7);
    assert(BYTHOS_FIELD_GPS_HDOP == 0x2B);
    // IMU
    assert(BYTHOS_FIELD_IMU_ROLL == 0x30);
    assert(BYTHOS_FIELD_IMU_PITCH == 0x31);
    assert(BYTHOS_FIELD_IMU_YAW == 0x32);
    assert(BYTHOS_FIELD_IMU_ACCEL_X == 0x33);
    assert(BYTHOS_FIELD_IMU_ACCEL_Y == 0x34);
    assert(BYTHOS_FIELD_IMU_ACCEL_Z == 0x35);
    assert(BYTHOS_FIELD_IMU_GYRO_X == 0x36);
    assert(BYTHOS_FIELD_IMU_GYRO_Y == 0x37);
    assert(BYTHOS_FIELD_IMU_GYRO_Z == 0x38);
    assert(BYTHOS_FIELD_IMU_YAW_RATE == 0x39);
    // Voo
    assert(BYTHOS_FIELD_FLIGHT_ALT_GPS == 0x40);
    assert(BYTHOS_FIELD_FLIGHT_ALT_BARO == 0x41);
    assert(BYTHOS_FIELD_FLIGHT_VSPEED == 0x42);
    assert(BYTHOS_FIELD_FLIGHT_AIRSPEED == 0x43);
    assert(BYTHOS_FIELD_FLIGHT_LOOPTIME == 0xA2);
    // Energia
    assert(BYTHOS_FIELD_POWER_BATT_V == 0x50);
    assert(BYTHOS_FIELD_POWER_BATT_I == 0x51);
    assert(BYTHOS_FIELD_POWER_BATT_CONS == 0x52);
    assert(BYTHOS_FIELD_POWER_BATT_TEMP == 0x53);
    assert(BYTHOS_FIELD_POWER_BATT_SOC == 0x54);
    // Temperatura
    assert(BYTHOS_FIELD_TEMP_1 == 0x60);
    assert(BYTHOS_FIELD_TEMP_2 == 0x61);
    assert(BYTHOS_FIELD_TEMP_3 == 0x62);
    assert(BYTHOS_FIELD_TEMP_4 == 0x63);
    assert(BYTHOS_FIELD_TEMP_ESP1 == 0x64);
    assert(BYTHOS_FIELD_TEMP_ESP2 == 0x65);
    // Sistema
    assert(BYTHOS_FIELD_SYSTEM_STATE == 0xC0);
    assert(BYTHOS_FIELD_SYSTEM_MODE == 0xC1);
    assert(BYTHOS_FIELD_SYSTEM_UPTIME == 0x82);
    assert(BYTHOS_FIELD_SYSTEM_FREE_HEAP == 0x83);
    assert(BYTHOS_FIELD_SYSTEM_CPU_LOAD == 0xC4);
    assert(BYTHOS_FIELD_SYSTEM_ESP1_LOAD == 0xC5);
    assert(BYTHOS_FIELD_SYSTEM_ESP2_LOAD == 0xC6);
    // Failsafe
    assert(BYTHOS_FIELD_FAILSAFE_REASON == 0xC8);
    assert(BYTHOS_FIELD_FAILSAFE_ACTION == 0xC9);
    assert(BYTHOS_FIELD_FAILSAFE_STATE == 0xCA);
    // Video
    assert(BYTHOS_FIELD_VIDEO_FRAME_ID == 0xA0);
    assert(BYTHOS_FIELD_VIDEO_CHUNK_ID == 0xC3);
    assert(BYTHOS_FIELD_VIDEO_TOTAL_CHUNKS == 0xCB);
    assert(BYTHOS_FIELD_VIDEO_PAYLOAD == 0x00);
    // Verificar encode/decode roundtrip para cada constante
    uint8_t type_out, id_out;
    bythos_field_id_decode(BYTHOS_FIELD_GPS_LATITUDE, &type_out, &id_out);
    assert(type_out == 1 && id_out == 6);
    bythos_field_id_decode(BYTHOS_FIELD_SYSTEM_STATE, &type_out, &id_out);
    assert(type_out == 6 && id_out == 0);
    bythos_field_id_decode(BYTHOS_FIELD_VIDEO_PAYLOAD, &type_out, &id_out);
    assert(type_out == 0 && id_out == 0);
    printf("OK\n");
}

// ============================================================================
// MAIN
// ============================================================================

int main(void) {
    printf("Bythos Protocol v3.0.0 — Unit Tests\n");
    printf("====================================\n\n");

    test_constants();
    test_field_id_encode_decode();
    test_field_id_invalid();
    test_can_id();
    test_signature();
    test_crc16();
    test_builder_basic();
    test_builder_multiple_tlvs();
    test_builder_overflow();
    test_builder_invalid_msg_id();
    test_parser_full_message();
    test_parser_invalid_crc();
    test_parser_invalid_signature();
    test_roundtrip();
    test_validate_tlv_count_overflow();
    test_validate_corrupt_tlv_data();
    test_validate_signature_wrong_key();
    test_init_invalid_params();
    test_clear_message();
    test_msg_priority_alignment();
    test_can_id_4bit_msg_type();
    test_field_id_constants_alignment();

    printf("\nTodos os testes passaram!\n");
    return 0;
}
