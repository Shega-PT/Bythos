# Bythos v3.0.0 — Guia FFI (C/C++)

## Visão Geral

A biblioteca `bythos` fornece uma API FFI completa para C/C++, permitindo construir, serializar, validar e parsear mensagens Bythos v3.0.0 sem escrever Rust.

---

## Header

Incluir `protocol_ffi.h`:

```c
#include "protocol_ffi.h"
```

---

## Construção de Mensagens

```c
TLVMessage msg;
bythos_init(&msg, 0x06, 0x42);  // node_id=0x06, key=0x42

bythos_set_seq(&msg, 1);

bythos_add_tlv_uint8(&msg, 0xC0, 2);        // SystemState
bythos_add_tlv_float(&msg, 0x26, -33.9f);    // GPS Latitude
bythos_add_tlv_float(&msg, 0x27, 151.2f);    // GPS Longitude

uint8_t buffer[1098];
ssize_t size = bythos_build_message(&msg, 0x11, buffer, sizeof(buffer));
```

---

## Validação

```c
uint8_t buffer[] = { ... };  // dados recebidos
uint8_t key = 0x42;

uint8_t result = bythos_validate_message(buffer, sizeof(buffer), key);
if (result == 0) {
    // Mensagem válida
} else {
    // Erro de validação
}
```

---

## Parsing (Receção byte-a-byte)

```c
BythosParser parser;
bythos_parser_init(&parser, 0x42);

for (int i = 0; i < len; i++) {
    BythosParserResult result = bythos_parser_feed(&parser, data[i]);
    if (result == PARSER_OK_MSG) {
        TLVMessage* msg = bythos_parser_get_message(&parser);
        // Processar msg...
        bythos_parser_reset(&parser);
    }
}
```

---

## Funções FFI Disponíveis

### Protocolo

| Função                                     | Descrição                    |
|--------------------------------------------|------------------------------|
| `bythos_init(msg, node, key)`              | Inicializar mensagem         |
| `bythos_set_seq(msg, seq)`                 | Definir sequência            |
| `bythos_add_tlv_f32(msg, id, v)`           | Adicionar campo f32          |
| `bythos_add_tlv_f16(msg, id, v)`           | Adicionar campo f16          |
| `bythos_add_tlv_i32(msg, id, v)`           | Adicionar campo i32          |
| `bythos_add_tlv_u32(msg, id, v)`           | Adicionar campo u32          |
| `bythos_add_tlv_u16(msg, id, v)`           | Adicionar campo u16          |
| `bythos_add_tlv_u8(msg, id, v)`            | Adicionar campo u8           |
| `bythos_add_tlv_bool(msg, id, v)`          | Adicionar campo bool         |
| `bythos_add_tlv_raw(msg, id, d, l)`        | Adicionar campo raw          |
| `bythos_build_message(msg, mid, buf, len)` | Serializar mensagem          |
| `bythos_validate_message(buf, len, key)`   | Validar mensagem recebida    |
| `bythos_free_message(msg)`                 | Libertar memória da mensagem |

### Parser

| Função                         | Descrição                |
|--------------------------------|--------------------------|
| `bythos_parser_init(p, key)`   | Inicializar parser       |
| `bythos_parser_feed(p, byte)`  | Alimentar um byte        |
| `bythos_parser_get_message(p)` | Obter mensagem completa  |
| `bythos_parser_has_message(p)` | Verificar se há mensagem |
| `bythos_parser_reset(p)`       | Reiniciar parser         |

### CRC

| Função                         | Descrição             |
|--------------------------------|-----------------------|
| `bythos_calc_crc8(data, len)`  | Calcular CRC-8/SMBUS  |
| `bythos_calc_crc16(data, len)` | Calcular CRC-16/CCITT |

### CAN ID

| Função                                  | Descrição                       |
|-----------------------------------------|---------------------------------|
| `bythos_make_can_id(p, src, dst, type)` | Construir CAN ID extended       |
| `bythos_can_id_priority(id)`            | Extrair prioridade              |
| `bythos_can_id_src_group(id)`           | Extrair grupo origem            |
| `bythos_can_id_dst_group(id)`           | Extrair grupo destino           |
| `bythos_can_id_msg_type(id)`            | Extrair tipo mensagem           |
| `bythos_is_safety_bus_id(id)`           | Verificar se é bus de segurança |

### FieldID

| Função                          | Descrição           |
|---------------------------------|---------------------|
| `bythos_field_id_encode(t, id)` | Codificar FieldID   |
| `bythos_field_id_decode(fid)`   | Decodificar FieldID |

### Assinatura

| Função                                       | Descrição           |
|----------------------------------------------|---------------------|
| `bythos_compute_signature(k, m, lo, hi)`     | Calcular assinatura |
| `bythos_validate_signature(s, k, m, lo, hi)` | Validar assinatura  |

### Utilidades

| Função                         | Descrição                |
|--------------------------------|--------------------------|
| `bythos_msg_id_valid(id)`      | Verificar MsgID válido   |
| `bythos_is_valid_field_id(id)` | Verificar FieldID válido |

---

## Linking

### Linux (compilação estática)

```bash
gcc -o meu_programa meu_programa.c -L. -lbythos -lm
```

### ESP-IDF (ESP32)

Adicionar ao `CMakeLists.txt`:
```cmake
idf_component_register(SRCS "main.c"
                       INCLUDE_DIRS "."
                       REQUIRES bythos)
```
