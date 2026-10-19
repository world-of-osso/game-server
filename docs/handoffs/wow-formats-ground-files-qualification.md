# Ground-files qualification — before-only

Approved scope: freeze actual `scripts/ground_files.py` ordered file/missing/stdout/error outcomes on existing real and synthetic inputs. No decoder replacement, deployment, extraction, CLI bridge, API, asset, dependency or pin change. CASC/WDC5 tools and BLP panics are separate unapproved work.

## Observer boundary

`scripts/tests/test_ground_files_qualification.py` imports the actual existing module, observes `GroundFiles` state after each walk (including partial progress on errors), and invokes the actual `main()` twice with argv: normal file listing and `--missing`. Only argv and stdout/stderr streams are redirected; no reader/SQL/filesystem method is replaced. Exception type/message and exact printed text are preserved. Existing receipts cannot be overwritten. New test is skipped unless a frozen manifest is explicitly supplied.

```text
WOW_GROUND_INPUTS=/home/osso/.worktrees/logs/wow_formats-phase8/inputs.json
WOW_GROUND_OUTPUT=/home/osso/.worktrees/logs/wow_formats-phase8/before
PYTHONDONTWRITEBYTECODE=1 agent-run wow_formats python3 -m unittest discover -s scripts/tests -p test_ground_files_qualification.py -v
```

## Frozen corpus and limits

91 cases:4 partial-cache views of unmodified real metadata (maps0,34,2991 and combined),1 empty-cache view of the same real Map rows,86 concrete synthetic cases.49 existing source files are symlinked read-only; no assets are copied, downloaded or extracted.655 input paths, including frozen SQLite fixtures and production reader/SQL/hosted-continent registry, have complete sizes/SHA-256 in `inputs.json`.

The real views use exact read-only Map/source-row projections from world.db but intentionally bounded cache contents, not whole deployed caches. Their valid empty vehicle catalogs isolate ground metadata while still executing the production SQL/schema. No full vehicle selection/catalog qualification is claimed. Actual source production files remain unchanged.

Synthetic fixtures cover partial/oversized frames, arbitrary tags/tails, short MPHD, MAID/MODF/MOHD/GFID lengths, first-tag/first-global-WMO selection, zero/path FDIDs, extra LOD groups, duplicate dependencies, per-map missing ordering, absent WDT IDs and explicit map/build errors. Assertions pin concrete complete-closure stdout/order, permissive truncated GFID behavior and duplicate/global selection; the rest are exact before-only observations, not correctness claims.

## Existing policy not to replace blindly

`chunks()` keeps first tags and slices truncated payloads permissively; incomplete final headers are ignored. MODF iterates complete64-byte entries. `add_wmo()` uses available GFID entries up to MOHD count rather than requiring all entries. Global WMO mode accepts the first nonzero MODF identity regardless of its FDID flag; obj0 mode requires flag0x8. Per-map `seen` and cumulative `present`/`missing` ordering belong to the script. Current strict runtime projections must not silently replace these behaviors.

Capture/check results and a proposed reuse boundary are pending. No production change follows qualification without separate approval.
