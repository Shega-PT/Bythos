#pragma once

/**
 * =================================================================================
 * PROTOCOL.H — PROTOCOLO TLV v2.0.0 (PARTILHADO)
 * =================================================================================
 * 
 * AUTOR:      ShegaPT
 * DATA:       2026-04-19
 * VERSÃO:     2.0.0
 * 
 * =================================================================================
 * DESCRIÇÃO GERAL
 * =================================================================================
 * 
 * Este ficheiro define o protocolo de comunicação TLV (Type-Length-Value) utilizado
 * em TODOS os componentes do sistema Bythos Protocol V2:
 * 
 *   • ESP1 (Flight Controller)    → recebe comandos da GS via 2.4GHz
 *   • ESP2 (Sensor Processor)     → recebe RAW do ESP1, envia SI e telemetria
 *   • Ground Station (GS)         → envia comandos, recebe telemetria
 * 
 * O protocolo é deterministico, fail-secure e foi projetado para operar em tempo
 * real com latência mínima e máxima fiabilidade.
 * 
 * =================================================================================
 * FORMATO DA MENSAGEM NO CANAL (BYTE STREAM)
 * =================================================================================
 * 
 * ┌─────────┬─────────┬───────────┬─────────────────┬─────────┬──────────────┬─────────┐
 * │ START   │ MSG ID  │ TLV COUNT │ TLV FIELDS      │ CRC8    │ HMAC (32B)   │ SEQ (4B)│
 * │ (1 byte)│ (1 byte)│ (1 byte)  │ (variável)      │ (1 byte)│ (opcional)   │ (opç.)  │
 * ├─────────┼─────────┼───────────┼─────────────────┼─────────┼──────────────┼─────────┤
 * │ 0xAA    │ 0x10-18 │ 0-32      │ ID(1)+LEN(1)+N  │ CRC-8   │ Segurança    │ Anti-   │
 * │         │         │           │                 │ SMBUS   │ (módulo      │ replay  │
 * │         │         │           │                 │ 0x07    │ Security)    │         │
 * └─────────┴─────────┴───────────┴─────────────────┴─────────┴──────────────┴─────────┘
 * 
 * NOTA: O HMAC e o SEQ são geridos pelo módulo Security e NÃO fazem parte da
 *       estrutura TLVMessage. São acrescentados após a serialização.
 * 
 * =================================================================================
 * REGRAS DE OURO PARA MODIFICAÇÕES
 * =================================================================================
 * 
 * 1. Nunca alterar IDs de mensagem ou campos existentes — apenas ADICIONAR novos.
 * 2. Incrementar PROTOCOL_VERSION_MINOR quando adicionar funcionalidades.
 * 3. Incrementar PROTOCOL_VERSION_MAJOR quando fizer alterações incompatíveis.
 * 4. Qualquer alteração neste ficheiro AFETA TODOS os componentes (ESP1, ESP2, GS).
 * 5. Testar em todas as plataformas antes de integrar.
 * 
 * =================================================================================
 * ENDIANNESS
 * =================================================================================
 * 
 * Todas as palavras multibyte (uint16_t, uint32_t, float) são transmitidas em
 * LITTLE-ENDIAN (padrão ESP32 e maioria das plataformas). A Ground Station em
 * x86/x64 também é little-endian, logo sem necessidade de conversão.
 * 
 * Para comunicação com sistemas big-endian (raro), usar funções de conversão
 * htole32() / le32toh() disponíveis em endian.h.
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
 * ================================================================================= */

#define PROTOCOL_VERSION_MAJOR 2
#define PROTOCOL_VERSION_MINOR 0
#define PROTOCOL_VERSION_PATCH 0
#define PROTOCOL_VERSION_STR   "2.0.0"

/* =================================================================================
 * CONSTANTES FUNDAMENTAIS
 * ================================================================================= */

/**
 * START_BYTE - Byte de sincronização de quadro
 * 
 * Valor fixo 0xAA (170 decimal). Padrão binário 10101010 facilmente distinguível
 * de ruído aleatório. O parser procura este byte para iniciar uma nova mensagem.
 */
#define START_BYTE 0xAA

/* =================================================================================
 * LIMITES DE SEGURANÇA E CAPACIDADE
 * =================================================================================
 */

/**
 * MAX_TLV_FIELDS - Número máximo de campos TLV por mensagem
 * 
 * Valor: 32 campos
 * Razão: Suficiente para telemetria completa (GPS, IMU, potência, temperatura)
 *        sem exceder o buffer da UART/LoRa. Aumentar pode causar buffer overflow.
 */
#define MAX_TLV_FIELDS     32

/**
 * MAX_TLV_DATA - Tamanho máximo do payload de um campo TLV normal
 * 
 * Valor: 32 bytes
 * Razão: Suporta floats (4B), uint32_t (4B), arrays pequenos. Aumentado de 16
 *        para 32 para maior flexibilidade (ex: strings curtas, vetores 3D).
 */
