//! # CRC-16/CCITT — Implementação Puro Rust
//!
//! Implementação do algoritmo CRC-16 utilizando o polinómio CCITT (0x1021).
//! Esta implementação é totalmente portátil, sem dependências externas,
//! e utiliza uma tabela lookup de 256 entradas para cálculo O(1) por byte.
//!
//! ## Especificação
//!
//! - **Polinómio:** 0x1021 (x^16 + x^12 + x^5 + 1)
//! - **Valor inicial:** 0xFFFF
//! - **Reflexão de entrada:** Não
//! - **Reflexão de saída:** Não
//! - **XOR final:** 0x00
//!
//! ## Referência
//!
//! CRC-16/CCITT é amplamente utilizado em protocolos de comunicação
//! (X.25, HDLC, Bluetooth, etc). Oferece boa detecção de erros para
//! pacotes até ~4KB, com probabilidade de falha ~2^-16.

/// Tabela lookup de 256 entradas para CRC-16/CCITT (polinómio 0x1021).
///
/// Pré-calculada para eficiência máxima. Cada entrada[i] representa
/// o CRC-16 do byte `i` considerando o polinómio 0x1021 com valor
/// inicial 0xFFFF.
const CRC16_TABLE: [u16; 256] = [
    0x0000, 0x1021, 0x2042, 0x3063, 0x4084, 0x50A5, 0x60C6, 0x70E7,
    0x8108, 0x9129, 0xA14A, 0xB16B, 0xC18C, 0xD1AD, 0xE1CE, 0xF1EF,
    0x1231, 0x0210, 0x3273, 0x2252, 0x52B5, 0x4294, 0x72F7, 0x62D6,
    0x9339, 0x8318, 0xB37B, 0xA35A, 0xD3BD, 0xC39C, 0xF3FF, 0xE3DE,
    0x2462, 0x3443, 0x0420, 0x1401, 0x64E6, 0x74C7, 0x44A4, 0x5485,
    0xA56A, 0xB54B, 0x8528, 0x9509, 0xE5EE, 0xF5CF, 0xC5AC, 0xD58D,
    0x3653, 0x2672, 0x1611, 0x0630, 0x76D7, 0x66F6, 0x5695, 0x46B4,
    0xB75B, 0xA77A, 0x9719, 0x8738, 0xF7DF, 0xE7FE, 0xD79D, 0xC7BC,
    0x48C4, 0x58E5, 0x6886, 0x78A7, 0x0840, 0x1861, 0x2802, 0x3823,
    0xC9CC, 0xD9ED, 0xE98E, 0xF9AF, 0x8948, 0x9969, 0xA90A, 0xB92B,
    0x5AF5, 0x4AD4, 0x7AB7, 0x6A96, 0x1A71, 0x0A50, 0x3A33, 0x2A12,
    0xDBFD, 0xCBDC, 0xFBBF, 0xEB9E, 0x9B79, 0x8B58, 0xBB3B, 0xAB1A,
    0x6CA6, 0x7C87, 0x4CE4, 0x5CC5, 0x2C22, 0x3C03, 0x0C60, 0x1C41,
    0xEDAE, 0xFD8F, 0xCDEC, 0xDDCD, 0xAD2A, 0xBD0B, 0x8D68, 0x9D49,
    0x7E97, 0x6EB6, 0x5ED5, 0x4EF4, 0x3E13, 0x2E32, 0x1E51, 0x0E70,
    0xFF9F, 0xEFBE, 0xDFDD, 0xCFFC, 0xBF1B, 0xAF3A, 0x9F59, 0x8F78,
    0x9188, 0x81A9, 0xB1CA, 0xA1EB, 0xD10C, 0xC12D, 0xF14E, 0xE16F,
    0x1080, 0x00A1, 0x30C2, 0x20E3, 0x5004, 0x4025, 0x7046, 0x6067,
    0x83B9, 0x9398, 0xA3FB, 0xB3DA, 0xC33D, 0xD31C, 0xE37F, 0xF35E,
    0x02B1, 0x1290, 0x22F3, 0x32D2, 0x4235, 0x5214, 0x6277, 0x7256,
    0xB5EA, 0xA5CB, 0x95A8, 0x8589, 0xF56E, 0xE54F, 0xD52C, 0xC50D,
    0x34E2, 0x24C3, 0x14A0, 0x0481, 0x7466, 0x6447, 0x5424, 0x4405,
    0xA7DB, 0xB7FA, 0x8799, 0x97B8, 0xE75F, 0xF77E, 0xC71D, 0xD73C,
    0x26D3, 0x36F2, 0x0691, 0x16B0, 0x6657, 0x7676, 0x4615, 0x5634,
    0xD94C, 0xC96D, 0xF90E, 0xE92F, 0x99C8, 0x89E9, 0xB98A, 0xA9AB,
    0x5844, 0x4865, 0x7806, 0x6827, 0x18C0, 0x08E1, 0x3882, 0x28A3,
    0xCB7D, 0xDB5C, 0xEB3F, 0xFB1E, 0x8BF9, 0x9BD8, 0xABBB, 0xBB9A,
    0x4A75, 0x5A54, 0x6A37, 0x7A16, 0x0AF1, 0x1AD0, 0x2AB3, 0x3A92,
    0xFD2E, 0xED0F, 0xDD6C, 0xCD4D, 0xBDAA, 0xAD8B, 0x9DE8, 0x8DC9,
    0x7C26, 0x6C07, 0x5C64, 0x4C45, 0x3CA2, 0x2C83, 0x1CE0, 0x0CC1,
    0xEF1F, 0xFF3E, 0xCF5D, 0xDF7C, 0xAF9B, 0xBFBA, 0x8FD9, 0x9FF8,
    0x6E17, 0x7E36, 0x4E55, 0x5E74, 0x2E93, 0x3EB2, 0x0ED1, 0x1EF0,
];

