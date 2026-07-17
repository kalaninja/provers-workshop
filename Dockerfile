# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64

# Base image for every tooling stage. Defaults to bookworm; the Verus lane overrides it to a newer
# Debian via --build-arg in `just tooling build-verus`, because the verus/cargo-verus release
# binaries need glibc >= 2.39 and bookworm ships 2.36. Every other lane keeps bookworm.
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

FROM tooling-base AS creusot-tooling

# FROM tooling-base, not rust-tooling: Creusot installs only its own nightly, keeping the image slim.
COPY mise/config.creusot.toml ./mise/config.creusot.toml

# Creusot stores its toolchain, Why3, provers, and Why3 confs under XDG paths, baked to a fixed
# /opt/creusot that run-tooling.sh points HOME/XDG/WHY3CONFIG at when MISE_ENV=creusot. Creusot writes
# prover defs to $XDG_DATA_HOME/creusot/creusot_why3.conf; WHY3CONFIG below is the user why3.conf.
ENV XDG_DATA_HOME=/opt/creusot/data
ENV XDG_CONFIG_HOME=/opt/creusot/config
ENV XDG_CACHE_HOME=/opt/creusot/cache
ENV OPAMROOT=/opt/creusot/opam
ENV WHY3CONFIG=/opt/creusot/config/creusot/why3.conf
# Let opam auto-install system depexts (e.g. zlib, GTK for why3-ide) non-interactively via apt.
ENV OPAMYES=1
ENV OPAMCONFIRMLEVEL=unsafe-yes

# Install Creusot v0.12.0 via its INSTALL script (opam why3/why3find, cargo-creusot + creusot-rustc,
# provers). `--external z3` reuses the apt Z3. apt + INSTALL share one layer with apt lists intact so
# opam depext can install Why3's system libs (zlib1g-dev baseline, GTK/cairo for why3-ide).
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

# Cheap self-test in a separate layer so the expensive install above stays cached.
RUN command -v cargo-creusot

FROM tooling-base AS kani-tooling

# FROM tooling-base, not rust-tooling: like Creusot, Kani needs only a base cargo to build
# kani-verifier and dispatch `cargo kani`; it ships/uses its OWN nightly toolchain + CBMC
# backend (fetched by `cargo kani setup`).
COPY mise/config.kani.toml ./mise/config.kani.toml

# `cargo kani setup` downloads the kani-compiler, CBMC/kissat backend, and a matching nightly
# under KANI_HOME. Bake it to a fixed /opt/kani that run-tooling.sh points KANI_HOME at when
# MISE_ENV=kani. Setup symlinks the toolchain into /opt/rustup (RUSTUP_HOME), baked here too.
ENV KANI_HOME=/opt/kani

# mise installs the wrapper stable rust and kani-verifier (cargo backend; rust goes first so the
# backend has a cargo to build with) and provisions rustup (RUSTUP_HOME=/opt/rustup);
# `cargo kani setup` shells out to rustup to install its nightly, then curls its release
# bundle (curl + tar are in tooling-base). The prebuilt CBMC binaries only need libstdc++
# (from build-essential) and the glibc loader, so no extra apt packages are required.
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E kani rust \
    && mise install -E kani "cargo:kani-verifier" \
    && MISE_ENV=kani mise exec -- cargo kani setup \
    && chmod -R a+rwX /opt/kani /opt/cargo /opt/mise /opt/rustup

# Cheap self-test in a separate layer so the expensive setup above stays cached.
RUN MISE_ENV=kani mise exec -- cargo kani --version

FROM tooling-base AS verus-tooling

# Two divergences from kani/creusot. (1) tooling-base is built on trixie here (TOOLING_BASE_IMAGE
# above): the verus/cargo-verus release binaries need glibc >= 2.39 and bookworm ships 2.36. (2)
# This lane needs a real rustup toolchain: Verus is a rustc driver (rust_verify dynamically links
# 1.96.0 with rustc-dev/rust-src/llvm-tools, and the launcher resolves it via rustup).
# mise/config.verus.toml provisions that toolchain plus the release bundle (verus + cargo-verus +
# bundled Z3 + precompiled vstd, github backend) and verusfmt (ubi backend).
COPY mise/config.verus.toml ./mise/config.verus.toml

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E verus rust "ubi:verus-lang/verusfmt" "github:verus-lang/verus" \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup

