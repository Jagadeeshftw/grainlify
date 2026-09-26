# Repository size after removing committed `target*` build output

Recorded as part of fixing issue #1889.

| Metric | Value |
| --- | --- |
| Upstream repo size (GitHub API `size`, KiB) before cleanup commit | 295195 |
| Tracked blobs removed under `target*` directories | 5112 |
| Directories removed | `target-final`, `target-fresh`, `target-gov`, `target-t2`, `contracts/bounty_escrow/contracts/escrow/target-test` |
| Note | GitHub's aggregate `size` field updates asynchronously and still includes unreachable blobs until GC; the authoritative post-change signal is **zero** `target*` paths in `git ls-files`. |

## Verification command

```bash
git ls-files | grep -E '(^|/)target(-[^/]*)?/' || echo 'OK: no target* paths tracked'
```
