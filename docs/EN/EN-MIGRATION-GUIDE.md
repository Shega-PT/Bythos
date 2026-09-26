# Bythos — Migration Guide (V3 → V4)

> The current version breaks the wire (`VER 0x03 → 0x04`) and cleans the public
> naming. Numeric `FieldID`, `MsgId` and CRC values are preserved; **names**,
> **address sizes** and **security** change. This is the complete map.

## 1. Concepts

| V3 (removed from public)              | Current (only public term)                      |
|---------------------------------------|-------------------------------------------------|
| CAN bus / CAN ID (29 bits, electrical arbitration) | Bythos Bus / `BythosBusId` (32 bits, same "lowest wins" rule, in firmware) — see `BYTHOS-BUS.md` |
| CanGroup `None/Device0…Device14`      | BythosGroup `BG-*: None/Control/Sensors/Actuators/Safety/Emergency/Vision…` |
| CanMsgType `Data/Cmd/Ack…`            | BythosKind `Data/Cmd/Ack/Event/Sync/State/Heart/Safety` |
| `PriorityLevel`                       | `BythosPriority`                                |
| TLV / TLV field / `TLVBuilder`        | Bythos field / `BythosBuilder`                  |
| `TLVField / TLVVideoField / TLVMessage` | `BythosField / BythosVideoField / BythosMessage` |
| `TLV_COUNT / MAX_TLV_* / TLV_HEADER`  | `FIELD_COUNT / MAX_FIELD_* / FIELD_HEADER`      |
| `NODE_ID u8` (CAN group)              | `SRC u16 + DST u16` (ring address)              |
| 1-byte XOR signature                  | `SEC_HDR 4 B + TAG 4 B` (hardware HMAC)         |
| `transport-can/uart/lora/spi/i2c`     | `bythos-bus-short / bythos-bus-long / bythos-rf-tunnel` |
| `D+/D-` in user docs                  | `Data-Pair` / `Clock-Pair` (2-pair RJ)          |

## 2. Rust API (`bythos::…`)

| V3                                    | Current                                     |
|---------------------------------------|---------------------------------------------|
| `protocol::types::{CanGroup, CanMsgType, TLVField, TLVVideoField, TLVMessage, FieldId, FieldType}` | `protocol::types::{BythosGroup, BythosKind, BythosField, BythosVideoField, BythosMessage, BythosFieldId, BythosFieldType}` |
| `make_can_id / can_id_priority / can_id_src_group / can_id_dst_group / can_id_msg_type / is_safety_bus_id` | `make_bythos_bus_id / BythosBusId::{priority, group, kind, src} / wins_over` (+ `make_bythos_route` for forwarding) |
| `compute_signature / validate_signature` (XOR) | `compute_legacy_tag` (compat) + `protocol::secure::{SealConfig, tagv4, verifyv4, ReplayWindow}` |
| `TLVBuilder::{new, add_raw, add_*_field, build}` | `BythosBuilder::{new(src, seal), …, build(msg, dst, buf)}` |
| `codec::{build_tlv, build_tlv_video, build_message, validate_message, parse_tlv}` | `codec::{build_field, build_field_video, build_message, validate_message, parse_fields}` |
| `parser::fsm::{ParserState::WaitTlv*, ParserError::ErrTlv*}` | `WaitField* / ErrField*` (+ `ErrOverflow`, `ErrHops`) |
| `Parser::new(key)`                    | `Parser::new()` (keyless; TAG verified on `completed_frame`) |

> `#[deprecated]` aliases of the 10 most used names exist for **one** release
> for mechanical migration. Then they are removed.

## 3. C API (`bythos.h`)

| V3                                    | Current                                     |
|---------------------------------------|---------------------------------------------|
| `BythosTLVField / BythosMessage.tlvs / BYTHOS_MAX_TLV_*` | `BythosField / BythosMessage.fields / BYTHOS_MAX_FIELD_*` |
| `bythos_tlv_add* / bythos_parse_tlv`  | `bythos_field_add* / bythos_parse_fields`   |
| `bythos_can_id_* / bythos_is_safety_bus / BythosCanGroup / BythosCanMsgType` | `bythos_ring_decide / bythos_kind_of / BythosGroup / BythosKind` |
| `bythos_init(msg, node_id, msg_id)`   | `bythos_init(msg, src, msg_id)` + `bythos_set_dst(msg, dst)` |
| `bythos_build(msg, msg_id, key, …)`   | `bythos_build(msg, msg_id, dst, key_id, ctr, key, …)` |
| `BYTHOS_VERSION 0x03 / HEADER 7 / OVERHEAD 10 / MAX 1098` | `0x04 / 11 / 21 / 1205` |

## 4. Wire (migration example)

```text
V3: [AA][03][NODE 06][MSG 11][SEQ 2A 00][N 02][…fields…][SIG][CRC LO HI]
V4: [AA][04][SRC 06 00][DST FF FF][MSG 11][SEQ 2A 00][N 02][HOP 20][…fields…][SEC_HDR 4][TAG 4][CRC LO HI]
```

Translation rule (`legacy-v3-compat`): `NODE → SRC`, `DST = 0xFFFF`
(broadcast), `HOPS = 32`, `SEC_HDR.KEY_ID = 0xFF`, `TAG = 0x00000000` (legacy marker).

## 5. Behaviors that change

1. `NODE_ID > 0x0F` accepted in Rust, refused in FFI — ambiguity is dead:
   addresses are always validated `u16` on both sides.
2. `validate_signature` without CRC (Rust) vs with CRC (C) — unified: `TAG` is
   only verified over CRC-valid frames.
3. Silent C `parse_tlv` vs Rust `Result` — unified: mirrored error codes.
4. `bythos_clear` zeroing `START/VERSION` in C — unified: restores `0xAA/0x04`.
5. Silent `min(32)` truncation — forbidden: overrun always returns an error.
