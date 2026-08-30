//! # Testes de Integração — Bythos v3.0.0
//!
//! Testes abrangentes para o protocolo Bythos v3.0.0.
//! Estes testes validam a interoperabilidade entre todos os módulos:
//! types, crc16, builder, codec e parser.

use bythos::protocol::types::*;
use bythos::protocol::crc16::*;
use bythos::protocol::builder::TLVBuilder;
use bythos::protocol::codec::*;
use bythos::parser::fsm::{Parser, ParserError};

// ============================================================================
// CONSTANTES
// ============================================================================

const TEST_KEY: u8 = 0x42;
const TEST_NODE: u8 = 0x06; // Device5 (node principal)

// ============================================================================
// TESTES — CONSTANTES Bythos v3.0.0
// ============================================================================

#[test]
fn test_bythos_v3_constants() {
    assert_eq!(START_BYTE, 0xAA);
    assert_eq!(BYTHOS_VERSION, 0x03);
    assert_eq!(BYTHOS_HEADER_SIZE, 7);
    assert_eq!(SIGNATURE_SIZE, 1);
    assert_eq!(CRC16_SIZE, 2);
    assert_eq!(BYTHOS_OVERHEAD, 10);
    assert_eq!(MAX_MESSAGE_SIZE, 1098);
}

#[test]
fn test_bythos_field_types() {
    assert_eq!(FieldType::Raw as u8, 0);
    assert_eq!(FieldType::Float32 as u8, 1);
    assert_eq!(FieldType::Float16 as u8, 2);
    assert_eq!(FieldType::Int32 as u8, 3);
    assert_eq!(FieldType::Uint32 as u8, 4);
    assert_eq!(FieldType::Uint16 as u8, 5);
    assert_eq!(FieldType::Uint8 as u8, 6);
    assert_eq!(FieldType::Bool as u8, 7);
}

#[test]
fn test_bythos_can_groups() {
    assert_eq!(CanGroup::None as u8, 0x0);
    assert_eq!(CanGroup::Device0 as u8, 0x1);
    assert_eq!(CanGroup::Device1 as u8, 0x2);
    assert_eq!(CanGroup::Device2 as u8, 0x3);
    assert_eq!(CanGroup::Device3 as u8, 0x4);
    assert_eq!(CanGroup::Device4 as u8, 0x5);
    assert_eq!(CanGroup::Device5 as u8, 0x6);
    assert_eq!(CanGroup::Device14 as u8, 0xF);
}

#[test]
fn test_bythos_can_msg_types() {
    assert_eq!(CanMsgType::Data as u8, 0x0);
    assert_eq!(CanMsgType::Cmd as u8, 0x1);
    assert_eq!(CanMsgType::Ack as u8, 0x2);
    assert_eq!(CanMsgType::Event as u8, 0x3);
    assert_eq!(CanMsgType::Sync as u8, 0x4);
    assert_eq!(CanMsgType::State as u8, 0x5);
    assert_eq!(CanMsgType::Heart as u8, 0x6);
    assert_eq!(CanMsgType::Safety as u8, 0x7);
}

// ============================================================================
// TESTES — FIELDID COM TIPO EMBUTIDO
// ============================================================================

#[test]
fn test_field_id_encode_decode_roundtrip() {
    for t in 0..=7u8 {
        for id in 0..=31u8 {
            let encoded = field_id_encode(t, id);
            let (decoded_t, decoded_id) = field_id_decode(encoded);
            assert_eq!(decoded_t, t, "Tipo não preservado para t={}, id={}", t, id);
            assert_eq!(decoded_id, id, "ID não preservado para t={}, id={}", t, id);
        }
    }
}

#[test]
fn test_field_id_specific_values() {
    // GPS Latitude: TYPE=1(f32) + ID=6 → 0x26
    assert_eq!(field_id_encode(1, 6), 0x26);
    // System State: TYPE=6(u8) + ID=0 → 0xC0
    assert_eq!(field_id_encode(6, 0), 0xC0);
    // Video Payload: TYPE=0(raw) + ID=0 → 0x00
    assert_eq!(field_id_encode(0, 0), 0x00);
    // Video FrameId: TYPE=5(u16) + ID=0 → 0xA0
    assert_eq!(field_id_encode(5, 0), 0xA0);
}

