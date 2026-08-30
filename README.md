# Bythos Protocol V2

> **Versão:** 2.0.0  
> **Autor:** ShegaPT  
> **Data:** 2026-04-19

Protocolo de comunicação TLV (Type-Length-Value) para o sistema UAS Bythos. Projetado para comunicação fiável entre **ESP1** (Flight Controller), **ESP2** (Sensor Processor) e **Ground Station** via UART e LoRa 2.4GHz.

---

## Arquitetura do Sistema

```
┌──────────────────┐      UART       ┌──────────────────┐
│   ESP1           │◄───────────────►│   ESP2           │
│  (Flight Ctrl)   │   115200 baud   │  (Sensor Proc)   │
│                  │                 │                   │
│  • Recebe cmds   │                 │  • Lê sensores    │
│  • Controla voo  │                 │  • Envia SI data  │
│  • PID/Nav       │                 │  • Telemetria     │
└────────┬─────────┘                 └───────────────────┘
         │
         │  LoRa 2.4GHz
         │
┌────────▼─────────┐
│ Ground Station   │
│  (GS)            │
│                  │
│  • Comandos      │
│  • Telemetria    │
│  • Vídeo         │
│  • Shell remoto  │
└──────────────────┘
```

---

## Formato da Mensagem

Cada mensagem segue o formato TLV compacto, serializado byte a byte:

```
┌─────────┬─────────┬───────────┬─────────────────┬─────────┐
│ START   │ MSG ID  │ TLV COUNT │ TLV FIELDS      │ CRC8    │
│ (1 byte)│ (1 byte)│ (1 byte)  │ (variável)      │ (1 byte)│
├─────────┼─────────┼───────────┼─────────────────┼─────────┤
│ 0xAA    │ 0x10-18 │ 0-32      │ ID(1)+LEN(1)+N  │ SMBUS   │
└─────────┴─────────┴───────────┴─────────────────┴─────────┘
```

Cada campo TLV é serializado como:

```
[ID (1 byte)] [LEN (1 byte)] [DATA (LEN bytes)]
```

---

## V2 vs V1 — O que mudou

### Expansão de Capacidade

| Parâmetro | V1 | V2 | Impacto |
|---|---|---|---|
| `MAX_MESSAGE_SIZE` | 256 bytes | **1024 bytes** | Compatibilidade com LoRa 2.4GHz |
| `MAX_TLV_FIELDS` | 8 | **32** | Telemetria completa numa só mensagem |
| `MAX_TLV_DATA` | 16 bytes | **32 bytes** | Strings curtas, vetores 3D, parâmetros |
| `MAX_TLV_VIDEO_DATA` | — | **128 bytes** | Streaming de vídeo por chunks |
| Tipos de mensagem | 6 | **9** | Vídeo, shell remoto, dados SI |

### Novos Campos TLV

| Categoria | Campos Novos | Descrição |
|---|---|---|
| **GPS** | `FLD_GPS_LINK`, `FLD_GPS_HDOP` | Qualidade do link e precisão horizontal |
| **IMU** | `FLD_ROLL_RATE`, `FLD_PITCH_RATE`, `FLD_YAW_RATE` | Velocidades angulares (°/s) |
| **Voo** | `FLD_LOOP_TIME` | Tempo do loop de controlo (µs) |
| **Energia** | `FLD_BATT_SOC` | Estado de carga da bateria (0-1) |
| **Temperatura** | `FLD_TEMP1-4`, `FLD_ESP1_TEMP`, `FLD_ESP2_TEMP` | 6 sensores de temperatura |
| **Sistema** | `FLD_STATE`, `FLD_MODE`, `FLD_ERRORS`, etc. | Estado completo do sistema |
| **Failsafe** | `FLD_FS_REASON`, `FLD_FS_ACTION`, `FLD_FS_STATE` | Diagnóstico de emergência |
| **Vídeo** | `FLD_VIDEO_FRAME_ID`, `FLD_VIDEO_CHUNK_ID`, etc. | Streaming fragmentado |

