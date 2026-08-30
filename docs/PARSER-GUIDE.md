# Bythos v3.0.0 — Guia do Parser FSM

Autor: ShegaPT | Licença: GPL-3.0

## Visão Geral

O parser FSM (Finite State Machine) reconstrói mensagens Bythos a partir de um fluxo de bytes, processando cada byte individualmente. É ideal para receção byte-a-byte em CAN bus ou Serial.

---

## Estados da FSM

```text
    ┌──────────────┐
    │  WaitStart   │ ← Estado inicial
    └──────┬───────┘
           │ byte == 0xAA
           ▼
    ┌──────────────┐
    │  WaitHeader  │ ← version, nodeId, msgId, seqLo, seqHi
    └──────┬───────┘
           │ 5 bytes lidos
           ▼
    ┌──────────────┐
    │ WaitTlvCount │ ← TLV_COUNT
    └──────┬───────┘
           │ 1 byte lido
           ▼
    ┌──────────────┐
    │   WaitTlvId  │ ← FIELD_ID
    └──────┬───────┘
           │ 1 byte lido
           ▼
    ┌──────────────┐
    │  WaitTlvLen  │ ← LEN
    └──────┬───────┘
           │ 1 byte lido
           ▼
    ┌──────────────┐
    │  WaitTlvData │ ← DATA[0..LEN-1]
    └──────┬───────┘
           │ LEN bytes lidos
           │ (ou LEN==0 → direto para Signature)
           ▼
    ┌──────────────┐
    │WaitSignature │ ← SIGNATURE
    └──────┬───────┘
           │ 1 byte lido
           ▼
    ┌──────────────┐
    │  WaitCrc16Lo │ ← CRC16_LO
    └──────┬───────┘
           │ 1 byte lido
           ▼
    ┌──────────────┐
    │  WaitCrc16Hi │ ← CRC16_HI
    └──────┬───────┘
           │ 1 byte lido → VALIDAR CRC
           ▼
    ┌──────────────┐
    │  Mensagem    │ → parser.has_message() == true
    │  Completa    │
    └──────────────┘
```

---

## Uso em Rust

```rust
use bythos::parser::fsm::{Parser, ParserError};

let mut parser = Parser::new(0x42); // signature_key = 0x42

// Alimentar byte a byte
for &byte in &message_bytes {
    match parser.feed(byte) {
        ParserError::Ok => continue,
        ParserError::ErrStart => { /* byte inválido, reiniciar */ }
        ParserError::ErrChecksum => { /* CRC inválido, mensagem corrompida */ }
        _ => { /* outro erro */ }
    }
}

// Verificar se há mensagem completa
if parser.has_message() {
    let msg = parser.get_message();
    // Processar msg...
}
```

---

## Uso via FFI (C/C++)

```c
#include "bythos.h"

BythosParser parser;
bythos_parser_init(&parser, 0x42);

for (int i = 0; i < len; i++) {
    BythosParserResult result = bythos_parser_feed(&parser, data[i]);
    if (result == PARSER_OK_MSG) {
        BythosMessage* msg = bythos_parser_get_message(&parser);
        // Processar msg...
    }
}
```

---

## Erros do Parser

| Erro          | Código | Descrição                         |
|---------------|:------:|-----------------------------------|
| Ok            | 0      | Operação bem-sucedida             |
| ErrStart      | 1      | Byte de início inválido           |
| ErrVersion    | 2      | Versão incompatível               |
| ErrMsgId      | 3      | ID da mensagem inválido           |
| ErrTlvCount   | 4      | Número de TLVs inválido           |
| ErrTlvId      | 5      | ID de campo TLV inválido          |
| ErrTlvLen     | 6      | Tamanho de campo TLV inválido     |
| ErrChecksum   | 7      | Checksum CRC16 inválido           |
| ErrSignature  | 8      | Assinatura inválida               |
| ErrTimeout    | 9      | Timeout entre bytes (frame gap)   |

---

## Validação

Após reconstruir a mensagem, o parser valida:
1. **CRC-16/CCITT** — integridade dos dados
2. **Signature** — autenticidade (se key != 0x00)

Se a validação falhar, `ParserError::ErrChecksum` ou `ParserError::ErrSignature` é retornado.
