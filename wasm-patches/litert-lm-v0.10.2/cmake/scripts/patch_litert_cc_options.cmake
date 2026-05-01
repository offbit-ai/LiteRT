# Copyright 2026 LiteRT-rs contributors. Apache-2.0.
#
# LiteRT-LM v0.10.2 pins LiteRT to commit fb16353a648 (2026-03-24), which
# ships a truncated `litert/cc/options/CMakeLists.txt` (17 lines of header
# comments, no `add_library(litert_cc_options ...)` definition). Multiple
# downstream targets (libLiteRt.dylib, apply_plugin_main, run_model,
# dispatch_api_GoogleTensor_so) reference `litert_cc_options` in their
# target_link_libraries, so the link fails with `library not found`.
#
# v2.1.4's version of the file defines `litert_cc_options` as an INTERFACE
# library that aggregates the C++ options API. Restore that here.
#
# Used by `cmake/packages/litert/litert.cmake`'s PATCH_COMMAND.

set(restored_content [=[
# Restored by litert-rs wasm-patches: upstream file truncated in
# google-ai-edge/LiteRT@fb16353a648 (LiteRT-LM v0.10.2 pin).
cmake_minimum_required(VERSION 3.20)

add_library(litert_cc_options INTERFACE)

target_include_directories(litert_cc_options
    INTERFACE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../..>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../../..>
        $<BUILD_INTERFACE:${TENSORFLOW_SOURCE_DIR}>
)

target_link_libraries(litert_cc_options
    INTERFACE
        litert_cc_api
        litert_c_options
        absl::status
        absl::statusor
        absl::strings
        absl::span
)
]=])

file(WRITE "${CC_OPTIONS_CMAKELISTS}" "${restored_content}")
message(STATUS "[litertlm-patch] restored litert_cc_options target at ${CC_OPTIONS_CMAKELISTS}")
