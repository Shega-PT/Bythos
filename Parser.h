#pragma once

/**
 * =================================================================================
 * PARSER.H — PARSER BYTE-A-BYTE DE MENSAGENS TLV (Bythos Protocol V1)
 * =================================================================================
 *
 * AUTOR:      ShegaPT
 * DATA:       2026-04-17
 * VERSÃO:     1.0.0
 *
 * =================================================================================
 * DESCRIÇÃO GERAL
 * =================================================================================
 *
 * Este módulo implementa a reconstrução de mensagens TLV completas a partir
 * de um stream contínuo de bytes recebidos via UART. Utiliza uma Máquina
 * de Estados Finita (FSM) determinística com 7 estados para processamento
 * byte-a-byte em tempo real.
 *
 * Características principais:
 *   • Processamento byte-a-byte — sem buffers intermédios desnecessários
 *   • Tolerância a erros — reset automático em qualquer invalidade
 *   • Proteção contra timeout — deteção de gaps entre bytes
 *   • Sem alocação dinâmica — toda a memória é estática e preditível
 *   • Compatível com ESP32 e Arduino (via condicionais de compilação)
 *
 * =================================================================================
 * ARQUITETURA FUNDAMENTAL
 * =================================================================================
 *
 *   rawBuffer[256]      → acumula bytes brutos do canal UART
 *   TLVMessage (struct) → resultado final estruturado após validação
 *
 * IMPORTANTE: Nunca fazer cast direto de rawBuffer para TLVMessage.
 *             A struct TLVMessage tem padding e alinhamento interno
 *             que diferem de um buffer plano → comportamento indefinido.
 *
 * =================================================================================
 * FLUXO COMPLETO DE RECEÇÃO (no loop principal do dispositivo)
 * =================================================================================
 *
 *   1. parser.feed(byte)          → acumula e processa byte a byte
 *   2. feed() retorna 1           → mensagem completa e estruturalmente válida
 *   3. Processar a mensagem:
 *        • MSG_HEARTBEAT → verificar ligação
 *        • MSG_TELEMETRY → processar dados
 *        • MSG_COMMAND   → executar comando
 *        • MSG_FAILSAFE  → tratar emergência
 *        • MSG_DEBUG     → log/diagnóstico
 *   4. parser.acknowledge()       → libertar parser para a próxima mensagem
 *
 * O Parser NÃO faz任何形式 de autenticação — isso é responsabilidade
 * de uma camada superior se necessário. O Parser garante apenas
 * integridade estrutural e CRC8.
 *
 * =================================================================================
 * MÁQUINA DE ESTADOS (FSM)
 * =================================================================================
 *
 *   WAIT_START ──(0xAA)──→ WAIT_MSGID ──(msgID válido)──→ WAIT_TLVCOUNT
 *                                                          │
 *                                          (tlvCount==0)   │ (tlvCount>0)
 *                                                  v        v
 *                                        WAIT_CHECKSUM   WAIT_TLV_ID
 *                                              ↑              │
 *                                              │              ↓
 *                                        (após CRC)    WAIT_TLV_LEN
 *                                              ↑              │
 *                                              │              ↓
 *                                        (último TLV)   WAIT_TLV_DATA
 *                                              ↑              │
 *                                              └──────────────┘
 *
 * =================================================================================
 */

#include "Protocol.h"
#include <stddef.h>
#include <stdint.h>

/* =================================================================================
 * ESTADOS DA MÁQUINA DE ESTADOS FINITA (FSM)
 * =================================================================================
 *
 * Cada estado representa uma fase do processamento de uma trama.
 * A transição entre estados é determinística e baseada no byte recebido.
 * Qualquer invalidade resulta em reset para WAIT_START.
 */

enum ParserState : uint8_t {
    PARSER_WAIT_START    = 0,   /* Estado inicial — aguarda START_BYTE (0xAA) */
    PARSER_WAIT_MSGID    = 1,   /* Aguarda e valida o byte de msgID */
    PARSER_WAIT_TLVCOUNT = 2,   /* Aguarda o número de campos TLV */
    PARSER_WAIT_TLV_ID   = 3,   /* Aguarda o ID do campo TLV atual */
    PARSER_WAIT_TLV_LEN  = 4,   /* Aguarda o comprimento do payload do TLV atual */
    PARSER_WAIT_TLV_DATA = 5,   /* Acumula os bytes do payload do TLV atual */
    PARSER_WAIT_CHECKSUM = 6    /* Aguarda o CRC8 final e valida a trama */
};

