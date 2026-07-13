# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64

# Base image for every tooling stage. Defaults to bookworm; the Verus lane overrides it to a newer
# Debian via --build-arg in `just tooling build-verus`, because the verus/cargo-verus release
# binaries need glibc >= 2.39 and bookworm ships 2.36. Every other lane keeps bookworm.
ARG TOOLING_BASE_IMAGE=debian:bookworm-slim

# Agave does not publish a Linux arm64 Solana release archive with cargo-build-sbf.
# Pin the tooling platform to amd64 so local Apple Silicon builds match CI.
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${TOOLING_BASE_IMAGE} AS tooling-base

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

COPY mise/config.toml ./mise/config.toml

# mise itself is pinned: it is the tool that pins every other tool, so an unpinned installer would
# be the one unreproducible link in the chain.
ARG MISE_VERSION=v2026.7.5

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && curl -fsSL https://mise.run | MISE_VERSION="${MISE_VERSION}" MISE_INSTALL_PATH=/usr/local/bin/mise sh \
    && mise --version \
    && mkdir -p /opt/cargo /workspace/.cache/cargo/git /workspace/.cache/cargo/registry \
    && ln -sf /workspace/.cache/cargo/git /opt/cargo/git \
    && ln -sf /workspace/.cache/cargo/registry /opt/cargo/registry

FROM tooling-base AS rust-tooling

COPY mise/config.nightly.toml ./mise/config.nightly.toml

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

COPY mise/config.solana.toml ./mise/config.solana.toml

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E solana "github:anza-xyz/agave" \
    && HOME=/opt/solana-home MISE_ENV=solana mise exec -- cargo build-sbf --install-only \
    && chmod -R a+rwX /opt/mise /opt/solana-home \
    && MISE_ENV=solana mise exec -- solana --version \
    && MISE_ENV=solana mise exec -- cargo build-sbf --version

FROM tooling-base AS creusot-tooling

# FROM tooling-base, not rust-tooling: Creusot installs only its own nightly, keeping the image slim.
COPY mise/config.creusot.toml ./mise/config.creusot.toml

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

FROM tooling-base AS kani-tooling

# FROM tooling-base, not rust-tooling: like Creusot, Kani needs only a base cargo to build
# kani-verifier and dispatch `cargo kani`; it ships/uses its OWN nightly toolchain + CBMC
# backend (fetched by `cargo kani setup`).
COPY mise/config.kani.toml ./mise/config.kani.toml

# `cargo kani setup` downloads the kani-compiler, CBMC/kissat backend, and a matching nightly
# under KANI_HOME. Bake it to a fixed /opt/kani that run-tooling.sh points KANI_HOME at when
# MISE_ENV=kani. Setup symlinks the toolchain into /opt/rustup (RUSTUP_HOME), baked here too.
ENV KANI_HOME=/opt/kani

# mise installs the wrapper stable rust and kani-verifier (cargo backend; rust goes first so the
# backend has a cargo to build with) and provisions rustup (RUSTUP_HOME=/opt/rustup);
# `cargo kani setup` shells out to rustup to install its nightly, then curls its release
# bundle (curl + tar are in tooling-base). The prebuilt CBMC binaries only need libstdc++
# (from build-essential) and the glibc loader, so no extra apt packages are required.
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E kani rust \
    && mise install -E kani "cargo:kani-verifier" \
    && MISE_ENV=kani mise exec -- cargo kani setup \
    && chmod -R a+rwX /opt/kani /opt/cargo /opt/mise /opt/rustup

# Cheap self-test in a separate layer so the expensive setup above stays cached.
RUN MISE_ENV=kani mise exec -- cargo kani --version

FROM tooling-base AS verus-tooling