#define MAX_TLV_DATA       32

/**
 * MAX_TLV_VIDEO_DATA - Tamanho máximo para payload de vídeo
 * 
 * Valor: 128 bytes
 * Razão: Permite enviar chunks de vídeo compresso (MJPEG pequeno) sem fragmentar
 *        excessivamente. Aumentado de 64 para 128 para maior eficiência.
 */
#define MAX_TLV_VIDEO_DATA 128

/**
 * MAX_MESSAGE_SIZE - Tamanho máximo de uma mensagem serializada
 * 
 * Valor: 1024 bytes
 * Cálculo: START(1) + MSGID(1) + COUNT(1) + 32*34 + CRC(1) = ~1090
 *          Limitado a 1024 para compatibilidade com buffers de UART/LoRa.
 */
#define MAX_MESSAGE_SIZE   1024

/**
 * MIN_MESSAGE_SIZE - Tamanho mínimo de uma mensagem válida
 * 
 * Valor: 5 bytes
 * Composição: START + MSGID + COUNT + CRC = 4 bytes MIN? Com COUNT=0 → 4?
 *            Na prática, 5 bytes com COUNT=1 e TLV vazio? Revisão: 4 bytes base.
 *            Deixamos 5 por precaução (START + MSGID + COUNT + TLV vazio? não)
 *            Correção: START(1)+MSGID(1)+COUNT(1)+CRC(1) = 4 bytes mínimo.
 *            COUNT=0 → sem TLVs → total = 4 bytes.
 */
#define MIN_MESSAGE_SIZE   4

/* =================================================================================
 * NÍVEIS DE PRIORIDADE (QoS - Quality of Service)
 * =================================================================================
 * 
 * Utilizados pelo módulo de gestão de filas para decidir qual mensagem enviar
 * primeiro quando há contenção no canal. Quanto MENOR o número, MAIOR a prioridade.
 */

enum PriorityLevel : uint8_t {
    PRIORITY_SUPER_CRITICAL = 0,  /* Failsafe, desarme de emergência */
    PRIORITY_CRITICAL       = 1,  /* Comandos de voo, mudanças de modo */
    PRIORITY_HIGH           = 2,  /* Heartbeat, telemetria crítica */
    PRIORITY_MEDIUM         = 3,  /* Telemetria normal, ACKs */
    PRIORITY_LOW            = 4,  /* Debug, logs não críticos */
    PRIORITY_SUPER_LOW      = 5   /* Vídeo, dados de diagnóstico extensivos */
};

/* =================================================================================
 * IDENTIFICADORES DE MENSAGEM (MsgID)
 * =================================================================================
 * 
 * Cada tipo de mensagem tem um ID único no intervalo 0x10 - 0x18.
 * Intervalos reservados:
 *   0x00-0x0F: Reservado para uso interno do protocolo
 *   0x10-0x1F: Mensagens de sistema e controlo
 *   0x20-0x7F: Campos de dados (ver Field* enums)
 *   0x80-0x9F: Reservado para expansão futura
 *   0xA0-0xBF: Mensagens de segurança e failsafe
 *   0xC0-0xFF: Comandos (ver Command* enums)
 */

enum MsgID : uint8_t {
    MSG_HEARTBEAT   = 0x10,   /* Coração do sistema — indica que ESP1 está vivo */
    MSG_TELEMETRY   = 0x11,   /* Dados de telemetria (ESP2 → GS via ESP1) */
    MSG_COMMAND     = 0x12,   /* Comando da GS para o ESP1 */
    MSG_ACK         = 0x13,   /* Confirmação de receção (opcional) */
    MSG_FAILSAFE    = 0x14,   /* Ativação/desativação de modo de segurança */
    MSG_DEBUG       = 0x15,   /* Mensagens de debug (apenas desenvolvimento) */
    MSG_VIDEO       = 0x16,   /* Dados de vídeo (streaming experimental) */
    MSG_SHELL_CMD   = 0x17,   /* Comando de shell remoto (diagnóstico) */
    MSG_SI_DATA     = 0x18    /* Dados em unidades SI (ESP2 → ESP1) */
};

/* =================================================================================
 * FUNÇÃO DE PRIORIDADE DINÂMICA
 * =================================================================================
 * 
 * Retorna o nível de prioridade de uma mensagem com base no seu ID e no estado
 * atual do sistema (especialmente importante para mensagens de debug durante
 * um failsafe — estas tornam-se SUPER_CRITICAL para diagnóstico).
 */

