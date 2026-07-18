# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64

# Base image for the shared base and core tooling stages. Prover Dockerfiles consume locally tagged
# bookworm/trixie variants of the `tooling-base` target through `just tooling build-<prover>`.
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
