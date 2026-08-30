# Bythos Protocol v3.0.0

**Generic TLV protocol library — Rust + C**

Bythos is a generic communication protocol based on TLV (Type-Length-Value), designed for embedded systems, unmanned aerial vehicles (UAV/UAS), and CAN networks. Independent implementations in Rust and C, with FFI interoperability.

---

## Features

- **Flexible TLV** — dynamic field types (u8, u16, u32, f32, raw)
- **CRC-16/CCITT** — data integrity with pre-calculated lookup table
- **XOR signature** — fast message authentication
- **Extended CAN ID** — 29-bit with priority, src/dst groups, and message type
- **FSM Parser** — byte-by-byte state machine parser
- **C FFI** — complete API for Rust↔C interoperability
- **`no_std`** — support for environments without standard library
- **Feature flags** — `std`, `transport-uart`, `transport-lora`, `transport-spi`, `transport-i2c`, `transport-can`, `esp`

---

## Structure

```
ACP/
├── Cargo.toml              # bythos crate
├── src/
│   ├── lib.rs              # Crate root
│   ├── protocol/
│   │   ├── types.rs        # Constants, enums, FieldId, CAN ID
│   │   ├── codec.rs        # TLV serialization/deserialization
│   │   ├── builder.rs      # Message builder
│   │   ├── crc8.rs         # CRC-8/SMBUS
│   │   ├── crc16.rs        # CRC-16/CCITT
│   │   ├── ffi.rs          # Rust FFI (bythos_*)
│   │   └── mod.rs          # Protocol module
│   └── parser/
│       ├── fsm.rs          # FSM parser (byte-by-byte)
│       ├── ffi.rs          # Parser FFI (bythos_parser_*)
│       └── mod.rs          # Parser module
├── tests/
│   └── test_acp.rs         # 42 integration tests
├── c_core/
│   ├── include/bythos.h    # C header
│   ├── src/bythos.c        # Standalone C implementation
│   ├── tests/test_bythos.c # 21 unit tests
│   ├── benchmark/          # Microbenchmarks
│   └── CMakeLists.txt      # CMake build
├── docs/                   # Documentation (PT-PT)
│   └── EN/                 # English documentation
├── .gitignore
├── README.md               # PT-PT version
├── README-EN.md            # This file (English)
└── CHANGELOG.md            # Version history
```

---

## Quick Start

### Rust

```rust
use bythos::protocol::types::*;
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::{build_message, validate_message};
use bythos::parser::fsm::Parser;

fn main() {
    // Build message
    let mut builder = TLVBuilder::new(0x06, MsgId::Telemetry as u8);
    builder.set_seq(1);
    builder.add_u8(FieldId::encode(FieldType::System, 0), 4);
    builder.add_f32(FieldId::encode(FieldType::Gps, 6), 40.0);

    let mut buffer = [0u8; BYTHOS_MAX_MESSAGE_SIZE];
    let size = build_message(&builder.msg, 0x42, &mut buffer).unwrap();

    // Validate
    let tlv_count = validate_message(&buffer[..size]).unwrap();

    // Parse byte-by-byte
    let mut parser = Parser::new(0x42);
    for &byte in &buffer[..size] {
        if parser.feed(byte) == ParserError::Ok && parser.has_message() {
            let msg = parser.copy_message();
            println!("Received message with {} TLVs", msg.tlv_count);
        }
    }
}
```

### C

```c
#include "bythos.h"
#include <stdio.h>

int main() {
    BythosMessage msg;
    bythos_init(&msg, 0x06, BYTHOS_MSG_TELEMETRY);
    bythos_set_seq(&msg, 1);
    bythos_tlv_add_u8(&msg, 0xE0, 4);

    uint8_t buffer[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t size = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, 0x42, buffer, sizeof(buffer));

    if (bythos_validate(buffer, size) != 0xFF) {
        printf("Valid message with %d bytes\n", size);
    }
    return 0;
}
```

---

## Build

### Rust

```bash
cargo build              # Standard build
cargo test               # 135 tests (89 unit + 42 integration + 4 doc)
cargo clippy             # Linting (0 warnings)
```

### C

```bash
cd c_core
mkdir build && cd build
cmake ..
make
./test_bythos            # 21 tests
./bench_bythos           # Microbenchmarks
```

---

## Message Format

```
| START (1B) | VER (1B) | NODE (1B) | MSG_ID (1B) | SEQ (2B) | TLV_COUNT (1B) | TLV... | SIG (1B) | CRC16 (2B) |
```

- **Header**: 7 bytes
- **Signature**: 1 byte (XOR key ⊕ msg_id ⊕ seq_lo ⊕ seq_hi)
- **CRC16**: 2 bytes (CCITT-FALSE)
- **Total overhead**: 10 bytes

---

## Extended CAN ID (29-bit)

```
Bits 28-26: Priority (3 bits)
Bits 25-22: Source group (4 bits)
Bits 21-18: Destination group (4 bits)
Bits 17-14: Message type (4 bits)
Bits 13-0:  Reserved (14 bits)
```

---

## License

GPL-3.0 — see [LICENSE](LICENSE)

**Author**: ShegaPT
