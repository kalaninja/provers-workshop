# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-bookworm:dev
ARG WORKSHOP_OPAM_BASE_IMAGE=workshop-tooling-base-opam:dev
# Single source for the hax source revision; stages re-import via a bare ARG. The Lean toolchain
# is not pinned here: it is derived from the pinned source tree (hax-lib/proof-libs/lean).
ARG HAX_REV=d8b5b3d3b666fee8943a351445d2b680105e8ea3

# Fetch the pinned hax source once (shallow SHA fetch); the names and OCaml builder stages copy
# from here instead of cloning the repository twice.
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS hax-src

ARG HAX_REV

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && git init /opt/hax/src \
    && git -C /opt/hax/src remote add origin https://github.com/cryspen/hax \
    && git -C /opt/hax/src fetch --depth 1 origin "${HAX_REV}" \
    && git -C /opt/hax/src checkout FETCH_HEAD

FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS hax-rust-base

COPY mise/config.hax.toml ./mise/config.hax.toml

ENV ELAN_HOME=/opt/hax/elan
ENV PATH=/opt/hax/elan/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E hax rust \
    && mise install -E hax \
        "cargo:cargo-hax" \
        "cargo:hax-driver" \
        "cargo:hax-rust-engine" \
        "ubi:leanprover/elan" \
    && mkdir -p /opt/hax \
    && chmod -R a+rwX /opt/hax /opt/cargo /opt/mise /opt/rustup

# Recreate the bundled sibling layout expected by cargo-hax while retaining exact mise pins.
RUN MISE_ENV=hax mise exec -- bash -c 'set -euo pipefail \
    && bin="$(dirname "$(command -v cargo-hax)")" \
    && ln -sf "$(command -v driver-hax-frontend-exporter)" "${bin}/driver-hax-frontend-exporter" \
    && ln -sf "$(command -v hax-rust-engine)" "${bin}/hax-rust-engine"'

FROM hax-rust-base AS hax-names-builder

COPY --from=hax-src /opt/hax/src /opt/hax-names-src

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=hax mise exec -- cargo install --locked \
        --path /opt/hax-names-src/engine/names/extract

FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_OPAM_BASE_IMAGE} AS hax-ocaml-builder

ENV OPAMROOT=/opt/hax-ocaml/opam

RUN apt-get update \
    && apt-get install --yes --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && opam init --bare --disable-sandboxing --yes \
    && opam switch create 5.1.1 ocaml-base-compiler.5.1.1 --yes

# Glob the version segment so this tracks the cargo-hax pin in mise/config.hax.toml
# (the source of truth) instead of hardcoding a version that drifts on every bump.
COPY --from=hax-rust-base \
    /opt/mise/installs/cargo-cargo-hax/*/bin/hax-export-json-schemas \
    /usr/local/bin/

COPY --from=hax-src /opt/hax/src /opt/hax/src

RUN eval "$(opam env --switch=5.1.1)" \
    && export OPAMJOBS=4 OPAMERRLOGLEN=0 OPAMASSUMEDEPEXTS=1 OCAMLRUNPARAM=o=20 \
    && opam install --deps-only --yes /opt/hax/src/engine

COPY --from=hax-names-builder \
    /opt/cargo/bin/hax-engine-names-extract \
    /usr/local/bin/hax-engine-names-extract

RUN eval "$(opam env --switch=5.1.1)" \
    && export OPAMJOBS=4 OPAMERRLOGLEN=0 OPAMASSUMEDEPEXTS=1 OCAMLRUNPARAM=o=20 \
    && (ulimit -s hard 2>/dev/null || ulimit -s unlimited 2>/dev/null || true) \
    && opam install --yes /opt/hax/src/engine \
    && install -m 0755 "$(opam var bin)/hax-engine" /opt/hax-ocaml/hax-engine \
    && test -x /opt/hax-ocaml/hax-engine

FROM hax-rust-base AS hax-tooling

COPY --from=hax-ocaml-builder /opt/hax/src /opt/hax/src
COPY --from=hax-ocaml-builder /opt/hax-ocaml/hax-engine /usr/local/bin/hax-engine

# The Lean toolchain is pinned by the hax source tree itself (the proof library's lean-toolchain,
# fixed by HAX_REV above); derive it from there instead of duplicating the version here.
RUN --mount=type=secret,id=github_token \
    --mount=type=bind,source=scripts/docker/install-lean-toolchain.sh,target=/tmp/install-lean-toolchain.sh \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=hax mise exec -- bash /tmp/install-lean-toolchain.sh /opt/hax/src/hax-lib/proof-libs/lean/lean-toolchain

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=hax mise exec -- bash -c \
        'cd /opt/hax/src/hax-lib/proof-libs/lean && lake build' \
    && chmod -R a+rwX /opt/hax /opt/cargo /opt/mise /opt/rustup

RUN MISE_ENV=hax mise exec -- cargo hax --version \
    && MISE_ENV=hax mise exec -- sh -c 'command -v hax-rust-engine' \
    && command -v hax-engine \
    && lean --version
