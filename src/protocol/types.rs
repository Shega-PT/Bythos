//! # Bythos — Tipos e Definições (v4.0.0)
//!
//! Este módulo é a única fonte de verdade do protocolo Bythos V4: constantes do
//! fio, endereçamento em anel, catálogo de campos e estruturas de mensagem.
//!
//! ## O que mudou da V3 para a V4 (ler antes de mexer)
//!
//! 1. **Nomenclatura exclusivamente Bythos.** `CAN`, `TLV` e `DeviceX` desapareceram
//!    da superfície pública (ver `docs/MIGRATION-GUIDE.md`). O que era `TLV`
//!    chama-se agora campo Bythos; o que era `CanGroup` chama-se `BythosGroup`.
//!    Os valores numéricos no fio foram preservados para não invalidar catálogos.
//! 2. **Endereço de anel `u16` em vez de `NODE_ID u8`.** O cabeçalho V3 só cabia em
//!    256 nós manuais; a V4 enumera sozinha até 65535 nós (`SRC` + `DST` no fio).
//!    Por isso o cabeçalho cresceu de 7 para 11 bytes e o máximo de 1098 para 1205.
//! 3. **Segurança real em vez de XOR.** O byte de assinatura deu lugar a `SEC_HDR`
//!    (4 B) + `TAG` (4 B, HMAC-SHA256 truncado, calculado em elemento seguro).
//!    Ver `crate::protocol::secure` para o porquê de cada byte.
//!
//! ## Formato da trama (normativo; ver `docs/BYTHOS-SPECIFICATION.md`)
//!
//! ```text
//! [INÍCIO:1][VERSÃO:1][ORIGEM:2 LE][DESTINO:2 LE][MSG:1][SEQ:2 LE]
//! [N_CAMPOS:1][SALTOS:1][CAMPOS...][SEC_HDR:4][TAG:4][CRC16:2 LE]
//! ```
//!
//! ## Campo Bythos — identificador com tipo embutido
//!
//! ```text
//! BythosFieldId = [TIPO:3 bits][ID:5 bits]
//!
//!   Bits 7-5: tipo de dado (0-7)
//!   Bits 4-0: id lógico do campo (0-31)
//! ```
//!
//! ## Endianness
//!
//! Todos os campos multi-byte viajam em little-endian (LE): é a ordem nativa do
//! ESP32/ARM/RISC-V que as pontes usam, logo zero conversões no caminho quente.

// ============================================================================
// CONSTANTES GLOBAIS — Bythos v4.0.0
// ============================================================================

/// Byte de início de cada trama. Serve de âncora de sincronização: o analisador
/// descarta tudo até encontrar `0xAA`, o que permite ressincronizar a meio de
/// ruído sem estado adicional.
pub const START_BYTE: u8 = 0xAA;

/// Versão do protocolo no fio. A V4 rejeita `0x03` por defeito; a aceitação de
/// V3 vive apenas atrás da feature `legacy-v3-compat` (migração, nunca produção).
pub const BYTHOS_VERSION: u8 = 0x04;

/// Versão V3, aceite somente com `legacy-v3-compat`. Existe para que uma ponte em
/// migração consiga ler o parque instalado sem bifurcar o código de análise.
#[cfg(feature = "legacy-v3-compat")]
pub const BYTHOS_VERSION_V3: u8 = 0x03;

/// Nº máximo de bytes de dados num campo normal.
///
/// 32 B é o compromisso histórico do Bythos: cabe em 1–2 fragmentos de qualquer
/// rádio (LoRa incluído) e mantém o tampão máximo da mensagem abaixo de ~1,2 KiB,
/// comportável na pilha de um MCU pequeno.
pub const MAX_FIELD_DATA: usize = 32;

/// Nº máximo de bytes de dados no campo de carga vídeo (apenas tipo `Raw`).
///
/// 128 B permite transportar macroblocos úteis sem rebentar o MTU do anel; mais
/// que isto obrigaria a janelas de remontagem grandes nas pontes. Só é aceite em
/// campos cujo tipo é `Raw` — um `f32` de 128 B seria corrupção, não vídeo.
pub const MAX_FIELD_VIDEO_DATA: usize = 128;

/// Nº máximo de campos por mensagem.
///
/// 32 campos × 34 B cobrem a telemetria completa de um nó (GPS+IMU+energia+estado)
/// numa só trama, sem alocar: o construtor e o analisador usam arrays fixos.
pub const MAX_FIELDS: usize = 32;

/// Tamanho do cabeçalho V4: início(1) + versão(1) + origem(2) + destino(2) +
/// msg(1) + seq(2) + n_campos(1) + saltos(1) = 11 bytes.
///
/// Cresceu 4 bytes face à V3 (7 B): o preço da origem/destino de 16 bits e do
/// limite de saltos que impede tempestades de repetição em erro de cablagem.
pub const BYTHOS_HEADER_SIZE: usize = 11;

/// Tamanho do CRC16 no fim da trama. Mantido da V3: deteção barata de corrupção
/// acidental antes de acordar o elemento seguro (que custa tempo e energia).
pub const CRC16_SIZE: usize = 2;

/// Tamanho do cabeçalho de segurança: `[KEY_ID:1][CTR:3 LE]`.
///
/// - `KEY_ID` escolhe a ranhura de chave no elemento seguro (`0xFF` = legado);
/// - `CTR` é o contador monotónico de 24 bits que derrota repetição de tramas.
pub const SEC_HDR_SIZE: usize = 4;

/// Tamanho da etiqueta de autenticação: HMAC-SHA256 truncado a 32 bits.
///
/// 4 B é o ponto de equilíbrio decidido para os dois modos físicos S/L: cabe no
/// MTU curto sem fragmentar e exige ~2³² tentativas em linha para forjar — o anel
/// sinaliza o atacante muito antes disso (ver `docs/THREAT-MODEL.md`).
pub const TAG_SIZE: usize = 4;

/// Tamanho do cabeçalho de cada campo: `[FIELD_ID:1][LEN:1]`.
pub const FIELD_HEADER_SIZE: usize = 2;

/// Sobrecarga total da trama: cabeçalho(11) + segurança(4) + etiqueta(4) + crc(2).
///
/// 21 bytes contra 10 da V3. O acréscimo compra: endereçamento ilimitado (4 B),
/// anti-replay (3 B), autenticação real (4 B) e limite de saltos (1 B).
pub const BYTHOS_OVERHEAD: usize = BYTHOS_HEADER_SIZE + SEC_HDR_SIZE + TAG_SIZE + CRC16_SIZE;

/// Tamanho máximo de uma trama V4 serializada, em bytes.
///
/// Pior caso comportável: 31 campos normais cheios + 1 campo vídeo cheio.
/// `11 + 31·(2+32) + (2+128) + 10 = 1205`. Duas cargas vídeo cheias na mesma
/// trama são recusadas na construção — o tampão do analisador tem de caber na
/// pilha de um MCU, e 1205 B já é o teto sensato.
pub const MAX_MESSAGE_SIZE: usize = BYTHOS_HEADER_SIZE
    + (MAX_FIELDS - 1) * (FIELD_HEADER_SIZE + MAX_FIELD_DATA)
    + (FIELD_HEADER_SIZE + MAX_FIELD_VIDEO_DATA)
    + SEC_HDR_SIZE
    + TAG_SIZE
    + CRC16_SIZE;

/// Endereço de difusão: trama para todos os nós do anel.
///
/// Reservar o topo (`0xFFFF`) em vez do zero permite que `0` seja a raiz `BG-0`
/// e que endereço "não atribuído" (`0` antes da enumeração) nunca seja difusão
/// por acidente.
pub const BROADCAST_ADDR: u16 = 0xFFFF;

/// Endereço da raiz do anel. O único endereço configurável à mão (jumper `ROOT`
/// na ponte); todos os outros nascem da enumeração automática, por ordem física.
pub const ROOT_ADDR: u16 = 0;

/// Limite de saltos por defeito. 32 cobre um anel de 1000 nós pelo sentido curto
/// (500 saltos no pior caso… ver nota) — na prática o encaminhador escolhe o
/// sentido com menos saltos, logo 32 chega com folga e trava loops de mis-wire.
///
/// Nota de dimensionamento: anel de 1000 nós tem diâmetro 500; se um dia o anel
/// passar disso, subir `HOP_DEFAULT` exige apenas acordo de frota, não fio novo.
pub const HOP_DEFAULT: u8 = 32;

/// `KEY_ID` que marca modo legado V3 (XOR, sem `TAG` real).
///
/// Existe só para migração com `legacy-v3-compat`. Produção recusa-o: o analisador
/// trata `0xFF` como "autenticação ausente" e o construtor nunca o emite sem a
/// feature ligada.
pub const KEY_ID_LEGACY: u8 = 0xFF;

