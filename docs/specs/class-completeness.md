# Level-80 class completeness audit and shared cast behavior

Source: `spell_cast`, `auras`, `spell_info`; [implementation and proof](../class-completeness.md), [spell-casting system](../wiki/systems/spell-casting.md#level80-class-completeness).

## What it must do

- [x] Record native cast/passive-sync outcomes, warning tuples and raw resources for all 13 classes/all 40 local Retail specs, using level-80 actors and real spell rows, without writing world.db or opening a realm port.
- [x] Preserve exact missing effect/aura types and source/state prerequisites; never close unidentified Dummy fields with no-ops or guessed coefficients.
- [x] Apply native aura537 damage percentages only for matching label and aura owner.
- [x] Apply aura457 haste only to its charge category; preserve running-clock progress and publish rate changes/replenishment without another cast.
- [x] Apply aura416 haste only to matching spell families, for both spell/category cooldowns, truncating native fractional milliseconds.
- [x] Preserve native signed class power costs, bounded generation, positive/negative sign constraints and atomic rejection.
- [x] Discounted primary misses consume neither runes nor negative-cost grants; school immunity is determined before payment, including composite schools covered by multiple immunity auras.

## How it works

- [Implementation/provenance and proof limits](../class-completeness.md)
- [Spell-casting system](../wiki/systems/spell-casting.md#level80-class-completeness)

## Implementation inventory

- `spell_cast/mod.rs`: cost, cooldown/charge and hit-plan payment decisions.
- `spell_cast/hit.rs`: whole-spell miss and pre-payment school immunity.
- `auras/handlers.rs`, `auras/mod.rs`: native handler dispatch, owner/label damage and school-mask union.
- `spell_cast/class_completeness_probe.rs`: opt-in controlled native-row census.

## Tests asserting this spec

- `crates/server/src/spell_cast/class_completeness_tests.rs`: native owner/label, charge/cooldown, level80 resources and immune/partial-school boundaries.
- `crates/server/src/spell_cast/class_completeness_probe.rs`: executed membership receipts (manifest supplied explicitly).
- Existing targeted resource/regen/generator tests remain bounded proofs, not every talent combination.

## Known gaps (current cycle)

- [ ] Full legal talent purchases, race/quest/achievement/ancestor/point-budget eligibility are not certified by a granted-spell build-union probe.
- [ ] Underivable Dummy actions, resource proc/reservoir algorithms, empower input, moving fields, soft-cap curves, guardian ownership/stat inheritance and missing imported rows remain explicit table gaps.
- [ ] Whole-spec rotation, PvP and all optional talent interactions are not established by handler coverage or these fixtures.

## Out of scope

- Master merge/deployment, world.db writes and UDP5000: explicitly excluded by the task.
- Client/protocol changes: existing resource/cast contracts carry these server corrections; no new client-visible payload required.
