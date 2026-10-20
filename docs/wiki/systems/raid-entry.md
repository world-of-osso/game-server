# Raid entry and instance saves

Verified: 2026-10-11. Raid admission and selection extend the existing map/copy and lock manager; no second raid lock store. The [contract](../../specs/raid-entry.md) owns requirements and remaining source gaps.

## Content

### Admission and difficulty

`InstanceEntrant` carries the group raid flag and separate primary/legacy selections. Current-expansion11 raids require a raid group; older raids permit solo entry, following TC a352b1fa `Map::PlayerCannotEnter`. The map's default Difficulty legacy flag chooses which selection is downscaled through existing DB2 rules. `RaidDifficulty` persists in redb; group creation/join adopts leader choices. Only the leader changes grouped difficulty, outside instances. Map.Flags_0 FlexibleRaidLocking is distinct from MapDifficulty loot-based locks: flex admission rejects encounters already completed by the entrant but not by the destination, rather than binding to one instance ID.

### Authentic death credit

`scripts/raid_encounters.py` reads local CASC JournalEncounter1240336/layoutB88D11CC and JournalEncounterCreature1301155/layout81A3C769; `--journal-build` records the extraction build in the report. DungeonEncounterID supplies map/bit. Exactly one journal creature must join one TDB creature on that map by display ID. Shared displays, multiple actors, absent joins and creatures linked to multiple encounters remain unbound; names never create bindings. The10-11 publication was raid-only (106 bindings) even after the preparer was widened to5-player dungeons; see [binding publication](#binding-publication--2026-10-20). Journal rows without a dungeon/raid DungeonEncounter are named gaps. Journal membership alone does not supply council or non-death completion rules.

Generated SQL replaces only `raid_encounter_creature` in a reviewable backup copy. The preparer never writes a database. Lead prepares a population-imported copy, generates SQL with `--db COPY --journal EXTRACTED/1240336.db2 --creatures EXTRACTED/1301155.db2 --journal-build BUILD --output raid-reference.sql --report raid-reference.json`, applies SQL to the copy, re-applies `encounter_cpp_bindings.py` profiles absent from the journal join (the SQL replaces the table), reviews gaps and publishes separately. Canonical world.db was not changed.

The real combat death boundary records mapped boss bits through `InstanceManager::update_instance_lock`; all characters in that copy save, other copies do not. Permanent/shared data use existing redb persistence. Mapped saved bosses are omitted on reload. Reset expires exactly Tuesday15:00 UTC; an extended owning lock postpones copy expiry. Unextension does not erase an active Retail bind.

### Binding publication — 2026-10-20

`RAMPARTS_BLOOD_FURNACE_BOSS_BINDINGS_UNPUBLISHED`: canonical held only the raid-only10-11 output plus three cpp profiles (121 rows; 5 of 144 five-player maps bound). Regenerated from12.1.5.70448 local-CASC journal binaries (`data/diagnostics/boss-bindings-2026-10-20/` in game-engine): 390 journal rows plus kystia/slagmaw/mutanus profiles = 393 rows. All121 prior rows are retained identically; 272 rows add 84 five-player maps, including Ramparts1891–1893 and Blood Furnace1922–1924. 57 five-player maps stay underivable (multi-actor/shared display). Encounter fixtures no longer insert542/543 bindings; Ramparts lifecycle tests load them from world data. Publication is the lead's.

### Bounded proof

Private UDP5760, two fb_* accounts, Modern/Forever: native menu/group/portal/Raid Info operations; controlled Unit::DealDamage deaths of authentic Lucifron and Magmadar saved both players in copy1 (mask1→3). Private restart retained saves and omitted Lucifron. Simulated October13,2026 15:00:02 UTC reset expired the unextended player, retained extension, allowed native reactivation, and unextension admitted a fresh copy2 with Lucifron alive. Normal spell requests did not reduce boss HP and are not combat-parity proof; existing level1 NPC scaling is not Retail raid numeric parity. Evidence: canonical engine `data/diagnostics/raid-entry-2026-10-11/`;10 inspected PNGs in AgentShared.

## Sources

- [Raid contract](../../specs/raid-entry.md)
- [Selections/death credit](../../../crates/server/src/maps/raid.rs)
- [Map copies](../../../crates/server/src/maps/instances.rs) and [locks](../../../crates/server/src/maps/locks.rs)
- [Copy-only reference preparer](../../../scripts/raid_encounters.py)
- Local pinned TC source/reference snapshots and journal DB2s retained in the evidence directory.

## See Also

- [[content-import]] — instance population prerequisite; branches dungeon-population17374e7 and dungeon-loot87d51eb.
- [[quest-share-resets]] — same Tuesday15:00 UTC weekly calendar.
- [[site-snapshots]] — separate future first-kill/guild-credit history; never infer lifetime progress from reset locks.
