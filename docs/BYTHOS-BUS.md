# Bythos Bus — O Barramento (guia do produto)

> Quem conhece CAN bus lê isto em 5 minutos — de propósito. O Bythos Bus copia
> os conceitos que fazem o CAN funcionar (arbitragem por identificador, par
> diferencial, multi-mestre, prioridades) e troca o resto: ficha RJ barata,
> relógio dedicado, segurança em hardware e nomes Bythos. Este documento é a
> referência comercial e técnica do barramento; a norma do fio vive em
> `BYTHOS-SPECIFICATION.md`.

## 1. O que é

O **Bythos Bus** é o barramento físico e lógico que liga as pontes
`BythosBridge` em anel: 1 cabo com 2 pares entrançados (dados + relógio),
fichas RJ de telefone, até 1000+ nós sem configurar endereços, com cada trama
autenticada em hardware. Cada nó precisa da sua ponte — tal como cada nó CAN
precisa do seu transceptor.

```text
BG-0.AFTER ──RJ──► BG-1 ──RJ──► BG-2 ──RJ──► … ──RJ──► BG-0.BEFORE
  (relógio nasce aqui)                              (relógio volta aqui)
```

## 2. Arbitragem: como no CAN, sem o CAN

No CAN, duas tramas que colidem resolvem-se no cobre: o identificador menor
(dominante) ganha sem destruir nenhuma trama. O RS-485 não tem dominante
elétrico, por isso o Bythos Bus faz o mesmo por firmware — e a regra se mantém:

**valor menor ganha; a trama perdedora recua e retransmite intacta.**

O identificador de arbitragem (`BythosBusId`, 32 bits):

```text
Bits 31-29: prioridade (0 = mais urgente)
Bits 28-25: grupo funcional (controlo, sensores, segurança…)
Bits 24-21: espécie de tráfego (dados, comando, segurança…)
Bits 20-5:  posição da origem no anel
Bits 4-0:   reservados
```

Exemplo: uma emergência (`prioridade 0, segurança, origem 4`) produz sempre um
ID menor que telemetria normal (`prioridade 3, sensores, origem 27`) — ganha
qualquer contenção sem configuração, exatamente como uma trama CAN de alta
prioridade. Em igualdade total, a origem mais baixa desempata (determinístico).

Na prática da ponte: CSMA (escutar antes de falar) + backoff ordenado por este
ID. O programador ordena com `<` direto — ver `BythosBusId::wins_over` (Rust)
e `bythos_bus_id_wins` (C).

## 3. Bythos Bus vs CAN bus (para quem compra)

| Aspeto              | CAN bus                        | Bythos Bus                              |
|---------------------|--------------------------------|-----------------------------------------|
| Ficha / cabo        | DB9 / industrial               | RJ de telefone, 2 pares (barato, ubíquo)|
| Fios                | 2 (diferencial)                | 4 (2 pares: dados + relógio dedicado)   |
| Controlador         | Obrigatório (SJA1000, bxCAN…)  | Nenhum — a ponte faz tudo               |
| Sincronização       | Bit-timing negociado           | 1 par de relógio comum (sem negociar)   |
| Endereços           | Declarados (filtros, máscaras) | Automáticos desde o BG-0                |
| Nós                 | ~110 teóricos                  | 1000+ testados em modelo                 |
| Arbitragem          | Dominante elétrico             | Firmware, mesma regra (menor ganha)     |
| Velocidade típica   | 125 kbps–1 Mbps                | 250 kbps–8 Mbps (modo S/L por jumper)   |
| Segurança           | Nenhuma ( SecOC à parte)       | TAG HMAC + anti-replay em chip, sempre   |
| Rádio               | Gateway caro e específico      | Túnel opaco p/ qualquer modem           |
| Preço por nó        | Transceptor + controlador + stack | 1 ponte (~3–6 €)                     |

Frase de venda honesta: *é o CAN que já conhece, sem o controlador, sem
configurar endereços, com segurança ligada por defeito — e a ficha é de telefone.*

## 4. Regras de instalação (as 5 que evitam 90% dos problemas)

1. **Um `ROOT`, um só**: jumper `ROOT` apenas no BG-0. Dois `ROOT` = `WiringFault`.
2. **120R só no BG-0 em modo L** (as 2 portas fecham o anel). Nós do meio sempre OFF.
3. **Todos no mesmo modo S/L** — divergência cai para `SAFE`, nunca corrompe.
4. **Cabo entrançado, nunca flat**: flat de telefone serve a `<2 m`; acima disso,
   2 pares entrançados (ou CAT5e no RJ45 em troços longos).
5. **Cadeia curta, sem estrelas**: stubs `<30 cm`; estrela passiva reflete e mata o sinal.

## 5. Perguntas de cliente (respostas de uma linha)

- *Preciso de controlador CAN?* Não. A ponte é transceptor + lógica + segurança.
- *Quantos nós?* De 2 a 1000+; a enumeração trata disso sozinha.
- *E se cortar um cabo?* O anel vira linha (`RING_OPEN`) e o tráfego segue pelo lado vivo.
- *E a segurança?* Cada trama leva etiqueta HMAC calculada no chip; sem a chave, forjar é ~2³² tentativas em linha.
- *Funciona por rádio?* Sim, com uma ponte em cada extremo e qualquer modem — o rádio transporta, a ponte autentica.
- *Posso misturar comprimentos?* Sim: modo S nos troços curtos, L nos longos, mesmo cabo, jumper por ponte.
