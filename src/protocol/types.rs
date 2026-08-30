//! # Bythos — Tipos e Definições (v3.0.0)
//!
//! Este módulo define todas as constantes, enums e structs utilizados pelo
//! protocolo de comunicação binário Bythos v3.0.0.
//!
//! O protocolo é partilhado por todos os módulos e sistemas, sendo compatível
//! retroativamente (backward-compatible) através de regras semânticas de versão:
//! apenas adicionar novos campos/IDs, nunca alterar os existentes.
//!
//! ## Formato da Mensagem Bythos v3.0.0
//!
//! ```text
//! [START_BYTE][VERSION][NODE_ID][MSG_ID][SEQ_NUM_LO][SEQ_NUM_HI]
//! [TLV_COUNT][TLV_FIELDS...][SIGNATURE][CRC16_LO][CRC16_HI]
//! ```
//!
//! ## Campo TLV — FieldID com Tipo Embutido
//!
//! ```text
//! FieldID = [TYPE:3bits][ID:5bits]
//!
//!   Bits 7-5: Tipo de dado (0-7)
//!   Bits 4-0: ID do campo (0-31)
//! ```
//!
//! ## Endianness
//!
//! Todos os campos multi-byte utilizam little-endian (LE), compatível
//! com arquiteturas embedded (ESP32/Xtensa, ARM, etc.).

// ============================================================================
// CONSTANTES GLOBAIS — Bythos v3.0.0
// ============================================================================

/// Byte de início de cada mensagem Bythos. Usado para sincronização de frame.
pub const START_BYTE: u8 = 0xAA;

/// Versão atual do protocolo Bythos.
pub const BYTHOS_VERSION: u8 = 0x03;

/// Chave de assinatura por defeito (0x00 = sem assinatura).
pub const DEFAULT_SIGNATURE_KEY: u8 = 0x00;

/// Número máximo de bytes de dados em um campo TLV normal.
pub const MAX_TLV_DATA: usize = 32;

/// Número máximo de bytes de dados em um campo TLV de vídeo.
pub const MAX_TLV_VIDEO_DATA: usize = 128;

/// Número máximo de campos TLV por mensagem.
pub const MAX_TLV_FIELDS: usize = 32;

/// Tamanho do cabeçalho da mensagem Bythos (start + version + nodeId + msgId + seq(2) + tlvCount).
pub const BYTHOS_HEADER_SIZE: usize = 7;

/// Tamanho do checksum CRC16 no final da mensagem.
pub const CRC16_SIZE: usize = 2;

/// Tamanho do campo de assinatura.
pub const SIGNATURE_SIZE: usize = 1;

/// Tamanho do cabeçalho de cada campo TLV (id + len).
pub const TLV_HEADER_SIZE: usize = 2;

/// Tamanho total de overhead da mensagem (header + signature + crc16).
/// Calculado como: 7 (header) + 1 (signature) + 2 (crc16) = 10
pub const BYTHOS_OVERHEAD: usize = BYTHOS_HEADER_SIZE + SIGNATURE_SIZE + CRC16_SIZE;

/// Tamanho máximo de uma mensagem Bythos serializada em bytes.
/// Calculado como: BYTHOS_HEADER_SIZE + MAX_TLV_FIELDS * (2 + MAX_TLV_DATA) + SIGNATURE_SIZE + CRC16_SIZE
/// = 7 + 32*(2+32) + 1 + 2 = 7 + 1088 + 3 = 1098
pub const MAX_MESSAGE_SIZE: usize = BYTHOS_HEADER_SIZE
    + MAX_TLV_FIELDS * (TLV_HEADER_SIZE + MAX_TLV_DATA)
    + SIGNATURE_SIZE
    + CRC16_SIZE;

// ============================================================================
// ENUMS — TIPOS DE DADO TLV (3 bits = 8 valores)
// ============================================================================

/// Tipos de dados suportados pelo Bythos.
///
/// Cada campo TLV tem um tipo embutido no FieldID (bits 7-5).
/// O tipo determina a semântica e o tamanho dos dados.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    /// Dados brutos (payload binário, vídeo, etc). Tamanho variável.
    Raw = 0,
    /// Float de 32 bits (sensores GPS, IMU, Flight).
    Float32 = 1,
    /// Float de 16 bits (half-precision, sensores com precisão reduzida).
    Float16 = 2,
    /// Inteiro sinalizado de 32 bits.
    Int32 = 3,
    /// Inteiro sem sinal de 32 bits (contadores, uptime).
    Uint32 = 4,
    /// Inteiro sem sinal de 16 bits (frame ID, counters).
    Uint16 = 5,
    /// Inteiro sem sinal de 8 bits (estado, flags, enum).
    Uint8 = 6,
    /// Booleano (0=false, 1=true).
    Bool = 7,
}

impl FieldType {
    /// Converte um valor u8 (0-7) para FieldType.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Raw),
            1 => Some(Self::Float32),
            2 => Some(Self::Float16),
            3 => Some(Self::Int32),
            4 => Some(Self::Uint32),
            5 => Some(Self::Uint16),
            6 => Some(Self::Uint8),
            7 => Some(Self::Bool),
            _ => None,
        }
    }

    /// Retorna o tamanho em bytes padrão para o tipo, ou 0 para Raw (variável).
    pub fn default_size(&self) -> usize {
        match self {
            Self::Raw => 0,
            Self::Float32 => 4,
            Self::Float16 => 2,
            Self::Int32 => 4,
            Self::Uint32 => 4,
            Self::Uint16 => 2,
            Self::Uint8 => 1,
            Self::Bool => 1,
        }
    }
}

// ============================================================================
// ENUMS — IDENTIFICADORES DE MENSAGEM (MsgID)
// ============================================================================

