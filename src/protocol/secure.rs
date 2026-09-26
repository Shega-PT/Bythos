//! # Selo Bythos — Autenticação e Anti-Repetição (v4.0.0)
//!
//! Este módulo responde à pergunta "esta trama veio mesmo de quem diz que veio,
//! e não é uma repetição?". Duas peças:
//!
//! 1. **`TAG` (autenticidade).** `trunc32(HMAC-SHA256(chave, trama))`, 4 bytes.
//!    O HMAC vive aqui em software puro (sem dependências, `no_std`-limpo) para
//!    que qualquer alvo compile; em produção a conta faz-se **dentro** do
//!    ATECC608 através do `SecureElement` — a chave nunca sai do chip. O caminho
//!    de software existe para testes, simulação e pontes sem chip (com aviso).
//! 2. **`CTR` + janela (frescura).** Contador monotónico de 24 bits por emissor;
//!    o recetor só aceita `CTR` estritamente maior que o último visto, com janela
//!    de 64 para absorver reordenação no anel. `SEQ u16` continua no fio mas é
//!    diagnóstico — a segurança não depende dele.
//!
//! Porquê HMAC-SHA256 truncado e não o XOR da V3? O XOR com chave de 1 byte
//! cai com 256 tentativas; o HMAC-32 exige ~2³² tentativas em linha, e o anel
//! denuncia o atacante (picos de `TAG` inválido) muito antes disso. Ver
//! `docs/THREAT-MODEL.md`.

use crate::protocol::types::{BROADCAST_ADDR, KEY_ID_LEGACY, TAG_SIZE};

// ============================================================================
// SHA-256 PURO (sem dependências — o Bythos não tem deps externas)
// ============================================================================
//
// Implementação direta da FIPS 180-4, tabela de constantes incluída. É mais
// código do que gostaríamos de carregar, mas a alternativa (depender de uma
// crate de hash) partiria o suporte `no_std` das pontes e o C standalone.
// Em produção com ATECC608 este código nem é chamado — o chip faz tudo.

/// Rotação à direita de 32 bits (o SHA-256 é feito disto).
///
/// Usa o intrínseco em vez de `(x >> n) | (x << (32 - n))`: mesma semântica,
/// mas o compilador emite `ror` direto e o `clippy` não reclama.
#[inline(always)]
fn rotr(x: u32, n: u32) -> u32 {
    x.rotate_right(n)
}

/// As 64 constantes da primeira metade dos primos (FIPS 180-4 §4.2.2).
///
/// Hardcoded em vez de geradas para compilar em `const`-pobreza e não gastar
/// arranque do MCU a crivar primos — 256 B de ROM bem empregues.
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Estado em curso de um cálculo SHA-256.
///
/// Desenhado para uso incremental (`update`/`finalize`) porque o `TAG` cobre a
/// trama inteira e a ponte pode não ter RAM para a copiar: alimenta-se por
/// bocados, diretamente do tampão de receção.
#[derive(Debug, Clone)]
pub struct Sha256 {
    /// Os 8 acumuladores (`h0–h7` da norma).
    state: [u32; 8],
    /// Bloco de 64 B em enchimento.
    block: [u8; 64],
    /// Bytes já absorvidos no bloco atual.
    block_len: usize,
    /// Comprimento total da mensagem, em bits (vai para o enchimento final).
    total_bits: u64,
}

