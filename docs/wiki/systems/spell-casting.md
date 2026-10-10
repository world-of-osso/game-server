# Spell Casting

Server-authoritative spell casting in `crates/server/src/spell_cast/`. `SpellCastPlugin` validates `SpellCastIntent`, runs cast bars and channels, spends power, starts GCD/cooldowns/charges, selects implicit targets and resolves effects from [[spell-info]] records. Power pools regenerate from the DB2 `power_type` table and replicate as `UnitPowers`. Aura effects, spell modifiers and aura stat modifiers come from [[auras]].

## Teleports and aura overrides

[Contract](../../specs/spell-teleports-overrides.md). Retail Blink1953 uses effect29 and target55's second radius (20yd); legacy teleport15 and current252 share dispatch. Forward movement samples GroundMap and stops 0.5yd before the first blocking baked LOS triangle using monotonic prefix queries. Missing ground/LOS never becomes an unchecked move. Same-map players use MovementControl's teleport epoch; cross-map moves reuse NewWorld/WorldPortAck.

Target9 uses HomeBind, persisted separately in redb character_home_binds and loaded at login. New characters persist their actual creation position in the character-creation transaction. Login only loads saved binds; existing characters without a row fail Hearthstone explicitly, never infer a bind from race or logout position. Innkeeper binding UI is not part of this change. Target17 reads authored `tdb_spell_target_position` from the established TDB1210.26091 archive through `import-azerothcore.sh` / `mysql_dump.py --prefix tdb_` and `tdb_schema.sql`. Import preserves the `(ID, EffectIndex, OrderIndex)` key and nullable Orientation; fixed destinations consume OrderIndex0. NULL orientation uses DifficultyID0 Retail SpellEffect.EffectPos_facing, converting degrees above 2π to radians, as TrinityCore a352b1fa SpellMgr::LoadSpellTargetPositions does. Rows with no matching Retail effect cannot supply a cast destination. Missing destinations still fail explicitly. Target106 ordered nearby selection is not implemented by this fixed-destination loader. Real TDB mage teleport3567 supplies the cross-map cast/ack fixture; no inferred city coordinates or alternate unprefixed table. Source: TrinityCore a352b1fa SpellMgr.cpp LoadSpellTargetPositions, Spell.cpp destination selection, SpellEffects.cpp EffectLeap/EffectTeleportUnits.

Harpoon190925's trigger186260 uses effect41; Feral Lunge196884 uses effect254/target53 then148. Both move the caster through the same endpoint/ground/LOS path, never the enemy. Effect254's target87 accepts SpellCastIntent.destination in Bevy coordinates, validates finite values and DB2 range, and retains it per cast/queue rather than in shared player state. Target grouping skips an earlier effect with unsupported targets: Infernal Strike189110's unhandled dummy87 cannot lead its supported jump254. This implements authoritative endpoints with movement teleports, not a parabolic motion presentation.

Aura332 resolves MiscValue-specific or SpellClassSet/mask substitutions using its amount as replacement ID. KnownSpells/action slots stay unchanged; removal/expiry restores originals. Queued requests retain the original ID and resolve the active override on release. UnitAuras now carries AuraOverride::ActionBar pairs and AuraOverride::Animation set IDs (aura312 MiscValue), including normal expiry/removal. Companion shared-protocol branch `spell-teleport-override` is required. Client must initialize new protocol fields, consume actionbar/animation overrides, and provide ground destinations; no client consumption or live rendering proof is claimed. TC a352b1fa Unit.cpp GetCastSpellInfo and SpellAuraEffects.cpp aura312 clientside mapping ground these behaviors.

## Temporary summons and spell area triggers

[Contract](../../specs/spell-summon-area-trigger.md). Effect 28 dispatches once per cast through `summon.rs`; `summon_properties` is loaded from world.db. `scripts/db2.py extract/import` adds local CASC SummonProperties (FDID 1345276, WDC5 layout A4CA5ECF). Missing metadata/template is logged, never synthesized. `SummonedCreature` records owner and SummonProperties; `UnitSummonedBy`, faction and `FollowOwner` reuse pet ownership/movement conventions without modifying PetStable. Positive durations expire on the aura clock; zero/negative durations are untimed. Owner removal/map departure also removes the summon.

Reference: TrinityCore a352b1fa `SpellEffects.cpp::EffectSummonType` / `SharedDefines.h::SummonTitle`. Matrix probe 49028 uses creature 27893, SummonProperties 3242 (ally/runeblade), duration 8000 ms. Other six summon roots' creatures are absent from current world data.

