// SPDX-License-Identifier: GPL-3.0-or-later

//! # TLVBuilder — Construtor de Mensagens Bythos v3.0.0
//!
//! Implementação do construtor fluent para mensagens Bythos v3.0.0.
//! O TLVBuilder permite construir mensagens de forma segura e eficiente,
//! adicionando campos um a um e serializando a mensagem completa no final.
//!
//! ## Formato da Mensagem
//!
//! ```text
//! [START_BYTE][VERSION][NODE_ID][MSG_ID][SEQ_NUM_LO][SEQ_NUM_HI]
//! [TLV_COUNT][TLV_FIELDS...][SIGNATURE][CRC16_LO][CRC16_HI]
//! ```
//!
//! ## Uso
//!
//! ```rust
//! use bythos::protocol::builder::TLVBuilder;
//!
//! let mut builder = TLVBuilder::new(0x06, 0x42);
//! builder.set_seq(1);
//! builder.add_u8_field(0, 2).unwrap();
//! builder.add_f32_field(0x10, 1.5).unwrap();
//!
//! let mut buffer = [0u8; 1098];
//! let size = builder.build(0x11, &mut buffer).unwrap();
//! ```

use crate::protocol::types::*;
use crate::protocol::crc16::calc_crc16;
use crate::protocol::codec::ProtocolError;

/// Construtor fluente para mensagens Bythos v3.0.0.
///
/// Permite construir mensagens Bythos de forma iterativa, adicionando campos
/// com diferentes tipos de dados. A mensagem é serializada e validada
/// quando `build()` é chamado.
///
/// O builder mantém internamente um array de campos TLV e um contador.
/// Quando `build()` é chamado, serializa todos os campos no buffer de
/// saída, calcula a assinatura e o CRC16.
pub struct TLVBuilder {
    /// Array interno de campos TLV.
    fields: [TLVField; MAX_TLV_FIELDS],
    /// Número de campos adicionados.
    tlv_count: u8,
    /// ID do nó transmissor (grupo CAN).
    node_id: u8,
    /// Número de sequência da mensagem.
    seq_num: u16,
    /// Chave de assinatura (XOR key).
    signature_key: u8,
}

impl TLVBuilder {
    /// Cria um novo TLVBuilder para o nó especificado.
    ///
    /// # Arguments
    ///
    /// * `node_id` — ID do grupo CAN deste nó.
    /// * `key` — Chave de assinatura partilhada (0x00 = sem assinatura).
    ///
    /// # Returns
    ///
    /// Um novo TLVBuilder pronto para uso.
    pub fn new(node_id: u8, key: u8) -> Self {
        Self {
            fields: [TLVField::new(); MAX_TLV_FIELDS],
            tlv_count: 0,
            node_id,
            seq_num: 0,
            signature_key: key,
        }
    }

    /// Reseta o builder para o estado inicial, mantendo node_id e key.
    pub fn reset(&mut self) {
        self.tlv_count = 0;
        self.seq_num = 0;
        for field in self.fields.iter_mut() {
            *field = TLVField::new();
        }
    }

    /// Retorna o número de campos TLV adicionados.
    pub fn get_tlv_count(&self) -> u8 {
        self.tlv_count
    }

    /// Define o número de sequência da mensagem.
    pub fn set_seq(&mut self, seq: u16) {
        self.seq_num = seq;
    }

    /// Retorna o número de sequência atual.
    pub fn get_seq(&self) -> u16 {
        self.seq_num
    }

    /// Define a chave de assinatura.
    pub fn set_key(&mut self, key: u8) {
        self.signature_key = key;
    }

    /// Retorna o node_id configurado.
    pub fn get_node_id(&self) -> u8 {
        self.node_id
    }

    // ========================================================================
    // MÉTODOS DE ADICIONAR CAMPOS — RAW
    // ========================================================================

    /// Adiciona um campo TLV com dados brutos (bytes).
    ///
    /// # Arguments
    ///
    /// * `id` — FieldID codificado ([TYPE:3][ID:5]).
    /// * `data` — Dados a incluir no campo.
    ///
    /// # Returns
    ///
    /// `Ok(())` em sucesso, `Err(ProtocolError::TooManyFields)` se o número
    /// máximo de campos for excedido.
    pub fn add_raw(&mut self, id: u8, data: &[u8]) -> Result<(), ProtocolError> {
        if self.tlv_count as usize >= MAX_TLV_FIELDS {
            return Err(ProtocolError::TooManyFields);
        }
        let len = data.len().min(MAX_TLV_DATA);
        let field = TLVField {
            id,
            len: len as u8,
            data: {
                let mut arr = [0u8; MAX_TLV_DATA];
                arr[..len].copy_from_slice(&data[..len]);
                arr
            },
        };
        self.fields[self.tlv_count as usize] = field;
        self.tlv_count += 1;
        Ok(())
    }

