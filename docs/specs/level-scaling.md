# Creature Level Scaling

The creatures are Retail spawns ([retail-spawn-source](retail-spawn-source.md)), and Retail gives a tuned creature a level range rather than a fixed level. A creature is tuned when its `creature_template_difficulty` row has a `ContentTuningID`. Every player sees and fights it at their own level, clamped to that range. For example, Hogger (ContentTuning 73, levels 1-30) is level 10 to a level 10 player and level 30 to a level 40 player. Health and damage come from the ExpectedStat DB2s, as in TrinityCore master a352b1fa. AzerothCore's `creature_classlevelstats` only has real base health up to level 83, which gave a level 90 Stormwind City Guard 2 HP.

## What it must do

Data
- [x] `db2.py extract` decodes ExpectedStat (FDID 1937326), ExpectedStatMod (1969773) and ContentTuningXExpected (2976765) from local CASC into `expected_stat`, `expected_stat_mod` and `content_tuning_x_expected`. The ID comes from the file's ID list (first column) and the parent field from its relationship map (last column).
- [x] A tuned `content_creature_template` row carries its ContentTuning range as `minlevel`/`maxlevel`, plus TrinityCore's difficulty-0 values:
  - `HealthScalingExpansion`;
  - `LevelScalingDeltaMin`/`LevelScalingDeltaMax`;
  - `HealthModifier`, `ArmorModifier` and `DamageModifier`.
  AzerothCore entries that TrinityCore tunes included.

Level
- [x] A tuned creature's own level is its range maximum plus a delta, rolled once per spawn from `LevelScalingDeltaMin..=Max` (Creature::SelectLevel, Creature.cpp:1611-1618; ApplyLevelScaling, :3122-3171).
- [x] Its level against a unit of level L is `clamp(L, min, max) + delta`, clamped to 1..=126 (Creature::GetLevelForTarget, :3226-3257). The rule lives once, in `shared::level_scaling::LevelScaling::level_for_target`, which server and client both call.
- [x] Untuned creatures keep one fixed level (`minlevel`) for everyone.

Health and damage
- [x] Max health is `ceil(ExpectedStat.CreatureHealth(level, expansion) × ContentTuningXExpected mods × class mod × HealthModifier)` at the creature's own level (GetMaxHealthByLevel, :3173-3179; DB2Manager::EvaluateExpectedStat, DB2Stores.cpp:2512-2618).
  - HealthScalingExpansion -1 means Midnight (11).
  - A level without a row for its expansion uses the ExpansionID -2 row.
  - Of two rows for the same (level, expansion), the later ID wins.
  - Examples: a level 90 Stormwind City Guard has 212,223 health, a level 30 Hogger 13,137.
