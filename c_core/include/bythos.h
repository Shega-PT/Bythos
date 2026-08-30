/**
 * @file bythos.h
 * @brief Bythos Protocol v3.0.0 — C/C++ Bindings
 *
 * Header principal para utilização do protocolo Bythos a partir de C e C++.
 * Fornece funções para construção, validação e parsing de mensagens TLV.
 *
 * ## Formato da Mensagem
 *
 * ```
 * [START_BYTE][VERSION][NODE_ID][MSG_ID][SEQ_NUM(2)][TLV_COUNT]
 * [TLV_FIELDS...][SIGNATURE][CRC16(2)]
 * ```
 *
 * ## Campo TLV — FieldID com Tipo Embutido
 *
 * ```
 * FieldID = [TYPE:3bits][ID:5bits]
 *
 *   Bits 7-5: Tipo de dado (0-7)
 *   Bits 4-0: ID do campo (0-31)
 * ```
 *
 * @author ShegaPT
 * @license GPL-3.0
 * @version 3.0.0
 */

#ifndef BYTHOS_H
#define BYTHOS_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

typedef int64_t bythos_ssize_t;

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// CONSTANTES
// ============================================================================

#define BYTHOS_START_BYTE       0xAA
#define BYTHOS_VERSION          0x03
#define BYTHOS_HEADER_SIZE      7
#define BYTHOS_SIGNATURE_SIZE   1
#define BYTHOS_CRC16_SIZE       2
#define BYTHOS_OVERHEAD         10
#define BYTHOS_MAX_TLV_FIELDS   32
#define BYTHOS_MAX_TLV_DATA     32
#define BYTHOS_MAX_MESSAGE_SIZE 1098
#define BYTHOS_TLV_HEADER_SIZE  2

// ============================================================================
// ENUMS — FIELD TYPES (3 bits)
// ============================================================================

typedef enum {
    BYTHOS_FIELD_RAW      = 0,
    BYTHOS_FIELD_FLOAT32  = 1,
    BYTHOS_FIELD_FLOAT16  = 2,
    BYTHOS_FIELD_INT32    = 3,
    BYTHOS_FIELD_UINT32   = 4,
    BYTHOS_FIELD_UINT16   = 5,
    BYTHOS_FIELD_UINT8    = 6,
    BYTHOS_FIELD_BOOL     = 7,
} BythosFieldType;

// ============================================================================
// FIELDID UNIFICADO — Constantes pré-codificadas [TYPE:3][ID:5]
//
// Alinhado com o enum Rust FieldId em protocol/types.rs.
// Cada valor combina o tipo de dado (bits 7-5) e o ID do campo (bits 4-0).
//
// Formato: FieldID = (field_type << 5) | field_id
// ============================================================================

// --- GPS — Tipo 1 (f32), IDs 0x06-0x0C ---
#define BYTHOS_FIELD_GPS_LATITUDE     0x26  // TYPE=1(f32) + ID=6
#define BYTHOS_FIELD_GPS_LONGITUDE    0x27  // TYPE=1(f32) + ID=7
#define BYTHOS_FIELD_GPS_ALTITUDE     0x28  // TYPE=1(f32) + ID=8
#define BYTHOS_FIELD_GPS_SPEED        0x29  // TYPE=1(f32) + ID=9
#define BYTHOS_FIELD_GPS_COURSE       0x2A  // TYPE=1(f32) + ID=10
#define BYTHOS_FIELD_GPS_SATELLITES   0xC7  // TYPE=6(u8)  + ID=7
#define BYTHOS_FIELD_GPS_HDOP         0x2B  // TYPE=1(f32) + ID=11

// --- IMU — Tipo 1 (f32), IDs 0x10-0x19 ---
#define BYTHOS_FIELD_IMU_ROLL         0x30  // TYPE=1(f32) + ID=0x10
#define BYTHOS_FIELD_IMU_PITCH        0x31  // TYPE=1(f32) + ID=0x11
#define BYTHOS_FIELD_IMU_YAW          0x32  // TYPE=1(f32) + ID=0x12
#define BYTHOS_FIELD_IMU_ACCEL_X      0x33  // TYPE=1(f32) + ID=0x13
#define BYTHOS_FIELD_IMU_ACCEL_Y      0x34  // TYPE=1(f32) + ID=0x14
#define BYTHOS_FIELD_IMU_ACCEL_Z      0x35  // TYPE=1(f32) + ID=0x15
#define BYTHOS_FIELD_IMU_GYRO_X       0x36  // TYPE=1(f32) + ID=0x16
#define BYTHOS_FIELD_IMU_GYRO_Y       0x37  // TYPE=1(f32) + ID=0x17
#define BYTHOS_FIELD_IMU_GYRO_Z       0x38  // TYPE=1(f32) + ID=0x18
#define BYTHOS_FIELD_IMU_YAW_RATE     0x39  // TYPE=1(f32) + ID=0x19