#[test]
fn test_is_valid_field_id() {
    // Todos os FieldIDs com tipo válido devem ser aceites
    assert!(is_valid_field_id(0x00)); // raw
    assert!(is_valid_field_id(0x26)); // f32
    assert!(is_valid_field_id(0xC0)); // u8
    assert!(is_valid_field_id(0xE0)); // bool
}

// ============================================================================
// TESTES — CAN ID
// ============================================================================

#[test]
fn test_make_can_id() {
    // Device5 envia telemetry para broadcast
    let can_id = make_can_id(
        PriorityLevel::High as u8,
        CanGroup::Device5 as u8,
        CanGroup::None as u8,
        CanMsgType::Data as u8,
    );
    assert_eq!(can_id_priority(can_id), PriorityLevel::High as u8);
    assert_eq!(can_id_src_group(can_id), CanGroup::Device5 as u8);
    assert_eq!(can_id_dst_group(can_id), CanGroup::None as u8);
    assert_eq!(can_id_msg_type(can_id), CanMsgType::Data as u8);
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
}

#[test]
fn test_can_id_no_conflict() {
    // Verificar que IDs diferentes produzem CAN IDs diferentes
    let id1 = make_can_id(2, 0x6, 0x0, 0x0);
    let id2 = make_can_id(2, 0x6, 0x1, 0x0);
    let id3 = make_can_id(2, 0x6, 0x0, 0x1);
    assert_ne!(id1, id2);
    assert_ne!(id1, id3);
    assert_ne!(id2, id3);
}

// ============================================================================
// TESTES — ASSINATURA
// ============================================================================

#[test]
fn test_signature_computation() {
    let sig = compute_signature(0x42, 0x11, 0x2A, 0x00);
    assert_eq!(sig, 0x42 ^ 0x11 ^ 0x2A ^ 0x00);
}

#[test]
fn test_signature_validation() {
    let sig = compute_signature(TEST_KEY, 0x11, 0x01, 0x00);
    assert!(validate_signature(sig, TEST_KEY, 0x11, 0x01, 0x00));
    assert!(!validate_signature(sig, 0x43, 0x11, 0x01, 0x00)); // key errada
    assert!(!validate_signature(sig, TEST_KEY, 0x12, 0x01, 0x00)); // msg_id errado
    assert!(!validate_signature(sig, TEST_KEY, 0x11, 0x02, 0x00)); // seq errado
}

#[test]
fn test_signature_zero_key() {
    // Com key=0, assinatura é XOR de msg_id, seq_lo, seq_hi
    let sig = compute_signature(0x00, 0x10, 0x00, 0x00);
    assert_eq!(sig, 0x10);
}

// ============================================================================
// TESTES — CRC-16
// ============================================================================

#[test]
fn test_crc16_known_vector() {
    assert_eq!(calc_crc16(b"123456789"), 0x29B1);
}

#[test]
fn test_crc16_consistency() {
    let data = [0xAA, 0x03, 0x06, 0x11, 0x2A, 0x00, 0x01, 0xC0, 0x01, 0x02];
    let crc1 = calc_crc16(&data);
    let crc2 = calc_crc16(&data);
    assert_eq!(crc1, crc2);
}

#[test]
fn test_crc16_different_data_different_crc() {
    let data1 = [0x01, 0x02, 0x03];
    let data2 = [0x01, 0x02, 0x04];
    assert_ne!(calc_crc16(&data1), calc_crc16(&data2));
}

// ============================================================================
// TESTES — BUILDER
// ============================================================================

#[test]
fn test_builder_basic() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    assert_eq!(size, BYTHOS_OVERHEAD + TLV_HEADER_SIZE + 1);
    assert_eq!(buffer[0], START_BYTE);
    assert_eq!(buffer[1], BYTHOS_VERSION);
    assert_eq!(buffer[2], TEST_NODE);
    assert_eq!(buffer[3], 0x11);
    assert_eq!(buffer[6], 1); // tlv_count
}

#[test]
fn test_builder_multiple_field_types() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(42);

    builder.add_f32_field(0x06, 40.0).unwrap();    // Latitude
    builder.add_f32_field(0x07, -8.0).unwrap();    // Longitude
    builder.add_u8_field(0, 4).unwrap();           // State = InFlight
    builder.add_u8_field(1, 3).unwrap();           // Mode = AltHold
    builder.add_u32_field(2, 3600).unwrap();       // Uptime
    builder.add_u16_field(0x20, 42).unwrap();      // FrameId
    builder.add_bool_field(0, true).unwrap();      // Bool field

    assert_eq!(builder.get_tlv_count(), 7);

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();
    assert!(size > BYTHOS_OVERHEAD);
}