- [x] Health is one pool per creature. Damage the creature takes is divided by (max health at the attacker's level ÷ its own max health) (Unit::DealDamage, Unit.cpp:829-830; GetHealthMultiplierForTarget, Creature.cpp:3181-3189).
  - This covers swings and spells.
  - A level 10 player's 10 damage removes 115 of Hogger's 13,137 health; a level 40 player's removes 10.
- [x] A tuned creature's swing does `ExpectedStat.CreatureAutoAttackDps(own level) × DamageModifier × 2 s`, rolled uniformly between 1× and 1.5× (UpdateLevelDependantStats, Creature.cpp:1650-1656; CalculateMinMaxDamage, StatSystem.cpp:1138-1147).
- [x] Any damage a tuned creature deals, swings and spells alike, is multiplied by (base DPS at the target's level ÷ base DPS at its own level) (Unit::DealDamageMods, Unit.cpp:814-815; GetDamageMultiplierForTarget, Creature.cpp:3198-3206).
- [x] Untuned creatures keep `creature_classlevelstats` health and the flat 10-damage swing.

Per-player rules
- [x] Kill XP uses the creature's level against each player: the killer, and every group member for the group's grey test and the highest non-grey member (Trinity::XP::Gain, Formulas.h:190; KillRewarder, KillRewarder.cpp:110).
- [x] The aggro radius compares the player's level with the creature's level against that player (Creature::GetAttackDistance, Creature.cpp:2175-2177).

Client
- [x] `LevelScaling { content_tuning_id, min_level, max_level, delta }` replicates on tuned creatures. Adding it changes the replicated component registry, so client and server must match.
- [x] The target frame shows the creature's level against the local player.
- [x] An attackable target's level text is coloured by its difficulty for the player (TargetFrameMixin:CheckLevel; GetRelativeDifficultyColor, UIParent.lua:2626-2640):
  - red at +5 levels or more;
  - orange at +3;
  - yellow from -4 to +2;
  - green above the grey level;
  - grey at or below the grey level (Trinity::XP::GetGrayLevel, Formulas.h:70-89).
  Friendly targets stay gold.
- [x] Live: a level 10 and a level 40 character targeting the same Hogger see level 10 (yellow) and 30 (grey). Their swing damage matches ExpectedStat at 10 and 30 (`game-engine/data/diagnostics/levelscaling-20260926/`).

## How it works

- [World data](../wiki/systems/world-data.md) — expected_stat tables, tuned template columns
- [Level scaling](../wiki/systems/level-scaling.md) — components, multipliers, examples

## Implementation inventory

- `scripts/db2.py`, `scripts/db2_casc.py`, `scripts/db2_schema.sql` — DB2 extraction (non-inline IDs, relationship maps)
- `scripts/content_policy.py` `build_templates` — tuned template columns
- `shared-protocol src/level_scaling.rs` — `LevelScaling`, `level_for_viewer`, `gray_level`, `LevelDifficulty`
- `crates/server/src/level_scaling.rs` — `ExpectedStats`, `LevelStats`, `ScaledStats`, `UnitScaling`, `damage_dealt`, `health_loss`
- `crates/server/src/world_data.rs` — `load_expected_stats`, `TemplateTuning`
- `crates/server/src/zones.rs` — `spawn_level_scaling`, spawn level and health
- `crates/server/src/combat.rs` `try_swing`/`swing_damage`, `spell_cast/effects.rs` `deal_damage` — damage scaling
- `crates/server/src/experience.rs` `KillVictim::level_for`, `creature_aggro.rs` `aggro_on_sight_system`
- `game-engine src/rendering/ui/unit_frames.rs` `build_target_state`, `src/network_runtime/replication.rs`

## Tests asserting this spec

- `scripts/tests/test_import_azerothcore.py`:
  - `LocalExpectedStatExtractTests`;
  - `test_retail_spawns_replace_azerothcore_spawns_on_maps_0_and_1` (tuned template columns).
- shared-protocol `level_scaling_tests.rs`: Hogger 10/30, range floor, delta, colours, grey level.
- `crates/server/src/level_scaling_tests.rs`: ExpectedStat evaluation, mods, expansion fallback, damage and health multipliers.
- `combat_tests.rs`:
  - `level_10_player_hit_takes_the_level_10_share_of_a_tuned_creatures_health`;
  - `level_40_player_hit_on_a_tuned_creature_is_unscaled`;
  - `tuned_creature_swings_expected_stat_damage_at_the_targets_level`.
- `experience_tests.rs`: `tuned_kill_gives_xp_for_the_killers_scaled_level`, `group_members_see_a_tuned_kill_at_their_own_level`.
- `creature_aggro_tests.rs`: `tuned_creature_aggro_radius_uses_its_level_against_the_player`.
- `zones_world_db_tests.rs` (real world.db, `--ignored`): `tuned_creature_health_is_expected_stat_health_at_its_range_maximum`.
- game-engine `unit_frames.rs`:
  - `tuned_target_shows_its_level_for_the_local_player_in_difficulty_colour`;
  - `hostile_target_level_colour_follows_the_level_difference`;
  - `friendly_target_level_stays_gold`.
- game-engine `replication.rs`: the replicated unit components round trip.

## Known gaps (current cycle)

- [ ] The client shows the raw shared health pool. Hogger reads 13,137 to everyone, while Retail shows each viewer max health at their own level.
- [ ] Creature armor (ExpectedStat CreatureArmor) and creature attack power are not modelled. No creature armor exists server-side.
- [ ] Creature attack speed is the server's flat 2 s, not `BaseAttackTime`.
- [ ] Players still swing a flat 10. A level 10 player needs about 114 swings for Hogger.
- [ ] Nameplates show no level. Retail's default nameplates show none either. Unit tooltips are the npctooltip branch's, which calls `level_for_viewer`.
- [ ] Kill reputation still uses the template's `max(minlevel, maxlevel)`. That matches the scaled level except for creatures with a negative delta.
- [ ] Mythic+ seasons are not modelled: ContentTuningXExpected rows with a `MinMythicPlusSeasonID` never apply, and rows with only a maximum season always do. No map 0 tuning uses either.

## Out of scope

- Player-side stat squish.
- Group scaling (`ScalingPlayerLevelDelta`), Chromie Time (`MaxCreatureScalingLevel`) and `ScalingFactionGroup`.
- Instance and map-difficulty ContentTuning (MapDifficulty/AreaTable fallbacks in ApplyLevelScaling).
- Retail's ÷10 XP for creatures of an older expansion stays omitted ([experience](experience.md)).
