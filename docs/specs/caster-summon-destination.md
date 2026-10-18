# Caster summon destination (target32)

Spell target32 follows pinned TC `a352b1fa` `TARGET_DEST_CASTER_SUMMON`, independently of target44. [Implementation and source boundaries](../wiki/systems/spell-casting.md#caster-summon-destination32--2026-10-18).

## What it must do

- [x] Select front-left at `max(1yd PET_FOLLOW_DIST, source combat reach)`, ignoring effect radius. Actual30717/source18812 reach10 proves this before summoned AI moves.
- [x] For represented gravity-enabled non-flyers on dry ground, clamp to floor. GravityDisabled makes the caster flight-capable: preserve above-floor altitude and raise below-floor altitude.
- [x] Attr9 FORCE_DEST_LOCATION uses a separate ten-step distance backoff with6yd floor-height-change limit, then ground placement; flight does not select ordinary allowed-Z behavior in this mode.
- [x] Preserve selector18 caster altitude and selector62/138 ground behavior, including existing enabled live-floor behavior.
- [ ] Missing/invalid height never rejects the cast: ordinary selection retains computed Z; FORCE rejects invalid-height offset candidates and preserves original Z if the final probe is also invalid. Actual688 unavailable-ground regression proof is pending.

## How it works

- [Selection, geometry reuse and precise residuals](../wiki/systems/spell-casting.md#caster-summon-destination32--2026-10-18).

## Implementation inventory

- `crates/server/src/spell_cast/targets.rs` — separate32 selector and source-reach XY; mutable destination planning invokes geometry selection.
- `crates/server/src/spell_cast/caster_summon_destination.rs` — existing live-GO clamp, dry-floor/flight adjustment and distinct FORCE backoff.

## Tests asserting this spec

- `scripted_ai/tests/herald_target32.rs` — real copy-only30717/Ramparts source actor/reach/geometry, ordinary summon before AI movement, independent flight casters, FORCE height-backoff cases.
- `spell_cast/target32_controls_tests.rs` — selector18/62/138 controls.
- `spell_cast/live_ground_destination_tests.rs` and existing podium test — retained ground-destination consumers.

## Known gaps (current cycle)

- [ ] `TARGET32_FIRST_COLLISION_PARITY_UNIMPLEMENTED`: existing live-GO ray1yd/clearance0.5yd is not TC nav/static/unit-halfheight tracing. Helper reuse is intentional; no new collision system.
- [ ] `TARGET32_HOVER_SWIM_STATE_UNIMPLEMENTED`: no HOVER/HoverHeight or UnitFlags2 swim projection; wet/hover parity is not certified.
- [ ] `TARGET32_HEIGHT_PROBE_PARITY_UNIMPLEMENTED`: existing sampling does not reproduce TC MAX_HEIGHT-versus-near-height selection.
- [ ] `CASTER_RELATIVE_DEST_ALLOWED_Z_UNIMPLEMENTED`: selectors42,44,47,49,50,72,55 remain unchanged.

## Out of scope

Herald INTRO2 choreography, normal flight loops, descent, paired encounter credit/reset and native capture. Primitive placement proof alone does not establish any of these.