#[test]
fn test_builder_signature() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(100);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    // Verificar assinatura
    let sig_offset = size - CRC16_SIZE - SIGNATURE_SIZE;
    let signature = buffer[sig_offset];
    let expected = compute_signature(TEST_KEY, 0x11, 100, 0);
    assert_eq!(signature, expected);
}

#[test]
fn test_builder_crc16() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    // Verificar CRC16
    let crc_offset = size - CRC16_SIZE;
    let crc_lo = buffer[crc_offset];
    let crc_hi = buffer[crc_offset + 1];
    let crc = (crc_hi as u16) << 8 | (crc_lo as u16);
    assert!(verify_crc16(&buffer[..crc_offset], crc));
}

// ============================================================================
// TESTES — CODEC VALIDATE
// ============================================================================

#[test]
fn test_validate_ok() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    assert_eq!(validate_message(&buffer[..size]), Ok(1));
}

#[test]
fn test_validate_invalid_start() {
    let mut buffer = [0u8; BYTHOS_OVERHEAD];
    buffer[0] = 0x00;
    buffer[1] = BYTHOS_VERSION;
    assert_eq!(validate_message(&buffer), Err(ProtocolError::InvalidStartByte));
}

#[test]
fn test_validate_invalid_version() {
    let mut buffer = [0u8; BYTHOS_OVERHEAD];
    buffer[0] = START_BYTE;
    buffer[1] = 0x99;
    assert_eq!(validate_message(&buffer), Err(ProtocolError::InvalidVersion));
}

#[test]
fn test_validate_invalid_msg_id() {
    let mut buffer = [0u8; BYTHOS_OVERHEAD];
    buffer[0] = START_BYTE;
    buffer[1] = BYTHOS_VERSION;
    buffer[3] = 0xFF;
    assert_eq!(validate_message(&buffer), Err(ProtocolError::InvalidMsgId));
}

#[test]
fn test_validate_invalid_crc() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();
    buffer[size - 1] = buffer[size - 1].wrapping_add(1);

    assert_eq!(validate_message(&buffer[..size]), Err(ProtocolError::InvalidChecksum));
}

#[test]
fn test_validate_signature_ok() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(42);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    assert!(validate_signature_in_message(&buffer[..size], TEST_KEY).is_ok());
}

#[test]
fn test_validate_signature_wrong_key() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(42);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    assert_eq!(
        validate_signature_in_message(&buffer[..size], 0x43),
        Err(ProtocolError::InvalidSignature)
    );
}

// ============================================================================
// TESTES — PARSER FSM
// ============================================================================

#[test]
fn test_parser_full_message() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    let mut parser = Parser::new(TEST_KEY);
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }

    assert!(parser.has_message());
    let msg = parser.get_message();
    assert_eq!(msg.start_byte, START_BYTE);
    assert_eq!(msg.version, BYTHOS_VERSION);
    assert_eq!(msg.node_id, TEST_NODE);
    assert_eq!(msg.msg_id, 0x11);
    assert_eq!(msg.seq_num, 1);
    assert_eq!(msg.tlv_count, 1);
    assert_eq!(msg.tlvs[0].id, 0xC0);
    assert_eq!(msg.tlvs[0].len, 1);
    assert_eq!(msg.tlvs[0].data[0], 2);

    parser.acknowledge();
    assert!(!parser.has_message());
}

#[test]
fn test_parser_multiple_tlvs() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(5);
    builder.add_f32_field(0x06, 40.0).unwrap();
    builder.add_f32_field(0x07, -8.0).unwrap();
    builder.add_u8_field(0, 4).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    let mut parser = Parser::new(TEST_KEY);
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }

    assert!(parser.has_message());
    let msg = parser.get_message();
    assert_eq!(msg.tlv_count, 3);
    assert_eq!(msg.seq_num, 5);

    // Verificar FieldIDs com tipo
    let (t0, _) = field_id_decode(msg.tlvs[0].id);
    assert_eq!(t0, FieldType::Float32 as u8);

    parser.acknowledge();
}

