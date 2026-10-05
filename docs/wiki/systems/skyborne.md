# Skyborne server support

Verified 2026-10-04. WoW Forever `wow_classic_beta` build `1.60.1.70205` supplies Skyborne character metadata; Retail remains the base world database. [Source snapshots](../../../scripts/reference/forever-1.60.1.70205/README.md) document the faction/map DB2 provenance; client `data/forever-1.60.1.70205/metadata/Skyborn-graph.json` records ChrRaces/CharBaseInfo.

## Race identity and creation

| Race | Faction | FactionTemplate | Race bit | Classes | Start level |
|---|---|---|---|---|---|
| 95 High Order Skyborne | Alliance (1) | 3629 | 32 | 1,3,4,8,11 | 1 |
| 96 Windshaper Skyborne | Horde (2) | 3628 | 33 | 1,3,4,7,11 | 1 |

`race_class.rs` validation and bit lookup cover both races. `faction.rs` supplies faction identity, group and template to login and existing faction consumers. ChrRaces.Flags does not set IsAlliedRace, so neither race enters the allied level-10 path.

`PlayerCreateInfos` initializes every supported pair before applying race-mask loadouts and authored action rows. No separate race seed is required: runtime has no `chr_races` loader. Real Retail kit data allowed all ten Forever pairs to create, persist across redb close/reopen, and enter through the existing SelectCharacter host-client message fixture. That fixture proves entry, not Forever racial spells, equipment or action-bar parity. Default skills/spells use Retail tables and race masks; no Forever racial/language/start-spell rows were imported or guessed.

## Faction and map import

`db2.py import` decodes the pinned Forever sources before opening its write transaction, then reapplies the three selected rows after Retail import in that same transaction. Full regeneration therefore retains the overlay. `db2_row_source` records row-level build, FDID, layout and SHA-256 without relabeling the whole Retail table. Standalone scratch publication: `python3 scripts/skyborne_data.py --db <scratch-world.db>`.

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

[[content-import#Forever Zephras overlay]] owns NPC/quest import counts and limits. Native quest messages prove a giver within 5 yd, Coming of Age acceptance and captured 40 XP at Rorian's turn-in; race 96 is rejected by its source race mask. Skysight rejects level 1, accepts level 2, and remains incomplete because its racial event is unimplemented. No scripted events, phasing or racial spell parity is claimed. Both starting givers' displays remain unresolved; server spawn-query proof is not rendered-client proof.

## Sources

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
