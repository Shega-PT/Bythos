# Changelog

Todas as alterações notáveis neste projeto são documentadas neste ficheiro.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

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
