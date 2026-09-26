/**
 * @file bythos.h
 * @brief Bythos Protocol v4.0.0 — Ligações C
 *
 * Cabeçalho único para usar o Bythos V4 a partir de C e C++: construção,
 * selagem (HMAC-SHA256 truncado), validação ponta-a-ponta, análise de campos,
 * decisão de sentido no anel e COBS para rádios.
 *
 * Nomenclatura exclusivamente Bythos (ver docs/MIGRATION-GUIDE.md):
 * não há `CAN`, `TLV` nem `DeviceX` nesta API.
 *
 * Trama V4 no fio:
 * [INÍCIO:1][VERSÃO:1][ORIGEM:2 LE][DESTINO:2 LE][MSG:1][SEQ:2 LE]
 * [N_CAMPOS:1][SALTOS:1][CAMPOS...][SEC_HDR:4][TAG:4][CRC16:2 LE]
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 4.0.0
 */

#ifndef BYTHOS_H
#define BYTHOS_H

#include <stdint.h>
#include <stddef.h>

typedef int64_t bythos_ssize_t;

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// CONSTANTES DO FIO (v4.0.0)
// ============================================================================

/// Âncora de sincronização (primeiro byte de cada trama).
#define BYTHOS_START_BYTE       0xAA
/// Versão do fio (a V4 rejeita 0x03 por defeito).
#define BYTHOS_VERSION          0x04
/// Cabeçalho: 1+1+2+2+1+2+1+1 = 11 bytes.
#define BYTHOS_HEADER_SIZE      11
/// Cabeçalho de segurança: [KEY_ID:1][CTR:3].
#define BYTHOS_SEC_HDR_SIZE     4
/// Etiqueta de autenticação (HMAC-SHA256 truncado a 32 bits).
#define BYTHOS_TAG_SIZE         4
/// CRC16 no fim da trama.
#define BYTHOS_CRC16_SIZE       2
/// Sobrecarga total: 11 + 4 + 4 + 2 = 21 bytes.
#define BYTHOS_OVERHEAD         21
/// Campos por mensagem (máximo).
#define BYTHOS_MAX_FIELDS       32
/// Dados por campo normal (máximo).
#define BYTHOS_MAX_FIELD_DATA   32
/// Dados da carga vídeo (máximo; só tipo raw, um por trama).
#define BYTHOS_MAX_VIDEO_DATA   128
/// Trama máxima: 11 + 31*34 + 130 + 10 = 1205 bytes.
#define BYTHOS_MAX_MESSAGE_SIZE 1205
/// Cabeçalho de cada campo: [FIELD_ID:1][LEN:1].
#define BYTHOS_FIELD_HEADER_SIZE 2
/// Chave de selagem (sempre 32 bytes).
#define BYTHOS_KEY_SIZE         32
/// Ranhura que marca modo legado V3 (sem TAG real; migração apenas).
#define BYTHOS_KEY_ID_LEGACY    0xFF
/// Difusão (destino para todos).
#define BYTHOS_BROADCAST        0xFFFF
/// Raiz do anel (único endereço configurável à mão).
#define BYTHOS_ROOT_ADDR        0x0000
/// Saltos por defeito.
#define BYTHOS_HOPS_DEFAULT     32
/// Teto do COBS: 1205 + teto(1205/254) + 1 = 1211.
#define BYTHOS_TUNNEL_MAX       1211

// ============================================================================
// TIPOS DE CAMPO (3 bits embutidos no identificador)
// ============================================================================

typedef enum {
    BYTHOS_FIELD_RAW      = 0, /**< Carga bruta (único até 128 B). */
    BYTHOS_FIELD_FLOAT32  = 1, /**< Vírgula flutuante 32 bits. */
    BYTHOS_FIELD_FLOAT16  = 2, /**< Meia precisão (ver bythos_f32_to_f16). */
    BYTHOS_FIELD_INT32    = 3, /**< Inteiro com sinal 32 bits. */
    BYTHOS_FIELD_UINT32   = 4, /**< Inteiro sem sinal 32 bits. */
    BYTHOS_FIELD_UINT16   = 5, /**< Inteiro sem sinal 16 bits. */
    BYTHOS_FIELD_UINT8    = 6, /**< Inteiro sem sinal 8 bits. */
    BYTHOS_FIELD_BOOL     = 7  /**< Booleano. */
} BythosFieldType;

