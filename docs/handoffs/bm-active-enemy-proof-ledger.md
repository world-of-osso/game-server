# BM dungeon active-enemy admission

## Contract and representation

Parent2026-10-24: separate generic active-enemy admission counts alive engaged hostile enemies in combat with actor's group; it is not a geometric in-area count. Generator derives minima from pinned APL, not a Wild Thrash threshold branch. Actual ST must never cast1264359;2+ engaged enemies retain real command/pet damage. Dispersion remains outside BM scope ([suspected division only](bm-native-periodic-proof-ledger.md#separate-suspected-division--deliberately-unmodified)).

`Conditions.min_active_enemies` is a JSON condition field, not a new actor type. `kit_ai::conditions_met` compares the existing `ai::engaged_enemies` slice length before other eligibility checks. That producer already filters current same-map alive hostiles using group/pet combat references, owner victim, threat and current victim; no historical PullHistory count, selected-target-only count or radius estimator. Existing `min_enemies` still belongs to `kit_area` geometry and is unchanged.

`scripts/follower_spec_kits.py` conservatively projects lower bounds from pinned literal BM APL: AND=max, OR=min, called-list guards combine with action guards; minimum across every reachable use wins. Unknown non-count runtime predicates are unrestricted. Unsupported count arithmetic/indirect variables, missing/recursive lists fail explicitly. All applicable BM actions are examined; unrestricted KC/Barbed/Cobra remain unrestricted. The output currently yields only Wild Thrash2, derived from source rather than a spell-ID/name rule. No rollout to unrequested specs.

Pinned source copied unchanged from SimC76f8118 `ActionPriorityLists/default/hunter_beast_mastery.simc`; SHA256 `5a7805fd4e99e1ee14d98839a3cb4dea966d649cc00fd9184a615d518685e9f2`. `st` requires `active_enemies>1`; unguarded cleave uses inherit called-list minimum2. Source revision/path/SHA and derived map accompany preparation receipts. This imports policy predicates, never spell coefficients or a replacement rotation.

## Behavioral proof

Diagnostics root: `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/`.

| Revision | Boundary | Result |
|---|---|---|
|b98856d7e|Initial native fixture|Three stop at incorrect NPC Player-KnownSpells assumption; NOT gameplay RED. No grants or overrides added.|
|4da3b88d0|Actual Austin ordinary ST/2/3 engaged combat|Genuine1FAIL/2PASS23.01s: ST29commands/21pet hits versus0;2targets25/9,3targets29/24 controlsPASS. Cobra positive avoids idle false pass. Compile25.67s.|
|9ebfc4d5f|Serialized real native BM kit output|Producer RED: missing minimum (`None !=2`),0.267s. Independent from gameplay RED.|
|a451878e4|Generic source projection9 + native BM serialized kit3|12GREEN1.270s; includes boolean/call reachability, unrestricted alternate uses, no geometric condition, generic unrelated action names, explicit unsupported cases.|
|a451878e4|Native runtime against normally prepared private input|3GREEN24.63s; clean16m12s compilation (all compiler JSON messages empty). ST0commands/0child/Cobra88;2engaged25/9/Cobra38,3engaged17/4/Cobra59.|

Each native phase retains compiler JSON, exact selected artifact/SHA, logs and command/environment entries in `proof-ledger.jsonl`; no filename-only source inference. Only targeted development tests run here; broad integration/publication remains parent-owned.

## Current source export and private preparation

Parent provided already-built verify2 server ELF `server-c413945aac3e58f5`, SHA256 `76bbefa5b9a132c54700e93dd774194a8169265373ec878c07e9f212b4185b74`. Provenance: `gate-hs1063-srv.log.head` revision23151172a, log18–19 compiles verify2 and227 runs that exact artifact. No rebuild.

Canonical runtime215d75ec was opened read-only with `sqlite3 .backup` into new `active-enemy-source-export/canonical.backup.db`, SHA256 `cd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6`. Exporter uses compile-time startup path, so a private bubblewrap read-only source view maps only that alias to the backup. Under `agent-run dps-parity-fix nice -n10`, exactly `follower_kit_export_native_source` passed16.20s:990loadouts/10trees, export SHA256 `68d7cadc76494e1e42e796db567f77cf0f039f4a3b9b5ebcfbaf96a41b372e67`. Original exported alias is retained; receipt records actual backup bytes, no JSON provenance relabeling.

Normal copy-only `follower_spec_kits.py --spec-ids253` runs under the same read-only view.107BM intervals regenerate; `active-enemy-inputs/kit.sql` changes exactly one row:1264359/minlevel23 gains `min_active_enemies:2`. All other rows match. New private world SHA256 `f10a75e62c4f252c26d8e8443031a865319efaf1688dae5a069a2a34c456d40a`; canonical and old frozen world remain unchanged. Export receipt, producer receipt, exact commands and input manifest are retained separately. Publication not performed.

## Matched bounded cadence and DPS

Qualified preserved a451 ELF SHA256 `b1fcc8d419d43c7cb07100f1bebd1e091ece6e57a7d0e5c550a69b385d0861d7`; worldf10a75e6 above. The9bb baseline uses prior qualified reports, not a rerun. Same profile/traits, seed70448/300s/Austin driver; changed inputs are generic runtime admission plus exactly one normally generated kit field. Diagnostics `bm-active-enemy-comparison/comparison.json` retains report SHA/paths and complete actions/Focus receipts; `after/` retains manifest, commands, ELF/source proof and reports.

| Nominal targets | Total DPS before→after | Thrash owner casts/pet hits before→after | Barbed/Cobra/KC casts before→after |
|---|---|---|---|
|1|53884.833→58222.937|29/21→0/0|42/59/63→46/88/68|
|3|44344.413→43712.837|29/24→17/4|38/45/59→40/59/60|

BW remains10casts in both. ST Barbed DPS26595.920→28400.133, Cobra10092.183→12578.417. Three-target total falls despite more Barbed/Cobra casts: do not tune around this one seed or claim universal improvement. Admission counts actual current group combat, not nominal target_count. Sparse existing3T snapshots show all three alive/unsuppressed-owner references at start/early/270s in both runs, and the fixture re-engages each30s wave. They do not record per-decision counts: fewer engaged enemies is NOT established as the cause of29→17 casts or the total drop. Timing/proc/count attribution remains unknown; no tuning or compensating change is justified. This qualifies sourced admission behavior, not every3T lifecycle or whole-DPS parity.

## Limits

No spell/priority/range/radius/position/clock/resource/stat/trait/chance/damage tuning. Native source output/rank accounting comes from normal starter validator, not rewritten old source exports. Geometry remains independent; no fake radius is assigned to the dummy command root. Current native positives qualify2/3 engaged enemies, not every dungeon pull/CC/ownership variant. Matched cadence/DPS is a bounded single-seed contrast, not whole-DPS certification. Parent broad integration and canonical publication remain pending.
