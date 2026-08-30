fn main() {
    // Script de build para geração de bindings FFI (opcional)
    // Quando esp-idf-sys está habilitado, as bindings são geradas automaticamente
    // Para compilação sem ESP-IDF (testes no host), este script é um no-op
    #[cfg(feature = "std")]
    {
        // esp-idf-sys configura tudo automaticamente via seu próprio build.rs
    }
}
