# Runtime terrain height reader consolidation

Active goal: move only MCNK/MCVT byte decoding to `wow-adt`/`wow-chunk`. Preserve source ordering, failure precedence (complete root collection, MH2O validation, MCNK decoding), raw coordinates, hole widening, query/seam/interpolation semantics and MH2O association. No data edits, deployment, BLP or other format/policy changes.

## Proof contract

Before production edits, `terrain_height::format_receipts_tests::` captures required numeric ADTs 778027/778232 with old readers. `WOW_FORMATS_HEIGHT_MODE=capture`, `WOW_FORMATS_HEIGHT_OUTPUT=<empty directory>` and `GAME_SERVER_GROUND_DIR=<engine data>` select capture. Replay uses `WOW_FORMATS_HEIGHT_MODE=replay`, a separate empty output and `WOW_FORMATS_HEIGHT_REFERENCE=<old output>`. Default test mode checks the required corpus without writing files.

Height records: THGT/version/count (12 B), source-order origin XYZ float bits, area u32, 145 height float bits and widened hole u64 (604 B/chunk). Query points: THQI/version/count then exact XY float bits; generated once by old readers and loaded unchanged during replay. Query outcomes: THQY/version/count then XY, tagged terrain height, area, Bevy height and liquid (35 B/query). None payloads are zeroed; float values retain their bits.

Synthetic input files freeze first-MCVT selection, absent optional render payloads, low/high holes, source order, short/truncated inputs and combined-invalid error precedence. Negative outcomes include exact errors or complete decoded height bytes. Compare complete inventories, counts, sizes, SHA-256 and actual bytes, not float tolerances. Freeze all real and synthetic input hashes before/after.

Additional acceptance: unchanged `terrain_height::tests::` and real Northshire/swimming/Jasperlode ground controls; fresh Stockade four LOS/four NAV byte identity. Library tests/fmt before publication; both consumers pin the published revision. Independent integration gates remain lead-owned.

## Proof ledger

- Baseline source: server `ff4595fa3`, library `bb989f276`, protocol `f9a91753bbbc15ad7ff7392955b52c4da989d22e`; old-reader test-only harness `c6fa736cd` passes23/0; four exact real ground controls4/0. Fresh Stockade bake exits0 (requires `navigation/bake`). Baseline receipt: `/home/osso/.worktrees/logs/wow_formats-phase4-reference-receipt.json`,35 outputs19,169,465B,115 real input hashes. Each ADT256 chunks/154636B;135428 frozen XY queries/4739992B outcomes;17 synthetic outcomes5685B.
- Shared library `ed180cecd7228471944dcdf1d427468e5cc6e7b5`: new public fields API RED on missing imports; four behavioral tests GREEN with default246/0/0, no-default166/0/0, check/fmt0. Published before either consumer pin. Server source `f750ef94d3304273b1e055b6d89dfe36319c9335` (includes current master) replays the frozen baseline:23/0 height/harness +4/0 exact real ground controls; fresh Stockade4LOS+4NAV; check/fmt0, no warnings. `/home/osso/.worktrees/logs/wow_formats-phase4-after-receipt.json` plus `wow_formats-phase4-receipt.py after` prove35/35 files byte-identical (19,169,465B), identical inventories/counts/sizes/SHA-256 and115/115 real inputs unchanged. Synthetic inputs are included in the compared output inventory. No coordinates were regenerated. No production changes after that proof; subsequent ledger edits do not invalidate it. Engine source95a7f014e uses the same pinned revision/protocol: helper core `adt`9/0, Godot `terrain::`45/0, helper check/fmt0. Initial legacy `wmo::collision_tests` filter selected0 and is excluded. Independent lead integration remains pending.
