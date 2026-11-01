#!/usr/bin/env bash
# SPDX-License-Identifier: EUPL-1.2
# Verification gate shared by git hooks, CI and agents (D-30, docs/testing.md).
#
# Usage: scripts/check.sh [TIER] [--base REV]
#   lint   whitespace, SPDX headers, rustfmt, markdownlint, shellcheck,
#          actionlint (seconds, no compilation)
#   test   CI: clippy with all features, clippy of each crate without its
#          features, and all tests including the slow ones (#[ignore])
#          with all features except GPU backends
#   quick  pre-commit: lint, plus clippy when code files are staged
#   push   pre-push: lint, plus clippy and the fast tests of the packages
#          affected by code changes since REV
#   full   (default) lint and test, as in CI
#   deep   scheduled CI: SKAT_RANDOM_GAMES seeded random games (default
#          1,000,000) and SKAT_MEPHISTO_GAMES Mephisto games (default
#          100), release build
#   code-changed REV  exit 0 if code files changed since REV, 1 if not
#
# With --base REV, `push` and `full` skip compiling and testing when no
# code file changed since REV.
# Linters missing locally are skipped with a warning; with CHECK_STRICT=1
# (set in CI) a missing linter fails.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

tier=${1:-full}
base=
if [ "${2:-}" = --base ]; then
    base=${3:?--base needs a revision}
fi

# Files whose change requires compiling and testing (docs/testing.md).
code_re='\.rs$|(^|/)Cargo\.(toml|lock)$|^rust-toolchain|^assets/|^scripts/check\.sh$|^\.github/workflows/ci\.yml$'
markdownlint_version=0.23.3

step() {
    printf '\n==> %s\n' "$*"
    "$@"
}

# Code files changed since the merge base with REV, committed or not. An
# unknown REV prints `?`, which stands for "everything changed".
changed_code() {
    local mb
    if ! mb=$(git merge-base "$1" HEAD 2>/dev/null); then
        echo '?'
        return
    fi
    { git diff --name-only "$mb" HEAD && git diff --name-only HEAD; } |
        grep -E "$code_re" | sort -u || true
}

code_changed() {
    [ -n "$(changed_code "$1")" ]
}

# Workspace packages to test after the code changes since REV: each package
# whose directory changed and every package depending on it. Any other code
# file (manifests, lock file, toolchain, assets, the gate) affects all and
# prints `--workspace`. One cargo argument per line.
affected() {
    local files f dir pkgs=()
    files=$(changed_code "$1")
    if grep -qvE '^(crates|tools)/[^/]+/' <<<"$files"; then
        echo --workspace
        return
    fi
    while IFS= read -r f; do
        dir=$(cut -d/ -f1-2 <<<"$f")
        pkgs+=("$(sed -n 's/^name = "\(.*\)"/\1/p' "$dir/Cargo.toml" | head -n 1)")
    done <<<"$files"
    for f in $(printf '%s\n' "${pkgs[@]}" | sort -u); do
        cargo tree --locked -i "$f" --workspace -e normal,dev --prefix none |
            awk '{ print $1 }'
    done | sort -u | awk '{ print "-p"; print }'
}

# Packages that declare features. Selected alone with `-p`, they build
# without them; the whole workspace cannot, because `skat-app` enables
# `serde` everywhere (feature unification).
feature_packages() {
    local f
    for f in crates/*/Cargo.toml tools/*/Cargo.toml; do
        if grep -q '^\[features\]' "$f"; then
            sed -n 's/^name = "\(.*\)"/\1/p' "$f" | head -n 1
        fi
    done
}

