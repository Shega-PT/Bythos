// SPDX-License-Identifier: GPL-3.0-or-later

//! # Parser FSM — Máquina de Estados Finita para Parsing de Mensagens Bythos v3.0.0
//!
//! Este módulo implementa um parser byte-a-byte para reconstruir mensagens
//! Bythos a partir de um fluxo serial. A FSM garante validação em tempo real
//! de cada byte recebido, com proteções contra timeout, overflow e corrupção.

use crate::protocol::types::*;
use crate::protocol::crc16::calc_crc16;
use crate::protocol::codec::parse_tlv;

/// Tipo da função de timestamp em microssegundos.
///
/// Utiliza ABI `extern "C"` para compatibilidade FFI com código C/C++.
/// Permite injeção de diferentes fontes de tempo:
/// - Em `std`: `SystemTime` (tempo real do sistema)
/// - Em `no_std`/ESP32: hardware timer do microcontrolador
/// - Em testes: função controlada para testes determinísticos
///
/// A função deve retornar o tempo atual em microssegundos desde um ponto
/// de referência arbitrário. Apenas a diferença entre chamadas consecutivas
/// é relevante para deteção de timeout.
pub type TimestampFn = extern "C" fn() -> u64;

/// Função de timestamp padrão — retorna o tempo real do sistema em `std`,
/// ou 0 em `no_std` (timeout desativado).
///
/// Em `no_std`, o utilizador deve fornecer a sua própria função via
/// `Parser::with_timestamp_fn()` ou `Parser::set_timestamp_fn()`.
#[cfg(feature = "std")]
extern "C" fn default_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_micros() as u64,
        Err(_) => 0,
    }
}

/// Função de timestamp padrão para `no_std` — retorna 0 sempre.
///
/// O timeout é desativado por defeito em ambientes sem biblioteca padrão.
/// O utilizador deve injetar a sua própria fonte de tempo via
/// `Parser::with_timestamp_fn()` ou `Parser::set_timestamp_fn()`.
#[cfg(not(feature = "std"))]
extern "C" fn default_timestamp() -> u64 {
    0
}

/// Estados da FSM do parser.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserState {
    /// Aguardando byte de início (START_BYTE = 0xAA).
    WaitStart = 0,
    /// Aguardando cabeçalho Bythos (version, nodeId, msgId, seqLo, seqHi).
    /// 5 bytes após START_BYTE.
    WaitHeader = 1,
    /// Aguardando número de campos TLV.
    WaitTlvCount = 2,
    /// Aguardando ID de um campo TLV.
    WaitTlvId = 3,
    /// Aguardando tamanho de um campo TLV.
    WaitTlvLen = 4,
    /// Aguardando dados de um campo TLV.
    WaitTlvData = 5,
    /// Aguardando byte de assinatura.
    WaitSignature = 6,
    /// Aguardando byte baixo do CRC16.
    WaitCrc16Lo = 7,
    /// Aguardando byte alto do CRC16.
    WaitCrc16Hi = 8,
}

/// Códigos de erro do parser.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserError {
    /// Operação bem-sucedida ou mensagem processada.
    Ok = 0,
    /// Byte de início inválido.
    ErrStart = 1,
    /// Versão do protocolo incompatível.
    ErrVersion = 2,
    /// ID da mensagem inválido.
    ErrMsgId = 3,
    /// Número de campos TLV inválido.
    ErrTlvCount = 4,
    /// ID de campo TLV inválido.
    ErrTlvId = 5,
    /// Tamanho de campo TLV inválido.
    ErrTlvLen = 6,
    /// Checksum CRC16 inválido.
    ErrChecksum = 7,
    /// Assinatura inválida.
    ErrSignature = 8,
    /// Timeout entre bytes (frame gap excedido).
    ErrTimeout = 9,
}