impl Sha256 {
    /// Estado inicial da norma (`h0–h7` = raízes quadradas fracionárias).
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            block: [0u8; 64],
            block_len: 0,
            total_bits: 0,
        }
    }

    /// Absorve bytes (pode chamar-se várias vezes; sem alocação).
    pub fn update(&mut self, mut data: &[u8]) {
        // Contador de 64 bits: tramas Bythos têm ~1 KiB, enrolar é impossível —
        // mas somar com `wrapping` documenta que pensámos nisso.
        self.total_bits = self.total_bits.wrapping_add((data.len() as u64) * 8);
        while !data.is_empty() {
            let take = (64 - self.block_len).min(data.len());
            self.block[self.block_len..self.block_len + take].copy_from_slice(&data[..take]);
            self.block_len += take;
            data = &data[take..];
            if self.block_len == 64 {
                self.compress();
                self.block_len = 0;
            }
        }
    }

    /// Uma volta de compressão sobre o bloco cheio (o coração da FIPS 180-4).
    fn compress(&mut self) {
        // Agenda de 64 palavras: 16 lidas do bloco (big-endian, ordem do SHA)
        // + 48 derivadas (σ0/σ1). Iterador com índice para calar o `clippy`
        // sem perder a correspondência 1:1 com a norma.
        let mut w = [0u32; 64];
        for (i, slot) in w.iter_mut().enumerate().take(16) {
            *slot = ((self.block[i * 4] as u32) << 24)
                | ((self.block[i * 4 + 1] as u32) << 16)
                | ((self.block[i * 4 + 2] as u32) << 8)
                | (self.block[i * 4 + 3] as u32);
        }
        for i in 16..64 {
            let s0 = rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
            let s1 = rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        // Oito registos de trabalho, 64 rondas de Ch/Maj/Σ.
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        // Realimentação nos acumuladores.
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    /// Enchimento final (`1` + zeros + comprimento de 64 bits) e digest de 32 B.
    pub fn finalize(mut self) -> [u8; 32] {
        // O `0x80` marca o fim; se não couberem os 8 B do comprimento neste
        // bloco, comprime-se e abre-se outro — o caso limite da norma.
        self.block[self.block_len] = 0x80;
        self.block_len += 1;
        if self.block_len > 56 {
            for b in self.block[self.block_len..].iter_mut() {
                *b = 0;
            }
            self.compress();
            self.block_len = 0;
        }
        for b in self.block[self.block_len..56].iter_mut() {
            *b = 0;
        }
        let bits = self.total_bits.to_be_bytes();
        self.block[56..64].copy_from_slice(&bits);
        self.compress();
        // Digest big-endian, como manda a norma.
        let mut out = [0u8; 32];
        for (i, s) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&s.to_be_bytes());
        }
        out
    }

    /// Atalho de uma só chamada para entradas pequenas (chaves, vetores).
    pub fn digest(data: &[u8]) -> [u8; 32] {
        let mut h = Self::new();
        h.update(data);
        h.finalize()
    }
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

/// HMAC-SHA256 (`RFC 2104`): `H((K ⊕ opad) ‖ H((K ⊕ ipad) ‖ m))`.
///
/// Chaves maiores que o bloco (64 B) são previamente condensadas — as chaves
/// Bythos têm 32 B, logo o caminho quente é só dois XORs + dois hashes.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    // Normaliza a chave para exatamente um bloco.
    let mut k_block = [0u8; 64];
    if key.len() > 64 {
        k_block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }
    // Almofadas interna/externa.
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k_block[i];
        opad[i] ^= k_block[i];
    }
    // Hash interno sobre `ipad ‖ mensagem`, depois externo sobre `opad ‖ interno`.
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner_digest);
    outer.finalize()
}

/// HMAC-SHA256 sobre mensagem em duas partes (`prefixo ‖ corpo`), sem concatenar.
///
/// Existe porque o `TAG` V4 autentica `(CTR ‖ trama)` e a ponte não tem RAM para
/// copiar a trama só para a selar: alimenta-se o contador e depois o tampão de
/// receção, direto, sem tampão intermédio de 1,2 KiB na pilha.
pub fn hmac_sha256_split(key: &[u8], prefix: &[u8], body: &[u8]) -> [u8; 32] {
    let mut k_block = [0u8; 64];
    if key.len() > 64 {
        k_block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k_block[i];
        opad[i] ^= k_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(prefix);
    inner.update(body);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner_digest);
    outer.finalize()
}

// ============================================================================
// COFRE DE SELAGEM (quem guarda a chave e conta o CTR)
// ============================================================================

/// Configuração de selagem de um emissor: que chave usar e que contador vai.
///
/// Em produção isto é preenchido a partir do ATECC608 (a chave de 32 B é
/// derivada por ECDH+HKDF no comissionamento e nunca escrita em flash legível).
/// No simulador/testes usa-se `software()` com chave efémera — com aviso.
#[derive(Debug, Clone)]
pub struct SealConfig {
    /// Ranhura de chave (`0–253`; `0xFF` = legado, sem TAG real).
    pub key_id: u8,
    /// Chave de 32 B (só caminho de software; com chip, zeros + driver).
    pub key: [u8; 32],
    /// Próximo contador a emitir (24 bits úteis; persiste no elemento seguro).
    pub next_ctr: u32,
    /// Endereço deste emissor (vai para `SRC`; evita forjar origem).
    pub src: u16,
}