# `--features` for the tests: every feature of the given packages (all
# without arguments) except GPU backends, which need an adapter that the
# Linux runner lacks and would replace Burn's default CPU device. Clippy
# still checks them with --all-features (docs/testing.md).
test_features() {
    local f name feats=()
    for f in crates/*/Cargo.toml tools/*/Cargo.toml; do
        name=$(sed -n 's/^name = "\(.*\)"/\1/p' "$f" | head -n 1)
        if [ $# -gt 0 ] && ! printf '%s\n' "$@" | grep -qxF "$name"; then
            continue
        fi
        while IFS= read -r feat; do
            feats+=("$name/$feat")
        done < <(sed -n '/^\[features\]/,/^\[/s/^\([A-Za-z0-9_-]*\) *=.*/\1/p' "$f" |
            grep -vxE 'default|vulkan')
    done
    if [ ${#feats[@]} -gt 0 ]; then
        (IFS=,; echo "--features=${feats[*]}")
    fi
}

have() {
    command -v "$1" > /dev/null && return 0
    if [ "${CHECK_STRICT:-}" = 1 ]; then
        echo "error: $1 is not installed" >&2
        exit 1
    fi
    echo "warning: $1 is not installed; skipped here, CI runs it" >&2
    return 1
}

# Every source file needs an SPDX header in its first lines (AGENTS.md).
spdx() {
    local missing=() f
    while IFS= read -r f; do
        [ -f "$f" ] || continue
        head -n 5 "$f" | grep -q 'SPDX-License-Identifier' || missing+=("$f")
    done < <(git ls-files -- '*.rs' '*.toml' '*.sh' '*.py' '*.yml' '*.yaml' '.githooks/*')
    if [ ${#missing[@]} -gt 0 ]; then
        printf 'missing SPDX header: %s\n' "${missing[@]}" >&2
        return 1
    fi
}

markdown() {
    local globs=('**/*.md' '#target')
    if command -v markdownlint-cli2 > /dev/null; then
        markdownlint-cli2 "${globs[@]}"
    elif have npx; then
        npx --yes "markdownlint-cli2@$markdownlint_version" "${globs[@]}"
    fi
}

shell_scripts() {
    local files
    mapfile -t files < <(git ls-files -- '*.sh' '.githooks/*')
    if have shellcheck; then
        shellcheck "${files[@]}"
    fi
}

workflows() {
    if have actionlint; then
        actionlint
    fi
}

lint() {
    step git diff --check "$(git hash-object -t tree /dev/null)"
    step spdx
    step cargo fmt --check
    step markdown
    step shell_scripts
    step workflows
}

clippy() {
    step cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
}

# The only difference without features is compile-level (derives, wire
# types), so clippy suffices (docs/testing.md).
features_off() {
    local p
    for p in $(feature_packages); do
        step cargo clippy --locked -p "$p" --all-targets -- -D warnings
    done
}

run_tests() {
    clippy
    features_off
    local features
    mapfile -t features < <(test_features)
    step cargo test --locked --workspace "${features[@]}" -- --include-ignored
}

# Fast tests only: slow ones carry #[ignore = "slow: ..."] and run in CI.
fast_tests() {
    local pkgs names=() features
    mapfile -t pkgs < <(affected "$1")
    if [ "${pkgs[0]}" != --workspace ]; then
        mapfile -t names < <(printf '%s\n' "${pkgs[@]}" | grep -vx -- -p)
    fi
    mapfile -t features < <(test_features "${names[@]}")
    step cargo test --locked "${features[@]}" "${pkgs[@]}"
}

case $tier in
    lint)
        lint
        ;;
    test)
        run_tests
        ;;
    quick)
        lint
        if git diff --cached --name-only | grep -qE "$code_re"; then
            clippy
        fi
        ;;
    push)
        lint
        if [ -z "$base" ]; then
            clippy
            mapfile -t features < <(test_features)
            step cargo test --locked --workspace "${features[@]}"
        elif code_changed "$base"; then
            clippy
            fast_tests "$base"
        else
            printf '\n==> no code changes since %s; tests skipped\n' "$base"
        fi
        ;;
    full)
        lint
        if [ -z "$base" ] || code_changed "$base"; then
            run_tests
        else
            printf '\n==> no code changes since %s; tests skipped\n' "$base"
        fi
        ;;
    deep)
        export SKAT_RANDOM_GAMES=${SKAT_RANDOM_GAMES:-1000000}
        export SKAT_MEPHISTO_GAMES=${SKAT_MEPHISTO_GAMES:-100}
        step cargo test --locked --release -p skat-core --test random_games
        step cargo test --locked --release -p skat-ai --test variants -- \
            --include-ignored --exact mephisto_never_loses_a_game_it_declares
        ;;
    code-changed)
        code_changed "${2:?code-changed needs a revision}"
        exit
        ;;
    *)
        echo "usage: $0 [lint|test|quick|push|full|deep] [--base REV] | code-changed REV" >&2
        exit 2
        ;;
esac
printf '\n==> check %s passed\n' "$tier"
