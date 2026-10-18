# Level Scaling

Tuned creatures (`ContentTuningID` != 0, 4,563 of the 4,824 templates spawned on map 0) have one level range. Each unit fights them at its own level, clamped to that range, with health and damage from ExpectedStat. The rules follow TrinityCore master a352b1fa. The contract is in [level-scaling](../../specs/level-scaling.md).

## Content

### Components

- `shared::level_scaling::LevelScaling` (replicated) holds the ContentTuningID, the range and the per-spawn delta. `level_for_target(L)` is `clamp(L, min, max) + delta`. `native_level()` is `max + delta`.
- `UnitLevel` of a tuned creature is its native level, so the combat log, `/who`-style reads and untuned code see the top of the range.
- `ScaledStats(Arc<LevelStats>)` (server-only) holds max health and base DPS per level from 0 to 126. Explicit template tuning is built once in `world_data.rs` and shared; regional ordinary CT0 stats are built at spawn from the selected tuning and source difficulty modifiers.
- `UnitScaling` (`level_scaling.rs`) combines a unit's `UnitLevel`, `LevelScaling` and `ScaledStats`. `damage_dealt` and `health_loss` take an attacker and a victim.

### Ordinary source context — 2026-10-17

`CreatureSpawnContext` carries the actual map copy, difficulty and terrain cache. `read_spawn_tuning` first honors [authored template source selection](world-data.md#template-source-selection), then explicit template tuning. Ordinary non-critter CT0 uses existing `SummonLevels` map/area ancestry and source deltas; `read_spawn_area` probes actual MCNK data rather than trusting the spawn's zone field. Known zero tuning remains fixed; missing input logs `CREATURE_LEVEL_CONTEXT_GAP`, including actor/spawn/map/copy/difficulty, then returns no entity before any spawn commands. Population fill continues with other resolvable creatures; it does not panic or substitute legacy stats. Temporary/script summons keep their existing separate initialization; no new owner-level policy is introduced.

Continent population waits for `ground::open_ground_map` and its deferred resource publication in PostStartup. Real production-order startup reaches missing-context RED before this dependency and strict GREEN1/1 at1050147f after it; the fixture uses sourced17536 at an actual imported continent position, not a canonical Nazan placement. Seven enabled context/source/startup cases plus affected controls pass54/54 in177.24s at33dcb7b6; 55 existing ignored cases are not acceptance evidence. [Herald boundary and remaining morph gap](../../specs/ramparts-herald.md).

### Gate486 fixture source audit

The controlled Lesser Voidwalker418 placement at WoW(3,0) samples MCNK area279 (Dalaran Crater), whose CT11 range7..30 plus source delta0 yields native30 under TC `ApplyLevelScaling`/`SelectLevel`. Cache20 is not its native level there; criterion21 uses29/30/31 to retain below/equal/above coverage. Lucifron's real map409 spawn20056605 supports9/18; installed MapDifficulty9 supplies CT1004. The lifecycle fixture's legacy3 had no map row and could not establish source context; it now uses actual9. Neither correction changes canonical data. [Requirements/tests](../../specs/level-scaling.md).

### Damage path

1. Swing base. A tuned creature rolls between 1× and 1.5× of `DPS(native) × DamageModifier × 2 s`. Everything else swings 10.
2. Aura done/taken multipliers (unchanged).
3. `damage_dealt` multiplies by `DPS(level vs target) ÷ DPS(native)` when the attacker is tuned. This is the value the combat log, threat and rage use.
4. For spells, absorbs apply before this step, as in TrinityCore.
5. `health_loss` divides by `maxHealth(level vs attacker) ÷ maxHealth(native)` when the victim is tuned. This is what leaves the one shared health pool.

### Measured values (world.db 2026-09-26)

- ExpectedStat -2 CreatureHealth: 78 at level 1, 377.34 at 10, 4,378.775 at 30 and 141,482.015625 at 90 (exact float32). It stays there from 90 to 133.
- Legion (6) has two rows at level 73 (566 and 569); 569 is used.
- Max health:
  - Stormwind City Guard (CT 883, HealthModifier 1.5): 212,224.
  - Hogger (CT 73, 3.0): 13,137.
  - Diseased Young Wolf (1.0): 4,379.
  - Training Dummy (CT 181, 25) and PvP Training Dummy (CT 864, 8), both level 90: 3,537,051 and 1,131,857.
- Hogger's swing: 553–829 against level 30+ players and 58–87 against a level 10 player.
- Live, Riverpaw Outrunner (CT 73) hit a level 10 player for 35–49 and Hogger hit a level 40 player for 553–823 ([spec](../../specs/level-scaling.md)).
- Only one ContentTuningXExpected row applies to a map 0 tuning, and no map 0 tuning uses Mythic+ season fields.

### Northshire native-pool audit (2026-10-05)

At server revision `c8cd38f`, Blackrock Spy (49874), Blackrock Worg (49871) and Diseased Young Wolf (299) use ContentTuning 73 (1–30), zero delta, warrior class, HealthScalingExpansion 0 and HealthModifier 1. The imported source difficulty rows agree. Their native level 30 and 4,379 health are intentional; against a level-1 player the ExpectedStat formula gives level 1 and 78 health. `npc-info` has no viewer argument and reports `UnitLevel`/`Health` directly (`maps/difficulty.rs`), not effective player-relative stats. These native values alone do not establish a combat-scaling failure.

The same distinction applies to Elwynn Stonetusk Boar (113), Mangy Wolf (525), and Durotar Elder Mottled Boar (3100), Vile Familiar (3101), Felstalker (3102), Razormane Quilboar (3111; ContentTuning 70, also 1–30). Current local data has 411 templates / 3,670 continent spawns using tuning 70 or 73; this is not a count of broken creatures. Flesh Eater (3, tuning 13, 10–30) must remain level 10 / 378 effective health for a level-1 player.

The reported client display/combat block remains unproven at the runtime boundary. Client source already converts target levels and health in `godot/rust/src/targeting.rs` (introduced by client commit `ec4a6eff`). Changing server native stats to a fixed starter range would contradict the scaling contract and higher-level viewers. No shared server or client runtime was touched. Real-data `creaturelevel_*` tests in `zones_world_db_tests.rs` cover spawning, player-relative levels, formula health and lethal native-pool damage; they require `--ignored`. All three passed at `2f35e0d` (7.11 s test execution, no warnings). This proves server spawn/scaling math, not live client display or the complete attack pipeline.

### Client

`network_runtime/replication.rs` mirrors `LevelScaling`. The target frame (`unit_frames.rs` `build_target_state`) shows `level_for_viewer(UnitLevel, LevelScaling, own level)` and colours an attackable target by `LevelDifficulty`.

## Sources

- [Real-spawn regressions](../../../crates/server/src/zones_world_db_tests.rs), [spawn stats](../../../crates/server/src/zones.rs), [scaling math](../../../crates/server/src/level_scaling.rs), [native admin output](../../../crates/server/src/maps/difficulty.rs)
- Original task evidence: `game-engine/data/diagnostics/playaudit-2026-10-05/admin-setup.log`, `kill-scene.log`, `kill-blocked.webp`; none alone proves player-relative combat math.
- TrinityCore `Creature.cpp` (SelectLevel, UpdateLevelDependantStats, ApplyLevelScaling, GetLevelForTarget, Get*MultiplierForTarget), `Unit.cpp` (DealDamageMods, DealDamage), `DB2Stores.cpp` (EvaluateExpectedStat, GetContentTuningData), `StatSystem.cpp` (CalculateMinMaxDamage), `Formulas.h`, `KillRewarder.cpp`
- FrameXML `Blizzard_UnitFrame/Mainline/TargetFrame.lua` CheckLevel, `Blizzard_UIParent/Mainline/UIParent.lua` GetRelativeDifficultyColor

## See Also

- [[world-data]] — ExpectedStat tables and tuned template columns
- [[combat]] — swing pipeline
- [[player-damage]] — player swings and creature armor, applied before these multipliers
- [[experience]] — kill XP at the scaled level
- [[creature-aggro]] — aggro radius at the scaled level
- [[content-import]] — ContentTuning import
