# Bythos Protocol V2 – Documentação Completa

> **Versão:** 2.0.0  
> **Data:** 2026-04-19  
> **Autor:** ShegaPT

Esta documentação detalha a **implementação do protocolo de comunicação** usado pelo sistema Bythos Protocol V2. Abrange os 4 arquivos da pasta: `Protocol.h`, `Protocol.cpp`, `Parser.h` e `Parser.cpp`, explicando a lógica, estruturas de dados, prioridades e mecanismos de segurança.

---

## 1. Objetivo do Protocolo

O protocolo foi criado para permitir:

- Comunicação segura e estruturada entre **ESP1(2.4GHz) ↔ ESP2(868MHz) ↔ Ground Station (GS)**.
- Transmissão de **mensagens TLV** (Type-Length-Value) contendo telemetria, comandos, vídeo e informações de failsafe.
- Definição clara de **prioridades** de mensagens para controle de voo crítico.
- Suporte a **failsafe** e **debug** sem comprometer a estabilidade.
- Compatibilidade futura através de **versionamento semântico** (MAJOR.MINOR.PATCH).

O protocolo é **simbiotico** – todos os componentes dependem dele para funcionar em conjunto. Qualquer alteração neste módulo AFETA todo o sistema.

---

## 2. Arquitetura do Protocolo

### 2.1 Formato da Mensagem no Canal

┌─────────┬─────────┬───────────┬─────────────────┬─────────┬──────────────┬─────────┐
│ START   │ MSG ID  │ TLV COUNT │ TLV FIELDS      │ CRC8    │ HMAC (32B)   │ SEQ (4B)│
│ (1 byte)│ (1 byte)│ (1 byte)  │ (variável)      │ (1 byte)│ (opcional)   │ (opç.)  │
├─────────┼─────────┼───────────┼─────────────────┼─────────┼──────────────┼─────────┤
│ 0xAA    │ 0x10-18 │ 0-32      │ ID(1)+LEN(1)+N  │ CRC-8   │ Segurança    │ Anti-   │
│         │         │           │                 │ SMBUS   │ (módulo      │ replay  │
│         │         │           │                 │ 0x07    │ Security)    │         │
└─────────┴─────────┴───────────┴─────────────────┴─────────┴──────────────┴─────────┘

**NOTA:** O HMAC e o SEQ são geridos pelo módulo `Security/` e NÃO fazem parte da estrutura `TLVMessage`. São acrescentados após a serialização.

### 2.2 Formato TLV Individual

Cada campo TLV é serializado de forma compacta:
[ID (1 byte)] [LEN (1 byte)] [DATA (LEN bytes)]

**Exemplo:** Um campo de rotação (float) seria:
0x30 0x04 0x00 0x00 0x80 0x3F → ID=0x30 (ROLL), LEN=4, DATA=1.0f

---

## 3. Arquivos da Pasta

### 3.1 `Protocol.h`

**Função principal:**  
Define estruturas de dados, IDs, enums e constantes usadas em TODO o sistema.

**Principais características:**

| Constante             | Valor   | Descrição                                   |
| ----------------------|---------|---------------------------------------------|
| `START_BYTE`          | 0xAA    | Byte de sincronização (padrão 10101010)     |
| `PROTOCOL_VERSION_STR`| "2.0.0" | Versão do protocolo (logging)               |
| `MAX_TLV_FIELDS`      | 32      | Número máximo de campos por mensagem        |
| `MAX_TLV_DATA`        | 32      | Tamanho máximo de payload normal (bytes)    |
| `MAX_TLV_VIDEO_DATA`  | 128     | Tamanho máximo de payload de vídeo (bytes)  |
| `MAX_MESSAGE_SIZE`    | 1024    | Tamanho máximo de uma mensagem serializada  |
| `MIN_MESSAGE_SIZE`    | 4       | Tamanho mínimo (START + MSGID + COUNT + CRC)|

**Estruturas principais:**

- `TLVField` – Campo TLV padrão (ID + LEN + DATA[32])
- `TLVVideoField` – Campo TLV para vídeo (ID + LEN + DATA[128])
- `TLVMessage` – Mensagem completa (START + MSGID + COUNT + TLVs + CRC)

**Enums definidos:**

