//! # Módulo Protocolo Bythos (v4.0.0)
//!
//! O fio, por camadas de custo crescente:
//!
//! - `types` — única fonte de verdade: constantes, endereços, catálogo de campos
//! - `crc8` — legado V3, mantido para ler o parque antigo em migração
//! - `crc16` — deteção barata de corrupção acidental (antes do HMAC)
//! - `secure` — SHA-256 + HMAC + selo + anti-replay (coração da V4)
//! - `builder` — construção fluente e selada (`BythosBuilder`)
//! - `codec` — serialização, validação (estrutura e ponta-a-ponta) e análise
//! - `ffi` — ABI C estável (`bythos_*`)

pub mod builder;
pub mod codec;
pub mod crc16;
pub mod crc8;
pub mod ffi;
pub mod secure;
pub mod types;
