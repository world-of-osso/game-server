# Dungeon simulator

Transport-free Stockade qualification runs a plain Bevy App inside the server test executable. It registers real combat, spell/aura, threat, scripted creature AI and follower systems directly. The game-server entry point, ServerPlugins and ProtocolPlugin are not used. [Contract](../../specs/dungeon-simulator.md).

## Frozen geometry

`content/dungeons/stockade-34.json` contains 143 Normal source packs and 367 native-query walk-distance edges (70 KB). Pack identity matches source formations, never proximity clusters. Extraction retains a minimum spanning forest plus four nearest neighbours; every node is connected. Only the explicitly ignored extraction test opens the nav bake. Runtime positions are flattened to a plane; travel consumes source speeds and frozen edge distances, not the straight-line endpoint distance.

## Execution

Build the filtered server test executable through `scripts/build-host.py --test -p server --bin game-server --profile sim --no-run`. Pass Cargo's printed executable to `scripts/run-dungeon-simulator.py --test-executable PATH --output DIRECTORY`, via `agent-run stockade` after capacity admission. The runner supplies repository-root working context because gameplay Startup loaders use `data/world.db`. It never launches a server or connects a client.

Five sourced kits: Garrick, Crenna, Shuja, Meredy, Austin. Austin is the in-process leader using the same kit controller, not a self-owned follower. This explicitly differs from the older live Fury72 owner.

## Proof and current limits

- Graph census passed against actual Normal spawn GUIDs and formation IDs.
- Isolated opening pack20375759 cleared at5.70 simulated seconds, with221 real combat events, before full-route work. That fixture deliberately narrows the source population to one pack; it is not dungeon-clear proof.
- Full-population runs produce actual HP/mana/position samples, native pull decisions and exact source-actor boss death receipts. No Stockade boss clear is qualified yet.
- Route3 stalled at160.05s on20375765: trailing wall-time history erased DPS during an OOC forecast/recovery hold. User clarified ten **active-combat** seconds, retained across OOC and reset at dungeon start/full wipe. Production correction passes all five targeted history tests, including active-window rollover, OOC retention/healing exclusion and wipe/new-copy reset. Five-second HP stability still uses wall time. Observed dead-spawn knowledge survives a same-copy wipe.
- Native RNG scoped replay and batch-summary accounting pass. The first seeded batch was incomplete: seed1 timed out at3600sim_s/505.211wall_s (7.13×), seed2 interrupted, no batch rate. A partial perf checkpoint measured11.4× steady speed: setup22.90s, update42.50s at10001 ticks, observed events7576, memory plateau1.106GB/no swap. No low-wipe-rate or live acceptance claim.
- Native-state inspection found the manual constructor omitted `CreatureDifficulty`; Garrick's real admission refused with that explicit error. The regression reproduces it at the native movement-consumer boundary. Population creation now uses production `zones::spawn_map_creatures`, not a simulator copy; unchanged consumer regression GREEN at50cc09c7e (33.21s). The optimized `sim` profile builds successfully after merging master27743876c and pinning protocol03cebdb.

## Catalog lifetime

Seeded batches move 24 typed immutable catalog resources out of each completed App after diagnostics, then install those same allocations into the next fresh App. Native plugin Build still registers all systems/observers and fresh queues, clocks and instance state; cached Apps skip catalog-loading Startup. No `WorldData` or `SpellStore` clones, no test-harness global catalog copies, no reused dynamic World. Optional native resource absence is preserved. First load calls the production world/spell loaders directly. The allocation/state-reset regression first failed with distinct spawn allocations (optimized profile, 7.33s); GREEN at497be2f37 (6.39s); optimized batch2 at2fa2b68d5/P03cebdb executes all ten seeded worlds with the cache. Setup drops from seconds on first load to milliseconds thereafter.

## Seeded batch2 — 2026-10-20

All ten runs completed in201.90wall_s; no boss died. This is failure evidence, not clear qualification. Native dead summons despawn after corpse decay: samples now retain their admitted IDs with `exists=false` and null observations (kill→native decay→sample RED8.34s→GREEN11.23s).

