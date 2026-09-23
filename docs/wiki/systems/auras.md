# Auras

Server aura runtime in `crates/server/src/auras/`. `AuraPlugin` keeps every unit's auras in `ActiveAuras`. It handles stacking and refresh, periodic ticks, procs, spell modifiers, stat modifiers, expiry and removal. The replicated view is `UnitAuras`, which changes only when the aura set changes. [[spell-casting]] resolves aura effects through `auras::apply_aura` and reads spell modifiers when it computes cost, cast time, cooldown, GCD, range and amounts. The data comes from [[spell-info]].

## Application

- **Aura effects:** SpellEffects 6 (APPLY_AURA), 35 (party area aura) and 65 (raid area aura). There is no group backend, so area auras reach only their resolved targets (always the caster in scope).
- **One aura per target:** `resolve_effects` collects the aura effects that hit the same unit in one resolution into an effect mask, then calls `apply_aura` once for that unit. Shadow Word: Pain 589 effects 1 (aura 3) and 3 (aura 269) form one instance on the target.
- **Amounts:** each effect's per-stack amount is set at application: `BasePoints` with Points and PointsIndexN mods, plus SP/AP coefficients (BonusCoefficient mods) for damage, heal and absorb types. The amount does not change after that. Ticks and modifiers use amount × stacks.
- **Duration:** `spell_duration` with Duration mods. A duration of 0 or -1 makes the aura permanent, and passives are always permanent.
- **Stacks and charges:** max stacks = `CumulativeAura` with MaxAuraStacks mods; 0 means the aura does not stack. Charges = `ProcCharges` with ProcCharges mods.
- **Dead targets:** get no auras unless the spell has `SPELL_ATTR3_ALLOW_AURA_WHILE_DEAD`.
- **Unhandled aura types:** still applied, tracked and replicated. A warning is logged once per (spell, aura type).

### Stacking rule

This is a simplified TrinityCore `Aura::CanStackWith`:

| Case | Result |
|---|---|
| Same spell, same caster, same source | Refresh: new duration, +1 stack up to the maximum, amounts and absorbs reset, tick phase kept. `SPELL_ATTR1_AURA_UNIQUE` auras are not refreshed. |
| Same spell, other caster | The auras coexist if the aura is harmful, has a periodic effect or has `SPELL_ATTR3_DOT_STACKING_RULE`. Otherwise the new aura replaces the old one. |

### Classification

- **Harmful:** `SPELL_ATTR0_AURA_IS_DEBUFF`, or a hostile caster when the spell lacks `SPELL_ATTR4_AURA_IS_BUFF`. Hostility follows the cast pipeline rule: player vs NPC.
- **Hidden:** passive (`SPELL_ATTR0` 0x40), or `SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG` (0x80).
- **From player:** the caster has `Player` at application.
- **Dispel type:** `spell_categories.DispelType`.

## Passives

- Passive spells with an aura effect in the unit's `KnownSpells` are applied as permanent self-cast auras (`AuraSource::KnownSpell`).
- When a spell leaves `KnownSpells`, its passive aura is removed (`Unlearned`).
- A `KnownSpells` change is queued in `Update` and synced in the next frame's `FixedUpdate`.
- Talent passives: `compute_known_spells` adds `TraitLoadout::active_talent_spells()` to `KnownSpells`, so passive talents apply and unapply through the same sync.

## Removal

