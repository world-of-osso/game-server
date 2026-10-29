# Log-derived follower population

The user's Retail combat logs are an approved source for otherwise absent follower-dungeon populations and boss timing. The logs describe combat events, not a complete authored dungeon population. `LOG_DERIVED_POPULATION` marks additions for later retirement; authored TrinityCore and existing content rows are not replaced. See the [follower dungeon policy](../../specs/follower-dungeons.md#log-derived-population-policy).

## Evidence and limits

`scripts/log_population_census.py` streams the logs. It records Creature GUIDs, NPC IDs, spawn UID and instance, event names and times, explicit summon destinations, and the unit named by an advanced event block. Its first positioned observation may occur after combat begins. The advanced block gives X/Y, UI map ID, facing and level, but no altitude. Resolver requests preserve unit, run, event context and neighboring observations; altitude must come from collision geometry and a defensible floor selection. A missing or ambiguous floor remains a gap.

Encounter extraction retains each pull's observed creature casts and uses `raid_encounter_creature` to identify known boss entries. This table is incomplete, so other encounter units remain unclassified, not automatically adds. Successful-pull summaries retain first-cast and repeat-interval samples and their observed range; timing varies and these samples do not prove an authored schedule. Phase markers are limited to explicit events in the log.

EXBossData's `TrashCDData.lua` provides addon-reported trash ability timings, with the source path and line retained in the output. For example, `TrashMobTraits.lua:1632` identifies NPC 187969 and `Factory.lua:4727` associates it with spell 1305225. Addon classifications and timers do not supply positions.

Cross-run matching is provisional. Within a connected 10-yard area of the same NPC and UI map, the maximum per-run GUID count supplies slot count. Minimum-distance assignment matches at most one GUID per slot per run; an alternative assignment within one yard of total cost is ambiguous. As a user-approved population policy, ambiguous slots whose candidate positions are all within five yards use their candidate centroid only after separately resolving that centroid's floor; the assignment margin and candidate spread are retained. Wider ambiguity remains a gap. Same NPC and a nearby position alone do not prove identity, especially for packs or units that moved after a pull. Player-controlled units and explicit summons are excluded from spawn matching but retained in the census. The patrol investigation uses pre-engagement samples and pack co-observations, but engagement is inferred from observed events and can be missed. A three-second exclusion around observed hostile action or casting is an analysis threshold, not a Retail rule. A path requires at least three distinct runs and navmesh validation; otherwise `PATROL_UNOBSERVED` remains. No unknown patrol, formation, unseen creature or floor is invented.

`scripts/import_log_population.py` requires a database copy and resolved, explicitly classified observations. It checks both `tdb_creature` and `content_creature` before adding rows. It projects missing runtime templates and their available raw TDB difficulty, model and addon data through the existing Retail template projection. It records template and spawn provenance separately. No shared template level is changed to match a log observation.

## Coverage snapshot

Read-only canonical database and log census, 2026-10-28. Counts are raw GUID observations and NPC IDs, not deduplicated spawns; they include summons and unclassified actors. TDB and content counts include all stored difficulties, not only Normal. The [copy-only import results and named gaps](../../specs/follower-dungeons.md#copy-only-coverage-2026-10-28) are recorded separately.

| Dungeon | Map | Runs | GUID observations | NPC IDs | TDB rows | Content rows |
|---|---:|---:|---:|---:|---:|---:|
| Kings' Rest | 1762 | 17 | 10,555 | 96 | 212 | 212 |
| Ruby Life Pools | 2521 | 10 | 6,153 | 66 | 306 | 307 |
| Altar of Fangs | 2993 | 12 | 7,825 | 60 | 0 | 51 |
| Murder Row | 2813 | 12 | 8,818 | 96 | 2 | 131 |
| Den of Nalorakk | 2825 | 13 | 8,877 | 89 | 217 | 217 |
| The Blinding Vale | 2859 | 8 | 3,499 | 73 | 0 | 216 |
| Voidscar Arena | 2923 | 8 | 4,980 | 70 | 0 | 198 |

The current extraction has 266 encounter pulls, including 253 successful pulls. A copy-only run inserted 1,214 source-tagged rows; read-only comparison found no pre-existing TDB or content creature rows missing, and SQLite `quick_check` passed. The sole multi-run pre-engagement movement candidate is Ghostly Brute135231 on map1762; all nine floor results are ambiguous, so no navmesh path was qualified or imported. The coverage snapshot is not a claim that a dungeon is complete.

## Sources

- [Follower dungeon policy](../../specs/follower-dungeons.md) — authorization and retirement boundary.
- `scripts/log_population_census.py`, `scripts/log_population_packs.py`, `scripts/prepare_log_population.py`, `scripts/import_log_population.py` — extraction, candidates, geometry join and copy-only import.
- User's Retail log directories and local `EXBossData` addon — private data and addon observations, not tracked in this repository.
- Canonical `data/world.db` — read-only TDB and content comparison.
