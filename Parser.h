#pragma once

/**
 * =================================================================================
 * PARSER.H — PARSER BYTE-A-BYTE DE MENSAGENS TLV
 * =================================================================================
 * 
 * AUTOR:      ShegaPT
 * DATA:       2026-04-17
 * VERSÃO:     2.0.0
 * 
 * =================================================================================
 * DESCRIÇÃO GERAL
 * =================================================================================
 * 
 * Este módulo reconstrói mensagens TLV completas a partir de um stream contínuo
 * de bytes (UART entre ESP1↔ESP2 ou LoRa 2.4GHz da Ground Station).
 * 
 * Implementa uma Máquina de Estados Finita (FSM) deterministico, tolerante a
 * erros e com proteção contra timeout entre bytes.
 * 
 * =================================================================================
 * ARQUITETURA FUNDAMENTAL
 * =================================================================================
 * 
 *   rawBuffer[]          → acumula bytes brutos do canal físico
 *   TLVMessage (struct)  → resultado final estruturado
 * 
 * IMPORTANTE: Nunca fazer cast direto de rawBuffer para TLVMessage.
 *             A struct TLVMessage tem padding e alinhamento interno
 *             que diferem de um buffer plano → comportamento indefinido.
 * 
 * =================================================================================
 * FLUXO COMPLETO DE RECEÇÃO (no loop principal do ESP1/ESP2)
 * =================================================================================
 * 
 *   1. parser.feed(byte)          → acumula e processa byte a byte
 *   2. feed() retorna 1           → mensagem completa e estruturalmente válida
 *   3. Security::verifyPacket()   → valida HMAC + sequência + autenticação GS
 *   4. Processar a mensagem:
 *        • MSG_TELEMETRY → FlightControl::updateState()
 *        • MSG_COMMAND   → FlightControl::processCommand() / Shell
 *        • MSG_FAILSAFE  → Actuators::failsafeBlock()/failsafeClear()
 *        • MSG_SHELL_CMD → Shell::processShellCommand()
 * 
 * O Parser NÃO faz validação de segurança (HMAC/SEQ) — isso é responsabilidade
 * exclusiva do módulo Security. O Parser apenas garante integridade estrutural
 * e CRC8.
 * 
 * =================================================================================
 * MÁQUINA DE ESTADOS
 * =================================================================================
 * 
 *   WAIT_START ──(0xAA)──→ WAIT_MSGID ──(msgID válido)──→ WAIT_TLVCOUNT
 *                                                                │
 *                                                (tlvCount==0)   │ (tlvCount>0)
 *                                                        ↓        ↓
 *                                              WAIT_CHECKSUM   WAIT_TLV_ID
 *                                                    ↑              │
 *                                                    │              ↓
 *                                              (após CRC)    WAIT_TLV_LEN
 *                                                    ↑              │
 *                                                    │              ↓
 *                                              (último TLV)   WAIT_TLV_DATA
 *                                                    ↑              │
 *                                                    └──────────────┘
 * 
 * =================================================================================
 */

#include "Protocol.h"
#include <stddef.h>
#include <stdint.h>

/* =================================================================================
 * ESTADOS DA MÁQUINA DE ESTADOS FINITA (FSM)
 * =================================================================================
 */

enum ParserState : uint8_t {
    PARSER_WAIT_START     = 0,   /* Aguarda START_BYTE (0xAA) — estado inicial */
    PARSER_WAIT_MSGID     = 1,   /* Aguarda msgID, valida imediatamente */
    PARSER_WAIT_TLVCOUNT  = 2,   /* Aguarda número de campos TLV */
    PARSER_WAIT_TLV_ID    = 3,   /* Aguarda ID do TLV atual */
    PARSER_WAIT_TLV_LEN   = 4,   /* Aguarda LEN do TLV atual */
    PARSER_WAIT_TLV_DATA  = 5,   /* Aguarda dados do TLV atual */
    PARSER_WAIT_CHECKSUM  = 6    /* Aguarda CRC8 final e valida */
};

/* =================================================================================
 * CÓDIGOS DE ERRO PARA DIAGNÓSTICO
 * =================================================================================
 */

enum ParserError : uint8_t {
    PARSER_OK                = 0,   /* Sem erro — operação normal */
    PARSER_ERR_OVERFLOW      = 1,   /* Buffer interno excedido (MAX_MESSAGE_SIZE) */
    PARSER_ERR_TIMEOUT       = 2,   /* Gap entre bytes excedeu maxFrameGapMicros */
    PARSER_ERR_INVALID_START = 3,   /* START_BYTE incorreto ou msgID inválido */
    PARSER_ERR_CHECKSUM      = 4,   /* CRC8 inválido — corrupção de dados */
    PARSER_ERR_TLV_COUNT     = 5,   /* tlvCount > MAX_TLV_FIELDS */
    PARSER_ERR_TLV_LEN       = 6    /* LEN de TLV > MAX_TLV_VIDEO_DATA */
};

/* =================================================================================
 * CLASSE PARSER
 * =================================================================================
 * 
 * Implementação thread-safe (desde que chamada de um único contexto).
 * Não utiliza alocações dinâmicas — todos os buffers são estáticos.
 */

class Parser {
public:
    /**
     * Construtor — inicializa o parser no estado WAIT_START
     */
    Parser();

