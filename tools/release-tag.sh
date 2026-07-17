#!/usr/bin/env bash
# Cut a release: pick the next version, sync Cargo.toml, commit, open a PR with
# auto-merge, wait for the squash merge to land on main, then push the tag.
#
# git-cliff derives the bump from the conventional commits since the last tag
# (feat -> minor, fix -> patch, breaking -> major). Override with VERSION=vX.Y.Z.
# The tag is pushed only after the PR merges, so the release workflow (which
# watches for tags on main) fires against the merge commit.
#
# Intended to be run via `mise run tag` (mise puts git-cliff on PATH). Requires
# `gh` for opening the pull request.
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

current_branch="$(git rev-parse --abbrev-ref HEAD)"
printf '\nTag and open a PR for %s (from %s -> main)? [y/N] ' "$version" "$current_branch"
read -r reply
[ "$reply" = y ] || { echo "aborted"; exit 0; }

# Keep the crate version in step with the tag so the binary's --version is honest.
if [ "$(uname)" = "Darwin" ]; then
  sed -i '' -E "s/^version = \".*\"/version = \"$number\"/" Cargo.toml
else
  sed -i -E "s/^version = \".*\"/version = \"$number\"/" Cargo.toml
fi
cargo check --quiet   # rewrite the version recorded in Cargo.lock
git add Cargo.toml Cargo.lock
# Only commit when the version actually moved. On the first release the tag is
# seeded from Cargo.toml, so it may already match and there is nothing to commit.
if git diff --cached --quiet; then
  echo "Cargo.toml already at $number; tagging existing commit."
else
  git commit -m "chore(release): $version"
fi
branch="release/$version"
git push --force origin "$current_branch:$branch"
pr_url="$(gh pr list --head "$branch" --json url --jq '.[0].url // empty')"
if [ -z "$pr_url" ]; then
  pr_url="$(gh pr create \
    --base main \
    --head "$branch" \
    --title "chore(release): $version" \
    --body "$(git cliff --unreleased --tag "$version" --strip header)")"
fi
gh pr merge "$pr_url" --auto --squash --subject "chore(release): $version"
echo "PR: $pr_url (auto-merge enabled). Waiting for merge..."
while :; do
  state="$(gh pr view "$pr_url" --json state --jq .state)"
  [ "$state" = MERGED ] && break
  [ "$state" = CLOSED ] && { echo "PR was closed without merging. Tag not pushed." >&2; exit 1; }
  sleep 10
done
echo "PR merged. Tagging and pushing $version on main."
git fetch origin main
if git rev-parse "$version" >/dev/null 2>&1; then
  echo "Tag $version already exists locally; deleting and recreating from origin/main."
  git tag -d "$version"
fi
git tag -a "$version" -m "Release $version" origin/main
git push --force origin "$version"
echo "Tag $version pushed to main. Release workflow will fire."
