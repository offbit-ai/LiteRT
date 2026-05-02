# Copyright 2026 LiteRT-rs contributors. Apache-2.0.
#
# LiteRT-LM v0.10.2's runtime calls several LiteRT C++ API methods that
# don't exist in the LiteRT commit it pins (fb16353a648 from 2026-03-24).
# These are mostly advanced/optional features added to LiteRT after the
# pin. For the WASM spike we stub each such call.
#
# Add new (file, pattern) pairs to this script as the build surfaces them.
# Each pattern's matched text is replaced with a /* PATCHED OUT */ no-op.
#
# Usage: cmake -DLITERTLM_ROOT=/path/to/litert-lm -P patch_litert_api_skew.cmake

if(NOT DEFINED LITERTLM_ROOT)
    message(FATAL_ERROR "LITERTLM_ROOT must be set (path to litert-lm checkout)")
endif()

# Each entry is FILE | REGEX. The regex matches a full statement; we replace
# it with a no-op. Multiple entries can target the same file.
set(_skews
    # 1. SetKernelBatchSize / SetDisableDelegateClustering — added to
    #    GpuOptions / RuntimeOptions later.
    "runtime/executor/llm_executor_settings_utils.cc|gpu_compilation_options\\.SetKernelBatchSize\\([^;]*\\);"
    "runtime/executor/llm_executor_settings_utils.cc|runtime_options\\.SetDisableDelegateClustering\\([^;]*\\);"
    # 2. SimpleTensor::HasQuantization / PerTensorQuantization — added to
    #    SimpleTensor's C++ API later. The if-block has an else branch
    #    (logs a warning, falls back to default scale=1.0, zero_point=0).
    #    Just take the else-branch path always: the LITERT_ASSIGN_OR_RETURN
    #    above already initialised quantization_params to those defaults.
    "runtime/executor/llm_litert_npu_compiled_model_executor.cc|if \\(logits_tensor\\.HasQuantization\\(\\)\\) \\{[^}]*\\} else \\{[^}]*\\}"
)

foreach(skew ${_skews})
    string(REPLACE "|" ";" parts "${skew}")
    list(GET parts 0 file)
    list(GET parts 1 pattern)
    set(path "${LITERTLM_ROOT}/${file}")
    if(NOT EXISTS "${path}")
        message(WARNING "[litertlm-patch] skew target not found: ${path}")
        continue()
    endif()
    file(READ "${path}" content)
    string(REGEX REPLACE
        "${pattern}"
        "/* PATCHED OUT for LiteRT API skew */ (void)0;"
        new_content "${content}")
    if(NOT "${new_content}" STREQUAL "${content}")
        file(WRITE "${path}" "${new_content}")
        message(STATUS "[litertlm-patch] stubbed in ${file}: ${pattern}")
    else()
        message(WARNING "[litertlm-patch] no match in ${file} for: ${pattern}")
    endif()
endforeach()
