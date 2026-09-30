# Training Dummies

Creatures with `creature_template.ScriptName = 'npc_training_dummy'` behave like AzerothCore's `npc_training_dummy` (`src/server/scripts/World/npcs_special.cpp`), so players can test damage on them. Server: `crates/server/src/creature_scripts.rs` (ScriptName dispatch at spawn in `zones.rs`), damage paths in `combat.rs` (melee) and `spell_cast/effects.rs` (spells and periodic damage).

## What it must do

- [x] Every spawn whose template has ScriptName `npc_training_dummy` gets the training dummy behaviour; other creatures are unchanged.
- [x] Damage taken is set to 0 (`DamageTaken`: `damage = 0`), so the dummy's health never drops and it never dies, however much damage it takes.
- [x] The combat log still reports the full hit, with overkill against the unchanged health (AzerothCore sends the log before `DealDamage` zeroes the damage): melee swings as `CombatEvent` MeleeDamage and as `CombatLogEvent` physical Damage without a spell (`SendAttackStateUpdate`) to the attacker and target, spells as `CombatLogEvent` Damage.
- [x] No death, so no kill XP, reputation or kill credit.
- [x] Health is the AzerothCore creature health (`Creature::SelectLevel`): `ceil(creature_classlevelstats.basehp<exp> × HealthModifier)`, so HealthModifier 0.000187 gives Grandmaster's (31144, level 80) 3 health and Expert's (32666, level 60) 1.
- [x] The dummy never attacks back and never moves (MovementType 0 spawns, no creature melee).
- [x] Each attacker's combat with the dummy ends five seconds after that attacker's last damage (TC `npc_training_dummy::_combatTimer`, npcs_special.cpp:1382-1415; a dummy has no threat list, so its combat references time out, `combat_refs.rs`), and the attacker leaves combat then unless it fights something else; auto-attacking from out of range keeps nobody in combat. When the last attacker's combat ends the dummy evades like AzerothCore's after its 5 s `resetTimer`: auras other units put on it are removed (`_EnterEvadeMode` → `RemoveEvadeAuras`), and players still auto-attacking it stop (`CombatStop` → `RemoveAllAttackers`).
- [x] Periodic damage does not restart an attacker's timer (TC `DamageTaken`, `damageType == DOT`): a DoT's ticks alone leave the attacker out of combat 5 s after the spell landed.

## Tests asserting this spec

- `crates/server/src/combat_tests.rs` `human_melee_on_training_dummy_logs_every_swing_and_never_kills_it` — human (FactionTemplate 1) auto-attacks Grandmaster's Training Dummy (31144, faction 7, 3 health): three 10-damage MeleeDamage events, no Death, health unchanged, no DeathState, dummy has no AutoAttack.
- `crates/server/src/creature_scripts_tests.rs` — Mortal Strike on the dummy logs Damage 520 ± 2.5% (Variance) with the overkill over its 3 health, no Death, health unchanged; Hamstring keeps it in combat and slowed at 4.5 s, and at 5.5 s it is out of combat with Hamstring removed (AuraRemoved logged); an attacker that walks out of range stops auto-attacking at the evade and is out of combat with it; a warrior's auto-attack puts two swings in the attacker's combat log (physical, 34-35 unarmed with 100 AP and overkill over 3, or a miss, dodge or parry).
- `crates/server/src/zones_world_db_tests.rs` `stormwind_training_dummies_get_their_script_and_stand_still` (`--ignored`, real world.db) — Stormwind spawns 201235 and 202726 get the behaviour, Northshire guard 79927 does not, and the dummies hold their position over 200 ticks.
- `crates/server/src/zones_world_db_tests.rs` `creature_health_is_class_level_base_hp_times_health_modifier` (`--ignored`, real world.db) — spawned health: dummies 201235 (31144) 3 and 202726 (32666) 1, Stormwind City Guard 189 15953, Diseased Young Wolf 79648 42.

## Known gaps

- [ ] `SPELL_STUN_PERMANENT` 61204 (Reset), knockback immunity and `mechanic_immune_mask` are not applied; creatures have no facing or knockback yet, and dummies never swing (no threat list, [creature-aggro](creature-aggro.md)), so nothing observable depends on them.
- [ ] `npc_target_dummy` (engineering target dummies 2673/2674/12426, summoned and self-killing after 15 s) is not implemented: nothing summons them.