/// Calcula o CRC-16/CCITT de um array de bytes.
///
/// Utiliza uma tabela lookup para cálculo eficiente (O(1) por byte).
/// O valor inicial é 0xFFFF, sem reflexão, sem XOR final.
///
/// # Arguments
///
/// * `data` - Slice de bytes sobre o qual calcular o CRC.
///
/// # Returns
///
/// O valor CRC-16 resultante (0x0000-0xFFFF).
///
/// # Examples
///
/// ```rust
/// use bythos::protocol::crc16::calc_crc16;
///
/// let data = [0x01, 0x02, 0x03];
/// let crc = calc_crc16(&data);
/// assert!(crc <= 0xFFFF);
/// ```
pub fn calc_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        let idx = ((crc >> 8) ^ byte as u16) & 0x00FF;
        crc = (crc << 8) ^ CRC16_TABLE[idx as usize];
    }
    crc
}

/// Calcula o CRC-16/CCITT com valor inicial customizado.
///
/// Permite iniciar o CRC com um valor diferente de 0xFFFF,
/// útil para cálculos incrementais ou protocólos que definem seed.
///
/// # Arguments
///
/// * `data` - Slice de bytes.
/// * `init` - Valor inicial do CRC.
///
/// # Returns
///
/// O valor CRC-16 resultante.
pub fn calc_crc16_with_init(data: &[u8], init: u16) -> u16 {
    let mut crc: u16 = init;
    for &byte in data {
        let idx = ((crc >> 8) ^ byte as u16) & 0x00FF;
        crc = (crc << 8) ^ CRC16_TABLE[idx as usize];
    }
    crc
}

/// Verifica se um CRC-16 é válido para os dados fornecidos.
///
/// # Arguments
///
/// * `data` - Dados sobre os quais verificar o CRC.
/// * `expected_crc` - CRC esperado.
///
/// # Returns
///
/// `true` se o CRC calculado corresponde ao CRC esperado.
pub fn verify_crc16(data: &[u8], expected_crc: u16) -> bool {
    calc_crc16(data) == expected_crc
}

/// Converte CRC-16 para array de 2 bytes (little-endian).
pub fn crc16_to_bytes(crc: u16) -> [u8; 2] {
    crc.to_le_bytes()
}

/// Converte 2 bytes (little-endian) para CRC-16.
pub fn bytes_to_crc16(bytes: &[u8; 2]) -> u16 {
    u16::from_le_bytes(*bytes)
}

/// Acessa a tabela lookup de CRC-16.
///
/// Útil para debug e testes. Retorna um slice de 256 u16.
pub fn get_crc16_table() -> &'static [u16; 256] {
    &CRC16_TABLE
}

