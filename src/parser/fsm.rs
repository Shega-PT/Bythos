//! # Analisador Bythos — Máquina de Estados Byte-a-Byte (v4.0.0)
//!
//! Reconstrói tramas V4 a partir de um fluxo série, sem alocar e sem chaves:
//! verifica **estrutura + CRC**; `TAG` e anti-replay ficam para o destino final
//! (`codec::validate_message` sobre `completed_frame`). Separar assim permite à
//! ponte encaminhar milhares de tramas alheias por segundo sem acordar o
//! elemento seguro.
//!
//! Endurecido face à V3: cada byte armazenado é medido contra o tampão (nunca
//! escreve fora, mesmo com `LEN` mentiroso), o timeout é imposto de verdade à
//! entrada de cada byte (na V3 existia mas nunca disparava), e o limite de
//! saltos é exposto para o encaminhador decidir antes de repetir.

use crate::protocol::codec::parse_fields;
use crate::protocol::crc16::calc_crc16;
use crate::protocol::types::*;

// ============================================================================
// ESTADOS E ERROS
// ============================================================================

/// Estados da máquina (10 na V4: o selo partiu-se em `SEC_HDR` + `TAG`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserState {
    /// À espera da âncora `START_BYTE`.
    WaitStart = 0,
    /// A juntar os 11 bytes do cabeçalho.
    WaitHeader = 1,
    /// À espera do identificador de um campo.
    WaitFieldId = 2,
    /// À espera do comprimento de um campo.
    WaitFieldLen = 3,
    /// A juntar os dados de um campo.
    WaitFieldData = 4,
    /// A juntar os 4 bytes do cabeçalho de segurança.
    WaitSecHdr = 5,
    /// A juntar os 4 bytes da etiqueta.
    WaitTag = 6,
    /// Byte baixo do CRC16.
    WaitCrc16Lo = 7,
    /// Byte alto do CRC16 — trama completa.
    WaitCrc16Hi = 8,
}

/// Códigos de erro (todos os modos de falha do fio, sem colapsar).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserError {
    /// Byte aceite / trama completa sem erros.
    Ok = 0,
    /// Byte fora de trama (sem `START_BYTE` à vista).
    ErrStart = 1,
    /// Versão diferente de `0x04`.
    ErrVersion = 2,
    /// Tipo de mensagem desconhecido.
    ErrMsgId = 3,
    /// Nº de campos acima de `MAX_FIELDS`.
    ErrFieldCount = 4,
    /// Identificador de campo inválido.
    ErrFieldId = 5,
    /// Comprimento de campo inválido (>32, ou >128 fora de carga vídeo).
    ErrFieldLen = 6,
    /// CRC16 não bate.
    ErrChecksum = 7,
    /// Silêncio entre bytes além do limite.
    ErrTimeout = 8,
    /// Tampão cheio antes do fim da trama (ataque ou `LEN` mentiroso).
    ErrOverflow = 9,
    /// Saltos esgotados (a trama não deve ser repetida).
    ErrHops = 10,
}

// ============================================================================
// ANALISADOR
// ============================================================================

/// Analisador de tramas V4: sem heap, sem chaves, sem relógio próprio.
///
/// Todo o estado cabe em ~1,3 KiB (`raw_buffer` de 1205 B + mensagem): corre na
/// pilha de um MCU sem pensar duas vezes. O `TAG` **não** é verificado aqui —
/// ver doc do módulo para o porquê.
pub struct Parser {
    /// Estado atual.
    state: ParserState,
    /// Bytes acumulados da trama em construção.
    raw_buffer: [u8; MAX_MESSAGE_SIZE],
    /// Posição de escrita (invariante: sempre `< MAX_MESSAGE_SIZE`).
    raw_offset: usize,
    /// Dados que faltam no campo atual.
    field_data_remaining: usize,
    /// Se já passou uma carga vídeo grande nesta trama (só uma, ver spec).
    video_big_seen: bool,
    /// Mensagem reconstruída (válida quando `has_message`).
    msg: BythosMessage,
    /// Cópia exata dos bytes da trama completa no fio.
    ///
    /// Necessária porque `validate_message` (TAG + replay) trabalha sobre bytes,
    /// não sobre a estrutura: sem esta cópia, o destino teria de re-serializar
    /// (e a re-serialização com outro `CTR` daria outro `TAG` — impossível).
    frame_copy: [u8; MAX_MESSAGE_SIZE],
    /// Comprimento da trama completa (para `completed_frame`).
    completed_len: usize,
    /// Silêncio máximo entre bytes, em µs (timeout de trama partida).
    max_frame_gap_us: u32,
    /// Carimbo do último byte aceite, em µs.
    last_byte_time_us: u64,
    /// Trama completa disponível.
    has_message: bool,
    /// Tramas boas desde o arranque (diagnóstico).
    success_count: u32,
    /// Erros desde o arranque (diagnóstico).
    error_count: u32,
    /// Último erro (persiste para diagnóstico após reinício interno).
    last_error: ParserError,
    /// Saída de depuração (só com `std`).
    debug: bool,
}

