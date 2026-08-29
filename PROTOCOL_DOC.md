# Bythos Protocol V1 — Documentação Completa

> **Versão:** 1.0.0  
> **Data:** 2026-04-19  
> **Autor:** ShegaPT

Esta documentação detalha a implementação do protocolo de comunicação TLV genérico versão 1.0.0, concebido para operar exclusivamente via UART com buffer reduzido e FieldIDs genéricos.

---

## 1. Objetivo do Protocolo

O protocolo v1.0.0 foi criado para permitir:

- Comunicação estruturada entre dispositivos via **UART**
- Transmissão de **mensagens TLV** (Type-Length-Value) contendo dados genéricos
- Integridade verificada por **CRC8/SMBUS**
- Versão mínima funcional e genérica — reutilizável em qualquer contexto
- Sem autenticação HMAC/SEQ — integridade depende exclusivamente do CRC8

### Características da v1.0.0

| Característica | v1.0.0 |
|----------------|--------|
| Transporte | UART apenas |
| Autenticação | Nenhuma (CRC8 apenas) |
| Buffer máximo | 256 bytes |
| Máximo TLVs/msg | 8 |
| Máximo payload/TLV | 16 bytes |
| Tipos de mensagem | 6 |
| Comandos | 5 |
| FieldIDs | 15 genéricos |

---

## 2. Arquitetura do Protocolo

### 2.1 Formato da Mensagem no Canal

```
┌─────────┬─────────┬───────────┬─────────────────┬─────────┐
│ START   │ MSG ID  │ TLV COUNT │ TLV FIELDS      │ CRC8    │
│ (1 byte)│ (1 byte)│ (1 byte)  │ (variável)      │ (1 byte)│
├─────────┼─────────┼───────────┼─────────────────┼─────────┤
│ 0xAA    │ 0x10-15 │ 0-8       │ ID(1)+LEN(1)+N  │ CRC-8   │
│         │         │           │                 │ SMBUS   │
│         │         │           │                 │ 0x07    │
└─────────┴─────────┴───────────┴─────────────────┴─────────┘
```

### 2.2 Formato TLV Individual

Cada campo TLV é serializado de forma compacta:
`[ID (1 byte)] [LEN (1 byte)] [DATA (LEN bytes)]`

**Exemplo:** Um campo com ID 0x20 e valor float 1.0:
`0x20 0x04 0x00 0x00 0x80 0x3F` → ID=0x20, LEN=4, DATA=1.0f

---

## 3. Arquivos da Pasta

### 3.1 Protocol.h

**Função principal:** Define constantes, estruturas, IDs e enums do protocolo.

| Constante             | Valor | Descrição                                  |
|-----------------------|-------|--------------------------------------------|
| `START_BYTE`          | 0xAA  | Byte de sincronização (10101010)           |
| `PROTOCOL_VERSION_STR`| "1.0.0" | Versão do protocolo                     |
| `MAX_TLV_FIELDS`      | 8     | Máximo de campos TLV por mensagem          |
| `MAX_TLV_DATA`        | 16    | Máximo de payload por campo (bytes)        |
| `MAX_MESSAGE_SIZE`    | 256   | Tamanho máximo da mensagem serializada     |
| `MIN_MESSAGE_SIZE`    | 4     | Tamanho mínimo (START+MSGID+COUNT+CRC)     |

**Estruturas principais:**

- `TLVField` — Campo TLV padrão (ID + LEN + DATA[16])
- `TLVMessage` — Mensagem completa (START + MSGID + COUNT + TLVs + CRC)

**Enums definidos:**

| Enum          | Descrição                          |
|---------------|------------------------------------|
| `MsgID`       | Tipos de mensagem (0x10-0x15)      |
| `FieldData`   | Dados genéricos (0x20-0x27)        |
| `FieldStatus` | Estado/status (0x30-0x32)          |
| `FieldCmd`    | Parâmetros de comando (0x40-0x42)  |
| `CommandBasic`| Comandos básicos (0xC0-0xC4)       |

