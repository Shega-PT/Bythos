//! # Túnel Bythos — Rádio Agnóstica e Vídeo Fragmentado (v4.0.0)
//!
//! Este módulo resolve dois problemas de transporte sem amarrar o protocolo a
//! nenhum rádio:
//!
//! 1. **Enquadramento sobre tubo opaco (COBS).** LoRa, Wi-Fi, ESP-NOW ou FSK
//!    entregam pacotes, não tramas com `0xAA` de sincronização. O COBS
//!    (`Consistent Overhead Byte Stuffing`) remove os zeros da carga para que o
//!    `0x00` sirva de delimitador inequívoco — sem escapes ambíguos, sem tabelas,
//!    com sobrecarga máxima de ~0,4%. A ponte faz `COBS + LEN + trama V4` para
//!    qualquer modem, e o modem continua "burro" (não conhece Bythos).
//! 2. **Vídeo maior que a trama.** Uma imagem não cabe em 1205 B; o emissor
//!    parte-a em fragmentos de ≤32 B (campos normais do fio) e o recetor
//!    remonta-a com `VideoReassembler` — sem heap, com deteção de fragmento em
//!    falta, duplicado e fora de ordem.
//!
//! Exigência de segurança (ver spec §7): o túnel transporta tramas V4 **já
//! seladas** — ambos os extremos têm ponte com a mesma `KEY_ID`/sessão, senão o
//! `TAG` chumba e o túnel descarta. Rádio aberta, criptografia fechada.

// ============================================================================
// COBS (RFC implícito — algoritmo de Stuart)
// ============================================================================

/// Teto de expansão do COBS: 1 byte por cada 254 + 1 de enquadramento.
///
/// Para 1205 B: `1205 + ceil(1205/254) + 1 = 1211`. O tampão do túnel usa
/// `TUNNEL_MAX` — dimensionar abaixo disto é truncar em silêncio, proibido aqui.
pub const TUNNEL_MAX: usize = 1211;

/// Codifica `data` em COBS para `output` (sem o delimitador final).
///
/// Retorna os bytes escritos, ou `None` se `output` for curto. Nunca escreve
/// `0x00` — invariante que o recetor usa como delimitador. Entrada vazia produz
/// saída `[0x01]` (o COBS define bloco vazio explícito, não saída vazia).
pub fn cobs_encode(data: &[u8], output: &mut [u8]) -> Option<usize> {
    // Precisa de pelo menos 2 bytes para qualquer entrada útil.
    if output.len() < 2 {
        return None;
    }
    let mut read = 0usize;
    let mut write = 1usize; // Reserva o byte de distância do primeiro bloco.
    let mut code_at = 0usize; // Onde vai a distância deste bloco.
    let mut code: u8 = 1;
    while read < data.len() {
        if data[read] == 0 {
            // Fim do bloco: escreve a distância e abre outro.
            if write >= output.len() {
                return None;
            }
            output[code_at] = code;
            code_at = write;
            write += 1;
            code = 1;
            read += 1;
        } else {
            if write >= output.len() {
                return None;
            }
            output[write] = data[read];
            write += 1;
            read += 1;
            code = code.wrapping_add(1);
            // Bloco cheio (254 dados): fecha e abre outro sem consumir zero.
            if code == 0xFF {
                if write >= output.len() {
                    return None;
                }
                output[code_at] = code;
                code_at = write;
                write += 1;
                code = 1;
            }
        }
    }
    if code_at >= output.len() {
        return None;
    }
    output[code_at] = code;
    Some(write)
}

/// Descodifica COBS de `data` para `output`.
///
/// Retorna os bytes escritos, ou `None` em trama inválida (distância que salta
/// fora da entrada). Nunca confia na distância sem medir — é aqui que pacotes
/// de rádio corrompidos morrem em vez de causarem OOB.
pub fn cobs_decode(data: &[u8], output: &mut [u8]) -> Option<usize> {
    let mut read = 0usize;
    let mut write = 0usize;
    while read < data.len() {
        let code = data[read];
        if code == 0 {
            return None; // Zero dentro de COBS é sempre corrupção.
        }
        read += 1;
        let span = (code - 1) as usize;
        // A distância não pode saltar para fora da entrada…
        if read + span > data.len() {
            return None;
        }
        // …nem a saída pode transbordar…
        if write + span > output.len() {
            return None;
        }
        // …nem os "dados" podem conter zero: o codificador nunca emite `0x00`,
        // logo zero aqui é corrupção do pacote de rádio, não carga.
        for b in &data[read..read + span] {
            if *b == 0 {
                return None;
            }
        }
        output[write..write + span].copy_from_slice(&data[read..read + span]);
        read += span;
        write += span;
        // Bloco fechado por zero real (código < 0xFF e ainda há entrada).
        if code < 0xFF && read < data.len() {
            if write >= output.len() {
                return None;
            }
            output[write] = 0;
            write += 1;
        }
    }
    Some(write)
}

// ============================================================================
// VÍDEO FRAGMENTADO (sobre campos normais do fio — ≤32 B cada)
// ============================================================================

