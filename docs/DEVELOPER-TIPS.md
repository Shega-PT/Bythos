# Bythos v3.0.0 — Dicas para Programadores

Autor: ShegaPT | Licença: GPL-3.0

## Compilação

### Host (Linux/macOS — testes e desenvolvimento)

```bash
RUSTUP_TOOLCHAIN=stable cargo test
RUSTUP_TOOLCHAIN=stable cargo check
```

### ESP32 (produção)

```bash
cargo build --release --target xtensa-esp32-none-elf
```

**Nota:** A toolchain `esp` deve estar instalada. No host, usar sempre `RUSTUP_TOOLCHAIN=stable`.

---

## Features

| Feature  | Descrição                                | Uso                  |
|----------|------------------------------------------|----------------------|
| `std`    | Biblioteca padrão (para testes/FFI)      | Host/FFI             |
| `no-std` | Sem biblioteca padrão (bare-metal ESP32) | ESP32 production     |
| `esp`    | Integração com esp-idf-sys               | ESP32 + FFI native   |

### Uso recomendado

- **Testes no host:** `cargo test` (default, com `std`)
- **ESP32 bare-metal:** `cargo build --no-default-features --no-std`
- **ESP32 com FFI:** `cargo build --features esp`

---

## Formato Wire

```text
Offset  Campo          Tamanho   Notas
------  -----          -------   -----
0       START_BYTE     1         0xAA
1       VERSION        1         0x03
2       NODE_ID        1         Grupo CAN
3       MSG_ID         1         0x10-0x1B
4-5     SEQ_NUM        2         u16 LE
6       TLV_COUNT      1         0-32
7+      TLV_FIELDS     Variável [ID][LEN][DATA...]
*       SIGNATURE      1         XOR(key, msg_id, seq_lo, seq_hi)
*-1     CRC16          2         CRC-16/CCITT LE
```

---

## FieldID — A Chave do Sistema

Cada campo TLV é identificado por um FieldID de 1 byte que embute o tipo:

```text
FieldID = [TYPE:3][ID:5]

Exemplo:
  FieldID 0x26 = tipo=1(f32), id=6 = GPS Latitude
  FieldID 0xC0 = tipo=6(u8),  id=0 = SystemState
```

---

## Adicionar Novo Campo

1. Escolher o tipo de dado (f32, u8, etc.)
2. Escolher um ID livre dentro da faixa do tipo
3. Definir o FieldID: `(tipo << 5) | id`
4. Adicionar à enum `FieldId` em `types.rs`
5. Documentar em `FIELD-ID-REFERENCE.md`

---

## Adicionar Nova Mensagem

1. Escolher um MsgID livre na faixa 0x10-0x1B
2. Definir a prioridade por defeito
3. Adicionar à enum `MsgId` em `types.rs`
4. Adicionar ao match em `get_msg_priority()`
5. Documentar em `MESSAGE-ID-REFERENCE.md`

---

## Adicionar Novo Grupo CAN

1. Escolher um valor 0x0-0xF livre
2. Adicionar à enum `CanGroup` em `types.rs`
3. Atualizar `make_can_id()` se necessário
4. Documentar em `CAN-ID-MAPPING.md`

---

## Erros Comuns

### CRC incorreto
- Verificar que o CRC é calculado sobre **todos os bytes anteriores** (header + TLV + signature)
- Verificar que o init value é 0xFFFF e não 0x0000
- Verificar que não há reflection

### Assinatura falha
- Verificar que a chave (key) está correta
- A assinatura é XOR de: `key ^ msg_id ^ seq_lo ^ seq_hi`
- Se key=0x00, a assinatura é apenas `msg_id ^ seq_lo ^ seq_hi`

### Parser não reconhece mensagem
- Verificar START_BYTE = 0xAA
- Verificar VERSION = 0x03
- Verificar MSG_ID na faixa 0x10-0x1B

### FFI não linka
- Usar `crate-type = ["staticlib", "rlib"]` em Cargo.toml
- Compilar com `--release` para optimized size

---

## Estrutura do Projeto

```text
Bythos/
├── Cargo.toml          # Configuração do crate
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
│   └── test_bythos.rs  # Integration tests
└── docs/               # Protocol documentation
    ├── BYTHOS-SPECIFICATION.md
    ├── FIELD-ID-REFERENCE.md
    ├── MESSAGE-ID-REFERENCE.md
    ├── CAN-ID-MAPPING.md
    ├── PARSER-GUIDE.md
    ├── BUILDER-GUIDE.md
    ├── FFI-GUIDE.md
    ├── DEVELOPER-TIPS.md
    └── EXAMPLES.md
```
