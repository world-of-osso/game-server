# Player Damage

How a player's auto-attacks and spells turn stats into damage, in the order TrinityCore master a352b1fa applies each step. The requirements and tests are in the [player damage spec](../../specs/player-damage.md).

## Auto-attack pipeline

`combat.rs` `auto_attack_system` ticks both hands of every attacker's `AutoAttack` (`timer`, `off_hand_timer`) and calls `try_swing` for the main hand, then the off hand. An `Attacker` with `CharacterStats` is a player; without it (creatures) the swing keeps the creature rules of [[level-scaling]].

1. **Weapon:** `CharacterStats.main_hand` (unarmed 1-2 at 2.0 s) or `off_hand` (none: no off-hand swing). Both come from `EquippedItems` through `item_stats.rs` ([[character-stats]]).
2. **Roll:** `melee::weapon_damage_range` = (min or max + AP / 6 × speed) × (1 + versatility), × 0.5 for the off hand. `melee::roll_weapon_damage` picks a whole number in the truncated bounds (Unit::CalculateDamage `urand`).
3. **Done/taken:** × aura physical damage done × MOD_AUTOATTACK_DAMAGE × target physical damage taken (`AuraStatMods`), rounded.
4. **Armor:** `combat::armor_mitigated` against `target_armor`: a creature's `CreatureArmor` through `UnitScaling::armor_against` (its ExpectedStat armor at its level for the attacker), a player's `CharacterStats.armor`. The constant is `ArmorConstants.at(attacker level against the target)`. Without the `ArmorConstants` resource (unit tests) there is no mitigation.
5. **Outcome:** `melee::player_swing_chances` + `roll_melee_outcome` (one roll in 0..10000, chances truncated to hundredths), then `outcome_damage`: avoidance 0, crit × 2, glancing −10%/level, block −30% (the blocked amount goes to the combat log).
6. **Level scaling:** `damage_dealt` (a tuned creature attacker's multiplier) and `health_loss` (a tuned creature target's health ratio), unchanged from [[level-scaling]].
7. **Timers:** the swung hand restarts at weapon speed / (1 + haste); the other hand is pushed to at least 0.2 s.

Creatures skip steps 1, 2 and 5: their base is the ExpectedStat roll (tuned) or 10, every 2 s, and they always hit. Step 4 applies to them too, so a player's gear armor mitigates creature swings.

Avoided swings emit `CombatEventType::Miss | Dodge | Parry` and `CombatLogKind::Miss(..)` with amount 0, add 0 threat, still give swing rage and raise no proc events.

## Hit table values

| | Chance | Source |
|---|---|---|
| Miss | 5%, +19% with an off-hand weapon | Unit::GetUnitMissChance, MeleeSpellMissChance |
| Dodge (creature) | 3% + 1.5%/level above | GetUnitDodgeChance |
| Parry (creature) | 6% + 1.5%/level above | GetUnitParryChance |
| Glancing (creature 4+ levels above) | 10% + 10%/level | RollMeleeOutcomeAgainst |
| Block (creature) | 3% + 1.5%/level above, stops 30% | GetUnitBlockChance, GetBlockPercent |
| Crit | `CharacterStats.crit_pct` | GetUnitCriticalChanceDone |

TrinityCore's creature values differ from SimC's Retail target (3% miss, 3% dodge, 3% parry at equal level). The server follows TrinityCore; the difference is unverified against Retail.

## Armor