// ============================================================================
// TESTES UNITÁRIOS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_empty_data() {
        // CRC-16/CCITT de dados vazios deve ser 0xFFFF (valor inicial)
        assert_eq!(calc_crc16(&[]), 0xFFFF);
    }

    #[test]
    fn test_crc16_single_byte() {
        // CRC-16 de [0x00] com init=0xFFFF
        let crc = calc_crc16(&[0x00]);
        // idx = ((0xFFFF >> 8) ^ 0x00) & 0xFF = (0xFF ^ 0x00) & 0xFF = 0xFF
        // crc = (0xFFFF << 8) ^ TABLE[0xFF] = 0xFF00 ^ TABLE[0xFF]
        assert_eq!(crc, (0xFFFFu16 << 8) ^ CRC16_TABLE[0xFF]);
    }

    #[test]
    fn test_crc16_known_vectors() {
        // Valores conhecidos CRC-16/CCITT (polinómio 0x1021, init 0xFFFF)
        // String "123456789" deve produzir 0x29B1
        let test_data = b"123456789";
        assert_eq!(calc_crc16(test_data), 0x29B1);

        // Dados ASCII "CRC"
        let crc_data = b"CRC";
        let _crc = calc_crc16(crc_data);
    }

    #[test]
    fn test_crc16_incremental() {
        // O CRC de N bytes deve ser igual ao CRC incremental
        let data1 = [0x01, 0x02];
        let data2 = [0x03];
        let combined = [0x01, 0x02, 0x03];

        let crc1 = calc_crc16(&data1);
        // CRC incremental: aplicar CRC sobre o CRC anterior com novos bytes
        let mut crc_inc = crc1;
        for &byte in &data2 {
            let idx = ((crc_inc >> 8) ^ byte as u16) & 0x00FF;
            crc_inc = (crc_inc << 8) ^ CRC16_TABLE[idx as usize];
        }

        assert_eq!(crc_inc, calc_crc16(&combined));
    }

    #[test]
    fn test_verify_crc16() {
        let data = [0x01, 0x02, 0x03];
        let crc = calc_crc16(&data);
        assert!(verify_crc16(&data, crc));
        assert!(!verify_crc16(&data, crc.wrapping_add(1)));
    }

    #[test]
    fn test_crc16_bytes_conversion() {
        let crc: u16 = 0x1234;
        let bytes = crc16_to_bytes(crc);
        assert_eq!(bytes, [0x34, 0x12]); // little-endian
        let recovered = bytes_to_crc16(&bytes);
        assert_eq!(recovered, crc);
    }

    #[test]
    fn test_crc16_table_lookup() {
        let table = get_crc16_table();
        assert_eq!(table.len(), 256);
        // TABLE[0] = 0x0000 (correto para CRC-16/CCITT-FALSE)
        assert_eq!(table[0], 0x0000);
        // TABLE[1] = 0x1021 (polinómio aplicado ao byte 0x01)
        assert_eq!(table[1], 0x1021);
        // Verificar que a tabela não é toda zeros
        assert_ne!(table[0x80], 0x0000);
    }

    #[test]
    fn test_crc16_with_init() {
        let data = [0x01, 0x02, 0x03];
        // Com init=0x0000 deve dar resultado diferente de init=0xFFFF
        let crc_default = calc_crc16(&data);
        let crc_custom = calc_crc16_with_init(&data, 0x0000);
        assert_ne!(crc_default, crc_custom);

        // Com init=0xFFFF deve ser igual ao padrão
        let crc_same = calc_crc16_with_init(&data, 0xFFFF);
        assert_eq!(crc_default, crc_same);
    }

    #[test]
    fn test_crc16_acp_header() {
        // Simular CRC de uma mensagem ACP: start(0xAA) + version(0x03) + nodeId(0x06) + msgID(0x10)
        let header = [0xAA, 0x03, 0x06, 0x10];
        let crc = calc_crc16(&header);
        // CRC não deve ser zero para estes dados específicos
        assert_ne!(crc, 0x0000);
        assert_ne!(crc, 0xFFFF);
    }

    #[test]
    fn test_crc16_message_consistency() {
        // Construir uma mensagem ACP manualmente e verificar CRC
        let mut msg = [0u8; 20];
        msg[0] = 0xAA; // START
        msg[1] = 0x03; // VERSION
        msg[2] = 0x06; // NODE_ID (Device5)
        msg[3] = 0x11; // MSG_ID (Telemetry)
        msg[4] = 0x2A; // SEQ_LO
        msg[5] = 0x00; // SEQ_HI
        msg[6] = 0x01; // TLV_COUNT

        // Campo TLV: type=6(u8), id=0 → FieldID=0xC0, len=1, data=0x02
        msg[7] = 0xC0; // FieldID
        msg[8] = 0x01; // Length
        msg[9] = 0x02; // Data

        // Signature
        msg[10] = 0x42 ^ 0x11 ^ 0x2A ^ 0x00;

        // CRC16 sobre bytes 0..11
        let crc = calc_crc16(&msg[..11]);
        msg[11] = (crc & 0xFF) as u8;
        msg[12] = ((crc >> 8) & 0xFF) as u8;

        // Verificar CRC
        assert!(verify_crc16(&msg[..11], crc));
    }
}