static inline uint8_t getMsgPriority(uint8_t msgID, bool failsafeActive) {
    switch (msgID) {
        case MSG_FAILSAFE:
            return PRIORITY_SUPER_CRITICAL;   /* Segurança em primeiro lugar */
        
        case MSG_COMMAND:
            return PRIORITY_CRITICAL;          /* Comandos são urgentes */
        
        case MSG_HEARTBEAT:
        case MSG_TELEMETRY:
        case MSG_SI_DATA:
            return PRIORITY_HIGH;              /* Dados de voo essenciais */
        
        case MSG_DEBUG:
            /* Debug torna-se crítico durante failsafe para diagnóstico remoto */
            return failsafeActive ? PRIORITY_SUPER_CRITICAL : PRIORITY_SUPER_LOW;
        
        case MSG_SHELL_CMD:
            return PRIORITY_LOW;               /* Shell é interativo, não urgente */
        
        case MSG_VIDEO:
            return PRIORITY_MEDIUM;            /* Vídeo tem prioridade média */
        
        case MSG_ACK:
            return PRIORITY_MEDIUM;            /* ACKs são importantes mas não críticos */
        
        default:
            return PRIORITY_SUPER_LOW;          /* Desconhecido → prioridade mínima */
    }
}

/* =================================================================================
 * ESTRUTURAS TLV (TYPE-LENGTH-VALUE)
 * =================================================================================
 * 
 * Arquitetura de memória: As structs NÃO devem ser serializadas diretamente
 * via memcpy() para o canal porque o compilador pode inserir padding.
 * 
 * Em vez disso, use as funções buildMessage() / parseTLV() que serializam
 * campo a campo de forma compacta.
 */

/**
 * TLVField - Campo TLV padrão para dados normais
 * 
 * Alinhamento: 1 byte (garantido por pragma pack, se disponível)
 * Padding: Nenhum (deliberadamente organizado para minimizar)
 */
typedef struct {
    uint8_t id;                     /* Identificador do campo (ver Field* enums) */
    uint8_t len;                    /* Comprimento do payload em bytes (0-32) */
    uint8_t data[MAX_TLV_DATA];     /* Payload (dados crus, little-endian) */
} TLVField;

/**
 * TLVVideoField - Campo TLV dedicado a dados de vídeo
 * 
 * Separado do TLVField padrão para permitir payloads maiores (até 128 bytes)
 * sem desperdiçar memória em mensagens normais.
 */
typedef struct {
    uint8_t id;                     /* Deve ser FLD_VIDEO_PAYLOAD (0xB3) */
    uint8_t len;                    /* Comprimento do chunk de vídeo (0-128) */
    uint8_t data[MAX_TLV_VIDEO_DATA]; /* Chunk de frame de vídeo */
} TLVVideoField;

/**
 * TLVMessage - Estrutura lógica de uma mensagem completa
 * 
 * NOTA: Esta struct é conveniente para manipulação em memória, mas NÃO
 *       deve ser enviada diretamente pelo canal. Use buildMessage() para
 *       serializar e validateMessage()/parseTLV() para desserializar.
 * 
 * A inclusão do startByte e checksum na struct é meramente conveniente
 * para manter toda a informação num só local.
 */
typedef struct {
    uint8_t     startByte;          /* Deve ser START_BYTE (0xAA) */
    uint8_t     msgID;              /* Tipo de mensagem (ver MsgID) */
    uint8_t     tlvCount;           /* Número de campos TLV (0-32) */
    TLVField    tlvs[MAX_TLV_FIELDS]; /* Array de campos TLV */
    uint8_t     checksum;           /* CRC8 calculado sobre header+TLVs */
} TLVMessage;

/* =================================================================================
 * ENUMS AUXILIARES PARA ESTADO DO SISTEMA
 * =================================================================================
 * 
 * Estes enums são usados como valores de campos TLV (ex: FLD_STATE, FLD_MODE)
 * e devem ser mantidos sincronizados entre ESP1, ESP2 e GS.
 */

/**
 * SystemState - Estado geral do sistema de voo
 * 
 * Transições típicas:
 *   BOOTING → INIT → IDLE → ARMED → FLYING → IDLE → DISARMED
 *   Qualquer estado → FAILSAFE (emergência)
 *   Qualquer estado → ERROR (falha irrecuperável)
 *   Qualquer estado → SHUTDOWN (desligamento controlado)
 */
enum SystemState : uint8_t {
    SYS_STATE_BOOTING   = 0,   /* Arranque do sistema, sensores a iniciar */
    SYS_STATE_INIT      = 1,   /* Inicialização, calibração de sensores */
    SYS_STATE_IDLE      = 2,   /* Pronto, à espera de comando ARM */
    SYS_STATE_ARMED     = 3,   /* Hélices prontas, motor pode ligar */
    SYS_STATE_FLYING    = 4,   /* Em voo ativo */
    SYS_STATE_FAILSAFE  = 5,   /* Modo de segurança ativo */
    SYS_STATE_ERROR     = 6,   /* Erro crítico, requer reset */
    SYS_STATE_SHUTDOWN  = 7    /* Desligamento em progresso */
};