- `world_data.rs` loads ExpectedStat `CreatureArmor` and `ArmorConstant` with their ExpectedStatMod columns, and `ArmorModifier` per template.
- `LevelStats::armor` holds a tuned template's armor per level; an untuned template's `CreatureTemplate.armor` is ExpectedStat armor at `min_level` (ContentTuning 0, expansion 0 falling back to -2).
- `zones.rs` inserts `CreatureArmor` (armor at the spawn's own level) on every spawn.
- `ArmorConstants` (a resource, index = level) is the ExpansionID -2 ArmorConstant row with no class mod, what TrinityCore passes for a player attacker. Every ExpectedStatMod class row has ArmorConstantMod 1, so creature attackers get the same value.

Level 10: CreatureArmor 90, ArmorConstant 210 (30%). Level 30: 155 and 361.

## Spells

`spell_cast/effects.rs`:
- `scaled_amount`: BasePoints + BonusCoefficient × SP + BonusCoefficientFromAP × AP (unchanged).
- `with_variance` on SCHOOL_DAMAGE and HEAL: × (1 ± Variance / 2). Most class spells have Variance 0.05. Test helper: `test_harness::varied`.
- `with_crit`: + 100% (× 2), with CritDamageAndHealing mods (unchanged).
- `deal_damage`: × damage done × versatility × damage taken, then armor if `reduced_by_armor` (physical school, no bleed mechanic 15), then absorbs and level scaling. `apply_heal` takes versatility too.
- TRIGGER_MISSILE (32) shares TRIGGER_SPELL's handler: Frostbolt 116 → 228597 (2.448 × SP).

Spell power is Intellect only for an Intellect main stat (`stats.rs`); warriors, rogues and other Strength or Agility specs have 0.

## Maximum-health percent effect165

TC a352b1fa SpellEffects5165–5173 adds `CountPctFromMaxHealth(effectValue)` directly to spell damage; Unit.h809 returns an integer. Native effect165 now selects real hit units and truncates authored percent of their maximum health before common damage resolution. It does not use current health or add SP/AP, direct damage-done or SpellDamageBonusTaken modifiers. Final crit, armor, resilience, absorbs and existing death/hit boundaries remain shared. `damage_resilience_multiplier` extracts the existing final versatility factor without changing ordinary damage-taken semantics.

Real BB gun67531 carries0.01percent and only effect165. Native RED9e862790 accepts its cast on authored16802 but leaves50000/100000 health unchanged instead of49990. Native GREEN4091b81b passes2/2(18.30s), including actual damage10, native Enrage26662/Barkskin22812 bonus exclusion and5000max sub-point truncation; zero damage still produces an actual hit and TargetCreatureId4 credit. Both-faction leader grants and durable re-entry are proved separately in [achievements](dungeon-achievements.md#pre-effect-spell-hit-target-creature-entries). Required workspace achievement/criteria/statistic/instance filter178/0/1ignored plus site-export1/1 and server/engine checks/three-repo fmt pass at source4091b81b; later docs-only commits preserve proof. Acquisition/equipping, holiday availability and world-leader content are not certified.

## Numbers

| Case | Working | Result |
|---|---|---|
| L10 Human Warrior, no spec, Worn Shortsword (0-1, 2.60) vs L10 Hogger | trunc(0-1 + 33 / 6 × 2.6) = 14-15; × 0.7 = 9-10; × 13,137 / 1,133 = 104-115 of the pool | ~133 swings (unit test: 110-160) |
| L10 Arms Warrior, same sword, vs L10 Mangy Wolf (live) | Str 33 × 1.05 (Plate Specialization) = AP 34.65 → 15-16; × 5.76 (Arms Warrior +476% auto-attack) = 86-92; × 0.7 = 60-64 | 7 hits + 3 dodges, dead in 24 s |
| L10 Human Mage Frostbolt | 2.448 × Int 38 = 93 ± 2.5% | 91-95, 181-191 on a crit |

Live proof: `game-engine/data/diagnostics/playerdamage-20260926/` (server and client combat logs, kill screenshot).

## Sources

- [melee.rs](../../../crates/server/src/melee.rs) — weapon damage, hit table, outcome, armor formula
- [combat.rs](../../../crates/server/src/combat.rs) — swing pipeline, timers, events
- [level_scaling.rs](../../../crates/server/src/level_scaling.rs) — `CreatureArmor`, `ArmorConstants`, `armor_against`
- [effects.rs](../../../crates/server/src/spell_cast/effects.rs) — spell amounts, variance, versatility, armor, TRIGGER_MISSILE
- TrinityCore master a352b1fa: `Entities/Unit/Unit.cpp` (CalculateMeleeDamage, RollMeleeOutcomeAgainst, CalcArmorReducedDamage, SpellBaseDamageBonusDone, SpellCriticalDamageBonus), `Entities/Unit/StatSystem.cpp` (Player::CalculateMinMaxDamage, UpdateAttackPowerAndDamage, GetPrimaryStat), `Spells/SpellEffects.cpp`, `Spells/SpellInfo.cpp`
- SimC `WEAPON_POWER_COEFFICIENT` (6) for AP per weapon DPS

## See Also

- [[combat]] — auto-attack systems and death transitions
- [[character-stats]] — AP, SP, crit, haste, versatility, weapons
- [[level-scaling]] — damage and health multipliers after armor
- [[spell-casting]] — effect handlers
- [[auras]] — `AuraStatMods` (MOD_AUTOATTACK_DAMAGE, damage done/taken)


### Shared weapon-spell defense roll

Weapon-spell miss, ranged deflection and block consume one native integer roll and cumulative truncated hundredths. Magic behavior remains separate. Existing live player DodgePercentage now feeds melee spell outcomes before block; source frontal/control/attribute guards and lifecycle have six native Evasion controls. Non-player caster aura240 now subtracts its native active total/4, clamped at zero. Source62627/38598 is authenticated; receiver-corrected ae251a10 records actual Strike `Miss(Dodge)` despite caster expertise10000. Earlier unrelated-connection results are invalid feature REDs. Corrected application/removal/foreign ownership controls and merged31 block/dodge/table/cw150 controls pass; current workspace/all-target check and fmt pass. Independent acceptance remains pending. Physical aura117 must add the victim's nonnegative maximum selected-mechanic total after miss and before NoActiveDefense. Effect mechanic overrides category; signed applied amounts/stacks sum only within a selected mechanic. Initial cd651cb7 full-table RED is invalid: neutral NPC6 omitted guarded baseline5, yielding85+10 rather than100. Native hostile Kobold40 replaces it; corrected2c4a7ecb records actual stun atdraw10 with verified hostility/miss90/live11710. Implementation follows this genuineRED; c31715af native3/3 GREEN proves256 miss/resist outcomes without stun, removal, foreign recipient and mismatched silence selector. Source-identical27 prior block/dodge/table controls retained; independent acceptance pending. Recipient/removal/silence-selector controls bound this proof, not full117, magic or ability certification. [Source and receipts](../../class-completeness6.md#physical-mechanic-resistance--native-slice-2026-10-14). Caster251 now adds its signed active total before expertise and final clamp at the existing player melee-dodge boundary. Native48250e2+20/110294e0−25 and actual hostile40/38598 give genuine damage RED at24fb3cb5, GREEN pending; recipient effects do not contribute. [Exact scope and proof](../../class-completeness6.md#signed-attacker-enemy-dodge-modifier--native-slice-2026-10-14). NPC dodge/parry, player expertise/248 modifiers and player hit baseline remain unimplemented boundaries. Two actual native cast invariants and quantization controls pass alongside retained block controls; 45 adjacent controls, workspace/all-target check and fmt also pass; independent acceptance remains pending. [Exact source and proof](../../class-completeness6.md#shared-weapon-spell-roll-boundary--batch59-in-progress).

### Native player critical/block hit state

Autoswings consume integer aura253 chance on the defender and double the already-truncated blocked amount. Player weapon-spell hit plans retain sparse per-unit block results through flight; the existing innermost matching `ResolvingHits` frame supplies that result before absorption. Critical spell blocks double the integer shield percentage before truncation/full-block cap, not the amount after truncation. Chance is read at impact; facing/block hit result belongs to launch. Source: TC a352b1fa Unit.cpp2564–2570,1469–1473,2610–2741,1243–1257.

This player slice does not certify NPC block flags/baseline/totems or full hit-table ordering. Ordinary NPC weapon-spell blocks consume source3% plus aura51 and positive effective-level bonus; authenticated NO_BLOCK0x10 excludes blocking, unlike NO_XP0x40. Native blocked damage is30%, doubled to60% for critical blocks. Four NPC controls plus eleven player controls and45 adjacent controls pass; workspace/all-target check and fmt pass. Independent acceptance remains pending. Source Totem/Lightwell runtime is still unsupported, not inferred from template type. Ordinary NPC implementation and its bounded proof are tracked separately in [the NPC boundary ledger](../../class-completeness6.md#ordinary-npc-spell-block-boundary--batch58-in-progress), including source-defined Totem/Lightwell identity and the unsupported summon-runtime boundary. Aura638 ownership conflicts between pinnedCPP and MartialExpert description; generic253/186 certificates remain absent. Exact source and proof ledger: [class completeness](../../class-completeness6.md).
