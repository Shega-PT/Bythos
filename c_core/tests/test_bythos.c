/**
 * @file test_bythos.c
 * @brief Bythos Protocol v4.0.0 — Testes unitários C
 *
 * Cobre construção, selagem, validação ponta-a-ponta, análise, anel e túnel.
 * O teste do vetor dourado ancora a interop Rust↔C: os mesmos bytes que o
 * teste `test_vetor_dourado_v4` do Rust — se divergirem, um dos lados mudou o
 * algoritmo sem mudar a versão do fio.
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 4.0.0
 */

#include "bythos.h"
#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <math.h>

/* Chave de ensaio (32 B de 0x42 — produção: elemento seguro). */
static const uint8_t TEST_KEY[32] = {
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42
};

static int failures = 0;
#define CHECK(cond) do { \
        if (!(cond)) { \
            printf("FALHOU %s:%d: %s\n", __FILE__, __LINE__, #cond); \
            failures++; \
        } \
    } while (0)

/* ============================================================================
 * CONSTANTES
 * ========================================================================== */

static void test_constantes(void) {
    printf("  constantes... ");
    CHECK(BYTHOS_START_BYTE == 0xAA);
    CHECK(BYTHOS_VERSION == 0x04);
    CHECK(BYTHOS_HEADER_SIZE == 11);
    CHECK(BYTHOS_SEC_HDR_SIZE == 4);
    CHECK(BYTHOS_TAG_SIZE == 4);
    CHECK(BYTHOS_CRC16_SIZE == 2);
    CHECK(BYTHOS_OVERHEAD == 21);
    CHECK(BYTHOS_MAX_FIELDS == 32);
    CHECK(BYTHOS_MAX_FIELD_DATA == 32);
    CHECK(BYTHOS_MAX_MESSAGE_SIZE == 1205);
    CHECK(BYTHOS_BROADCAST == 0xFFFF);
    CHECK(BYTHOS_ROOT_ADDR == 0x0000);
    printf("OK\n");
}

/* ============================================================================
 * CAMPO (identificador com tipo)
 * ========================================================================== */

static void test_campo_id(void) {
    printf("  identificador de campo... ");
    /* Ida-volta completa 8×32. */
    for (int t = 0; t <= 7; t++) {
        for (int id = 0; id <= 31; id++) {
            uint8_t enc = bythos_field_id_encode((uint8_t)t, (uint8_t)id);
            uint8_t dt, di;
            bythos_field_id_decode(enc, &dt, &di);
            CHECK(dt == t && di == id);
        }
    }
    CHECK(bythos_field_id_encode(8, 0) == 0xFF);
    CHECK(bythos_field_id_encode(0, 32) == 0xFF);
    CHECK(bythos_field_id_valid(0x26) == 1);
    printf("OK\n");
}

/* ============================================================================
 * SHA-256 + HMAC (vetores publicados FIPS 180-4 / RFC 4231)
 * ========================================================================== */

static void test_sha256(void) {
    printf("  SHA-256... ");
    /* "abc" — vetor canónico da norma. */
    uint8_t d[32];
    bythos_sha256((const uint8_t*)"abc", 3, d);
    static const uint8_t abc[32] = {
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde,
        0x5d, 0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c,
        0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad
    };
    CHECK(memcmp(d, abc, 32) == 0);
    printf("OK\n");
}

static void test_hmac(void) {
    printf("  HMAC-SHA256 (RFC 4231 caso 1)... ");
    uint8_t key[20];
    memset(key, 0x0b, 20);
    uint8_t mac[32];
    bythos_hmac_sha256(key, 20, (const uint8_t*)"Hi There", 8, mac);
    static const uint8_t exp[32] = {
        0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce,
        0xaf, 0x0b, 0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7,
        0x26, 0xe9, 0x37, 0x6c, 0x2e, 0x32, 0xcf, 0xf7
    };
    CHECK(memcmp(mac, exp, 32) == 0);
    /* TAG muda com o CTR (vinculação anti-recorte). */
    uint8_t t1[4], t2[4];
    const uint8_t cov[] = {0xAA, 0x04, 0x06, 0x00};
    CHECK(bythos_tag_compute(TEST_KEY, 7, cov, 4, t1) == 0);
    CHECK(bythos_tag_compute(TEST_KEY, 8, cov, 4, t2) == 0);
    CHECK(memcmp(t1, t2, 4) != 0);
    CHECK(bythos_tag_verify(TEST_KEY, 7, cov, 4, t1) == 1);
    CHECK(bythos_tag_verify(TEST_KEY, 7, cov, 4, t2) == 0);
    printf("OK\n");
}

static void test_crc(void) {
    printf("  CRC... ");
    CHECK(bythos_calc_crc16((const uint8_t*)"123456789", 9) == 0x29B1);
    CHECK(bythos_calc_crc16(NULL, 0) == 0xFFFF);
    CHECK(bythos_calc_crc8(NULL, 0) == 0x00);
    printf("OK\n");
}