// ============================================================================
// CATÁLOGO — IDENTIFICADORES JÁ CODIFICADOS [TIPO:3][ID:5]
// ============================================================================
// Valores herdados da V3, intocados; só os nomes são Bythos.

#define BYTHOS_FIELD_GPS_LATITUDE     0x26
#define BYTHOS_FIELD_GPS_LONGITUDE    0x27
#define BYTHOS_FIELD_GPS_ALTITUDE     0x28
#define BYTHOS_FIELD_GPS_SPEED        0x29
#define BYTHOS_FIELD_GPS_COURSE       0x2A
#define BYTHOS_FIELD_GPS_SATELLITES   0xC7
#define BYTHOS_FIELD_GPS_HDOP         0x2B

#define BYTHOS_FIELD_IMU_ROLL         0x30
#define BYTHOS_FIELD_IMU_PITCH        0x31
#define BYTHOS_FIELD_IMU_YAW          0x32
#define BYTHOS_FIELD_IMU_ACCEL_X      0x33
#define BYTHOS_FIELD_IMU_ACCEL_Y      0x34
#define BYTHOS_FIELD_IMU_ACCEL_Z      0x35
#define BYTHOS_FIELD_IMU_GYRO_X       0x36
#define BYTHOS_FIELD_IMU_GYRO_Y       0x37
#define BYTHOS_FIELD_IMU_GYRO_Z       0x38
#define BYTHOS_FIELD_IMU_YAW_RATE     0x39

#define BYTHOS_FIELD_FLIGHT_ALT_GPS   0x40
#define BYTHOS_FIELD_FLIGHT_ALT_BARO  0x41
#define BYTHOS_FIELD_FLIGHT_VSPEED    0x42
#define BYTHOS_FIELD_FLIGHT_AIRSPEED  0x43
#define BYTHOS_FIELD_FLIGHT_LOOPTIME  0xA2

#define BYTHOS_FIELD_POWER_BATT_V     0x50
#define BYTHOS_FIELD_POWER_BATT_I     0x51
#define BYTHOS_FIELD_POWER_BATT_CONS  0x52
#define BYTHOS_FIELD_POWER_BATT_TEMP  0x53
#define BYTHOS_FIELD_POWER_BATT_SOC   0x54

#define BYTHOS_FIELD_TEMP_1           0x60
#define BYTHOS_FIELD_TEMP_2           0x61
#define BYTHOS_FIELD_TEMP_3           0x62
#define BYTHOS_FIELD_TEMP_4           0x63
#define BYTHOS_FIELD_TEMP_BRIDGE1     0x64
#define BYTHOS_FIELD_TEMP_BRIDGE2     0x65

#define BYTHOS_FIELD_SYSTEM_STATE     0xC0
#define BYTHOS_FIELD_SYSTEM_MODE      0xC1
#define BYTHOS_FIELD_SYSTEM_UPTIME    0x82
#define BYTHOS_FIELD_SYSTEM_FREE_HEAP 0x83
#define BYTHOS_FIELD_SYSTEM_CPU_LOAD  0xC4
#define BYTHOS_FIELD_SYSTEM_BRIDGE1_LOAD 0xC5
#define BYTHOS_FIELD_SYSTEM_BRIDGE2_LOAD 0xC6

#define BYTHOS_FIELD_FAILSAFE_REASON  0xC8
#define BYTHOS_FIELD_FAILSAFE_ACTION  0xC9
#define BYTHOS_FIELD_FAILSAFE_STATE   0xCA

