//! # Codificador Bythos — Serialização, Validação e Análise (v4.0.0)
//!
//! Este módulo é o fio em código: transforma `BythosMessage` em bytes e bytes em
//! `BythosMessage`, com três níveis de verificação separados de propósito:
//!
//! 1. **Estrutura** (`peek_header`, `validate_structure`):preâmbulo, versão, gamas,
//!    comprimentos e CRC16. Não precisa de chaves — é o que uma ponte usa para
//!    **encaminhar** sem ser destinatária.
//! 2. **Autenticidade** (`validate_message`): `TAG` HMAC + `CTR` anti-replay, com
//!    chave e tabela de pares do chamador. Só o destino final paga este custo.
//! 3. **Campos** (`parse_fields`): cada campo é medido contra o tamanho canónico
//!    do seu tipo — um `f32` com `LEN != 4` é corrupção, mesmo cabendo nos 32 B.
//!
//! Separar 1 de 2 é decisão de arquitetura do anel: obrigar cada ponte a verificar
//! o `TAG` de tráfego alheio acordaria o elemento seguro milhares de vezes por
//! segundo sem necessidade. Encaminhar exige CRC válido; consumir exige `TAG` válido.

use crate::protocol::secure::{tagv4, verifyv4, PeerTable};
use crate::protocol::types::*;

// ============================================================================
// ERROS DO CODIFICADOR
// ============================================================================

/// Todos os modos de falha do fio, sem colapsar significados.
///
/// Regra: cada variante responde a "o que faço agora?" — `CorruptedData` =
/// deitar fora e ressincronizar; `InvalidTag` = contar e sinalizar (possível
/// ataque); `ReplayRejected` = contar em silêncio (retransmissão benigna ou
/// sonda); `BufferTooSmall` = chamar com tampão maior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// Tampão de saída menor que a trama.
    BufferTooSmall,
    /// Mais campos que `MAX_FIELDS`.
    TooManyFields,
    /// Primeiro byte diferente de `START_BYTE`.
    InvalidStartByte,
    /// Versão diferente de `BYTHOS_VERSION` (e sem `legacy-v3-compat`).
    InvalidVersion,
    /// Tipo de mensagem desconhecido.
    InvalidMsgId,
    /// Nº de campos acima de `MAX_FIELDS`.
    InvalidFieldCount,
    /// Comprimento de campo acima do permitido (32 normal, 128 vídeo raw).
    InvalidFieldLength,
    /// CRC16 não bate.
    InvalidChecksum,
    /// `TAG` não bate (chave errada ou trama forjada/adulterada).
    InvalidTag,
    /// `CTR` repetido ou fora da janela (replay).
    ReplayRejected,
    /// Saltos esgotados (`HOPS == 0` numa retransmissão).
    InvalidHops,
    /// Trama maior que `MAX_MESSAGE_SIZE` ou inconsistente internamente.
    CorruptedData,
    /// Campo com dados acima do limite (construção recusada, nunca truncada).
    FieldDataTooLong,
    /// Ranhura de chave desconhecida / modo legado recusado em produção.
    UnknownKey,
}

// ============================================================================
// VISTA DO CABEÇALHO (para encaminhar sem chaves)
// ============================================================================

/// Os 11 bytes do cabeçalho já interpretados.
///
/// É o que a ponte precisa para decidir o sentido no anel (`dst`, `hops`) sem
/// tocar em chaves. Obtém-se com `peek_header`, que exige CRC válido — um
/// cabeçalho com CRC mau nunca deve orientar encaminhamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderView {
    /// Endereço do emissor.
    pub src: u16,
    /// Endereço do destino (`0xFFFF` = difusão).
    pub dst: u16,
    /// Tipo de mensagem.
    pub msg_id: u8,
    /// Sequência do emissor (diagnóstico).
    pub seq: u16,
    /// Nº de campos.
    pub field_count: u8,
    /// Saltos restantes.
    pub hops: u8,
    /// Ranhura de chave (`SEC_HDR`).
    pub key_id: u8,
    /// Contador anti-replay (`SEC_HDR`, 24 bits).
    pub ctr: u32,
}