    /**
     * Alimenta um byte recebido ao parser
     * 
     * Esta é a função principal do parser. Deve ser chamada para cada byte
     * recebido do canal físico (UART ou LoRa).
     * 
     * @param byte  Byte recebido do canal físico
     * @return uint8_t  
     *         1 = mensagem completa e estruturalmente válida (pronta para Security)
     *         0 = ainda a aguardar mais bytes ou erro (reset automático)
     * 
     * NOTA: Quando retorna 1, a mensagem está disponível via getMessage()
     *       ou copyMessage(). Deve-se chamar acknowledge() após processar.
     */
    uint8_t feed(uint8_t byte);

    /**
     * Verifica se existe uma mensagem completa disponível
     * 
     * @return uint8_t  1 = mensagem disponível, 0 = nenhuma mensagem
     */
    uint8_t hasMessage() const;

    /**
     * Retorna ponteiro para a última mensagem válida
     * 
     * NOTA: O ponteiro aponta para buffer interno. Copie os dados
     *       imediatamente ou use copyMessage() se precisar guardar.
     *       O conteúdo pode ser sobrescrito na próxima chamada de feed().
     * 
     * @return TLVMessage*  Ponteiro para a mensagem (ou nullptr se não disponível)
     */
    TLVMessage* getMessage();

    /**
     * Copia a mensagem atual para um buffer externo (mais seguro)
     * 
     * Esta é a forma recomendada de obter a mensagem, pois não depende
     * da persistência do buffer interno.
     * 
     * @param output  Ponteiro para TLVMessage de destino
     * @return uint8_t  1 = sucesso, 0 = falha (output nullptr ou sem mensagem)
     */
    uint8_t copyMessage(TLVMessage* output);

    /**
     * Define o timeout máximo entre bytes (em microssegundos)
     * 
     * Se o intervalo entre bytes recebidos exceder este valor, o parser
     * reinicia automaticamente (descartando a mensagem incompleta).
     * 
     * @param micros  Timeout em microssegundos (0 = timeout desativado)
     */
    void setMaxFrameGap(uint32_t micros);

    /**
     * Reset completo do parser
     * 
     * Limpa todos os buffers, estatísticas e volta ao estado WAIT_START.
     * Útil após um erro irrecuperável ou para reiniciar o sistema.
     */
    void reset();

    /**
     * Confirma a mensagem atual e prepara para a próxima
     * 
     * Deve ser chamada APÓS processar a mensagem (e após a validação Security).
     * Liberta o parser para aceitar a próxima mensagem.
     */
    void acknowledge();

    /* =========================================================================
     * FUNÇÕES DE DIAGNÓSTICO
     * ========================================================================= */

    /**
     * Verifica se ocorreu timeout na receção da mensagem atual
     * 
     * @return uint8_t  1 = timeout ocorreu, 0 = dentro do prazo
     */
    uint8_t isTimedOut() const;

    /**
     * Retorna o último erro ocorrido
     * 
     * @return ParserError  Código do erro (PARSER_OK se nenhum erro)
     */
    ParserError getLastError() const;

    /**
     * Retorna o número total de mensagens processadas com sucesso
     * 
     * @return uint32_t  Contador de sucessos (nunca reset a não ser pelo construtor)
     */
    uint32_t getSuccessCount() const;

    /**
     * Retorna o número total de erros ocorridos
     * 
     * @return uint32_t  Contador de erros (nunca reset a não ser pelo construtor)
     */
    uint32_t getErrorCount() const;

    /**
     * Retorna o estado atual da máquina de estados
     * 
     * @return ParserState  Estado atual (útil para debug)
     */
    ParserState getCurrentState() const;

private:
    /* Buffer bruto de receção — acumula os bytes da mensagem */
    uint8_t rawBuffer[MAX_MESSAGE_SIZE];
    size_t  rawOffset;              /* Posição atual no buffer */

    /* Mensagem estruturada final — preenchida após validação */
    TLVMessage msg;

    /* Máquina de estados */
    ParserState state;              /* Estado atual da FSM */
    ParserError lastError;          /* Último erro ocorrido */

    /* Controlo de TLVs durante a receção */
    uint8_t expectedTLVs;           /* Número de TLVs esperados (do campo COUNT) */
    uint8_t currentTLV;             /* Índice do TLV atual (0..expectedTLVs-1) */
    uint8_t tlvBytesReceived;       /* Bytes já recebidos do TLV atual */
    uint8_t currentTLVLen;          /* Comprimento esperado do TLV atual */

    /* Timeout entre bytes */
    uint32_t lastActivityTime;      /* Timestamp do último byte recebido (micros) */
    uint32_t maxFrameGapMicros;     /* Timeout máximo configurado (0 = desativado) */

    /* Estatísticas */
    uint32_t successCount;          /* Total de mensagens válidas recebidas */
    uint32_t errorCount;            /* Total de erros ocorridos */

    /* Flag de mensagem pronta */
    bool messageReady;              /* true = há uma mensagem completa disponível */

    /* Métodos privados */
    void updateTimestamp();         /* Atualiza lastActivityTime com o tempo atual */
    void setError(ParserError error); /* Regista erro e faz reset */
};

/* =================================================================================
 * FUNÇÕES AUXILIARES PARA DEBUG
 * =================================================================================
 */

/**
 * Converte um estado da FSM para string legível
 * 
 * @param state  Estado a converter
 * @return const char*  String descritiva (ex: "WAIT_START")
 */
const char* parserStateToString(ParserState state);

/**
 * Converte um código de erro para string legível
 * 
 * @param error  Código de erro a converter
 * @return const char*  String descritiva (ex: "CHECKSUM")
 */
const char* parserErrorToString(ParserError error);