/// Carga útil máxima de um fragmento de vídeo no fio.
///
/// 32 B = `MAX_FIELD_DATA`: o fragmento viaja como campo `raw` normal, logo
/// atravessa validador, analisador e pontes sem caminho especial. Fragmentos
/// maiores exigiriam o caminho de 128 B, que só comporta um por trama.
pub const VIDEO_CHUNK_MAX: usize = 32;

/// Fragmentos máximos por imagem remontada.
///
/// 8 × 32 B = 256 B por imagem: suficiente para telemetria visual comprimida e
/// thumbnails; vídeo real usa sequência de imagens, não imagens maiores. Oito
/// bits de máscara cabem num `u8` — sem arrays de estado, sem heap.
pub const VIDEO_FRAME_CHUNKS: usize = 8;

/// Descritor de um fragmento (cabe nos 3 campos pequenos do catálogo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoFrag {
    /// Imagem a que pertence.
    pub frame_id: u16,
    /// Índice do fragmento (`0–total`).
    pub chunk_id: u8,
    /// Total de fragmentos da imagem.
    pub total: u8,
}

impl VideoFrag {
    /// Valida coerência (`total` dentro do teto, `chunk < total`).
    pub fn valid(self) -> bool {
        self.total > 0 && (self.total as usize) <= VIDEO_FRAME_CHUNKS && self.chunk_id < self.total
    }
}

/// Remontador de uma imagem (sem heap: tampão fixo + máscara de recebidos).
///
/// Política: primeira imagem que chega ancora (`frame_id`); fragmentos de outra
/// imagem reiniciam a remontagem (o emissor só tem uma imagem em voo de cada
/// vez por destino). Duplicado conta mas não corrompe; em falta = incompleta.
pub struct VideoReassembler {
    /// Imagem em remontagem (`None` = vazio).
    frame_id: Option<u16>,
    /// Total esperado.
    total: u8,
    /// Máscara de fragmentos recebidos (bit `i` = fragmento `i` presente).
    mask: u8,
    /// Cargas concatenadas por índice (`chunk × 32 B`).
    chunks: [[u8; VIDEO_CHUNK_MAX]; VIDEO_FRAME_CHUNKS],
    /// Comprimentos reais por fragmento (o último pode ser curto).
    lens: [u8; VIDEO_FRAME_CHUNKS],
}

impl VideoReassembler {
    /// Remontador vazio.
    pub fn new() -> Self {
        Self {
            frame_id: None,
            total: 0,
            mask: 0,
            chunks: [[0u8; VIDEO_CHUNK_MAX]; VIDEO_FRAME_CHUNKS],
            lens: [0u8; VIDEO_FRAME_CHUNKS],
        }
    }

    /// Ingere um fragmento; `true` = a imagem ficou completa nesta chamada.
    ///
    /// Carga acima de 32 B é recusada (fragmento malformado, não "grande") e
    /// imagem nova reinicia o estado — nunca mistura metades de duas imagens.
    pub fn push(&mut self, frag: VideoFrag, payload: &[u8]) -> bool {
        if !frag.valid() || payload.len() > VIDEO_CHUNK_MAX {
            return false;
        }
        // Imagem nova (ou vazio): reinicia sem piedade nem fuga de estado.
        if self.frame_id != Some(frag.frame_id) {
            self.frame_id = Some(frag.frame_id);
            self.total = frag.total;
            self.mask = 0;
            self.lens = [0u8; VIDEO_FRAME_CHUNKS];
        }
        // Guarda contra emissor que muda `total` a meio da imagem.
        if frag.total != self.total {
            return false;
        }
        let i = frag.chunk_id as usize;
        self.chunks[i][..payload.len()].copy_from_slice(payload);
        self.lens[i] = payload.len() as u8;
        self.mask |= 1 << i;
        self.is_complete()
    }

    /// `true` se todos os fragmentos `0–total` já chegaram.
    pub fn is_complete(&self) -> bool {
        if self.frame_id.is_none() || self.total == 0 {
            return false;
        }
        // Máscara cheia = (1 << total) - 1.
        let full: u8 = (1u16 << self.total as u32).wrapping_sub(1) as u8;
        self.mask & full == full
    }

    /// Comprimento total da imagem remontada (`None` se incompleta).
    pub fn assembled_len(&self) -> Option<usize> {
        if !self.is_complete() {
            return None;
        }
        let mut n = 0usize;
        for i in 0..self.total as usize {
            n += self.lens[i] as usize;
        }
        Some(n)
    }

    /// Copia a imagem remontada para `output` (`None` se incompleta ou curta).
    pub fn assemble(&self, output: &mut [u8]) -> Option<usize> {
        let n = self.assembled_len()?;
        if output.len() < n {
            return None;
        }
        let mut at = 0;
        for i in 0..self.total as usize {
            let len = self.lens[i] as usize;
            output[at..at + len].copy_from_slice(&self.chunks[i][..len]);
            at += len;
        }
        Some(n)
    }

