# Bythos v3.0.0 — Developer Tips

## Compilation

### Host (Linux/macOS — testing and development)

```bash
cd rust/
RUSTUP_TOOLCHAIN=stable cargo test
RUSTUP_TOOLCHAIN=stable cargo check
```

### ESP32 (production)

```bash
cd rust/
cargo build --release --target xtensa-esp32-none-elf
```

**Note:** The `esp` toolchain must be installed. On the host, always use `RUSTUP_TOOLCHAIN=stable`.

---

## Features

| Feature  | Description                                | Usage                |
|----------|--------------------------------------------|----------------------|
| `std`    | Standard library (for tests/FFI)           | Host/FFI             |
| `no-std` | No standard library (bare-metal ESP32)     | ESP32 production     |
| `esp`    | esp-idf-sys integration                   | ESP32 + native FFI   |

### Recommended usage

- **Host testing:** `cargo test` (default, with `std`)
- **ESP32 bare-metal:** `cargo build --no-default-features --no-std`
- **ESP32 with FFI:** `cargo build --features esp`

---

## Wire Format

```text
Offset  Field          Size     Notes
------  -----          ----     -----
0       START_BYTE     1        0xAA
1       VERSION        1        0x03
2       NODE_ID        1        CAN group
3       MSG_ID         1        0x10-0x1B
4-5     SEQ_NUM        2        u16 LE
6       TLV_COUNT      1        0-32
7+      TLV_FIELDS     Var.     [ID][LEN][DATA...]
*       SIGNATURE      1        XOR(key, msg_id, seq_lo, seq_hi)
*-1     CRC16          2        CRC-16/CCITT LE
```

---

## FieldID — The Key to the System

Each TLV field is identified by a 1-byte FieldID that embeds the type:

```text
FieldID = [TYPE:3][ID:5]

Example:
  FieldID 0x26 = type=1(f32), id=6 = GPS Latitude
  FieldID 0xC0 = type=6(u8),  id=0 = SystemState
```

---

## Adding a New Field

1. Choose the data type (f32, u8, etc.)
2. Choose a free ID within the type's range
3. Define the FieldID: `(type << 5) | id`
4. Add to the `FieldId` enum in `types.rs`
5. Document in `FIELD-ID-REFERENCE.md`

---

## Adding a New Message

1. Choose a free MsgID in the range 0x10-0x1B
2. Define the default priority
3. Add to the `MsgId` enum in `types.rs`
4. Add to the match in `get_msg_priority()`
5. Document in `MESSAGE-ID-REFERENCE.md`

---

## Adding a New CAN Group

1. Choose a free value 0x0-0xF
2. Add to the `CanGroup` enum in `types.rs`
3. Update `make_can_id()` if necessary
4. Document in `CAN-ID-MAPPING.md`

---

## Common Errors

### Incorrect CRC
- Verify CRC is calculated over **all preceding bytes** (header + TLV + signature)
- Verify init value is 0xFFFF, not 0x0000
- Verify there is no reflection

### Signature failure
- Verify the key is correct
- Signature is XOR of: `key ^ msg_id ^ seq_lo ^ seq_hi`
- If key=0x00, signature is just `msg_id ^ seq_lo ^ seq_hi`

### Parser doesn't recognize message
- Verify START_BYTE = 0xAA
- Verify VERSION = 0x03
- Verify MSG_ID in range 0x10-0x1B

### FFI doesn't link
- Use `crate-type = ["staticlib", "rlib"]` in Cargo.toml
- Compile with `--release` for optimized size

---

## Project Structure

```text
rust/
├── Cargo.toml          # Crate configuration
├── src/
│   ├── lib.rs          # Crate root (conditional no_std)
│   ├── protocol/
│   │   ├── mod.rs      # Module declarations
│   │   ├── types.rs    # Core types, constants, enums
│   │   ├── crc8.rs     # CRC-8/SMBUS
│   │   ├── crc16.rs    # CRC-16/CCITT
│   │   ├── builder.rs  # TLVBuilder
│   │   ├── codec.rs    # Serialization, validation
│   │   └── ffi.rs      # C FFI bindings
│   └── parser/
│       ├── mod.rs      # Parser module
│       ├── fsm.rs      # 9-state FSM parser
│       └── ffi.rs      # Parser FFI bindings
├── tests/
│   └── test_acp.rs     # Integration tests
└── docs/               # Protocol documentation
    ├── ACP-SPECIFICATION.md
    ├── FIELD-ID-REFERENCE.md
    ├── MESSAGE-ID-REFERENCE.md
    ├── CAN-ID-MAPPING.md
    ├── PARSER-GUIDE.md
    ├── BUILDER-GUIDE.md
    ├── FFI-GUIDE.md
    ├── DEVELOPER-TIPS.md
    └── EXAMPLES.md
```
