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

## SmartAI combat casts

`smart_ai.rs` previously supported only SPELLHIT, HAS_AURA, RESET and LINK, and only self/invoker targets. Kobold Geomancer 476's UPDATE_IC (0), CAST Fireball 20793 with CMC (64), VICTIM (2) row was loaded but never processed. Combat updates now initialize from event params 1/2, pause outside combat, and repeat from params 3/4. Due casts wait while another cast is running.

CMC accepted casts insert `CombatMovementDisabled`; chase holds position through the repeat interval, without losing the victim or disabling melee in reach. A failed cast removes it and retries next update, preserving the chance result. New AI clears it (TC constructor defaults movement on); ordinary Reset retains movement permission. Existing `CastState` movement/swing gating still applies during casts. Real-template/spawn/spell/script tests exercise Fireball targeting and repeats, position/Motion, exhausted mana, school-lock rejection, melee recovery and prompt retry.

Impact (world.db, verified 2026-10-05): 26,854 UPDATE_IC rows / 9,796 distinct entryorguid values, 1,730 CAST+CMC rows / 1,566 values. Positive source-0 SmartAI rows actually loaded by the server: 9,371 / 5,537 creature entries and 1,411 / 1,260 respectively. Other unsupported event/action/target types remain unsupported; these counts are mechanism exposure, not blanket script completeness.

### Sources

- [TC a352b1fa SmartScript.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/AI/SmartScripts/SmartScript.cpp#L565-L637): CAST result, CMC movement (612–615), retry; UPDATE_IC (3201–3204), InitTimer/UpdateTimer (3751–3806), VICTIM (2791–2794).
- [TC a352b1fa SmartAI.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/game/AI/SmartScripts/SmartAI.cpp#L782-L815): SetCombatMove; UpdateAI (281–299) refreshes victim first; AttackStart (572–591) respects combat-movement permission.
- `crates/server/src/scripted_ai/tests/smart_combat.rs` — real database behavioral regression.

## SmartAI escort coverage (2026-10-11)

Wizzle3439 fixed-schedule reproduction found two drivers moving the same idle actor: SmartAI point motion and spawn wandering. The extra wander stride was0.125 yards at20Hz. Point movement excludes idle roaming; escort ownership persists through pauses/delays and ends with the path. Debug driver logs distinguish point/wander, spawn-leash/home return and escort-distance failure. A second integration RED reproduced point motion advancing concurrently with combat chase (0.124355-yard extra deviation). Active escort points now suspend for non-idle combat/home states. An instrumented live follow still stalled at1106.9545/82.21661/3099.6099; the cart disappeared and reappeared at spawn120seconds later, without an observed spawn-leash or escort-range trace. Death/respawn versus return remains to be distinguished by the added death/chase/state traces. Neither regression proves full live escort completion. [Contract](../../specs/smartai-escort.md), regression `smartai_coverage_wizzle_point_movement_excludes_spawn_wander_in_fixed_schedule`.

Quest accept/reward hooks execute source-9 lists and waypoint paths. Timed lists honor OOC/IC/unconditional gates, initial min/max delay, chance and override policy; TC explicitly forbids replacing the currently executing list from itself (SmartScript.cpp:4296–4327). Waypoint tables supply orientation and node delays; repeated paths restart at node one. Waypoint event point-ID wildcard is UINT32_MAX, not zero (SmartScript.cpp waypoint case). Wizzlecrank's authentic three-leg path chain exercises this boundary. Generic escort death/distance failure no longer depends on an explicit DEATH script row. Party targets use current group members in the invoker's map, not just the invoker.

Private-copy [SmartAI overlay instructions](../../specs/smartai-escort.md) reconcile legacy ScriptName selection without replacing stats/spawns. Force-despawn uses a deferred timer without death credit or loot. Spawn creatures remain hidden awaiting their normal or explicitly forced respawn; temporary summons notify their summoner. Successful waypoint stop/end credits its quest and runs the requested hide-then-five-second despawn lifecycle (SmartAI.cpp:984–1001). SUMMON_CREATURE uses template spawning, authored absolute/relative positions, count, stored lists and attack/prefer-invoker flags, with the eight pinned temporary-summon lifetime modes (ObjectDefines.h / TemporarySummon.cpp). Creature SUMMON_CREATURE_GROUP (107) loads exact summonerType=0 TDB rows, including count-by-row, positions, lifetime, optional invoker attack and stored targets. Existing world.db lacks that support table; the explicit `--tdb-world-sql` overlay import supplies it from the local pinned dump. Missing table/group is diagnosed and that group action remains disabled; other SmartAI vocabulary still loads. Personal/spell-property summons remain explicitly diagnosed, not substituted. Closest-creature targets filter entry, distance, map and pinned alive/feign states; TALK can use the selected creature as speaker. FOLLOW updates a relative destination and cancels on an empty target; endpoint arrival follows pinned StopFollow's absent player credit (GUID is cleared before lookup), with its event and fade/despawn. Action152 credits event/area objectives and auto-rewards tracking-event server flags even without prior acceptance; rewarded flags are idempotent. The authentic tracking row50804 has no content template yet, so its flag persistence is fixture-proven, not a live area-trigger claim. Remaining: broader target/action vocabulary and both-skin full quest live proof. No full escort parity claim.

Sources: `crates/server/src/scripted_ai/smart_{ai,timed,paths,escort,targets,lifetimes}.rs`; real-row and table fixtures in `smart_escort_tests.rs`; pinned TC SmartAI.cpp:185–259,549–554,912–950 and archived SmartScript.cpp in engine `data/diagnostics/smartai-coverage-2026-10-11/`.

## Wailing Caverns — Lord Cobrahn (2026-10-11)

`wailing_caverns.rs` identifies only Cobrahn3669 as instance data0 / encounter586. Local `tdb_smart_scripts` death row0 sets data0→DONE3; `dungeon_encounter` names the same creature on map43/Normal1. `SmartAi` retains the authored spell timers and adds optional `BossAi` lifecycle; the existing hook boundary emits encounter/frame messages and death credit, with no separate spell schedule or new persistence model. AGGRO4 runs TALK1 using existing CreatureText selection.

Sources: read-only canonical world.db `tdb_smart_scripts`3669, `content_creature_text`3669, `dungeon_encounter`586; [contract](../../specs/boss-scripts.md); real-data `scripted_ai/tests/wailing_caverns.rs`. Full escort/doors/other bosses and unsupported script actions remain separate gaps. [Native proof and blockers](../../specs/boss-scripts.md#cobrahn-native-proof--2026-10-11) records both-skin entry/pull/death/credit, absent actor4213 and a reproduced population-copy loot-binding loss. Population gate/importer is a separate dependency, not duplicated here.

## Pitfalls found

- Removing `NetworkVisibility` to hide an entity made it visible to every client that connected later (`on_replication_client_added` only hides entities that have it): all invisibility-aura creatures of both continents replicated to new clients, and the client grew to 36 GB until earlyoom killed it. Hiding is an interest flag now. Decayed corpses still use the old removal.
- TrinityCore's Hogger and Moloch `Reset` skip `BossAI::_Reset`; ported with `_Reset` so wipes reset them.
- Teleport points outside the Stockade WMO faces fall to the void ([stockade-floor-fall](../../../../game-engine/docs/wiki/investigations/stockade-floor-fall.md)); live tests stand on creature spawn points.

## Related

[[creature-aggro]], [[creature-ai]], [[spell-casting]], [[auras]], [[zone-architecture]], [[dungeon-achievements]]
