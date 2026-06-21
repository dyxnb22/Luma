# Testing Iterations

## Current Automated Coverage

The current suite simulates the requested features through module-level and user-flow tests:

- App search and launch-action generation.
- Translate command result flow.
- Clipboard filtering, search, duplicate handling, pinning, pruning, and oversized rejection.
- Secrets vault lock/unlock/search/reveal safety.
- Window layout frame calculation for halves, center, offset, and small screens.
- Notes graph parsing for wiki links, tags, duplicate tags, empty notes, and malformed links.
- Wordbook review scheduling for known, fuzzy, and unknown responses.
- Keychain round-trip integration for secrets.
- Notes vault create/scan/graph integration.
- Existing wordbot SQLite read integration.
- Feature card catalog presence and unique positions.
- Combined user flow touching every requested feature once.

## Iteration Rule

For each round:

1. Run the basic suite.
2. Add divergent tests equal to roughly 50% of the new baseline surface.
3. If a test fails, keep it as part of the baseline.
4. Fix the implementation or test setup.
5. Re-run until the suite passes.

## Current Status

The latest suite passes with 35 tests and no known failing test items.