// --- Voo — Tipo 1 (f32), IDs 0x20-0x23; LoopTime é u16 (0xA2) ---
#define BYTHOS_FIELD_FLIGHT_ALT_GPS   0x40  // TYPE=1(f32) + ID=0x20
#define BYTHOS_FIELD_FLIGHT_ALT_BARO  0x41  // TYPE=1(f32) + ID=0x21
#define BYTHOS_FIELD_FLIGHT_VSPEED    0x42  // TYPE=1(f32) + ID=0x22
#define BYTHOS_FIELD_FLIGHT_AIRSPEED  0x43  // TYPE=1(f32) + ID=0x23
#define BYTHOS_FIELD_FLIGHT_LOOPTIME  0xA2  // TYPE=5(u16) + ID=2

// --- Energia — Tipo 1 (f32), IDs 0x30-0x34 ---
#define BYTHOS_FIELD_POWER_BATT_V     0x50  // TYPE=1(f32) + ID=0x30
#define BYTHOS_FIELD_POWER_BATT_I     0x51  // TYPE=1(f32) + ID=0x31
#define BYTHOS_FIELD_POWER_BATT_CONS  0x52  // TYPE=1(f32) + ID=0x32
#define BYTHOS_FIELD_POWER_BATT_TEMP  0x53  // TYPE=1(f32) + ID=0x33
#define BYTHOS_FIELD_POWER_BATT_SOC   0x54  // TYPE=1(f32) + ID=0x34

// --- Temperatura — Tipo 1 (f32), IDs 0x40-0x45 ---
#define BYTHOS_FIELD_TEMP_1           0x60  // TYPE=1(f32) + ID=0x40
#define BYTHOS_FIELD_TEMP_2           0x61  // TYPE=1(f32) + ID=0x41
#define BYTHOS_FIELD_TEMP_3           0x62  // TYPE=1(f32) + ID=0x42
#define BYTHOS_FIELD_TEMP_4           0x63  // TYPE=1(f32) + ID=0x43
#define BYTHOS_FIELD_TEMP_ESP1        0x64  // TYPE=1(f32) + ID=0x44
#define BYTHOS_FIELD_TEMP_ESP2        0x65  // TYPE=1(f32) + ID=0x45

// --- Sistema — Tipos mistos (u8, u32) ---
#define BYTHOS_FIELD_SYSTEM_STATE     0xC0  // TYPE=6(u8)  + ID=0
#define BYTHOS_FIELD_SYSTEM_MODE      0xC1  // TYPE=6(u8)  + ID=1
#define BYTHOS_FIELD_SYSTEM_UPTIME    0x82  // TYPE=4(u32) + ID=2
#define BYTHOS_FIELD_SYSTEM_FREE_HEAP 0x83  // TYPE=4(u32) + ID=3
#define BYTHOS_FIELD_SYSTEM_CPU_LOAD  0xC4  // TYPE=6(u8)  + ID=4
#define BYTHOS_FIELD_SYSTEM_ESP1_LOAD 0xC5  // TYPE=6(u8)  + ID=5
#define BYTHOS_FIELD_SYSTEM_ESP2_LOAD 0xC6  // TYPE=6(u8)  + ID=6

// --- Failsafe — Tipo 6 (u8) ---
#define BYTHOS_FIELD_FAILSAFE_REASON  0xC8  // TYPE=6(u8) + ID=8
#define BYTHOS_FIELD_FAILSAFE_ACTION  0xC9  // TYPE=6(u8) + ID=9
#define BYTHOS_FIELD_FAILSAFE_STATE   0xCA  // TYPE=6(u8) + ID=10