impl Parser {
    /// Analisador virgem, no estado `WaitStart`.
    pub fn new() -> Self {
        Self {
            state: ParserState::WaitStart,
            raw_buffer: [0u8; MAX_MESSAGE_SIZE],
            raw_offset: 0,
            field_data_remaining: 0,
            video_big_seen: false,
            msg: BythosMessage::new(),
            frame_copy: [0u8; MAX_MESSAGE_SIZE],
            completed_len: 0,
            // 1 s por defeito: folgado para 250 kbps (trama máxima ≈ 40 ms) e
            // apertado o suficiente para não segurar lixo uma eternidade.
            max_frame_gap_us: 1_000_000,
            last_byte_time_us: 0,
            has_message: false,
            success_count: 0,
            error_count: 0,
            last_error: ParserError::Ok,
            debug: false,
        }
    }

    /// Guarda um byte no tampão com medição de limite.
    ///
    /// O ponto único de escrita: quem o contorna está a introduzir OOB. Devolve
    /// `false` se o tampão estiver cheio — o chamador reinicia com `ErrOverflow`.
    fn store(&mut self, byte: u8) -> bool {
        if self.raw_offset >= MAX_MESSAGE_SIZE {
            return false;
        }
        self.raw_buffer[self.raw_offset] = byte;
        self.raw_offset += 1;
        true
    }

    /// Falha com reinício: conta, regista, volta ao início e devolve o código.
    ///
    /// Centralizar aqui garante que nenhum caminho de erro se esquece de contar
    /// ou deixa a máquina presa a meio de uma trama morta.
    fn fail(&mut self, err: ParserError) -> ParserError {
        self.last_error = err;
        self.error_count += 1;
        self.state = ParserState::WaitStart;
        self.raw_offset = 0;
        self.field_data_remaining = 0;
        self.video_big_seen = false;
        err
    }

