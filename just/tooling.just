import 'common.just'

# Build a Dockerfile tooling stage. Passes GITHUB_TOKEN as a BuildKit secret when available
# (GitHub-hosted tools download via the API; the token avoids anonymous rate limits).
_build target image build_args="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "${SKIP_IMAGE_BUILD:-}" ]]; then exit 0; fi
    token="${GITHUB_TOKEN:-}"
    if [[ -z "${token}" ]] && command -v gh >/dev/null 2>&1; then
        token="$(gh auth token 2>/dev/null || true)"
    fi
    if [[ -n "${token}" ]]; then
        GITHUB_TOKEN="${token}" DOCKER_BUILDKIT=1 docker build \
            --target {{ target }} {{ build_args }} \
            --secret id=github_token,env=GITHUB_TOKEN \
            --tag {{ image }} .
    else
        DOCKER_BUILDKIT=1 docker build \
            --target {{ target }} {{ build_args }} \
            --tag {{ image }} .
    fi

# Build the slim Rust tooling image
build-rust: (_build "rust-tooling" rust_tooling_image)

# Build the Solana tooling image (rust + Agave/SBF)
build-solana: (_build "solana-tooling" solana_tooling_image)

# Build the Node tooling image (codama client codegen only)
build-node: (_build "node-tooling" node_tooling_image)

# Build the Creusot formal-verification tooling image (creusot nightly + Why3/SMT)
build-creusot: (_build "creusot-tooling" creusot_tooling_image)

# Build the Kani bounded-model-checking tooling image (kani-verifier + CBMC backend)
build-kani: (_build "kani-tooling" kani_tooling_image)

# Build the Verus tooling image (verus + cargo-verus + bundled Z3 over the pinned 1.96.0
# toolchain). Overrides the base to trixie: the verus binaries need glibc >= 2.39 (bookworm has 2.36).
build-verus: (_build "verus-tooling" verus_tooling_image "--build-arg TOOLING_BASE_IMAGE=debian:trixie-slim")

# Build the Flux refinement-typing tooling image (flux built from source + liquid-fixpoint + Z3
# over the pinned nightly-2025-11-25 toolchain). Overrides the base to trixie: the prebuilt Z3
# release asset needs glibc >= 2.39 (bookworm has 2.36).
build-flux: (_build "flux-tooling" flux_tooling_image "--build-arg TOOLING_BASE_IMAGE=debian:trixie-slim")
