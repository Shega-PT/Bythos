#pragma once

/**
 * =================================================================================
 * PROTOCOL.H — Bythos Protocol V1 (GENÉRICO / UART)
 * =================================================================================
 *
 * AUTOR:      ShegaPT
 * DATA:       2026-04-19
 * VERSÃO:     1.0.0
 *
 * =================================================================================
 * DESCRIÇÃO GERAL
 * =================================================================================
 *
 * Este ficheiro define o protocolo de comunicação TLV (Type-Length-Value) versão
 * 1.0.0 — uma versão simplificada e genérica do Bythos Protocol V1.
 *
 * Características da v1.0.0:
 *   • Suporte APENAS a UART (sem LoRa, sem GSM, sem camadas extras)
 *   • Sem autenticação HMAC/SEQ (integridade assegurada apenas por CRC8)
 *   • FieldIDs genéricos — não presos a nenhum domínio específico
 *   • Buffer reduzido (256 bytes) para ambientes UART
 *   • 6 tipos de mensagem, 5 comandos básicos
 *
 * Formato da mensagem no canal (byte stream):
 *
 *   ┌─────────┬─────────┬───────────┬─────────────────┬─────────┐
 *   │ START   │ MSG ID  │ TLV COUNT │ TLV FIELDS      │ CRC8    │
 *   │ (1 byte)│ (1 byte)│ (1 byte)  │ (variável)      │ (1 byte)│
 *   ├─────────┼─────────┼───────────┼─────────────────┼─────────┤
 *   │ 0xAA    │ 0x10-15 │ 0-8       │ ID(1)+LEN(1)+N  │ CRC-8   │
 *   │         │         │           │                 │ SMBUS   │
 *   │         │         │           │                 │ 0x07    │
 *   └─────────┴─────────┴───────────┴─────────────────┴─────────┘
 *
 * Regras de ouro para futuras modificações:
 *   1. Nunca alterar IDs existentes — apenas ADICIONAR novos.
 *   2. Incrementar PROTOCOL_VERSION_MINOR para funcionalidades novas.
 *   3. Incrementar PROTOCOL_VERSION_MAJOR para alterações incompatíveis.
 *
 * Endianness:
 *   Todas as palavras multibyte são transmitidas em LITTLE-ENDIAN.
 *
 * =================================================================================
 */

#include <stdint.h>
#include <string.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* =================================================================================
 * VERSÃO DO PROTOCOLO
 * =================================================================================
 * Identificação única desta versão. Usada em logging e compatibilidade.
 * v1.0.0 = primeira versão funcional do protocolo genérico.
 */

#define PROTOCOL_VERSION_MAJOR 1
#define PROTOCOL_VERSION_MINOR 0
#define PROTOCOL_VERSION_PATCH 0
#define PROTOCOL_VERSION_STR   "1.0.0"

/* =================================================================================
 * CONSTANTES FUNDAMENTAIS
 * =================================================================================
 * Definem os limites físicos e lógicos do protocolo.
 * Todos os buffers e validações dependem destas constantes.
 */

/**
 * START_BYTE — Byte de sincronização de trama.
 *
 * Valor 0xAA (170 decimal) = padrão binário 10101010.
 * Facilmente distinguível de ruído aleatório e de bytes de controlo ASCII.
 * O parser procura activamente este byte para iniciar uma nova trama.
 */
#define START_BYTE 0xAA

/**
 * MAX_TLV_FIELDS — Número máximo de campos TLV por mensagem.
 *
 * Valor: 8 campos.
 * Razão: Suficiente para uma mensagem genérica de telemetria ou comando
 *        sem desperdiçar memória em buffers estáticos. Ambientes UART
 *        tipicamente trabalham com tramas curtas para minimizar latência.
 */
#define MAX_TLV_FIELDS     8

/**
 * MAX_TLV_DATA — Tamanho máximo do payload de um campo TLV normal.
 *
 * Valor: 16 bytes.
 * Razão: Suporta float (4B), int32_t (4B), uint32_t (4B), arrays pequenos
 *        e strings curtas. Reduzido de 32 para 16 porque a v1.0.0 é genérica
 *        e não precisa de payloads extensos.
 */