| Enum              | Descrição                                         |
| ............------|---------------------------------------------------|
| `PriorityLevel`   | Prioridades de 0 (SUPER_CRITICAL) a 5 (SUPER_LOW) |
| `MsgID`           | Tipos de mensagem (0x10 a 0x18)                   |
| `SystemState`     | Estado do sistema (BOOTING, IDLE, FLYING, etc.)   |
| `FlightMode`      | Modos de voo (MANUAL, STABILIZE, AUTO, RTL, etc.) |
| `FailsafeReason`  | Causas do failsafe (perda de link, bateria, etc.) |
| `FailsafeAction`  | Ações do failsafe (pairar, RTL, aterrar, etc.)    |
| `FieldGPS`        | IDs de campos GPS (0x20-0x26)                     |
| `FieldIMU`        | IDs de campos IMU (0x30-0x39)                     |
| `FieldFlight`     | IDs de campos de voo (0x40-0x44)                  |
| `FieldPower`      | IDs de campos de energia (0x50-0x54)              |
| `FieldTemp`       | IDs de campos de temperatura (0x60-0x65)          |
| `FieldSystem`     | IDs de campos de sistema (0x70-0x76)              |
| `FieldFailsafe`   | IDs de campos de failsafe (0xA1-0xA3)             |
| `FieldVideo`      | IDs de campos de vídeo (0xB0-0xB3)                |
| `CommandBasic`    | Comandos básicos (0xC0-0xC4)                      |
| `CommandControl`  | Comandos de controlo (0xD0-0xD5)                  |
| `CommandAdvanced` | Comandos avançados (0xE0-0xE3)                    |
| `CommandNav`      | Comandos de navegação (0xF0-0xF2)                 |

**Funções de conversão (little-endian):**

| Função                                | Descrição             |
| --------------------------------------|-----------------------|
| `floatToBytes()` / `bytesToFloat()`   | Conversão de float    |
| `int32ToBytes()` / `bytesToInt32()`   | Conversão de int32_t  |
| `uint32ToBytes()` / `bytesToUint32()` | Conversão de uint32_t |
| `uint16ToBytes()` / `bytesToUint16()` | Conversão de uint16_t |

**Funções `constexpr` (validação em tempo de compilação):**

- `isValidMsgID(id)` – Verifica se o ID está no intervalo 0x10-0x18
- `isValidFieldID(id)` – Verifica se está nos intervalos reservados (0x20-0x7F ou 0xA1-0xFF)

**Classe `TLVBuilder` (C++):**  
Facilitador para construção de mensagens com interface fluente:

