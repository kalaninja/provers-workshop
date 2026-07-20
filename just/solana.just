import 'common.just'

mod tooling

# SBF artefacts land in the shared workspace target; the test runners locate the built .so through it.
sbf_env := "SBF_OUT_DIR=/workspace/target/deploy"

# Show pinned Solana and SBF toolchain versions
version: tooling::build-solana
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} solana --version
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo build-sbf --version

# Build one Solana program crate for SBF (defaults to the Creusot lane; every lane implements the
# same wire protocol and decisions, e.g. `just solana build ticketing verus`).
# Build one lane's Solana program for SBF (defaults to ticketing/Creusot).
build package='ticketing' lane='creusot': tooling::build-solana
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo build-sbf --skip-tools-install --manifest-path crates/{{ package }}/{{ lane }}/Cargo.toml --features bpf-entrypoint

# Full Solana test suite for a package/lane: build the SBF artefact once, run the generated client's
# lane-agnostic SBF suite (via the `test-client-rust` dependency; just deduplicates the shared build),
# then run that program crate's own test targets. The Creusot crate has additional Mollusk integration
# tests; other lanes currently rely primarily on the shared client suite plus their unit tests.
# Build and run the complete Solana test suite for one package and lane.
test package='ticketing' lane='creusot': (build package lane) (test-client-rust package lane)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env {{ sbf_env }} cargo nextest run --manifest-path crates/{{ package }}/{{ lane }}/Cargo.toml --features bpf-entrypoint

# Fast check: only the program's Mollusk integration tests (defaults to the Creusot lane).
test-it package='ticketing' lane='creusot': (build package lane)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env {{ sbf_env }} cargo nextest run --manifest-path crates/{{ package }}/{{ lane }}/Cargo.toml --features bpf-entrypoint -E 'kind(test)'

# Build a lane's SBF artefact, then run the generated Rust client's integration tests against it.
# The client suite is lane-agnostic (it loads `ticketing_<lane>.so`), so it must pass for every
# lane: `just solana test-client-rust ticketing <lane>`.
# Run the shared client SBF suite for one lane.
test-client-rust package='ticketing' lane='creusot': (build package lane)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env {{ sbf_env }} TICKETING_PROGRAM_NAME=ticketing_{{ lane }} cargo nextest run -p {{ package }}-client --features svm-tests -E 'kind(test)'

# Generate the codama IDL for a Solana program
gen-idl package='ticketing': tooling::build-solana
    ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo run -p {{ package }}-interface --features idl-gen --bin idl-gen

# Render the codama Rust client from the committed IDL
gen-client package='ticketing': tooling::build-rust tooling::build-node
    MISE_EXEC_TOOLS="node pnpm" ./scripts/docker/run-tooling.sh {{ node_tooling_image }} env CI=true bash -c 'cd clients && pnpm install --frozen-lockfile && pnpm exec codama run rust'
    MISE_ENV=nightly ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo fmt -p {{ package }}-client
