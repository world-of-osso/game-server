# Spell Casting

Server-authoritative spell casting in `crates/server/src/spell_cast/`. `SpellCastPlugin` validates `SpellCastIntent`, runs cast bars and channels, spends power, starts GCD/cooldowns/charges, selects implicit targets and resolves effects from [[spell-info]] records. Power pools regenerate from the DB2 `power_type` table and replicate as `UnitPowers`. Aura effects, spell modifiers and aura stat modifiers come from [[auras]].

## Temporary summons and spell area triggers

[Contract](../../specs/spell-summon-area-trigger.md). Effect 28 dispatches once per cast through `summon.rs`; `summon_properties` is loaded from world.db. `scripts/db2.py extract/import` adds local CASC SummonProperties (FDID 1345276, WDC5 layout A4CA5ECF). Missing metadata/template is logged, never synthesized. `SummonedCreature` records owner and SummonProperties; `UnitSummonedBy`, faction and `FollowOwner` reuse pet ownership/movement conventions without modifying PetStable. Positive durations expire on the aura clock; zero/negative durations are untimed. Owner removal/map departure also removes the summon.

Reference: TrinityCore a352b1fa `SpellEffects.cpp::EffectSummonType` / `SharedDefines.h::SummonTitle`. Matrix probe 49028 uses creature 27893, SummonProperties 3242 (ally/runeblade), duration 8000 ms. Other six summon roots' creatures are absent from current world data.

## Pipeline

1. **Receive** (`Update`): only an authenticated in-world connection's `PlayerLink` player casts. The spell resolves by ID or unique exact name. Explicit target = `target_entity`, else the server `CombatTarget`.
2. **Queue or validate:** after spell resolution and known-spell admission, hold one `QueuedSpellCast` (spell snapshot, explicit target and witness) while cast/GCD remaining time is positive and at most 400 ms. New admitted requests replace it. No resource, target or cooldown validation at admission. Outside that window use immediate validation. `start_queued_casts` runs after `advance_casts` and due missile effects in the fixed chain, waits for both active cast and applicable GCD to end, takes the slot once and calls the normal start path. Full validation then runs in order:
   - player casters must know the spell (`class_progression::knows_spell` on [[class-progression]] `KnownSpells`; NPCs exempt; triggered spells skip it);
   - caster alive;
   - not stunned, silenced or pacified ([[auras]] crowd control);
   - no active `CastState`;
   - school not locked out;
   - GCD of the spell's `StartRecoveryCategory`;
   - spell and category cooldown;
   - charges;
   - moving (see below), when the cast bar or the channel would break on movement;
   - every handled effect's implicit targets supported;
   - explicit target;
   - costs.
3. **Explicit target** comes from the first handled effect that selects one:
   - enemy, ally or any relation;
   - alive;
   - `spell_range` min/max, hostile (`_0`) or friendly (`_1`); a max of 0 means 5 yd melee.
   - An ally spell with no valid ally target self-casts (retail auto self-cast).
   - Facing is not checked.
4. **Start:**
   - The GCD starts here: `StartRecoveryTime`, keyed by category, and sends `SpellCooldownUpdate{is_gcd}`.
   - Instant spells execute now.
   - Cast-time spells insert a replicated `CastState` and log `CastStart`.
   - Channeled spells (`SPELL_ATTR1` 0x4/0x40 with a positive duration) execute when their cast time, if any, is over (TrinityCore `Spell::handle_immediate`); `CastState::channel` then runs for the spell duration (Duration mods) and the channel's periodic effects tick as [[auras]].
