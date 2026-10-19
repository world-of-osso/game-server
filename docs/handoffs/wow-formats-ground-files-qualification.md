# Ground-files qualification — before-only

Approved scope: freeze actual `scripts/ground_files.py` ordered file/missing/stdout/error outcomes on existing real and synthetic inputs. No decoder replacement, deployment, extraction, CLI bridge, API, asset, dependency or pin change. CASC/WDC5 tools and BLP panics are separate unapproved work.

## Observer boundary

`scripts/tests/test_ground_files_qualification.py` imports the actual existing module, observes `GroundFiles` state after each walk (including partial progress on errors), and runs the actual standalone CLI twice in fresh subprocesses: normal file listing and `--missing`. No reader/SQL/filesystem method is replaced. Direct-walk exception type/message and complete CLI exit code/stdout/stderr (including actual tracebacks) are preserved. Existing receipts cannot be overwritten. New test is skipped unless a frozen manifest is explicitly supplied.

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

## Captured before-only proof

Observer at commitdaff59c6f passes1/0 in5.817s. It executes91direct walks and182real standalone CLI processes. Raw walks:79success/12struct.error. Each CLI mode:77exit0/14exit1; the two extra failures are explicit unknown-map/build selection.92complete JSON receipts total1,027,362B, with full count/size/SHA-256 inventory. All655 input paths/size/hash/symlink targets remain unchanged. The audit also asserts all77successful normal/missing outputs equal the direct ordered lists exactly, including empty-list newlines; raw errors appear verbatim in standalone stderr.

| Real view | Present | Missing | CLI |
| --- | ---: | ---: | ---: |
| Map0 partial |12|2387|0|
| Map34 existing closure |29|0|0|
| Map2991 partial |5|145|0|
| Combined partial |46|2532|0|
| All parents absent |0|3|0|

Full evidence is `/home/osso/.worktrees/logs/wow_formats-phase8/{inputs,before-receipt}.json` and `before/*.json`; capture log `wow_formats-phase8-capture.log`. Generator/audit scripts are `wow_formats-phase8-{inputs,audit}.py`. Those receipts preserve original failures, not a parser-correctness or old/new equivalence claim. No deployment/extraction command was run. Production source and both SQL files are byte-identical to the frozen input hashes.

## Proposed boundary, not approved replacement

Do not substitute current runtime parsers. Observed permissive short chunk/tail behavior, MAID floor(len/32), MODF complete64-byte iteration, missing-MOHD count0 and count-limited availableGFID differ from stricter full-runtime prerequisites. For example, count2/oneGFID accepts group30 without inventing missinggroup31;8GFID bytes then report group31missing.

The smallest safe common layer would decode raw chunk headers, oneMAID root/obj0 pair, MODFidentity/flags and MOHDcount/availableGFID words only. Keep selection/defaults/validation timing/error class+message, cache/map/build traversal/dedup/output policy in Python. Existing `parse_gfid`, `modf_flags`, `parse_mphd_flags` and word readers cover much of that layer; a per-record MAID/header projection and cross-language transport need separate approval.

Keep the script unchanged for now. A batched raw-record entry point in an existing Rust package could avoid a fifth format crate, but its invocation/deployment contract is a real new dependency decision, not a free parser refactor. Do not invent it for one caller, opportunistically discover binaries, add fallback parsing or normalize existing refusal behavior. CASC/WDC5 and BLP panics remain separate. Detailed proposal: `/home/osso/.worktrees/logs/wow_formats-phase8-boundary-proposal.md`. No production/API/CLI-bridge change follows without approval.
