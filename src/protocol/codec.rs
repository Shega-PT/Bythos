//! # Codec Bythos — Serialização, Validação e Parsing (v3.0.0)
//!
//! Este módulo implementa as operações fundamentais de serialização,
//! validação e parsing de mensagens Bythos v3.0.0.

use crate::protocol::types::*;
use crate::protocol::crc16::calc_crc16;

/// Erros possíveis nas operações de codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// Tamanho do buffer insuficiente para a operação.
    BufferTooSmall,
    /// Número máximo de campos TLV excedido.
    TooManyFields,
    /// Byte de início inválido na mensagem.
    InvalidStartByte,
    /// Versão do protocolo incompatível.
    InvalidVersion,
    /// ID da mensagem inválido.
    InvalidMsgId,
    /// Tamanho do campo TLV inválido.
    InvalidTlvLength,
    /// Checksum CRC16 inválido.
    InvalidChecksum,
    /// Assinatura inválida.
    InvalidSignature,
    /// Dados de entrada corrompidos ou incompletos.
    CorruptedData,
    /// Campo TLV com tamanho maior que o permitido.
    TlvDataTooLong,
}

// ============================================================================
// FUNÇÕES DE SERIALIZAÇÃO
// ============================================================================

/// Serializa um campo TLV individual em bytes.
///
/// Formato resultante: `[ID][LEN][DATA...]`
///
/// # Arguments
///
/// * `id` — FieldID codificado ([TYPE:3][ID:5]).
/// * `data` — Dados do campo.
/// * `output` — Buffer de saída.
///
/// # Returns
///
/// `Ok(bytes_written)` em sucesso, `Err(ProtocolError)` em caso de erro.
pub fn build_tlv(id: u8, data: &[u8], output: &mut [u8]) -> Result<usize, ProtocolError> {
    let data_len = data.len().min(MAX_TLV_DATA);
    let required_size = TLV_HEADER_SIZE + data_len;

    if output.len() < required_size {
        return Err(ProtocolError::BufferTooSmall);
    }

    output[0] = id;
    output[1] = data_len as u8;
    output[TLV_HEADER_SIZE..TLV_HEADER_SIZE + data_len]
        .copy_from_slice(&data[..data_len]);

    Ok(required_size)
}

/// Serializa um campo TLV de vídeo em bytes.
///
/// Utiliza `MAX_TLV_VIDEO_DATA` (128 bytes) em vez de `MAX_TLV_DATA` (32 bytes).
///
/// # Arguments
///
/// * `data` — Dados do campo de vídeo.
/// * `output` — Buffer de saída.
///
/// # Returns
///
/// `Ok(bytes_written)` em sucesso, `Err(ProtocolError)` em caso de erro.
pub fn build_tlv_video(data: &[u8], output: &mut [u8]) -> Result<usize, ProtocolError> {
    let data_len = data.len().min(MAX_TLV_VIDEO_DATA);
    let required_size = TLV_HEADER_SIZE + data_len;

    if output.len() < required_size {
        return Err(ProtocolError::BufferTooSmall);
    }

    output[0] = FieldVideo::Payload as u8;
    output[1] = data_len as u8;
    output[TLV_HEADER_SIZE..TLV_HEADER_SIZE + data_len]
        .copy_from_slice(&data[..data_len]);

    Ok(required_size)
}