// ============================================================================
// TIPOS DE DADO DO CAMPO (3 bits = 8 valores)
// ============================================================================

/// Tipos de dado que um campo Bythos pode transportar.
///
/// O tipo viaja embutido nos bits 7-5 do identificador (ver `BythosFieldId`), de
/// modo que o recetor sabe o tamanho esperado sem tabela externa — essencial
/// para validar `LEN` contra corrupção antes de copiar.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BythosFieldType {
    /// Dados brutos (carga binária, vídeo). Tamanho variável, único tipo que
    /// pode usar os 128 B de `MAX_FIELD_VIDEO_DATA`.
    Raw = 0,
    /// Vírgula flutuante de 32 bits (GPS, IMU, voo, energia).
    Float32 = 1,
    /// Vírgula flutuante de 16 bits (sensores de precisão reduzida).
    Float16 = 2,
    /// Inteiro com sinal de 32 bits.
    Int32 = 3,
    /// Inteiro sem sinal de 32 bits (contadores, uptime).
    Uint32 = 4,
    /// Inteiro sem sinal de 16 bits (frame id, contadores curtos).
    Uint16 = 5,
    /// Inteiro sem sinal de 8 bits (estado, modos, cargas).
    Uint8 = 6,
    /// Booleano (`0` = falso, diferente de `0` = verdadeiro).
    Bool = 7,
}

impl BythosFieldType {
    /// Converte um valor de 3 bits no tipo correspondente.
    ///
    /// Retorna `None` para valores > 7 — o que, vindos do fio, significa
    /// necessariamente corrupção ou dessincronização, nunca "tipo futuro".
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

    /// Tamanho canónico do tipo em bytes; `0` para `Raw` (variável).
    ///
    /// O validador usa isto para apertar a malha: um campo `Float32` com
    /// `LEN != 4` é rejeitado mesmo que caiba nos 32 B genéricos.
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
// IDENTIFICADORES DE MENSAGEM (BythosMsgId)
// ============================================================================

/// Tipos de mensagem do Bythos V4.
///
/// A base `0x10–0x1B` é a V3 (valores intocados); `0x1C–0x1F` nascem com o anel:
/// enumeração, supervisão de fecho e diagnóstico de cablagem. Nada acima de `0x1F`
/// é válido — reserva-se espaço sem prometer semântica.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BythosMsgId {
    /// Presença periódica do nó.
    Heartbeat = 0x10,
    /// Dados de sensores e estado.
    Telemetry = 0x11,
    /// Instrução para o nó.
    Command = 0x12,
    /// Confirmação de receção.
    Ack = 0x13,
    /// Estado de segurança de emergência.
    Failsafe = 0x14,
    /// Diagnóstico.
    Debug = 0x15,
    /// Vídeo fragmentado (ver `crate::tunnel`).
    Video = 0x16,
    /// Consola remota.
    Shell = 0x17,
    /// Dados de integridade estrutural.
    SiData = 0x18,
    /// Vigilância (keepalive de supervisão).
    Watchdog = 0x19,
    /// Eco para medir latência do anel.
    Ping = 0x1A,
    /// Relógio + anúncio de modo físico S/L.
    Clock = 0x1B,
    /// Enumeração: carrega a posição proposta no anel. **Novo V4.**
    Hello = 0x1C,
    /// Enumeração: difunde o total `N` após fecho. **Novo V4.**
    Count = 0x1D,
    /// Anel partido: modo linha, encaminhar pelo lado vivo. **Novo V4.**
    RingOpen = 0x1E,
    /// Erro de cablagem (porta cruzada, duplo ROOT…). **Novo V4.**
    WiringFault = 0x1F,
}

impl BythosMsgId {
    /// Converte um byte no tipo correspondente; `None` = reservado/inválido.
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
            0x1C => Some(Self::Hello),
            0x1D => Some(Self::Count),
            0x1E => Some(Self::RingOpen),
            0x1F => Some(Self::WiringFault),
            _ => None,
        }
    }

    /// Atalho de validação para o caminho quente do analisador.
    pub fn is_valid(id: u8) -> bool {
        Self::from_u8(id).is_some()
    }
}

// ============================================================================
// PRIORIDADE BYTHOS
// ============================================================================

/// Níveis de prioridade para ordenação e backoff no anel.
///
/// Valor menor = mais urgente. Herdam a semântica V3 (que os mapeava nos 3 bits
/// do antigo CAN ID); na V4 viajam implicitamente no tipo de mensagem e decidem
/// quem recua primeiro em contenção no par de dados.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BythosPriority {
    /// Emergência: processa já, não adia.
    SuperCritical = 0,
    /// Urgente: precede todo o tráfego normal.
    Critical = 1,
    /// Alta: precede tarefas normais (comandos, relógio).
    High = 2,
    /// Normal: telemetria rotineira.
    Medium = 3,
    /// Fundo: diagnóstico e vídeo preenchem o que sobra.
    Low = 4,
}

impl BythosPriority {
    /// Conversão saturante: valores desconhecidos caem para `Low` em vez de
    /// falharem — prioridade é dica de escalonamento, nunca gate de segurança.
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
// ESTADO DO SISTEMA, MODO DE VOO, FAILSAFE (inalterados da V3 — só vivem aqui)
// ============================================================================

/// Estados possíveis do sistema anfitrião (cargas úteis continuam a usá-los).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemState {
    /// Arranque a frio, periféricos ainda a inicializar.
    Booting = 0,
    /// A configurar, ainda sem operar.
    Initializing = 1,
    /// Pronto para operar.
    Ready = 2,
    /// Armado (atuadores energizados).
    Armed = 3,
    /// Em missão.
    InFlight = 4,
    /// A aterrar/recolher.
    Landing = 5,
    /// Erro que exige atenção.
    Error = 6,
    /// A desligar de forma ordeira.
    Shutdown = 7,
}

impl SystemState {
    /// Conversão estrita: fora de `0–7` é corrupção, não estado futuro.
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

/// Modos de voo para cargas UAV/UAS.
///
/// Mantido no núcleo por compatibilidade com o parque V3; frotas não-UAV
/// ignoram-no sem custo (é só um byte num campo `Uint8`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightMode {
    /// Controlo manual total.
    Manual = 0,
    /// Atitude estabilizada.
    Stabilize = 1,
    /// Altitude mantida.
    AltHold = 2,
    /// Missão automática.
    Auto = 3,
    /// Guiado por estação.
    Guided = 4,
    /// Regresso ao ponto de partida.
    Rtl = 5,
}

impl FlightMode {
    /// Conversão estrita (`None` fora de `0–5`).
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

/// Motivos de ativação do failsafe.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailsafeReason {
    /// Sem failsafe.
    None = 0,
    /// Perda de ligação com a estação/anel.
    SignalLost = 1,
    /// Bateria abaixo do limiar.
    LowBattery = 2,
    /// Perda de GPS.
    GpsLost = 3,
    /// Sensor crítico em falha.
    SensorFailure = 4,
    /// Acionamento manual.
    ManualTrigger = 5,
}

impl FailsafeReason {
    /// Conversão estrita (`None` fora de `0–5`).
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

/// Ações executadas quando o failsafe dispara.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailsafeAction {
    /// Nada a fazer.
    None = 0,
    /// Pairar no ponto.
    Hover = 1,
    /// Aterrar onde está.
    Land = 2,
    /// Regressar ao ponto de partida.
    Rtl = 3,
    /// Prosseguir a missão.
    Continue = 4,
    /// Desarmar atuadores.
    Disarm = 5,
}

impl FailsafeAction {
    /// Conversão estrita (`None` fora de `0–5`).
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
// GRUPOS BYTHOS (papel funcional do nó — ex-CanGroup, renomeado e funcional)
// ============================================================================

/// Papel funcional de um nó no sistema.
///
/// A V3 chamava a isto `CanGroup` com nomes `Device0…Device14` — opacos e presos
/// ao barramento antigo. A V4 dá nomes de **função** (o fio só transporta os 4
/// bits; o nome é para humanos e para o backoff de contenção). Os valores são os
/// mesmos da V3 para não quebrar tabelas de comissionamento.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BythosGroup {
    /// Sem grupo / difusão funcional.
    None = 0x0,
    /// Orquestração central (tipicamente o BG-0 físico, mas papel ≠ endereço).
    Control = 0x1,
    /// Aquisição de sensores.
    Sensors = 0x2,
    /// Controlo de atuadores.
    Actuators = 0x3,
    /// Supervisão de segurança.
    Safety = 0x4,
    /// Resposta a emergência.
    Emergency = 0x5,
    /// Visão por computador.
    Vision = 0x6,
    /// Reserva funcional 7.
    Aux7 = 0x7,
    /// Reserva funcional 8.
    Aux8 = 0x8,
    /// Reserva funcional 9.
    Aux9 = 0x9,
    /// Reserva funcional 10.
    Aux10 = 0xA,
    /// Reserva funcional 11.
    Aux11 = 0xB,
    /// Reserva funcional 12.
    Aux12 = 0xC,
    /// Reserva funcional 13.
    Aux13 = 0xD,
    /// Reserva funcional 14.
    Aux14 = 0xE,
    /// Reserva funcional 15.
    Aux15 = 0xF,
}

