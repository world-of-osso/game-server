# Training Dummies

Creatures with `creature_template.ScriptName = 'npc_training_dummy'` behave like AzerothCore's `npc_training_dummy` (`src/server/scripts/World/npcs_special.cpp`), so players can test damage on them. Server: `crates/server/src/creature_scripts.rs` (ScriptName dispatch at spawn in `zones.rs`), damage paths in `combat.rs` (melee) and `spell_cast/effects.rs` (spells and periodic damage).

## What it must do

- [x] Every spawn whose template has ScriptName `npc_training_dummy` gets the training dummy behaviour; other creatures are unchanged.
- [x] Damage taken is set to 0 (`DamageTaken`: `damage = 0`), so the dummy's health never drops and it never dies, however much damage it takes.
- [x] The combat log still reports the full hit, with overkill against the unchanged health (AzerothCore sends the log before `DealDamage` zeroes the damage): melee swings as `CombatEvent` MeleeDamage and as `CombatLogEvent` physical Damage without a spell (`SendAttackStateUpdate`) to the attacker and target, spells as `CombatLogEvent` Damage.
- [x] No death, so no kill XP, reputation or kill credit.
- [x] The dummy never attacks back and never moves (MovementType 0 spawns, no creature melee).
- [x] Five seconds after the last damage (`resetTimer = 5000`, restarted by each hit) the dummy leaves combat and evades: auras other units put on it are removed (`_EnterEvadeMode` → `RemoveEvadeAuras`).

## Tests asserting this spec

- `crates/server/src/combat_tests.rs` `human_melee_on_training_dummy_logs_every_swing_and_never_kills_it` — human (FactionTemplate 1) auto-attacks Grandmaster's Training Dummy (31144, faction 7, 0.0187 health): three 10-damage MeleeDamage events, no Death, health unchanged, no DeathState, dummy has no AutoAttack.
- `crates/server/src/creature_scripts_tests.rs` — Mortal Strike on the dummy logs Damage 520 with overkill 519, no Death, health unchanged; Hamstring keeps it in combat and slowed at 4.5 s, and at 5.5 s it is out of combat with Hamstring removed (AuraRemoved logged); a warrior's auto-attack puts two swings (physical, 10 damage, overkill 9) in the attacker's combat log.
- `crates/server/src/zones_world_db_tests.rs` `stormwind_training_dummies_get_their_script_and_stand_still` (`--ignored`, real world.db) — Stormwind spawns 201235 and 202726 get the behaviour, Northshire guard 79927 does not, and the dummies hold their position over 200 ticks.

## Known gaps

- [ ] `SPELL_STUN_PERMANENT` 61204 (Reset), knockback immunity and `mechanic_immune_mask` are not applied; creatures have no melee, facing or knockback yet, so nothing observable depends on them.
- [ ] Evade does not stop players still auto-attacking the dummy out of range (`CombatStop` → `RemoveAllAttackers`).
- [ ] `npc_target_dummy` (engineering target dummies 2673/2674/12426, summoned and self-killing after 15 s) is not implemented: nothing summons them.
- [ ] Capital dummies have `HealthModifier` 0.000187, and creature health is `HealthModifier × 100`, so they show 0 health; creature health does not use class/level base health.
- [ ] A lethal melee swing sends no `CombatLogEvent` Death (spells do), so melee kills are missing from the chat Combat Log.
