# Bythos Protocol v3.0.0

**Biblioteca genérica para protocolos TLV — Rust + C**

Bythos é um protocolo de comunicação binário baseado em TLV (Type-Length-Value), concebido para sistemas embedded, veículos aéreos não tripulados (UAV/UAS) e redes CAN. Implementações independentes em Rust e C, com interoperabilidade via FFI.

---

## Características

- **FieldID com tipo embutido** — `[TYPE:3][ID:5]` — 8 tipos (Raw, Float32, Float16, Int32, Uint32, Uint16, Uint8, Bool) × 32 IDs = 60+ campos
- **12 tipos de mensagem** — Heartbeat, Telemetry, Command, Ack, Failsafe, Debug, Video, Shell, SiData, Watchdog, Ping, Clock
- **CRC-16/CCITT** — integridade de dados com tabela pré-calculada
- **Assinatura XOR** — autenticação rápida de mensagens
- **CAN ID extended** — 29 bits com prioridade (5 níveis), 14 grupos origem/destino e 8 tipos de mensagem
- **Parser FSM** — máquina de estados de 9 estados para parsing byte-a-byte, com timeout configurável via injeção de timestamp
- **FFI C completa** — API idêntica em Rust e C (`bythos_*`, `bythos_parser_*`)
- **`no_std`** — suporte para ambientes sem biblioteca padrão
- **Feature flags** — `std`, `transport-uart`, `transport-lora`, `transport-spi`, `transport-i2c`, `transport-can`, `esp`

---

## Estrutura

```text
Bythos/
├── Cargo.toml              # Crate bythos
├── src/
│   ├── lib.rs              # Raiz da crate
│   ├── protocol/
│   │   ├── types.rs        # Constantes, enums, FieldId, CAN ID
│   │   ├── codec.rs        # Serialização/deserialização TLV
│   │   ├── builder.rs      # Construtor de mensagens (TLVBuilder)
│   │   ├── crc8.rs         # CRC-8/SMBUS
│   │   ├── crc16.rs        # CRC-16/CCITT
│   │   ├── ffi.rs          # FFI Rust (bythos_*)
│   │   └── mod.rs          # Módulo protocol
│   └── parser/
│       ├── fsm.rs          # Parser FSM (9 estados, byte-a-byte)
│       ├── ffi.rs          # FFI parser (bythos_parser_*)
│       └── mod.rs          # Módulo parser
├── tests/
│   └── test_bythos.rs      # 42 testes de integração
├── c_core/
│   ├── include/bythos.h    # Header C completo
│   ├── src/bythos.c        # Implementação C standalone
│   ├── tests/test_bythos.c # 21 testes unitários C
│   ├── benchmark/          # Microbenchmarks C
│   └── CMakeLists.txt      # Build CMake
├── docs/                   # Documentação PT-PT
│   └── EN/                 # Documentação English
├── .gitignore
├── README.md               # Este ficheiro (PT-PT)
├── README-EN.md            # English version
├── CHANGELOG.md            # Histórico de versões
└── LICENSE                 # GPL-3.0
```

---

## Início Rápido

### Rust

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::validate_message;
use bythos::parser::fsm::Parser;

