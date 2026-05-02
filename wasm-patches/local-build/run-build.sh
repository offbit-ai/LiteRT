#!/usr/bin/env bash
# Reproduces the GHA build-litert-lm-wasm.yml steps inside the container.
# Designed to be re-runnable: idempotent on the LiteRT-LM clone + build dir
# in /cache so iteration after a failure is just re-running this.
#
# Inside the container:
#   /work/run-build.sh         # apply patches + configure + build
#   /work/run-build.sh build   # only re-run cmake --build (skip patches)
#   /work/run-build.sh resume  # cd into the source dir for manual editing
#
# A failure leaves /cache/litert-lm/build-wasm/ in place; you can edit
# files in /cache/litert-lm and re-run.

set -euo pipefail

REPO=${REPO:-/repo}
CACHE=${CACHE:-/cache}
LITERTLM_TAG=${LITERTLM_TAG:-v0.10.2}
LITERTLM_DIR=${CACHE}/litert-lm

mode=${1:-full}

source /opt/emsdk/emsdk_env.sh

# ---------------------------------------------------------------------------
# Step 1: clone LiteRT-LM into the cache (one-time).
# ---------------------------------------------------------------------------
if [ ! -d "${LITERTLM_DIR}/.git" ]; then
  echo "==> cloning LiteRT-LM ${LITERTLM_TAG} into ${LITERTLM_DIR}"
  git clone --depth=1 --branch="${LITERTLM_TAG}" \
    https://github.com/google-ai-edge/LiteRT-LM.git "${LITERTLM_DIR}"
fi

cd "${LITERTLM_DIR}"

# ---------------------------------------------------------------------------
# Step 2: apply patches (skip if `build` mode).
# ---------------------------------------------------------------------------
if [ "${mode}" = "full" ]; then
  echo "==> resetting LiteRT-LM checkout to clean ${LITERTLM_TAG}"
  # `git checkout -- .` restores tracked files (incl. upstream
  # cmake/scripts/compile_flatbuffers.cmake & friends).
  # `git clean -fd` removes untracked files (incl. our previously-copied
  # helper scripts so we don't accumulate stale ones).
  git checkout -- . && git clean -fd

  echo "==> applying wasm-patches/litert-lm-v0.10.2/*.patch"
  for patch in "${REPO}"/wasm-patches/litert-lm-v0.10.2/*.patch; do
    echo "    $(basename "${patch}")"
    git apply "${patch}"
  done

  echo "==> copying helper scripts"
  mkdir -p cmake/scripts
  cp "${REPO}"/wasm-patches/litert-lm-v0.10.2/cmake/scripts/*.cmake cmake/scripts/
  ls cmake/scripts/

  echo "==> running source-level patchers"
  cmake -DLITERTLM_ROOT=. -P cmake/scripts/patch_litert_api_skew.cmake
  cmake -DLITERT_LM_CMAKELISTS=cmake/packages/litert_lm/CMakeLists.txt \
        -P cmake/scripts/patch_swap_main_to_aggregator.cmake
fi

# ---------------------------------------------------------------------------
# Step 3: configure (idempotent — skip if build dir already exists with
# CMakeCache.txt).
# ---------------------------------------------------------------------------
BUILD_DIR=${LITERTLM_DIR}/build-wasm
if [ ! -f "${BUILD_DIR}/CMakeCache.txt" ]; then
  echo "==> configure (emcmake)"
  rm -rf "${BUILD_DIR}"
  emcmake cmake -B "${BUILD_DIR}" -S . \
    -DCMAKE_BUILD_TYPE=Release \
    -DLITERTLM_TOOLCHAIN_ARGS="-DCMAKE_BUILD_TYPE=Release;-DCMAKE_FIND_PACKAGE_PREFER_CONFIG=ON"
fi

if [ "${mode}" = "resume" ]; then
  exec /bin/bash -l
fi

# ---------------------------------------------------------------------------
# Step 4: build.
# ---------------------------------------------------------------------------
echo "==> build"
set -o pipefail
emmake cmake --build "${BUILD_DIR}" -j "$(nproc)" 2>&1 | tee "${CACHE}/build.log"
