//! # FFI Analisador — ABI C do Bythos V4
//!
//! Espelho C do `Parser`: sem chaves, sem alocação além do próprio analisador.
//! O `TAG` verifica-se depois, sobre `bythos_parser_frame`, com
//! `bythos_validate` + chave + tabela de pares.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::parser::fsm::*;
use crate::protocol::types::*;

/// Cria um analisador (só `std`; sem `std` o firmware reserva-o estático e usa
/// `bythos_parser_init`).
#[cfg(feature = "std")]
#[no_mangle]
pub extern "C" fn bythos_parser_new() -> *mut Parser {
    Box::into_raw(Box::new(Parser::new()))
}

/// Liberta o analisador (nulo = sem efeito).
#[cfg(feature = "std")]
#[no_mangle]
pub extern "C" fn bythos_parser_free(parser: *mut Parser) {
    if !parser.is_null() {
        unsafe {
            let _ = Box::from_raw(parser);
        }
    }
}

/// Reinicia um analisador existente (caminho sem `std`).
#[no_mangle]
pub extern "C" fn bythos_parser_init(parser: *mut Parser) {
    if parser.is_null() {
        return;
    }
    unsafe { &mut *parser }.reset();
}

/// Alimenta um byte (devolve o código de erro; 0 = aceite).
#[no_mangle]
pub extern "C" fn bythos_parser_feed(parser: *mut Parser, byte: u8) -> u8 {
    if parser.is_null() {
        return ParserError::ErrStart as u8;
    }
    unsafe { &mut *parser }.feed(byte) as u8
}

/// `true` (1) se há trama completa.
#[no_mangle]
pub extern "C" fn bythos_parser_has_message(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return 0;
    }
    u8::from(unsafe { &*parser }.has_message())
}

/// Mensagem reconstruída (nulo sem trama completa).
///
/// Atenção ao tempo de vida: o ponteiro aponta para dentro do analisador —
/// `acknowledge`/`reset`/`feed` invalidam-no. Para guardar, copiar.
#[no_mangle]
pub extern "C" fn bythos_parser_get_message(parser: *const Parser) -> *const BythosMessage {
    if parser.is_null() {
        return core::ptr::null();
    }
    let parser = unsafe { &*parser };
    if parser.has_message() {
        parser.get_message() as *const BythosMessage
    } else {
        core::ptr::null()
    }
}

/// Bytes exatos da trama no fio + comprimento em `*len_out`.
///
/// É sobre estes bytes que corre `bythos_validate` (TAG + replay). Nulo sem
/// trama completa.
#[no_mangle]
pub extern "C" fn bythos_parser_frame(parser: *const Parser, len_out: *mut usize) -> *const u8 {
    if parser.is_null() || len_out.is_null() {
        return core::ptr::null();
    }
    let parser = unsafe { &*parser };
    match parser.completed_frame() {
        Some(frame) => {
            unsafe {
                *len_out = frame.len();
            }
            frame.as_ptr()
        }
        None => core::ptr::null(),
    }
}

/// Copia a mensagem (1 = ok).
#[no_mangle]
pub extern "C" fn bythos_parser_copy_message(
    parser: *const Parser,
    output: *mut BythosMessage,
) -> u8 {
    if parser.is_null() || output.is_null() {
        return 0;
    }
    u8::from(unsafe { &*parser }.copy_message(unsafe { &mut *output }))
}

/// Confirma o consumo (liberta para a próxima trama).
#[no_mangle]
pub extern "C" fn bythos_parser_acknowledge(parser: *mut Parser) {
    if parser.is_null() {
        return;
    }
    unsafe { &mut *parser }.acknowledge();
}

/// Reinício total.
#[no_mangle]
pub extern "C" fn bythos_parser_reset(parser: *mut Parser) {
    if parser.is_null() {
        return;
    }
    unsafe { &mut *parser }.reset();
}

/// Silêncio máximo entre bytes, em µs.
#[no_mangle]
pub extern "C" fn bythos_parser_set_frame_gap(parser: *mut Parser, micros: u32) {
    if parser.is_null() {
        return;
    }
    unsafe { &mut *parser }.set_max_frame_gap(micros);
}

/// `true` (1) se em timeout.
#[no_mangle]
pub extern "C" fn bythos_parser_timed_out(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return 0;
    }
    u8::from(unsafe { &*parser }.is_timed_out())
}

/// Último erro (código; 0 = ok).
#[no_mangle]
pub extern "C" fn bythos_parser_last_error(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return ParserError::ErrStart as u8;
    }
    unsafe { &*parser }.get_last_error() as u8
}

/// Estado atual (0–8).
#[no_mangle]
pub extern "C" fn bythos_parser_state(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return ParserState::WaitStart as u8;
    }
    unsafe { &*parser }.get_current_state() as u8
}

/// Tramas boas acumuladas.
#[no_mangle]
pub extern "C" fn bythos_parser_success_count(parser: *const Parser) -> u32 {
    if parser.is_null() {
        return 0;
    }
    unsafe { &*parser }.get_success_count()
}

/// Erros acumulados.
#[no_mangle]
pub extern "C" fn bythos_parser_error_count(parser: *const Parser) -> u32 {
    if parser.is_null() {
        return 0;
    }
    unsafe { &*parser }.get_error_count()
}

/// Interruptor de depuração (1 = liga).
#[no_mangle]
pub extern "C" fn bythos_parser_set_debug(parser: *mut Parser, enable: u8) {
    if parser.is_null() {
        return;
    }
    unsafe { &mut *parser }.set_debug(enable != 0);
}

/// Nome do estado (string estática; não libertar).
#[no_mangle]
pub extern "C" fn bythos_parser_state_str(state: u8) -> *const core::ffi::c_char {
    let s = match state {
        0 => "WAIT_START\0",
        1 => "WAIT_HEADER\0",
        2 => "WAIT_FIELD_ID\0",
        3 => "WAIT_FIELD_LEN\0",
        4 => "WAIT_FIELD_DATA\0",
        5 => "WAIT_SEC_HDR\0",
        6 => "WAIT_TAG\0",
        7 => "WAIT_CRC16_LO\0",
        8 => "WAIT_CRC16_HI\0",
        _ => "UNKNOWN\0",
    };
    s.as_ptr() as *const core::ffi::c_char
}

/// Nome do erro (string estática; não libertar).
#[no_mangle]
pub extern "C" fn bythos_parser_error_str(error: u8) -> *const core::ffi::c_char {
    let s = match error {
        0 => "OK\0",
        1 => "ERR_START\0",
        2 => "ERR_VERSION\0",
        3 => "ERR_MSGID\0",
        4 => "ERR_FIELD_COUNT\0",
        5 => "ERR_FIELD_ID\0",
        6 => "ERR_FIELD_LEN\0",
        7 => "ERR_CHECKSUM\0",
        8 => "ERR_TIMEOUT\0",
        9 => "ERR_OVERFLOW\0",
        10 => "ERR_HOPS\0",
        _ => "UNKNOWN\0",
    };
    s.as_ptr() as *const core::ffi::c_char
}