# Cheap self-test in a separate layer so the expensive downloads above stay cached. `verus
# --version` (not `cargo verus --version`: the pinned rolling cargo-verus takes no such flag).
RUN MISE_ENV=verus mise exec -- verus --version \
    && MISE_ENV=verus mise exec -- sh -c 'command -v cargo-verus' \
    && MISE_ENV=verus mise exec -- verusfmt --version

FROM tooling-base AS flux-tooling

# Two things this lane owns that mise cannot express, both here in the Dockerfile. (1) apt: libgmp10
# for the (GHC-built) fixpoint binary and libgomp1 for Z3 - system libraries, not mise tools. (2)
# Flux itself is BUILT FROM SOURCE: it has no release, crates.io crate, or rustup component, and its
# install is a custom `cargo xtask install` (builds the flux-driver + precompiled spec libs, not a
# plain `cargo install`), so - like Creusot's opam build and Kani's `cargo kani setup` - it cannot
# ride a mise backend. Everything mise CAN pin, it does in mise/config.flux.toml: the nightly
# toolchain and Flux's two prebuilt solver backends, Z3 and liquid-fixpoint.
#
# Like Creusot/Verus, Flux is a rustc driver that pins but does not ship its toolchain
# (nightly-2025-11-25 with rustc-dev/rust-src/llvm-tools, resolved through rustup); cargo-flux shells
# out to `rustup which --toolchain nightly-2025-11-25` at runtime. Z3's release asset is glibc-2.39,
# so - like Verus - this stage overrides the base to trixie (bookworm ships 2.36). The flux sysroot
# (flux-driver + precompiled flux_rs libs) lands at $HOME/.flux (xtask hardcodes it), so HOME is
# redirected for the build and the result moved to the /opt/flux that run-tooling.sh points
# FLUX_SYSROOT at.
COPY mise/config.flux.toml ./mise/config.flux.toml

ENV FLUX_SYSROOT=/opt/flux
# No upstream tags/releases; pin by commit. Re-verify HEAD + its rust-toolchain.toml when bumping.
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

# Cheap self-test in a separate layer so the expensive build above stays cached. Confirms the
# from-source binaries are on PATH, mise resolves both solvers, and the pinned nightly resolves
# through rustup - what `cargo flux` shells out to at runtime to find the rustc the driver links.
RUN command -v cargo-flux && command -v flux \
    && MISE_ENV=flux mise exec -- sh -c 'command -v z3 && command -v fixpoint' \
    && MISE_ENV=flux mise exec -- rustup which --toolchain nightly-2025-11-25 cargo

FROM tooling-base AS hax-rust-base

# hax extracts Rust through its rustc driver, translates the resulting model with its Rust engine,
# and emits Lean. All published hax binaries + the matching nightly are pinned by mise; elan owns the
# Lean toolchain. This reusable stage also supplies hax-export-json-schemas to the OCaml builder:
# hax-engine uses it while generating its universe hash.
COPY mise/config.hax.toml ./mise/config.hax.toml

ENV ELAN_HOME=/opt/hax/elan
ENV PATH=/opt/hax/elan/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

ARG HAX_REV=d8b5b3d3b666fee8943a351445d2b680105e8ea3
ARG HAX_LEAN_TOOLCHAIN=leanprover/lean4:v4.29.0-rc1

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

# cargo-hax's released launcher expects its sibling protocol workers next to its own executable (the
# layout of upstream's bundled install). mise's cargo backend intentionally isolates each package,
# so recreate that layout with symlinks while retaining independent, exact package pins.
RUN MISE_ENV=hax mise exec -- bash -c 'set -euo pipefail \
    && bin="$(dirname "$(command -v cargo-hax)")" \
    && ln -sf "$(command -v driver-hax-frontend-exporter)" "${bin}/driver-hax-frontend-exporter" \
    && ln -sf "$(command -v hax-rust-engine)" "${bin}/hax-rust-engine"'

FROM hax-rust-base AS hax-names-builder

