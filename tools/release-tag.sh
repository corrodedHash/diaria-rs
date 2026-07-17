#!/usr/bin/env bash
# Cut a release: create a release branch, bump version, PR, merge, then tag main.
#
# git-cliff derives the bump from the conventional commits since the last tag
# (feat -> minor, fix -> patch, breaking -> major). Override with VERSION=vX.Y.Z.
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
  version="v$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
else
  version="$(git cliff --bumped-version)"
fi
number="${version#v}"

echo "== Notes for $version =="
git cliff --unreleased --tag "$version" --strip header

printf '\nRelease %s? [y/N] ' "$version"
read -r reply
[ "$reply" = y ] || { echo "aborted"; exit 0; }

branch="release/$version"

# If a merged PR already exists for this release, skip straight to tagging.
pr_url="$(gh pr list --head "$branch" --json url --jq '.[0].url // empty')"
if [ -n "$pr_url" ]; then
  state="$(gh pr view "$pr_url" --json state --jq .state)"
  if [ "$state" = MERGED ]; then
    echo "PR $pr_url already merged."
    git switch main
    git pull --ff-only origin main
    git tag -a "$version" -m "Release $version"
    git push origin "$version"
    echo "Tag $version pushed to main."
    exit 0
  fi
  if [ "$state" = CLOSED ]; then
    echo "PR $pr_url is closed. Open or delete it first." >&2
    exit 1
  fi
fi

# Create release branch from current main HEAD.
git rev-parse --verify "$branch" >/dev/null 2>&1 && git branch -D "$branch"
git switch -c "$branch"

# Bump version and commit.
if [ "$(uname)" = "Darwin" ]; then
  sed -i '' -E "s/^version = \".*\"/version = \"$number\"/" Cargo.toml
else
  sed -i -E "s/^version = \".*\"/version = \"$number\"/" Cargo.toml
fi
cargo check --quiet
git add Cargo.toml Cargo.lock
git commit -m "chore(release): $version"

git push origin "$branch"

pr_url="$(gh pr create \
  --base main \
  --head "$branch" \
  --title "chore(release): $version" \
  --body "$(git cliff --unreleased --tag "$version" --strip header)")"

gh pr merge --auto --squash --subject "chore(release): $version"
echo "PR: $pr_url (auto-merge enabled). Waiting for merge..."
while :; do
  state="$(gh pr view "$pr_url" --json state --jq .state)"
  [ "$state" = MERGED ] && break
  [ "$state" = CLOSED ] && { echo "PR was closed without merging. Tag not pushed." >&2; exit 1; }
  sleep 10
done

echo "PR merged. Tagging and pushing $version on main."
git switch main
git pull --ff-only origin main
git rev-parse "$version" >/dev/null 2>&1 && git tag -d "$version"
git tag -a "$version" -m "Release $version"
git push --force origin "$version"
echo "Tag $version pushed to main. Release workflow will fire."