/// Serializa uma mensagem Bythos completa a partir de uma struct TLVMessage.
///
/// Formato resultante:
/// ```text
/// [START_BYTE][VERSION][NODE_ID][MSG_ID][SEQ_NUM(2)][TLV_COUNT]
/// [TLV_FIELDS...][SIGNATURE][CRC16(2)]
/// ```
///
/// # Arguments
///
/// * `msg` — Mensagem Bythos a serializar.
/// * `msg_id` — Identificador do tipo de mensagem (sobrepõe `msg.msg_id`).
/// * `signature_key` — Chave de assinatura (XOR key).
/// * `buffer` — Buffer de saída.
///
/// # Returns
///
/// `Ok(bytes_written)` em sucesso, `Err(ProtocolError)` em caso de erro.
pub fn build_message(
    msg: &TLVMessage,
    msg_id: u8,
    signature_key: u8,
    buffer: &mut [u8],
) -> Result<usize, ProtocolError> {
    // Calcular tamanho necessário baseado no LEN real de cada TLV
    let fields_size: usize = (0..msg.tlv_count as usize)
        .map(|i| TLV_HEADER_SIZE + msg.tlvs[i].len as usize)
        .sum();
    let required_size = BYTHOS_HEADER_SIZE + fields_size + SIGNATURE_SIZE + CRC16_SIZE;

    if buffer.len() < required_size {
        return Err(ProtocolError::BufferTooSmall);
    }

    let mut offset = 0;

    // --- Cabeçalho Bythos (7 bytes) ---
    buffer[offset] = START_BYTE;
    offset += 1;
    buffer[offset] = BYTHOS_VERSION;
    offset += 1;
    buffer[offset] = msg.node_id;
    offset += 1;
    buffer[offset] = msg_id;
    offset += 1;

    // SEQ_NUM (2 bytes, little-endian)
    let seq_bytes = msg.seq_num.to_le_bytes();
    buffer[offset] = seq_bytes[0];
    offset += 1;
    buffer[offset] = seq_bytes[1];
    offset += 1;

    buffer[offset] = msg.tlv_count;
    offset += 1;

    // --- Campos TLV ---
    for i in 0..msg.tlv_count as usize {
        let tlv = &msg.tlvs[i];
        buffer[offset] = tlv.id;
        offset += 1;
        buffer[offset] = tlv.len;
        offset += 1;
        let len = tlv.len as usize;
        buffer[offset..offset + len].copy_from_slice(&tlv.data[..len]);
        offset += len;
    }

    // --- Assinatura ---
    let signature = compute_signature(
        signature_key,
        msg_id,
        seq_bytes[0],
        seq_bytes[1],
    );
    buffer[offset] = signature;
    offset += 1;

    // --- CRC16 ---
    let crc = calc_crc16(&buffer[..offset]);
    let crc_bytes = crc.to_le_bytes();
    buffer[offset] = crc_bytes[0];
    offset += 1;
    buffer[offset] = crc_bytes[1];
    offset += 1;

    Ok(offset)
}

// ============================================================================
// FUNÇÕES DE VALIDAÇÃO
// ============================================================================

/// Valida a integridade e estrutura de uma mensagem Bythos serializada.
///
/// Verifica:
/// 1. Tamanho mínimo
/// 2. Byte de início (START_BYTE = 0xAA)
/// 3. Versão do protocolo (BYTHOS_VERSION = 0x03)
/// 4. ID da mensagem válido
/// 5. Número de campos TLV dentro do limite
/// 6. Checksum CRC16
/// 7. IDs dos campos TLV válidos
/// 8. Tamanhos dos campos TLV dentro dos limites
///
/// # Arguments
///
/// * `buffer` — Buffer contendo a mensagem serializada.
///
/// # Returns
///
/// `Ok(tlv_count)` em sucesso (número de campos TLV válidos),
/// `Err(ProtocolError)` em caso de erro.
pub fn validate_message(buffer: &[u8]) -> Result<u8, ProtocolError> {
    // Verificar tamanho mínimo: header(7) + signature(1) + crc16(2) = 10
    if buffer.len() < BYTHOS_OVERHEAD {
        return Err(ProtocolError::CorruptedData);
    }

    // Verificar byte de início
    if buffer[0] != START_BYTE {
        return Err(ProtocolError::InvalidStartByte);
    }

    // Verificar versão
    if buffer[1] != BYTHOS_VERSION {
        return Err(ProtocolError::InvalidVersion);
    }

    // Verificar ID da mensagem
    let msg_id = buffer[3];
    if !MsgId::is_valid(msg_id) {
        return Err(ProtocolError::InvalidMsgId);
    }

    // Verificar número de campos TLV
    let tlv_count = buffer[6];
    if tlv_count as usize > MAX_TLV_FIELDS {
        return Err(ProtocolError::InvalidTlvLength);
    }

    // Calcular tamanho total da mensagem baseado no LEN real de cada TLV
    let mut calculated_size = BYTHOS_HEADER_SIZE;
    let mut offset = BYTHOS_HEADER_SIZE;
    for _ in 0..tlv_count {
        if offset + TLV_HEADER_SIZE > buffer.len() {
            return Err(ProtocolError::CorruptedData);
        }
        let field_len = buffer[offset + 1] as usize;
        if field_len > MAX_TLV_DATA {
            return Err(ProtocolError::TlvDataTooLong);
        }
        offset += TLV_HEADER_SIZE + field_len;
        calculated_size += TLV_HEADER_SIZE + field_len;
    }

    // Adicionar signature + crc16
    calculated_size += SIGNATURE_SIZE + CRC16_SIZE;

    if buffer.len() < calculated_size {
        return Err(ProtocolError::CorruptedData);
    }

    // Verificar CRC16 (últimos 2 bytes)
    let crc_offset = calculated_size - CRC16_SIZE;
    let crc_lo = buffer[crc_offset];
    let crc_hi = buffer[crc_offset + 1];
    let expected_crc = (crc_hi as u16) << 8 | (crc_lo as u16);
    let computed_crc = calc_crc16(&buffer[..crc_offset]);
    if computed_crc != expected_crc {
        return Err(ProtocolError::InvalidChecksum);
    }

    // Validar campos TLV (IDs e tamanhos)
    offset = BYTHOS_HEADER_SIZE;
    for _ in 0..tlv_count {
        let field_id = buffer[offset];
        let field_len = buffer[offset + 1] as usize;

        // Validar tipo do FieldID
        let (field_type, _) = field_id_decode(field_id);
        if FieldType::from_u8(field_type).is_none() {
            return Err(ProtocolError::InvalidTlvLength);
        }

        if field_len > MAX_TLV_DATA {
            return Err(ProtocolError::TlvDataTooLong);
        }

        offset += TLV_HEADER_SIZE + field_len;
    }

    Ok(tlv_count)
}

