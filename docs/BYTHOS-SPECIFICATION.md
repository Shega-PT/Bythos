# Bythos — Especificação do Protocolo e do Anel

**Versão do protocolo:** 4.0.0 (branch `V4`) · **Autor:** ShegaPT · **Licença:** GPL-3.0

> Documento normativo da V4. Em caso de conflito com guias, prevalece este ficheiro.
> Nomenclatura pública é exclusivamente Bythos: não existem `CAN`, `TLV` nem
> `DeviceX` na API, nos documentos de utilizador ou no fio. Internamente o desenho
> inspira-se nessas tecnologias (arbitragem por prioridade, campos tipo-comprimento-valor),
> mas sem expor os termos.

---

## 1. Visão geral

A V4 transforma o Bythos num barramento em **anel fechado**, seguro por hardware e sem
limite prático de nós:

- **1 PCB idêntica por nó** (`BythosBridge`): transceptor + regenerador de relógio +
  elemento seguro. Não existe placa controladora nem placa master — `master` é apenas
  o papel temporário do nó `BG-0`.
- **Anel físico**: cada PCB tem 2 fichas RJ (`BEFORE`/`AFTER`), cabo único de 2 pares
  entrançados (`Dados+/Dados-` + `Relógio+/Relógio-`). O anel fecha-se em `BG-0`.
- **Endereçamento automático**: só `BG-0` tem endereço fixo (`0`, jumper `ROOT`) e gera
  o relógio. Todos os outros numeram-se sozinhos por ordem física de ligação.
- **Bidirecional curta**: cada trama segue pelo sentido com menos saltos. `BG-27 → BG-2`
  viaja para trás, sem dar a volta ao anel.
- **Segurança real**: etiqueta de autenticação de 4 bytes por mensagem, calculada em
  elemento seguro (ATECC608), com anti-repetição. O XOR da V3 sobrevive apenas como
  modo de compatibilidade (`KEY_ID = 0xFF`).
- **Rádio agnóstica**: qualquer modem (LoRa, Wi-Fi, ESP-NOW, FSK) transporta tramas V4
  como tubo opaco, desde que ambos os extremos tenham a sua `BythosBridge`.

---

## 2. Camadas

```text
┌──────────────────────────────────────────────┐
│ Aplicação (telemetria, comando, vídeo, ...)  │
├──────────────────────────────────────────────┤
│ BythosField (campo tipado: tipo+id+dados)    │  ← ex-TLV, renomeado
├──────────────────────────────────────────────┤
│ BythosMessage (cabeçalho + campos +SEC+CRC)  │
├──────────────────────────────────────────────┤
│ Segurança (SEC_HDR + TAG 4B + anti-replay)   │  ← elemento seguro
├──────────────────────────────────────────────┤
│ Anel (enumeração, encaminhamento, saltos)    │  ← Hello/Count
├──────────────────────────────────────────────┤
│ Físico (2 pares diferenciais + relógio)      │  ← BythosBridge
└──────────────────────────────────────────────┘
```

---

## 3. Formato da trama V4 no fio

### 3.1 Estrutura

```text
[INÍCIO:1][VERSÃO:1][ORIGEM:2][DESTINO:2][MSG:1][SEQ:2 LE][N_CAMPOS:1][SALTOS:1]
[CAMPOS...][SEC_HDR:4][TAG:4][CRC16:2 LE]
```

| Desloc. | Campo     | Tam. | Descrição                                              |
|:-------:|-----------|:----:|--------------------------------------------------------|
| 0       | INÍCIO    | 1    | Fixo `0xAA`                                            |
| 1       | VERSÃO    | 1    | `0x04` (V4 rejeita `0x03`; ver migração)               |
| 2–3     | ORIGEM    | 2    | Endereço Bythos do emissor, LE (`0–65535`)             |
| 4–5     | DESTINO   | 2    | Endereço Bythos do destino, LE (`0xFFFF` = difusão)    |
| 6       | MSG       | 1    | Tipo de mensagem (`0x10–0x1F`, §5)                     |
| 7–8     | SEQ       | 2    | Sequência do emissor, LE                               |
| 9       | N_CAMPOS  | 1    | Nº de campos Bythos (`0–32`)                           |
| 10      | SALTOS    | 1    | Limite de saltos; cada ponte decrementa; `0` = descarta|
| 11…     | CAMPOS    | var  | `[FIELD_ID:1][LEN:1][DADOS:LEN]`, LEN `0–32` (128 só vídeo raw) |
| …       | SEC_HDR   | 4    | `[KEY_ID:1][CTR:3 LE]` — chave + contador anti-replay  |
| …       | TAG       | 4    | Autenticação truncada (HMAC-SHA256-32 por defeito)     |
| …       | CRC16     | 2    | CCITT `0x1021/init 0xFFFF`, sobre tudo antes dele, LE  |