impl BythosGroup {
    /// Conversão estrita de 4 bits; `None` fora de `0x0–0xF`.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(Self::None),
            0x1 => Some(Self::Control),
            0x2 => Some(Self::Sensors),
            0x3 => Some(Self::Actuators),
            0x4 => Some(Self::Safety),
            0x5 => Some(Self::Emergency),
            0x6 => Some(Self::Vision),
            0x7 => Some(Self::Aux7),
            0x8 => Some(Self::Aux8),
            0x9 => Some(Self::Aux9),
            0xA => Some(Self::Aux10),
            0xB => Some(Self::Aux11),
            0xC => Some(Self::Aux12),
            0xD => Some(Self::Aux13),
            0xE => Some(Self::Aux14),
            0xF => Some(Self::Aux15),
            _ => None,
        }
    }
}

// ============================================================================
// ESPÉCIE BYTHOS (natureza do tráfego — ex-CanMsgType, renomeado)
// ============================================================================

/// Natureza do tráfego, usada no escalonamento e no diagnóstico do anel.
///
/// Na V3 estes 4 bits viajavam no CAN ID de 29 bits; na V4 o fio do anel não tem
/// CAN ID — a espécie deriva-se do `MSG` (ver `kind_of`) e serve o firmware da
/// ponte (filas, contadores, deteção do barramento de segurança).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BythosKind {
    /// Telemetria / dados de sensores.
    Data = 0x0,
    /// Comandos.
    Cmd = 0x1,
    /// Confirmação de receção.
    Ack = 0x2,
    /// Eventos / failsafe.
    Event = 0x3,
    /// Sincronização temporal.
    Sync = 0x4,
    /// Difusão de estado.
    State = 0x5,
    /// Presença.
    Heart = 0x6,
    /// Tráfego de segurança (prioridade máxima efetiva).
    Safety = 0x7,
    /// Gestão do anel (Hello/Count/RingOpen/WiringFault). **Novo V4.**
    Ring = 0x8,
}

impl BythosKind {
    /// Conversão estrita (`None` fora de `0x0–0x8`).
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
            0x8 => Some(Self::Ring),
            _ => None,
        }
    }

    /// Deriva a espécie a partir do tipo de mensagem do fio.
    ///
    /// É o ponto único de verdade para classificar tráfego sem CAN ID: qualquer
    /// fila de prioridade da ponte deve chamar aqui, nunca adivinhar por `MSG`.
    pub fn kind_of(msg_id: u8) -> Option<Self> {
        match msg_id {
            0x10 => Some(Self::Heart),
            0x11 => Some(Self::Data),
            0x12 => Some(Self::Cmd),
            0x13 => Some(Self::Ack),
            0x14 => Some(Self::Safety),
            0x15 => Some(Self::Data),
            0x16 => Some(Self::Data),
            0x17 => Some(Self::Cmd),
            0x18 => Some(Self::Data),
            0x19 => Some(Self::Heart),
            0x1A => Some(Self::Sync),
            0x1B => Some(Self::Sync),
            0x1C..=0x1F => Some(Self::Ring),
            _ => None,
        }
    }
}

// ============================================================================
// ENDEREÇO BYTHOS (ordem no anel — substitui o CAN ID de 29 bits)
// ============================================================================

/// Endereço de um nó no anel: posição física após enumeração (`0–65535`).
///
/// Substitui o CAN ID extended da V3. Onde a V3 empacotava prioridade+grupos+tipo
/// em 29 bits de arbitragem, a V4 endereça por posição e arbitra por
/// prioridade+grupo no firmware da ponte — separação limpa entre "para onde vai"
/// (este endereço) e "com que urgência" (`BythosPriority` + `BythosGroup`).
///
/// Representação compacta no fio: apenas o `u16` de posição. Os metadados de
/// escalonamento viajam fora do fio quente ou derivam-se do `MSG`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BythosAddress {
    /// Posição no anel (`0` = raiz, `0xFFFF` = difusão).
    pub pos: u16,
    /// Papel funcional do nó (dica de escalonamento, não encaminhamento).
    pub group: BythosGroup,
    /// Natureza do tráfego (dica de escalonamento, não encaminhamento).
    pub kind: BythosKind,
    /// Prioridade efetiva (decide backoff em contenção).
    pub priority: BythosPriority,
}

impl BythosAddress {
    /// Constrói um endereço com validação de gamas.
    ///
    /// `None` se o grupo ou a espécie forem desconhecidos — programar um nó com
    /// papel inválido deve falhar no comissionamento, nunca a meio do voo.
    pub fn new(pos: u16, group: u8, kind: u8, priority: u8) -> Option<Self> {
        Some(Self {
            pos,
            group: BythosGroup::from_u8(group)?,
            kind: BythosKind::from_u8(kind)?,
            priority: BythosPriority::from_u8(priority),
        })
    }

    /// Atalho para difusão com espécie e prioridade dadas.
    pub fn broadcast(kind: BythosKind, priority: BythosPriority) -> Self {
        Self {
            pos: BROADCAST_ADDR,
            group: BythosGroup::None,
            kind,
            priority,
        }
    }

    /// `true` se for o barramento de segurança (espécie `Safety`).
    ///
    /// Substitui `is_safety_bus_id` da V3: mesma semântica, nome Bythos.
    pub fn is_safety(&self) -> bool {
        self.kind == BythosKind::Safety
    }
}

/// Constrói o par de encaminhamento `(origem, destino)` validado.
///
/// Substitui `make_can_id` da V3. Não há 29 bits nem máscaras: o anel encaminha
/// por comparação de posições (ver `crate::ring`), por isso só se valida que os
/// metadados são conhecidos.
pub fn make_bythos_route(
    src_pos: u16,
    dst_pos: u16,
    group: u8,
    kind: u8,
    priority: u8,
) -> Option<(BythosAddress, BythosAddress)> {
    let group_v = BythosGroup::from_u8(group)?;
    let kind_v = BythosKind::from_u8(kind)?;
    let prio_v = BythosPriority::from_u8(priority);
    Some((
        BythosAddress {
            pos: src_pos,
            group: group_v,
            kind: kind_v,
            priority: prio_v,
        },
        BythosAddress {
            pos: dst_pos,
            group: group_v,
            kind: kind_v,
            priority: prio_v,
        },
    ))
}

// ============================================================================
// IDENTIFICADOR DE CAMPO COM TIPO EMBUTIDO
// ============================================================================

/// Codifica um identificador de campo: `[TIPO:3][ID:5]`.
///
/// O mascaramento (`& 0x07`, `& 0x1F`) é intencional: IDs lógicos acima de 31
/// dobram em vez de rebentar o byte — o catálogo (`BythosFieldId`) só usa IDs
/// válidos, e o validador aperta `LEN` por tipo a seguir.
pub fn bythos_field_id_encode(field_type: u8, field_id: u8) -> u8 {
    ((field_type & 0x07) << 5) | (field_id & 0x1F)
}

/// Decodifica um identificador nos seus componentes `(tipo, id)`.
pub fn bythos_field_id_decode(field_id: u8) -> (u8, u8) {
    ((field_id >> 5) & 0x07, field_id & 0x1F)
}

/// Valida o tipo embutido (bits 7-5).
///
/// Nota honesta: qualquer byte tem tipo `0–7`, logo isto é sempre verdade — o
/// valor real está em `BythosFieldType::from_u8` + verificação de `LEN` por tipo
/// no validador. Mantém-se como barreira de leitura, não como prova.
pub fn is_valid_bythos_field_id(field_id: u8) -> bool {
    let (field_type, _) = bythos_field_id_decode(field_id);
    BythosFieldType::from_u8(field_type).is_some()
}

// ============================================================================
// ETIQUETA LEGADA (XOR — migração apenas, nunca produção)
// ============================================================================

/// Calcula a etiqueta XOR legada V3 (`chave ^ msg ^ seq_lo ^ seq_hi`).
///
/// Existe para que pontes em migração (`legacy-v3-compat`) consigam **ler** o
/// parque V3. Escrever V3 é desencorajado e exige a feature; produção usa `TAG`.
pub fn compute_legacy_tag(key: u8, msg_id: u8, seq_lo: u8, seq_hi: u8) -> u8 {
    key ^ msg_id ^ seq_lo ^ seq_hi
}