    /// Alimenta um byte à máquina.
    ///
    /// Ordem de verificações: timeout primeiro (trama velha morre antes de
    /// misturar bytes de duas emissões), depois limite de tampão, depois a
    /// lógica do estado. `Ok` não significa "trama pronta" — significa "byte
    /// aceite"; a trama pronta anuncia-se com `has_message()`.
    pub fn feed(&mut self, byte: u8) -> ParserError {
        // Timeout real: se o silêncio excedeu o limite a meio de uma trama, a
        // trama morre aqui — na V3 esta verificação existia mas nunca corria.
        if self.state != ParserState::WaitStart && self.is_timed_out() {
            self.fail(ParserError::ErrTimeout);
            // Cai para `WaitStart` e reprocessa o byte como possível âncora.
        }
        let now = self.get_timestamp_us();
        match self.state {
            // --- Âncora: só o START_BYTE abre trama; o resto é ruído. ---
            ParserState::WaitStart => {
                self.last_byte_time_us = now;
                if byte == START_BYTE {
                    self.raw_offset = 0;
                    self.video_big_seen = false;
                    // `store` não falha aqui (tampão vazio), mas mede-se na
                    // mesma — invariantes absolutas não têm exceções.
                    if !self.store(byte) {
                        return self.fail(ParserError::ErrOverflow);
                    }
                    self.state = ParserState::WaitHeader;
                    ParserError::Ok
                } else {
                    self.last_error = ParserError::ErrStart;
                    self.error_count += 1;
                    ParserError::ErrStart
                }
            }

            // --- Cabeçalho: junta 11 bytes e valida versão/tipo/contagem. ---
            ParserState::WaitHeader => {
                self.last_byte_time_us = now;
                if !self.store(byte) {
                    return self.fail(ParserError::ErrOverflow);
                }
                if self.raw_offset >= BYTHOS_HEADER_SIZE {
                    if self.raw_buffer[1] != BYTHOS_VERSION {
                        return self.fail(ParserError::ErrVersion);
                    }
                    if !BythosMsgId::is_valid(self.raw_buffer[6]) {
                        return self.fail(ParserError::ErrMsgId);
                    }
                    let count = self.raw_buffer[9];
                    if count as usize > MAX_FIELDS {
                        return self.fail(ParserError::ErrFieldCount);
                    }
                    // Saltos a zero vindos do fio não viajam: a ponte que recebe
                    // `HOPS == 0` consome localmente e nunca repete (ver `ErrHops`
                    // no fecho da trama).
                    self.state = if count == 0 {
                        ParserState::WaitSecHdr
                    } else {
                        ParserState::WaitFieldId
                    };
                }
                ParserError::Ok
            }

            // --- Identificador do campo: tipo tem de ser conhecido. ---
            ParserState::WaitFieldId => {
                self.last_byte_time_us = now;
                if !self.store(byte) {
                    return self.fail(ParserError::ErrOverflow);
                }
                let (ftype, _) = bythos_field_id_decode(byte);
                if BythosFieldType::from_u8(ftype).is_none() {
                    return self.fail(ParserError::ErrFieldId);
                }
                self.state = ParserState::WaitFieldLen;
                ParserError::Ok
            }

            // --- Comprimento: 32 normal; 128 só em carga vídeo raw, uma vez. ---
            ParserState::WaitFieldLen => {
                self.last_byte_time_us = now;
                if !self.store(byte) {
                    return self.fail(ParserError::ErrOverflow);
                }
                let len = byte as usize;
                if len <= MAX_FIELD_DATA {
                    self.field_data_remaining = len;
                    if len == 0 {
                        self.check_next_field_or_sec();
                    } else {
                        self.state = ParserState::WaitFieldData;
                    }
                    return ParserError::Ok;
                }
                // Caminho grande: só carga vídeo `raw` 0x00, só uma por trama.
                let fid = self.raw_buffer[self.raw_offset - 2];
                let (ftype, _) = bythos_field_id_decode(fid);
                let is_video =
                    ftype == BythosFieldType::Raw as u8 && fid == BythosFieldId::VideoPayload as u8;
                if len <= MAX_FIELD_VIDEO_DATA && is_video && !self.video_big_seen {
                    self.video_big_seen = true;
                    self.field_data_remaining = len;
                    self.state = ParserState::WaitFieldData;
                    ParserError::Ok
                } else {
                    self.fail(ParserError::ErrFieldLen)
                }
            }

            // --- Dados: conta em baixo até zero, depois decide o próximo. ---
            ParserState::WaitFieldData => {
                self.last_byte_time_us = now;
                if !self.store(byte) {
                    return self.fail(ParserError::ErrOverflow);
                }
                self.field_data_remaining -= 1;
                if self.field_data_remaining == 0 {
                    self.check_next_field_or_sec();
                }
                ParserError::Ok
            }

            // --- Segurança e etiqueta: 8 bytes opacos (o destino valida). ---
            ParserState::WaitSecHdr | ParserState::WaitTag => {
                self.last_byte_time_us = now;
                if !self.store(byte) {
                    return self.fail(ParserError::ErrOverflow);
                }
                // 4 bytes de SEC_HDR após os campos, depois 4 de TAG.
                let fields_end = self.fields_end_offset();
                match self.state {
                    ParserState::WaitSecHdr if self.raw_offset >= fields_end + SEC_HDR_SIZE => {
                        self.state = ParserState::WaitTag;
                    }
                    ParserState::WaitTag
                        if self.raw_offset >= fields_end + SEC_HDR_SIZE + TAG_SIZE =>
                    {
                        self.state = ParserState::WaitCrc16Lo;
                    }
                    _ => {}
                }
                ParserError::Ok
            }

            // --- CRC baixo/alto: no alto, a trama fecha e valida-se. ---
            ParserState::WaitCrc16Lo => {
                self.last_byte_time_us = now;
                if !self.store(byte) {
                    return self.fail(ParserError::ErrOverflow);
                }
                self.state = ParserState::WaitCrc16Hi;
                ParserError::Ok
            }
            ParserState::WaitCrc16Hi => {
                self.last_byte_time_us = now;
                if !self.store(byte) {
                    return self.fail(ParserError::ErrOverflow);
                }
                // CRC sobre tudo menos ele próprio.
                let crc_at = self.raw_offset - CRC16_SIZE;
                let received =
                    u16::from_le_bytes([self.raw_buffer[crc_at], self.raw_buffer[crc_at + 1]]);
                if calc_crc16(&self.raw_buffer[..crc_at]) != received {
                    return self.fail(ParserError::ErrChecksum);
                }
                // Extrai os campos para a mensagem (sem verificar TAG: sem chaves).
                let fields_start = BYTHOS_HEADER_SIZE;
                let fields_end = crc_at - SEC_HDR_SIZE - TAG_SIZE;
                let mut parsed = [BythosField::new(); MAX_FIELDS];
                match parse_fields(&self.raw_buffer[fields_start..fields_end], &mut parsed) {
                    Ok(count) => {
                        // Coerência: o analisado tem de casar com o anunciado.
                        if count != self.raw_buffer[9] as usize {
                            return self.fail(ParserError::ErrChecksum);
                        }
                        self.msg.start_byte = self.raw_buffer[0];
                        self.msg.version = self.raw_buffer[1];
                        self.msg.src = u16::from_le_bytes([self.raw_buffer[2], self.raw_buffer[3]]);
                        self.msg.dst = u16::from_le_bytes([self.raw_buffer[4], self.raw_buffer[5]]);
                        self.msg.msg_id = self.raw_buffer[6];
                        self.msg.seq_num =
                            u16::from_le_bytes([self.raw_buffer[7], self.raw_buffer[8]]);
                        self.msg.field_count = count as u8;
                        self.msg.hops = self.raw_buffer[10];
                        self.msg.fields[..count].copy_from_slice(&parsed[..count]);
                        self.msg.key_id = self.raw_buffer[fields_end];
                        let ctr = (self.raw_buffer[fields_end + 1] as u32)
                            | ((self.raw_buffer[fields_end + 2] as u32) << 8)
                            | ((self.raw_buffer[fields_end + 3] as u32) << 16);
                        self.msg.ctr = ctr;
                        self.msg.tag.copy_from_slice(
                            &self.raw_buffer
                                [fields_end + SEC_HDR_SIZE..fields_end + SEC_HDR_SIZE + TAG_SIZE],
                        );
                        self.msg.checksum = received;
                        self.frame_copy[..self.raw_offset]
                            .copy_from_slice(&self.raw_buffer[..self.raw_offset]);
                        self.completed_len = self.raw_offset;
                        self.has_message = true;
                        self.success_count += 1;
                        self.last_error = ParserError::Ok;
                        self.state = ParserState::WaitStart;
                        self.raw_offset = 0;
                        self.video_big_seen = false;
                        // Saltos esgotados: a trama é válida localmente, mas o
                        // encaminhador não a pode repetir — sinaliza-se com o
                        // último erro sem invalidar a mensagem.
                        if self.msg.hops == 0 {
                            self.last_error = ParserError::ErrHops;
                        }
                        ParserError::Ok
                    }
                    Err(_) => self.fail(ParserError::ErrChecksum),
                }
            }
        }
    }