impl SealConfig {
    /// Cofre de software para testes/simulação — **nunca em produção**.
    ///
    /// A chave vive em RAM legível; serve para validar o fio e os vetores sem
    /// hardware. O nome grita a intenção para ninguém o ligar ao ATECC608 real.
    pub fn software(key_id: u8, key: [u8; 32], src: u16) -> Self {
        Self {
            key_id,
            key,
            next_ctr: 0,
            src,
        }
    }

    /// Reserva o contador seguinte e avança (monotónico, sem reciclar).
    ///
    /// Máscara de 24 bits: a 100 Hz demora ~46 h a enrolar; ao enrolar, a frota
    /// deve rodar a chave (o verificador rejeita `CTR` repetido de qualquer
    /// forma — enrolar não reabre replay, só exige re-sincronização).
    pub fn take_ctr(&mut self) -> u32 {
        let ctr = self.next_ctr & 0xFF_FFFF;
        self.next_ctr = self.next_ctr.wrapping_add(1) & 0xFF_FFFF;
        ctr
    }
}

/// Elemento seguro: o contrato que o ATECC608 (ou stub) implementa.
///
/// Desenhado para I2C sem alocação: o driver recebe tampões do chamador e
/// devolve códigos, nunca `Result` com heap. O `SoftwareVault` abaixo é a
/// implementação de referência para testes.
pub trait SecureElement {
    /// Calcula o `TAG` de 4 B sobre `covered` (cabeçalho+campos+SEC_HDR).
    fn seal(&mut self, key_id: u8, ctr: u32, covered: &[u8]) -> [u8; TAG_SIZE];
    /// Contador persistente atual (para retomar após reset sem repetir).
    fn stored_ctr(&self, key_id: u8) -> u32;
    /// Avança o contador persistente (chamado após cada selagem emitida).
    fn advance_ctr(&mut self, key_id: u8);
}

/// Cofre 100% em software (testes/simulação). A chave está em RAM — ver aviso
/// em `SealConfig::software`. A lógica (HMAC + CTR) é a mesma da produção.
#[derive(Debug, Clone)]
pub struct SoftwareVault {
    /// Chaves por ranhura (só as configuradas são válidas).
    keys: [[u8; 32]; 4],
    /// Contadores persistentes por ranhura.
    ctrs: [u32; 4],
}

impl SoftwareVault {
    /// Cofre vazio (sem chaves — selar falha até provisionar).
    pub fn new() -> Self {
        Self {
            keys: [[0u8; 32]; 4],
            ctrs: [0u32; 4],
        }
    }

    /// Provisiona uma ranhura (`0–3`) com chave de 32 B.
    ///
    /// Nos testes isto substitui a cerimónia ECDH+HKDF do comissionamento real.
    pub fn provision(&mut self, slot: usize, key: [u8; 32]) {
        if slot < 4 {
            self.keys[slot] = key;
            self.ctrs[slot] = 0;
        }
    }

