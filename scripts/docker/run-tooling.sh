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