/// Valida a assinatura de uma mensagem Bythos serializada.
///
/// # Arguments
///
/// * `buffer` — Mensagem serializada (com signature já incluída).
/// * `signature_key` — Chave de assinatura esperada.
///
/// # Returns
///
/// `Ok(())` se a assinatura é válida, `Err(ProtocolError::InvalidSignature)` caso contrário.
pub fn validate_signature_in_message(buffer: &[u8], signature_key: u8) -> Result<(), ProtocolError> {
    if buffer.len() < BYTHOS_OVERHEAD {
        return Err(ProtocolError::CorruptedData);
    }

    let msg_id = buffer[3];
    let seq_lo = buffer[4];
    let seq_hi = buffer[5];
    let signature = buffer[buffer.len() - CRC16_SIZE - SIGNATURE_SIZE];

    let expected = compute_signature(signature_key, msg_id, seq_lo, seq_hi);
    if signature != expected {
        return Err(ProtocolError::InvalidSignature);
    }

    Ok(())
}

// ============================================================================
// FUNÇÕES DE PARSING
// ============================================================================

/// Deserializa campos TLV de um buffer de bytes brutos.
///
/// Assume que o buffer contém apenas campos TLV serializados (sem cabeçalho
/// de mensagem). Cada campo tem o formato: `[ID][LEN][DATA...]`.
///
/// # Arguments
///
/// * `data` — Buffer com campos TLV serializados.
/// * `output` — Array de saída para os campos TLV deserializados.
///
/// # Returns
///
/// `Ok(count)` com o número de campos TLV deserializados,
/// `Err(ProtocolError)` em caso de erro.
pub fn parse_tlv(data: &[u8], output: &mut [TLVField]) -> Result<usize, ProtocolError> {
    let mut offset = 0;
    let mut count = 0;

    while offset + TLV_HEADER_SIZE <= data.len() && count < output.len() {
        let id = data[offset];
        let len = data[offset + 1] as usize;

        if len > MAX_TLV_DATA {
            return Err(ProtocolError::TlvDataTooLong);
        }

        if offset + TLV_HEADER_SIZE + len > data.len() {
            return Err(ProtocolError::CorruptedData);
        }

        output[count] = TLVField {
            id,
            len: len as u8,
            data: {
                let mut arr = [0u8; MAX_TLV_DATA];
                arr[..len].copy_from_slice(&data[offset + TLV_HEADER_SIZE..offset + TLV_HEADER_SIZE + len]);
                arr
            },
        };

        offset += TLV_HEADER_SIZE + len;
        count += 1;
    }

    Ok(count)
}