```cpp
TLVBuilder builder;
builder.addFloat(FLD_ROLL, 0.12f);
builder.addInt32(FLD_GPS_LAT, 412345678);
builder.addUint8(FLD_STATE, SYS_STATE_FLYING);
size_t len = builder.build(MSG_TELEMETRY, buffer, sizeof(buffer));
´´´

3.2 Protocol.cpp

Função principal:

- Implementação das funções de serialização, desserialização, validação CRC8 e conversão de dados.

Funções implementadas:

Função                                      Descrição
calcCRC8(data, len)                         Calcula CRC8 usando tabela de lookup (polinómio 0x07)
buildTLV(id, data, len, output)             Serializa um campo TLV individual
buildTLVVideo(data, len, output)            Serializa um campo TLV de vídeo
buildMessage(msg, msgID, buffer, size)      Serializa uma mensagem TLV completa
validateMessage(buffer, length)             Valida estrutura e CRC8 de uma mensagem
parseTLV(data, length, output, count)       Extrai campos TLV de um buffer
printTLVField(field) / printMessage(msg)    Funções de debug

Lógica de validação (validateMessage):

- Verifica START_BYTE (deve ser 0xAA)
- Verifica msgID (deve estar em 0x10-0x18)
- Verifica tlvCount (≤ MAX_TLV_FIELDS)
- Percorre todos os TLVs validando estrutura
- Verifica CRC8 (calculado vs recebido)

Debug condicional:

- Define PROTOCOL_DEBUG para ativar mensagens de diagnóstico com prefixo [Protocol]. Seguro manter ativo em produção – apenas afeta a consola série, nunca o canal de comunicação.

3.3 Parser.h

Função principal:

- Implementa uma Máquina de Estados Finita (FSM) para reconstruir mensagens TLV a partir de um fluxo contínuo de bytes.

Estados da FSM:

Estado                  Descrição
PARSER_WAIT_START       Aguarda START_BYTE (0xAA)
PARSER_WAIT_MSGID       Aguarda e valida msgID
PARSER_WAIT_TLVCOUNT    Aguarda número de TLVs
PARSER_WAIT_TLV_ID      Aguarda ID do TLV atual
PARSER_WAIT_TLV_LEN     Aguarda comprimento do payload
PARSER_WAIT_TLV_DATA    Acumula bytes do payload
PARSER_WAIT_CHECKSUM    Aguarda CRC8 e valida

Códigos de erro:

Erro                        Descrição
PARSER_OK                   Sem erro
PARSER_ERR_OVERFLOW         Buffer interno excedido
PARSER_ERR_TIMEOUT          Gap entre bytes excedeu timeout
PARSER_ERR_INVALID_START    START_BYTE ou msgID inválido
PARSER_ERR_CHECKSUM         CRC8 inválido
PARSER_ERR_TLV_COUNT        tlvCount > MAX_TLV_FIELDS
PARSER_ERR_TLV_LEN          LEN > MAX_TLV_VIDEO_DATA

Métodos principais:

Método                  Descrição
feed(byte)              Alimenta um byte ao parser (retorna 1 quando mensagem completa)
hasMessage()            Verifica se há mensagem disponível
getMessage()            Retorna ponteiro para a mensagem
copyMessage(output)     Copia mensagem para buffer externo (recomendado)
acknowledge()           Confirma processamento e liberta parser
reset()                 Reset completo do parser
setMaxFrameGap(micros)  Configura timeout entre bytes
setDebug(enable)        Ativa/desativa debug

3.4 Parser.cpp

Função principal:

- Implementa a lógica da FSM definida em Parser.h.

Fluxo byte-a-byte:

WAIT_START ──(0xAA)──→ WAIT_MSGID ──(msgID válido)──→ WAIT_TLVCOUNT
                                               |           │
                                        (tlvCount==0)  (tlvCount>0)
                                               ↓           ↓
                                         WAIT_CHECKSUM   WAIT_TLV_ID
                                               ↑              │
                                               │              ↓
                                         (após CRC)    WAIT_TLV_LEN
                                               ↑              │
                                               │              ↓
                                         (último TLV)   WAIT_TLV_DATA
                                               ↑              │
                                               └──────────────┘

Características de segurança:

- Timeout automático – Se o gap entre bytes exceder maxFrameGapMicros, o parser reinicia.
- Proteção contra buffer overflow – Verifica rawOffset < MAX_MESSAGE_SIZE.
- Validação em tempo real – Cada byte é validado à medida que chega.
- Reset automático em erro – Qualquer erro faz o parser reiniciar, pronto para a próxima mensagem.
- Fallback para non-ESP32 – Timeout usando millis()*1000 quando esp_timer_get_time() não disponível.

4. Prioridades de Mensagem

A função getMsgPriority(msgID, failsafeActive) retorna a prioridade baseada no tipo de mensagem e estado do sistema:

Prioridade      Valor   Mensagens                                  Comportamento
SUPER_CRITICAL  0       MSG_FAILSAFE, MSG_DEBUG (em failsafe)      Nunca descartado
CRITICAL        1       MSG_COMMAND                                Sempre transmitido
HIGH            2       MSG_HEARTBEAT, MSG_TELEMETRY, MSG_SI_DATA  Prioridade alta
MEDIUM          3       MSG_VIDEO, MSG_ACK                         Prioridade normal
LOW             4       MSG_SHELL_CMD                              Pode ser atrasado
SUPER_LOW       5       MSG_DEBUG (voo normal)                     Descartável se necessário

Regra especial: MSG_DEBUG torna-se SUPER_CRITICAL quando failsafeActive == true – permite diagnóstico remoto durante emergências.

5. IDs de Mensagem (MsgID)

ID      Constante       Descrição
0x10    MSG_HEARTBEAT   Coração do sistema – indica que ESP1 está vivo
0x11    MSG_TELEMETRY   Dados de telemetria normal
0x12    MSG_COMMAND     Comandos da GS para o ESP1
0x13    MSG_ACK         Confirmação de receção
0x14    MSG_FAILSAFE    Ativação/desativação do modo de segurança
0x15    MSG_DEBUG       Mensagens de debug (prioridade dinâmica)
0x16    MSG_VIDEO       Dados de vídeo (streaming experimental)
0x17    MSG_SHELL_CMD   Comando de shell remoto (diagnóstico)
0x18    MSG_SI_DATA     Dados em unidades SI (ESP2 → ESP1)

6. IDs de Campos TLV (COMPLETO v2.0.0)

GPS / Navegação (0x20-0x2F)

ID      Constante       Tipo        Descrição
0x20    FLD_GPS_LAT     int32_t     Latitude (graus × 10⁷)
0x21    FLD_GPS_LON     int32_t     Longitude (graus × 10⁷)
0x22    FLD_GPS_ALT     int32_t     Altitude GPS (mm)
0x23    FLD_GPS_SPEED   uint16_t    Velocidade GPS (cm/s)
0x24    FLD_GPS_SATS    uint8_t     Número de satélites
0x25    FLD_GPS_LINK    uint8_t     Qualidade do link GPS (%)
0x26    FLD_GPS_HDOP    uint16_t    HDOP (precisão horizontal ×100) — NOVO

IMU / Atitude (0x30-0x3F)

ID      Constante       Tipo    Descrição
0x30    FLD_ROLL        float   Ângulo de rolamento (rad)
0x31    FLD_PITCH       float   Ângulo de inclinação (rad)
0x32    FLD_YAW         float   Ângulo de guinada (rad)
0x33    FLD_VX          float   Velocidade linear X (m/s)
0x34    FLD_VY          float   Velocidade linear Y (m/s)
0x35    FLD_VZ          float   Velocidade linear Z (m/s)
0x36    FLD_HEADING     float   Rumo magnético (graus)
0x37    FLD_ROLL_RATE   float   Velocidade angular roll (°/s) — NOVO
0x38    FLD_PITCH_RATE  float   Velocidade angular pitch (°/s) — NOVO
0x39    FLD_YAW_RATE    float   Velocidade angular yaw (°/s) — NOVO

Estado de Voo (0x40-0x4F)

ID      Constante       Tipo        Descrição
0x40    FLD_ALT_GPS     float       Altitude GPS (m)
0x41    FLD_ALT_BARO    float       Altitude barométrica (m)
0x42    FLD_VEL_GPS     float       Velocidade GPS (m/s)
0x43    FLD_VEL_CALC    float       Velocidade por fusão (m/s)
0x44    FLD_LOOP_TIME   uint16_t    Tempo do último loop (µs) — NOVO

Energia (0x50-0x5F)

ID      Constante       Tipo    Descrição
0x50    FLD_BATT_V      float   Tensão da bateria (V)
0x51    FLD_BATT_A      float   Corrente (A)
0x52    FLD_BATT_W      float   Potência (W)
0x53    FLD_BATT_CHG    float   Carga consumida (Ah)
0x54    FLD_BATT_SOC    float   Estado de carga (0-1) — NOVO

Temperatura (0x60-0x6F)

ID      Constante       Tipo    Descrição
0x60    FLD_TEMP1       float   Temperatura ESC1 (°C)
0x61    FLD_TEMP2       float   Temperatura ESC2 (°C)
0x62    FLD_TEMP3       float   Temperatura ESC3 (°C)
0x63    FLD_TEMP4       float   Temperatura ESC4 (°C)
0x64    FLD_ESP1_TEMP   float   Temperatura ESP1 (°C)
0x65    FLD_ESP2_TEMP   float   Temperatura ESP2 (°C)

Sistema (0x70-0x7F)

ID      Constante       Tipo        Descrição
0x70    FLD_STATE       uint8_t     Estado do sistema (SystemState)
0x71    FLD_MODE        uint8_t     Modo de voo (FlightMode)
0x72    FLD_ERRORS      uint32_t    Mapa de bits de erros
0x73    FLD_RX_LINK     uint8_t     Qualidade RX (%)
0x74    FLD_TX_LINK     uint8_t     Qualidade TX (%)
0x75    FLD_ESP1_LOAD   uint8_t     Carga CPU ESP1 (%)
0x76    FLD_ESP2_LOAD   uint8_t     Carga CPU ESP2 (%)

Failsafe (0xA1-0xAF)

ID      Constante       Tipo        Descrição
0xA1    FLD_FS_REASON   uint8_t     Razão (FailsafeReason)
0xA2    FLD_FS_ACTION   uint8_t     Ação (FailsafeAction)
0xA3    FLD_FS_STATE    uint8_t     Estado (1=ativo, 0=inativo)

Vídeo (0xB0-0xBF)

ID      Constante           Tipo        Descrição
0xB0    FLD_VIDEO_FRAME_ID  uint16_t    Número do frame
0xB1    FLD_VIDEO_CHUNK_ID  uint8_t     Índice do chunk
0xB2    FLD_VIDEO_TOTAL     uint8_t     Total de chunks no frame
0xB3    FLD_VIDEO_PAYLOAD   uint8_t[]   Dados de vídeo crus

7. IDs de Comando (0xC0–0xFF)

Comandos Básicos (0xC0–0xCF)

ID      Constante       Parâmetros              Descrição
0xC0    CMD_ARM         Nenhum                  Armar motores
0xC1    CMD_DISARM      Nenhum                  Desarmar motores
0xC2    CMD_SET_MODE    uint8_t (FlightMode)    Mudar modo de voo
0xC3    CMD_REBOOT      Nenhum                  Reiniciar ESP1
0xC4    CMD_SHUTDOWN    Nenhum                  Desligamento controlado

Comandos de Controlo (0xD0–0xDF)

ID      Constante           Parâmetros      Descrição
0xD0    CMD_SET_ALT_TARGET  float (metros)  Definir altitude alvo
0xD1    CMD_SET_THROTTLE    float (0-1)     Definir throttle
0xD2    CMD_SET_ROLL        float (graus)   Definir ângulo de roll
0xD3    CMD_SET_PITCH       float (graus)   Definir ângulo de pitch
0xD4    CMD_SET_YAW         float (°/s)     Definir velocidade de yaw
0xD5    CMD_SET_HEADING     float (graus)   Definir heading desejado — NOVO

Comandos Avançados (0xE0–0xEF)

ID      Constante           Parâmetros          Descrição
0xE0    CMD_SENSOR_CALIB    Nenhum              Calibrar sensores
0xE1    CMD_ACTUATOR_CALIB  Nenhum              Calibrar atuadores
0xE2    CMD_SET_PARAM       ID param + valor    Definir parâmetro
0xE3    CMD_GET_ALL         Nenhum              Pedir telemetria completa — NOVO

Comandos de Navegação (0xF0–0xFF)

ID      Constante           Parâmetros              Descrição
0xF0    CMD_NEXT_WAYPOINTS  Lista TLV de waypoints  Enviar waypoints
0xF1    CMD_RTL             Nenhum                  Return To Launch
0xF2    CMD_SET_POSITION    lat, lon, alt           Definir posição alvo — NOVO

8. Exemplo de Uso

Construir mensagem com TLVBuilder

```cpp
TLVBuilder builder;
builder.addFloat(FLD_ROLL, 0.12f);
builder.addFloat(FLD_PITCH, -0.05f);
builder.addInt32(FLD_GPS_LAT, 412345678);
builder.addUint8(FLD_STATE, SYS_STATE_FLYING);
builder.addUint8(FLD_MODE, MODE_STABILIZE);

