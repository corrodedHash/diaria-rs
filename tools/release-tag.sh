#!/usr/bin/env bash
# Cut a release: pick the next version, sync Cargo.toml, commit, tag, push.
#
# git-cliff derives the bump from the conventional commits since the last tag
# (feat -> minor, fix -> patch, breaking -> major). Override with VERSION=vX.Y.Z.
# Pushing the tag is what triggers the release workflow.
#
# Intended to be run via `mise run tag` (mise puts git-cliff on PATH).
set -euo pipefail

git diff --quiet && git diff --cached --quiet || {
  echo "error: working tree is not clean; commit or stash first" >&2
  exit 1
}

if [ -n "${VERSION:-}" ]; then
  version="$VERSION"
elif [ -z "$(git tag)" ]; then
  # First release: no tag to bump from, so seed from Cargo.toml.
  version="v$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
else
  version="$(git cliff --bumped-version)"
fi
number="${version#v}"

echo "== Notes for $version =="
git cliff --unreleased --tag "$version" --strip header

printf '\nTag and push %s? [y/N] ' "$version"
read -r reply
[ "$reply" = y ] || { echo "aborted"; exit 0; }

# Keep the crate version in step with the tag so the binary's --version is honest.
sed -i '' -E "s/^version = \".*\"/version = \"$number\"/" Cargo.toml
cargo check --quiet   # rewrite the version recorded in Cargo.lock
git add Cargo.toml Cargo.lock
git commit -m "chore(release): $version"
git tag -a "$version" -m "Release $version"
git push --follow-tags
echo "Pushed $version — the release workflow will build and publish it."
