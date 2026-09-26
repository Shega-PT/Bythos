# Bythos Protocol v4.0.0

**Hardware-secured ring protocol — Rust + C**

Bythos V4: one identical PCB per node (`BythosBridge`), a closed ring over 2 twisted pairs (data + clock), automatic enumeration from `BG-0`, 16-bit addresses with no practical limit, and real authentication (4-byte HMAC-SHA256 `TAG` in an ATECC608 secure element). Radio-agnostic via an opaque tunnel. Independent Rust and C implementations with a shared golden vector.

---

## Features

- **Bidirectional ring** — shortest-path routing, `BG-27 → BG-2` without looping
- **Bythos Bus** — CAN-style arbitration (lowest ID wins), RJ jack, no controller — see `docs/BYTHOS-BUS.md`
- **Auto-enumeration** — only `BG-0` has a fixed address; the rest number themselves
- **4-byte TAG + anti-replay** — truncated HMAC-SHA256, 24-bit `CTR`, window of 64
- **Secure element** — keys in the ATECC608 (software path for tests only)
- **f16 and bool everywhere** — including the C FFI (V3 debt closed)
- **Fragmented video** — 32 B per chunk, heapless reassembly + COBS for radios
- **FSM parser** — 10 states, real bounds, real timeout, no keys
- **`no_std` by feature** — `cargo build --no-default-features` proves it on PC
- **Rust↔C interop** — golden vector frozen on both sides

---

## Layout

```
├── Cargo.toml              # bythos crate v4.0.0
├── src/
│   ├── lib.rs              # Root (compiling example in docs)
│   ├── protocol/
│   │   ├── types.rs        # Constants, addresses, BythosFieldId catalog
│   │   ├── secure.rs       # SHA-256 + HMAC + seal + anti-replay
│   │   ├── builder.rs      # BythosBuilder (fluent and sealed)
│   │   ├── codec.rs        # Wire: build, peek, validate, parse
│   │   ├── crc8.rs         # Legacy CRC-8 (migration)
│   │   ├── crc16.rs        # CRC-16/CCITT
│   │   ├── ffi.rs          # C ABI (bythos_*)
│   │   └── mod.rs
│   ├── parser/
│   │   ├── fsm.rs          # Byte-by-byte parser (no keys)
│   │   ├── ffi.rs          # Parser FFI (bythos_parser_*)
│   │   └── mod.rs
│   ├── ring.rs             # Enumeration + shortest path + supervision
│   └── tunnel.rs           # COBS + fragmented video reassembly
├── tests/
│   ├── test_bythos.rs      # 14 end-to-end tests
│   └── test_ring.rs        # 4 ring tests (5 nodes, cut, BG-27→BG-2)
├── c_core/                 # Standalone C (SHA-256+HMAC included)
├── hardware/bythos-bridge/ # PCB: description, BOM, bring-up
├── docs/
│   ├── BYTHOS-SPECIFICATION.md # Wire, ring and security norm
│   ├── BYTHOS-BUS.md           # The bus (product guide)
│   ├── MIGRATION-GUIDE.md      # V3 → V4 migration
│   ├── THREAT-MODEL.md         # Threat model
│   └── ...                 # V3 guides (historical, see migration)
└── CHANGELOG.md
```

---

## Quick Start

### Rust

```rust
use bythos::protocol::builder::BythosBuilder;
use bythos::protocol::codec::validate_message;
use bythos::protocol::secure::{PeerTable, SealConfig};
use bythos::parser::fsm::Parser;
use bythos::protocol::types::MAX_MESSAGE_SIZE;

fn main() {
    // Source 27, software key (tests; production: secure element).
    let key = [0x42u8; 32];
    let mut b = BythosBuilder::new(27, SealConfig::software(0, key, 27));
    b.set_seq(7);
    b.add_u8_field(0, 2).unwrap();     // system state
    b.add_f32_field(6, 40.0).unwrap(); // latitude

    let mut buf = [0u8; MAX_MESSAGE_SIZE];
    let n = b.build(0x11, 0xFFFF, &mut buf).unwrap(); // broadcast telemetry

    // Final destination: structure + TAG + replay.
    let mut peers = PeerTable::new();
    let view = validate_message(&buf[..n], &key, &mut peers).unwrap();
    println!("Frame from {} with {} fields", view.header.src, view.header.field_count);

    // Bridge on the path: parses without keys.
    let mut p = Parser::new();
    for &byte in &buf[..n] {
        p.feed(byte);
    }
    assert!(p.has_message());
}
```

### C

```c
#include "bythos.h"
#include <stdio.h>

int main(void) {
    static const uint8_t key[32] = { [0] = 0x42 }; /* trial; production: ATECC608 */
    BythosMessage msg;
    bythos_init(&msg, 27, BYTHOS_MSG_TELEMETRY);
    bythos_set_dst(&msg, BYTHOS_BROADCAST);
    bythos_set_seq(&msg, 7);
    bythos_field_add_u8(&msg, BYTHOS_FIELD_SYSTEM_STATE, 2);

    uint8_t buf[BYTHOS_MAX_MESSAGE_SIZE];
    bythos_ssize_t n = bythos_build(&msg, BYTHOS_MSG_TELEMETRY, BYTHOS_BROADCAST,
                                    0, 41, key, buf, sizeof(buf));

    BythosPeers peers;
    bythos_peers_clear(&peers);
    if (bythos_validate(buf, (size_t)n, key, &peers) != 0xFF) {
        printf("Valid frame with %d bytes\n", (int)n);
    }
    return 0;
}
```

---

## Build

### Rust

```bash
cargo build                        # PC, with std
cargo build --no-default-features  # proves no_std
cargo test                         # 107 tests (85 lib + 18 integration + 4 doc)
cargo clippy --all-targets         # 0 warnings
cargo fmt --check                  # clean
```

### C

```bash
cmake -S c_core -B build && cmake --build build
./build/test_bythos   # 13 groups, incl. Rust↔C golden vector
./build/bench_bythos  # microbenchmarks (TAG ~9 µs in software)
```

---

## Frame Format (normative: `docs/BYTHOS-SPECIFICATION.md`)

```
| START (1B) | VER=0x04 (1B) | SRC (2B) | DST (2B) | MSG (1B) |
| SEQ (2B) | FIELD_COUNT (1B) | HOPS (1B) | FIELDS... | SEC_HDR (4B) | TAG (4B) | CRC16 (2B) |
```

- **Header**: 11 bytes · **Trailer**: 10 bytes · **Overhead**: 21 bytes
- **Maximum**: 1205 bytes (31 normal fields + 1 video)
- **Golden vector**: `AA040600FFFF1101000120C001020029000082234BEF6248`

---

## License

GPL-3.0 — see [LICENSE](LICENSE)

**Author**: ShegaPT
