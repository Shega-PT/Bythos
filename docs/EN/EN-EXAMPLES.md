# Bythos v3.0.0 — Examples

Author: ShegaPT | License: GPL-3.0

## Example 1: Build and Validate Message

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::validate_message;

let mut builder = TLVBuilder::new(0x06, 0x42);
builder.set_seq(1);

builder.add_u8_field(0, 2).unwrap();        // SystemState = Ready (ID=0 → 0xC0)
builder.add_f32_field(6, -33.8999).unwrap(); // GPS Latitude (ID=6 → 0x26)
builder.add_f32_field(7, 151.2093).unwrap(); // GPS Longitude (ID=7 → 0x27)
builder.add_f32_field(16, 1.5).unwrap();     // IMU Roll (ID=16 → 0x30)
builder.add_u32_field(2, 3600).unwrap();     // SystemUptime (ID=2 → 0x82)

let mut buffer = [0u8; 1098];
let size = builder.build(0x11, &mut buffer).unwrap();

// Validate
assert!(validate_message(&buffer[..size]).is_ok());
```

## Example 2: Byte-by-Byte Parser

```rust
use bythos::parser::fsm::{Parser, ParserError};

let mut parser = Parser::new(0x42);

for &byte in &buffer[..size] {
    match parser.feed(byte) {
        ParserError::Ok => {}
        ParserError::ErrChecksum => eprintln!("Invalid CRC!"),
        _ => {}
    }
}

if parser.has_message() {
    let msg = parser.get_message();
    println!("Received message: MSG_ID=0x{:02X}", msg.msg_id);
}
```

## Example 3: Complete Roundtrip

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::validate_message;
use bythos::parser::fsm::Parser;

// 1. Build
let mut builder = TLVBuilder::new(0x06, 0x42);
builder.set_seq(42);
builder.add_u8_field(0, 2).unwrap();
builder.add_f32_field(16, 1.5).unwrap();

let mut buffer = [0u8; 1098];
let size = builder.build(0x11, &mut buffer).unwrap();

// 2. Validate
assert!(validate_message(&buffer[..size]).is_ok());

// 3. Parse
let mut parser = Parser::new(0x42);
for &byte in &buffer[..size] {
    parser.feed(byte);
}
assert!(parser.has_message());
```

## Example 4: CAN ID

```rust
use bythos::protocol::types::*;

// Build CAN ID for Device5 → broadcast
let can_id = make_can_id(
    PriorityLevel::High as u8,
    CanGroup::Device5 as u8,
    CanGroup::None as u8,
    CanMsgType::Data as u8,
);

assert_eq!(can_id_priority(can_id), PriorityLevel::High as u8);
assert_eq!(can_id_src_group(can_id), CanGroup::Device5 as u8);

// Safety bus
let safety_id = make_can_id(
    PriorityLevel::SuperCritical as u8,
    CanGroup::Device3 as u8,
    CanGroup::Device4 as u8,
    CanMsgType::Safety as u8,
);
assert!(is_safety_bus_id(safety_id));
```

## Example 5: FFI (C)

```c
#include "bythos.h"
#include <stdio.h>

int main() {
    BythosMessage msg;
    bythos_init(&msg, 0x06, 0x11);  // node_id=0x06, msg_id=0x11 (Telemetry)
    bythos_set_seq(&msg, 1);

    bythos_tlv_add_u8(&msg, 0xC0, 2);
    bythos_tlv_add_f32(&msg, 0x26, -33.9f);
    bythos_tlv_add_f32(&msg, 0x27, 151.2f);

    uint8_t buffer[1098];
    bythos_ssize_t size = bythos_build(&msg, 0x11, 0x42, buffer, sizeof(buffer));

    if (size > 0) {
        printf("Message built: %zd bytes\n", size);

        // Validate
        uint8_t result = bythos_validate(buffer, size);
        printf("Validation: %s\n", result != 0xFF ? "OK" : "ERROR");
    }

    return 0;
}
```

## Example 6: FieldID Encoding

```rust
use bythos::protocol::types::{field_id_encode, field_id_decode, FieldType};

// Encode: type=Float32(1), id=6 → GPS Latitude
let field_id = field_id_encode(FieldType::Float32 as u8, 6);
assert_eq!(field_id, 0x26);

// Decode
let (tipo, id) = field_id_decode(0x26);
assert_eq!(tipo, 1); // Float32
assert_eq!(id, 6);
```