    /// Esquece a imagem atual (após entregar ou por timeout do firmware).
    pub fn reset(&mut self) {
        self.frame_id = None;
        self.total = 0;
        self.mask = 0;
    }
}

impl Default for VideoReassembler {
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

    #[test]
    fn test_cobs_ida_volta() {
        // Carga com zeros no meio, no início e no fim — o caso que parte
        // enquadramentos ingénuos baseados em "byte reservado".
        let data = [0x00u8, 0x11, 0x00, 0x22, 0x33, 0x00];
        let mut enc = [0u8; 16];
        let n = cobs_encode(&data, &mut enc).unwrap();
        // Garantia do algoritmo: nem um zero na saída.
        assert!(!enc[..n].contains(&0));
        let mut dec = [0u8; 16];
        let m = cobs_decode(&enc[..n], &mut dec).unwrap();
        assert_eq!(&dec[..m], &data);
    }

    #[test]
    fn test_cobs_bloco_cheio_254() {
        // 254 bytes sem zero forçam o fecho de bloco por tamanho (código 0xFF).
        let data = [0x7Eu8; 254];
        let mut enc = [0u8; 300];
        let n = cobs_encode(&data, &mut enc).unwrap();
        assert_eq!(enc[0], 0xFF);
        assert!(!enc[..n].contains(&0));
        let mut dec = [0u8; 300];
        let m = cobs_decode(&enc[..n], &mut dec).unwrap();
        assert_eq!(&dec[..m], &data);
    }

    #[test]
    fn test_cobs_corrompido_recusa() {
        // Distância que salta fora da entrada = pacote de rádio comido.
        assert!(cobs_decode(&[0x05, 0x01], &mut [0u8; 8]).is_none());
        // Zero dentro do COBS = corrupção inequívoca.
        assert!(cobs_decode(&[0x02, 0x00, 0x01], &mut [0u8; 8]).is_none());
        // Saída curta: recusa em vez de truncar.
        let data = [0x01u8, 0x02, 0x03];
        let mut enc = [0u8; 8];
        let n = cobs_encode(&data, &mut enc).unwrap();
        assert!(cobs_decode(&enc[..n], &mut [0u8; 2]).is_none());
    }

    #[test]
    fn test_remontagem_ordem_trocada_e_duplicado() {
        // 3 fragmentos fora de ordem + 1 duplicado: completa uma vez, sem lixo.
        let mut r = VideoReassembler::new();
        let frag = |c| VideoFrag {
            frame_id: 42,
            chunk_id: c,
            total: 3,
        };
        assert!(!r.push(frag(2), &[0xCC; 32]));
        assert!(!r.push(frag(0), &[0xAA; 32]));
        assert!(!r.push(frag(0), &[0xAA; 32])); // Duplicado: conta, não corrompe.
        assert!(!r.is_complete());
        assert!(r.push(frag(1), &[0xBB; 10])); // Último curto: completa.
        assert_eq!(r.assembled_len(), Some(32 + 32 + 10));
        let mut out = [0u8; 256];
        let n = r.assemble(&mut out).unwrap();
        assert_eq!(n, 74);
        assert_eq!(out[0], 0xAA);
        assert_eq!(out[32], 0xBB);
        assert_eq!(out[42], 0xCC);
    }

    #[test]
    fn test_remontagem_em_falta_e_imagem_nova() {
        // Em falta = incompleta; imagem nova reinicia sem misturar.
        let mut r = VideoReassembler::new();
        let frag = |f, c| VideoFrag {
            frame_id: f,
            chunk_id: c,
            total: 2,
        };
        assert!(!r.push(frag(7, 0), &[0x11; 32]));
        assert!(r.assemble(&mut [0u8; 256]).is_none());
        assert!(!r.push(frag(8, 0), &[0x22; 32])); // Nova imagem, ainda incompleta.
        assert!(!r.is_complete());
        let mut out = [0u8; 256];
        assert!(r.assemble(&mut out).is_none());
        assert!(r.push(frag(8, 1), &[0x33; 5]));
        let n = r.assemble(&mut out).unwrap();
        assert_eq!(n, 37);
        assert_eq!(out[0], 0x22);
        assert_eq!(out[32], 0x33);
    }

    #[test]
    fn test_fragmento_invalido_recusa() {
        let mut r = VideoReassembler::new();
        // total 0, chunk >= total, total acima do teto, carga > 32.
        assert!(!r.push(
            VideoFrag {
                frame_id: 1,
                chunk_id: 0,
                total: 0
            },
            &[0x00]
        ));
        assert!(!r.push(
            VideoFrag {
                frame_id: 1,
                chunk_id: 2,
                total: 2
            },
            &[0x00]
        ));
        assert!(!r.push(
            VideoFrag {
                frame_id: 1,
                chunk_id: 0,
                total: 9
            },
            &[0x00]
        ));
        assert!(!r.push(
            VideoFrag {
                frame_id: 1,
                chunk_id: 0,
                total: 1
            },
            &[0x00; 33]
        ));
        assert!(!r.is_complete());
    }
}
