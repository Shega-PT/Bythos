//! # Construtor Bythos — Mensagens Fluentes e Seladas (v4.0.0)
//!
//! O `BythosBuilder` é a forma recomendada de emitir: acumula campos tipados,
//! conta o `CTR` sozinho e sela com `TAG` ao construir. Quem usa o construtor
//! nunca toca em `SEC_HDR`, `TAG` ou CRC — esses bytes nascem no `build()`.
//!
//! ```rust
//! use bythos::protocol::builder::BythosBuilder;
//! use bythos::protocol::secure::SealConfig;
//! use bythos::protocol::types::MAX_MESSAGE_SIZE;
//!
//! let key = [0x42u8; 32];
//! let mut b = BythosBuilder::new(27, SealConfig::software(0, key, 27));
//! b.set_seq(7);
//! b.add_u8_field(0, 2).unwrap();      // estado do sistema
//! b.add_f32_field(6, 40.0).unwrap();  // latitude
//! let mut buf = [0u8; MAX_MESSAGE_SIZE];
//! let n = b.build(0x11, 0xFFFF, &mut buf).unwrap(); // telemetria em difusão
//! ```

use crate::protocol::codec::{build_message_sealed, ProtocolError, SoftwareTagger};
use crate::protocol::secure::SealConfig;
use crate::protocol::types::*;

// ============================================================================
// CONSTRUTOR
// ============================================================================

/// Construtor fluente de tramas Bythos V4 seladas.
///
/// Guarda o rascunho (campos + rota + sequência) e o selo (`SealConfig` com
/// chave de software + contador). Cada `build()` consome um `CTR` — construir
/// duas vezes sem mudar nada produz duas tramas distintas no fio, como deve ser
/// (repetir bytes seria repetir `CTR`, e a janela do recetor morderia).
pub struct BythosBuilder {
    /// Rascunho da mensagem (campos, origem, sequência, saltos).
    draft: BythosMessage,
    /// Selo: ranhura de chave + chave de software + próximo contador.
    seal: SealConfig,
}

impl BythosBuilder {
    /// Cria um construtor para o endereço `src` com o selo dado.
    ///
    /// O `seal.src` deve coincidir com `src` — o construtor não impõe (o fio
    /// é que manda), mas divergir é forjar origem própria, sem sentido.
    pub fn new(src: u16, seal: SealConfig) -> Self {
        let mut draft = BythosMessage::new();
        draft.src = src;
        Self { draft, seal }
    }

    /// Repõe campos e sequência; mantém origem, saltos e selo.
    ///
    /// Reutilizar o construtor entre emissões evita realocar o rascunho — e
    /// manter o selo garante que o `CTR` nunca anda para trás entre mensagens.
    pub fn reset(&mut self) {
        let src = self.draft.src;
        let hops = self.draft.hops;
        self.draft.clear();
        self.draft.src = src;
        self.draft.hops = hops;
    }

    /// Nº de campos acumulados.
    pub fn field_count(&self) -> u8 {
        self.draft.field_count
    }

    /// Define a sequência (diagnóstico; anti-replay usa o `CTR`, não isto).
    pub fn set_seq(&mut self, seq: u16) {
        self.draft.seq_num = seq;
    }

    /// Lê a sequência atual.
    pub fn get_seq(&self) -> u16 {
        self.draft.seq_num
    }

    /// Define os saltos restantes (por defeito `HOP_DEFAULT`).
    pub fn set_hops(&mut self, hops: u8) {
        self.draft.hops = hops;
    }

    /// Lê o endereço de origem configurado.
    pub fn get_src(&self) -> u16 {
        self.draft.src
    }

    /// Troca a ranhura de chave (rotação sem reconstruir o resto).
    pub fn set_key_id(&mut self, key_id: u8) {
        self.seal.key_id = key_id;
    }

    // ========================================================================
    // CAMPOS — CARGA BRUTA
    // ========================================================================

