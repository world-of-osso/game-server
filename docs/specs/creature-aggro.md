# Creature Aggro

World creatures attack players: hostile creatures aggro players that come near, creatures fight back when attacked, chase and auto-attack their top-threat target, and evade home when they have no valid target or are pulled too far. Retail behaviour is the target; AzerothCore's creature AI supplies the rules. Server: `crates/server/src/creature_aggro.rs`, movement in `crates/server/src/creature_ai.rs`. How it works: [creature-aggro wiki](../wiki/systems/creature-aggro.md).

## What it must do

### Who takes part (`AggroMode`, from `creature_template` at load)

- [x] Critters (`type` 8), non-combat pets (`type` 12), triggers (`flags_extra` 0x80), `IMMUNE_TO_PC` units (`unit_flags` 0x100) and `npc_training_dummy` creatures have no threat list: they never aggro, never chase and never swing, however they are attacked.
- [x] Civilians (`flags_extra` 0x2, e.g. Kobold Vermin) never aggro on sight but fight back when attacked.
- [x] Every other creature aggroes on sight when it is hostile to the player (FactionTemplate reaction Hostile); a neutral creature (e.g. Stonetusk Boar, template 189) only fights back.

### Aggro on sight (`CreatureAI::MoveInLineOfSight`, `Creature::CanStartAttack`)

- [x] Player and creature proximity use pinned TC a352b1fa `Creature::GetAttackDistance` (Creature.cpp:2165–2206), not the pet-only GetAggroRange or legacy detection_range. Start at15 minus the creature's authored model combat reach; add both detect-range aura modifiers when creature level+5≤90, and add the level difference. Above the spawned difficulty's HealthScalingExpansion maximum level, use expansion maximum minus target level instead, without detect modifiers. Clamp attack distance to5..45, then add both scaled combat reaches for strict3D distance comparison (Object.cpp:408–428). Defaults: Rate.Creature.Aggro1, MaxPlayerLevel90 (World.cpp:1002,751; SharedDefines.h:109–140). Level30 versus32 with1.5yd reaches has a14.5yd center boundary:14.49 engages,14.5 and15.7 do not. Missing model/template authority reports an error and does not substitute guessed geometry.
- [x] An idle, alive creature attacks the nearest hostile player inside its radius.
- [x] Dead players, ghosts and players on a flight path are never aggroed.
- [x] Aggro puts the creature and the player in combat, starts the creature's chase and its auto-attack on the player.

### Creature versus creature (`CreatureRelocationNotifier`, `Creature::CanStartAttack`): `crates/server/src/creature_sight.rs`

