#!/usr/bin/env bash
set -euo pipefail

# agent-sandbox.sh — run opencode in a Docker container on a throwaway clone.
#
# A git worktree is not self-contained: its .git is a pointer back into the
# main repository's object database, so mounting only the worktree dir into a
# container leaves every git operation dangling.  Instead we make a full clone,
# which the container owns end to end.  The main repository's refs, objects and
# HEAD are never mounted, so the agent physically cannot touch them.
#
# On exit the agent's branch is fetched back into the main repo as
# refs/heads/<BRANCH_NAME> — a purely local, host-side transfer of objects
# straight from the clone dir, so neither the container nor the host needs
# to push to (or auth with) a remote.  From your normal checkout you can then
# review/merge/PR it.
#
# Usage:
#   tools/agent-sandbox.sh "fix the build"
#   tools/agent-sandbox.sh --no-auto "add a feature"
#
# Defaults to `--agent sandbox --auto`. `--no-auto` opts out of auto-approval.
# Any other arguments are forwarded to `opencode`.

REPO_ROOT="$(git rev-parse --show-toplevel)"
TIMESTAMP="$(date +%s)"
BRANCH_NAME="agent-${TIMESTAMP}"
SANDBOX_DIR="/tmp/diaria-${TIMESTAMP}"

cd "$REPO_ROOT"

# ── args ──────────────────────────────────────────────────────────────
# Defaults; user args may override (or --no-auto drops --auto).
opencode_args=(--agent sandbox --auto)
for arg in "$@"; do
    case "$arg" in
        --no-auto) opencode_args=(${opencode_args[*]/--auto}) ;;
        *)         opencode_args+=("$arg") ;;
    esac
done

# ── cleanup ───────────────────────────────────────────────────────────
cleanup() {
    echo "→ Importing branch '${BRANCH_NAME}' into main repo …" >&2
    git fetch --quiet "$SANDBOX_DIR" \
        "refs/heads/${BRANCH_NAME}:refs/heads/${BRANCH_NAME}" 2>/dev/null \
        || echo "  (clone gone or branch missing — nothing to import)" >&2

    echo "→ Removing sandbox clone at ${SANDBOX_DIR} …" >&2
    rm -rf "$SANDBOX_DIR"
}
trap cleanup EXIT

# ── clone ─────────────────────────────────────────────────────────────
echo "→ Cloning repo to ${SANDBOX_DIR} (branch: ${BRANCH_NAME})" >&2
git clone --no-hardlinks --quiet "$REPO_ROOT" "$SANDBOX_DIR"
git -C "$SANDBOX_DIR" checkout -b "$BRANCH_NAME"

# ── Docker image ──────────────────────────────────────────────────────
echo "→ Building Docker image (diaria-agent) …" >&2
docker build -q -t diaria-agent "$REPO_ROOT" >/dev/null

# ── run ───────────────────────────────────────────────────────────────
# The container is read-only except /workspace (the clone) and tmpfs /tmp, so
# git identity is supplied via env rather than a writable ~/.gitconfig.
echo "→ Starting opencode in container (branch: ${BRANCH_NAME})" >&2
docker run -it --rm \
    --name "diaria-agent-${TIMESTAMP}" \
    -v "${SANDBOX_DIR}:/workspace" \
    -v "${HOME}/.local/share/opencode/auth.json:/home/agent/.local/share/opencode/auth.json:ro" \
    -w /workspace \
    --network host \
    -e "OPENCODE_AUTO_SHARE=false" \
    -e "GIT_AUTHOR_NAME=diaria-agent" \
    -e "GIT_AUTHOR_EMAIL=agent@diaria.local" \
    -e "GIT_COMMITTER_NAME=diaria-agent" \
    -e "GIT_COMMITTER_EMAIL=agent@diaria.local" \
    diaria-agent \
    "${opencode_args[@]}"