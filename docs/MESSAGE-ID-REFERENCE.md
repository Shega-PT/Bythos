# Bythos v3.0.0 — Referência de MsgIDs

## Mensagens do Sistema (0x10-0x1B)

| MsgID | Nome      | Prioridade por defeito | Descrição                               |
|:-----:|-----------|:----------------------:|-----------------------------------------|
| 0x10  | Heartbeat | Medium                 | Sinal de vida, enviado periodicamente   |
| 0x11  | Telemetry | Medium                 | Dados de sensores e estado do sistema   |
| 0x12  | Command   | High                   | Instrução enviada a um módulo           |
| 0x13  | Ack       | High                   | Confirmação de receção de mensagem      |
| 0x14  | Failsafe  | SuperCritical          | Estado de segurança de emergência       |
| 0x15  | Debug     | Low                    | Mensagens de depuração                  |
| 0x16  | Video     | Low                    | Dados de vídeo fragmentados             |
| 0x17  | Shell     | Medium                 | Acesso a consola remota                 |
| 0x18  | SiData    | Medium                 | Dados de sensores SI (Sensor Interface) |
| 0x19  | Watchdog  | Medium                 | Keepalive de monitorização              |
| 0x1A  | Ping      | Medium                 | Teste de conectividade                  |
| 0x1B  | Clock     | High                   | Sincronização temporal                  |

## Descrição por Mensagem

### Heartbeat (0x10)
Mensagem periódica enviada por cada módulo para indicar que está operacional.
Campos típicos: `SystemState`, `SystemMode`, `SystemUptime`.

### Telemetry (0x11)
Mensagem com dados completos de sensores.
Campos típicos: GPS, IMU, Energia, Temperatura, Sistema.

### Command (0x12)
Comando enviado a um módulo específico.
Campos típicos: Campo de comando com payload específico.

### Ack (0x13)
Confirmação de que uma mensagem foi recebida e processada.
Campos típicos: ID da mensagem confirmada, estado.

### Failsafe (0x14)
Mensagem de emergência com prioridade SuperCritical.
Campos típicos: `FailsafeReason`, `FailsafeAction`, `FailsafeState`.

### Debug (0x15)
Mensagens de depuração com dados variáveis. Prioridade baixa.

### Video (0x16)
Dados de vídeo fragmentados em chunks.
Campos típicos: `VideoFrameId`, `VideoChunkId`, `VideoTotalChunks`, `VideoPayload`.

### Shell (0x17)
Acesso a consola remota para diagnóstico e configuração.

### SiData (0x18)
Dados de sensores externos (Sensor Interface).

### Watchdog (0x19)
Keepalive de monitorização entre módulos.

### Ping (0x1A)
Teste de conectividade. Resposta esperada: Ack.

### Clock (0x1B)
Sincronização temporal entre módulos.

---

## Prioridades de Mensagem

| Prioridade      | Valor | Descrição                             |
|-----------------|:-----:|---------------------------------------|
| SuperCritical   | 0     | Processamento imediato (failsafe)     |
| Critical        | 1     | Processamento urgente                 |
| High            | 2     | Processamento urgente (comandos, ACK) |
| Medium          | 3     | Processamento padrão (telemetry)      |
| Low             | 4     | Quando disponível (debug, vídeo)      |
