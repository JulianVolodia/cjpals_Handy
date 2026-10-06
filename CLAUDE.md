# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

For all standard development commands, architecture, tech stack, and contribution workflow, read
[AGENTS.md](AGENTS.md) first — it is the canonical reference and this file does not repeat it.

## What this fork is

This is `JulianVolodia/cjpals_Handy`, a personal fork of [cjpais/Handy](https://github.com/cjpais/Handy)
(upstream remote `origin`). Branch layout:

- **`main`** — untouched mirror of `origin/main`. Sync from upstream here (fast-forward only); never
  commit rebrand or feature work directly to this branch.
- **`jv-main`** — `main` + a cosmetic rebrand only, described below. Safe to treat as "stock Handy,
  reinstallable alongside the real thing."
- **`jv-dev`** (this branch) — `jv-main` + actual feature work, described below. Rebase it onto an
  updated `jv-main` when syncing upstream, not the other way around.

### Rebrand (on both `jv-main` and `jv-dev`)

`productName` → `HandyJV`, `identifier` → `com.jv.handyjv` (was `com.pais.handy`) in
`src-tauri/tauri.conf.json`, and every app + tray icon (`src-tauri/icons/**`,
`src-tauri/resources/tray_*.png`) rotated 180° — so a running build is visually and identity-distinct
from a stock Handy install on the same machine. Regenerate derived icon formats after changing the
master icon with `bun run tauri icon src-tauri/icons/icon.png`.

### Feature: constrain Nemotron auto-detect to an allowed-language set (`jv-dev` only)

Reimplements the goal of [cjpais/Handy#2138](https://github.com/cjpais/Handy/pull/2138) (closed,
unmerged by the maintainer) — letting a user pick e.g. English + Polish so Nemotron streaming's
auto-detect doesn't wander across all ~40 locales it supports.

**This is deliberately not that PR's approach.** #2138 needed decode-time language-token masking added
to transcribe-cpp's native C++ decoder (`RunOptions.allowed_languages`), which only exists on an
unmerged `luantak/transcribe.cpp` branch that sits one commit behind current transcribe-cpp `main` and
conflicts non-trivially (generated bindings, ABI hash, the C++ decoder itself) if ported forward by hand
— that was tried and abandoned here as too risky for a personal fork to carry long-term.

Instead, `managers/transcription.rs` implements it entirely in Rust against transcribe-cpp **exactly as
published on crates.io** (no patch, no forked native library, no version pin):

- When `selected_language` is `"auto"` and `allowed_languages` is non-empty, the stream worker buffers
  ~700ms of audio (`LANGUAGE_TRIAL_AUDIO_MS`) at stream start instead of starting the stream immediately.
- `identify_language_by_trial()` runs one forced `RunOptions{language: candidate}` trial decode per
  allowed language on that buffer and scores each by average per-token confidence (`Token::p`, a softmax
  probability parakeet's decoder already reports — see transcribe-cpp's `result.rs`).
- The real stream then begins pinned to whichever candidate decoded most confidently, and the buffered
  audio is replayed into it as the first feed so nothing is lost to the trial.
- `nemotron_trial_candidates()` narrows the setting to codes the loaded model actually advertises.

Tradeoff versus true constrained decoding: this identifies a language once per utterance (from the first
~700ms) rather than constraining every decode step, and costs one extra short decode per candidate at
the start of each stream. In exchange it needs zero native code and tracks transcribe-cpp's published
releases normally.

- Setting: `AppSettings.allowed_languages: Vec<String>` (`src-tauri/src/settings.rs`), empty = unrestricted
  auto-detect (unchanged behavior).
- Tauri command: `change_allowed_languages_setting` (`src-tauri/src/shortcut/mod.rs`), registered in
  `lib.rs`'s `collect_commands!`. Needed because the frontend's `updateSetting()` dispatches through a
  per-field `settingUpdaters` map (`src/stores/settingsStore.ts`), not a fully generic setter.
  `bun run tauri dev`/`tauri build` regenerates `src/bindings.ts` on startup (debug builds only;
  `cargo build` alone does not trigger the specta export).
- Frontend: a row of toggle chips in `src/components/settings/LanguageSelector.tsx`, shown only when
  `selected_language` resolves to `"auto"` and the model offers more than one language. Writes the
  model's exact advertised codes (e.g. `en-US`) straight through — matches what
  `nemotron_trial_candidates()` compares against.

### Working with upstream

- `origin` = `cjpais/Handy` (read-only reference point). Sync this fork's `main` from it normally
  (fast-forward only); rebrand/feature work never touches `main`.
- Push rebrand changes to `jv-main`, rebase `jv-dev` on top, re-verify (`cargo build` + `cargo test --lib`
  + `bun run build` + `bun run lint`) before pushing `jv-dev`.
