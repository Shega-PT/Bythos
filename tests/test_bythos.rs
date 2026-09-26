// SPDX-License-Identifier: GPL-3.0-or-later

//! # Testes de Integração — Bythos v4.0.0
//!
//! Ponta a ponta entre todos os módulos com a API pública final: construtor,
//! selagem, validação, análise, anel e túnel. Cada teste usa só nomes V4 —
//! se um identificador V3 (`TLV*`, `Can*`, `Device*`) vazar para aqui, é bug.

use bythos::parser::fsm::{Parser, ParserError};
use bythos::protocol::builder::BythosBuilder;
use bythos::protocol::codec::*;
use bythos::protocol::crc16::*;
use bythos::protocol::secure::{PeerTable, SealConfig};
use bythos::protocol::types::*;
use bythos::ring::{decide, Direction};
use bythos::tunnel::{cobs_decode, cobs_encode, VideoFrag, VideoReassembler, TUNNEL_MAX};

// ============================================================================
// FIXTURES
// ============================================================================

/// Chave de ensaio (32 B). Produção usa o elemento seguro — ver `SealConfig`.
const TEST_KEY: [u8; 32] = [0x42u8; 32];

/// Construtor de ensaio: origem 6, selo de software, `CTR` a zero.
fn builder() -> BythosBuilder {
    BythosBuilder::new(6, SealConfig::software(0, TEST_KEY, 6))
}

/// Emite telemetria típica e devolve os bytes no fio.
fn telemetria(seq: u16) -> ([u8; MAX_MESSAGE_SIZE], usize) {
    let mut b = builder();
    b.set_seq(seq);
    b.add_u8_field(0, 2).unwrap(); // estado = Ready
    b.add_f32_field(6, -33.8999).unwrap(); // latitude
    b.add_f32_field(7, 151.2093).unwrap(); // longitude
    b.add_f32_field(0x10, 1.5).unwrap(); // rolamento
    b.add_u32_field(2, 3600).unwrap(); // uptime
    let mut buf = [0u8; MAX_MESSAGE_SIZE];
    let n = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
    (buf, n)
}

// ============================================================================
// CONSTANTES E CATÁLOGO
// ============================================================================

#[test]
fn test_constantes_v4() {
    // O fio novo em números: 11 de cabeça, 21 de sobrecarga, 1205 de teto.
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
}

#[test]
fn test_tipos_e_grupos_sem_can() {
    // Nomes de função, valores herdados — e nenhum `DeviceX` à vista.
    assert_eq!(BythosFieldType::Raw as u8, 0);
    assert_eq!(BythosFieldType::Bool as u8, 7);
    assert_eq!(BythosGroup::None as u8, 0x0);
    assert_eq!(BythosGroup::Control as u8, 0x1);
    assert_eq!(BythosGroup::Safety as u8, 0x4);
    assert_eq!(BythosGroup::Vision as u8, 0x6);
    assert_eq!(BythosKind::Safety as u8, 0x7);
    assert_eq!(BythosKind::Ring as u8, 0x8);
    // Mensagens do anel existem e as V3 continuam válidas.
    for id in 0x10..=0x1F {
        assert!(BythosMsgId::is_valid(id), "0x{id:02X} devia ser válido");
    }
    assert!(!BythosMsgId::is_valid(0x20));
}

// ============================================================================
// IDA-VOLTA COMPLETA (construir → selar → validar → analisar)
// ============================================================================

#[test]
fn test_ida_volta_ponta_a_ponta() {
    let (buf, n) = telemetria(1);
    // Destino final: estrutura + TAG + replay, tudo de uma vez.
    let mut peers = PeerTable::new();
    let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
    assert_eq!(view.header.src, 6);
    assert_eq!(view.header.dst, BROADCAST_ADDR);
    assert_eq!(view.header.msg_id, 0x11);
    assert_eq!(view.header.field_count, 5);
    // Ponte no caminho: só estrutura + CRC, sem chaves.
    let head = peek_header(&buf[..n]).unwrap();
    assert_eq!(head.src, 6);
    // Analisador byte-a-byte entrega a mesma mensagem.
    let mut p = Parser::new();
    for &byte in &buf[..n] {
        assert_eq!(p.feed(byte), ParserError::Ok);
    }
    assert!(p.has_message());
    let msg = p.get_message();
    assert_eq!(msg.field_count, 5);
    assert_eq!(msg.fields[0].id, 0xC0);
    // E a trama completa do analisador valida ponta-a-ponta.
    let frame = p.completed_frame().unwrap();
    let mut peers2 = PeerTable::new();
    assert!(validate_message(frame, &TEST_KEY, &mut peers2).is_ok());
}

