# Copyright 2026 LiteRT-rs contributors. Apache-2.0.
#
# LiteRT-LM v0.10.2's `cmake/packages/litert_lm/CMakeLists.txt` ends with
# `add_executable(litert_lm_main ...)` and a target_link_libraries() block
# that pulls in dozens of internal libraries. The link fails because the
# upstream CMake graph is structurally incomplete: many .cc files
# (Gemma4DataProcessor::Create, GetOpenChannelName,
# ExtractChannelContent, InsertChannelContentIntoMessage, LogTensor, ...)
# exist in the source tree but aren't compiled into any library — so
# litert_lm_main can't link.
#
# We don't actually need the litert_lm_main CLI for litert-lm-sys; we
# need the static archives. Swap add_executable(litert_lm_main) for
# add_library(litert_lm_main STATIC <stub.cc>) — same deps still get
# built (CMake's transitive build graph), our tarball collection step
# still picks up `lib*.a`, and the broken final executable link is
# avoided entirely.
#
# Usage: cmake -DLITERT_LM_CMAKELISTS=path/to/cmake/packages/litert_lm/CMakeLists.txt
#              -P patch_swap_main_to_aggregator.cmake

if(NOT DEFINED LITERT_LM_CMAKELISTS)
    message(FATAL_ERROR "LITERT_LM_CMAKELISTS must be set")
endif()

file(READ "${LITERT_LM_CMAKELISTS}" content)

# Replace `add_executable(litert_lm_main "...litert_lm_main.cc")` with
# an aggregator add_library invocation. The downstream
# target_link_libraries(litert_lm_main ...) keeps working since CMake
# accepts add_library targets as targets too.
string(REGEX REPLACE
    "add_executable\\(litert_lm_main\n[^)]*\\)"
    "# PATCHED: add_executable -> add_library aggregator (litert-rs wasm-patches)
file(WRITE \"\${CMAKE_CURRENT_BINARY_DIR}/litert_lm_main_stub.cc\"
     \"// Aggregator stub for litert-rs WASM spike.\\n\")
add_library(litert_lm_main STATIC \"\${CMAKE_CURRENT_BINARY_DIR}/litert_lm_main_stub.cc\")"
    new_content "${content}")

if("${new_content}" STREQUAL "${content}")
    message(FATAL_ERROR "[litertlm-patch] no add_executable(litert_lm_main) match in ${LITERT_LM_CMAKELISTS}")
endif()

# Also strip `target_link_options(litert_lm_main PRIVATE ...)` since
# linker options don't apply to static archives.
string(REGEX REPLACE
    "target_link_options\\(litert_lm_main[^)]*\\)"
    "# PATCHED OUT: target_link_options for aggregator (litert-rs wasm-patches)"
    new_content "${new_content}")

# And `target_link_libraries(litert_lm_main PUBLIC <stuff>)` is still
# present after our swap. CMake permits target_link_libraries on
# STATIC archives — it records transitive deps for downstream consumers
# without doing an actual link, exactly what we want.

file(WRITE "${LITERT_LM_CMAKELISTS}" "${new_content}")
message(STATUS "[litertlm-patch] swapped litert_lm_main exe -> aggregator library in ${LITERT_LM_CMAKELISTS}")