// --- Vídeo — Tipos mistos (u16, u8, raw) ---
#define BYTHOS_FIELD_VIDEO_FRAME_ID   0xA0  // TYPE=5(u16) + ID=0
#define BYTHOS_FIELD_VIDEO_CHUNK_ID   0xC3  // TYPE=6(u8)  + ID=3
#define BYTHOS_FIELD_VIDEO_TOTAL_CHUNKS 0xCB // TYPE=6(u8) + ID=11
#define BYTHOS_FIELD_VIDEO_PAYLOAD    0x00  // TYPE=0(raw) + ID=0

// ============================================================================
// ENUMS — MSG IDs
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
} BythosMsgId;

// ============================================================================
// ENUMS — CAN GROUPS
// ============================================================================

typedef enum {
    BYTHOS_GROUP_NONE     = 0x0,
    BYTHOS_GROUP_DEVICE0  = 0x1,
    BYTHOS_GROUP_DEVICE1  = 0x2,
    BYTHOS_GROUP_DEVICE2  = 0x3,
    BYTHOS_GROUP_DEVICE3  = 0x4,
    BYTHOS_GROUP_DEVICE4  = 0x5,
    BYTHOS_GROUP_DEVICE5  = 0x6,
    BYTHOS_GROUP_DEVICE6  = 0x7,
    BYTHOS_GROUP_DEVICE7  = 0x8,
    BYTHOS_GROUP_DEVICE8  = 0x9,
    BYTHOS_GROUP_DEVICE9  = 0xA,
    BYTHOS_GROUP_DEVICE10 = 0xB,
    BYTHOS_GROUP_DEVICE11 = 0xC,
    BYTHOS_GROUP_DEVICE12 = 0xD,
    BYTHOS_GROUP_DEVICE13 = 0xE,
    BYTHOS_GROUP_DEVICE14 = 0xF,
} BythosCanGroup;

// ============================================================================
// ENUMS — CAN MSG TYPES
// ============================================================================

typedef enum {
    BYTHOS_CAN_DATA   = 0x0,
    BYTHOS_CAN_CMD    = 0x1,
    BYTHOS_CAN_ACK    = 0x2,
    BYTHOS_CAN_EVENT  = 0x3,
    BYTHOS_CAN_SYNC   = 0x4,
    BYTHOS_CAN_STATE  = 0x5,
    BYTHOS_CAN_HEART  = 0x6,
    BYTHOS_CAN_SAFETY = 0x7,
} BythosCanMsgType;

// ============================================================================
// ENUMS — PRIORITY LEVELS
// ============================================================================

typedef enum {
    BYTHOS_PRIORITY_SUPER_CRITICAL = 0,
    BYTHOS_PRIORITY_CRITICAL       = 1,
    BYTHOS_PRIORITY_HIGH           = 2,
    BYTHOS_PRIORITY_MEDIUM         = 3,
    BYTHOS_PRIORITY_LOW            = 4,
} BythosPriorityLevel;

// ============================================================================
// ENUMS — SYSTEM STATE
// ============================================================================

typedef enum {
    BYTHOS_STATE_BOOTING      = 0,
    BYTHOS_STATE_INITIALIZING = 1,
    BYTHOS_STATE_READY        = 2,
    BYTHOS_STATE_ARMED        = 3,
    BYTHOS_STATE_IN_FLIGHT    = 4,
    BYTHOS_STATE_LANDING      = 5,
    BYTHOS_STATE_ERROR        = 6,
    BYTHOS_STATE_SHUTDOWN     = 7,
} BythosSystemState;

// ============================================================================
// ENUMS — FLIGHT MODE (UAV/UAS)
// ============================================================================

typedef enum {
    BYTHOS_FLIGHT_MANUAL    = 0,
    BYTHOS_FLIGHT_STABILIZE = 1,
    BYTHOS_FLIGHT_ALT_HOLD  = 2,
    BYTHOS_FLIGHT_AUTO      = 3,
    BYTHOS_FLIGHT_GUIDED    = 4,
    BYTHOS_FLIGHT_RTL       = 5,
} BythosFlightMode;

// ============================================================================
// ENUMS — FAILSAFE
// ============================================================================

typedef enum {
    BYTHOS_FAILSAFE_NONE           = 0,
    BYTHOS_FAILSAFE_SIGNAL_LOST    = 1,
    BYTHOS_FAILSAFE_LOW_BATTERY    = 2,
    BYTHOS_FAILSAFE_GPS_LOST       = 3,
    BYTHOS_FAILSAFE_SENSOR_FAILURE = 4,
    BYTHOS_FAILSAFE_MANUAL_TRIGGER = 5,
} BythosFailsafeReason;

