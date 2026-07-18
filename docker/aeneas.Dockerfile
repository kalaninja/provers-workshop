# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-bookworm:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS aeneas-tooling

COPY mise/config.aeneas.toml ./mise/config.aeneas.toml

ENV XDG_DATA_HOME=/opt/aeneas/data
ENV XDG_CONFIG_HOME=/opt/aeneas/config
ENV XDG_CACHE_HOME=/opt/aeneas/cache
ENV OPAMROOT=/opt/aeneas/opam
ENV ELAN_HOME=/opt/aeneas/elan
ENV PATH=/opt/aeneas/elan/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ENV OPAMYES=1
ENV OPAMCONFIRMLEVEL=unsafe-yes

ARG AENEAS_REV=d71d2e3f2cb763f7faf15ab606ac9cf32da8dead
ARG CHARON_REV=5a5017333554ec1e95cae3c4fe05f5bd90b44487

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
        zlib1g-dev \
    && mise install -E aeneas rust "ubi:leanprover/elan" \
    && opam init --bare --disable-sandboxing --yes \
    && opam switch create 5.3.0 \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup /opt/aeneas

RUN opam install --switch=5.3.0 --yes \
        calendar core core_unix domainslib easy_logging menhir ocamlformat.0.27.0 \
        ocamlgraph odoc ppx_deriving ppx_deriving_yojson progress unionFind visitors yojson zarith \
    && rm -rf /var/lib/apt/lists/* \
    && chmod -R a+rwX /opt/aeneas

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && git clone https://github.com/AeneasVerif/aeneas /opt/aeneas/src \
    && (cd /opt/aeneas/src && git checkout "${AENEAS_REV}") \
    && git clone https://github.com/AeneasVerif/charon /opt/aeneas/src/charon \
    && (cd /opt/aeneas/src/charon && git checkout "${CHARON_REV}") \
    && MISE_ENV=aeneas mise exec rust -- bash -c 'eval "$(opam env --switch=5.3.0)" \
        && cd /opt/aeneas/src/charon && make build \
        && cd /opt/aeneas/src && make' \
    && ln -sf /opt/aeneas/src/bin/aeneas /usr/local/bin/aeneas \
    && ln -sf /opt/aeneas/src/charon/bin/charon /usr/local/bin/charon \
    && ln -sf /opt/aeneas/src/charon/bin/charon-driver /usr/local/bin/charon-driver \
    && chmod -R a+rwX /opt/aeneas /opt/cargo /opt/rustup

ARG LEAN_TOOLCHAIN=leanprover/lean4:v4.31.0
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=aeneas mise exec -- elan toolchain install "${LEAN_TOOLCHAIN}" \
    && tc="$(ls -d /opt/aeneas/elan/toolchains/*/ | head -n1)" \
    && for b in lean lake leanc; do ln -sf "${tc}bin/${b}" "/usr/local/bin/${b}"; done \
    && lean --version

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=aeneas mise exec -- bash -c 'cd /opt/aeneas/src/backends/lean \
        && lake exe cache get \
        && lake build' \
    && chmod -R a+rwX /opt/aeneas

RUN command -v charon && command -v charon-driver && command -v aeneas \
    && aeneas --help >/dev/null \
    && ELAN_HOME=/opt/aeneas/elan lean --version
