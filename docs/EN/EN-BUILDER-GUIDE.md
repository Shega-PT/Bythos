# Bythos v3.0.0 — Builder Guide

Author: ShegaPT | License: GPL-3.0

## Overview

The `TLVBuilder` is a fluent constructor for Bythos v3.0.0 messages. It enables building messages safely and efficiently by adding fields one at a time and serializing the complete message at the end.

---

## Rust Usage

```rust
use bythos::protocol::builder::TLVBuilder;

// Create builder with node_id=0x06, signature_key=0x42
let mut builder = TLVBuilder::new(0x06, 0x42);

// Set sequence number
builder.set_seq(1);

// Add TLV fields (field_id is the logical ID 0-31; type is auto-encoded)
builder.add_u8_field(0, 2).unwrap();          // SystemState = Ready (ID=0, type=6(u8) → 0xC0)
builder.add_f32_field(6, -33.8999).unwrap();  // GPS Latitude (ID=6, type=1(f32) → 0x26)
builder.add_f32_field(7, 151.2093).unwrap();  // GPS Longitude (ID=7, type=1(f32) → 0x27)
builder.add_f32_field(16, 1.5).unwrap();      // IMU Roll (ID=16, type=1(f32) → 0x30)
builder.add_u32_field(2, 3600).unwrap();      // SystemUptime (ID=2, type=4(u32) → 0x82)

// Serialize to buffer
let mut buffer = [0u8; 1098];
let size = builder.build(0x11, &mut buffer).unwrap();
// buffer[..size] contains the complete Bythos message
```

---

## Builder Methods

### Creation

| Method                          | Description                                    |
|---------------------------------|----------------------------------------------|
| `TLVBuilder::new(node_id, key)` | Create new builder                           |
| `set_seq(seq)`                  | Set sequence number (u16)                    |

### Adding Fields

| Method                          | Description                              |
|---------------------------------|----------------------------------------|
| `add_f32_field(id, value)`      | Add f32 field (4 bytes)                |
| `add_f16_field(id, value)`      | Add f16 field (2 bytes)                |
| `add_i32_field(id, value)`      | Add i32 field (4 bytes)                |
| `add_u32_field(id, value)`      | Add u32 field (4 bytes)                |
| `add_u16_field(id, value)`      | Add u16 field (2 bytes)                |
| `add_u8_field(id, value)`       | Add u8 field (1 byte)                  |
| `add_bool_field(id, value)`     | Add bool field (1 byte)                |
| `add_raw_field(id, data)`       | Add raw field (variable)               |
| `add_tlv(field)`                | Add pre-built TLV field                |

### Serialization

| Method                          | Description                              |
|---------------------------------|----------------------------------------|
| `build(msg_id, buffer)`         | Serialize message to buffer            |

### Info

| Method                          | Description                              |
|---------------------------------|----------------------------------------|
| `get_tlv_count()`                | Number of fields added                 |

---

## FFI Usage (C/C++)

```c
#include "bythos.h"

BythosMessage msg;
bythos_init(&msg, 0x06, 0x11);  // node_id=0x06, msg_id=0x11 (Telemetry)

// Add fields
bythos_tlv_add_u8(&msg, 0xC0, 2);
bythos_tlv_add_f32(&msg, 0x26, -33.8999f);
bythos_tlv_add_f32(&msg, 0x27, 151.2093f);

// Set sequence
bythos_set_seq(&msg, 1);

// Serialize
uint8_t buffer[1098];
bythos_ssize_t size = bythos_build(&msg, 0x11, 0x42, buffer, sizeof(buffer));
```

---

## Validation

The `build()` method validates:
1. Field count does not exceed `MAX_TLV_FIELDS` (32)
2. TLV field size does not exceed `MAX_TLV_DATA` (32 bytes)
3. Buffer has sufficient size (`MAX_MESSAGE_SIZE` = 1098 bytes)
4. CRC-16/CCITT and signature are calculated automatically

---

## Errors

| Error             | Description                              |
|-------------------|----------------------------------------|
| `BufferTooSmall`  | Output buffer too small                |
| `TooManyFields`   | More than 32 TLV fields                |
| `TlvDataTooLong`  | TLV field with data > 32 bytes         |
