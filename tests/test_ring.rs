// SPDX-License-Identifier: GPL-3.0-or-later

//! # Testes de Anel — Enumeração, Encaminhamento e Corte (v4.0.0)
//!
//! Simula o anel em software puro: N nós com `Enumeration` + `RingMonitor`,
//! tramas reais no fio entre eles. Prova as três promessas do projeto:
//! numeração sem declarar endereços, sentido mais curto e sobrevivência a corte.

use bythos::parser::fsm::Parser;
use bythos::protocol::builder::BythosBuilder;
use bythos::protocol::codec::validate_message;
use bythos::protocol::secure::{PeerTable, SealConfig};
use bythos::protocol::types::*;
use bythos::ring::{decide, Direction, EnumPhase, Enumeration, NodeRole, RingMonitor};

const KEY: [u8; 32] = [0x42u8; 32];

/// Um nó simulado: identidade, geografia e monitor.
struct SimNode {
    en: Enumeration,
    mon: RingMonitor,
}

impl SimNode {
    fn node() -> Self {
        Self {
            en: Enumeration::new(NodeRole::Node),
            mon: RingMonitor::new(),
        }
    }

    fn root() -> Self {
        let mut n = Self {
            en: Enumeration::new(NodeRole::Root),
            mon: RingMonitor::new(),
        };
        // A raiz nasce com o relógio provado (gera-o ela mesma).
        n.mon.on_clock_return();
        n
    }
}

/// Emite trama V4 real de `src` para `dst` com CTR dado.
fn emit(src: u16, dst: u16, msg_id: u8, ctr: u32) -> ([u8; MAX_MESSAGE_SIZE], usize) {
    let mut seal = SealConfig::software(0, KEY, src);
    seal.next_ctr = ctr; // CTR exato para a prova (o construtor conta sozinho).
    let mut b = BythosBuilder::new(src, seal);
    b.add_u8_field(0, 1).unwrap();
    let mut buf = [0u8; MAX_MESSAGE_SIZE];
    let n = b.build(msg_id, dst, &mut buf).unwrap();
    (buf, n)
}

#[test]
fn test_anel_enumera_5_nos_sem_declarar_endereco() {
    // BG-0 injeta Hello{0}; cada nó soma 1; o Hello volta e fecha N=5.
    let mut root = SimNode::root();
    let mut nodes: Vec<SimNode> = (0..4).map(|_| SimNode::node()).collect();

    // Nenhum sabe quem é antes do Hello.
    assert!(nodes.iter().all(|n| !n.en.is_ready()));

    // Propaga: raiz emite 0, cada nó assume +1 e repete.
    let mut pos = 0u16;
    for node in nodes.iter_mut() {
        let next = node.en.on_hello(pos).unwrap();
        assert_eq!(next, pos + 1);
        pos = next;
    }
    assert_eq!(pos, 4);

    // O Hello regressa à raiz pela porta BEFORE: total = 4 + 1.
    assert_eq!(root.en.on_hello_return(pos), Some(5));

    // Count{5} difunde-se; todos ficam prontos com geografia completa.
    for node in nodes.iter_mut() {
        assert!(node.en.on_count(5));
        assert!(node.en.is_ready());
        assert_eq!(node.en.total, 5);
    }

    // Endereços sequenciais por ordem física, sem declarar nenhum.
    let addrs: Vec<u16> = nodes.iter().map(|n| n.en.addr).collect();
    assert_eq!(addrs, vec![1, 2, 3, 4]);
}

#[test]
fn test_bg27_para_bg2_viaja_pelo_curto_no_fio() {
    // Anel de 30, trama real BG-27 → BG-2: decide(27,30,2) = After, 5 saltos.
    // Simula salto a salto: cada ponte analisa, decide e decrementa HOPS.
    assert_eq!(decide(27, 30, 2), Direction::After);
    let (buf, n) = emit(27, 2, 0x11, 100);

    // Salta pela frente até BG-2: 27→28→29→0→1→2 (5 saltos).
    // Cada ponte analisa a trama do fio, decide o sentido e repete os bytes
    // (cut-through com HOPS-1; aqui modela-se a decisão + posição).
    let mut at = 27u16;
    for hop in 0..5 {
        let mut p = Parser::new();
        for &byte in &buf[..n] {
            p.feed(byte);
        }
        assert!(p.has_message(), "salto {hop} devia analisar");
        let msg = p.get_message().clone();
        assert_eq!(msg.dst, 2);
        assert!(msg.hops > 0, "saltos esgotados antes do destino");
        at = match decide(at, 30, msg.dst) {
            Direction::After => (at + 1) % 30,
            Direction::Before => (at + 30 - 1) % 30,
            Direction::Local => break,
        };
        if at == 2 {
            break;
        }
    }
    assert_eq!(at, 2, "devia chegar a BG-2 pelo sentido curto");

    // O destino valida ponta-a-ponta a trama original.
    let mut peers = PeerTable::new();
    let view = validate_message(&buf[..n], &KEY, &mut peers).unwrap();
    assert_eq!(view.header.src, 27);
    assert_eq!(view.header.dst, 2);
    assert_eq!(view.header.msg_id, 0x11);
}

#[test]
fn test_corte_muda_para_modo_linha() {
    // Anel de 30 fechado; cai a porta AFTER do BG-10: tudo pelo lado vivo.
    let mut mon = RingMonitor::new();
    mon.on_clock_return();
    assert_eq!(mon.route(27, 30, 2), Direction::After);
    mon.on_side_down(Direction::After);
    // BG-27 → BG-2 agora só pode ir por trás (lado vivo).
    assert_eq!(mon.route(27, 30, 2), Direction::Before);
    // Difusão e próprio continuam locais.
    assert_eq!(mon.route(27, 30, BROADCAST_ADDR), Direction::Local);
    assert_eq!(mon.route(27, 30, 27), Direction::Local);
}

#[test]
fn test_fases_de_enumeracao() {
    // Listening → Numbered → Ready, sem saltar etapas.
    let mut n = Enumeration::new(NodeRole::Node);
    assert_eq!(n.phase, EnumPhase::Listening);
    assert!(!n.is_ready());
    n.on_hello(9);
    assert_eq!((n.addr, n.phase), (10, EnumPhase::Numbered));
    assert!(!n.is_ready());
    assert!(n.on_count(30));
    assert_eq!(n.phase, EnumPhase::Ready);
    assert!(n.is_ready());
    // Count zero nunca fecha geografia.
    let mut m = Enumeration::new(NodeRole::Node);
    m.on_hello(0);
    assert!(!m.on_count(0));
    assert!(!m.is_ready());
}