/// Identificadores de mensagem (MsgID).
///
/// Cada tipo de mensagem tem um ID único que define o seu propósito
/// e o conjunto de campos TLV que contém.
///
/// Valores na faixa 0x10-0x1F para mensagens do sistema.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MsgId {
    /// Heartbeat — sinal de vida do módulo, enviado periodicamente.
    Heartbeat = 0x10,
    /// Telemetria — dados de sensores e estado do sistema.
    Telemetry = 0x11,
    /// Comando — instrução enviada ao módulo.
    Command = 0x12,
    /// Confirmação (ACK) — confirmação de receção de mensagem.
    Ack = 0x13,
    /// Failsafe — estado de segurança de emergência.
    Failsafe = 0x14,
    /// Debug — mensagens de depuração.
    Debug = 0x15,
    /// Vídeo — dados de vídeo fragmentados.
    Video = 0x16,
    /// Shell — acesso a consola remota.
    Shell = 0x17,
    /// Dados de sensores SI (Sensor Interface).
    SiData = 0x18,
    /// Watchdog — keepalive de monitorização.
    Watchdog = 0x19,
    /// Ping — teste de conectividade.
    Ping = 0x1A,
    /// Clock — sincronização temporal.
    Clock = 0x1B,
}

impl MsgId {
    /// Converte um valor u8 para MsgId, retornando None se inválido.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x10 => Some(Self::Heartbeat),
            0x11 => Some(Self::Telemetry),
            0x12 => Some(Self::Command),
            0x13 => Some(Self::Ack),
            0x14 => Some(Self::Failsafe),
            0x15 => Some(Self::Debug),
            0x16 => Some(Self::Video),
            0x17 => Some(Self::Shell),
            0x18 => Some(Self::SiData),
            0x19 => Some(Self::Watchdog),
            0x1A => Some(Self::Ping),
            0x1B => Some(Self::Clock),
            _ => None,
        }
    }

    /// Retorna true se o ID da mensagem é válido.
    pub fn is_valid(id: u8) -> bool {
        Self::from_u8(id).is_some()
    }
}

// ============================================================================
// ENUMS — NÍVEIS DE PRIORIDADE
// ============================================================================

/// Níveis de prioridade para ordenação de mensagens.
///
/// Valores menores indicam prioridade mais alta.
/// Mapeia diretamente para os 3 bits de prioridade no CAN ID.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PriorityLevel {
    /// Crítico — processamento imediato, não pode ser adiado.
    SuperCritical = 0,
    /// Crítico — processamento urgente, precede todas as tarefas normais.
    Critical = 1,
    /// Alta — processamento urgente, precede tarefas normais.
    High = 2,
    /// Normal — processamento padrão.
    Medium = 3,
    /// Baixa — processamento quando disponível.
    Low = 4,
}

impl PriorityLevel {
    /// Converte um valor u8 para PriorityLevel.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::SuperCritical,
            1 => Self::Critical,
            2 => Self::High,
            3 => Self::Medium,
            _ => Self::Low,
        }
    }
}

// ============================================================================
// ENUMS — ESTADO DO SISTEMA
// ============================================================================

/// Estados possíveis do sistema.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemState {
    Booting = 0,
    Initializing = 1,
    Ready = 2,
    Armed = 3,
    InFlight = 4,
    Landing = 5,
    Error = 6,
    Shutdown = 7,
}

impl SystemState {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Booting),
            1 => Some(Self::Initializing),
            2 => Some(Self::Ready),
            3 => Some(Self::Armed),
            4 => Some(Self::InFlight),
            5 => Some(Self::Landing),
            6 => Some(Self::Error),
            7 => Some(Self::Shutdown),
            _ => None,
        }
    }
}

// ============================================================================
// ENUMS — MODO DE VOO (UAV/UAS)
// ============================================================================

/// Modos de voo disponíveis.
///
/// **UAV/UAS específico** — Este enum é preservado no core do Bythos
/// para compatibilidade com sistemas de voo não tripulado (UAV/UAS).
/// Aplicações que não sejam UAV/UAS podem ignorar este enum.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightMode {
    Manual = 0,
    Stabilize = 1,
    AltHold = 2,
    Auto = 3,
    Guided = 4,
    Rtl = 5,
}

impl FlightMode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Manual),
            1 => Some(Self::Stabilize),
            2 => Some(Self::AltHold),
            3 => Some(Self::Auto),
            4 => Some(Self::Guided),
            5 => Some(Self::Rtl),
            _ => None,
        }
    }
}

// ============================================================================
// ENUMS — MOTIVO E AÇÃO DE FAILSAFE
// ============================================================================

/// Motivos para ativação do failsafe.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailsafeReason {
    None = 0,
    SignalLost = 1,
    LowBattery = 2,
    GpsLost = 3,
    SensorFailure = 4,
    ManualTrigger = 5,
}

impl FailsafeReason {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::SignalLost),
            2 => Some(Self::LowBattery),
            3 => Some(Self::GpsLost),
            4 => Some(Self::SensorFailure),
            5 => Some(Self::ManualTrigger),
            _ => None,
        }
    }
}

/// Ações a tomar quando o failsafe é ativado.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailsafeAction {
    None = 0,
    Hover = 1,
    Land = 2,
    Rtl = 3,
    Continue = 4,
    Disarm = 5,
}

impl FailsafeAction {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Hover),
            2 => Some(Self::Land),
            3 => Some(Self::Rtl),
            4 => Some(Self::Continue),
            5 => Some(Self::Disarm),
            _ => None,
        }
    }
}

// ============================================================================
// ENUMS — GRUPOS COMPUTACIONAIS (CAN)
// ============================================================================

