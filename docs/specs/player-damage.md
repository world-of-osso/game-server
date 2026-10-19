# Player Damage

Players deal Retail damage: auto-attacks swing the equipped weapon with attack power, through the melee hit table and the target's armor, and spells scale with spell or attack power. The rules are TrinityCore master a352b1fa's, except the weapon-power coefficient (Retail 6, TrinityCore 3.5). Level scaling's health-ratio rule still applies on top ([level scaling](level-scaling.md)).

## What it must do

Auto-attack
- [x] Attack admission and hostile spell targeting honor authored UNIT_FLAG_NON_ATTACKABLE, NON_ATTACKABLE_2, NOT_SELECTABLE and IMMUNE_TO_PC. PC immunity blocks players/player-controlled sources, not independent creatures. Neutral unflagged creatures remain attackable; faction masks/lists are unchanged.
- [x] A player's swing rolls a whole number between trunc((weapon min + AP / 6 × weapon speed) × (1 + versatility)) and the same for weapon max (Player::CalculateMinMaxDamage, StatSystem.cpp:435-485; Unit::CalculateDamage, Unit.cpp:2490-2539). AP / 6 is SimC's `WEAPON_POWER_COEFFICIENT`; TrinityCore divides by 3.5 (StatSystem.cpp:455).
- [x] Without a weapon the main hand is unarmed: 1-2 at 2.0 s (`BASE_MINDAMAGE`/`BASE_MAXDAMAGE`).
- [x] The swing takes the aura physical damage done, auto-attack (MOD_AUTOATTACK_DAMAGE 344) and damage taken multipliers.
- [x] Physical school immunity prevents melee health loss and logs `Immune`; matching shields consume capacity before health loss, log `Absorb` on full absorption or damage plus absorbed amount on partial absorption, and disappear on depletion.
- [x] The main hand swings every weapon speed / (1 + haste) (Unit::ApplyAttackTimePercentMod, Unit.cpp:11032).
- [x] An equipped off-hand weapon swings on its own timer at half damage (UNIT_MOD_DAMAGE_OFFHAND TOTAL_PCT 0.5, Unit.cpp:350). A swing of either hand holds the other at least 200 ms away (ATTACK_DISPLAY_DELAY, Unit.cpp:2203-2230).
- [x] Swing rage uses the unhasted weapon speed, halved for the off hand (CalcMeleeAttackRageGain, Unit.cpp:2239-2249).
- [x] Units without `CharacterStats` (creatures) keep their ExpectedStat or flat 10 swing every 2 s.