#[test]
fn test_todos_os_tipos_de_mensagem() {
    // Os 16 tipos atravessam o fio (vazios, só preâmbulo + reboque).
    for msg_id in 0x10..=0x1F {
        let mut b = builder();
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(msg_id, BROADCAST_ADDR, &mut buf).unwrap();
        assert_eq!(n, BYTHOS_OVERHEAD);
        let mut peers = PeerTable::new();
        let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
        assert_eq!(view.header.msg_id, msg_id);
    }
}

#[test]
fn test_enderecos_u16_sem_limite_baixo() {
    // Origens altas (impossíveis no NODE_ID u8 da V3) viajam na V4.
    for src in [0u16, 1, 255, 256, 1000, 60000] {
        let mut b = BythosBuilder::new(src, SealConfig::software(0, TEST_KEY, src));
        b.add_u8_field(0, 1).unwrap();
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(0x11, 2, &mut buf).unwrap();
        let mut peers = PeerTable::new();
        let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
        assert_eq!(view.header.src, src);
        assert_eq!(view.header.dst, 2);
    }
}

#[test]
fn test_sequencias_limite() {
    // Fronteiras do u16 (0, 255/256, máximo) — o SEQ é diagnóstico, não cerca.
    for seq in [0u16, 1, 255, 256, 65535] {
        let (buf, n) = telemetria(seq);
        let mut peers = PeerTable::new();
        let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
        assert_eq!(view.header.seq, seq);
    }
}

#[test]
fn test_trama_maxima_32_campos() {
    // Teto de campos: 32 passam, o 33.º é recusado na construção.
    let mut b = builder();
    for i in 0u8..32 {
        b.add_u8_field(i % 32, i).unwrap();
    }
    assert_eq!(b.field_count(), 32);
    assert_eq!(b.add_u8_field(0, 0), Err(ProtocolError::TooManyFields));
    let mut buf = [0u8; MAX_MESSAGE_SIZE];
    let n = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
    assert!(n <= MAX_MESSAGE_SIZE);
    let mut peers = PeerTable::new();
    assert_eq!(
        validate_message(&buf[..n], &TEST_KEY, &mut peers)
            .unwrap()
            .header
            .field_count,
        32
    );
}

#[test]
fn test_f16_no_fio() {
    // Meia precisão atravessa o fio em 2 bytes (dívida V3 fechada).
    let mut b = builder();
    b.add_f16_field(4, 21.5).unwrap();
    let mut buf = [0u8; MAX_MESSAGE_SIZE];
    let n = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
    let mut peers = PeerTable::new();
    let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
    assert_eq!(view.header.field_count, 1);
    let mut out = [BythosField::new(); 4];
    let k = parse_fields(&buf[11..view.fields_end], &mut out).unwrap();
    assert_eq!(k, 1);
    assert_eq!(out[0].len, 2);
    let half = u16::from_le_bytes([out[0].data[0], out[0].data[1]]);
    assert!((f16_to_f32(half) - 21.5).abs() < 0.05);
}

// ============================================================================
// SEGURANÇA (adversário e repetidor)
// ============================================================================

#[test]
fn test_adulteracao_morre_no_crc_ou_tag() {
    // Qualquer bit trocado — carga, SEC_HDR ou TAG — invalida a trama.
    let (base, n) = telemetria(9);
    for idx in [11usize, 14, n - 11, n - 5, n - 1] {
        let mut buf = base;
        buf[idx] ^= 0x01;
        let mut peers = PeerTable::new();
        assert!(
            validate_message(&buf[..n], &TEST_KEY, &mut peers).is_err(),
            "byte {idx}"
        );
    }
}