fn main() {
    // Construir mensagem
    let mut builder = TLVBuilder::new(0x06, 0x42);
    builder.set_seq(1);
    builder.add_u8_field(0, 4).unwrap();        // SystemState (ID=0 → 0xC0)
    builder.add_f32_field(6, 40.0).unwrap();    // GPS Latitude (ID=6 → 0x26)

    let mut buffer = [0u8; 1098];
    let size = builder.build(0x11, &mut buffer).unwrap();

    // Validar
    let _tlv_count = validate_message(&buffer[..size]).unwrap();

    // Parse byte-a-byte
    let mut parser = Parser::new(0x42);
    for &byte in &buffer[..size] {
        if parser.feed(byte) == bythos::parser::fsm::ParserError::Ok && parser.has_message() {
            let msg = parser.get_message();
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
    bythos_tlv_add_u8(&msg, BYTHOS_FIELD_SYSTEM_STATE, 4);

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
# No host (Linux/macOS) — usar RUSTUP_TOOLCHAIN=stable porque rust-toolchain.toml usa canal "esp"
RUSTUP_TOOLCHAIN=stable cargo build
RUSTUP_TOOLCHAIN=stable cargo test
RUSTUP_TOOLCHAIN=stable cargo clippy

# No ESP32 — toolchain "esp" deve estar instalada
cargo build --release --target xtensa-esp32-none-elf
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

```text
| START (1B) | VER (1B) | NODE (1B) | MSG_ID (1B) | SEQ (2B) | TLV_COUNT (1B) | TLV... | SIG (1B) | CRC16 (2B) |
```

- **Header**: 7 bytes
- **Signature**: 1 byte (XOR key ⊕ msg_id ⊕ seq_lo ⊕ seq_hi)
- **CRC16**: 2 bytes (CCITT-FALSE)
- **Overhead total**: 10 bytes

### Campo TLV — FieldID com Tipo Embutido

```text
FieldID = [TYPE:3][ID:5]

  Bits 7-5: Tipo de dado (0-7)
  Bits 4-0: ID do campo (0-31)
```

Exemplo: `0x26` = TYPE=1 (Float32) + ID=6 (GPS Latitude)

### 8 Tipos de Dado

| Tipo | Valor | Tamanho | Exemplo |
|------|:-----:|:-------:|---------|
| Raw | 0 | Variável | Payload vídeo |
| Float32 | 1 | 4B | GPS, IMU, Voo |
| Float16 | 2 | 2B | Sensores reduzidos |
| Int32 | 3 | 4B | Valores sinalizados |
| Uint32 | 4 | 4B | Uptime, heap livre |
| Uint16 | 5 | 2B | Frame ID, contadores |
| Uint8 | 6 | 1B | Estado, flags |
| Bool | 7 | 1B | Verdadeiro/falso |

### 12 Tipos de Mensagem (MsgID)

| MsgID | Nome | Valor | Prioridade padrão |
|:-----:|------|:-----:|:-----------------:|
| 0x10 | Heartbeat | 16 | Low |
| 0x11 | Telemetry | 17 | Medium |
| 0x12 | Command | 18 | Medium |
| 0x13 | Ack | 19 | Medium |
| 0x14 | Failsafe | 20 | Critical |
| 0x15 | Debug | 21 | Low (SuperCritical em failsafe) |
| 0x16 | Video | 22 | Low |
| 0x17 | Shell | 23 | Medium |
| 0x18 | SiData | 24 | Medium |
| 0x19 | Watchdog | 25 | High |
| 0x1A | Ping | 26 | Low |
| 0x1B | Clock | 27 | Medium |

---

## CAN ID Extended (29-bit)

```text
Bits 28-26: Prioridade (3 bits) — 5 níveis
Bits 25-22: Grupo origem (4 bits) — 14 grupos (Device0-Device14)
Bits 21-18: Grupo destino (4 bits) — 0x0 = broadcast
Bits 17-14: Tipo mensagem (4 bits) — 8 tipos (Data, Cmd, Ack, Event, Sync, State, Heart, Safety)
Bits 13-0:  Reservado (14 bits)
```

---

## Parser FSM

9 estados: `WaitStart` → `WaitHeader` → `WaitTlvCount` → `WaitTlvId` → `WaitTlvLen` → `WaitTlvData` → `WaitSignature` → `WaitCrc16Lo` → `WaitCrc16Hi`

- **Timeout configurável** via `Parser::with_timestamp_fn()` (injeção de fonte de tempo)
- **Contadores** de sucesso/erro por parser
- **last_error** persistente para diagnóstico
- **Proteções**: overflow, timeout, CRC inválido, assinatura inválida

---

## Testes

```text
Rust:   92 unitários + 42 integração + 4 doc-tests = 138 total
C:      21 unitários + microbenchmarks
Total:  159 testes
```

---

## Aviso de Segurança

A chave de assinatura padrão (`0x00`) não fornece autenticação — qualquer dispositivo pode enviar mensagens legítimas.
Para uso em produção, configure uma chave de assinatura própria via `set_key()` (Rust) ou `bythos_init()` (C).
A assinatura Bythos é um XOR de integridade (1 byte), não uma assinatura criptográfica.

---

## Documentação

| Ficheiro | Descrição |
|----------|-----------|
| [docs/BYTHOS-SPECIFICATION.md](docs/BYTHOS-SPECIFICATION.md) | Especificação completa do protocolo |
| [docs/BUILDER-GUIDE.md](docs/BUILDER-GUIDE.md) | Guia do construtor de mensagens |
| [docs/PARSER-GUIDE.md](docs/PARSER-GUIDE.md) | Guia do parser FSM |
| [docs/FFI-GUIDE.md](docs/FFI-GUIDE.md) | Guia da API FFI C |
| [docs/FIELD-ID-REFERENCE.md](docs/FIELD-ID-REFERENCE.md) | Referência de todos os FieldIDs |
| [docs/MESSAGE-ID-REFERENCE.md](docs/MESSAGE-ID-REFERENCE.md) | Referência de MsgIDs |
| [docs/CAN-ID-MAPPING.md](docs/CAN-ID-MAPPING.md) | Mapeamento CAN ID |
| [docs/EXAMPLES.md](docs/EXAMPLES.md) | Exemplos de uso |
| [docs/DEVELOPER-TIPS.md](docs/DEVELOPER-TIPS.md) | Dicas de desenvolvimento |
| [docs/EN/](docs/EN/) | Versões em English |

---

## Licença

GPL-3.0 — ver [LICENSE](LICENSE)

**Autor**: ShegaPT
