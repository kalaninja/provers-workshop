# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-trixie:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS verus-tooling

COPY mise/config.verus.toml ./mise/config.verus.toml

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E verus rust "ubi:verus-lang/verusfmt" "github:verus-lang/verus" \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup

RUN MISE_ENV=verus mise exec -- verus --version \
    && MISE_ENV=verus mise exec -- sh -c 'command -v cargo-verus' \
    && MISE_ENV=verus mise exec -- verusfmt --version
