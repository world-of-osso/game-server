# Dungeon simulator

Transport-free Stockade qualification runs a plain Bevy App inside the server test executable. It registers real combat, spell/aura, threat, scripted creature AI and follower systems directly. The game-server entry point, ServerPlugins and ProtocolPlugin are not used. [Contract](../../specs/dungeon-simulator.md).

## Frozen geometry

`content/dungeons/stockade-34.json` contains 143 Normal source packs and 367 native-query walk-distance edges (70 KB). Pack identity matches source formations, never proximity clusters. Extraction retains a minimum spanning forest plus four nearest neighbours; every node is connected. Only the explicitly ignored extraction test opens the nav bake. Native source heights now remain intact for every graph (approved2026-10-21). Frozen node/entrance heights drive full3D nearest-node selection; native range/aggro retain floor separation. Travel consumes source speeds and frozen edge distances, not straight-line endpoint distance. Samples store Bevy[x,y,z]. No runtime navmesh, ground, LOS or physics is installed.

## Azure graph

`content/dungeons/azure-vault-2515.json` retains225 Normal source actors in214 native formation packs plus authored AreaPOI7116 as an empty topology node. Qualified staged navigation supplies540 walk-distance edges; [the shared production book policy](azure-translocation.md) supplies nine directed teleport links. Authored179-hostile-pack route follows Leymor→Azureblade→Telash→Umbrelskul. Two off-route source actors29001897/29001912 are nav-unreachable and remain named, not connected synthetically. Runtime uses `DUNGEON_SIMULATOR_MAP=2515`, native difficulty205, Normal population projection and `DUNGEON_SIMULATOR_LEVEL=70` (MapDifficulty5417→ContentTuning2155 range10–70). Default five profiles remain unchanged. Seeded clear proof pending.

## Native-height acceptance and Azure baseline —2026-10-21

Revision`a4c03c0b7`, protocol03cebdb, optimized native executable. Stockade batch12/default five/level32/seeds1–10/`--jobs3`: **10clears,30boss Deaths,0wipe/stall/block/timeout**,371.463s total batch wall time. Both frozen graphs pass source-leader/entrance height equality checks; stacked floors and directed teleport refusal pass native fixtures. Source heights do not invalidate Stockade completion.

Azure batch1/default five/level70/difficulty205/seeds1–10/`--jobs3`: **0clears,0wipes,10stalls,0blocks/timeouts,0boss Deaths**,220.861s total batch wall time. All ten native reports exist; worker101 reflects each failed-clear assertion, not a missing worker/report.

| Seeds | Terminal cause | Failed pack |
|---|---|---|
|1,2,5,7,9|Four admitted members die; Meredy survives. Fatal sources include Arcane Tender29001796, Conjured Lasher29001801 and Global Affix Stalker29001806.|29001796|
|3,4,6,8,10|All five survive; ready party remains held by alive, unattackable Global Affix Stalker history after combat flags disappear.|29001821|

All ten runs contain actual171322 melee receipts (5–168 party hits). Source171322 carries uninteractible flag0x02000000, faction14, blank AI/script and flags_extra0. TC's watcher-side `CanStartAttack` does not reject uninteractibility; the party's target-side `IsValidAttackTarget` does. Do **not** invent a per-entry passive exclusion. Native qualification now proves own player sight: at0.85sim_s Austin crosses within7.401693yd of the stalker's actual7.5yd radius. The native `player proximity acquisition` receipt identifies source29001806; no recruitment divergence is established, so its native aggression stays unchanged. A separate native incoming-damage→valid187159 Whelp damage/Death history fixture gives genuine RED while171322 remains alive/unattackable. Native-validity-filtered history/clear logic is the follow-up. No stat tuning or budget-impossibility claim.

Evidence under `/home/osso/.worktrees/logs/stockade-simulator-2026-10-20/`: `sim-batch12-native-heights/`, `azure-batch1/`, and the prior single-seed calibration `azure-native-probe1/`. The calibration is not the ten-seed proof. No book was reached; the [production endpoint policy](azure-translocation.md) passes native ingress/party tests but not full seeded traversal yet.

## Execution

Build the filtered server test executable through `scripts/build-host.py --test -p server --bin game-server --profile sim --no-run`. Pass Cargo's printed executable to `scripts/run-dungeon-simulator.py --test-executable PATH --output DIRECTORY`, via `agent-run stockade` after capacity admission. The runner supplies repository-root working context because gameplay Startup loaders use `data/world.db`. It never launches a server or connects a client.

