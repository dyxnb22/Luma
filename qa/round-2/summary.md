# Round 2 Summary

| Area | Result |
|------|--------|
| gh swift package | ✅ GitHub Search row |
| Safari | ✅ Safari app row |
| kill preview | ⚠️ Preview not running on host |
| mb fold | ⚠️ Menu cache empty in automation |
| Unit tests | ✅ 362 passed |

## Root causes fixed
1. **Ranker** filtered all module results when full query didn’t fuzzy-match titles (`gh swift package` vs `GitHub`, `kill preview` vs `Preview`).
2. **AppScanner** missed symlinked Safari.app on macOS Cryptexes layout.
3. **QA harness** session restore raced with automation and concatenated queries.

## Next (Round 3)
- [ ] Run full `run_round1_smoke.sh` with updated `drive.sh`
- [ ] `kill preview` with Preview.app launched
- [ ] `mb fold` manual check or increase menu walk budget
- [ ] `doctor` keystroke→paint p95 screenshot
