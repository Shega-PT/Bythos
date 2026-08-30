# Bythos v3.0.0 — Protocol Specification

**Bythos Protocol v3.0.0**
Version: 3.0.0 | Author: ShegaPT | License: GPL-3.0

---

## 1. Overview

The Bythos Protocol is a binary communication protocol shared by all modules and systems within Bythos. It uses TLV (Type-Length-Value) with a FieldID field that embeds the data type, allowing 8 types × 32 IDs = 256 possible fields.

### Features
- **Compact binary format** — minimizes overhead on CAN bus and WiFi
- **Extensible TLV** — new fields can be added without breaking compatibility
- **FieldID with embedded type** — 3 bits of type + 5 bits of ID = 1 byte per field
- **XOR signature** — basic authenticity verification
- **CRC-16/CCITT** — data integrity
- **SEQ_NUM u16** — anti-replay at 100Hz
- **Endianness:** Little-endian (compatible with ESP32/Xtensa)

---

## 2. Message Format

### 2.1 Wire Structure

```text
[START_BYTE:1][VERSION:1][NODE_ID:1][MSG_ID:1][SEQ_NUM:2 LE][TLV_COUNT:1]
[TLV_FIELDS...][SIGNATURE:1][CRC16:2 LE]
```

### 2.2 Sizes

| Component          | Size (bytes) |
|--------------------|:------------:|
| Header (fixed)     | 7            |
| Signature          | 1            |
| CRC16              | 2            |
| **Total Overhead** | **10**       |
| TLV Header (per field) | 2        |
| TLV Data (max per field) | 32     |
| Max TLV fields per msg | 32       |
| **Max message**    | **1098**     |

### 2.3 Header Fields

| Offset | Field       | Size  | Description                            |
|:------:|-------------|:-----:|----------------------------------------|
| 0      | START_BYTE  | 1     | Fixed: `0xAA` (170)                    |
| 1      | VERSION     | 1     | Protocol version: `0x03` (v3)          |
| 2      | NODE_ID     | 1     | Transmitter node ID (CAN group)        |
| 3      | MSG_ID      | 1     | Message type (0x10-0x1B)               |
| 4-5    | SEQ_NUM     | 2     | Sequence number (u16, LE)              |
| 6      | TLV_COUNT   | 1     | Number of TLV fields (0-32)            |

### 2.4 TLV Field (each field)

```text
[FIELD_ID:1][LEN:1][DATA:LEN]
```

- **FIELD_ID:** Embedded type `[TYPE:3][ID:5]` — 1 byte
- **LEN:** Number of data bytes (0-32)
- **DATA:** Raw data

### 2.5 Trailer

```text
[SIGNATURE:1][CRC16_LO:1][CRC16_HI:1]
```

---

## 3. FieldID — Encoding with Embedded Type

The 1-byte FieldID combines type and identifier:

```text
  Bit:  7  6  5  4  3  2  1  0
        [  TYPE  ] [    ID     ]
         3 bits      5 bits
```

### 3.1 Data Types (3 bits)

| Value | Type     | Default Size | Description                          |
|:-----:|----------|:------------:|--------------------------------------|
| 0     | Raw      | Variable     | Raw data (binary payload)            |
| 1     | Float32  | 4            | 32-bit float                         |
| 2     | Float16  | 2            | 16-bit float (half-precision)        |
| 3     | Int32    | 4            | Signed 32-bit integer                |
| 4     | Uint32   | 4            | Unsigned 32-bit integer              |
| 5     | Uint16   | 2            | Unsigned 16-bit integer              |
| 6     | Uint8    | 1            | Unsigned 8-bit integer               |
| 7     | Bool     | 1            | Boolean (0=false, 1=true)            |

### 3.2 FieldID Ranges by Type

| Type  | FieldID Range | Examples                 |
|-------|:-------------:|--------------------------|
| Raw   | 0x00 - 0x1F  | VideoPayload (0x00)      |
| f32   | 0x20 - 0x3F  | GPS, IMU, Energy         |
| f16   | 0x40 - 0x5F  | (reserved)               |
| i32   | 0x60 - 0x7F  | (reserved)               |
| u32   | 0x80 - 0x9F  | SystemUptime, etc.       |
| u16   | 0xA0 - 0xBF  | FlightLoopTime, etc.     |
| u8    | 0xC0 - 0xDF  | SystemState, etc.        |
| Bool  | 0xE0 - 0xFF  | (reserved)               |

### 3.3 Formula

```rust
// Encode
field_id = (field_type & 0x07) << 5 | (field_id & 0x1F);

// Decode
field_type = (field_id >> 5) & 0x07;
id = field_id & 0x1F;
```

---

## 4. Signature (XOR Key)

The signature is a byte calculated as XOR of 4 values:

```rust
signature = key XOR msg_id XOR seq_lo XOR seq_hi
```

| Component | Description                        |
|-----------|------------------------------------|
| key       | Shared key per node (u8)           |
| msg_id    | Message ID                         |
| seq_lo    | Low byte of SEQ_NUM               |
| seq_hi    | High byte of SEQ_NUM              |

The default key is `0x00` (no effective signature). Each node may have a different key.

---

## 5. CRC-16/CCITT

- **Polynomial:** 0x1021 (CCITT)
- **Initialization:** 0xFFFF
- **Reflection:** None
- **Final XOR:** None (0x0000)
- **Calculation:** Over all message bytes (header + TLV + signature), **before** CRC is added

Known test vector: `"123456789"` → CRC = 0x29B1

---

## 6. Anti-Replay (SEQ_NUM)

- Field `SEQ_NUM` is u16 (0-65535), little-endian
- Each node maintains a counter that increments with each message sent
- The receiver must verify that the received SEQ_NUM is progressive
- Valid for rates up to ~100Hz per node

---

## 7. Backward Compatibility

Semantic versioning rules:
- **Add:** New TLV fields, new MsgIDs (reserved), new FieldIDs
- **Never change:** Existing FieldID, MsgID, or wire structure values
- **Version:** Increment VERSION only on breaking changes
