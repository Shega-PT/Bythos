# Guia Rápido — Bythos Protocol V1

> **Versão:** 1.0.0  
> **Uso:** Referência rápida para implementação e debug

---

## 1. Constantes

| Constante             | Valor | Descrição                     |
|-----------------------|-------|-------------------------------|
| `START_BYTE`          | 0xAA  | Sincronização de trama        |
| `MAX_TLV_FIELDS`      | 8     | Máx. campos TLV por mensagem  |
| `MAX_TLV_DATA`        | 16    | Máx. payload normal (bytes)   |
| `MAX_MESSAGE_SIZE`    | 256   | Tamanho máximo da mensagem    |
| `MIN_MESSAGE_SIZE`    | 4     | Tamanho mínimo                |

---

## 2. Mensagens (`MsgID`)

| ID   | Constante       | Descrição                    |
|------|-----------------|------------------------------|
| 0x10 | `MSG_HEARTBEAT` | Heartbeat — nó está vivo     |
| 0x11 | `MSG_TELEMETRY` | Dados de telemetria          |
| 0x12 | `MSG_COMMAND`   | Comando entre nós            |
| 0x13 | `MSG_ACK`       | Confirmação de receção       |
| 0x14 | `MSG_FAILSAFE`  | Modo de segurança            |
| 0x15 | `MSG_DEBUG`     | Debug/diagnóstico            |

---

## 3. FieldIDs — Dados (0x20-0x2F)

| ID   | Constante    | Tipo    |
|------|--------------|---------|
| 0x20 | `FLD_DATA_0` | uint8   |
| 0x21 | `FLD_DATA_1` | uint8   |
| 0x22 | `FLD_DATA_2` | uint16  |
| 0x23 | `FLD_DATA_3` | uint16  |
| 0x24 | `FLD_DATA_4` | int32   |
| 0x25 | `FLD_DATA_5` | int32   |
| 0x26 | `FLD_DATA_6` | float   |
| 0x27 | `FLD_DATA_7` | float   |

## 4. FieldIDs — Estado (0x30-0x37)

| ID   | Constante      | Tipo    |
|------|----------------|---------|
| 0x30 | `FLD_STATUS_0` | uint8   |
| 0x31 | `FLD_STATUS_1` | uint8   |
| 0x32 | `FLD_STATUS_2` | uint16  |

## 5. FieldIDs — Comando (0x40-0x44)

| ID   | Constante    | Tipo    |
|------|--------------|---------|
| 0x40 | `FLD_CMD_0`  | uint8   |
| 0x41 | `FLD_CMD_1`  | uint8   |
| 0x42 | `FLD_CMD_2`  | float   |

---

## 6. Comandos (0xC0-0xC4)

| ID   | Constante      | Parâmetros | Descrição             |
|------|----------------|------------|------------------------|
| 0xC0 | `CMD_ARM`      | Nenhum     | Armar — modo ativo     |
| 0xC1 | `CMD_DISARM`   | Nenhum     | Desarmar — modo seguro |
| 0xC2 | `CMD_SET_MODE` | uint8_t    | Definir modo           |
| 0xC3 | `CMD_REBOOT`   | Nenhum     | Reiniciar              |
| 0xC4 | `CMD_SHUTDOWN` | Nenhum     | Desligar               |

---

## 7. Funções de Conversão

| Função                                | Descrição        |
|---------------------------------------|------------------|
| `floatToBytes()` / `bytesToFloat()`   | float ↔ bytes    |
| `int32ToBytes()` / `bytesToInt32()`   | int32_t ↔ bytes  |
| `uint32ToBytes()` / `bytesToUint32()` | uint32_t ↔ bytes |
| `uint16ToBytes()` / `bytesToUint16()` | uint16_t ↔ bytes |

---

## 8. Parser — Estados da FSM

| Estado                 | Valor | Descrição            |
|------------------------|-------|----------------------|
| `PARSER_WAIT_START`    | 0     | Aguarda 0xAA         |
| `PARSER_WAIT_MSGID`    | 1     | Aguarda msgID        |
| `PARSER_WAIT_TLVCOUNT` | 2     | Aguarda nº TLVs      |
| `PARSER_WAIT_TLV_ID`   | 3     | Aguarda ID do TLV    |
| `PARSER_WAIT_TLV_LEN`  | 4     | Aguarda LEN          |
| `PARSER_WAIT_TLV_DATA` | 5     | Aguarda dados        |
| `PARSER_WAIT_CHECKSUM` | 6     | Aguarda CRC8         |

---

## 9. Códigos de Erro

| Erro                       | Valor | Descrição            |
|----------------------------|-------|----------------------|
| `PARSER_OK`                | 0     | Sem erro             |
| `PARSER_ERR_OVERFLOW`      | 1     | Buffer excedido      |
| `PARSER_ERR_TIMEOUT`       | 2     | Timeout entre bytes  |
| `PARSER_ERR_INVALID_START` | 3     | START_BYTE inválido  |
| `PARSER_ERR_CHECKSUM`      | 4     | CRC8 inválido        |
| `PARSER_ERR_TLV_COUNT`     | 5     | tlvCount > 8         |
| `PARSER_ERR_TLV_LEN`       | 6     | LEN > 16             |

---

## 10. Exemplos Rápidos

### Construir mensagem

```cpp
TLVMessage msg;
msg.startByte = START_BYTE;
msg.msgID = MSG_TELEMETRY;
msg.tlvCount = 0;

addTLVUint8(&msg, FLD_STATUS_0, 1);
addTLVFloat(&msg, FLD_DATA_6, 3.14f);

uint8_t buffer[MAX_MESSAGE_SIZE];
size_t len = buildMessage(&msg, MSG_TELEMETRY, buffer, sizeof(buffer));
```

### Parse byte-a-byte

```cpp
Parser parser;
while (Serial.available()) {
    if (parser.feed(Serial.read())) {
        TLVMessage* msg = parser.getMessage();
        // processar msg...
        parser.acknowledge();
    }
}
```

### Validar manualmente

```cpp
if (validateMessage(buffer, length)) {
    // mensagem válida
}
```

### Calcular CRC8

```cpp
uint8_t crc = calcCRC8(data, dataLen);
```

---

## 11. Debug

```cpp
#define PROTOCOL_DEBUG   // Debug do Protocol
#define PARSER_DEBUG     // Debug do Parser
```

Output com prefixos `[Protocol]` e `[Parser]`.

---

Fim do Guia Rápido — Bythos Protocol V1
