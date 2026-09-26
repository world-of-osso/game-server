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

### Client

`network_runtime/replication.rs` mirrors `LevelScaling`. The target frame (`unit_frames.rs` `build_target_state`) shows `level_for_viewer(UnitLevel, LevelScaling, own level)` and colours an attackable target by `LevelDifficulty`.

## Sources

- TrinityCore `Creature.cpp` (SelectLevel, UpdateLevelDependantStats, ApplyLevelScaling, GetLevelForTarget, Get*MultiplierForTarget), `Unit.cpp` (DealDamageMods, DealDamage), `DB2Stores.cpp` (EvaluateExpectedStat, GetContentTuningData), `StatSystem.cpp` (CalculateMinMaxDamage), `Formulas.h`, `KillRewarder.cpp`
- FrameXML `Blizzard_UnitFrame/Mainline/TargetFrame.lua` CheckLevel, `Blizzard_UIParent/Mainline/UIParent.lua` GetRelativeDifficultyColor

## See Also

- [[world-data]] — ExpectedStat tables and tuned template columns
- [[combat]] — swing pipeline
- [[experience]] — kill XP at the scaled level
- [[creature-aggro]] — aggro radius at the scaled level
- [[content-import]] — ContentTuning import