Hit table (player and creature swings; one roll in 0..10000, Unit::RollMeleeOutcomeAgainst, Unit.cpp:2378-2488)
- [x] Miss 5% (Unit::GetUnitMissChance, Unit.cpp:2825-2830), +19% while an off-hand weapon is equipped (Unit.cpp:12509-12512).
- [x] Against a creature: dodge 3%, parry 6%, block 3%, each +1.5% per level the creature is above the player (Unit.cpp:2749-2861). A block stops 30% (Unit::GetBlockPercent, Unit.h:1000).
- [x] Victim native `flags_extra` NO_PARRY0x4 and NO_BLOCK0x10 remove only the corresponding autoattack outcome, before the unchanged single roll. Copy flags from the authoritative template at ordinary spawn, spell-summon, controlled-pet spawn and same-actor entry projection; no template lookup per swing or wire change. A manually constructed NPC missing the cached flags remains attackable with its prior unfiltered avoidance and reports one explicit error per entity. Player defense and existing order/RNG/clocks remain unchanged. Runtime IsTotem and other flags remain separate gaps; there is no NO_DODGE or SUNWELL flag in pinned CreatureData.h.
- [x] The victim's inclusive180° front arc, or actual IGNORE_HIT_DIRECTION aura288, permits the existing direction-sensitive defenses. Behind it, neither creatures nor players parry/block; players also cannot dodge, while creature dodge remains available. Existing cached masks, casting/CC exclusions, native RNG order and swing clocks remain authoritative. Facing uses the same rotation conversion/front-dot semantics as incoming spell defenses (TC a352b1fa Unit.cpp2404–2414).
- [x] Autoattack dodge adds the actual attacker's signed MOD_ENEMY_DODGE aura251 sum to the victim's base dodge before clamping at zero (TC a352b1fa Unit.cpp2389/2750–2782, exact2775). Recipient/foreign modifiers do not substitute for the attacker. Positive251 never restores dodge prohibited by casting/CC or behind-player direction; missing-player-stats behavior, other hit-table outcomes and native RNG/clocks remain unchanged. Player expertise and248 remain separate gaps.
- [x] NPC/non-player attacker autoattack expertise adds the actual signed aura240 sum divided by4 as a reduction to dodge and parry, after attacker251 for dodge and before final zero clamps (TC a352b1fa Unit.cpp2777–2782/2817–2822). Block remains unchanged. The same casting/CC, behind-direction and cached mask exclusions still win; preserve native rows, swing clocks/RNG and Guardian same-tick one-shot health commit. Player expertise/rating,248 and runtime IsTotem remain separate gaps.
- [x] Glancing blows only against creatures 4 or more levels above: 10% + 10% per level, for 10% less damage per level, at most 30% (Unit.cpp:2444-2454, 1474-1486).
- [x] Crit at the player's crit chance doubles the damage (Unit.cpp:1429-1436).
- [x] A creature swings with 5% miss less its MOD_HIT_CHANCE auras (Unit::MeleeSpellMissChance) and 5% crit (Unit::GetUnitCriticalChanceDone, Unit.cpp:2885-2890); a player without `CharacterStats` (no class stat data) always hits.
- [x] Against a player the victim's own avoidance applies (TrinityCore master, StatSystem.cpp:490-715): dodge is MOD_DODGE_PERCENT 49 plus dodge rating after the class diminishing return (`GetDodgeFromAgility` is a stub, Player.cpp:5031); parry is 5% plus parry rating after diminishing returns, for a class with a parry cap that knows a `SPELL_EFFECT_PARRY` spell and holds a weapon; block is 5% plus block rating for one that knows a `SPELL_EFFECT_BLOCK` spell and holds a shield. A player's block stops `ShieldBlock` (shield armor × 2.5) / (`ShieldBlock` + the attacker's level ArmorConstant), at most 85% (Player::GetBlockPercent, Player.cpp:26838-26847).
- [x] A victim that is casting, stunned or confused cannot dodge, parry or block (Unit.cpp:2408-2413).
- [x] Crits reach clients as `CombatEventType::CriticalHit` and blocks as `CombatEventType::Block` with the damage that got through (`HITINFO_CRITICALHIT` / `HITINFO_BLOCK`); a glancing blow is `MeleeDamage`.
- [x] Misses, dodges and parries do no damage, are logged as `CombatLogKind::Miss(Miss | Dodge | Parry)` and `CombatEventType::Miss | Dodge | Parry`, and raise no procs (proc HitMask defaults to normal and critical hits).

Armor
- [x] Every creature spawns with `CreatureArmor` = ExpectedStat CreatureArmor at its own level × ArmorModifier (Creature::GetBaseArmorForLevel, Creature.cpp:3208-3214). A tuned creature's armor against an attacker is its base armor at its level for that attacker (GetArmorMultiplierForTarget, Creature.cpp:3216-3224).
- [x] Physical damage is × (1 − min(armor / (armor + ArmorConstant), 85%)), truncated, where ArmorConstant is the ExpansionID -2 ExpectedStat row at the attacker's level against the target (Unit::CalcArmorReducedDamage, Unit.cpp:1689-1777). A player target uses its gear armor.
- [x] Armor applies to every swing, and to spell damage with a physical school unless the spell has a bleed mechanic (Unit::IsDamageReducedByArmor, Unit.cpp:1680-1687; SpellMgr.cpp:3036-3038).
- [x] The level-scaling damage multiplier and health ratio apply after armor and the outcome (Unit::DealDamageMods, Unit::DealDamage).

Spells
- [x] Source effect165 DAMAGE_FROM_MAX_HEALTH_PCT selects and hits its actual unit target, dealing the integer-truncated authored percent of that target's maximum health, not its current health or caster power. Unlike school damage, omit SpellDamageBonusDone/Taken while retaining common crit, armor, resilience, absorbs, death and hit/achievement boundaries. Preserve sub-one damage truncation to zero; never invent a minimum of one. Native BB gun67531 supplies authored0.01percent; acquisition/equipping and world-leader content remain separate.
- [x] Spell power is Intellect when the main stat is Intellect, else 0 (Unit::SpellBaseDamageBonusDone, Unit.cpp:7113-7119). The main stat is the spec's, else the class's `chr_classes.PrimaryStatPriority` (Player::GetPrimaryStat, StatSystem.cpp:297-314).
- [x] Healing power (heals, periodic heals, absorbs) is Intellect for every mana class, so a Strength paladin's Flash of Light heals 3.156 × Intellect (Unit::SpellBaseHealingBonusDone, Unit.cpp:7622-7624).
- [x] OVERRIDE_SPELL_POWER_BY_AP_PCT (366, e.g. Retribution 1258011 96%, Protection 137028 101%) replaces both with int32(AP × summed pct + 0.5) (Unit.cpp:7103-7108, 7602-7607).
- [x] School damage and heals are BasePoints + BonusCoefficient × SP + BonusCoefficientFromAP × AP, then × (1 + a uniform roll in ± Variance / 2) (Spell::EffectSchoolDMG, SpellEffects.cpp:565-567; SpellEffectInfo::CalcValue, SpellInfo.cpp:560-567).
- [x] The caster's versatility raises spell damage and healing (Unit::SpellDamagePctDone, Unit.cpp:6944-6946).
- [x] A spell crit doubles the damage (Unit::SpellCriticalDamageBonus, Unit.cpp:7297-7325), as a melee crit does.
- [x] TRIGGER_MISSILE (32) triggers its spell at the unit target (Spell::EffectTriggerMissileSpell, SpellEffects.cpp:725-778). Frostbolt 116 deals its damage through 228597.

Examples
- A level 10 Human Warrior without a spec (Str 33 = AP 33, 5% crit) with the Worn Shortsword (0-1 at 2.60) swings 14-15. Against a level 10 Hogger (armor 90, ArmorConstant 210: 30%) that is 9-10, 104-115 of Hogger's 13,137 pool. With the hit table the average is 98.8 of the pool a swing: about 133 swings, 5.8 minutes.
- A level 10 Human Mage (Int 38 = SP 38) casts Frostbolt for 2.448 × 38 = 93 ± 2.5%, 186 ± 2.5% on a crit.
- Live: a level 10 Arms Warrior with the Worn Shortsword killed a level 10 Mangy Wolf in 10 swings (7 hits of 60-64, 3 dodges) in 24 s. Arms Warrior 137049 gives auto-attacks +476%; Plate Specialization 86101 gives +5% Strength (35 AP); 15-16 × 5.76 = 86-92, less 30% armor (`game-engine/data/diagnostics/playerdamage-20260926/`).

## How it works

- [Player damage](../wiki/systems/player-damage.md) — swing pipeline, armor, spell amounts
- [Combat](../wiki/systems/combat.md) — auto-attack systems
- [Character stats](../wiki/systems/character-stats.md) — attack power, spell power, crit, haste, versatility, weapons

## Implementation inventory

- `crates/server/src/melee.rs` — weapon damage range and roll, swing time, hit-table chances and roll, outcome damage, armor reduction
- `crates/server/src/combat.rs` — `Attacker`, `try_swing`, `armor_mitigated`, `AutoAttack.off_hand_timer`, swing log and events
- `crates/server/src/level_scaling.rs` — `CreatureArmor`, `ArmorConstants`, `LevelStats::armor`, `UnitScaling::armor_against`
- `crates/server/src/world_data.rs` — ExpectedStat CreatureArmor/ArmorConstant and ArmorModifier loading
- `crates/server/src/zones.rs` — `CreatureArmor` at spawn
- `crates/server/src/stats.rs` — spell power rule, class main-stat fallback, `CharacterStats.off_hand`
- `crates/server/src/spell_cast/effects.rs` — TRIGGER_MISSILE, variance, versatility, armor on physical spells

## Tests asserting this spec

- `crates/server/src/melee_tests.rs` — weapon damage, off hand, roll, haste, hit-table chances and order (player and creature swingers, a victim that cannot avoid), outcomes, a player's shield block share, armor.
- `crates/server/src/combat_npc_expertise_tests.rs` — real Evasion versus native NPC62627, cancellation/foreign controls,512-swing native NPC dodge/parry/block, and runtime-qualified native28305/14 partial reduction/removal. NPC-to-NPC receipts use actual all-client CombatEvent broadcasts, not fake player ownership.
- `crates/server/src/combat_enemy_dodge_tests.rs` — native attacker251 negative/removal, positive/removal, signed sum, recipient ownership, behind-player and stunned-player exclusions through ordinary autoattack events.
- `crates/server/src/combat_tests/outcomes.rs` — a crit is sent as `CriticalHit`; a creature's swing meets a player's block (`Block`) and parry (`Parry`); a casting player cannot parry.
- `crates/server/src/stats_tests.rs` `player_avoidance_follows_trinitycore_master` — parry, block, dodge rating diminishing returns.
- `crates/server/src/combat_tests/player_damage.rs` — a swing through Hogger's armor, the weapon-speed/haste timer, full crit chance, the off-hand timer and damage, a level 10 warrior killing Hogger with the Worn Shortsword (real item data).
- `crates/server/src/level_scaling_tests.rs` — tuned armor at the attacker's level, untuned armor and ArmorConstants.
- `crates/server/src/spell_cast/tests.rs` — `level_10_mage_frostbolt_deals_spell_power_times_its_coefficient`, `a_spell_crit_doubles_the_damage`, `physical_spell_damage_takes_the_creatures_armor`.
- `crates/server/src/stats_tests.rs` — no spell power for a Strength warrior or a spec-less paladin.
- `crates/server/src/zones_world_db_tests.rs` (real world.db, `--ignored`): `spawned_creatures_carry_expected_stat_armor`.

## Known gaps (current cycle)

- [ ] Facing is not modelled: every swing lands in front, so creatures parry and block from behind.
- [x] A glancing swing sets `CombatLogEvent.glancing` (`HITINFO_GLANCING`, Retail CLEU SWING_DAMAGE `glancing`). Patch 6.0.2 kept glancing blows only against creatures 4 or more levels above, as TrinityCore does.
- [x] Every `CombatLogEvent` carries the server wall clock when `log_event` sent it, `timestamp_unix_ms` (CLEU `timestamp`).
- [x] An interrupt names the interrupted cast in `extra_spell_id` (`SMSG_SPELL_INTERRUPT_LOG.InterruptedSpellID`, CLEU SPELL_INTERRUPT `extraSpellId`); a dispel's `spell_id` is the dispelling spell and `extra_spell_id` the removed aura, one line per aura (`SMSG_SPELL_DISPELL_LOG`, SPELL_DISPEL). Spell steal: no effect source yet.
- [x] Environmental damage logs `CombatLogKind::Environmental` (ENVIRONMENTAL_DAMAGE, `Player::EnvironmentalDamage`): landing fall damage and falling below the map (`DAMAGE_FALL_TO_VOID`, logged as Falling) in `networking.rs`, drowning and fatigue in `mirror_timers.rs`; no source, Physical, overkill in `overflow`. Fire, lava and slime are not dealt. Tests: `networking_tests/death.rs` `a_survived_fall_logs_falling_environmental_damage_to_the_owner`, `spell_cast/tests.rs` `kick_interrupts_target_cast_and_locks_its_school`, `auras/tests.rs` `purify_dispels_a_magic_debuff`.
- [ ] Crushing blows are not modelled or carried: whether Retail 12.x still produces them is unverified (3.0.2 moved them to creatures 4+ levels above), and TrinityCore master's `attackerLevel - victimLevel * 1000 - 1500` is always negative (Unit.cpp:2475).
- [ ] Avoidance auras (MOD_DODGE/PARRY/BLOCK_PERCENT), creature crit auras, the attacker's MOD_ATTACKER_MELEE_CRIT_CHANCE on the victim, sitting victims being crit, evading victims (`MELEE_HIT_EVADE`), `CREATURE_FLAG_EXTRA_NO_PARRY/NO_BLOCK/NO_CRIT` and totems are not modelled.
- [ ] Block 107 is on the general Defense skill line 95, which class progression does not learn, so players do not block yet; Parry is on class skill lines (Warrior 3127, Death Knight 82246).
- [ ] Hit rating and expertise (TrinityCore `m_modMeleeHitChance`, GetExpertiseDodgeOrParryReduction), MOD_CRIT_DAMAGE_BONUS and armor penetration are not modelled.
- [ ] Ranged auto-shot does not exist on the server; ranged weapons do not swing.
- [ ] Dual wield is not tracked: the inventory refuses one-handers in the off hand, so only off-hand-only weapons swing there.
- [ ] Proc hit masks are not modelled: avoided swings raise no procs, so dodge- or parry-triggered procs never fire.
- [ ] Spell hit and resist rolls, and the ArmorItemLevelDiminishing curve at the maximum level, are not modelled.
- [ ] SpellEffect `Coefficient` with a `ScalingClass` (SpellScaling game table base points) is not modelled; those effects use `EffectBasePointsF`.
- [ ] The Worn Shortsword rolls 0-1 (item level 1). Retail's tooltip was not checked.
- [ ] Plate Specialization (86101) applies without plate armor (aura runtime: armor specialization conditions are not checked).

## Out of scope

- Talents' passive damage mods beyond what the aura runtime already applies, mastery beyond its stat, PvP scaling and set bonuses.