/// Parser FSM para mensagens Bythos v3.0.0.
///
/// Reconstrói mensagens Bythos completas a partir de um fluxo de bytes
/// recebidos serialmente. Utiliza uma FSM de 9 estados com proteções
/// contra timeout, overflow e erros de integridade.
///
/// O timeout entre bytes é verificado automaticamente em cada chamada
/// a `feed()`. Se o intervalo entre dois bytes consecutivos exceder
/// `max_frame_gap_us`, o parser é resetado automaticamente e o erro
/// `ErrTimeout` é retornado.
pub struct Parser {
    /// Estado atual da FSM.
    state: ParserState,
    /// Buffer para acumular bytes da mensagem em construção.
    raw_buffer: [u8; MAX_MESSAGE_SIZE],
    /// Número de bytes acumulados no buffer.
    raw_offset: usize,
    /// Número de bytes de dados que faltam para o campo TLV atual.
    tlv_data_remaining: usize,
    /// Mensagem Bythos reconstruída (resultado do parsing).
    msg: TLVMessage,
    /// Máximo intervalo entre bytes em microssegundos (timeout).
    max_frame_gap_us: u32,
    /// Timestamp do último byte recebido em microssegundos.
    last_byte_time_us: u64,
    /// Indica se uma mensagem completa está disponível.
    has_message: bool,
    /// Contador de mensagens processadas com sucesso.
    success_count: u32,
    /// Contador de erros de parsing.
    error_count: u32,
    /// Último erro registado pelo parser.
    last_error: ParserError,
    /// Chave de assinatura (XOR key) para validação.
    signature_key: u8,
    /// Função de timestamp para medir intervalos entre bytes.
    /// Permite injeção de diferentes fontes de tempo (sistema, hardware, teste).
    timestamp_fn: TimestampFn,
}

impl Parser {
    /// Cria um novo parser FSM com a fonte de timestamp padrão.
    ///
    /// Em `std`, utiliza `SystemTime` para obter o timestamp real.
    /// Em `no_std`, o timeout é desativado (retorna 0 sempre).
    /// Para personalizar a fonte de tempo, usar `with_timestamp_fn()`.
    ///
    /// # Arguments
    ///
    /// * `key` — Chave de assinatura (XOR key) para validação de mensagens.
    pub fn new(key: u8) -> Self {
        Self::with_timestamp_fn(key, default_timestamp)
    }

    /// Cria um novo parser FSM com uma função de timestamp personalizada.
    ///
    /// Permite ao utilizador fornecer a sua própria fonte de tempo,
    /// útil para testes determinísticos ou ambientes embedded com
    /// hardware timers específicos.
    ///
    /// # Arguments
    ///
    /// * `key` — Chave de assinatura (XOR key) para validação de mensagens.
    /// * `ts_fn` — Função `extern "C"` que retorna o timestamp em microssegundos.
    pub fn with_timestamp_fn(key: u8, ts_fn: TimestampFn) -> Self {
        Self {
            state: ParserState::WaitStart,
            raw_buffer: [0u8; MAX_MESSAGE_SIZE],
            raw_offset: 0,
            tlv_data_remaining: 0,
            msg: TLVMessage::new(),
            max_frame_gap_us: 5_000_000, // 5 segundos por defeito
            last_byte_time_us: 0,
            has_message: false,
            success_count: 0,
            error_count: 0,
            last_error: ParserError::Ok,
            signature_key: key,
            timestamp_fn: ts_fn,
        }
    }