/// Identificadores de grupo no bus CAN.
///
/// Cada módulo/hardware do sistema pertence a um grupo computacional.
/// O grupo é utilizado no CAN ID extended (29-bit) para roteamento.
///
/// Nomes genéricos (Device0..Device14) para uso em qualquer sistema.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanGroup {
    /// Nenhum grupo / broadcast.
    None = 0x0,
    /// Grupo de dispositivo genérico 0.
    Device0 = 0x1,
    /// Grupo de dispositivo genérico 1.
    Device1 = 0x2,
    /// Grupo de dispositivo genérico 2.
    Device2 = 0x3,
    /// Grupo de dispositivo genérico 3.
    Device3 = 0x4,
    /// Grupo de dispositivo genérico 4.
    Device4 = 0x5,
    /// Grupo de dispositivo genérico 5.
    Device5 = 0x6,
    /// Grupo de dispositivo genérico 6.
    Device6 = 0x7,
    /// Grupo de dispositivo genérico 7.
    Device7 = 0x8,
    /// Grupo de dispositivo genérico 8.
    Device8 = 0x9,
    /// Grupo de dispositivo genérico 9.
    Device9 = 0xA,
    /// Grupo de dispositivo genérico 10.
    Device10 = 0xB,
    /// Grupo de dispositivo genérico 11.
    Device11 = 0xC,
    /// Grupo de dispositivo genérico 12.
    Device12 = 0xD,
    /// Grupo de dispositivo genérico 13.
    Device13 = 0xE,
    /// Grupo de dispositivo genérico 14.
    Device14 = 0xF,
}

impl CanGroup {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::None),
            0x1 => Some(Self::Device0),
            0x2 => Some(Self::Device1),
            0x3 => Some(Self::Device2),
            0x4 => Some(Self::Device3),
            0x5 => Some(Self::Device4),
            0x6 => Some(Self::Device5),
            0x7 => Some(Self::Device6),
            0x8 => Some(Self::Device7),
            0x9 => Some(Self::Device8),
            0xA => Some(Self::Device9),
            0xB => Some(Self::Device10),
            0xC => Some(Self::Device11),
            0xD => Some(Self::Device12),
            0xE => Some(Self::Device13),
            0xF => Some(Self::Device14),
            _ => None,
        }
    }
}

// ============================================================================
// ENUMS — TIPOS DE MENSAGEM CAN
// ============================================================================

/// Tipos de mensagem no CAN ID extended (4 bits).
///
/// Utilizados no campo TIPO_MSG do CAN ID (bits 17-14).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanMsgType {
    /// Dados de telemetria / sensores.
    Data = 0x0,
    /// Comandos.
    Cmd = 0x1,
    /// Confirmação de receção (ACK).
    Ack = 0x2,
    /// Eventos / failsafe.
    Event = 0x3,
    /// Sincronização temporal.
    Sync = 0x4,
    /// Broadcast de estado.
    State = 0x5,
    /// Heartbeat.
    Heart = 0x6,
    /// Dados de segurança.
    Safety = 0x7,
}

impl CanMsgType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::Data),
            0x1 => Some(Self::Cmd),
            0x2 => Some(Self::Ack),
            0x3 => Some(Self::Event),
            0x4 => Some(Self::Sync),
            0x5 => Some(Self::State),
            0x6 => Some(Self::Heart),
            0x7 => Some(Self::Safety),
            _ => None,
        }
    }
}

// ============================================================================
// FUNÇÕES — CAN ID
// ============================================================================

/// Constrói um CAN ID extended (29-bit) a partir dos seus componentes.
///
/// Formato:
/// ```text
/// [PRIORIDADE:3][GRUPO_ORIGEM:4][GRUPO_DESTINO:4][TIPO_MSG:4][RESERVADO:14]
/// ```
///
/// # Arguments
/// * `priority` — Nível de prioridade (0-4).
/// * `src_group` — Grupo de origem (0x0-0xF).
/// * `dst_group` — Grupo de destino (0x0= broadcast).
/// * `msg_type` — Tipo de mensagem (0x0-0x7).
///
/// # Returns
/// CAN ID de 29 bits.
pub fn make_can_id(priority: u8, src_group: u8, dst_group: u8, msg_type: u8) -> u32 {
    let mut can_id: u32 = 0;
    can_id |= ((priority as u32) & 0x07) << 26;
    can_id |= ((src_group as u32) & 0x0F) << 22;
    can_id |= ((dst_group as u32) & 0x0F) << 18;
    can_id |= ((msg_type as u32) & 0x0F) << 14;
    can_id
}

/// Extrai a prioridade de um CAN ID extended.
pub fn can_id_priority(can_id: u32) -> u8 {
    ((can_id >> 26) & 0x07) as u8
}

/// Extrai o grupo de origem de um CAN ID extended.
pub fn can_id_src_group(can_id: u32) -> u8 {
    ((can_id >> 22) & 0x0F) as u8
}

/// Extrai o grupo de destino de um CAN ID extended.
pub fn can_id_dst_group(can_id: u32) -> u8 {
    ((can_id >> 18) & 0x0F) as u8
}

/// Extrai o tipo de mensagem de um CAN ID extended.
pub fn can_id_msg_type(can_id: u32) -> u8 {
    ((can_id >> 14) & 0x0F) as u8
}

/// Verifica se o CAN ID é do bus de segurança (msg_type == Safety).
pub fn is_safety_bus_id(can_id: u32) -> bool {
    can_id_msg_type(can_id) == CanMsgType::Safety as u8
}

// ============================================================================
// FUNÇÕES — FIELDID COM TIPO EMBUTIDO
// ============================================================================

/// Codifica um FieldID com tipo embutido (3 bits tipo + 5 bits id).
///
/// # Arguments
/// * `field_type` — Tipo de dado (0-7).
/// * `field_id` — Identificador do campo dentro do tipo (0-31).
///
/// # Returns
/// FieldID codificado: `[TYPE:3][ID:5]`
pub fn field_id_encode(field_type: u8, field_id: u8) -> u8 {
    ((field_type & 0x07) << 5) | (field_id & 0x1F)
}

/// Decodifica um FieldID nos seus componentes (tipo e id).
///
/// # Arguments
/// * `field_id` — FieldID codificado.
///
/// # Returns
/// Tuplo (tipo, id) onde tipo ∈ [0,7] e id ∈ [0,31].
pub fn field_id_decode(field_id: u8) -> (u8, u8) {
    let field_type = (field_id >> 5) & 0x07;
    let id = field_id & 0x1F;
    (field_type, id)
}

