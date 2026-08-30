# Guia Rápido – Bythos Protocol V2

> **Versão:** 2.0.0  
> **Uso:** Referência rápida para debug, manutenção e desenvolvimento

---

## 1. Constantes Fundamentais

| Constante             | Valor | Descrição                     |
| ----------------------|-------|-------------------------------|
| `START_BYTE`          | 0xAA  | Sincronização de quadro       |
| `MAX_TLV_FIELDS`      | 32    | Máx. campos por mensagem      |
| `MAX_TLV_DATA`        | 32    | Máx. payload normal (bytes)   |
| `MAX_TLV_VIDEO_DATA`  | 128   | Máx. payload vídeo (bytes)    |
| `MAX_MESSAGE_SIZE`    | 1024  | Tamanho máximo da mensagem    |
| `MIN_MESSAGE_SIZE`    | 4     | Tamanho mínimo                |

---

## 2. Mensagens (`MsgID`)

| ID   | Constante       | Descrição                    |
| -----|-----------------|------------------------------|
| 0x10 | `MSG_HEARTBEAT` | Coração do sistema           |
| 0x11 | `MSG_TELEMETRY` | Telemetria normal            |
| 0x12 | `MSG_COMMAND`   | Comandos GS → ESP            |
| 0x13 | `MSG_ACK`       | Confirmação de receção       |
| 0x14 | `MSG_FAILSAFE`  | Notificação de failsafe      |
| 0x15 | `MSG_DEBUG`     | Debug (prioridade dinâmica)  |
| 0x16 | `MSG_VIDEO`     | Pacote de vídeo              |
| 0x17 | `MSG_SHELL_CMD` | Comando shell remoto         |
| 0x18 | `MSG_SI_DATA`   | Dados SI (ESP2 → ESP1)       |

---

## 3. Prioridades (`PriorityLevel`)

| Prioridade        | Valor | Uso                   |
| ------------------|-------|-----------------------|
| `SUPER_CRITICAL`  | 0     | Failsafe              |
| `CRITICAL`        | 1     | Comandos GS           |
| `HIGH`            | 2     | Telemetria crítica    |
| `MEDIUM`          | 3     | Telemetria normal     |
| `LOW`             | 4     | Shell / debug normal  |
| `SUPER_LOW`       | 5     | Debug avançado        |

**Regra especial:** `MSG_DEBUG` → `SUPER_CRITICAL` se failsafe ativo.

---

## 4. Estados do Sistema (`SystemState`)

| Valor | Constante             | Descrição     |
| ------|-----------------------|---------------|
| 0     | `SYS_STATE_BOOTING`   | Arranque      |
| 1     | `SYS_STATE_INIT`      | Inicialização |
| 2     | `SYS_STATE_IDLE`      | Pronto        |
| 3     | `SYS_STATE_ARMED`     | Armado        |
| 4     | `SYS_STATE_FLYING`    | Em voo        |
| 5     | `SYS_STATE_FAILSAFE`  | Modo segurança|
| 6     | `SYS_STATE_ERROR`     | Erro crítico  |
| 7     | `SYS_STATE_SHUTDOWN`  | Desligamento  |

---

## 5. Modos de Voo (`FlightMode`)

| Valor | Constante         | Descrição              |
| ------|-------------------|------------------------|
| 0     | `MODE_MANUAL`     | Controlo direto        |
| 1     | `MODE_STABILIZE`  | Auto-nivelamento       |
| 2     | `MODE_ALT_HOLD`   | Manutenção de altitude |
| 3     | `MODE_POSHOLD`    | Manutenção de posição  |
| 4     | `MODE_AUTO`       | Navegação autónoma     |
| 5     | `MODE_RTL`        | Return To Launch       |

---

## 6. Failsafe

### Razões (`FailsafeReason`)

| Valor | Constante                  | Descrição            |
| ------|----------------------------|----------------------|
| 0     | `FS_REASON_NONE`           | Sem failsafe         |
| 1     | `FS_REASON_LOST_LINK`      | Perda de comunicação |
| 2     | `FS_REASON_LOW_BATTERY`    | Bateria baixa        |
| 3     | `FS_REASON_GPS_LOST`       | Perda de GPS         |
| 4     | `FS_REASON_IMU_ERROR`      | Erro IMU             |
| 5     | `FS_REASON_MANUAL_TRIGGER` | Ativação manual      |

### Ações (`FailsafeAction`)

| Valor | Constante                 | Descrição         |
| ------|---------------------------|-------------------|
| 0     | `FS_ACTION_NONE`          | Nenhuma           |
| 1     | `FS_ACTION_HOVER`         | Pairar            |
| 2     | `FS_ACTION_RTL`           | Regressar à origem|
| 3     | `FS_ACTION_LAND`          | Aterrar           |
| 4     | `FS_ACTION_CUT_THROTTLE`  | Cortar motores    |
| 5     | `FS_ACTION_DISARM`        | Desarmar          |

