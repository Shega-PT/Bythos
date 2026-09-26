//! # Anel Bythos — Enumeração e Encaminhamento (v4.0.0)
//!
//! Este módulo é a "geografia" do protocolo: quem sou, quantos somos e para que
//! lado segue cada trama. Três responsabilidades, sem tocar em chaves nem no fio:
//!
//! 1. **Enumeração** (`Enumeration`): máquina de estados que numera o nó por
//!    ordem física a partir do `BG-0`, sem declarar endereços à mão.
//! 2. **Encaminhamento** (`decide`): dado `eu`, `N` e `destino`, escolhe o sentido
//!    com menos saltos — é isto que permite `BG-27 → BG-2` viajar para trás.
//! 3. **Supervisão** (`RingMonitor`): deteta anel partido (relógio que não volta)
//!    e ordena a queda para modo linha.
//!
//! O transporte físico (relógio diferencial, RS-485, RJ) vive na PCB
//! `BythosBridge` (ver `hardware/bythos-bridge/README.md`); aqui vive só a decisão.

use crate::protocol::types::{BROADCAST_ADDR, HOP_DEFAULT, ROOT_ADDR};

// ============================================================================
// SENTIDO NO ANEL
// ============================================================================

/// Lado físico de saída de uma trama a retransmitir.
///
/// `After` = porta a jusante (sentido horário, o do relógio); `Before` = porta
/// a montante (sentido anti-horário). Numa linha partida, um dos lados está
/// morto e o monitor força tudo para o lado vivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Para a frente (porta `AFTER`, sentido do relógio).
    After,
    /// Para trás (porta `BEFORE`, contra o relógio).
    Before,
    /// Consome localmente (sou o destino ou difusão que termina aqui).
    Local,
}

/// Decide o sentido de uma trama: o caminho mais curto no anel.
///
/// - Difusão ou destino próprio → `Local` (consome; a ponte repete à parte se
///   não for fim de linha — decisão de repetição, não de sentido).
/// - Caso contrário compara os saltos nos dois sentidos e escolhe o menor;
///   em empate prefere `After` (o sentido do relógio, com temporização mais
///   previsível por viajar com o relógio e não contra ele).
/// - `total == 0` (anel ainda sem `Count`) → `Local`: sem geografia não se
///   encaminha, consome-se ou descarta-se em cima.
pub fn decide(me: u16, total: u16, dst: u16) -> Direction {
    if dst == BROADCAST_ADDR || dst == me || total == 0 {
        return Direction::Local;
    }
    // Aritmética modular: funciona com qualquer `N` até 65535, sem tabelas.
    let total32 = total as u32;
    let cw = (dst as u32 + total32 - me as u32) % total32;
    let ccw = (me as u32 + total32 - dst as u32) % total32;
    if cw <= ccw {
        Direction::After
    } else {
        Direction::Before
    }
}

/// Saltos do sentido escolhido (para diagnóstico e para `Ping`).
///
/// Devolve `(sentido, saltos)`: útil para a estação estimar latência
/// (`saltos × ~2 µs`) antes de pedir um comando com prazo.
pub fn hops_to(me: u16, total: u16, dst: u16) -> (Direction, u32) {
    if dst == BROADCAST_ADDR || dst == me || total == 0 {
        return (Direction::Local, 0);
    }
    let total32 = total as u32;
    let cw = (dst as u32 + total32 - me as u32) % total32;
    let ccw = (me as u32 + total32 - dst as u32) % total32;
    if cw <= ccw {
        (Direction::After, cw)
    } else {
        (Direction::Before, ccw)
    }
}

// ============================================================================
// ENUMERAÇÃO AUTOMÁTICA
// ============================================================================

/// Papel da ponte na enumeração (lido do jumper em hardware).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeRole {
    /// Gera relógio, injeta `Hello{0}`, fecha o anel. Um só por anel.
    Root,
    /// Numerado automaticamente por ordem física.
    Node,
}

/// Fase da enumeração vista por um `Node`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnumPhase {
    /// À espera do primeiro `Hello` (endereço ainda inválido).
    Listening,
    /// Numerado, à espera do `Count{N}` que fecha a geografia.
    Numbered,
    /// Operacional: endereço e total conhecidos.
    Ready,
}

/// Estado de enumeração de um nó (máquina sem temporizadores próprios).
///
/// Os temporizadores (retransmitir `Hello`, expirar `Count`) vivem no firmware
/// da ponte, que tem o relógio; aqui vive só a transição — testável sem hardware.
#[derive(Debug, Clone, Copy)]
pub struct Enumeration {
    /// Papel (jumper).
    pub role: NodeRole,
    /// Fase atual.
    pub phase: EnumPhase,
    /// Endereço assumido (`u16::MAX` = ainda sem).
    pub addr: u16,
    /// Total do anel (`0` = desconhecido).
    pub total: u16,
}

