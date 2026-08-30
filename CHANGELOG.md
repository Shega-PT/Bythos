# Changelog

Todas as alterações notáveis neste projeto são documentadas neste ficheiro.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

---

## [3.0.0] — 2026-08-29

Reescrita completa do Bythos como biblioteca standalone genérica (Rust + C), removendo toda a dependência do ACP/BeaconFly/Visor.

### Added

- **Biblioteca standalone Bythos** — reescrita completa em Rust + C, sem dependências externas
- **FieldID com tipo embutido** — `[TYPE:3][ID:5]` — 8 tipos × 32 IDs = 60+ campos
  - Funções: `field_id_encode()`, `field_id_decode()`, `is_valid_field_id()` (Rust)
  - Funções: `bythos_field_id_encode()`, `bythos_field_id_decode()`, `bythos_field_id_valid()` (C)
- **8 tipos de dados TLV**: Raw, Float32, Float16, Int32, Uint32, Uint16, Uint8, Bool
- **12 tipos de mensagem** (MsgID 0x10–0x1B): Heartbeat, Telemetry, Command, Ack, Failsafe, Debug, Video, Shell, SiData, Watchdog, Ping, Clock
- **60+ FieldIDs** organizados em categorias:
  - GPS (Latitude, Longitude, Altitude, Speed, Course, Satellites, HDOP)
  - IMU (Roll, Pitch, Yaw, Accel X/Y/Z, Gyro X/Y/Z, YawRate)
  - Flight (AltGPS, AltBaro, VSpeed, Airspeed, LoopTime)
  - Power (BattV, BattI, BattCons, BattTemp, BattSOC)
  - Temperature (Temp1–4, ESP1, ESP2)
  - System (State, Mode, Uptime, FreeHeap, CPULoad, ESP1Load, ESP2Load)
  - Failsafe (Reason, Action, State)
  - Video (FrameID, ChunkID, TotalChunks, Payload)
- **Parser FSM de 9 estados** — WaitStart → WaitHeader → WaitTlvCount → WaitTlvId → WaitTlvLen → WaitTlvData → WaitSignature → WaitCrc16Lo → WaitCrc16Hi
- **Timeout configurável via injeção de timestamp** — `TimestampFn`, `Parser::with_timestamp_fn()`, `Parser::set_timestamp_fn()`
- **last_error persistente** no parser FSM para diagnóstico
- **Contadores de sucesso/erro** por parser
- **CAN ID extended (29-bit)** com 5 níveis de prioridade, 14 grupos (Device0–Device14) e 8 tipos de mensagem (Data, Cmd, Ack, Event, Sync, State, Heart, Safety)
- **Funções CAN ID**: `make_can_id()`, `can_id_priority()`, `can_id_src_group()`, `can_id_dst_group()`, `can_id_msg_type()`, `is_safety_bus_id()` (Rust); `bythos_can_id_make()`, `bythos_can_id_priority()`, `bythos_can_id_src()`, `bythos_can_id_dst()`, `bythos_can_id_type()`, `bythos_is_safety_bus()` (C)
- **Assinatura XOR** — `compute_signature()`, `bythos_signature_compute()`, `bythos_signature_validate()`
- **FFI completa** — API Rust (`bythos_*`, `bythos_parser_*`) e C (`bythos.h`) idênticas
- **Conversões de bytes** — `f32_to_bytes`/`bytes_to_f32`, `i32_to_bytes`/`bytes_to_i32`, `u32_to_bytes`/`bytes_to_u32`, `u16_to_bytes`/`bytes_to_u16` (Rust + C)
- **Utilitários** — `bythos_version()`, `bythos_overhead()`, `bythos_max_message_size()`, `bythos_msg_id_valid()`, `bythos_msg_priority()` (C)
- **Build completo C** — `bythos_build()` com 5 parâmetros (msg, msg_id, signature_key, buffer, buffer_size)
- **Validação completa** — `bythos_validate()` valida header + campos TLV + CRC16
- **Validação de assinatura** — `bythos_validate_signature()` separada da validação de integridade
- **138 testes Rust** — 92 unitários + 42 integração + 4 doc-tests
- **21 testes C** unitários + microbenchmarks
- **Microbenchmarks C** — CRC, assinatura, build, validate, parse
- **7 feature flags** — `std`, `transport-uart`, `transport-lora`, `transport-spi`, `transport-i2c`, `transport-can`, `esp`
- **`no_std`** — suporte completo para ambientes sem biblioteca padrão
- **Documentação completa** — 9 guias PT-PT + 9 guias EN

