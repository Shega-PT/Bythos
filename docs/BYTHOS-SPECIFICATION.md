# Bythos v3.0.0 — Especificação do Protocolo

**Bythos Protocol v3.0.0**
Versão: 3.0.0 | Autor: ShegaPT | Licença: GPL-3.0

---

## 1. Visão Geral

O Bythos Protocol é um protocolo de comunicação binário compartilhado por todos os módulos e sistemas do Bythos. Utiliza TLV (Type-Length-Value) com campo FieldID que embute o tipo de dado, permitindo 8 tipos × 32 IDs = 256 campos possíveis.

### Características
- **Formato binário compacto** — minimiza overhead em CAN bus e WiFi
- **TLV extensível** — novos campos podem ser adicionados sem quebrar compatibilidade
- **FieldID com tipo embutido** — 3 bits de tipo + 5 bits de ID = 1 byte por campo
- **Assinatura XOR** — verificação básica de autenticidade
- **CRC-16/CCITT** — integridade de dados
- **SEQ_NUM u16** — anti-replay a 100Hz
- **Endianness:** Little-endian (compatível com ESP32/Xtensa)

---

## 2. Formato da Mensagem

### 2.1 Estrutura Wire

```text
[START_BYTE:1][VERSION:1][NODE_ID:1][MSG_ID:1][SEQ_NUM:2 LE][TLV_COUNT:1]
[TLV_FIELDS...][SIGNATURE:1][CRC16:2 LE]
```

### 2.2 Tamanhos

| Componente         | Tamanho (bytes) |
|--------------------|:--------------:|
| Header (fixed)     | 7              |
| Signature          | 1              |
| CRC16              | 2              |
| **Overhead Total** | **10**         |
| TLV Header (por campo) | 2         |
| TLV Data (máx. por campo) | 32      |
| Máx. campos TLV por msg | 32        |
| **Máx. mensagem** | **1098**       |

### 2.3 Campos do Header

| Offset | Campo       | Tamanho | Descrição                            |
|:------:|-------------|:-------:|--------------------------------------|
| 0      | START_BYTE  | 1       | Fixo: `0xAA` (170)                   |
| 1      | VERSION     | 1       | Versão do protocolo: `0x03` (v3)     |
| 2      | NODE_ID     | 1       | ID do nó transmissor (grupo CAN)     |
| 3      | MSG_ID      | 1       | Tipo de mensagem (0x10-0x1B)         |
| 4-5    | SEQ_NUM     | 2       | Número de sequência (u16, LE)        |
| 6      | TLV_COUNT   | 1       | Número de campos TLV (0-32)          |

### 2.4 Campo TLV (cada campo)

```text
[FIELD_ID:1][LEN:1][DATA:LEN]
```

- **FIELD_ID:** Tipo embutido `[TYPE:3][ID:5]` — 1 byte
- **LEN:** Número de bytes de dados (0-32)
- **DATA:** Dados brutos

### 2.5 Trailer

```text
[SIGNATURE:1][CRC16_LO:1][CRC16_HI:1]
```

---

## 3. FieldID — Codificação com Tipo Embutido

O FieldID de 1 byte combina tipo e identificador:

```text
  Bit:  7  6  5  4  3  2  1  0
        [  TYPE  ] [    ID     ]
         3 bits      5 bits
```

### 3.1 Tipos de Dado (3 bits)

| Valor | Tipo     | Tamanho padrão | Descrição                          |
|:-----:|----------|:--------------:|------------------------------------|
| 0     | Raw      | Variável       | Dados brutos (payload binário)     |
| 1     | Float32  | 4              | Float 32 bits                      |
| 2     | Float16  | 2              | Float 16 bits (half-precision)     |
| 3     | Int32    | 4              | Inteiro sinalizado 32 bits         |
| 4     | Uint32   | 4              | Inteiro sem sinal 32 bits          |
| 5     | Uint16   | 2              | Inteiro sem sinal 16 bits          |
| 6     | Uint8    | 1              | Inteiro sem sinal 8 bits           |
| 7     | Bool     | 1              | Booleano (0=false, 1=true)         |

### 3.2 Faixas de FieldID por Tipo

| Tipo  | Faixa FieldID | Exemplos               |
|-------|:-------------:|------------------------|
| Raw   | 0x00 - 0x1F  | VideoPayload (0x00)    |
| f32   | 0x20 - 0x3F  | GPS, IMU, Energia      |
| f16   | 0x40 - 0x5F  | (reservado)            |
| i32   | 0x60 - 0x7F  | (reservado)            |
| u32   | 0x80 - 0x9F  | SystemUptime, etc.     |
| u16   | 0xA0 - 0xBF  | FlightLoopTime, etc.   |
| u8    | 0xC0 - 0xDF  | SystemState, etc.      |
| Bool  | 0xE0 - 0xFF  | (reservado)            |

### 3.3 Fórmula

```rust
// Codificar
field_id = (field_type & 0x07) << 5 | (field_id & 0x1F);

// Decodificar
field_type = (field_id >> 5) & 0x07;
id = field_id & 0x1F;
```

---

## 4. Assinatura (XOR Key)

A assinatura é um byte calculado como XOR de 4 valores:

```rust
signature = key XOR msg_id XOR seq_lo XOR seq_hi
```

| Componente | Descrição                        |
|------------|----------------------------------|
| key        | Chave partilhada por nó (u8)     |
| msg_id     | ID da mensagem                   |
| seq_lo     | Byte baixo do SEQ_NUM            |
| seq_hi     | Byte alto do SEQ_NUM             |

A chave por defeito é `0x00` (sem assinatura efetiva). Cada nó pode ter uma chave diferente.

---

## 5. CRC-16/CCITT

- **Polinómio:** 0x1021 (CCITT)
- **Inicialização:** 0xFFFF
- **Reflexão:** Sem
- **XOR final:** Sem (0x0000)
- **Cálculo:** Sobre todos os bytes da mensagem (header + TLV + signature), **antes** do CRC ser adicionado

Vetor conhecido: `"123456789"` → CRC = 0x29B1

---

## 6. Anti-Replay (SEQ_NUM)

- Campo `SEQ_NUM` é u16 (0-65535), little-endian
- Cada nó mantém um contador que incrementa a cada mensagem enviada
- O receptor deve verificar que o SEQ_NUM recebido é progressivo
- Validade para taxas até ~100Hz por nó

---

## 7. Compatibilidade Retroativa

Regras semânticas de versão:
- **Adicionar:** Novos campos TLV, novos MsgIDs (reservados), novos FieldIDs
- **Nunca alterar:** Valores existentes de FieldID, MsgID, ou estrutura wire
- **Versão:** Incrementar VERSION apenas em quebras de compatibilidade
