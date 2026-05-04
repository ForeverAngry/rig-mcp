# rig-mcp task runner.
#
# Install just: https://github.com/casey/just
#   brew install just

default:
    @just --list

build:
    cargo build --all-targets

check: fmt clippy test doc

fmt:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all-targets --all-features

doc:
    RUSTDOCFLAGS="-D warnings -D rustdoc::broken_intra_doc_links" cargo doc --all-features --no-deps

publish-dry-run:
    cargo publish --dry-run

release-preview:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp="$(mktemp -d)"
    trap 'rm -rf "${tmp}"' EXIT
    rsync -a --exclude target --exclude .git ./ "${tmp}/"
    cd "${tmp}"
    git init -q
    git config user.email "release-preview@example.invalid"
    git config user.name "Release Preview"
    git add .
    git commit -q -m "feat: prepare rig-mcp release preview"
    release-plz update --repo-url https://github.com/ForeverAngry/rig-mcp

release-pr:
    release-plz release-pr

next-version:
    @just release-preview 2>&1 | grep -E "(next version|already up-to-date|rig-mcp)" || true