#define BYTHOS_FIELD_VIDEO_FRAME_ID   0xA0
#define BYTHOS_FIELD_VIDEO_CHUNK_ID   0xC3
#define BYTHOS_FIELD_VIDEO_TOTAL_CHUNKS 0xCB
#define BYTHOS_FIELD_VIDEO_PAYLOAD    0x00

// ============================================================================
// TIPOS DE MENSAGEM (0x10-0x1F; 0x1C-0x1F nascem com o anel)
// ============================================================================

typedef enum {
    BYTHOS_MSG_HEARTBEAT  = 0x10,
    BYTHOS_MSG_TELEMETRY  = 0x11,
    BYTHOS_MSG_COMMAND    = 0x12,
    BYTHOS_MSG_ACK        = 0x13,
    BYTHOS_MSG_FAILSAFE   = 0x14,
    BYTHOS_MSG_DEBUG      = 0x15,
    BYTHOS_MSG_VIDEO      = 0x16,
    BYTHOS_MSG_SHELL      = 0x17,
    BYTHOS_MSG_SIDATA     = 0x18,
    BYTHOS_MSG_WATCHDOG   = 0x19,
    BYTHOS_MSG_PING       = 0x1A,
    BYTHOS_MSG_CLOCK      = 0x1B,
    BYTHOS_MSG_HELLO      = 0x1C, /**< Enumeração (posição). Novo V4. */
    BYTHOS_MSG_COUNT      = 0x1D, /**< Enumeração (total N). Novo V4. */
    BYTHOS_MSG_RING_OPEN  = 0x1E, /**< Anel partido. Novo V4. */
    BYTHOS_MSG_WIRING_FAULT = 0x1F /**< Erro de cablagem. Novo V4. */
} BythosMsgId;

// ============================================================================
// GRUPOS BYTHOS (papel funcional — nomes de função, valores V3)
// ============================================================================

typedef enum {
    BYTHOS_GROUP_NONE      = 0x0,
    BYTHOS_GROUP_CONTROL   = 0x1, /**< Orquestração central. */
    BYTHOS_GROUP_SENSORS   = 0x2, /**< Aquisição de sensores. */
    BYTHOS_GROUP_ACTUATORS = 0x3, /**< Controlo de atuadores. */
    BYTHOS_GROUP_SAFETY    = 0x4, /**< Supervisão de segurança. */
    BYTHOS_GROUP_EMERGENCY = 0x5, /**< Resposta a emergência. */
    BYTHOS_GROUP_VISION    = 0x6, /**< Visão por computador. */
    BYTHOS_GROUP_AUX7      = 0x7,
    BYTHOS_GROUP_AUX8      = 0x8,
    BYTHOS_GROUP_AUX9      = 0x9,
    BYTHOS_GROUP_AUX10     = 0xA,
    BYTHOS_GROUP_AUX11     = 0xB,
    BYTHOS_GROUP_AUX12     = 0xC,
    BYTHOS_GROUP_AUX13     = 0xD,
    BYTHOS_GROUP_AUX14     = 0xE,
    BYTHOS_GROUP_AUX15     = 0xF
} BythosGroup;

// ============================================================================
// ESPÉCIES DE TRÁFEGO (natureza — deriva-se do MSG, sem CAN ID)
// ============================================================================

typedef enum {
    BYTHOS_KIND_DATA   = 0x0,
    BYTHOS_KIND_CMD    = 0x1,
    BYTHOS_KIND_ACK    = 0x2,
    BYTHOS_KIND_EVENT  = 0x3,
    BYTHOS_KIND_SYNC   = 0x4,
    BYTHOS_KIND_STATE  = 0x5,
    BYTHOS_KIND_HEART  = 0x6,
    BYTHOS_KIND_SAFETY = 0x7,
    BYTHOS_KIND_RING   = 0x8 /**< Gestão do anel. Novo V4. */
} BythosKind;

// ============================================================================
// PRIORIDADES
// ============================================================================