/**
 * FlightMode - Modo de voo ativo
 * 
 * Define o comportamento do controlador de voo (PID, navegação).
 */
enum FlightMode : uint8_t {
    MODE_MANUAL     = 0,   /* Controlo direto pelo piloto (rate mode) */
    MODE_STABILIZE  = 1,   /* Auto-nivelamento (angle mode) */
    MODE_ALT_HOLD   = 2,   /* Manutenção de altitude (barómetro/GPS) */
    MODE_POSHOLD    = 3,   /* Manutenção de posição (GPS) */
    MODE_AUTO       = 4,   /* Navegação autónoma por waypoints */
    MODE_RTL        = 5    /* Return To Launch — regressar à origem */
};

/**
 * FailsafeReason - Causa da ativação do modo failsafe
 * 
 * Permite diagnóstico remoto e logging para análise pós-voo.
 */
enum FailsafeReason : uint8_t {
    FS_REASON_NONE          = 0,   /* Sem failsafe ativo */
    FS_REASON_LOST_LINK     = 1,   /* Perda de comunicação com GS (>5s) */
    FS_REASON_LOW_BATTERY   = 2,   /* Tensão da bateria abaixo do mínimo */
    FS_REASON_GPS_LOST      = 3,   /* Perda de sinal GPS em voo */
    FS_REASON_IMU_ERROR     = 4,   /* Leitura IMU anómala (vibração excessiva) */
    FS_REASON_MANUAL_TRIGGER = 5   /* Ativado manualmente via GS ou switch */
};

/**
 * FailsafeAction - Ação a executar quando failsafe é ativado
 */
enum FailsafeAction : uint8_t {
    FS_ACTION_NONE          = 0,   /* Nenhuma ação (apenas alerta) */
    FS_ACTION_HOVER         = 1,   /* Pairar no lugar */
    FS_ACTION_RTL           = 2,   /* Regressar ao ponto de descolagem */
    FS_ACTION_LAND          = 3,   /* Aterragem imediata no local */
    FS_ACTION_CUT_THROTTLE  = 4,   /* Cortar motores (emergência extrema) */
    FS_ACTION_DISARM        = 5    /* Desarmar completamente */
};

/* =================================================================================
 * IDs DE CAMPOS TLV (ORGANIZADOS POR CATEGORIA)
 * =================================================================================
 * 
 * Os IDs estão organizados em blocos para facilitar a manutenção e evitar
 * colisões. Cada bloco tem um intervalo reservado de 16 valores.
 * 
 * Intervalos atribuídos:
 *   0x20-0x2F: GPS/Navegação
 *   0x30-0x3F: IMU/Atitude
 *   0x40-0x4F: Estado de voo
 *   0x50-0x5F: Energia/Bateria
 *   0x60-0x6F: Temperatura
 *   0x70-0x7F: Sistema
 *   0xA0-0xAF: Failsafe
 *   0xB0-0xBF: Vídeo
 *   0xC0-0xFF: Comandos (ver Command enums)
 */

/* GPS / Navegação (0x20-0x2F) */
enum FieldGPS : uint8_t {
    FLD_GPS_LAT     = 0x20,   /* Latitude (int32_t, graus * 10^7) */
    FLD_GPS_LON     = 0x21,   /* Longitude (int32_t, graus * 10^7) */
    FLD_GPS_ALT     = 0x22,   /* Altitude GPS (int32_t, mm) */
    FLD_GPS_SPEED   = 0x23,   /* Velocidade GPS (uint16_t, cm/s) */
    FLD_GPS_SATS    = 0x24,   /* Número de satélites visíveis (uint8_t) */
    FLD_GPS_LINK    = 0x25,   /* Qualidade do link GPS (uint8_t, 0-100%) */
    FLD_GPS_HDOP    = 0x26    /* HDOP (uint16_t, precisão horizontal ×100) — NOVO v2.0.0 */
};

/* IMU / Atitude (0x30-0x3F) */
enum FieldIMU : uint8_t {
    FLD_ROLL        = 0x30,   /* Ângulo de rolamento (float, radianos) */
    FLD_PITCH       = 0x31,   /* Ângulo de inclinação (float, radianos) */
    FLD_YAW         = 0x32,   /* Ângulo de guinada (float, radianos) */
    FLD_VX          = 0x33,   /* Velocidade linear X (float, m/s) */
    FLD_VY          = 0x34,   /* Velocidade linear Y (float, m/s) */
    FLD_VZ          = 0x35,   /* Velocidade linear Z (float, m/s) */
    FLD_HEADING     = 0x36,   /* Rumo magnético (float, graus) */
    FLD_ROLL_RATE   = 0x37,   /* Velocidade angular roll (float, °/s) — NOVO v2.0.0 */
    FLD_PITCH_RATE  = 0x38,   /* Velocidade angular pitch (float, °/s) — NOVO v2.0.0 */
    FLD_YAW_RATE    = 0x39    /* Velocidade angular yaw (float, °/s) — NOVO v2.0.0 */
};