### Changed

- `ACP_VERSION` → `BYTHOS_VERSION`
- `ACP_HEADER_SIZE` → `BYTHOS_HEADER_SIZE`
- `ACP_OVERHEAD` → `BYTHOS_OVERHEAD`
- `AcpFieldType` → `FieldType` (8 tipos com tipo embutido no FieldID)
- `CanGroup`: `Device0..Device14` (genérico, sem nomes específicos de projeto)
- `visor_*` FFI → `bythos_*` FFI
- MAX_MESSAGE_SIZE: 256 → 1098 bytes (expandido para 32 campos × 34 bytes)
- MsgPriority alinhada entre C e Rust (5 níveis: SuperCritical, Critical, High, Medium, Low)
- `bythos_init()` zera estrutura completa com `memset`
- `bythos_validate()` valida campos TLV (não só header)
- Union type-punning → `memcpy` em conversão float
- Parser FSM expandido de 7 para 9 estados (adição de WaitSignature e WaitCrc16Lo/Hi separados)
- CRC8 → CRC16/CCITT como checksum principal
- Timeout entre bytes via injeção de função de timestamp (não mais hardcodado)

### Fixed

- **[CRÍTICO] `bythos_validate()` CRC validation against wrong length** — validava CRC contra tamanho do header em vez do tamanho total da mensagem
- **[CRÍTICO] `bythos_validate_signature()` CRC validation against wrong length** — mesmo bug na validação de assinatura
- **[MODERADO] `FieldFlight::LoopTime` type mismatch** — era 0x44 (Float32), corrigido para 0xA2 (Uint16)
- **[MODERADO] `build_tlv_video()` missing `id` parameter** — função não recebia o ID do campo
- **[MODERADO] Parser timeout functional** — adicionados `TimestampFn`, `with_timestamp_fn()`, `set_timestamp_fn()`, 3 testes de timeout
- **[MENOR] Dead `debug` field and `set_debug()`** — removidos (debug é controlado por feature flag)
- **[MENOR] `crc8_table` missing `const`** — adicionado qualifier `const` em C
- **[MENOR] `is_valid_field_id()` undocumented no-op** — documentada a semântica correta
- CAN ID bits 17-14 alinhados entre C e Rust
- `extern crate alloc` feature gate corrigido
- Benchmark `ssize_t` → `bythos_ssize_t`
- CRC16 test comment corrigido
- PriorityLevel doc comments (Critical ≠ High)

### Technical Details

- START_BYTE: 0xAA (10101010)
- BYTHOS_VERSION: 0x03
- BYTHOS_HEADER_SIZE: 7 bytes (start + version + nodeId + msgId + seq(2) + tlvCount)
- MAX_TLV_FIELDS: 32
- MAX_TLV_DATA: 32 bytes (normal), 128 bytes (vídeo)
- MAX_MESSAGE_SIZE: 1098 bytes
- BYTHOS_OVERHEAD: 10 bytes (header 7 + signature 1 + crc16 2)
- FieldID: [TYPE:3][ID:5] — tipo embutido no byte de ID
- CRC-16/CCITT: polinómio 0x1021, tabela de 256 entradas em ROM
- CRC-8/SMBUS: polinómio 0x07 (legado)
- Parser: FSM determinística, 9 estados, sem alocação dinâmica
- Timeout: 100ms default, configurável via TimestampFn
- Endianness: little-endian
- Alinhamento: 1 byte (sem padding em structs TLV)
- Prioridades: 5 níveis (SuperCritical, Critical, High, Medium, Low)

---

## [2.0.0] — 2026-04-19

### Added