/// Verifica a etiqueta XOR legada em tempo constante ingénuo.
///
/// Comparação direta de 1 byte; sem early-exit observável relevante a este
/// tamanho — e tratando-se de compatibilidade, não de segurança.
pub fn validate_legacy_tag(tag: u8, key: u8, msg_id: u8, seq_lo: u8, seq_hi: u8) -> bool {
    tag == compute_legacy_tag(key, msg_id, seq_lo, seq_hi)
}

// ============================================================================
// CATÁLOGO DE CAMPOS — IDENTIFICADORES LÓGICOS POR DOMÍNIO
// ============================================================================
//
// Os enums abaixo usam IDs lógicos de domínio (podem exceder 31: são chaves de
// catálogo, não o byte do fio). O byte do fio vive em `BythosFieldId`.
// Mantêm-se para compatibilidade de código aplicacional V3.

/// Campos GPS (IDs lógicos de catálogo).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldGps {
    /// Latitude em graus decimais (`f32`).
    Latitude = 0x20,
    /// Longitude em graus decimais (`f32`).
    Longitude = 0x21,
    /// Altitude em metros (`f32`).
    Altitude = 0x22,
    /// Velocidade sobre o solo em m/s (`f32`).
    Speed = 0x23,
    /// Rumo em graus (`f32`).
    Course = 0x24,
    /// Satélites em vista (`u8`).
    Satellites = 0x25,
    /// Diluição horizontal de precisão (`f32`).
    Hdop = 0x26,
}

/// Campos da unidade inercial.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldImu {
    /// Rolamento em graus (`f32`).
    Roll = 0x30,
    /// Arfagem em graus (`f32`).
    Pitch = 0x31,
    /// Guinada em graus (`f32`).
    Yaw = 0x32,
    /// Aceleração X em m/s² (`f32`).
    AccelX = 0x33,
    /// Aceleração Y em m/s² (`f32`).
    AccelY = 0x34,
    /// Aceleração Z em m/s² (`f32`).
    AccelZ = 0x35,
    /// Girómetro X em °/s (`f32`).
    GyroX = 0x36,
    /// Girómetro Y em °/s (`f32`).
    GyroY = 0x37,
    /// Girómetro Z em °/s (`f32`).
    GyroZ = 0x38,
    /// Taxa de guinada em °/s (`f32`).
    YawRate = 0x39,
}

/// Campos de voo.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldFlight {
    /// Altitude GPS em metros (`f32`).
    AltGps = 0x40,
    /// Altitude barométrica em metros (`f32`).
    AltBaro = 0x41,
    /// Velocidade vertical em m/s (`f32`).
    VSpeed = 0x42,
    /// Velocidade aerodinâmica em m/s (`f32`).
    Airspeed = 0x43,
    /// Duração do ciclo de controlo em µs (`u16`).
    LoopTime = 0x44,
}

/// Campos de energia.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldPower {
    /// Tensão da bateria em V (`f32`).
    BattVoltage = 0x50,
    /// Corrente em A (`f32`).
    BattCurrent = 0x51,
    /// Carga consumida em mAh (`f32`).
    BattConsumed = 0x52,
    /// Temperatura da bateria em °C (`f32`).
    BattTemp = 0x53,
    /// Estado de carga 0.0–1.0 (`f32`).
    BattSoc = 0x54,
}

/// Campos de temperatura.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldTemp {
    /// Sensor 1 em °C (`f32`).
    Temp1 = 0x60,
    /// Sensor 2 em °C (`f32`).
    Temp2 = 0x61,
    /// Sensor 3 em °C (`f32`).
    Temp3 = 0x62,
    /// Sensor 4 em °C (`f32`).
    Temp4 = 0x63,
    /// Temperatura da ponte 1 (`f32`).
    Bridge1Temp = 0x64,
    /// Temperatura da ponte 2 (`f32`).
    Bridge2Temp = 0x65,
}

/// Campos de sistema.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldSystem {
    /// Estado (`SystemState`, `u8`).
    State = 0x70,
    /// Modo (`u8`).
    Mode = 0x71,
    /// Tempo ligado em s (`u32`).
    Uptime = 0x72,
    /// RAM livre em B (`u32`).
    FreeHeap = 0x73,
    /// Carga CPU em % (`u8`).
    CpuLoad = 0x74,
    /// Carga da ponte 1 em % (`u8`).
    Bridge1Load = 0x75,
    /// Carga da ponte 2 em % (`u8`).
    Bridge2Load = 0x76,
}

/// Campos de failsafe.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldFailsafe {
    /// Motivo (`FailsafeReason`, `u8`).
    Reason = 0xA1,
    /// Ação (`FailsafeAction`, `u8`).
    Action = 0xA2,
    /// Estado do autómato de failsafe (`u8`).
    State = 0xA3,
}

/// Campos de vídeo.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldVideo {
    /// Identificador da trama (`u16`).
    FrameId = 0xB0,
    /// Índice do fragmento (`u8`).
    ChunkId = 0xB1,
    /// Total de fragmentos (`u8`).
    TotalChunks = 0xB2,
    /// Carga do fragmento (`raw`, até 128 B).
    Payload = 0xB3,
}

// ============================================================================
// CATÁLOGO UNIFICADO — IDENTIFICADORES JÁ CODIFICADOS PARA O FIO
// ============================================================================

/// Todos os campos com o byte exato do fio (`[TIPO:3][ID:5]`).
///
/// Usar estas constantes em vez de codificar à mão elimina a classe de bugs
/// "tipo certo, id errado" — o compilador carrega o catálogo, o programador
/// escolhe o nome. Valores herdados da V3, intocados.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BythosFieldId {
    // --- GPS: f32 nos angulares/velocidades, u8 nos satélites ---
    /// Latitude em graus (`f32`).
    GpsLatitude = 0x26,
    /// Longitude em graus (`f32`).
    GpsLongitude = 0x27,
    /// Altitude em metros (`f32`).
    GpsAltitude = 0x28,
    /// Velocidade em m/s (`f32`).
    GpsSpeed = 0x29,
    /// Rumo em graus (`f32`).
    GpsCourse = 0x2A,
    /// Satélites em vista (`u8`).
    GpsSatellites = 0xC7,
    /// HDOP (`f32`).
    GpsHdop = 0x2B,

    // --- IMU: tudo f32 ---
    /// Rolamento (`f32`).
    ImuRoll = 0x30,
    /// Arfagem (`f32`).
    ImuPitch = 0x31,
    /// Guinada (`f32`).
    ImuYaw = 0x32,
    /// Aceleração X (`f32`).
    ImuAccelX = 0x33,
    /// Aceleração Y (`f32`).
    ImuAccelY = 0x34,
    /// Aceleração Z (`f32`).
    ImuAccelZ = 0x35,
    /// Girómetro X (`f32`).
    ImuGyroX = 0x36,
    /// Girómetro Y (`f32`).
    ImuGyroY = 0x37,
    /// Girómetro Z (`f32`).
    ImuGyroZ = 0x38,
    /// Taxa de guinada (`f32`).
    ImuYawRate = 0x39,

    // --- Voo ---
    /// Altitude GPS (`f32`, id histórico fora de faixa — preservado).
    FlightAltGps = 0x40,
    /// Altitude barométrica (`f32`, id histórico — preservado).
    FlightAltBaro = 0x41,
    /// Velocidade vertical (`f32`, id histórico — preservado).
    FlightVSpeed = 0x42,
    /// Velocidade aerodinâmica (`f32`, id histórico — preservado).
    FlightAirspeed = 0x43,
    /// Duração do ciclo em µs (`u16`).
    FlightLoopTime = 0xA2,

    // --- Energia (ids históricos preservados) ---
    /// Tensão (`f32`).
    PowerBattV = 0x50,
    /// Corrente (`f32`).
    PowerBattI = 0x51,
    /// Consumo (`f32`).
    PowerBattCons = 0x52,
    /// Temperatura (`f32`).
    PowerBattTemp = 0x53,
    /// Estado de carga (`f32`).
    PowerBattSoc = 0x54,

    // --- Temperatura (ids históricos preservados) ---
    /// Sensor 1 (`f32`).
    Temp1 = 0x60,
    /// Sensor 2 (`f32`).
    Temp2 = 0x61,
    /// Sensor 3 (`f32`).
    Temp3 = 0x62,
    /// Sensor 4 (`f32`).
    Temp4 = 0x63,
    /// Ponte 1 (`f32`).
    TempBridge1 = 0x64,
    /// Ponte 2 (`f32`).
    TempBridge2 = 0x65,

    // --- Sistema ---
    /// Estado (`u8`).
    SystemState = 0xC0,
    /// Modo (`u8`).
    SystemMode = 0xC1,
    /// Uptime s (`u32`).
    SystemUptime = 0x82,
    /// RAM livre (`u32`).
    SystemFreeHeap = 0x83,
    /// Carga CPU % (`u8`).
    SystemCpuLoad = 0xC4,
    /// Carga ponte 1 % (`u8`).
    SystemBridge1Load = 0xC5,
    /// Carga ponte 2 % (`u8`).
    SystemBridge2Load = 0xC6,

    // --- Failsafe (`u8`) ---
    /// Motivo (`u8`).
    FailsafeReason = 0xC8,
    /// Ação (`u8`).
    FailsafeAction = 0xC9,
    /// Estado (`u8`).
    FailsafeState = 0xCA,

    // --- Vídeo ---
    /// Id da trama (`u16`).
    VideoFrameId = 0xA0,
    /// Id do fragmento (`u8`).
    VideoChunkId = 0xC3,
    /// Total de fragmentos (`u8`).
    VideoTotalChunks = 0xCB,
    /// Carga do fragmento (`raw`, até 128 B).
    VideoPayload = 0x00,
}

