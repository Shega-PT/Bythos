//! # FFI Protocolo — ABI C Estável do Bythos V4
//!
//! Camada `extern "C"` para as pontes e o C standalone usarem o núcleo Rust sem
//! duplicar a criptografia: o `TAG` calcula-se aqui (software) ou no firmware
//! (elemento seguro), nunca em dois HMACs divergentes.
//!
//! ## Regras de validação (todas as funções)
//!
//! - Ponteiros nulos → erro, nunca pânico (a FFI não pode desenrolar para C).
//! - Gamas verificadas (`msg_id`, contagens, comprimentos) antes de tocar.
//! - A chave de selagem tem sempre 32 bytes; chave nula ou curta = erro.
//!
//! ## Memória
//!
//! `bythos_parser_new`/`bythos_peers_new` alocam com `Box` (só com `std`); o
//! chamador liberta com o `_free` correspondente. Todo o resto trabalha sobre
//! tampões do chamador, sem alocar.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::protocol::codec::*;
use crate::protocol::crc16::calc_crc16;
use crate::protocol::secure::{tagv4, verifyv4, PeerTable};
use crate::protocol::types::*;
use crate::ring::decide;

// ============================================================================
// CONSTANTES
// ============================================================================

/// Versão (string C terminada em nulo; o C não liberta — é estática).
pub const BYTHOS_VERSION_STR: &[u8] = b"4.0.0\0";

/// Tamanho da chave de selagem em bytes (HMAC-SHA256 usa 32 B).
pub const BYTHOS_KEY_SIZE: usize = 32;

// ============================================================================
// CRC
// ============================================================================

/// CRC-16/CCITT de uma fatia (`0xFFFF` em ponteiro nulo ou vazio).
#[no_mangle]
pub extern "C" fn bythos_calc_crc16(data: *const u8, len: usize) -> u16 {
    if data.is_null() || len == 0 {
        return 0xFFFF;
    }
    calc_crc16(unsafe { core::slice::from_raw_parts(data, len) })
}

/// CRC-8/SMBUS legado (só para ler o parque V3 em migração).
#[no_mangle]
pub extern "C" fn bythos_calc_crc8(data: *const u8, len: usize) -> u8 {
    if data.is_null() || len == 0 {
        return 0x00;
    }
    crate::protocol::crc8::calc_crc8(unsafe { core::slice::from_raw_parts(data, len) })
}

// ============================================================================
// SELO (TAG — HMAC-SHA256 truncado a 4 B)
// ============================================================================

/// Calcula o `TAG` V4 sobre `covered` com a chave de 32 B.
///
/// `covered` = do `INÍCIO` ao fim do `SEC_HDR`. O `ctr` entra vinculado (só os
/// 24 bits úteis). `tag_out` recebe 4 bytes. Devolve 0 em erro (nulos).
#[no_mangle]
pub extern "C" fn bythos_tag_compute(
    key: *const u8,
    ctr: u32,
    covered: *const u8,
    covered_len: usize,
    tag_out: *mut u8,
) -> i8 {
    if key.is_null() || covered.is_null() || tag_out.is_null() {
        return -1;
    }
    let key_slice = unsafe { core::slice::from_raw_parts(key, BYTHOS_KEY_SIZE) };
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key_slice);
    let data = unsafe { core::slice::from_raw_parts(covered, covered_len) };
    let tag = tagv4(&key_arr, ctr, data);
    unsafe {
        core::ptr::copy_nonoverlapping(tag.as_ptr(), tag_out, TAG_SIZE);
    }
    0
}

/// Verifica o `TAG` em tempo constante (0 = inválido/erro, 1 = válido).
#[no_mangle]
pub extern "C" fn bythos_tag_verify(
    key: *const u8,
    ctr: u32,
    covered: *const u8,
    covered_len: usize,
    tag: *const u8,
) -> u8 {
    if key.is_null() || covered.is_null() || tag.is_null() {
        return 0;
    }
    let key_slice = unsafe { core::slice::from_raw_parts(key, BYTHOS_KEY_SIZE) };
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key_slice);
    let data = unsafe { core::slice::from_raw_parts(covered, covered_len) };
    let mut tag_arr = [0u8; TAG_SIZE];
    unsafe {
        core::ptr::copy_nonoverlapping(tag, tag_arr.as_mut_ptr(), TAG_SIZE);
    }
    u8::from(verifyv4(&key_arr, ctr, data, &tag_arr))
}