    /// Alimenta um byte ao parser.
    ///
    /// Este é o método principal do parser. Cada byte recebido do
    /// fluxo serial deve ser passado através deste método. A FSM
    /// transita entre estados conforme os bytes são processados.
    ///
    /// # Arguments
    ///
    /// * `byte` — Byte a processar.
    ///
    /// # Returns
    ///
    /// `ParserError::Ok` se o byte foi processado com sucesso,
    /// ou um código de erro específico.
    pub fn feed(&mut self, byte: u8) -> ParserError {
        // Verificar timeout entre bytes — se exceder o gap máximo,
        // resetar o parser e reportar erro. Apenas verificamos se já
        // estamos a processar uma mensagem (não em WaitStart).
        if self.state != ParserState::WaitStart {
            let now = (self.timestamp_fn)();
            let elapsed = now.wrapping_sub(self.last_byte_time_us);
            if elapsed > self.max_frame_gap_us as u64 {
                self.last_error = ParserError::ErrTimeout;
                self.error_count += 1;
                self.state = ParserState::WaitStart;
                self.raw_offset = 0;
                self.tlv_data_remaining = 0;
                return ParserError::ErrTimeout;
            }
        }

        // Atualizar timestamp do último byte
        self.last_byte_time_us = (self.timestamp_fn)();

        match self.state {
            // ====================================================================
            // ESTADO 0: Aguardar byte de início
            // ====================================================================
            ParserState::WaitStart => {
                if byte == START_BYTE {
                    self.raw_offset = 0;
                    self.raw_buffer[0] = byte;
                    self.raw_offset = 1;
                    self.state = ParserState::WaitHeader;
                    ParserError::Ok
                } else {
                    self.last_error = ParserError::ErrStart;
                    self.error_count += 1;
                    ParserError::ErrStart
                }
            }

            // ====================================================================
            // ESTADO 1: Aguardar cabeçalho (6 bytes: ver, node, msg, seqLo, seqHi)
            // ====================================================================
            ParserState::WaitHeader => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;

                // Cabeçalho sem TLV_COUNT: START + ver + node + msg + seqLo + seqHi = 6 bytes
                if self.raw_offset >= (BYTHOS_HEADER_SIZE - 1) {
                    // Validar versão
                    if self.raw_buffer[1] != BYTHOS_VERSION {
                        self.last_error = ParserError::ErrVersion;
                        self.error_count += 1;
                        self.state = ParserState::WaitStart;
                        return ParserError::ErrVersion;
                    }

                    // Validar msg_id
                    let msg_id = self.raw_buffer[3];
                    if !MsgId::is_valid(msg_id) {
                        self.last_error = ParserError::ErrMsgId;
                        self.error_count += 1;
                        self.state = ParserState::WaitStart;
                        return ParserError::ErrMsgId;
                    }

                    self.state = ParserState::WaitTlvCount;
                }

                ParserError::Ok
            }

            // ====================================================================
            // ESTADO 2: Aguardar número de campos TLV
            // ====================================================================
            ParserState::WaitTlvCount => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;

                if (byte as usize) <= MAX_TLV_FIELDS {
                    if byte == 0 {
                        // Sem campos TLV — ir direto para assinatura
                        self.state = ParserState::WaitSignature;
                    } else {
                        self.state = ParserState::WaitTlvId;
                    }
                    ParserError::Ok
                } else {
                    self.last_error = ParserError::ErrTlvCount;
                    self.error_count += 1;
                    self.state = ParserState::WaitStart;
                    ParserError::ErrTlvCount
                }
            }

            // ====================================================================
            // ESTADO 3: Aguardar ID de campo TLV
            // ====================================================================
            ParserState::WaitTlvId => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;

