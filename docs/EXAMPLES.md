# Bythos v3.0.0 — Exemplos

## Exemplo 1: Construir e Validar Mensagem

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::validate_message;

let mut builder = TLVBuilder::new(0x06, 0x42);
builder.set_seq(1);

builder.add_u8_field(0xC0, 2).unwrap();        // SystemState = Ready
builder.add_f32_field(0x26, -33.8999).unwrap(); // GPS Latitude
builder.add_f32_field(0x27, 151.2093).unwrap(); // GPS Longitude
builder.add_f32_field(0x30, 1.5).unwrap();      // IMU Roll
builder.add_u32_field(0x82, 3600).unwrap();     // SystemUptime

let mut buffer = [0u8; 1098];
let size = builder.build(0x11, &mut buffer).unwrap();

// Validar
assert!(validate_message(&buffer[..size]).is_ok());
```

## Exemplo 2: Parser Byte a Byte

```rust
use bythos::parser::fsm::{Parser, ParserError};

let mut parser = Parser::new(0x42);

for &byte in &buffer[..size] {
    match parser.feed(byte) {
        ParserError::Ok => {}
        ParserError::ErrCrc => eprintln!("CRC inválido!"),
        _ => {}
    }
}

if parser.has_message() {
    let msg = parser.get_message();
    println!("Mensagem recebida: MSG_ID=0x{:02X}", msg.msg_id);
}
```

## Exemplo 3: Roundtrip Completo

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::validate_message;
use bythos::parser::fsm::Parser;

// 1. Construir
let mut builder = TLVBuilder::new(0x06, 0x42);
builder.set_seq(42);
builder.add_u8_field(0xC0, 2).unwrap();
builder.add_f32_field(0x30, 1.5).unwrap();

let mut buffer = [0u8; 1098];
let size = builder.build(0x11, &mut buffer).unwrap();

// 2. Validar
assert!(validate_message(&buffer[..size]).is_ok());

// 3. Parse
let mut parser = Parser::new(0x42);
for &byte in &buffer[..size] {
    parser.feed(byte);
}
assert!(parser.has_message());
```

## Exemplo 4: CAN ID

```rust
use bythos::protocol::types::*;

// Construir CAN ID para Device5 → broadcast
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

## Exemplo 5: FFI (C)

```c
#include "protocol_ffi.h"
#include <stdio.h>

int main() {
    TLVMessage msg;
    bythos_init(&msg, 0x06, 0x42);
    bythos_set_seq(&msg, 1);

    bythos_add_tlv_uint8(&msg, 0xC0, 2);
    bythos_add_tlv_float(&msg, 0x26, -33.9f);
    bythos_add_tlv_float(&msg, 0x27, 151.2f);

    uint8_t buffer[1098];
    ssize_t size = bythos_build_message(&msg, 0x11, buffer, sizeof(buffer));

    if (size > 0) {
        printf("Mensagem construída: %zd bytes\n", size);

        // Validar
        uint8_t result = bythos_validate_message(buffer, size, 0x42);
        printf("Validação: %s\n", result == 0 ? "OK" : "ERRO");
    }

    bythos_free_message(&msg);
    return 0;
}
```

## Exemplo 6: FieldID Encoding

```rust
use bythos::protocol::types::*;

// Codificar: tipo=f32(1), id=6 → GPS Latitude
let field_id = FieldId::encode(FieldType::Gps, 6);
assert_eq!(field_id, 0x26);

// Decodificar
let (tipo, id) = FieldId::decode(0x26);
assert_eq!(tipo, FieldType::Gps);
assert_eq!(id, 6);
```