- [x] Once a second, each creature that moved, spawned, woke or changed model/scale is offered to nearby creatures and they to it. NpcGrid uses45yd cells with per-copy conservative maximum combat reach to cover the post-clamp body extents; no per-tick all-pairs scan.
- [x] An idle aggressive creature attacks the nearest creature within its aggro radius (each side's level against the other) whose FactionTemplate reaction is Hostile and that it can attack; both enter combat, chase and swing. Stormwind Infantry (2321) and a Blackrock Invader (14) fight; the Infantry and the Blackrock Worg (32) are neutral and never do (the Northshire "fight" is emote 333).
- [x] Creatures immune to NPCs (`unit_flags` 0x200) neither attack nor are attacked; civilians never start a fight but fight back.
- [ ] Line of sight, the Z attack range, `IsNeutralToAll`, `CallAssistance` and GuardAI.

### Aggro on being attacked

- [x] Melee and direct spell damage add threat equal to the damage dealt; an idle creature starts attacking the attacker (`Unit::DealDamage` → `AddThreat`, `CombatStart`).
- [x] A harmful spell landing without damage (e.g. Silence) engages the creature with 0 threat (`Spell::DoSpellHitOnUnit` → `CombatStart`).
- [x] An evading creature ignores damage and does not re-aggro until it is home.

### Threat and target selection (`Creature::SelectVictim`, `ThreatMgr`)

- Effective healing generates 0.5 threat per point, split across the healed unit's engaged creatures, never across unrelated mobs; overheal adds none (TC a352b1fa `Spell.cpp:2893`, `ThreatManager.cpp:733-760`). Controlled creatures retain membership with zero assist threat.
- Damage and healing apply active school-matching `SPELL_AURA_MOD_THREAT` (10), including Righteous Fury when applied (`ThreatManager.cpp:677-730`). Physical swings use the physical school.
- `ThreatUpdate` sends the complete descending creature table only to its engaged players; unchanged tables are not re-sent, changes coalesce at most every 250 ms. New engaged connections receive current state. Removed membership, reset, death or despawn sends an empty table to previous recipients (`ThreatManager.cpp:829-875`, `ThreatClear` / `ThreatUpdate`). Entries include identity, raw threat, threat status and percentages relative to the current victim and 110%/130% aggro thresholds.

- [ ] `THREAT_CURRENT_VICTIM_RESELECTION`: keep a server-owned threat current-victim reference, distinct from active attack/chase projection. TC a352b1fa `ThreatManager.cpp:216–248,581–657` and `ThreatManager.h:86` require periodic1000ms reselection even while passive and lazy refresh when absent/offline. Passive reselection must not start attacks/chase. Active aggro projects the reference into existing CreatureState and CombatTarget; scripted MaxThreat uses current-reference-first ordering (`UnitAI.cpp:319–348`).
- [ ] Available fixate wins; taunt priority precedes ordinary threat. Otherwise retain the incumbent at exact110% melee and130% ranged; only strict excess switches (`ThreatManager.cpp:620–626,674–681`). If the highest candidate is ranged below130%, a lower-ranked melee candidate above110% can win. Melee geometry uses the candidate's authored combat reach and the actor's native autoattack range (`Unit.cpp:691–710`), not a universal5yd center cutoff. Unavailable references do not win selection; reset/death/clear must not retain a stale current reference.
- [x] A target that dies is dropped from the threat list.
- [ ] Ghosts, players on a flight path, despawned units and units the creature's faction cannot attack are dropped the same way (same check as aggro on sight; not asserted on an engaged creature).

### Chase and melee

- Shared chase stops and drops its cached route when the current victim disappears, is dead/ghost, leaves the map copy, or the actor's explicit attack victim no longer matches that chase. A valid victim already within the existing2yd stop distance (or a smaller existing follower reach) stops movement only with a present server LOS resource and a clear body ray, before advancing cached intermediate corners. Missing LOS authority must not certify this new pre-stop. Do not enlarge melee/aggro radii, bypass blocked LOS or alter companion target reselection. TC a352b1fa `ChaseMovementGenerator.cpp:28–30,44–55,99–110,125–140`; `CreatureAI.cpp:244–261` and `ScriptedFollowerAI.cpp:145–147` preserve selection of a remaining valid attacker after a kill. [Production cached-corner reproduction](../wiki/systems/creature-ai.md#current-victim-stop-contract--2026-10-17).

- [x] The creature runs (`speed_run` × 7 yd/s) toward its target until 2 yards away horizontally, following the target's height, and auto-attacks through the player auto-attack path.

### Evade (`CreatureAI::EnterEvadeMode`)

- [x] A victim more than 103 yards (the 100 yd continent visibility range plus both 1.5 yd combat reaches) from the creature's spawn point is no longer a valid target, unless the creature took direct (non-periodic) damage in the last 10 s (`Creature::CanCreatureAttack`, Creature.cpp:2731-2755; `MAX_AGGRO_RESET_TIME`, Unit.h:44; `SetLastDamagedTime`, Unit.cpp:1085-1089) or is in a dungeon. A rooted or stationary creature that cannot leash thus drops a player who walks away and evades. Not modelled: world bosses (always checked), taunt auras, `CanNotReachTarget` (no path data).
- [x] With no valid target left, or more than 40 yards (2D) from its spawn point, the creature evades: threat cleared, combat stopped (it and everyone auto-attacking it stop swinging), and it runs home at run speed with its health unchanged, resuming its normal AI on arrival (`CreatureAI::_EnterEvadeMode` touches no health). At home a creature without `RegenHealth` is restored to full (`HomeMovementGenerator::DoFinalize` → `SetSpawnHealth`; `curHealthPct` is not loaded); a `RegenHealth` creature regenerates a third of its max health every 2 s once it is no longer evading (`Creature::RegenerateHealth`).
- [x] A creature that dies forgets its threat and target and stops chasing.

## How it works

- [creature-aggro](../wiki/systems/creature-aggro.md)
- [creature-ai](../wiki/systems/creature-ai.md) — chase, leash, return-to-spawn movement
- [combat](../wiki/systems/combat.md) — auto-attack, combat state

## Implementation inventory

- `crates/server/src/creature_aggro.rs` — `AggroMode`, spawn components, aggro-on-sight, victim update, threat/engage/evade/death helpers
- `crates/server/src/creature_ai.rs` — chase (run speed, 2 yd reach), leash check, return home
- `crates/server/src/world_data.rs` — `detection_range` and `AggroMode` per template
- `crates/server/src/zones.rs` — aggro components on world spawns
- `crates/server/src/combat.rs` — melee threat, death clears the fight
- `crates/server/src/spell_cast/effects.rs`, `spell_cast/mod.rs` — spell damage threat, harmful spell hits

## Tests asserting this spec

- `crates/server/src/threat_updates_tests.rs`: production damage/heal threat ratios, heal splitting against the healed unit's enemies, real protocol loopback delivery to engaged players only, unchanged-table suppression and reset clears.
- `spell_cast/tests.rs` `threat_heal_clamps_to_max_and_logs_overheal`: real Flash Heal adds 50 threat for 100 effective healing regardless of overheal.

- `crates/server/src/creature_aggro_tests.rs` — aggro radius formula, `AggroMode` from flags, hostile spider aggroes/chases/hits, level-scaled radius, nearest player first, dead/ghost/flying ignored, neutral boar fights back only when hit (threat = damage), hostile civilian fights back only when hit, training dummy never fights, 110%/130% threat swap, target death evades home keeping its damage and heals at home, leash evade stops attackers and ignores damage while evading, killed creature forgets its fight, Fireball threat 388 on a neutral creature, Silence engages without damage; a rooted spider drops a victim 120 yd from its home 10 s after its last hit and evades, and keeps one 60 yd away.
- `crates/server/src/zones_world_db_tests.rs` `world_spawns_aggro_by_template_hostile_attack_civilian_and_dummy_do_not` (`--ignored`, real world.db) — Riverpaw Outrunner 80474 and Forest Spider 79883 aggro and hit a level 5 Human 4 yd away; Kobold Vermin 79992 (civilian) and Heroic Training Dummy 48325 do not.
- `crates/server/src/creature_ai_tests/` — chase reach and run speed, leash evade, return-to-spawn run speed.
- Live 2-process proof (2026-09-25, `game-server/data/diagnostics/aggro-20260925/`, `live-proof.sh`): server on :5071 in a private-/tmp bwrap, `game-cli chat listen` puts a level 1 Human 4 yd from Forest Spider spawn 79883 without targeting anything; `game-server-admin combat-log 25` shows 10 MeleeDamage events (10 each, every 2.05 s) from an NPC entity to the player entity. The attacker entity was not resolved to a name.

## Known gaps (current cycle)

- [ ] Social/assist aggro: nearby creatures of the same pack do not join (`Creature::CallAssistance`, `CreatureGroup` assist flags).
- [x] Dungeon maps (party/raid/scenario) skip the 40-yard spawn leash, matching pinned TC `Creature::CanCreatureAttack` (Creature.cpp:2731). Script-authored boundaries still apply. Slagmaw's East teleport119425 remains engaged ~48yd from spawn; the former Stockade39.9yd leash proof is superseded, not current behavior ([preserved evidence](../wiki/systems/navigation.md#stationary-arch-and-leash-acceptance)).
- [x] Open-world maps retain the existing inclusive40yd spawn leash unchanged: continent map0 stays engaged at40yd and evades at40.1yd (`leash_open_world_map_keeps_forty_yard_boundary`).
- [ ] Open-world spawn leash remains flat40yd rather than AzerothCore's30yd home-distance/timer policy; no open-world policy change is included in the dungeon exemption.
- [ ] Patrollers return to their spawn point, not to where they left their path (`GetResetPosition`).
- [ ] Periodic spell damage goes through the same `deal_damage` threat hook but has no test.
- [ ] No 5 s post-respawn aggro grace (`m_respawnedTime`), no stunned check, no taunt. Spell-specific flat/AP threat and NO_THREAT/NO_HELPFUL_THREAT attributes are not modelled.
- [ ] No GM mode exists, so GM players cannot be excluded yet.

## Out of scope

- Line of sight and Z-difference checks: the server has no collision data and creature heights are not kept on the ground while they move.
- Pets, spell-casting creature AI (SmartAI), pathfinding.