/* ============================================================================
 * VETOR DOURADO (contrato Rust↔C — ver teste Rust test_vetor_dourado_v4)
 * ========================================================================== */

static void test_vetor_dourado(void) {
    printf("  vetor dourado V4... ");
    BythosMessage msg;
    bythos_init(&msg, 0x0006, BYTHOS_MSG_TELEMETRY);
    bythos_set_dst(&msg, BYTHOS_BROADCAST);
    bythos_set_seq(&msg, 1);
    CHECK(bythos_field_add_u8(&msg, 0xC0, 0x02) == 0);

    uint8_t buf[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t n = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, BYTHOS_BROADCAST,
                                    0, 41, TEST_KEY, buf, sizeof(buf));
    CHECK(n == 24);
    /* Cabeçalho de 11 B + campo + SEC_HDR, byte a byte. */
    static const uint8_t head[18] = {
        0xAA, 0x04, 0x06, 0x00, 0xFF, 0xFF, 0x11, 0x01, 0x00, 0x01, 0x20,
        0xC0, 0x01, 0x02, 0x00, 0x29, 0x00, 0x00
    };
    CHECK(memcmp(buf, head, 18) == 0);
    /* TAG congelado (HMAC-SHA256-32 da entrada acima). */
    static const uint8_t tag[4] = {0x82, 0x23, 0x4B, 0xEF};
    CHECK(memcmp(buf + 18, tag, 4) == 0);
    /* CRC cobre tudo antes dele. */
    uint16_t crc = (uint16_t)buf[22] | ((uint16_t)buf[23] << 8);
    CHECK(crc == bythos_calc_crc16(buf, 22));

    /* E valida ponta-a-ponta com a mesma chave. */
    BythosPeers peers;
    bythos_peers_clear(&peers);
    CHECK(bythos_validate(buf, (size_t)n, TEST_KEY, &peers) == 1);
    /* Repetir = replay. */
    CHECK(bythos_validate(buf, (size_t)n, TEST_KEY, &peers) == 0xFF);
    printf("OK\n");
}

/* ============================================================================
 * CONSTRUÇÃO / VALIDAÇÃO / ANÁLISE
 * ========================================================================== */

static void test_construcao_roundtrip(void) {
    printf("  construção e roundtrip... ");
    BythosMessage msg;
    bythos_init(&msg, 27, BYTHOS_MSG_TELEMETRY);
    bythos_set_dst(&msg, 2);
    bythos_set_seq(&msg, 7);
    bythos_set_hops(&msg, 16);
    CHECK(bythos_field_add_u8(&msg, BYTHOS_FIELD_SYSTEM_STATE, 2) == 0);
    CHECK(bythos_field_add_f32(&msg, BYTHOS_FIELD_GPS_LATITUDE, -33.9f) == 0);
    CHECK(bythos_field_add_u32(&msg, BYTHOS_FIELD_SYSTEM_UPTIME, 3600) == 0);
    CHECK(bythos_field_add_bool(&msg, 0xE0, 1) == 0);
    /* Oitavo teste: o 0xC2 da V3 era ID errado para u32; aqui usa-se 0x82. */
    CHECK(msg.field_count == 4);

    uint8_t buf[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t n = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, 2, 0, 100,
                                    TEST_KEY, buf, sizeof(buf));
    CHECK(n > BYTHOS_OVERHEAD);

    /* Ponte no caminho: só estrutura. */
    CHECK(bythos_peek(buf, (size_t)n) == 4);

    /* Destino: ponta-a-ponta. */
    BythosPeers peers;
    bythos_peers_clear(&peers);
    CHECK(bythos_validate(buf, (size_t)n, TEST_KEY, &peers) == 4);

    /* Análise dos campos (fatia após os 11 B do cabeçalho). */
    size_t fields_len = (size_t)n - BYTHOS_HEADER_SIZE - BYTHOS_SEC_HDR_SIZE -
                        BYTHOS_TAG_SIZE - BYTHOS_CRC16_SIZE;
    BythosField out[8];
    size_t cnt = 8;
    bythos_parse_fields(buf + BYTHOS_HEADER_SIZE, fields_len, out, &cnt);
    CHECK(cnt == 4);
    CHECK(out[0].id == BYTHOS_FIELD_SYSTEM_STATE && out[0].len == 1);
    CHECK(fabsf(bythos_bytes_to_f32(out[1].data) - (-33.9f)) < 0.001f);
    CHECK(bythos_bytes_to_u32(out[2].data) == 3600);
    CHECK(out[3].data[0] == 1);
    printf("OK\n");
}

