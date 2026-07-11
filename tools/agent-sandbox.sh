#!/usr/bin/env bash
set -euo pipefail

# agent-sandbox.sh — run opencode in a Docker container on a git worktree.
#
# Creates a throwaway worktree on a new branch, mounts it into a container,
# and runs opencode.  The agent can read, write, commit, and push on that
# branch without touching the main working tree.
#
# Usage:
#   tools/agent-sandbox.sh --auto "fix the build"
#   tools/agent-sandbox.sh --auto --agent sandbox "add a feature"
#
# Any arguments are forwarded to `opencode`.

REPO_ROOT="$(git rev-parse --show-toplevel)"
TIMESTAMP="$(date +%s)"
BRANCH_NAME="agent-${TIMESTAMP}"
WORKTREE_DIR="/tmp/diaria-${TIMESTAMP}"

cd "$REPO_ROOT"

# ── cleanup ───────────────────────────────────────────────────────────
cleanup() {
    echo "→ Cleaning up worktree …" >&2
    git worktree remove "$WORKTREE_DIR" 2>/dev/null || true
    git branch -D "$BRANCH_NAME" 2>/dev/null || true
}
trap cleanup EXIT

# ── worktree ──────────────────────────────────────────────────────────
echo "→ Creating worktree at ${WORKTREE_DIR} (branch: ${BRANCH_NAME})" >&2
git worktree add -b "$BRANCH_NAME" "$WORKTREE_DIR"

# ── Docker image ──────────────────────────────────────────────────────
echo "→ Building Docker image (diaria-agent) …" >&2
docker build -q -t diaria-agent "$REPO_ROOT" >/dev/null

# ── run ───────────────────────────────────────────────────────────────
echo "→ Starting opencode in container (worktree: ${BRANCH_NAME})" >&2
exec docker run -it --rm \
    --name "diaria-agent-${TIMESTAMP}" \
    -v "${WORKTREE_DIR}:/workspace" \
    -w /workspace \
    --network host \
    -e "OPENCODE_AUTO_SHARE=false" \
    diaria-agent \
    "$@"