typedef enum {
    BYTHOS_PRIORITY_SUPER_CRITICAL = 0,
    BYTHOS_PRIORITY_CRITICAL       = 1,
    BYTHOS_PRIORITY_HIGH           = 2,
    BYTHOS_PRIORITY_MEDIUM         = 3,
    BYTHOS_PRIORITY_LOW            = 4
} BythosPriorityLevel;

// ============================================================================
// SISTEMA / VOO / FAILSAFE (cargas úteis — inalterados da V3)
// ============================================================================

typedef enum {
    BYTHOS_STATE_BOOTING      = 0,
    BYTHOS_STATE_INITIALIZING = 1,
    BYTHOS_STATE_READY        = 2,
    BYTHOS_STATE_ARMED        = 3,
    BYTHOS_STATE_IN_FLIGHT    = 4,
    BYTHOS_STATE_LANDING      = 5,
    BYTHOS_STATE_ERROR        = 6,
    BYTHOS_STATE_SHUTDOWN     = 7
} BythosSystemState;

typedef enum {
    BYTHOS_FLIGHT_MANUAL    = 0,
    BYTHOS_FLIGHT_STABILIZE = 1,
    BYTHOS_FLIGHT_ALT_HOLD  = 2,
    BYTHOS_FLIGHT_AUTO      = 3,
    BYTHOS_FLIGHT_GUIDED    = 4,
    BYTHOS_FLIGHT_RTL       = 5
} BythosFlightMode;

typedef enum {
    BYTHOS_FAILSAFE_NONE           = 0,
    BYTHOS_FAILSAFE_SIGNAL_LOST    = 1,
    BYTHOS_FAILSAFE_LOW_BATTERY    = 2,
    BYTHOS_FAILSAFE_GPS_LOST       = 3,
    BYTHOS_FAILSAFE_SENSOR_FAILURE = 4,
    BYTHOS_FAILSAFE_MANUAL_TRIGGER = 5
} BythosFailsafeReason;

typedef enum {
    BYTHOS_FAILSAFE_ACTION_NONE     = 0,
    BYTHOS_FAILSAFE_ACTION_HOVER    = 1,
    BYTHOS_FAILSAFE_ACTION_LAND     = 2,
    BYTHOS_FAILSAFE_ACTION_RTL      = 3,
    BYTHOS_FAILSAFE_ACTION_CONTINUE = 4,
    BYTHOS_FAILSAFE_ACTION_DISARM   = 5
} BythosFailsafeAction;

// ============================================================================
// ESTRUTURAS
// ============================================================================

/** Um campo Bythos: identificador com tipo + comprimento + dados. */
typedef struct {
    uint8_t id;                             /**< Identificador [TIPO:3][ID:5]. */
    uint8_t len;                            /**< Bytes válidos em `data`. */
    uint8_t data[BYTHOS_MAX_FIELD_DATA];    /**< Carga (só `len` bytes valem). */
} BythosField;

/** Rascunho de mensagem V4 (imagem em memória; o fio nasce em `bythos_build`). */
typedef struct {
    uint8_t start_byte;                     /**< Sempre 0xAA. */
    uint8_t version;                        /**< Sempre 0x04. */
    uint16_t src;                           /**< Origem no anel. */
    uint16_t dst;                           /**< Destino (0xFFFF = difusão). */
    uint8_t msg_id;                         /**< Tipo de mensagem. */
    uint16_t seq_num;                       /**< Sequência (diagnóstico). */
    uint8_t field_count;                    /**< Campos válidos em `fields`. */
    uint8_t hops;                           /**< Saltos restantes. */
    BythosField fields[BYTHOS_MAX_FIELDS];  /**< Os campos. */
    uint8_t key_id;                         /**< Ranhura que selou (após build). */
    uint32_t ctr;                           /**< Contador que selou (24 úteis). */
    uint8_t tag[BYTHOS_TAG_SIZE];           /**< Etiqueta (após build). */
    uint16_t checksum;                      /**< CRC16 (após build). */
} BythosMessage;

