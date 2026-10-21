# Empowered spells

Player and follower empowers use native build70448 stage metadata and release real native spell effects. Timing/lifecycle source: TrinityCore a352b1fa `Spell.cpp:4019–4038,4303–4344,4409–4428`, `Spell.h:88–89`; qualified Devastation consequences additionally use SimC76f8118 `sc_evoker.cpp:5265,5507`. DB2 fields take precedence over descriptions and older SimC coefficients. [Implementation and proof](../wiki/investigations/follower-data.md#native-empower-pipeline--2026-10-21).

## What it must do

- [x] Resolve native Font override spells through the shared follower action resolver before admission and retain the native three/four-stage metadata and TC1000ms maximum-stage hold. Modifier-specific timing still needs focused proof.
- [ ] Release at an explicitly selected completed stage, on a matching explicit player release after the first-stage minimum, or automatically after maximum hold; pass the completed stage into the release effects/scripts.
- [ ] Defer the GCD until empower finish and use native359115's500ms clock. Cancellation must remain distinct from release and retain the sourced cancellation/resource lifecycle.
- [x] Native rank3 Fire Breath357208 releases real357209 damage with its remaining8000ms DoT. Native rank3 Eternity Surge359073 hits3 recipients, or6 with actual Eternity's Span. Fire Breath's dispel preserves enemy Magic buffs without378438 and removes them with native Scouring Flame.
- [ ] Prove converted-tick direct damage across selected ranks, Blast Furnace extension, Font variant release consequences and native Scouring count2. Preserve current effect5 fields, not obsolete TC silence/index/rank-count fields.
- [ ] Accept player release only for the authenticated owner's matching active empower. `ReleaseEmpowerSpell {spell_id}` and `StopSpellCast` remain separate messages; the client sends release when the cast binding goes up.

## How it works

- [Native pipeline and qualified release scripts](../wiki/investigations/follower-data.md#native-empower-pipeline--2026-10-21).
- [Follower selected-rotation requirements](follower-dungeons.md#selected-dps-rotation-decisions).

## Implementation inventory

- `crates/server/src/spell_cast/empower.rs` — stage progress, hold and qualified completion.
- `crates/server/src/spell_cast/{mod.rs,creature.rs}` — shared admission, selected-stage NPC casts, progress and channel cleanup; `followers/kit_overrides.rs` owns follower override resolution before eligibility.
- `crates/server/src/followers/empower_policy.rs` — SimC ES/fb_fs/fb_sc rank order for the qualified native selected book.
- `crates/server/src/spell_cast/scripts/empower_evoker.rs` — sourced Fire Breath/Eternity Surge release consequences.
- `crates/server/src/spell_info/mod.rs` — per-cast selected/completed rank snapshots.
- `crates/server/src/spell_cast/effects.rs` — converted periodic damage before direct critical-strike resolution.

## Tests asserting this spec

- `crates/server/src/followers/kit_empower_tests.rs` — native hold/GCD admission, Fire release/DoT and Scouring gate, Surge/Span recipients and Font variants; REDs and separately labelled native controls.

## Known gaps (current cycle)

- [ ] Add explicit player release, cancel/refund and selected-stage behavior tests plus matching protocol/client consumers.
- [ ] Prove follower single-target Fire Breath rank1 and cancellation cooldown/GCD/no-release controls GREEN.
- [ ] Complete optional Legacy of the Lifebinder/Volatility purchased-rank branches; current starter book lacks those talents, so they remain unqualified rather than inferred from spell presence.

## Out of scope

- Empower-stage cast-bar UI: explicitly assigned to a later client slice.
- Non-owned specialization release scripts: unqualified handlers remain explicitly unsupported rather than receiving synthetic child effects.