uint8_t buffer[MAX_MESSAGE_SIZE];
size_t len = builder.build(MSG_TELEMETRY, buffer, sizeof(buffer));

// Enviar via UART/LoRa
Serial.write(buffer, len);
´´´

Receber e processar mensagem

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
                        case CMD_SET_ROLL:
                            float roll = bytesToFloat(msg->tlvs[i].data);
                            break;
                        case CMD_SET_HEADING:
                            float heading = bytesToFloat(msg->tlvs[i].data);
                            break;
                    }
                }
            }
            
            parser.acknowledge();
        }
    }
}
´´´

9. Segurança e Robustez

Mecanismo                   Descrição
START_BYTE = 0xAA           Padrão binário 10101010, facilmente distinguível de ruído
CRC8 SMBUS (0x07)           Deteta erros de 1-2 bits, rajadas ≤8 bits
Intervalos de IDs seguros   0x20-0x7F e 0xA1-0xFF evitam colisão com ASCII de controlo
Limites máximos             Previnem buffer overflow e alocações dinâmicas
Parser com timeout          Recupera automaticamente de streams corrompidos
Reset automático em erro    Qualquer erro reinicia o parser para a próxima mensagem
Sem alocações dinâmicas     Toda a memória é estática – comportamento determinístico

10. Observações Finais

O protocolo foi desenhado para ser simbiotico entre ESP1, ESP2 e GS.
Facilita futuras expansões de TLVs e IDs de mensagens (basta adicionar novos enums).
Debug e telemetria podem ser priorizados dinamicamente conforme o estado do sistema.
Mensagens podem ser transmitidas via UART, LoRa 2.4GHz, LoRa 868MHz, ou qualquer meio serial confiável.
Os arquivos .h e .cpp devem ser incluídos nos projetos ESP1, ESP2 e Ground Station conforme a arquitetura modular.

Fim da documentação – Bythos Protocol V2
