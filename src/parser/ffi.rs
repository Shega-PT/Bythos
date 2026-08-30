// SPDX-License-Identifier: GPL-3.0-or-later

//! # FFI Parser — Funções extern "C" para Interoperação C/C++ (Bythos v3.0.0)

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::parser::fsm::*;
use crate::protocol::types::*;

/// Alloc é necessário apenas em ambientes no_std.
/// Em ambientes std, Box já está disponível via biblioteca padrão.
#[cfg(not(feature = "std"))]
extern crate alloc;

/// Cria um novo parser FSM.
///
/// # Arguments
/// * `key` — Chave de assinatura (XOR key) para validação de mensagens.
///
/// # Returns
/// Ponteiro para o parser criado, ou NULL em erro.
#[cfg(feature = "std")]
#[no_mangle]
pub extern "C" fn bythos_parser_new(key: u8) -> *mut Parser {
    let parser = Parser::new(key);
    Box::into_raw(Box::new(parser))
}

/// Destrói um parser FSM e liberta a memória.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser a destruir.
#[cfg(feature = "std")]
#[no_mangle]
pub extern "C" fn bythos_parser_free(parser: *mut Parser) {
    if !parser.is_null() {
        unsafe {
            let _ = Box::from_raw(parser);
        }
    }
}

/// Inicializa um parser existente (no_std-friendly).
///
/// # Arguments
/// * `parser` — Ponteiro para o parser já alocado.
/// * `key` — Chave de assinatura.
#[no_mangle]
pub extern "C" fn bythos_parser_init(parser: *mut Parser, key: u8) {
    if parser.is_null() {
        return;
    }
    let p = unsafe { &mut *parser };
    p.reset();
    p.set_key(key);
}

/// Alimenta um byte ao parser.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
/// * `byte` — Byte a processar.
///
/// # Returns
/// Código de erro (0 = OK).
#[no_mangle]
pub extern "C" fn bythos_parser_feed(parser: *mut Parser, byte: u8) -> u8 {
    if parser.is_null() {
        return ParserError::ErrStart as u8;
    }
    let parser = unsafe { &mut *parser };
    parser.feed(byte) as u8
}

/// Retorna true (1) se uma mensagem completa está disponível.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
#[no_mangle]
pub extern "C" fn bythos_parser_has_message(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return 0;
    }
    let parser = unsafe { &*parser };
    if parser.has_message() { 1 } else { 0 }
}

/// Retorna ponteiro para a mensagem reconstruída.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
///
/// # Returns
/// Ponteiro para a mensagem Bythos, ou NULL se não houver mensagem.
#[no_mangle]
pub extern "C" fn bythos_parser_get_message(parser: *const Parser) -> *const TLVMessage {
    if parser.is_null() {
        return core::ptr::null();
    }
    let parser = unsafe { &*parser };
    if parser.has_message() {
        parser.get_message() as *const TLVMessage
    } else {
        core::ptr::null()
    }
}

/// Copia a mensagem reconstruída para um buffer de saída.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
/// * `output` — Ponteiro para o buffer de saída.
///
/// # Returns
/// 1 em sucesso, 0 em erro.
#[no_mangle]
pub extern "C" fn bythos_parser_copy_message(
    parser: *const Parser,
    output: *mut TLVMessage,
) -> u8 {
    if parser.is_null() || output.is_null() {
        return 0;
    }
    let parser = unsafe { &*parser };
    let output = unsafe { &mut *output };
    if parser.copy_message(output) { 1 } else { 0 }
}

/// Reconhece a mensagem processada.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
#[no_mangle]
pub extern "C" fn bythos_parser_acknowledge(parser: *mut Parser) {
    if parser.is_null() {
        return;
    }
    let parser = unsafe { &mut *parser };
    parser.acknowledge();
}

/// Reseta o parser para o estado inicial.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
#[no_mangle]
pub extern "C" fn bythos_parser_reset(parser: *mut Parser) {
    if parser.is_null() {
        return;
    }
    let parser = unsafe { &mut *parser };
    parser.reset();
}

/// Define o intervalo máximo entre bytes (timeout) em microssegundos.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
/// * `micros` — Timeout em microssegundos.
#[no_mangle]
pub extern "C" fn bythos_parser_set_frame_gap(parser: *mut Parser, micros: u32) {
    if parser.is_null() {
        return;
    }
    let parser = unsafe { &mut *parser };
    parser.set_max_frame_gap(micros);
}