5. **Execute** (instant, cast completion, before a channel), in order:
   - recheck caster, target and costs;
   - plan all targets;
   - spend costs;
   - start cooldown and charges;
   - log `CastSuccess`, send `SpellGo`, and put caster and targets in combat (`starts_combat`, [[combat]] Combat State);
   - resolve effects in index order, with each target's aura effects applied as one aura after the other effects. A spell with a missile `Speed` or `LaunchDelay` resolves on each unit target after its flight (`spell_cast/missiles.rs`, TrinityCore `Spell::HandleDelayed`): `LaunchDelay + max(max(distance, 5) / Speed, MinDuration)` (Spell.cpp:2486-2518), one missile for a destination spell (`CalculateDelayMomentForDst`, Spell.cpp:880-896). The delay is fixed at launch; a target that moved is still hit, one that died or left the world is not (Spell.cpp:2746-2748, 2807). Frostbolt at 20 yd lands 0.57 s after `SpellGo`; its damage, Chilled, threat, procs and combat log come then. With `SPELL_ATTR4_BOUNCY_CHAIN_MISSILES` (Avenger's Shield) each chained unit is hit the flight from the unit before it after that unit's hit, without another `LaunchDelay` (Spell.cpp:2489-2505). Spell reflection (`spell_cast/reflect.rs`, `WorldObject::SpellHitResult`, Object.cpp:1947-1978): a reflectable spell (`m_canReflect`: magic and not `IS_ABILITY`, or `ALLOW_SPELL_REFLECTION`; not `NO_REFLECTION`, `NO_IMMUNITIES` or passive) rolls each unit target's REFLECT_SPELLS(_SCHOOL) chance at launch; a reflect logs `Miss(Reflect)` with the launch, procs the reflect aura (`PROC_HIT_REFLECT`, which reflect auras alone proc off) at the target's delay and resolves the spell on its caster at 1.5 × that delay (Spell.cpp:2524-2545); `REFLECTION_ONLY_DEFENDS` deflects instead. A Frostbolt into Spell Reflection 23920 at 20 yd spends its charge at 0.57 s and hits the mage at 0.86 s. Speed-0 spells resolve in the launch tick;
   - push proc events (cast success, one spell hit per distinct target) and spend charged spell mods.
   - A failed completion sends `CastFailed`.
6. **Cancel:** stop requests, caster death and movement all remove `CastState`. An ended channel removes its caster's auras of that spell (`auras::channel_ended`) and the ritual objects it held (`meeting_stone::ritual::channel_ended`).

### Pending cast cancellation