Cabeçalho: **11 bytes**. Reboque: **10 bytes**. Sobrecarga total: **21 bytes**.

### 3.2 Tamanhos máximos

| Símbolo V4                  | Valor | Fórmula                              |
|-----------------------------|------:|--------------------------------------|
| Campos por mensagem         | 32    | —                                    |
| Dados por campo normal      | 32 B  | —                                    |
| Dados do campo vídeo (raw)  | 128 B | só `FIELD_ID` de carga vídeo         |
| Mensagem máxima             | 1205 B| `11 + 31·34 + 130 + 10` (31 normais + 1 vídeo) |

> Uma mensagem com dois campos de 128 B excede o máximo e é recusada na construção
> (`CampoDemasiadoLongo`/`TampãoPequeno`). O analisador nunca escreve fora do tampão:
> excesso de bytes no fio = `ErrOverflow` + reinício.

### 3.3 Campo Bythos (ex-TLV)

```text
[FIELD_ID:1][LEN:1][DADOS:LEN]
```

`FIELD_ID = [TIPO:3][ID:5]`, 1 byte. Os 8 tipos e os 60+ identificadores mantêm os
**valores** da V3 (compatibilidade de catálogo), mudando só os **nomes**
(`BythosFieldId::GpsLatitude = 0x26`, etc.). Ver `MIGRATION-GUIDE.md`.

### 3.4 Cabeçalho de segurança (SEC_HDR)

```text
[KEY_ID:1][CTR:3 LE]
```

- `KEY_ID`: ranhura de chave no elemento seguro (`0–253`). `0xFF` = modo legado V3
  (XOR, sem TAG real — só para migração, nunca em produção).
- `CTR`: contador monotónico de 24 bits do emissor (persistido no elemento seguro).
  O recetor aceita apenas `CTR` estritamente maior que o último visto por
  `(ORIGEM, KEY_ID)` — janela de 64 para tolerar reordenação no anel.

### 3.5 TAG de autenticação (4 bytes, ambos os modos S/L)

`TAG = trunc32(HMAC-SHA256(chave[KEY_ID], INÍCIO…SEC_HDR))`, calculado **dentro** do
elemento seguro (a chave nunca sai do chip). Cobre cabeçalho + campos + SEC_HDR.
O CRC16 continua a existir para deteção rápida de corrupção acidental antes de
acordar o elemento seguro.

### 3.6 CRC16

Idêntico à V3: polinómio `0x1021`, `init 0xFFFF`, sem reflexão, sobre todos os bytes
anteriores. Vetor conhecido `"123456789" → 0x29B1`.

---

## 4. Endereçamento e anel

### 4.1 Grupos e endereços

- `BythosGroup` (`BG-*`): papel funcional do nó (controlo, sensores, segurança…),
  4 bits, herdado do desenho anterior mas com nomes funcionais em vez de `DeviceX`.
- `BythosAddress`: posição no anel, `u16` (`0–65535`). `0` = raiz (`BG-0`).
  `0xFFFF` = difusão.
- Cabeçalho CAN de 29 bits da V3 **desaparece do fio**; a prioridade passa a viajar
  em `MSG + BythosGroup` e na ordem de emissão. O endereço Bythos de 16 bits é o
  único identificador de encaminhamento.

### 4.2 Topologia

```text
     ┌──────────────────────────────────────────────┐
     │                    ANEL                      │
     │  BG-0.AFTER → BG-1 → BG-2 → … → BG-N.AFTER ──┘
     │  BG-0.BEFORE ← ─────────────────────────────┘
     │  (BG-0 fecha o anel; gera o relógio em AFTER)
     └──────────────────────────────────────────────┘
```

Cada ponte regenera dados e relógio (`BEFORE→AFTER` e `AFTER→BEFORE`) com
*corte direto* (~1–2 µs/salto). Latência pior caso com 1000 nós ≈ 500 saltos no
sentido curto ≈ 0,5–1 ms.

### 4.3 Enumeração automática

1. `BG-0` (único com jumper `ROOT`) injeta `Hello{pos=0}` em `AFTER`.
2. Cada nó assume `eu = recebido + 1` e repete para `AFTER`.
3. Quando o `Hello` regressa a `BG-0.BEFORE`, `BG-0` conhece `N` e difunde
   `Count{N}` nos dois sentidos.
4. Inserção a quente a jusante re-enumera só a jusante; remoção = perda de
   relógio/ligação e re-anúncio. Nunca se declara endereço à mão.

### 4.4 Encaminhamento (sentido mais curto)

