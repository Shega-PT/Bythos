# Bythos v3.0.0 — Guia FFI (C/C++)

Autor: ShegaPT | Licença: GPL-3.0

## Visão Geral

A biblioteca `bythos` fornece uma API FFI completa para C/C++, permitindo construir, serializar, validar e parsear mensagens Bythos v3.0.0 sem escrever Rust.

---

## Header

Incluir `bythos.h`:

```c
#include "bythos.h"
```

---

## Construção de Mensagens

```c
BythosMessage msg;
bythos_init(&msg, 0x06, 0x11);  // node_id=0x06, msg_id=0x11 (Telemetry)

bythos_set_seq(&msg, 1);

bythos_tlv_add_u8(&msg, 0xC0, 2);        // SystemState
bythos_tlv_add_f32(&msg, 0x26, -33.9f);   // GPS Latitude
bythos_tlv_add_f32(&msg, 0x27, 151.2f);   // GPS Longitude

uint8_t buffer[1098];
bythos_ssize_t size = bythos_build(&msg, 0x11, 0x42, buffer, sizeof(buffer));
//                                                          ^^^
//                                              signature_key = 0x42
```

---

## Validação

```c
uint8_t buffer[] = { ... };  // dados recebidos

uint8_t result = bythos_validate(buffer, sizeof(buffer));
if (result != 0xFF) {
    // Mensagem válida (result = tlv_count)
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
        BythosMessage* msg = bythos_parser_get_message(&parser);
        // Processar msg...
        bythos_parser_reset(&parser);
    }
}
```

---

## Funções FFI Disponíveis

### Protocolo

| Função                                            | Descrição                                    |
|---------------------------------------------------|----------------------------------------------|
| `bythos_init(msg, node_id, msg_id)`               | Inicializar mensagem                         |
| `bythos_set_seq(msg, seq)`                        | Definir sequência                            |
| `bythos_tlv_add_f32(msg, id, v)`                  | Adicionar campo f32                          |
| `bythos_tlv_add_i32(msg, id, v)`                  | Adicionar campo i32                          |
| `bythos_tlv_add_u32(msg, id, v)`                  | Adicionar campo u32                          |
| `bythos_tlv_add_u16(msg, id, v)`                  | Adicionar campo u16                          |
| `bythos_tlv_add_u8(msg, id, v)`                   | Adicionar campo u8                           |
| `bythos_tlv_add(msg, id, data, len)`              | Adicionar campo raw                          |
| `bythos_build(msg, msg_id, key, buf, buf_size)`   | Serializar mensagem                          |
| `bythos_validate(buf, len)`                       | Validar mensagem recebida                    |
| `bythos_clear(msg)`                               | Limpar mensagem                              |

### Parser

| Função                                            | Descrição                                    |
|---------------------------------------------------|----------------------------------------------|
| `bythos_parser_init(p, key)`                      | Inicializar parser                           |
| `bythos_parser_feed(p, byte)`                     | Alimentar um byte                            |
| `bythos_parser_get_message(p)`                    | Obter mensagem completa                      |
| `bythos_parser_has_message(p)`                    | Verificar se há mensagem                     |
| `bythos_parser_reset(p)`                          | Reiniciar parser                             |

### CRC

| Função                                            | Descrição                                    |
|---------------------------------------------------|----------------------------------------------|
| `bythos_calc_crc8(data, len)`                     | Calcular CRC-8/SMBUS                         |
| `bythos_calc_crc16(data, len)`                    | Calcular CRC-16/CCITT                        |

### CAN ID

| Função                                            | Descrição                                    |
|---------------------------------------------------|----------------------------------------------|
| `bythos_can_id_make(p, src, dst, type)`           | Construir CAN ID extended                    |
| `bythos_can_id_priority(id)`                      | Extrair prioridade                           |
| `bythos_can_id_src(id)`                           | Extrair grupo origem                         |
| `bythos_can_id_dst(id)`                           | Extrair grupo destino                        |
| `bythos_can_id_type(id)`                          | Extrair tipo mensagem                        |
| `bythos_is_safety_bus(can_id)`                    | Verificar se é bus de segurança              |

### FieldID

| Função                                            | Descrição                                    |
|---------------------------------------------------|----------------------------------------------|
| `bythos_field_id_encode(type, id)`                | Codificar FieldID                            |
| `bythos_field_id_decode(fid, type_out, id_out)`   | Decodificar FieldID                          |
| `bythos_field_id_valid(fid)`                      | Verificar FieldID válido                     |

### Assinatura

| Função                                            | Descrição                                    |
|---------------------------------------------------|----------------------------------------------|
| `bythos_signature_compute(key, msg_id, lo, hi)`   | Calcular assinatura                          |
| `bythos_validate_signature(sig, key, msg_id, lo, hi)` | Validar assinatura                      |

### Utilidades

| Função                                            | Descrição                                    |
|---------------------------------------------------|----------------------------------------------|
| `bythos_msg_id_valid(id)`                         | Verificar MsgID válido                       |
| `bythos_version()`                                | Versão do protocolo (string)                 |
| `bythos_overhead()`                               | Tamanho overhead (10 bytes)                  |
| `bythos_max_message_size()`                       | Tamanho máximo mensagem (1098 bytes)         |

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