# The OCaml engine's universe hash also needs this source-only Rust helper. It is not published on
# crates.io at v0.3.7, so build it from the same pinned hax commit as the engine and Lean library.
ARG HAX_REV=d8b5b3d3b666fee8943a351445d2b680105e8ea3

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && git clone https://github.com/cryspen/hax /opt/hax-names-src \
    && (cd /opt/hax-names-src && git checkout "${HAX_REV}") \
    && MISE_ENV=hax mise exec -- cargo install --locked \
        --path /opt/hax-names-src/engine/names/extract

FROM tooling-base AS hax-ocaml-builder

# hax-rust-engine is the Lean backend, but at v0.3.7 it still delegates THIR import and shared
# phases to the legacy OCaml hax-engine over a JSON protocol. Build that one required helper in an
# isolated stage so opam, the OCaml compiler, and build dependencies do not enter the runtime image.
ENV OPAMROOT=/opt/hax-ocaml/opam
ENV OPAMYES=1
ENV OPAMCONFIRMLEVEL=unsafe-yes

ARG HAX_REV=d8b5b3d3b666fee8943a351445d2b680105e8ea3

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        autoconf \
        libgmp-dev \
        m4 \
        nodejs \
        opam \
        rsync \
        zlib1g-dev \
    && opam init --bare --disable-sandboxing --yes \
    && opam switch create 5.1.1 ocaml-base-compiler.5.1.1 --yes \
    && rm -rf /var/lib/apt/lists/*

# hax-engine's build hashes the Rust/OCaml schema universe with this cargo-hax companion binary.
COPY --from=hax-rust-base \
    /opt/mise/installs/cargo-cargo-hax/0.3.7/bin/hax-export-json-schemas \
    /usr/local/bin/hax-export-json-schemas

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && git clone https://github.com/cryspen/hax /opt/hax/src \
    && (cd /opt/hax/src && git checkout "${HAX_REV}")

# Cache the large opam dependency graph separately from hax-engine itself.
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

# Bring in only the pinned source tree (for the Lean proof library) and native OCaml protocol
# helper. The final image contains neither opam nor the OCaml compiler used to build it.
COPY --from=hax-ocaml-builder /opt/hax/src /opt/hax/src
COPY --from=hax-ocaml-builder /opt/hax-ocaml/hax-engine /usr/local/bin/hax-engine

ARG HAX_LEAN_TOOLCHAIN=leanprover/lean4:v4.29.0-rc1

# mise's ubi backend installs elan as a bare binary, so it does not create lean/lake proxy shims.
# Link the real binaries from the exact downloaded toolchain, as in the Aeneas stage below. Keeping
# provisioning separate from the proof-library build also preserves this expensive layer on failure.
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=hax mise exec -- elan toolchain install "${HAX_LEAN_TOOLCHAIN}" \
    && tc="$(ls -d /opt/hax/elan/toolchains/*/ | head -n1)" \
    && for b in lean lake leanc; do ln -sf "${tc}bin/${b}" "/usr/local/bin/${b}"; done \
    && lean --version

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=hax mise exec -- bash -c \
        'cd /opt/hax/src/hax-lib/proof-libs/lean && lake build' \
    && chmod -R a+rwX /opt/hax /opt/cargo /opt/mise /opt/rustup

# Cheap self-test in a separate layer so the installs and Lean library build above remain cached.
RUN MISE_ENV=hax mise exec -- cargo hax --version \
    && MISE_ENV=hax mise exec -- sh -c 'command -v hax-rust-engine' \
    && command -v hax-engine \
    && lean --version

FROM tooling-base AS aeneas-tooling

# The heaviest lane, and the most structurally different: Aeneas does not check Rust in place. Charon
# (a rustc driver) extracts the crate to an LLBC model, Aeneas translates that to Lean, and the
# proofs are hand-written Lean checked with Lean. So this stage bakes three toolchains: an
# OCaml/opam environment (Charon-ML + Aeneas are OCaml, like Creusot's Why3), the Charon+Aeneas
# binaries built from source at Aeneas's pinned commits (no release/crate/rustup component exists),
# and a Lean toolchain via elan whose Aeneas base library pulls in mathlib. Everything mise CAN pin
# it does in mise/config.aeneas.toml (Charon's nightly + elan); the from-source builds live here.
#
# Baked under a fixed /opt/aeneas that run-tooling.sh points HOME/XDG/OPAMROOT/ELAN_HOME at when
# MISE_ENV=aeneas, mirroring the Creusot stage's opam plumbing.
COPY mise/config.aeneas.toml ./mise/config.aeneas.toml

