# Dungeon simulator

Transport-free Stockade qualification runs a plain Bevy App inside the server test executable. It registers real combat, spell/aura, threat, scripted creature AI and follower systems directly. The game-server entry point, ServerPlugins and ProtocolPlugin are not used. [Contract](../../specs/dungeon-simulator.md).

## Native DPS parity adapter — 2026-10-21

The [DPS contract](../../specs/dps-parity.md) uses a separate ignored300s fixture over this App and `followers::simulator`. Exact reference item instances traverse production Stats; allocated and effective SimC talent ranks remain distinct. One actual NPC follower owns the damage measurement. Its native Player controller stays outside melee range, with zero controller damage required; a missing-Health controller had its attack intent removed on the first tick and produced misleading zero/two-hit runs.

Targets are native31146 training dummies with their real armor, level and health-loss script. The three-target adapter keeps its main dummy and normally summons/retires two adds every30s; receipts require10 waves,20 distinct add identities and actual retirement. Pet catalog/level-up tables are loaded explicitly; `runtime::build_pet_app` registers ordinary HunterPet/PetControl plugins before App finish/cleanup. Adding plugins to the already finished dungeon App is rejected by Bevy. Private native Duck174884 closure has a stable-key SQL delta/receipt, never a canonical mutation. At d698d1d3b, all three opt-in qualification cases pass: Hunter253 executes76 learned pet casts with141335 pet damage, Demonology266 executes50 with56163 pet damage, and Paladin66 keeps one main across10 waves/20 distinct adds with zero retired entities remaining. Each runs300s with sustained final30s damage and zero controller damage. These qualify execution, not DPS parity.

The current master merge has24 targeted GREEN cases including the original party initializer and equipment/link/source controls. Current d698d1d3b subsequently passes16 native300s execution cases (eight specs, single/timed-three);1467 remains a strict off-spec trait rejection. Timed-three is **blocked by FOLLOWER_ENGAGED_IGNORES_COMBAT_REFS**: actual31146 dummies have no threat list/current victim, while `followers::ai::engaged_enemies` recognizes the owner's victim, hostile victims of party members, or threat references—not CombatRefs. Only the main dummy is the owner's victim; Fire63/BM253/Feral103 produce identical single/three DPS, and Fire's add snapshots remain out of combat. Do not seed fake threat or alter dummy flags to conceal this admission gap. Rotation/AI changes remain lead-owned.

