# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64

# Agave does not publish a Linux arm64 Solana release archive with cargo-build-sbf.
# Pin the tooling platform to amd64 so local Apple Silicon builds match CI.
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} debian:bookworm-slim AS tooling-base

ENV DEBIAN_FRONTEND=noninteractive
ENV CARGO_HOME=/opt/cargo
ENV MISE_DATA_DIR=/opt/mise
ENV MISE_TRUSTED_CONFIG_PATHS=/workspace
ENV MISE_YES=1
# Tools are baked per image at build time; mise must never auto-install on exec (build or runtime).
ENV MISE_EXEC_AUTO_INSTALL=false
ENV PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ENV RUSTUP_HOME=/opt/rustup

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        bash \
        bzip2 \
        build-essential \
        ca-certificates \
        curl \
        git \
        jq \
        libssl-dev \
        pkg-config \
        unzip \
        xz-utils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY mise.toml ./

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && curl -fsSL https://mise.run | MISE_INSTALL_PATH=/usr/local/bin/mise sh \
    && mise --version \
    && mkdir -p /opt/cargo /workspace/.cache/cargo/git /workspace/.cache/cargo/registry \
    && ln -sf /workspace/.cache/cargo/git /opt/cargo/git \
    && ln -sf /workspace/.cache/cargo/registry /opt/cargo/registry

FROM tooling-base AS rust-tooling

COPY mise.nightly.toml ./

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install rust "ubi:bnjbvr/cargo-machete" "aqua:nextest-rs/nextest/cargo-nextest" \
    && mise install -E nightly rust \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup \
    && mise exec rust -- cargo --version \
    && mise exec rust "ubi:bnjbvr/cargo-machete" -- cargo machete --version \
    && mise exec rust "aqua:nextest-rs/nextest/cargo-nextest" -- cargo nextest --version \
    && MISE_ENV=nightly mise exec rust -- cargo fmt --version

FROM tooling-base AS node-tooling

ENV NPM_CONFIG_CACHE=/workspace/.cache/npm

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install node pnpm \
    && chmod -R a+rwX /opt/mise \
    && mise exec node -- node --version \
    && mise exec node -- npm --version \
    && mise exec node pnpm -- pnpm --version

FROM rust-tooling AS solana-tooling

COPY mise.solana.toml ./

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E solana "github:anza-xyz/agave" \
    && HOME=/opt/solana-home MISE_ENV=solana mise exec -- cargo build-sbf --install-only \
    && chmod -R a+rwX /opt/mise /opt/solana-home \
    && MISE_ENV=solana mise exec -- solana --version \
    && MISE_ENV=solana mise exec -- cargo build-sbf --version

FROM tooling-base AS creusot-tooling

# FROM tooling-base, not rust-tooling: Creusot installs only its own nightly, keeping the image slim.
COPY mise.creusot.toml ./

# Creusot stores its toolchain, Why3, provers, and Why3 confs under XDG paths, baked to a fixed
# /opt/creusot that run-tooling.sh points HOME/XDG/WHY3CONFIG at when MISE_ENV=creusot. Creusot writes
# prover defs to $XDG_DATA_HOME/creusot/creusot_why3.conf; WHY3CONFIG below is the user why3.conf.
ENV XDG_DATA_HOME=/opt/creusot/data
ENV XDG_CONFIG_HOME=/opt/creusot/config
ENV XDG_CACHE_HOME=/opt/creusot/cache
ENV OPAMROOT=/opt/creusot/opam
ENV WHY3CONFIG=/opt/creusot/config/creusot/why3.conf
# Let opam auto-install system depexts (e.g. zlib, GTK for why3-ide) non-interactively via apt.
ENV OPAMYES=1
ENV OPAMCONFIRMLEVEL=unsafe-yes

# Install Creusot v0.12.0 via its INSTALL script (opam why3/why3find, cargo-creusot + creusot-rustc,
# provers). `--external z3` reuses the apt Z3. apt + INSTALL share one layer with apt lists intact so
# opam depext can install Why3's system libs (zlib1g-dev baseline, GTK/cairo for why3-ide).
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && apt-get update \
    && apt-get install --yes --no-install-recommends \
        autoconf \
        libgmp-dev \
        m4 \
        opam \
        pkg-config \
        rsync \
        z3 \
        zlib1g-dev \
    && mise install -E creusot rust \
    && mkdir -p /opt/creusot \
    && opam init --bare --disable-sandboxing --yes \
    && git clone --depth 1 --branch v0.12.0 https://github.com/creusot-rs/creusot /tmp/creusot \
    && MISE_ENV=creusot mise exec rust -- bash -c 'cd /tmp/creusot && ./INSTALL --external z3' \
    && ln -sf /opt/cargo/bin/cargo-creusot /usr/local/bin/cargo-creusot \
    && rm -rf /tmp/creusot /var/lib/apt/lists/* \
    && chmod -R a+rwX /opt/creusot /opt/cargo /opt/mise /opt/rustup

# Cheap self-test in a separate layer so the expensive install above stays cached.
RUN command -v cargo-creusot