#define MAX_TLV_DATA       16

/**
 * MAX_MESSAGE_SIZE — Tamanho máximo de uma mensagem serializada.
 *
 * Valor: 256 bytes.
 * Cálculo: START(1) + MSGID(1) + COUNT(1) + 8*18 + CRC(1) = ~150
 *          Limitado a 256 para compatibilidade com buffers UART típicos
 *          (115200 baud, 256-512 bytes de RX buffer no ESP32).
 */
#define MAX_MESSAGE_SIZE   256

/**
 * MIN_MESSAGE_SIZE — Tamanho mínimo de uma mensagem válida.
 *
 * Valor: 4 bytes.
 * Composição: START(1) + MSGID(1) + COUNT(1) + CRC(1) = 4 bytes.
 * Uma mensagem com tlvCount=0 não contém campos TLV — apenas cabeçalho + CRC.
 */
#define MIN_MESSAGE_SIZE   4

/* =================================================================================
 * IDENTIFICADORES DE MENSAGEM (MsgID)
 * =================================================================================
 *
 * Cada tipo de mensagem tem um ID único no intervalo 0x10 - 0x15.
 * Apenas 6 tipos de mensagem são suportados na v1.0.0.
 *
 * Intervalos atribuídos:
 *   0x00-0x0F: Reservado para uso interno do protocolo
 *   0x10-0x1F: Mensagens de sistema e controlo (v1.0.0 usa 0x10-0x15)
 *   0x20-0x7F: Campos de dados TLV (ver Field* enums)
 *   0x80-0x9F: Reservado para expansão futura
 *   0xA0-0xAF: Reservado para segurança/failsafe futuro
 *   0xC0-0xFF: Comandos (ver Command* enums)
 */

enum MsgID : uint8_t {
    MSG_HEARTBEAT = 0x10,   /* Trama deheartbeat — indica que o emissor está vivo */
    MSG_TELEMETRY = 0x11,   /* Dados de telemetria genérica (estado, sensores) */
    MSG_COMMAND   = 0x12,   /* Comando enviado de um nó para outro */
    MSG_ACK       = 0x13,   /* Confirmação de receção de uma mensagem */
    MSG_FAILSAFE  = 0x14,   /* Notificação de modo de segurança / emergência */
    MSG_DEBUG     = 0x15    /* Mensagens de debug (prioridade dinâmica) */
};

/* =================================================================================
 * ESTRUTURAS TLV (TYPE-LENGTH-VALUE)
 * =================================================================================
 *
 * Arquitetura de memória: As structs NÃO devem ser serializadas diretamente
 * via memcpy() para o canal porque o compilador pode inserir padding.
 *
 * Em vez disso, use as funções buildMessage() / parseTLV() que serializam
 * campo a campo de forma compacta e segura.
 */

/**
 * TLVField — Campo TLV padrão para dados genéricos.
 *
 * Estrutura de 18 bytes em memória (com alinhamento a 1 byte):
 *   [ID: 1B] [LEN: 1B] [DATA: até 16B]
 *
 * O array data[] tem tamanho fixo MAX_TLV_DATA para evitar alocação dinâmica.
 * O campo len indica quantos bytes do array estão efetivamente preenchidos.
 */
typedef struct {
    uint8_t id;                     /* Identificador do campo (ver Field* enums) */
    uint8_t len;                    /* Comprimento útil do payload em bytes (0-16) */
    uint8_t data[MAX_TLV_DATA];     /* Payload — dados crus em little-endian */
} TLVField;

/**
 * TLVMessage — Estrutura lógica de uma mensagem completa.
 *
 * Conveniente para manipulação em memória, mas NÃO deve ser enviada
 * diretamente pelo canal. Use buildMessage() para serializar e
 * validateMessage() + parseTLV() para desserializar.
 *
 * A struct contém todos os campos necessários para representar
 * uma trama completa incluindo o byte de início e checksum.
 */
