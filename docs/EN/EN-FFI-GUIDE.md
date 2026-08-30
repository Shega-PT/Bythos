# Bythos v3.0.0 — FFI Guide (C/C++)

## Overview

The `bythos` library provides a complete FFI API for C/C++, enabling building, serializing, validating, and parsing Bythos v3.0.0 messages without writing Rust.

---

## Header

Include `protocol_ffi.h`:

```c
#include "protocol_ffi.h"
```

---

## Message Construction

```c
TLVMessage msg;
bythos_init(&msg, 0x06, 0x42);  // node_id=0x06, key=0x42

bythos_set_seq(&msg, 1);

bythos_add_tlv_uint8(&msg, 0xC0, 2);        // SystemState
bythos_add_tlv_float(&msg, 0x26, -33.9f);    // GPS Latitude
bythos_add_tlv_float(&msg, 0x27, 151.2f);    // GPS Longitude

uint8_t buffer[1098];
ssize_t size = bythos_build_message(&msg, 0x11, buffer, sizeof(buffer));
```

---

## Validation

```c
uint8_t buffer[] = { ... };  // received data
uint8_t key = 0x42;

uint8_t result = bythos_validate_message(buffer, sizeof(buffer), key);
if (result == 0) {
    // Valid message
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
        TLVMessage* msg = bythos_parser_get_message(&parser);
        // Process msg...
        bythos_parser_reset(&parser);
    }
}
```

---

## Available FFI Functions

### Protocol

| Function                        | Description                                    |
|---------------------------------|----------------------------------------------|
| `bythos_init(msg, node, key)`  | Initialize message                            |
| `bythos_set_seq(msg, seq)`    | Set sequence number                           |
| `bythos_add_tlv_f32(msg, id, v)` | Add f32 field                              |
| `bythos_add_tlv_f16(msg, id, v)` | Add f16 field                              |
| `bythos_add_tlv_i32(msg, id, v)` | Add i32 field                              |
| `bythos_add_tlv_u32(msg, id, v)` | Add u32 field                              |
| `bythos_add_tlv_u16(msg, id, v)` | Add u16 field                              |
| `bythos_add_tlv_u8(msg, id, v)`  | Add u8 field                               |
| `bythos_add_tlv_bool(msg, id, v)`| Add bool field                             |
| `bythos_add_tlv_raw(msg, id, d, l)` | Add raw field                         |
| `bythos_build_message(msg, mid, buf, len)` | Serialize message             |
| `bythos_validate_message(buf, len, key)` | Validate received message     |
| `bythos_free_message(msg)`    | Free message memory                           |

### Parser

| Function                        | Description                                    |
|---------------------------------|----------------------------------------------|
| `bythos_parser_init(p, key)`  | Initialize parser                              |
| `bythos_parser_feed(p, byte)` | Feed a byte                                    |
| `bythos_parser_get_message(p)`| Get complete message                           |
| `bythos_parser_has_message(p)`| Check if message is available                  |
| `bythos_parser_reset(p)`      | Reset parser                                   |

### CRC

| Function                        | Description                                    |
|---------------------------------|----------------------------------------------|
| `bythos_calc_crc8(data, len)` | Calculate CRC-8/SMBUS                          |
| `bythos_calc_crc16(data, len)`| Calculate CRC-16/CCITT                         |

### CAN ID

| Function                        | Description                                    |
|---------------------------------|----------------------------------------------|
| `bythos_make_can_id(p, src, dst, type)` | Build extended CAN ID            |
| `bythos_can_id_priority(id)`  | Extract priority                               |
| `bythos_can_id_src_group(id)` | Extract source group                           |
| `bythos_can_id_dst_group(id)` | Extract destination group                      |
| `bythos_can_id_msg_type(id)`  | Extract message type                           |
| `bythos_is_safety_bus_id(id)` | Check if safety bus ID                         |

### FieldID

| Function                        | Description                                    |
|---------------------------------|----------------------------------------------|
| `bythos_field_id_encode(t, id)`| Encode FieldID                                |
| `bythos_field_id_decode(fid)` | Decode FieldID                                 |

### Signature

| Function                        | Description                                    |
|---------------------------------|----------------------------------------------|
| `bythos_compute_signature(k, m, lo, hi)` | Calculate signature                |
| `bythos_validate_signature(s, k, m, lo, hi)` | Validate signature          |

### Utilities

| Function                        | Description                                    |
|---------------------------------|----------------------------------------------|
| `bythos_msg_id_valid(id)`     | Check if MsgID is valid                        |
| `bythos_is_valid_field_id(id)`| Check if FieldID is valid                      |

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