Effect 179 dispatches once per cast through `area_trigger.rs`. Radius is the first supplied SpellRadius (including the second radius slot); duration comes from the cast spell. An explicit EffectTriggerSpell executes through the normal triggered pipeline, on entry when EffectAuraPeriod is zero, periodically otherwise. Inside selection uses caster map, living units, radius and the trigger's explicit relation; depth and casting item level survive delayed area execution. Missing trigger/radius logs a lifecycle-only entity, not an invented action.

The nine matrix roots all lack trigger bindings, including Wildfire Bomb's child 265163. Purging Breath 1287715 → 1287711 supplies an authentic non-matrix damage binding for engine proof. Local AreaTriggerCreateProperties (FDID 1937227) contains none of these matrix create-property IDs: their server actions/scripts cannot be reconstructed from that client table. Selector87 destination propagation was added by master toy-effects4; area-trigger actions without bindings and moving-orb behavior remain unclaimed.

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
- 87/104/134 now support selected destination, authored destination-directed enemy cone and line;47 selects the caster-front destination. Other unimplemented IDs still fail a handled cast.
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

## Class Dummy scripts

### Eighth batch: projectile, taunt and low-coverage paths

[Contract](../../specs/spell-dummy-scripts.md#eighth-batch-locally-derived-paths), [implementation](../../../crates/server/src/spell_cast/scripts/batch8.rs), [cast tests](../../../crates/server/src/spell_cast/dummy8_tests.rs), [incoming-damage test](../../../crates/server/src/spell_cast/dummy8_defense_tests.rs). Mainline12.1.0.69933 local CSVs and read-only imported rows provide all IDs/amounts; TDB1210 routes named scripts.22 IDs/130 CSV rows match both the slot and canonical databases exactly. No new external fetch, data import or realm process.

| Root | Implemented and observed | Uncertified / missing |
|---|---|---|
|1226019 Reap|Named1225827 damage follows87/134 caster-to-destination line: catalog30yd radius/Width20, multiple enemies, destination direction rather than caster facing; behind/far/ally/other-map exclusions.|Soul collection, optional Fury, aura4/312; unit-center geometry lacks combat-reach edge expansion.|
|115546 Provoke|116189 actual114 attack-me equalizes highest threat and changes victim; active aura11 holds victim6000ms and multiplies this Monk's generated threat by9. Owned61146 statue selects118635's10yd enemy area; foreign statue/ordinary ally fail before costs.|Optional328670 speed; overlapping/reapplied taunt precedence; no player forced-target claim.|
|264178 Demonbolt|AfterCast280127 grants20 raw Soul Shards once, retaining damage.|Zero Dummy1/460553 marker; not relabelled no-ops.|
|187874 Crash Lightning|Catalog104 cone120 degrees/radius8;187878 enhancement lasts12sec.|Root Dummy1, proc318, cleave consumption/195592 split damage. Current Mainline tooltip lacks old TDB two-target gate.|
|115181 Breath of Fire|Catalog104 cone90 degrees/radius12; only owned Keg targets receive123725 DoT and deal5% less damage to that Monk, not foreign players; expiry restores normal damage.|Secondary falloff; no generic269 semantic change.123725s2 is consumed by a spell-specific incoming-damage script.|
|51723 Fan of Knives|One combo point for a successful cast's first damage hit; none for empty area, never per enemy.|Poisons, damage soft cap5.|
|34861 Sanctify|Five injured allies in requested10yd ground area; healthy/sixth/far units excluded.|Dummy3; Prayer recharge reduction retained.|
|29722 Incinerate|244670 supplies two base fragments once for root damage, not each splash unit.|Critical/talent additions and other markers.|
|30108 Unstable Affliction|233490 named base periodic aura applies/ticks/expires.|Five-instance accumulation, vulnerability, dispel/death rewards.|
|44425 Arcane Barrage|AfterCast consumes all Arcane Charges.|Charge-scaled damage, secondary40% hits, optional mana refund.|

Selectors104/134 use authored destination direction, radius, ConeDegrees/Width, map/life/faction selection. Missing required restriction metadata fails explicitly before costs; no guessed universal cone angle/width. Requested destination also now reaches ordinary31 ground-area selection, rather than accidentally centering Sanctify on its self-cast explicit target. Older batch6/7 plans pass `spell.cast_destination` to master's five-argument `effect_dest` API.

Targeted proof:13/13 at`70c4f20`, plus real incoming Breath damage/owner exclusion/expiry1/1 at`2d883e8`, after behavioral RED12/13 and a separate missing-reduction RED. After merging current origin/master`afecfa6`, the requested broad `cargo test --locked -p server --bin game-server spell_cast` ran once at`2fb37d8`: **261/261 pass**, including all14 batch8 functions and the incomplete-metadata104 refusal. Native format/check exit0, warning-free. No live/full-project/rotation certification.

Census compares frozen post-master-integration baseline`52d359f` with`2fb37d8`: **172→171** unresolved Dummy core roots (Provoke closes); reviewed/partial/deferred70→73, un-derived102→98. E3 admissible IDs/memberships124/132→123/131 (additional D1→0), a4 63/71→64/72 (D1 unchanged), a22619/20 unchanged (D0). Master toy-target support accounts for baseline admission differences versus batch7's historical117/125, not new dummy8 scripts. Crash exposes latent187878 Dummy markers. Scope stays13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships. No skipped-child concealment.

Durable receipts, frozen code, census generator, ranked remaining roots and logs: `game-engine/data/diagnostics/spell-dummy8-2026-10-10/`. Delivery/proof ledger: `/home/osso/.worktrees/handoff-spell-dummy8.md`; `SPELL_BATCH8_MODE=before python3 <evidence>/regenerate.py --write` reproduces baseline; default mode regenerates after. Parent retains independent acceptance responsibility.

### Seventh batch: TDB-routed Retail derivations

Implementation [batch7.rs](../../../crates/server/src/spell_cast/scripts/batch7.rs), Survival lifecycle in [aura scripts](../../../crates/server/src/auras/scripts.rs); [bounded contract](../../specs/spell-dummy-scripts.md#seventh-batch-locally-derived-paths). TDB1210.26091 registrations route investigation, not behavior inference from names alone. Across the172 ranked roots,88 have exact script bindings, one has a proc override and four have linked rows. No spell_bonus_data table exists: local Mainline12.1.0.69933 SpellEffect SP/AP/scaling/resource coefficients provide numerical equivalents. Pinned TC a352b1fa hooks plus local descriptions/effect chains derive the selected paths;217 CSV rows across38 IDs match read-only SQLite with no semantic drift.

| Root | Runtime hook / source | Retained boundary |
|---|---|---|
|167105 Colossus Smash|After damage0, apply only208086 aura271; ordinary caster-owned damage multiplier and expiry execute it.|Child343/self marker, soft cap and optional talents.|
|61336 Survival Instincts|Aura apply/remove owns50322 effect0, excluding its unknown Dummy2.|Parent marker/child Dummy2; no full Druid-form parity.|
|206930 Heart Strike|Dummy0 on each native chain target,221536 gate,210738 energize.|Rank316575, extra mirrors and native chain front-arc limit.|
|194509 Radiance|Spell-owned native heal plan, s3 additional cap; stable owned-Atonement priority within injured allies.|Actual Atonement duration/application; raid/pet priority.|
|85288 Raging Blow|Root s1 roll with383854,280270 application, restore one latest charge and notify owner.|Marker may be consumed by root proc; other reset talents.|
|190411 Whirlwind|Caster Dummy1 before weapon children; select count from Dummy0 native area, generate modified280715 and apply85739 maximum stacks.|Cleave copies/consumption and damage soft cap.|
|360995 Verdant Embrace|AfterCast Panacea387761 gates387763 self heal.|Flight, primary heal and root Dummy0.|
|1822 Rake|Initial damage and stun hooks require stealth plus231052 or405834, reading s4.|Child periodic snapshot remains open; s4 stays counted.|

Native proof:21 batch7 functions pass at Rust`81c2bf6`, including100 Survival variance samples, two real Raging casts preserving oldest recharge, both Rake rank IDs, Whirlwind four-stack catalog cap, ownership/rank/radius/expiry exclusions. Format/check exit0, warning-free. The requested full spell_cast filter ran **once**,244/245 at`df14af6`: Survival's fixture omitted Bear5487 effect12's existing3% Fire reduction, creating random overlapping-bound failures. Instrumented RED reproduced it; corrected expected97%/48.5% damage multipliers pass in the100-sample fixture and21/21 batch. Production scripts did not change after that broad run; the other244 passing functions remain valid. No repeated full filter or fabricated successful245/245 claim. Parent owns integration acceptance; no realm/full-project/rotation certification.

Census baseline172 reproduced; after172, reviewed/partial/deferred63→70, un-derived109→102. E3 stays117 IDs/125 memberships: Raging's root E3 closes while Survival exposes its child's unresolved Dummy2. Latent child markers increase a4 from57/65 to60/68 (Colossus, Raging, Whirlwind); a226 stays14/15. Gate memberships8/4/5 and scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged. No no-op registrations or skipped-child concealment.

Reap1226019→1225827 and Provoke115546→116189 were derived but deferred in batch7; their projectile geometry and actual taunt/owned statue paths are now implemented in [batch8](#eighth-batch-projectile-taunt-and-low-coverage-paths). Durable source/archive extracts, schema-aware172-root mining, before/after census and proof ledger: `game-engine/data/diagnostics/spell-dummy7-2026-10-10/`; `/home/osso/.worktrees/handoff-spell-dummy7.md` records publication/release status.

### Sixth batch: local Retail rows

Implementation [batch6.rs](../../../crates/server/src/spell_cast/scripts/batch6.rs); [bounded contract](../../specs/spell-dummy-scripts.md#sixth-batch-locally-derived-paths). Eight ranked paths plus supplementary Ice Lance, emphasizing Balance/Resto, Ret, Brewmaster, Survival, Shadow and Frost. Local Mainline12.1.0.69933 DB2 descriptions/effects are checked against read-only SQLite; description newline normalization is not semantic drift. No external sources, invented IDs or world.db imports.

| Root | Authored action | Remaining boundary |
|---|---|---|
|93402 Sunfire|164815s1/o2/d base bridge;231050 explicitly spreads only the DoT through its native child area.|Root Dummy1 marker and optional child271/537 effects remain open.|
|85256 Templar's Verdict|224266s1 damage bridge; apply only child effect0.|Unknown parent/child Dummy markers remain open.|
|30455 Ice Lance (supplement)|228598s1 damage bridge.|Freezing/shatter unimplemented; not a Dummy census closure.|
|194153 Starfire|s3=70% secondary damage; native area/destination/resource retained.|429523 variant, soft-cap8 and effect3/77 semantics remain open.|
|121253 Keg Smash|s8=50% primary bonus; native secondary damage/slow retained.|Root markers, Shuffle/Brew reduction and damage soft cap remain open.|
|212436 Butchery|294029 grants s2=3000ms per successful damage-effect target (including full absorption), at most s3=5; existing259495 recharge only.|Damage soft-cap formula/talent bleed not certified; recharge proof is not Wildfire Bomb damage proof.|
|48438 Wild Growth|s2=5 injured allies inside native target-centered area; existing periodic aura execution.|Declining tick curve/Tree of Life target additions unimplemented.|
|32379 Shadow Word: Death|s6=5% caster-max-health backlash after target survives, ordinary damage/absorb/death/log path.|Other damage-effect/execute variants not certified.|
|191034 Starfall|327541s1=3sec owned Moonfire/Sunfire extension, s2=28sec remaining cap; do not reset ticks.|Base overlap/tick-child contracts remain open.|

Source receipts, RED/GREEN and final proof ledger: `game-engine/data/diagnostics/spell-dummy6-2026-10-10/`; delivery state: `/home/osso/.worktrees/handoff-spell-dummy6.md`. Initial14 cast functions reproduced11 missing-path failures and3 negative gates. At Rust revision`7cff842`, requested full `cargo test --locked -p server --bin game-server spell_cast` ran once: **224/224 pass**, including17 batch6 functions. Format/check exit0, warning-free. Fully absorbed Butchery regression uses authentic17 (200SP): corrected RED proves missing recharge; damage-effect hit ledger now records zero-loss landed hits without creating damage procs. Wild Growth's initial GREEN tick assertion was corrected to wait through catalog cast time plus first1000ms period. Parent owns independent integration proof; no live realm/full-project/rotation-parity claim.

Census frozen baseline **173** reproduced exactly; after **172**, only Wild Growth's Dummy target-cap field closes a full type gap (not its declining-heal semantics). E3 admissible IDs/memberships118/126→117/125; a4=57/65, a226=14/15 unchanged, additional gated memberships8/4/5 unchanged. Remaining172:63 reviewed/partial/deferred,109 un-derived; scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged. Butchery's s3 remains unresolved for its damage-soft-cap action despite cooldown-cap consumption; no false whole-root closure. Sunfire/Verdict child markers remain latent even when base scripts exclude them.

Batch6 left selector104 unsupported. [Batch8](#eighth-batch-projectile-taunt-and-low-coverage-paths) implements authored caster-to-destination cones; missing ConeDegrees/Width metadata still fails explicitly, preserving the old incomplete-metadata104 fixture. Augmentation candidates need healer-role/selection, stat-distribution or empower/flight contracts beyond these bounded derivations.

### Fifth batch: local Retail rows

Implementation: `spell_cast/scripts/batch5.rs`; [contract](../../specs/spell-dummy-scripts.md#fifth-batch-locally-derived-paths). Sources exclusively local Mainline12.1.0.69933 DB2, cross-checked against read-only SQLite. No fabricated IDs or external source fetches. Eight ranked paths plus supplementary Lava Burst favor DH, Evoker, Warlock, Rogue and DK over previously frequent Mage/Monk additions; Shaman receives the Lava Burst path.

| Root | Authored action / native proof | Retained limits |
|---|---|---|
|162243 Demon's Bite|Applied258876's s3/s4 grant5–10 additional Fury; two casts prove rank/base25.|Root Dummy0 unidentified; full talent damage parity not certified.|
|162794 Chaos Strike|Tooltip197125h=20% rolls once on root hit, refunds193840s1=20 Fury. Chance100/0 catalog clones prove two weapon hits and one/no refund.|Root30 value is not separately consumed; ordinary probability frequency and talent variants not certified.|
|356995 Disintegrate|Own370898s1/d applies50% slow at channel application, not each tick; one cast proves damage ticks and3sec expiry.|Caster-side zero-valued aura4 remains unidentified.|
|116858 Chaos Bolt|Own description guarantees crit and crit-percentage scaling; zero/30% stat casts prove858/1116 damage within catalog variance.|Root Dummy1 Base20 remains unresolved.|
|324536 Malefic Rapture|Own324540s1 reference; count owned periodic damage/leech/percent effects, not heals/foreign auras. One cast proves one/two effects, ally/foreign/heal exclusions.|Base root E3 action closed, not full talent variants.|
|197835 Shuriken Storm|Own s2=one combo point per target hit. One cast proves two enemies→two points and ally/far exclusion.|Dummy2 stealth50 and Dummy3 soft-cap8 remain unimplemented.|
|195292 Death's Caress|Own s3=two Bone Shield195181 stacks. Two casts prove initial2 and9→10 cap while retaining damage/Blood Plague.|Child Bone Shield marker/consumption/haste/armor and Blood Plague leech not certified.|
|207317 Epidemic|Own212739s1/s2 gives infected-primary/other-nearby96/39 damage (AP100), only owned Virulent Plague. Own77575s3 extends owned Dread/Virulent by1000ms.|One cast proves ownership/radius/ally and Virulent expiry; separate Dread tick-clock assertion absent. Latent plague variants unimplemented.|
|51505 Lava Burst (supplement)|Own Flame Shock condition guarantees child285452 crit and own description scales with crit stats. Two casts prove condition/no condition and preserved8/0 Maelstrom.|343725 already adds8 to parent effect1 through generic PointsIndex1; no extra Energize script. No full talent/overload parity claim.|

At Rust revision`f6b842a`, the requested full `cargo test --locked -p server --bin game-server spell_cast` ran once:207/207 pass, including14 new functions and the updated deterministic gross-cost Chaos Strike fixture. A test-only coefficient-bound correction at`e89d353` avoids rare rounded-boundary flakes;14/14 batch5 functions re-proved there. Other193 filtered functions and production cargo-check proof remain valid; no redundant full-filter rerun. Format renewed exit0. These are native process/App proofs, not live realm or independent integration proof; parent owns that gate.

Census baseline reproduces176 exactly; after173. Script-owned E3 actions close162794/324536/207317, not every numeric field or optional talent. E3 admissible IDs/memberships121/129→118/126; a4=57/65, a226=14/15 unchanged. Remaining173:56 reviewed/partial/deferred,117 un-derived; class/spec scope unchanged13/40,630 book IDs/1551 memberships,302 core IDs/355 memberships. Source parser now recognizes qualified `batchN::NAME` registrations; the unchanged baseline still reproduces176. No no-op registrations.

Crash Lightning187874 was attempted then deferred in batch5; [batch8](#eighth-batch-projectile-taunt-and-low-coverage-paths) now supplies native104 cone and initial enhancement, with weapon-cleave/proc damage still uncertified. Durable before/after snapshots, ranked reasons, raw34-ID receipts, census generator and proof logs: `game-engine/data/diagnostics/spell-dummy5-2026-10-10/`. Delivery/remote/release receipts: `/home/osso/.worktrees/handoff-spell-dummy5.md`.

### Third batch: local Retail rows

Implementation: `spell_cast/scripts/batch3.rs`; [contract](../../specs/spell-dummy-scripts.md#third-batch-locally-derived-paths). Sources are read-only local Retail12.1.0.69933 `spell`, `spell_effect`, duration/radius/category/aura-options/PPM rows, not older expansion assumptions. Durable receipts/proofs: `game-engine/data/diagnostics/spell-dummy3-2026-10-10/`; pending proof status is tracked in `/home/osso/.worktrees/handoff-spell-dummy3.md`.

- 596 tooltip "your target and ${$s3-1} injured allies" supplies five total targets; effect1's radius23 supplies40yd. Spell-owned plan retains98367 marker trigger, caps additional injured allies, uses existing nearest-first area order. No claim that this order is a Mainline injury-priority formula.
- 703 conditional `a196861` names1330;196861 names256148. Regular Stealth1784 aura at OnHit gates these five-second children before normal proc interrupts run.256148 applies only after the caster's1330 actually lands:196861 says "Enemies silenced by Garrote"; a real Unending Resolve104773 effect3 silence-immunity cast fixture reproduces and prevents the incorrect debuff. Zero-valued703 aura4 stays unresolved; Improved Garrote window not inferred.
- 2050 `$s2`=6 seconds reduces its running category1673 recharge after2061 completes; existing timer mutation sends charge updates. Optional Prayer1246517 is not implemented.
- 2948 `$s2`=30 is a strict health threshold. Crit hook now receives the target before damage, keeping normal crit/mod calculation above the threshold and existing Fire Blast guarantee.
- 5221 `a231063`/`$231063s2` supplies25% bonus against bleeding targets. Applied aura spell/effect mechanic15 identifies a bleed, not merely any DoT. Other talent/stealth variants remain unsupported.
- 6343 names435203s1/d; damage-effect hit casts the20% ten-second snare per enemy.20243's chance is sourced from231834s1=30%, not its unused mirrored Dummy. Full timer reset sends the ordinary cooldown update; absent rank leaves it untouched.
- 8921 names164812s1/o2/d; base bridge launches only child effects0/1 (SP0.212 direct, SP0.184 per2000ms). It does not accidentally apply optional effect7's20% snare. Other Moonfire talent/resource fields remain unsupported.

Batch3 census (verified2026-10-10, code `c7a69c7`): Dummy-bearing core roots **180→177**; complete root-type closures2050/2948/8921. E3 admissible IDs/memberships **125/133→122/130**; a4 **57/65 unchanged**; a226 **14/15 unchanged**. D memberships remain8/4/5. Scope remains13 classes/40 specs,630 book IDs/1551 memberships,302 core probes/355 memberships. Five newly scripted roots retain unresolved fields; retained177 list has45 reviewed/partial/deferred and132 un-derived entries. Moonfire's child closure also exposes latent aura269; this is not hidden by the base-only effect mask.

Native proof:16 new cast tests. `dummy-integration.log` at `d80ba12` passes51; only its three Garrote tests were invalidated by the later silence-immunity fix, renewed and expanded to four passing Garrote tests at `c7a69c7`. Thus52 unique Dummy tests have surviving proof. Separate29 generic cast regressions survive the Garrote-only change; final format/check exit0, warning-free. Readability manually inspected; independent artifact verifier failed OAuth refresh, so no independent or PR-ready claim. No live realm/client proof. SQL opens mode=ro;21 actual IDs' descriptions and effects match raw local DB2 CSVs (`raw-db2-receipts.json`). No missing imports required for these bounded paths.


### Fourth-batch local Retail rows

`spell_cast/scripts/batch4.rs` adds ten bounded paths: eight ranked Dummy-root candidates and supplementary Vivify/Eruption behavior. Sources exclusively local Retail12.1.0.69933 descriptions/effects; no older-expansion scripts imported. [Contract and exclusions](../../specs/spell-dummy-scripts.md#fourth-batch-locally-derived-paths).

- 85222s2 caps healing at five injured allies after its normal radius selection. Current Mainline description says "radius around you", not an older cone; nearest-first ordering is retained, not invented injury ranking. Additional Dummy2 remains unresolved.
- 56641 references77443s1=20 Focus.53351s2 supplies the strict20% admission threshold before cost/cooldown. Steady cooldown/level normalization and Kill Shot talent overrides remain unimplemented.
- 33917/231064 gates its20% bonus on an applied bleed mechanic15.115151 names119611o1/d; only child effect0 is applied. Overheal travel and231606 tick procs remain unimplemented.
- 116670/274586 names425804, whose own132/31 area selection is narrowed to caster-owned119611 marks. Triggering one unrestricted child per mark incorrectly healed unmarked allies; one narrowed child plan fixes that boundary. Foreign/far/unmarked allies receive no splash.
- 188443 with343725 reads passive effect4=two Maelstrom per damage-hit target; rank absent produces none.49184 applies55095's base periodic effect to hit enemies only; CAP/secondary damage and Frost Fever resource procs remain separate gaps.
- 34861s3 supplies six seconds removed after596 completes; only running timers change.395160 gates Ebon Might extension on395153s1=1000ms, once after cast; existing finite owned395152/395296 expiries extend without creating buffs or changing tick clocks.

Batch4 census (verified2026-10-10, code`83891f4`): Dummy-bearing core roots **177→176**. Only Kill Shot53351 closes its entire Dummy-type gap; the other seven ranked paths retain fields. E3 admissible IDs/memberships **122/130→121/129**; a4 **57/65** and a226 **14/15** unchanged, with D memberships8/4/5 unchanged. Remaining176 roots:50 reviewed/partial/deferred,126 un-derived. Vivify/Eruption are supplementary non-Dummy-root paths, not census closures. New Howling Blast child closure exposes latent aura380/381 requirements; base mask does not conceal these.

Native proof:17 new functions.69 Dummy cast tests passed at`b5ca03e`; later ally-selection extraction invalidated only four Dawn/Vivify tests, renewed2+2 at`83891f4`. Final fmt/check exit0, warning-free.24 real IDs' descriptions/effects match local raw client DB2 CSVs. Native cast proofs, source snapshots, regeneration and census receipts: `game-engine/data/diagnostics/spell-dummy4-2026-10-10/`. Readability manually inspected; no independent/full-suite/live-realm proof claimed. Parent owns integration gate.

See [contract](../../specs/spell-dummy-scripts.md). Effect-hit scripts bridge Shield Block2565 to132404, Bloodthirst23881 to117313 and a chance-based184362, and Rampage184367 to184362. Damage/resource effects continue through normal dispatch. Bloodthirst reads modified effect2 base points, not ProcChance101 or PPM; local DB2 has no PPM entry for these casts.

Sources: TrinityCore `a352b1fa` `src/server/scripts/Spells/spell_warrior.cpp` (`spell_warr_shield_block`, `spell_warr_bloodthirst`, `spell_warr_bloodthirst_enrage`); Retail12.1.0.69933 `spell`/`spell_effect` rows2565,132404,23881,117313,184367,184362 in read-only `data/world.db`. Rampage's tooltip states "Enrages you"; its existing effect triggers name the attack spells. Fresh Meat and other optional talent variants remain unproved.

Holy Shock20473 branches on ally/enemy to25914/25912; parent ENERGIZE retains1 Holy Power. Sources: TC `a352b1fa` `spell_paladin.cpp:1261-1311`, `spell_pal_holy_shock`; Retail tooltip `$25912s1`/`$25914s1`, local SP coefficients2.16/11.17, no PPM row. Conditional talent272906 crit bonus is not covered.

Barkskin22812 PERIODIC_DUMMY effect2 casts213709 immediately (Attributes5 EXTRA_INITIAL_PERIOD), then on each1000ms tick while Brambles203953 is present; `auras/scripts.rs` registers this callback. TC `a352b1fa` `spell_druid.cpp:297-316` names the condition and child; Retail203953 tooltip supplies `$213709s1`/`$22812t3`, child AP coefficient0.075. No PPM roll: this is a timed conditional pulse. Brambles absorb/reflection semantics are outside this script.

Penance47540 DUMMY0 starts47757 healing or47758 damage channel with the shared normal channel-state constructor, then resolves the child without a second cost/GCD. The channel target is present before periodic aura application; target77 bolts select it. Source: TC `a352b1fa` `spell_priest.cpp:3070-3125,5546`; local child aura23 triggers47666/47750 at1000ms with extra-initial-period and2000ms duration (three bolts), SP0.932/4.011. No PPM/chance roll. Target marker aura4, Atonement and Power of the Dark Side talent semantics remain unproved.

Cobra Shot193455 reads effect2 (`$s3` =1 second) and reduces Kill Command34026 recharge after its damage hit. Retail12.1.0.69933 tooltip: "Reduces the cooldown of Kill Command by $s3 sec." Kill Command has ChargeCategory2067, so reducing only RecoveryTime500ms is incorrect: the script updates the running charge deadline and sends SpellChargesUpdate. No aura-options/PPM row. Tests use a real Kill Command cast before Cobra Shot. Dummy0 remains unresolved metadata; this does not certify all fields.

Blackout Kick100784 rank-two261917 reads effect2 (1000ms) after damage and reduces both Rising Sun Kick107428 charge recharge (category1518) and Fists of Fury113656 spell cooldown. Source: local Retail261917 description `$100784m3/1000`; parent100784 conditional `s261917`. No PPM row; ProcChance101 is metadata, not a roll. Other Dummy fields/Teachings variants remain unresolved.

Backstab53 damage-amount hook consumes Dummy3 ($s4=20%) only when the caster is in the target rear half-plane. The normal damage/crit/armor/log path stays shared. Source: pinned TC a352b1fa spell_rogue.cpp:225-251 spell_rog_backstab; local Retail53 AP1.0731 and effect3 BasePoints20. No aura-options/PPM. Dummy0 and optional Find Weakness remain unresolved.

Pyre357211 missile impact393568 triggers357212 at the hit position (TC a352b1fa spell_evoker.cpp:601-618 spell_evo_pyre). The spell script selects enemies using child357212 radius and cap and resolves through the existing hit/missile/effect pipeline. Script-owned destination87 is not added to generic target dispatch; no unit-target reinterpretation or caster-centered explosion. Retail tooltip `$357212s1`/`$357212A1`, SP4.4; no PPM. Optional Volatility369089 re-explosion remains unsupported.

Drain Soul198590 dynamically multiplies each damage tick by effect1 (100%) only below effect2 (20%) health; TC a352b1fa spell_warlock.cpp:823-850, local tooltip `$s2`/`$s3` plus `$o1`/`$d`/`$t1`, SP1.11375/1000ms/5000ms. Aura removal hooks now receive RemoveReason (including replacement), so only Death triggers205292/10 raw Soul Shards. Channel expiry/StopSpellCast produce no shard. No aura-options/PPM entries.

Fire Blast108853 crit override (TC a352b1fa spell_mage.cpp:804-815 spell_mage_fire_blast) returns100% independently of caster crit stats. Retail tooltip `${$s1*2}`/SP0.98625; no proc or PPM row. This is a core rotational behavior absent from the Dummy-only census, not a closed Dummy gap. Cast-while-casting and Hot Streak remain uncertified.

Death Coil47541 branches47632 damage/47633 healing (Retail tooltip `$47632s1`/`$47633s1`, AP2.11812/4.6375). TC a352b1fa spell_dk.cpp:589-609 only supplies the damage bridge; Retail tooltip defines the additional Undead healing and `$77575s3/1000` plague extension. Lichborne49039 says "become Undead"; friendly humanoids are rejected before costs, while NPCs use authoritative template type6. Existing caster-owned Dread1240996/Virulent191587 finite expirations extend1000ms without a periodic-timer reset; other casters’ auras are not extended. ProcChance101/PPM0 are not a random bridge. Optional Unholy Vigor not implemented.

Chain Heal1064 has a spell-specific target plan, not generic target45 admission. The explicit ally receives the first heal; up to three subsequent targets use greatest absolute missing HP within20yd of the current jump source, then30% falloff per jump. Sources: Retail1064 `$s2`=30/$s3=20, effect0 ChainTargets4/SP2.31/ChainAmplitude0.7; TC a352b1fa Spell.cpp:2216-2322 SearchChainTargets establishes missing-HP priority and advancing source. Old TC default12.5yd is superseded by the local tooltip20yd. Additional-jump LOS and optional talent236502 range increase remain uncertified. No proc/PPM roll.

Demon Spikes203720 applies203819 (TC a352b1fa spell_dh.cpp:1035-1055); effect0 is excluded without Deflecting Spikes321028 (same file:891-910). Retail tooltip `$203819s2*$AGI/100` =75% Agility armor, `$203819d`=12000ms, optional8-point parry modifier. Its child needed real aura268 support, not an inert aura marker: SpellAuraEffects.cpp:3951-3963 and StatSystem.cpp:255-276 use current primary stats. StatModifiers now carries per-stat armor percentages and MiscValue-2 primary-stat percentage; CharacterStats recomputes their armor contribution on stat changes. Calcified Spikes/Retaliation remain unsupported. No PPM roll.

Cooldown reductions send the original catalog/modded full cooldown duration alongside the reduced remaining time; otherwise the native client resets its sweep to100%. Fists of Fury cast -> completed channel -> Blackout Kick rank two test asserts both fields (source: native player_spells.rs CooldownTimer contract). Pyre plan selection and timer mutation are named helpers; behavior unchanged.

Cobra Shot193455 unconditional tooltip `$<mult>` resolves through SpellXDescriptionVariables to variable331: `(min(player_level-1,19)*12+112)/340`. The damage hook applies this real early-level curve (unity from20 onward), in addition to the cooldown script. Level10/19/20/80 casts exercise it; raw AP coefficients alone overstated low-level damage. No guessed curve or server DB edits.

Backstab53 unconditional `$<mult>` joins variable339: `(max(0,min(player_level-10,10))*11+260)/370`. Apply this level curve before the position bonus; level10/19/20/80 front/rear casts prove it. Variable615 confirms Chain Heal jumps =ChainTargets-1; variables385/84 contain optional or legacy Monk/DK branches and do not replace current explicit tooltip fields. Exact joins are retained in description-variables.json in batch2 evidence.
