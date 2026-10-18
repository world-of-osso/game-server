# Skyborne server support

Verified 2026-10-04. WoW Forever `wow_classic_beta` build `1.60.1.70205` supplies Skyborne character metadata; Retail remains the base world database. [Source snapshots](../../../scripts/reference/forever-1.60.1.70205/README.md) document the faction/map DB2 provenance; client `data/forever-1.60.1.70205/metadata/Skyborn-graph.json` records ChrRaces/CharBaseInfo.

## Race identity and creation

| Race | Faction | FactionTemplate | Race bit | Classes | Start level |
|---|---|---|---|---|---|
| 95 High Order Skyborne | Alliance (1) | 3629 | 32 | 1,3,4,8,11 | 1 |
| 96 Windshaper Skyborne | Horde (2) | 3628 | 33 | 1,3,4,7,11 | 1 |

`race_class.rs` validation and bit lookup cover both races. `faction.rs` supplies faction identity, group and template to login and existing faction consumers. ChrRaces.Flags does not set IsAlliedRace, so neither race enters the allied level-10 path.

`PlayerCreateInfos` initializes every supported pair before applying race-mask loadouts and authored action rows. No separate race seed is required; [installed Retail eligibility](race-class-eligibility.md) preserves these separately authored Forever gates instead of borrowing Retail NPC metadata. Historical Retail-kit entry fixtures did not prove authored kit coverage. Historical full-workspace evidence at `6878f6a` reported empty kits for nine Skyborne pairs and six leaked Retail items for race 95/class 1. The corrected source-isolated ten-pair creation/reopen test passes at `e405232`; that does not relabel the earlier broad suite as passing or establish native E2E acceptance. That fixture proves entry, not Forever racial spells, equipment or action-bar parity. Default skills/spells use Retail tables and race masks; no Forever racial/language/start-spell rows were imported or guessed.

## Authored starting kits

Verified 2026-10-05: pinned local sources decode six loadouts, 45 memberships and all 30 referenced Item/ItemSparse rows. [Source snapshots](../../../scripts/reference/forever-1.60.1.70205/README.md) own exact layouts, hashes and decoder differences. [Contract](../../specs/skyborne-races.md) requires no Retail kit leakage. Required appearance/effect parents resolve from selected clear rows; unrelated encrypted membership completeness remains unknown.