/* Estado de voo (0x40-0x4F) */
enum FieldFlight : uint8_t {
    FLD_ALT_GPS     = 0x40,   /* Altitude GPS (float, metros) */
    FLD_ALT_BARO    = 0x41,   /* Altitude barométrica (float, metros) */
    FLD_VEL_GPS     = 0x42,   /* Velocidade GPS (float, m/s) */
    FLD_VEL_CALC    = 0x43,   /* Velocidade calculada por fusão (float, m/s) */
    FLD_LOOP_TIME   = 0x44    /* Tempo do último loop (uint16_t, µs) — NOVO v2.0.0 */
};

/* Energia / Bateria (0x50-0x5F) */
enum FieldPower : uint8_t {
    FLD_BATT_V      = 0x50,   /* Tensão da bateria (float, volts) */
    FLD_BATT_A      = 0x51,   /* Corrente (float, amperes) */
    FLD_BATT_W      = 0x52,   /* Potência (float, watts) */
    FLD_BATT_CHG    = 0x53,   /* Carga consumida (float, Ah) */
    FLD_BATT_SOC    = 0x54    /* Estado de carga (float, 0-1) — NOVO v2.0.0 */
};

/* Temperatura (0x60-0x6F) */
enum FieldTemp : uint8_t {
    FLD_TEMP1       = 0x60,   /* Temperatura ESC1 (float, °C) */
    FLD_TEMP2       = 0x61,   /* Temperatura ESC2 (float, °C) */
    FLD_TEMP3       = 0x62,   /* Temperatura ESC3 (float, °C) */
    FLD_TEMP4       = 0x63,   /* Temperatura ESC4 (float, °C) */
    FLD_ESP1_TEMP   = 0x64,   /* Temperatura interna ESP1 (float, °C) */
    FLD_ESP2_TEMP   = 0x65    /* Temperatura interna ESP2 (float, °C) */
};

/* Sistema (0x70-0x7F) */
enum FieldSystem : uint8_t {
    FLD_STATE       = 0x70,   /* Estado do sistema (SystemState) */
    FLD_MODE        = 0x71,   /* Modo de voo ativo (FlightMode) */
    FLD_ERRORS      = 0x72,   /* Mapa de bits de erros (uint32_t) */
    FLD_RX_LINK     = 0x73,   /* Qualidade do link de receção (uint8_t, %) */
    FLD_TX_LINK     = 0x74,   /* Qualidade do link de transmissão (uint8_t, %) */
    FLD_ESP1_LOAD   = 0x75,   /* Carga da CPU ESP1 (uint8_t, %) */
    FLD_ESP2_LOAD   = 0x76    /* Carga da CPU ESP2 (uint8_t, %) */
};

/* Failsafe (0xA0-0xAF) */
enum FieldFailsafe : uint8_t {
    FLD_FS_REASON   = 0xA1,   /* Razão do failsafe (FailsafeReason) */
    FLD_FS_ACTION   = 0xA2,   /* Ação em curso (FailsafeAction) */
    FLD_FS_STATE    = 0xA3    /* Estado do failsafe (1=ativo, 0=inativo) */
};

/* Vídeo (0xB0-0xBF) */
enum FieldVideo : uint8_t {
    FLD_VIDEO_FRAME_ID   = 0xB0,   /* Número do frame (uint16_t) */
    FLD_VIDEO_CHUNK_ID   = 0xB1,   /* Índice do chunk (uint8_t) */
    FLD_VIDEO_TOTAL      = 0xB2,   /* Total de chunks neste frame (uint8_t) */
    FLD_VIDEO_PAYLOAD    = 0xB3    /* Payload de vídeo (dados crus) */
};

/* =================================================================================
 * IDs DE COMANDO (MSG_COMMAND)
 * =================================================================================
 * 
 * Estes valores são enviados no payload de um TLV com ID apropriado dentro
 * de uma mensagem MSG_COMMAND. O primeiro byte do payload indica o tipo de
 * comando, seguido pelos parâmetros específicos.
 */

/* Comandos básicos (0xC0-0xCF) */
enum CommandBasic : uint8_t {
    CMD_ARM         = 0xC0,   /* Armar motores (preparar para voo) */
    CMD_DISARM      = 0xC1,   /* Desarmar motores (cortar) */
    CMD_SET_MODE    = 0xC2,   /* Mudar modo de voo (parâmetro: FlightMode) */
    CMD_REBOOT      = 0xC3,   /* Reiniciar o ESP1 */
    CMD_SHUTDOWN    = 0xC4    /* Desligamento controlado */
};

