# BythosBridge — PCB do Nó (barramento Bythos / Bythos Bus, rev A)

> Uma PCB idêntica por nó: transceptor + regenerador de relógio + elemento
> seguro, para o barramento Bythos (Bythos Bus). Sem placa controladora, sem placa master — `master` é o papel do
> `BG-0`, não hardware. Cada nó precisa da sua ponte para falar no anel ou
> via rádio (o modem de RF é "burro"; a segurança vive na ponte).

## 1. Diagrama de blocos

```text
                    ┌──────────────── BYTHOSBRIDGE ───────────────────┐
                    │                                                 │
  RJ BEFORE ──►┌────────┐    ┌──────────┐     ┌─────────┐    ┌────┐   │
  (D±,CLK±)    │ 2×RS485│───►│ MCU      │◄───►│ATECC608B│    │LDO │◄──│ VIN 3.3–5V
               │ D+CLK  │    │ bridge   │ I2C │ seguro  │    │3V3 │   │
  RJ AFTER ──► └────────┘    │ CH32V003 │     └─────────┘    └────┘   │
  (D±,CLK±)         ▲        │ 48 MHz   │         ▲                   │
                    │        └────┬─────┘         │                   │
               OE_DIR         UART+IRQ          chaves nunca          │
               +TERM          (host MCU)        saem do chip          │
                    │             │                                   │
                    └─────────────┴── jumper ROOT/NODE ── jumper S/L ─┘
                                          LED estado + test points
```

## 2. Fichas e pinout (iguais nas duas portas)

Ficha `RJ11 6P4C`, cabo telefone **de 2 pares entrançados** (não flat silver-satin):

| Pino RJ11 | Sinal   | Par   | Notas                                        |
|-----------|---------|-------|----------------------------------------------|
| 1         | NC      | —     | Drain da blindagem (se existir)              |
| 2         | CLK+    | par 2 | Relógio diferencial                          |
| 3         | D+      | par 1 | Dados diferenciais (centro = menos diafonia) |
| 4         | D−      | par 1 | Dados diferenciais                           |
| 5         | CLK−    | par 2 | Relógio diferencial                          |
| 6         | NC      | —     | Drain da blindagem (se existir)              |

Cablagem pino-a-pino (cabo direito); cruzar `AFTER→BEFORE` entre nós nunca —
o `WiringFault` denuncia, mas o correto é não precisar dele.

## 3. Modos S/L (jumper, mesmo cabo)

| Bloco                    | S — curta `<5 m`      | L — longa `10–20 m`              |
|--------------------------|-----------------------|----------------------------------|
| 120R D+/D−               | OFF (Hi-Z)            | ON **só em BG-0** (fecha o anel) |
| 120R CLK+/CLK−           | OFF                   | ON **só em BG-0**                |
| Polarização              | OFF                   | ON no BG-0 (680R/680R)           |
| Slew-rate (THVD1400 SRL) | rápido (GND)          | limitado (VCC, ~1 Mbps)          |
| Relógio                  | 4–8 MHz               | 250 kHz–1 MHz                    |
| Dados                    | 2–8 Mbps              | 250 kbps–1 Mbps                  |
| TAG                      | 4 B                   | 4 B (igual)                      |

O jumper `MODE` é lido pelo MCU no arranque e anunciado em `Clock`; divergência
`S vs L` no anel força `SAFE 250 kbps` + `MODE_MISMATCH`. Fábrica: `L 500 kbps,
terminação OFF` (arranca sempre). Em voo, jumper soldado + verniz (DIP só no kit dev).

## 4. Lista de material (alvo S ~3–4 €, L ~4,5–6 € @100)

| Ref     | Parte                      | Notas                                           |
|---------|----------------------------|-------------------------------------------------|
| U1      | CH32V003F4P6 (~0,15 €)     | MCU bridge (48 MHz, UART+GPIO)                  |
| U2      | ATECC608B-SSHDA (~0,8 €)   | Seguro (SOIC-8; footprint compatível ATSHA204A) |
| U3/U4   | THVD1400 / MAX13487E ×2    | RS-485 dados (auto-dir) + relógio               |
| J1/J2   | RJ11 6P4C ×2               | BEFORE / AFTER                                  |
| RN      | 120R comutável + 22R série | Anti-contenção transitória                      |
| D       | SM712 ×2 + PTC             | TVS por par + fusível resetável                 |
| PWR     | LDO 3V3 + C                | VIN 3,3–5 V                                     |
| JP1/JP2 | Jumpers ROOT/NODE, S/L     | + LED estado (verde=anel, vermelho=falha)       |

## 5. Layout (regras, não sugestões)

1. Pares diferenciais `100 Ω`, mismatch `<2 mm`, sem vias desnecessárias.
2. TVS junto ao conector; GND sólido sem fendas sob os pares.
3. ATECC608 longe dos drivers; I2C curto (`<20 mm`) com pull-ups 4k7.
4. Série `22 Ω` em cada saída CLK (limita corrente de contenção transitória se
   dois `OE_CLK` se sobrepuserem na eleição).
5. Silk obrigatória: `120R ON SÓ EM BG-0 (MODO L)` + seta AFTER→ (sentido do relógio).

## 6. Rádio (a ponte não tem rádio)

Saída `UART COBS` (115200–921600) + `IRQ/READY` para qualquer modem
(LoRa SX1262, ESP-NOW, Wi-Fi UDP, FSK). O gateway conta como 1 endereço virtual;
o relógio regenera-se localmente. Ambos os extremos com ponte + mesma sessão,
senão o `TAG` descarta.

## 7. Bring-up (5 passos, sem osciloscópio caro)

1. Alimenta 1 ponte: LED deve piscar `ROOT?` (jumper NODE sem anel = `Island`).
2. Liga 2 pontes AFTER→BEFORE sem fechar: `Hello` numera 1, 2 (UART debug).
3. Fecha o anel na 2.ª porta do BG-0: `Count{N}` + LED verde nas duas.
4. Corta 1 cabo: `RING_OPEN` + tráfego segue pelo lado vivo.
5. Troca S/L errado: `SAFE 250 kbps` + `MODE_MISMATCH` (nunca silêncio).
