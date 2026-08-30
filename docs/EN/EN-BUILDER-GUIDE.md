# Bythos v3.0.0 — Builder Guide

## Overview

The `TLVBuilder` is a fluent constructor for Bythos v3.0.0 messages. It enables building messages safely and efficiently by adding fields one at a time and serializing the complete message at the end.

---

## Rust Usage

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::types::FieldType;

// Create builder with node_id=0x06, signature_key=0x42
let mut builder = TLVBuilder::new(0x06, 0x42);

// Set sequence number
builder.set_seq(1);

// Add TLV fields
builder.add_u8_field(0xC0, 2).unwrap();          // SystemState = Ready
builder.add_f32_field(0x26, -33.8999).unwrap();  // GPS Latitude
builder.add_f32_field(0x27, 151.2093).unwrap();  // GPS Longitude
builder.add_f32_field(0x30, 1.5).unwrap();       // IMU Roll
builder.add_u32_field(0x82, 3600).unwrap();      // SystemUptime

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
| `field_count()`                 | Number of fields added                 |

---

## FFI Usage (C/C++)

```c
#include "protocol_ffi.h"

TLVMessage msg;
bythos_init(&msg, 0x06, 0x42);

// Add fields
bythos_add_tlv_uint8(&msg, 0xC0, 2);
bythos_add_tlv_float(&msg, 0x26, -33.8999f);
bythos_add_tlv_float(&msg, 0x27, 151.2093f);

// Set sequence
bythos_set_seq(&msg, 1);

// Serialize
uint8_t buffer[1098];
ssize_t size = bythos_build_message(&msg, 0x11, buffer, sizeof(buffer));
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
