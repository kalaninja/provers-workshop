import 'common.just'

mod tooling

# Show pinned Solana and SBF toolchain versions
version: tooling::build-solana
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} solana --version
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo build-sbf --version

# Build one Solana program crate for SBF (defaults to the Creusot lane; the program is
# behavior-identical across lanes, e.g. `just solana build ticketing verus`).
build package='ticketing' lane='creusot': tooling::build-solana
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo build-sbf --skip-tools-install --manifest-path crates/{{ package }}/{{ lane }}/Cargo.toml --features bpf-entrypoint

# Full Solana test suite for a package/lane: build the SBF artefact once, then run the
# program's unit + Mollusk integration tests AND the generated client's SBF tests.
test package='ticketing' lane='creusot': (build package lane)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy cargo nextest run --manifest-path crates/{{ package }}/{{ lane }}/Cargo.toml --features bpf-entrypoint
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy TICKETING_PROGRAM_NAME=ticketing_{{ lane }} cargo nextest run -p {{ package }}-client --features svm-tests -E 'kind(test)'

# Fast check: only the program's Mollusk integration tests (defaults to the Creusot lane).
test-it package='ticketing' lane='creusot': (build package lane)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy cargo nextest run --manifest-path crates/{{ package }}/{{ lane }}/Cargo.toml --features bpf-entrypoint -E 'kind(test)'

# Build a lane's SBF artefact, then run the generated Rust client's integration tests against it.
# The client suite is lane-agnostic (it loads `ticketing_<lane>.so`), so it must pass for every
# lane: `just solana test-client-rust ticketing creusot|kani|verus`.
test-client-rust package='ticketing' lane='creusot': (build package lane)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy TICKETING_PROGRAM_NAME=ticketing_{{ lane }} cargo nextest run -p {{ package }}-client --features svm-tests -E 'kind(test)'

# Generate the codama IDL for a Solana program
gen-idl package='ticketing': tooling::build-solana
    ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo run -p {{ package }}-interface --features idl-gen --bin idl-gen

# Render the codama Rust client from the committed IDL
gen-client package='ticketing': tooling::build-rust tooling::build-node
    MISE_EXEC_TOOLS="node pnpm" ./scripts/docker/run-tooling.sh {{ node_tooling_image }} env CI=true bash -c 'cd clients && pnpm install --frozen-lockfile && pnpm exec codama run rust'
    MISE_ENV=nightly ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo fmt -p {{ package }}-client