/// Valida se um FieldID codificado tem um tipo válido.
pub fn is_valid_field_id(field_id: u8) -> bool {
    let (field_type, _) = field_id_decode(field_id);
    FieldType::from_u8(field_type).is_some()
}

// ============================================================================
// FUNÇÕES — ASSINATURA (XOR Key)
// ============================================================================

/// Calcula a assinatura de uma mensagem Bythos.
///
/// A assinatura é calculada como XOR de:
/// - `key` — Chave partilhada do nó transmissor
/// - `msg_id` — ID da mensagem
/// - `seq_lo` — Byte baixo do número de sequência
/// - `seq_hi` — Byte alto do número de sequência
///
/// # Arguments
/// * `key` — Chave partilhada (1 byte).
/// * `msg_id` — ID da mensagem.
/// * `seq_lo` — Byte baixo do SEQ_NUM.
/// * `seq_hi` — Byte alto do SEQ_NUM.
///
/// # Returns
/// Byte de assinatura (0x00-0xFF).
pub fn compute_signature(key: u8, msg_id: u8, seq_lo: u8, seq_hi: u8) -> u8 {
    key ^ msg_id ^ seq_lo ^ seq_hi
}

/// Valida a assinatura de uma mensagem Bythos.
///
/// # Arguments
/// * `signature` — Assinatura recebida.
/// * `key` — Chave partilhada esperada.
/// * `msg_id` — ID da mensagem.
/// * `seq_lo` — Byte baixo do SEQ_NUM.
/// * `seq_hi` — Byte alto do SEQ_NUM.
///
/// # Returns
/// `true` se a assinatura é válida, `false` caso contrário.
pub fn validate_signature(signature: u8, key: u8, msg_id: u8, seq_lo: u8, seq_hi: u8) -> bool {
    signature == compute_signature(key, msg_id, seq_lo, seq_hi)
}

// ============================================================================
// ENUMS — CAMPOS TLV (GPS, IMU, Voo, Energia, Temperatura, Sistema, etc.)
// ============================================================================

/// Identificadores de campos TLV para dados GPS.
///
/// Cada campo tem um tipo associado (f32 para a maioria).
/// Faixa de IDs: 0x20-0x2F (tipo=1=f32, id=0x00-0x0F).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldGps {
    Latitude = 0x20,
    Longitude = 0x21,
    Altitude = 0x22,
    Speed = 0x23,
    Course = 0x24,
    Satellites = 0x25,
    Hdop = 0x26,
}

/// Identificadores de campos TLV para dados IMU (Unidade de Medição Inercial).
///
/// Faixa de IDs: 0x30-0x3F (tipo=1=f32, id=0x10-0x1F).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldImu {
    Roll = 0x30,
    Pitch = 0x31,
    Yaw = 0x32,
    AccelX = 0x33,
    AccelY = 0x34,
    AccelZ = 0x35,
    GyroX = 0x36,
    GyroY = 0x37,
    GyroZ = 0x38,
    YawRate = 0x39,
}

/// Identificadores de campos TLV para dados de voo.
///
/// Faixa de IDs: 0x40-0x4F (tipo=1=f32, id=0x20-0x2F).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldFlight {
    AltGps = 0x40,
    AltBaro = 0x41,
    VSpeed = 0x42,
    Airspeed = 0x43,
    LoopTime = 0x44,
}

/// Identificadores de campos TLV para dados de energia.
///
/// Faixa de IDs: 0x50-0x5F (tipo=1=f32, id=0x30-0x3F).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldPower {
    BattVoltage = 0x50,
    BattCurrent = 0x51,
    BattConsumed = 0x52,
    BattTemp = 0x53,
    BattSoc = 0x54,
}

/// Identificadores de campos TLV para dados de temperatura.
///
/// Faixa de IDs: 0x60-0x6F (tipo=1=f32, id=0x40-0x4F).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldTemp {
    Temp1 = 0x60,
    Temp2 = 0x61,
    Temp3 = 0x62,
    Temp4 = 0x63,
    Esp1Temp = 0x64,
    Esp2Temp = 0x65,
}

/// Identificadores de campos TLV para dados de sistema.
///
/// Faixa de IDs: 0x70-0x7F (mistura de tipos: u8, u32).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldSystem {
    State = 0x70,
    Mode = 0x71,
    Uptime = 0x72,
    FreeHeap = 0x73,
    CpuLoad = 0x74,
    Esp1Load = 0x75,
    Esp2Load = 0x76,
}

/// Identificadores de campos TLV para dados de failsafe.
///
/// Faixa de IDs: 0xA1-0xAF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldFailsafe {
    Reason = 0xA1,
    Action = 0xA2,
    State = 0xA3,
}

/// Identificadores de campos TLV para dados de vídeo.
///
/// Faixa de IDs: 0xB0-0xBF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldVideo {
    FrameId = 0xB0,
    ChunkId = 0xB1,
    TotalChunks = 0xB2,
    Payload = 0xB3,
}

// ============================================================================
// ENUMS — FIELDID UNIFICADO (com tipo embutido)
// ============================================================================

