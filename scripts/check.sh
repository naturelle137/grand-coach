#!/usr/bin/env bash
# SPDX-License-Identifier: EUPL-1.2
# Verification gate shared by git hooks, CI and agents (D-30, docs/testing.md).
#
# Usage: scripts/check.sh [TIER] [--base REV]
#   lint   whitespace, SPDX headers, rustfmt, markdownlint, shellcheck,
#          actionlint, offline Markdown links and fixtures (seconds, no compilation)
#   pr     Ubuntu PR: clippy/features, complete FAST workspace + app integration
#   native Windows: all-target/feature compilation, IO/model/GUI + render smoke
#   complete (alias test) full platform suite including ignored tests
#   regression main: complete tests without repeating clippy
#   route / ci-ok  classification / strict mandatory-result aggregation
#   test   full platform: clippy with all features, clippy of each crate without its
#          features, and all tests including the slow ones (#[ignore])
#          with all features except GPU backends
#   quick  pre-commit: lint, plus clippy when code files are staged
#   push   pre-push: lint, plus clippy and the fast tests of the packages
#          affected by code changes since REV (conservative whole workspace)
#   full   (default) lint and complete platform tests
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
# Only known documentation is cheap; every unknown path requires verification.
markdownlint_version=0.23.3

step() {
    printf '\n==> %s\n' "$*"
    local started=$SECONDS result=0
    "$@" || result=$?
    printf '==> timing: %ss (exit %s): %s\n' "$((SECONDS - started))" "$result" "$*"
    return "$result"
}

# Shared conservative classification for local hooks/full. Uncertainty must
# compile/test rather than silently skip; CI reports the classifier error.
code_changed() {
    local route
    if ! route=$(python3 scripts/ci/route.py route --base "$1"); then
        return 0
    fi
    grep -qx 'code=true' <<<"$route"
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
    if have python3; then
        step python3 scripts/ci/test_route.py
        step python3 scripts/markdown-links/test_check.py
        step python3 scripts/markdown-links/check.py
    fi
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

# Fast tests only: ignored cases remain owned by main/nightly/manual suites.
fast_tests() {
    local features
    mapfile -t features < <(test_features)
    step cargo test --locked --workspace "${features[@]}"
}

# Native compilation includes GPU features; execution keeps CPU device selection.
native() {
    step cargo check --locked --workspace --all-targets --all-features
    local p features test
    for p in $(feature_packages); do
        step cargo check --locked -p "$p" --all-targets
    done
    mapfile -t features < <(test_features skat-nn)
    step cargo test --locked -p skat-nn --test model "${features[@]}"
    mapfile -t features < <(test_features skat-ai)
    step cargo test --locked -p skat-ai --test trained_model "${features[@]}"
    mapfile -t features < <(test_features nn-train)
    for test in the_cache_returns_exactly_the_loaded_data cache_files_keep_wide_features; do
        step cargo test --locked -p nn-train --test data "${features[@]}" -- --exact "$test"
    done
    for test in launch_shows_the_new_series_settings every_card_face_renders \
        language_and_faces_switch_without_touching_the_game free_play_ends_on_request; do
        step cargo test --locked -p gui-egui --test gui -- --exact "$test"
    done
    local out
    out=$(mktemp -d)
    step cargo run --locked -p gui-egui --example screenshots -- "$out" launch
    test -s "$out/launch.png"
    if [ -n "${NATIVE_RENDER_DIR:-}" ]; then
        mkdir -p "$NATIVE_RENDER_DIR"
        cp "$out/launch.png" "$NATIVE_RENDER_DIR/launch.png"
    fi
    rm -rf "$out"
}

portable() {
    clippy
    features_off
    local features
    mapfile -t features < <(test_features)
    step cargo test --locked --workspace "${features[@]}"
    # Privacy at every strength stays mandatory, including existing slow fixtures.
    mapfile -t features < <(test_features skat-ai)
    step cargo test --locked -p skat-ai --test fairness "${features[@]}" -- --ignored
    # Retain deterministic async/stale-move and privileged-seat integration pre-merge.
    mapfile -t features < <(test_features skat-app)
    step cargo test --locked -p skat-app --test series "${features[@]}" -- --ignored
}

case $tier in
    lint)
        lint
        ;;
    route)
        shift
        exec python3 scripts/ci/route.py route "$@"
        ;;
    ci-ok)
        exec python3 scripts/ci/route.py aggregate
        ;;
    pr)
        portable
        ;;
    native)
        native
        ;;
    regression)
        mapfile -t features < <(test_features)
        step cargo test --locked --workspace "${features[@]}" -- --include-ignored
        ;;
    test|complete)
        run_tests
        ;;
    quick)
        lint
        if ! git diff --cached --name-only | python3 -B -c 'import sys; sys.path.insert(0, "scripts/ci"); from route import docs; sys.exit(0 if all(docs(p.rstrip("\n")) for p in sys.stdin) else 1)'; then
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
            fast_tests
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
        echo "usage: $0 [lint|pr|native|complete|regression|test|quick|push|full|deep|route|ci-ok] [--base REV] | code-changed REV" >&2
        exit 2
        ;;
esac
printf '\n==> check %s passed\n' "$tier"
