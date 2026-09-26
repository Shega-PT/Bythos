# Bythos — Protocol and Ring Specification

**Protocol version:** 4.0.0 (`V4` branch) · **Author:** ShegaPT · **License:** GPL-3.0

> Normative document. In case of conflict with guides, this file prevails.
> Public naming is exclusively Bythos: there is no `CAN`, `TLV` or `DeviceX` in
> the API, user documents or wire. Internally the design is inspired by those
> technologies (priority arbitration, type-length-value fields), without
> exposing the terms.

---

## 1. Overview

V4 turns Bythos into a **closed ring** bus, secured in hardware, with no
practical node limit:

- **1 identical PCB per node** (`BythosBridge`): transceiver + clock regenerator +
  secure element. No controller board, no master board — `master` is only the
  temporary role of node `BG-0`.
- **Physical ring**: each PCB has 2 RJ jacks (`BEFORE`/`AFTER`), one cable type
  with 2 twisted pairs (`Data+/Data-` + `Clock+/Clock-`). The ring closes at `BG-0`.
- **Automatic addressing**: only `BG-0` has a fixed address (`0`, `ROOT` jumper)
  and generates the clock. All others number themselves in physical order.
- **Short bidirectional**: each frame takes the fewest-hop direction. `BG-27 → BG-2`
  travels backwards, never loops the ring.
- **Real security**: 4-byte authentication tag per message, computed in a secure
  element (ATECC608), with anti-replay. V3 XOR survives only as compatibility
  mode (`KEY_ID = 0xFF`).
- **Agnostic radio**: any modem (LoRa, Wi-Fi, ESP-NOW, FSK) carries V4 frames as
  an opaque tube, as long as both ends have their `BythosBridge`.

---

## 2. Layers

```text
┌──────────────────────────────────────────────┐
│ Application (telemetry, command, video, ...) │
├──────────────────────────────────────────────┤
│ BythosField (typed field: type+id+data)      │
├──────────────────────────────────────────────┤
│ BythosMessage (header + fields + SEC + CRC)  │
├──────────────────────────────────────────────┤
│ Security (SEC_HDR + 4B TAG + anti-replay)    │  ← secure element
├──────────────────────────────────────────────┤
│ Ring (enumeration, forwarding, hops)         │  ← Hello/Count
├──────────────────────────────────────────────┤
│ Physical (2 differential pairs + clock)      │  ← BythosBridge
└──────────────────────────────────────────────┘
```

---

## 3. Wire frame format

### 3.1 Layout

```text
[START:1][VERSION:1][SRC:2][DST:2][MSG:1][SEQ:2 LE][FIELD_COUNT:1][HOPS:1]
[FIELDS...][SEC_HDR:4][TAG:4][CRC16:2 LE]
```

| Offset | Field     | Size | Description                                              |
|:------:|-----------|:----:|----------------------------------------------------------|
| 0      | START     | 1    | Fixed `0xAA`                                             |
| 1      | VERSION   | 1    | `0x04` (V4 rejects `0x03`; see migration)                |
| 2–3    | SRC       | 2    | Sender Bythos address, LE (`0–65535`)                    |
| 4–5    | DST       | 2    | Destination Bythos address, LE (`0xFFFF` = broadcast)    |
| 6      | MSG       | 1    | Message type (`0x10–0x1F`, §5)                           |
| 7–8    | SEQ       | 2    | Sender sequence, LE                                      |
| 9      | FIELD_COUNT | 1  | Number of Bythos fields (`0–32`)                         |
| 10     | HOPS      | 1    | Hop limit; each bridge decrements; `0` = drop            |
| 11…    | FIELDS    | var  | `[FIELD_ID:1][LEN:1][DATA:LEN]`, LEN `0–32` (128 video raw only) |
| …      | SEC_HDR   | 4    | `[KEY_ID:1][CTR:3 LE]` — key + anti-replay counter      |
| …      | TAG       | 4    | Truncated authentication (HMAC-SHA256-32 by default)     |
| …      | CRC16     | 2    | CCITT `0x1021/init 0xFFFF`, over everything before, LE   |