# Two divergences from kani/creusot. (1) tooling-base is built on trixie here (TOOLING_BASE_IMAGE
# above): the verus/cargo-verus release binaries need glibc >= 2.39 and bookworm ships 2.36. (2)
# This lane needs a real rustup toolchain: Verus is a rustc driver (rust_verify dynamically links
# 1.96.0 with rustc-dev/rust-src/llvm-tools, and the launcher resolves it via rustup).
# mise/config.verus.toml provisions that toolchain plus the release bundle (verus + cargo-verus +
# bundled Z3 + precompiled vstd, github backend) and verusfmt (ubi backend).
COPY mise/config.verus.toml ./mise/config.verus.toml

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E verus rust "ubi:verus-lang/verusfmt" "github:verus-lang/verus" \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup

# Cheap self-test in a separate layer so the expensive downloads above stay cached. `verus
# --version` (not `cargo verus --version`: the pinned rolling cargo-verus takes no such flag).
RUN MISE_ENV=verus mise exec -- verus --version \
    && MISE_ENV=verus mise exec -- sh -c 'command -v cargo-verus' \
    && MISE_ENV=verus mise exec -- verusfmt --version

FROM tooling-base AS flux-tooling

# Two things this lane owns that mise cannot express, both here in the Dockerfile. (1) apt: libgmp10
# for the (GHC-built) fixpoint binary and libgomp1 for Z3 - system libraries, not mise tools. (2)
# Flux itself is BUILT FROM SOURCE: it has no release, crates.io crate, or rustup component, and its
# install is a custom `cargo xtask install` (builds the flux-driver + precompiled spec libs, not a
# plain `cargo install`), so - like Creusot's opam build and Kani's `cargo kani setup` - it cannot
# ride a mise backend. Everything mise CAN pin, it does in mise/config.flux.toml: the nightly
# toolchain and Flux's two prebuilt solver backends, Z3 and liquid-fixpoint.
#
# Like Creusot/Verus, Flux is a rustc driver that pins but does not ship its toolchain
# (nightly-2025-11-25 with rustc-dev/rust-src/llvm-tools, resolved through rustup); cargo-flux shells
# out to `rustup which --toolchain nightly-2025-11-25` at runtime. Z3's release asset is glibc-2.39,
# so - like Verus - this stage overrides the base to trixie (bookworm ships 2.36). The flux sysroot
# (flux-driver + precompiled flux_rs libs) lands at $HOME/.flux (xtask hardcodes it), so HOME is
# redirected for the build and the result moved to the /opt/flux that run-tooling.sh points
# FLUX_SYSROOT at.
COPY mise/config.flux.toml ./mise/config.flux.toml

ENV FLUX_SYSROOT=/opt/flux
# No upstream tags/releases; pin by commit. Re-verify HEAD + its rust-toolchain.toml when bumping.
ARG FLUX_REV=1d98676057a7839c6c468aa3dd556ab769a150fa

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && apt-get update \
    && apt-get install --yes --no-install-recommends libgmp10 libgomp1 \
    && rm -rf /var/lib/apt/lists/* \
    && mise install -E flux rust "github:Z3Prover/z3" "github:ucsd-progsys/liquid-fixpoint" \
    && git clone https://github.com/flux-rs/flux /tmp/flux \
    && (cd /tmp/flux && git checkout "${FLUX_REV}") \
    && HOME=/tmp/fluxhome MISE_ENV=flux mise exec -- bash -c 'cd /tmp/flux && cargo xtask install' \
    && mv /tmp/fluxhome/.flux /opt/flux \
    && ln -sf /opt/cargo/bin/cargo-flux /usr/local/bin/cargo-flux \
    && ln -sf /opt/cargo/bin/flux /usr/local/bin/flux \
    && rm -rf /tmp/flux /tmp/fluxhome \
    && chmod -R a+rwX /opt/flux /opt/cargo /opt/mise /opt/rustup

# Cheap self-test in a separate layer so the expensive build above stays cached. Confirms the
# from-source binaries are on PATH, mise resolves both solvers, and the pinned nightly resolves
# through rustup - what `cargo flux` shells out to at runtime to find the rustc the driver links.
RUN command -v cargo-flux && command -v flux \
    && MISE_ENV=flux mise exec -- sh -c 'command -v z3 && command -v fixpoint' \
    && MISE_ENV=flux mise exec -- rustup which --toolchain nightly-2025-11-25 cargo
