/**
 * @file bench_bythos.c
 * @brief Bythos Protocol v4.0.0 — Microbenchmarks
 *
 * Mede o caminho quente: CRC, selagem HMAC, construção selada, validação
 * ponta-a-ponta e análise de campos. Números para dimensionar a ponte
 * (o HMAC em software é o custo dominante — com ATECC608 ele sai da CPU).
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 4.0.0
 */

#include "bythos.h"
#include <stdio.h>
#include <string.h>
#include <time.h>

#define BENCH_CRC      100000
#define BENCH_TAG      10000
#define BENCH_BUILD    10000
#define BENCH_VALIDATE 10000
#define BENCH_PARSE    10000

/* Chave de ensaio (produção: elemento seguro). */
static const uint8_t BENCH_KEY[32] = {
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42,
    0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42
};

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1000.0 + (double)ts.tv_nsec / 1000000.0;
}

static void bench_crc16(void) {
    printf("  CRC-16 (100K)... ");
    const uint8_t data[] = {0xAA, 0x04, 0x06, 0x00, 0xFF, 0xFF, 0x11, 0x2A, 0x00, 0x01, 0x20};
    double s = now_ms();
    volatile uint16_t r;
    for (int i = 0; i < BENCH_CRC; i++) r = bythos_calc_crc16(data, sizeof(data));
    double e = now_ms() - s;
    printf("%.2f ms (%.2f us/op)\n", e, e * 1000.0 / BENCH_CRC);
}

static void bench_tag(void) {
    printf("  TAG HMAC (10K)... ");
    const uint8_t cov[] = {0xAA, 0x04, 0x06, 0x00, 0xFF, 0xFF, 0x11, 0x01, 0x00, 0x01, 0x20,
                           0xC0, 0x01, 0x02, 0x00, 0x29, 0x00, 0x00};
    double s = now_ms();
    volatile int8_t r;
    for (int i = 0; i < BENCH_TAG; i++) {
        uint8_t tag[4];
        r = bythos_tag_compute(BENCH_KEY, 41, cov, sizeof(cov), tag);
    }
    double e = now_ms() - s;
    printf("%.2f ms (%.2f us/op)\n", e, e * 1000.0 / BENCH_TAG);
}

static void bench_build(void) {
    printf("  Construcao selada (10K)... ");
    double s = now_ms();
    volatile bythos_ssize_t r;
    for (int i = 0; i < BENCH_BUILD; i++) {
        BythosMessage msg;
        bythos_init(&msg, 27, BYTHOS_MSG_TELEMETRY);
        bythos_set_dst(&msg, BYTHOS_BROADCAST);
        bythos_set_seq(&msg, (uint16_t)i);
        bythos_field_add_f32(&msg, 0x26, 40.0f);
        bythos_field_add_f32(&msg, 0x27, -8.0f);
        bythos_field_add_u8(&msg, 0xC0, 4);
        bythos_field_add_u32(&msg, 0x82, 3600);
        uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
        r = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, BYTHOS_BROADCAST,
                         0, (uint32_t)i, BENCH_KEY, buffer, sizeof(buffer));
    }
    double e = now_ms() - s;
    printf("%.2f ms (%.2f us/op)\n", e, e * 1000.0 / BENCH_BUILD);
}

static void bench_validate(void) {
    printf("  Validacao ponta-a-ponta (10K)... ");
    BythosMessage msg;
    bythos_init(&msg, 27, BYTHOS_MSG_TELEMETRY);
    bythos_field_add_f32(&msg, 0x26, 40.0f);
    bythos_field_add_u8(&msg, 0xC0, 4);
    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t n = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, BYTHOS_BROADCAST,
                                    0, 7, BENCH_KEY, buffer, sizeof(buffer));
    BythosPeers peers;
    bythos_peers_clear(&peers);
    /* Nota: validar a mesma trama 10K vezes conta replay à 2.ª — para medir o
       caminho quente puro, renova-se a janela a cada 64 (custo realista). */
    double s = now_ms();
    volatile uint8_t r;
    for (int i = 0; i < BENCH_VALIDATE; i++) {
        if ((i % 64) == 0) bythos_peers_clear(&peers);
        r = bythos_validate(buffer, (size_t)n, BENCH_KEY, &peers);
    }
    double e = now_ms() - s;
    printf("%.2f ms (%.2f us/op)\n", e, e * 1000.0 / BENCH_VALIDATE);
}

static void bench_parse(void) {
    printf("  Analise de campos (10K)... ");
    const uint8_t data[] = {
        0x26, 4, 0x00, 0x00, 0x20, 0x42,
        0x27, 4, 0x00, 0x00, 0x00, 0xC0,
        0xC0, 1, 0x04,
        0x82, 4, 0x70, 0x0E, 0x00, 0x00,
    };
    double s = now_ms();
    volatile size_t r;
    for (int i = 0; i < BENCH_PARSE; i++) {
        BythosField out[BYTHOS_MAX_FIELDS];
        size_t count = BYTHOS_MAX_FIELDS;
        bythos_parse_fields(data, sizeof(data), out, &count);
        r = count;
    }
    double e = now_ms() - s;
    printf("%.2f ms (%.2f us/op)\n", e, e * 1000.0 / BENCH_PARSE);
}

int main(void) {
    printf("Bythos V4 — microbenchmarks (software; com ATECC608 o TAG sai da CPU)\n");
    bench_crc16();
    bench_tag();
    bench_build();
    bench_validate();
    bench_parse();
    printf("OK\n");
    return 0;
}
