#!/usr/bin/env bash

set -euo pipefail

image="${1:?usage: run-tooling.sh <image> <command> [args...]}"
shift

image_platform="${WORKSHOP_IMAGE_PLATFORM:-linux/amd64}"
repo_root="${PWD}"
host_cache_root="${repo_root}/.cache"
# Env vars must carry container paths: the repo is mounted at /workspace, so
# the host-side cache tree resolves to /workspace/.cache inside the container.
cache_root="/workspace/.cache"
home_dir="${cache_root}/home"

xdg_cache_home="${cache_root}/xdg/cache"
xdg_config_home="${cache_root}/xdg/config"
xdg_data_home="${cache_root}/xdg/data"
extra_env=()

if [[ "${MISE_ENV:-}" == "solana" ]]; then
  home_dir="/opt/solana-home"
fi

# Creusot's toolchain/Why3/provers/confs are baked under /opt/creusot (creusot-tooling stage);
# point HOME/XDG/OPAM/WHY3CONFIG there so they resolve despite the per-run HOME/XDG rewrite below.
if [[ "${MISE_ENV:-}" == "creusot" ]]; then
  home_dir="/opt/creusot/home"
  xdg_cache_home="/opt/creusot/cache"
  xdg_config_home="/opt/creusot/config"
  xdg_data_home="/opt/creusot/data"
  extra_env+=(
    --env "OPAMROOT=/opt/creusot/opam"
    --env "WHY3CONFIG=/opt/creusot/config/creusot/why3.conf"
  )
fi

# Kani bakes its compiler, CBMC backend, and matching toolchain under /opt/kani (kani-tooling
# stage, KANI_HOME=/opt/kani); point KANI_HOME there so `cargo kani` resolves them despite the
# per-run HOME/XDG rewrite. The baked toolchain symlink resolves via RUSTUP_HOME=/opt/rustup.
if [[ "${MISE_ENV:-}" == "kani" ]]; then
  extra_env+=(--env "KANI_HOME=/opt/kani")
fi

# Flux bakes its sysroot (flux-driver + precompiled flux_rs/flux_attrs libs) under /opt/flux
# (flux-tooling stage, FLUX_SYSROOT=/opt/flux); point FLUX_SYSROOT there so `cargo flux` resolves the
# driver and extern crates despite the per-run HOME rewrite. The pinned nightly resolves via
# RUSTUP_HOME=/opt/rustup; z3 and fixpoint are on PATH from /usr/local/bin.
if [[ "${MISE_ENV:-}" == "flux" ]]; then
  extra_env+=(--env "FLUX_SYSROOT=/opt/flux")
fi

# hax's elan-managed Lean toolchain and the prebuilt Hax proof library live under /opt/hax in the
# hax-tooling stage. The frontend/driver/engine and matching Rust nightly resolve through mise;
# only ELAN_HOME must survive the per-run HOME rewrite.
if [[ "${MISE_ENV:-}" == "hax" ]]; then
  extra_env+=(--env "ELAN_HOME=/opt/hax/elan")
fi

# Aeneas bakes its opam switch, Charon/Aeneas binaries, and elan-managed Lean toolchain (with the
# built Aeneas + mathlib Lean packages) under /opt/aeneas (aeneas-tooling stage); point HOME/XDG/
# OPAMROOT/ELAN_HOME there so opam, dune, and lean/lake resolve despite the per-run HOME/XDG rewrite.
# Charon's pinned nightly resolves via RUSTUP_HOME=/opt/rustup; charon/aeneas/lean/lake are on PATH
# from /usr/local/bin.
if [[ "${MISE_ENV:-}" == "aeneas" ]]; then
  home_dir="/opt/aeneas/home"
  xdg_cache_home="/opt/aeneas/cache"
  xdg_config_home="/opt/aeneas/config"
  xdg_data_home="/opt/aeneas/data"
  extra_env+=(
    --env "OPAMROOT=/opt/aeneas/opam"
    --env "ELAN_HOME=/opt/aeneas/elan"
  )
fi

mkdir -p \
  "${host_cache_root}/cargo/git" \
  "${host_cache_root}/cargo/registry" \
  "${host_cache_root}/home" \
  "${host_cache_root}/npm" \
  "${host_cache_root}/xdg/cache" \
  "${host_cache_root}/xdg/config" \
  "${host_cache_root}/xdg/data"

docker_cmd=(
  docker run --rm
  --platform "${image_platform}"
  --user "$(id -u):$(id -g)"
  --volume "${repo_root}:/workspace"
  --workdir /workspace
  --env "HOME=${home_dir}"
  --env MISE_DATA_DIR=/opt/mise
  --env MISE_TRUSTED_CONFIG_PATHS=/workspace
  --env MISE_YES=1
  --env RUSTUP_HOME=/opt/rustup
  --env "XDG_CACHE_HOME=${xdg_cache_home}"
  --env "XDG_CONFIG_HOME=${xdg_config_home}"
  --env "XDG_DATA_HOME=${xdg_data_home}"
)

if [[ "${#extra_env[@]}" -gt 0 ]]; then
  docker_cmd+=("${extra_env[@]}")
fi

if [[ -n "${MISE_ENV:-}" ]]; then
  docker_cmd+=(--env "MISE_ENV=${MISE_ENV}")
fi

mise_exec_tools=()
if [[ -n "${MISE_EXEC_TOOLS:-}" ]]; then
  docker_cmd+=(--env "MISE_EXEC_TOOLS=${MISE_EXEC_TOOLS}")
  read -r -a mise_exec_tools <<< "${MISE_EXEC_TOOLS}"
fi

docker_cmd+=("${image}" mise exec)
if [[ "${#mise_exec_tools[@]}" -gt 0 ]]; then
  docker_cmd+=("${mise_exec_tools[@]}")
fi
docker_cmd+=(-- "$@")

"${docker_cmd[@]}"
