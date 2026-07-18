# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-bookworm:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS creusot-tooling

# Creusot installs only its own nightly, keeping the image slim.
COPY mise/config.creusot.toml ./mise/config.creusot.toml

ENV XDG_DATA_HOME=/opt/creusot/data
ENV XDG_CONFIG_HOME=/opt/creusot/config
ENV XDG_CACHE_HOME=/opt/creusot/cache
ENV OPAMROOT=/opt/creusot/opam
ENV WHY3CONFIG=/opt/creusot/config/creusot/why3.conf
ENV OPAMYES=1
ENV OPAMCONFIRMLEVEL=unsafe-yes

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

RUN command -v cargo-creusot
