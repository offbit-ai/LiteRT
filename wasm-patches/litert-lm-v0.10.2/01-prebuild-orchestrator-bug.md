# Issue: LiteRT-LM top-level orchestrator inherits emcmake env in prebuild stage

## Symptom

```
clang: error: unsupported option '-arch' for target 'wasm32-unknown-emscripten'
gmake[2]: *** [CMakeFiles/litert_lm_prebuild.dir/build.make:92: prebuild/stamps/litert_lm_prebuild-configure] Error 1
```

## Root cause

`CMakeLists.txt` (top-level orchestrator) declares the host prebuild as:

```cmake
ExternalProject_Add(
    litert_lm_prebuild
    SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/cmake/packages/litert_lm"
    ...
    CMAKE_ARGS
        "-DLITERTLM_PROJECT_ROOT=${LITERTLM_PROJECT_ROOT}"
    INSTALL_COMMAND ""
)
```

The prebuild is intended to build native protoc + flatc on the host using the
host compiler. But `ExternalProject_Add` inherits the parent CMake's
environment — when invoked under `emcmake`, that env has `CC=emcc CXX=em++`
plus `CMAKE_TOOLCHAIN_FILE=$EMSDK/.../Emscripten.cmake` already exported. The
prebuild stage thus tries to use emcc with macOS-specific flags (`-arch`)
inherited from the orchestrator project, and fails the basic compiler sanity
check.

## Fix (patch for our fork)

The prebuild's `ExternalProject_Add` must explicitly opt out of the
emscripten toolchain. Replace its `CMAKE_ARGS` block with:

```cmake
ExternalProject_Add(
    litert_lm_prebuild
    SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/cmake/packages/litert_lm"
    ...
    CMAKE_ARGS
        "-DLITERTLM_PROJECT_ROOT=${LITERTLM_PROJECT_ROOT}"
        # Force native host build, override emcc env when configuring
        "-UCMAKE_TOOLCHAIN_FILE"
        "-DCMAKE_C_COMPILER=${CMAKE_HOST_C_COMPILER}"
        "-DCMAKE_CXX_COMPILER=${CMAKE_HOST_CXX_COMPILER}"
    CMAKE_CACHE_ARGS
        # Wipe any inherited toolchain settings
        "-DCMAKE_SYSTEM_NAME:STRING="
        "-DCMAKE_CROSSCOMPILING:BOOL=FALSE"
    INSTALL_COMMAND ""
)
```

Plus add cache vars at top-level:

```cmake
set(CMAKE_HOST_C_COMPILER "/usr/bin/clang" CACHE FILEPATH "Native host C compiler for prebuild")
set(CMAKE_HOST_CXX_COMPILER "/usr/bin/clang++" CACHE FILEPATH "Native host C++ compiler for prebuild")
```

## Workaround (no fork needed, less elegant)

Bypass the orchestrator entirely. Pre-build host tools natively in a
separate invocation, then pass paths explicitly:

```bash
# 1. Pre-build host tools natively (no emcmake)
cmake -S cmake/packages/litert_lm -B host-prebuild -DCMAKE_BUILD_TYPE=Release
cmake --build host-prebuild --target protoc flatc -j 8

# 2. Run main build with explicit host tool paths
emcmake cmake -S cmake/packages/litert_lm -B build-wasm \
  -DLITERTLM_HOST_PROTOC=$(pwd)/host-prebuild/.../bin/protoc \
  -DLITERTLM_HOST_FLATC=$(pwd)/host-prebuild/.../bin/flatc \
  -DCMAKE_BUILD_TYPE=Release
emmake cmake --build build-wasm -j 8
```

## Status

The orchestrator bug is the first of likely many emscripten issues in the
LiteRT-LM CMake build (it was originally designed for native + Android
cross-compile, never tested with emcc). Subsequent issues will surface in
the actual cross-compile stage once the prebuild is fixed.

Until our fork is ready, **ship `litert-sys` WASM support first** (Phase 1.0a
proved that works) and add `litert-lm-sys` WASM in a follow-up release once
the LiteRT-LM fork is debugged.
