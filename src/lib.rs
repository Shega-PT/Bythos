// SPDX-License-Identifier: GPL-3.0-or-later

//! # Bythos — Biblioteca Principal
//!
//! Esta crate implementa o protocolo de comunicação binário Bythos v3.0.0,
//! um protocolo TLV (Type-Length-Value) para sistemas embedded.
//!
//! ## Módulos
//!
//! - `protocol` — Protocolo Bythos: tipos, CRC8/CRC16, builder, codec e FFI
//! - `parser` — Parser FSM para reconstrução de mensagens Bythos
//!
//! ## Formato da Mensagem Bythos v3.0.0
//!
//! ```text
//! [START_BYTE][VERSION][NODE_ID][MSG_ID][SEQ_NUM(2)][TLV_COUNT]
//! [TLV_FIELDS...][SIGNATURE][CRC16(2)]
//! ```
//!
//! ## Uso em Rust
//!
//! ```rust
//! use bythos::protocol::builder::TLVBuilder;
//! use bythos::protocol::codec::validate_message;
//! use bythos::parser::fsm::Parser;
//!
//! // Construir mensagem
//! let mut builder = TLVBuilder::new(0x06, 0x42);
//! builder.add_u8_field(0, 2).unwrap();
//! builder.add_f32_field(0x10, 1.5).unwrap();
//! builder.set_seq(42);
//! let mut buffer = [0u8; 1098];
//! let size = builder.build(0x11, &mut buffer).unwrap();
//!
//! // Validar
//! assert!(validate_message(&buffer[..size]).is_ok());
//!
//! // Parse
//! let mut parser = Parser::new(0x42);
//! for &byte in &buffer[..size] {
//!     parser.feed(byte);
//! }
//! assert!(parser.has_message());
//! ```
//!
//! ## Uso em C/C++ (via FFI)
//!
//! Incluir `bythos_ffi.h` e linking com a library estática:
//!
//! ```c
//! #include "bythos_ffi.h"
//!
//! TLVMessage msg;
//! bythos_init(&msg, 0x06, 0x11);
//! bythos_tlv_add_u8(&msg, 0xC0, 2);
//! bythos_tlv_add_f32(&msg, 0x30, 1.5);
//!
//! uint8_t buffer[1098];
//! ssize_t size = bythos_build(&msg, 0x11, buffer, sizeof(buffer));
//! ```

#![cfg_attr(target_os = "none", no_std)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub mod protocol;
pub mod parser;