/// Lê e verifica a estrutura da trama **sem** verificar o `TAG`.
///
/// Diferença para `validate_message`: não precisa da chave nem da tabela de
/// pares — serve o encaminhador. Garante: preâmbulo, versão, gamas, coerência
/// de comprimentos (o `LEN` real casa com o tamanho total) e CRC16. O `TAG`
/// continua por verificar no destino final.
pub fn peek_header(buffer: &[u8]) -> Result<HeaderView, ProtocolError> {
    // Tamanho mínimo: cabeçalho(11) + segurança(4) + etiqueta(4) + crc(2) = 21.
    if buffer.len() < BYTHOS_OVERHEAD {
        return Err(ProtocolError::CorruptedData);
    }
    // Âncora de sincronização.
    if buffer[0] != START_BYTE {
        return Err(ProtocolError::InvalidStartByte);
    }
    // Versão: V4 por defeito; V3 só com a feature de migração.
    if buffer[1] != BYTHOS_VERSION {
        return Err(ProtocolError::InvalidVersion);
    }
    // Campos do cabeçalho (tudo LE, ver spec §3.1).
    let src = u16::from_le_bytes([buffer[2], buffer[3]]);
    let dst = u16::from_le_bytes([buffer[4], buffer[5]]);
    let msg_id = buffer[6];
    if !BythosMsgId::is_valid(msg_id) {
        return Err(ProtocolError::InvalidMsgId);
    }
    let seq = u16::from_le_bytes([buffer[7], buffer[8]]);
    let field_count = buffer[9];
    if field_count as usize > MAX_FIELDS {
        return Err(ProtocolError::InvalidFieldCount);
    }
    let hops = buffer[10];

    // Percorre os campos somando comprimentos reais — é aqui que tramas
    // truncadas ou com `LEN` mentiroso morrem, antes de qualquer cópia.
    let mut offset = BYTHOS_HEADER_SIZE;
    let mut video_big_seen = false;
    for _ in 0..field_count {
        if offset + FIELD_HEADER_SIZE > buffer.len() {
            return Err(ProtocolError::CorruptedData);
        }
        let fid = buffer[offset];
        let flen = buffer[offset + 1] as usize;
        // Teto genérico: 128 (teto do vídeo). O aperto por tipo vem a seguir e
        // em `parse_fields`; aqui garante-se só que a aritmética não enrola.
        if flen > MAX_FIELD_VIDEO_DATA {
            return Err(ProtocolError::InvalidFieldLength);
        }
        // Só carga vídeo `raw` pode passar dos 32 B — o resto é corrupção.
        if flen > MAX_FIELD_DATA {
            let (ftype, _) = bythos_field_id_decode(fid);
            let is_video_payload =
                ftype == BythosFieldType::Raw as u8 && fid == BythosFieldId::VideoPayload as u8;
            if !is_video_payload || video_big_seen {
                // Um só campo grande por trama (ver `MAX_MESSAGE_SIZE`).
                return Err(ProtocolError::InvalidFieldLength);
            }
            video_big_seen = true;
        }
        offset += FIELD_HEADER_SIZE + flen;
    }

    // Reboque: segurança(4) + etiqueta(4) + crc(2) têm de caber exatamente —
    // bytes a mais (ou a menos) após o fim calculado = trama malformada.
    let trailer = SEC_HDR_SIZE + TAG_SIZE + CRC16_SIZE;
    if offset + trailer != buffer.len() {
        return Err(ProtocolError::CorruptedData);
    }
    // CRC cobre tudo menos ele próprio (cabeçalho + campos + segurança + TAG).
    let crc_at = buffer.len() - CRC16_SIZE;
    let received = u16::from_le_bytes([buffer[crc_at], buffer[crc_at + 1]]);
    if crate::protocol::crc16::calc_crc16(&buffer[..crc_at]) != received {
        return Err(ProtocolError::InvalidChecksum);
    }
    // Cabeçalho de segurança (após os campos, antes da etiqueta).
    let sec_at = offset;
    let key_id = buffer[sec_at];
    let ctr = (buffer[sec_at + 1] as u32)
        | ((buffer[sec_at + 2] as u32) << 8)
        | ((buffer[sec_at + 3] as u32) << 16);

    Ok(HeaderView {
        src,
        dst,
        msg_id,
        seq,
        field_count,
        hops,
        key_id,
        ctr,
    })
}

// ============================================================================
// SERIALIZAÇÃO DE CAMPOS
// ============================================================================

/// Serializa um campo individual: `[ID][LEN][DADOS…]`.
///
/// Recusa dados acima de `MAX_FIELD_DATA` em vez de truncar — a V3 cortava em
/// silêncio (`min`), e telemetria cortada sem erro é o pior bug de campo.
pub fn build_field(id: u8, data: &[u8], output: &mut [u8]) -> Result<usize, ProtocolError> {
    if data.len() > MAX_FIELD_DATA {
        return Err(ProtocolError::FieldDataTooLong);
    }
    let required = FIELD_HEADER_SIZE + data.len();
    if output.len() < required {
        return Err(ProtocolError::BufferTooSmall);
    }
    output[0] = id;
    output[1] = data.len() as u8;
    output[FIELD_HEADER_SIZE..required].copy_from_slice(data);
    Ok(required)
}

/// Serializa um campo de carga vídeo (até 128 B).
///
/// O `id` viaja no fio (normalmente `VideoPayload = 0x00`); exigir o parâmetro
/// em vez de o forçar fecha o bug V3 onde a função ignorava o id do chamador.
/// Recusa acima de 128 B — fragmentar é trabalho do `crate::tunnel`, não daqui.
pub fn build_field_video(id: u8, data: &[u8], output: &mut [u8]) -> Result<usize, ProtocolError> {
    if data.len() > MAX_FIELD_VIDEO_DATA {
        return Err(ProtocolError::FieldDataTooLong);
    }
    let required = FIELD_HEADER_SIZE + data.len();
    if output.len() < required {
        return Err(ProtocolError::BufferTooSmall);
    }
    output[0] = id;
    output[1] = data.len() as u8;
    output[FIELD_HEADER_SIZE..required].copy_from_slice(data);
    Ok(required)
}

