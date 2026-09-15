#!/usr/bin/env bash
# What is allowed to live in this repository.
#
# The deliverable is a SQLite file built from SQL, TSV and Rust. Anything else
# arriving is either a mistake or a decision, and a decision should be visible
# in a diff to this file rather than in a directory nobody looked at.
#
#   check-tree.sh            every tracked file (what CI runs)
#   check-tree.sh --staged   only what is about to be committed (the hook)
#
# CI is the guard. The hook is the fast answer -- it has to be installed, and
# whoever most needs it is least likely to have installed it.
set -uo pipefail
cd "$(dirname "$0")/.."

# Extensions in the tree today, plus `png` for a diagram nobody has drawn yet.
# Adding one is a one-line diff, which is the point: it is a decision and it
# should look like one.
ALLOWED='sql|rs|md|tsv|yml|yaml|json|toml|sh|lock|source|gitignore|png'

# The largest tracked file is a 2 MB generated overlay. The mapdb is 41 MB and
# was committed once before being stripped out of history; four megabytes sits
# clear of everything legitimate and well under that.
MAX_BYTES=4194304

if [ "${1:-}" = "--staged" ]; then
    files=$(git diff --cached --name-only --diff-filter=ACM)
else
    files=$(git ls-files)
fi
[ -z "$files" ] && exit 0

fail=0
say() { printf '%s\n' "$*" >&2; fail=1; }

while IFS= read -r f; do
    [ -z "$f" ] && continue
    base=${f##*/}

    # A private key, a token file, an environment. None of these belong in any
    # repository, and this one is public.
    case "$base" in
        *.pem|*.key|*.p12|*.pfx|id_rsa*|id_ed25519*|.env|.env.*|*.crt)
            say "refusing $f: that looks like a credential, and this repository is public." ;;
    esac

    # The mapdb is fetched, not vendored. `.gitignore` covers it; `git add -f`
    # does not care, and the history was rewritten once to undo exactly this.
    case "$f" in
        vendor/map.json)
            say "refusing $f: the mapdb is fetched by scripts/vendor.sh, not committed. vendor/mapdb.source is the pin." ;;
    esac

    # Git hooks are named by git and must be extensionless -- `pre-commit`, not
    # `pre-commit.sh`. The first thing this script ever did was refuse itself.
    case "$f" in
        .githooks/*) continue ;;
    esac

    # LICENSE and the like have no extension and are fine; a *new* one is worth
    # a glance, so only known names pass.
    if [ "$base" = "${base%.*}" ]; then
        case "$base" in
            LICENSE|CONTRIBUTING|AGENTS|CLAUDE|README|Makefile|Dockerfile) ;;
            *) say "refusing $f: no extension, and not one of the names this repository expects." ;;
        esac
    else
        ext=${base##*.}
        if ! printf '%s' "$ext" | grep -qxE "$ALLOWED"; then
            say "refusing $f: .$ext is not a file type this repository keeps. If it should be, add it to ALLOWED in scripts/check-tree.sh and say why."
        fi
    fi

    # Size last, so a file fails on *what it is* before it fails on how big.
    if [ -f "$f" ]; then
        bytes=$(wc -c < "$f")
        if [ "$bytes" -gt "$MAX_BYTES" ]; then
            say "refusing $f: $bytes bytes, over the $MAX_BYTES limit. Large inputs are fetched and pinned, not committed."
        fi
    fi
done <<< "$files"

if [ "$fail" -ne 0 ]; then
    printf '\n%s\n' "Nothing was committed. Each line above says what to do about it." >&2
    exit 1
fi
