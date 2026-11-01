#!/bin/sh
# fetch.sh: clone latex2e + latex3 at the SHAs pinned in PINS.txt into
# tools/latex-suites/.cache/ (git-ignored; upstream files are NEVER committed).
# Network use: `git clone`/`git fetch` over HTTPS against the two public
# repos https://github.com/latex3/latex2e and https://github.com/latex3/latex3
# only. Usage: sh tools/latex-suites/fetch.sh
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CACHE="$HERE/.cache"

clone_at() {
    repo=$1 tag=$2 sha=$3
    dir="$CACHE/$repo"
    url="https://github.com/latex3/$repo"
    if [ -d "$dir/.git" ]; then
        git -C "$dir" fetch --depth 1 origin "refs/tags/$tag:refs/tags/$tag" 2>/dev/null \
            || git -C "$dir" fetch --depth 1 origin "$sha"
        git -C "$dir" checkout -q --detach "$sha"
    else
        git clone -q --depth 1 --branch "$tag" "$url" "$dir"
        git -C "$dir" checkout -q --detach "$sha"
    fi
    got=$(git -C "$dir" rev-parse HEAD)
    if [ "$got" != "$sha" ]; then
        echo "fetch.sh: SHA mismatch in $repo: got $got, want $sha" >&2
        exit 1
    fi
    echo "fetch.sh: $repo at $tag ($got)"
}

mkdir -p "$CACHE"
grep -v '^[[:space:]]*#' "$HERE/PINS.txt" | grep -v '^[[:space:]]*$' | while read -r repo tag sha; do
    clone_at "$repo" "$tag" "$sha"
done