// ============================================================================
// SERIALIZAÇÃO DE MENSAGENS (caminho de software — chave em RAM)
// ============================================================================

/// Sela e serializa uma mensagem completa no fio V4.
///
/// Escreve cabeçalho(11) + campos + `SEC_HDR` + `TAG` + `CRC16`, por esta ordem —
/// o `TAG` cobre tudo antes dele e o CRC cobre tudo antes dele (incluindo o
/// `TAG`). O `ctr` é fornecido pelo chamador (normalmente `SealConfig::take_ctr`,
/// monotónico e persistente no elemento seguro).
///
/// # Quando usar
///
/// Testes, simulação e pontes sem chip. Produção com ATECC608 usa
/// `build_message_sealed`, onde o chip calcula o `TAG` sem a chave sair.
pub fn build_message(
    msg: &BythosMessage,
    msg_id: u8,
    dst: u16,
    key_id: u8,
    ctr: u32,
    key: &[u8; 32],
    buffer: &mut [u8],
) -> Result<usize, ProtocolError> {
    build_message_inner(
        msg,
        msg_id,
        dst,
        key_id,
        ctr,
        &mut SoftwareTagger { key: *key },
        buffer,
    )
}

/// Interface mínima de selagem: tudo o que o codificador precisa de um cofre.
///
/// Existe para o fio não depender do ATECC608: o software entrega
/// `SoftwareTagger`, o firmware entrega o driver I2C — o `build_message_sealed`
/// é idêntico nos dois casos.
pub trait Tagger {
    /// Etiqueta de 4 B sobre a região coberta (cabeçalho + campos + `SEC_HDR`).
    fn tag(&mut self, key_id: u8, ctr: u32, covered: &[u8]) -> [u8; TAG_SIZE];
}

/// Selador de software (chave em RAM — testes e pontes sem chip, nunca produção
/// com tráfego real; ver aviso em `SealConfig::software`).
pub struct SoftwareTagger {
    /// Chave de 32 B em RAM legível.
    pub key: [u8; 32],
}

impl Tagger for SoftwareTagger {
    fn tag(&mut self, _key_id: u8, ctr: u32, covered: &[u8]) -> [u8; TAG_SIZE] {
        tagv4(&self.key, ctr, covered)
    }
}

/// Sela com um cofre externo (ex. driver do ATECC608 sobre I2C).
///
/// A única diferença para `build_message` é quem calcula o `TAG`: aqui o chip,
/// sem a chave atravessar RAM. O fio produzido é bit-idêntico.
pub fn build_message_sealed<T: Tagger>(
    msg: &BythosMessage,
    msg_id: u8,
    dst: u16,
    key_id: u8,
    ctr: u32,
    tagger: &mut T,
    buffer: &mut [u8],
) -> Result<usize, ProtocolError> {
    build_message_inner(msg, msg_id, dst, key_id, ctr, tagger, buffer)
}

