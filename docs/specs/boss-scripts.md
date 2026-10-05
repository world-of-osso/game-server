# Boss scripts (server)

Dungeons phase 3: creature scripts selected by `creature_template.ScriptName`, ported from TrinityCore master a352b1fa (`CreatureAI`/`ScriptedAI`/`BossAI`, `InstanceScript`, `CreatureTextMgr`) and the Stockade scripts (src/server/scripts/EasternKingdoms/TheStockade/). Code: `crates/server/src/scripted_ai/`. Protocol: shared-protocol `protocol/encounter_messages.rs`, `ChatType::Monster*`. Client: game-engine `docs/specs/boss-encounters.md`. Data: `content_creature_text` (TDB1210 creature_text, `scripts/import-creature-text.sh`) with `content_broadcast_text`.

## What it must do

### Framework
- [x] `EventMap`: events run in due order (ties in schedule order); `Repeat` reschedules the last event from now; `Reset` clears it (`EventMap.cpp`).
- [x] A creature with a ported ScriptName gets its AI when it enters the world and only in a map whose `instance_template.script` is its instance script (`GetInstanceAI`); the AI's `Reset` runs then and on respawn.
- [x] Hooks: `JustEngagedWith` when the creature first gains threat, `DamageTaken` (health before the hit and the health loss) on melee and spell damage, `JustDied`, `EnterEvadeMode` → `Reset`, `JustSummoned`/`SummonedCreatureDespawn`, `MovementInform(POINT)` at the end of `MovePoint`, `UpdateAI` every tick while alive. A hook raised while the creature's own AI runs is queued behind it.
- [x] `BossAI::UpdateAI`: events advance only while the boss has a victim and execute only while it is not casting. `_JustEngagedWith` sets the boss `IN_PROGRESS`, `_JustDied` `DONE`, `_Reset` `NOT_STARTED` (unless `DONE`), despawning summons.
- [x] `InstanceScript` per dungeon copy: boss states `NOT_STARTED`/`IN_PROGRESS`/`DONE`, `DONE` never goes back; lives until the copy unloads. `IN_PROGRESS` sends `EncounterStart { encounter_id }`, `DONE` `EncounterEnd { success: true }`, a reset from `IN_PROGRESS` `EncounterEnd { success: false }` (Retail SMSG_ENCOUNTER_END; TrinityCore sends nothing) — only to the copy's players.
- [x] Engaging a boss sends `EncounterEngageUnit` (boss frame, priority 1); its reset or death `EncounterDisengageUnit` (Retail; the TrinityCore Stockade scripts do not call `SendEncounterUnit`).
- [x] `Talk(group)`: a not-yet-said text of the group by probability; BroadcastText text (female text for female/genderless speakers when present); Type 12/14/16/41 → `MonsterSay`/`MonsterYell`/`MonsterEmote`/`RaidBossEmote` carrying the speaker; TextRange 3 reaches every player of the speaker's copy, 0 the chat type's range (25/300 yd).
- [x] Evade (`_EnterEvadeMode`): auras removed except `AURA_STAYS_AFTER_COMBAT`, cast cancelled, cooldowns reset, full health; at home mana is full and addon auras return. An evading creature does not evade again.
- [x] `REACT_PASSIVE`: no aggro on sight, no victim selection, attack stopped; back to aggressive resumes attacking the victim.
- [x] Summons (`SummonCreature`, manual despawn): never respawn, go with their corpse or the summoner's `SummonList`; `MovePoint` at walk or run speed.

