# Bythos Bus — The Bus (product guide)

> If you know CAN bus, you will read this in 5 minutes — on purpose. Bythos Bus
> copies the concepts that make CAN work (identifier arbitration, differential
> pair, multi-master, priorities) and swaps the rest: cheap RJ jacks, dedicated
> clock, hardware security and Bythos names. This document is the commercial and
> technical reference of the bus; the wire norm lives in `EN-BYTHOS-SPECIFICATION.md`.

## 1. What it is

**Bythos Bus** is the physical and logical bus linking `BythosBridge` boards in
a ring: 1 cable with 2 twisted pairs (data + clock), telephone RJ jacks, up to
1000+ nodes with no address configuration, every frame authenticated in
hardware. Each node needs its bridge — just as each CAN node needs its transceiver.

```text
BG-0.AFTER ──RJ──► BG-1 ──RJ──► BG-2 ──RJ──► … ──RJ──► BG-0.BEFORE
  (clock born here)                                 (clock returns here)
```

## 2. Arbitration: like CAN, without CAN

On CAN, two colliding frames resolve on the copper: the smaller identifier
(dominant) wins without destroying either frame. RS-485 has no electrical
dominant, so Bythos Bus does the same in firmware — and the rule stands:

**lower value wins; the losing frame backs off and retransmits intact.**

The arbitration identifier (`BythosBusId`, 32 bits):

```text
Bits 31-29: priority (0 = most urgent)
Bits 28-25: functional group (control, sensors, safety…)
Bits 24-21: traffic kind (data, command, safety…)
Bits 20-5:  sender ring position
Bits 4-0:   reserved
```

Example: an emergency (`priority 0, safety, source 4`) always yields a smaller
ID than normal telemetry (`priority 3, sensors, source 27`) — it wins any
contention with zero configuration, exactly like a high-priority CAN frame.
On full ties, the lower source wins (deterministic).

Bridge practice: CSMA (listen before talk) + backoff ordered by this ID.
Order with plain `<` — see `BythosBusId::wins_over` (Rust) and
`bythos_bus_id_wins` (C).

## 3. Bythos Bus vs CAN bus (for buyers)

| Aspect              | CAN bus                        | Bythos Bus                              |
|---------------------|--------------------------------|-----------------------------------------|
| Connector / cable   | DB9 / industrial               | Telephone RJ, 2 pairs (cheap, ubiquitous)|
| Wires               | 2 (differential)               | 4 (2 pairs: data + dedicated clock)     |
| Controller          | Required (SJA1000, bxCAN…)     | None — the bridge does it all           |
| Sync                | Negotiated bit-timing          | 1 common clock pair (nothing to tune)   |
| Addresses           | Declared (filters, masks)      | Automatic from BG-0                     |
| Nodes               | ~110 theoretical               | 1000+ in model                          |
| Arbitration         | Electrical dominant            | Firmware, same rule (lowest wins)       |
| Typical speed       | 125 kbps–1 Mbps                | 250 kbps–8 Mbps (S/L jumper mode)       |
| Security            | None (SecOC aside)             | HMAC TAG + anti-replay in chip, always  |
| Radio               | Expensive, specific gateway    | Opaque tunnel for any modem             |
| Price per node      | Transceiver + controller + stack | 1 bridge (~€3–6)                      |

Honest pitch: *the CAN you already know, minus the controller, minus address
setup, with security on by default — and a telephone jack.*

## 4. Installation rules (the 5 that prevent 90% of issues)

1. **One `ROOT`, only one**: `ROOT` jumper on BG-0 alone. Two `ROOT`s = `WiringFault`.
2. **120R only on BG-0 in L mode** (both ports close the ring). Middle nodes always OFF.
3. **Everyone on the same S/L mode** — mismatch drops to `SAFE`, never corrupts.
4. **Twisted cable, never flat**: flat telephone works `<2 m`; above that, 2
   twisted pairs (or CAT5e on RJ45 for long runs).
5. **Short daisy chain, no stars**: stubs `<30 cm`; passive stars reflect and kill the signal.

## 5. Customer questions (one-line answers)

- *Do I need a CAN controller?* No. The bridge is transceiver + logic + security.
- *How many nodes?* From 2 to 1000+; enumeration handles it alone.
- *What if a cable is cut?* The ring becomes a line (`RING_OPEN`) and traffic takes the live side.
- *What about security?* Every frame carries a chip-computed HMAC tag; forging without the key is ~2³² online tries.
- *Does it work over radio?* Yes, with a bridge on each end and any modem — the radio carries, the bridge authenticates.
- *Can I mix lengths?* Yes: S mode on short runs, L on long ones, same cable, jumper per bridge.