/* =================================================================================
 * CÓDIGOS DE ERRO PARA DIAGNÓSTICO
 * =================================================================================
 *
 * Cada código identifica unicamente uma causa de falha.
 * Permitem diagnóstico remoto e logging para análise de problemas.
 */

enum ParserError : uint8_t {
    PARSER_OK                = 0,   /* Operação sem erros */
    PARSER_ERR_OVERFLOW      = 1,   /* Buffer interno excedeu MAX_MESSAGE_SIZE */
    PARSER_ERR_TIMEOUT       = 2,   /* Gap entre bytes excedeu o timeout configurado */
    PARSER_ERR_INVALID_START = 3,   /* START_BYTE incorreto ou msgID fora do range */
    PARSER_ERR_CHECKSUM      = 4,   /* CRC8 recebido não corresponde ao calculado */
    PARSER_ERR_TLV_COUNT     = 5,   /* tlvCount recebido > MAX_TLV_FIELDS */
    PARSER_ERR_TLV_LEN       = 6    /* LEN de TLV excedeu MAX_TLV_DATA */
};

/* =================================================================================
 * CLASSE PARSER — MÁQUINA DE ESTADOS PARA BYTE STREAM
 * =================================================================================
 *
 * Implementação thread-safe (desde que chamada de um único contexto).
 * Todos os buffers são estáticos — sem alocação dinâmica em runtime.
 *
 * Ciclo de vida típico:
 *   1. Construtor → inicializa em WAIT_START
 *   2. feed(byte) → processa cada byte recebido do UART
 *   3. feed() == 1 → mensagem disponível via getMessage() ou copyMessage()
 *   4. acknowledge() → processou a mensagem, libertar para a próxima
 *
 * NOTA: O timeout entre bytes é configurável (default: 100ms).
 *       Se excedido, o parser reinicia automaticamente.
 */

class Parser {
public:
    /**
     * Construtor — inicializa o parser no estado WAIT_START.
     *
     * Todos os buffers são zero-init e o timeout é definido para 100ms.
     */
    Parser();

    /**
     * Alimenta um byte recebido do canal UART ao parser.
     *
     * Esta é a função principal do parser — deve ser chamada para
     * cada byte recebido. Implementa toda a lógica da FSM.
     *
     * @param byte Byte recebido do UART
     * @return uint8_t
     *         1 = mensagem completa e estruturalmente válida
     *         0 = aguardando mais bytes ou erro (reset automático)
     *
     * Quando retorna 1, a mensagem está disponível via getMessage()
     * ou copyMessage(). Chamar acknowledge() após processar.
     */
    uint8_t feed(uint8_t byte);

    /**
     * Verifica se existe uma mensagem completa disponível.
     *
     * @return uint8_t 1 = mensagem disponível, 0 = nenhuma
     */
    uint8_t hasMessage() const;

    /**
     * Retorna ponteiro para a última mensagem válida.
     *
     * ATENÇÃO: O ponteiro aponta para buffer interno. O conteúdo
     * pode ser sobrescrito na próxima chamada de feed(). Para uso
     * seguro, preferir copyMessage().
     *
     * @return TLVMessage* Ponteiro para a mensagem (ou nullptr)
     */
    TLVMessage* getMessage();

    /**
     * Copia a mensagem atual para um buffer externo (forma segura).
     *
     * Esta é a forma recomendada de obter a mensagem pois não
     * depende da persistência do buffer interno do parser.
     *
     * @param output Ponteiro para TLVMessage de destino
     * @return uint8_t 1 = sucesso, 0 = falha
     */
    uint8_t copyMessage(TLVMessage* output);

    /**
     * Define o timeout máximo entre bytes (em microssegundos).
     *
     * Se o intervalo entre bytes recebidos exceder este valor,
     * o parser reinicia automaticamente (descarta mensagem incompleta).
     *
     * @param micros Timeout em µs (0 = timeout desativado)
     */
    void setMaxFrameGap(uint32_t micros);

