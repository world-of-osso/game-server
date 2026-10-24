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

## Dependencies

pd1035 is landed at2333d924b and merged locally; exact-target Prepared/Player dispatch remains14775-owned. Resource query010f7f761 is GREEN9/9 on arms-literal-resource-hooks; it ignores only the requested PowerType affordability while retaining other native checks. Integrate its shared API after pd, not the old613 scaffold. No unselected legacy kit fallback and no WW-specific tier2pc path.
