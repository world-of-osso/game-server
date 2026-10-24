# Shared literal precombat lifecycle

2026-10-23; implementation pending. Native Demo first-engagement production RED read and qualified below; local four-case packet runtime pending. [Contract](../specs/dps-parity.md#literal-precombat-execution).

## Observed boundary and scope

Current prepare_reference advances four native frames before explicit fixture engagement. measure then engages before advance_combat loads the WW controller/precombat. Feral's ordinary kit can cast1126 at0.05s before takeover. Pinned optimal_raid0 has no such buff; capture must precede warmup, not erase its aura later. Current Profile also ignores top-level warlock.soul_shards0/default_petfelguard, leaving native initial3 shards.

Pinned player.cpp6138–6183 defines one ordered precombat pass, eligible nonharmful actions and the first ready harmful action. Native implementation must use real cast completion/GCD progression, out of combat before explicit fixture CombatRefs and300s clock. No cast/time/permission/resource bypass. Native pet already matching the configured default makes summon_pet a once-only no-op. WW snapshot/item behavior remains a control.

## First native RED packet

prepare_literal is an explicit entry scaffold mirroring the current late handoff; it still delegates to prepare before capturing. It is not claimed implemented. precombat_tests exercises actual103/266 prepared profiles and current native data:

- Literal Feral must have neither1126 nor an earlier kit1126 CastSuccess at pull; current late capture should fail.
- Nonliteral Feral retains1126/native kit, proving controller isolation.
- Literal Demo must honor declared initial0 shards before precombat and retain native selected Felguard17252; no grants or after-cast resets.
- Native Demo ST must deal first damage within10s after the pull, with one target and no add-wave/re-engagement loop. The test retains Follower/NPC identity and the normal native controller, so literal takeover cannot itself manufacture a failure.

All four are opt-in with prepared profile/native source-view reasons. Runtime RED pending before implementation. These cases do not yet qualify ordered multi-action selection, cast/GCD completion at pull, all start options or complete precombat execution. Native first-damage control complements rather than replaces the cast/GCD boundary assertions.

## Native Demo first-pull RED and local compile

14861's actual `demo-parity-2026-10-23/native-lifecycle-810/266.first-engagement-10s.json` and `first-engagement-red.proof.json` were read. Test-only c8b2ccf32 on unchanged81034539f production exits101, private DB SHA dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae. Both1T and3T have0damage through10s. Only30146 succeeds, at pull+5.85s. Owner refs exist at+1..4s, are gone at+5s; after+6s pet exists but actor has neither target nor combat refs. This is genuine production-ordering RED, not proof that the executor exists.

The local ff8ba4206 four-case test packet is compiling in worker01a16b35-1801-715f-acd5-c2293e103f39 after scoped ADD; native outputs will be under `ww-parity-2026-10-23/native-precombat-red-ff8ba4206`. Rust remains frozen until the build terminates. The same binary qualifies WW target seam; no separate WW-only compile.

Actual266 precombat ends with power_siphon, conditional demonbolt, shadow_bolt. Initial0 shards must be checked BEFORE those native builders, not forced back to0 afterward. Split the scaffold's combined initial-resource/pet-at-pull assertion into explicit phases when implementing. Nonharmful pre-casts finish OOC before explicit engagement; preserve the first harmful precast's real impact rather than clearing it in engage(). Any later post-engage target/ref loss is a separate gap, not repaired by recurring pulls.

## First implementation slice — runtime GREEN pending

Own ff8ba4206 run now qualifies3RED/1GREEN in32.32s: literal Feral unauthorized1126, Demo initial30raw rather than0, and Demo ST0damage after10s all fail; native Feral kit control passes. Binary SHA12797a5dff29152d58cf99ed04f539997f7d400031b3f2c35a0b3b25d3598ba8. `native-precombat-red-ff8ba4206/runtime.*` records command/revision; its newly taken canonical/copy hashes are568200017bcb3b5ed6f17931fc0994fff502c5c801f93f96197fc718bcd493bf/dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae.

Implementation splits spawn from warmup so explicit literal capture precedes all four frames, while native-controller fixtures retain their real kit. Native setup then advances actual pending cast/GCD state before returning for engagement, bounded at30s with explicit state on failure. No cast cancellation, cooldown reset, new combat refs or resource grants are used. Profile now retains raw top-level options; declared class resource tokens resolve through read-only native PowerType NameGlobalStringTag, convert display units using native DisplayModifier, validate actual pool bounds, and initialize once before setup. Undeclared pools remain unchanged.

The initial0-shard assertion is now separate from the native Felguard-at-pull control, which also checks real cast/GCD completion. This is five opt-in cases; the added boundary assertion uses the same already observed native30146 failure, not an easier controller. Shared ordered Driver traversal, generic configured summon_pet selection, WW lifecycle wiring and full literal266 precasts remain OPEN. This slice is not claimed to implement them.

## Ordered selector RED scaffold

Three env-free library cases exercise source action order/exhaustion, skipped unready rows remaining consumed after native state changes, and variables sampling facts only at their own source position. At1c1a9ac5e, Program::select_next delegated the old priority scan without consuming its cursor. The native-helper library run produced3 genuine REDs after1m19 compile: repeated snapshot instead of cat_form/prowl and revisited summon_pet instead of shadow_bolt. Evidence `ww-parity-2026-10-23/precombat-order-red.{proof.json,output.log}`. Implementation now consumes direct top-level variable/action rows once using existing target/condition/assignment selection; Driver uses a persistent cursor only for precombat, retaining normal combat priority scans. Nested call/run precombat controls are unqualified and fail explicitly, not silently flattened; ordinary combat controls are unchanged. No nested call/run precombat rows were found in pinned MID2 source. A fourth control checks explicit refusal; library GREEN is pending. Native lifecycle/summon integration remains separately open; do not treat the selector alone as the completed executor.

## Native lifecycle integration — GREEN pending

Ordered selector4/4 GREEN at171c8ec5c (`precombat-order-green.*`,29.01s native-helper library build); this does not qualify the server binary. Native WW measurement now captures before warmup, loads its adapter, runs one ordered precombat pass with actual native cast/GCD frame progression between actions, and only then establishes the fixture fight origin and engages. Rotation::load no longer executes a hidden world-only precombat loop. Literal engagement retains actual precast events, including a first harmful impact, instead of clearing them; native nonliteral engagement keeps its prior log boundary. Player dispatch/target-aware Prepared are unchanged.

Driver caps the pass after its first ready native harmful execution using native effect positivity, not hardcoded WW coefficients. New opt-in controls check WW snapshot exists with no owner/actor opponent refs and time0 before pull, plus actual TP→naturally affordable BOK: BOK remains native-ready after real GCD recovery but cannot become a second harmful precombat action. Native cast/GCD drain inspects all actual GCD deadlines, including item-use categories. Native/non-Fixture and dungeon rejection controls remain unchanged. Runtime integration proof is pending; generic configured summon_pet binding/no-op remains OPEN.

## Configured native pet binding — GREEN pending

ConfiguredPet resolves `<class>.default_pet` against the actor's learned native spell names, then requires the actual native SUMMON_PET effect/creature entry. No spell/creature IDs or spec266 fallback are invented. Current raw70448 records confirm Imp688→416, Felguard30146→17252, Sayaad366222→1863; runtime code reads those records rather than matching numeric constants. Missing/ambiguous learned bindings fail explicitly. An existing living matching native pet omits the summon action; a missing/different pet uses the normal cast path.

Native-controller fixtures start this declared summon through existing validated Player/Creature execution before their setup frames, then complete real cast/GCD state before pull. Their native kit and identity remain unchanged; classes without a declaration are unaffected. Explicit literal fixtures instead expose the same configured action to their ordered source pass; no hidden out-of-order summon is inserted. The bounded native266 control executes actual summon_pet/snapshot_stats rows, requires a real positive-duration cast bar and elapsed time, then repeats with the existing pet and checks one total30146 CastSuccess and the same pet entity. It does not claim the later full Demo APL/trinket variables are routed by this branch.

## First native integration attempt —4 PASS/4 FAIL at7e532fc90

Clean18m34 build; eight actual cases finish29.16s on a new current backup (source568200.../copydd1de...). `native-precombat-green-7e532fc90/runtime.*` is a FAILED qualification attempt, not GREEN: early Feral capture, native Feral control, declared0shards and WW snapshot-OOC/time0 pass. Configured pet, native pet-at-pull and Demo first damage fail because native30146 costs10raw SoulShards while the declared initial pool is0. BOK later-readiness counterfactual also fails; do not weaken it without actual readiness/power diagnostics.

Native PowerType7 has center30, RegenPeace−5 and12000ms OOC interrupt; existing regen_step raises a below-center pool naturally after that delay. Fix waits bounded native affordability/cast/GCD readiness before the configured summon, advancing real App frames only; it does not change costs, grant shards or reset at pull. If the native kit completes the same requested pet during the wait, it becomes the existing-pet no-op. Instant NPC casts have no GCD, so cast-completion drain must also settle at least one actual frame for queued native effects before the next row. BOK failure output now includes exact native admission/Chi/override state if it persists. Runtime requalification remains pending.

## Native affordability requalification —6 PASS/2 FAIL ated76/f5

Clean13m27 build, actual eight-case runtime34.01s with new current backup; `native-precombat-green2-ed76b76f3/runtime.*`, binary SHA691c17c30e9653fe5ff56b19e3bba5780f947ab86ce0fd207bfaee9df58fa1be. Configured native summon waits real OOC regeneration, uses a real cast, retains one CastSuccess across existing-pet repetition, and native pet-at-pull now passes. Early capture, ordinary Feral control, initial0shards and WW snapshot-OOC/time0 remain passing. This is six qualified controls, not full GREEN.

Remaining first-harmful counterfactual has actual Chi3/max6, resolvedBOK100784, native refusal OutOfRange; it is not a remaining power/GCD failure. Demo's first10s still has0damage after pet completion, but actor retains target/combat refs and Pursuing; its position41.6686,−14.1273,−0.504 is outside caster range from dummy1.5,0,0. Native Tyrant265187 and Curse702 execute, not damage. No recurring engagement was added.

Parent identified this as the copied Stockade Space routing dummy coordinates during genuine OOC setup.14755 owns `follower-parity-movement` off8e7c7a685: parity drops Space after copy creation, movement routes directly only when Space is absent, and checks in-reach before acquiring a route. This agent must NOT edit movement. Await/cherry-pick that packet, then rerun unchanged native controls and WW receipts with fresh current inputs. Poison composite remains a separate unqualified seam.

## Composite poison cast boundary — OPEN

Parent's13120 packet `assassination-parity-2026-10-23/apply-poison-source-packet-v1.json` was read. Bare apply_poison selects native2823 Deadly,3408 Crippling,381664 DTB Amplifying,381637 DTB Atrophic in that order (selected DTB381801). All four have CastingTimeIndex16/Base1500/Minimum1500. Packet is source inventory, not native execution proof.14775 owns composite selection; this agent owns completing each actual cast and final GCD before engagement. Incremental executions or an observable pending plan are required: dispatching four world-only casts without App progression would collide with the first cast. Current7e native GREEN scope does not yet qualify that composite seam; no coatings may be dropped or treated as instant.

Staged ignored `native_poison_precombat_completes_every_selected_coating_before_pull` uses actual259 profile/selected DTB and the native Player fixture (`DPS_PARITY_ACTOR=player`). It explicitly dispatches the four source-selected self-coatings, requires a real positive-duration CastState and native elapsed completion after EACH, then four ordered CastSuccess receipts and all four retained auras before engagement. This isolates this owner's cast boundary, not14775's composite selection/dispatch. Native permission failures are not bypassed; runtime qualification is pending. Run separately from the eight follower controls with prepared259 inputs, in the same next precombat binary.

## Composite lifecycle integration — genuine RED, GREEN pending

Forwarded4ad3b8d41 was cherry-picked as9907b66ab, preserving the existing default-pet module. Its `with_composites` is still a no-op selector scaffold, not qualified selection. `literal_precombat_composite_waits_for_native_coatings_and_effects` uses the real259 fixture and verifies individual native admission before selecting the named ordered plan. An App observer must see all four actual cast bars; elapsed time must cover their sum, actual CastSuccess order and retained owned auras must match, cast/GCD must be finished, engagement absent, and the next snapshot row executable. No internal vector-shape assertion substitutes for these effects.

Atd49f4951c the real259 packet reaches selected Composite after all individual native admission premises and fails at `composite precombat requires a clock-aware executor` (39.98s), a genuine consumption RED. Adjacent manual four-coating/native-bar control passes28.20s, retaining every aura. Selection's existing-coating case separately fails Amplifying's native admission premise (6PASS/1FAIL), not its selection assertion; owner14775 receives that gap.

`execute_precombat_plan` now unwraps a nonempty selected plan and dispatches one normal child at a time, calling real `finish_pending_setup` App advancement after EACH, including the final child. Native dispatch rechecks admission; errors preserve earlier real effects and stop later steps. Next source row/pull cannot occur until return. Empty plans and world-only Composite execution remain explicit errors; no poison IDs, instant completion or recursive world-only loop. Combined GREEN pending; no extra WW-only compile.

## Integrated native runtime and retained-follow fix — GREEN pending

Atd49f4951c the eight original controls run32.73s:7PASS/1FAIL. Demo now deals damage inside10s after pull. BOK still has Chi3/resolved100784 but is OutOfRange at actorx10/target1.5. `spawn_parity_attacker` keeps a real quiet owner atx20, and the actor retained native FollowOwner during literal setup. This is distinct from the fixed copied-Stockade graph detour.

Parent explicitly approved removing only the captured actor's FollowOwner after Encounter::Fixture admission, leaving Follower/NPC identity, native AA chase and dungeon rejection unchanged. No pose reset/recurring engagement. New ignored control rejects Dungeon capture without mutation, then observes an uncaptured native follower actually move closer to its real owner while retaining FollowOwner/Npc. Combined GREEN pending; neither the new control nor the capture fix is yet runtime-qualified.

## Forwarded integration before combined RED build

Parent forwarded masterb67952ca4 (landed er1048 tier redirected-base-stats and dm1051 dummy timerba0783c26), qualified movementmv1071/2d9855a69 and selection7cfc2e4e6 (its native GREEN still pending). This branch merges master as0b1e639f4, accepts movement test/fix chain asc2a23e43b/91e954749/1cda9e258, and cherry-picks selection asea3addb82. Only module-list/wiki conflicts needed combination; no independent movement behavior was edited. New dependency code is not qualified by old f5 receipts.

Master now puts server modules in `lib.rs` and leaves a minimal `main.rs`: combined native builds must target `--lib`, not old `--bin game-server`. Record the new test executable/hash and nonzero selected test counts. Existing eight precombat controls, composite selection/lifecycle, native moving/off-GCD gates and Mystic Touch controls still need execution on this integrated source. Consumption, row gates and Mystic producer are not implemented before their real native REDs.

## Dependencies

pd1035 is landed at2333d924b and merged locally; exact-target Prepared/Player dispatch remains14775-owned. Resource query010f7f761 is GREEN9/9 on arms-literal-resource-hooks; it ignores only the requested PowerType affordability while retaining other native checks. Integrate its shared API after pd, not the old613 scaffold. No unselected legacy kit fallback and no WW-specific tier2pc path.
