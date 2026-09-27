# Boss scripts

Creature scripts by `creature_template.ScriptName`, ported from TrinityCore master a352b1fa: the `CreatureAI`/`ScriptedAI`/`BossAI` layer, `InstanceScript`, `CreatureTextMgr` and the Stockade scripts. Contract: `docs/specs/boss-scripts.md`. Code: `crates/server/src/scripted_ai/`.

## How a script runs

- A spawned creature whose ScriptName `the_stockade::is_creature_script` knows gets `CreatureScriptName`; `initialize_ai` creates its `CreatureAi` in the same command flush (only when the map's `instance_template.script` is `instance_the_stockade`, `GetInstanceAI`) and dispatches `Reset`. Respawn removes the `CreatureAiState`, so `init_creature_ai` makes a fresh one.
- Hooks are raised where the generic systems reach TrinityCore's call sites: `creature_aggro::add_threat` (Idle → Pursuing: `JustEngagedWith`), `enter_evade_mode` (`EnterEvadeMode` → `Reset`), `combat::insert_npc_death` (`JustDied`, killer = `LastDamager`), melee `try_swing` / `effects::deal_damage` / `combat::deal_direct_damage` (`DamageTaken` with the health before the hit), `zones::summon_creature` / `unsummon` (`JustSummoned` / `SummonedCreatureDespawn`), `movement::point_movement_system` (`MovementInform`). `UpdateAI` runs every FixedUpdate tick of a living creature, between `update_victim_system` and chase/auto-attack.
- `dispatch` takes the AI box out of its component while it runs; a hook raised meanwhile for the same creature (Mortimer's `KillSelf` → `JustDied`, Hogger's `JustDied` → `SummonCreature` → `JustSummoned`) is queued and runs right after.
- `Ctx` holds `&mut World` and `me`: the `ScriptedAI` helpers (`DoCast*`, `Talk`, `SelectTarget(Random)`, `SetReactState`, `RemoveAllAuras`, `SummonCreature`, `MovePoint`, `KillSelf`, `DoZoneInCombat`).
- `BossAI` is a struct each boss embeds; `boss_update_ai` is `BossAI::UpdateAI`. Warden/Mortimer's `TaskScheduler` tasks are `EventMap` events.

## Creature spells

`spell_cast::creature_cast`: victim default target, triggered casts, no GCD. Base points of damage effects follow ExpectedStat `CreatureSpellDamage` (`effects::calc_base_value`); weapon % damage uses `combat::creature_weapon_damage`. `area_aura` holds persistent area aura dynamic objects. `auras::unit_state` derives invisibility (interest `set_hidden`) and `NON_ATTACKABLE_2` from the aura set.

## Pitfalls found

- Removing `NetworkVisibility` to hide an entity made it visible to every client that connected later (`on_replication_client_added` only hides entities that have it): all invisibility-aura creatures of both continents replicated to new clients, and the client grew to 36 GB until earlyoom killed it. Hiding is an interest flag now. Decayed corpses still use the old removal.
- TrinityCore's Hogger and Moloch `Reset` skip `BossAI::_Reset`; ported with `_Reset` so wipes reset them.
- Teleport points outside the Stockade WMO faces fall to the void ([stockade-floor-fall](../../../../game-engine/docs/wiki/investigations/stockade-floor-fall.md)); live tests stand on creature spawn points.

## Related

[[creature-aggro]], [[creature-ai]], [[spell-casting]], [[auras]], [[zone-architecture]]
