# Copyright 2026 LiteRT-rs contributors. Apache-2.0.
#
# LiteRT-LM v0.10.2's runtime/executor/llm_executor_settings_utils.cc
# calls litert::GpuOptions::SetKernelBatchSize() and
# litert::RuntimeOptions::SetDisableDelegateClustering(). Neither method
# exists in the LiteRT commit v0.10.2 pins (fb16353a648, 2026-03-24) —
# they were added to LiteRT later. The result is two compile errors
# during the host prebuild stage.
#
# For the 0.4.0 spike we don't need either feature (both are advanced
# tuning options gated behind has_value()/bool checks). Stub both calls.
#
# Usage: cmake -DSETTINGS_FILE=path/to/llm_executor_settings_utils.cc
#              -P patch_llm_executor_settings.cmake

file(READ "${SETTINGS_FILE}" content)

# Replace the SetKernelBatchSize call (multi-line, ends in `);`).
string(REGEX REPLACE
    "gpu_compilation_options\\.SetKernelBatchSize\\([^;]*\\);"
    "/* PATCHED OUT for LiteRT API skew */ (void)0\;"
    content "${content}")

# Replace the SetDisableDelegateClustering call.
string(REGEX REPLACE
    "runtime_options\\.SetDisableDelegateClustering\\([^;]*\\);"
    "/* PATCHED OUT for LiteRT API skew */ (void)0\;"
    content "${content}")

file(WRITE "${SETTINGS_FILE}" "${content}")
message(STATUS "[litertlm-patch] stubbed missing-API calls in ${SETTINGS_FILE}")