/// Etiqueta XOR legada V3 (migração; produção usa `bythos_tag_*`).
#[no_mangle]
pub extern "C" fn bythos_legacy_tag_compute(key: u8, msg_id: u8, seq_lo: u8, seq_hi: u8) -> u8 {
    compute_legacy_tag(key, msg_id, seq_lo, seq_hi)
}

/// Verifica a etiqueta XOR legada (1 = válida).
#[no_mangle]
pub extern "C" fn bythos_legacy_tag_validate(
    tag: u8,
    key: u8,
    msg_id: u8,
    seq_lo: u8,
    seq_hi: u8,
) -> u8 {
    u8::from(validate_legacy_tag(tag, key, msg_id, seq_lo, seq_hi))
}

// ============================================================================
// IDENTIFICADOR DE CAMPO
// ============================================================================

/// Codifica `[TIPO:3][ID:5]`; `0xFF` se tipo > 7 ou id > 31.
#[no_mangle]
pub extern "C" fn bythos_field_id_encode(field_type: u8, field_id: u8) -> u8 {
    if field_type > 7 || field_id > 31 {
        return 0xFF;
    }
    bythos_field_id_encode(field_type, field_id)
}

/// Decodifica nos componentes (nulos = sem efeito).
#[no_mangle]
pub extern "C" fn bythos_field_id_decode(field_id: u8, type_out: *mut u8, id_out: *mut u8) {
    if type_out.is_null() || id_out.is_null() {
        return;
    }
    let (t, id) = crate::protocol::types::bythos_field_id_decode(field_id);
    unsafe {
        *type_out = t;
        *id_out = id;
    }
}

/// Valida o tipo embutido (1 = válido).
#[no_mangle]
pub extern "C" fn bythos_field_id_valid(field_id: u8) -> u8 {
    u8::from(is_valid_bythos_field_id(field_id))
}

// ============================================================================
// ENDEREÇAMENTO (substitui o CAN ID — sem 29 bits na V4)
// ============================================================================

/// Espécie de tráfego a partir do `MSG` (`0xFF` = desconhecido).
#[no_mangle]
pub extern "C" fn bythos_kind_of(msg_id: u8) -> u8 {
    match BythosKind::kind_of(msg_id) {
        Some(k) => k as u8,
        None => 0xFF,
    }
}

/// `true` (1) se o grupo funcional é conhecido.
#[no_mangle]
pub extern "C" fn bythos_group_valid(group: u8) -> u8 {
    u8::from(BythosGroup::from_u8(group).is_some())
}

/// `true` (1) se a espécie é a de segurança.
#[no_mangle]
pub extern "C" fn bythos_is_safety_kind(kind: u8) -> u8 {
    u8::from(kind == BythosKind::Safety as u8)
}

/// Sentido no anel: 0 = `AFTER` (frente), 1 = `BEFORE` (trás), 2 = local.
#[no_mangle]
pub extern "C" fn bythos_ring_decide(me: u16, total: u16, dst: u16) -> u8 {
    match decide(me, total, dst) {
        crate::ring::Direction::After => 0,
        crate::ring::Direction::Before => 1,
        crate::ring::Direction::Local => 2,
    }
}

// ============================================================================
// IDENTIFICADOR DO BYTHOS BUS (arbitragem — ver `BythosBusId`)
// ============================================================================

/// Empacota o identificador de arbitragem (`0` = metadados inválidos).
///
/// Layout `u32`: `[PRIO:3][GRUPO:4][ESPÉCIE:4][ORIGEM:16][RES:5]`. Valor menor =
/// maior prioridade (convenção CAN). A ponte ordena contenção com `<` direto.
///
/// Nota: o `0` também seria o ID da raiz em emergência total — na dúvida,
/// validar antes com `bythos_group_valid` + `bythos_kind_of` (a mesma
/// ambiguidade existia no CAN ID da V3; mantém-se por compatibilidade de estilo).
#[no_mangle]
pub extern "C" fn bythos_bus_id_make(priority: u8, group: u8, kind: u8, src: u16) -> u32 {
    make_bythos_bus_id(priority, group, kind, src).unwrap_or(0)
}

