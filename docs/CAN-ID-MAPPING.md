# Bythos v3.0.0 — Mapeamento CAN ID Extended (29-bit)

## Estrutura do CAN ID

```text
Bit:  28 27 26 | 25 24 23 22 | 21 20 19 18 | 17 16 15 14 | 13 ... 0
      [ PRIO  ] [  SRC_GRP  ] [  DST_GRP  ] [  MSG_TYPE ] [RESERVADO]
       3 bits     4 bits        4 bits        4 bits       14 bits
```

### Fórmulas

```rust
can_id = (priority << 26) | (src_group << 22) | (dst_group << 18) | (msg_type << 14);

priority  = (can_id >> 26) & 0x07;
src_group = (can_id >> 22) & 0x0F;
dst_group = (can_id >> 18) & 0x0F;
msg_type  = (can_id >> 14) & 0x0F;
```

---

## Grupos Computacionais (4 bits)

| Valor | Nome     | Nível | Descrição                        |
|:-----:|----------|:-----:|----------------------------------|
| 0x0   | None     | -     | Nenhum grupo / broadcast         |
| 0x1   | Device0  | 1     | Orquestração central             |
| 0x2   | Device1  | 2     | Aquisição de sensores            |
| 0x3   | Device2  | 2     | Controlo de atuadores            |
| 0x4   | Device3  | 0     | Segurança / supervisão           |
| 0x5   | Device4  | 1     | Emergência                       |
| 0x6   | Device5  | 2     | Visão por computador             |
| 0x7-F | Reserved | -     | Reservado para expansão futura   |

---

## Tipos de Mensagem CAN (4 bits)

| Valor | Nome  | Descrição                         |
|:-----:|-------|-----------------------------------|
| 0x0   | Data  | Dados de telemetria / sensores    |
| 0x1   | Cmd   | Comandos                          |
| 0x2   | Ack   | Confirmação de receção (ACK)      |
| 0x3   | Event | Eventos / failsafe                |
| 0x4   | Sync  | Sincronização temporal            |
| 0x5   | State | Broadcast de estado               |
| 0x6   | Heart | Heartbeat                         |
| 0x7   | Safety| Dados de segurança                |

---

## Prioridades (3 bits)

| Valor | Nome          | Descrição                         |
|:-----:|---------------|-----------------------------------|
| 0     | SuperCritical | Processamento imediato            |
| 1     | Critical      | Processamento urgente             |
| 2     | High          | Processamento urgente             |
| 3     | Medium        | Processamento padrão              |
| 4     | Low           | Quando disponível                 |

---

## Exemplos de CAN ID

### Device5 envia telemetry (broadcast)

```text
Priority = High (2)
SrcGroup = Device5 (6)
DstGroup = None (0) = broadcast
MsgType  = Data (0)

can_id = (2 << 26) | (6 << 22) | (0 << 18) | (0 << 14)
       = 0x08000000 | 0x01800000 | 0x00000000 | 0x00000000
       = 0x09800000
```

### Device3 envia Safety

```text
Priority = SuperCritical (0)
SrcGroup = Device3 (4)
DstGroup = Device4 (5)
MsgType  = Safety (7)

can_id = (0 << 26) | (4 << 22) | (5 << 18) | (7 << 14)
       = 0x00000000 | 0x01000000 | 0x00140000 | 0x0001C000
       = 0x0115C000
```

### Detecção de Safety Bus

```rust
fn is_safety_bus_id(can_id: u32) -> bool {
    can_id_msg_type(can_id) == 0x07  // Safety
}
```

---

## CAN ID por Módulo

| Módulo    | SrcGroup | CAN IDs típicos (Data)     |
|-----------|:--------:|----------------------------|
| Device0   | 0x1      | 0x04000000 - 0x07FFFFFF    |
| Device1   | 0x2      | 0x08000000 - 0x0BFFFFFF    |
| Device2   | 0x3      | 0x0C000000 - 0x0FFFFFFF    |
| Device3   | 0x4      | 0x10000000 - 0x13FFFFFF    |
| Device4   | 0x5      | 0x14000000 - 0x17FFFFFF    |
| Device5   | 0x6      | 0x18000000 - 0x1BFFFFFF    |