    /// Adiciona campo com identificador já codificado e carga bruta.
    ///
    /// Recusa (nunca trunca) se a mensagem estiver cheia ou a carga exceder
    /// `MAX_FIELD_DATA` — ver `BythosField::with_data` para o porquê.
    pub fn add_raw(&mut self, id: u8, data: &[u8]) -> Result<(), ProtocolError> {
        if self.draft.field_count as usize >= MAX_FIELDS {
            return Err(ProtocolError::TooManyFields);
        }
        match BythosField::with_data(id, data) {
            Some(field) => {
                self.draft.fields[self.draft.field_count as usize] = field;
                self.draft.field_count += 1;
                Ok(())
            }
            None => Err(ProtocolError::FieldDataTooLong),
        }
    }

    // ========================================================================
    // CAMPOS — TIPOS (codificam o identificador sozinhos)
    // ========================================================================

    /// Adiciona `f32` com identificador já codificado.
    pub fn add_f32(&mut self, id: u8, value: f32) -> Result<(), ProtocolError> {
        self.add_raw(id, &float_to_bytes(value))
    }

    /// Adiciona `f32` pelo id lógico (codifica `Float32` automaticamente).
    pub fn add_f32_field(&mut self, field_id: u8, value: f32) -> Result<(), ProtocolError> {
        self.add_f32(
            bythos_field_id_encode(BythosFieldType::Float32 as u8, field_id),
            value,
        )
    }

    /// Adiciona `f16` (meia precisão) pelo id lógico.
    ///
    /// Fecha a dívida V3: o tipo existia no fio mas sem construtor. Útil para
    /// telemetria densa (metade dos bytes, precisão de sobra para °C, % e m/s).
    pub fn add_f16_field(&mut self, field_id: u8, value: f32) -> Result<(), ProtocolError> {
        let half = f32_to_f16(value);
        self.add_raw(
            bythos_field_id_encode(BythosFieldType::Float16 as u8, field_id),
            &half.to_le_bytes(),
        )
    }

    /// Adiciona `i32` com identificador já codificado.
    pub fn add_i32(&mut self, id: u8, value: i32) -> Result<(), ProtocolError> {
        self.add_raw(id, &int32_to_bytes(value))
    }

    /// Adiciona `i32` pelo id lógico.
    pub fn add_i32_field(&mut self, field_id: u8, value: i32) -> Result<(), ProtocolError> {
        self.add_i32(
            bythos_field_id_encode(BythosFieldType::Int32 as u8, field_id),
            value,
        )
    }

    /// Adiciona `u32` com identificador já codificado.
    pub fn add_u32(&mut self, id: u8, value: u32) -> Result<(), ProtocolError> {
        self.add_raw(id, &uint32_to_bytes(value))
    }

    /// Adiciona `u32` pelo id lógico.
    pub fn add_u32_field(&mut self, field_id: u8, value: u32) -> Result<(), ProtocolError> {
        self.add_u32(
            bythos_field_id_encode(BythosFieldType::Uint32 as u8, field_id),
            value,
        )
    }

    /// Adiciona `u16` com identificador já codificado.
    pub fn add_u16(&mut self, id: u8, value: u16) -> Result<(), ProtocolError> {
        self.add_raw(id, &uint16_to_bytes(value))
    }

    /// Adiciona `u16` pelo id lógico.
    pub fn add_u16_field(&mut self, field_id: u8, value: u16) -> Result<(), ProtocolError> {
        self.add_u16(
            bythos_field_id_encode(BythosFieldType::Uint16 as u8, field_id),
            value,
        )
    }

    /// Adiciona `u8` com identificador já codificado.
    pub fn add_u8(&mut self, id: u8, value: u8) -> Result<(), ProtocolError> {
        self.add_raw(id, &[value])
    }

    /// Adiciona `u8` pelo id lógico.
    pub fn add_u8_field(&mut self, field_id: u8, value: u8) -> Result<(), ProtocolError> {
        self.add_u8(
            bythos_field_id_encode(BythosFieldType::Uint8 as u8, field_id),
            value,
        )
    }

    /// Adiciona booleano com identificador já codificado.
    pub fn add_bool(&mut self, id: u8, value: bool) -> Result<(), ProtocolError> {
        self.add_raw(id, &[u8::from(value)])
    }