| Reason | Trigger |
|---|---|
| Expired | `expires_at` passed (after that frame's ticks) |
| Death | the unit's health reached 0: every non-passive aura without `ALLOW_AURA_WHILE_DEAD` is removed |
| Cancelled | `CancelAura{spell_id}` from the owner: removes the owner's non-harmful, non-passive auras of that spell, unless the spell has `SPELL_ATTR0_NO_AURA_CANCEL` |
| Dispelled | effect 38 DISPEL (below) |
| Interrupted | aura interrupt flags (below) |
| ChannelEnded | a channel completed or was cancelled: that caster's auras of the channel spell are removed from every unit |
| Consumed | the last charge was spent, or the absorb was depleted |
| Unlearned | the passive left `KnownSpells` |
| CasterGone | the caster entity despawned, since ticks need a source |

Every removal logs `CombatLogEvent{AuraRemoved}`.

### Dispel (effect 38)

- Removes up to `BasePoints` auras (at least 1) whose dispel type is `EffectMiscValue_0`. Type 7 matches every dispel type.
- On a hostile target it removes buffs; on a friendly target, debuffs. Passives are never dispelled.
- Each removed aura logs `Dispel`, with source = the dispeller and spell = the removed aura's spell, plus `AuraRemoved`.
- Example: Purify 527 removes Shadow Word: Pain (DispelType 1).

### Interrupt flags

`SpellInterrupts.AuraInterruptFlags_0/_1` are implemented where a matching system exists. Counts are in-scope aura spells carrying the flag.

| Flag | Bit | Raised by | In-scope spells |
|---|---|---|---:|
| HostileActionReceived | 0 0x1 | taken melee swing, or taken harmful spell, ability or periodic | 3 |
| Damage | 0 0x2 | any damage taken above 0 after absorbs | 7 |
| Action | 0 0x4 | the unit successfully cast a spell | 8 |
| Moving | 0 0x8 | `Moving(true)` | 3 |
| Attacking | 0 0x1000 | the unit swung in melee | 11 |
| NonPeriodicDamage | 0 0x1000000 | non-periodic damage taken | 0 |
| EnteringCombat | 0 0x10000000 | `CombatStatus` false → true | 1 |
| LeavingCombat | 0 0x80000000 | `CombatStatus` true → false | 0 |
| ChangeSpec | 1 0x40 | `ActiveSpec` changed | 131 |

- Event interrupts skip auras of the spell that raised the event, so a spell's own hit does not remove the aura it just applied.
- Not implemented (no system): LeaveWorld 0x80000 (97 spells; despawn removes the auras anyway), EnterWorld 0x400000 (56), DamageCancelsScript 0x8000000 (42, NYI in TrinityCore too), Interacting, Mount, ItemUse, Login, Looting, Turning, Anim, Summon, and the flags-2 raid/instance/talent/glyph events. ChangeTalent (flags 2 0x4000, 82 spells) waits for the trait system.

## Periodic effects

- Each periodic effect ticks at `applied + k × period` while the tick time is at or before expiry. A 16 s aura with a 2 s period ticks 8 times.
- Period = `EffectAuraPeriod` with Period mods.
- The aura caster is the tick source. Tick events are not triggered, so they can proc.

| Aura | Tick |
|---|---|
| 3 PERIODIC_DAMAGE | `deal_damage(periodic)` |
| 89 PERIODIC_DAMAGE_PERCENT | amount% of the target's max health as damage |
| 8 PERIODIC_HEAL | `apply_heal(periodic)` |
| 20 OBS_MOD_HEALTH | amount% of max health as a heal |
| 24 PERIODIC_ENERGIZE | `EffectMiscValue_0` power, raw units |
| 23 PERIODIC_TRIGGER_SPELL | casts `EffectTriggerSpell` from the caster at the aura's unit, as a triggered spell (depth 1) |
| 226 PERIODIC_DUMMY | `AuraScripts` `on_periodic` |

Examples: Shadow Word: Pain 589 ticks 12 (0.1246 × SP 100) eight times. Renew 139 heals 61 five times.

## Damage and healing

`spell_cast::effects::deal_damage` applies these steps in order:

1. SCHOOL_IMMUNITY 39: the target takes nothing and `Miss(Immune)` is logged.
2. HealingAndDamage mods (PeriodicHealingAndDamage for ticks).
3. Caster `damage_done` (MOD_DAMAGE_PERCENT_DONE 79).
4. Target `damage_taken` (87) × MOD_SPELL_DAMAGE_FROM_CASTER 271 auras from this attacker.
5. SCHOOL_ABSORB 69: matching absorbs (MiscValue school mask) soak the damage, oldest first. A depleted absorb removes its aura. `absorbed` is logged.

Healing takes the same mods, then MOD_HEALING_DONE_PERCENT 136 and the target's healing taken (118, 283).

Crits:
- Chance = `CharacterStats.crit_pct` (base, rating and MOD_CRIT_PCT auras) + CritChance mods.
- A crit adds 100% of the amount, scaled by CritDamageAndHealing mods.

Melee auto-attack (`combat.rs`) takes the attacker's physical `damage_done` × `autoattack_damage_mult` (344) and the target's physical `damage_taken`. It does not apply absorbs or immunity: the swing path is a query system outside the World-based damage path.

## Procs

Emitters push `AuraEvent{actor, target, actor_mask, target_mask, spell, can_proc, periodic}` into `PendingAuraEvents`. `process_aura_events` drains the queue in `FixedUpdate`. Masks use TrinityCore `ProcFlags`/`ProcFlags2`.

| Event | Actor | Target |
|---|---|---|
| Melee swing (`auto_attack_system`) | DEAL_MELEE_SWING + MAIN_HAND_WEAPON_SWING | TAKE_MELEE_SWING + TAKE_ANY_DAMAGE |
| Spell lands (one per distinct target) | by `DefenseType`: MELEE_ABILITY / RANGED_ABILITY / HARMFUL or HELPFUL_SPELL (magic) / HARMFUL or HELPFUL_ABILITY (none) | the TAKE counterpart |
| Damage taken above 0 | — | TAKE_ANY_DAMAGE |
| Periodic damage / heal | DEAL_HARMFUL / DEAL_HELPFUL_PERIODIC | TAKE_HARMFUL (+ TAKE_ANY_DAMAGE) / TAKE_HELPFUL_PERIODIC |
| Kill | KILL | DEATH |
| Cast success | flags 2 CAST_SUCCESSFUL | — |

An aura procs when all of these hold (autogenerated TrinityCore proc entries):
- its `ProcTypeMask` intersects the event side's mask;
- it has a trigger-type effect (`handlers::can_proc`: 4, 226, 42, 231, 12, 26, 455, 69, 79, 87, 271, 110, 138);
- for spell events, when the trigger effects' `SpellClassMask` union is non-zero, the event spell has the aura's `SpellClassSet` and an overlapping mask;
- the event is not from a triggered spell (resolution depth > 0);
- `ProcCategoryRecovery` has elapsed;
- the roll passes. Chance is RPPM (`BaseProcRate`, TrinityCore `CalcPPMProcChance` with the 10 s attempt cap and bad-luck factor) when present, else `ProcChance`, with ProcChance mods.

When an aura procs:
- each PROC_TRIGGER_SPELL (42) or PROC_TRIGGER_SPELL_WITH_VALUE (231) effect casts `EffectTriggerSpell` from the aura's unit at the other party, at depth 1;
- the aura's `AuraScripts` `on_proc` runs;
- one charge is spent.

Example: Devastator 236279, a passive with mask 0x4 and 100% chance, casts 236282 (131 damage at AP 100) on every swing.

Not modeled: `SpellTypeMask` (damage/heal/no-damage filter), hit masks (there are no misses or crit-only procs), `PROC_ATTR_REQ_EXP_OR_HONOR`, and proc events for ranged auto-attack, jump, enter combat, dispel and loot.

## Spell modifiers

- **Types:** ADD_FLAT_MODIFIER 107 and ADD_PCT_MODIFIER 108 match the aura spell's `SpellClassSet` plus an overlap between the effect's `SpellClassMask` and the target spell's `spell_class_options` mask. The label types 219 (flat) and 218 (pct) match `EffectMiscValue_1` against the target spell's `SpellLabel` rows (`SpellInfo.labels`).
- **Op:** `EffectMiscValue_0` is the `SpellModOp`.
- **Formula:** `(base + Σflat) × Π(1 + pct/100)`, with amount × stacks.
- **Charges:** a charged mod aura that affects a cast spell loses one charge on each cast and is removed at 0.

| Op | Name | Applied to |
|---:|---|---|
| 0 | HealingAndDamage | direct damage and heal amounts |
| 1 | Duration | aura duration, channel duration |
| 3, 12, 23, 32, 33 | PointsIndex0-4 | base points of that effect index |
| 4 | ProcCharges | aura charges |
| 5 | Range | max range check |
| 7 | CritChance | crit chance (percentage points) |
| 8 | Points | every effect's base points |
| 10 | ChangeCastTime | cast time (before spell haste) |
| 11 | Cooldown | spell and category cooldown |
| 14, 34, 39 | PowerCost0-2 | `SpellPower` row by `OrderIndex` |
| 15 | CritDamageAndHealing | crit bonus |
| 18 | ProcChance | proc chance of the aura |
| 19 | Period | periodic aura period |
| 21 | StartCooldown | GCD |
| 22 | PeriodicHealingAndDamage | periodic damage and heal ticks |
| 24 | BonusCoefficient | SP/AP coefficients |
| 37 | MaxAuraStacks | max stacks |

In-scope uses of the ops listed above, as (aura, op) pairs, number more than 30. The most common are (108, 0) with 624 effects, (108, 22) with 271, (219, 3) with 148, (218, 0) with 133 and (219, 12) with 91.

Not applied:
- 2 Hate: no threat system.
- 6 Radius.
- 9 ResistPushback: no pushback.
- 17 ChainTargets: no chains.
- 20, 27 Amplitude.
- 31 Doses.
- 40 MaxTargets.
- The PvP modifier auras 647/648/649: there is no PvP combat context.

Examples:
- Unwavering Will 373456 cuts Flash Heal 2061's cast time to 1425 ms.
- Mental Agility 341167 halves Purify's cost to 65 mana.
- Shadow Priest 137033 scales Mind Blast 8092 by 0.93 × 4.00 (class mask) × 2.37 (label 4040).

## Cast pipeline integration

- **Costs:** a `SpellPower` row with `RequiredAuraSpellID` applies only while the caster has that aura. Priest costs are gated on the spec passives 137031/137032/137033/417191, so a priest pays the row of whichever spec passive it knows; with none known the spell is free.
- **Cast time:** ChangeCastTime mods, then ÷ `1 + CharacterStats.haste_pct/100`.
- **Charges:** max charges and recharge follow MOD_MAX_CHARGES 411, CHARGE_RECOVERY_MOD 453 and CHARGE_RECOVERY_MULTIPLIER 454 on the charge category (`EffectMiscValue_0`). Double Time 103827 gives Charge 100 2 charges and a 17 s recharge.
- **Power regen:** MOD_POWER_REGEN_PERCENT 110 and MOD_MANA_REGEN_PCT 379 scale regen and decay rates.
- **Movement:** movement reads `AuraStatMods.movement_speed`. It replaced the unused `shared::aura::Auras` read. The combination is the strongest MOD_INCREASE_SPEED 31 × every MOD_SPEED_ALWAYS 129 × the strongest MOD_DECREASE_SPEED 33, and 0 under MOD_ROOT 26, MOD_ROOT_2 455 or MOD_STUN 12.
- **Crowd control:** cast validation fails with `Stunned` under MOD_STUN 12 (unless `SPELL_ATTR5_ALLOW_WHILE_STUNNED`), `Silenced` under MOD_SILENCE 27 or MOD_PACIFY_SILENCE 60 for `PreventionType` 1 spells, and `Pacified` under MOD_PACIFY 25 or 60 for `PreventionType` 2 spells. A cast already in progress is not interrupted.

## Stat modifiers

Every aura change recomputes two components, each written only when its value changed:

- **`stats::StatModifiers`** (additive percentages, [[character-stats]]): `update_character_stats` folds it into `CharacterStats`, so each stat below is applied exactly once, there.

| Field | Aura types |
|---|---|
| `primary_flat` | MOD_STAT 29 (MiscValue stat index 0 Str, 1 Agi, 2 Sta, 3 Int; negative = all) |
| `primary_pct` | MOD_TOTAL_STAT_PERCENTAGE 137 (MiscValueB stat bitmask; 0 = all, TrinityCore `HandleModTotalPercentStat`) |
| `crit_pct` | MOD_CRIT_PCT 290 |
| `haste_pct` | 138, 140, 192, 342, 193, 216, plus the strongest MOD_CASTING_SPEED_NOT_STACK 65 (retail haste is one stat) |
| `mastery_points` | MASTERY 318 |
| `max_health_pct` | MOD_INCREASE_HEALTH_PERCENT 133 |

  Consumers: spell crit rolls `CharacterStats.crit_pct` (+ CritChance mods); cast time divides by `1 + CharacterStats.haste_pct/100`. NPCs have no `CharacterStats`, so these auras do not change NPC stats.
- **`AuraStatMods`** (multipliers, `1 + amount/100` each): combat paths read it directly.

| Field | Aura types | Consumer |
|---|---|---|
| `damage_done_mult[7]` / `damage_taken_mult[7]` | 79 / 87 per school bit | spell, periodic and melee damage |
| `healing_done_mult` / `healing_taken_mult` | 136 / 118, 283 | heals |
| `autoattack_damage_mult` | 344 | melee swing |
| `power_regen_mult` | 110 (by power), 379 (mana) | regen |
| `movement_speed` | 31, 129, 33, 26, 455, 12 | movement |

Aura amounts add SP/AP coefficients only for PERIODIC_DAMAGE 3, PERIODIC_HEAL 8 and SCHOOL_ABSORB 69; every other type uses its base points (Bloodlust 2825 is +30% haste regardless of spell power).

## Replication

- Every `Player` and `Npc` gets `ActiveAuras`, `UnitAuras` and `AuraStatMods`.
- `sync_unit_auras` rebuilds `UnitAuras` only for units whose aura set changed (apply, refresh, removal, charge use). It then marks the entity in `ReplicationDirtyQueue`.
- `remaining_ms` is measured when the view is built, and the client counts down from it. Ticks do not resend the view.
- `AuraView` fields:
  - `instance_id` is unique per unit;
  - `caster` is entity bits;
  - `stacks` and `charges`;
  - `duration_ms` is 0 for permanent auras;
  - `harmful` and `dispel_type`;
  - `flags` carry PASSIVE, HIDDEN and FROM_PLAYER.
- Combat log: `AuraApplied`, `AuraRefreshed` and `AuraRemoved` go to the owners of the caster and the unit, with `amount` = stacks. `Dispel` goes out as described above.

## Script hooks

`AuraScripts(HashMap<spell_id, AuraScript>)` holds four optional hooks: `on_apply`, `on_remove`, `on_periodic` and `on_proc`. The registry is empty. DUMMY 4 and PERIODIC_DUMMY 226 do nothing without a script.

## Coverage

- Handled aura IDs (`handlers::kind`): 3, 4, 8, 12, 20, 23, 24, 25, 26, 27, 29, 31, 33, 39, 42, 60, 65, 69, 79, 87, 89, 107, 108, 110, 118, 129, 133, 136, 137, 138, 140, 192, 193, 216, 218, 219, 226, 231, 271, 283, 290, 318, 342, 344, 379, 411, 453, 454, 455.
- Of the 95% cut (77 IDs, 6576 instances in [spell coverage](../../spell-coverage.md)), 36 IDs covering 5693 instances (87%) are handled. DUMMY 4 counts as handled, but through the empty script registry.
- Unhandled cut IDs, all tracked and replicated:
  - no pet or summon system: 429, 382;
  - client or visual only: 332 action bar override, 56 transform, 61 scale, 260 screen effect, 403 visual override, 44 tracking;
  - no mount, shapeshift or area-trigger system: 78, 36, 275, 395;
  - no PvP context: 647, 648, 649;
  - crowd control without a cast-fail mapping or movement model: 5 confuse, 7 fear, 321 no actions;
  - no mechanics or diminishing returns: 77, 232;
  - no armor, block or avoidance model: 142, 51, 187, 229;
  - no haste-regen stat: 416, 417, 457;
  - no consumer: 418 max power, 443 leech;
  - not yet: 63 additional power cost, 168 damage vs creature type, 213 rage from damage, 220 ability school, 285 linked, 468/396 trigger on health/power, 191/305 speed rules, 467 stat bonus pct, 320, 531.

## Sources

- [auras/mod.rs](../../../crates/server/src/auras/mod.rs) — instances, stacking, removal, replication, CancelAura, dispel, passives
- [auras/periodic.rs](../../../crates/server/src/auras/periodic.rs) — ticks
- [auras/procs.rs](../../../crates/server/src/auras/procs.rs) — events, procs, interrupts
- [auras/spell_mods.rs](../../../crates/server/src/auras/spell_mods.rs) — `SpellModOp`, matching, charges
- [auras/stat_mods.rs](../../../crates/server/src/auras/stat_mods.rs) — `StatModifiers` and `AuraStatMods` from auras
- [auras/handlers.rs](../../../crates/server/src/auras/handlers.rs) — aura type registry
- [auras/tests.rs](../../../crates/server/src/auras/tests.rs) — real-DB App tests
- [spec](../../specs/auras.md)
- TrinityCore master `a96d8977` `SpellAuras.cpp`, `SpellMgr.cpp` (`CanSpellTriggerProcOnEvent`, proc autogeneration), `SpellDefines.h` (`SpellModOp`, `SpellAuraInterruptFlags`)

## See Also

- [[spell-casting]] — the pipeline that applies auras and reads spell mods
- [[spell-info]] — aura options, interrupts, class options, labels
- [[class-progression]] — `KnownSpells` passives
- [[combat]] — melee swing events
- [[character-stats]] — `StatModifiers` fold-in