    /// Índice interno da ranhura (`key_id` público → 0–3); `None` = desconhecida.
    fn slot_of(key_id: u8) -> Option<usize> {
        if key_id < 4 {
            Some(key_id as usize)
        } else {
            None
        }
    }
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureElement for SoftwareVault {
    fn seal(&mut self, key_id: u8, ctr: u32, covered: &[u8]) -> [u8; TAG_SIZE] {
        // Ranhura desconhecida = zeros (o verificador vai recusar de qualquer
        // forma; nunca inventar etiqueta "plausível").
        let Some(slot) = Self::slot_of(key_id) else {
            return [0u8; TAG_SIZE];
        };
        // O `TAG` vincula o contador: sem isto, recortar-e-colar etiquetas entre
        // tramas com `CTR` diferente passaria — o essencial do anti-replay.
        tagv4(&self.keys[slot], ctr, covered)
    }

    fn stored_ctr(&self, key_id: u8) -> u32 {
        Self::slot_of(key_id).map(|s| self.ctrs[s]).unwrap_or(0)
    }

    fn advance_ctr(&mut self, key_id: u8) {
        if let Some(slot) = Self::slot_of(key_id) {
            self.ctrs[slot] = self.ctrs[slot].wrapping_add(1) & 0xFF_FFFF;
        }
    }
}

// ============================================================================
// SELAGEM E VERIFICAÇÃO DO FIO (caminho quente — simples e auditável)
// ============================================================================

/// Calcula o `TAG` V4 sobre a região coberta da trama.
///
/// `covered` = bytes do `INÍCIO` ao fim do `SEC_HDR`, inclusive. O `TAG` cobre o
/// `CTR`, logo cada contador tem etiqueta única — recortar etiquetas entre
/// tramas não passa.
pub fn tagv4(key: &[u8; 32], ctr: u32, covered: &[u8]) -> [u8; TAG_SIZE] {
    // Sem tampão intermédio: o contador (3 B LE) entra como prefixo do HMAC e a
    // trama segue direta — ver `hmac_sha256_split`.
    let ctr3 = (ctr & 0xFF_FFFF).to_le_bytes();
    let full = hmac_sha256_split(key, &ctr3[..3], covered);
    [full[0], full[1], full[2], full[3]]
}

/// Verifica o `TAG` em tempo constante.
///
/// Sem early-exit: compara os 4 bytes sempre, para a duração não denunciar onde
/// divergiu. Com 4 bytes o ganho é teórico, mas o hábito é grátis.
pub fn verifyv4(key: &[u8; 32], ctr: u32, covered: &[u8], tag: &[u8; TAG_SIZE]) -> bool {
    let expected = tagv4(key, ctr, covered);
    let mut diff = 0u8;
    for i in 0..TAG_SIZE {
        diff |= expected[i] ^ tag[i];
    }
    diff == 0
}

// ============================================================================
// ANTI-REPETIÇÃO (janela por emissor)
// ============================================================================

/// Tamanho da janela de aceitação: cobre reordenação do anel sem aceitar replay.
///
/// 64 tramas de folga chegam para os dois sentidos do anel entregarem fora de
/// ordem; acima disso o emissor está a repetir ou a frota dessincronizou.
pub const REPLAY_WINDOW: u32 = 64;

/// Nº de emissores seguidos em simultâneo por verificador.
///
/// 8 entradas cobrem vizinhança + difusões típicas sem hash nem heap; com mais
/// emissores ativos, a entrada menos usada é despejada (política LRU ingénua em
/// array — determinística e sem alocação).
pub const PEER_SLOTS: usize = 8;

/// Janela anti-replay de um emissor: último `CTR` aceite + máscara de recentes.
///
/// Representa "vi os CTRs (last-63 … last]" de forma compacta: `mask[0]` = se
/// `last` foi visto (sempre 1 após aceitar), `mask[k]` = se `last-k` foi visto.
/// Avançar a janela desloca a máscara; aceitar dentro da janela marca o bit.
#[derive(Debug, Clone, Copy)]
pub struct ReplayWindow {
    /// Último `CTR` aceite (24 bits).
    pub last: u32,
    /// Máscara dos 64 anteriores (bit `k` = `last-k` visto).
    pub mask: u64,
    /// Se já aceitou alguma trama (distingue "last=0 visto" de "nada visto").
    pub seen_any: bool,
}

impl ReplayWindow {
    /// Janela virgem (nada aceite).
    pub fn new() -> Self {
        Self {
            last: 0,
            mask: 0,
            seen_any: false,
        }
    }

