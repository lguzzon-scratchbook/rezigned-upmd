# Git hooks
Install: `sh scripts/install-git-hooks.sh`
Runs `cargo fmt -- --check` and `cargo clippy -- -D warnings` (mirrors CI lint job).
Bypass (rare): `git commit --no-verify`
Re-run install anytime; it is idempotent.