Five sourced kits: Garrick, Crenna, Shuja, Meredy, Austin. Austin is the in-process leader using the same kit controller, not a self-owned follower. This explicitly differs from the older live Fury72 owner.

## Proof and current limits

- Graph census passed against actual Normal spawn GUIDs and formation IDs.
- Isolated opening pack20375759 cleared at5.70 simulated seconds, with221 real combat events, before full-route work. That fixture deliberately narrows the source population to one pack; it is not dungeon-clear proof.
- Full-population runs produce actual HP/mana/position samples, native pull decisions and exact source-actor boss death receipts. Current seeded qualification is recorded below; earlier run failures remain historical evidence.
- Route3 stalled at160.05s on20375765: trailing wall-time history erased DPS during an OOC forecast/recovery hold. User clarified ten **active-combat** seconds, retained across OOC and reset at dungeon start/full wipe. Production correction passes all five targeted history tests, including active-window rollover, OOC retention/healing exclusion and wipe/new-copy reset. Five-second HP stability still uses wall time. Observed dead-spawn knowledge survives a same-copy wipe.
- Native RNG scoped replay and batch-summary accounting pass. The first seeded batch was incomplete: seed1 timed out at3600sim_s/505.211wall_s (7.13×), seed2 interrupted, no batch rate. A partial perf checkpoint measured11.4× steady speed: setup22.90s, update42.50s at10001 ticks, observed events7576, memory plateau1.106GB/no swap. No low-wipe-rate or live acceptance claim.
- Native-state inspection found the manual constructor omitted `CreatureDifficulty`; Garrick's real admission refused with that explicit error. The regression reproduces it at the native movement-consumer boundary. Population creation now uses production `zones::spawn_map_creatures`, not a simulator copy; unchanged consumer regression GREEN at50cc09c7e (33.21s). The optimized `sim` profile builds successfully after merging master27743876c and pinning protocol03cebdb.

## Catalog lifetime

Seeded batches move 25 typed immutable catalog resources out of each completed App after diagnostics, then install those same allocations into the next fresh App. Native plugin Build still registers all systems/observers and fresh queues, clocks and instance state; cached Apps skip catalog-loading Startup. No `WorldData` or `SpellStore` clones, no test-harness global catalog copies, no reused dynamic World. Optional native resource absence is preserved. First load calls the production world/spell loaders directly. The allocation/state-reset regression first failed with distinct spawn allocations (optimized profile, 7.33s); GREEN at497be2f37 (6.39s); optimized batch2 at2fa2b68d5/P03cebdb executes all ten seeded worlds with the cache. Setup drops from seconds on first load to milliseconds thereafter.

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

Batch4 at aefef7da6/P03cebdb executed seeds1–10:0clears/0wipes/8stalls/2timeouts/0blocks. Seeds4/10 retain the Moloch Polymorph loop; the other eight stop during real OOC mana recovery. Seed3 mana185.5→2193.2 over60s proves the former no-damage/no-leader-movement stall rule was wrong. Resource gains now count as party progress; a full-resource stationary party still does not manufacture progress. Native Regrowth-spend→OOC-recovery regression RED plus no-change control PASS at3d402c97b (14.03s); GREEN2/2 at1cf5605d4 (10.99s). Evidence `sim-batch4/` in the current diagnostics log directory.

### FOLLOWER_REFRESHMENT_NPC_CONSUMER_UNIMPLEMENTED

