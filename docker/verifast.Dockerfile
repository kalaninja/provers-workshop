# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-bookworm:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS verifast-tooling

COPY mise/config.verifast.toml ./mise/config.verifast.toml

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E verifast rust "github:verifast/verifast" \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup

RUN MISE_ENV=verifast mise exec -- sh -c 'command -v verifast' \
    && MISE_ENV=verifast mise exec -- rustup which --toolchain nightly-2025-11-25 rustc
