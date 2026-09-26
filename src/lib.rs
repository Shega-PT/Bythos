//! # Bythos — Protocolo em Anel com Segurança por Hardware (v4.0.0)
//!
//! Biblioteca `no_std`-limpa do protocolo Bythos V4: campos tipados, selagem
//! HMAC com anti-replay, analisador byte-a-byte, enumeração e encaminhamento em
//! anel, e túnel opaco para rádios.
//!
//! ## Módulos
//!
//! - `protocol` — fio: tipos, CRC, selo (`secure`), construtor, codificador, FFI
//! - `parser` — analisador FSM byte-a-byte + FFI
//! - `ring` — geografia do anel: enumeração, sentido mais curto, supervisão
//! - `tunnel` — COBS para rádios + remontagem de vídeo fragmentado
//!
//! ## Trama (ver `docs/BYTHOS-SPECIFICATION.md`)
//!
//! ```text
//! [INÍCIO][VERSÃO=0x04][ORIGEM:2][DESTINO:2][MSG][SEQ:2][N_CAMPOS][SALTOS]
//! [CAMPOS...][SEC_HDR:4][TAG:4][CRC16:2]
//! ```
//!
//! ## Emitir (caminho feliz)
//!
//! ```rust
//! use bythos::protocol::builder::BythosBuilder;
//! use bythos::protocol::codec::validate_message;
//! use bythos::protocol::secure::{PeerTable, SealConfig};
//! use bythos::parser::fsm::Parser;
//! use bythos::protocol::types::MAX_MESSAGE_SIZE;
//!
//! // Origem 27, chave de software (testes; produção usa o elemento seguro).
//! let key = [0x42u8; 32];
//! let mut b = BythosBuilder::new(27, SealConfig::software(0, key, 27));
//! b.add_u8_field(0, 2).unwrap();
//! b.set_seq(7);
//! let mut buf = [0u8; MAX_MESSAGE_SIZE];
//! let n = b.build(0x11, 0xFFFF, &mut buf).unwrap(); // telemetria em difusão
//!
//! // Validar de ponta a ponta (destino final, com chave + janela).
//! let mut peers = PeerTable::new();
//! let view = validate_message(&buf[..n], &key, &mut peers).unwrap();
//! assert_eq!(view.header.src, 27);
//!
//! // Analisar byte-a-byte (ponte, sem chaves — só estrutura + CRC).
//! let mut p = Parser::new();
//! for &byte in &buf[..n] {
//!     p.feed(byte);
//! }
//! assert!(p.has_message());
//! ```
//!
//! ## Uso em C (via FFI)
//!
//! ```c
//! #include "bythos.h"
//!
//! BythosMessage msg;
//! uint8_t key[32] = {0x42}; /* chave de ensaio — produção: elemento seguro */
//! bythos_init(&msg, 27, 0x11);
//! bythos_set_dst(&msg, 0xFFFF);
//! bythos_field_add_u8(&msg, 0xC0, 2);
//!
//! uint8_t buf[BYTHOS_MAX_MESSAGE_SIZE];
//! BythosSeal seal = {0, {0}, 0};
//! bythos_ssize_t n = bythos_build(&msg, 0x11, &seal, key, buf, sizeof(buf));
//! ```

// `no_std` se a feature `std` estiver desligada (pontes e firmware).
// Nota: a V3 ligava isto a `target_os = "none"`, o que impedia testar `no_std`
// no PC. Aqui é por feature — `cargo test --no-default-features` prova o fio
// sem biblioteca padrão no próprio portátil.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Tratador de pânico mínimo só em firmware real (sem `std` e fora de testes).
// Nos testes, mesmo sem `std`, o harness do `cargo test` fornece o seu — dois
// tratadores no mesmo binário é erro de compilação (`E0152`), daí o `not(test)`.
#[cfg(all(not(feature = "std"), not(test)))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub mod parser;
pub mod protocol;
pub mod ring;
pub mod tunnel;
