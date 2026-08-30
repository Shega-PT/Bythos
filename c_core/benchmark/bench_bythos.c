/**
 * @file bench_bythos.c
 * @brief Bythos Protocol v3.0.0 — Microbenchmarks
 *
 * Microbenchmarks para o protocolo Bythos v3.0.0.
 * Mede o desempenho de operações críticas: CRC, assinatura,
 * construção de mensagens e validação.
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 3.0.0
 */

#include "bythos.h"
#include <stdio.h>
#include <string.h>
#include <time.h>

// ============================================================================
// CONSTANTES
// ============================================================================

#define BENCH_ITERATIONS_CRC      100000
#define BENCH_ITERATIONS_SIG      100000
#define BENCH_ITERATIONS_BUILD    10000
#define BENCH_ITERATIONS_VALIDATE 10000
#define BENCH_ITERATIONS_PARSE    10000

// ============================================================================
// HELPER DE TEMPO
// ============================================================================

static double get_time_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1000.0 + (double)ts.tv_nsec / 1000000.0;
}

// ============================================================================
// BENCHMARK — CRC-16
// ============================================================================

void bench_crc16(void) {
    printf("  CRC-16 (100K iterações)... ");
    uint8_t data[] = {0xAA, 0x03, 0x06, 0x11, 0x2A, 0x00, 0x01, 0xC0, 0x01, 0x02};

    double start = get_time_ms();
    volatile uint16_t result;
    for (int i = 0; i < BENCH_ITERATIONS_CRC; i++) {
        result = bythos_calc_crc16(data, sizeof(data));
    }
    double elapsed = get_time_ms() - start;
    printf("%.2f ms (%.2f µs/op)\n", elapsed, elapsed * 1000.0 / BENCH_ITERATIONS_CRC);
}

// ============================================================================
// BENCHMARK — ASSINATURA
// ============================================================================

void bench_signature(void) {
    printf("  Assinatura (100K iterações)... ");
    double start = get_time_ms();
    volatile uint8_t result;
    for (int i = 0; i < BENCH_ITERATIONS_SIG; i++) {
        result = bythos_signature_compute(0x42, 0x11, 0x2A, 0x00);
    }
    double elapsed = get_time_ms() - start;
    printf("%.2f ms (%.2f µs/op)\n", elapsed, elapsed * 1000.0 / BENCH_ITERATIONS_SIG);
}

// ============================================================================
// BENCHMARK — CONSTRUÇÃO DE MENSAGEM
// ============================================================================

void bench_build(void) {
    printf("  Construção (10K iterações)... ");
    double start = get_time_ms();
    volatile bythos_ssize_t result;
    for (int i = 0; i < BENCH_ITERATIONS_BUILD; i++) {
        BythosMessage msg;
        bythos_init(&msg, 0x06, BYTHOS_MSG_TELEMETRY);
        bythos_set_seq(&msg, i & 0xFFFF);
        bythos_tlv_add_f32(&msg, 0x26, 40.0f);
        bythos_tlv_add_f32(&msg, 0x27, -8.0f);
        bythos_tlv_add_u8(&msg, 0xC0, 4);
        bythos_tlv_add_u32(&msg, 0xC2, 3600);

        uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
        result = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, 0x42, buffer, sizeof(buffer));
    }
    double elapsed = get_time_ms() - start;
    printf("%.2f ms (%.2f µs/op)\n", elapsed, elapsed * 1000.0 / BENCH_ITERATIONS_BUILD);
}

// ============================================================================
// BENCHMARK — VALIDAÇÃO
// ============================================================================

void bench_validate(void) {
    printf("  Validação (10K iterações)... ");

    // Preparar mensagem
    BythosMessage msg;
    bythos_init(&msg, 0x06, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 42);
    bythos_tlv_add_f32(&msg, 0x26, 40.0f);
    bythos_tlv_add_f32(&msg, 0x27, -8.0f);
    bythos_tlv_add_u8(&msg, 0xC0, 4);
    bythos_tlv_add_u32(&msg, 0xC2, 3600);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, 0x42, buffer, sizeof(buffer));

    double start = get_time_ms();
    volatile uint8_t result;
    for (int i = 0; i < BENCH_ITERATIONS_VALIDATE; i++) {
        result = bythos_validate(buffer, size);
    }
    double elapsed = get_time_ms() - start;
    printf("%.2f ms (%.2f µs/op)\n", elapsed, elapsed * 1000.0 / BENCH_ITERATIONS_VALIDATE);
}

// ============================================================================
// BENCHMARK — PARSING TLV
// ============================================================================

void bench_parse_tlv(void) {
    printf("  Parsing TLV (10K iterações)... ");

    // Preparar dados TLV
    uint8_t tlv_data[] = {
        0x26, 4, 0x00, 0x00, 0x20, 0x42,  // Latitude f32
        0x27, 4, 0x00, 0x00, 0x00, 0xC0,  // Longitude f32
        0xC0, 1, 0x04,                      // State u8
        0xC2, 4, 0x70, 0x0E, 0x00, 0x00,   // Uptime u32
    };

    double start = get_time_ms();
    volatile size_t result;
    for (int i = 0; i < BENCH_ITERATIONS_PARSE; i++) {
        BythosTLVField tlvs[BYTHOS_MAX_TLV_FIELDS];
        size_t count = BYTHOS_MAX_TLV_FIELDS;
        bythos_parse_tlv(tlv_data, sizeof(tlv_data), tlvs, &count);
        result = count;
    }
    double elapsed = get_time_ms() - start;
    printf("%.2f ms (%.2f µs/op)\n", elapsed, elapsed * 1000.0 / BENCH_ITERATIONS_PARSE);
}

// ============================================================================
// MAIN
// ============================================================================

int main(void) {
    printf("Bythos Protocol v3.0.0 — Microbenchmarks\n");
    printf("==========================================\n\n");

    bench_crc16();
    bench_signature();
    bench_build();
    bench_validate();
    bench_parse_tlv();

    printf("\nBenchmark concluído!\n");
    return 0;
}