typedef enum {
    BYTHOS_FAILSAFE_ACTION_NONE     = 0,
    BYTHOS_FAILSAFE_ACTION_HOVER    = 1,
    BYTHOS_FAILSAFE_ACTION_LAND     = 2,
    BYTHOS_FAILSAFE_ACTION_RTL      = 3,
    BYTHOS_FAILSAFE_ACTION_CONTINUE = 4,
    BYTHOS_FAILSAFE_ACTION_DISARM   = 5,
} BythosFailsafeAction;

// ============================================================================
// STRUCTS
// ============================================================================

/**
 * @brief Campo TLV (Type-Length-Value).
 *
 * FieldID contém tipo embutido: [TYPE:3][ID:5]
 */
typedef struct {
    uint8_t id;                    ///< FieldID codificado [TYPE:3][ID:5]
    uint8_t len;                   ///< Número de bytes de dados
    uint8_t data[BYTHOS_MAX_TLV_DATA]; ///< Dados do campo
} BythosTLVField;

/**
 * @brief Mensagem Bythos completa.
 *
 * Representa uma mensagem Bythos com todos os campos necessários
 * para serialização e validação.
 */
typedef struct {
    uint8_t start_byte;            ///< Byte de início (0xAA)
    uint8_t version;               ///< Versão do protocolo (0x03)
    uint8_t node_id;               ///< ID do grupo CAN deste nó
    uint8_t msg_id;                ///< ID do tipo de mensagem
    uint16_t seq_num;              ///< Número de sequência (16 bits)
    uint8_t tlv_count;             ///< Número de campos TLV
    BythosTLVField tlvs[BYTHOS_MAX_TLV_FIELDS]; ///< Campos TLV
    uint8_t signature;             ///< Assinatura XOR
    uint16_t checksum;             ///< Checksum CRC16
} BythosMessage;

// ============================================================================
// FFI — CRC
// ============================================================================

/**
 * Calcula CRC-16/CCITT de um array de bytes.
 *
 * @param data  Ponteiro para os dados
 * @param len   Número de bytes
 * @return      Valor CRC-16 (0x0000-0xFFFF)
 */
uint16_t bythos_calc_crc16(const uint8_t* data, size_t len);

/**
 * Calcula CRC-8/SMBUS de um array de bytes (legado).
 *
 * @param data  Ponteiro para os dados
 * @param len   Número de bytes
 * @return      Valor CRC-8 (0x00-0xFF)
 */
uint8_t bythos_calc_crc8(const uint8_t* data, size_t len);

// ============================================================================
// FFI — ASSINATURA
// ============================================================================

/**
 * Calcula a assinatura de uma mensagem Bythos.
 * signature = XOR(key, msg_id, seq_lo, seq_hi)
 *
 * @param key     Chave partilhada
 * @param msg_id  ID da mensagem
 * @param seq_lo  Byte baixo do SEQ_NUM
 * @param seq_hi  Byte alto do SEQ_NUM
 * @return        Byte de assinatura (0x00-0xFF)
 */
uint8_t bythos_signature_compute(uint8_t key, uint8_t msg_id, uint8_t seq_lo, uint8_t seq_hi);

/**
 * Valida a assinatura de uma mensagem Bythos.
 *
 * @param signature  Assinatura recebida
 * @param key        Chave partilhada
 * @param msg_id     ID da mensagem
 * @param seq_lo     Byte baixo do SEQ_NUM
 * @param seq_hi     Byte alto do SEQ_NUM
 * @return           1 se válida, 0 caso contrário
 */
uint8_t bythos_signature_validate(uint8_t signature, uint8_t key, uint8_t msg_id, uint8_t seq_lo, uint8_t seq_hi);

// ============================================================================
// FFI — FIELDID COM TIPO
// ============================================================================

/**
 * Codifica um FieldID com tipo embutido.
 * FieldID = [TYPE:3][ID:5]
 *
 * @param field_type  Tipo de dado (0-7)
 * @param field_id    ID do campo (0-31)
 * @return            FieldID codificado, ou 0xFF em erro
 */
uint8_t bythos_field_id_encode(uint8_t field_type, uint8_t field_id);

