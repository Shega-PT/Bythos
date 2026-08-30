// SPDX-License-Identifier: GPL-3.0-or-later

//! # Módulo Protocolo Bythos
//!
//! Implementação completa do protocolo de comunicação binário Bythos v3.0.0.
//!
//! Este módulo fornece:
//! - Definição de tipos e constantes (`types`) — CAN groups, FieldID com tipo, etc.
//! - Cálculo CRC-8/SMBUS (`crc8`) — legado, mantido por retrocompatibilidade
//! - Cálculo CRC-16/CCITT (`crc16`) — checksum padrão do Bythos v3.0.0
//! - Construção de mensagens Bythos (`builder`)
//! - Serialização/deserialização e validação (`codec`)
//! - Camada FFI para interoperação com C/C++ (`ffi`)

pub mod types;
pub mod crc8;
pub mod crc16;
pub mod builder;
pub mod codec;
pub mod ffi;
