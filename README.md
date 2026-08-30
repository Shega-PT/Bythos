# Bythos Protocol v3.0.0

**Biblioteca genérica para protocolos TLV — Rust + C**

Bythos é um protocolo de comunicação genérico baseado em TLV (Type-Length-Value), concebido para sistemas embedded, veículos aéreos não tripulados (UAV/UAS) e redes CAN. Implementações independentes em Rust e C, com interoperabilidade via FFI.

---

## Características

- **TLV flexível** — campos com tipos dinâmicos (u8, u16, u32, f32, raw)
- **CRC-16/CCITT** — integridade de dados com tabela pré-calculada
- **Assinatura XOR** — autenticação rápida de mensagens
- **CAN ID extended** — 29 bits com prioridade, grupo origem/destino e tipo
- **Parser FSM** — máquina de estados para parsing byte-a-byte
- **FFI C** — API completa para interoperabilidade Rust↔C
- **`no_std`** — suporte para ambientes sem biblioteca padrão
- **Feature flags** — `std`, `transport-uart`, `transport-lora`, `transport-spi`, `transport-i2c`, `transport-can`, `esp`

---

## Estrutura

```
ACP/
├── Cargo.toml              # Crate bythos
├── src/
│   ├── lib.rs              # Raiz da crate
│   ├── protocol/
│   │   ├── types.rs        # Constantes, enums, FieldId, CAN ID
│   │   ├── codec.rs        # Serialização/deserialização TLV
│   │   ├── builder.rs      # Construtor de mensagens
│   │   ├── crc8.rs         # CRC-8/SMBUS
│   │   ├── crc16.rs        # CRC-16/CCITT
│   │   ├── ffi.rs          # FFI Rust (bythos_*)
│   │   └── mod.rs          # Módulo protocol
│   └── parser/
│       ├── fsm.rs          # Parser FSM (byte-a-byte)
│       ├── ffi.rs          # FFI parser (bythos_parser_*)
│       └── mod.rs          # Módulo parser
├── tests/
│   └── test_acp.rs         # 42 testes de integração
├── c_core/
│   ├── include/bythos.h    # Header C
│   ├── src/bythos.c        # Implementação C standalone
│   ├── tests/test_bythos.c # 21 testes unitários C
│   ├── benchmark/          # Microbenchmarks
│   └── CMakeLists.txt      # Build CMake
├── docs/                   # Documentação PT-PT
│   └── EN/                 # Documentação English
├── .gitignore
├── README.md               # Este ficheiro (PT-PT)
├── README-EN.md            # English version
└── CHANGELOG.md            # Histórico de versões
```

---

## Início Rápido

### Rust

```rust
use bythos::protocol::types::*;
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::{build_message, validate_message};
use bythos::parser::fsm::Parser;

fn main() {
    // Construir mensagem
    let mut builder = TLVBuilder::new(0x06, MsgId::Telemetry as u8);
    builder.set_seq(1);
    builder.add_u8(FieldId::encode(FieldType::System, 0), 4);
    builder.add_f32(FieldId::encode(FieldType::Gps, 6), 40.0);

    let mut buffer = [0u8; BYTHOS_MAX_MESSAGE_SIZE];
    let size = build_message(&builder.msg, 0x42, &mut buffer).unwrap();

    // Validar
    let tlv_count = validate_message(&buffer[..size]).unwrap();

    // Parse byte-a-byte
    let mut parser = Parser::new(0x42);
    for &byte in &buffer[..size] {
        if parser.feed(byte) == ParserError::Ok && parser.has_message() {
            let msg = parser.copy_message();
            println!("Recebida mensagem com {} TLVs", msg.tlv_count);
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
        printf("Mensagem válida com %d bytes\n", size);
    }
    return 0;
}
```

---

## Build

### Rust

```bash
cargo build              # Build padrão
cargo test               # 135 testes (89 unit + 42 integration + 4 doc)
cargo clippy             # Linting (0 warnings)
```

### C

```bash
cd c_core
mkdir build && cd build
cmake ..
make
./test_bythos            # 21 testes
./bench_bythos           # Microbenchmarks
```

---

## Formato da Mensagem

```
| START (1B) | VER (1B) | NODE (1B) | MSG_ID (1B) | SEQ (2B) | TLV_COUNT (1B) | TLV... | SIG (1B) | CRC16 (2B) |
```

- **Header**: 7 bytes
- **Signature**: 1 byte (XOR key ⊕ msg_id ⊕ seq_lo ⊕ seq_hi)
- **CRC16**: 2 bytes (CCITT-FALSE)
- **Overhead total**: 10 bytes

---

## CAN ID Extended (29-bit)

```
Bits 28-26: Prioridade (3 bits)
Bits 25-22: Grupo origem (4 bits)
Bits 21-18: Grupo destino (4 bits)
Bits 17-14: Tipo mensagem (4 bits)
Bits 13-0:  Reservado (14 bits)
```

---

## Licença

GPL-3.0 — ver [LICENSE](LICENSE)

**Autor**: ShegaPT