```text
se destino == eu ou difusão: consome (e repete se não for fim de linha)
senão:
  saltos_horário     = (destino - eu + N) mod N
  saltos_anti_horário = (eu - destino + N) mod N
  envia para AFTER se saltos_horário <= saltos_anti_horário, senão para BEFORE
```

Cada retransmissão decrementa `SALTOS`; a `0`, descarta (protege contra erro de
cablagem em anel acidental). Corte de 1 ponto → modo linha + `RING_OPEN`; 2
cortes → ilha isolada em `SAFE` (UART assíncrona 115200 sobre o par de dados).

### 4.5 Relógio

Só `BG-0` gera `CLK+/CLK-` contínuo (mesmo em vazio). As pontes regeneram, não
geram. `BG-0.BEFORE` recebe o próprio relógio de volta = batimento do anel; se
não regressar em `X ms`, anel partido. Reserva opcional `ROOT_BACKUP` assume como
novo `BG-0` após silêncio (mesmo hardware, papel diferente — sem placa extra).

---

## 5. Tipos de mensagem

Base V3 (`0x10–0x1B`) + 4 do anel (`0x1C–0x1F`):

| ID   | Nome V4      | Uso                                    |
|:----:|--------------|----------------------------------------|
| 0x10 | Heartbeat    | Presença periódica                     |
| 0x11 | Telemetry    | Dados de sensores                      |
| 0x12 | Command      | Comandos                               |
| 0x13 | Ack          | Confirmação                            |
| 0x14 | Failsafe     | Segurança de voo                       |
| 0x15 | Debug        | Diagnóstico                            |
| 0x16 | Video        | Tramas fragmentadas                    |
| 0x17 | Shell        | Consola remota                         |
| 0x18 | SiData       | Dados de integridade estrutural        |
| 0x19 | Watchdog     | Vigilância                             |
| 0x1A | Ping         | Eco/medida de latência                 |
| 0x1B | Clock        | Relógio + anúncio de modo S/L          |
| 0x1C | Hello        | Enumeração (posição) — **novo**        |
| 0x1D | Count        | Enumeração (total N) — **novo**        |
| 0x1E | RingOpen     | Anel partido, modo linha — **novo**    |
| 0x1F | WiringFault  | Erro de cablagem — **novo**            |

---

## 6. Modos físicos S/L (mesmo cabo, jumper na PCB)

| Aspeto            | S — curta `<5 m` | L — longa `10–20 m`      |
|-------------------|------------------|--------------------------|
| Terminação 120R   | OFF (Hi-Z)       | ON só em `BG-0` (as 2 portas fecham o anel) |
| Polarização fail-safe | OFF           | ON no `BG-0`             |
| Slew-rate         | rápido           | limitado (~1 Mbps)       |
| Relógio           | 4–8 MHz          | 250 kHz–1 MHz            |
| Dados             | 2–8 Mbps         | 250 kbps–1 Mbps          |
| TAG               | 4 B              | 4 B (igual)              |

O modo é lido pelo MCU da ponte no arranque, anunciado em `Clock`, e divergência
`S vs L` força `SAFE 250 kbps` + `MODE_MISMATCH` (nunca corrompe o barramento).
Fábrica: `L 500 kbps, terminação OFF` — arranca sempre, mesmo que devagar.

Barramento físico, arbitragem e comparação com CAN: ver `BYTHOS-BUS.md`.

---

## 7. Rádio (túnel agnóstico)

A ponte não implementa rádio. Expõe `UART COBS` (`COBS + LEN + trama V4 + CRC16`)
para qualquer modem. O *gateway* conta como 1 endereço virtual; o relógio é
regenerado localmente. Exigência: **ambos os extremos com a sua ponte e a mesma
`KEY_ID`/sessão**, senão o `TAG` falha e a trama é descartada. Fragmentação:
`~200 B` LoRa, `~250 B` ESP-NOW, `8 B` CAN (se ainda usado como carga, nunca como
identidade do protocolo).

---

## 8. Compatibilidade

- Fio V3 (`VER=0x03`, `NODE u8`, XOR) é **rejeitado** por defeito (`VersãoInválida`).
- Feature `legacy-v3-compat`: aceita `0x03` só para migração, com `TAG` ausente e
  aviso; tradução `V3→V4` atribui `SRC/DST` por tabela estática de comissionamento.
- Regras semânticas: adicionar campos/IDs novos é compatível; alterar valores
  existentes ou o fio exige `VERSÃO+1`.

---

## 9. Ameaças (resumo; detalhe em `THREAT-MODEL.md`)

XOR não autentica; CRC não é segurança; `SEQ u16` a 100 Hz repete em ~10 min —
por isso a V4 autentica com `TAG` em hardware, anti-repetição com `CTR` de 24
bits + janela 64, chaves que nunca saem do chip, e identidade física (nº de série
72-bit + ECDSA no *join*).
