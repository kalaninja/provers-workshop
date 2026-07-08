import 'common.just'

mod tooling

# Show pinned Solana and SBF toolchain versions
version: tooling::build-solana
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} solana --version
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo build-sbf --version

# Build one Solana program crate for SBF
build package='ticketing': tooling::build-solana
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo build-sbf --skip-tools-install --manifest-path crates/{{ package }}/program/Cargo.toml --features bpf-entrypoint

# Full Solana test suite for a package: build the SBF artefact once, then run the
# program's unit + Mollusk integration tests AND the generated client's SBF tests.
test package='ticketing': (build package)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy cargo nextest run --manifest-path crates/{{ package }}/program/Cargo.toml --features bpf-entrypoint
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy cargo nextest run -p {{ package }}-client --features svm-tests -E 'kind(test)'

# Fast check: only the program's Mollusk integration tests.
test-it package='ticketing': (build package)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy cargo nextest run --manifest-path crates/{{ package }}/program/Cargo.toml --features bpf-entrypoint -E 'kind(test)'

# Build the SBF artefact, then run the generated Rust client's integration tests
test-client-rust package='ticketing': (build package)
    MISE_ENV=solana ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} env SBF_OUT_DIR=/workspace/target/deploy cargo nextest run -p {{ package }}-client --features svm-tests -E 'kind(test)'

# Generate the codama IDL for a Solana program
gen-idl package='ticketing': tooling::build-solana
    ./scripts/docker/run-tooling.sh {{ solana_tooling_image }} cargo run -p {{ package }}-interface --features idl-gen --bin idl-gen

# Render the codama Rust client from the committed IDL
gen-client package='ticketing': tooling::build-rust tooling::build-node
    MISE_EXEC_TOOLS="node pnpm" ./scripts/docker/run-tooling.sh {{ node_tooling_image }} env CI=true bash -c 'cd clients && pnpm install --frozen-lockfile && pnpm exec codama run rust'
    MISE_ENV=nightly ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo fmt -p {{ package }}-client