ENV XDG_DATA_HOME=/opt/aeneas/data
ENV XDG_CONFIG_HOME=/opt/aeneas/config
ENV XDG_CACHE_HOME=/opt/aeneas/cache
ENV OPAMROOT=/opt/aeneas/opam
ENV ELAN_HOME=/opt/aeneas/elan
ENV PATH=/opt/aeneas/elan/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
# Let opam auto-install system depexts non-interactively via apt.
ENV OPAMYES=1
ENV OPAMCONFIRMLEVEL=unsafe-yes

# Aeneas has no semantic releases (only dated nightly-* tags); pin its HEAD commit. Charon's commit
# is not chosen here - it is whatever Aeneas's charon-pin names (kept in sync; re-check on bump).
ARG AENEAS_REV=d71d2e3f2cb763f7faf15ab606ac9cf32da8dead
ARG CHARON_REV=5a5017333554ec1e95cae3c4fe05f5bd90b44487

# (1) apt (OCaml build deps, same family as Creusot) + mise tools (Charon's nightly toolchain + elan)
# + an opam switch. apt lists are kept through the opam package install below so opam depext can pull
# system libraries, then removed. opam 5.3.0 is the switch Aeneas develops against.
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

# (2) opam packages Aeneas (and Charon-ML) build against. Its own layer so this expensive OCaml
# compile stays cached across iterations of the Charon/Aeneas/Lean builds below. List mirrors
# Aeneas's README setup command; `core` covers `core_unix`.
RUN opam install --switch=5.3.0 --yes \
        calendar core core_unix domainslib easy_logging menhir ocamlformat.0.27.0 \
        ocamlgraph odoc ppx_deriving ppx_deriving_yojson progress unionFind visitors yojson zarith \
    && rm -rf /var/lib/apt/lists/* \
    && chmod -R a+rwX /opt/aeneas

# (3) Build Charon then Aeneas from source. Charon is cloned into aeneas/charon at the charon-pin
# commit (Aeneas's check-charon expects it there); `make build` compiles the release Charon binaries
# (cargo under Charon's rustup-pinned nightly) and Charon-ML (dune under the opam switch). Aeneas's
# `make` then builds bin/aeneas. Both run with the opam env active AND the mise nightly on PATH.
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

# (4) Lean toolchain + the Aeneas Lean base library. elan (installed as a bare binary via the mise
# ubi backend, so it never sets up the lean/lake proxy shims) fetches the toolchain named in
# backends/lean/lean-toolchain (leanprover/lean4:v4.31.0) into ELAN_HOME. We then link lean/lake/leanc
# straight to that toolchain's real binaries on PATH - they locate their own sysroot from their real
# path, so no elan proxy is needed at run time. `lake exe cache get` downloads mathlib's prebuilt
# oleans (the library `require`s mathlib, and compiling it from source would take hours); `lake build`
# then builds the Aeneas library. Its packages stay under /opt/aeneas/src so verif/ticketing_aeneas
# can reuse them at verify/replay time.
ARG LEAN_TOOLCHAIN=leanprover/lean4:v4.31.0
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=aeneas mise exec -- elan toolchain install "${LEAN_TOOLCHAIN}" \
    && tc="$(ls -d /opt/aeneas/elan/toolchains/*/ | head -n1)" \
    && for b in lean lake leanc; do ln -sf "${tc}bin/${b}" "/usr/local/bin/${b}"; done \
    && lean --version

# Fetch mathlib's prebuilt oleans and build the Aeneas Lean library, in its own layer so re-running it
# does not re-download the Lean toolchain above.
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && MISE_ENV=aeneas mise exec -- bash -c 'cd /opt/aeneas/src/backends/lean \
        && lake exe cache get \
        && lake build' \
    && chmod -R a+rwX /opt/aeneas

# Cheap self-test in a separate layer so the expensive builds above stay cached.
RUN command -v charon && command -v charon-driver && command -v aeneas \
    && aeneas --help >/dev/null \
    && ELAN_HOME=/opt/aeneas/elan lean --version
