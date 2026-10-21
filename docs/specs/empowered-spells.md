# Empowered spells

Player and follower empowers use native build70448 stage metadata and release real native spell effects. Timing/lifecycle source: TrinityCore a352b1fa `Spell.cpp:4019–4038,4303–4344,4409–4428`, `Spell.h:88–89`; qualified Devastation consequences additionally use SimC76f8118 `sc_evoker.cpp:5265,5507`. DB2 fields take precedence over descriptions and older SimC coefficients. [Implementation and proof](../wiki/investigations/follower-data.md#native-empower-pipeline--2026-10-21).

## What it must do

- [ ] Resolve native override spells before ordinary admission, derive stage thresholds from their DB2 durations and applicable native modifiers, and retain the TC1000ms maximum-stage hold.
- [ ] Release at an explicitly selected completed stage, on a matching explicit player release after the first-stage minimum, or automatically after maximum hold; pass the completed stage into the release effects/scripts.
- [ ] Defer the GCD until empower finish and use native359115's500ms clock. Cancellation must remain distinct from release and retain the sourced cancellation/resource lifecycle.
- [ ] Fire Breath357208/382266 releases actual357209, converting three periodic ticks per extra rank into direct damage and preserving the remaining native DoT plus Blast Furnace extension. Eternity Surge359073/382411 releases359077 against the completed rank's recipient count, doubled by native Eternity's Span. Fire Breath's Magic dispel requires native Scouring Flame378438 and retains current effect5's native count2, not the obsolete TC silence/index/rank-count fields.
- [ ] Accept player release only for the authenticated owner's matching active empower. `ReleaseEmpowerSpell {spell_id}` and `StopSpellCast` remain separate messages; the client sends release when the cast binding goes up.

## How it works

- [Native pipeline and qualified release scripts](../wiki/investigations/follower-data.md#native-empower-pipeline--2026-10-21).
- [Follower selected-rotation requirements](follower-dungeons.md#selected-dps-rotation-decisions).

## Implementation inventory

- `crates/server/src/spell_cast/empower.rs` — stage progress, hold and qualified completion.
- `crates/server/src/spell_cast/mod.rs` — shared admission, progress and channel cleanup; `followers/kit_overrides.rs` owns follower override resolution before eligibility.
- `crates/server/src/spell_cast/scripts/empower_evoker.rs` — sourced Fire Breath/Eternity Surge release consequences.
- `crates/server/src/spell_info/mod.rs` — per-cast selected/completed rank snapshots.
- `crates/server/src/spell_cast/effects.rs` — converted periodic damage before direct critical-strike resolution.

## Tests asserting this spec

- `crates/server/src/followers/kit_empower_tests.rs` — native hold/GCD admission, Fire release/DoT and Scouring gate, Surge/Span recipients and Font variants; REDs and separately labelled native controls.

## Known gaps (current cycle)

- [ ] Prove the four initial native REDs GREEN on current master and the fresh canonical backup.
- [ ] Add explicit player release, cancel/refund and selected-stage behavior tests plus matching protocol/client consumers.
- [ ] Qualify and gate the current native Scouring dispel without changing generic utility handling.
- [ ] Select follower empower stages from the applicable SimC APL branch, without inventing ranks or talent identities.

## Out of scope

- Empower-stage cast-bar UI: explicitly assigned to a later client slice.
- Non-owned specialization release scripts: unqualified handlers remain explicitly unsupported rather than receiving synthetic child effects.