typedef struct {
    uint8_t     startByte;              /* Deve ser START_BYTE (0xAA) */
    uint8_t     msgID;                  /* Tipo de mensagem (ver MsgID) */
    uint8_t     tlvCount;               /* Número de campos TLV (0-8) */
    TLVField    tlvs[MAX_TLV_FIELDS];   /* Array de campos TLV */
    uint8_t     checksum;               /* CRC8 calculado sobre header+TLVs */
} TLVMessage;

/* =================================================================================
 * IDs DE CAMPOS TLV — GENÉRICOS (v1.0.0)
 * =================================================================================
 *
 * Os FieldIDs da v1.0.0 são deliberadamente genéricos e não presos a
 * nenhum domínio específico (GPS, IMU, etc.). Isto permite que o protocolo
 * seja reutilizado em qualquer contexto sem necessidade de redefinir campos.
 *
 * Intervalos atribuídos:
 *   0x20-0x2F: Dados genéricos (payload variável)
 *   0x30-0x37: Estado / status genérico
 *   0x40-0x44: Parâmetros de comando
 *
 * Cada implementação deve documentar localmente o significado de cada ID
 * utilizado, mantendo o protocolo genérico à nível de transport.
 */

/* Dados genéricos (0x20-0x2F) — payloads variáveis conforme a aplicação */
enum FieldData : uint8_t {
    FLD_DATA_0  = 0x20,   /* Dado genérico 0 — uso livre (uint8 padrão) */
    FLD_DATA_1  = 0x21,   /* Dado genérico 1 — uso livre (uint8 padrão) */
    FLD_DATA_2  = 0x22,   /* Dado genérico 2 — uso livre (uint16 padrão) */
    FLD_DATA_3  = 0x23,   /* Dado genérico 3 — uso livre (uint16 padrão) */
    FLD_DATA_4  = 0x24,   /* Dado genérico 4 — uso livre (int32 padrão) */
    FLD_DATA_5  = 0x25,   /* Dado genérico 5 — uso livre (int32 padrão) */
    FLD_DATA_6  = 0x26,   /* Dado genérico 6 — uso livre (float padrão) */
    FLD_DATA_7  = 0x27    /* Dado genérico 7 — uso livre (float padrão) */
};

/* Estado / status genérico (0x30-0x37) — indicadores de estado do sistema */
enum FieldStatus : uint8_t {
    FLD_STATUS_0 = 0x30,  /* Estado genérico 0 — uso livre (uint8) */
    FLD_STATUS_1 = 0x31,  /* Estado genérico 1 — uso livre (uint8) */
    FLD_STATUS_2 = 0x32   /* Estado genérico 2 — uso livre (uint16) */
};

/* Parâmetros de comando (0x40-0x44) — valores associados a comandos */
enum FieldCmd : uint8_t {
    FLD_CMD_0    = 0x40,  /* Parâmetro de comando 0 — uso livre (uint8) */
    FLD_CMD_1    = 0x41,  /* Parâmetro de comando 1 — uso livre (uint8) */
    FLD_CMD_2    = 0x42   /* Parâmetro de comando 2 — uso livre (float) */
};

/* =================================================================================
 * IDs DE COMANDO (0xC0-0xC4)
 * =================================================================================
 *
 * Apenas 5 comandos básicos são suportados na v1.0.0.
 * Estes valores são enviados no payload de um TLV com ID apropriado
 * dentro de uma mensagem MSG_COMMAND.
 *
 * Formato do payload de um MSG_COMMAND:
 *   [CMD_ID (1 byte)] [PARÂMETROS (opcional, variável)]
 */

enum CommandBasic : uint8_t {
    CMD_ARM       = 0xC0,   /* Armar — prepara o sistema para operação ativa */
    CMD_DISARM    = 0xC1,   /* Desarmar — coloca o sistema em modo seguro */
    CMD_SET_MODE  = 0xC2,   /* Definir modo — parâmetro: uint8_t com o modo */
    CMD_REBOOT    = 0xC3,   /* Reiniciar — reinicia o dispositivo remoto */
    CMD_SHUTDOWN  = 0xC4    /* Desligar — desligamento controlado do sistema */
};