Explicit `StopSpellCast` for an active cast removes its `QueuedSpellCast` before canceling the cast. Hostile interruption, movement and death use the existing interrupt/cancel paths and retain the pending slot: release revalidates movement, school lockout, death, targets and costs. A failed start sends owner `CastFailed` and consumes the slot. Policy, pinned sources and proof limits: [spell queue contract](../../specs/cast-presentation.md#spell-queue-window).

### Movement

- Casting still freezes translation (`networking_movement.rs`).
- `process_player_inputs` records `Moving(bool)`: whether the newest input has a direction.
- `advance_casts` runs after input and cancels a non-cast-while-moving cast when `Moving` is true.
- Cast-while-moving is TrinityCore `Spell::CheckMovement`: a cast bar breaks only with `SpellInterrupts.InterruptFlags` Movement (0x1); a channel breaks when `ChannelInterruptFlags_0` holds Moving or Turning (0x8 | 0x10, `SpellInfo::IsMoveAllowedChannel`). Summoning Stone Effect (59782) has InterruptFlags 0 and ChannelInterruptFlags 15406: its 5 s cast survives walking, its channel does not.
- `cast_granted` casts a spell a game object grants its user (meeting stones), without a spellbook check.
- `MovementSpeed` cannot serve as the signal: it holds run/walk speed even while idle.

### Spell modifiers

The caster's [[auras]] spell modifiers (107/108/218/219) adjust cast time (ChangeCastTime, then ÷ spell haste), channel duration, cooldown and category cooldown, GCD (StartCooldown), max range, costs (PowerCostN by `OrderIndex`), effect base points, coefficients, crit and damage/heal amounts.

### Costs

- Every `SpellPower` row with `RequiredAuraSpellID = 0` applies; a row with `RequiredAuraSpellID` applies only while the caster has that aura (priest spec passives gate most priest costs).
- Amount (raw units) = `ManaCost + PowerCostPct% × base`. For mana the base is `CharacterStats.base_mana` (gtBaseMP, [[character-stats]]); a caster without `CharacterStats` fails with `NotEnoughResource`. For other pools and health it is the max pool (TrinityCore `Spell::CalcPowerCost` uses the max for non-mana powers).
- Power type -2 costs health.
- A missing pool or insufficient power fails with `NotEnoughResource`.

### Cooldowns and charges

`SpellHistory` (server-only component) stores absolute ready times from `Time` elapsed seconds:

| What | Keyed by | Source |
|---|---|---|
| Spell cooldown | spell | `RecoveryTime` |
| Category cooldown | `Category` | `CategoryRecoveryTime` |
| GCD | `StartRecoveryCategory` | `StartRecoveryTime` |
| Charges | `ChargeCategory` | `spell_category` `MaxCharges` and `ChargeRecoveryTime` with charge auras 411/453/454, recharging one at a time |
| School lockouts | school bit | interrupts |

- `SpellCooldownUpdate` carries `max(recovery, category recovery)`.
- Haste scales a 750-1500 ms GCD only through MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN 417 whose class mask affects the spell: GCD ÷ (1 + haste), kept within 750-1500 ms (Spell.cpp:9220-9224; paladin passive 137026 for Divine Toll and Hammer of Light). The category-133 spell-haste rule for non-melee spells (Spell.cpp:9209-9218) is not modelled.

## Messages

| Message | Recipients | When |
|---|---|---|
| `CastFailed{reason, detail}` | caster owner | rejected request or failed completion |
| `SpellCooldownUpdate` | caster owner | GCD start; cooldown start |
| `SpellChargesUpdate` | caster owner | charge spent |
| `CombatLogEvent` | owners of source and target (once each) | CastStart, CastSuccess, Damage, Heal, Energize, Interrupt, Death, Miss(Immune); aura events in [[auras]] |

- Other clients in range do not receive combat log events yet.
- Melee auto-attack still broadcasts the old `CombatEvent` to every client. Retirement: remove it when the client renders `CombatLogEvent`.
- Spell effects emit only `CombatLogEvent`.
- `log_event` stamps every event with server wall-clock Unix milliseconds before recording damage or sending to owners, including queued melee events. Constructors leave the timestamp unset until that boundary.
- `Interrupt.extra_spell_id` identifies the interrupted cast; `Dispel.extra_spell_id` identifies the removed aura. Other kinds leave it `None`; `spell_id` remains the interrupting or dispelling spell.

Failures use the contract `CastFailReason` directly (`TooClose`, `SchoolLockedOut`, `SpellInProgress`, `NoChargesRemain`, ...). `NotSupported` carries the cause in `detail`: "unknown spell", "spell not known", "implicit target N not supported", "power type N". `NotInFront` is unused (no facing check).

## Implicit targets

Relations come from faction reactions ([[factions]]): enemy = `can_attack`, ally = `can_assist` (the caster is always its own ally). The hostile range applies when the caster's reaction to the target is Hostile, else the friendly range.

| IDs | Handling |
|---|---|
| 0 | none. If both slots are 0, the effect hits the explicit target, else the caster |
| 1 | caster |
| 6 / 21 / 25 | explicit enemy / ally / any |
| 18, 22 | area center = caster |
| 53 / 132 / 63 | area center = explicit enemy / ally / any |
| 15 / 30 | area around the source (default caster), enemy / ally |
| 16 / 31 | area around the destination (default explicit target, else caster), enemy / ally |
| 56, 120 | caster (no group or pet backend) |
| 57, 118 | explicit ally |

- Radius: slot A uses `EffectRadiusIndex_0`; slot B uses `_1`, falling back to `_0` (TrinityCore `CalcRadius`).
- Area results are nearest first, capped by `SpellTargetRestrictions.MaxTargets`.
- Any other ID (for example 87 DEST_DEST, 104 cone, 47 front) fails the cast when it belongs to a handled effect.
- Effects with the same implicit targets (and, for areas, the same radii) share the first one's selection, as `Spell::SelectSpellTargets` (Spell.cpp:741-781): Avenger's Shield's interrupt and silence hit every unit its damage chains to.
- Chains (`targets::chain_targets`, `Spell::SearchChainTargets`, Spell.cpp:2216-2320): an explicit unit target with `EffectChainTargets` > 1 jumps to the living unit nearest the last one within the jump radius (7.5 yd ranged, 5 yd melee, 10 yd otherwise), searched within the radius × the jumps left; `SPELL_ATTR2_CHAIN_FROM_CASTER` searches from the caster over the spell range and `ChainFromInitialTarget` keeps the first target as the source. Each jump multiplies damage and healing by `EffectChainAmplitude` (`m_damageMultipliers`, Spell.cpp:8631-8636). Not modelled: line of sight, the melee front arc (`SPELL_ATTR5_MELEE_CHAIN_TARGETING`), chain heal (target 45 is unsupported) and the ChainTargets/ChainJumpDistance/ChainAmplitude spell mods.
- Triggered spells take their explicit unit whatever its relation to the caster (`TRIGGERED_IGNORE_TARGET_CHECK`, Spell.cpp:3489-3491).

## Effects

Registry: `effects::handler(effect_id)`. Unhandled effect IDs are skipped and logged once per (spell, effect).

| ID | Name | Handling |
|---|---|---|
| 2 | SCHOOL_DAMAGE | scaled amount (below), crit, shared death path |
| 6, 35, 65 | APPLY_AURA, APPLY_AREA_AURA_PARTY/RAID | grouped per target, `auras::apply_aura` |
| 10 | HEAL | scaled amount, crit, clamped to max, overheal logged |
| 30 | ENERGIZE | Points/PointsIndexN-modified base points in raw units of `EffectMiscValue_0`, clamped, excess logged |
| 31 | WEAPON_PERCENT_DAMAGE | `BasePoints`% of a weapon hit |
| 38 | DISPEL | `auras::dispel` of `EffectMiscValue_0`, up to `BasePoints` (min 1) auras |
| 58, 121 | WEAPON_DAMAGE, NORMALIZED_WEAPON_DMG | weapon hit + `BasePoints` |
| 64 | TRIGGER_SPELL | casts `EffectTriggerSpell` at each target, bounded (below) |
| 68 | INTERRUPT_CAST | cancels an interruptible `CastState` and locks the interrupted spell's school mask for this spell's duration (Kick 1766: 6 s) |
| 136 | HEAL_PCT | `BasePoints`% of target max health |

TRIGGER_SPELL:
- The triggered spell ignores cost, cooldown, GCD and cast time.
- Depth is capped at `MAX_TRIGGER_DEPTH` = 3 hops, the same as the coverage closure.

Spell scripts (`spell_cast/scripts.rs`, TrinityCore `SpellScript`): behaviour the DB2 effects do not carry, keyed by spell ID. `on_hit` runs once per distinct planned unit after the spell's effects and auras resolved there (`SpellHitFn`, `Spell::DoSpellHitOnUnit`), before the hit proc events.
- Frostbolt 116: its only effect is TRIGGER_MISSILE 32 to 228597 (SCHOOL_DAMAGE); neither carries a slow. The hook casts Chilled 205708 (APPLY_AURA MOD_DECREASE_SPEED 33, -50%, `SpellDuration` 31 = 8 s, `SPELL_ATTR0_AURA_IS_DEBUFF`) at the hit unit, triggered (TrinityCore `spell_mage_frostbolt::HandleChilled`, spell_mage.cpp:978-995, registered on 116 in sql/old/9.x/world/22012_2022_03_06/2022_02_10_02_world.sql:5).
  A Frostbolt-pulled Blackrock Spy chases at 3 yd/s instead of 6 (game-server `polymorph_tests.rs`).

New-class scripts use per-effect `on_effect_hit` hooks; a DUMMY effect with a hook participates in target validation/planning rather than being silently skipped. `area_target_filter` narrows selected area targets before hit resolution.
- Chaos Strike Initial 344862 effect 0 triggers 162794, whose effects trigger the existing damage spells 222031/199547 (TrinityCore `spell_dh_chaos_strike_initial`).
- Living Flame 361469 effect 0 triggers heal 361509 for an assistable target or damage 361500 otherwise (`spell_evo_living_flame`). Energizing Flame's mana refund remains unimplemented.
- Azure Strike 362969 effect 1 retains the explicit enemy plus effect 0's baseline count minus one other selected enemy (`spell_evo_azure_strike`). Other enemies are nearest-first, not TrinityCore's random selection; count-changing talent modifiers remain unimplemented.
- Tiger Palm 100780 already has SCHOOL_DAMAGE and ENERGIZE effects. ENERGIZE now applies existing spell-point modifiers: Windwalker passive 1258122 adds 2 Chi to effect 1's zero baseline. Blackout Kick uses existing damage and aura-gated cost handlers. Power pool sizing/spec/form selection remains separate powers-agent work.

Formulas:
- **Scaled amount:** `BasePoints + BonusCoefficient × SP + BonusCoefficientFromAP × AP`.
  - SP and AP are `CharacterStats.spell_power` / `attack_power` ([[character-stats]]).
  - Units without `CharacterStats` (NPCs) have SP = AP = 0.
- **Crit:** `CharacterStats.crit_pct` (5% base for players, 0 for NPCs; MOD_CRIT_PCT auras fold in through `StatModifiers`) + CritChance mods. A crit adds 100% of the amount, scaled by CritDamageAndHealing mods.
- **Damage and heal mods:** see [[auras]]: spell mods, done/taken multipliers, immunity and absorbs.
- **Weapon hit:** a roll in the main hand's `[min, max]` + `AP / 6 × speed`; 121 uses the weapon type's normalized speed. Main-hand damage comes from the equipped weapon ([[character-stats]] Gear), unarmed 1–2 at 2.0 s.

Damage and heal side effects:
- Damage sets `LastDamager` and puts source and target in combat with each other (`combat_refs::set_in_combat_with`, [[combat]] Combat State).
- A lethal hit on an NPC applies `DeathState` and removes `Replicate`.
- A lethal hit on a player writes `PlayerDeathTransition`.

## Powers

- **Pool list:** the class list comes from `chr_classes_x_power_types`, minus the alternate bars 10/23/24/25.
- **Order:** `chr_classes.DisplayPower` first, then ascending ID. Example: DK = Runic Power, Runes.
- **Initial values:** each non-mana pool starts at `DefaultPower`, with max `MaxBasePower`.
- **Max** (`Player::UpdateMaxPower`): `MaxBasePower` + MOD_INCREASE_ENERGY 35 + MOD_MAX_POWER 418 of that power ([[auras]]), rounded; a lower max clamps current. `power_type` holds the retail base, passives the rest: Holy Power 3 + Boundless Conviction 115675 (+2, auto-learned) = 5; Chi 4 + Windwalker Monk 1258122 (+1) = 5, + Ascension 115396 = 6; Runes 6 + Runic Longevity 194653.
- **Display power** (`Unit::CalculateDisplayPowerType`): Cat Form and Ghoul → Energy, Bear/Dire Bear → Rage, Travel/Ghost Wolf → Mana, otherwise `DisplayPower`. The display power leads `order` (the first `UnitPowers` entry); the rest stay ascending. See MOD_SHAPESHIFT in [[auras]].
- **Runes** (`spell_cast/runes.rs`, TrinityCore `Runes`, `SetRuneCooldown`, `Player::RegenerateAll`, `Spell::TakeRunePower`): per-rune cooldowns, not a regenerating pool. A spent rune (lowest ready index first) gets the rune cooldown and joins the back of the depletion order; only the first 3 (`MAX_RECHARGING_RUNES`) recharge, a finished rune leaves the order and its leftover time is dropped. The Runes pool holds the ready count, so costs check it like any pool; ENERGIZE of runes finishes the front of the order. Cooldown: 10 s (`RUNE_BASE_COOLDOWN`) ÷ (MOD_POWER_REGEN_PERCENT on runes × (1 + haste)); TrinityCore multiplies by (1 − pct) instead.
- **Mana** stays in the legacy `Mana` component, with its max from `stats.rs`. `PowerPools` only orders it. Retirement: move mana into `PowerPools` and delete `Mana` when the client reads `UnitPowers`.
- **Units:** pools hold the raw DB2 values, the same units as `SpellPower` costs and ENERGIZE.
  - Rage, Runic Power, Soul Shards, Lunar Power and Pain are tenths (`DisplayModifier` 10). Insanity is hundredths.
  - `UnitPowers` sends raw `current`/`max` floored to integers, `partial` fractional thousandths and signed `regen_per_sec`; `charged_points` contains the aura-derived 1-based combo-point indices. The client divides by the display modifier for `PowerType`.
  - Resend on whole value, max, rate, display order or charged-point changes, not partial alone. The sent fraction includes that tick's regeneration. The rate becomes zero on the tick a pool reaches its max or decay center.
  - `UnitRunes` sends `duration_ms` and rune-indexed `ready_in_ms`; queued runes include their wait, so values can exceed duration. Resend on depletion-order, duration or rune-count changes, not countdown ticks. A max-only rune passive must update the view even when every rune is ready.
- **Regen** (raw units per second, TrinityCore `Player::Regenerate`):
  - In combat: `RegenCombat`.
  - Out of combat: `RegenPeace`, starting only after `RegenInterruptTimeMS` since leaving combat.
  - A positive rate fills toward max. A negative rate moves toward `CenterPower`: Rage/RP decay to 0, and Soul Shards settle at 3.
  - Examples: Energy +10/s. Rage −12.5/s (−1.25 rage/s) out of combat only. Holy Power −0.1/s after 10 s.
  - Mana has rate 0 in the data. It regenerates 1% of base mana (gtBaseMP, `CharacterStats.base_mana`, not the pool max) per second, in and out of combat, times the aura regen multiplier (TrinityCore `Player::UpdatePowerRegen` `basemana / 100`; SimC `init_resources`).
  - Class/spec passives supply the multiplier via MOD_MANA_REGEN_PCT/MOD_POWER_REGEN_PERCENT: class −36% (Paladin, Priest, Shaman, Druid), healer spec +25%, hybrid spec +60%, Arcane +60%. Level 80 (base 50 000): Holy Paladin 400/s, Frost Mage 500/s, Arcane Mage 800/s.
  - Not implemented: flat MOD_POWER_REGEN 85 (per 5 s); haste scaling of Energy/Focus regen (`power_type` flag `RegenAffectedByHaste` 0x400).
  - No five-second rule: Mana lacks `UseRegenInterrupt` (0x2), so spending mana pauses nothing (TrinityCore `Player::GetPowerRegen`). Focus +5/s in and out of combat; Runic Power holds in combat and decays −12.5/s (−1.25/s) out of combat.
  - Dead players regenerate nothing (`Player::Update` calls `RegenerateAll` only while alive).
- **Player health regen** (`powers::regenerate_player_health`, TrinityCore `Player::RegenerateAll`/`RegenerateHealth`, Player.cpp:1594-1802): `PlayerHealthRegen` counts ms while alive. Every 2000 ms, out of combat, health gains 20% of max ÷ level below level 15, else 1.5% of max. The whole part is added and the fraction carried; reaching max sets max and clears the fraction. Out of combat that base is × MOD_HEALTH_REGEN_PERCENT 88, plus 0.4 × MOD_REGEN 84 (per 5 s to per 2 s); in combat only MOD_REGEN_DURING_COMBAT 116 regenerates, `Rate.Health` (1) × its percent. Either way ×1.5 while not standing ([[stand-state]]). Always added: MOD_HEALTH_REGEN_IN_COMBAT 161 and gear `ITEM_MOD_HEALTH_REGEN` ÷ 2.5 (`CharacterStats.health_regen`, `m_baseHealthRegen`). In combat it runs only with 116, 161 or gear regen; while polymorphed, max ÷ 3 even in combat. Level 10, max 1110: 22.2 per tick, 666 → 1110 in 20 ticks (40 s).
- **SpellScaling amounts** (`effects::calc_base_value`, TrinityCore `SpellEffectInfo::CalcBaseValue`): an effect with a scaling `Coefficient` is its scaling class column of gtSpellScaling (`spell_scaling_data.rs`, generated by `scripts/gen_spell_scaling_table.py` from local CASC `gametables/spellscaling.txt`) at the caster's level, clamped to its `spell_scaling` range, × the coefficient. Food 1127 at level 10: Consumable 1000 × 0.25 = 250 MOD_REGEN. Not modelled: `SPELL_ATTR11_SCALES_WITH_ITEM_LEVEL` (item-level RandPropPoints; such spells keep BasePoints), `SPELL_ATTR8` target level, the casting item's -6/-7 multipliers.
- **Rage from auto-attacks** (`combat.rs` `reward_swing_rage`, Retail values from SimC `sc_warrior.cpp`; AzerothCore's Wrath `RewardRage` damage/level conversion is not used). Only units whose primary power is Rage (AzerothCore `HasActivePowerType(POWER_RAGE)`):
  - Dealt: every landed swing gives swing time (2 s) × 1.75 rage × (1 + Σ MOD_RAGE_FROM_DAMAGE_DEALT 213 %), rounded to 0.1 rage (`melee_t::trigger_rage_gain`). Unspecced 3.5 rage; Arms Warrior 137049 (+250%) 12.3 rage; Protection 1258163 (−56%).
  - Taken: a damaging swing taken gives 3 rage, at most once per second (`warrior_t::assess_damage`).
- **NPCs** get `UnitLevel` (template `min_level`) and empty `UnitPowers`. Creature class/level power stats are not imported.
- **Players:** `UnitLevel` is owned by [[class-progression]]; the cast pipeline only reads levels.

## Sources

- [spell_cast/mod.rs](../../../crates/server/src/spell_cast/mod.rs) — pipeline, `SpellHistory`, messages
- [spell_cast/targets.rs](../../../crates/server/src/spell_cast/targets.rs) — implicit targets, range
- [spell_cast/effects.rs](../../../crates/server/src/spell_cast/effects.rs) — handlers, damage/heal paths
- [spell_cast/powers.rs](../../../crates/server/src/spell_cast/powers.rs) — `PowerTypeTable`, pools, regen, `UnitPowers`
- [spell_cast/runes.rs](../../../crates/server/src/spell_cast/runes.rs) — Death Knight rune cooldowns
- [spell_cast/class_resource_tests.rs](../../../crates/server/src/spell_cast/class_resource_tests.rs) — maxima, forms, runes, charged points
- [spell_cast/tests.rs](../../../crates/server/src/spell_cast/tests.rs) — real-DB App tests
- [spell_cast/test_harness.rs](../../../crates/server/src/spell_cast/test_harness.rs) — shared real-DB harness (also used by aura tests)
- [spell_cast/new_class_tests.rs](../../../crates/server/src/spell_cast/new_class_tests.rs) — six core spells with local DB2 and HostClient delivery
- [new-class spec](../../specs/new-classes.md) — bounded behavior and remaining integration work
- [spec](../../specs/cast-presentation.md)

## See Also

- [[spell-info]] — the data this resolves
- [[character-stats]] — spell power, attack power, crit, base mana
- [[auras]] — aura runtime, procs, spell and stat modifiers
- [[combat]] — melee, combat state, death
- [[networking]] — input ordering, replication dirty queue
