# Bythos v3.0.0 — MsgID Reference

## System Messages (0x10-0x1B)

| MsgID | Name      | Default Priority  | Description                                     |
|:-----:|-----------|:-----------------:|-----------------------------------------------|
| 0x10  | Heartbeat | Medium            | Life signal, sent periodically                |
| 0x11  | Telemetry | Medium            | Sensor and system state data                  |
| 0x12  | Command   | High              | Instruction sent to a module                  |
| 0x13  | Ack       | High              | Message reception acknowledgment              |
| 0x14  | Failsafe  | SuperCritical     | Emergency safety state                        |
| 0x15  | Debug     | Low               | Debug messages                                |
| 0x16  | Video     | Low               | Fragmented video data                         |
| 0x17  | Shell     | Medium            | Remote console access                         |
| 0x18  | SiData    | Medium            | Sensor Interface data                         |
| 0x19  | Watchdog  | Medium            | Monitoring keepalive                          |
| 0x1A  | Ping      | Medium            | Connectivity test                             |
| 0x1B  | Clock     | High              | Temporal synchronization                      |

## Message Descriptions

### Heartbeat (0x10)
Periodic message sent by each module to indicate it is operational.
Typical fields: `SystemState`, `SystemMode`, `SystemUptime`.

### Telemetry (0x11)
Message with complete sensor data.
Typical fields: GPS, IMU, Power, Temperature, System.

### Command (0x12)
Command sent to a specific module.
Typical fields: Command field with specific payload.

### Ack (0x13)
Acknowledgment that a message was received and processed.
Typical fields: ID of acknowledged message, state.

### Failsafe (0x14)
Emergency message with SuperCritical priority.
Typical fields: `FailsafeReason`, `FailsafeAction`, `FailsafeState`.

### Debug (0x15)
Debug messages with variable data. Low priority.

### Video (0x16)
Video data fragmented into chunks.
Typical fields: `VideoFrameId`, `VideoChunkId`, `VideoTotalChunks`, `VideoPayload`.

### Shell (0x17)
Remote console access for diagnostics and configuration.

### SiData (0x18)
External sensor data (Sensor Interface).

### Watchdog (0x19)
Monitoring keepalive between modules.

### Ping (0x1A)
Connectivity test. Expected response: Ack.

### Clock (0x1B)
Temporal synchronization between modules.

---

## Message Priorities

| Priority      | Value | Description                              |
|---------------|:-----:|----------------------------------------|
| SuperCritical | 0     | Immediate processing (failsafe)        |
| Critical      | 1     | Urgent processing                      |
| High          | 2     | Urgent processing (commands, ACK)      |
| Medium        | 3     | Standard processing (telemetry)        |
| Low           | 4     | When available (debug, video)          |