/// Núcleo comum de construção (as duas variantes só diferem na selagem).
fn build_message_inner<T: Tagger>(
    msg: &BythosMessage,
    msg_id: u8,
    dst: u16,
    key_id: u8,
    ctr: u32,
    tagger: &mut T,
    buffer: &mut [u8],
) -> Result<usize, ProtocolError> {
    // Tipo conhecido e contagem dentro do teto — falhar aqui é bug do chamador.
    if !BythosMsgId::is_valid(msg_id) {
        return Err(ProtocolError::InvalidMsgId);
    }
    if msg.field_count as usize > MAX_FIELDS {
        return Err(ProtocolError::TooManyFields);
    }
    // Soma os comprimentos reais (nunca confia em `len` sem medir o tampão).
    let mut fields_size = 0usize;
    let mut video_big_seen = false;
    for i in 0..msg.field_count as usize {
        let field = &msg.fields[i];
        let len = field.len as usize;
        if len > MAX_FIELD_VIDEO_DATA {
            return Err(ProtocolError::FieldDataTooLong);
        }
        if len > MAX_FIELD_DATA {
            // Regra do fio: só uma carga vídeo raw pode exceder 32 B.
            let is_video_payload = field.field_type() == Some(BythosFieldType::Raw)
                && field.id == BythosFieldId::VideoPayload as u8;
            if !is_video_payload || video_big_seen {
                return Err(ProtocolError::FieldDataTooLong);
            }
            video_big_seen = true;
        }
        // A carga tem de caber no tampão do campo (defesa contra struct
        // construída à mão com `len` mentiroso — o OOB crítico da V3 em C).
        if len > field.data.len() {
            return Err(ProtocolError::CorruptedData);
        }
        fields_size += FIELD_HEADER_SIZE + len;
    }
    let required = BYTHOS_HEADER_SIZE + fields_size + SEC_HDR_SIZE + TAG_SIZE + CRC16_SIZE;
    if required > MAX_MESSAGE_SIZE {
        return Err(ProtocolError::BufferTooSmall);
    }
    if buffer.len() < required {
        return Err(ProtocolError::BufferTooSmall);
    }

    // --- Cabeçalho (11 bytes, tudo LE) ---
    let mut offset = 0;
    buffer[offset] = START_BYTE;
    offset += 1;
    buffer[offset] = BYTHOS_VERSION;
    offset += 1;
    buffer[offset..offset + 2].copy_from_slice(&msg.src.to_le_bytes());
    offset += 2;
    buffer[offset..offset + 2].copy_from_slice(&dst.to_le_bytes());
    offset += 2;
    buffer[offset] = msg_id;
    offset += 1;
    buffer[offset..offset + 2].copy_from_slice(&msg.seq_num.to_le_bytes());
    offset += 2;
    buffer[offset] = msg.field_count;
    offset += 1;
    buffer[offset] = msg.hops;
    offset += 1;

    // --- Campos ---
    for i in 0..msg.field_count as usize {
        let field = &msg.fields[i];
        let len = field.len as usize;
        buffer[offset] = field.id;
        buffer[offset + 1] = field.len;
        buffer[offset + FIELD_HEADER_SIZE..offset + FIELD_HEADER_SIZE + len]
            .copy_from_slice(&field.data[..len]);
        offset += FIELD_HEADER_SIZE + len;
    }

    // --- Cabeçalho de segurança ---
    buffer[offset] = key_id;
    let ctr24 = ctr & 0xFF_FFFF;
    buffer[offset + 1] = (ctr24 & 0xFF) as u8;
    buffer[offset + 2] = ((ctr24 >> 8) & 0xFF) as u8;
    buffer[offset + 3] = ((ctr24 >> 16) & 0xFF) as u8;
    offset += SEC_HDR_SIZE;

    // --- Etiqueta (cobre tudo antes dela, incluindo o SEC_HDR) ---
    let tag = tagger.tag(key_id, ctr24, &buffer[..offset]);
    buffer[offset..offset + TAG_SIZE].copy_from_slice(&tag);
    offset += TAG_SIZE;

    // --- CRC16 (cobre tudo antes dele, incluindo o TAG) ---
    let crc = crate::protocol::crc16::calc_crc16(&buffer[..offset]);
    buffer[offset..offset + CRC16_SIZE].copy_from_slice(&crc.to_le_bytes());
    offset += CRC16_SIZE;

    Ok(offset)
}

// ============================================================================
// VALIDAÇÃO COMPLETA (destino final: estrutura + TAG + replay)
// ============================================================================

/// Resultado de uma trama validada de ponta a ponta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedView {
    /// Cabeçalho interpretado.
    pub header: HeaderView,
    /// Deslocamento onde começam os campos (sempre `BYTHOS_HEADER_SIZE`).
    pub fields_at: usize,
    /// Deslocamento do fim dos campos (= início do `SEC_HDR`).
    pub fields_end: usize,
}

/// Valida tudo: estrutura + CRC + `TAG` + anti-replay.
///
/// Ordem deliberada e barata-primeiro: preâmbulo → versão → gamas → comprimentos
/// → CRC → `TAG` → janela. Cada etapa evita a seguinte; o HMAC (a mais cara)
/// só corre sobre tramas estruturalmente sãs. `peers` atualiza-se como efeito
/// colateral — validar duas vezes a mesma trama conta como replay à segunda.
pub fn validate_message(
    buffer: &[u8],
    key: &[u8; 32],
    peers: &mut PeerTable,
) -> Result<ValidatedView, ProtocolError> {
    let header = peek_header(buffer)?;
    // Modo legado recusado em produção: sem `TAG` não há o que verificar.
    if header.key_id == crate::protocol::types::KEY_ID_LEGACY {
        return Err(ProtocolError::UnknownKey);
    }
    let fields_at = BYTHOS_HEADER_SIZE;
    let fields_end = buffer.len() - SEC_HDR_SIZE - TAG_SIZE - CRC16_SIZE;
    let tag_at = fields_end + SEC_HDR_SIZE;
    // Região coberta = do INÍCIO ao fim do SEC_HDR (o TAG cobre o CTR).
    let covered = &buffer[..fields_end + SEC_HDR_SIZE];
    let mut tag = [0u8; TAG_SIZE];
    tag.copy_from_slice(&buffer[tag_at..tag_at + TAG_SIZE]);
    if !verifyv4(key, header.ctr, covered, &tag) {
        return Err(ProtocolError::InvalidTag);
    }
    // Só depois do TAG válido é que o CTR entra na janela — aceitar CTR de
    // trama forjada envenenaria a janela e faria DoS às tramas boas seguintes.
    if !peers.accept(header.src, header.key_id, header.ctr) {
        return Err(ProtocolError::ReplayRejected);
    }
    Ok(ValidatedView {
        header,
        fields_at,
        fields_end,
    })
}

// ============================================================================
// ANÁLISE DE CAMPOS (com aperto por tipo)
// ============================================================================