| Seed | Clear / wipe / stall | Other outcome | Failed pack | sim:wall |
|---|---|---|---|---|
|1|0 / 0 / 0|3600s timeout; repeated Moloch Polymorph hold|20375718|50.0×|
|2|0 / 0 / 0|Garrick dead205s, later despawned; no tank route anchor|20375765|41.7×|
|3|0 / 0 / 1|Scripted boss requires explicit pull decision|20375766|39.7×|
|4|0 / 0 / 0|3600s timeout; repeated Moloch Polymorph hold|20375718|73.2×|
|5|0 / 0 / 0|Garrick dead223.5s, later despawned; no tank route anchor|20375765|102.7×|
|6|0 / 0 / 0|3600s timeout; repeated Moloch Polymorph hold|20375717|101.9×|
|7|0 / 1 / 0|Garrick dead143s, remaining party then dies; Thug still7644HP|20375765|93.4×|
|8|0 / 0 / 1|Scripted boss requires explicit pull decision|20375766|85.2×|
|9|0 / 1 / 0|Garrick dead144.5s, remaining party then dies; Thug still8196HP|20375765|100.3×|
|10|0 / 0 / 1|Scripted boss requires explicit pull decision|20375766|89.7×|

Totals:0clears,2wipes,3stalls,3timeouts,2blocks. Ratios vary with host contention; ≥100× is not consistently qualified. Earlier diagnostic seed1 achieved94.59× overall/111× steady. Seeds1/4/6 hold a full-health Polymorphed Moloch indefinitely; the exact generic cause remains under investigation. Seeds3/8/10 correctly refuse to invent scripted forecasts. Seeds2/5/7/9 need native damage/heal event causality before a production fix. Crenna selecting Meredy may be ordinary healing pursuit, not proof of hostile party targeting. Complete terminal combat events are now serialized for diagnosis.

Evidence: `/home/osso/.worktrees/logs/stockade-simulator-2026-10-20/sim-batch2/{summary,run-1..10}.json`, revision2fa2b68d5; event-trace addition requires a new diagnostic batch.

## Sources

- [Simulator spec](../../specs/dungeon-simulator.md)
- [Follower policy](../../specs/follower-dungeons.md#shared-tank-pull-project-policies)
- [Runtime](../../../crates/server/src/dungeon_simulator/runtime.rs)
- [Route/report](../../../crates/server/src/dungeon_simulator/campaign.rs)
- [Frozen graph](../../../content/dungeons/stockade-34.json)

## See Also

- [[follower-data]] — full native class/spec kits and tank policies.

## Ready pulls and recovery — 2026-10-20

User policies `SCRIPTED_BOSS_PULL_WHEN_READY` and `CLEARED_STATE_ALWAYS_PULLS_NEXT_PACK` now admit the next single authored formation when the complete admitted party is alive, in its copy, out of combat and at full HP/mana, with a healer and no explicit target hazard. No numerical forecast is fabricated; full positive tankHP preserves the30% reserve. Overlap still uses native forecast composition. Exact native regressions: scripted boss refusal and short-fight→cleared→Wait(Measurements) RED2/2 at65bb1e5f1, GREEN2/2 at aefef7da6 (13.56s), including no-add and dead-healer controls.

Batch4 at aefef7da6/P03cebdb executed seeds1–10:0clears/0wipes/8stalls/2timeouts/0blocks. Seeds4/10 retain the Moloch Polymorph loop; the other eight stop during real OOC mana recovery. Seed3 mana185.5→2193.2 over60s proves the former no-damage/no-leader-movement stall rule was wrong. Resource gains now count as party progress; a full-resource stationary party still does not manufacture progress. Native Regrowth-spend→OOC-recovery regression RED plus no-change control PASS at3d402c97b (14.03s); GREEN pending. Evidence `sim-batch4/` in the current diagnostics log directory.

### FOLLOWER_REFRESHMENT_NPC_CONSUMER_UNIMPLEMENTED

Drink audit verified local `follower_spec_kit`: Fire63 has utility190336, minimum level5. `spell_effect`190336 is only Dummy3/self target, no item or triggered child. Pinned [TrinityCore a352b1fa spell_mage.cpp526–551](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/scripts/Spells/spell_mage.cpp#L526-L551) runs its callback only for a Player caster, choosing116136 solo or167145 when grouped. Meredy remains an NPC under the approved player-spec bridge; its refreshment script, conjured-item/table consumer and follower bags are unimplemented. A vendor water definition is not evidence that Crenna owns an item. Do not grant water, tag the NPC as a Player, or cast an unsourced consumption spell. Plain native OOC mana regeneration remains active; no drinking-speed benefit is claimed.