/// Prioridade do identificador (bits 31-29).
#[no_mangle]
pub extern "C" fn bythos_bus_id_priority(id: u32) -> u8 {
    BythosBusId(id).priority()
}

/// Grupo funcional (bits 28-25).
#[no_mangle]
pub extern "C" fn bythos_bus_id_group(id: u32) -> u8 {
    BythosBusId(id).group()
}

/// Espécie de tráfego (bits 24-21).
#[no_mangle]
pub extern "C" fn bythos_bus_id_kind(id: u32) -> u8 {
    BythosBusId(id).kind()
}

/// Origem no anel (bits 20-5).
#[no_mangle]
pub extern "C" fn bythos_bus_id_src(id: u32) -> u16 {
    BythosBusId(id).src()
}

/// Arbitragem: 1 se `a` ganha a `b` (menor valor ganha, como no CAN).
#[no_mangle]
pub extern "C" fn bythos_bus_id_wins(a: u32, b: u32) -> u8 {
    u8::from(BythosBusId(a).wins_over(BythosBusId(b)))
}

// ============================================================================
// TABELA DE PARES (anti-replay além da FFI)
// ============================================================================

/// Cria a tabela anti-replay (só `std`; sem `std` o firmware reserva-a estática).
#[cfg(feature = "std")]
#[no_mangle]
pub extern "C" fn bythos_peers_new() -> *mut PeerTable {
    Box::into_raw(Box::new(PeerTable::new()))
}

/// Liberta a tabela (nulo = sem efeito).
#[cfg(feature = "std")]
#[no_mangle]
pub extern "C" fn bythos_peers_free(peers: *mut PeerTable) {
    if !peers.is_null() {
        unsafe {
            let _ = Box::from_raw(peers);
        }
    }
}

// ============================================================================
// CONSTRUÇÃO E VALIDAÇÃO
// ============================================================================

/// Sela e serializa a mensagem (`-1` em qualquer erro).
///
/// `key` = 32 bytes (nunca nulo). `ctr` = contador monotónico do chamador (só os
/// 24 bits viajam). O `dst` do fio é o parâmetro (sobrepõe o rascunho).
#[no_mangle]
pub extern "C" fn bythos_build(
    msg: *const BythosMessage,
    msg_id: u8,
    dst: u16,
    key_id: u8,
    ctr: u32,
    key: *const u8,
    buffer: *mut u8,
    buffer_size: usize,
) -> isize {
    if msg.is_null() || buffer.is_null() || key.is_null() {
        return -1;
    }
    if !BythosMsgId::is_valid(msg_id) || buffer_size < BYTHOS_OVERHEAD {
        return -1;
    }
    let key_slice = unsafe { core::slice::from_raw_parts(key, BYTHOS_KEY_SIZE) };
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key_slice);
    let msg = unsafe { &*msg };
    let buf = unsafe { core::slice::from_raw_parts_mut(buffer, buffer_size) };
    match build_message(msg, msg_id, dst, key_id, ctr, &key_arr, buf) {
        Ok(n) => n as isize,
        Err(_) => -1,
    }
}

/// Só estrutura + CRC (`peek_header`): nº de campos ou `0xFF` em erro.
///
/// É o que a ponte usa para encaminhar sem chaves. Não consome `peers`.
#[no_mangle]
pub extern "C" fn bythos_peek(buffer: *const u8, length: usize) -> u8 {
    if buffer.is_null() || length < BYTHOS_OVERHEAD {
        return 0xFF;
    }
    let slice = unsafe { core::slice::from_raw_parts(buffer, length) };
    match peek_header(slice) {
        Ok(h) => h.field_count,
        Err(_) => 0xFF,
    }
}

