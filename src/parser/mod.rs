//! # Módulo Analisador (v4.0.0)
//!
//! - `fsm` — máquina de estados byte-a-byte (estrutura + CRC, sem chaves)
//! - `ffi` — ABI C do analisador (`bythos_parser_*`)

pub mod ffi;
pub mod fsm;
