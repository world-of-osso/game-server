# Creature Aggro

World creatures attack players: hostile creatures aggro players that come near, creatures fight back when attacked, chase and auto-attack their top-threat target, and evade home when they have no valid target or are pulled too far. Retail behaviour is the target; AzerothCore's creature AI supplies the rules. Server: `crates/server/src/creature_aggro.rs`, movement in `crates/server/src/creature_ai.rs`. How it works: [creature-aggro wiki](../wiki/systems/creature-aggro.md).

## What it must do

### Who takes part (`AggroMode`, from `creature_template` at load)

- [x] Critters (`type` 8), non-combat pets (`type` 12), triggers (`flags_extra` 0x80), `IMMUNE_TO_PC` units (`unit_flags` 0x100) and `npc_training_dummy` creatures have no threat list: they never aggro, never chase and never swing, however they are attacked.
- [x] Civilians (`flags_extra` 0x2, e.g. Kobold Vermin) never aggro on sight but fight back when attacked.
- [x] Every other creature aggroes on sight when it is hostile to the player (FactionTemplate reaction Hostile); a neutral creature (e.g. Stonetusk Boar, template 189) only fights back.

### Aggro on sight (`CreatureAI::MoveInLineOfSight`, `Creature::CanStartAttack`)

- [x] Aggro radius = `detection_range` + (creature level − player level), the level difference capped at 25 below the creature, clamped to 5..45 yards; a detection range under 1 yard never aggroes (`Creature::GetAggroRange`). Forest Spider (level 5, 18 yd): 13 yd against a level 10 player, 22 yd against a level 1 player.
- [x] An idle, alive creature attacks the nearest hostile player inside its radius.
- [x] Dead players, ghosts and players on a flight path are never aggroed.
- [x] Aggro puts the creature and the player in combat, starts the creature's chase and its auto-attack on the player.

### Aggro on being attacked

- [x] Melee and direct spell damage add threat equal to the damage dealt; an idle creature starts attacking the attacker (`Unit::DealDamage` → `AddThreat`, `CombatStart`).
- [x] A harmful spell landing without damage (e.g. Silence) engages the creature with 0 threat (`Spell::DoSpellHitOnUnit` → `CombatStart`).
- [x] An evading creature ignores damage and does not re-aggro until it is home.

### Threat and target selection (`Creature::SelectVictim`, `ThreatMgr`)

- [x] The creature attacks the highest-threat valid unit; a challenger takes over only above 110% of the current target's threat inside 5 yards, 130% beyond.
- [x] A target that dies is dropped from the threat list.
- [ ] Ghosts, players on a flight path, despawned units and units the creature's faction cannot attack are dropped the same way (same check as aggro on sight; not asserted on an engaged creature).

### Chase and melee

- [x] The creature runs (`speed_run` × 7 yd/s) toward its target until 2 yards away horizontally, following the target's height, and auto-attacks through the player auto-attack path.

### Evade (`CreatureAI::EnterEvadeMode`)

- [x] With no valid target left, or more than 40 yards (2D) from its spawn point, the creature evades: threat cleared, combat stopped (it and everyone auto-attacking it stop swinging), health restored to full, and it runs home at run speed, resuming its normal AI on arrival.
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

- `crates/server/src/creature_aggro_tests.rs` — aggro radius formula, `AggroMode` from flags, hostile spider aggroes/chases/hits, level-scaled radius, nearest player first, dead/ghost/flying ignored, neutral boar fights back only when hit (threat = damage), hostile civilian fights back only when hit, training dummy never fights, 110%/130% threat swap, target death evades home at full health, leash evade stops attackers and ignores damage while evading, killed creature forgets its fight, Fireball threat 388 on a neutral creature, Silence engages without damage.
- `crates/server/src/zones_world_db_tests.rs` `world_spawns_aggro_by_template_hostile_attack_civilian_and_dummy_do_not` (`--ignored`, real world.db) — Riverpaw Outrunner 80474 and Forest Spider 79883 aggro and hit a level 5 Human 4 yd away; Kobold Vermin 79992 (civilian) and Heroic Training Dummy 48325 do not.
- `crates/server/src/creature_ai_tests/` — chase reach and run speed, leash evade, return-to-spawn run speed.

## Known gaps (current cycle)

- [ ] Social/assist aggro: nearby creatures of the same pack do not join (`Creature::CallAssistance`, `CreatureGroup` assist flags).
- [ ] AzerothCore's leash is 30 yards from the home position unless the creature took damage in the last 11–16 s (`Creature::CanCreatureAttack`, `GetLeashTimer`); the server evades at a flat 40 yards from spawn.
- [ ] Patrollers return to their spawn point, not to where they left their path (`GetResetPosition`).
- [ ] Periodic spell damage goes through the same `deal_damage` threat hook but has no test.
- [ ] No 5 s post-respawn aggro grace (`m_respawnedTime`), no stunned check, no taunt, no threat modifiers (Righteous Fury etc.), no heal threat.
- [ ] No GM mode exists, so GM players cannot be excluded yet.

## Out of scope

- Line of sight and Z-difference checks: the server has no collision data and creature heights are not kept on the ground while they move.
- Pets, spell-casting creature AI (SmartAI), pathfinding.
