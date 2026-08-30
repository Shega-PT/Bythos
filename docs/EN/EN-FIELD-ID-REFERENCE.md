# Bythos v3.0.0 — FieldID Reference

## FieldIDs Encoded by Domain

### GPS (Type 1 = f32, IDs 0x06-0x0B)

| FieldID | Name            | Type | ID  | Description            |
|:-------:|-----------------|:----:|:---:|------------------------|
| 0x26    | GpsLatitude     | f32  | 6   | Latitude (degrees)     |
| 0x27    | GpsLongitude    | f32  | 7   | Longitude (degrees)    |
| 0x28    | GpsAltitude     | f32  | 8   | GPS altitude (m)       |
| 0x29    | GpsSpeed        | f32  | 9   | Speed (m/s)            |
| 0x2A    | GpsCourse       | f32  | 10  | Course (degrees)       |
| 0x2B    | GpsHdop         | f32  | 11  | HDOP (quality)         |
| 0xC7    | GpsSatellites   | u8   | 7   | Number of satellites   |

### IMU (Type 1 = f32, IDs 0x10-0x19)

| FieldID | Name       | Type | ID  | Description                  |
|:-------:|------------|:----:|:---:|------------------------------|
| 0x30    | ImuRoll    | f32  | 16  | Roll angle (degrees)         |
| 0x31    | ImuPitch   | f32  | 17  | Pitch angle (degrees)        |
| 0x32    | ImuYaw     | f32  | 18  | Yaw angle (degrees)          |
| 0x33    | ImuAccelX  | f32  | 19  | Acceleration X (m/s²)        |
| 0x34    | ImuAccelY  | f32  | 20  | Acceleration Y (m/s²)        |
| 0x35    | ImuAccelZ  | f32  | 21  | Acceleration Z (m/s²)        |
| 0x36    | ImuGyroX   | f32  | 22  | Gyroscope X (°/s)            |
| 0x37    | ImuGyroY   | f32  | 23  | Gyroscope Y (°/s)            |
| 0x38    | ImuGyroZ   | f32  | 24  | Gyroscope Z (°/s)            |
| 0x39    | ImuYawRate | f32  | 25  | Yaw rate (°/s)               |

### Flight (Type 1 = f32, IDs 0x20-0x23)

| FieldID | Name          | Type | ID  | Description                    |
|:-------:|---------------|:----:|:---:|--------------------------------|
| 0x40    | FlightAltGps  | f32  | 32  | GPS altitude (m)               |
| 0x41    | FlightAltBaro | f32  | 33  | Barometric altitude (m)        |
| 0x42    | FlightVSpeed  | f32  | 34  | Vertical speed (m/s)           |
| 0x43    | FlightAirspeed| f32  | 35  | Airspeed (m/s)                 |
| 0xA2    | FlightLoopTime| u16  | 2   | Loop time (µs)                 |

### Power (Type 1 = f32, IDs 0x30-0x34)

| FieldID | Name          | Type | ID  | Description                    |
|:-------:|---------------|:----:|:---:|--------------------------------|
| 0x50    | PowerBattV    | f32  | 48  | Battery voltage (V)            |
| 0x51    | PowerBattI    | f32  | 49  | Battery current (A)            |
| 0x52    | PowerBattCons | f32  | 50  | Total consumption (mAh)        |
| 0x53    | PowerBattTemp | f32  | 51  | Battery temperature (°C)       |
| 0x54    | PowerBattSoc  | f32  | 52  | State of charge (%)            |

### Temperature (Type 1 = f32, IDs 0x40-0x45)

| FieldID | Name     | Type | ID  | Description                      |
|:-------:|----------|:----:|:---:|----------------------------------|
| 0x60    | Temp1    | f32  | 64  | Temperature sensor 1 (°C)        |
| 0x61    | Temp2    | f32  | 65  | Temperature sensor 2 (°C)        |
| 0x62    | Temp3    | f32  | 66  | Temperature sensor 3 (°C)        |
| 0x63    | Temp4    | f32  | 67  | Temperature sensor 4 (°C)        |
| 0x64    | TempEsp1 | f32  | 68  | Device1 temperature (°C)         |
| 0x65    | TempEsp2 | f32  | 69  | Device2 temperature (°C)         |

### System (Mixed types)

| FieldID | Name          | Type | ID  | Description                      |
|:-------:|---------------|:----:|:---:|----------------------------------|
| 0xC0    | SystemState   | u8   | 0   | System state (enum)              |
| 0xC1    | SystemMode    | u8   | 1   | Flight mode (enum)               |
| 0x82    | SystemUptime  | u32  | 2   | Uptime (s)                       |
| 0x83    | SystemFreeHeap| u32  | 3   | Free memory (bytes)              |
| 0xC4    | SystemCpuLoad | u8   | 4   | CPU load (%)                     |
| 0xC5    | SystemDev1Load| u8   | 5   | Device1 load (%)                 |
| 0xC6    | SystemDev2Load| u8   | 6   | Device2 load (%)                 |

### Failsafe

| FieldID | Name           | Type | ID  | Description                       |
|:-------:|----------------|:----:|:---:|-----------------------------------|
| 0xC8    | FailsafeReason | u8   | 8   | Failsafe reason (enum)            |
| 0xC9    | FailsafeAction | u8   | 9   | Failsafe action (enum)            |
| 0xCA    | FailsafeState  | u8   | 10  | Failsafe state                    |

### Video

| FieldID | Name            | Type   | ID  | Description                       |
|:-------:|-----------------|:------:|:---:|-----------------------------------|
| 0xA0    | VideoFrameId    | u16    | 0   | Video frame ID                    |
| 0xC3    | VideoChunkId    | u8     | 3   | Chunk ID                          |
| 0xCB    | VideoTotalChunks| u8     | 11  | Total chunks                      |
| 0x00    | VideoPayload    | raw    | 0   | Video payload (variable)          |

---

## FieldID Table by Type

```text
Type 0 (Raw):    0x00-0x1F (32 slots)
Type 1 (f32):    0x20-0x3F (32 slots)
Type 2 (f16):    0x40-0x5F (32 slots)
Type 3 (i32):    0x60-0x7F (32 slots)
Type 4 (u32):    0x80-0x9F (32 slots)
Type 5 (u16):    0xA0-0xBF (32 slots)
Type 6 (u8):     0xC0-0xDF (32 slots)
Type 7 (Bool):   0xE0-0xFF (32 slots)
```
