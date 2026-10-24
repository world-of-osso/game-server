# Aura proc event-time candidates

## Scope

Lead approved two logical commits from exact master51a28da8a06f3880cbf5852923a8b5080c30826d: generic event-time aura candidates, then Ret Resonance producer at existing AfterCast. The generic commit has no spell-ID branch or metadata override. Echo, stack spending and500ms ICD are excluded. Parent owns full gate, remote push and landing; Ret development must not pick this up before landing.

## Source and failure boundary

Pinned TC a352b1fa `Unit.cpp10611–10630` gathers the current aura map and10638–10670 collects candidates synchronously before executing procs. `Spell.cpp3843→3918→3950` orders OnCast→cast-success procs→AfterCast. `SpellAuras.cpp1834–1980` has no blanket castId exemption. The previous native queue stored raw events and scanned the later live aura map, incorrectly admitting auras applied after the event.

Native non-Ret453758 Leydrinker has a self ApplyAura/Dummy4 effect, max1, charge1, chance100, mask[0,4], no special attributes and30s duration. An actual ordinary self cast raises cast-success before its aura applies; that older event must not consume the newly applied aura. A subsequent actual133 Fireball cast must still consume its native charge. The fixture qualifies native application/event ordering, not Leydrinker talent availability. No data or chance edits.

At51a28da8a plus test-only diff, the actual self cast succeeds and AuraApplied is observed, then retained-aura assertion fails: **1genuineRED22.91s**, zero-warning4m53compile (`ley-drinker-red-v2.log`). First attempt was interrupted without output; its orphan scope was stopped and process exit verified. It is not a gameplay result (`red-v1-interruption.txt`). Canonical native input is the checkout's existing read-only `data/world.db`, resolving to engine `data/server-world/world.db`; no DPS measurement or canonical writes.

## Implementation and pending proof

Queue entries pin actor/target aura instance identity sets at event raise. Deferred native proc selection intersects live instances with these sets, retaining native order and existing filters/chance/cooldown/charge behavior. Removed or replaced instances do not match a new application. Real melee's former direct queue append routes through the same entry point. Interrupt behavior is unchanged; no claim of full synchronous TC execution or new proc-phase support.

At5aa686ad8abf36fe3e13971bda4f35d8ed32ac71, original unchanged Leydrinker case is GREEN, including retained native charge then actual Fireball consumption. Requested existing `proc OR charge --include-ignored` filters executed ONCE: **168PASS/3FAIL335.40s**, zero-warning4m34compile (`proc-charge-unchanged-once-v1.*`), not a full-pass claim. Two ignored follower proc_definition cases fail initial fixture preconditions (391876/449646 already present before Tiger's Fury). At lead's request only those2 ran ONCE on exactbase51a28da8a and both reproduce unchanged failures:19.39s/zero-warning2m53compile (`base-51a-two-proc-definitions-once-v1.*`), classified pre-existing. Trading Post's ignored case requires absent TRADING_POST_WORLD_COPY, was not rerun and is not a proc regression. No broad rerun. Full gate remains parent-owned. Source/runtime artifacts: engine `data/diagnostics/aura-proc-event-candidates-2026-10-24/`.

## Ret producer phase

Approved `SIMC_RET_DIVINE_RESONANCE_PRODUCER` belongs at existing AfterCast, not OnCast. SimC76f8118 `sc_paladin.cpp1901–1907` (approved surrounding1880–1905),3686–3688 grants after baseexecute; `action.cpp1971–1985` already dispatches cast-success callbacks. TC `Spell.cpp3843/3918/3950` distinguishes these phases. No castId or per-spell proc exemption is invented.

New independent producer cases use actual selected384027, nativeToll375576, native1266308 maximum/duration, real CastSuccess-before-AuraApplied order and retention across the first four proc updates. Ordinary20271/noToll and actual alternate choice115467/379391 at93181 are negatives; native point validation and Toll action knowledge remain intact. The exact51a base lacks unlanded Toll child dispatch, so this packet does not import or claim that relationship. This qualifies producer/phase/lifetime only, not child damage, echoes or whole Ret rotation.

At5aa686ad8 plus test-only diff, corrected phase-red-v3 is **1genuineRED/2controlsPASS42.78s**, zero-warning26.36compile: selected actualToll succeeds but1266308 is absent. phase-red-v1/v2 are invalid compile attempts (native duration path, then signed/unsigned comparison), never gameplay RED. Existing approved Ret development lifetime failure additionally recorded actual3stacks/1charge immediately after OnCast, removalat0.25 before30.2 expiry; that branch remains separate.

Implementation adds only the producer to existing AfterCast: validated selection/spec70, maximum from native aura options, existing triggered-cast stack initialization. Native duration untouched; no consumer/charge override, no500ms ICD or echo implementation. At a180b2e98f4fd56f18f52148629f667246929cd6 original unchanged3cases earn **3GREEN46.93s**, zero-warning57.67scompile (`resonance-aftercast-green-v1.*`). Real CastSuccess precedes native AuraApplied, max3/native30s survive the first proc updates, and both ordinary/noToll and legal other-choice negatives pass. Changed-file rustfmt checkPASS0 at that same source. Final second-commit amendment records results in docs only; generic runtime remains exactly5aa686ad8 and producer/test source remains exactlya180b2e98. No broad regression rerun, no Ret-development merge before parent landing, no full-gate or DPS claim.
