# Copyright 2026 LiteRT-rs contributors. Apache-2.0.
#
# Patches libpng's pngpriv.h to drop TARGET_OS_MAC from the conditional that
# triggers <fp.h> inclusion. Cross-platform via CMake file() commands so we
# don't depend on sed/perl flag differences across host platforms.
#
# Used by `cmake/modules/fetch_content.cmake`'s libpng FetchContent_Declare
# PATCH_COMMAND.

file(READ "${PNGPRIV_H}" content)
string(REPLACE " || defined(TARGET_OS_MAC)" "" content "${content}")
file(WRITE "${PNGPRIV_H}" "${content}")
message(STATUS "[litertlm-patch] removed TARGET_OS_MAC from ${PNGPRIV_H}")