- Bythos Protocol V2 — protocolo TLV completo para comunicação ESP1 ↔ ESP2 ↔ Ground Station
- 3 novos tipos de mensagem: MSG_VIDEO (0x16), MSG_SHELL_CMD (0x17), MSG_SI_DATA (0x18) — total de 9 MsgIDs
- TLVVideoField — campo TLV dedicado a payload de vídeo com capacidade até 128 bytes
- Campos GPS expandidos: FLD_GPS_LINK (qualidade do link), FLD_GPS_HDOP (precisão horizontal)
- Campos IMU expandidos: FLD_ROLL_RATE, FLD_PITCH_RATE, FLD_YAW_RATE (velocidades angulares em °/s)
- Campo FLD_LOOP_TIME — medição do tempo do último loop de controlo em microssegundos
- Campo FLD_BATT_SOC — estado de carga da bateria normalizado (0.0 a 1.0)
- Sistema completo de campos de temperatura: FLD_TEMP1 a FLD_TEMP4 (ESCs), FLD_ESP1_TEMP, FLD_ESP2_TEMP
- Sistema de campos de estado: FLD_STATE, FLD_MODE, FLD_ERRORS (bitmask), FLD_RX_LINK, FLD_TX_LINK, FLD_ESP1_LOAD, FLD_ESP2_LOAD
- Sistema de campos de failsafe: FLD_FS_REASON, FLD_FS_ACTION, FLD_FS_STATE
- Sistema de campos de vídeo: FLD_VIDEO_FRAME_ID, FLD_VIDEO_CHUNK_ID, FLD_VIDEO_TOTAL, FLD_VIDEO_PAYLOAD
- Enums completos: SystemState (8 estados), FlightMode (6 modos), FailsafeReason (6 causas), FailsafeAction (6 ações)
- Comandos de controlo: CMD_SET_ALT_TARGET, CMD_SET_THROTTLE, CMD_SET_ROLL, CMD_SET_PITCH, CMD_SET_YAW, CMD_SET_HEADING
- Comandos avançados: CMD_SENSOR_CALIB, CMD_ACTUATOR_CALIB, CMD_SET_PARAM, CMD_GET_ALL
- Comandos de navegação: CMD_NEXT_WAYPOINTS, CMD_RTL, CMD_SET_POSITION
- Sistema de prioridades dinâmica com 6 níveis (SUPER_CRITICAL a SUPER_LOW) via getMsgPriority()
- MSG_DEBUG com prioridade dinâmica — promovida a SUPER_CRITICAL durante estado de failsafe para diagnóstico remoto
- Classe TLVBuilder — interface fluente para construção expressiva de mensagens TLV
- Conversões de tipos: int32ToBytes, uint32ToBytes, uint16ToBytes (além das existentes float/int32)
- Função isValidFieldID() — validação em tempo de compilação de IDs de campo TLV
- Buffer de mensagens expandido para 1024 bytes (anteriormente 256) — compatibilidade com LoRa 2.4GHz
- MAX_TLV_FIELDS expandido para 32 campos por mensagem (anteriormente 8)
- MAX_TLV_DATA expandido para 32 bytes por payload normal (anteriormente 16)
- MAX_TLV_VIDEO_DATA: 128 bytes — capacidade máxima para chunks de vídeo comprimido
- Documentação completa em PT-PT (PROTOCOL_DOC.md) com exemplos de uso e diagramas FSM
- Guia de referência rápida (PROTOCOL_QUICK.md) para consulta em campo

### Fixed