/// Comandos básicos do sistema (`0xC0–0xC4`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandBasic {
    /// Armar atuadores.
    Arm = 0xC0,
    /// Desarmar atuadores.
    Disarm = 0xC1,
    /// Mudar de modo.
    SetMode = 0xC2,
    /// Paragem de emergência.
    EmergencyStop = 0xC3,
    /// Desligar.
    Shutdown = 0xC4,
}

/// Comandos de controlo (`0xD0–0xD5`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandControl {
    /// Alvo de altitude.
    SetAltTarget = 0xD0,
    /// Alvo de velocidade.
    SetSpeed = 0xD1,
    /// Alvo de arfagem.
    SetPitch = 0xD2,
    /// Alvo de rolamento.
    SetRoll = 0xD3,
    /// Alvo de guinada.
    SetYaw = 0xD4,
    /// Alvo de rumo.
    SetHeading = 0xD5,
}

/// Comandos avançados (`0xE0–0xE3`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandAdvanced {
    /// Calibrar sensores.
    SensorCalib = 0xE0,
    /// Iniciar registo.
    StartLog = 0xE1,
    /// Parar registo.
    StopLog = 0xE2,
    /// Pedir tudo.
    GetAll = 0xE3,
}

/// Comandos de navegação (`0xF0–0xF2`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandNav {
    /// Próximos waypoints.
    NextWaypoints = 0xF0,
    /// Definir ponto de regresso.
    SetReturnPoint = 0xF1,
    /// Definir posição.
    SetPosition = 0xF2,
}

/// Todos os comandos unificados (mesmos valores, um só tipo para o fio).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Armar.
    Arm = 0xC0,
    /// Desarmar.
    Disarm = 0xC1,
    /// Mudar de modo.
    SetMode = 0xC2,
    /// Paragem de emergência.
    EmergencyStop = 0xC3,
    /// Desligar.
    Shutdown = 0xC4,
    /// Alvo de altitude.
    SetAltTarget = 0xD0,
    /// Alvo de velocidade.
    SetSpeed = 0xD1,
    /// Alvo de arfagem.
    SetPitch = 0xD2,
    /// Alvo de rolamento.
    SetRoll = 0xD3,
    /// Alvo de guinada.
    SetYaw = 0xD4,
    /// Alvo de rumo.
    SetHeading = 0xD5,
    /// Calibrar sensores.
    SensorCalib = 0xE0,
    /// Iniciar registo.
    StartLog = 0xE1,
    /// Parar registo.
    StopLog = 0xE2,
    /// Pedir tudo.
    GetAll = 0xE3,
    /// Próximos waypoints.
    NextWaypoints = 0xF0,
    /// Definir ponto de regresso.
    SetReturnPoint = 0xF1,
    /// Definir posição.
    SetPosition = 0xF2,
}

// ============================================================================
// ESTRUTURAS — CAMPOS E MENSAGENS (layout C estável para FFI)
// ============================================================================

/// Um campo Bythos: identificador com tipo + comprimento + dados.
///
/// `#[repr(C)]` congela o layout para a FFI não precisar de tradutores: o C usa
/// a mesma imagem binária. Só os primeiros `len` bytes de `data` são válidos —
/// invariante mantida pelo construtor e verificada pelo validador.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BythosField {
    /// Identificador codificado `[TIPO:3][ID:5]`.
    pub id: u8,
    /// Bytes válidos em `data`.
    pub len: u8,
    /// Carga (capacidade `MAX_FIELD_DATA`; conteúdo além de `len` é lixo).
    pub data: [u8; MAX_FIELD_DATA],
}

impl BythosField {
    /// Campo vazio (id e comprimento a zero).
    pub fn new() -> Self {
        Self {
            id: 0,
            len: 0,
            data: [0u8; MAX_FIELD_DATA],
        }
    }

    /// Constrói a partir de identificador já codificado e fatia de dados.
    ///
    /// Excesso de dados é **recusado** com `None` — a V3 truncava em silêncio
    /// (`min`), o que escondia telemetria cortada. Falhar alto aqui poupa
    /// horas de depuração no terreno.
    pub fn with_data(id: u8, data: &[u8]) -> Option<Self> {
        if data.len() > MAX_FIELD_DATA {
            return None;
        }
        let mut field = Self::new();
        field.id = id;
        field.len = data.len() as u8;
        field.data[..data.len()].copy_from_slice(data);
        Some(field)
    }

    /// Constrói a partir de tipo + id lógico (codifica o identificador).
    pub fn with_type_data(field_type: BythosFieldType, id: u8, data: &[u8]) -> Option<Self> {
        Self::with_data(bythos_field_id_encode(field_type as u8, id), data)
    }

    /// Tipo embutido no identificador (`None` = byte corrompido).
    pub fn field_type(&self) -> Option<BythosFieldType> {
        let (t, _) = bythos_field_id_decode(self.id);
        BythosFieldType::from_u8(t)
    }

    /// Id lógico (bits 4-0).
    pub fn field_id(&self) -> u8 {
        let (_, id) = bythos_field_id_decode(self.id);
        id
    }

    /// Vista só-leitura da carga válida.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

impl Default for BythosField {
    fn default() -> Self {
        Self::new()
    }
}

/// Campo de vídeo: mesma forma, tampão de 128 B para a carga do fragmento.
///
/// Existe separado do `BythosField` porque 128 B × 32 campos não cabem na pilha
/// de um MCU. A mensagem comporta **um** campo destes (ver `MAX_MESSAGE_SIZE`);
/// o túnel de vídeo (`crate::tunnel`) fragmenta e remonta por cima.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BythosVideoField {
    /// Identificador (normalmente `VideoPayload = 0x00`).
    pub id: u8,
    /// Bytes válidos em `data`.
    pub len: u8,
    /// Carga do fragmento.
    pub data: [u8; MAX_FIELD_VIDEO_DATA],
}

impl BythosVideoField {
    /// Campo de vídeo vazio.
    pub fn new() -> Self {
        Self {
            id: 0,
            len: 0,
            data: [0u8; MAX_FIELD_VIDEO_DATA],
        }
    }

    /// Constrói com verificação de limite (recusa, não trunca — ver `with_data`).
    pub fn with_data(id: u8, data: &[u8]) -> Option<Self> {
        if data.len() > MAX_FIELD_VIDEO_DATA {
            return None;
        }
        let mut field = Self::new();
        field.id = id;
        field.len = data.len() as u8;
        field.data[..data.len()].copy_from_slice(data);
        Some(field)
    }
}

impl Default for BythosVideoField {
    fn default() -> Self {
        Self::new()
    }
}

/// Mensagem Bythos V4 completa (imagem em memória, não o fio).
///
/// O fio é produzido por `codec::build_message`; esta estrutura é o rascunho
/// validado antes de selar (a selagem precisa da chave, que vive no cofre —
/// ver `crate::protocol::secure`). `#[repr(C)]` para partilha direta com C.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BythosMessage {
    /// Sempre `START_BYTE` (âncora de sincronização).
    pub start_byte: u8,
    /// Sempre `BYTHOS_VERSION` (`0x04`).
    pub version: u8,
    /// Endereço do emissor no anel (`0` = raiz).
    pub src: u16,
    /// Endereço do destino (`0xFFFF` = difusão).
    pub dst: u16,
    /// Tipo de mensagem.
    pub msg_id: u8,
    /// Sequência do emissor (diagnóstico; anti-replay usa `CTR`, não isto).
    pub seq_num: u16,
    /// Nº de campos válidos em `fields`.
    pub field_count: u8,
    /// Saltos restantes (decrementado a cada ponte).
    pub hops: u8,
    /// Os campos (só os primeiros `field_count` valem).
    pub fields: [BythosField; MAX_FIELDS],
    /// Ranhura de chave que selou a trama (`0xFF` = legado).
    pub key_id: u8,
    /// Contador monotónico que selou a trama (24 bits úteis).
    pub ctr: u32,
    /// Etiqueta de autenticação (válida após selar/analisar).
    pub tag: [u8; TAG_SIZE],
    /// CRC16 da trama selada (válido após selar/analisar).
    pub checksum: u16,
}

