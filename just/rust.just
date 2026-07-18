import 'common.just'

mod tooling

# Format the Rust workspace with pinned nightly rustfmt
fmt args='': tooling::build-rust
    MISE_ENV=nightly ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo fmt --all {{ args }}

# Check Rust formatting with pinned nightly rustfmt (the CI/pre-push variant of `fmt`)
fmt-check: (fmt "--check")

# Run Clippy for all workspace targets
clippy: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo clippy --workspace --all-targets -- -D warnings

# Run Clippy for one package
clippy-package package: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo clippy -p {{ package }} --all-targets -- -D warnings

# Scan the workspace for unused dependencies
machete: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo machete

# Run cargo check for the whole workspace
check: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo check --workspace

# Run cargo check for one package
check-package package: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo check -p {{ package }}

# Run cargo nextest for the whole workspace
test: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo nextest run --workspace

# Run cargo nextest for one package
test-package package: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo nextest run -p {{ package }}

# Run cargo nextest for workspace unit tests
test-ut: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo nextest run --workspace --lib

# Run cargo nextest for workspace integration tests
test-it: tooling::build-rust
    ./scripts/docker/run-tooling.sh {{ rust_tooling_image }} cargo nextest run --workspace -E 'kind(test)' --no-tests=pass