Shared Item IDs materially differ between builds: 2947 changes junk to throwing weapon with buy/stack count 200→1, 2512 changes junk to ammunition, 2101 changes bag to quiver, and five weapon IDs change binding. Strict publication rejects collisions and rolls back, preserving Retail rows. User selected Retail preservation with isolated Forever definitions. Six `skyborne_item*` tables now hold selected source definitions; `skyborne_content_item` reuses the existing content-policy mapping and regeneration hook. Its provenance points to the exact ItemSparse row; related raw Item/appearance/effect tables retain their own exact source records. Creation/count loading uses only Forever-provenance loadouts for races 95/96 and only Retail loadouts for other races; shared masks are unchanged. Retail item overrides do not apply to those two races. `PlayerCreateInfos` owns 30 source-local template/goods rows for new-character creation, while ordinary creation still receives the Retail resources. Required content misses and empty authored kits fail explicitly. The importer-generated ten-pair fixture covers genuine gear/counts, GUID source markers and redb reopen at `e405232`. `NewCharacterKit::new_item` returns the selected item, template and source; inventory creation records that source rather than later inferring it from race. Subsequent ownership/reload uses immutable GUID identity. Source-qualified runtime catalogs, validation, transfers and equipped scaling are implemented; [[inventory#Definition source]] owns their mechanism and bounded proof, and [[persistence#Character customization selections]] owns the frozen disk ABI. Final integration and native E2E acceptance remain open. No implicit replacement, Retail substitution or invented item IDs. Standalone publication starts `BEGIN IMMEDIATE` before the overlay savepoint, so projection/schema failure rolls back factions, loadouts, isolated definitions and provenance together. Without that outer transaction, releasing the savepoint committed the overlay before projection; a process fixture reproduces the boundary using an incompatible pre-existing view. The additive `add_player_create_info.py` refresh also preloads and reapplies the pinned overlay after replacing Retail kit tables; it no longer erases the 45 authored memberships.

## Source-local item scaling

`python3 scripts/skyborne_scaling_data.py --catalog <Forever-items-dir> --db <existing-owned-scratch-world.db>` publishes exactly nine isolated scaling tables. Names: `skyborne_item_armor_quality`, `skyborne_item_armor_total`, `skyborne_item_armor_shield`, `skyborne_armor_location`, `skyborne_item_damage_one_hand`, `skyborne_item_damage_one_hand_caster`, `skyborne_item_damage_two_hand`, `skyborne_item_damage_two_hand_caster`, `skyborne_rand_prop_points`. Level-keyed exports retain `ItemLevel` as primary key; quality/location/budget exports retain `ID`. Numeric columns reuse Retail schema names/types only where present in the authored CSV. No `ArmorLocation.Modifier`, RandProp integer arrays or damage placeholders are fabricated. `load_forever_item_scaling_tables` loads this prefixed namespace into `ForeverItemScalingTables` at startup, without Retail's squish curve. Equipped stats select the resource by owned source; scalar formulas remain unchanged. This is source isolation, not full Classic damage/stat parity.

[Maintained pins](../../../scripts/reference/forever-scaling-70205.json) record source FDID, table/layout hashes, raw/schema SHA-256 and exact headers from the local build `1.60.1.70205` catalog. Import checks those against the manifest and each CSV hash/count/IDs before its transaction. Levels 1/2 and manifest-authored kit levels (currently 1/2/3/5) must exist in every level table; existing `skyborne_item_sparse.ItemLevel` and repeatable `--required-level N` add consumer requirements. Every imported row receives `db2_row_source` with its exact raw-source hash, not the CSV hash. The importer requires an existing destination, replaces only its nine tables' rows/provenance atomically, and performs no canonical/live provisioning. Missing/unreadable source fails explicitly. Retail `db2.py` regeneration and `skyborne_data.py` refresh do not drop these prefixed tables or their provenance, so no extra refresh hook is needed. Rerun the standalone CLI when refreshing this catalog.

[Behavioral fixtures](../../../scripts/tests/fixtures/forever-scaling-70205/manifest.json) contain the exact nine exported CSVs, not invented curves. [Targeted tests](../../../scripts/tests/test_skyborne_scaling_data.py) cover concrete float preservation, Retail SQL/blob/provenance preservation, deterministic refresh and failure rollback; runtime/native acceptance remains separate.

## Faction and map import

`db2.py import` decodes the pinned Forever sources before opening its write transaction, then reapplies the three selected rows after Retail import in that same transaction. Full regeneration therefore retains the overlay. `db2_row_source` records row-level build, FDID, layout and SHA-256 without relabeling the whole Retail table. Standalone scratch publication: `python3 scripts/skyborne_data.py --source-dir "$FOREVER_DB2_SOURCE" --db <scratch-world.db>`. Raw DB2s live only in the untracked engine `data/db2/forever-1.60.1.70205/` cache; Python tests and the full importer require `FOREVER_DB2_SOURCE` pointing there. The [reference README](../../../scripts/reference/forever-1.60.1.70205/README.md#local-data-paths) owns exact paths, fixture environment and explicit-source CLI usage; decoded text/provenance stay tracked.

FactionTemplate 3628 references Faction 2794, flags 72 and groups 5/4/10; 3629 references 2795, flags 72 and groups 3/2/12. Values come from the decoder, not authored constants. The reaction loader reads only `faction_template`, treating Faction IDs as identifiers; it does not load a Faction table. Referenced Faction rows are therefore unnecessary for creation/login and were not imported. Scratch-imported rows passed Rust friendly/hostile/assist/PvP-gate checks. The maintained Rust reaction test uses [generated SQL](../../../crates/server/src/skyborne_faction_fixture.rs) from those exact decoded rows in an in-memory SQLite database, so the original Retail world.db need not be regenerated to run it.

## Map 2991 reconstructed starts

The imported Map row registers Zephras Isle, directory `2991`, InstanceType 0, WdtFileDataID 7198644. `maps/store.rs` loads it as a continent; `ground.rs::open_ground_map` derives its WDT mapping directly from MapStore. Ground loading needs no hardcoded WDT registry; hosting still requires the shared continent list below.

`start_location` now uses the user-approved reconstruction ([contract](../../specs/skyborne-races.md)). Race 95 starts at (4088.501,1847.5,976.169), 3 yd east of Coming of Age giver Ailee Farheart 251362. Race 96 starts at (4051.417,1858.625,977.620), **inferred** near Ventaari Brightwish 251487: no captured race-96 level-1 intro exists; her Gift of Skysight 92598 requires level 2.

`CharacterData` holds coordinates; `create_character_with_appearance` already persists `start_location.map_id` separately in `character_maps`. No serialization change or existing-character relocation is needed. Login reads that map. `HOSTED_CONTINENTS` includes 2991: map registration alone previously failed the instance manager's hosting gate. The shared host list enables login, transfers and startup content spawning.

Facing follows `atan2(giver Y−start Y,giver X−start X)` = π/2 for both starts, converted to native replicated yaw π. Persistence has no facing field; this rule applies only at the exact selected start. Other saved positions retain prior login-facing behavior.

Ground/asset requirements remain:
1. Cache WDT 7198644 as `terrain/7198644.wdt` and extract its MAID root/object tiles plus referenced WMO/GFID groups from the Forever CASC build. The existing probe `game-engine/data/forever-1.60.1.70205/skyborne-probe/7198644.dat` is a readable WDT (294,988 bytes, MPHD 970, 72 root tiles). Initial root/object FDIDs include 7199799/7199800. This proves WDT availability, not complete ADT/WMO availability.
2. Provision the explicit registered map with `ground_files.py`/`extract_ground.py --map 2991`; Map-row build provenance selects Forever local CASC. No invented creature spawns are needed. [[ground-assets]] records the extracted closure, remaining local-asset gaps and parser corrections.
3. Bake required LOS tiles with `los-bake --data <cache> --world-db <scratch-world.db> --out <cache>/los --map 2991 --all` (or a bounded authored-start area). The baker reads Map.WdtFileDataID. [[ground-assets]] records the partial available-tile bake and runtime loading/hit proof; full-map closure remains blocked.

Both approved starts use available root 7199904 (tile row 28, column 24), with native `ground_at` support within 0.01 yd of selected Z. Real-kit creation/reopen/SelectCharacter proves all ten pairs persist map 2991, select there and send `LoadTerrain` directory `2991`, tile (28,24). Northshire/Kalimdor entry regressions are retained. Proof ledger: checkout `target/zephras-content-proof/ledger.md`.

[[content-import#Forever Zephras overlay]] owns NPC/quest import counts and limits. Native quest messages prove a giver within 5 yd, Coming of Age acceptance and captured 40 XP at Rorian's turn-in; race 96 is rejected by its source race mask. Skysight rejects level 1, accepts level 2, and remains incomplete because its racial event is unimplemented. No scripted events, phasing or racial spell parity is claimed. Refreshed captured NPC pages resolve Ailee→136968 and Ventaari→139694; native fixtures now load those model rows and spawn named giver entities carrying ModelDisplay. Historical fixture spawn/message proof is not production startup or rendered-client proof; the historical startup blocker and current estimated-stat proof below distinguish those fixtures from rendered-client acceptance.

## Native entry and NPC startup source gap (2026-10-05)

Main reported actual native world entry for both races on map 2991 at the approved starts using server `e405232`; this establishes entry only, not NPC/giver/quest acceptance. Startup had zero NPCs because `NPC_APPEARANCE_MAPS` excluded 2991, independently of `HOSTED_CONTINENTS`.

The minimal gate change `2af05c1` retained a real imported-row regression expecting 1,662 spawns. Its attempted GREEN reached `zones.rs:625`: template 249363, level 5, class 0 had no class-level-stat row. Local captured sources do not establish authored class/health for 249363, Ailee 251362 or Ventaari 251487. Appearance mappings and quest text cannot supply those stats.

Historical user choice kept acceptance blocked: `34e23d5` restored the appearance allowlist `[0,1,34,670]` and explicitly ignored the startup regression. Ignored was not passing. Later explicit approval of reasonable health/unit-class estimates supersedes that hold, not the missing authored-stat evidence. [Content import](content-import.md#forever-zephras-overlay) owns the estimate policy and current workspace/startup proof: map2991 is now included and the actual1662-spawn regression runs. [Engine native matrix](../../../../game-engine-skyborn/docs/wiki/systems/forever-data.md#current-scoped-capability-matrix) owns bounded race95 native92460 acceptance and race96 visible Ventaari evidence, including Ailee's withheld visual and pending independent artifact gate. No authored/full Classic, all-NPC visual or complete quest-gameplay claim.

## Rebased source and runtime checkpoint (2026-10-07)

Main reports all 90 commits replayed without skips onto `8b3819b0`, clean source `bef3fa74577e6810e72bdbace1d7baeda1afa910`, pushed as `origin/skyborn`. Backup ref: `refs/backup/skyborn-pre-rebase-4fac87a243f2`. Cold data/binary backup: `target/skyborn-e2e/backups/before-master-4fac87a243f2`; engine `target/skyborn-source-items/server-pre-rebase-cold-backup.json` records the exact backup. Character30 retains40XP; never reset.

Current pushed server checkpoint is `708d493`; engine production `041fa0f54` was pushed earlier, while current test rename `f7643e3e9` is not yet pushed. Earlier `bef3fa745` and protocol48-versus510748 rejection are historical, superseded by matching scratch world entry below; historical1448 suite remains revision-scoped, not fresh rebased proof.

| Boundary | Evidence | Current gap |
|---|---|---|
| Loader source applicability | `c1cdf6e` restricts spell budgets to Retail; normal Forever scaling stays mandatory and Forever spell-budget map empty. Matching loader tests GREEN3/3 | No Forever item-spell support, fabricated columns, borrowed budgets or fallback |
| Independent bounded gate | [Ledger](../../../target/final-bounded-server-3eb1181/ledger.md) at `708d493`: fmt/check exit0, Rust8/8, changed Python1/1; loader3/3 reused by matching hashes, unchanged Python18 retained by exact scopes | Not a fresh whole-suite pass; loader fixture-helper readability advisory retained |
| Matching portable runtime | Server SHA256 `c818f4541057feb805756106dd59d00719de1ad6e4d61dca4b31b6077e9daed0` from `3eb1181`, installed owned scratch only; matching extension `62eddda…` authenticated/world entered | Production unchanged; startup initially lacked AreaTable and achievements |
| Native boundary | Character30 retains40XP/log`[]`; seven source-kit queries; Ailee attached12 textured batches then actual fixture FAIL Batch94/geoset3601, texture_count1/base_texture empty | Historical Batch94 RED superseded by current native type20 binding below; strict Ailee now FAILS Batch117 type7. No full Ailee GREEN/pixels or whole-goal claim |

Pinned Forever70205 RandPropPoints has16 columns and lacks Retail-only `DamageReplaceStatF`/`DamageSecondaryF`; producer faithful. The former shared-loader requirement was a consumer defect, now corrected, not missing authored data.

### Scratch prerequisite provenance and limits

Existing supported narrow importers provisioned only owned scratch. [Area receipt](../../../../game-engine-skyborn/target/skyborn-source-items/scratch-area-import.json): local AreaTable CSV9848 rows, SHA256 `8559dee102d82ad2dcfba36360d9d94947baf2880a145d2b69c83ba1d6f3342b`, explicitly stamped `local-csv-sha256:8559dee102d82ad2dcfba36360d9d94947baf2880a145d2b69c83ba1d6f3342b;build-unverified`. Build **UNVERIFIED**, not pinned Retail build proof.

[Achievement receipt](../../../../game-engine-skyborn/target/skyborn-source-items/scratch-achievement-import.json) and [read-only source transfer/hashes](../../../../game-engine-skyborn/target/skyborn-source-items/scratch-achievement-source-transfer.json): four pinned local Desktop DB2s69933, imported Achievement14081/Category243/Criteria66340/CriteriaTree120821. Exactly1 Criteria and8 CriteriaTree encrypted/unavailable rows remain excluded under existing policy; no fabricated rows.

### Current bounded checkpoint: import preservation and native DONE

Main-supplied independent data-import gate278 records **all270 old tables/10,365,014 rows unchanged**, five expected added tables. AreaTable build remains UNVERIFIED;1 Criteria/8 CriteriaTree encrypted/unavailable gaps remain. This is SQLite import preservation, not physical redb byte preservation: redb bytes changed after login, while character30 retains40XP/log`[]`.

[Current engine checkpoint](../../../../game-engine-skyborn/docs/wiki/systems/forever-data.md#corrective-native-checkpoint--observed-functional-done-2026-10-07) owns corrective742 source-hash-guarded build exit0, mapped extension2763a276…/serverc818f454… and pinned protocol510. Strict unchanged `ailee-assets` fixture now reaches functional DONE: original7478494/bake7352105,15 visible textured batches, selected Batch94/97 type20 at256²; source-unselected117 omitted under user-approved policy. Original Grove7579617/18 Left6/Right5 identity meshes retain256²/64² active textures. Ten saved PNGs include front/back and material captures; Main observes correct robe/body and both shoulder pads. Earlier mapped type7/type2 failures are historical, superseded only within this bounded scope.

Current CPU13 passed (seven prior plus six physical-file/policy cases); independent304 source/audit active and scoped fmt/check pending. Earlier omitted image payloads confer no independent pixel proof. Async helper captures DONE, not native OS exit0 or general shutdown. Character30/40XP/log[]/tracker empty; no quest accepted/rewarded. Cleanup receipt records inactive/MainPID0, UDP15002 empty and owned cage/client/server absent; physical redb changed, not unchanged-byte proof. Eleven out-of-coverage NPC errors/unmapped visible capes, full stock/PBR/body/hair parity, provenance and encrypted records remain open. Final-gate completion deferred until304; whole goal OPEN. No operations/builds/tests/data edits/delegation/push in this documentation checkpoint.

## Reported motion fixture correction (2026-10-07)

`player_motion_airborne_walkoff_and_touchdown_replicate_without_jump` failed because its replacement ground was not under the player, not because physics retained stale FALLING. Original corner100 and replacement99 both select terrain X-cell3; player x66.51667 occupies cell2. Instrumented original ordering showed ten Unsupported ticks, y49.02→39.22, motion2049 correctly falling. [Diagnostic ledger](../../../target/reported-skyborn-motion-diagnostic/ledger.md) owns exact RED and coordinate evidence.

Test-only `a7616cd`, subsequently pushed to origin/skyborn per Main, uses adjacent corner `100 - 100/3`. It asserts Supported(48) before gravity, first no-input-tick landing within existing snap tolerance, grounded tracker and motion1; retains ten ticks and final motion assertion. No weakened assertion, wait, production physics, protocol or real-data change.

[GREEN/final ledger](../../../target/reported-skyborn-motion-green-a7616cd/ledger.md) retains exact1/1 PASS; independent302 scoped fmt/check-server-tests exit0, no warnings or own readability findings. Production/protocol hashes unchanged; prior60-case proof remains valid separately, not a fresh combined-suite claim. HostClient input/gravity boundary only: no UDP, rendered animation, real terrain or whole-goal acceptance.

[Engine handoff checkpoint](../../../../game-engine-skyborn/docs/wiki/systems/forever-data.md#desktop-handoff-checkpoint-2026-10-07) owns the staged OssoBuild packet and metadata-refresh boundary. Packet availability is not remote test consumption or landing. Current bounded Ailee/Grove functional DONE supersedes the earlier type2 RED; independent304 remains pending, not full NPC acceptance. Provenance, encrypted records, NPC/PBR/blue-hair limits remain; goal OPEN.

## Sources

- Main-supplied 2026-10-07 final bounded checkpoint — corrective742 native DONE, mapped/build/status/cleanup receipts and pending304; engine checkpoint owns exact artifacts and final handoff metadata hashes. No new tests or remote operations in this docs audit.

- Historical Main-supplied checkpoint (2026-10-07) — independent import278, traced native retry, separate shoulder scope and then-pending Grove291/type7 semantics; corrective742 bounded DONE supersedes that native blocker, not provenance/import scope. [Engine checkpoint](../../../../game-engine-skyborn/docs/wiki/systems/forever-data.md#current-sourceproof-checkpoint-2026-10-07) owns linked current CPU, authentic binding/geometry and cleanup artifacts.

- Main-supplied 2026-10-07 source finding — pinned Forever70205 RandPropPoints schema, faithful producer, shared-loader defect, Retail-only correction `c1cdf6e`, matching3/3, independent bounded gate and matching native material FAIL; prior pending status historical.
- Main-supplied 2026-10-07 checkpoint — rebase, pushed revisions, conflict resolutions, runtime fingerprint diagnosis, cold backup and character preservation; no fresh runtime verification by this documentation audit.
- [Cold backup receipt](../../../../game-engine-skyborn/target/skyborn-source-items/server-pre-rebase-cold-backup.json) — exact pre-rebase data/binary backup target.

- [Startup gate](../../../crates/server/src/maps/instances.rs), [real startup regression](../../../crates/server/src/zones_world_db_tests.rs), [spawn stats](../../../crates/server/src/zones.rs) — startup boundary; historical entry/panic observations and current evidence supplied by Main for this audit.
- [race_class.rs](../../../crates/server/src/race_class.rs), [faction.rs](../../../crates/server/src/faction.rs), [persistence_character.rs](../../../crates/server/src/persistence_character.rs)
- [player_create_info.rs](../../../crates/server/src/player_create_info.rs), [auth_character_tests.rs](../../../crates/server/src/auth_character_tests.rs)
- [skyborne_data.py](../../../scripts/skyborne_data.py), [test_skyborne_data.py](../../../scripts/tests/test_skyborne_data.py)
- [zephras_content.py](../../../scripts/zephras_content.py), [zephras_quest_tests.rs](../../../crates/server/src/zephras_quest_tests.rs) — labelled overlay and native message proof
- [ground_files.py](../../../scripts/ground_files.py), [extract_ground.py](../../../scripts/extract_ground.py), [los-bake.rs](../../../crates/los/src/bin/los-bake.rs)

## See Also

- [[factions]] — reaction rules
- [[class-progression]] — default skills, kits and known spells
- [[content-import]] — Retail base import
- [[ground-assets]] — explicit-map provisioning, real parser proof and remaining closure gaps
- [[inventory]] — immutable GUID provenance, source-qualified runtime lookup and remaining transfer acceptance
- [[persistence]] — frozen character disk ABI and owned-source hydration