/* =================================================================================
 * FUNÇÕES DE VALIDAÇÃO EM TEMPO DE COMPILAÇÃO
 * =================================================================================
 *
 * Estas funções constexpr permitem validação em tempo de compilação
 * quando possível, ou em tempo de运行 com overhead zero (inline).
 */

/**
 * isValidMsgID — Verifica se um ID de mensagem é válido.
 *
 * @param id ID da mensagem a validar
 * @return true se estiver no intervalo 0x10-0x15
 */
static inline constexpr bool isValidMsgID(uint8_t id) {
    return (id >= 0x10 && id <= 0x15);
}

/**
 * isValidFieldID — Verifica se um ID de campo TLV é válido.
 *
 * Intervalos válidos na v1.0.0:
 *   0x20-0x2F (dados genéricos)
 *   0x30-0x37 (estado/status)
 *   0x40-0x44 (parâmetros de comando)
 *
 * @param id ID do campo a validar
 * @return true se estiver num intervalo reservado
 */
static inline constexpr bool isValidFieldID(uint8_t id) {
    return ((id >= 0x20 && id <= 0x2F) ||
            (id >= 0x30 && id <= 0x37) ||
            (id >= 0x40 && id <= 0x44));
}

/* =================================================================================
 * FUNÇÕES DE CONVERSÃO DE TIPOS (LITTLE-ENDIAN)
 * =================================================================================
 *
 * Estas funções garantem transmissão consistente de dados multibyte
 * entre emissor e recetor. Utilizam memcpy() para preservar a representação
 * binária IEEE 754 dos floats e a ordem de bytes little-endian.
 *
 * Nota: O ESP32 é little-endian nativo, mas estas funções garantem
 * compatibilidade mesmo em plataformas big-endian (raro, mas possível).
 */

/**
 * floatToBytes — Converte float para array de 4 bytes (little-endian).
 *
 * @param value Float de entrada (IEEE 754)
 * @param bytes Array de 4 bytes de saída
 */
static inline void floatToBytes(float value, uint8_t* bytes) {
    memcpy(bytes, &value, 4);
}

/**
 * bytesToFloat — Converte array de 4 bytes para float (little-endian).
 *
 * @param bytes Array de 4 bytes de entrada
 * @return Float reconstruído
 */
static inline float bytesToFloat(const uint8_t* bytes) {
    float value;
    memcpy(&value, bytes, 4);
    return value;
}

/**
 * int32ToBytes — Converte int32_t para array de 4 bytes (little-endian).
 *
 * @param val Inteiro de 32 bits com sinal
 * @param bytes Array de 4 bytes de saída
 */
static inline void int32ToBytes(int32_t val, uint8_t* bytes) {
    memcpy(bytes, &val, 4);
}

/**
 * bytesToInt32 — Converte array de 4 bytes para int32_t (little-endian).
 *
 * @param bytes Array de 4 bytes de entrada
 * @return int32_t reconstruído
 */
static inline int32_t bytesToInt32(const uint8_t* bytes) {
    int32_t val;
    memcpy(&val, bytes, 4);
    return val;
}

/**
 * uint32ToBytes — Converte uint32_t para array de 4 bytes (little-endian).
 *
 * @param val Inteiro de 32 bits sem sinal
 * @param bytes Array de 4 bytes de saída
 */
static inline void uint32ToBytes(uint32_t val, uint8_t* bytes) {
    memcpy(bytes, &val, 4);
}

/**
 * bytesToUint32 — Converte array de 4 bytes para uint32_t (little-endian).
 *
 * @param bytes Array de 4 bytes de entrada
 * @return uint32_t reconstruído
 */
static inline uint32_t bytesToUint32(const uint8_t* bytes) {
    uint32_t val;
    memcpy(&val, bytes, 4);
    return val;
}