/// Desserializa campos de uma fatia crua `[ID][LEN][DADOS…]`.
///
/// Para além dos limites genéricos, aperta `LEN` contra o tamanho canónico do
/// tipo (`f32` exige 4, `u8` exige 1…); só `Raw` é livre (32 B, ou 128 B se for
/// a carga vídeo `0x00`). É isto que impede um `LEN` "válido mas absurdo" de
/// atravessar para a aplicação.
pub fn parse_fields(data: &[u8], output: &mut [BythosField]) -> Result<usize, ProtocolError> {
    let mut offset = 0;
    let mut count = 0;
    while offset + FIELD_HEADER_SIZE <= data.len() && count < output.len() {
        let id = data[offset];
        let len = data[offset + 1] as usize;
        // Tipo conhecido (byte corrompido morre aqui, não no `copy_from_slice`).
        let (ftype_raw, _) = bythos_field_id_decode(id);
        let ftype = match BythosFieldType::from_u8(ftype_raw) {
            Some(t) => t,
            None => return Err(ProtocolError::InvalidFieldLength),
        };
        // Aperto por tipo.
        let ok = match ftype {
            BythosFieldType::Raw => {
                len <= MAX_FIELD_DATA
                    || (id == BythosFieldId::VideoPayload as u8 && len <= MAX_FIELD_VIDEO_DATA)
            }
            _ => len == ftype.default_size(),
        };
        if !ok {
            return Err(ProtocolError::InvalidFieldLength);
        }
        if offset + FIELD_HEADER_SIZE + len > data.len() {
            return Err(ProtocolError::CorruptedData);
        }
        let mut field = BythosField::new();
        field.id = id;
        field.len = len as u8;
        // Carga vídeo grande não cabe em `BythosField.data` (32 B): quem precisar
        // dela usa `parse_video_payload` — misturar seria truncar em silêncio.
        if len > field.data.len() {
            return Err(ProtocolError::FieldDataTooLong);
        }
        field.data[..len]
            .copy_from_slice(&data[offset + FIELD_HEADER_SIZE..offset + FIELD_HEADER_SIZE + len]);
        output[count] = field;
        offset += FIELD_HEADER_SIZE + len;
        count += 1;
    }
    Ok(count)
}

/// Extrai a carga de um campo vídeo grande (até 128 B) já localizado.
///
/// `field_bytes` = fatia exata `[ID][LEN][DADOS]` do campo. Devolve erro se não
/// for carga vídeo raw — chamar isto para um `f32` é bug do chamador.
pub fn parse_video_payload(field_bytes: &[u8]) -> Result<BythosVideoField, ProtocolError> {
    if field_bytes.len() < FIELD_HEADER_SIZE {
        return Err(ProtocolError::CorruptedData);
    }
    let id = field_bytes[0];
    let len = field_bytes[1] as usize;
    if id != BythosFieldId::VideoPayload as u8 {
        return Err(ProtocolError::InvalidFieldLength);
    }
    if len > MAX_FIELD_VIDEO_DATA || field_bytes.len() < FIELD_HEADER_SIZE + len {
        return Err(ProtocolError::CorruptedData);
    }
    let mut out = BythosVideoField::new();
    out.id = id;
    out.len = len as u8;
    out.data[..len].copy_from_slice(&field_bytes[FIELD_HEADER_SIZE..FIELD_HEADER_SIZE + len]);
    Ok(out)
}

/// Imprime um campo para depuração.
///
/// Só existe com `std`: nas pontes não há consola, e a função inteira
/// desaparece sem ela (nem o nome, para ninguém a chamar por engano).
#[cfg(feature = "std")]
pub fn print_field(field: &BythosField) {
    {
        let (ftype_raw, fid) = bythos_field_id_decode(field.id);
        let tname = match BythosFieldType::from_u8(ftype_raw) {
            Some(BythosFieldType::Raw) => "raw",
            Some(BythosFieldType::Float32) => "f32",
            Some(BythosFieldType::Float16) => "f16",
            Some(BythosFieldType::Int32) => "i32",
            Some(BythosFieldType::Uint32) => "u32",
            Some(BythosFieldType::Uint16) => "u16",
            Some(BythosFieldType::Uint8) => "u8",
            Some(BythosFieldType::Bool) => "bool",
            None => "???",
        };
        print!(
            "[Bythos] ID=0x{:02X} TIPO={}({}) LEN={}",
            field.id, tname, fid, field.len
        );
        if field.len > 0 {
            print!(" DADOS=[");
            for (i, b) in field.as_bytes().iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{b:02X}");
            }
            print!("]");
        }
        println!();
    }
}

/// Imprime uma mensagem para depuração.
///
/// Só existe com `std` (ver `print_field`).
#[cfg(feature = "std")]
pub fn print_message(msg: &BythosMessage) {
    {
        println!(
            "[Bythos] INÍCIO=0x{:02X} VER=0x{:02X} SRC={} DST={} MSG=0x{:02X} SEQ={} CAMPOS={} SALTOS={} CHAVE={} CTR={} CRC=0x{:04X}",
            msg.start_byte,
            msg.version,
            msg.src,
            msg.dst,
            msg.msg_id,
            msg.seq_num,
            msg.field_count,
            msg.hops,
            msg.key_id,
            msg.ctr,
            msg.checksum
        );
        for field in msg.valid_fields() {
            print_field(field);
        }
    }
}

