# Local-build environment for the 0.4.0 LiteRT-LM WASM spike.
# Mirrors the ubuntu-latest GHA runner that progressed furthest, so local
# debugging matches CI behavior bit-for-bit.
#
# Build:
#   docker build -t litertlm-wasm-build wasm-patches/local-build/
#
# Run interactively (mount repo + persistent build cache):
#   docker run --rm -it \
#     -v $(pwd):/repo \
#     -v litertlm-wasm-cache:/cache \
#     -v litertlm-wasm-emsdk:/opt/emsdk \
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
