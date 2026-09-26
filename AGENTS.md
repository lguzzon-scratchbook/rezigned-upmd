# AGENTS.md

## Agent skills

- **Issue tracker**: GitHub issues in fork `lguzzon-scratchbook/rezigned-upmd`; pin `gh` with `-R lguzzon-scratchbook/rezigned-upmd`. See `docs/agents/issue-tracker.md`.
- **Triage labels**: five canonical labels mapped to tracker strings. See `docs/agents/triage-labels.md`.
- **Domain docs**: single-context layout (`CONTEXT.md` + `docs/adr/`); consumer rules only. See `docs/agents/domain.md`.

## Pointers

- Runner registry: `crates/upmd-runner/src/languages/mod.rs`; `javascript.rs` is the clone template for new runners.
- TUI preview: `src/apps/tui/preview/mod.rs`, `src/apps/tui/markdown.rs`, `src/apps/tui/wrap.rs`.
- Skills live in `~/.agents/skills/<name>/SKILL.md`; invoked prompts may truncate, read full `SKILL.md`.