/* Comandos de controlo (0xD0-0xDF) */
enum CommandControl : uint8_t {
    CMD_SET_ALT_TARGET = 0xD0,   /* Definir altitude alvo (float, metros) */
    CMD_SET_THROTTLE   = 0xD1,   /* Definir throttle (float, 0-1) */
    CMD_SET_ROLL       = 0xD2,   /* Definir ângulo de roll (float, graus) */
    CMD_SET_PITCH      = 0xD3,   /* Definir ângulo de pitch (float, graus) */
    CMD_SET_YAW        = 0xD4,   /* Definir velocidade de yaw (float, °/s) */
    CMD_SET_HEADING    = 0xD5    /* Definir heading desejado (float, graus) — NOVO v2.0.0 */
};

/* Comandos avançados (0xE0-0xEF) */
enum CommandAdvanced : uint8_t {
    CMD_SENSOR_CALIB   = 0xE0,   /* Calibrar sensores (IMU, barómetro) */
    CMD_ACTUATOR_CALIB = 0xE1,   /* Calibrar atuadores (ESCs) */
    CMD_SET_PARAM      = 0xE2,   /* Definir parâmetro de configuração */
    CMD_GET_ALL        = 0xE3    /* Pedir telemetria completa (resposta MSG_TELEMETRY) — NOVO v2.0.0 */
};

/* Comandos de navegação (0xF0-0xFF) */
enum CommandNav : uint8_t {
    CMD_NEXT_WAYPOINTS = 0xF0,   /* Enviar lista de waypoints (formato TLV) */
    CMD_RTL            = 0xF1,   /* Return To Launch imediato */
    CMD_SET_POSITION   = 0xF2    /* Definir posição alvo (latitude, longitude, altitude) — NOVO v2.0.0 */
};

/* =================================================================================
 * MACROS E FUNÇÕES CONSTEXPR PARA VALIDAÇÃO EM COMPILAÇÃO
 * =================================================================================
 * 
 * Estas funções permitem validação em tempo de compilação, reduzindo erros
 * e melhorando a performance (zero overhead em runtime).
 */

/**
 * isValidMsgID - Verifica se um ID de mensagem é válido
 * 
 * @param id ID da mensagem a validar
 * @return true se estiver no intervalo 0x10-0x18
 */
static inline constexpr bool isValidMsgID(uint8_t id) {
    return (id >= 0x10 && id <= 0x18);
}

/**
 * isValidFieldID - Verifica se um ID de campo TLV é válido
 * 
 * @param id ID do campo a validar
 * @return true se estiver nos intervalos reservados para dados
 */
static inline constexpr bool isValidFieldID(uint8_t id) {
    return ((id >= 0x20 && id <= 0x7F) || (id >= 0xA1 && id <= 0xFF));
}

/* =================================================================================
 * FUNÇÕES DE CONVERSÃO DE TIPOS (LITTLE-ENDIAN)
 * =================================================================================
 */

/**
 * floatToBytes - Converte float para bytes (little-endian)
 * 
 * @param value Float de entrada
 * @param bytes Array de 4 bytes de saída
 */
static inline void floatToBytes(float value, uint8_t* bytes) {
    memcpy(bytes, &value, 4);
}

/**
 * bytesToFloat - Converte bytes para float (little-endian)
 * 
 * @param bytes Array de 4 bytes de entrada
 * @return      Float reconstruído
 */
static inline float bytesToFloat(const uint8_t* bytes) {
    float value;
    memcpy(&value, bytes, 4);
    return value;
}

/**
 * int32ToBytes - Converte int32_t para bytes (little-endian)
 * 
 * @param val   Inteiro de 32 bits com sinal
 * @param bytes Array de 4 bytes de saída
 */
static inline void int32ToBytes(int32_t val, uint8_t* bytes) {
    memcpy(bytes, &val, 4);
}

/**
 * bytesToInt32 - Converte bytes para int32_t (little-endian)
 * 
 * @param bytes Array de 4 bytes de entrada
 * @return      int32_t reconstruído
 */
static inline int32_t bytesToInt32(const uint8_t* bytes) {
    int32_t val;
    memcpy(&val, bytes, 4);
    return val;
}

/**
 * uint32ToBytes - Converte uint32_t para bytes (little-endian)
 * 
 * @param val   Inteiro de 32 bits sem sinal
 * @param bytes Array de 4 bytes de saída
 */
static inline void uint32ToBytes(uint32_t val, uint8_t* bytes) {
    memcpy(bytes, &val, 4);
}

/**
 * bytesToUint32 - Converte bytes para uint32_t (little-endian)
 * 
 * @param bytes Array de 4 bytes de entrada
 * @return      uint32_t reconstruído
 */