// ============================================================================
// LEGADO V3 (migração apenas — atrás da feature `legacy-v3-compat`)
// ============================================================================

/// Valida estrutura de trama V3 (`VER=0x03`, cabeçalho 7 B, XOR 1 B).
///
/// Existe para ler o parque instalado durante a migração. Não verifica
/// anti-replay (a V3 não tem `CTR`) e o XOR não é segurança — o chamador deve
/// tratar o resultado como "legível, não confiável".
#[cfg(feature = "legacy-v3-compat")]
pub fn validate_v3_legacy(buffer: &[u8], key: u8) -> Result<u8, ProtocolError> {
    // Cabeçalho V3: início(1)+ver(1)+nó(1)+msg(1)+seq(2)+n(1) = 7; +xor(1)+crc(2).
    const V3_HEADER: usize = 7;
    const V3_OVERHEAD: usize = 10;
    if buffer.len() < V3_OVERHEAD {
        return Err(ProtocolError::CorruptedData);
    }
    if buffer[0] != START_BYTE {
        return Err(ProtocolError::InvalidStartByte);
    }
    if buffer[1] != 0x03 {
        return Err(ProtocolError::InvalidVersion);
    }
    let msg_id = buffer[3];
    if !BythosMsgId::is_valid(msg_id) {
        return Err(ProtocolError::InvalidMsgId);
    }
    let count = buffer[6];
    if count as usize > MAX_FIELDS {
        return Err(ProtocolError::InvalidFieldCount);
    }
    let mut offset = V3_HEADER;
    for _ in 0..count {
        if offset + FIELD_HEADER_SIZE > buffer.len() {
            return Err(ProtocolError::CorruptedData);
        }
        let flen = buffer[offset + 1] as usize;
        if flen > MAX_FIELD_DATA {
            return Err(ProtocolError::FieldDataTooLong);
        }
        offset += FIELD_HEADER_SIZE + flen;
    }
    if offset + 3 != buffer.len() {
        return Err(ProtocolError::CorruptedData);
    }
    let crc_at = buffer.len() - CRC16_SIZE;
    let received = u16::from_le_bytes([buffer[crc_at], buffer[crc_at + 1]]);
    if crate::protocol::crc16::calc_crc16(&buffer[..crc_at]) != received {
        return Err(ProtocolError::InvalidChecksum);
    }
    let tag = buffer[crc_at - 1];
    if !validate_legacy_tag(tag, key, msg_id, buffer[4], buffer[5]) {
        return Err(ProtocolError::InvalidTag);
    }
    Ok(count)
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::builder::BythosBuilder;
    use crate::protocol::secure::{SealConfig, SoftwareVault};

    /// Chave fixa de teste — vetores determinísticos (âncora de interop com o C).
    const TEST_KEY: [u8; 32] = [0x42u8; 32];

    /// Constrói mensagem selada de teste (origem 6, destino difusão).
    fn sealed(seq: u16, ctr: u32, fields: &[(u8, &[u8])]) -> ([u8; MAX_MESSAGE_SIZE], usize) {
        let mut msg = BythosMessage::with_route(6, BROADCAST_ADDR, 0x11);
        msg.seq_num = seq;
        for (id, data) in fields {
            let f = BythosField::with_data(*id, data).unwrap();
            msg.fields[msg.field_count as usize] = f;
            msg.field_count += 1;
        }
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = build_message(&msg, 0x11, BROADCAST_ADDR, 0, ctr, &TEST_KEY, &mut buf).unwrap();
        (buf, n)
    }

    #[test]
    fn test_cabecalho_v4_no_fio() {
        // Posições exatas de cada byte do cabeçalho de 11 B.
        let (buf, n) = sealed(0x1234, 7, &[(0xC0, &[0x02])]);
        assert_eq!(buf[0], START_BYTE);
        assert_eq!(buf[1], BYTHOS_VERSION);
        assert_eq!(&buf[2..4], &6u16.to_le_bytes());
        assert_eq!(&buf[4..6], &BROADCAST_ADDR.to_le_bytes());
        assert_eq!(buf[6], 0x11);
        assert_eq!(&buf[7..9], &0x1234u16.to_le_bytes());
        assert_eq!(buf[9], 1);
        assert_eq!(buf[10], HOP_DEFAULT);
        // Reboque: SEC_HDR + TAG + CRC no fim exato.
        assert_eq!(buf[n - 10], 0); // KEY_ID
        assert_eq!(&buf[n - 9..n - 6], &[7, 0, 0]); // CTR=7 LE 24
        assert!(n > BYTHOS_OVERHEAD);
    }

    #[test]
    fn test_ida_volta_estrutura_tag_replay() {
        // Caminho feliz completo: estrutura → TAG → janela.
        let (buf, n) = sealed(1, 41, &[(0xC0, &[0x02]), (0x30, &float_to_bytes(1.5))]);
        let mut peers = PeerTable::new();
        let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
        assert_eq!(view.header.src, 6);
        assert_eq!(view.header.field_count, 2);
        assert_eq!(view.header.ctr, 41);
        // Repetir a mesma trama = replay (a janela morde à segunda).
        assert_eq!(
            validate_message(&buf[..n], &TEST_KEY, &mut peers),
            Err(ProtocolError::ReplayRejected)
        );
    }

    #[test]
    fn test_tag_errada_chumba_antes_da_janela() {
        // 1 bit no TAG parte o CRC primeiro (o CRC cobre o TAG): barato-primeiro.
        // É a ordem deliberada — o HMAC só corre sobre tramas íntegras.
        let (mut buf, n) = sealed(1, 50, &[(0xC0, &[0x02])]);
        buf[n - 3] ^= 0x01;
        let mut peers = PeerTable::new();
        assert_eq!(
            validate_message(&buf[..n], &TEST_KEY, &mut peers),
            Err(ProtocolError::InvalidChecksum)
        );
        // Chave errada (TAG íntegro, HMAC divergente) = InvalidTag puro.
        let (buf2, n2) = sealed(1, 51, &[(0xC0, &[0x02])]);
        assert_eq!(
            validate_message(&buf2[..n2], &[0x43u8; 32], &mut peers),
            Err(ProtocolError::InvalidTag)
        );
    }

    #[test]
    fn test_crc_errado_chumba_antes_do_tag() {
        // Barato-primeiro: CRC mau nem chega ao HMAC.
        let (mut buf, n) = sealed(1, 60, &[(0xC0, &[0x02])]);
        buf[11] ^= 0xFF; // Dentro dos campos.
        let mut peers = PeerTable::new();
        assert_eq!(
            validate_message(&buf[..n], &TEST_KEY, &mut peers),
            Err(ProtocolError::InvalidChecksum)
        );
    }

    #[test]
    fn test_versao_e_preambulo() {
        let (mut buf, n) = sealed(1, 61, &[]);
        buf[0] = 0x00;
        let mut peers = PeerTable::new();
        assert_eq!(
            validate_message(&buf[..n], &TEST_KEY, &mut peers),
            Err(ProtocolError::InvalidStartByte)
        );
        let (mut buf, n) = sealed(1, 62, &[]);
        buf[1] = 0x03; // V3 sem feature = versão inválida.
        assert_eq!(
            validate_message(&buf[..n], &TEST_KEY, &mut peers),
            Err(ProtocolError::InvalidVersion)
        );
    }

    #[test]
    fn test_comprimentos_mentirosos() {
        // LEN=100 num campo u8: recusado na caminhada, sem ler fora.
        let (mut buf, n) = sealed(1, 63, &[(0xC0, &[0x02])]);
        buf[12] = 100; // LEN do primeiro campo.
        let mut peers = PeerTable::new();
        assert_eq!(
            validate_message(&buf[..n], &TEST_KEY, &mut peers),
            Err(ProtocolError::InvalidFieldLength)
        );
        // Bytes a mais no fim: malformada.
        let (buf, n) = sealed(1, 64, &[(0xC0, &[0x02])]);
        let mut peers = PeerTable::new();
        assert_eq!(
            validate_message(&buf[..n + 1], &TEST_KEY, &mut peers),
            Err(ProtocolError::CorruptedData)
        );
    }

    #[test]
    fn test_parse_fields_aperta_por_tipo() {
        // f32 com LEN!=4 morre; u8 com LEN!=1 morre; raw livre até 32.
        let mut out = [BythosField::new(); 8];
        assert!(parse_fields(&[0x30, 0x03, 0x01, 0x02, 0x03], &mut out).is_err());
        assert!(parse_fields(&[0xC0, 0x02, 0x01, 0x02], &mut out).is_err());
        assert_eq!(parse_fields(&[0xC0, 0x01, 0x02], &mut out).unwrap(), 1);
        assert_eq!(out[0].id, 0xC0);
        // Vazio = zero campos, sem erro.
        assert_eq!(parse_fields(&[], &mut out).unwrap(), 0);
    }

    #[test]
    fn test_campo_video_grande_ponta_a_ponta() {
        // 100 B de carga vídeo atravessam serializar→extrair sem truncar.
        // (Na mensagem completa, a carga vídeo viaja fragmentada pelo túnel —
        // ver `crate::tunnel`; aqui testa-se o átomo de 128 B.)
        let payload = [0x5Au8; 100];
        let mut wire = [0u8; 132];
        let n = build_field_video(BythosFieldId::VideoPayload as u8, &payload, &mut wire).unwrap();
        assert_eq!(n, 102);
        assert_eq!(wire[0], 0x00);
        assert_eq!(wire[1], 100);
        let v = parse_video_payload(&wire[..n]).unwrap();
        assert_eq!(v.len, 100);
        assert_eq!(v.data[99], 0x5A);
        // Acima de 128 recusa.
        assert_eq!(
            build_field_video(0x00, &[0u8; 129], &mut wire),
            Err(ProtocolError::FieldDataTooLong)
        );
    }

    #[test]
    fn test_construtor_recusa_duas_cargas_grandes() {
        // Duas cargas >32 B na mesma trama excedem o modelo → erro, nunca OOB.
        let mut msg = BythosMessage::with_route(6, BROADCAST_ADDR, 0x16);
        for i in 0..2 {
            msg.fields[i] = BythosField {
                id: BythosFieldId::VideoPayload as u8,
                len: 100,
                data: [0u8; MAX_FIELD_DATA],
            };
        }
        msg.field_count = 2;
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        // `len`=100 não cabe em `data` de 32 → CorruptedData honesto.
        assert!(build_message(&msg, 0x16, BROADCAST_ADDR, 0, 1, &TEST_KEY, &mut buf).is_err());
    }

    #[test]
    fn test_builder_ida_volta_v4() {
        // O construtor de alto nível fala o fio V4 fluentemente.
        let mut b = BythosBuilder::new(27, SealConfig::software(0, TEST_KEY, 27));
        b.set_seq(7);
        b.add_u8_field(0, 2).unwrap();
        b.add_f32_field(6, 40.0).unwrap();
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
        let mut peers = PeerTable::new();
        let view = validate_message(&buf[..n], &TEST_KEY, &mut peers).unwrap();
        assert_eq!(view.header.src, 27);
        assert_eq!(view.header.field_count, 2);
        // O CTR avançou sozinho (o construtor conta por nós).
        let n2 = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
        let view2 = validate_message(&buf[..n2], &TEST_KEY, &mut peers).unwrap();
        assert_eq!(view2.header.ctr, view.header.ctr + 1);
    }

    #[test]
    fn test_vetor_dourado_v4() {
        // Âncora de interop Rust↔C: estes bytes são o contrato. Se o C produzir
        // outra coisa com a mesma chave/ctr, um dos lados está errado.
        let mut msg = BythosMessage::with_route(0x0006, 0xFFFF, 0x11);
        msg.seq_num = 1;
        msg.hops = 32;
        msg.fields[0] = BythosField::with_data(0xC0, &[0x02]).unwrap();
        msg.field_count = 1;
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = build_message(&msg, 0x11, 0xFFFF, 0, 41, &TEST_KEY, &mut buf).unwrap();
        assert_eq!(n, 11 + 3 + 10);
        assert_eq!(
            &buf[..14],
            &[0xAA, 0x04, 0x06, 0x00, 0xFF, 0xFF, 0x11, 0x01, 0x00, 0x01, 0x20, 0xC0, 0x01, 0x02]
        );
        assert_eq!(&buf[14..18], &[0x00, 0x29, 0x00, 0x00]); // SEC_HDR: key 0, ctr 41
                                                             // TAG e CRC ficam registados aqui como regressão criptográfica:
                                                             // mudam SOMENTE se o algoritmo mudar (e aí muda a versão do fio).
                                                             // Vetor dourado Rust↔C (gerado 2026-09-26, HMAC-SHA256-32):
        assert_eq!(&buf[18..22], &[0x82, 0x23, 0x4B, 0xEF]);
        let tag = &buf[18..22];
        let crc = u16::from_le_bytes([buf[22], buf[23]]);
        assert_eq!(crc, crate::protocol::crc16::calc_crc16(&buf[..22]));
        // Valor congelado do TAG para esta entrada (HMAC-SHA256-32).
        let expected_tag = tagv4(&TEST_KEY, 41, &buf[..18]);
        assert_eq!(tag, &expected_tag);
        // E valida de ponta a ponta com o cofre de software.
        let mut vault = SoftwareVault::new();
        vault.provision(0, TEST_KEY);
        let mut peers = PeerTable::new();
        assert!(validate_message(&buf[..n], &TEST_KEY, &mut peers).is_ok());
    }

    /// Migração: trama V3 construída à mão valida atrás da feature legada.
    ///
    /// Prova que o parque instalado continua legível durante a transição —
    /// sem esta feature, a mesma trama morre em `InvalidVersion`.
    #[cfg(feature = "legacy-v3-compat")]
    #[test]
    fn test_legado_v3_le_na_migracao() {
        // Trama V3 mínima: [AA][03][NODE][MSG][SEQ2][N=1][C0 01 02][SIG][CRC2].
        let mut wire = [0u8; 13];
        wire[0] = START_BYTE;
        wire[1] = 0x03;
        wire[2] = 0x06;
        wire[3] = 0x11;
        wire[4] = 0x2A;
        wire[5] = 0x00;
        wire[6] = 0x01;
        wire[7] = 0xC0;
        wire[8] = 0x01;
        wire[9] = 0x02;
        wire[10] = compute_legacy_tag(0x42, 0x11, 0x2A, 0x00);
        let crc = crate::protocol::crc16::calc_crc16(&wire[..11]);
        wire[11..13].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(validate_v3_legacy(&wire, 0x42), Ok(1));
        assert_eq!(
            validate_v3_legacy(&wire, 0x43),
            Err(ProtocolError::InvalidTag)
        );
        // …mas o caminho V4 recusa-a sempre (curta demais para V4 sequer).
        let mut peers = PeerTable::new();
        assert_eq!(
            validate_message(&wire, &TEST_KEY, &mut peers),
            Err(ProtocolError::CorruptedData)
        );
    }
}
