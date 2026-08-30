# Changelog

Todas as alterações notáveis neste projeto são documentadas neste ficheiro.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

---

## [3.0.0] — 2026-08-29

### Added
- Biblioteca standalone Bythos (Rust + C)
- Parser FSM byte-a-byte com timeout configurável
- FFI completa (`bythos_*` e `bythos_parser_*`)
- CAN ID extended (29-bit) com bits 17-14 para msg_type
- Validação completa de TLV (bounds + tamanhos)
- `last_error` persistente no parser FSM
- `get_timestamp_us()` real com feature `std`
- 21 testes unitários C + 135 testes Rust
- Microbenchmarks C (CRC, assinatura, build, validate, parse)
- `.gitignore`, `README.md`, `README-EN.md`, `CHANGELOG.md`
- Documentação PT-PT e EN

### Changed
- `ACP_VERSION` → `BYTHOS_VERSION`
- `ACP_HEADER_SIZE` → `BYTHOS_HEADER_SIZE`
- `ACP_OVERHEAD` → `BYTHOS_OVERHEAD`
- `AcpFieldType` → `FieldType`
- CanGroup: `Device0..Device14` (genérico)
- `visor_*` FFI → `bythos_*` FFI
- MsgPriority alinhada entre C e Rust
- `bythos_init()` zera estrutura completa com `memset`
- `bythos_validate()` valida campos TLV (não só header)
- Union type-punning → `memcpy` em conversão float

### Fixed
- CAN ID bits 17-14 alinhados entre C e Rust
- `is_valid_field_id()` documentada como genérica
- `extern crate alloc` feature gate corrigido
- Benchmark `ssize_t` → `bythos_ssize_t`
- CRC16 test comment: VISOR → Device5
- PriorityLevel doc comments (Critical ≠ High)

---

## [2.0.0]

<!-- Placeholder — versão anterior ao refactoring para Bythos -->

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