Drink audit verified local `follower_spec_kit`: Fire63 has utility190336, minimum level5. `spell_effect`190336 is only Dummy3/self target, no item or triggered child. Pinned [TrinityCore a352b1fa spell_mage.cpp570–597](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa/src/server/scripts/Spells/spell_mage.cpp#L570-L597) runs its callback only for a Player caster, choosing116136 solo or167145 when grouped. Meredy remains an NPC under the approved player-spec bridge; its refreshment script, conjured-item/table consumer and follower bags are unimplemented. A vendor water definition is not evidence that Crenna owns an item. Do not grant water, tag the NPC as a Player, or cast an unsourced consumption spell. Plain native OOC mana regeneration remains active; no drinking-speed benefit is claimed.

## Moloch breakable-control cycle

After recovery accounting, batch5 at1cf5605d4 executes all10seeds to the3600sim_s bound:0clears/0wipes/0stalls/0blocks/0bosses,40–43Polymorph casts per seed. Total36,000sim_s/319.69wall=112.61×. Native hold protects118; Moloch regenerates, then Meredy re-casts the same control at the next scripted cast after expiry. No tuning of damage, costs, timers or native10%-maxHP break budget is justified. Pinned native template CreatureImmunitiesId=0 and boss script provide no Polymorph-immunity override.

An isolated real Moloch/full-five-kit regression reproduces the offending last-enemy Polymorph (RED1vs0 at f2eb2d4cc,5.07s), with actual86726 channel success as the control opportunity. Kit admission now excludes damage-breakable control only when exactly one enemy is engaged; kicks/nonbreakable stuns remain unchanged, as do protection/expiry of existing CC. Metadata uses the same AnyDamageMask as live aura protection. Multi-enemy Mage/Monk control fixtures retain their real interrupt→spent-kick→control behavior with a second engaged native actor. GREEN and new full batch pending.

## Native leader and encounter state gaps after batch6

CC fix4976ac6e1 passes the isolated Moloch regression plus retained CC families:12PASS20.88s. Batch6 then kills Moloch in6/10seeds; seeds7/10 also kill46254. No complete clear yet:0wipes,3stalls,5blocks,2timeouts,14,957.9sim_s/177.62wall=84.21×.

|Seed|Outcome|Boss receipts|Observed cause|
|---|---|---|---|
|1|stall|46383|All companions recover, Austin stays7581/7840HP|
|2|block|none|Vanish at54299/134429HP treated as InvalidTarget opening|
|3|block|none|Vanish at53788/134429HP treated as InvalidTarget opening|
|4|block|none|Vanish at54308/134429HP treated as InvalidTarget opening|
|5|block|none|Vanish at54356/134429HP treated as InvalidTarget opening|
|6|block|46383|Dead/despawned Garrick, no route anchor; recovery/res gap remains|
|7|timeout|46383,46254|Further movement/recovery diagnosis pending|
|8|stall|46383|All companions recover, Austin stays7565/7840HP|
|9|stall|46383|All companions recover, Austin stays7376/7840HP|
|10|timeout|46383,46254|Further movement/recovery diagnosis pending|

Austin was manually given PowerPools by the follower initializer before native `init_player_powers`. Its Without<PowerPools> query consequently never installed PlayerHealthRegen. The native wound→OOC test reproduces7740!=7840 (RED5a9341f6c). The seam now invokes that actual production system before profile initialization, instead of copying a timer/formula or granting health. Native recovery and retained lone-boss CC regression both GREEN at b3a5db1c9 (2PASS in18.38s composite).

Moloch retains real health during55964 and temporarily clears combat/attackability. Those are not Death receipts or a cleared encounter. A naturally isolated fight did not reproduce the full route's concurrent cast/cleanup timing. The regression now drives ordinary native CombatStop (the follower regroup boundary) after real damage-triggered55964, with the boss still alive/unattackable and the party OOC: RED1new pull vs0 at d46a72f32 (6.09s). Native history already retained active enemies until Death; its snapshot now exposes that set to cleared-state/overlap admission, and the simulator controller uses the same validated native history before issuing a new pull. Lost detection or CombatStatus cannot masquerade as encounter completion. GREEN8/8 at358cb0756 (37.39s), including ready-pull policies and the full five-case native history family.

## Extra recruitment: implicit area in a buff row

Current opening regression remains RED before any generic fix (358cb0756,6.77s). Diagnostics falsify stale level/radius theories: all affected sources see tank32, and both native acquisition and projected radii are14.5yd while the new hostile units are~25yd away. Self-backtrace instrumentation (no ptrace) traces zero threat to `push_hit_events`, not sight acquisition. Exact spell375576/Divine Toll selects seven hostile units in native implicit effect1; its authored kit row is buff/self with `min_enemies=0`. The previous area guard ran only for positive min-enemy thresholds, so native target hits recruited six new source packs before damage.

Zero-threshold entries now inspect supported native implicit-area unit selections before hit rolls, using native caps and the same enemy/CC rule as existing area safety. Friendly areas and non-area buffs are not blocked; no375576 blacklist, radius/priority edits, or native threat suppression. The full-population opening fixture stops after the actual opening source Death receipt so later legitimate pulls are not mistaken for bootstrap violations. A positive Garrick/Divine Toll fixture retains the native cast when its area is already engaged. Temporary trace logging is removed. GREEN and new batch pending.

## Authored route, combat-only areas and first clears —2026-10-20

User-required authored route now freezes99hostile pull steps/graph-source anchors in the143-node topology. Friendly guards/prisoners/noncombat stalkers remain movement nodes only. Controller never selects by proximity; accidental combat takes precedence and already-dead route packs are skipped. Route order/skip native tests GREEN2/2 at0d4d493bb,11.06s; explicit unique/finite-anchor graph test passes. An absent tank no longer erases the route anchor, but cannot cast a fabricated opener.

Production target selection implements explicit `AOE_HITS_ONLY_IN_COMBAT_UNITS` policy for375576 hostile area and Starfall50286 periodic selector, before native caps. No recognized combat-only flag was found in local70448 rows or pinned TC a352b1fa source; this is named project policy, not a new SpellAttr. Native5-case RED3FAIL/2controls atb500b4a9c→GREEN5/5 at637294cb4,10.38s. Earthquake61882, Divine Storm53385 and Chain Lightning188443 pull controls remain. Superseded zero-min kit guard removed. Opening exposure regression still fails later at4s on a healer-targeted20375760 zero-threat engagement, not prior2.55s six-pack Divine Toll recruitment; no all-pulls isolation claim.

Batch8 at0d4d493bb/P03cebdb gives first6clears (1,5,6,8,9,10),0wipes/blocks/timeouts and4stalls (2,3,4,7) at20375683. Exact common cause: native Hogger Death then86604 bleed damage from the dead Hogger reinserts it into history active enemies. Real86604 cast→kill→positive postmortem periodic damage regression RED at e2ce3f47f,9.14s. Production history only adds a living source identity; bleed damage remains unchanged. GREEN7/7 at5cdcdb5a6,16.90s includes temporary living Moloch Vanish and all five history cases.

Batch9 at5cdcdb5a6/P03cebdb: **10/10clears,0wipes/stalls/blocks/timeouts**, all30real boss Death receipts.27794.5sim_s/288.402run-wall=96.37×; whole test291.49s=95.35×. Cold setup9.147s; warm seeds mostly101–105×, seeds8/10=99.96/97.02×. Consistent inclusive100× and final live acceptance remain open. Reports/trace ledger: `/home/osso/.worktrees/logs/stockade-simulator-2026-10-20/sim-batch9/`.

### Larger batch and cargo-test gate

Atdeb62e43e/P03cebdb (master070ae0a97 merged), seeds1–50 all clear:50/50,0wipes/stalls/blocks/timeouts,150boss Deaths.138186.35sim_s/1925.606run-wall=71.76×; whole test1947.57s=70.95× under shared contention. Median2768.225sim_s, range2602.75–2930.5. This qualifies the measured50seed sample, not an unobserved zero-failure probability.

Opening oracle now proves actual20375760 accidental acquisition, positive damage, Death and route skipping. Native producer trace: `creature_sight_system`→`attack_start(0)`, Crenna15.7928yd from watcher inside native16yd. Her reach3yd differs from Garrick1.5yd; prior tank-only14.5yd diagnostic was not this acquisition. Positioning avoidance is the next authorized change, not a threat fix.

Cargo-test gate exposed cwd-relative Startup catalog loading and a stale Overheat oracle: real Meredy Counterspell2139 legitimately ends12466 on the first tick, while all10unengaged-route movement refusals remain. Test catalog paths now resolve fromCARGO_MANIFEST_DIR; production path unchanged. Updated oracle permits cast disappearance only with the actual2139→12466 interrupt receipt. At570e080b5 full `followers::`+`dungeon_simulator::` through Cargo gives278PASS/1environmentFAIL/10ignored456.44s; sole failure is Leymor's explicit private2515asset receipt prerequisite, waived by lead. No behavior gate remains from the former cwd/Overheat failures.

## Non-tank sight-envelope positioning

Production non-tank combat/heal pursuit and idle regroup now inspect live same-copy unpulled aggressive hostiles using the existing native radius calculation (levels, scaling and both reaches). If an approach segment would enter a sight circle, keep the current position and continue any native cast valid from there. Tank planning is unchanged; no pack-specific offsets or radius changes. This conservative chooser holds unsafe approaches; it does not invent alternative terrain paths. Exact natural20375760/Crenna regression RED15.7928<16yd at3.55s→GREEN, with deliberate native accidental-attack handling control retained (2PASS12.39s).

Post-change batch11 atf3b7b8295/P03cebdb:10/10clears,0wipes/stalls/blocks/timeouts,30boss Deaths.27601.6sim_s/441.435run-wall=62.53×; whole process445.98s. Prior50seed result remains explicitly pre-positioning. Throughput is secondary per user; unchanged target still open.