                // Validar tipo do FieldID
                let (field_type, _) = field_id_decode(byte);
                if FieldType::from_u8(field_type).is_some() {
                    self.state = ParserState::WaitTlvLen;
                    ParserError::Ok
                } else {
                    self.last_error = ParserError::ErrTlvId;
                    self.error_count += 1;
                    self.state = ParserState::WaitStart;
                    ParserError::ErrTlvId
                }
            }

            // ====================================================================
            // ESTADO 4: Aguardar tamanho de campo TLV
            // ====================================================================
            ParserState::WaitTlvLen => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;

                if (byte as usize) <= MAX_TLV_DATA {
                    self.tlv_data_remaining = byte as usize;

                    if byte == 0 {
                        // Campo sem dados — verificar se há mais campos
                        self.check_next_tlv_or_signature();
                    } else {
                        self.state = ParserState::WaitTlvData;
                    }
                    ParserError::Ok
                } else {
                    self.last_error = ParserError::ErrTlvLen;
                    self.error_count += 1;
                    self.state = ParserState::WaitStart;
                    ParserError::ErrTlvLen
                }
            }

            // ====================================================================
            // ESTADO 5: Aguardar dados de campo TLV
            // ====================================================================
            ParserState::WaitTlvData => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;
                self.tlv_data_remaining -= 1;

                if self.tlv_data_remaining == 0 {
                    // Todos os bytes deste campo foram recebidos
                    self.check_next_tlv_or_signature();
                }

                ParserError::Ok
            }

            // ====================================================================
            // ESTADO 6: Aguardar byte de assinatura
            // ====================================================================
            ParserState::WaitSignature => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;
                self.state = ParserState::WaitCrc16Lo;
                ParserError::Ok
            }

            // ====================================================================
            // ESTADO 7: Aguardar byte baixo do CRC16
            // ====================================================================
            ParserState::WaitCrc16Lo => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;
                self.state = ParserState::WaitCrc16Hi;
                ParserError::Ok
            }

            // ====================================================================
            // ESTADO 8: Aguardar byte alto do CRC16 — Mensagem completa!
            // ====================================================================
            ParserState::WaitCrc16Hi => {
                self.raw_buffer[self.raw_offset] = byte;
                self.raw_offset += 1;

                // Validar CRC16
                let crc_offset = self.raw_offset - CRC16_SIZE;
                let crc_lo = self.raw_buffer[crc_offset];
                let crc_hi = self.raw_buffer[crc_offset + 1];
                let expected_crc = (crc_hi as u16) << 8 | (crc_lo as u16);
                let computed_crc = calc_crc16(&self.raw_buffer[..crc_offset]);

                if computed_crc != expected_crc {
                    self.last_error = ParserError::ErrChecksum;
                    self.error_count += 1;
                    self.state = ParserState::WaitStart;
                    return ParserError::ErrChecksum;
                }

                // Validar assinatura
                let sig_offset = crc_offset - SIGNATURE_SIZE;
                let signature = self.raw_buffer[sig_offset];
                let msg_id = self.raw_buffer[3];
                let seq_lo = self.raw_buffer[4];
                let seq_hi = self.raw_buffer[5];
                let expected_sig = compute_signature(
                    self.signature_key,
                    msg_id,
                    seq_lo,
                    seq_hi,
                );

                if signature != expected_sig {
                    self.last_error = ParserError::ErrSignature;
                    self.error_count += 1;
                    self.state = ParserState::WaitStart;
                    return ParserError::ErrSignature;
                }

                // Parsing bem-sucedido — extrair campos TLV
                let tlv_data_start = BYTHOS_HEADER_SIZE;
                let tlv_data_end = sig_offset;

                let mut tlv_output = [TLVField::new(); MAX_TLV_FIELDS];
                match parse_tlv(
                    &self.raw_buffer[tlv_data_start..tlv_data_end],
                    &mut tlv_output,
                ) {
                    Ok(parsed_count) => {
                        self.msg.start_byte = self.raw_buffer[0];
                        self.msg.version = self.raw_buffer[1];
                        self.msg.node_id = self.raw_buffer[2];
                        self.msg.msg_id = self.raw_buffer[3];
                        self.msg.seq_num = (seq_hi as u16) << 8 | (seq_lo as u16);
                        self.msg.tlv_count = parsed_count as u8;
                        self.msg.tlvs[..parsed_count].copy_from_slice(&tlv_output[..parsed_count]);
                        self.msg.signature = signature;
                        self.msg.checksum = expected_crc;
                        self.has_message = true;
                        self.success_count += 1;
                        self.last_error = ParserError::Ok;
                        self.state = ParserState::WaitStart;

                        ParserError::Ok
                    }
                    Err(_) => {
                        self.last_error = ParserError::ErrChecksum;
                        self.error_count += 1;
                        self.state = ParserState::WaitStart;
                        ParserError::ErrChecksum
                    }
                }
            }
        }
    }

    /// Verifica se o próximo byte deve ser ID de um novo campo TLV
    /// ou se devemos avançar para a assinatura.
    fn check_next_tlv_or_signature(&mut self) {
        // Contar campos TLV processados calculando o offset
        let tlv_count = self.raw_buffer[6] as usize;
        let mut temp_offset = BYTHOS_HEADER_SIZE;
        let mut campos_processados = 0;

        while campos_processados < tlv_count && temp_offset + TLV_HEADER_SIZE <= self.raw_offset {
            let field_len = self.raw_buffer[temp_offset + 1] as usize;
            temp_offset += TLV_HEADER_SIZE + field_len;
            campos_processados += 1;
        }

        if campos_processados >= tlv_count {
            self.state = ParserState::WaitSignature;
        } else {
            self.state = ParserState::WaitTlvId;
        }
    }

    /// Retorna true se uma mensagem completa está disponível.
    pub fn has_message(&self) -> bool {
        self.has_message
    }

    /// Retorna uma referência à mensagem reconstruída.
    pub fn get_message(&self) -> &TLVMessage {
        &self.msg
    }

    /// Copia a mensagem reconstruída para um buffer de saída.
    ///
    /// # Arguments
    ///
    /// * `output` — Buffer de saída para a mensagem.
    ///
    /// # Returns
    ///
    /// `true` se a mensagem foi copiada com sucesso, `false` caso contrário.
    pub fn copy_message(&self, output: &mut TLVMessage) -> bool {
        if !self.has_message {
            return false;
        }
        *output = self.msg.clone();
        true
    }

    /// Reconhece a mensagem processada, permitindo que o parser processe a próxima.
    ///
    /// Deve ser chamado após processar a mensagem obtida via `get_message()`.
    pub fn acknowledge(&mut self) {
        self.has_message = false;
        self.msg.clear();
    }

    /// Reseta o parser para o estado inicial.
    pub fn reset(&mut self) {
        self.state = ParserState::WaitStart;
        self.raw_offset = 0;
        self.tlv_data_remaining = 0;
        self.has_message = false;
        self.msg.clear();
    }

    /// Define o intervalo máximo entre bytes (timeout) em microssegundos.
    ///
    /// Se dois bytes consecutivos demorarem mais do que este intervalo,
    /// o parser será resetado automaticamente.
    pub fn set_max_frame_gap(&mut self, micros: u32) {
        self.max_frame_gap_us = micros;
    }

    /// Verifica se o parser excedeu o timeout entre bytes.
    ///
    /// Retorna `true` se o intervalo entre o último byte recebido
    /// e o momento atual exceder `max_frame_gap_us`.
    pub fn is_timed_out(&self) -> bool {
        if self.state == ParserState::WaitStart {
            return false;
        }
        let now = (self.timestamp_fn)();
        let elapsed = now.wrapping_sub(self.last_byte_time_us);
        elapsed > self.max_frame_gap_us as u64
    }

    /// Retorna o último erro registado pelo parser.
    pub fn get_last_error(&self) -> ParserError {
        self.last_error
    }

    /// Retorna o estado atual da FSM.
    pub fn get_current_state(&self) -> ParserState {
        self.state
    }

    /// Retorna o número de mensagens processadas com sucesso.
    pub fn get_success_count(&self) -> u32 {
        self.success_count
    }

    /// Retorna o número de erros de parsing.
    pub fn get_error_count(&self) -> u32 {
        self.error_count
    }

    /// Retorna a chave de assinatura configurada.
    pub fn get_key(&self) -> u8 {
        self.signature_key
    }

    /// Define a chave de assinatura.
    pub fn set_key(&mut self, key: u8) {
        self.signature_key = key;
    }

    /// Define a função de timestamp utilizada pelo parser.
    ///
    /// Permite alterar a fonte de tempo após a criação do parser.
    /// Útil para alternar entre timers ou desativar o timeout.
    ///
    /// # Arguments
    ///
    /// * `ts_fn` — Nova função de timestamp em microssegundos.
    pub fn set_timestamp_fn(&mut self, ts_fn: TimestampFn) {
        self.timestamp_fn = ts_fn;
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Converte um estado do parser para string legível.
pub fn parser_state_to_string(state: ParserState) -> &'static str {
    match state {
        ParserState::WaitStart => "WAIT_START",
        ParserState::WaitHeader => "WAIT_HEADER",
        ParserState::WaitTlvCount => "WAIT_TLV_COUNT",
        ParserState::WaitTlvId => "WAIT_TLV_ID",
        ParserState::WaitTlvLen => "WAIT_TLV_LEN",
        ParserState::WaitTlvData => "WAIT_TLV_DATA",
        ParserState::WaitSignature => "WAIT_SIGNATURE",
        ParserState::WaitCrc16Lo => "WAIT_CRC16_LO",
        ParserState::WaitCrc16Hi => "WAIT_CRC16_HI",
    }
}