/**
 * uint16ToBytes — Converte uint16_t para array de 2 bytes (little-endian).
 *
 * @param val Inteiro de 16 bits sem sinal
 * @param bytes Array de 2 bytes de saída
 */
static inline void uint16ToBytes(uint16_t val, uint8_t* bytes) {
    bytes[0] = val & 0xFF;
    bytes[1] = (val >> 8) & 0xFF;
}

/**
 * bytesToUint16 — Converte array de 2 bytes para uint16_t (little-endian).
 *
 * @param bytes Array de 2 bytes de entrada
 * @return uint16_t reconstruído
 */
static inline uint16_t bytesToUint16(const uint8_t* bytes) {
    return (uint16_t)bytes[0] | ((uint16_t)bytes[1] << 8);
}

/* =================================================================================
 * FUNÇÕES AUXILIARES (IMPLEMENTADAS EM Protocol.cpp)
 * =================================================================================
 *
 * Estas funções são o coração do protocolo — serialização, validação
 * e desserialização de mensagens TLV.
 */

/**
 * calcCRC8 — Calcula CRC8/SMBUS sobre um buffer de dados.
 *
 * Utiliza polinómio 0x07 com tabela de lookup para O(1) por byte.
 * O CRC é calculado sobre TODO o pacote (START_BYTE até ao último TLV),
 * mas NÃO sobre o próprio byte de CRC.
 *
 * @param data Ponteiro para os dados
 * @param len  Número de bytes a processar
 * @return     CRC8 calculado (0x00-0xFF)
 */
uint8_t calcCRC8(const uint8_t* data, size_t len);

/**
 * buildTLV — Serializa um campo TLV individual para um buffer.
 *
 * Formato de saída: [ID (1B)] [LEN (1B)] [DATA (LEN bytes)]
 *
 * @param id     Identificador do campo
 * @param data   Ponteiro para os dados a serializar
 * @param len    Comprimento dos dados (máx MAX_TLV_DATA)
 * @param output Buffer de saída (mínimo len+2 bytes)
 * @return       Número de bytes escritos (0 em caso de erro)
 */
size_t buildTLV(uint8_t id, const uint8_t* data, uint8_t len, uint8_t* output);

/**
 * buildMessage — Serializa uma mensagem TLV completa para transmissão.
 *
 * Formato de saída:
 *   [START_BYTE][MSGID][TLV_COUNT][TLV0][TLV1]...[TLVn][CRC8]
 *
 * O CRC8 é calculado sobre todo o conteúdo incluindo START_BYTE,
 * mas excluindo o próprio byte de CRC.
 *
 * @param msg        Ponteiro para a mensagem estruturada
 * @param msgID      ID da mensagem (sobrescreve msg->msgID se !=0)
 * @param buffer     Buffer de saída
 * @param bufferSize Tamanho do buffer (deve ser >= MIN_MESSAGE_SIZE)
 * @return           Número de bytes escritos (0 = erro)
 */
size_t buildMessage(TLVMessage* msg, uint8_t msgID, uint8_t* buffer, size_t bufferSize);

/**
 * validateMessage — Valida estruturalmente uma mensagem recebida.
 *
 * Verificações realizadas:
 *   1. START_BYTE correto (0xAA)
 *   2. msgID válido (0x10-0x15)
 *   3. tlvCount dentro dos limites (<= MAX_TLV_FIELDS)
 *   4. Estrutura dos TLVs (sem truncamentos)
 *   5. CRC8 correspondente
 *
 * NOTA: Não existe verificação HMAC/SEQ — a integridade depende
 *       apenas do CRC8. Para ambientes mais seguros, adicionar
 *       uma camada de autenticação acima desta.
 *
 * @param buffer Buffer com a mensagem recebida
 * @param length Comprimento do buffer
 * @return       1 = válida, 0 = inválida
 */
uint8_t validateMessage(const uint8_t* buffer, size_t length);