impl BythosMessage {
    /// Mensagem vazia com preâmbulo correto e resto a zero.
    pub fn new() -> Self {
        Self {
            start_byte: START_BYTE,
            version: BYTHOS_VERSION,
            src: 0,
            dst: BROADCAST_ADDR,
            msg_id: 0,
            seq_num: 0,
            field_count: 0,
            hops: HOP_DEFAULT,
            fields: [BythosField::new(); MAX_FIELDS],
            key_id: 0,
            ctr: 0,
            tag: [0u8; TAG_SIZE],
            checksum: 0,
        }
    }

    /// Atalho com origem, destino e tipo preenchidos.
    pub fn with_route(src: u16, dst: u16, msg_id: u8) -> Self {
        let mut msg = Self::new();
        msg.src = src;
        msg.dst = dst;
        msg.msg_id = msg_id;
        msg
    }

    /// Repõe o rascunho: preâmbulo correto, resto a zero.
    ///
    /// Ao contrário da V3 em C (que zerava `START/VERSION`), aqui o preâmbulo
    /// sobrevive — um rascunho limpo continua a ser "uma trama", não lixo.
    pub fn clear(&mut self) {
        self.start_byte = START_BYTE;
        self.version = BYTHOS_VERSION;
        self.src = 0;
        self.dst = BROADCAST_ADDR;
        self.msg_id = 0;
        self.seq_num = 0;
        self.field_count = 0;
        self.hops = HOP_DEFAULT;
        for field in self.fields.iter_mut() {
            *field = BythosField::new();
        }
        self.key_id = 0;
        self.ctr = 0;
        self.tag = [0u8; TAG_SIZE];
        self.checksum = 0;
    }

    /// Fatia dos campos válidos (para iterar sem contar à mão).
    pub fn valid_fields(&self) -> &[BythosField] {
        &self.fields[..self.field_count as usize]
    }
}

impl Default for BythosMessage {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CONVERSÕES LITTLE-ENDIAN (ordem do fio)
// ============================================================================

/// `f32` → 4 bytes LE.
pub fn float_to_bytes(value: f32) -> [u8; 4] {
    value.to_le_bytes()
}

/// 4 bytes LE → `f32`.
pub fn bytes_to_float(bytes: &[u8; 4]) -> f32 {
    f32::from_le_bytes(*bytes)
}

/// `f16` (meia precisão, bits IEEE-754) → 2 bytes LE.
///
/// Conversão por software com arredondamento para o mais próximo: a V3 declarava
/// o tipo mas nunca o implementava — aqui fecha-se essa dívida sem tabelas, para
/// continuar sem alocação e sem dependências.
pub fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 0x1) as u32;
    let exp = ((bits >> 10) & 0x1F) as i32;
    let mant = (bits & 0x3FF) as u32;
    let f32_bits: u32 = if exp == 0 {
        // Subnormal ou zero: renormaliza por deslocamento até ao bit implícito.
        if mant == 0 {
            sign << 31
        } else {
            let mut m = mant;
            let mut e = -14i32;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x3FF;
            (sign << 31) | (((e + 127) as u32) << 23) | (m << 13)
        }
    } else if exp == 31 {
        // Infinito ou NaN: preserva a carga útil do NaN.
        (sign << 31) | (0xFF << 23) | (mant << 13)
    } else {
        // Normal: rebasa o expoente de 15 para 127.
        (sign << 31) | (((exp + 112) as u32) << 23) | (mant << 13)
    };
    f32::from_bits(f32_bits)
}

/// `f32` → `f16` (bits), com saturação para infinito em vez de enrolar.
pub fn f32_to_f16(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 31) & 0x1) as u16;
    let exp = ((bits >> 23) & 0xFF) as i32 - 127;
    let mant = bits & 0x7FFFFF;
    if exp > 15 {
        // Transborda: infinito com o sinal (saturar é mais honesto que enrolar).
        return (sign << 15) | (0x1F << 10);
    }
    if exp < -24 {
        // Irrepresentável: zero com o sinal.
        return sign << 15;
    }
    if exp < -14 {
        // Subnormal: desloca a mantissa sem bit implícito.
        let shift = (-exp - 14) as u32;
        let m = (mant | 0x800000) >> (shift + 13);
        return (sign << 15) | (m as u16);
    }
    // Normal: rebasa e arredonda (meio-para-cima chega para telemetria).
    let m = ((mant + 0x1000) >> 13) as u16;
    let e = (exp + 15) as u16;
    if m == 0x400 {
        // O arredondamento transbordou a mantissa: sobe o expoente.
        return (sign << 15) | ((e + 1) << 10);
    }
    (sign << 15) | (e << 10) | (m & 0x3FF)
}

/// `i32` → 4 bytes LE.
pub fn int32_to_bytes(value: i32) -> [u8; 4] {
    value.to_le_bytes()
}

/// 4 bytes LE → `i32`.
pub fn bytes_to_int32(bytes: &[u8; 4]) -> i32 {
    i32::from_le_bytes(*bytes)
}

/// `u32` → 4 bytes LE.
pub fn uint32_to_bytes(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// 4 bytes LE → `u32`.
pub fn bytes_to_uint32(bytes: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*bytes)
}