### Novos Comandos

| Categoria | Comandos | Descrição |
|---|---|---|
| **Controlo** | `CMD_SET_ALT_TARGET`, `CMD_SET_THROTTLE`, `CMD_SET_ROLL`, `CMD_SET_PITCH`, `CMD_SET_YAW`, `CMD_SET_HEADING` | Controlo granular de voo |
| **Avançados** | `CMD_SENSOR_CALIB`, `CMD_ACTUATOR_CALIB`, `CMD_SET_PARAM`, `CMD_GET_ALL` | Calibração e configuração |
| **Navegação** | `CMD_NEXT_WAYPOINTS`, `CMD_RTL`, `CMD_SET_POSITION` | Navegação autónoma |

### Novos Enums

| Enum | Valores | Descrição |
|---|---|---|
| `SystemState` | 8 estados | BOOTING → INIT → IDLE → ARMED → FLYING → FAILSAFE → ERROR → SHUTDOWN |
| `FlightMode` | 6 modos | MANUAL, STABILIZE, ALT_HOLD, POSHOLD, AUTO, RTL |
| `FailsafeReason` | 6 causas | LOST_LINK, LOW_BATTERY, GPS_LOST, IMU_ERROR, MANUAL_TRIGGER |
| `FailsafeAction` | 6 ações | HOVER, RTL, LAND, CUT_THROTTLE, DISARM |
| `PriorityLevel` | 6 níveis | SUPER_CRITICAL → CRITICAL → HIGH → MEDIUM → LOW → SUPER_LOW |

### Sistema de Prioridades Dinâmica

A V2 introduz prioridades dinâmicas que se adaptam ao estado do sistema:

```
MSG_FAILSAFE          → SUPER_CRITICAL (sempre)
MSG_COMMAND           → CRITICAL
MSG_HEARTBEAT         → HIGH
MSG_TELEMETRY         → HIGH
MSG_SI_DATA           → HIGH
MSG_VIDEO             → MEDIUM
MSG_ACK               → MEDIUM
MSG_SHELL_CMD         → LOW
MSG_DEBUG (voo)       → SUPER_LOW
MSG_DEBUG (failsafe)  → SUPER_CRITICAL  ← PROMOVIDO durante emergência
```

### Correções de Bugs (V1 → V2)

A V2 corrige 3 bugs críticos detetados na V1:

- **Timeout entre bytes inoperacional** — a deteção de gaps nunca disparava
- **Buffer overflow em `parseTLV()`** — corrompia memória adjacente
- **`getLastError()` sempre retornava `PARSER_OK`** — diagnosticamento silenciado

---

## Estrutura de Ficheiros

```
BythosV2/
├── Protocol.h          Definições partilhadas (structs, enums, constantes, TLVBuilder)
├── Protocol.cpp        Implementação (CRC8, serialização, validação, parseTLV)
├── Parser.h            FSM byte-a-byte para reconstrução de mensagens
├── Parser.cpp          Implementação do parser (7 estados, timeout, reset)
├── CHANGELOG.md        Histórico de versões
├── PROTOCOL_DOC.md     Documentação completa em PT-PT
└── PROTOCOL_QUICK.md   Guia de referência rápida
```

---

## Integração

### 1. Incluir nos projetos

```cpp
// No ESP1, ESP2 e Ground Station:
#include "Protocol.h"
#include "Parser.h"
```

### 2. Construir uma mensagem (envio)

```cpp
#include "Protocol.h"

uint8_t buffer[MAX_MESSAGE_SIZE];

// Usando TLVBuilder (interface fluente):
TLVBuilder builder;
builder.addFloat(FLD_ROLL, 0.12f);
builder.addFloat(FLD_PITCH, -0.05f);
builder.addInt32(FLD_GPS_LAT, 412345678);
builder.addUint8(FLD_STATE, SYS_STATE_FLYING);
builder.addUint8(FLD_MODE, MODE_STABILIZE);

size_t len = builder.build(MSG_TELEMETRY, buffer, sizeof(buffer));

// Enviar via UART/LoRa:
Serial.write(buffer, len);
```

