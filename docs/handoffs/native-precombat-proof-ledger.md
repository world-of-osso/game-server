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

## Dependencies

pd1035 is landed at2333d924b and merged locally; exact-target Prepared/Player dispatch remains14775-owned. Resource query010f7f761 is GREEN9/9 on arms-literal-resource-hooks; it ignores only the requested PowerType affordability while retaining other native checks. Integrate its shared API after pd, not the old613 scaffold. No unselected legacy kit fallback and no WW-specific tier2pc path.