Header: **11 bytes**. Trailer: **10 bytes**. Total overhead: **21 bytes**.

### 3.2 Maximum sizes

| Symbol                        | Value | Formula                                  |
|-------------------------------|------:|------------------------------------------|
| Fields per message            | 32    | —                                        |
| Data per normal field         | 32 B  | —                                        |
| Video field data (raw)        | 128 B | `FIELD_ID` video payload only            |
| Max message                   | 1205 B| `11 + 31·34 + 130 + 10` (31 normal + 1 video) |

> A message with two 128 B fields exceeds the maximum and is refused at build
> time. The parser never writes outside its buffer: excess wire bytes mean
> `ErrOverflow` + reset.

### 3.3 Bythos field

```text
[FIELD_ID:1][LEN:1][DATA:LEN]
```

`FIELD_ID = [TYPE:3][ID:5]`, 1 byte. The 8 types and 60+ identifiers keep the V3
**numeric values** (catalog compatibility); only the **names** changed
(`BythosFieldId::GpsLatitude = 0x26`, etc.). See `MIGRATION-GUIDE.md`.

### 3.4 Security header (SEC_HDR)

```text
[KEY_ID:1][CTR:3 LE]
```

- `KEY_ID`: key slot in the secure element (`0–253`). `0xFF` = legacy V3 mode
  (XOR, no real TAG — migration only, never production).
- `CTR`: 24-bit monotonic sender counter (persisted in the secure element).
  The receiver only accepts `CTR` strictly greater than the last seen per
  `(SRC, KEY_ID)` — window of 64 to absorb ring reordering.

### 3.5 Authentication tag (4 bytes, both S/L modes)

`TAG = trunc32(HMAC-SHA256(key[KEY_ID], START…SEC_HDR))`, computed **inside**
the secure element (the key never leaves the chip). Covers header + fields +
SEC_HDR. CRC16 stays for cheap accidental-corruption detection before waking
the secure element.

### 3.6 CRC16

Same as V3: polynomial `0x1021`, `init 0xFFFF`, no reflection, over all previous
bytes. Known vector `"123456789" → 0x29B1`.

---

## 4. Addressing and ring

### 4.1 Groups and addresses

- `BythosGroup` (`BG-*`): functional role of a node (control, sensors, safety…),
  4 bits, inherited from the previous design but with functional names.
- `BythosAddress`: ring position, `u16` (`0–65535`). `0` = root (`BG-0`).
  `0xFFFF` = broadcast.
- The V3 29-bit CAN header **is gone from the wire**; priority travels in
  `MSG + BythosGroup` and emission order. The 16-bit Bythos address is the only
  forwarding identifier.

### 4.2 Topology

```text
     ┌──────────────────────────────────────────────┐
     │                    RING                      │
     │  BG-0.AFTER → BG-1 → BG-2 → … → BG-N.AFTER ──┘
     │  BG-0.BEFORE ← ─────────────────────────────┘
     │  (BG-0 closes the ring; generates the clock on AFTER)
     └──────────────────────────────────────────────┘
```

Each bridge regenerates data and clock (`BEFORE→AFTER` and `AFTER→BEFORE`) with
cut-through (~1–2 µs/hop). Worst case with 1000 nodes ≈ 500 hops short-path ≈
0.5–1 ms.

### 4.3 Automatic enumeration

1. `BG-0` (only one with the `ROOT` jumper) injects `Hello{pos=0}` on `AFTER`.
2. Each node takes `me = received + 1` and repeats on `AFTER`.
3. When `Hello` returns to `BG-0.BEFORE`, `BG-0` learns `N` and broadcasts
   `Count{N}` both ways.
4. Hot insertion downstream re-enumerates downstream only; removal = clock/link
   loss and re-announce. Addresses are never set by hand.

### 4.4 Forwarding (shortest path)

```text
if dst == me or broadcast: consume (and repeat unless end of line)
else:
  cw_hops  = (dst - me + N) mod N
  ccw_hops = (me - dst + N) mod N
  send AFTER if cw_hops <= ccw_hops, else BEFORE
```