static void test_recusas(void) {
    printf("  recusas (nulos, gamas, adulteração)... ");
    uint8_t buf[BYTHOS_MAX_MESSAGE_SIZE];
    BythosPeers peers;
    bythos_peers_clear(&peers);

    /* Nulos nunca rebentam. */
    CHECK(bythos_build(NULL, 0x11, 0, 0, 0, TEST_KEY, buf, sizeof(buf)) == -1);
    CHECK(bythos_peek(NULL, 10) == 0xFF);
    CHECK(bythos_validate(NULL, 10, TEST_KEY, &peers) == 0xFF);
    size_t cnt = 8;
    bythos_parse_fields(NULL, 10, NULL, &cnt);

    /* Mensagem válida para adulterar. */
    BythosMessage msg;
    bythos_init(&msg, 6, BYTHOS_MSG_TELEMETRY);
    bythos_field_add_u8(&msg, 0xC0, 2);
    bythos_ssize_t n = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, BYTHOS_BROADCAST,
                                    0, 200, TEST_KEY, buf, sizeof(buf));
    CHECK(n > 0);
    /* 1 bit na carga, no TAG e no CRC: tudo chumba. */
    uint8_t tmp[BYTHOS_MAX_MESSAGE_SIZE];
    memcpy(tmp, buf, (size_t)n);
    tmp[12] ^= 0x01;
    CHECK(bythos_validate(tmp, (size_t)n, TEST_KEY, &peers) == 0xFF);
    memcpy(tmp, buf, (size_t)n);
    tmp[n - 5] ^= 0x01;
    CHECK(bythos_validate(tmp, (size_t)n, TEST_KEY, &peers) == 0xFF);
    memcpy(tmp, buf, (size_t)n);
    tmp[n - 1] ^= 0x01;
    CHECK(bythos_validate(tmp, (size_t)n, TEST_KEY, &peers) == 0xFF);
    /* Chave errada chumba. */
    uint8_t wrong[32] = {0};
    CHECK(bythos_validate(buf, (size_t)n, wrong, &peers) == 0xFF);
    /* Transbordo de campos e carga grande recusam, sem truncar. */
    BythosMessage full;
    bythos_init(&full, 6, BYTHOS_MSG_TELEMETRY);
    for (int i = 0; i < 32; i++) CHECK(bythos_field_add_u8(&full, 0xC0, 0) == 0);
    CHECK(bythos_field_add_u8(&full, 0xC0, 0) == -1);
    uint8_t big[33] = {0};
    CHECK(bythos_field_add(&full, 0x00, big, 33) == -1);
    printf("OK\n");
}

static void test_init_clear(void) {
    printf("  init/clear... ");
    BythosMessage msg;
    /* Tipos inválidos = sem efeito (estrutura intocada). */
    memset(&msg, 0x5A, sizeof(msg));
    bythos_init(&msg, 6, 0x00);
    CHECK(msg.start_byte == 0x5A);
    bythos_init(&msg, 6, BYTHOS_MSG_TELEMETRY);
    CHECK(msg.start_byte == BYTHOS_START_BYTE);
    CHECK(msg.version == BYTHOS_VERSION);
    CHECK(msg.src == 6);
    CHECK(msg.dst == BYTHOS_BROADCAST);
    CHECK(msg.hops == BYTHOS_HOPS_DEFAULT);
    /* Limpar repõe preâmbulo (na V3 zerava START/VERSION — divergência fechada). */
    bythos_clear(&msg);
    CHECK(msg.start_byte == BYTHOS_START_BYTE);
    CHECK(msg.version == BYTHOS_VERSION);
    CHECK(msg.field_count == 0);
    printf("OK\n");
}

/* ============================================================================
 * ANEL, PRIORIDADE, CONVERSÕES, COBS
 * ========================================================================== */

static void test_bus_id(void) {
    printf("  Bythos Bus ID... ");
    /* Ida-volta dos 4 campos + reserva a zero. */
    uint32_t id = bythos_bus_id_make(2, 0x6, 0x0, 27);
    CHECK(bythos_bus_id_priority(id) == 2);
    CHECK(bythos_bus_id_group(id) == 0x6);
    CHECK(bythos_bus_id_kind(id) == 0x0);
    CHECK(bythos_bus_id_src(id) == 27);
    CHECK((id & 0x1F) == 0);
    /* Layout bit a bit (contrato com o firmware da ponte). */
    CHECK(id == ((uint32_t)2 << 29 | (uint32_t)0x6 << 25 | (uint32_t)27 << 5));
    /* Gamas inválidas recusam (0 = inválido). */
    CHECK(bythos_bus_id_make(5, 0x6, 0x0, 1) == 0);
    CHECK(bythos_bus_id_make(2, 0x10, 0x0, 1) == 0);
    CHECK(bythos_bus_id_make(2, 0x6, 0x9, 1) == 0);
    /* Menor ganha, como no CAN: emergência esmaga telemetria. */
    uint32_t emergencia = bythos_bus_id_make(0, 0x4, 0x7, 4);
    uint32_t telemetria = bythos_bus_id_make(3, 0x2, 0x0, 27);
    CHECK(bythos_bus_id_wins(emergencia, telemetria) == 1);
    CHECK(bythos_bus_id_wins(telemetria, emergencia) == 0);
    printf("OK\n");
}