#[test]
fn test_parser_empty_tlv() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(0);

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x10, &mut buffer).unwrap(); // Heartbeat

    let mut parser = Parser::new(TEST_KEY);
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }

    assert!(parser.has_message());
    assert_eq!(parser.get_message().tlv_count, 0);
    parser.acknowledge();
}

#[test]
fn test_parser_invalid_crc() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();
    buffer[size - 1] = buffer[size - 1].wrapping_add(1);

    let mut parser = Parser::new(TEST_KEY);
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }

    assert!(!parser.has_message());
    assert!(parser.get_error_count() > 0);
}

#[test]
fn test_parser_invalid_signature() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();
    let sig_idx = size - CRC16_SIZE - SIGNATURE_SIZE;
    buffer[sig_idx] = buffer[sig_idx].wrapping_add(1);

    let mut parser = Parser::new(TEST_KEY);
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }

    assert!(!parser.has_message());
    assert!(parser.get_error_count() > 0);
}

#[test]
fn test_parser_wrong_key() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    let mut parser = Parser::new(0x43); // Key errada
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }

    assert!(!parser.has_message());
    assert!(parser.get_error_count() > 0);
}

#[test]
fn test_parser_consecutive_messages() {
    let mut builder1 = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder1.set_seq(1);
    builder1.add_u8_field(0, 2).unwrap();
    let mut buffer1 = [0u8; MAX_MESSAGE_SIZE];
    let size1 = builder1.build(0x11, &mut buffer1).unwrap();

    let mut builder2 = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder2.set_seq(2);
    builder2.add_u16_field(0x20, 42).unwrap();
    let mut buffer2 = [0u8; MAX_MESSAGE_SIZE];
    let size2 = builder2.build(0x16, &mut buffer2).unwrap();

    let mut parser = Parser::new(TEST_KEY);

    // Primeira mensagem
    for &byte in &buffer1[..size1] {
        parser.feed(byte);
    }
    assert!(parser.has_message());
    assert_eq!(parser.get_message().msg_id, 0x11);
    parser.acknowledge();

    // Segunda mensagem
    for &byte in &buffer2[..size2] {
        parser.feed(byte);
    }
    assert!(parser.has_message());
    assert_eq!(parser.get_message().msg_id, 0x16);
    assert_eq!(parser.get_success_count(), 2);
    parser.acknowledge();
}

#[test]
fn test_parser_byte_a_byte() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(10);
    builder.add_u8_field(0, 2).unwrap();
    builder.add_f32_field(0x10, 3.14).unwrap();

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    let mut parser = Parser::new(TEST_KEY);
    let mut state_transitions = Vec::new();

    for i in 0..size {
        let state_before = parser.get_current_state();
        let result = parser.feed(buffer[i]);
        let state_after = parser.get_current_state();

        if state_before != state_after {
            state_transitions.push((state_before as u8, state_after as u8));
        }

        assert_eq!(result, ParserError::Ok, "Erro no byte {}", i);
    }

    assert!(parser.has_message());
    // Deve ter transicionado por vários estados
    assert!(state_transitions.len() >= 5);
}

// ============================================================================
// TESTES — ROUNDTRIP COMPLETO
// ============================================================================

#[test]
fn test_roundtrip_builder_validate_parse() {
    // Construir mensagem
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(42);
    builder.add_f32_field(0x06, 40.0).unwrap();    // Latitude
    builder.add_f32_field(0x07, -8.0).unwrap();    // Longitude
    builder.add_f32_field(0x10, 0.5).unwrap();     // Roll
    builder.add_u8_field(0, 4).unwrap();           // State
    builder.add_u8_field(1, 3).unwrap();           // Mode
    builder.add_u32_field(2, 3600).unwrap();       // Uptime
    builder.add_u16_field(0x20, 42).unwrap();      // FrameId

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    // Validar
    let tlv_count = validate_message(&buffer[..size]).unwrap();
    assert_eq!(tlv_count, 7);

    // Validar assinatura
    assert!(validate_signature_in_message(&buffer[..size], TEST_KEY).is_ok());

    // Parse com FSM
    let mut parser = Parser::new(TEST_KEY);
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }
    assert!(parser.has_message());

    let msg = parser.get_message();
    assert_eq!(msg.tlv_count, 7);
    assert_eq!(msg.seq_num, 42);
    assert_eq!(msg.node_id, TEST_NODE);

    // Verificar dados
    assert_eq!(msg.tlvs[0].id, 0x26); // Latitude f32
    assert_eq!(msg.tlvs[3].id, 0xC0); // State u8
    assert_eq!(msg.tlvs[3].data[0], 4);
    assert_eq!(msg.tlvs[6].id, 0xA0); // FrameId u16

    parser.acknowledge();
}