/// `u16` → 2 bytes LE.
pub fn uint16_to_bytes(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

/// 2 bytes LE → `u16`.
pub fn bytes_to_uint16(bytes: &[u8; 2]) -> u16 {
    u16::from_le_bytes(*bytes)
}

// ============================================================================
// VALIDAÇÃO DE ALTO NÍVEL
// ============================================================================

/// `true` se o tipo de mensagem é conhecido.
pub fn is_valid_msg_id(id: u8) -> bool {
    BythosMsgId::is_valid(id)
}

/// Prioridade efetiva de uma mensagem, com regra de failsafe.
///
/// Com failsafe ativo, tudo sobe a `SuperCritical` exceto `Debug`, que desce a
/// `Low` — diagnóstico nunca pode preemptar a própria emergência que o gerou.
/// Sem failsafe, a tabela fixa abaixo decide (vídeo e debug vão no fundo).
pub fn get_msg_priority(msg_id: u8, failsafe_active: bool) -> u8 {
    if failsafe_active {
        return if msg_id == BythosMsgId::Debug as u8 {
            BythosPriority::Low as u8
        } else {
            BythosPriority::SuperCritical as u8
        };
    }

    match msg_id {
        0x10 => BythosPriority::Medium as u8,        // Heartbeat
        0x11 => BythosPriority::Medium as u8,        // Telemetry
        0x12 => BythosPriority::High as u8,          // Command
        0x13 => BythosPriority::High as u8,          // Ack
        0x14 => BythosPriority::SuperCritical as u8, // Failsafe
        0x15 => BythosPriority::Low as u8,           // Debug
        0x16 => BythosPriority::Low as u8,           // Video
        0x17 => BythosPriority::Medium as u8,        // Shell
        0x18 => BythosPriority::Medium as u8,        // SiData
        0x19 => BythosPriority::Medium as u8,        // Watchdog
        0x1A => BythosPriority::Medium as u8,        // Ping
        0x1B => BythosPriority::High as u8,          // Clock
        0x1C..=0x1F => BythosPriority::High as u8,   // Gestão do anel
        _ => BythosPriority::Low as u8,
    }
}

// ============================================================================
// IDENTIFICADOR DO BYTHOS BUS (arbitragem — a "cópia" do CAN, em Bythos)
// ============================================================================
//
// Quem conhece CAN reconhece isto de imediato — e essa é a intenção comercial:
// o Bythos Bus arbitra como o CAN (menor identificador ganha, sem destruir a
// trama), mas o identificador é 100% Bythos e o fio é o anel, não CAN.
//
// ```text
// BythosBusId (u32):
//   Bits 31-29: prioridade (3 bits, 0 = mais urgente)
//   Bits 28-25: grupo funcional (4 bits)
//   Bits 24-21: espécie de tráfego (4 bits)
//   Bits 20-5:  posição da origem no anel (16 bits)
//   Bits 4-0:   reservados (0)
// ```
//
// Convenção herdada do CAN: **valor menor = maior prioridade**. Emergência
// (`SuperCritical`, grupo `Safety`, espécie `Safety`, origem baixa) produz
// sempre os menores IDs — ganha qualquer contenção sem configuração.
//
// Nota de implementação: o RS-485 não tem dominante/recessivo elétrico como o
// CAN, por isso a arbitragem é por firmware (CSMA + backoff ordenado por este
// ID) na ponte, não no cobre. A semântica "menor ganha" é o contrato; o meio
// é detalhe da ponte (ver `docs/BYTHOS-BUS.md`).

/// Identificador de arbitragem do Bythos Bus (u32, valor menor = mais urgente).
///
/// Empacota prioridade + grupo + espécie + origem num inteiro comparável: para
/// ordenar transmissores basta comparar `u32` — sem tabelas, sem heap, sem
/// ramificações no caminho quente da contenção.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BythosBusId(pub u32);

impl BythosBusId {
    /// Reserva dos bits baixos (futuro: sub-prioridade ou CRC curto do emissor).
    const RESERVED_MASK: u32 = 0x1F;

    /// Constrói com validação estrita (`None` = metadados desconhecidos).
    ///
    /// Gamas: prioridade `0–4`, grupo `0x0–0xF`, espécie `0x0–0x8`, origem
    /// qualquer `u16`. Recusar aqui (comissionamento) em vez de mascarar evita
    /// dois nós com IDs colidentes por erro de configuração.
    pub fn new(priority: u8, group: u8, kind: u8, src: u16) -> Option<Self> {
        if priority > BythosPriority::Low as u8 || group > 0x0F {
            return None;
        }
        let kind_v = BythosKind::from_u8(kind)?;
        let id = ((priority as u32) << 29)
            | ((group as u32) << 25)
            | ((kind_v as u32) << 21)
            | ((src as u32) << 5);
        Some(Self(id))
    }

    /// Prioridade (bits 31-29).
    pub fn priority(self) -> u8 {
        ((self.0 >> 29) & 0x07) as u8
    }

    /// Grupo funcional (bits 28-25).
    pub fn group(self) -> u8 {
        ((self.0 >> 25) & 0x0F) as u8
    }

    /// Espécie de tráfego (bits 24-21).
    pub fn kind(self) -> u8 {
        ((self.0 >> 21) & 0x0F) as u8
    }

    /// Posição da origem (bits 20-5).
    pub fn src(self) -> u16 {
        ((self.0 >> 5) & 0xFFFF) as u16
    }

    /// Bits reservados (devem ser 0; futuro sem quebrar comparação).
    pub fn reserved(self) -> u8 {
        (self.0 & Self::RESERVED_MASK) as u8
    }

    /// Arbitragem: `true` se `self` ganha a `other` (menor valor ganha).
    ///
    /// É a regra de ouro do barramento, herdada do CAN: não-destrutiva na
    /// semântica (a trama perdedora recua e retransmite intacta), ainda que o
    /// mecanismo seja backoff por firmware e não dominante elétrico.
    pub fn wins_over(self, other: Self) -> bool {
        self.0 < other.0
    }

    /// Deriva o ID diretamente de um endereço + mensagem do fio.
    ///
    /// Atalho para a ponte: classifica (`kind_of`) e empacota numa chamada,
    /// sem o firmware decorar a tabela de espécies.
    pub fn from_route(
        src: u16,
        group: BythosGroup,
        msg_id: u8,
        priority: BythosPriority,
    ) -> Option<Self> {
        let kind = BythosKind::kind_of(msg_id)?;
        Self::new(priority as u8, group as u8, kind as u8, src)
    }
}

/// Atalho funcional (mesma validação, sem construir o struct primeiro).
pub fn make_bythos_bus_id(priority: u8, group: u8, kind: u8, src: u16) -> Option<u32> {
    BythosBusId::new(priority, group, kind, src).map(|id| id.0)
}

// ============================================================================
// ALIASES DEPRECADOS V3 → V4 (uma release; depois apagam-se)
// ============================================================================
//
// Migração mecânica: `sed -i 's/TLVBuilder/BythosBuilder/g'` e por aí fora.
// Cada alias aponta para o nome novo para o código V3 continuar a compilar com
// avisos durante a transição — sem bifurcar a lógica.

/// Nome V3 de `BythosFieldType`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosFieldType`")]
pub type FieldType = BythosFieldType;
/// Nome V3 de `BythosFieldId`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosFieldId`")]
pub type FieldId = BythosFieldId;
/// Nome V3 de `BythosField`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosField`")]
pub type TLVField = BythosField;
/// Nome V3 de `BythosVideoField`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosVideoField`")]
pub type TLVVideoField = BythosVideoField;
/// Nome V3 de `BythosMessage`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosMessage`")]
pub type TLVMessage = BythosMessage;
/// Nome V3 de `BythosPriority`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosPriority`")]
pub type PriorityLevel = BythosPriority;
/// Nome V3 de `BythosGroup`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosGroup`")]
pub type CanGroup = BythosGroup;
/// Nome V3 de `BythosKind`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosKind`")]
pub type CanMsgType = BythosKind;
/// Nome V3 de `BythosMsgId`. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `BythosMsgId`")]
pub type MsgId = BythosMsgId;
/// Constante V3 renomeada. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `MAX_FIELD_DATA`")]
pub const MAX_TLV_DATA: usize = MAX_FIELD_DATA;
/// Constante V3 renomeada. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `MAX_FIELD_VIDEO_DATA`")]
pub const MAX_TLV_VIDEO_DATA: usize = MAX_FIELD_VIDEO_DATA;
/// Constante V3 renomeada. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `MAX_FIELDS`")]
pub const MAX_TLV_FIELDS: usize = MAX_FIELDS;
/// Constante V3 renomeada. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `FIELD_HEADER_SIZE`")]
pub const TLV_HEADER_SIZE: usize = FIELD_HEADER_SIZE;
/// Função V3 renomeada. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `bythos_field_id_encode`")]
pub fn field_id_encode(field_type: u8, field_id: u8) -> u8 {
    bythos_field_id_encode(field_type, field_id)
}
/// Função V3 renomeada. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `bythos_field_id_decode`")]
pub fn field_id_decode(field_id: u8) -> (u8, u8) {
    bythos_field_id_decode(field_id)
}
/// Função V3 renomeada. A usar só durante a migração.
#[deprecated(since = "4.0.0", note = "usar `is_valid_bythos_field_id`")]
pub fn is_valid_field_id(field_id: u8) -> bool {
    is_valid_bythos_field_id(field_id)
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constantes_v4() {
        // Estrada nova: cabeçalho 11, sobrecarga 21, máximo 1205.
        assert_eq!(START_BYTE, 0xAA);
        assert_eq!(BYTHOS_VERSION, 0x04);
        assert_eq!(BYTHOS_HEADER_SIZE, 11);
        assert_eq!(SEC_HDR_SIZE, 4);
        assert_eq!(TAG_SIZE, 4);
        assert_eq!(CRC16_SIZE, 2);
        assert_eq!(BYTHOS_OVERHEAD, 21);
        assert_eq!(MAX_MESSAGE_SIZE, 1205);
        assert_eq!(BROADCAST_ADDR, 0xFFFF);
        assert_eq!(ROOT_ADDR, 0);
        assert_eq!(HOP_DEFAULT, 32);
    }

    #[test]
    fn test_msg_id_v4_completo() {
        // Base V3 intacta + 4 do anel.
        assert!(BythosMsgId::is_valid(0x10));
        assert!(BythosMsgId::is_valid(0x1B));
        assert!(BythosMsgId::is_valid(0x1C));
        assert!(BythosMsgId::is_valid(0x1F));
        assert!(!BythosMsgId::is_valid(0x00));
        assert!(!BythosMsgId::is_valid(0x20));
        assert_eq!(BythosMsgId::Hello as u8, 0x1C);
        assert_eq!(BythosMsgId::Count as u8, 0x1D);
        assert_eq!(BythosMsgId::RingOpen as u8, 0x1E);
        assert_eq!(BythosMsgId::WiringFault as u8, 0x1F);
    }

    #[test]
    fn test_tipo_campo() {
        assert_eq!(BythosFieldType::from_u8(0), Some(BythosFieldType::Raw));
        assert_eq!(BythosFieldType::from_u8(1), Some(BythosFieldType::Float32));
        assert_eq!(BythosFieldType::from_u8(7), Some(BythosFieldType::Bool));
        assert_eq!(BythosFieldType::from_u8(8), None);
        assert_eq!(BythosFieldType::Float32.default_size(), 4);
        assert_eq!(BythosFieldType::Raw.default_size(), 0);
    }

    #[test]
    fn test_field_id_encode_decode() {
        // Tipo 1 (f32), id 6 → 0x26 (valor herdado da V3).
        let fid = bythos_field_id_encode(1, 6);
        assert_eq!(fid, 0x26);
        assert_eq!(bythos_field_id_decode(fid), (1, 6));
        // Tipo 6 (u8), id 0 → 0xC0.
        assert_eq!(bythos_field_id_encode(6, 0), 0xC0);
        // Tipo 0 (raw), id 0 → 0x00 (carga vídeo).
        assert_eq!(bythos_field_id_encode(0, 0), 0x00);
    }

    #[test]
    fn test_endereco_bythos() {
        // Rota válida com metadados conhecidos.
        let route = make_bythos_route(27, 2, 0x2, 0x0, 2).unwrap();
        assert_eq!(route.0.pos, 27);
        assert_eq!(route.1.pos, 2);
        // Grupo/espécie desconhecidos recusam no comissionamento.
        assert!(make_bythos_route(1, 2, 0x10, 0x0, 2).is_none());
        assert!(make_bythos_route(1, 2, 0x2, 0x9, 2).is_none());
        // Difusão e segurança.
        let bc = BythosAddress::broadcast(BythosKind::Data, BythosPriority::Medium);
        assert_eq!(bc.pos, BROADCAST_ADDR);
        let safe = BythosAddress {
            pos: 4,
            group: BythosGroup::Safety,
            kind: BythosKind::Safety,
            priority: BythosPriority::SuperCritical,
        };
        assert!(safe.is_safety());
        assert!(!bc.is_safety());
    }

    #[test]
    fn test_kind_of() {
        // Classificação sem CAN ID: deriva do MSG.
        assert_eq!(BythosKind::kind_of(0x11), Some(BythosKind::Data));
        assert_eq!(BythosKind::kind_of(0x12), Some(BythosKind::Cmd));
        assert_eq!(BythosKind::kind_of(0x14), Some(BythosKind::Safety));
        assert_eq!(BythosKind::kind_of(0x1C), Some(BythosKind::Ring));
        assert_eq!(BythosKind::kind_of(0x1F), Some(BythosKind::Ring));
        assert_eq!(BythosKind::kind_of(0x20), None);
    }

    #[test]
    fn test_etiqueta_legada() {
        // XOR herdado: só para ler o parque V3 em migração.
        let tag = compute_legacy_tag(0x42, 0x11, 0x2A, 0x01);
        assert_eq!(tag, 0x42 ^ 0x11 ^ 0x2A ^ 0x01);
        assert!(validate_legacy_tag(tag, 0x42, 0x11, 0x2A, 0x01));
        assert!(!validate_legacy_tag(tag, 0x43, 0x11, 0x2A, 0x01));
    }

    #[test]
    fn test_conversoes() {
        // Ida-volta f32/i32/u32/u16.
        let f: f32 = 2.5;
        assert!((f - bytes_to_float(&float_to_bytes(f))).abs() < f32::EPSILON);
        assert_eq!(bytes_to_int32(&int32_to_bytes(-12345)), -12345);
        assert_eq!(bytes_to_uint32(&uint32_to_bytes(0xDEADBEEF)), 0xDEADBEEF);
        assert_eq!(bytes_to_uint16(&uint16_to_bytes(0x1234)), 0x1234);
    }

    #[test]
    fn test_f16_ida_volta() {
        // Dívida V3 fechada: f16 funciona por software, sem tabelas.
        for v in [0.0f32, 1.0, -1.5, 100.25, 0.0001, 65504.0] {
            let half = f32_to_f16(v);
            let back = f16_to_f32(half);
            let tol = v.abs() * 0.002 + 0.001;
            assert!((v - back).abs() <= tol, "v={v} back={back}");
        }
        // Casos especiais: zero com sinal, infinito, NaN.
        assert_eq!(f16_to_f32(0x8000).to_bits() & 0x8000_0000, 0x8000_0000);
        assert!(f16_to_f32(0x7C00).is_infinite());
        assert!(f16_to_f32(0x7E00).is_nan());
        assert_eq!(f32_to_f16(f32::INFINITY), 0x7C00);
    }

    #[test]
    fn test_campo_recusa_em_vez_de_truncar() {
        // 33 B num campo normal: recusado (a V3 truncava em silêncio).
        assert!(BythosField::with_data(0xC0, &[0u8; 33]).is_none());
        assert!(BythosField::with_data(0xC0, &[0u8; 32]).is_some());
        // Vídeo aceita até 128.
        assert!(BythosVideoField::with_data(0x00, &[0u8; 128]).is_some());
        assert!(BythosVideoField::with_data(0x00, &[0u8; 129]).is_none());
        // Tipo/id lidos do byte.
        let f =
            BythosField::with_type_data(BythosFieldType::Float32, 6, &float_to_bytes(1.5)).unwrap();
        assert_eq!(f.id, 0x26);
        assert_eq!(f.field_type(), Some(BythosFieldType::Float32));
        assert_eq!(f.field_id(), 6);
    }

    #[test]
    fn test_mensagem_rascunho() {
        let mut msg = BythosMessage::with_route(27, 2, 0x11);
        assert_eq!(msg.src, 27);
        assert_eq!(msg.dst, 2);
        assert_eq!(msg.hops, HOP_DEFAULT);
        assert_eq!(msg.start_byte, START_BYTE);
        msg.clear();
        // Limpar repõe preâmbulo e difusão — nunca lixo.
        assert_eq!(msg.start_byte, START_BYTE);
        assert_eq!(msg.version, BYTHOS_VERSION);
        assert_eq!(msg.dst, BROADCAST_ADDR);
        assert_eq!(msg.field_count, 0);
    }

    #[test]
    fn test_prioridade_com_failsafe() {
        assert_eq!(
            get_msg_priority(0x14, false),
            BythosPriority::SuperCritical as u8
        );
        assert_eq!(get_msg_priority(0x16, false), BythosPriority::Low as u8);
        assert_eq!(
            get_msg_priority(0x10, true),
            BythosPriority::SuperCritical as u8
        );
        // Debug nunca preempta a emergência que o gerou.
        assert_eq!(get_msg_priority(0x15, true), BythosPriority::Low as u8);
        // Gestão do anel é urgente.
        assert_eq!(get_msg_priority(0x1C, false), BythosPriority::High as u8);
    }

    #[test]
    fn test_grupos_funcionais() {
        // Nomes de função, valores herdados da V3.
        assert_eq!(BythosGroup::None as u8, 0x0);
        assert_eq!(BythosGroup::Control as u8, 0x1);
        assert_eq!(BythosGroup::Sensors as u8, 0x2);
        assert_eq!(BythosGroup::Safety as u8, 0x4);
        assert_eq!(BythosGroup::Vision as u8, 0x6);
        assert!(BythosGroup::from_u8(0x10).is_none());
    }

    #[test]
    fn test_bus_id_empacota_e_desempacota() {
        // Ida-volta total dos 4 campos + reserva a zero.
        let id = BythosBusId::new(2, 0x6, 0x0, 27).unwrap();
        assert_eq!(id.priority(), 2);
        assert_eq!(id.group(), 0x6);
        assert_eq!(id.kind(), 0x0);
        assert_eq!(id.src(), 27);
        assert_eq!(id.reserved(), 0);
        // Layout bit a bit (o contrato com o firmware da ponte; a espécie 0
        // não desloca nada, por isso não aparece na expressão).
        assert_eq!(id.0, (2u32 << 29) | (0x6u32 << 25) | (27u32 << 5));
        // Gamas inválidas recusam no comissionamento, sem mascarar.
        assert!(BythosBusId::new(5, 0x6, 0x0, 1).is_none());
        assert!(BythosBusId::new(2, 0x10, 0x0, 1).is_none());
        assert!(BythosBusId::new(2, 0x6, 0x9, 1).is_none());
        assert_eq!(make_bythos_bus_id(2, 0x6, 0x0, 27), Some(id.0));
        assert_eq!(make_bythos_bus_id(9, 0x6, 0x0, 27), None);
    }

    #[test]
    fn test_bus_id_menor_ganha_como_no_can() {
        // Emergência de segurança esmaga telemetria normal — a regra de ouro.
        let emergencia = BythosBusId::new(0, 0x4, 0x7, 4).unwrap();
        let telemetria = BythosBusId::new(3, 0x2, 0x0, 27).unwrap();
        assert!(emergencia.wins_over(telemetria));
        assert!(!telemetria.wins_over(emergencia));
        // Em igualdade de urgência, a origem mais baixa desempata (determinístico).
        let a = BythosBusId::new(2, 0x2, 0x0, 5).unwrap();
        let b = BythosBusId::new(2, 0x2, 0x0, 6).unwrap();
        assert!(a.wins_over(b));
        // Ordenação total: dá para ordenar filas com `<` direto.
        assert!(emergencia < telemetria);
    }

    #[test]
    fn test_bus_id_da_rota_do_fio() {
        // Da mensagem real para o ID de arbitragem numa chamada.
        let id = BythosBusId::from_route(27, BythosGroup::Sensors, 0x11, BythosPriority::Medium)
            .unwrap();
        assert_eq!(
            (id.priority(), id.group(), id.kind(), id.src()),
            (3, 0x2, 0x0, 27)
        );
        // Tipo desconhecido não arbitra (não há espécie, não há fila).
        assert!(
            BythosBusId::from_route(27, BythosGroup::Sensors, 0x20, BythosPriority::Medium)
                .is_none()
        );
    }
}