impl Enumeration {
    /// Estado inicial: raiz pronta a emitir, nó a escutar.
    pub fn new(role: NodeRole) -> Self {
        match role {
            NodeRole::Root => Self {
                role,
                phase: EnumPhase::Ready,
                addr: ROOT_ADDR,
                total: 0,
            },
            NodeRole::Node => Self {
                role,
                phase: EnumPhase::Listening,
                addr: u16::MAX,
                total: 0,
            },
        }
    }

    /// Processa um `Hello{pos}` recebido em `BEFORE`; devolve a posição a
    /// repetir em `AFTER` (`pos + 1`), ou `None` se não há nada a fazer.
    ///
    /// O `+1` com enrolamento protege contra anel patológico com mais de 65535
    /// nós (impossível na prática): em vez de enrolar para `BG-0` e colidir com
    /// a raiz, satura em `0xFFFE` e sinaliza — colisão silenciosa, nunca.
    pub fn on_hello(&mut self, pos: u16) -> Option<u16> {
        if self.role == NodeRole::Root {
            // A raiz não se renumera; o `Hello` que lhe volta fecha o anel
            // (tratado em `on_hello_return`).
            return None;
        }
        let mine = pos.wrapping_add(1);
        let mine = if mine == BROADCAST_ADDR || mine == ROOT_ADDR {
            0xFFFE
        } else {
            mine
        };
        self.addr = mine;
        self.phase = EnumPhase::Numbered;
        Some(mine)
    }

    /// A raiz fecha o anel quando o seu `Hello` regressa: o total é a posição
    /// regressada + 1 (a raiz conta-se a si mesma).
    pub fn on_hello_return(&mut self, pos: u16) -> Option<u16> {
        if self.role != NodeRole::Root {
            return None;
        }
        let total = pos.wrapping_add(1);
        self.total = total;
        Some(total)
    }

    /// Processa `Count{N}`: fecha a geografia e liberta o encaminhamento.
    pub fn on_count(&mut self, total: u16) -> bool {
        if total == 0 {
            return false;
        }
        self.total = total;
        if self.role == NodeRole::Root || self.phase == EnumPhase::Numbered {
            self.phase = EnumPhase::Ready;
            return true;
        }
        false
    }

    /// `true` se pode encaminhar (geografia completa).
    pub fn is_ready(&self) -> bool {
        self.phase == EnumPhase::Ready && self.total != 0
    }

    /// Perda de relógio/ligação: volta a escutar (re-enumeração a jusante).
    ///
    /// Não limpa o endereço de imediato — se o corte for transitório, a ponte
    /// continua a responder pelo antigo até o novo `Hello` chegar, evitando
    /// flap de identidade no resto do anel.
    pub fn on_link_loss(&mut self) {
        if self.role == NodeRole::Node {
            self.phase = EnumPhase::Listening;
            self.total = 0;
        }
    }
}

// ============================================================================
// SUPERVISÃO DO ANEL
// ============================================================================

/// Modo de encaminhamento ditado pela saúde do anel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RingMode {
    /// Anel fechado: sentido mais curto.
    Closed,
    /// Anel partido: tudo pelo lado vivo (`fallback`).
    Open,
    /// Ilha isolada: sem relógio, só assíncrono local.
    Island,
}

/// Monitor de saúde do anel (a ponte alimenta-o com eventos físicos).
///
/// - `on_clock_return`: o relógio da raiz voltou a `BEFORE` → anel fechado.
/// - `on_clock_timeout`: silêncio além do limite → partido.
/// - `on_side_down(side)`: porta física caída → modo linha pelo outro lado.
/// - `hops`: limite a carimbar nas tramas emitidas (cai em modo linha para
///   não atravessar o corte e voltar pela outra ponta — que não existe).
#[derive(Debug, Clone, Copy)]
pub struct RingMonitor {
    /// Modo atual.
    pub mode: RingMode,
    /// Lado vivo em modo linha (`None` = ambos / fechado).
    pub live_side: Option<Direction>,
}

impl RingMonitor {
    /// Começa fechado por otimismo? Não — começa em `Island` até o primeiro
    /// batimento provar o anel. Otimismo em supervisão é como se perdem frotas.
    pub fn new() -> Self {
        Self {
            mode: RingMode::Island,
            live_side: None,
        }
    }

    /// Batimento do relógio: anel provado fechado.
    pub fn on_clock_return(&mut self) {
        self.mode = RingMode::Closed;
        self.live_side = None;
    }

    /// Silêncio de relógio: partido até prova em contrário.
    pub fn on_clock_timeout(&mut self) {
        if self.mode == RingMode::Closed {
            self.mode = RingMode::Open;
        }
    }

    /// Porta caída: modo linha pelo lado oposto.
    pub fn on_side_down(&mut self, dead: Direction) {
        self.mode = RingMode::Open;
        self.live_side = Some(match dead {
            Direction::After => Direction::Before,
            Direction::Before => Direction::After,
            Direction::Local => Direction::After,
        });
    }

