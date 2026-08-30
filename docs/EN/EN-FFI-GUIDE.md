# Bythos v3.0.0 — FFI Guide (C/C++)

Author: ShegaPT | License: GPL-3.0

## Overview

The `bythos` library provides a complete FFI API for C/C++, enabling building, serializing, validating, and parsing Bythos v3.0.0 messages without writing Rust.

---

## Header

Include `bythos.h`:

```c
#include "bythos.h"
```

---

## Message Construction

```c
BythosMessage msg;
bythos_init(&msg, 0x06, 0x11);  // node_id=0x06, msg_id=0x11 (Telemetry)

bythos_set_seq(&msg, 1);

bythos_tlv_add_u8(&msg, 0xC0, 2);        // SystemState
bythos_tlv_add_f32(&msg, 0x26, -33.9f);   // GPS Latitude
bythos_tlv_add_f32(&msg, 0x27, 151.2f);   // GPS Longitude

uint8_t buffer[1098];
bythos_ssize_t size = bythos_build(&msg, 0x11, 0x42, buffer, sizeof(buffer));
//                                                          ^^^
//                                              signature_key = 0x42
```

---

## Validation

```c
uint8_t buffer[] = { ... };  // received data

uint8_t result = bythos_validate(buffer, sizeof(buffer));
if (result != 0xFF) {
    // Valid message (result = tlv_count)
} else {
    // Validation error
}
```

---

## Parsing (Byte-by-byte reception)

```c
BythosParser parser;
bythos_parser_init(&parser, 0x42);

for (int i = 0; i < len; i++) {
    BythosParserResult result = bythos_parser_feed(&parser, data[i]);
    if (result == PARSER_OK_MSG) {
        BythosMessage* msg = bythos_parser_get_message(&parser);
        // Process msg...
        bythos_parser_reset(&parser);
    }
}
```

---

## Available FFI Functions

### Protocol

| Function                                            | Description                                    |
|-----------------------------------------------------|----------------------------------------------|
| `bythos_init(msg, node_id, msg_id)`                 | Initialize message                            |
| `bythos_set_seq(msg, seq)`                          | Set sequence number                           |
| `bythos_tlv_add_f32(msg, id, v)`                    | Add f32 field                                 |
| `bythos_tlv_add_i32(msg, id, v)`                    | Add i32 field                                 |
| `bythos_tlv_add_u32(msg, id, v)`                    | Add u32 field                                 |
| `bythos_tlv_add_u16(msg, id, v)`                    | Add u16 field                                 |
| `bythos_tlv_add_u8(msg, id, v)`                     | Add u8 field                                  |
| `bythos_tlv_add(msg, id, data, len)`                | Add raw field                                 |
| `bythos_build(msg, msg_id, key, buf, buf_size)`     | Serialize message                             |
| `bythos_validate(buf, len)`                         | Validate received message                     |
| `bythos_clear(msg)`                                 | Clear message                                 |

### Parser

| Function                                            | Description                                    |
|-----------------------------------------------------|----------------------------------------------|
| `bythos_parser_init(p, key)`                        | Initialize parser                             |
| `bythos_parser_feed(p, byte)`                       | Feed a byte                                   |
| `bythos_parser_get_message(p)`                      | Get complete message                          |
| `bythos_parser_has_message(p)`                      | Check if message is available                 |
| `bythos_parser_reset(p)`                            | Reset parser                                  |

### CRC

| Function                                            | Description                                    |
|-----------------------------------------------------|----------------------------------------------|
| `bythos_calc_crc8(data, len)`                       | Calculate CRC-8/SMBUS                         |
| `bythos_calc_crc16(data, len)`                      | Calculate CRC-16/CCITT                        |

### CAN ID

| Function                                            | Description                                    |
|-----------------------------------------------------|----------------------------------------------|
| `bythos_can_id_make(p, src, dst, type)`             | Build extended CAN ID                         |
| `bythos_can_id_priority(id)`                        | Extract priority                              |
| `bythos_can_id_src(id)`                             | Extract source group                          |
| `bythos_can_id_dst(id)`                             | Extract destination group                     |
| `bythos_can_id_type(id)`                            | Extract message type                          |
| `bythos_is_safety_bus(can_id)`                      | Check if safety bus                           |

### FieldID

| Function                                            | Description                                    |
|-----------------------------------------------------|----------------------------------------------|
| `bythos_field_id_encode(type, id)`                  | Encode FieldID                                |
| `bythos_field_id_decode(fid, type_out, id_out)`     | Decode FieldID                                |
| `bythos_field_id_valid(fid)`                        | Check if FieldID is valid                     |

### Signature

| Function                                            | Description                                    |
|-----------------------------------------------------|----------------------------------------------|
| `bythos_signature_compute(key, msg_id, lo, hi)`     | Calculate signature                           |
| `bythos_validate_signature(sig, key, msg_id, lo, hi)` | Validate signature                        |

### Utilities

| Function                                            | Description                                    |
|-----------------------------------------------------|----------------------------------------------|
| `bythos_msg_id_valid(id)`                           | Check if MsgID is valid                       |
| `bythos_version()`                                  | Protocol version (string)                     |
| `bythos_overhead()`                                 | Overhead size (10 bytes)                      |
| `bythos_max_message_size()`                         | Max message size (1098 bytes)                 |

---

## Linking

### Linux (static compilation)

```bash
gcc -o my_program my_program.c -L. -lbythos -lm
```

### ESP-IDF (ESP32)

Add to `CMakeLists.txt`:
```cmake
idf_component_register(SRCS "main.c"
                       INCLUDE_DIRS "."
                       REQUIRES bythos)
```
