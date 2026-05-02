# Copyright 2026 LiteRT-rs contributors. Apache-2.0.
#
# LiteRT-LM v0.10.2 pins LiteRT to commit fb16353a648 (2026-03-24), which
# ships a truncated `litert/cc/options/CMakeLists.txt` (17 lines of header
# comments, no `add_library(litert_cc_options ...)` definition). Multiple
# downstream targets (libLiteRt.dylib, apply_plugin_main, run_model,
# dispatch_api_GoogleTensor_so) reference `litert_cc_options` in their
# target_link_libraries, so the link fails with `library not found`.
#
# v2.1.4 defined `litert_cc_options` as INTERFACE — fine for that
# project. LiteRT-LM v0.10.2's CMake explicitly references the produced
# `cc/options/liblitert_cc_options.a` file in its `litert_lm_main`
# dependency rule, so an INTERFACE target (no archive file) breaks the
# make graph with "No rule to make target ... liblitert_cc_options.a".
# Make it STATIC with a stub empty.cc instead.
#
# Used by `cmake/packages/litert/litert.cmake`'s PATCH_COMMAND.

set(restored_content [=[
# Restored by litert-rs wasm-patches: upstream file truncated in
# google-ai-edge/LiteRT@fb16353a648 (LiteRT-LM v0.10.2 pin).
cmake_minimum_required(VERSION 3.20)

# Stub TU so STATIC has something to compile. LiteRT-LM expects a real
# liblitert_cc_options.a file at link time.
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/litert_cc_options_stub.cc"
     "// Empty TU for litert_cc_options stub library (litert-rs wasm-patches).\n")

add_library(litert_cc_options STATIC
    "${CMAKE_CURRENT_BINARY_DIR}/litert_cc_options_stub.cc")

target_include_directories(litert_cc_options
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../..>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../../..>
        $<BUILD_INTERFACE:${TENSORFLOW_SOURCE_DIR}>
)

target_link_libraries(litert_cc_options
    PUBLIC
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
