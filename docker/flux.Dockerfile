# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-trixie:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS flux-tooling

COPY mise/config.flux.toml ./mise/config.flux.toml

ENV FLUX_SYSROOT=/opt/flux

RUN apt-get update \
    && apt-get install --yes --no-install-recommends libgmp10 libgomp1 \
    && rm -rf /var/lib/apt/lists/*

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E flux rust "github:Z3Prover/z3" "github:ucsd-progsys/liquid-fixpoint" \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup

ARG FLUX_REV=1d98676057a7839c6c468aa3dd556ab769a150fa

# The heavy layer: flux built from source (shallow SHA fetch). A FLUX_REV bump re-runs only this
# layer and the smoke below; the chmod stays scoped to what this layer creates (a broader tree
# would copy-up into the layer).
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && git init /tmp/flux \
    && git -C /tmp/flux remote add origin https://github.com/flux-rs/flux \
    && git -C /tmp/flux fetch --depth 1 origin "${FLUX_REV}" \
    && git -C /tmp/flux checkout FETCH_HEAD \
    && HOME=/tmp/fluxhome MISE_ENV=flux mise exec -- bash -c 'cd /tmp/flux && cargo xtask install' \
    && mv /tmp/fluxhome/.flux /opt/flux \
    && ln -sf /opt/cargo/bin/cargo-flux /usr/local/bin/cargo-flux \
    && ln -sf /opt/cargo/bin/flux /usr/local/bin/flux \
    && rm -rf /tmp/flux /tmp/fluxhome \
    && chmod -R a+rwX /opt/flux /opt/cargo/bin

RUN command -v cargo-flux && command -v flux \
    && MISE_ENV=flux mise exec -- sh -c 'command -v z3 && command -v fixpoint' \
    && MISE_ENV=flux mise exec -- sh -c 'rustup which cargo && rustc --version'