/// Unificação de todos os identificadores de campos TLV.
///
/// O FieldID codificado inclui o tipo de dado (3 bits) e o ID (5 bits).
/// Este enum fornece constantes pré-codificadas para uso direto.
///
/// Formato: `[TYPE:3][ID:5]`
/// - Tipo 0 (Raw): IDs 0x00-0x1F → FieldIDs 0x00-0x1F
/// - Tipo 1 (f32): IDs 0x00-0x1F → FieldIDs 0x20-0x3F
/// - Tipo 2 (f16): IDs 0x00-0x1F → FieldIDs 0x40-0x5F
/// - Tipo 3 (i32): IDs 0x00-0x1F → FieldIDs 0x60-0x7F
/// - Tipo 4 (u32): IDs 0x00-0x1F → FieldIDs 0x80-0x9F
/// - Tipo 5 (u16): IDs 0x00-0x1F → FieldIDs 0xA0-0xBF
/// - Tipo 6 (u8):  IDs 0x00-0x1F → FieldIDs 0xC0-0xDF
/// - Tipo 7 (bool):IDs 0x00-0x1F → FieldIDs 0xE0-0xFF
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    // GPS — Tipo 1 (f32), IDs 0x06-0x0C
    GpsLatitude = 0x26,   // TYPE=1(f32) + ID=6
    GpsLongitude = 0x27,  // TYPE=1(f32) + ID=7
    GpsAltitude = 0x28,   // TYPE=1(f32) + ID=8
    GpsSpeed = 0x29,      // TYPE=1(f32) + ID=9
    GpsCourse = 0x2A,     // TYPE=1(f32) + ID=10
    GpsSatellites = 0xC7, // TYPE=6(u8)  + ID=7
    GpsHdop = 0x2B,       // TYPE=1(f32) + ID=11

    // IMU — Tipo 1 (f32), IDs 0x10-0x19
    ImuRoll = 0x30,       // TYPE=1(f32) + ID=0x10
    ImuPitch = 0x31,      // TYPE=1(f32) + ID=0x11
    ImuYaw = 0x32,        // TYPE=1(f32) + ID=0x12
    ImuAccelX = 0x33,     // TYPE=1(f32) + ID=0x13
    ImuAccelY = 0x34,     // TYPE=1(f32) + ID=0x14
    ImuAccelZ = 0x35,     // TYPE=1(f32) + ID=0x15
    ImuGyroX = 0x36,      // TYPE=1(f32) + ID=0x16
    ImuGyroY = 0x37,      // TYPE=1(f32) + ID=0x17
    ImuGyroZ = 0x38,      // TYPE=1(f32) + ID=0x18
    ImuYawRate = 0x39,    // TYPE=1(f32) + ID=0x19

    // Voo — Tipo 1 (f32), IDs 0x20-0x24
    FlightAltGps = 0x40,  // TYPE=1(f32) + ID=0x20
    FlightAltBaro = 0x41, // TYPE=1(f32) + ID=0x21
    FlightVSpeed = 0x42,  // TYPE=1(f32) + ID=0x22
    FlightAirspeed = 0x43, // TYPE=1(f32) + ID=0x23
    FlightLoopTime = 0xA2, // TYPE=5(u16) + ID=2

    // Energia — Tipo 1 (f32), IDs 0x30-0x34
    PowerBattV = 0x50,    // TYPE=1(f32) + ID=0x30
    PowerBattI = 0x51,    // TYPE=1(f32) + ID=0x31
    PowerBattCons = 0x52, // TYPE=1(f32) + ID=0x32
    PowerBattTemp = 0x53, // TYPE=1(f32) + ID=0x33
    PowerBattSoc = 0x54,  // TYPE=1(f32) + ID=0x34

    // Temperatura — Tipo 1 (f32), IDs 0x40-0x45
    Temp1 = 0x60,         // TYPE=1(f32) + ID=0x40
    Temp2 = 0x61,         // TYPE=1(f32) + ID=0x41
    Temp3 = 0x62,         // TYPE=1(f32) + ID=0x42
    Temp4 = 0x63,         // TYPE=1(f32) + ID=0x43
    TempEsp1 = 0x64,      // TYPE=1(f32) + ID=0x44
    TempEsp2 = 0x65,      // TYPE=1(f32) + ID=0x45

    // Sistema — Tipos mistos (u8, u32)
    SystemState = 0xC0,   // TYPE=6(u8)  + ID=0
    SystemMode = 0xC1,    // TYPE=6(u8)  + ID=1
    SystemUptime = 0x82,  // TYPE=4(u32) + ID=2
    SystemFreeHeap = 0x83, // TYPE=4(u32) + ID=3
    SystemCpuLoad = 0xC4, // TYPE=6(u8)  + ID=4
    SystemEsp1Load = 0xC5, // TYPE=6(u8)  + ID=5
    SystemEsp2Load = 0xC6, // TYPE=6(u8)  + ID=6

    // Failsafe — Tipos mistos (u8)
    FailsafeReason = 0xC8, // TYPE=6(u8) + ID=8
    FailsafeAction = 0xC9, // TYPE=6(u8) + ID=9
    FailsafeState = 0xCA,  // TYPE=6(u8) + ID=10

    // Vídeo — Tipos mistos (u16, u8, raw)
    VideoFrameId = 0xA0,   // TYPE=5(u16) + ID=0
    VideoChunkId = 0xC3,   // TYPE=6(u8)  + ID=3
    VideoTotalChunks = 0xCB, // TYPE=6(u8) + ID=11
    VideoPayload = 0x00,   // TYPE=0(raw) + ID=0
}

/// Comandos básicos do sistema.
///
/// Faixa de IDs: 0xC0-0xCF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandBasic {
    Arm = 0xC0,
    Disarm = 0xC1,
    SetMode = 0xC2,
    EmergencyStop = 0xC3,
    Shutdown = 0xC4,
}

/// Comandos de controlo de voo.
///
/// Faixa de IDs: 0xD0-0xDF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandControl {
    SetAltTarget = 0xD0,
    SetSpeed = 0xD1,
    SetPitch = 0xD2,
    SetRoll = 0xD3,
    SetYaw = 0xD4,
    SetHeading = 0xD5,
}

/// Comandos avançados de diagnóstico.
///
/// Faixa de IDs: 0xE0-0xEF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandAdvanced {
    SensorCalib = 0xE0,
    StartLog = 0xE1,
    StopLog = 0xE2,
    GetAll = 0xE3,
}

/// Comandos de navegação.
///
/// Faixa de IDs: 0xF0-0xFF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandNav {
    NextWaypoints = 0xF0,
    SetReturnPoint = 0xF1,
    SetPosition = 0xF2,
}