**Funções de conversão (little-endian):**

| Função                                | Descrição        |
|---------------------------------------|------------------|
| `floatToBytes()` / `bytesToFloat()`   | float ↔ bytes    |
| `int32ToBytes()` / `bytesToInt32()`   | int32_t ↔ bytes  |
| `uint32ToBytes()` / `bytesToUint32()` | uint32_t ↔ bytes |
| `uint16ToBytes()` / `bytesToUint16()` | uint16_t ↔ bytes |

**Funções `constexpr`:**

- `isValidMsgID(id)` — Verifica intervalo 0x10-0x15
- `isValidFieldID(id)` — Verifica intervalos 0x20-0x2F, 0x30-0x37, 0x40-0x44

**Funções inline C++:**

- `addTLV()` — Adiciona campo TLV genérico
- `addTLVFloat()` — Adiciona campo float
- `addTLVInt32()` — Adiciona campo int32
- `addTLVUint32()` — Adiciona campo uint32
- `addTLVUint16()` — Adiciona campo uint16
- `addTLVUint8()` — Adiciona campo uint8

### 3.2 Protocol.cpp

**Função principal:** Implementação das funções de serialização, validação e desserialização.

| Função                                   | Descrição                                      |
|------------------------------------------|------------------------------------------------|
| `calcCRC8(data, len)`                    | Calcula CRC8/SMBUS via tabela de lookup        |
| `buildTLV(id, data, len, output)`        | Serializa um campo TLV individual              |
| `buildMessage(msg, msgID, buffer, size)` | Serializa uma mensagem TLV completa            |
| `validateMessage(buffer, length)`        | Valida estrutura e CRC8 de uma mensagem        |
| `parseTLV(data, length, output, count)`  | Extrai campos TLV de um buffer bruto           |

### 3.3 Parser.h

**Função principal:** Define a classe Parser com FSM de 7 estados para byte stream.

| Estado                 | Descrição                          |
|------------------------|------------------------------------|
| `PARSER_WAIT_START`    | Aguarda START_BYTE (0xAA)          |
| `PARSER_WAIT_MSGID`    | Aguarda e valida msgID             |
| `PARSER_WAIT_TLVCOUNT` | Aguarda número de TLVs             |
| `PARSER_WAIT_TLV_ID`   | Aguarda ID do TLV atual            |
| `PARSER_WAIT_TLV_LEN`  | Aguarda comprimento do payload     |
| `PARSER_WAIT_TLV_DATA` | Acumula bytes do payload           |
| `PARSER_WAIT_CHECKSUM` | Aguarda CRC8 e valida              |

**Códigos de erro:**

| Erro                       | Descrição                          |
|----------------------------|------------------------------------|
| `PARSER_OK`                | Sem erro                           |
| `PARSER_ERR_OVERFLOW`      | Buffer interno excedido            |
| `PARSER_ERR_TIMEOUT`       | Timeout entre bytes                |
| `PARSER_ERR_INVALID_START` | START_BYTE ou msgID inválido       |
| `PARSER_ERR_CHECKSUM`      | CRC8 inválido                      |
| `PARSER_ERR_TLV_COUNT`     | tlvCount > MAX_TLV_FIELDS          |
| `PARSER_ERR_TLV_LEN`       | LEN > MAX_TLV_DATA                 |

**Métodos principais:**

| Método                   | Descrição                                     |
|--------------------------|-----------------------------------------------|
| `feed(byte)`             | Alimenta um byte ao parser (retorna 1 quando completo) |
| `hasMessage()`           | Verifica se há mensagem disponível            |
| `getMessage()`           | Retorna ponteiro para a mensagem              |
| `copyMessage(output)`    | Copia mensagem para buffer externo (recomendado) |
| `acknowledge()`          | Confirma processamento e liberta parser       |
| `reset()`                | Reset completo do parser                      |
| `setMaxFrameGap(micros)` | Configura timeout entre bytes                 |
| `setDebug(enable)`       | Ativa/desativa debug                         |

