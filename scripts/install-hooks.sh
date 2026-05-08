#!/usr/bin/env bash
# Install Mitosis-Clustering git hooks as symlinks into .git/hooks.
#
# Run once after cloning the repo:
#     ./scripts/install-hooks.sh
#
# Idempotent — re-running replaces the symlink.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
hooks_dir="$repo_root/.githooks"
target_dir="$repo_root/.git/hooks"

if [[ ! -d "$hooks_dir" ]]; then
    echo "install-hooks: $hooks_dir not found; nothing to install" >&2
    exit 1
fi

mkdir -p "$target_dir"

for hook in "$hooks_dir"/*; do
    name="$(basename "$hook")"
    link_path="$target_dir/$name"
    rm -f "$link_path"
    ln -s "$hook" "$link_path"
    echo "installed: $link_path -> $hook"
done