    /// Tenta aceitar `ctr`: `true` = fresco (atualiza), `false` = replay/velho.
    ///
    /// Aritmética em 24 bits com enrolamento: a distância mede-se no círculo
    /// (`new - last mod 2²⁴`), logo o enrolar legítimo do contador continua a
    /// funcionar sem cerimónia de re-sincronização.
    pub fn accept(&mut self, ctr: u32) -> bool {
        let ctr = ctr & 0xFF_FFFF;
        if !self.seen_any {
            // Primeira trama: aceita qualquer valor como âncora.
            self.last = ctr;
            self.mask = 1;
            self.seen_any = true;
            return true;
        }
        // Distância para a frente no círculo de 24 bits.
        let fwd = ctr.wrapping_sub(self.last) & 0xFF_FFFF;
        if fwd == 0 {
            return false; // Repetição exata.
        }
        if fwd < (1 << 23) {
            // Para a frente: avança a janela (corta o que cai fora dos 64).
            if fwd >= 64 {
                self.mask = 1;
            } else {
                self.mask <<= fwd;
                self.mask |= 1;
            }
            self.last = ctr;
            return true;
        }
        // Para trás: aceita se dentro da janela e ainda não visto.
        let back = self.last.wrapping_sub(ctr) & 0xFF_FFFF;
        if back == 0 || back > REPLAY_WINDOW {
            return false;
        }
        let bit = 1u64 << back;
        if self.mask & bit != 0 {
            return false; // Já visto = replay.
        }
        self.mask |= bit;
        true
    }
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// Tabela de janelas por emissor (sem heap: array fixo + despejo LRU).
///
/// Chaveada por `(SRC, KEY_ID)` — a mesma origem com chaves diferentes tem
/// contadores independentes (rotação de chave não invalida a janela antiga).
#[derive(Debug, Clone)]
pub struct PeerTable {
    /// Entradas `(src, key_id, janela, usada)`.
    slots: [(u16, u8, ReplayWindow, bool); PEER_SLOTS],
    /// Contador monotónico para LRU (o menor `stamp` é despejado).
    stamps: [u32; PEER_SLOTS],
    /// Relógio LRU.
    tick: u32,
}

impl PeerTable {
    /// Tabela vazia.
    pub fn new() -> Self {
        Self {
            slots: [(0, 0, ReplayWindow::new(), false); PEER_SLOTS],
            stamps: [0u32; PEER_SLOTS],
            tick: 0,
        }
    }

    /// Tenta aceitar `(src, key_id, ctr)`; `false` = replay ou emissor velho.
    ///
    /// Endereço de difusão nunca entra aqui: `BROADCAST_ADDR` não é emissor e
    /// aceitá-lo envenenaria a tabela com `CTR` de toda a gente.
    pub fn accept(&mut self, src: u16, key_id: u8, ctr: u32) -> bool {
        if src == BROADCAST_ADDR || key_id == KEY_ID_LEGACY {
            return false;
        }
        self.tick = self.tick.wrapping_add(1);
        // Emissor conhecido: atualiza janela e carimbo.
        for i in 0..PEER_SLOTS {
            if self.slots[i].3 && self.slots[i].0 == src && self.slots[i].1 == key_id {
                self.stamps[i] = self.tick;
                return self.slots[i].2.accept(ctr);
            }
        }
        // Novo: ocupa ranhura vazia primeiro; cheia, despeja a menos usada.
        // (Procura em duas passadas para o compilador não se queixar de
        // atribuição morta — clareza que também cala o `clippy`.)
        let mut victim: Option<usize> = None;
        for i in 0..PEER_SLOTS {
            if !self.slots[i].3 {
                victim = Some(i);
                break;
            }
        }
        let victim = match victim {
            Some(i) => i,
            None => {
                let mut oldest = 0usize;
                for i in 1..PEER_SLOTS {
                    if self.stamps[i] < self.stamps[oldest] {
                        oldest = i;
                    }
                }
                oldest
            }
        };
        let mut w = ReplayWindow::new();
        let ok = w.accept(ctr);
        self.slots[victim] = (src, key_id, w, true);
        self.stamps[victim] = self.tick;
        ok
    }
}

impl Default for PeerTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Formata um resumo em hexadecimal (64 ASCII) sem heap — só para asserts.
    fn hex_of(digest: &[u8; 32]) -> [u8; 64] {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = [0u8; 64];
        for (i, b) in digest.iter().enumerate() {
            out[i * 2] = HEX[(b >> 4) as usize];
            out[i * 2 + 1] = HEX[(b & 0xF) as usize];
        }
        out
    }

