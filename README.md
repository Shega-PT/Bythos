# Bythos Protocol v4.0.0

**Protocolo em anel com segurança por hardware — Rust + C**

Bythos V4: 1 PCB idêntica por nó (`BythosBridge`), anel fechado de 2 pares entrançados (dados + relógio), enumeração automática desde o `BG-0`, endereços de 16 bits sem limite prático, e autenticação real (`TAG` HMAC-SHA256 de 4 B em elemento seguro ATECC608). Rádio agnóstica via túnel opaco. Implementações independentes em Rust e C com vetor dourado partilhado.

---

## Características

- **Anel bidirecional** — sentido mais curto, `BG-27 → BG-2` sem dar a volta
- **Bythos Bus** — arbitragem estilo CAN (menor ID ganha), ficha RJ, sem controlador — ver `docs/BYTHOS-BUS.md`
- **Auto-enumeração** — só o `BG-0` tem endereço fixo; o resto numera-se sozinho
- **TAG de 4 B + anti-replay** — HMAC-SHA256 truncado, `CTR` de 24 bits, janela 64
- **Elemento seguro** — chaves no ATECC608 (caminho de software só para testes)
- **f16 e bool em todo o lado** — incluindo FFI C (dívida V3 fechada)
- **Vídeo fragmentado** — 32 B por fragmento, remontagem sem heap + COBS para rádios
- **Analisador FSM** — 10 estados, limites reais, timeout a sério, sem chaves
- **`no_std` por feature** — `cargo build --no-default-features` prova no PC
- **Interop Rust↔C** — vetor dourado congelado nos dois lados

---

## Estrutura

```
├── Cargo.toml              # Crate bythos v4.0.0
├── src/
│   ├── lib.rs              # Raiz (exemplo compilável no doc)
│   ├── protocol/
│   │   ├── types.rs        # Constantes, endereços, catálogo BythosFieldId
│   │   ├── secure.rs       # SHA-256 + HMAC + selo + anti-replay
│   │   ├── builder.rs      # BythosBuilder (fluente e selado)
│   │   ├── codec.rs        # Fio: construir, espreitar, validar, analisar
│   │   ├── crc8.rs         # CRC-8 legado (migração)
│   │   ├── crc16.rs        # CRC-16/CCITT
│   │   ├── ffi.rs          # ABI C (bythos_*)
│   │   └── mod.rs
│   ├── parser/
│   │   ├── fsm.rs          # Analisador byte-a-byte (sem chaves)
│   │   ├── ffi.rs          # FFI analisador (bythos_parser_*)
│   │   └── mod.rs
│   ├── ring.rs             # Enumeração + sentido mais curto + supervisão
│   └── tunnel.rs           # COBS + remontagem de vídeo
├── tests/
│   ├── test_bythos.rs      # 14 testes ponta-a-ponta
│   └── test_ring.rs        # 4 testes de anel (5 nós, corte, BG-27→BG-2)
├── c_core/                 # C standalone (SHA-256+HMAC incluídos)
├── hardware/bythos-bridge/ # PCB: descrição, BOM, bring-up
├── docs/
│   ├── BYTHOS-SPECIFICATION.md # Norma do fio, anel e segurança
│   ├── BYTHOS-BUS.md           # O barramento (guia do produto)
│   ├── MIGRATION-GUIDE.md      # Migração V3 → V4
│   ├── THREAT-MODEL.md         # Modelo de ameaças
│   └── ...                 # Guias V3 (históricos, ver migração)
└── CHANGELOG.md
```

---

## Início Rápido

### Rust

```rust
use bythos::protocol::builder::BythosBuilder;
use bythos::protocol::codec::validate_message;
use bythos::protocol::secure::{PeerTable, SealConfig};
use bythos::parser::fsm::Parser;
use bythos::protocol::types::MAX_MESSAGE_SIZE;

fn main() {
    // Origem 27, chave de software (testes; produção: elemento seguro).
    let key = [0x42u8; 32];
    let mut b = BythosBuilder::new(27, SealConfig::software(0, key, 27));
    b.set_seq(7);
    b.add_u8_field(0, 2).unwrap();     // estado do sistema
    b.add_f32_field(6, 40.0).unwrap(); // latitude

    let mut buf = [0u8; MAX_MESSAGE_SIZE];
    let n = b.build(0x11, 0xFFFF, &mut buf).unwrap(); // telemetria em difusão

    // Destino final: estrutura + TAG + replay.
    let mut peers = PeerTable::new();
    let view = validate_message(&buf[..n], &key, &mut peers).unwrap();
    println!("Trama de {} com {} campos", view.header.src, view.header.field_count);

    // Ponte no caminho: analisa sem chaves.
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
    static const uint8_t key[32] = { [0] = 0x42 }; /* ensaio; produção: ATECC608 */
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
        printf("Trama válida com %d bytes\n", (int)n);
    }
    return 0;
}
```

---

## Build

### Rust

```bash
cargo build                        # PC, com std
cargo build --no-default-features  # prova no_std
cargo test                         # 107 testes (85 lib + 18 integração + 4 doc)
cargo clippy --all-targets         # 0 avisos
cargo fmt --check                  # limpo
```

### C

```bash
cmake -S c_core -B build && cmake --build build
./build/test_bythos   # 13 grupos, inclui vetor dourado Rust↔C
./build/bench_bythos  # microbenchmarks (TAG ~9 µs em software)
```

---

## Formato da Trama (normativo: `docs/BYTHOS-SPECIFICATION.md`)

```
| INÍCIO (1B) | VER=0x04 (1B) | ORIGEM (2B) | DESTINO (2B) | MSG (1B) |
| SEQ (2B) | N_CAMPOS (1B) | SALTOS (1B) | CAMPOS... | SEC_HDR (4B) | TAG (4B) | CRC16 (2B) |
```

- **Cabeçalho**: 11 bytes · **Reboque**: 10 bytes · **Sobrecarga**: 21 bytes
- **Máximo**: 1205 bytes (31 campos normais + 1 vídeo)
- **Vetor dourado**: `AA040600FFFF1101000120C001020029000082234BEF6248`

---

## Licença

GPL-3.0 — ver [LICENSE](LICENSE)

**Autor**: ShegaPT