#[test]
fn test_replay_entre_dois_emissores() {
    // O mesmo CTR de emissores diferentes não colide (janelas separadas).
    let mut a = BythosBuilder::new(10, SealConfig::software(0, TEST_KEY, 10));
    let mut b = BythosBuilder::new(20, SealConfig::software(0, TEST_KEY, 20));
    a.add_u8_field(0, 1).unwrap();
    b.add_u8_field(0, 1).unwrap();
    let mut bufa = [0u8; MAX_MESSAGE_SIZE];
    let mut bufb = [0u8; MAX_MESSAGE_SIZE];
    let na = a.build(0x11, BROADCAST_ADDR, &mut bufa).unwrap();
    let nb = b.build(0x11, BROADCAST_ADDR, &mut bufb).unwrap();
    let mut peers = PeerTable::new();
    assert!(validate_message(&bufa[..na], &TEST_KEY, &mut peers).is_ok());
    assert!(validate_message(&bufb[..nb], &TEST_KEY, &mut peers).is_ok());
    // Repetir qualquer uma: replay.
    assert_eq!(
        validate_message(&bufa[..na], &TEST_KEY, &mut peers),
        Err(ProtocolError::ReplayRejected)
    );
}

#[test]
fn test_crc_conhecido_123456789() {
    // Âncora CCITT partilhada com o C: nunca pode mudar sem mudar o fio.
    assert_eq!(calc_crc16(b"123456789"), 0x29B1);
}

// ============================================================================
// ANEL (decisão) E TÚNEL (cobs + vídeo)
// ============================================================================

#[test]
fn test_direcao_bg27_para_bg2() {
    // O caso canónico do projeto: 5 saltos pela frente, 25 por trás.
    assert_eq!(decide(27, 30, 2), Direction::After);
    assert_eq!(decide(2, 30, 27), Direction::Before);
}

#[test]
fn test_tunel_cobs_com_trama_real() {
    // Trama V4 com zeros na carga atravessa o tubo opaco intacta.
    let (buf, n) = telemetria(3);
    let mut enc = [0u8; TUNNEL_MAX];
    let m = cobs_encode(&buf[..n], &mut enc).unwrap();
    assert!(!enc[..m].contains(&0));
    let mut dec = [0u8; MAX_MESSAGE_SIZE];
    let k = cobs_decode(&enc[..m], &mut dec).unwrap();
    assert_eq!(&dec[..k], &buf[..n]);
    let mut peers = PeerTable::new();
    assert!(validate_message(&dec[..k], &TEST_KEY, &mut peers).is_ok());
}

#[test]
fn test_video_fragmentado_no_fio() {
    // Imagem de 70 B em 3 fragmentos (32+32+6) viaja em 3 tramas e remonta.
    let image = [0x5Au8; 70];
    let mut re = VideoReassembler::new();
    let chunks: [&[u8]; 3] = [&image[..32], &image[32..64], &image[64..]];
    for (i, payload) in chunks.iter().enumerate() {
        // Cada fragmento viaja como trama V4 normal (carga raw ≤ 32 B).
        let mut b = builder();
        b.add_u16_field(0, 42).unwrap(); // frame id
        b.add_u8_field(3, i as u8).unwrap(); // chunk id
        b.add_u8_field(11, 3).unwrap(); // total
        b.add_raw(BythosFieldId::VideoPayload as u8, payload)
            .unwrap();
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(0x16, BROADCAST_ADDR, &mut buf).unwrap();
        let mut peers = PeerTable::new();
        let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
        assert_eq!(view.header.msg_id, 0x16);
        // Extrai e ingere no remontador.
        let mut out = [BythosField::new(); 8];
        let k = parse_fields(&buf[11..view.fields_end], &mut out).unwrap();
        assert_eq!(k, 4);
        let frag = VideoFrag {
            frame_id: 42,
            chunk_id: i as u8,
            total: 3,
        };
        let done = re.push(frag, &out[3].data[..out[3].len as usize]);
        assert_eq!(done, i == 2);
    }
    let mut img = [0u8; 256];
    let n = re.assemble(&mut img).unwrap();
    assert_eq!(n, 70);
    assert_eq!(&img[..70], &image);
}