/**
 * Decodifica um FieldID nos seus componentes.
 *
 * @param field_id  FieldID codificado
 * @param type_out  Ponteiro para o tipo de dado (output)
 * @param id_out    Ponteiro para o ID do campo (output)
 */
void bythos_field_id_decode(uint8_t field_id, uint8_t* type_out, uint8_t* id_out);

/**
 * Valida se um FieldID codificado tem um tipo válido.
 *
 * @param field_id  FieldID codificado
 * @return          1 se válido, 0 caso contrário
 */
uint8_t bythos_field_id_valid(uint8_t field_id);

// ============================================================================
// FFI — CAN ID
// ============================================================================

/**
 * Constrói um CAN ID extended (29-bit).
 *
 * Formato:
 *   Bits 28-26: Prioridade (3 bits)
 *   Bits 25-22: Grupo origem (4 bits)
 *   Bits 21-18: Grupo destino (4 bits)
 *   Bits 17-14: Tipo mensagem (4 bits)
 *   Bits 13-0:  Reservado (14 bits)
 *
 * @param priority   Nível de prioridade (0-4)
 * @param src_group  Grupo de origem (0x0-0xF)
 * @param dst_group  Grupo de destino (0x0=broadcast)
 * @param msg_type   Tipo de mensagem (0x0-0x7)
 * @return           CAN ID de 29 bits
 */
uint32_t bythos_can_id_make(uint8_t priority, uint8_t src_group, uint8_t dst_group, uint8_t msg_type);

/**
 * Extrai a prioridade de um CAN ID extended.
 */
uint8_t bythos_can_id_priority(uint32_t can_id);

/**
 * Extrai o grupo de origem de um CAN ID extended.
 */
uint8_t bythos_can_id_src(uint32_t can_id);

/**
 * Extrai o grupo de destino de um CAN ID extended.
 */
uint8_t bythos_can_id_dst(uint32_t can_id);

/**
 * Extrai o tipo de mensagem de um CAN ID extended.
 */
uint8_t bythos_can_id_type(uint32_t can_id);

/**
 * Verifica se o CAN ID pertence ao bus de segurança.
 */
uint8_t bythos_is_safety_bus(uint32_t can_id);

// ============================================================================
// FFI — SERIALIZAÇÃO
// ============================================================================

/**
 * Serializa uma mensagem Bythos completa num buffer de saída.
 *
 * @param msg           Mensagem a serializar
 * @param msg_id        Identificador do tipo de mensagem
 * @param signature_key Chave de assinatura
 * @param buffer        Buffer de saída
 * @param buffer_size   Tamanho do buffer de saída
 * @return              Número de bytes escritos, ou -1 em erro
 */
bythos_ssize_t bythos_build(const BythosMessage* msg, uint8_t msg_id, uint8_t signature_key,
                           uint8_t* buffer, size_t buffer_size);

/**
 * Valida a integridade e estrutura de uma mensagem Bythos serializada.
 *
 * @param buffer  Mensagem serializada
 * @param length  Tamanho da mensagem em bytes
 * @return        Número de campos TLV válidos, ou 0xFF em erro
 */
/**
 * Valida uma mensagem Bythos completa num buffer.
 *
 * Verifica: byte de início, versão, msg_id, tlv_count,
 * integridade dos campos TLV (bounds e tamanhos) e CRC16.
 *
 * @param buffer  Buffer contendo a mensagem
 * @param length  Tamanho do buffer
 * @return        tlv_count se válido, 0xFF em erro
 */
uint8_t bythos_validate(const uint8_t* buffer, size_t length);

/**
 * Valida a assinatura de uma mensagem Bythos serializada.
 *
 * @param buffer         Mensagem serializada
 * @param length         Tamanho da mensagem em bytes
 * @param signature_key  Chave de assinatura esperada
 * @return               1 se válida, 0 caso contrário
 */
uint8_t bythos_validate_signature(const uint8_t* buffer, size_t length, uint8_t signature_key);

/**
 * Deserializa campos TLV de um buffer de bytes brutos.
 *
 * @param data    Bytes dos campos TLV
 * @param length  Tamanho dos dados em bytes
 * @param output  Array de saída de BythosTLVField
 * @param count   [input] Capacidade do array, [output] Número de campos real
 */
void bythos_parse_tlv(const uint8_t* data, size_t length, BythosTLVField* output, size_t* count);

// ============================================================================
// FFI — ADICIONAR CAMPOS TLV
// ============================================================================