static inline uint32_t bytesToUint32(const uint8_t* bytes) {
    uint32_t val;
    memcpy(&val, bytes, 4);
    return val;
}

/**
 * uint16ToBytes - Converte uint16_t para bytes (little-endian)
 * 
 * @param val   Inteiro de 16 bits sem sinal
 * @param bytes Array de 2 bytes de saída
 */
static inline void uint16ToBytes(uint16_t val, uint8_t* bytes) {
    bytes[0] = val & 0xFF;
    bytes[1] = (val >> 8) & 0xFF;
}

/**
 * bytesToUint16 - Converte bytes para uint16_t (little-endian)
 * 
 * @param bytes Array de 2 bytes de entrada
 * @return      uint16_t reconstruído
 */
static inline uint16_t bytesToUint16(const uint8_t* bytes) {
    return (uint16_t)bytes[0] | ((uint16_t)bytes[1] << 8);
}

/* =================================================================================
 * FUNÇÕES AUXILIARES (IMPLEMENTADAS EM Protocol.cpp)
 * =================================================================================
 */

/**
 * calcCRC8 - Calcula CRC8 usando polinómio SMBUS (0x07)
 * 
 * Utilizado para deteção de erros no canal de comunicação.
 * O CRC é calculado sobre TODO o pacote (incluindo START_BYTE e campos TLV),
 * mas NÃO sobre o próprio byte de CRC.
 * 
 * @param data  Ponteiro para os dados
 * @param len   Número de bytes
 * @return      CRC8 calculado (0x00-0xFF)
 */
uint8_t calcCRC8(const uint8_t* data, size_t len);

/**
 * buildTLV - Serializa um campo TLV individual
 * 
 * @param id     Identificador do campo
 * @param data   Ponteiro para os dados
 * @param len    Comprimento dos dados (máx MAX_TLV_DATA)
 * @param output Buffer de saída (mínimo len+2 bytes)
 * @return       Número de bytes escritos (0 em caso de erro)
 */
size_t buildTLV(uint8_t id, const uint8_t* data, uint8_t len, uint8_t* output);

/**
 * buildTLVVideo - Serializa um campo TLV de vídeo
 * 
 * @param data   Ponteiro para os dados de vídeo
 * @param len    Comprimento dos dados (máx MAX_TLV_VIDEO_DATA)
 * @param output Buffer de saída (mínimo len+2 bytes)
 * @return       Número de bytes escritos (0 em caso de erro)
 */
size_t buildTLVVideo(const uint8_t* data, uint8_t len, uint8_t* output);

/**
 * buildMessage - Serializa uma mensagem TLV completa
 * 
 * @param msg        Ponteiro para a mensagem estruturada
 * @param msgID      ID da mensagem (sobrescreve msg.msgID se !=0)
 * @param buffer     Buffer de saída
 * @param bufferSize Tamanho do buffer
 * @return           Número de bytes escritos (0 em caso de erro)
 */
size_t buildMessage(TLVMessage* msg, uint8_t msgID, uint8_t* buffer, size_t bufferSize);

/**
 * validateMessage - Valida estruturalmente uma mensagem recebida
 * 
 * Verifica START_BYTE, integridade estrutural dos TLVs e CRC8.
 * NÃO verifica HMAC/SEQ (responsabilidade do módulo Security).
 * 
 * @param buffer Buffer com a mensagem recebida
 * @param length Comprimento do buffer
 * @return       1 se válida, 0 caso contrário
 */
uint8_t validateMessage(const uint8_t* buffer, size_t length);

/**
 * parseTLV - Extrai campos TLV de um buffer bruto
 * 
 * @param data   Buffer com dados TLV (sem START_BYTE/CRC)
 * @param length Comprimento do buffer
 * @param output Array de saída para os campos TLV
 * @param count  Número de campos extraídos (output)
 */
void parseTLV(const uint8_t* data, size_t length, TLVField* output, size_t* count);

/**
 * printTLVField - Imprime um campo TLV (debug)
 * 
 * @param field Campo TLV a imprimir
 */
void printTLVField(const TLVField* field);

/**
 * printMessage - Imprime uma mensagem TLV completa (debug)
 * 
 * @param msg Mensagem a imprimir
 */
void printMessage(const TLVMessage* msg);

/* =================================================================================
 * CLASSE TLVBUILDER (C++ apenas)
 * =================================================================================
 * 
 * Facilitador para construção de mensagens TLV com interface fluente.
 * Exemplo de uso:
 * 
 *   TLVBuilder builder;
 *   builder.addFloat(FLD_ROLL, 0.12f);
 *   builder.addFloat(FLD_PITCH, -0.05f);
 *   builder.addUint8(FLD_STATE, SYS_STATE_FLYING);
 *   size_t len = builder.build(MSG_TELEMETRY, buffer, sizeof(buffer));
 */

