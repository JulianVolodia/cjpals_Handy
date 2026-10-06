# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

For all standard development commands, architecture, tech stack, and contribution workflow, read
[AGENTS.md](AGENTS.md) first — it is the canonical reference and this file does not repeat it.

## What this fork is

This is `JulianVolodia/cjpals_Handy`, a personal fork of [cjpais/Handy](https://github.com/cjpais/Handy)
(upstream remote `origin`). Branch layout:

- **`main`** — untouched mirror of `origin/main`. Sync from upstream here (fast-forward only); never
  commit rebrand or feature work directly to this branch.
- **`jv-main`** (this branch) — `main` + a cosmetic rebrand only, described below. Safe to treat as
  "stock Handy, reinstallable alongside the real thing."
- **`jv-dev`** — `jv-main` + actual feature work (a from-scratch reimplementation of
  [cjpais/Handy#2138](https://github.com/cjpais/Handy/pull/2138)'s goal — see that branch's own
  CLAUDE.md for the detail). Rebase it onto an updated `jv-main` when syncing upstream.

### Rebrand (on both `jv-main` and `jv-dev`)

`productName` → `HandyJV`, `identifier` → `com.jv.handyjv` (was `com.pais.handy`) in
`src-tauri/tauri.conf.json`, and every app + tray icon (`src-tauri/icons/**`,
`src-tauri/resources/tray_*.png`) rotated 180° — so a running build is visually and identity-distinct
from a stock Handy install on the same machine. Regenerate derived icon formats after changing the
master icon with `bun run tauri icon src-tauri/icons/icon.png`.