/// Validação ponta-a-ponta (`TAG` + replay): nº de campos ou `0xFF`.
///
/// Exige chave de 32 B e tabela válida (criada com `bythos_peers_new` ou
/// estática do firmware). Repetir a mesma trama devolve `0xFF` à segunda.
#[no_mangle]
pub extern "C" fn bythos_validate(
    buffer: *const u8,
    length: usize,
    key: *const u8,
    peers: *mut PeerTable,
) -> u8 {
    if buffer.is_null() || key.is_null() || peers.is_null() || length < BYTHOS_OVERHEAD {
        return 0xFF;
    }
    let slice = unsafe { core::slice::from_raw_parts(buffer, length) };
    let key_slice = unsafe { core::slice::from_raw_parts(key, BYTHOS_KEY_SIZE) };
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key_slice);
    let peers = unsafe { &mut *peers };
    match validate_message(slice, &key_arr, peers) {
        Ok(v) => v.header.field_count,
        Err(_) => 0xFF,
    }
}

/// Desserializa campos (`*count`: entrada = capacidade, saída = lidos; erro = 0).
#[no_mangle]
pub extern "C" fn bythos_parse_fields(
    data: *const u8,
    length: usize,
    output: *mut BythosField,
    count: *mut usize,
) {
    if data.is_null() || output.is_null() || count.is_null() {
        return;
    }
    let capacity = unsafe { *count };
    if capacity == 0 {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, length) };
    let out = unsafe { core::slice::from_raw_parts_mut(output, capacity) };
    match parse_fields(slice, out) {
        Ok(n) => unsafe { *count = n },
        Err(_) => unsafe { *count = 0 },
    }
}

// ============================================================================
// CAMPOS NUMA MENSAGEM
// ============================================================================

/// Adiciona carga bruta (`-1` em erro; nunca trunca).
#[no_mangle]
pub extern "C" fn bythos_field_add(
    msg: *mut BythosMessage,
    id: u8,
    data: *const u8,
    len: u8,
) -> i8 {
    if msg.is_null() || data.is_null() {
        return -1;
    }
    if !is_valid_bythos_field_id(id) || len as usize > MAX_FIELD_DATA {
        return -1;
    }
    let msg = unsafe { &mut *msg };
    if msg.field_count as usize >= MAX_FIELDS {
        return -1;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, len as usize) };
    match BythosField::with_data(id, slice) {
        Some(f) => {
            msg.fields[msg.field_count as usize] = f;
            msg.field_count += 1;
            0
        }
        None => -1,
    }
}

/// Adiciona `f32` (`-1` em erro).
#[no_mangle]
pub extern "C" fn bythos_field_add_f32(msg: *mut BythosMessage, id: u8, value: f32) -> i8 {
    let bytes = float_to_bytes(value);
    bythos_field_add(msg, id, bytes.as_ptr(), 4)
}

/// Adiciona `f16` em meia precisão (`-1` em erro). Novo V4: sem isto o tipo
/// existia no fio mas era impossível de emitir.
#[no_mangle]
pub extern "C" fn bythos_field_add_f16(msg: *mut BythosMessage, id: u8, value: f32) -> i8 {
    let bytes = f32_to_f16(value).to_le_bytes();
    bythos_field_add(msg, id, bytes.as_ptr(), 2)
}

/// Adiciona `i32` (`-1` em erro).
#[no_mangle]
pub extern "C" fn bythos_field_add_i32(msg: *mut BythosMessage, id: u8, value: i32) -> i8 {
    let bytes = int32_to_bytes(value);
    bythos_field_add(msg, id, bytes.as_ptr(), 4)
}

/// Adiciona `u32` (`-1` em erro).
#[no_mangle]
pub extern "C" fn bythos_field_add_u32(msg: *mut BythosMessage, id: u8, value: u32) -> i8 {
    let bytes = uint32_to_bytes(value);
    bythos_field_add(msg, id, bytes.as_ptr(), 4)
}

/// Adiciona `u16` (`-1` em erro).
#[no_mangle]
pub extern "C" fn bythos_field_add_u16(msg: *mut BythosMessage, id: u8, value: u16) -> i8 {
    let bytes = uint16_to_bytes(value);
    bythos_field_add(msg, id, bytes.as_ptr(), 2)
}

/// Adiciona `u8` (`-1` em erro).
#[no_mangle]
pub extern "C" fn bythos_field_add_u8(msg: *mut BythosMessage, id: u8, value: u8) -> i8 {
    bythos_field_add(msg, id, &value as *const u8, 1)
}