### 3. Receber e processar mensagens

```cpp
#include "Parser.h"

Parser parser;

void loop() {
    while (Serial.available()) {
        uint8_t byte = Serial.read();
        
        if (parser.feed(byte)) {
            TLVMessage* msg = parser.getMessage();
            
            if (msg->msgID == MSG_COMMAND) {
                for (uint8_t i = 0; i < msg->tlvCount; i++) {
                    switch (msg->tlvs[i].id) {
                        case CMD_SET_ROLL:
                            float roll = bytesToFloat(msg->tlvs[i].data);
                            break;
                        case CMD_SET_HEADING:
                            float heading = bytesToFloat(msg->tlvs[i].data);
                            break;
                    }
                }
            }
            
            parser.acknowledge();  // Libertar parser para próxima mensagem
        }
    }
}
```

### 4. Validar mensagem (receção)

```cpp
if (validateMessage(buffer, length)) {
    // Mensagem estruturalmente válida + CRC8 correto
    // Proceder para Security::verifyPacket() (HMAC + SEQ)
}
```

---

## Parser — Máquina de Estados

O parser reconstrói mensagens TLV a partir de um stream contínuo de bytes:

```
WAIT_START ──(0xAA)──→ WAIT_MSGID ──(válido)──→ WAIT_TLVCOUNT
                                                    │
                                    (tlvCount==0)   │ (tlvCount>0)
                                            ↓       ↓
                                      WAIT_CHECKSUM  WAIT_TLV_ID
                                            ↑             │
                                            │             ↓
                                      (após CRC)   WAIT_TLV_LEN
                                            ↑             │
                                            │             ↓
                                      (último TLV)  WAIT_TLV_DATA
                                            ↑             │
                                            └─────────────┘
```

**Características:**
- Timeout configurável entre bytes (default: 100ms)
- Reset automático em qualquer erro
- Sem alocações dinâmicas — memória estática
- Tolerante a streams corrompidos

---

## Constantes Técnicas

| Constante | Valor | Descrição |
|---|---|---|
| `START_BYTE` | `0xAA` | Sincronização de trama (10101010) |
| `MAX_TLV_FIELDS` | 32 | Máximo de campos por mensagem |
| `MAX_TLV_DATA` | 32 bytes | Payload máximo por campo normal |
| `MAX_TLV_VIDEO_DATA` | 128 bytes | Payload máximo para vídeo |
| `MAX_MESSAGE_SIZE` | 1024 bytes | Tamanho máximo da mensagem |
| `MIN_MESSAGE_SIZE` | 4 bytes | START + MSGID + COUNT + CRC |
| CRC | SMBUS (0x07) | Tabela de 256 entradas em ROM |
| Endianness | Little-endian | Padrão ESP32 e x86/x64 |

---

## Debug

O debug é controlado por macros de compilação:

```cpp
#define PROTOCOL_DEBUG   // Ativa debug do Protocol [Protocol]
#define PARSER_DEBUG     // Ativa debug do Parser [Parser]
```

Mensagens de debug são impressas para Serial/console com prefixos `[Protocol]` e `[Parser]`. Seguro manter ativo em produção — não afeta o canal de comunicação.

---

## Documentação

- **[PROTOCOL_DOC.md](PROTOCOL_DOC.md)** — Documentação completa com todos os IDs, enums, structs e exemplos
- **[PROTOCOL_QUICK.md](PROTOCOL_QUICK.md)** — Guia de referência rápida para campo
- **[CHANGELOG.md](CHANGELOG.md)** — Histórico de versões (v1.0.0 → v2.0.0)

---

## Licença

Desenvolvido por **ShegaPT** para o sistema UAS Bythos.
