# Bythos — Guia de Migração (V3 → V4)

> A versão atual quebra o fio (`VER 0x03 → 0x04`) e limpa a nomenclatura pública.
> Valores numéricos de `FieldID`, `MsgId` e CRC mantêm-se; mudam **nomes**,
> **tamanhos de endereço** e **segurança**. Este é o mapa completo.

## 1. Conceitos

| V3 (removido do público)              | V4 (único termo público)                    |
|---------------------------------------|---------------------------------------------|
| CAN bus / CAN ID (29 bits, arbitragem elétrica) | Bythos Bus / `BythosBusId` (32 bits, mesma regra "menor ganha", por firmware) — ver `BYTHOS-BUS.md` |
| CanGroup `None/Device0…Device14`      | BythosGroup `BG-*: None/Control/Sensors/Actuators/Safety/Emergency/Vision…` |
| CanMsgType `Data/Cmd/Ack…`            | BythosKind `Data/Cmd/Ack/Event/Sync/State/Heart/Safety` |
| `PriorityLevel`                       | `BythosPriority`                            |
| TLV / campo TLV / `TLVBuilder`        | Campo Bythos / `BythosBuilder`              |
| `TLVField / TLVVideoField / TLVMessage` | `BythosField / BythosVideoField / BythosMessage` |
| `TLV_COUNT / MAX_TLV_* / TLV_HEADER`  | `FIELD_COUNT / MAX_FIELD_* / FIELD_HEADER`  |
| `NODE_ID u8` (grupo CAN)              | `SRC u16 + DST u16` (endereço de anel)      |
| Assinatura XOR 1 B                    | `SEC_HDR 4 B + TAG 4 B` (HMAC em hardware)  |
| `transport-can/uart/lora/spi/i2c`     | `bythos-bus-short / bythos-bus-long / bythos-rf-tunnel` |
| `D+/D-` em docs de utilizador         | `Par-Dados` / `Par-Relógio` (RJ 2 pares)    |

## 2. API Rust (`bythos::…`)

| V3                                    | V4                                          |
|---------------------------------------|---------------------------------------------|
| `protocol::types::{CanGroup, CanMsgType, TLVField, TLVVideoField, TLVMessage, FieldId, FieldType}` | `protocol::types::{BythosGroup, BythosKind, BythosField, BythosVideoField, BythosMessage, BythosFieldId, BythosFieldType}` |
| `make_can_id / can_id_priority / can_id_src_group / can_id_dst_group / can_id_msg_type / is_safety_bus_id` | `make_bythos_bus_id / BythosBusId::{priority, group, kind, src} / wins_over` (+ `make_bythos_route` p/ encaminhamento) |
| `field_id_encode / decode / is_valid_field_id` | `bythos_field_id_encode / decode / is_valid` (métodos de `BythosFieldId`) |
| `compute_signature / validate_signature` (XOR) | `compute_legacy_tag` (compat) + `protocol::secure::{SealConfig, tagv4, verifyv4, ReplayWindow}` |
| `TLVBuilder::{new, add_raw, add_*_field, build}` | `BythosBuilder::{new(src, key_id), …, build(msg, dst, buf)}` |
| `codec::{build_tlv, build_tlv_video, build_message, validate_message, parse_tlv}` | `codec::{build_field, build_field_video, build_message, validate_message, parse_fields}` |
| `parser::fsm::{ParserState::WaitTlv*, ParserError::ErrTlv*}` | `WaitField* / ErrField*` (+ `ErrOverflow`, `ErrHops`, `ErrTag`) |
| `Parser::new(key)`                    | `Parser::new(seal: SealConfig)`             |

> Aliases `#[deprecated]` dos 10 nomes mais usados (`TLVBuilder`, `TLVMessage`,
> `CanGroup`…) existem durante **uma** release para migração mecânica via
> `sed`. Depois são removidos.

## 3. API C (`bythos.h`)

| V3                                    | V4                                          |
|---------------------------------------|---------------------------------------------|
| `BythosTLVField / BythosMessage.tlvs / BYTHOS_MAX_TLV_*` | `BythosField / BythosMessage.fields / BYTHOS_MAX_FIELD_*` |
| `bythos_tlv_add* / bythos_parse_tlv`  | `bythos_field_add* / bythos_parse_fields`   |
| `bythos_can_id_* / bythos_is_safety_bus / BythosCanGroup / BythosCanMsgType` | `bythos_addr_* / bythos_is_safety / BythosGroup / BythosKind` |
| `bythos_init(msg, node_id, msg_id)`   | `bythos_init(msg, src, msg_id)` + `bythos_set_dst(msg, dst)` |
| `bythos_build(msg, msg_id, key, …)`   | `bythos_build(msg, msg_id, seal, …)` (selo vai no `SEC_HDR`) |
| `BYTHOS_VERSION 0x03 / HEADER 7 / OVERHEAD 10 / MAX 1098` | `0x04 / 11 / 21 / 1205` |

## 4. Fio (exemplo de migração)

```text
V3: [AA][03][NODE 06][MSG 11][SEQ 2A 00][N 02][…campos…][SIG][CRC LO HI]
V4: [AA][04][SRC 06 00][DST FF FF][MSG 11][SEQ 2A 00][N 02][HOP 20][…campos…][SEC_HDR 4][TAG 4][CRC LO HI]
```

Regra de tradução `legacy-v3-compat`: `NODE → SRC`, `DST = 0xFFFF` (difusão),
`HOP = 32`, `SEC_HDR.KEY_ID = 0xFF`, `TAG = 0x00000000` (marcador de legado).

## 5. Comportamentos que mudam

1. `NODE_ID > 0x0F` era aceite no Rust e recusado na FFI — morre a ambiguidade:
   endereço é sempre `u16` validado nos dois lados.
2. `validate_signature` sem CRC (Rust) vs com CRC (C) — unificado: `TAG` só se
   verifica sobre trama com CRC válido.
3. `parse_tlv` silencioso em C vs `Result` em Rust — unificado: códigos de erro
   espelhados (`-1-field`, `-2-overflow`, …).
4. `bythos_clear` zerava `START/VERSION` em C — unificado: repõe `0xAA/0x04`.
5. Truncagem silenciosa `min(32)` — proibida: exceder retorna erro sempre.
