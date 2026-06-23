# ADR-015: App search fuzzy matching, pinyin, and aliases

## Status

Accepted (2026-06-22)

## Context

Users expect launcher app search to behave like a Chinese-aware fuzzy finder: `微信` / `wx` / `wechat` must open WeChat; `vsc` must find Visual Studio Code.

## Decision

1. **AppRecord v2** stores `localizedName`, `aliases`, `pinyinFull`, `pinyinInitials` alongside bundle path metadata.
2. **Alias table** (`AppAliasTable.swift`) is hand-maintained (~30 bundle IDs) for domestic apps; no third-party pinyin library.
3. **Pinyin** via Apple `CFStringTransform` (`PinyinIndex.swift`); fallback to raw text on failure.
4. **Scoring tiers**: exact → prefix → substring → subsequence fuzzy on names and pinyin initials.
5. **Cache** migrates to `app-index-v2.json`; v1 cache ignored on load.

## Consequences

- Warmup cost grows slightly (one transform per app); acceptable in background Task.
- Alias table drifts over time; monthly manual review recommended.
- No fzf-style character weights in v0.2.