    /// Fim dos campos já acumulados (percorre o que está no tampão).
    ///
    /// Re-percorrer a cada campo é O(n²) no pior caso (32 campos): irrelevante
    /// — n ≤ 32 e cada passo são 2 leituras. Clareza primeiro; o caminho quente
    /// do anel é encaminhar pelo `DST` do cabeçalho, não re-analisar campos.
    fn fields_end_offset(&self) -> usize {
        let count = if self.raw_offset > 9 {
            self.raw_buffer[9] as usize
        } else {
            0
        };
        let mut at = BYTHOS_HEADER_SIZE;
        let mut done = 0;
        while done < count && at + FIELD_HEADER_SIZE <= self.raw_offset {
            let len = self.raw_buffer[at + 1] as usize;
            at += FIELD_HEADER_SIZE + len;
            done += 1;
        }
        at
    }

    /// Decide entre mais um campo ou o reboque de segurança.
    fn check_next_field_or_sec(&mut self) {
        let count = self.raw_buffer[9] as usize;
        let mut at = BYTHOS_HEADER_SIZE;
        let mut done = 0;
        while done < count && at + FIELD_HEADER_SIZE <= self.raw_offset {
            let len = self.raw_buffer[at + 1] as usize;
            at += FIELD_HEADER_SIZE + len;
            done += 1;
        }
        self.state = if done >= count {
            ParserState::WaitSecHdr
        } else {
            ParserState::WaitFieldId
        };
    }