- **[CRÍTICO] Timeout entre bytes completamente inoperacional** — updateTimestamp() era chamado antes da verificação isTimedOut(), tornando a diferença temporal sempre ~0 e impossibilitando a deteção de gaps entre bytes
- **[CRÍTICO] Buffer overflow em parseTLV()** — memset escrevia para além dos limites do array TLVField.data[] corrompendo campos adjacentes na memória quando payload excedia MAX_TLV_DATA
- **[CRÍTICO] getLastError() sempre retornava PARSER_OK** — setError() definia o erro antes de chamar reset(), que repunha lastError a PARSER_OK, silenciando completamente o diagnosticamento de erros
- **[MODERADO] Header guard #ifdef __cplusplus desbalanceado em Protocol.h** — em modo C, todo o conteúdo do header era ignorado pelo compilador devido a directives de pré-processador desalinhadas
- **[MODERADO] Flag debugEnabled sem efeito** — a variável era definida mas nunca verificada, uma vez que o output de debug é controlado por macro de compilação PARSER_DEBUG
- **[MENOR] #include "esp_timer.h" dentro de corpo de função** — movido para topo do ficheiro dentro de bloco condicional #ifdef ESP32, seguindo boas práticas de organização de código
- **[MENOR] Definições duplicadas de floatToBytes/bytesToFloat** — removidas de Protocol.cpp, mantidas apenas as implementações static inline em Protocol.h para evitar conflitos de ligação

### Changed

- Autor do protocolo alterado de "BeaconFly UAS Team" para "ShegaPT"
- Designação do sistema alterada de "BeaconFly" para "Bythos Protocol V2"

### Technical Details

- START_BYTE: 0xAA (10101010) — sincronização de trama
- MAX_TLV_FIELDS: 32 — máximo de campos TLV por mensagem
- MAX_TLV_DATA: 32 bytes — payload máximo por campo normal
- MAX_TLV_VIDEO_DATA: 128 bytes — payload máximo para vídeo
- MAX_MESSAGE_SIZE: 1024 bytes — buffer expandido para LoRa 2.4GHz
- MIN_MESSAGE_SIZE: 4 bytes — START + MSGID + COUNT + CRC
- CRC-8/SMBUS: polinómio 0x07, tabela de 256 entradas em ROM
- Parser: FSM determinística, 7 estados, sem alocação dinâmica
- Timeout: 100ms default, configurável via setMaxFrameGap()
- Endianness: little-endian (padrão ESP32 e x86/x64)
- Alinhamento: 1 byte (sem padding em structs TLV)
- Prioridades: 6 níveis dinâmicos baseados em tipo de mensagem e estado do sistema

---

## [1.0.0] — 2026-01-22

### Added

- Bythos Protocol V1 — protocolo TLV genérico para comunicação serial via UART
- Parser FSM byte-a-byte com 7 estados e timeout configurável
- CRC8/SMBUS (polinómio 0x07) com tabela de lookup O(1) por byte
- 6 tipos de mensagem: HEARTBEAT, TELEMETRY, COMMAND, ACK, FAILSAFE, DEBUG
- 15 FieldIDs genéricos organizados em 3 categorias (dados, estado, comando)
- 5 comandos básicos: ARM, DISARM, SET_MODE, REBOOT, SHUTDOWN
- Buffer de 256 bytes optimizado para UART (115200 baud)
- Conversões little-endian: float, int32, uint32, uint16
- Funções auxiliares inline C++ para construção de TLVs (addTLV, addTLVFloat, etc.)
- Validação estrutural completa (START_BYTE, msgID, tlvCount, TLVs, CRC8)
- Proteção contra buffer overflow e timeout entre bytes
- Reset automático em erro — recuperação imediata do parser
- Debug condicional com prefixos [Protocol] e [Parser]
- Documentação completa em PT-PT (PROTOCOL_DOC.md)
- Guia de referência rápida (PROTOCOL_QUICK.md)

### Technical Details

- START_BYTE: 0xAA (10101010) — sincronização de trama
- MAX_TLV_FIELDS: 8 — máximo de campos TLV por mensagem
- MAX_TLV_DATA: 16 bytes — payload máximo por campo
- MAX_MESSAGE_SIZE: 256 bytes — buffer optimizado para UART
- MIN_MESSAGE_SIZE: 4 bytes — START + MSGID + COUNT + CRC
- CRC-8/SMBUS: polinómio 0x07, tabela de 256 entradas em ROM
- Parser: FSM determinística, 7 estados, sem alocação dinâmica
- Timeout: 100ms default, configurável via setMaxFrameGap()
- Endianness: little-endian (padrão ESP32 e x86/x64)
- Alinhamento: 1 byte (sem padding em structs TLV)

