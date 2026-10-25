# Windwalker Xuen's Battlegear FoF recovery

## Mastera804 reconciliation — verification pending

Frozen36eeb2339 is merged, not rebased, into a new `ww-xuen-reconcile-a804` branch from exact mastera804452eb5be0c2665e33d83a525a23e7f34678e. Gatexr1111 ran no tests: its merge stopped on scripts.rs and three additive wiki conflicts. Preserve both the current SCK `periodic_trigger` and incoming four-argument `after_direct_damage`; Flurry forwards once and Xuen receives actual target/crit. SCK registers before Xuen extends the existing101546 lifecycle entry, retaining snapshot cleanup and natural completion. Both wiki sections are kept at every conflict; no gameplay redesign or new policy.

The merged source will run native Xuen13, retained Flurry6 and SCK snapshot4 against one source-pinned binary, plus formatter/diff/readability proof. Current results below remain historicalf188 evidence until the new receipts are recorded. No300s rerun; the lead owns the re-gate and plain production --bins requirement.

Approved2026-10-24: `SIMC_WW_XUENS_BATTLEGEAR_FOF_RECOVERY`, third after Flurry and SCK. `ww-xuens-recovery` starts at origin/master4996e5967, merges production-reader fix3f897bd38, then current master7b10a2c9b at1caa0ab7b. Tested production source is f188c66c1b9827087d8a07b9803de3cf2c453817; only proof documentation follows. Flurry is landed; SCKc743e921e/sk1106 remains separately frozen/gated.

## Named policy, not a native recovered relationship

SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc `engine/class_modules/monk/sc_monk.cpp954–958` requires primary (`!state->chain_target`) and `RESULT_CRIT` before adjusting existing FoF recovery by effectN(2). Native392993 zero-based effect1 supplies4000ms. RWK damage inherits that same `rising_sun_kick_t::base_damage_t` through its alias at1143–1150; native468179 supplies a real cone secondary boundary, while185099 is single-target. Actual native TargetInfo order, not mutable CombatTarget or first successful damage, supplies primary role.

SimC1541–1546 records unique impacted actor IDs without a damage-result predicate. Its1596–1609 expiration callback caps count at effectN(5)/effectN(4), adjusts existing recovery and clears the set. Native zero-based effect3=500ms/effect4=2500ms give at most5 actors/2500ms. Native101546/107270 retain their actual timing, geometry, coefficients and damage; no manual SimC pulse schedule is imported.

**TC/DB2 absence:** pinned TC a352b1fa `spell_monk.cpp` has no392993/Battlegear binding under the source packet's search terms. Native392993 effects1/3/4 are aura4 dummy values with EffectTriggerSpell0; they do not encode this conditional primary-critical/unique-expiration relationship. Current `tdb_spell_proc` has no392993 row. Native ProcChance101/ProcTypeMask4112 remain unchanged; their presence is not an execution link. This consumer is explicitly the approved SimC named policy. Native effects0/2 are not reinterpreted.

Exact excerpts, rows, hashes and absence scope: `game-engine/data/diagnostics/ww-parity-2026-10-23/xuen-recovery-source-packet.json`.

## Coverage and proof

| Boundary | Strict baseline f0ab339b6 | Current f188c66c1 |
|---|---|---|
| Primary actual185099/RWK468179 critical impact | Two genuineRED:0/1ms rounding versus4000ms | GREEN:4000ms, once even with two RWK critical targets |
| Noncrit, secondary-only critical, unselected RSK | Three controlsPASS | GREEN: all0 |
| SCK1/3/6 unique actors across repeated native impacts | Three genuineRED:0 versus500/1500/2500ms | GREEN:500/1500/2500ms;4/11/23 actual damage events, native TargetInfos retain all1/3/6 actors |
| No-hit SCK, unselected SCK, absent FoF timer | Three controlsPASS | GREEN:0; absent-timer case proves repeated real impacts and never creates a timer |
| Later no-hit parent; actual stop-cast after impacts | GenuineRED at first500ms precondition; stop-cast added for completion correction | GREEN: later set not reused; interrupted channel grants0 |

