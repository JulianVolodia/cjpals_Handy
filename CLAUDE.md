# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

For all standard development commands, architecture, tech stack, and contribution workflow, read
[AGENTS.md](AGENTS.md) first — it is the canonical reference and this file does not repeat it.

## What this fork is

This is `JulianVolodia/cjpals_Handy`, a personal fork of [cjpais/Handy](https://github.com/cjpais/Handy)
(upstream remote `origin`; this fork's own GitHub remote is typically added as e.g. `jv`). It exists to
run a feature upstream rejected (closed, not merged) under a distinct local identity so it can coexist
with a stock Handy install.

### Fork-specific changes vs upstream `main`

1. **Nemotron auto-detect language allowlist** (`feat: select allowed auto-detection languages in Handy`,
   cherry-picked from `luantak/Handy#feat/nemotron-language-picker-pr`, upstream PR
   [cjpais/Handy#2138](https://github.com/cjpais/Handy/pull/2138), closed unmerged by the maintainer).
   Lets the user pick multiple preferred languages (e.g. English + Polish) that constrain Nemotron
   streaming's automatic language detection, instead of detecting across all ~40 supported locales.
   - New setting: `AppSettings.allowed_languages: Vec<String>` (`src-tauri/src/settings.rs`).
   - Resolved per-run via `nemotron_allowed_languages()` in `src-tauri/src/managers/transcription.rs`,
     only applied when `selected_language == "auto"` and a model supports language detection; threaded
     into `transcribe-cpp`'s `RunOptions.allowed_languages`.
   - Frontend multi-select UI in `src/components/settings/LanguageSelector.tsx`.
   - **This required pinning `transcribe-cpp`/`transcribe-cpp-sys` to version `0.2.3` (not upstream's
     `0.3.0`/`0.3.1`)** via `[patch.crates-io]` in `src-tauri/Cargo.toml`, pointing at
     `luantak/transcribe.cpp@feat/nemotron-language-allowlist` (rev `a2b3e9e`) — the only rev with the
     `RunOptions.allowed_languages` field. That branch sits one commit behind current
     `handy-computer/transcribe.cpp` main and conflicts non-trivially (C++ decoder, generated bindings,
     ABI hash) if cherry-picked forward, so this fork trades away whatever upstream changed in
     transcribe-cpp 0.3.x (e.g. "parakeet ultra") to get the allowlist feature. Re-evaluate this pin
     before merging future upstream Handy bumps that touch `transcribe-cpp`.
2. **Rebrand for side-by-side install** (`chore: rebrand to HandyJV`): `productName` → `HandyJV`,
   `identifier` → `com.jv.handyjv` (was `com.pais.handy`) in `src-tauri/tauri.conf.json`, and every app +
   tray icon (`src-tauri/icons/**`, `src-tauri/resources/tray_*.png`) rotated 180° so the running app is
   visually distinguishable from a stock Handy instance. Regenerate derived icon formats after changing
   the master icon with `bun run tauri icon src-tauri/icons/icon.png`.

### Working with upstream

- `origin` = `cjpais/Handy` (read-only reference point). Sync the fork's own `main` from it normally.
- Do not rebase the two fork-specific commits onto a new upstream bump without re-checking the
  `transcribe-cpp` pin above — a bump to `transcribe-cpp` in upstream `Cargo.toml` will silently make
  cargo ignore the `[patch.crates-io]` override again (version mismatch) and the build will fail with
  `RunOptions has no field named allowed_languages` rather than a merge conflict.
- Push feature/rebrand work to dedicated branches on the fork, not directly to `main`.
