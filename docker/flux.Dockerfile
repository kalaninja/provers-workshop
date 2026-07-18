# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-trixie:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS flux-tooling

COPY mise/config.flux.toml ./mise/config.flux.toml

ENV FLUX_SYSROOT=/opt/flux
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

RUN command -v cargo-flux && command -v flux \
    && MISE_ENV=flux mise exec -- sh -c 'command -v z3 && command -v fixpoint' \
    && MISE_ENV=flux mise exec -- rustup which --toolchain nightly-2025-11-25 cargo