/// Adiciona booleano (`-1` em erro). Novo V4: faltava na FFI.
#[no_mangle]
pub extern "C" fn bythos_field_add_bool(msg: *mut BythosMessage, id: u8, value: u8) -> i8 {
    let b = u8::from(value != 0);
    bythos_field_add(msg, id, &b as *const u8, 1)
}

// ============================================================================
// INICIALIZAÇÃO
// ============================================================================

/// Prepara o rascunho com origem e tipo (nulo ou tipo inválido = sem efeito).
#[no_mangle]
pub extern "C" fn bythos_init(msg: *mut BythosMessage, src: u16, msg_id: u8) {
    if msg.is_null() || !BythosMsgId::is_valid(msg_id) {
        return;
    }
    let msg = unsafe { &mut *msg };
    // Preenche campo a campo (não `memset`: o `fields` tem de ficar válido e o
    // preâmbulo correto — ver a divergência V3 documentada na migração).
    msg.clear();
    msg.src = src;
    msg.msg_id = msg_id;
}

/// Define o destino (`0xFFFF` = difusão).
#[no_mangle]
pub extern "C" fn bythos_set_dst(msg: *mut BythosMessage, dst: u16) {
    if msg.is_null() {
        return;
    }
    unsafe { &mut *msg }.dst = dst;
}

/// Define a sequência (diagnóstico).
#[no_mangle]
pub extern "C" fn bythos_set_seq(msg: *mut BythosMessage, seq: u16) {
    if msg.is_null() {
        return;
    }
    unsafe { &mut *msg }.seq_num = seq;
}

/// Define os saltos restantes.
#[no_mangle]
pub extern "C" fn bythos_set_hops(msg: *mut BythosMessage, hops: u8) {
    if msg.is_null() {
        return;
    }
    unsafe { &mut *msg }.hops = hops;
}

/// Repõe o rascunho (preâmbulo correto, resto a zero).
#[no_mangle]
pub extern "C" fn bythos_clear(msg: *mut BythosMessage) {
    if msg.is_null() {
        return;
    }
    unsafe { &mut *msg }.clear();
}

// ============================================================================
// CONVERSÕES (tudo LE, ordem do fio)
// ============================================================================

/// `f32` → 4 bytes LE.
#[no_mangle]
pub extern "C" fn bythos_f32_to_bytes(value: f32, bytes: *mut u8) {
    if bytes.is_null() {
        return;
    }
    let r = float_to_bytes(value);
    unsafe { core::ptr::copy_nonoverlapping(r.as_ptr(), bytes, 4) };
}

/// 4 bytes LE → `f32` (nulo = 0.0).
#[no_mangle]
pub extern "C" fn bythos_bytes_to_f32(bytes: *const u8) -> f32 {
    if bytes.is_null() {
        return 0.0;
    }
    let mut a = [0u8; 4];
    unsafe { core::ptr::copy_nonoverlapping(bytes, a.as_mut_ptr(), 4) };
    bytes_to_float(&a)
}

/// `f32` → meia precisão (bits, saturando). Novo V4.
#[no_mangle]
pub extern "C" fn bythos_f32_to_f16(value: f32) -> u16 {
    f32_to_f16(value)
}

/// Meia precisão → `f32`. Novo V4.
#[no_mangle]
pub extern "C" fn bythos_f16_to_f32(bits: u16) -> f32 {
    f16_to_f32(bits)
}

/// `i32` → 4 bytes LE.
#[no_mangle]
pub extern "C" fn bythos_i32_to_bytes(value: i32, bytes: *mut u8) {
    if bytes.is_null() {
        return;
    }
    let r = int32_to_bytes(value);
    unsafe { core::ptr::copy_nonoverlapping(r.as_ptr(), bytes, 4) };
}

/// 4 bytes LE → `i32` (nulo = 0).
#[no_mangle]
pub extern "C" fn bythos_bytes_to_i32(bytes: *const u8) -> i32 {
    if bytes.is_null() {
        return 0;
    }
    let mut a = [0u8; 4];
    unsafe { core::ptr::copy_nonoverlapping(bytes, a.as_mut_ptr(), 4) };
    bytes_to_int32(&a)
}