    // ========================================================================
    // MÉTODOS DE ADICIONAR CAMPOS — TIPOS ESPECÍFICOS
    // ========================================================================

    /// Adiciona um campo TLV com valor float (32 bits).
    /// O FieldID é codificado automaticamente com tipo=1 (f32).
    pub fn add_f32(&mut self, id: u8, value: f32) -> Result<(), ProtocolError> {
        self.add_raw(id, &float_to_bytes(value))
    }

    /// Adiciona um campo TLV com valor float (32 bits) usando FieldID com tipo.
    /// `field_id` é o ID lógico (0-31), o tipo é codificado automaticamente.
    pub fn add_f32_field(&mut self, field_id: u8, value: f32) -> Result<(), ProtocolError> {
        let encoded = field_id_encode(FieldType::Float32 as u8, field_id);
        self.add_f32(encoded, value)
    }

    /// Adiciona um campo TLV com valor inteiro sinalizado (32 bits).
    pub fn add_i32(&mut self, id: u8, value: i32) -> Result<(), ProtocolError> {
        self.add_raw(id, &int32_to_bytes(value))
    }

    /// Adiciona um campo TLV com valor i32 usando FieldID com tipo.
    pub fn add_i32_field(&mut self, field_id: u8, value: i32) -> Result<(), ProtocolError> {
        let encoded = field_id_encode(FieldType::Int32 as u8, field_id);
        self.add_i32(encoded, value)
    }

    /// Adiciona um campo TLV com valor inteiro sem sinal (32 bits).
    pub fn add_u32(&mut self, id: u8, value: u32) -> Result<(), ProtocolError> {
        self.add_raw(id, &uint32_to_bytes(value))
    }

    /// Adiciona um campo TLV com valor u32 usando FieldID com tipo.
    pub fn add_u32_field(&mut self, field_id: u8, value: u32) -> Result<(), ProtocolError> {
        let encoded = field_id_encode(FieldType::Uint32 as u8, field_id);
        self.add_u32(encoded, value)
    }

    /// Adiciona um campo TLV com valor inteiro sem sinal (16 bits).
    pub fn add_u16(&mut self, id: u8, value: u16) -> Result<(), ProtocolError> {
        self.add_raw(id, &uint16_to_bytes(value))
    }

    /// Adiciona um campo TLV com valor u16 usando FieldID com tipo.
    pub fn add_u16_field(&mut self, field_id: u8, value: u16) -> Result<(), ProtocolError> {
        let encoded = field_id_encode(FieldType::Uint16 as u8, field_id);
        self.add_u16(encoded, value)
    }

    /// Adiciona um campo TLV com valor inteiro sem sinal (8 bits).
    pub fn add_u8(&mut self, id: u8, value: u8) -> Result<(), ProtocolError> {
        self.add_raw(id, &[value])
    }

    /// Adiciona um campo TLV com valor u8 usando FieldID com tipo.
    pub fn add_u8_field(&mut self, field_id: u8, value: u8) -> Result<(), ProtocolError> {
        let encoded = field_id_encode(FieldType::Uint8 as u8, field_id);
        self.add_u8(encoded, value)
    }

    /// Adiciona um campo TLV com valor booleano (1 byte).
    pub fn add_bool(&mut self, id: u8, value: bool) -> Result<(), ProtocolError> {
        self.add_raw(id, &[if value { 1u8 } else { 0u8 }])
    }

    /// Adiciona um campo TLV com valor bool usando FieldID com tipo.
    pub fn add_bool_field(&mut self, field_id: u8, value: bool) -> Result<(), ProtocolError> {
        let encoded = field_id_encode(FieldType::Bool as u8, field_id);
        self.add_bool(encoded, value)
    }

    // ========================================================================
    // SERIALIZAÇÃO
    // ========================================================================