### 3.4 Parser.cpp

**Função principal:** Implementação completa da FSM e funções de diagnóstico.

Características de segurança:

- **Timeout automático** — Gap entre bytes > maxFrameGapMicros → reset
- **Proteção overflow** — rawOffset < MAX_MESSAGE_SIZE
- **Validação em tempo real** — Cada byte é validado à medida que chega
- **Reset automático em erro** — Qualquer erro reinicia o parser
- **Fallback não-ESP32** — millis()*1000 quando esp_timer não disponível

---

## 4. IDs de Mensagem (MsgID)

| ID   | Constante       | Descrição                                 |
|------|-----------------|-------------------------------------------|
| 0x10 | `MSG_HEARTBEAT` | Trama de heartbeat — indica que o nó está vivo |
| 0x11 | `MSG_TELEMETRY` | Dados de telemetria genérica              |
| 0x12 | `MSG_COMMAND`   | Comando enviado entre nós                 |
| 0x13 | `MSG_ACK`       | Confirmação de receção                    |
| 0x14 | `MSG_FAILSAFE`  | Notificação de modo de segurança          |
| 0x15 | `MSG_DEBUG`     | Mensagens de debug/diagnóstico            |

---

## 5. IDs de Campos TLV — Genéricos (v1.0.0)

### Dados (0x20-0x2F)

| ID   | Constante    | Tipo    | Descrição                        |
|------|--------------|---------|----------------------------------|
| 0x20 | `FLD_DATA_0` | uint8   | Dado genérico 0 — uso livre      |
| 0x21 | `FLD_DATA_1` | uint8   | Dado genérico 1 — uso livre      |
| 0x22 | `FLD_DATA_2` | uint16  | Dado genérico 2 — uso livre      |
| 0x23 | `FLD_DATA_3` | uint16  | Dado genérico 3 — uso livre      |
| 0x24 | `FLD_DATA_4` | int32   | Dado genérico 4 — uso livre      |
| 0x25 | `FLD_DATA_5` | int32   | Dado genérico 5 — uso livre      |
| 0x26 | `FLD_DATA_6` | float   | Dado genérico 6 — uso livre      |
| 0x27 | `FLD_DATA_7` | float   | Dado genérico 7 — uso livre      |

### Estado / Status (0x30-0x37)

| ID   | Constante      | Tipo    | Descrição                        |
|------|----------------|---------|----------------------------------|
| 0x30 | `FLD_STATUS_0` | uint8   | Estado genérico 0 — uso livre    |
| 0x31 | `FLD_STATUS_1` | uint8   | Estado genérico 1 — uso livre    |
| 0x32 | `FLD_STATUS_2` | uint16  | Estado genérico 2 — uso livre    |

### Parâmetros de Comando (0x40-0x44)

| ID   | Constante    | Tipo    | Descrição                        |
|------|--------------|---------|----------------------------------|
| 0x40 | `FLD_CMD_0`  | uint8   | Parâmetro de comando 0 — uso livre |
| 0x41 | `FLD_CMD_1`  | uint8   | Parâmetro de comando 1 — uso livre |
| 0x42 | `FLD_CMD_2`  | float   | Parâmetro de comando 2 — uso livre |

---

## 6. IDs de Comando (0xC0-0xC4)

| ID   | Constante      | Parâmetros              | Descrição               |
|------|----------------|-------------------------|--------------------------|
| 0xC0 | `CMD_ARM`      | Nenhum                  | Armar — modo ativo       |
| 0xC1 | `CMD_DISARM`   | Nenhum                  | Desarmar — modo seguro   |
| 0xC2 | `CMD_SET_MODE` | uint8_t                 | Definir modo de operação |
| 0xC3 | `CMD_REBOOT`   | Nenhum                  | Reiniciar dispositivo    |
| 0xC4 | `CMD_SHUTDOWN` | Nenhum                  | Desligamento controlado  |

