# syntax=docker/dockerfile:1
ARG WORKSHOP_IMAGE_PLATFORM=linux/amd64
ARG WORKSHOP_TOOLING_BASE_IMAGE=workshop-tooling-base-opam:dev
FROM --platform=${WORKSHOP_IMAGE_PLATFORM} ${WORKSHOP_TOOLING_BASE_IMAGE} AS creusot-tooling

# Creusot installs only its own nightly, keeping the image slim.
COPY mise/config.creusot.toml ./mise/config.creusot.toml

ENV XDG_DATA_HOME=/opt/creusot/data
ENV XDG_CONFIG_HOME=/opt/creusot/config
ENV XDG_CACHE_HOME=/opt/creusot/cache
ENV OPAMROOT=/opt/creusot/opam
ENV WHY3CONFIG=/opt/creusot/config/creusot/why3.conf

# z3 stays on apt deliberately: the prebuilt Z3 release assets need glibc >= 2.39 (this image is
# bookworm, 2.36), and bookworm's 4.8.12 is frozen — the committed proof sessions replay against
# exactly that version.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends z3 \
    && rm -rf /var/lib/apt/lists/*

RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && mise install -E creusot rust \
    && chmod -R a+rwX /opt/cargo /opt/mise /opt/rustup

RUN mkdir -p /opt/creusot \
    && opam init --bare --disable-sandboxing --yes

ARG CREUSOT_REV=v0.12.0

# The heavy layer: Creusot's INSTALL builds Why3 and the solver setup into the opam root and
# installs cargo-creusot. opam resolves depexts (gtk/sourceview for the Why3 IDE) through apt
# during INSTALL, so the apt lists must be fresh inside this layer. A version bump re-runs only
# this layer and the smoke below; the chmod stays scoped to what this layer creates (a broader
# tree would copy-up into the layer).
RUN --mount=type=secret,id=github_token \
    export GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    && apt-get update \
    && git clone --depth 1 --branch "${CREUSOT_REV}" https://github.com/creusot-rs/creusot /tmp/creusot \
    && MISE_ENV=creusot mise exec rust -- bash -c 'cd /tmp/creusot && ./INSTALL --external z3' \
    && ln -sf /opt/cargo/bin/cargo-creusot /usr/local/bin/cargo-creusot \
    && rm -rf /tmp/creusot /var/lib/apt/lists/* \
    && chmod -R a+rwX /opt/creusot /opt/cargo/bin

# Smoke: cargo-creusot answers through the pinned toolchain (it has no version flag), and the
# baked Why3 platform plus the external z3 both respond.
RUN MISE_ENV=creusot mise exec rust -- cargo creusot --help >/dev/null \
    && /opt/creusot/data/creusot/bin/why3 --version \
    && z3 --version