For the first three qualified scenario reports (253single/266single/66three),113 observed action spell IDs cover206 base effect rows. Pinned SimC70391 generated DBC versus native70448 rows agree in16 compared identity/type/scaling/coefficient/trigger/target fields. This excludes base-row drift for that bounded comparison, not different effective talent/hotfix/class-module arithmetic. Native stat sheets, profile options, observed action counts and pet/proc execution still differ; low DPS alone does not prove a coefficient bug. Normal SimC fight health phases versus a full-health native dummy, unmodeled consumables/item options, and remaining kit/class effects stay explicit measurement gaps. See [character stats](character-stats.md#instance-properties--2026-10-21) for equipment proof instead of treating nominal item IDs as equal effective gear.

The40-spec reference inventory now has30 selected shipped profiles (29MID2 plus the explicitly approved1467MID1 exception), with60 completed300s/1000-iteration/one-thread benchmarks. Ten specs have no selected reference;102/104 prior-tier candidates are not silently substituted. All30 three-target references have actual enemy-recipient evidence: positive target-effect uptimes or separate one-iteration logged positive damage to main/add1/add2. Targetless global buffs are not enemy recipients. The final JSON can still report `fight_style=Patchwerk`: pinned `sim.cpp2441–2443` changes the default enum after preserving explicit raid events, whereas the explicit Patchwerk case clears them at2317–2318. Qualify the actual adds/recipients, not this final label.

Eight single-target factor ledgers separate unobserved output channels, hit-event counts and per-hit residuals. Their accounting order is explicit, not a causal claim that all missing output is rotation or all per-hit difference is a coefficient bug. Shaman262 spell power is slightly above reference while major output channels are absent; exact344 weapon curves reproduce66/262 recorded ranges. `FISTS_OF_FURY_CHANNEL_MULTIPLICITY_GAP` goes in the opposite direction: existing native269 has525 child hits across21 casts on one dummy, versus the pinned SimC core's approximately5 hits/cast (`monk/sc_monk.cpp1793–1796`). The native113656 aura226 heartbeat is166ms; `auras/periodic.rs269–298` triggers each unscripted tick. Mechanics fixes remain separate from this diagnostic adapter.

Rare guardian action sample means are conditional on their own sample count. Full-fight ledgers use raw sample sums divided by the player's DPS sample count; otherwise Fenryr/Hati and some Demo guardians slightly overstate total reference action damage. Derived ledgers reconcile exactly to reference DPS×300; the maintained normalizer still needs this correction. Evidence under sibling `game-engine/data/diagnostics/dps-parity-2026-10-21/`: `all-reference-proof-summary.json`, `all-spec-comparison-current.{md,json}` and `measured-spec-single-factor-ledger.{md,json}`. Reference completion is not native40-spec parity.

## Real-player DPS bridge and fixed-time health —2026-10-22

The test-only bridge adds a real Player path for specs without follower kits. It reuses the pinned engine `CooldownAssistCatalog::recommend` and `assisted_combat::evaluate_rule` unchanged, then routes admitted choices through ordinary `cast_for_player` and its queue. Unknown potentially applicable source rules stop the attempt with rule ID/type/facts; absent follower profiles no longer imply absent native player support. Exact reference gear, level90, strict effective traits, native powers/passives and real pet availability remain required. The Arms spawn fixture passed; its first decision separately stops at rule1187/type12, not a successful cast or DPS result.

`SIMC_FIXED_TIME_TARGET_HEALTH` is fixture-only: before every50ms update it sets dummy health from the global300s100→0 clock, including newly replaced adds. It removes only fixture targets' native health-regeneration timer; template flags, armor, damage handling and production scripts remain unchanged. Full300s/three-target/replacement health and explicit player-rule-gap controls passed2/2 at564d46598. Lifecycle qualification does not establish three-target rotation parity.

At that exact revision, the policy-on single-target cohort has10 execution-qualified measurements: eight existing followers plus Fury72 and Enhancement263 players. Fury produces24346.17DPS with241Whirlwind casts; Enhancement3522.27DPS with200Windfury Weapon casts and one Lightning Shield. These are observed decisions/outputs, not authored rotation parity. Against the eight original health baselines, only Windwalker269 changes cast counts (Touch of Death322109:0→1); Protection66 and269 change hit counts. Protection's DPS delta includes per-hit differences, so two fewer endpoint hits alone do not explain it.

The current single-target receipt inventory is `all-spec-fixed-time-health-comparison-v2.{json,md}` under sibling `game-engine/data/diagnostics/dps-parity-2026-10-21/`:10 execution-qualified historical measurements,16 source/cast blocks, three strict-trait rejections, one full300s zero-output failure and10 without selected references. No selected-reference case remains unattempted. Earlier inventory versions and raw attempts remain intact. Companion JSON hashes each attempt/report/factor receipt, retaining the invalid-source64 and race-adapter251 attempts as history rather than acceptance. `fixed-time-inventory-consolidation-v2.proof.json` checks40 rows, the eight continuation snapshots/report hashes and all10 existing factor ledgers' input hashes/conservation. No new factor ledger is warranted for the eight blocked/nonqualified attempts; no causal mechanics fix or current-master DPS parity is claimed.

Startup readers use absolute worldDB aliases, so environment/cwd alone cannot freeze their source. `scripts/dps_parity_native.py` runs the compiled fixture in a bubblewrap read-only snapshot mount over the resolved startup path, with no unsandboxed alternate path. The process regression observed mutable data before the fix, then frozen/read-only bytes with unchanged host/snapshot files. Atc04ecc5b9, using isolated protocol2abc7fa (the then-landed canonical/master protocol), all five native controls pass158.48s: Human1, source file identity, full health clock/add replacement, explicit player rule gap and real-player gear/spawn. Locked all-target server check passes28.85s and formatting passes, with no warnings. The first post-merge native run had2passes/3startup failures because common and optional-pet paths registered PetControlPlugin twice; the existing sibling harness-only fix597d30ded was reused as c04ecc5b9, preserving one common registration. This changes no production pet mechanics. The Human adapter now requires native row1, matching pinned SimC; row33 is ThinHuman rather than another valid Human match.

The frozen-view continuation atc04ecc5b9 attempted exactly eight authorized single-target player cases, with unchanged input hashes and stable prepared snapshot throughout. All eight fail qualification; no repeat of the earlier10 positive-output cases was performed.

| Specs | Observed boundary | Proof level |
|---|---|---|
|64,265|Rule4512/type11; rule14223/type18 respectively|Explicit source-rule error;64 now has valid frozen-source provenance|
|268|Touch of Death322109 rejected by native admission as InvalidTarget|Actual admission refusal, no invented predicate/target health|
|577,581|Rules21553 and21486 respectively, both type12|Explicit source-rule error|
|251,1480|Node76073 requires76072 fully ranked; node108144 requires110172 fully ranked respectively|Strict reference-trait rejection; Human lookup no longer blocks251|
|267|300s,6000decisions,61Summon Felhunter691 casts, zero damage and `pet.present=false`|Raw output retained; positive-damage gate fails, not an accepted0DPS comparison or isolated summon diagnosis|

Reports and exact terminal proofs are `native-frozen-source-v2/<spec>.native.1.json` and `cohort-<spec>-player.proof.json`. Native3-target parity remains separate from these single-target/source-view controls and awaits the parent-owned CombatRefs admission integration.

For those later runs, native `combat.damage_by_target` records each measurement target's real damage and hit count, including retired adds and zero-output targets. Actor/spell action totals and casts are unchanged; heals, unrelated victims and targetless events are excluded. This is additive accounting, not an AI/admission change or proof that three spawned targets took damage. The concrete accounting test at77e8b0623 first failed on the missing recipient field, with the existing600damage total intact;96d695ee6 passes the unchanged test1/1 with no warnings. That unit proof covers accounting, not actual3-target execution. `pet.present=false` elsewhere is the primary-pet lookup result, not proof that no guardian entity existed.

## Continuous-edge seeded qualification —2026-10-22

Archived5f7d4ff85/P03cebdb/frozen40d95b0 clears **Stockade50/50,150native boss Deaths,0failures**, with actual34/1/level32 validated in all reports. Fresh seeds1–49/jobs2 take3083.914376s batch wall; separately retained same-code50 takes124.597531s individual wall. No contiguous50-run wall is claimed. `stockade-continuous-edge-5f7d4ff85-combined-summary.json` preserves source paths/hashes and both timing receipts.

Azure requests ten runs but finishes with **nine process panics/missing reports and one valid tank-absence block (seed7,1963sim_s)**,0clears,1323.826085s batch wall. All nine panic logs identify mover164v0 and disconnected frozen walk components after post-Leymor progression; do not count these as nine timeouts or nine valid reports. Evidence: `azure-continuous-edge-5f7d4ff85-seeds1-10/{summary.json,seed-*.log}`. The cohorts used a verified immutable executable, so the later449096a75/protocol2abc7fa integration did not mix their inputs.

## Qualified path-candidate Stockade cohort —2026-10-21

Production2f443fb5a, protocol03cebdb, frozen worldDB40d95b0, unchanged native Stockade32/default five: **50/50clears,150native boss Deaths,0failures**, with actual34/1/level32 verified in every report. Fresh seeds1–49/jobs2 clear49/49 in2575.069794s batch wall; same-code seed50 was already qualified by the full native regression (83.707545s individual wall) and is retained without rerunning it. There is no contiguous50-run batch wall. This qualifies that simulator cohort, not Azure or live acceptance. The subsequent mid-edge walking correction intersects these paths, so this is historical2f443fb5a evidence, not qualification of that correction.

`stockade-path-candidate-2f443fb5a-combined-summary.json` preserves every source report's path/SHA256, reached-pack denominators and separate timing receipts. The1–49 command and summary live under `stockade-path-candidate-2f443fb5a-seeds1-49`; seed50 remains under `stockade50-native-path-green`. [Azure qualification still fails](#post-path-azure-qualification-2026-10-21). No further production movement change has been made while diagnosing it.

## Fresh post-integration baseline —2026-10-21

Archived executable13610f262 (productioncb7a1ffe4, includes master8e3777ef6), protocol03cebdb, frozen read-only worldDB SHA256`40d95b0a21a243edcf12dc9a1fa1ca56ad4b45f2828026622d609d62fab5cca5`: Stockade32/default five/seeds1–10/jobs2 gives **10clears,30native boss Deaths,0failures**, all reports present,416.926s batch wall. Total27597.3sim_s/814.318run-wall_s=33.89× under shared-host contention. The prior corridor seed2 failure does not reproduce here; no deterministic-replay or causal-fix claim follows. This is baseline before the required-boss hold candidate and lead-owned model-default integration, not current-candidate acceptance. Evidence: `stockade-postmaster-baseline-13610f262/summary.json` and `stockade-postmaster-baseline-command.json` under the existing proof-ledger directory.

## Surviving-formation admission boundary —2026-10-21

The same frozen post-integration baseline gives Azure70/205/default five/seeds1–10/jobs2 **0clears,1wipe,2blocks,7timeouts,0boss Deaths**,786.026s batch wall, all ten reports present. Every timeout reaches formation29001816's living members but repeatedly reports `Projected source GUID 29001816 is not present in this run`. Movement now approaches surviving29001823; the next controller step still projected the complete frozen formation, including its observed-dead leader. Production source-presence accounting correctly excludes that leader.

The first focused attemptdd7847cda passed pre-fix because it killed the leader before admission initialized party history; it is not RED evidence. Corrected0cbc74372 admits the formation first, then executes native191164 Death, selects the living member and reaches actual native admission: genuine missing-GUID RED5.89s. Fix51c652a82 filters the controller footprint to living native formation members, retaining original formation identity and frozen route. Atfa7ce6714, corrected admission and retained living-member approach both pass22.52s. Same-input candidate Azure batch gives0clears,1wipe,3tank-absence blocks,6timeouts in603.004s; every missing-leader-GUID refusal is gone and six runs kill Leymor, then time out during later native progression within the unchanged1h bound. Stockade candidate retains10/10clears,30boss Deaths,0failures in523.276s. Full Azure reliability remains open. Evidence: `azure-postmaster-baseline-13610f262/` and `formation-admission-red2.log` under the existing proof-ledger directory.

The wipe and two tank-absence blocks require separate diagnosis. Immediately before tank deaths in seeds2/4/5, Crenna is full mana within1.2–2.7yd of Garrick, whose HP is338/72/1368 of40391. These three failures do not establish far-away healer positioning. Seed2's last600 native events before tank Death include9Regrowth successes,2Swiftmend,2Rejuvenation and2Lifebloom;15native411144 energizes restore375mana each. Full mana does not establish inactivity: healing continues until death against incoming native melee from three actors. No healer-policy or balance change follows from these samples alone.

## Henry movement after shared model defaults —2026-10-21

Frozen worldDB Henry215011 uses display121275 without a model-info row. Before the [shared model-geometry policy](model-geometry.md), native Henry remains exactly at the entrance after200sim_s while Garrick moves; focused movement RED8.63s and Henry-specific acquisition capture fails. No follower-specific geometry implementation or fabricated database row was added.

Main-owned merge5d464ebcb includes sharedmm871. Its native build passes8m58s without warnings; Henry200s plus both world/aggro scaled missing-row and authored-row preservation controls pass3/3 in64.58s. Henry moves to[100.57416,-25.57767,0.28981], has4230.20/4430mana and is in combat. This certifies the movement boundary, not an entire Henry dungeon cohort or healer-policy parity. Evidence: `mm871-henry-controls-command.json` and `.log` under the existing proof-ledger directory.

## Autonomous leader native sight hold —2026-10-21

Extending candidatefa7ce6714 to disjoint Stockade seeds11–50 yields38clears,1stall(seed28),1tank-absence block(seed33),118boss Deaths,1377.821s batch wall. Together with seeds1–10 this is **48/50clears,148boss Deaths**, not zero-failure acceptance. Seed28's Austin dies to Slag Fury20375735 around2145sim_s while companions remain roughly55yd behind; the run stalls2343sim_s. Exact `run(28)` reproduces that same boundary at586220889 (43.59s). Native checkpoint motion traces distinguish route MoveIntent from later AI pursuit; no synthetic positions or timers are used.

`positioning::hold_unsafe_approach` returned immediately for every unit lacking a literal `Follower` component. The autonomous owner carries `LeaderProfile`, so it bypassed the non-tank native-sight hold. A focused actual Slag Fury engagement from native source positions gives RED8.50s: the owner moves56.6821→73.39671 alongx in2sim_s despite idle watchers on the approach. Fix865989982 reuses the existing actual follower-role/active-specialization tank predicate; it adds no role type or alternate movement path. Focused GREEN7.31s, three retained native prevention/passive/accidental-combat controls GREEN21.92s, then exact seed28 clears all three native bosses at2421.4sim_s (52.78s test). Full reliability and seed33 requalification remain pending; no overall completion claim follows.

Evidence: `stockade-candidate-fa7ce6714-seeds11-50/`, `isolated-leader28-{red,motion,role-green}.log`, and `leader-sight-{red,green,native-controls}.log` under the existing proof-ledger directory. [Contract](../../specs/dungeon-simulator.md#authored-route).

## Post-path Azure qualification —2026-10-21

Candidate2f443fb5a, protocol03cebdb and the same frozen40d95b0 database, native Azure70/205/default five/seeds1–10/jobs2: **0clears,7DeadMember stalls,3tank-absence blocks,0timeouts,0boss Deaths**, all ten reports present,570.406761s batch wall. This is not reliability and does not preserve the prior role candidate's six Leymor Deaths. Seeds1/2/4/5/6/8/10 lose Austin; seeds3/7/9 lose Garrick.

Seed1's sample at1802.5sim_s shows Austin dead after native Bubbling Sapling29001840 damage, at[-5077.2725,561.16675,-1282.4409]. Crenna remains[-5057.316,581.2637,-1232.7773]; Garrick is moving on the upper floor. Nearby healer events include Regrowth and HoT healing, so full mana is again not inactivity proof. Combat-event Unix timestamps are wall time, not simulated seconds: the Death interval comes from sampled health. The native-order ignored regression32b89ca7a reproduces owner Death at1802.3sim_s (106.58s test). Its1765.4sim_s capture confirms29001853 is idle/fullHP with native18yd acquisition. The healer's fresh path overshoots the moved target through the original lower anchor then backtracks14.01yd. Garrick's actual cached travel descends that edge, but a fresh route from his mid-edge feet goes back to upper-floor nodes first; he is Idle without a cast. Separate recorded-position route/time controls qualify this endpoint boundary before any backend change. Reports alone did not authorize a speculative movement change; the subsequent native RED and recorded-edge controls established the endpoint/time boundary below. Evidence: `azure-path-candidate-2f443fb5a-seeds1-10/` and its command receipt.

## Continuous frozen walk endpoints —2026-10-21

Native seed1 Sapling owner-Death RED106.58s is followed by two recorded-position REDs ated5e93e8c (0.12s): the follow path enters29001853's18yd sphere at10.055763yd, and the mover does not arrive after its remaining81.82497native-yard budget. Both use the exact1765.4sim_s capture, not fabricated positions or elapsed combat time.

Walking now attaches endpoints to the closest full3D existing walk edge or node. An edge attachment contributes its proportional native distance to each endpoint; shortest-path search can connect those attachments directly along the same edge or through existing nodes. Timed movement and safety queries consume the same resulting legs. It no longer assigns an entire edge's distance after a partial-edge retarget. Isolated nodes remain candidates, so a nearby edge in a different component is not an invented connector. Directed book planning is separate and unchanged. Only sub0.01yd f32 projection roundoff is absorbed while retaining the actual endpoint positions; recorded errors were below0.002yd. At5f7d4ff85 the same-profile native build passes5m43 without warnings; both recorded-edge controls pass0.01s and are enabled as default regressions. Native Azure1 now survives through the Sapling's Death (110.78s test), while Stockade's recorded safe follow and fullseed50 both pass135.84s; seed50 clears all3bosses at2386.75sim_s. Four graph controls retain stacked native heights, walking disconnection and directed translocation behavior; the weighted-time control and six native leader/sight/passive/book/living-formation controls also pass. Fresh Stockade/Azure seeded qualification remains pending; this is bounded counterexample proof, not reliability.

## Disconnected chase parity —2026-10-22

The archived Azure panic maps mover164v0 to unselectable Polymorphic Rune195638/source29001884. Ordinary server chase movement treats a failed route as `Step::Blocked`, records `CannotReachTarget` and runs the existing five-second `no_path_evade_system` before native home arrival. The simulator omitted that lifecycle and turned a failed frozen path into a process panic.

The original55c880f8a fixture fails its setup assertion, not movement: first hop29001868→29001870 lands on the Rune's same508yd component. Corrected ignored539b08c09 uses native second book29002007→29001886 (508→426yd). Genuine RED11.79s identifies source29001884 at[-5353.02,508.083,-1117.58] chasing[-5336.61,426.341,-1113.19] and panics at movement's disconnected-component boundary. No fabricated positions or links are needed.

Simulator movement now records failure through the same production `mark_chase_blocked` helper and runs the actual `no_path_evade_system` after movement. Reachable travel removes the flag; native home arrival remains unchanged. No physical-navigation plugin, duplicate timer or Rune-specific policy is installed. Candidate1bf739d7a/protocol2abc7fa/frozen40d95b0 passes the corrected native Rune regression16.62s (build6m05, no warnings). Retained controls: Stockade50+safe-path2PASS127.01s, Azure1 Sapling1PASS128.30s, edge3PASS, graph4PASS/2extractors ignored, hazards7PASS22.16s, leader/watcher/passive/book/living-approach/admission6PASS. Stockade50 clears3bosses at2438.65sim_s/114.473run-wall_s; no full-cohort acceptance follows. The optional production `navigation_no_path_` family has3FAIL/1PASS, identically reproduced by the retained pre-parity executable; it is not green evidence and is not changed here. New seeded qualification is running. No hazard-policy change was made for sm909's separately confirmed canonical kit-parser drift. Receipts: engine `data/diagnostics/sim-azure-evade-2026-10-22/{inputs.json,controls-ledger.json,native-controls-ledger.json,pre-parity-native-nopath.log}`.

## Recorded late-healer route boundary —2026-10-21

After shared model defaults, revisiona55ce7383 reproduces Stockade seed50's tank loss at2360.60sim_s with two boss Deaths (65.037s wall). The reduced fixture uses its recorded healer/owner positions and four surviving idle watchers. Native acquisition geometry confirms the straight endpoint chord crosses all four spheres while `Space.route` avoids them; the healer nevertheless remains stationary. Both ignored regressions fail in76.66s. This isolates an incorrect chord check in `followers/positioning.rs`, not missing native geometry or an authored-route defect.

Positioning now checks complete movement-path legs using the same frozen route as simulator movement. Ordinary runtime reads authored navigation asynchronously under the same `Approach` query as hostile-field qualification and holds while pending or unavailable. Both checks share identical endpoints; an empty field set no longer cancels their shared probe. Navigation retains one continuation per actor, so separate sight/field queries would overwrite each other. Native radii and role-based tank exemption are unchanged. At2f443fb5a the same-profile native build passes4m26 without warnings; both RED regressions pass94.01s. Seed50 now clears all three native bosses at2384.55sim_s (83.708s wall), with its tank alive. Three retained leader/sight/passive controls pass21.22s; seven hostile-field controls pass13.60s and eight deferred-navigation controls pass1.20s. One fixed seed alone is not reliability; see [subsequent seeded qualification](#qualified-path-candidate-stockade-cohort-2026-10-21). Evidence: `stockade50-native-path-{red,green}-command.json` and `.log`, plus `path-{native-*,hazard-controls,deferred-navigation-controls}` in the existing proof-ledger directory.

## Required-boss phase hold —2026-10-21

A source can be alive but currently unattackable without counting as a completed boss. The route still selects other pullable packs first. If no such pack remains and a required boss actor is alive, the controller holds rather than returning an incomplete-boss error; when attackability returns, normal route selection resumes. An absent boss with no native Death receipt remains an error. No boss credit is fabricated.

The native Moloch damage→55964 Vanish→party CombatStop fixture reproduced the incorrect no-pack error at13610f262 (8.05s) andab6b77925 (12.41s), frozenDB SHA256`40d95b0a21a243edcf12dc9a1fa1ca56ad4b45f2828026622d609d62fab5cca5`, protocol03cebdb. Separate native controls prove other pullable packs take precedence and an absent boss cannot become an endless hold. Fix15447ad03 uses required boss entries from the frozen graph. At4a0a99f24, all three native hold/other-pack/absent-boss cases pass21.77s; nine admission cases pass18.38s and entrance multi-pack/reserve proof passes7.10s. The latter tests now separate cleared-ready admission from numerical overlap forecasts: full-ready single pulls need no invented forecast, real mana spend requires Recovery, and an idle23-pack footprint violates single-pack pacing. Raw30% reserve and finite-stat expiry remain independently asserted through the native forecast report. These bounded proofs do not establish seeded dungeon clears. [Contract](../../specs/dungeon-simulator.md#authored-route).

## Frozen geometry

`content/dungeons/stockade-34.json` contains 143 Normal source packs and 367 native-query walk-distance edges (70 KB). Pack identity matches source formations, never proximity clusters. Extraction retains a minimum spanning forest plus four nearest neighbours; every node is connected. Only the explicitly ignored extraction test opens the nav bake. Native source heights now remain intact for every graph (approved2026-10-21). Frozen node/entrance heights drive full3D nearest-node selection; native range/aggro retain floor separation. Travel consumes source speeds and frozen edge distances, not straight-line endpoint distance. Samples store Bevy[x,y,z]. No runtime navmesh, ground, LOS or physics is installed.

## Azure graph

`content/dungeons/azure-vault-2515.json` retains225 Normal source actors in214 native formation packs plus authored AreaPOI7116 as an empty topology node. Qualified staged navigation supplies540 walk-distance edges; [the shared production book policy](azure-translocation.md) supplies nine directed teleport links. Frozen66-pull boss-path corridor follows Leymor→Azureblade→Telash→Umbrelskul; native acquisition spheres and explicit Leymor gates determine pulls rather than the former179-pack all-hostile traversal. Two off-route source actors29001897/29001912 are nav-unreachable and remain named, not connected synthetically. Runtime uses `DUNGEON_SIMULATOR_MAP=2515`, native difficulty205, Normal population projection and `DUNGEON_SIMULATOR_LEVEL=70` (MapDifficulty5417→ContentTuning2155 range10–70). Default five profiles remain unchanged. Seeded clear proof pending.

## Boss-path corridor qualification —2026-10-21

`FOLLOWER_ROUTE_BOSS_PATH_CORRIDOR` uses deterministic shortest frozen paths through sourced boss order, includes native eligible acquisition-sphere intersections and explicit boss gates, and never exposes intermediate space across directed book hops. `scripts/freeze_follower_route.py` consumes frozen native profiles (`*-corridor-inputs.json`) plus `route-requirements.json`; source population changes require recapture. Native profiles capture maximum radius across the actual default party at Stockade32/Azure70, not guessed aggression distances.

Data commits507a7393f/70383c5a1 reduce Stockade99→59 pulls (40 removed, none added), preserving Moloch→Hogger→Overheat; boss steps25/44/58. Azure179→66 pulls (115 removed,2 added) reaches Leymor at23 rather than51; subsequent bosses42/64/65. Seven focused route tests pass, including real boss order/Leymor-first-floor checks and five algorithm cases. Compared with the former effective67-step completion prefix, the new59 includes nine formerly trailing packs and excludes17; therefore fewer planned pulls does not establish completion equivalence. Older batches below cover older routes only.

Stockade corridor batch13 at092a20a42/defaultfive32/seeds1–10/jobs3 gives9clears,1stall,29native boss Deaths,0wipes/blocks/timeouts/missing reports in330.938s. Seed2's Austin dies to20375677 Rumbling Earth at1899.5sim_s while the other four remain roughly44yd behind; terminal2015.95sim_s holds `Wait(DeadMember)` before20375735. This is a recorded orchestration failure, not a clear or a stat-impossibility claim; native pre-death tank-decision diagnosis remains pending. Azure corridor batch2 at the same revision/defaultfive70/205/seeds1–10/jobs3 gives0clears,6stalls,2blocks,2timeouts,0boss Deaths in549.997s with all ten reports. Seeds1/2/3/4/8/10 stop at29001816 after its first191164 dies: remaining formation members29001823/29001856 are alive roughly70yd away, but the fixed pack anchor still points to the dead first member. Seeds6/7 lose Garrick (fatal29001794 Conjured Lasher/29001825 Arcane Tender); native corpse despawn makes the missing-opener block explicit. Seeds5/9 retain active fights until the unchanged1h bound. The focused native first-member Death→living-member approach test gives genuine RED at9bb405a76: distance68.85183yd remains unchanged after20sim_s. Generic follow-up uses the living member's actual position once the pack leader is no longer selected, preserving formation identity and the initial leader anchor; GREEN remains pending. The exact-mainloop Stockade seed2 probe independently clears three bosses before this fix, so it does not reproduce or resolve the earlier seed2 failure; full-run deterministic replay remains unqualified.

The native Felguard last-hit/evade fixture reproduces a separate simulator defect: after40 simulated seconds the actor remains `Returning` because frozen movement had no home destination/arrival. Fix9451e194d adds frozen-edge home travel using native HomePosition→ScriptedPath.home→SpawnPoint priority and shares native resource/addon/pose/home-hook reset. Revision092a20a42 gives actual Felguard evade/home/Idle/Still GREEN11.57s, retained native return-to-spawn5GREEN and graph/census4GREEN. Native build5m47s exits0 without warnings. Seeded route qualification remains pending.

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
- [DPS adapter](../../../crates/server/src/dungeon_simulator/dps_parity/mod.rs), [native inputs](../../../crates/server/src/dungeon_simulator/dps_parity/inputs.rs), [pet source delta](../../../scripts/dps_parity_duck_174884.sql)
- [Route/report](../../../crates/server/src/dungeon_simulator/campaign.rs)
- [Frozen graph](../../../content/dungeons/stockade-34.json)

## See Also

- [[follower-data]] — full native class/spec kits and tank policies.
- [[character-stats]] — source-qualified instance gear and equipment auras.

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