    /// Trama completa disponível para consumo.
    pub fn has_message(&self) -> bool {
        self.has_message
    }

    /// Mensagem reconstruída (só válida com `has_message()`).
    pub fn get_message(&self) -> &BythosMessage {
        &self.msg
    }

    /// Bytes exatos da trama completa no fio (para `validate_message` no destino
    /// final: `TAG` + replay com chave e tabela de pares).
    pub fn completed_frame(&self) -> Option<&[u8]> {
        if !self.has_message {
            return None;
        }
        Some(&self.frame_copy[..self.completed_len])
    }

    /// Copia a mensagem para o destino (falso sem trama completa).
    pub fn copy_message(&self, output: &mut BythosMessage) -> bool {
        if !self.has_message {
            return false;
        }
        *output = self.msg.clone();
        true
    }

    /// Confirma o consumo: liberta a máquina para a próxima trama.
    ///
    /// Esquecer o `acknowledge` deixa `has_message` preso em verdadeiro — erro
    /// clássico de integração, documentado aqui para não se repetir.
    pub fn acknowledge(&mut self) {
        self.has_message = false;
        self.completed_len = 0;
        self.msg.clear();
    }

    /// Reinício total (estado, tampão, mensagem pendente).
    pub fn reset(&mut self) {
        self.state = ParserState::WaitStart;
        self.raw_offset = 0;
        self.field_data_remaining = 0;
        self.video_big_seen = false;
        self.has_message = false;
        self.completed_len = 0;
        self.msg.clear();
    }

    /// Silêncio máximo entre bytes, em µs.
    pub fn set_max_frame_gap(&mut self, micros: u32) {
        self.max_frame_gap_us = micros;
    }

    /// `true` se o silêncio atual excede o limite (fora de `WaitStart`).
    pub fn is_timed_out(&self) -> bool {
        if self.state == ParserState::WaitStart {
            return false;
        }
        let now = self.get_timestamp_us();
        now.wrapping_sub(self.last_byte_time_us) > self.max_frame_gap_us as u64
    }

    /// Último erro (persiste após reinício interno — diagnóstico, não estado).
    pub fn get_last_error(&self) -> ParserError {
        self.last_error
    }

    /// Estado atual (diagnóstico e testes).
    pub fn get_current_state(&self) -> ParserState {
        self.state
    }

    /// Contador de tramas boas.
    pub fn get_success_count(&self) -> u32 {
        self.success_count
    }

    /// Contador de erros.
    pub fn get_error_count(&self) -> u32 {
        self.error_count
    }

    /// Interruptor de depuração (só produz saída com `std`).
    pub fn set_debug(&mut self, enable: bool) {
        self.debug = enable;
    }

    /// Relógio em µs; sem `std` devolve 0 (timeout desligado, documentado).
    fn get_timestamp_us(&self) -> u64 {
        #[cfg(feature = "std")]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(d) => d.as_micros() as u64,
                Err(_) => 0,
            }
        }
        #[cfg(not(feature = "std"))]
        {
            0
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// Nome de estado para diagnóstico.
pub fn parser_state_to_string(state: ParserState) -> &'static str {
    match state {
        ParserState::WaitStart => "WAIT_START",
        ParserState::WaitHeader => "WAIT_HEADER",
        ParserState::WaitFieldId => "WAIT_FIELD_ID",
        ParserState::WaitFieldLen => "WAIT_FIELD_LEN",
        ParserState::WaitFieldData => "WAIT_FIELD_DATA",
        ParserState::WaitSecHdr => "WAIT_SEC_HDR",
        ParserState::WaitTag => "WAIT_TAG",
        ParserState::WaitCrc16Lo => "WAIT_CRC16_LO",
        ParserState::WaitCrc16Hi => "WAIT_CRC16_HI",
    }
}