---

## 7. IDs de Campos TLV (RÁPIDO)

### GPS (0x20-0x2F)

| ID   | Constante       | Tipo     |
| -----|-----------------|----------|
| 0x20 | `FLD_GPS_LAT`   | int32_t  |
| 0x21 | `FLD_GPS_LON`   | int32_t  |
| 0x22 | `FLD_GPS_ALT`   | int32_t  |
| 0x23 | `FLD_GPS_SPEED` | uint16_t |
| 0x24 | `FLD_GPS_SATS`  | uint8_t  |
| 0x25 | `FLD_GPS_LINK`  | uint8_t  |
| 0x26 | `FLD_GPS_HDOP`  | uint16_t |

### IMU (0x30-0x3F)

| ID   | Constante       | Tipo  |
| -----|-----------------|-------|
| 0x30 | `FLD_ROLL`      | float |
| 0x31 | `FLD_PITCH`     | float |
| 0x32 | `FLD_YAW`       | float |
| 0x33 | `FLD_VX`        | float |
| 0x34 | `FLD_VY`        | float |
| 0x35 | `FLD_VZ`        | float |
| 0x36 | `FLD_HEADING`   | float |
| 0x37 | `FLD_ROLL_RATE` | float |
| 0x38 | `FLD_PITCH_RATE`| float |
| 0x39 | `FLD_YAW_RATE`  | float |

### Estado de Voo (0x40-0x4F)

| ID   | Constante       | Tipo     |
| -----|-----------------|----------|
| 0x40 | `FLD_ALT_GPS`   | float    |
| 0x41 | `FLD_ALT_BARO`  | float    |
| 0x42 | `FLD_VEL_GPS`   | float    |
| 0x43 | `FLD_VEL_CALC`  | float    |
| 0x44 | `FLD_LOOP_TIME` | uint16_t |

### Energia (0x50-0x5F)

| ID   | Constante      | Tipo  |
| -----|----------------|-------|
| 0x50 | `FLD_BATT_V`   | float |
| 0x51 | `FLD_BATT_A`   | float |
| 0x52 | `FLD_BATT_W`   | float |
| 0x53 | `FLD_BATT_CHG` | float |
| 0x54 | `FLD_BATT_SOC` | float |

### Temperatura (0x60-0x6F)

| ID   | Constante       | Tipo  |
| -----|-----------------|-------|
| 0x60 | `FLD_TEMP1`     | float |
| 0x61 | `FLD_TEMP2`     | float |
| 0x62 | `FLD_TEMP3`     | float |
| 0x63 | `FLD_TEMP4`     | float |
| 0x64 | `FLD_ESP1_TEMP` | float |
| 0x65 | `FLD_ESP2_TEMP` | float |

### Sistema (0x70-0x7F)

| ID   | Constante       | Tipo    |
| -----|-----------------|---------|
| 0x70 | `FLD_STATE`     | uint8_t |
| 0x71 | `FLD_MODE`      | uint8_t |
| 0x72 | `FLD_ERRORS`    | uint32_t|
| 0x73 | `FLD_RX_LINK`   | uint8_t |
| 0x74 | `FLD_TX_LINK`   | uint8_t |
| 0x75 | `FLD_ESP1_LOAD` | uint8_t |
| 0x76 | `FLD_ESP2_LOAD` | uint8_t |

### Failsafe (0xA1-0xAF)

| ID   | Constante       | Tipo    |
| -----|-----------------|---------|
| 0xA1 | `FLD_FS_REASON` | uint8_t |
| 0xA2 | `FLD_FS_ACTION` | uint8_t |
| 0xA3 | `FLD_FS_STATE`  | uint8_t |

### Vídeo (0xB0-0xBF)

| ID   | Constante            | Tipo      |
| -----|----------------------|-----------|
| 0xB0 | `FLD_VIDEO_FRAME_ID` | uint16_t  |
| 0xB1 | `FLD_VIDEO_CHUNK_ID` | uint8_t   |
| 0xB2 | `FLD_VIDEO_TOTAL`    | uint8_t   |
| 0xB3 | `FLD_VIDEO_PAYLOAD`  | uint8_t[] |

---

## 8. Comandos (0xC0–0xFF)

### Básicos (0xC0–0xCF)

| ID   | Constante      | Parâmetros |
| -----|----------------|------------|
| 0xC0 | `CMD_ARM`      | Nenhum     |
| 0xC1 | `CMD_DISARM`   | Nenhum     |
| 0xC2 | `CMD_SET_MODE` | uint8_t    |
| 0xC3 | `CMD_REBOOT`   | Nenhum     |
| 0xC4 | `CMD_SHUTDOWN` | Nenhum     |

### Controlo (0xD0–0xDF)

