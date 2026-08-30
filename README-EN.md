# Bythos Protocol v3.0.0

**Generic TLV protocol library — Rust + C**

Bythos is a generic binary communication protocol based on TLV (Type-Length-Value), designed for embedded systems, unmanned aerial vehicles (UAV/UAS), and CAN networks. Independent implementations in Rust and C, with FFI interoperability.

---

## Features

- **FieldID with embedded type** — `[TYPE:3][ID:5]` — 8 types (Raw, Float32, Float16, Int32, Uint32, Uint16, Uint8, Bool) × 32 IDs = 60+ fields
- **12 message types** — Heartbeat, Telemetry, Command, Ack, Failsafe, Debug, Video, Shell, SiData, Watchdog, Ping, Clock
- **CRC-16/CCITT** — data integrity with pre-calculated lookup table
- **XOR signature** — fast message authentication
- **Extended CAN ID** — 29-bit with 5 priority levels, 14 src/dst groups, and 8 message types
- **FSM Parser** — 9-state byte-by-byte state machine parser with configurable timeout via timestamp injection
- **Complete C FFI** — identical API in Rust and C (`bythos_*`, `bythos_parser_*`)
- **`no_std`** — support for environments without standard library
- **Feature flags** — `std`, `transport-uart`, `transport-lora`, `transport-spi`, `transport-i2c`, `transport-can`, `esp`

---

## Structure

```text
Bythos/
├── Cargo.toml              # bythos crate
├── src/
│   ├── lib.rs              # Crate root
│   ├── protocol/
│   │   ├── types.rs        # Constants, enums, FieldId, CAN ID
│   │   ├── codec.rs        # TLV serialization/deserialization
│   │   ├── builder.rs      # Message builder (TLVBuilder)
│   │   ├── crc8.rs         # CRC-8/SMBUS
│   │   ├── crc16.rs        # CRC-16/CCITT
│   │   ├── ffi.rs          # Rust FFI (bythos_*)
│   │   └── mod.rs          # Protocol module
│   └── parser/
│       ├── fsm.rs          # FSM parser (9 states, byte-by-byte)
│       ├── ffi.rs          # Parser FFI (bythos_parser_*)
│       └── mod.rs          # Parser module
├── tests/
│   └── test_bythos.rs      # 42 integration tests
├── c_core/
│   ├── include/bythos.h    # Complete C header
│   ├── src/bythos.c        # Standalone C implementation
│   ├── tests/test_bythos.c # 21 unit tests
│   ├── benchmark/          # C microbenchmarks
│   └── CMakeLists.txt      # CMake build
├── docs/                   # Documentation (PT-PT)
│   └── EN/                 # English documentation
├── .gitignore
├── README.md               # PT-PT version
├── README-EN.md            # This file (English)
├── CHANGELOG.md            # Version history
└── LICENSE                 # GPL-3.0
```

---

## Quick Start

### Rust

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::validate_message;
use bythos::parser::fsm::Parser;

