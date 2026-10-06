# Dungeon achievements

Server loads pinned local Retail DB2 criteria, tracks copy-specific defeated encounters, sends achievement snapshots/toasts and dungeon objectives, and serves bounded achievement catalog pages on request. [Contract](../../specs/dungeon-achievements.md).

## Data and provenance

Verified 2026-10-06: `/mnt/c/World of Warcraft/.build.info` names retail build `12.1.0.69933`, build key `dcfc90fffd79ba00406ae46f5f657592`. Local community listfile maps Achievement=1260179, Criteria=1263817, CriteriaTree=1263818, Achievement_Category=1324299. Phase 2 extracts Category from the same local build to server data; layout `67B2B4BD`, 243 rows. Achievement display metadata adds Category column 7 and IconFileID column 12. Category columns are Name/ID/Parent/Ui_order; the relationship repeats Ui_order, not a parent ID.

Extraction (under game-engine's agent-run, no CDN/web):

```sh
scripts/agent/agent-run dungeonach cargo run --release --manifest-path /home/osso-test/Projects/world-of-osso/asset-resolver/Cargo.toml --bin casc-local -- 1260179 1263817 1263818 -o data/db2/12.1.0.69933
python3 /home/osso-test/.worktrees/game-server-recup/scripts/import_achievements.py --db2-dir /home/osso-test/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933 --db /home/osso-test/.worktrees/game-server-recup/data/world.db
```

Layout hashes: Achievement `6FC5281B`, Criteria `53D03513`, CriteriaTree `FF97AE12`. Exports contain 14,081 / 66,340 / 120,821 rows (copies included). Missing TACT key `CF9022A9913B1B4D` leaves one Criteria and eight CriteriaTree records unavailable; decoder reports zero-filled sections and skips them. Supported achievement trees require every referenced row to exist. Stockade and level rows are present.

Existing `scripts/db2.py extract` uses `db2_casc.py`; this importer follows that decoder path. Engine core's lighting reader is table-specific, not a general CSV exporter. Adjacent `.csv.import` files are Godot's `csv_translation` metadata, not DB2 export tools. No generated import/uid files are needed. CSV exports and original binaries remain uncommitted under game-engine `data/db2/12.1.0.69933/`. SQL subsets live in shared server `data/world.db`; `db2_build` stamps all three tables. Tests load concrete imported SQL rows; `sync_test_data` already mirrors `game-server/world.db`, so they need no additional CSV mirror entries.

## Runtime

`MapStore::encounters_for_map` filters map/difficulty (0/-1 apply to all difficulties) and sorts OrderIndex/ID. Each copy's existing LockData keeps script DONE bits separately from DB2 encounter bits. Every successful SetBossState(DONE) updates masks before achievement evaluation; unchanged DONE and wipes do not credit. Scheduled locks retain both masks in the existing redb instance-lock table. Normal Stockade checkpoints retain owner/map/difficulty/copy identity and both masks in `normal_instance_locks`, saved in the same transaction as scheduled locks. Startup reserves checkpoint IDs and resumes the owner's copy. Normal checkpoints do not impose scheduled binds or block reset: manual reset and copy unload remove them.

Stockade map 34 Normal has encounters 1144/1145/1146 at OrderIndex 1000/2000/3000, bits 3/4/5. Script boss indices run in a different order. Achievement 633, “Stormwind Stockade”, uses root 3385 (Any, amount 1), child 3386 (Single, amount 1), criterion 18527 (kill creature 46254 Hogger). Phase 1 adds the requested full-dungeon grant boundary: even when Hogger dies first, its achievement toast waits for the copy's entire encounter mask. This full-run boundary is a server requirement, not a claim that Retail's Hogger-only criterion itself requires all bosses.

Supported counters: kill creature type 0 accumulates, reach-level type 5 retains highest level, encounter type 165 accumulates matching DungeonEncounter IDs. Scripted bosses credit their configured creature on successful encounter completion; ordinary death credit excludes these bosses to avoid double counting. Current progress and earned IDs persist per character as JSON in existing redb `achievements`. Old JSON rows without counters undergo a one-time migration of placeholder IDs 1–6 to the actual earned Retail level IDs; the rewritten row includes counters and is never remapped again.

DB2 Retail level rows: 6/7/9/14783/15805/19459 for levels 10/20/40/60/70/80; these are not the old placeholder 1–6. Additional active Retail level rows (30,45,50) are also data-driven.

## CriteriaTree coverage

No TrinityCore checkout is available under `~/Repos`; interpretation uses observed local DB2 fields, not a fetched source. Operator 0 compares a criterion counter with Amount; 4 requires all children; 8 requires Amount completed children; 7 counts direct children with positive criterion counters. Operators 5/6/9 aggregate descendant counters by sum/max/weighted sum. Supported catalog trees exclude missing references, cycles, unknown/inverse operators, unsupported criterion types and conditional/timed criteria. These interpretations are inferred DB2 semantics, not independently source-verified Retail parity. Concrete level and Stockade trees anchor the All/Any behavior; count tests distinguish positive progress from completed thresholds.

## Proof ledger

- RED: baseline `c8cd38f` + Retail-ID assertion failed because placeholder achievement 1 was granted instead of Retail 6. `/tmp/claude/dungeonach-red.log`.
- `702cb80`: detached build-lock + agent-run + desktop-server-build with explicit `--build-host local`, filter `dungeonach`: eight tests passed; Stockade fixture lost its toast by draining after two frames. The Lightyear host receiver clears each frame (`lightyear_messages-0.28.0/src/receive.rs`, `clear_typed`). Saved `/tmp/claude/dungeonach-second-green.log`.
- `96cad53`: only that fixture changed to drain both frames. Same locked detached runner, filter `dungeonach_stockade_real_boss_deaths_record_ordered_progress_and_send_final_toast`: PASS, including real scripted boss deaths, ordered masks, copy isolation, final toast/snapshot, duplicate DONE and persisted character state. `target/server-test.log`, `/tmp/claude/dungeonach.out`.
- `702cb80`: native helper `--build-host local --check -p server`: PASS without warnings, 78s. Later fixture-only edit does not invalidate production check. `/tmp/claude/dungeonach-check.out`.
- `96cad53`: `cargo fmt --all -- --check`: PASS. Existing WDC5 decoder's `test_decrypted_sections_and_virtual_string_offsets`: PASS; later changes are Rust-only.

The eight other passing tests cover actual level thresholds, real encounter asset matching, All/Any/direct count, character progress reopen, one-time placeholder migration, scheduled lock restore and normal Stockade checkpoint restore/manual reset. No broad suites, protocol edits, push or merge.

## Phase 2 runtime and client handoff

[Client hookup contract, bounds and pinned Retail file:line references](../../dungeon-achievements-client.md). `achievement_window.rs` keeps full presentation metadata separate from the supported grant catalog: listing an unsupported achievement never enables its criteria. Fixed ID-cursor pages include the first criteria page, plus a separate continuation request. Criterion descriptions and order come from CriteriaTree. Catalog data is request-only; old snapshot/toast delivery remains unchanged.

`dungeon_progress.rs` detects new InWorld and copy changes, broadcasting a full replacement snapshot to authenticated in-world connections in that copy. The successful encounter hook sends after the mask is saved. Duplicate DONE is already rejected upstream. Encounter names/flags load from the existing localized DungeonEncounter table. Maximum 32 rows per applicable map/difficulty is validated at startup. Optionality is explicitly unknown: no verified optional Flags bit is assumed. DungeonEncounter also contains continent world bosses (map 0, Nathanos Blightcaller 2416). Snapshot rows must come from an actual InstanceManager copy, not from a map lookup with an invented difficulty 0; otherwise leaving Stockade incorrectly displays that world boss. The transfer fixture reproduces this with real DB2 data.

Newly earned character IDs acquire `earned_at` Unix seconds from ServerClock at the gameplay boundary and persist with counters. Old saved JSON lacks dates; serde defaults the map empty, preserving unknown dates rather than inventing history. Catalog evaluation remains pure; only the world event boundary reads the clock.

Phase-2 RED: locked detached local helper compiled the new Stockade/continuation tests against missing messages/page method and failed with unresolved imports and missing `page`, exit 101. Python import RED lacked achievement_category; GREEN returned the concrete Stockade category 14808 (Classic, parent 168), icon 134163, 10 points and 243 categories. The existing missing encrypted records still produce explicit decoder warnings; none affects these fixtures. Rust GREEN pending.

## Sources

- `scripts/import_achievements.py`, `scripts/db2_casc.py` — local extraction/export/import.
- `crates/server/src/achievement_catalog.rs`, `achievement_window.rs`, `achievement.rs`, `dungeon_progress.rs` — catalog, bounded presentation and gameplay delivery.
- [Client hookup](../../dungeon-achievements-client.md) — protocol fields, bounds and Retail sources.
- `crates/server/src/maps/store.rs`, `instances.rs`, `locks.rs` — encounter ordering, masks and lock persistence.
- `crates/server/src/scripted_ai/tests/dungeonach.rs` — real Stockade death/message proof.

## See Also

- [[boss-scripts]] — encounter state transitions.
- [[persistence]] — character and instance storage.
- [[zone-architecture]] — map copies and lock lifecycle.
