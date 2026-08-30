# Bythos v3.0.0 — Guia do Builder

## Visão Geral

O `TLVBuilder` é um construtor fluente para mensagens Bythos v3.0.0. Permite construir mensagens de forma segura e eficiente, adicionando campos um a um e serializando a mensagem completa no final.

---

## Uso em Rust

```rust
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::types::FieldType;

// Criar builder com node_id=0x06, signature_key=0x42
let mut builder = TLVBuilder::new(0x06, 0x42);

// Configurar número de sequência
builder.set_seq(1);

// Adicionar campos TLV
builder.add_u8_field(0xC0, 2).unwrap();          // SystemState = Ready
builder.add_f32_field(0x26, -33.8999).unwrap();  // GPS Latitude
builder.add_f32_field(0x27, 151.2093).unwrap();  // GPS Longitude
builder.add_f32_field(0x30, 1.5).unwrap();       // IMU Roll
builder.add_u32_field(0x82, 3600).unwrap();      // SystemUptime

// Serializar para buffer
let mut buffer = [0u8; 1098];
let size = builder.build(0x11, &mut buffer).unwrap();
// buffer[..size] contém a mensagem Bythos completa
```

---

## Métodos do Builder

### Criação

| Método                        | Descrição                                    |
|-------------------------------|----------------------------------------------|
| `TLVBuilder::new(node_id, key)` | Criar novo builder                         |
| `set_seq(seq)`                | Definir número de sequência (u16)            |

### Adicionar Campos

| Método                          | Descrição                              |
|---------------------------------|----------------------------------------|
| `add_f32_field(id, value)`      | Adicionar campo f32 (4 bytes)          |
| `add_f16_field(id, value)`      | Adicionar campo f16 (2 bytes)          |
| `add_i32_field(id, value)`      | Adicionar campo i32 (4 bytes)          |
| `add_u32_field(id, value)`      | Adicionar campo u32 (4 bytes)          |
| `add_u16_field(id, value)`      | Adicionar campo u16 (2 bytes)          |
| `add_u8_field(id, value)`       | Adicionar campo u8 (1 byte)            |
| `add_bool_field(id, value)`     | Adicionar campo bool (1 byte)          |
| `add_raw_field(id, data)`       | Adicionar campo raw (variável)         |
| `add_tlv(field)`                | Adicionar campo TLV pré-construído     |

### Serialização

| Método                          | Descrição                              |
|---------------------------------|----------------------------------------|
| `build(msg_id, buffer)`         | Serializar mensagem para buffer        |

### Informações

| Método                          | Descrição                              |
|---------------------------------|----------------------------------------|
| `field_count()`                 | Número de campos adicionados           |

---

## Uso via FFI (C/C++)

```c
#include "protocol_ffi.h"

TLVMessage msg;
bythos_init(&msg, 0x06, 0x42);

// Adicionar campos
bythos_add_tlv_uint8(&msg, 0xC0, 2);
bythos_add_tlv_float(&msg, 0x26, -33.8999f);
bythos_add_tlv_float(&msg, 0x27, 151.2093f);

// Definir sequência
bythos_set_seq(&msg, 1);

// Serializar
uint8_t buffer[1098];
ssize_t size = bythos_build_message(&msg, 0x11, buffer, sizeof(buffer));
```

---

## Validação

O `build()` valida:
1. Número de campos não excede `MAX_TLV_FIELDS` (32)
2. Tamanho do campo TLV não excede `MAX_TLV_DATA` (32 bytes)
3. Buffer tem tamanho suficiente (`MAX_MESSAGE_SIZE` = 1098 bytes)
4. Calcula CRC-16/CCITT e assinatura automaticamente

---

## Erros

| Erro              | Descrição                              |
|-------------------|----------------------------------------|
| `BufferTooSmall`  | Buffer de saída muito pequeno          |
| `TooManyFields`   | Mais de 32 campos TLV                  |
| `TlvDataTooLong`  | Campo TLV com dados > 32 bytes         |