    /**
     * Ativa ou desativa mensagens de debug no parser.
     *
     * @param enable true = ativar, false = desativar
     */
    void setDebug(bool enable);

    /**
     * Reset completo do parser.
     *
     * Limpa todos os buffers, reinicia a FSM e mantém as estatísticas
     * (successCount/errorCount) para monitorização de longo prazo.
     */
    void reset();

    /**
     * Confirma a mensagem e prepara o parser para a próxima.
     *
     * Deve ser chamada APÓS processar a mensagem recebida.
     * Liberta o parser para aceitar uma nova trama.
     */
    void acknowledge();

    /* =================================================================
     * FUNÇÕES DE DIAGNÓSTICO
     * =================================================================
     * Permitem monitorizar o estado e desempenho do parser em runtime.
     */

    /**
     * Verifica se ocorreu timeout na receção da mensagem atual.
     *
     * @return uint8_t 1 = timeout ocorreu, 0 = dentro do prazo
     */
    uint8_t isTimedOut() const;

    /**
     * Retorna o último código de erro registado.
     *
     * @return ParserError Código do erro (PARSER_OK se nenhum)
     */
    ParserError getLastError() const;

    /**
     * Retorna o número total de mensagens processadas com sucesso.
     *
     * @return uint32_t Contador de sucessos
     */
    uint32_t getSuccessCount() const;

    /**
     * Retorna o número total de erros ocorridos desde o arranque.
     *
     * @return uint32_t Contador de erros
     */
    uint32_t getErrorCount() const;

    /**
     * Retorna o estado atual da FSM.
     *
     * @return ParserState Estado atual (útil para debug)
     */
    ParserState getCurrentState() const;

private:
    /* Buffer de receção — acumula os bytes da mensagem atual */
    uint8_t rawBuffer[MAX_MESSAGE_SIZE];   /* Buffer bruto de 256 bytes */
    size_t  rawOffset;                      /* Posição livre atual no buffer */

    /* Mensagem estruturada — preenchida após validação completa */
    TLVMessage msg;

    /* Estado da máquina de estados */
    ParserState state;                      /* Estado atual da FSM */
    ParserError lastError;                  /* Último erro registado */

    /* Controlo de receção de TLVs */
    uint8_t expectedTLVs;                   /* Total de TLVs esperados (do COUNT) */
    uint8_t currentTLV;                     /* Índice do TLV a processar (0..N-1) */
    uint8_t tlvBytesReceived;               /* Bytes recebidos do TLV atual */
    uint8_t currentTLVLen;                  /* Comprimento esperado do TLV atual */

    /* Controlo de timeout entre bytes */
    uint32_t lastActivityTime;              /* Timestamp do último byte (µs) */
    uint32_t maxFrameGapMicros;             /* Timeout configurado (0 = desativado) */

    /* Estatísticas de operação */
    uint32_t successCount;                  /* Total de mensagens válidas recebidas */
    uint32_t errorCount;                    /* Total de erros desde o arranque */

    /* Controlo de debug */
    bool debugEnabled;                      /* Flag de ativação de debug */

    /* Flag de estado — indica se há mensagem pronta para processamento */
    bool messageReady;                      /* true = mensagem disponível */

    /* Métodos privados auxiliares */
    void updateTimestamp();                 /* Regista o tempo atual como última atividade */
    void setError(ParserError error);       /* Regista erro, incrementa contador e reinicia */
};

/* =================================================================================
 * FUNÇÕES AUXILIARES PARA DEBUG
 * =================================================================================
 *
 * Convertem enums internos em strings legíveis para logging.
 * Úteis para diagnóstico remoto e monitorização em tempo real.
 */

/**
 * Converte um estado da FSM para a sua representação textual.
 *
 * @param state Estado a converter
 * @return const char* String descritiva (ex: "WAIT_START")
 */
const char* parserStateToString(ParserState state);

/**
 * Converte um código de erro para a sua representação textual.
 *
 * @param error Código de erro a converter
 * @return const char* String descritiva (ex: "CHECKSUM")
 */
const char* parserErrorToString(ParserError error);