#ifdef __cplusplus
}

class TLVBuilder {
private:
    TLVMessage msg;

public:
    /**
     * Construtor - Inicializa uma mensagem vazia
     */
    TLVBuilder() {
        msg.startByte = START_BYTE;
        msg.msgID = 0;
        msg.tlvCount = 0;
        msg.checksum = 0;
        memset(msg.tlvs, 0, sizeof(msg.tlvs));
    }

    /**
     * Adiciona um campo TLV do tipo float
     */
    void addFloat(uint8_t id, float value) {
        if (msg.tlvCount >= MAX_TLV_FIELDS) return;
        uint8_t bytes[4];
        floatToBytes(value, bytes);
        addTLV(&msg, id, bytes, 4);
    }

    /**
     * Adiciona um campo TLV do tipo int32_t
     */
    void addInt32(uint8_t id, int32_t value) {
        if (msg.tlvCount >= MAX_TLV_FIELDS) return;
        uint8_t bytes[4];
        int32ToBytes(value, bytes);
        addTLV(&msg, id, bytes, 4);
    }

    /**
     * Adiciona um campo TLV do tipo uint32_t
     */
    void addUint32(uint8_t id, uint32_t value) {
        if (msg.tlvCount >= MAX_TLV_FIELDS) return;
        uint8_t bytes[4];
        uint32ToBytes(value, bytes);
        addTLV(&msg, id, bytes, 4);
    }

    /**
     * Adiciona um campo TLV do tipo uint16_t
     */
    void addUint16(uint8_t id, uint16_t value) {
        if (msg.tlvCount >= MAX_TLV_FIELDS) return;
        uint8_t bytes[2];
        uint16ToBytes(value, bytes);
        addTLV(&msg, id, bytes, 2);
    }

    /**
     * Adiciona um campo TLV do tipo uint8_t
     */
    void addUint8(uint8_t id, uint8_t value) {
        if (msg.tlvCount >= MAX_TLV_FIELDS) return;
        addTLV(&msg, id, &value, 1);
    }

    /**
     * Adiciona um campo TLV com dados arbitrários
     */
    void addRaw(uint8_t id, const void* data, uint8_t len) {
        if (msg.tlvCount >= MAX_TLV_FIELDS || len > MAX_TLV_DATA) return;
        addTLV(&msg, id, data, len);
    }

    /**
     * Constrói e serializa a mensagem
     * 
     * @param msgID      ID da mensagem
     * @param outBuffer  Buffer de saída
     * @param bufferSize Tamanho do buffer
     * @return           Número de bytes escritos (0 = erro)
     */
    size_t build(uint8_t msgID, uint8_t* outBuffer, size_t bufferSize) {
        msg.msgID = msgID;
        return buildMessage(&msg, msgID, outBuffer, bufferSize);
    }

    /**
     * Reseta o builder para uma nova mensagem
     */
    void reset() {
        msg.startByte = START_BYTE;
        msg.msgID = 0;
        msg.tlvCount = 0;
        msg.checksum = 0;
        memset(msg.tlvs, 0, sizeof(msg.tlvs));
    }

    /**
     * Retorna o número atual de TLVs
     */
    uint8_t getTLVCount() const { return msg.tlvCount; }
};

/* Funções C++ inline para compatibilidade com código existente */
inline void addTLV(TLVMessage* msg, uint8_t id, const void* data, uint8_t len) {
    if (msg->tlvCount >= MAX_TLV_FIELDS || len > MAX_TLV_DATA) return;
    TLVField* f = &msg->tlvs[msg->tlvCount];
    f->id = id;
    f->len = len;
    memcpy(f->data, data, len);
    msg->tlvCount++;
}

inline void addTLVFloat(TLVMessage* msg, uint8_t id, float value) {
    uint8_t bytes[4];
    floatToBytes(value, bytes);
    addTLV(msg, id, bytes, 4);
}

inline void addTLVInt32(TLVMessage* msg, uint8_t id, int32_t value) {
    uint8_t bytes[4];
    int32ToBytes(value, bytes);
    addTLV(msg, id, bytes, 4);
}

inline void addTLVUint32(TLVMessage* msg, uint8_t id, uint32_t value) {
    uint8_t bytes[4];
    uint32ToBytes(value, bytes);
    addTLV(msg, id, bytes, 4);
}

inline void addTLVUint16(TLVMessage* msg, uint8_t id, uint16_t value) {
    uint8_t bytes[2];
    uint16ToBytes(value, bytes);
    addTLV(msg, id, bytes, 2);
}

inline void addTLVUint8(TLVMessage* msg, uint8_t id, uint8_t value) {
    addTLV(msg, id, &value, 1);
}

#endif /* PROTOCOL_H */