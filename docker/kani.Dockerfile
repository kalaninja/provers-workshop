# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-bookworm:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS kani-tooling

COPY mise/config.kani.toml ./mise/config.kani.toml

ENV KANI_HOME=/opt/kani

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E kani rust \
    && mise install -E kani "cargo:kani-verifier" \
    && MISE_ENV=kani mise exec -- cargo kani setup \
    && chmod -R a+rwX /opt/kani /opt/cargo /opt/mise /opt/rustup

RUN MISE_ENV=kani mise exec -- cargo kani --version