    #[test]
    fn test_sha256_vetor_conhecido() {
        // "abc" é o vetor canónico da FIPS 180-4 — se isto falha, o resto é ruído.
        let d = Sha256::digest(b"abc");
        assert_eq!(
            &hex_of(&d),
            b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sha256_vetor_longo() {
        // "123456789" (o mesmo do teste CRC) — resumo publicado.
        let d = Sha256::digest(b"123456789");
        assert_eq!(
            &hex_of(&d),
            b"15e2b0d3c33891ebb0f1ef609ec419420c20e320ce94c65fbc8c3312448eb225"
        );
    }

    #[test]
    fn test_hmac_rfc4231_caso1() {
        // RFC 4231, caso 1: chave 20×0x0b, dados "Hi There" — HMAC completo publicado.
        let key = [0x0bu8; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        let expected: [u8; 32] = [
            0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b,
            0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c,
            0x2e, 0x32, 0xcf, 0xf7,
        ];
        assert_eq!(mac, expected);
    }

    #[test]
    fn test_tag_ida_volta_e_tempo_constante() {
        // Sela e verifica; 1 bit trocado no TAG ou no CTR chumba.
        let key = [0x42u8; 32];
        let covered = [0xAAu8, 0x04, 0x06, 0x00];
        let tag = tagv4(&key, 7, &covered);
        assert!(verifyv4(&key, 7, &covered, &tag));
        let mut bad = tag;
        bad[0] ^= 1;
        assert!(!verifyv4(&key, 7, &covered, &bad));
        assert!(!verifyv4(&key, 8, &covered, &tag));
        // Chave errada chumba.
        assert!(!verifyv4(&[0x43u8; 32], 7, &covered, &tag));
    }

    #[test]
    fn test_replay_janela_basica() {
        // Crescente passa; repetição e velho chumbam.
        let mut w = ReplayWindow::new();
        assert!(w.accept(10));
        assert!(w.accept(11));
        assert!(!w.accept(11));
        assert!(!w.accept(10));
        assert!(w.accept(12));
    }

    #[test]
    fn test_replay_reordenacao_na_janela() {
        // Fora de ordem dentro dos 64 passa uma vez; segunda é replay.
        let mut w = ReplayWindow::new();
        assert!(w.accept(100));
        assert!(w.accept(105));
        assert!(w.accept(103));
        assert!(!w.accept(103));
        // Fora da janela (100 quando last=200) chumba.
        assert!(w.accept(200));
        assert!(!w.accept(100));
    }

    #[test]
    fn test_replay_enrolamento_24bits() {
        // Enrolar 0xFFFFFF → 0 continua a funcionar (aritmética circular).
        let mut w = ReplayWindow::new();
        assert!(w.accept(0xFF_FFFE));
        assert!(w.accept(0xFF_FFFF));
        assert!(w.accept(0x00_0000));
        assert!(w.accept(0x00_0001));
        assert!(!w.accept(0xFF_FFFF));
    }

    #[test]
    fn test_peer_table_isola_emissores() {
        // Contadores independentes por (SRC, KEY_ID); difusão recusada.
        let mut t = PeerTable::new();
        assert!(t.accept(6, 0, 1));
        assert!(t.accept(7, 0, 1));
        assert!(!t.accept(6, 0, 1));
        assert!(t.accept(6, 1, 1));
        assert!(!t.accept(BROADCAST_ADDR, 0, 2));
        assert!(!t.accept(6, KEY_ID_LEGACY, 2));
    }

    #[test]
    fn test_peer_table_despejo_lru() {
        // Mais emissores que ranhuras: os antigos caem sem pânico.
        let mut t = PeerTable::new();
        for s in 0..(PEER_SLOTS as u16 + 3) {
            assert!(t.accept(100 + s, 0, 1));
        }
        // O despejado reentra como "novo" (âncora) — sem replay falso-positivo.
        assert!(t.accept(100, 0, 2));
    }

    #[test]
    fn test_seal_config_ctr_monotonico() {
        let mut s = SealConfig::software(1, [9u8; 32], 27);
        assert_eq!(s.take_ctr(), 0);
        assert_eq!(s.take_ctr(), 1);
        s.next_ctr = 0xFF_FFFF;
        assert_eq!(s.take_ctr(), 0xFF_FFFF);
        assert_eq!(s.take_ctr(), 0); // Enrola mascarado, nunca repete sem aviso.
    }

    #[test]
    fn test_software_vault_sela_e_avanca() {
        let mut v = SoftwareVault::new();
        v.provision(0, [0x42u8; 32]);
        let t1 = v.seal(0, 5, b"dados");
        let t2 = v.seal(0, 5, b"dados");
        assert_eq!(t1, t2); // Determinístico para o mesmo CTR.
        let t3 = v.seal(0, 6, b"dados");
        assert_ne!(t1, t3); // CTR diferente, etiqueta diferente.
        assert_eq!(v.stored_ctr(0), 0);
        v.advance_ctr(0);
        assert_eq!(v.stored_ctr(0), 1);
        // Ranhura desconhecida: zeros (verificador recusa).
        assert_eq!(v.seal(9, 5, b"dados"), [0u8; TAG_SIZE]);
    }
}