/// Unificação de todos os comandos.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Arm = 0xC0,
    Disarm = 0xC1,
    SetMode = 0xC2,
    EmergencyStop = 0xC3,
    Shutdown = 0xC4,
    SetAltTarget = 0xD0,
    SetSpeed = 0xD1,
    SetPitch = 0xD2,
    SetRoll = 0xD3,
    SetYaw = 0xD4,
    SetHeading = 0xD5,
    SensorCalib = 0xE0,
    StartLog = 0xE1,
    StopLog = 0xE2,
    GetAll = 0xE3,
    NextWaypoints = 0xF0,
    SetReturnPoint = 0xF1,
    SetPosition = 0xF2,
}

// ============================================================================
// STRUCTS — CAMPOS E MENSAGENS TLV
// ============================================================================

/// Campo TLV (Type-Length-Value) genérico.
///
/// Representa um campo individual dentro de uma mensagem Bythos.
/// O array `data` contém até `MAX_TLV_DATA` bytes de dados.
///
/// **Importante:** Este struct utiliza `#[repr(C)]` para garantir layout
/// compatível com C/C++ através de FFI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TLVField {
    /// Identificador do tipo do campo (com tipo embutido: [TYPE:3][ID:5]).
    pub id: u8,
    /// Número de bytes de dados válidos no array `data`.
    pub len: u8,
    /// Dados do campo. Apenas os primeiros `len` bytes são válidos.
    pub data: [u8; MAX_TLV_DATA],
}

impl TLVField {
    /// Cria um novo campo TLV vazio.
    pub fn new() -> Self {
        Self {
            id: 0,
            len: 0,
            data: [0u8; MAX_TLV_DATA],
        }
    }

    /// Cria um novo campo TLV com o ID e dados especificados.
    pub fn with_data(id: u8, data: &[u8]) -> Self {
        let mut field = Self::new();
        field.id = id;
        let len = data.len().min(MAX_TLV_DATA);
        field.len = len as u8;
        field.data[..len].copy_from_slice(&data[..len]);
        field
    }

    /// Cria um campo TLV com tipo e ID separados.
    pub fn with_type_data(field_type: FieldType, id: u8, data: &[u8]) -> Self {
        let encoded_id = field_id_encode(field_type as u8, id);
        Self::with_data(encoded_id, data)
    }

    /// Retorna o tipo de dado do campo (bits 7-5 do ID).
    pub fn field_type(&self) -> Option<FieldType> {
        let (t, _) = field_id_decode(self.id);
        FieldType::from_u8(t)
    }

    /// Retorna o ID lógico do campo (bits 4-0 do ID).
    pub fn field_id(&self) -> u8 {
        let (_, id) = field_id_decode(self.id);
        id
    }
}

impl Default for TLVField {
    fn default() -> Self {
        Self::new()
    }
}

/// Campo TLV específico para dados de vídeo.
///
/// Utiliza `MAX_TLV_VIDEO_DATA` (128 bytes) em vez de `MAX_TLV_DATA` (32 bytes)
/// para acomodar payloads de vídeo maiores.
///
/// **Importante:** Este struct utiliza `#[repr(C)]` para garantir layout
/// compatível com C/C++ através de FFI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TLVVideoField {
    /// Identificador do tipo do campo.
    pub id: u8,
    /// Número de bytes de dados válidos no array `data`.
    pub len: u8,
    /// Dados do campo de vídeo.
    pub data: [u8; MAX_TLV_VIDEO_DATA],
}

impl TLVVideoField {
    pub fn new() -> Self {
        Self {
            id: 0,
            len: 0,
            data: [0u8; MAX_TLV_VIDEO_DATA],
        }
    }
}

impl Default for TLVVideoField {
    fn default() -> Self {
        Self::new()
    }
}

/// Mensagem Bythos completa (v3.0.0).
///
/// Estrutura principal do protocolo. Contém o cabeçalho Bythos, até `MAX_TLV_FIELDS`
/// campos TLV, um byte de assinatura e checksum CRC16 para integridade.
///
/// Formato na memória:
/// ```text
/// [startByte][version][nodeId][msgId][seqNum(2)][tlvCount]
/// [tlv[0]...tlv[N]][signature][checksum(2)]
/// ```
///
/// **Importante:** Este struct utiliza `#[repr(C)]` para garantir layout
/// compatível com C/C++ através de FFI. O tamanho total é fixo.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TLVMessage {
    /// Byte de início (sempre `START_BYTE` = 0xAA).
    pub start_byte: u8,
    /// Versão do protocolo Bythos (0x03).
    pub version: u8,
    /// ID do nó transmissor (grupo CAN).
    pub node_id: u8,
    /// Identificador do tipo de mensagem.
    pub msg_id: u8,
    /// Número de sequência (16 bits, little-endian).
    pub seq_num: u16,
    /// Número de campos TLV válidos na mensagem.
    pub tlv_count: u8,
    /// Array de campos TLV. Apenas os primeiros `tlv_count` são válidos.
    pub tlvs: [TLVField; MAX_TLV_FIELDS],
    /// Byte de assinatura (XOR key).
    pub signature: u8,
    /// Checksum CRC16 de toda a mensagem.
    pub checksum: u16,
}

impl TLVMessage {
    /// Cria uma nova mensagem Bythos vazia.
    pub fn new() -> Self {
        Self {
            start_byte: START_BYTE,
            version: BYTHOS_VERSION,
            node_id: 0,
            msg_id: 0,
            seq_num: 0,
            tlv_count: 0,
            tlvs: [TLVField::new(); MAX_TLV_FIELDS],
            signature: 0,
            checksum: 0,
        }
    }