Complete strict baseline: six genuineRED/six controlsPASS36.52s, compile25.77s/zero warnings. Current integrated result: **13/13GREEN41.09s**, plus **six retained Flurry controlsGREEN23.72s** using the same binary; compile496.00s/zero warnings. Binary SHA-256 `60e8c9b60ef5636fffb178e5622d342f8d2acbccef7d2a5619ea26908b0b5fef`.

Requested plain production command `agent-run ww-xuens-recovery cargo check -p server --bins -j2` passes once at f188c66c1 in87.49s/zero warnings. It selects no test-support/tests/all-features. Format check passes; changed-line readability is manual because rust-code-analysis-cli is unavailable. The lead's separate full gate is not claimed complete.

## Native fixture and development ordering

The tests earn Chi with real Tiger Palm, earn RWK through native Combo Breaker/Blackout Kick, and cast real FoF to create its timer. Unselected125017 is removed from the declared profile before native progression. A read-only observer records actual impact-resolution TargetInfo order including misses. No aura/stack/power/stat/crit grants, coefficient changes or clock/RNG resets. Cooldown projection permits at most2ms tolerance for integer-millisecond reads.

Initial ccb16df6b stopped at E0603 on a test-only private PowerType import; no native execution and no RED claim. f8af512e0 then produced six genuineRED/five controlsPASS plus one sampling-premise failure. d9a66e6b8 recorded all eight earned RWK outcomes at native focus crit chance50.7174%: noncrit primaries occurred, but not yet with secondary critical. Extending that same continuing native trajectory to32 cycles reached the real joint outcome in f0ab339b6; no stats or outcomes were manufactured.

First implementation976173e6a passed8/12 but left all four SCK recovery assertions0. The initial expiry-only hook missed native channel completion: ChannelComplete flushes final aura ticks, then end_cast removes with ChannelEnded. Interruption shares ChannelEnded. f188 therefore calls the natural-end hook from the explicit full-duration channel path, not from all ChannelEnded removals. The real stop-cast control proves that distinction.

## Implementation representation

`HitFrame.selected` stores native TargetInfo Entity IDs; its matching-frame slice is distinct from original-request `cast_primary_target`. A generic resolved-direct-damage callback supplies actual target/crit. One script-layer wrapper forwards Flurry once and applies Xuen only to primary RSK/RWK critical impacts via the existing gameplay reduce_cooldown helper. There are no core spell-ID cases or test-gated production dependencies.

A private `SckImpacts` component holds the actual101546 aura instance and a unique Entity set. Its parent context hit initializes the set after aura application; actual107270 resolution records native TargetInfos. `AuraScript.on_natural_end` runs after final due ticks on explicit full-duration timed-channel completion, or after on_remove for ordinary expiry. Existing removal reasons and independent Unbroken Rhythm on_remove cleanup remain unchanged. The class consumer clears the matching set and applies native effect4/effect3 count cap and effect3 time without starting a cooldown.

## Artifacts and limits

Evidence root `game-engine/data/diagnostics/ww-parity-2026-10-23/`: `xuen-recovery-source-packet.json`, `native-ww-xuen-red-f0ab339b6/`, `xuen-green-f188c66c1.build*`, `native-ww-xuen-green-f188c66c1/`, `native-ww-xuen-flurry-controls-f188c66c1/`, `xuen-f188c66c1-native-proof-summary.json`, production/format/readability receipts and `xuen-final-local-proof-ledger.json`. Each runtime records its fresh current read-only SQLite backup, exact input/binary hashes and complete command/output.

Other107270 parent322729 is not separately runtime-certified here. No broad numerical remeasurement/parity claim. Known stale Zenith/movement fixture items remain a separately approved next slice. No worker push or spawned agents.
