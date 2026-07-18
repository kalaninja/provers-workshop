# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-opam:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS aeneas-tooling

COPY mise/config.aeneas.toml ./mise/config.aeneas.toml

ENV XDG_DATA_HOME=/opt/aeneas/data
ENV XDG_CONFIG_HOME=/opt/aeneas/config
ENV XDG_CACHE_HOME=/opt/aeneas/cache
ENV OPAMROOT=/opt/aeneas/opam
ENV ELAN_HOME=/opt/aeneas/elan
ENV PATH=/opt/aeneas/elan/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

ARG AENEAS_REV=d71d2e3f2cb763f7faf15ab606ac9cf32da8dead
ARG CHARON_REV=5a5017333554ec1e95cae3c4fe05f5bd90b44487

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E aeneas rust "ubi:leanprover/elan" \
    && opam init --bare --disable-sandboxing --yes \
    && opam switch create 5.3.0 \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup /opt/aeneas

# Fresh apt lists inside the layer: opam resolves any depexts of these packages through apt.
RUN apt-get update \
    && opam install --switch=5.3.0 --yes \
        calendar core core_unix domainslib easy_logging menhir ocamlformat.0.27.0 \
        ocamlgraph odoc ppx_deriving ppx_deriving_yojson progress unionFind visitors yojson zarith \
    && rm -rf /var/lib/apt/lists/* \
    && chmod -R a+rwX /opt/aeneas

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && git init /opt/aeneas/src \
    && git -C /opt/aeneas/src remote add origin https://github.com/AeneasVerif/aeneas \
    && git -C /opt/aeneas/src fetch --depth 1 origin "${AENEAS_REV}" \
    && git -C /opt/aeneas/src checkout FETCH_HEAD \
    && git init /opt/aeneas/src/charon \
    && git -C /opt/aeneas/src/charon remote add origin https://github.com/AeneasVerif/charon \
    && git -C /opt/aeneas/src/charon fetch --depth 1 origin "${CHARON_REV}" \
    && git -C /opt/aeneas/src/charon checkout FETCH_HEAD \
    && MISE_ENV=aeneas mise exec rust -- bash -c 'eval "$(opam env --switch=5.3.0)" \
        && cd /opt/aeneas/src/charon && make build \
        && cd /opt/aeneas/src && make' \
    && ln -sf /opt/aeneas/src/bin/aeneas /usr/local/bin/aeneas \
    && ln -sf /opt/aeneas/src/charon/bin/charon /usr/local/bin/charon \
    && ln -sf /opt/aeneas/src/charon/bin/charon-driver /usr/local/bin/charon-driver \
    && chmod -R a+rwX /opt/aeneas /opt/cargo /opt/rustup

# The Lean toolchain is pinned by the aeneas source tree itself (backends/lean/lean-toolchain,
# fixed by AENEAS_REV above); derive it from there instead of duplicating the version here.
RUN --mount=type=secret,id=github_token \
    --mount=type=bind,source=scripts/docker/install-lean-toolchain.sh,target=/tmp/install-lean-toolchain.sh \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=aeneas mise exec -- bash /tmp/install-lean-toolchain.sh /opt/aeneas/src/backends/lean/lean-toolchain

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=aeneas mise exec -- bash -c 'cd /opt/aeneas/src/backends/lean \
        && lake exe cache get \
        && lake build' \
    && chmod -R a+rwX /opt/aeneas

RUN command -v charon && command -v charon-driver && command -v aeneas \
    && aeneas --help >/dev/null \
    && ELAN_HOME=/opt/aeneas/elan lean --version
