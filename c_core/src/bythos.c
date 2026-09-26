/**
 * @file bythos.c
 * @brief Bythos Protocol v4.0.0 — Implementação C standalone
 *
 * Reimplementação completa em C (sem depender do Rust): o firmware da ponte
 * compila só isto + o driver I2C do ATECC608. Sem heap em lado nenhum — todos
 * os tampões são do chamador.
 *
 * Convenções (ver bythos.h):
 * - Nulos e gamas verificam-se sempre; erro nunca é pânico.
 * - Recusar em vez de truncar (a V3 truncava `min(32)` em silêncio).
 * - `memcpy` em vez de `union` para ler bits de `float` (a união é
 *   comportamento indefinido em C estrito e parte em otimização alta).
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 4.0.0
 */

#include "bythos.h"
#include <string.h>

// ============================================================================
// TABELA CRC-16/CCITT (polinómio 0x1021 — 256 entradas em ROM)
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

/* CRC-8/SMBUS legado (migração V3). `const` para viver em ROM, não RAM. */
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
// TABELA DE TIPOS VÁLIDOS (0x10-0x1F)
// ============================================================================

static const uint8_t valid_msg_ids[] = {
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F
};
#define NUM_VALID_MSG_IDS (sizeof(valid_msg_ids) / sizeof(valid_msg_ids[0]))

/** 1 se o tipo de mensagem é conhecido. */
static int is_valid_msg_id(uint8_t id) {
    for (size_t i = 0; i < NUM_VALID_MSG_IDS; i++) {
        if (valid_msg_ids[i] == id) return 1;
    }
    return 0;
}

/**
 * 1 se o identificador tem tipo embutido válido.
 *
 * Nota honesta (igual ao Rust): qualquer byte tem tipo 0-7, por isso isto é
 * barreira de leitura, não prova — o aperto real é LEN-por-tipo na validação.
 */
static int is_valid_field_id(uint8_t field_id) {
    return ((field_id >> 5) & 0x07) <= 7;
}

// ============================================================================
// CRC
// ============================================================================

