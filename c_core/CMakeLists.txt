# SPDX-License-Identifier: GPL-3.0-or-later

cmake_minimum_required(VERSION 3.10)
project(bythos_c_core VERSION 3.0.0 LANGUAGES C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# ============================================================================
# BIBLIOTECA PRINCIPAL
# ============================================================================

add_library(bythos STATIC
    src/bythos.c
)

target_include_directories(bythos PUBLIC
    include
)

target_compile_options(bythos PRIVATE
    -Wall -Wextra -Wpedantic
    -O2
)

# ============================================================================
# TESTES
# ============================================================================

add_executable(test_bythos
    tests/test_bythos.c
)

target_link_libraries(test_bythos PRIVATE bythos m)
target_include_directories(test_bythos PRIVATE include)

add_test(NAME bythos_unit_tests COMMAND test_bythos)

# ============================================================================
# BENCHMARK
# ============================================================================

add_executable(bench_bythos
    benchmark/bench_bythos.c
)

target_link_libraries(bench_bythos PRIVATE bythos m)
target_include_directories(bench_bythos PRIVATE include)

target_compile_options(bench_bythos PRIVATE -O2)

# ============================================================================
# OPÇÕES
# ============================================================================

option(BUILD_BENCHMARKS "Construir microbenchmarks" ON)

if(NOT BUILD_BENCHMARKS)
    set_target_properties(bench_bythos PROPERTIES EXCLUDE_FROM_ALL TRUE)
endif()

# ============================================================================
# INSTALAÇÃO
# ============================================================================

install(TARGETS bythos ARCHIVE DESTINATION lib)
install(FILES include/bythos.h DESTINATION include)