/**
 * Adiciona um campo TLV com dados brutos a uma mensagem.
 *
 * @param msg  Mensagem Bythos
 * @param id   FieldID codificado
 * @param data Dados do campo
 * @param len  Número de bytes de dados
 * @return     0 em sucesso, -1 em erro
 */
int8_t bythos_tlv_add(BythosMessage* msg, uint8_t id, const uint8_t* data, uint8_t len);

/**
 * Adiciona um campo TLV com valor float (f32).
 * @return 0 em sucesso, -1 em erro
 */
int8_t bythos_tlv_add_f32(BythosMessage* msg, uint8_t id, float value);

/**
 * Adiciona um campo TLV com valor i32.
 * @return 0 em sucesso, -1 em erro
 */
int8_t bythos_tlv_add_i32(BythosMessage* msg, uint8_t id, int32_t value);

/**
 * Adiciona um campo TLV com valor u32.
 * @return 0 em sucesso, -1 em erro
 */
int8_t bythos_tlv_add_u32(BythosMessage* msg, uint8_t id, uint32_t value);

/**
 * Adiciona um campo TLV com valor u16.
 * @return 0 em sucesso, -1 em erro
 */
int8_t bythos_tlv_add_u16(BythosMessage* msg, uint8_t id, uint16_t value);

/**
 * Adiciona um campo TLV com valor u8.
 * @return 0 em sucesso, -1 em erro
 */
int8_t bythos_tlv_add_u8(BythosMessage* msg, uint8_t id, uint8_t value);

// ============================================================================
// FFI — INICIALIZAÇÃO DE MENSAGEM
// ============================================================================

/**
 * Inicializa uma mensagem Bythos com node_id e campos padrão.
 *
 * @param msg     Mensagem a inicializar
 * @param node_id ID do grupo CAN deste nó (0-15)
 * @param msg_id  ID da mensagem
 */
void bythos_init(BythosMessage* msg, uint8_t node_id, uint8_t msg_id);

/**
 * Define o número de sequência de uma mensagem.
 */
void bythos_set_seq(BythosMessage* msg, uint16_t seq);

/**
 * Limpa uma mensagem Bythos (reseta todos os campos).
 */
void bythos_clear(BythosMessage* msg);

// ============================================================================
// FFI — CONVERSÃO DE BYTES
// ============================================================================

/**
 * Converte float para bytes (little-endian).
 */
void bythos_f32_to_bytes(float value, uint8_t* bytes);

/**
 * Converte bytes (little-endian) para float.
 */
float bythos_bytes_to_f32(const uint8_t* bytes);

/**
 * Converte i32 para bytes (little-endian).
 */
void bythos_i32_to_bytes(int32_t value, uint8_t* bytes);

/**
 * Converte bytes (little-endian) para i32.
 */
int32_t bythos_bytes_to_i32(const uint8_t* bytes);

/**
 * Converte u32 para bytes (little-endian).
 */
void bythos_u32_to_bytes(uint32_t value, uint8_t* bytes);

/**
 * Converte bytes (little-endian) para u32.
 */
uint32_t bythos_bytes_to_u32(const uint8_t* bytes);

/**
 * Converte u16 para bytes (little-endian).
 */
void bythos_u16_to_bytes(uint16_t value, uint8_t* bytes);

/**
 * Converte bytes (little-endian) para u16.
 */
uint16_t bythos_bytes_to_u16(const uint8_t* bytes);

// ============================================================================
// FFI — VALIDAÇÃO
// ============================================================================

/**
 * Retorna true (1) se o ID da mensagem é válido.
 */
uint8_t bythos_msg_id_valid(uint8_t id);

/**
 * Retorna a prioridade de uma mensagem.
 */
uint8_t bythos_msg_priority(uint8_t msg_id, uint8_t failsafe_active);

// ============================================================================
// FFI — UTILITÁRIOS
// ============================================================================

/**
 * Retorna a versão do protocolo como string estática.
 * @return Ponteiro para string "3.0.0" (não libertar)
 */
const char* bythos_version(void);

/**
 * Retorna o tamanho do overhead Bythos (header + signature + crc16).
 */
size_t bythos_overhead(void);

/**
 * Retorna o tamanho máximo de uma mensagem Bythos.
 */
size_t bythos_max_message_size(void);

#ifdef __cplusplus
}
#endif

#endif /* BYTHOS_H */