/// `u32` → 4 bytes LE.
#[no_mangle]
pub extern "C" fn bythos_u32_to_bytes(value: u32, bytes: *mut u8) {
    if bytes.is_null() {
        return;
    }
    let r = uint32_to_bytes(value);
    unsafe { core::ptr::copy_nonoverlapping(r.as_ptr(), bytes, 4) };
}

/// 4 bytes LE → `u32` (nulo = 0).
#[no_mangle]
pub extern "C" fn bythos_bytes_to_u32(bytes: *const u8) -> u32 {
    if bytes.is_null() {
        return 0;
    }
    let mut a = [0u8; 4];
    unsafe { core::ptr::copy_nonoverlapping(bytes, a.as_mut_ptr(), 4) };
    bytes_to_uint32(&a)
}

/// `u16` → 2 bytes LE.
#[no_mangle]
pub extern "C" fn bythos_u16_to_bytes(value: u16, bytes: *mut u8) {
    if bytes.is_null() {
        return;
    }
    let r = uint16_to_bytes(value);
    unsafe { core::ptr::copy_nonoverlapping(r.as_ptr(), bytes, 2) };
}

/// 2 bytes LE → `u16` (nulo = 0).
#[no_mangle]
pub extern "C" fn bythos_bytes_to_u16(bytes: *const u8) -> u16 {
    if bytes.is_null() {
        return 0;
    }
    let mut a = [0u8; 2];
    unsafe { core::ptr::copy_nonoverlapping(bytes, a.as_mut_ptr(), 2) };
    bytes_to_uint16(&a)
}

// ============================================================================
// VALIDAÇÃO LEVE E UTILITÁRIOS
// ============================================================================

/// `true` (1) se o tipo de mensagem é conhecido.
#[no_mangle]
pub extern "C" fn bythos_msg_id_valid(id: u8) -> u8 {
    u8::from(BythosMsgId::is_valid(id))
}

/// Prioridade efetiva (tipo inválido = `Low`, unificado com o núcleo — a V3 em
/// FFI devolvia 0 aqui e o núcleo 4; divergência fechada).
#[no_mangle]
pub extern "C" fn bythos_msg_priority(msg_id: u8, failsafe_active: u8) -> u8 {
    if !BythosMsgId::is_valid(msg_id) {
        return BythosPriority::Low as u8;
    }
    get_msg_priority(msg_id, failsafe_active != 0)
}

/// Versão como string estática (`"4.0.0"`; não libertar).
#[no_mangle]
pub extern "C" fn bythos_version() -> *const core::ffi::c_char {
    BYTHOS_VERSION_STR.as_ptr() as *const core::ffi::c_char
}

/// Sobrecarga do fio (21).
#[no_mangle]
pub extern "C" fn bythos_overhead() -> usize {
    BYTHOS_OVERHEAD
}

/// Trama máxima (1205).
#[no_mangle]
pub extern "C" fn bythos_max_message_size() -> usize {
    MAX_MESSAGE_SIZE
}

// ============================================================================
// TÚNEL COBS (rádios)
// ============================================================================

/// Codifica COBS (`-1` em erro; devolve bytes escritos).
#[no_mangle]
pub extern "C" fn bythos_cobs_encode(
    data: *const u8,
    len: usize,
    output: *mut u8,
    out_size: usize,
) -> isize {
    if data.is_null() || output.is_null() {
        return -1;
    }
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    let out = unsafe { core::slice::from_raw_parts_mut(output, out_size) };
    match crate::tunnel::cobs_encode(input, out) {
        Some(n) => n as isize,
        None => -1,
    }
}

/// Descodifica COBS (`-1` em erro; devolve bytes escritos).
#[no_mangle]
pub extern "C" fn bythos_cobs_decode(
    data: *const u8,
    len: usize,
    output: *mut u8,
    out_size: usize,
) -> isize {
    if data.is_null() || output.is_null() {
        return -1;
    }
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    let out = unsafe { core::slice::from_raw_parts_mut(output, out_size) };
    match crate::tunnel::cobs_decode(input, out) {
        Some(n) => n as isize,
        None => -1,
    }
}
