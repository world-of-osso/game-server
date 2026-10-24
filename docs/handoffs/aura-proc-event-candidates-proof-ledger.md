# Aura proc event-time candidates

## Scope

Lead approved two logical commits from exact master51a28da8a06f3880cbf5852923a8b5080c30826d: generic event-time aura candidates, then Ret Resonance producer at existing AfterCast. The generic commit has no spell-ID branch or metadata override. Echo, stack spending and500ms ICD are excluded. Parent owns full gate, remote push and landing; Ret development must not pick this up before landing.

## Source and failure boundary

Pinned TC a352b1fa `Unit.cpp10611–10630` gathers the current aura map and10638–10670 collects candidates synchronously before executing procs. `Spell.cpp3843→3918→3950` orders OnCast→cast-success procs→AfterCast. `SpellAuras.cpp1834–1980` has no blanket castId exemption. The previous native queue stored raw events and scanned the later live aura map, incorrectly admitting auras applied after the event.

Native non-Ret453758 Leydrinker has a self ApplyAura/Dummy4 effect, max1, charge1, chance100, mask[0,4], no special attributes and30s duration. An actual ordinary self cast raises cast-success before its aura applies; that older event must not consume the newly applied aura. A subsequent actual133 Fireball cast must still consume its native charge. The fixture qualifies native application/event ordering, not Leydrinker talent availability. No data or chance edits.

At51a28da8a plus test-only diff, the actual self cast succeeds and AuraApplied is observed, then retained-aura assertion fails: **1genuineRED22.91s**, zero-warning4m53compile (`ley-drinker-red-v2.log`). First attempt was interrupted without output; its orphan scope was stopped and process exit verified. It is not a gameplay result (`red-v1-interruption.txt`). Canonical native input is the checkout's existing read-only `data/world.db`, resolving to engine `data/server-world/world.db`; no DPS measurement or canonical writes.

## Implementation and pending proof

Queue entries pin actor/target aura instance identity sets at event raise. Deferred native proc selection intersects live instances with these sets, retaining native order and existing filters/chance/cooldown/charge behavior. Removed or replaced instances do not match a new application. Real melee's former direct queue append routes through the same entry point. Interrupt behavior is unchanged; no claim of full synchronous TC execution or new proc-phase support.

Targeted original Leydrinker GREEN and unchanged existing proc/charge filters ONCE pending. Full gate remains parent-owned. Source/runtime artifacts: engine `data/diagnostics/aura-proc-event-candidates-2026-10-24/`.