/// Verifica se o parser excedeu o timeout.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
///
/// # Returns
/// 1 se em timeout, 0 caso contrário.
#[no_mangle]
pub extern "C" fn bythos_parser_timed_out(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return 0;
    }
    let parser = unsafe { &*parser };
    if parser.is_timed_out() { 1 } else { 0 }
}

/// Retorna o último erro registado.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
///
/// # Returns
/// Código do erro (0 = OK).
#[no_mangle]
pub extern "C" fn bythos_parser_last_error(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return ParserError::ErrStart as u8;
    }
    let parser = unsafe { &*parser };
    parser.get_last_error() as u8
}

/// Retorna o estado atual da FSM.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
///
/// # Returns
/// Código do estado (0-8).
#[no_mangle]
pub extern "C" fn bythos_parser_state(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return ParserState::WaitStart as u8;
    }
    let parser = unsafe { &*parser };
    parser.get_current_state() as u8
}

/// Retorna o número de mensagens processadas com sucesso.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
#[no_mangle]
pub extern "C" fn bythos_parser_success_count(parser: *const Parser) -> u32 {
    if parser.is_null() {
        return 0;
    }
    let parser = unsafe { &*parser };
    parser.get_success_count()
}

/// Retorna o número de erros de parsing.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
#[no_mangle]
pub extern "C" fn bythos_parser_error_count(parser: *const Parser) -> u32 {
    if parser.is_null() {
        return 0;
    }
    let parser = unsafe { &*parser };
    parser.get_error_count()
}

/// Define a função de timestamp do parser.
///
/// Permite ao código C fornecer a sua própria fonte de tempo
/// para deteção de timeout entre bytes.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
/// * `ts_fn` — Ponteiro para função `extern "C"` que retorna timestamp em microssegundos.
#[no_mangle]
pub extern "C" fn bythos_parser_set_timestamp_fn(parser: *mut Parser, ts_fn: TimestampFn) {
    if parser.is_null() {
        return;
    }
    let parser = unsafe { &mut *parser };
    parser.set_timestamp_fn(ts_fn);
}

/// Retorna a chave de assinatura do parser.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
///
/// # Returns
/// Chave de assinatura (0x00-0xFF).
#[no_mangle]
pub extern "C" fn bythos_parser_get_key(parser: *const Parser) -> u8 {
    if parser.is_null() {
        return 0;
    }
    let parser = unsafe { &*parser };
    parser.get_key()
}

/// Define a chave de assinatura do parser.
///
/// # Arguments
/// * `parser` — Ponteiro para o parser.
/// * `key` — Nova chave de assinatura.
#[no_mangle]
pub extern "C" fn bythos_parser_set_key(parser: *mut Parser, key: u8) {
    if parser.is_null() {
        return;
    }
    let parser = unsafe { &mut *parser };
    parser.set_key(key);
}

/// Converte um estado do parser para string legível.
///
/// # Arguments
/// * `state` — Código do estado.
///
/// # Returns
/// Ponteiro para string estática com o nome do estado.
#[no_mangle]
pub extern "C" fn bythos_parser_state_str(state: u8) -> *const core::ffi::c_char {
    // Nota: retorna ponteiro para string estática — não libertar
    let s = match state {
        0 => "WAIT_START\0",
        1 => "WAIT_HEADER\0",
        2 => "WAIT_TLV_COUNT\0",
        3 => "WAIT_TLV_ID\0",
        4 => "WAIT_TLV_LEN\0",
        5 => "WAIT_TLV_DATA\0",
        6 => "WAIT_SIGNATURE\0",
        7 => "WAIT_CRC16_LO\0",
        8 => "WAIT_CRC16_HI\0",
        _ => "UNKNOWN\0",
    };
    s.as_ptr() as *const core::ffi::c_char
}

/// Converte um código de erro do parser para string legível.
///
/// # Arguments
/// * `error` — Código do erro.
///
/// # Returns
/// Ponteiro para string estática com o nome do erro.
#[no_mangle]
pub extern "C" fn bythos_parser_error_str(error: u8) -> *const core::ffi::c_char {
    let s = match error {
        0 => "OK\0",
        1 => "ERR_START\0",
        2 => "ERR_VERSION\0",
        3 => "ERR_MSGID\0",
        4 => "ERR_TLV_COUNT\0",
        5 => "ERR_TLV_ID\0",
        6 => "ERR_TLV_LEN\0",
        7 => "ERR_CHECKSUM\0",
        8 => "ERR_SIGNATURE\0",
        9 => "ERR_TIMEOUT\0",
        _ => "UNKNOWN\0",
    };
    s.as_ptr() as *const core::ffi::c_char
}