    /// Serializa a mensagem Bythos completa no buffer de saída.
    ///
    /// Constrói a mensagem no formato:
    /// ```text
    /// [START_BYTE][VERSION][NODE_ID][MSG_ID][SEQ_NUM_LO][SEQ_NUM_HI]
    /// [TLV_COUNT][TLV_FIELDS...][SIGNATURE][CRC16_LO][CRC16_HI]
    /// ```
    ///
    /// # Arguments
    ///
    /// * `msg_id` — Identificador do tipo de mensagem.
    /// * `buffer` — Buffer de saída para serialização.
    ///
    /// # Returns
    ///
    /// `Ok(size)` com o número total de bytes serializados em sucesso,
    /// ou `Err(ProtocolError)` em caso de erro.
    pub fn build(&self, msg_id: u8, buffer: &mut [u8]) -> Result<usize, ProtocolError> {
        // Calcular tamanho necessário
        let fields_size: usize = (0..self.tlv_count as usize)
            .map(|i| TLV_HEADER_SIZE + self.fields[i].len as usize)
            .sum();
        let required_size = BYTHOS_HEADER_SIZE + fields_size + SIGNATURE_SIZE + CRC16_SIZE;

        if buffer.len() < required_size {
            return Err(ProtocolError::BufferTooSmall);
        }

        let mut offset = 0;

        // --- Cabeçalho Bythos (7 bytes) ---

        // START_BYTE
        buffer[offset] = START_BYTE;
        offset += 1;

        // VERSION
        buffer[offset] = BYTHOS_VERSION;
        offset += 1;

        // NODE_ID
        buffer[offset] = self.node_id;
        offset += 1;

        // MSG_ID
        buffer[offset] = msg_id;
        offset += 1;

        // SEQ_NUM (2 bytes, little-endian)
        let seq_bytes = self.seq_num.to_le_bytes();
        buffer[offset] = seq_bytes[0];
        offset += 1;
        buffer[offset] = seq_bytes[1];
        offset += 1;

        // TLV_COUNT
        buffer[offset] = self.tlv_count;
        offset += 1;

        // --- Campos TLV ---
        for i in 0..self.tlv_count as usize {
            let field = &self.fields[i];
            buffer[offset] = field.id;
            offset += 1;
            buffer[offset] = field.len;
            offset += 1;
            let len = field.len as usize;
            buffer[offset..offset + len].copy_from_slice(&field.data[..len]);
            offset += len;
        }

        // --- Assinatura ---
        let signature = compute_signature(
            self.signature_key,
            msg_id,
            seq_bytes[0],
            seq_bytes[1],
        );
        buffer[offset] = signature;
        offset += 1;

        // --- CRC16 (sobre todos os bytes exceto o CRC em si) ---
        let crc = calc_crc16(&buffer[..offset]);
        let crc_bytes = crc.to_le_bytes();
        buffer[offset] = crc_bytes[0];
        offset += 1;
        buffer[offset] = crc_bytes[1];
        offset += 1;

        Ok(offset)
    }

    /// Serializa a mensagem Bythos completa com msg_id específico.
    ///
    /// Equivalente a `build(msg_id, buffer)`, mantido por compatibilidade.
    pub fn serialize(&self, msg_id: u8, buffer: &mut [u8]) -> Result<usize, ProtocolError> {
        self.build(msg_id, buffer)
    }
}

impl Default for TLVBuilder {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::crc16::verify_crc16;

    #[test]
    fn test_builder_new() {
        let builder = TLVBuilder::new(0x06, 0x42);
        assert_eq!(builder.get_tlv_count(), 0);
        assert_eq!(builder.get_node_id(), 0x06);
    }