    /// Adiciona booleano pelo id lógico.
    pub fn add_bool_field(&mut self, field_id: u8, value: bool) -> Result<(), ProtocolError> {
        self.add_bool(
            bythos_field_id_encode(BythosFieldType::Bool as u8, field_id),
            value,
        )
    }

    // ========================================================================
    // SELAGEM
    // ========================================================================

    /// Sela e serializa a trama para `dst`.
    ///
    /// Consome um `CTR` do selo (mesmo em erro de tampão? **não** — o contador
    /// só avança após selagem bem-sucedida, para um `BufferTooSmall` não queimar
    /// um contador e abrir buraco na janela do recetor).
    pub fn build(
        &mut self,
        msg_id: u8,
        dst: u16,
        buffer: &mut [u8],
    ) -> Result<usize, ProtocolError> {
        let ctr = self.seal.next_ctr & 0xFF_FFFF;
        let mut tagger = SoftwareTagger { key: self.seal.key };
        let n = build_message_sealed(
            &self.draft,
            msg_id,
            dst,
            self.seal.key_id,
            ctr,
            &mut tagger,
            buffer,
        )?;
        // Só avança depois de selar com sucesso (ver doc acima).
        self.seal.next_ctr = self.seal.next_ctr.wrapping_add(1) & 0xFF_FFFF;
        Ok(n)
    }

    /// Sinónimo de `build` (compatibilidade de leitura com a V3).
    pub fn serialize(
        &mut self,
        msg_id: u8,
        dst: u16,
        buffer: &mut [u8],
    ) -> Result<usize, ProtocolError> {
        self.build(msg_id, dst, buffer)
    }
}

impl Default for BythosBuilder {
    /// Construtor vazio (origem 0, selo de software zerado — só para testes).
    fn default() -> Self {
        Self::new(0, SealConfig::software(0, [0u8; 32], 0))
    }
}