| ID   | Constante            | Parâmetros  |
| -----|----------------------|-------------|
| 0xD0 | `CMD_SET_ALT_TARGET` | float       |
| 0xD1 | `CMD_SET_THROTTLE`   | float       |
| 0xD2 | `CMD_SET_ROLL`       | float       |
| 0xD3 | `CMD_SET_PITCH`      | float       |
| 0xD4 | `CMD_SET_YAW`        | float       |
| 0xD5 | `CMD_SET_HEADING`    | float       |

### Avançados (0xE0–0xEF)

| ID   | Constante            | Parâmetros  |
| -----|----------------------|-------------|
| 0xE0 | `CMD_SENSOR_CALIB`   | Nenhum      |
| 0xE1 | `CMD_ACTUATOR_CALIB` | Nenhum      |
| 0xE2 | `CMD_SET_PARAM`      | ID + valor  |
| 0xE3 | `CMD_GET_ALL`        | Nenhum      |

### Navegação (0xF0–0xFF)

| ID   | Constante            | Parâmetros    |
| -----|----------------------|---------------|
| 0xF0 | `CMD_NEXT_WAYPOINTS` | Lista TLV     |
| 0xF1 | `CMD_RTL`            | Nenhum        |
| 0xF2 | `CMD_SET_POSITION`   | lat, lon, alt |

---

## 9. Funções de Conversão

| Função                                | Descrição        |
| --------------------------------------|------------------|
| `floatToBytes()` / `bytesToFloat()`   | float ↔ bytes    |
| `int32ToBytes()` / `bytesToInt32()`   | int32_t ↔ bytes  |
| `uint32ToBytes()` / `bytesToUint32()` | uint32_t ↔ bytes |
| `uint16ToBytes()` / `bytesToUint16()` | uint16_t ↔ bytes |

---

## 10. Parser – Estados da FSM

| Estado                 | Valor | Descrição            |
| -----------------------|-------|----------------------|
| `PARSER_WAIT_START`    | 0     | Aguarda 0xAA         |
| `PARSER_WAIT_MSGID`    | 1     | Aguarda msgID        |
| `PARSER_WAIT_TLVCOUNT` | 2     | Aguarda nº TLVs      |
| `PARSER_WAIT_TLV_ID`   | 3     | Aguarda ID do TLV    |
| `PARSER_WAIT_TLV_LEN`  | 4     | Aguarda LEN          |
| `PARSER_WAIT_TLV_DATA` | 5     | Aguarda dados        |
| `PARSER_WAIT_CHECKSUM` | 6     | Aguarda CRC8         |

---

## 11. Códigos de Erro do Parser

| Erro                       | Valor | Descrição            |
| ---------------------------|-------|----------------------|
| `PARSER_OK`                | 0     | Sem erro             |
| `PARSER_ERR_OVERFLOW`      | 1     | Buffer excedido      |
| `PARSER_ERR_TIMEOUT`       | 2     | Timeout entre bytes  |
| `PARSER_ERR_INVALID_START` | 3     | START_BYTE inválido  |
| `PARSER_ERR_CHECKSUM`      | 4     | CRC8 inválido        |
| `PARSER_ERR_TLV_COUNT`     | 5     | tlvCount > 32        |
| `PARSER_ERR_TLV_LEN`       | 6     | LEN > 128            |

---

## 12. Exemplos Rápidos

### Construir mensagem

```cpp
TLVBuilder builder;
builder.addFloat(FLD_ROLL, 0.12f);
builder.addInt32(FLD_GPS_LAT, 412345678);
builder.addUint8(FLD_STATE, SYS_STATE_FLYING);
size_t len = builder.build(MSG_TELEMETRY, buffer, sizeof(buffer));
´´´

Parse byte-a-byte

```cpp
Parser parser;
while (Serial.available()) {
    if (parser.feed(Serial.read())) {
        TLVMessage* msg = parser.getMessage();
        // processar msg...
        parser.acknowledge();
    }
}
´´´

Validar mensagem manualmente

```cpp
if (validateMessage(buffer, length)) {
    // mensagem válida
}
´´´

Calcular CRC8

```cpp
uint8_t crc = calcCRC8(data, dataLen);
´´´

13. Debug – Ativação

```cpp
#define PROTOCOL_DEBUG   // Ativa debug do Protocol
#define PARSER_DEBUG     // Ativa debug do Parser
Mensagens com prefixos [Protocol] e [Parser].
´´´

14. Novidades da v2.0.0

Adição                                              Descrição
FLD_GPS_HDOP                                        Precisão horizontal do GPS
FLD_ROLL_RATE, FLD_PITCH_RATE, FLD_YAW_RATE         Velocidades angulares
FLD_LOOP_TIME                                       Tempo do loop de controlo
FLD_BATT_SOC                                        Estado de carga da bateria
CMD_SET_HEADING                                     Definir heading desejado
CMD_GET_ALL                                         Pedir telemetria completa
CMD_SET_POSITION                                    Definir posição alvo
Funções int32ToBytes, uint32ToBytes, uint16ToBytes  Conversão de tipos

Fim do Guia Rápido – Bythos Protocol V2