#[test]
fn test_roundtrip_various_msg_types() {
    let msg_types = [
        (0x10, "Heartbeat"),
        (0x11, "Telemetry"),
        (0x12, "Command"),
        (0x13, "Ack"),
        (0x14, "Failsafe"),
        (0x15, "Debug"),
        (0x16, "Video"),
        (0x17, "Shell"),
        (0x18, "SiData"),
        (0x19, "Watchdog"),
        (0x1A, "Ping"),
        (0x1B, "Clock"),
    ];

    for (msg_id, name) in msg_types {
        let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
        builder.set_seq(1);
        builder.add_u8_field(0, 2).unwrap();

        let mut buffer = [0u8; MAX_MESSAGE_SIZE];
        let size = builder.build(msg_id, &mut buffer).unwrap();

        let mut parser = Parser::new(TEST_KEY);
        for &byte in &buffer[..size] {
            parser.feed(byte);
        }

        assert!(parser.has_message(), "Falha para msg_type {} ({})", msg_id, name);
        assert_eq!(parser.get_message().msg_id, msg_id);
        parser.acknowledge();
    }
}

#[test]
fn test_roundtrip_various_nodes() {
    let nodes = [
        (0x01, "Device0"),
        (0x02, "Device1"),
        (0x03, "Device2"),
        (0x04, "Device3"),
        (0x05, "Device4"),
        (0x06, "Device5"),
    ];

    for (node_id, name) in nodes {
        let mut builder = TLVBuilder::new(node_id, TEST_KEY);
        builder.set_seq(1);
        builder.add_u8_field(0, 2).unwrap();

        let mut buffer = [0u8; MAX_MESSAGE_SIZE];
        let size = builder.build(0x11, &mut buffer).unwrap();

        assert_eq!(buffer[2], node_id, "Node ID incorreto para {}", name);

        let mut parser = Parser::new(TEST_KEY);
        for &byte in &buffer[..size] {
            parser.feed(byte);
        }

        assert!(parser.has_message(), "Falha para nó {}", name);
        assert_eq!(parser.get_message().node_id, node_id);
        parser.acknowledge();
    }
}

#[test]
fn test_roundtrip_various_seq_numbers() {
    let seqs: [u16; 5] = [0, 1, 255, 256, 65535];

    for seq in seqs {
        let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
        builder.set_seq(seq);
        builder.add_u8_field(0, 2).unwrap();

        let mut buffer = [0u8; MAX_MESSAGE_SIZE];
        let size = builder.build(0x11, &mut buffer).unwrap();

        let mut parser = Parser::new(TEST_KEY);
        for &byte in &buffer[..size] {
            parser.feed(byte);
        }

        assert!(parser.has_message(), "Falha para seq={}", seq);
        assert_eq!(parser.get_message().seq_num, seq);
        parser.acknowledge();
    }
}

// ============================================================================
// TESTES — CASOS EXTREMOS
// ============================================================================

#[test]
fn test_max_fields() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.set_seq(1);

    for i in 0..MAX_TLV_FIELDS as u8 {
        builder.add_u8(i, 0).unwrap();
    }

    let mut buffer = [0u8; MAX_MESSAGE_SIZE];
    let size = builder.build(0x11, &mut buffer).unwrap();

    let mut parser = Parser::new(TEST_KEY);
    for &byte in &buffer[..size] {
        parser.feed(byte);
    }

    assert!(parser.has_message());
    assert_eq!(parser.get_message().tlv_count, MAX_TLV_FIELDS as u8);
    parser.acknowledge();
}

#[test]
fn test_builder_overflow() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    for i in 0..MAX_TLV_FIELDS {
        builder.add_u8(i as u8, 0).unwrap();
    }

    assert!(builder.add_u8(0xFF, 0).is_err());
}

#[test]
fn test_builder_buffer_too_small() {
    let mut builder = TLVBuilder::new(TEST_NODE, TEST_KEY);
    builder.add_u8_field(0, 2).unwrap();

    let mut buffer = [0u8; 5];
    assert!(builder.build(0x11, &mut buffer).is_err());
}
