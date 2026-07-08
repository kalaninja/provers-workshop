import 'common.just'

# Build a Dockerfile tooling stage. Passes GITHUB_TOKEN as a BuildKit secret when available
# (GitHub-hosted tools download via the API; the token avoids anonymous rate limits).
_build target image:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "${SKIP_IMAGE_BUILD:-}" ]]; then exit 0; fi
    token="${GITHUB_TOKEN:-}"
    if [[ -z "${token}" ]] && command -v gh >/dev/null 2>&1; then
        token="$(gh auth token 2>/dev/null || true)"
    fi
    if [[ -n "${token}" ]]; then
        GITHUB_TOKEN="${token}" DOCKER_BUILDKIT=1 docker build \
            --target {{ target }} \
            --secret id=github_token,env=GITHUB_TOKEN \
            --tag {{ image }} .
    else
        DOCKER_BUILDKIT=1 docker build \
            --target {{ target }} \
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