    /// Encaminhamento efetivo: em anel fechado decide pelo mais curto; em modo
    /// linha força o lado vivo; em ilha só consome local.
    pub fn route(&self, me: u16, total: u16, dst: u16) -> Direction {
        match self.mode {
            RingMode::Closed => decide(me, total, dst),
            RingMode::Open => {
                if dst == BROADCAST_ADDR || dst == me {
                    Direction::Local
                } else {
                    self.live_side.unwrap_or(Direction::After)
                }
            }
            RingMode::Island => Direction::Local,
        }
    }

    /// Saltos a carimbar: cheios em anel, curtos em linha (não há volta).
    pub fn emit_hops(&self) -> u8 {
        match self.mode {
            RingMode::Closed => HOP_DEFAULT,
            _ => HOP_DEFAULT / 2,
        }
    }
}

impl Default for RingMonitor {
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
    fn test_decide_sentido_mais_curto() {
        // Convenção de portas: `After` = números crescentes (sentido do relógio),
        // `Before` = decrescentes. O que importa é serem os saltos mínimos.
        // BG-27 → BG-2 em anel de 30: 27→28→29→0→1→2 = 5 saltos (After).
        assert_eq!(decide(27, 30, 2), Direction::After);
        // O par inverso: 2→1→0→29→28→27 = 5 saltos (Before).
        assert_eq!(decide(2, 30, 27), Direction::Before);
        // Vizinho à frente: After.
        assert_eq!(decide(5, 30, 6), Direction::After);
        // Vizinho atrás: Before.
        assert_eq!(decide(5, 30, 4), Direction::Before);
        // Empate (anel de 10, 5 saltos cada lado): prefere After.
        assert_eq!(decide(0, 10, 5), Direction::After);
        // Difusão, próprio e sem geografia: Local.
        assert_eq!(decide(5, 30, BROADCAST_ADDR), Direction::Local);
        assert_eq!(decide(5, 30, 5), Direction::Local);
        assert_eq!(decide(5, 0, 9), Direction::Local);
    }

    #[test]
    fn test_hops_to_conta_bem() {
        // 27 → 2 em anel de 30: 5 pela frente, 25 por trás.
        assert_eq!(hops_to(27, 30, 2), (Direction::After, 5));
        assert_eq!(hops_to(2, 30, 27), (Direction::Before, 5));
        assert_eq!(hops_to(0, 10, 5), (Direction::After, 5));
    }

    #[test]
    fn test_enumeracao_sequencial() {
        // Cadeia BG-0 → A → B: o Hello soma 1 por salto.
        let mut a = Enumeration::new(NodeRole::Node);
        let mut b = Enumeration::new(NodeRole::Node);
        assert_eq!(a.on_hello(0), Some(1));
        assert_eq!(a.addr, 1);
        assert_eq!(b.on_hello(1), Some(2));
        assert_eq!(b.addr, 2);
        // Ainda sem Count: não encaminha.
        assert!(!a.is_ready());
        assert!(a.on_count(3));
        assert!(a.is_ready());
        assert_eq!(decide(a.addr, a.total, 2), Direction::After);
    }

    #[test]
    fn test_raiz_fecha_o_anel() {
        // O Hello da raiz dá a volta com pos=2 → total 3.
        let mut root = Enumeration::new(NodeRole::Root);
        assert_eq!(root.addr, ROOT_ADDR);
        assert_eq!(root.on_hello_return(2), Some(3));
        assert_eq!(root.total, 3);
        // Raiz ignora Hello alheio (não se renumera).
        assert_eq!(root.on_hello(7), None);
        assert_eq!(root.addr, ROOT_ADDR);
    }

    #[test]
    fn test_corte_e_reenumeracao() {
        // Perda de ligação: volta a escutar, mantém endereço até novo Hello.
        let mut n = Enumeration::new(NodeRole::Node);
        n.on_hello(4);
        n.on_count(30);
        assert!(n.is_ready());
        n.on_link_loss();
        assert!(!n.is_ready());
        assert_eq!(n.addr, 5); // Identidade retida contra flap.
    }

    #[test]
    fn test_monitor_modo_linha() {
        // Sem batimento nasce ilha; com batimento fecha; com corte abre.
        let mut m = RingMonitor::new();
        assert_eq!(m.route(5, 30, 9), Direction::Local);
        m.on_clock_return();
        assert_eq!(m.route(27, 30, 2), Direction::After);
        m.on_side_down(Direction::After);
        assert_eq!(m.mode, RingMode::Open);
        assert_eq!(m.route(27, 30, 2), Direction::Before); // Lado vivo.
        m.on_clock_timeout(); // Já aberto: mantém.
        assert_eq!(m.mode, RingMode::Open);
        assert_eq!(m.emit_hops(), HOP_DEFAULT / 2);
    }
}
