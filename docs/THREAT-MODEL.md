# Bythos — Modelo de Ameaças (resumo operativo)

## 1. O que a V3 não garantia

| Falha V3 | Consequência | Correção V4 |
|---|---|---|
| XOR com `key=0x00` chamado "autenticação" | Qualquer nó forja tramas | `TAG` HMAC-32 em elemento seguro |
| `SEQ` ecoado, nunca verificado | Repetição de tramas antigas | `CTR` 24-bit + janela 64 por `(SRC, KEY_ID)` |
| CRC como única integridade | Corrupção acidental ok, atacante recalcula | CRC (acidental) + TAG (adversário) |
| Chave partilhada em RAM do MCU | Extração por `dump`/sonda | Chave nasce e vive no ATECC608 |
| Sem identidade de hardware | Clone de nó com MCU barato | Série 72-bit + desafio ECDSA no `join` |
| `SEQ u16` a 100 Hz repete em ~10 min | Replay tardio passa | `CTR` 24-bit (≈ 46 h a 100 Hz) + monotónico persistente |

## 2. Garantias V4

1. **Autenticidade**: sem a chave do `KEY_ID`, forjar `TAG` de 4 B exige ~2³²
   tentativas em linha — o anel sinaliza `WiringFault`/bloqueio muito antes.
2. **Frescura**: `CTR` estritamente crescente por emissor; janela 64 absorve
   reordenação do anel sem aceitar repetição.
3. **Confidencialidade seletiva**: o `TAG` autentica sempre; cifrar carga
   (`AES-128` do ATECC608) é opcional por `KEY_ID` para telemetria sensível.
4. **Arranque**: `secure boot` do MCU da ponte + `join` com ECDSA antes de
   receber `KEY_ID` de sessão (derivada por ECDH + HKDF, nunca transmitida).

## 3. Fora do modelo (não-promessas)

- Canal físico aberto: o par de dados pode ser escutado — a proteção é
  criptográfica, não escuridão do cabo.
- Negação de serviço por corte físico: detetada (`RING_OPEN`/ilha `SAFE`),
  mas cabo cortado não se resolve em software.
- `TAG` de 4 B não é assinatura pública de longo prazo: repúdio resolve-se com
  ECDSA fora do caminho quente.
- Modo legado `KEY_ID=0xFF` (XOR) **não é seguro** — existe só para migração,
  com aviso em compilação (`legacy-v3-compat`) e recusa em produção.