---

## 7. Exemplo de Uso

### Construir mensagem

```cpp
TLVMessage msg;
msg.startByte = START_BYTE;
msg.msgID = MSG_TELEMETRY;
msg.tlvCount = 0;

/* Adicionar dados genéricos */
addTLVUint8(&msg, FLD_STATUS_0, 1);           /* Estado ativo */
addTLVFloat(&msg, FLD_DATA_6, 3.14f);         /* Valor float */
addTLVInt32(&msg, FLD_DATA_4, 123456);        /* Valor inteiro */

uint8_t buffer[MAX_MESSAGE_SIZE];
size_t len = buildMessage(&msg, MSG_TELEMETRY, buffer, sizeof(buffer));

/* Enviar via UART */
Serial.write(buffer, len);
```

### Receber e processar

```cpp
Parser parser;

void loop() {
    while (Serial.available()) {
        uint8_t byte = Serial.read();

        if (parser.feed(byte)) {
            TLVMessage* msg = parser.getMessage();

            if (msg->msgID == MSG_COMMAND) {
                for (uint8_t i = 0; i < msg->tlvCount; i++) {
                    switch (msg->tlvs[i].id) {
                        case FLD_CMD_0:
                            uint8_t cmd = msg->tlvs[i].data[0];
                            break;
                        case FLD_CMD_2:
                            float param = bytesToFloat(msg->tlvs[i].data);
                            break;
                    }
                }
            }

            parser.acknowledge();
        }
    }
}
```

### Validar mensagem manualmente

```cpp
if (validateMessage(buffer, length)) {
    /* Mensagem válida — processar */
}
```

### Calcular CRC8

```cpp
uint8_t crc = calcCRC8(data, dataLen);
```

---

## 8. Segurança e Robustez

| Mecanismo                   | Descrição                                        |
|-----------------------------|--------------------------------------------------|
| START_BYTE = 0xAA           | Padrão 10101010, distinguível de ruído           |
| CRC8/SMBUS (0x07)           | Deteta erros de 1-2 bits, rajadas ≤8 bits        |
| Intervalos de IDs seguros   | 0x20-0x2F, 0x30-0x37, 0x40-0x44 evitam colisões |
| Limites máximos             | Previnem overflow e alocações dinâmicas           |
| Parser com timeout          | Recupera automaticamente de streams corrompidos   |
| Reset automático em erro    | Qualquer erro reinicia o parser imediatamente     |
| Sem alocações dinâmicas     | Toda a memória é estática — comportamento preditivo |

---

## 9. Debug

### Ativação

```cpp
#define PROTOCOL_DEBUG   /* Ativa debug do Protocol.cpp */
#define PARSER_DEBUG     /* Ativa debug do Parser.cpp */
```

### Prefixos de output

- `[Protocol]` — Mensagens do módulo de protocolo
- `[Parser]` — Mensagens do parser FSM

### Exemplo de output

```
[Parser] START_BYTE recebido (0xAA) → WAIT_MSGID
[Parser] msgID=0x11 válido → WAIT_TLVCOUNT
[Parser] tlvCount=3 → WAIT_TLV_ID
[Parser] TLV[0] ID=0x20 → WAIT_TLV_LEN
[Parser] TLV[0] len=4 → WAIT_TLV_DATA
[Parser] TLV[0] completo → próximo TLV
[Protocol] buildMessage: msgID=0x11, 3 TLVs, tamanho=19 bytes
```

---

## 10. Notas Finais

- O protocolo v1.0.0 é **genérico** — os FieldIDs devem ser documentados pela aplicação
- A integridade depende apenas de **CRC8** — para segurança adicionar camada superior
- O **timeout** entre bytes é configurável (default 100ms)
- O protocolo é **transport-agnostic** mas optimizado para **UART**
- Versões futuras podem adicionar HMAC, mais FieldIDs ou mais comandos

Fim da documentação — Bythos Protocol V1
