# Creature Aggro

How world creatures decide to fight: `crates/server/src/creature_aggro.rs` (`CreatureAggroPlugin`, FixedUpdate, before `chase_system`) ports AzerothCore's default creature AI (AggressorAI/ReactorAI): aggro on sight, threat from damage, top-threat victim selection and evade. Movement (chase, leash, return home) stays in [[creature-ai]]; swings go through the player auto-attack path in [[combat]]. Contract: [creature-aggro spec](../../specs/creature-aggro.md).

## Spawn components

`CreatureTemplate.aggro_mode` is computed at load from `type`, `flags_extra`, `unit_flags` and `ScriptName` (`AggroMode::for_template`); `zones.rs` calls `insert_aggro_components`:

| Mode | Templates | Components |
|------|-----------|------------|
| `Passive` | critters (type 8), non-combat pets (12), triggers (flags_extra 0x80), `IMMUNE_TO_PC` (unit_flags 0x100), `npc_training_dummy` | none |
| `Reactive` | civilians (flags_extra 0x2) | `ThreatTable`, `CreatureState::Idle` |
| `Aggressive` | everything else | + `AggroRange` proximity marker |

Without a `ThreatTable` a creature never engages: `add_threat` ignores it. Hostility is not part of the mode: an aggressive creature that is neutral to the player (Stonetusk Boar, template 189) never passes the on-sight reaction check, matching AzerothCore where `MoveInLineOfSight` tests `IsHostileTo`.

Measured on world.db (2026-09-25): many Elwynn low-level templates are civilian in AzerothCore data (Kobold Vermin 6, Diseased Young/Timber Wolf 299/69, flags_extra 2), so they only fight back; Forest Spider 30 (faction 22) and Riverpaw Outrunner 478 (faction 20) aggro Humans.

## Aggro on sight

`aggro_on_sight_system`: each idle, alive, non-dormant AggroRange creature engages the nearest valid hostile player inside the source-derived center-distance boundary and server LOS. Creature relocation perception uses the same calculation for NPC targets, including followers.

`creature_aggro_distance.rs` implements pinned TC a352b1fa Creature.cpp:2165–2206 GetAttackDistance:15 minus own reach, level difference, detect modifiers below the max-player-level gate, and the spawned difficulty's HealthScalingExpansion cap; clamp5..45 before adding both combat reaches. Defaults are Rate.Creature.Aggro1 and MaxPlayerLevel90. Model reach comes from encounter_creature_model_info and UnitScale; missing authority reports an error, without guessed reach. The old detection_range formula was the wrong pet-style boundary. At level30 versus32 with1.5yd reaches, the center boundary is14.5yd rather than18. The NPC grid covers post-clamp reaches with a conservative per-copy maximum, including display/scale changes. LOS remains unchanged; the separate Z attack gate remains unimplemented.

Fixture boundary (2026-10-18): NPC relocation tests must load `ModelReaches` with `load_model_reaches` from read-only world.db. Without that resource `AggroDistance::combat_reach` returns None and `index_new_npc` cannot put either NPC in the sight grid; the untouched Northshire fixture reproduces Idle-before-Pursuing on master03a4e6d14. Loading the existing authored model rows (32729/33142/33144/33145/33146, each1.5yd) restores acquisition and the unchanged death/evade lifecycle. No guessed reach, flags, timing, RNG or production change. The corrected short-window oracles require swing attempts and validate landed amounts; all-avoidance windows are valid. At1a9c633d2 the two pet cases, Meredy's authored Fireball CastSuccess, and hostile/neutral Northshire controls pass5/5,21.24s; the hostile case is enabled only after that proof. Ordinary discovery subsequently passes the enabled case1/1 in20.91s, with check65.78s/fmt4.12s warning-free at5c2e9b4df. Independent lead gate pending.

Valid victim (`valid_victim`, used for on-sight and engaged targets): health above 0, no NPC `DeathState`, `PlayerDeathState` Alive (or absent), not on a `TaxiFlight`, and `FactionTemplates::can_attack(creature, victim)`.

## Threat and engage

`add_threat(world, creature, attacker, amount)` handles physical threat; `add_school_threat` handles spell-school threat:

- melee: `combat::record_swing_events` queues it with the swing damage
- spells: `spell_cast::effects::deal_damage` (direct and periodic) with the final damage; `spell_cast::push_hit_events` with 0 for every hit unit the caster can attack (`Relation::Enemy`), so a non-damaging debuff engages
- on sight: `attack_start` = `add_threat(.., 0.0)`

It does nothing for a creature without a threat list, dead, at 0 health, or `Returning` (AzerothCore `UNIT_STATE_EVADE` blocks `AddThreat`). Otherwise it adds the threat; an idle creature becomes `Pursuing { attacker }` with `CombatTarget(Some(attacker))`, which `start_attack_system` turns into an `AutoAttack`. Both units enter combat (`combat::enter_combat`).

`add_healing_threat` uses effective (non-overheal) healing × 0.5, split across creatures that have the healed unit on their tables. Controlled creatures gain membership with no assist threat. `add_school_threat` applies active school-matching MOD_THREAT (10) aura multipliers; taunt is not implemented. References: TrinityCore a352b1fa `Spell.cpp:2893`, `ThreatManager.cpp:677-760`.

## Threat updates