/// Nome de erro para diagnóstico.
pub fn parser_error_to_string(error: ParserError) -> &'static str {
    match error {
        ParserError::Ok => "OK",
        ParserError::ErrStart => "ERR_START",
        ParserError::ErrVersion => "ERR_VERSION",
        ParserError::ErrMsgId => "ERR_MSGID",
        ParserError::ErrFieldCount => "ERR_FIELD_COUNT",
        ParserError::ErrFieldId => "ERR_FIELD_ID",
        ParserError::ErrFieldLen => "ERR_FIELD_LEN",
        ParserError::ErrChecksum => "ERR_CHECKSUM",
        ParserError::ErrTimeout => "ERR_TIMEOUT",
        ParserError::ErrOverflow => "ERR_OVERFLOW",
        ParserError::ErrHops => "ERR_HOPS",
    }
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::builder::BythosBuilder;
    use crate::protocol::secure::SealConfig;

    /// Trama selada de teste (origem 6, chave fixa), sem `std` nem heap.
    fn sealed(msg_id: u8, fields: &[(u8, &[u8])], seq: u16) -> ([u8; MAX_MESSAGE_SIZE], usize) {
        let mut b = BythosBuilder::new(6, SealConfig::software(0, [0x42u8; 32], 6));
        b.set_seq(seq);
        for (id, data) in fields {
            b.add_raw(*id, data).unwrap();
        }
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(msg_id, BROADCAST_ADDR, &mut buf).unwrap();
        (buf, n)
    }

    /// Alimenta tudo e exige `Ok` em cada byte.
    fn feed_all(p: &mut Parser, bytes: &[u8]) {
        for &byte in bytes {
            assert_eq!(p.feed(byte), ParserError::Ok);
        }
    }

    #[test]
    fn test_novo_e_ancora() {
        let mut p = Parser::new();
        assert_eq!(p.get_current_state(), ParserState::WaitStart);
        assert!(!p.has_message());
        assert_eq!(p.feed(0x00), ParserError::ErrStart);
        assert_eq!(p.feed(START_BYTE), ParserError::Ok);
        assert_eq!(p.get_current_state(), ParserState::WaitHeader);
    }

    #[test]
    fn test_trama_completa() {
        let (wire, n) = sealed(0x11, &[(0xC0, &[0x02])], 1);
        let mut p = Parser::new();
        feed_all(&mut p, &wire[..n]);
        assert!(p.has_message());
        let msg = p.get_message();
        assert_eq!(msg.src, 6);
        assert_eq!(msg.dst, BROADCAST_ADDR);
        assert_eq!(msg.msg_id, 0x11);
        assert_eq!(msg.seq_num, 1);
        assert_eq!(msg.field_count, 1);
        assert_eq!(msg.fields[0].id, 0xC0);
        assert_eq!(msg.hops, HOP_DEFAULT);
        // A trama completa está disponível para validação ponta-a-ponta.
        assert_eq!(p.completed_frame(), Some(&wire[..n]));
        p.acknowledge();
        assert!(!p.has_message());
    }

    #[test]
    fn test_multiplos_campos_e_vazia() {
        let (wire, n) = sealed(0x16, &[(0xA0, &42u16.to_le_bytes()), (0xC3, &[0x03])], 5);
        let mut p = Parser::new();
        feed_all(&mut p, &wire[..n]);
        assert!(p.has_message());
        assert_eq!(p.get_message().field_count, 2);
        p.acknowledge();

        let (empty, m) = sealed(0x10, &[], 0);
        feed_all(&mut p, &empty[..m]);
        assert!(p.has_message());
        assert_eq!(p.get_message().field_count, 0);
        p.acknowledge();
        assert_eq!(p.get_success_count(), 2);
    }

    #[test]
    fn test_crc_errado() {
        let (mut wire, n) = sealed(0x11, &[(0xC0, &[0x02])], 1);
        wire[n - 1] = wire[n - 1].wrapping_add(1);
        let mut p = Parser::new();
        let mut saw_err = false;
        for &byte in &wire[..n] {
            if p.feed(byte) == ParserError::ErrChecksum {
                saw_err = true;
            }
        }
        assert!(saw_err);
        assert!(!p.has_message());
        assert!(p.get_error_count() > 0);
    }

    #[test]
    fn test_versao_errada() {
        let (mut wire, n) = sealed(0x11, &[(0xC0, &[0x02])], 1);
        wire[1] = 0x03; // V3 sem feature: inválida.
        let mut p = Parser::new();
        let mut saw_err = false;
        for &byte in &wire[..n] {
            if p.feed(byte) == ParserError::ErrVersion {
                saw_err = true;
                break;
            }
        }
        assert!(saw_err);
        assert!(!p.has_message());
    }

    #[test]
    fn test_len_mentiroso_recusa_sem_panico() {
        // Conta 1, id u8, LEN=33: ErrFieldLen imediato, máquina de volta ao início.
        let mut p = Parser::new();
        assert_eq!(p.feed(START_BYTE), ParserError::Ok);
        // Cabeçalho de 11 B: ver 04, src 6, dst ffff, msg 11, seq 0, count 1, hops 32.
        for b in [0x04u8, 0x06, 0x00, 0xFF, 0xFF, 0x11, 0x00, 0x00, 1, 32] {
            assert_eq!(p.feed(b), ParserError::Ok);
        }
        assert_eq!(p.feed(0xC0), ParserError::Ok); // id u8
        assert_eq!(p.feed(33), ParserError::ErrFieldLen); // LEN impossível
        assert_eq!(p.get_current_state(), ParserState::WaitStart);
        assert!(!p.has_message());
    }

    #[test]
    fn test_enxurrada_de_lixo_sem_panico() {
        // 3000 bytes de ruído determinístico (com âncoras 0xAA pelo meio): a
        // máquina nunca entra em pânico, conta erros e não inventa mensagens.
        // (Com comprimentos válidos o tampão de 1205 B nunca transborda por
        // construção — o máximo válido cabe exato; aqui prova-se a robustez.)
        let mut p = Parser::new();
        let mut x: u32 = 0x1234_5678;
        for _ in 0..3000 {
            // Gerador congruencial simples: determinístico, sem dependências.
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            let b = (x >> 16) as u8;
            let _ = p.feed(b);
        }
        assert!(!p.has_message());
        assert!(p.get_error_count() > 0);
    }

    #[test]
    fn test_mensagens_consecutivas() {
        let (m1, n1) = sealed(0x11, &[(0xC0, &[0x01])], 1);
        let (m2, n2) = sealed(0x16, &[(0xA0, &42u16.to_le_bytes())], 2);
        let mut p = Parser::new();
        feed_all(&mut p, &m1[..n1]);
        assert_eq!(p.get_message().msg_id, 0x11);
        p.acknowledge();
        feed_all(&mut p, &m2[..n2]);
        assert_eq!(p.get_message().msg_id, 0x16);
        assert_eq!(p.get_success_count(), 2);
        p.acknowledge();
    }

    #[test]
    fn test_copia_e_sem_mensagem() {
        let (wire, n) = sealed(0x11, &[(0xC0, &[0x02])], 1);
        let mut p = Parser::new();
        let mut out = BythosMessage::new();
        assert!(!p.copy_message(&mut out));
        assert!(p.completed_frame().is_none());
        feed_all(&mut p, &wire[..n]);
        assert!(p.copy_message(&mut out));
        assert_eq!(out.msg_id, 0x11);
        assert_eq!(out.src, 6);
    }

    #[test]
    fn test_nomes() {
        assert_eq!(parser_state_to_string(ParserState::WaitStart), "WAIT_START");
        assert_eq!(
            parser_state_to_string(ParserState::WaitCrc16Hi),
            "WAIT_CRC16_HI"
        );
        assert_eq!(parser_error_to_string(ParserError::Ok), "OK");
        assert_eq!(parser_error_to_string(ParserError::ErrHops), "ERR_HOPS");
        assert_eq!(
            parser_error_to_string(ParserError::ErrOverflow),
            "ERR_OVERFLOW"
        );
    }

    #[test]
    fn test_reset() {
        let mut p = Parser::new();
        p.feed(START_BYTE);
        p.feed(BYTHOS_VERSION);
        assert_ne!(p.get_current_state(), ParserState::WaitStart);
        p.reset();
        assert_eq!(p.get_current_state(), ParserState::WaitStart);
    }
}