/// Imprime informações de debug sobre um campo TLV.
///
/// Útil para depuração. Em produção, pode ser desativado.
pub fn print_tlv_field(field: &TLVField) {
    #[cfg(feature = "std")]
    {
        let (field_type, _field_id) = field_id_decode(field.id);
        let type_name = match FieldType::from_u8(field_type) {
            Some(FieldType::Raw) => "raw",
            Some(FieldType::Float32) => "f32",
            Some(FieldType::Float16) => "f16",
            Some(FieldType::Int32) => "i32",
            Some(FieldType::Uint32) => "u32",
            Some(FieldType::Uint16) => "u16",
            Some(FieldType::Uint8) => "u8",
            Some(FieldType::Bool) => "bool",
            None => "???",
        };
        print!("[TLV] ID=0x{:02X} TYPE={}({}) LEN={}", field.id, type_name, field_type, field.len);
        if field.len > 0 && (field.len as usize) <= field.data.len() {
            print!(" DATA=[");
            for i in 0..field.len as usize {
                if i > 0 {
                    print!(" ");
                }
                print!("{:02X}", field.data[i]);
            }
            print!("]");
        }
        println!();
    }
}

/// Imprime informações de debug sobre uma mensagem Bythos.
///
/// Útil para depuração. Em produção, pode ser desativado.
pub fn print_message(msg: &TLVMessage) {
    #[cfg(feature = "std")]
    {
        println!("[BYTHOS] START=0x{:02X} VER=0x{:02X} NODE=0x{:02X} MSGID=0x{:02X} SEQ={} TLV_COUNT={} SIG=0x{:02X} CRC=0x{:04X}",
            msg.start_byte, msg.version, msg.node_id, msg.msg_id,
            msg.seq_num, msg.tlv_count, msg.signature, msg.checksum);
        for i in 0..msg.tlv_count as usize {
            print_tlv_field(&msg.tlvs[i]);
        }
    }
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::builder::TLVBuilder;

    #[test]
    fn test_build_tlv() {
        let data = [0x01, 0x02, 0x03];
        let mut output = [0u8; 100];
        let size = build_tlv(0xC0, &data, &mut output).unwrap();

        assert_eq!(output[0], 0xC0);  // id
        assert_eq!(output[1], 3);     // len
        assert_eq!(output[2], 0x01);  // data[0]
        assert_eq!(output[3], 0x02);  // data[1]
        assert_eq!(output[4], 0x03);  // data[2]
        assert_eq!(size, 5);          // TLV_HEADER_SIZE + 3
    }

    #[test]
    fn test_build_tlv_video() {
        let data = [0xAA, 0xBB, 0xCC];
        let mut output = [0u8; 200];
        let size = build_tlv_video(&data, &mut output).unwrap();

        assert_eq!(output[0], FieldVideo::Payload as u8);  // id
        assert_eq!(output[1], 3);                            // len
        assert_eq!(size, 5);
    }

    #[test]
    fn test_build_message() {
        let mut msg = TLVMessage::with_params(0x11, 0x06);
        msg.tlv_count = 1;
        msg.tlvs[0] = TLVField::with_data(0xC0, &[0x02]);

        let mut buffer = [0u8; 1098];
        let size = build_message(&msg, 0x11, 0x42, &mut buffer).unwrap();

        assert_eq!(buffer[0], START_BYTE);
        assert_eq!(buffer[1], BYTHOS_VERSION);
        assert_eq!(buffer[2], 0x06);
        assert_eq!(buffer[3], 0x11);
        assert!(size > 0);
    }

    #[test]
    fn test_validate_message_ok() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.set_seq(1);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        let result = validate_message(&buffer[..size]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_validate_message_invalid_start() {
        let mut buffer = [0u8; 1098];
        buffer[0] = 0x00;  // Byte de início inválido
        buffer[1] = BYTHOS_VERSION;
        buffer[2] = 0x06;
        buffer[3] = 0x11;
        buffer[4] = 0;
        buffer[5] = 0;
        buffer[6] = 0;

        let result = validate_message(&buffer);
        assert_eq!(result, Err(ProtocolError::InvalidStartByte));
    }

    #[test]
    fn test_validate_message_invalid_version() {
        let mut buffer = [0u8; 1098];
        buffer[0] = START_BYTE;
        buffer[1] = 0x99;  // Versão inválida
        buffer[2] = 0x06;
        buffer[3] = 0x11;

        let result = validate_message(&buffer);
        assert_eq!(result, Err(ProtocolError::InvalidVersion));
    }

    #[test]
    fn test_validate_message_invalid_msg_id() {
        let mut buffer = [0u8; 1098];
        buffer[0] = START_BYTE;
        buffer[1] = BYTHOS_VERSION;
        buffer[2] = 0x06;
        buffer[3] = 0xFF;  // ID inválido

        let result = validate_message(&buffer);
        assert_eq!(result, Err(ProtocolError::InvalidMsgId));
    }

    #[test]
    fn test_validate_message_invalid_crc() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        // Corromper CRC
        buffer[size - 1] = buffer[size - 1].wrapping_add(1);

        let result = validate_message(&buffer[..size]);
        assert_eq!(result, Err(ProtocolError::InvalidChecksum));
    }

    #[test]
    fn test_validate_signature_ok() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.set_seq(42);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        assert!(validate_signature_in_message(&buffer[..size], 0x42).is_ok());
    }

    #[test]
    fn test_validate_signature_wrong_key() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.set_seq(42);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        assert_eq!(
            validate_signature_in_message(&buffer[..size], 0x43),
            Err(ProtocolError::InvalidSignature)
        );
    }

    #[test]
    fn test_parse_tlv() {
        let mut tlv_buf = [0u8; 100];
        let mut offset = 0;
        offset += build_tlv(0xC0, &[0x02], &mut tlv_buf[offset..]).unwrap();
        offset += build_tlv(0x30, &float_to_bytes(1.5), &mut tlv_buf[offset..]).unwrap();

        let mut output = [TLVField::new(); 10];
        let count = parse_tlv(&tlv_buf[..offset], &mut output).unwrap();

        assert_eq!(count, 2);
        assert_eq!(output[0].id, 0xC0);
        assert_eq!(output[0].len, 1);
        assert_eq!(output[1].id, 0x30);
        assert_eq!(output[1].len, 4);
    }

    #[test]
    fn test_parse_tlv_empty() {
        let mut output = [TLVField::new(); 10];
        let count = parse_tlv(&[], &mut output).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_roundtrip_build_validate() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.add_f32_field(0x10, 3.14).unwrap();
        builder.add_u16_field(0x20, 42).unwrap();
        builder.set_seq(7);

        let mut buffer = [0u8; 1098];
        let size = builder.build(0x11, &mut buffer).unwrap();

        // Validar
        let tlv_count = validate_message(&buffer[..size]).unwrap();
        assert_eq!(tlv_count, 3);

        // Validar assinatura
        assert!(validate_signature_in_message(&buffer[..size], 0x42).is_ok());
    }

    #[test]
    fn test_roundtrip_build_parse() {
        let mut builder = TLVBuilder::new(0x06, 0x42);
        builder.add_u8_field(0, 2).unwrap();
        builder.add_f32_field(0x10, 3.14).unwrap();
        builder.set_seq(10);

        let mut buffer = [0u8; 1098];
        let _size = builder.build(0x11, &mut buffer).unwrap();

        // Extrair TLV fields (após header, antes de signature+crc)
        let tlv_count = buffer[6] as usize;
        let tlv_data_start = BYTHOS_HEADER_SIZE;
        let mut tlv_data_end = tlv_data_start;
        for _ in 0..tlv_count {
            let field_len = buffer[tlv_data_end + 1] as usize;
            tlv_data_end += TLV_HEADER_SIZE + field_len;
        }

        let mut output = [TLVField::new(); 32];
        let count = parse_tlv(&buffer[tlv_data_start..tlv_data_end], &mut output).unwrap();
        assert_eq!(count, 2);
        assert_eq!(output[0].id, 0xC0); // u8 type
        assert_eq!(output[1].id, 0x30); // f32 type
    }
}