static void test_anel_e_prioridade(void) {
    printf("  anel e prioridade... ");
    /* BG-27 → BG-2 em anel de 30: frente (0), 5 saltos. */
    CHECK(bythos_ring_decide(27, 30, 2) == 0);
    CHECK(bythos_ring_decide(2, 30, 27) == 1);
    CHECK(bythos_ring_decide(5, 30, BYTHOS_BROADCAST) == 2);
    /* Espécies derivadas do MSG, sem CAN ID. */
    CHECK(bythos_kind_of(0x11) == BYTHOS_KIND_DATA);
    CHECK(bythos_kind_of(0x1C) == BYTHOS_KIND_RING);
    CHECK(bythos_kind_of(0x20) == 0xFF);
    CHECK(bythos_is_safety_kind(BYTHOS_KIND_SAFETY) == 1);
    CHECK(bythos_group_valid(0x4) == 1 && bythos_group_valid(0x10) == 0);
    /* Prioridades (inválido = Low, unificado). */
    CHECK(bythos_msg_priority(0x14, 0) == BYTHOS_PRIORITY_SUPER_CRITICAL);
    CHECK(bythos_msg_priority(0x10, 1) == BYTHOS_PRIORITY_SUPER_CRITICAL);
    CHECK(bythos_msg_priority(0x15, 1) == BYTHOS_PRIORITY_LOW);
    CHECK(bythos_msg_priority(0x00, 0) == BYTHOS_PRIORITY_LOW);
    printf("OK\n");
}

static void test_conversoes(void) {
    printf("  conversões... ");
    uint8_t b[4];
    bythos_f32_to_bytes(1.5f, b);
    CHECK(fabsf(bythos_bytes_to_f32(b) - 1.5f) < 1e-6f);
    bythos_u32_to_bytes(0xDEADBEEFu, b);
    CHECK(bythos_bytes_to_u32(b) == 0xDEADBEEFu);
    bythos_i32_to_bytes(-12345, b);
    CHECK(bythos_bytes_to_i32(b) == -12345);
    uint8_t b2[2];
    bythos_u16_to_bytes(0x1234, b2);
    CHECK(bythos_bytes_to_u16(b2) == 0x1234);
    /* f16 ida-volta (dívida V3 fechada também em C). */
    uint16_t h = bythos_f32_to_f16(21.5f);
    CHECK(fabsf(bythos_f16_to_f32(h) - 21.5f) < 0.05f);
    CHECK(bythos_bytes_to_f32(NULL) == 0.0f);
    printf("OK\n");
}

static void test_cobs(void) {
    printf("  COBS... ");
    const uint8_t data[] = {0x00, 0x11, 0x00, 0x22, 0x33, 0x00};
    uint8_t enc[16], dec[16];
    bythos_ssize_t m = bythos_cobs_encode(data, sizeof(data), enc, sizeof(enc));
    CHECK(m > 0);
    for (bythos_ssize_t i = 0; i < m; i++) CHECK(enc[i] != 0);
    bythos_ssize_t k = bythos_cobs_decode(enc, (size_t)m, dec, sizeof(dec));
    CHECK(k == (bythos_ssize_t)sizeof(data));
    CHECK(memcmp(dec, data, sizeof(data)) == 0);
    /* Corrompido recusa. */
    const uint8_t bad[] = {0x05, 0x01};
    CHECK(bythos_cobs_decode(bad, 2, dec, sizeof(dec)) == -1);
    printf("OK\n");
}

static void test_versao(void) {
    printf("  versão... ");
    CHECK(strcmp(bythos_version(), "4.0.0") == 0);
    CHECK(bythos_overhead() == 21);
    CHECK(bythos_max_message_size() == 1205);
    CHECK(bythos_msg_id_valid(0x1F) == 1 && bythos_msg_id_valid(0x20) == 0);
    printf("OK\n");
}

int main(void) {
    printf("Bythos V4 — testes C\n");
    test_constantes();
    test_campo_id();
    test_sha256();
    test_hmac();
    test_crc();
    test_vetor_dourado();
    test_construcao_roundtrip();
    test_recusas();
    test_init_clear();
    test_bus_id();
    test_anel_e_prioridade();
    test_conversoes();
    test_cobs();
    test_versao();
    if (failures == 0) {
        printf("TODOS OS TESTES C PASSARAM\n");
        return 0;
    }
    printf("%d FALHAS\n", failures);
    return 1;
}