/** Tabela anti-replay (8 emissores, sem heap; o firmware reserva-a estática). */
typedef struct {
    uint16_t src[8];        /**< Emissor de cada ranhura. */
    uint8_t key_id[8];      /**< Ranhura de chave. */
    uint32_t last[8];       /**< Último CTR aceite (24 bits). */
    uint64_t mask[8];       /**< Janela dos 64 anteriores. */
    uint8_t used[8];        /**< Ranhura ocupada. */
    uint8_t seen[8];        /**< Já aceitou alguma trama. */
    uint32_t stamps[8];     /**< Relógio LRU para despejo. */
    uint32_t tick;          /**< Relógio global. */
} BythosPeers;

// ============================================================================
// CÓDIGOS DE ERRO (espelham ProtocolError do Rust, sem colapsar)
// ============================================================================

typedef enum {
    BYTHOS_OK = 0,
    BYTHOS_ERR_BUFFER = -1,   /**< Tampão curto ou nulo. */
    BYTHOS_ERR_FIELDS = -2,   /**< Demasiados campos. */
    BYTHOS_ERR_START = -3,    /**< Sem âncora 0xAA. */
    BYTHOS_ERR_VERSION = -4,  /**< Versão recusada. */
    BYTHOS_ERR_MSG = -5,      /**< Tipo desconhecido. */
    BYTHOS_ERR_COUNT = -6,    /**< Contagem acima do teto. */
    BYTHOS_ERR_LEN = -7,      /**< Comprimento de campo inválido. */
    BYTHOS_ERR_CRC = -8,      /**< CRC não bate. */
    BYTHOS_ERR_TAG = -9,      /**< TAG não bate. */
    BYTHOS_ERR_REPLAY = -10,  /**< CTR repetido/fora da janela. */
    BYTHOS_ERR_HOPS = -11,    /**< Saltos esgotados. */
    BYTHOS_ERR_KEY = -12      /**< Chave/ranhura desconhecida. */
} BythosStatus;

// ============================================================================
// SHA-256 + HMAC (software puro; produção usa o ATECC608 via driver)
// ============================================================================

/** SHA-256 de `data` (32 bytes em `out`). */
void bythos_sha256(const uint8_t* data, size_t len, uint8_t out[32]);

/** HMAC-SHA256 completo (32 bytes em `out`). */
void bythos_hmac_sha256(const uint8_t* key, size_t key_len,
                        const uint8_t* msg, size_t msg_len, uint8_t out[32]);

/** TAG V4 (4 bytes em `tag_out`); 0 = ok, -1 = erro. */
int8_t bythos_tag_compute(const uint8_t key[32], uint32_t ctr,
                          const uint8_t* covered, size_t covered_len,
                          uint8_t tag_out[4]);

/** Verifica o TAG em tempo constante (1 = válido). */
uint8_t bythos_tag_verify(const uint8_t key[32], uint32_t ctr,
                          const uint8_t* covered, size_t covered_len,
                          const uint8_t tag[4]);

/** Etiqueta XOR legada V3 (migração apenas). */
uint8_t bythos_legacy_tag_compute(uint8_t key, uint8_t msg_id, uint8_t seq_lo, uint8_t seq_hi);

/** Verifica a etiqueta legada (1 = válida). */
uint8_t bythos_legacy_tag_validate(uint8_t tag, uint8_t key, uint8_t msg_id,
                                   uint8_t seq_lo, uint8_t seq_hi);

// ============================================================================
// CRC
// ============================================================================

/** CRC-16/CCITT (0xFFFF em nulo/vazio). */
uint16_t bythos_calc_crc16(const uint8_t* data, size_t len);

/** CRC-8/SMBUS legado (migração). */
uint8_t bythos_calc_crc8(const uint8_t* data, size_t len);

// ============================================================================
// IDENTIFICADOR DE CAMPO
// ============================================================================

/** Codifica [TIPO:3][ID:5]; 0xFF se tipo > 7 ou id > 31. */
uint8_t bythos_field_id_encode(uint8_t field_type, uint8_t field_id);