/**
 * parseTLV — Extrai campos TLV de um buffer bruto.
 *
 * Percorre a sequência de TLVs (sem START_BYTE/MSGID/COUNT/CRC)
 * e preenche um array de TLVField com os campos extraídos.
 *
 * @param data   Buffer com dados TLV
 * @param length Comprimento do buffer
 * @param output Array de saída (mínimo MAX_TLV_FIELDS)
 * @param count  Número de campos extraídos (output)
 */
void parseTLV(const uint8_t* data, size_t length, TLVField* output, size_t* count);

/* =================================================================================
 * FUNÇÕES INLINE PARA CONSTRUÇÃO DE TLVs (C++)
 * =================================================================================
 *
 * Estas funções auxiliam na construção de mensagens TLV de forma
 * ergonómica, eliminando a necessidade de manipular bytes manualmente.
 * Disponíveis apenas em compiladores C++ devido ao uso de sobrecarga.
 */

#ifdef __cplusplus
}

/**
 * addTLV — Adiciona um campo TLV genérico a uma mensagem.
 *
 * Verifica automaticamente os limites de MAX_TLV_FIELDS e MAX_TLV_DATA.
 * Se os limites forem excedidos, a operação é ignorada silenciosamente.
 *
 * @param msg  Ponteiro para a mensagem de destino
 * @param id   Identificador do campo
 * @param data Ponteiro para os dados
 * @param len  Comprimento dos dados
 */
inline void addTLV(TLVMessage* msg, uint8_t id, const void* data, uint8_t len) {
    if (msg->tlvCount >= MAX_TLV_FIELDS || len > MAX_TLV_DATA) return;
    TLVField* f = &msg->tlvs[msg->tlvCount];
    f->id = id;
    f->len = len;
    memcpy(f->data, data, len);
    msg->tlvCount++;
}

/**
 * addTLVFloat — Adiciona um campo TLV com valor float (4 bytes).
 *
 * @param msg   Ponteiro para a mensagem de destino
 * @param id    Identificador do campo
 * @param value Valor float a transmitir
 */
inline void addTLVFloat(TLVMessage* msg, uint8_t id, float value) {
    uint8_t bytes[4];
    floatToBytes(value, bytes);
    addTLV(msg, id, bytes, 4);
}

/**
 * addTLVInt32 — Adiciona um campo TLV com valor int32_t (4 bytes).
 *
 * @param msg   Ponteiro para a mensagem de destino
 * @param id    Identificador do campo
 * @param value Valor inteiro de 32 bits com sinal
 */
inline void addTLVInt32(TLVMessage* msg, uint8_t id, int32_t value) {
    uint8_t bytes[4];
    int32ToBytes(value, bytes);
    addTLV(msg, id, bytes, 4);
}

/**
 * addTLVUint32 — Adiciona um campo TLV com valor uint32_t (4 bytes).
 *
 * @param msg   Ponteiro para a mensagem de destino
 * @param id    Identificador do campo
 * @param value Valor inteiro de 32 bits sem sinal
 */
inline void addTLVUint32(TLVMessage* msg, uint8_t id, uint32_t value) {
    uint8_t bytes[4];
    uint32ToBytes(value, bytes);
    addTLV(msg, id, bytes, 4);
}

/**
 * addTLVUint16 — Adiciona um campo TLV com valor uint16_t (2 bytes).
 *
 * @param msg   Ponteiro para a mensagem de destino
 * @param id    Identificador do campo
 * @param value Valor inteiro de 16 bits sem sinal
 */
inline void addTLVUint16(TLVMessage* msg, uint8_t id, uint16_t value) {
    uint8_t bytes[2];
    uint16ToBytes(value, bytes);
    addTLV(msg, id, bytes, 2);
}

/**
 * addTLVUint8 — Adiciona um campo TLV com valor uint8_t (1 byte).
 *
 * @param msg   Ponteiro para a mensagem de destino
 * @param id    Identificador do campo
 * @param value Valor inteiro de 8 bits sem sinal
 */
inline void addTLVUint8(TLVMessage* msg, uint8_t id, uint8_t value) {
    addTLV(msg, id, &value, 1);
}

#endif /* __cplusplus */