/// Nome V3 do construtor (migração de uma release).
#[deprecated(since = "4.0.0", note = "usar `BythosBuilder`")]
pub type TLVBuilder = BythosBuilder;

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::secure::PeerTable;

    /// Construtor de teste: origem 6, chave fixa.
    fn tb() -> BythosBuilder {
        BythosBuilder::new(6, SealConfig::software(0, [0x42u8; 32], 6))
    }

    #[test]
    fn test_construtor_base() {
        let b = tb();
        assert_eq!(b.field_count(), 0);
        assert_eq!(b.get_src(), 6);
    }

    #[test]
    fn test_adiciona_tipos() {
        let mut b = tb();
        b.add_u8(0xC0, 0x02).unwrap();
        b.add_u16(0xA0, 0x1234).unwrap();
        b.add_u32(0x82, 0xDEADBEEF).unwrap();
        b.add_i32(0x60, -12345).unwrap();
        b.add_f32(0x30, 2.5).unwrap();
        b.add_bool(0xE0, true).unwrap();
        assert_eq!(b.field_count(), 6);
    }

    #[test]
    fn test_codificacao_automatica() {
        // Ids lógicos viram bytes do catálogo sem intervenção.
        let mut b = tb();
        b.add_u8_field(0, 0x02).unwrap();
        assert_eq!(b.draft.fields[0].id, 0xC0);
        b.add_f32_field(6, -33.9).unwrap();
        assert_eq!(b.draft.fields[1].id, 0x26);
        b.add_f16_field(4, 1.5).unwrap();
        assert_eq!(b.draft.fields[2].len, 2);
    }

    #[test]
    fn test_recusa_em_vez_de_truncar() {
        // 33 B: erro honesto (a V3 cortava em silêncio).
        let mut b = tb();
        assert_eq!(
            b.add_raw(0x00, &[0u8; 33]),
            Err(ProtocolError::FieldDataTooLong)
        );
        assert_eq!(b.field_count(), 0);
    }

    #[test]
    fn test_transbordo_de_campos() {
        let mut b = tb();
        for i in 0..MAX_FIELDS {
            b.add_u8(0xC0, i as u8).unwrap();
        }
        assert_eq!(b.field_count(), MAX_FIELDS as u8);
        assert_eq!(b.add_u8(0xC0, 0), Err(ProtocolError::TooManyFields));
    }

    #[test]
    fn test_sela_no_fio() {
        // Cabeçalho de 11 B + SEC_HDR com CTR + TAG + CRC.
        let mut b = tb();
        b.add_u8_field(0, 2).unwrap();
        b.set_seq(42);
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
        assert_eq!(buf[0], START_BYTE);
        assert_eq!(buf[1], BYTHOS_VERSION);
        assert_eq!(&buf[2..4], &6u16.to_le_bytes());
        assert_eq!(&buf[4..6], &BROADCAST_ADDR.to_le_bytes());
        assert_eq!(buf[6], 0x11);
        assert_eq!(&buf[7..9], &42u16.to_le_bytes());
        assert_eq!(buf[9], 1);
        assert_eq!(buf[10], HOP_DEFAULT);
        assert!(n > BYTHOS_OVERHEAD);
        assert!(n <= MAX_MESSAGE_SIZE);
        // E valida de ponta a ponta.
        let mut peers = PeerTable::new();
        let view =
            crate::protocol::codec::validate_message(&buf[..n], &[0x42u8; 32], &mut peers).unwrap();
        assert_eq!(view.header.src, 6);
        assert_eq!(view.header.ctr, 0);
    }

    #[test]
    fn test_ctr_avanca_por_construcao() {
        // Duas construções seguidas: CTRs 0 e 1 (nunca repete).
        let mut b = tb();
        b.add_u8_field(0, 2).unwrap();
        let mut buf1 = [0u8; MAX_MESSAGE_SIZE];
        let mut buf2 = [0u8; MAX_MESSAGE_SIZE];
        let n1 = b.build(0x11, BROADCAST_ADDR, &mut buf1).unwrap();
        let n2 = b.build(0x11, BROADCAST_ADDR, &mut buf2).unwrap();
        let mut peers = PeerTable::new();
        let v1 = crate::protocol::codec::validate_message(&buf1[..n1], &[0x42u8; 32], &mut peers)
            .unwrap();
        let v2 = crate::protocol::codec::validate_message(&buf2[..n2], &[0x42u8; 32], &mut peers)
            .unwrap();
        assert_eq!(v1.header.ctr, 0);
        assert_eq!(v2.header.ctr, 1);
    }

    #[test]
    fn test_tampao_pequeno_nao_queima_ctr() {
        // Falha de tampão antes de selar: o CTR fica para a próxima.
        let mut b = tb();
        b.add_u8_field(0, 2).unwrap();
        let mut tiny = [0u8; 5];
        assert_eq!(
            b.build(0x11, BROADCAST_ADDR, &mut tiny),
            Err(ProtocolError::BufferTooSmall)
        );
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
        let mut peers = PeerTable::new();
        let view =
            crate::protocol::codec::validate_message(&buf[..n], &[0x42u8; 32], &mut peers).unwrap();
        assert_eq!(view.header.ctr, 0);
    }

    #[test]
    fn test_mensagem_vazia() {
        // Só preâmbulo + reboque: 21 B.
        let mut b = tb();
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let n = b.build(0x10, BROADCAST_ADDR, &mut buf).unwrap();
        assert_eq!(n, BYTHOS_OVERHEAD);
        assert_eq!(buf[9], 0);
    }

    #[test]
    fn test_reset_mantem_selo() {
        let mut b = tb();
        b.add_u8_field(0, 2).unwrap();
        b.set_seq(9);
        b.reset();
        assert_eq!(b.field_count(), 0);
        assert_eq!(b.get_seq(), 0);
        assert_eq!(b.get_src(), 6);
        // O CTR não anda para trás com o reset.
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        b.add_u8_field(0, 2).unwrap();
        let n = b.build(0x11, BROADCAST_ADDR, &mut buf).unwrap();
        let mut peers = PeerTable::new();
        let view =
            crate::protocol::codec::validate_message(&buf[..n], &[0x42u8; 32], &mut peers).unwrap();
        assert_eq!(view.header.ctr, 0);
    }
}