    /// Cria uma nova mensagem Bythos com o ID de mensagem e nó especificados.
    pub fn with_params(msg_id: u8, node_id: u8) -> Self {
        Self {
            start_byte: START_BYTE,
            version: BYTHOS_VERSION,
            node_id,
            msg_id,
            seq_num: 0,
            tlv_count: 0,
            tlvs: [TLVField::new(); MAX_TLV_FIELDS],
            signature: 0,
            checksum: 0,
        }
    }

    /// Limpa a mensagem, redefinindo todos os campos para os valores padrão.
    pub fn clear(&mut self) {
        self.start_byte = START_BYTE;
        self.version = BYTHOS_VERSION;
        self.node_id = 0;
        self.msg_id = 0;
        self.seq_num = 0;
        self.tlv_count = 0;
        for tlv in self.tlvs.iter_mut() {
            *tlv = TLVField::new();
        }
        self.signature = 0;
        self.checksum = 0;
    }
}

impl Default for TLVMessage {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FUNÇÕES DE CONVERSÃO DE BYTES (Little-Endian)
// ============================================================================

/// Converte um valor f32 para 4 bytes em formato little-endian.
pub fn float_to_bytes(value: f32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Converte 4 bytes em formato little-endian para f32.
pub fn bytes_to_float(bytes: &[u8; 4]) -> f32 {
    f32::from_le_bytes(*bytes)
}

/// Converte um valor i32 para 4 bytes em formato little-endian.
pub fn int32_to_bytes(value: i32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Converte 4 bytes em formato little-endian para i32.
pub fn bytes_to_int32(bytes: &[u8; 4]) -> i32 {
    i32::from_le_bytes(*bytes)
}

/// Converte um valor u32 para 4 bytes em formato little-endian.
pub fn uint32_to_bytes(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Converte 4 bytes em formato little-endian para u32.
pub fn bytes_to_uint32(bytes: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*bytes)
}

/// Converte um valor u16 para 2 bytes em formato little-endian.
pub fn uint16_to_bytes(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

/// Converte 2 bytes em formato little-endian para u16.
pub fn bytes_to_uint16(bytes: &[u8; 2]) -> u16 {
    u16::from_le_bytes(*bytes)
}

// ============================================================================
// FUNÇÕES DE VALIDAÇÃO
// ============================================================================

/// Retorna true se o ID da mensagem é válido.
pub fn is_valid_msg_id(id: u8) -> bool {
    MsgId::is_valid(id)
}

/// Retorna a prioridade de uma mensagem com base no seu ID e estado de failsafe.
///
/// Se o failsafe estiver ativo, todas as mensagens recebem prioridade SUPER_CRITICAL,
/// exceto mensagens de debug que ficam em LOW.
pub fn get_msg_priority(msg_id: u8, failsafe_active: bool) -> u8 {
    if failsafe_active {
        return if msg_id == MsgId::Debug as u8 {
            PriorityLevel::Low as u8
        } else {
            PriorityLevel::SuperCritical as u8
        };
    }

    match msg_id {
        0x10 => PriorityLevel::Medium as u8,     // Heartbeat
        0x11 => PriorityLevel::Medium as u8,     // Telemetry
        0x12 => PriorityLevel::High as u8,       // Command
        0x13 => PriorityLevel::High as u8,       // ACK
        0x14 => PriorityLevel::SuperCritical as u8, // Failsafe
        0x15 => PriorityLevel::Low as u8,        // Debug
        0x16 => PriorityLevel::Low as u8,        // Video
        0x17 => PriorityLevel::Medium as u8,     // Shell
        0x18 => PriorityLevel::Medium as u8,     // SI Data
        0x19 => PriorityLevel::Medium as u8,     // Watchdog
        0x1A => PriorityLevel::Medium as u8,     // Ping
        0x1B => PriorityLevel::High as u8,       // Clock
        _ => PriorityLevel::Low as u8,
    }
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bythos_constants() {
        assert_eq!(START_BYTE, 0xAA);
        assert_eq!(BYTHOS_VERSION, 0x03);
        assert_eq!(BYTHOS_HEADER_SIZE, 7);
        assert_eq!(CRC16_SIZE, 2);
        assert_eq!(SIGNATURE_SIZE, 1);
        assert_eq!(BYTHOS_OVERHEAD, 10);
    }

    #[test]
    fn test_msg_id_validity() {
        assert!(MsgId::is_valid(0x10));
        assert!(MsgId::is_valid(0x1B));
        assert!(!MsgId::is_valid(0x00));
        assert!(!MsgId::is_valid(0x1C));
    }

    #[test]
    fn test_field_type_from_u8() {
        assert_eq!(FieldType::from_u8(0), Some(FieldType::Raw));
        assert_eq!(FieldType::from_u8(1), Some(FieldType::Float32));
        assert_eq!(FieldType::from_u8(7), Some(FieldType::Bool));
        assert_eq!(FieldType::from_u8(8), None);
    }

    #[test]
    fn test_field_type_default_size() {
        assert_eq!(FieldType::Raw.default_size(), 0);
        assert_eq!(FieldType::Float32.default_size(), 4);
        assert_eq!(FieldType::Uint8.default_size(), 1);
        assert_eq!(FieldType::Bool.default_size(), 1);
    }

    #[test]
    fn test_field_id_encode_decode() {
        // Tipo 1 (f32), ID 6 → FieldID = 0x26
        let fid = field_id_encode(1, 6);
        assert_eq!(fid, 0x26);
        let (t, id) = field_id_decode(fid);
        assert_eq!(t, 1);
        assert_eq!(id, 6);

        // Tipo 6 (u8), ID 0 → FieldID = 0xC0
        let fid = field_id_encode(6, 0);
        assert_eq!(fid, 0xC0);
        let (t, id) = field_id_decode(fid);
        assert_eq!(t, 6);
        assert_eq!(id, 0);

        // Tipo 0 (raw), ID 0 → FieldID = 0x00
        let fid = field_id_encode(0, 0);
        assert_eq!(fid, 0x00);
    }

    #[test]
    fn test_can_id_make_extract() {
        // Prioridade=2(High), Src=0x6(Device5), Dst=0x0(Broadcast), Type=0x0(Data)
        let can_id = make_can_id(2, 0x6, 0x0, 0x0);
        assert_eq!(can_id_priority(can_id), 2);
        assert_eq!(can_id_src_group(can_id), 0x6);
        assert_eq!(can_id_dst_group(can_id), 0x0);
        assert_eq!(can_id_msg_type(can_id), 0x0);
        assert!(!is_safety_bus_id(can_id));
    }

    #[test]
    fn test_can_id_safety_bus() {
        let safety_id = make_can_id(
            PriorityLevel::SuperCritical as u8,
            CanGroup::Device3 as u8,
            CanGroup::Device4 as u8,
            CanMsgType::Safety as u8,
        );
        assert!(is_safety_bus_id(safety_id));
        let normal_id = make_can_id(
            PriorityLevel::High as u8,
            CanGroup::Device5 as u8,
            CanGroup::None as u8,
            CanMsgType::Data as u8,
        );
        assert!(!is_safety_bus_id(normal_id));
    }

    #[test]
    fn test_signature_computation() {
        // key=0x42, msg_id=0x11, seq_lo=0x2A, seq_hi=0x00
        let sig = compute_signature(0x42, 0x11, 0x2A, 0x00);
        assert_eq!(sig, 0x42 ^ 0x11 ^ 0x2A ^ 0x00);
        assert!(validate_signature(sig, 0x42, 0x11, 0x2A, 0x00));
        assert!(!validate_signature(sig, 0x43, 0x11, 0x2A, 0x00));
    }

    #[test]
    fn test_float_conversion() {
        let value: f32 = 3.14;
        let bytes = float_to_bytes(value);
        let recovered = bytes_to_float(&bytes);
        assert!((value - recovered).abs() < f32::EPSILON);
    }

    #[test]
    fn test_int32_conversion() {
        let value: i32 = -12345;
        let bytes = int32_to_bytes(value);
        let recovered = bytes_to_int32(&bytes);
        assert_eq!(value, recovered);
    }

    #[test]
    fn test_uint32_conversion() {
        let value: u32 = 0xDEADBEEF;
        let bytes = uint32_to_bytes(value);
        let recovered = bytes_to_uint32(&bytes);
        assert_eq!(value, recovered);
    }

    #[test]
    fn test_uint16_conversion() {
        let value: u16 = 0x1234;
        let bytes = uint16_to_bytes(value);
        let recovered = bytes_to_uint16(&bytes);
        assert_eq!(value, recovered);
    }

    #[test]
    fn test_tlv_field_new() {
        let field = TLVField::new();
        assert_eq!(field.id, 0);
        assert_eq!(field.len, 0);
    }

    #[test]
    fn test_tlv_field_with_data() {
        let data = [1u8, 2, 3, 4];
        let field = TLVField::with_data(0xB0, &data);
        assert_eq!(field.id, 0xB0);
        assert_eq!(field.len, 4);
        assert_eq!(field.data[0], 1);
        assert_eq!(field.data[3], 4);
    }

    #[test]
    fn test_tlv_field_with_type_data() {
        let field = TLVField::with_type_data(FieldType::Float32, 6, &float_to_bytes(1.5));
        assert_eq!(field.id, 0x26);
        assert_eq!(field.len, 4);
        assert_eq!(field.field_type(), Some(FieldType::Float32));
        assert_eq!(field.field_id(), 6);
    }

    #[test]
    fn test_tlv_message_new() {
        let msg = TLVMessage::new();
        assert_eq!(msg.start_byte, START_BYTE);
        assert_eq!(msg.version, BYTHOS_VERSION);
        assert_eq!(msg.tlv_count, 0);
    }

    #[test]
    fn test_tlv_message_with_params() {
        let msg = TLVMessage::with_params(0x11, 0x06);
        assert_eq!(msg.msg_id, 0x11);
        assert_eq!(msg.node_id, 0x06);
    }

    #[test]
    fn test_tlv_message_clear() {
        let mut msg = TLVMessage::with_params(0x16, 0x06);
        msg.tlv_count = 3;
        msg.clear();
        assert_eq!(msg.tlv_count, 0);
        assert_eq!(msg.msg_id, 0);
        assert_eq!(msg.node_id, 0);
    }

    #[test]
    fn test_get_msg_priority_normal() {
        assert_eq!(get_msg_priority(0x14, false), PriorityLevel::SuperCritical as u8);
        assert_eq!(get_msg_priority(0x12, false), PriorityLevel::High as u8);
        assert_eq!(get_msg_priority(0x10, false), PriorityLevel::Medium as u8);
        assert_eq!(get_msg_priority(0x16, false), PriorityLevel::Low as u8);
        assert_eq!(get_msg_priority(0x15, false), PriorityLevel::Low as u8);
    }

    #[test]
    fn test_get_msg_priority_failsafe() {
        assert_eq!(get_msg_priority(0x10, true), PriorityLevel::SuperCritical as u8);
        assert_eq!(get_msg_priority(0x15, true), PriorityLevel::Low as u8);
    }

    #[test]
    fn test_can_group_values() {
        assert_eq!(CanGroup::None as u8, 0x0);
        assert_eq!(CanGroup::Device0 as u8, 0x1);
        assert_eq!(CanGroup::Device1 as u8, 0x2);
        assert_eq!(CanGroup::Device2 as u8, 0x3);
        assert_eq!(CanGroup::Device3 as u8, 0x4);
        assert_eq!(CanGroup::Device4 as u8, 0x5);
        assert_eq!(CanGroup::Device5 as u8, 0x6);
    }

    #[test]
    fn test_can_msg_type_values() {
        assert_eq!(CanMsgType::Data as u8, 0x0);
        assert_eq!(CanMsgType::Cmd as u8, 0x1);
        assert_eq!(CanMsgType::Ack as u8, 0x2);
        assert_eq!(CanMsgType::Safety as u8, 0x7);
    }
}
