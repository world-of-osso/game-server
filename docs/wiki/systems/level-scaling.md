# Level Scaling

Tuned creatures (`ContentTuningID` != 0, 4,563 of the 4,824 templates spawned on map 0) have one level range. Each unit fights them at its own level, clamped to that range, with health and damage from ExpectedStat. The rules follow TrinityCore master a352b1fa. The contract is in [level-scaling](../../specs/level-scaling.md).

## Content

### Components

- `shared::level_scaling::LevelScaling` (replicated) holds the ContentTuningID, the range and the per-spawn delta. `level_for_target(L)` is `clamp(L, min, max) + delta`. `native_level()` is `max + delta`.
- `UnitLevel` of a tuned creature is its native level, so the combat log, `/who`-style reads and untuned code see the top of the range.
- `ScaledStats(Arc<LevelStats>)` (server-only) holds max health and base DPS per level from 0 to 126 for the template. It is built once per template in `world_data.rs` and shared by every spawn.
- `UnitScaling` (`level_scaling.rs`) combines a unit's `UnitLevel`, `LevelScaling` and `ScaledStats`. `damage_dealt` and `health_loss` take an attacker and a victim.

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

The reported client display/combat block remains unproven at the runtime boundary. Client source already converts target levels and health in `godot/rust/src/targeting.rs` (introduced by client commit `ec4a6eff`). Changing server native stats to a fixed starter range would contradict the scaling contract and higher-level viewers. No shared server or client runtime was touched. Real-data `creaturelevel_*` tests in `zones_world_db_tests.rs` cover spawning, player-relative levels, formula health and lethal native-pool damage; they require `--ignored`.

### Client

`network_runtime/replication.rs` mirrors `LevelScaling`. The target frame (`unit_frames.rs` `build_target_state`) shows `level_for_viewer(UnitLevel, LevelScaling, own level)` and colours an attackable target by `LevelDifficulty`.

## Sources

- TrinityCore `Creature.cpp` (SelectLevel, UpdateLevelDependantStats, ApplyLevelScaling, GetLevelForTarget, Get*MultiplierForTarget), `Unit.cpp` (DealDamageMods, DealDamage), `DB2Stores.cpp` (EvaluateExpectedStat, GetContentTuningData), `StatSystem.cpp` (CalculateMinMaxDamage), `Formulas.h`, `KillRewarder.cpp`
- FrameXML `Blizzard_UnitFrame/Mainline/TargetFrame.lua` CheckLevel, `Blizzard_UIParent/Mainline/UIParent.lua` GetRelativeDifficultyColor

## See Also

- [[world-data]] — ExpectedStat tables and tuned template columns
- [[combat]] — swing pipeline
- [[player-damage]] — player swings and creature armor, applied before these multipliers
- [[experience]] — kill XP at the scaled level
- [[creature-aggro]] — aggro radius at the scaled level
- [[content-import]] — ContentTuning import