    #[test]
    fn test_builder_add_u8() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8(0xC0, 0x02).unwrap();
        assert_eq!(builder.get_tlv_count(), 1);
    }

    #[test]
    fn test_builder_add_u8_field() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 0x02).unwrap(); // type=6(u8) + id=0 → 0xC0
        assert_eq!(builder.get_tlv_count(), 1);
        assert_eq!(builder.fields[0].id, 0xC0);
    }

    #[test]
    fn test_builder_add_u16() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u16(0xA0, 0x1234).unwrap();
        assert_eq!(builder.get_tlv_count(), 1);
    }

    #[test]
    fn test_builder_add_u32() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u32(0x82, 0xDEADBEEF).unwrap();
        assert_eq!(builder.get_tlv_count(), 1);
    }

    #[test]
    fn test_builder_add_i32() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_i32(0x60, -12345).unwrap();
        assert_eq!(builder.get_tlv_count(), 1);
    }

    #[test]
    fn test_builder_add_f32() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_f32(0x30, 3.14).unwrap();
        assert_eq!(builder.get_tlv_count(), 1);
    }

    #[test]
    fn test_builder_add_f32_field() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_f32_field(0x10, 1.5).unwrap(); // id=0x10 → TYPE=1(f32) + ID=0x10 → 0x30
        assert_eq!(builder.get_tlv_count(), 1);
        assert_eq!(builder.fields[0].id, 0x30);
    }

    #[test]
    fn test_builder_add_raw() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        let data = [1u8, 2, 3, 4, 5];
        builder.add_raw(0x00, &data).unwrap(); // raw type
        assert_eq!(builder.get_tlv_count(), 1);
    }

    #[test]
    fn test_builder_add_bool() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_bool(0xE0, true).unwrap();
        assert_eq!(builder.get_tlv_count(), 1);
        assert_eq!(builder.fields[0].data[0], 1);
    }

    #[test]
    fn test_builder_multiple_fields() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();     // State
        builder.add_f32_field(0x10, 1.5).unwrap(); // Roll
        builder.add_u16_field(0x20, 42).unwrap(); // FrameId
        builder.add_u8_field(3, 0).unwrap();      // ChunkId
        assert_eq!(builder.get_tlv_count(), 4);
    }

    #[test]
    fn test_builder_build() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.add_f32_field(0x10, 1.5).unwrap();
        builder.set_seq(42);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        // Verificar cabeçalho Bythos
        assert_eq!(buffer[0], START_BYTE);       // 0xAA
        assert_eq!(buffer[1], BYTHOS_VERSION);   // 0x03
        assert_eq!(buffer[2], 0x06);             // node_id
        assert_eq!(buffer[3], 0x11);             // msg_id (Telemetry)
        assert_eq!(buffer[4], 42);               // seq_lo
        assert_eq!(buffer[5], 0);                // seq_hi
        assert_eq!(buffer[6], 2);                // tlv_count

        // Verificar que o tamanho é razoável
        assert!(size > BYTHOS_OVERHEAD);
        assert!(size <= MAX_MESSAGE_SIZE);
    }

    #[test]
    fn test_builder_build_verifies_crc() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.set_seq(1);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        // CRC deve ser válido (últimos 2 bytes)
        let crc_offset = size - 2;
        let crc_lo = buffer[crc_offset];
        let crc_hi = buffer[crc_offset + 1];
        let crc = (crc_hi as u16) << 8 | (crc_lo as u16);
        assert!(verify_crc16(&buffer[..crc_offset], crc));
    }

    #[test]
    fn test_builder_build_verifies_signature() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.set_seq(42);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        // Assinatura: byte antes dos CRC16
        let sig_offset = size - 3;
        let signature = buffer[sig_offset];
        let expected = compute_signature(0x42, 0x11, 42, 0);
        assert_eq!(signature, expected);
    }

    #[test]
    fn test_builder_reset() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.add_f32_field(0x10, 1.5).unwrap();
        assert_eq!(builder.get_tlv_count(), 2);

        builder.reset();
        assert_eq!(builder.get_tlv_count(), 0);
        assert_eq!(builder.get_seq(), 0);
        // node_id e key devem ser mantidos
        assert_eq!(builder.get_node_id(), 0x06);
    }

    #[test]
    fn test_builder_overflow() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        for i in 0..MAX_TLV_FIELDS {
            builder.add_u8(i as u8, 0).unwrap();
        }
        assert_eq!(builder.get_tlv_count(), MAX_TLV_FIELDS as u8);

        let result = builder.add_u8(0xFF, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_buffer_too_small() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();

        let mut buffer = [0u8; 5]; // Buffer muito pequeno
        let result = builder.build(0x11, &mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_empty_message() {
        let builder = TLVBuilder::new(0x06, 0x42);
        let mut buffer = [0u8; 1098];
        let size = builder.build(0x10, &mut buffer).unwrap(); // Heartbeat

        // Mensagem vazia: header(7) + signature(1) + crc16(2) = 10
        assert_eq!(size, BYTHOS_OVERHEAD);
        assert_eq!(buffer[6], 0); // tlv_count = 0
    }

    #[test]
    fn test_builder_full_message_types() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.set_seq(100);

        builder.add_f32_field(0x06, 40.0).unwrap();   // Latitude
        builder.add_f32_field(0x07, -8.0).unwrap();    // Longitude
        builder.add_f32_field(0x10, 0.5).unwrap();     // Roll
        builder.add_f32_field(0x11, -1.2).unwrap();    // Pitch
        builder.add_u8_field(0, 4).unwrap();           // State = InFlight
        builder.add_u8_field(1, 3).unwrap();           // Mode = AltHold
        builder.add_u32_field(2, 3600).unwrap();       // Uptime
        builder.add_u8_field(4, 75).unwrap();          // CpuLoad

        assert_eq!(builder.get_tlv_count(), 8);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();
        assert!(size > BYTHOS_OVERHEAD + 8 * TLV_HEADER_SIZE);
    }
}