### Creature spells
- [x] Creature casts: no GCD; no explicit target → the victim when a unit target is needed; triggered casts skip cast time, costs, cooldowns and caster auras; `DoCast(spellId)` targets per `FillAISpellInfo` (self/victim/enemy/debuff).
- [x] A creature's school damage and periodic damage base points are `BasePoints`% of ExpectedStat `CreatureSpellDamage` at its level (`CreatureAutoAttackDps` with `SCALES_WITH_CREATURE_LEVEL`), rounded; weapon % damage uses its ExpectedStat weapon damage.
- [x] Creatures of mana classes (2, 8) have `basemana × ManaModifier` mana; with `UNIT_FLAG2_REGENERATE_POWER` they regain 22 mana / 2 s except while evading.
- [x] A casting creature neither chases (if movement interrupts the cast) nor swings; a stunned unit does not swing; stunned/rooted creatures do not chase and chase speed follows speed auras.
- [x] A creature's melee-class spell misses hostile units by 5% − MOD_HIT_CHANCE.
- [x] Effects/targets added: 27 PERSISTENT_AREA_AURA (dynamic object; enemies inside carry the aura with the object's expiry and tick phase, leavers lose it), 252 TELEPORT_UNITS, targets 28 DEST_DYNOBJ_ENEMY, 65 DEST_TARGET_BACK, 77 UNIT_CHANNEL_TARGET.
- [x] Auras added: 18 MOD_INVISIBILITY (hidden from every client, no valid target), 93 MOD_UNATTACKABLE (`NON_ATTACKABLE_2`, attack stopped in a dungeon), 54 MOD_HIT_CHANCE, 252 MOD_SPEED_SLOW_ALL (attack and cast time ×(100−amount)/100).

### SmartAI combat casts
- [x] `SMART_EVENT_UPDATE_IC` advances initial and repeat min/max timers only while engaged; a due cast waits for any active cast to finish.
- [x] `SMART_TARGET_VICTIM` resolves the current victim. `SMARTCAST_COMBAT_MOVE` disables chase after an accepted cast, including between casts; a rejected cast restores chase/melee and retries on the next update without rerolling event chance.
- [x] Reset restores combat movement and initial timers. Real Kobold Geomancer 476 rows cast Fireball 20793, with a 0 ms initial delay and 3000–5000 ms repeat.

### The Stockade
- [x] Hogger (46254, boss 2, encounter 1144): pull yell; Maddening Call 1-2 s then every 15-20 s; Vicious Slice 3-4 s then every 10-14 s; once below 30%: boss emote "Hogger enrages!" and Enrage (+30% melee haste, +10% physical damage); death yell, Warden Thelwater summoned, runs to (152.0, 106.2) and speaks at +1/+5/+8 s.
- [x] Lord Overheat (46264, boss 1, encounter 1145): pull and death yells; Fireball 2 s then every 2 s; Overheat on a random player 9-11 s then 9-10 s; Rain of Fire at the victim 10-13 s then 15-20 s.
- [x] Randolph Moloch (46383, boss 0, encounter 1146): pull yell; Sweep 2-3 s then 6-7 s; Wildly Stabbing channel 4-5 s then 8-12 s; hits taking him below 71% (from above 59%) and below 41% (from above 29%) each vanish him once 1 s later: boss emote, auras cleared, Vanish, passive; 2 s later Shadowstep behind a random player, aggressive; death yell, Mortimer Moloch walks to him, yells at +4 s and collapses (emote, dies) at +7 s.
- [x] A wipe resets every Stockade boss (deviation: TrinityCore's Hogger and Moloch `Reset` skip `_Reset`).

### Grim Batol
- [x] `instance_grim_batol` (map 670): 4 bosses (General Umbriss 39625, Forgemaster Throngus 40177, Drahga Shadowburner 40319, Erudax 40484; encounters 1051, 1050, 1048, 1049), `LoadObjectData(creatureData)`.
- [x] Deviation: TrinityCore has no Grim Batol creature scripts, so no kill there sets a boss state or locks anyone. Each `creatureData` boss runs a spell-less `BossAI`: engage → `IN_PROGRESS`, wipe → `NOT_STARTED`, death → `DONE` (then the instance lock, docs/specs/instances.md). No boss abilities.
- [x] An instance script created for a copy with an instance lock starts with the lock's `DONE` bosses (`InstanceScript::Load`); encounter messages carry the copy's difficulty.

## How it works
- `docs/wiki/systems/boss-scripts.md`

## Implementation inventory
- `crates/server/src/scripted_ai/mod.rs` — AI component, hooks, dispatch, `Ctx` (ScriptedAI helpers)
- `scripted_ai/event_map.rs`, `boss_ai.rs`, `instance_script.rs`, `creature_text.rs`, `movement.rs`, `the_stockade.rs`
- `spell_cast/creature.rs` (creature casts, AI targets, spell misses), `spell_cast/area_aura.rs` (dynamic objects), `spell_cast/effects.rs` (ExpectedStat base value, teleport), `spell_cast/targets.rs`
- `auras/unit_state.rs` (invisibility, unattackable), `auras/stat_mods.rs`
- `creature_aggro.rs` (engage/evade hooks, react state), `creature_ai.rs` (chase gating, home), `combat.rs` (swing gating, JustDied, kill, summons' corpses), `zones.rs` (summons, creature mana)
- `scripts/import-creature-text.sh`, `scripts/retail_content.py` `build_creature_text`

## Tests asserting this spec
`cargo test -p server --bin game-server scripted_ai::` (EventMap, texts, all three bosses on real world.db), plus `combat::`, `creature_aggro::`, `spell_cast::`, `auras::`, `maps::` regression suites.

## Known gaps (current cycle)
- [ ] Creature auto-attacks still never miss; dodge/parry/block of creature attacks and magic spell misses are not modelled.
- [ ] Channel targets out of range do not end a channel; `MovePositionToFirstCollision` has no collision.
- [ ] Creature text sounds and emotes (`Sound`, `Emote`, BroadcastText SoundKit) are not played; ranges other than 0/3 are not ported.
- [ ] Player casters' damage base points are not ExpectedStat-scaled.

## Out of scope
- Dungeon Finder (phase 5), doors (the Stockade has none), `FAIL`/`_DespawnAtEvade` (unused by these scripts), Grim Batol boss abilities.

## Live evidence (2026-09-27)
Own server :5085 (world.db = `world-stockadebosses-20260927.db`, copied game.redb), one headless client (Xpbar, level 80 mage; bosses hit with `game-server-admin damage`), evidence in game-engine `data/diagnostics/stockadebosses-20260927/` (`server-run3-hogger.log` for Hogger, `server.log` for the others):
- Hogger: pull yell, EncounterStart 1144 and Boss1TargetFrame (`02`, `ui-02`); Maddening Call and Vicious Slice debuffs on the player (`03`); a hit to 29% gave the center-screen "Hogger enrages!" (`04`); leaving him reset the encounter (EncounterEnd success false, frame hidden, full health, no enrage; `06`); death: "Yiiipe!", EncounterEnd success true, Warden Thelwater ran to (152.0, 106.2) and said all three lines (`07`, `08`, `ui-08`).
- Lord Overheat: pull yell, Fireball chain, Overheat, Rain of Fire's dynamic object at the player, mana 4177/4313 on his boss frame (`11`-`13`); leaving reset him, a new pull and his death gave "FIRE... EXTINGUISHED!" and DONE (`14`).
- Randolph Moloch: pull yell, Sweep and Wildly Stabbing (`20`, `21`); a hit to 70% gave "Randolph Moloch vanishes!", he left the client, Shadowstep brought him back beside the player (`22`, `23`); death yell, DONE, Mortimer walked to him, yelled and collapsed (`24`-`26`).
- Not shown live: the second vanish, Rain of Fire's radius against a second player (unit tests cover both).