Each retransmission decrements `HOPS`; at `0`, drop (protects against accidental
ring mis-wire loops). One cut → line mode + `RING_OPEN`; 2 cuts → isolated
island in `SAFE` (115200 async UART over the data pair).

### 4.5 Clock

Only `BG-0` generates continuous `CLK+/CLK-`. Bridges regenerate, never
generate. `BG-0.BEFORE` receives its own clock back = ring heartbeat. Optional
`ROOT_BACKUP` takes over as new `BG-0` after silence (same hardware, different
role — no extra board).

---

## 5. Message types

V3 base (`0x10–0x1B`) + 4 ring types (`0x1C–0x1F`):

| ID   | V4 name      | Use                                    |
|:----:|--------------|----------------------------------------|
| 0x10 | Heartbeat    | Periodic presence                      |
| 0x11 | Telemetry    | Sensor data and state                  |
| 0x12 | Command      | Commands                               |
| 0x13 | Ack          | Reception acknowledgement              |
| 0x14 | Failsafe     | Emergency safety state                 |
| 0x15 | Debug        | Diagnostics                            |
| 0x16 | Video        | Fragmented frames                      |
| 0x17 | Shell        | Remote console                         |
| 0x18 | SiData       | Structural integrity data              |
| 0x19 | Watchdog     | Supervision keepalive                  |
| 0x1A | Ping         | Echo/ring latency                      |
| 0x1B | Clock        | Clock + S/L mode announcement          |
| 0x1C | Hello        | Enumeration (position) — **new**       |
| 0x1D | Count        | Enumeration (total N) — **new**        |
| 0x1E | RingOpen     | Broken ring, line mode — **new**       |
| 0x1F | WiringFault  | Wiring error — **new**                 |

---

## 6. S/L physical modes (same cable, PCB jumper)

| Aspect            | S — short `<5 m` | L — long `10–20 m`             |
|-------------------|------------------|--------------------------------|
| 120R D+/D−        | OFF (Hi-Z)       | ON only on `BG-0` (closes ring)|
| 120R CLK+/CLK−    | OFF              | ON only on `BG-0`              |
| Fail-safe bias    | OFF              | ON on `BG-0`                   |
| Slew-rate         | fast             | limited (~1 Mbps)              |
| Clock             | 4–8 MHz          | 250 kHz–1 MHz                  |
| Data              | 2–8 Mbps         | 250 kbps–1 Mbps                |
| TAG               | 4 B              | 4 B (same)                     |

The mode is read by the bridge MCU at boot, announced in `Clock`, and an `S`
vs `L` mismatch forces `SAFE 250 kbps` + `MODE_MISMATCH` (never corrupts the
bus). Factory default: `L 500 kbps, termination OFF` — always boots, even if slow.

Physical bus, arbitration and CAN comparison: see `BYTHOS-BUS.md`.

---

## 7. Radio (agnostic tunnel)

The bridge implements no radio. It exposes `UART COBS` (`COBS + LEN + V4 frame
+ CRC16`) to any modem. The gateway counts as 1 virtual address; the clock is
regenerated locally. Requirement: **both ends with bridge and same `KEY_ID` /
session**, else the `TAG` fails and the frame is dropped. Fragmentation:
`~200 B` LoRa, `~250 B` ESP-NOW, `8 B` CAN (as payload only, never identity).

---

## 8. Compatibility

- V3 wire (`VER=0x03`, `NODE u8`, XOR) is **rejected** by default.
- `legacy-v3-compat` feature: accepts `0x03` for migration only, no `TAG`,
  with warning; `V3→V4` translation assigns `SRC/DST` from a static
  commissioning table.
- Semantic rules: adding fields/IDs is compatible; changing existing values or
  the wire requires `VERSION+1`.

---

## 9. Threats (summary; detail in `THREAT-MODEL.md`)

XOR does not authenticate; CRC is not security; `SEQ u16` at 100 Hz repeats in
~10 min — so the current version authenticates with hardware `TAG`,
anti-replay with 24-bit `CTR` + window of 64, keys that never leave the chip,
and physical identity (72-bit serial + ECDSA at join).