`threat_updates::ThreatUpdatesPlugin` runs in PostUpdate. Every 250 ms it snapshots engaged tables, derives raw/scaled percent and status from `CombatTarget`, and compares each authenticated connection's tables with its last delivery. Only players on a creature's table receive it. Unchanged tables send nothing; lost membership, empty/dead/despawned creatures send an empty `ThreatUpdate` to prior recipients. New connections receive existing tables. `UnitThreatList` remains the independent nameplate-membership replication. The client meter selects one current target, not a damage session. Contract and tests: [creature-aggro spec](../../specs/creature-aggro.md). TC `ThreatManager.cpp:829-875` sends `ThreatClear` / `ThreatUpdate` / `HighestThreatUpdate`.

## Victim update

`creature_threat.rs` stores a server-owned `CurrentVictim { target, next_update }`, separate from `CreatureState::Pursuing`/`CombatTarget`. One selector filters unavailable candidates, prefers available fixate and the existing taunt recipient, then applies strict110% melee/130% ranged hysteresis. At exact thresholds the incumbent stays. When the highest ranged candidate is below130%, another melee candidate above110% can win. Melee membership uses authored combat reaches and the actor's native autoattack-range modifier, not a universal5yd center cutoff.

`update_victim_system` refreshes the reference on its1000ms deadline, including passive or scripted-update-paused creatures. Active creatures project the reference to existing chase and attack fields; passive/paused updates do not project attacks or change movement. `current_victim` and `Ctx::select_target_max_threat` refresh absent/offline/unlisted references lazily without issuing attacks; valid references wait for the periodic update. Lazy refresh preserves the periodic deadline. Unavailable entries retain their numeric threat; removing a fixated membership clears the stale fixate. Evade and death remove the reference/deadline alongside clearing threat. Ordinary active no-victim evade remains in the projection path.

Source: pinned TC a352b1fa `ThreatManager.h:86`, `ThreatManager.cpp:216–248,469–474,539–545,581–657,674–681`, `UnitAI.cpp:319–348`, `Unit.cpp:691–710`. Regression status and exact proof scope are in the branch gate packet; this description is not a full threat-manager parity claim.

`TAUNT_IMMEDIACY_RETAIL_UNVERIFIED`: complete pinned `TauntUpdate`→`EvaluateSuppressed` and `EffectTaunt`→`MatchUnitThreatToHighestThreat` call-chain inspection finds no forced current-reference reselection on an already-engaged target with a healthy listed rival. Provoke's initial observation waits across the1000ms deadline; all caster/threat/expiry assertions remain. Retail play appears immediate, but that timing parity is unverified and is not a product-change requirement. [Qualification](../../specs/creature-aggro.md#taunt-timing-qualification).

## Evade and death

`enter_evade_mode` (`CreatureAI::EnterEvadeMode`), from `update_victim_system` or `creature_ai::leash_check_system` (40 yd from spawn outside dungeon maps; dungeon scripts retain their own boundaries): `Returning`, threat reset, `CombatTarget(None)`, health untouched (TrinityCore `CreatureAI::_EnterEvadeMode`), then `combat::combat_stop` so the creature and everyone auto-attacking it stop. `replicate_threat_list_system` (after `update_victim_system`) copies each creature's `ThreatTable` units into the replicated `UnitThreatList` when they change (TrinityCore `ThreatManager::SendThreatListToClients`, membership only): the client turns a neutral creature's nameplate hostile red while the player is on it (`CompactUnitFrame_IsOnThreatListWithPlayer`), including while it is polymorphed and has no target; evade and death clear it. `return_to_spawn_system` runs it home, then `Idle`; at home a creature without `CreatureHealthRegenTimer` (no `RegenHealth`) gets full health (`SetSpawnHealth`, which TrinityCore skips for regenerating creatures because regeneration refills them), the others regenerate a third every 2 s from then on. Before 2026-09-29 evade restored full health at once, so a pulled mob that killed its target snapped back to full the instant it evaded.

`combat::insert_npc_death` calls `forget_combat`: threat reset, `Idle`, `CombatTarget(None)`, so a corpse does not chase and respawns idle.

`RemoveAurasOnEvade` preserves all existing auras for player-owned units (`faction::player_affected`, TC a352b1fa Unit.cpp4459–4463). Ordinary unowned units still lose auras without AURA_STAYS_AFTER_COMBAT and restore addon auras at home. This prevents no-path/leash evade from erasing follower class/spec/threat passives; no reapplication, stat override or extra spell grant. [Native failure and regression](../investigations/follower-data.md#owned-evade-aura-retention--2026-10-15).

## Sources

- [crates/server/src/creature_aggro.rs](../../../crates/server/src/creature_aggro.rs)
- [crates/server/src/creature_aggro_tests.rs](../../../crates/server/src/creature_aggro_tests.rs)
- AzerothCore `src/server/game/AI/CreatureAI.cpp`, `Entities/Creature/Creature.cpp` (`CanStartAttack`, `GetAggroRange`, `CanCreatureAttack`), `Entities/Unit/Unit.cpp` (`DealDamage`, `SelectVictim`), `Spells/Spell.cpp` (`DoSpellHitOnUnit`)

## See Also

- [[creature-ai]] — chase, leash, return-to-spawn movement
- [[combat]] — auto-attack, combat state, `combat_stop`
- [[factions]] — reaction and `can_attack` rules
- [training-dummies spec](../../specs/training-dummies.md) — passive creatures that take hits without fighting back
