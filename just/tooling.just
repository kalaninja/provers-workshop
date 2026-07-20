import 'common.just'

# Build a tooling stage from the selected Dockerfile. Passes GITHUB_TOKEN as a BuildKit secret
# (GitHub-hosted tools download via the API; the token avoids anonymous rate limits).
_build dockerfile target image build_args="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "${SKIP_IMAGE_BUILD:-}" ]]; then exit 0; fi
    token="${GITHUB_TOKEN:-}"
    if [[ -z "${token}" ]] && command -v gh >/dev/null 2>&1; then
        token="$(gh auth token 2>/dev/null || true)"
    fi
    if [[ -n "${token}" ]]; then
        GITHUB_TOKEN="${token}" DOCKER_BUILDKIT=1 docker build \
            --file {{ dockerfile }} \
            --target {{ target }} {{ build_args }} \
            --secret id=github_token,env=GITHUB_TOKEN \
            --tag {{ image }} .
    else
        DOCKER_BUILDKIT=1 docker build \
            --file {{ dockerfile }} \
            --target {{ target }} {{ build_args }} \
            --tag {{ image }} .
    fi

# Build-arg that points a split prover Dockerfile's FROM at its shared base image tag.
bookworm_base_build_arg := "--build-arg WORKSHOP_TOOLING_BASE_IMAGE=" + tooling_base_bookworm_image
trixie_base_build_arg := "--build-arg WORKSHOP_TOOLING_BASE_IMAGE=" + tooling_base_trixie_image
opam_base_build_arg := "--build-arg WORKSHOP_TOOLING_BASE_IMAGE=" + tooling_base_opam_image
# hax builds on the plain bookworm base but runs its isolated OCaml helper stage on the opam base.
hax_base_build_args := bookworm_base_build_arg + " --build-arg WORKSHOP_OPAM_BASE_IMAGE=" + tooling_base_opam_image

_build-bookworm-prover dockerfile target image: \
    (_build "Dockerfile" "tooling-base" tooling_base_bookworm_image) \
    (_build dockerfile target image bookworm_base_build_arg)

_build-trixie-prover dockerfile target image: \
    (_build "Dockerfile" "tooling-base" tooling_base_trixie_image "--build-arg TOOLING_BASE_IMAGE=debian:trixie-slim") \
    (_build dockerfile target image trixie_base_build_arg)

_build-opam-prover dockerfile target image: \
    (_build "Dockerfile" "opam-base" tooling_base_opam_image) \
    (_build dockerfile target image opam_base_build_arg)

# Build the slim Rust tooling image
build-rust: (_build "Dockerfile" "rust-tooling" rust_tooling_image)

# Build the Solana tooling image (rust + Agave/SBF)
build-solana: (_build "Dockerfile" "solana-tooling" solana_tooling_image)

# Build the Node tooling image (Codama codegen plus Markdown and link checks)
build-node: (_build "Dockerfile" "node-tooling" node_tooling_image)

# Build the Creusot formal-verification tooling image (creusot nightly + Why3/SMT). Builds on the
# shared opam base (bookworm) for the opam-built Why3 toolchain.
# Build the Creusot tooling image.
build-creusot: (_build-opam-prover "docker/creusot.Dockerfile" "creusot-tooling" creusot_tooling_image)

# Build the Kani bounded-model-checking tooling image (kani-verifier + CBMC backend)
build-kani: (_build-bookworm-prover "docker/kani.Dockerfile" "kani-tooling" kani_tooling_image)

# Build the Verus tooling image (verus + cargo-verus + bundled Z3 over the pinned 1.96.0
# toolchain). Uses the shared trixie base: the binaries need glibc >= 2.39 (bookworm has 2.36).
# Build the Verus tooling image.
build-verus: (_build-trixie-prover "docker/verus.Dockerfile" "verus-tooling" verus_tooling_image)

# Build the Flux refinement-typing tooling image (flux built from source + liquid-fixpoint + Z3
# over the pinned nightly-2025-11-25 toolchain). Overrides the base to trixie: the prebuilt Z3
# release asset needs glibc >= 2.39 (bookworm has 2.36).
# Build the Flux tooling image.
build-flux: (_build-trixie-prover "docker/flux.Dockerfile" "flux-tooling" flux_tooling_image)

# Build the hax-to-Lean tooling image (published cargo frontend/driver/Rust engine, the matching
# nightly, elan/Lean, the native OCaml protocol helper, and hax's prebuilt Lean proof library).
# Stays on bookworm; opam/OCaml exist only in the isolated helper-builder stage (opam base).
# Build the hax tooling image.
build-hax: \
    (_build "Dockerfile" "tooling-base" tooling_base_bookworm_image) \
    (_build "Dockerfile" "opam-base" tooling_base_opam_image) \
    (_build "docker/hax.Dockerfile" "hax-tooling" hax_tooling_image hax_base_build_args)

# Build the Aeneas interactive-proof tooling image (Charon + Aeneas built from source via opam, plus
# a Lean/elan toolchain and the mathlib-backed Aeneas Lean library). Builds on the shared opam base
# (bookworm — Lean/elan and the rustup nightly support glibc 2.36).
# Build the Aeneas tooling image.
build-aeneas: (_build-opam-prover "docker/aeneas.Dockerfile" "aeneas-tooling" aeneas_tooling_image)

# Build the VeriFast separation-logic tooling image (prebuilt binary distribution from GitHub
# Releases: the bin/verifast CLI + built-in Redux prover). Stays on bookworm.
# Build the VeriFast tooling image.
build-verifast: (_build-bookworm-prover "docker/verifast.Dockerfile" "verifast-tooling" verifast_tooling_image)
