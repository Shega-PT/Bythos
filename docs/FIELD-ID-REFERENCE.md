# Bythos v3.0.0 — Referência de FieldIDs

Autor: ShegaPT | Licença: GPL-3.0

## FieldIDs Codificados por Domínio

### GPS (Tipo 1 = f32, IDs 0x06-0x0B)

| FieldID | Nome            | Tipo | ID  | Descrição            |
|:-------:|-----------------|:----:|:---:|----------------------|
| 0x26    | GpsLatitude     | f32  | 6   | Latitude (graus)     |
| 0x27    | GpsLongitude    | f32  | 7   | Longitude (graus)    |
| 0x28    | GpsAltitude     | f32  | 8   | Altitude GPS (m)     |
| 0x29    | GpsSpeed        | f32  | 9   | Velocidade (m/s)     |
| 0x2A    | GpsCourse       | f32  | 10  | Rumo (graus)         |
| 0x2B    | GpsHdop         | f32  | 11  | HDOP (qualidade)     |
| 0xC7    | GpsSatellites   | u8   | 7   | Número de satélites  |

### IMU (Tipo 1 = f32, IDs 0x10-0x19)

| FieldID | Nome       | Tipo | ID  | Descrição                |
|:-------:|------------|:----:|:---:|--------------------------|
| 0x30    | ImuRoll    | f32  | 16  | Ângulo de roll (graus)   |
| 0x31    | ImuPitch   | f32  | 17  | Ângulo de pitch (graus)  |
| 0x32    | ImuYaw     | f32  | 18  | Ângulo de yaw (graus)    |
| 0x33    | ImuAccelX  | f32  | 19  | Aceleração X (m/s²)      |
| 0x34    | ImuAccelY  | f32  | 20  | Aceleração Y (m/s²)      |
| 0x35    | ImuAccelZ  | f32  | 21  | Aceleração Z (m/s²)      |
| 0x36    | ImuGyroX   | f32  | 22  | Giroscópio X (°/s)       |
| 0x37    | ImuGyroY   | f32  | 23  | Giroscópio Y (°/s)       |
| 0x38    | ImuGyroZ   | f32  | 24  | Giroscópio Z (°/s)       |
| 0x39    | ImuYawRate | f32  | 25  | Taxa de yaw (°/s)        |

### Voo (Tipo 1 = f32, IDs 0x20-0x23)

| FieldID | Nome          | Tipo | ID  | Descrição                  |
|:-------:|---------------|:----:|:---:|----------------------------|
| 0x40    | FlightAltGps  | f32  | 32  | Altitude GPS (m)           |
| 0x41    | FlightAltBaro | f32  | 33  | Altitude barométrica (m)   |
| 0x42    | FlightVSpeed  | f32  | 34  | Velocidade vertical (m/s)  |
| 0x43    | FlightAirspeed| f32  | 35  | Velocidade aérea (m/s)     |
| 0xA2    | FlightLoopTime| u16  | 2   | Tempo de loop (µs)         |

### Energia (Tipo 1 = f32, IDs 0x30-0x34)

| FieldID | Nome          | Tipo | ID  | Descrição                  |
|:-------:|---------------|:----:|:---:|----------------------------|
| 0x50    | PowerBattV    | f32  | 48  | Tensão da bateria (V)      |
| 0x51    | PowerBattI    | f32  | 49  | Corrente da bateria (A)    |
| 0x52    | PowerBattCons | f32  | 50  | Consumo total (mAh)        |
| 0x53    | PowerBattTemp | f32  | 51  | Temperatura da bateria (°C)|
| 0x54    | PowerBattSoc  | f32  | 52  | Estado de carga (%)        |

### Temperatura (Tipo 1 = f32, IDs 0x40-0x45)

| FieldID | Nome     | Tipo | ID  | Descrição                    |
|:-------:|----------|:----:|:---:|------------------------------|
| 0x60    | Temp1    | f32  | 64  | Sensor de temperatura 1 (°C) |
| 0x61    | Temp2    | f32  | 65  | Sensor de temperatura 2 (°C) |
| 0x62    | Temp3    | f32  | 66  | Sensor de temperatura 3 (°C) |
| 0x63    | Temp4    | f32  | 67  | Sensor de temperatura 4 (°C) |
| 0x64    | TempEsp1 | f32  | 68  | Temperatura Device1 (°C)     |
| 0x65    | TempEsp2 | f32  | 69  | Temperatura Device2 (°C)     |

### Sistema (Tipos mistos)

| FieldID | Nome          | Tipo | ID  | Descrição                    |
|:-------:|---------------|:----:|:---:|------------------------------|
| 0xC0    | SystemState   | u8   | 0   | Estado do sistema (enum)     |
| 0xC1    | SystemMode    | u8   | 1   | Modo de voo (enum)           |
| 0x82    | SystemUptime  | u32  | 2   | Tempo de atividade (s)       |
| 0x83    | SystemFreeHeap| u32  | 3   | Memória livre (bytes)        |
| 0xC4    | SystemCpuLoad | u8   | 4   | Carga da CPU (%)             |
| 0xC5    | SystemDev1Load| u8   | 5   | Carga Device1 (%)            |
| 0xC6    | SystemDev2Load| u8   | 6   | Carga Device2 (%)            |

### Failsafe

| FieldID | Nome           | Tipo | ID  | Descrição                     |
|:-------:|----------------|:----:|:---:|-------------------------------|
| 0xC8    | FailsafeReason | u8   | 8   | Motivo do failsafe (enum)     |
| 0xC9    | FailsafeAction | u8   | 9   | Ação do failsafe (enum)       |
| 0xCA    | FailsafeState  | u8   | 10  | Estado do failsafe            |

### Vídeo

| FieldID | Nome           | Tipo   | ID  | Descrição                     |
|:-------:|----------------|:------:|:---:|-------------------------------|
| 0xA0    | VideoFrameId   | u16    | 0   | ID do frame de vídeo          |
| 0xC3    | VideoChunkId   | u8     | 3   | ID do chunk                   |
| 0xCB    | VideoTotalChunks| u8    | 11  | Total de chunks               |
| 0x00    | VideoPayload   | raw    | 0   | Payload de vídeo (variável)   |

---

## Tabela de FieldIDs por Tipo

```text
Tipo 0 (Raw):    0x00-0x1F (32 slots)
Tipo 1 (f32):    0x20-0x3F (32 slots)
Tipo 2 (f16):    0x40-0x5F (32 slots)
Tipo 3 (i32):    0x60-0x7F (32 slots)
Tipo 4 (u32):    0x80-0x9F (32 slots)
Tipo 5 (u16):    0xA0-0xBF (32 slots)
Tipo 6 (u8):     0xC0-0xDF (32 slots)
Tipo 7 (Bool):   0xE0-0xFF (32 slots)
```