fn main() {
    // Build message
    let mut builder = TLVBuilder::new(0x06, 0x42);
    builder.set_seq(1);
    builder.add_u8_field(0, 4).unwrap();        // SystemState (ID=0 → 0xC0)
    builder.add_f32_field(6, 40.0).unwrap();    // GPS Latitude (ID=6 → 0x26)

    let mut buffer = [0u8; 1098];
    let size = builder.build(0x11, &mut buffer).unwrap();

    // Validate
    let _tlv_count = validate_message(&buffer[..size]).unwrap();

    // Parse byte-by-byte
    let mut parser = Parser::new(0x42);
    for &byte in &buffer[..size] {
        if parser.feed(byte) == bythos::parser::fsm::ParserError::Ok && parser.has_message() {
            let msg = parser.get_message();
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
    bythos_tlv_add_u8(&msg, BYTHOS_FIELD_SYSTEM_STATE, 4);

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
# On host (Linux/macOS) — use RUSTUP_TOOLCHAIN=stable since rust-toolchain.toml uses "esp" channel
RUSTUP_TOOLCHAIN=stable cargo build
RUSTUP_TOOLCHAIN=stable cargo test
RUSTUP_TOOLCHAIN=stable cargo clippy

# On ESP32 — "esp" toolchain must be installed
cargo build --release --target xtensa-esp32-none-elf
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

```text
| START (1B) | VER (1B) | NODE (1B) | MSG_ID (1B) | SEQ (2B) | TLV_COUNT (1B) | TLV... | SIG (1B) | CRC16 (2B) |
```

- **Header**: 7 bytes
- **Signature**: 1 byte (XOR key ⊕ msg_id ⊕ seq_lo ⊕ seq_hi)
- **CRC16**: 2 bytes (CCITT-FALSE)
- **Total overhead**: 10 bytes

### FieldID with Embedded Type

```text
FieldID = [TYPE:3][ID:5]

  Bits 7-5: Data type (0-7)
  Bits 4-0: Field ID (0-31)
```

Example: `0x26` = TYPE=1 (Float32) + ID=6 (GPS Latitude)

### 8 Data Types

| Type | Value | Size | Example |
|------|:-----:|:----:|---------|
| Raw | 0 | Variable | Video payload |
| Float32 | 1 | 4B | GPS, IMU, Flight |
| Float16 | 2 | 2B | Reduced sensors |
| Int32 | 3 | 4B | Signed values |
| Uint32 | 4 | 4B | Uptime, free heap |
| Uint16 | 5 | 2B | Frame ID, counters |
| Uint8 | 6 | 1B | State, flags |
| Bool | 7 | 1B | True/false |

### 12 Message Types (MsgID)

| MsgID | Name | Value | Default Priority |
|:-----:|------|:-----:|:----------------:|
| 0x10 | Heartbeat | 16 | Low |
| 0x11 | Telemetry | 17 | Medium |
| 0x12 | Command | 18 | Medium |
| 0x13 | Ack | 19 | Medium |
| 0x14 | Failsafe | 20 | Critical |
| 0x15 | Debug | 21 | Low (SuperCritical in failsafe) |
| 0x16 | Video | 22 | Low |
| 0x17 | Shell | 23 | Medium |
| 0x18 | SiData | 24 | Medium |
| 0x19 | Watchdog | 25 | High |
| 0x1A | Ping | 26 | Low |
| 0x1B | Clock | 27 | Medium |

---

## Extended CAN ID (29-bit)

```text
Bits 28-26: Priority (3 bits) — 5 levels
Bits 25-22: Source group (4 bits) — 14 groups (Device0-Device14)
Bits 21-18: Destination group (4 bits) — 0x0 = broadcast
Bits 17-14: Message type (4 bits) — 8 types (Data, Cmd, Ack, Event, Sync, State, Heart, Safety)
Bits 13-0:  Reserved (14 bits)
```

---

## FSM Parser

9 states: `WaitStart` → `WaitHeader` → `WaitTlvCount` → `WaitTlvId` → `WaitTlvLen` → `WaitTlvData` → `WaitSignature` → `WaitCrc16Lo` → `WaitCrc16Hi`

- **Configurable timeout** via `Parser::with_timestamp_fn()` (timestamp injection)
- **Success/error counters** per parser
- **Persistent last_error** for diagnostics
- **Protections**: overflow, timeout, invalid CRC, invalid signature

---

## Tests

```text
Rust:   92 unit + 42 integration + 4 doc-tests = 138 total
C:      21 unit + microbenchmarks
Total:  159 tests
```

---

## Security Notice

The default signature key (`0x00`) provides no authentication — any device can send legitimate messages.
For production use, configure your own signature key via `set_key()` (Rust) or `bythos_init()` (C).
The Bythos signature is a 1-byte XOR integrity check, not a cryptographic signature.

---

## Documentation

| File | Description |
|------|-------------|
| [docs/EN/EN-BYTHOS-SPECIFICATION.md](docs/EN/EN-BYTHOS-SPECIFICATION.md) | Full protocol specification |
| [docs/EN/EN-BUILDER-GUIDE.md](docs/EN/EN-BUILDER-GUIDE.md) | Message builder guide |
| [docs/EN/EN-PARSER-GUIDE.md](docs/EN/EN-PARSER-GUIDE.md) | FSM parser guide |
| [docs/EN/EN-FFI-GUIDE.md](docs/EN/EN-FFI-GUIDE.md) | C FFI API guide |
| [docs/EN/EN-FIELD-ID-REFERENCE.md](docs/EN/EN-FIELD-ID-REFERENCE.md) | FieldID reference |
| [docs/EN/EN-MESSAGE-ID-REFERENCE.md](docs/EN/EN-MESSAGE-ID-REFERENCE.md) | MsgID reference |
| [docs/EN/EN-CAN-ID-MAPPING.md](docs/EN/EN-CAN-ID-MAPPING.md) | CAN ID mapping |
| [docs/EN/EN-EXAMPLES.md](docs/EN/EN-EXAMPLES.md) | Usage examples |
| [docs/EN/EN-DEVELOPER-TIPS.md](docs/EN/EN-DEVELOPER-TIPS.md) | Developer tips |
| [docs/](docs/) | Documentação PT-PT |

---

## License

GPL-3.0 — see [LICENSE](LICENSE)

**Author**: ShegaPT