/** Decodifica nos componentes (nulos = sem efeito). */
void bythos_field_id_decode(uint8_t field_id, uint8_t* type_out, uint8_t* id_out);

/** Valida o tipo embutido (1 = válido). */
uint8_t bythos_field_id_valid(uint8_t field_id);

// ============================================================================
// IDENTIFICADOR DO BYTHOS BUS (arbitragem — a "cópia" do CAN, em Bythos)
// ============================================================================
//
// Layout u32: [PRIO:3][GRUPO:4][ESPÉCIE:4][ORIGEM:16][RES:5].
// Valor menor = maior prioridade (convenção CAN: o menor ganha sem colisão).
// O RS-485 arbitra por firmware (backoff ordenado), não por dominante
// elétrico — a semântica "menor ganha" é o contrato (ver docs/BYTHOS-BUS.md).

/** Empacota o ID de arbitragem; 0 = metadados inválidos (validar antes). */
uint32_t bythos_bus_id_make(uint8_t priority, uint8_t group, uint8_t kind, uint16_t src);

/** Prioridade (bits 31-29). */
uint8_t bythos_bus_id_priority(uint32_t id);

/** Grupo funcional (bits 28-25). */
uint8_t bythos_bus_id_group(uint32_t id);

/** Espécie de tráfego (bits 24-21). */
uint8_t bythos_bus_id_kind(uint32_t id);

/** Origem no anel (bits 20-5). */
uint16_t bythos_bus_id_src(uint32_t id);

/** 1 se `a` ganha a `b` (menor valor ganha, como no CAN). */
uint8_t bythos_bus_id_wins(uint32_t a, uint32_t b);

// ============================================================================
// ENDEREÇAMENTO (sem CAN ID na V4)
// ============================================================================

/** Espécie a partir do MSG (0xFF = desconhecido). */
uint8_t bythos_kind_of(uint8_t msg_id);

/** 1 se o grupo funcional é conhecido. */
uint8_t bythos_group_valid(uint8_t group);

/** 1 se a espécie é a de segurança. */
uint8_t bythos_is_safety_kind(uint8_t kind);

/** Sentido no anel: 0 = AFTER (frente), 1 = BEFORE (trás), 2 = local. */
uint8_t bythos_ring_decide(uint16_t me, uint16_t total, uint16_t dst);

// ============================================================================
// CONSTRUÇÃO E VALIDAÇÃO
// ============================================================================

/**
 * Sela e serializa (devolve bytes escritos, -1 em erro).
 *
 * @param msg     Rascunho (campos + origem + sequência + saltos).
 * @param msg_id  Tipo (sobrepõe o rascunho).
 * @param dst     Destino (sobrepõe o rascunho).
 * @param key_id  Ranhura de chave (0-253).
 * @param ctr     Contador monotónico (só 24 bits viajam).
 * @param key     Chave de 32 bytes (nunca nula).
 */
bythos_ssize_t bythos_build(const BythosMessage* msg, uint8_t msg_id, uint16_t dst,
                            uint8_t key_id, uint32_t ctr, const uint8_t key[32],
                            uint8_t* buffer, size_t buffer_size);

/** Só estrutura + CRC (para encaminhar sem chaves); nº de campos ou 0xFF. */
uint8_t bythos_peek(const uint8_t* buffer, size_t length);

/** Ponta-a-ponta (TAG + replay); nº de campos ou 0xFF. */
uint8_t bythos_validate(const uint8_t* buffer, size_t length,
                        const uint8_t key[32], BythosPeers* peers);

/** Zera a tabela anti-replay (comissionamento / troca de chave). */
void bythos_peers_clear(BythosPeers* peers);

/** Desserializa campos; `*count`: entrada = capacidade, saída = lidos (0 em erro). */
void bythos_parse_fields(const uint8_t* data, size_t length,
                         BythosField* output, size_t* count);

// ============================================================================
// CAMPOS
// ============================================================================