uint16_t bythos_calc_crc16(const uint8_t* data, size_t len) {
    if (data == NULL || len == 0) return 0xFFFF;
    uint16_t crc = 0xFFFF;
    for (size_t i = 0; i < len; i++) {
        crc = (uint16_t)((crc << 8) ^ crc16_table[((crc >> 8) ^ data[i]) & 0xFF]);
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
// SHA-256 (FIPS 180-4, software puro — produção usa o ATECC608)
// ============================================================================

#define ROTR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))

static const uint32_t sha256_k[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

/** Uma volta de compressão sobre um bloco de 64 bytes. */
static void sha256_compress(uint32_t s[8], const uint8_t block[64]) {
    uint32_t w[64];
    for (int i = 0; i < 16; i++) {
        w[i] = ((uint32_t)block[i * 4] << 24) | ((uint32_t)block[i * 4 + 1] << 16) |
               ((uint32_t)block[i * 4 + 2] << 8) | (uint32_t)block[i * 4 + 3];
    }
    for (int i = 16; i < 64; i++) {
        uint32_t s0 = ROTR(w[i - 15], 7) ^ ROTR(w[i - 15], 18) ^ (w[i - 15] >> 3);
        uint32_t s1 = ROTR(w[i - 2], 17) ^ ROTR(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    uint32_t a = s[0], b = s[1], c = s[2], d = s[3];
    uint32_t e = s[4], f = s[5], g = s[6], h = s[7];
    for (int i = 0; i < 64; i++) {
        uint32_t S1 = ROTR(e, 6) ^ ROTR(e, 11) ^ ROTR(e, 25);
        uint32_t ch = (e & f) ^ ((~e) & g);
        uint32_t t1 = h + S1 + ch + sha256_k[i] + w[i];
        uint32_t S0 = ROTR(a, 2) ^ ROTR(a, 13) ^ ROTR(a, 22);
        uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
        uint32_t t2 = S0 + maj;
        h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    s[0] += a; s[1] += b; s[2] += c; s[3] += d;
    s[4] += e; s[5] += f; s[6] += g; s[7] += h;
}

/** Núcleo incremental: alimenta `prefixo ‖ corpo` sem os concatenar. */
static void sha256_two(const uint8_t* pfx, size_t pfx_len,
                       const uint8_t* body, size_t body_len, uint8_t out[32]) {
    uint32_t s[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                     0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
    uint8_t block[64];
    size_t blen = 0;
    uint64_t total_bits = 0;

    /* Macro local: absorve uma fatia no bloco, comprimindo quando enche. */
#define FEED(ptr, n) do { \
        size_t _off = 0; \
        total_bits += (uint64_t)(n) * 8; \
        while (_off < (n)) { \
            size_t _take = 64 - blen; \
            if (_take > (n) - _off) _take = (n) - _off; \
            memcpy(block + blen, (ptr) + _off, _take); \
            blen += _take; _off += _take; \
            if (blen == 64) { sha256_compress(s, block); blen = 0; } \
        } \
    } while (0)

    FEED(pfx, pfx_len);
    FEED(body, body_len);
#undef FEED

    /* Enchimento final: 0x80, zeros, comprimento de 64 bits big-endian. */
    block[blen++] = 0x80;
    if (blen > 56) {
        memset(block + blen, 0, 64 - blen);
        sha256_compress(s, block);
        blen = 0;
    }
    memset(block + blen, 0, 56 - blen);
    for (int i = 0; i < 8; i++) {
        block[56 + i] = (uint8_t)((total_bits >> (56 - 8 * i)) & 0xFF);
    }
    sha256_compress(s, block);
    for (int i = 0; i < 8; i++) {
        out[i * 4] = (uint8_t)((s[i] >> 24) & 0xFF);
        out[i * 4 + 1] = (uint8_t)((s[i] >> 16) & 0xFF);
        out[i * 4 + 2] = (uint8_t)((s[i] >> 8) & 0xFF);
        out[i * 4 + 3] = (uint8_t)(s[i] & 0xFF);
    }
}

void bythos_sha256(const uint8_t* data, size_t len, uint8_t out[32]) {
    static const uint8_t empty = 0;
    if (out == NULL) return;
    if (data == NULL) data = &empty, len = 0;
    sha256_two(NULL, 0, data, len, out);
}

void bythos_hmac_sha256(const uint8_t* key, size_t key_len,
                        const uint8_t* msg, size_t msg_len, uint8_t out[32]) {
    uint8_t kblk[64] = {0};
    if (key != NULL && key_len > 0) {
        if (key_len > 64) {
            /* Chave longa: condensa primeiro (RFC 2104). */
            uint8_t d[32];
            sha256_two(NULL, 0, key, key_len, d);
            memcpy(kblk, d, 32);
        } else {
            memcpy(kblk, key, key_len);
        }
    }
    uint8_t ipad[64], opad[64];
    for (int i = 0; i < 64; i++) {
        ipad[i] = kblk[i] ^ 0x36;
        opad[i] = kblk[i] ^ 0x5C;
    }
    /* Interno: H(ipad ‖ msg); externo: H(opad ‖ interno). */
    uint8_t inner[32];
    uint8_t combo[64 + 1205];
    static const uint8_t empty = 0;
    const uint8_t* m = (msg != NULL) ? msg : &empty;
    size_t ml = (msg != NULL) ? msg_len : 0;
    /* Sem heap: o interno processa ipad e msg em duas alimentações via cópia
       para `combo` só se couber; acima de 1205+64 recusa (tramas Bythos cabem). */
    if (ml > 1205) { memset(out, 0, 32); return; }
    memcpy(combo, ipad, 64);
    memcpy(combo + 64, m, ml);
    sha256_two(NULL, 0, combo, 64 + ml, inner);
    memcpy(combo, opad, 64);
    memcpy(combo + 64, inner, 32);
    sha256_two(NULL, 0, combo, 64 + 32, out);
}

// ============================================================================
// SELO (TAG — HMAC-SHA256 truncado, com CTR vinculado)
// ============================================================================

int8_t bythos_tag_compute(const uint8_t key[32], uint32_t ctr,
                          const uint8_t* covered, size_t covered_len,
                          uint8_t tag_out[4]) {
    if (key == NULL || covered == NULL || tag_out == NULL) return -1;
    if (covered_len > BYTHOS_MAX_MESSAGE_SIZE) return -1;
    /* O CTR (3 bytes LE) entra como prefixo do HMAC: cada contador tem etiqueta
       única, logo recortar etiquetas entre tramas não passa. */
    uint8_t ctr3[3] = {(uint8_t)(ctr & 0xFF), (uint8_t)((ctr >> 8) & 0xFF),
                       (uint8_t)((ctr >> 16) & 0xFF)};
    uint8_t msg[3 + BYTHOS_MAX_MESSAGE_SIZE];
    memcpy(msg, ctr3, 3);
    memcpy(msg + 3, covered, covered_len);
    uint8_t full[32];
    bythos_hmac_sha256(key, 32, msg, 3 + covered_len, full);
    memcpy(tag_out, full, 4);
    return 0;
}

uint8_t bythos_tag_verify(const uint8_t key[32], uint32_t ctr,
                          const uint8_t* covered, size_t covered_len,
                          const uint8_t tag[4]) {
    uint8_t expected[4];
    if (bythos_tag_compute(key, ctr, covered, covered_len, expected) != 0) return 0;
    /* Tempo constante: compara os 4 bytes sempre. */
    uint8_t diff = 0;
    for (int i = 0; i < 4; i++) diff |= (uint8_t)(expected[i] ^ tag[i]);
    return (diff == 0) ? 1 : 0;
}

uint8_t bythos_legacy_tag_compute(uint8_t key, uint8_t msg_id, uint8_t seq_lo, uint8_t seq_hi) {
    return (uint8_t)(key ^ msg_id ^ seq_lo ^ seq_hi);
}

uint8_t bythos_legacy_tag_validate(uint8_t tag, uint8_t key, uint8_t msg_id,
                                   uint8_t seq_lo, uint8_t seq_hi) {
    return (tag == bythos_legacy_tag_compute(key, msg_id, seq_lo, seq_hi)) ? 1 : 0;
}

// ============================================================================
// IDENTIFICADOR DE CAMPO
// ============================================================================

uint8_t bythos_field_id_encode(uint8_t field_type, uint8_t field_id) {
    if (field_type > 7 || field_id > 31) return 0xFF;
    return (uint8_t)((field_type << 5) | (field_id & 0x1F));
}

void bythos_field_id_decode(uint8_t field_id, uint8_t* type_out, uint8_t* id_out) {
    if (type_out == NULL || id_out == NULL) return;
    *type_out = (uint8_t)((field_id >> 5) & 0x07);
    *id_out = field_id & 0x1F;
}

uint8_t bythos_field_id_valid(uint8_t field_id) {
    return is_valid_field_id(field_id) ? 1 : 0;
}

// ============================================================================
// IDENTIFICADOR DO BYTHOS BUS (arbitragem estilo CAN, nomes Bythos)
// ============================================================================

uint32_t bythos_bus_id_make(uint8_t priority, uint8_t group, uint8_t kind, uint16_t src) {
    /* Validação estrita (comissionamento): recusar em vez de mascarar, para
       dois nós nunca colidirem por erro de configuração. */
    if (priority > BYTHOS_PRIORITY_LOW || group > 0x0F) return 0;
    if (kind > BYTHOS_KIND_RING) return 0;
    return ((uint32_t)priority << 29) | ((uint32_t)group << 25) |
           (((uint32_t)kind & 0x0F) << 21) | ((uint32_t)src << 5);
}

uint8_t bythos_bus_id_priority(uint32_t id) {
    return (uint8_t)((id >> 29) & 0x07);
}

uint8_t bythos_bus_id_group(uint32_t id) {
    return (uint8_t)((id >> 25) & 0x0F);
}

uint8_t bythos_bus_id_kind(uint32_t id) {
    return (uint8_t)((id >> 21) & 0x0F);
}

uint16_t bythos_bus_id_src(uint32_t id) {
    return (uint16_t)((id >> 5) & 0xFFFF);
}

uint8_t bythos_bus_id_wins(uint32_t a, uint32_t b) {
    return (a < b) ? 1 : 0;
}

// ============================================================================
// ENDEREÇAMENTO (sem CAN ID na V4)
// ============================================================================

uint8_t bythos_kind_of(uint8_t msg_id) {
    switch (msg_id) {
        case 0x10: return BYTHOS_KIND_HEART;
        case 0x11: return BYTHOS_KIND_DATA;
        case 0x12: return BYTHOS_KIND_CMD;
        case 0x13: return BYTHOS_KIND_ACK;
        case 0x14: return BYTHOS_KIND_SAFETY;
        case 0x15: return BYTHOS_KIND_DATA;
        case 0x16: return BYTHOS_KIND_DATA;
        case 0x17: return BYTHOS_KIND_CMD;
        case 0x18: return BYTHOS_KIND_DATA;
        case 0x19: return BYTHOS_KIND_HEART;
        case 0x1A: return BYTHOS_KIND_SYNC;
        case 0x1B: return BYTHOS_KIND_SYNC;
        case 0x1C: case 0x1D: case 0x1E: case 0x1F: return BYTHOS_KIND_RING;
        default: return 0xFF;
    }
}

uint8_t bythos_group_valid(uint8_t group) {
    return (group <= 0x0F) ? 1 : 0;
}

uint8_t bythos_is_safety_kind(uint8_t kind) {
    return (kind == BYTHOS_KIND_SAFETY) ? 1 : 0;
}

/**
 * Sentido no anel: 0 = AFTER (frente, números crescentes), 1 = BEFORE (trás),
 * 2 = local (difusão, próprio ou sem geografia).
 */
uint8_t bythos_ring_decide(uint16_t me, uint16_t total, uint16_t dst) {
    if (dst == BYTHOS_BROADCAST || dst == me || total == 0) return 2;
    uint32_t t = total;
    uint32_t cw = ((uint32_t)dst + t - me) % t;
    uint32_t ccw = ((uint32_t)me + t - dst) % t;
    return (cw <= ccw) ? 0 : 1;
}

// ============================================================================
// CONSTRUÇÃO
// ============================================================================

bythos_ssize_t bythos_build(const BythosMessage* msg, uint8_t msg_id, uint16_t dst,
                            uint8_t key_id, uint32_t ctr, const uint8_t key[32],
                            uint8_t* buffer, size_t buffer_size) {
    if (msg == NULL || buffer == NULL || key == NULL) return -1;
    if (!is_valid_msg_id(msg_id)) return -1;
    if (buffer_size < BYTHOS_OVERHEAD) return -1;
    if (msg->field_count > BYTHOS_MAX_FIELDS) return -1;

    /* Mede os campos primeiro — `len` de struct C não-confiável valida-se aqui
       (na V3 lia-se `memcpy` direto: OOB em leitura e escrita). */
    size_t fields_size = 0;
    int video_big_seen = 0;
    for (uint8_t i = 0; i < msg->field_count; i++) {
        uint8_t len = msg->fields[i].len;
        if (len > BYTHOS_MAX_VIDEO_DATA) return -1;
        if (len > BYTHOS_MAX_FIELD_DATA) {
            uint8_t ftype = (msg->fields[i].id >> 5) & 0x07;
            if (ftype != BYTHOS_FIELD_RAW || msg->fields[i].id != BYTHOS_FIELD_VIDEO_PAYLOAD ||
                video_big_seen) return -1;
            video_big_seen = 1;
        }
        fields_size += BYTHOS_FIELD_HEADER_SIZE + len;
    }
    size_t required = BYTHOS_HEADER_SIZE + fields_size + BYTHOS_SEC_HDR_SIZE +
                      BYTHOS_TAG_SIZE + BYTHOS_CRC16_SIZE;
    if (required > BYTHOS_MAX_MESSAGE_SIZE || buffer_size < required) return -1;

    size_t off = 0;
    buffer[off++] = BYTHOS_START_BYTE;
    buffer[off++] = BYTHOS_VERSION;
    buffer[off++] = (uint8_t)(msg->src & 0xFF);
    buffer[off++] = (uint8_t)((msg->src >> 8) & 0xFF);
    buffer[off++] = (uint8_t)(dst & 0xFF);
    buffer[off++] = (uint8_t)((dst >> 8) & 0xFF);
    buffer[off++] = msg_id;
    buffer[off++] = (uint8_t)(msg->seq_num & 0xFF);
    buffer[off++] = (uint8_t)((msg->seq_num >> 8) & 0xFF);
    buffer[off++] = msg->field_count;
    buffer[off++] = msg->hops;

    for (uint8_t i = 0; i < msg->field_count; i++) {
        uint8_t len = msg->fields[i].len;
        buffer[off++] = msg->fields[i].id;
        buffer[off++] = len;
        memcpy(&buffer[off], msg->fields[i].data, len);
        off += len;
    }

    uint32_t ctr24 = ctr & 0xFFFFFF;
    buffer[off++] = key_id;
    buffer[off++] = (uint8_t)(ctr24 & 0xFF);
    buffer[off++] = (uint8_t)((ctr24 >> 8) & 0xFF);
    buffer[off++] = (uint8_t)((ctr24 >> 16) & 0xFF);

    uint8_t tag[4];
    if (bythos_tag_compute(key, ctr24, buffer, off, tag) != 0) return -1;
    memcpy(&buffer[off], tag, 4);
    off += 4;

    uint16_t crc = bythos_calc_crc16(buffer, off);
    buffer[off++] = (uint8_t)(crc & 0xFF);
    buffer[off++] = (uint8_t)((crc >> 8) & 0xFF);

    return (bythos_ssize_t)off;
}

// ============================================================================
// VALIDAÇÃO
// ============================================================================

/**
 * Estrutura + CRC, sem chaves (para encaminhar). Devolve nº de campos ou 0xFF.
 * Exige comprimento exato: bytes a mais ou a menos = malformada.
 */
uint8_t bythos_peek(const uint8_t* buffer, size_t length) {
    if (buffer == NULL || length < BYTHOS_OVERHEAD) return 0xFF;
    if (buffer[0] != BYTHOS_START_BYTE) return 0xFF;
    if (buffer[1] != BYTHOS_VERSION) return 0xFF;
    if (!is_valid_msg_id(buffer[6])) return 0xFF;
    uint8_t count = buffer[9];
    if (count > BYTHOS_MAX_FIELDS) return 0xFF;

    size_t off = BYTHOS_HEADER_SIZE;
    int video_big_seen = 0;
    for (uint8_t i = 0; i < count; i++) {
        if (off + BYTHOS_FIELD_HEADER_SIZE > length) return 0xFF;
        uint8_t fid = buffer[off];
        uint8_t flen = buffer[off + 1];
        if (flen > BYTHOS_MAX_VIDEO_DATA) return 0xFF;
        if (flen > BYTHOS_MAX_FIELD_DATA) {
            uint8_t ftype = (fid >> 5) & 0x07;
            if (ftype != BYTHOS_FIELD_RAW || fid != BYTHOS_FIELD_VIDEO_PAYLOAD ||
                video_big_seen) return 0xFF;
            video_big_seen = 1;
        }
        off += BYTHOS_FIELD_HEADER_SIZE + flen;
    }
    if (off + BYTHOS_SEC_HDR_SIZE + BYTHOS_TAG_SIZE + BYTHOS_CRC16_SIZE != length) return 0xFF;

    size_t crc_at = length - BYTHOS_CRC16_SIZE;
    uint16_t rx = (uint16_t)buffer[crc_at] | ((uint16_t)buffer[crc_at + 1] << 8);
    if (bythos_calc_crc16(buffer, crc_at) != rx) return 0xFF;
    return count;
}

void bythos_peers_clear(BythosPeers* peers) {
    if (peers == NULL) return;
    memset(peers, 0, sizeof(BythosPeers));
}

/** Janela anti-replay de uma ranhura (espelha o Rust, aritmética circular). */
static int peers_accept(BythosPeers* p, int slot, uint32_t ctr) {
    ctr &= 0xFFFFFF;
    if (!p->seen[slot]) {
        p->last[slot] = ctr;
        p->mask[slot] = 1;
        p->seen[slot] = 1;
        return 1;
    }
    uint32_t last = p->last[slot] & 0xFFFFFF;
    uint32_t fwd = (ctr - last) & 0xFFFFFF;
    if (fwd == 0) return 0;
    if (fwd < (1u << 23)) {
        /* Para a frente: avança (corta o que cai fora dos 64). */
        p->mask[slot] = (fwd >= 64) ? 1 : ((p->mask[slot] << fwd) | 1);
        p->last[slot] = ctr;
        return 1;
    }
    uint32_t back = (last - ctr) & 0xFFFFFF;
    if (back == 0 || back > 64) return 0;
    uint64_t bit = 1ULL << back;
    if (p->mask[slot] & bit) return 0;
    p->mask[slot] |= bit;
    return 1;
}

/**
 * Ponta-a-ponta: estrutura + CRC + TAG + replay. Nº de campos ou 0xFF.
 * O CTR só entra na janela depois do TAG válido (aceitar CTR de trama forjada
 * envenenaria a janela e faria DoS às tramas boas seguintes).
 */
uint8_t bythos_validate(const uint8_t* buffer, size_t length,
                        const uint8_t key[32], BythosPeers* peers) {
    if (buffer == NULL || key == NULL || peers == NULL) return 0xFF;
    uint8_t count = bythos_peek(buffer, length);
    if (count == 0xFF) return 0xFF;

    uint16_t src = (uint16_t)buffer[2] | ((uint16_t)buffer[3] << 8);
    size_t fields_end = length - BYTHOS_SEC_HDR_SIZE - BYTHOS_TAG_SIZE - BYTHOS_CRC16_SIZE;
    uint8_t key_id = buffer[fields_end];
    if (key_id == BYTHOS_KEY_ID_LEGACY) return 0xFF; /* Sem TAG não há o que verificar. */
    if (src == BYTHOS_BROADCAST) return 0xFF;        /* Difusão não é emissor. */
    uint32_t ctr = (uint32_t)buffer[fields_end + 1] |
                   ((uint32_t)buffer[fields_end + 2] << 8) |
                   ((uint32_t)buffer[fields_end + 3] << 16);
    size_t tag_at = fields_end + BYTHOS_SEC_HDR_SIZE;
    if (!bythos_tag_verify(key, ctr, buffer, fields_end + BYTHOS_SEC_HDR_SIZE, &buffer[tag_at]))
        return 0xFF;

    /* Procura a ranhura (src, key_id); nova ocupa vazia ou despeja a mais velha. */
    int slot = -1;
    for (int i = 0; i < 8; i++) {
        if (peers->used[i] && peers->src[i] == src && peers->key_id[i] == key_id) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        int victim = -1;
        uint32_t oldest = 0xFFFFFFFF;
        for (int i = 0; i < 8; i++) {
            if (!peers->used[i]) { victim = i; break; }
            if (peers->stamps[i] < oldest) { oldest = peers->stamps[i]; victim = i; }
        }
        slot = victim;
        peers->src[slot] = src;
        peers->key_id[slot] = key_id;
        peers->last[slot] = 0;
        peers->mask[slot] = 0;
        peers->seen[slot] = 0;
        peers->used[slot] = 1;
    }
    peers->tick++;
    peers->stamps[slot] = peers->tick;
    if (!peers_accept(peers, slot, ctr)) return 0xFF;
    return count;
}

void bythos_parse_fields(const uint8_t* data, size_t length,
                         BythosField* output, size_t* count) {
    if (data == NULL || output == NULL || count == NULL) return;
    size_t capacity = *count;
    size_t off = 0, parsed = 0;
    /* Tamanhos canónicos por tipo (raw = livre até 32; vídeo raw até 128). */
    static const uint8_t canon[8] = {0, 4, 2, 4, 4, 2, 1, 1};
    while (off + BYTHOS_FIELD_HEADER_SIZE <= length && parsed < capacity) {
        uint8_t id = data[off];
        uint8_t len = data[off + 1];
        uint8_t ftype = (id >> 5) & 0x07;
        int ok;
        if (ftype == BYTHOS_FIELD_RAW) {
            ok = (len <= BYTHOS_MAX_FIELD_DATA) ||
                 (id == BYTHOS_FIELD_VIDEO_PAYLOAD && len <= BYTHOS_MAX_VIDEO_DATA);
        } else {
            ok = (len == canon[ftype]);
        }
        if (!ok) break;
        if (off + BYTHOS_FIELD_HEADER_SIZE + len > length) break;
        if (len > BYTHOS_MAX_FIELD_DATA) break; /* Não cabe em BythosField. */
        output[parsed].id = id;
        output[parsed].len = len;
        memcpy(output[parsed].data, &data[off + BYTHOS_FIELD_HEADER_SIZE], len);
        off += BYTHOS_FIELD_HEADER_SIZE + len;
        parsed++;
    }
    *count = parsed;
}

// ============================================================================
// CAMPOS
// ============================================================================

int8_t bythos_field_add(BythosMessage* msg, uint8_t id, const uint8_t* data, uint8_t len) {
    if (msg == NULL || data == NULL) return -1;
    if (!is_valid_field_id(id)) return -1;
    if (len > BYTHOS_MAX_FIELD_DATA) return -1;
    if (msg->field_count >= BYTHOS_MAX_FIELDS) return -1;
    BythosField* f = &msg->fields[msg->field_count];
    f->id = id;
    f->len = len;
    memcpy(f->data, data, len);
    msg->field_count++;
    return 0;
}

int8_t bythos_field_add_f32(BythosMessage* msg, uint8_t id, float value) {
    uint8_t b[4];
    bythos_f32_to_bytes(value, b);
    return bythos_field_add(msg, id, b, 4);
}

int8_t bythos_field_add_f16(BythosMessage* msg, uint8_t id, float value) {
    uint16_t h = bythos_f32_to_f16(value);
    uint8_t b[2] = {(uint8_t)(h & 0xFF), (uint8_t)((h >> 8) & 0xFF)};
    return bythos_field_add(msg, id, b, 2);
}

int8_t bythos_field_add_i32(BythosMessage* msg, uint8_t id, int32_t value) {
    uint8_t b[4];
    bythos_i32_to_bytes(value, b);
    return bythos_field_add(msg, id, b, 4);
}

int8_t bythos_field_add_u32(BythosMessage* msg, uint8_t id, uint32_t value) {
    uint8_t b[4];
    bythos_u32_to_bytes(value, b);
    return bythos_field_add(msg, id, b, 4);
}

int8_t bythos_field_add_u16(BythosMessage* msg, uint8_t id, uint16_t value) {
    uint8_t b[2];
    bythos_u16_to_bytes(value, b);
    return bythos_field_add(msg, id, b, 2);
}

int8_t bythos_field_add_u8(BythosMessage* msg, uint8_t id, uint8_t value) {
    return bythos_field_add(msg, id, &value, 1);
}

int8_t bythos_field_add_bool(BythosMessage* msg, uint8_t id, uint8_t value) {
    uint8_t b = (value != 0) ? 1 : 0;
    return bythos_field_add(msg, id, &b, 1);
}

// ============================================================================
// INICIALIZAÇÃO
// ============================================================================

void bythos_init(BythosMessage* msg, uint16_t src, uint8_t msg_id) {
    if (msg == NULL) return;
    if (!is_valid_msg_id(msg_id)) return;
    /* Campo a campo (não memset puro): o preâmbulo fica correto e `fields`
       fica válido — ver divergência V3 na migração. */
    bythos_clear(msg);
    msg->src = src;
    msg->msg_id = msg_id;
}

void bythos_set_dst(BythosMessage* msg, uint16_t dst) {
    if (msg == NULL) return;
    msg->dst = dst;
}

void bythos_set_seq(BythosMessage* msg, uint16_t seq) {
    if (msg == NULL) return;
    msg->seq_num = seq;
}

void bythos_set_hops(BythosMessage* msg, uint8_t hops) {
    if (msg == NULL) return;
    msg->hops = hops;
}

void bythos_clear(BythosMessage* msg) {
    if (msg == NULL) return;
    memset(msg, 0, sizeof(BythosMessage));
    msg->start_byte = BYTHOS_START_BYTE;
    msg->version = BYTHOS_VERSION;
    msg->dst = BYTHOS_BROADCAST;
    msg->hops = BYTHOS_HOPS_DEFAULT;
}

// ============================================================================
// CONVERSÕES (little-endian; float via memcpy, nunca union)
// ============================================================================

void bythos_f32_to_bytes(float value, uint8_t* bytes) {
    if (bytes == NULL) return;
    memcpy(bytes, &value, sizeof(float));
}

float bythos_bytes_to_f32(const uint8_t* bytes) {
    if (bytes == NULL) return 0.0f;
    float r;
    memcpy(&r, bytes, sizeof(float));
    return r;
}

/** f32 → f16 (bits): rebasa o expoente 127→15, satura em infinito. */
uint16_t bythos_f32_to_f16(float value) {
    uint32_t b;
    memcpy(&b, &value, 4);
    uint16_t sign = (uint16_t)((b >> 31) & 0x1);
    int32_t exp = (int32_t)((b >> 23) & 0xFF) - 127;
    uint32_t mant = b & 0x7FFFFF;
    if (exp > 15) return (uint16_t)((sign << 15) | (0x1F << 10));
    if (exp < -24) return (uint16_t)(sign << 15);
    if (exp < -14) {
        uint32_t shift = (uint32_t)(-exp - 14);
        uint32_t m = (mant | 0x800000) >> (shift + 13);
        return (uint16_t)((sign << 15) | m);
    }
    uint16_t m = (uint16_t)((mant + 0x1000) >> 13);
    int32_t e = exp + 15;
    if (m == 0x400) return (uint16_t)((sign << 15) | ((e + 1) << 10));
    return (uint16_t)((sign << 15) | (e << 10) | (m & 0x3FF));
}

/** f16 (bits) → f32: rebasa 15→127, preserva subnormais/NaN. */
float bythos_f16_to_f32(uint16_t h) {
    uint32_t sign = ((uint32_t)h >> 15) & 0x1;
    int32_t exp = (((uint32_t)h >> 10) & 0x1F);
    uint32_t mant = h & 0x3FF;
    uint32_t b;
    if (exp == 0) {
        if (mant == 0) {
            b = sign << 31;
        } else {
            int32_t e = -14;
            while ((mant & 0x400) == 0) { mant <<= 1; e--; }
            mant &= 0x3FF;
            b = (sign << 31) | ((uint32_t)(e + 127) << 23) | (mant << 13);
        }
    } else if (exp == 31) {
        b = (sign << 31) | (0xFFu << 23) | (mant << 13);
    } else {
        b = (sign << 31) | ((uint32_t)(exp + 112) << 23) | (mant << 13);
    }
    float r;
    memcpy(&r, &b, 4);
    return r;
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
    return (uint16_t)((uint16_t)bytes[0] | ((uint16_t)bytes[1] << 8));
}

// ============================================================================
// DIVERSOS
// ============================================================================

uint8_t bythos_msg_id_valid(uint8_t id) {
    return is_valid_msg_id(id) ? 1 : 0;
}

uint8_t bythos_msg_priority(uint8_t msg_id, uint8_t failsafe_active) {
    /* Tipo inválido = Low (unificado com o núcleo; a V3 divergia aqui). */
    if (!is_valid_msg_id(msg_id)) return BYTHOS_PRIORITY_LOW;
    if (failsafe_active) {
        if (msg_id == BYTHOS_MSG_DEBUG) return BYTHOS_PRIORITY_LOW;
        return BYTHOS_PRIORITY_SUPER_CRITICAL;
    }
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
        case BYTHOS_MSG_HELLO:      return BYTHOS_PRIORITY_HIGH;
        case BYTHOS_MSG_COUNT:      return BYTHOS_PRIORITY_HIGH;
        case BYTHOS_MSG_RING_OPEN:  return BYTHOS_PRIORITY_HIGH;
        case BYTHOS_MSG_WIRING_FAULT: return BYTHOS_PRIORITY_HIGH;
        default:                    return BYTHOS_PRIORITY_LOW;
    }
}

const char* bythos_version(void) {
    return "4.0.0";
}

size_t bythos_overhead(void) {
    return BYTHOS_OVERHEAD;
}

size_t bythos_max_message_size(void) {
    return BYTHOS_MAX_MESSAGE_SIZE;
}

// ============================================================================
// COBS (tubo opaco para rádios — sem zeros na saída)
// ============================================================================

bythos_ssize_t bythos_cobs_encode(const uint8_t* data, size_t len,
                                 uint8_t* output, size_t out_size) {
    if (data == NULL || output == NULL || out_size < 2) return -1;
    size_t read = 0, write = 1, code_at = 0;
    uint8_t code = 1;
    output[0] = 0; /* Reserva a distância do primeiro bloco. */
    while (read < len) {
        if (data[read] == 0) {
            if (write >= out_size) return -1;
            output[code_at] = code;
            code_at = write++;
            code = 1;
            read++;
        } else {
            if (write >= out_size) return -1;
            output[write++] = data[read++];
            code++;
            if (code == 0xFF) {
                if (write >= out_size) return -1;
                output[code_at] = code;
                code_at = write++;
                code = 1;
            }
        }
    }
    output[code_at] = code;
    return (bythos_ssize_t)write;
}

bythos_ssize_t bythos_cobs_decode(const uint8_t* data, size_t len,
                                 uint8_t* output, size_t out_size) {
    if (data == NULL || output == NULL) return -1;
    size_t read = 0, write = 0;
    while (read < len) {
        uint8_t code = data[read++];
        if (code == 0) return -1;
        size_t span = code - 1;
        if (read + span > len || write + span > out_size) return -1;
        for (size_t i = 0; i < span; i++) {
            if (data[read + i] == 0) return -1; /* Zero em COBS = corrupção. */
        }
        memcpy(output + write, data + read, span);
        read += span;
        write += span;
        if (code < 0xFF && read < len) {
            if (write >= out_size) return -1;
            output[write++] = 0;
        }
    }
    return (bythos_ssize_t)write;
}
