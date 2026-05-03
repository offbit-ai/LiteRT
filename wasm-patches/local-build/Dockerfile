# Local-build environment for the 0.4.0 LiteRT-LM WASM spike.
# Mirrors the ubuntu-latest GHA runner that progressed furthest, so local
# debugging matches CI behavior bit-for-bit.
#
# Apple Silicon caveats (verified May 2026):
#  * Native arm64 build hits XNNPACK's `-mmark-bti-property` flag in
#    aarch64 assembly compile. clang 18 rejects (unsupported option for
#    aarch64-unknown-linux-gnu); gcc-13 rejects (unrecognized).
#    The flag is GCC-only and ARM-specific; CI's x86_64 host doesn't see
#    it. To make native arm64 work, patch XNNPACK's CMake to drop the
#    flag on clang (not done in this image).
#  * Rosetta-emulated linux/amd64 works but is ~5-10x slower than CI
#    (each TFLite kernel .o takes minutes; full host prebuild
#    extrapolates to ~15h+).
#  * Recommended: continue iterating in CI for now; use this image for
#    post-mortem inspection (mount the host's checkout/build dir into
#    /cache and `bash` in to inspect intermediate state).
#
# Build:
#   docker build -t litertlm-wasm-build wasm-patches/local-build/
#
# Run (full build attempt):
#   docker run --rm \
#     -v $(pwd):/repo \
#     -v $(pwd)/../litertlm-wasm-cache:/cache \
#     litertlm-wasm-build \
#     bash /repo/wasm-patches/local-build/run-build.sh
#
# Run interactively (post-failure inspection):
#   docker run --rm -it \
#     -v $(pwd):/repo \
#     -v $(pwd)/../litertlm-wasm-cache:/cache \
#     litertlm-wasm-build

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC
ENV LANG=C.UTF-8

RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    cmake \
    ninja-build \
    git \
    curl \
    wget \
    unzip \
    python3 \
    python3-pip \
    pkg-config \
    ca-certificates \
    file \
    less \
    ripgrep \
    vim \
  && rm -rf /var/lib/apt/lists/*

# Force clang as the default host C/C++ compiler. Ubuntu 24.04's `cc`
# defaults to gcc-13, which on arm64 doesn't accept some compile flags
# that XNNPACK's BTI-aware ARM assembly emits (e.g. -mmark-bti-property).
# clang accepts them all.
#
# Also override the `cc` symlink so .S assembly rules (which CMake invokes
# via the `cc` name regardless of CMAKE_C_COMPILER) pick up clang too.
RUN ln -sf /usr/bin/clang /etc/alternatives/cc \
 && ln -sf /usr/bin/clang++ /etc/alternatives/c++ \
 && cc --version | head -1 \
 && c++ --version | head -1
ENV CC=/usr/bin/clang
ENV CXX=/usr/bin/clang++
ENV ASM=/usr/bin/clang

# emsdk install — pinned same version GHA uses. Source emsdk_env.sh
# (not exec) so its env exports persist for the embuilder line below.
ENV EMSDK_VERSION=5.0.7
RUN git clone --depth=1 https://github.com/emscripten-core/emsdk.git /opt/emsdk \
  && cd /opt/emsdk \
  && ./emsdk install ${EMSDK_VERSION} \
  && ./emsdk activate ${EMSDK_VERSION}

RUN /bin/bash -c "source /opt/emsdk/emsdk_env.sh && embuilder build zlib"

# Rust + wasm32 target.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
  && /root/.cargo/bin/rustup target add wasm32-unknown-emscripten

ENV PATH=/root/.cargo/bin:/opt/emsdk:/opt/emsdk/upstream/emscripten:$PATH
ENV EMSDK=/opt/emsdk
ENV EMSDK_NODE=/opt/emsdk/node/22.16.0_64bit/bin/node

# Source emsdk env on every shell entry.
RUN echo 'source /opt/emsdk/emsdk_env.sh > /dev/null 2>&1' >> /root/.bashrc

WORKDIR /work

# /repo  — bind-mount of the LiteRT-rs working tree (read-only ok)
# /cache — persistent build cache (LiteRT-LM checkout + build-wasm dir)
CMD ["/bin/bash", "-l"]
