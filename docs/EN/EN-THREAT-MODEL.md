# Bythos — Threat Model (working summary)

## 1. What V3 did not guarantee

| V3 flaw | Consequence | Fix |
|---|---|---|
| XOR with `key=0x00` called "authentication" | Any node forges frames | `TAG` HMAC-32 in secure element |
| `SEQ` echoed, never verified | Old frames replayed | 24-bit `CTR` + window of 64 per `(SRC, KEY_ID)` |
| CRC as sole integrity | Accidental corruption ok, attacker recomputes | CRC (accidental) + TAG (adversary) |
| Shared key in MCU RAM | Extraction via dump/probe | Key born and living in the ATECC608 |
| No hardware identity | Node cloned with a cheap MCU | 72-bit serial + ECDSA challenge at join |
| `SEQ u16` at 100 Hz repeats in ~10 min | Late replay passes | 24-bit `CTR` (~46 h at 100 Hz) + persistent monotonic |

## 2. Current guarantees

1. **Authenticity**: without the `KEY_ID` key, forging a 4-byte `TAG` takes
   ~2³² online tries — the ring flags `WiringFault`/lockout long before that.
2. **Freshness**: strictly increasing `CTR` per sender; window of 64 absorbs
   ring reordering without accepting replays.
3. **Selective confidentiality**: the `TAG` always authenticates; encrypting
   payload (`AES-128` on the ATECC608) is opt-in per `KEY_ID` for sensitive
   telemetry.
4. **Boot**: bridge MCU `secure boot` + ECDSA `join` before receiving the
   session `KEY_ID` (derived via ECDH + HKDF, never transmitted).

## 3. Out of model (non-promises)

- Open physical medium: the data pair can be sniffed — protection is
  cryptographic, not cable obscurity.
- Denial of service by physical cut: detected (`RING_OPEN`/island `SAFE`), but
  cut cable is not fixed in software.
- 4-byte `TAG` is not long-term public signature: repudiation is solved with
  off-hot-path ECDSA.
- Legacy mode `KEY_ID=0xFF` (XOR) **is not secure** — migration only, with a
  build warning (`legacy-v3-compat`) and refusal in production.
