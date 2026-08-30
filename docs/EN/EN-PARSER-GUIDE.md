# Bythos v3.0.0 — FSM Parser Guide

## Overview

The FSM (Finite State Machine) parser reconstructs Bythos messages from a byte stream, processing each byte individually. It is ideal for byte-by-byte reception on CAN bus or Serial.

---

## FSM States

```text
    ┌──────────────┐
    │  WaitStart   │ ← Initial state
    └──────┬───────┘
           │ byte == 0xAA
           ▼
    ┌──────────────┐
    │  WaitHeader  │ ← version, nodeId, msgId, seqLo, seqHi
    └──────┬───────┘
           │ 5 bytes read
           ▼
    ┌──────────────┐
    │ WaitTlvCount │ ← TLV_COUNT
    └──────┬───────┘
           │ 1 byte read
           ▼
    ┌──────────────┐
    │   WaitTlvId  │ ← FIELD_ID
    └──────┬───────┘
           │ 1 byte read
           ▼
    ┌──────────────┐
    │  WaitTlvLen  │ ← LEN
    └──────┬───────┘
           │ 1 byte read
           ▼
    ┌──────────────┐
    │  WaitTlvData │ ← DATA[0..LEN-1]
    └──────┬───────┘
           │ LEN bytes read
           │ (or LEN==0 → directly to Signature)
           ▼
    ┌──────────────┐
    │WaitSignature │ ← SIGNATURE
    └──────┬───────┘
           │ 1 byte read
           ▼
    ┌──────────────┐
    │  WaitCrc16Lo │ ← CRC16_LO
    └──────┬───────┘
           │ 1 byte read
           ▼
    ┌──────────────┐
    │  WaitCrc16Hi │ ← CRC16_HI
    └──────┬───────┘
           │ 1 byte read → VALIDATE CRC
           ▼
    ┌──────────────┐
    │  Message     │ → parser.has_message() == true
    │  Complete    │
    └──────────────┘
```

---

## Rust Usage

```rust
use bythos::parser::fsm::{Parser, ParserError};

let mut parser = Parser::new(0x42); // signature_key = 0x42

// Feed byte by byte
for &byte in &message_bytes {
    match parser.feed(byte) {
        ParserError::Ok => continue,
        ParserError::ErrStart => { /* invalid byte, restart */ }
        ParserError::ErrCrc => { /* invalid CRC, corrupted message */ }
        _ => { /* other error */ }
    }
}

// Check if complete message is available
if parser.has_message() {
    let msg = parser.get_message();
    // Process msg...
}
```

---

## FFI Usage (C/C++)

```c
#include "protocol_ffi.h"

BythosParser parser;
bythos_parser_init(&parser, 0x42);

for (int i = 0; i < len; i++) {
    BythosParserResult result = bythos_parser_feed(&parser, data[i]);
    if (result == PARSER_OK_MSG) {
        TLVMessage* msg = bythos_parser_get_message(&parser);
        // Process msg...
    }
}
```

---

## Parser Errors

| Error         | Code | Description                         |
|---------------|:----:|-------------------------------------|
| Ok            | 0    | Successful operation                |
| ErrStart      | 1    | Invalid start byte                  |
| ErrVersion    | 2    | Incompatible version                |
| ErrMsgId      | 3    | Invalid message ID                  |
| ErrTlvCount   | 4    | Invalid TLV count                   |
| ErrTlvId      | 5    | Invalid TLV field ID                |
| ErrTlvLen     | 6    | Invalid TLV field length            |
| ErrCrc        | 7    | Invalid CRC16 checksum              |
| ErrBufferFull | 8    | Internal buffer full                |
| ErrSignature  | 9    | Invalid signature                   |

---

## Validation

After reconstructing the message, the parser validates:
1. **CRC-16/CCITT** — data integrity
2. **Signature** — authenticity (if key != 0x00)

If validation fails, `ParserError::ErrCrc` or `ParserError::ErrSignature` is returned.