/// Converte um código de erro do parser para string legível.
pub fn parser_error_to_string(error: ParserError) -> &'static str {
    match error {
        ParserError::Ok => "OK",
        ParserError::ErrStart => "ERR_START",
        ParserError::ErrVersion => "ERR_VERSION",
        ParserError::ErrMsgId => "ERR_MSGID",
        ParserError::ErrTlvCount => "ERR_TLV_COUNT",
        ParserError::ErrTlvId => "ERR_TLV_ID",
        ParserError::ErrTlvLen => "ERR_TLV_LEN",
        ParserError::ErrChecksum => "ERR_CHECKSUM",
        ParserError::ErrSignature => "ERR_SIGNATURE",
        ParserError::ErrTimeout => "ERR_TIMEOUT",
    }
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::builder::TLVBuilder;
    use std::vec::Vec;

    /// Helper: constrói uma mensagem serializada para testes.
    fn build_test_message(msg_id: u8, key: u8, tlv_data: &[(u8, &[u8])], seq: u16) -> Vec<u8> {
        let mut builder = TLVBuilder::new(0x06, key);
        builder.set_seq(seq);
        for (id, data) in tlv_data {
            builder.add_raw(*id, data).unwrap();
        }
        let mut buffer = [0u8; MAX_MESSAGE_SIZE];
        let size = builder.build(msg_id, &mut buffer).unwrap();
        buffer[..size].to_vec()
    }

    #[test]
    fn test_parser_new() {
        let parser = Parser::new(0x42);
        assert_eq!(parser.get_current_state(), ParserState::WaitStart);
        assert!(!parser.has_message());
        assert_eq!(parser.get_success_count(), 0);
        assert_eq!(parser.get_error_count(), 0);
        assert_eq!(parser.get_key(), 0x42);
    }

    #[test]
    fn test_parser_feed_start_byte() {
        let mut parser = Parser::new(0x42);
        let result = parser.feed(START_BYTE);
        assert_eq!(result, ParserError::Ok);
        assert_eq!(parser.get_current_state(), ParserState::WaitHeader);
    }

    #[test]
    fn test_parser_feed_invalid_start() {
        let mut parser = Parser::new(0x42);
        let result = parser.feed(0x00);
        assert_eq!(result, ParserError::ErrStart);
        assert_eq!(parser.get_current_state(), ParserState::WaitStart);
    }

    #[test]
    fn test_parser_full_message() {
        let serialized = build_test_message(0x11, 0x42, &[(0xC0, &[0x02])], 1);
        let mut parser = Parser::new(0x42);

        for &byte in &serialized {
            let result = parser.feed(byte);
            assert_eq!(result, ParserError::Ok);
        }

        assert!(parser.has_message());
        let msg = parser.get_message();
        assert_eq!(msg.start_byte, START_BYTE);
        assert_eq!(msg.version, BYTHOS_VERSION);
        assert_eq!(msg.node_id, 0x06);
        assert_eq!(msg.msg_id, 0x11);
        assert_eq!(msg.seq_num, 1);
        assert_eq!(msg.tlv_count, 1);
        assert_eq!(msg.tlvs[0].id, 0xC0);
        assert_eq!(msg.tlvs[0].len, 1);
        assert_eq!(msg.tlvs[0].data[0], 0x02);

        parser.acknowledge();
        assert!(!parser.has_message());
    }

    #[test]
    fn test_parser_multiple_tlvs() {
        let serialized = build_test_message(0x16, 0x42, &[
            (0xA0, &42u16.to_le_bytes()),
            (0xC3, &[0x03]),
            (0xC4, &[0x10]),
        ], 5);
        let mut parser = Parser::new(0x42);

        for &byte in &serialized {
            parser.feed(byte);
        }

        assert!(parser.has_message());
        let msg = parser.get_message();
        assert_eq!(msg.tlv_count, 3);
        assert_eq!(msg.tlvs[0].id, 0xA0);
        assert_eq!(msg.tlvs[1].id, 0xC3);
        assert_eq!(msg.tlvs[2].id, 0xC4);
        assert_eq!(msg.seq_num, 5);

        parser.acknowledge();
    }

    #[test]
    fn test_parser_invalid_crc() {
        let mut serialized = build_test_message(0x11, 0x42, &[(0xC0, &[0x02])], 1);
        // Corromper CRC
        let last = serialized.len() - 1;
        serialized[last] = serialized[last].wrapping_add(1);

        let mut parser = Parser::new(0x42);
        for &byte in &serialized {
            parser.feed(byte);
        }

        assert!(!parser.has_message());
        assert!(parser.get_error_count() > 0);
    }

    #[test]
    fn test_parser_invalid_signature() {
        let mut serialized = build_test_message(0x11, 0x42, &[(0xC0, &[0x02])], 1);
        // Corromper assinatura (byte antes do CRC16)
        let sig_idx = serialized.len() - 3;
        serialized[sig_idx] = serialized[sig_idx].wrapping_add(1);

        let mut parser = Parser::new(0x42);
        for &byte in &serialized {
            parser.feed(byte);
        }

        assert!(!parser.has_message());
        assert!(parser.get_error_count() > 0);
    }

    #[test]
    fn test_parser_wrong_key() {
        let serialized = build_test_message(0x11, 0x42, &[(0xC0, &[0x02])], 1);
        let mut parser = Parser::new(0x43); // Key errada

        for &byte in &serialized {
            parser.feed(byte);
        }

        assert!(!parser.has_message());
        assert!(parser.get_error_count() > 0);
    }

    #[test]
    fn test_parser_invalid_version() {
        let mut serialized = build_test_message(0x11, 0x42, &[(0xC0, &[0x02])], 1);
        serialized[1] = 0x99; // Versão inválida

        let mut parser = Parser::new(0x42);
        for &byte in &serialized {
            parser.feed(byte);
        }

        assert!(!parser.has_message());
        assert!(parser.get_error_count() > 0);
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = Parser::new(0x42);
        parser.feed(START_BYTE);
        parser.feed(BYTHOS_VERSION);
        assert_ne!(parser.get_current_state(), ParserState::WaitStart);

        parser.reset();
        assert_eq!(parser.get_current_state(), ParserState::WaitStart);
        assert_eq!(parser.raw_offset, 0);
    }

    #[test]
    fn test_parser_copy_message() {
        let serialized = build_test_message(0x11, 0x42, &[(0xC0, &[0x02])], 1);
        let mut parser = Parser::new(0x42);

        for &byte in &serialized {
            parser.feed(byte);
        }

        let mut output = TLVMessage::new();
        assert!(parser.copy_message(&mut output));
        assert_eq!(output.msg_id, 0x11);
        assert_eq!(output.node_id, 0x06);
    }

    #[test]
    fn test_parser_copy_message_no_message() {
        let parser = Parser::new(0x42);
        let mut output = TLVMessage::new();
        assert!(!parser.copy_message(&mut output));
    }

    #[test]
    fn test_parser_state_to_string() {
        assert_eq!(parser_state_to_string(ParserState::WaitStart), "WAIT_START");
        assert_eq!(parser_state_to_string(ParserState::WaitCrc16Hi), "WAIT_CRC16_HI");
    }

    #[test]
    fn test_parser_error_to_string() {
        assert_eq!(parser_error_to_string(ParserError::Ok), "OK");
        assert_eq!(parser_error_to_string(ParserError::ErrChecksum), "ERR_CHECKSUM");
        assert_eq!(parser_error_to_string(ParserError::ErrSignature), "ERR_SIGNATURE");
    }

    #[test]
    fn test_parser_consecutive_messages() {
        let msg1 = build_test_message(0x11, 0x42, &[(0xC0, &[0x01])], 1);
        let msg2 = build_test_message(0x16, 0x42, &[(0xA0, &42u16.to_le_bytes())], 2);
        let mut parser = Parser::new(0x42);

        // Primeira mensagem
        for &byte in &msg1 {
            parser.feed(byte);
        }
        assert!(parser.has_message());
        assert_eq!(parser.get_message().msg_id, 0x11);
        parser.acknowledge();

        // Segunda mensagem
        for &byte in &msg2 {
            parser.feed(byte);
        }
        assert!(parser.has_message());
        assert_eq!(parser.get_message().msg_id, 0x16);
        assert_eq!(parser.get_success_count(), 2);

        parser.acknowledge();
    }

    #[test]
    fn test_parser_empty_tlv_message() {
        let serialized = build_test_message(0x10, 0x42, &[], 0); // Heartbeat, sem TLVs
        let mut parser = Parser::new(0x42);

        for &byte in &serialized {
            parser.feed(byte);
        }

        assert!(parser.has_message());
        let msg = parser.get_message();
        assert_eq!(msg.msg_id, 0x10);
        assert_eq!(msg.tlv_count, 0);

        parser.acknowledge();
    }

    #[test]
    fn test_parser_field_id_with_type() {
        // FieldID com tipo embutido: TYPE=1(f32) + ID=0x10 → 0x30
        let serialized = build_test_message(0x11, 0x42, &[(0x30, &float_to_bytes(1.5))], 3);
        let mut parser = Parser::new(0x42);

        for &byte in &serialized {
            parser.feed(byte);
        }

        assert!(parser.has_message());
        let msg = parser.get_message();
        assert_eq!(msg.tlv_count, 1);

        let (field_type, field_id) = field_id_decode(msg.tlvs[0].id);
        assert_eq!(field_type, FieldType::Float32 as u8);
        assert_eq!(field_id, 0x10);

        parser.acknowledge();
    }

    // ========================================================================
    // TESTES — TIMEOUT
    // ========================================================================

    /// Contador global para testes de timestamp controlado.
    /// Cada chamada avança o relógio em 1000 microssegundos.
    static mut TEST_COUNTER: u64 = 0;

    /// Função de timestamp mock para testes — avança 1ms por chamada.
    extern "C" fn mock_timestamp() -> u64 {
        unsafe {
            TEST_COUNTER += 1000;
            TEST_COUNTER
        }
    }

    /// Reset do contador de teste entre testes.
    fn reset_test_counter() {
        unsafe { TEST_COUNTER = 0; }
    }

    #[test]
    fn test_parser_timeout_detection() {
        reset_test_counter();
        let mut parser = Parser::with_timestamp_fn(0x42, mock_timestamp);
        parser.set_max_frame_gap(5000); // 5ms timeout

        // Alimentar START_BYTE — estado passa para WaitHeader
        let result = parser.feed(START_BYTE);
        assert_eq!(result, ParserError::Ok);
        assert_eq!(parser.get_current_state(), ParserState::WaitHeader);

        // Alimentar bytes suficientes para avançar para WaitHeader completo
        // O parser aguarda 5 bytes após START: ver, node, msg, seqLo, seqHi
        // Cada feed() consome 1ms do mock, mas o timeout é 5ms
        let result = parser.feed(BYTHOS_VERSION);
        assert_eq!(result, ParserError::Ok);

        // O timestamp_mock avança 1ms por feed, mas o timeout é 5ms
        // Portanto não deve haver timeout entre bytes normais
        assert_ne!(parser.get_current_state(), ParserState::WaitStart);
    }

    #[test]
    fn test_parser_timeout_triggers_reset() {
        reset_test_counter();
        let mut parser = Parser::with_timestamp_fn(0x42, mock_timestamp);
        parser.set_max_frame_gap(2); // 2μs timeout — muito curto

        // Feed START_BYTE — primeiro byte não verifica timeout (WaitStart)
        let result = parser.feed(START_BYTE);
        assert_eq!(result, ParserError::Ok);
        assert_eq!(parser.get_current_state(), ParserState::WaitHeader);

        // Próximo feed chama mock_timestamp que avança 1000μs > 2μs
        // Deve detectar timeout e resetar
        let result = parser.feed(BYTHOS_VERSION);
        assert_eq!(result, ParserError::ErrTimeout);
        assert_eq!(parser.get_current_state(), ParserState::WaitStart);
        assert!(parser.get_error_count() > 0);
    }

    #[test]
    fn test_parser_timeout_no_check_in_wait_start() {
        reset_test_counter();
        let mut parser = Parser::with_timestamp_fn(0x42, mock_timestamp);
        parser.set_max_frame_gap(0); // Timeout zero

        // Em WaitStart, o timeout NÃO é verificado
        let result = parser.feed(0x00); // byte inválido
        assert_eq!(result, ParserError::ErrStart);
        // Não deve retornar ErrTimeout mesmo com timeout=0
    }
}
