#!/bin/sh
set -e
# Idempotent: safe to re-run; uses core.hooksPath, never touches .git/hooks.
chmod +x .githooks/pre-commit
git config core.hooksPath .githooks
echo "git hooks installed (core.hooksPath=.githooks)"