/** Adiciona carga bruta (0 = ok, -1 = erro; nunca trunca). */
int8_t bythos_field_add(BythosMessage* msg, uint8_t id, const uint8_t* data, uint8_t len);

/** Adiciona f32. */
int8_t bythos_field_add_f32(BythosMessage* msg, uint8_t id, float value);

/** Adiciona f16 (meia precisão, 2 bytes). Novo V4. */
int8_t bythos_field_add_f16(BythosMessage* msg, uint8_t id, float value);

/** Adiciona i32. */
int8_t bythos_field_add_i32(BythosMessage* msg, uint8_t id, int32_t value);

/** Adiciona u32. */
int8_t bythos_field_add_u32(BythosMessage* msg, uint8_t id, uint32_t value);

/** Adiciona u16. */
int8_t bythos_field_add_u16(BythosMessage* msg, uint8_t id, uint16_t value);

/** Adiciona u8. */
int8_t bythos_field_add_u8(BythosMessage* msg, uint8_t id, uint8_t value);

/** Adiciona booleano. Novo V4. */
int8_t bythos_field_add_bool(BythosMessage* msg, uint8_t id, uint8_t value);

// ============================================================================
// INICIALIZAÇÃO
// ============================================================================

/** Prepara o rascunho com origem e tipo (inválidos = sem efeito). */
void bythos_init(BythosMessage* msg, uint16_t src, uint8_t msg_id);

/** Define o destino (0xFFFF = difusão). */
void bythos_set_dst(BythosMessage* msg, uint16_t dst);

/** Define a sequência (diagnóstico). */
void bythos_set_seq(BythosMessage* msg, uint16_t seq);

/** Define os saltos restantes. */
void bythos_set_hops(BythosMessage* msg, uint8_t hops);

/** Repõe o rascunho (preâmbulo correto, resto a zero). */
void bythos_clear(BythosMessage* msg);

// ============================================================================
// CONVERSÕES (tudo little-endian, ordem do fio)
// ============================================================================

void bythos_f32_to_bytes(float value, uint8_t* bytes);
float bythos_bytes_to_f32(const uint8_t* bytes);
/** f32 → meia precisão (bits, saturando). Novo V4. */
uint16_t bythos_f32_to_f16(float value);
/** Meia precisão → f32. Novo V4. */
float bythos_f16_to_f32(uint16_t bits);
void bythos_i32_to_bytes(int32_t value, uint8_t* bytes);
int32_t bythos_bytes_to_i32(const uint8_t* bytes);
void bythos_u32_to_bytes(uint32_t value, uint8_t* bytes);
uint32_t bythos_bytes_to_u32(const uint8_t* bytes);
void bythos_u16_to_bytes(uint16_t value, uint8_t* bytes);
uint16_t bythos_bytes_to_u16(const uint8_t* bytes);

// ============================================================================
// DIVERSOS
// ============================================================================

/** 1 se o tipo de mensagem é conhecido. */
uint8_t bythos_msg_id_valid(uint8_t id);

/** Prioridade efetiva (tipo inválido = Low, unificado com o núcleo). */
uint8_t bythos_msg_priority(uint8_t msg_id, uint8_t failsafe_active);

/** Versão ("4.0.0", estática, não libertar). */
const char* bythos_version(void);

/** Sobrecarga (21). */
size_t bythos_overhead(void);

/** Trama máxima (1205). */
size_t bythos_max_message_size(void);

// ============================================================================
// TÚNEL COBS (rádios)
// ============================================================================

/** Codifica COBS (bytes escritos, -1 em erro). */
bythos_ssize_t bythos_cobs_encode(const uint8_t* data, size_t len,
                                 uint8_t* output, size_t out_size);

/** Descodifica COBS (bytes escritos, -1 em erro). */
bythos_ssize_t bythos_cobs_decode(const uint8_t* data, size_t len,
                                 uint8_t* output, size_t out_size);

#ifdef __cplusplus
}
#endif

#endif /* BYTHOS_H */
