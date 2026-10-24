# Triggered cast publication

Native successful casts publish CastSuccess and SpellGo at launch, before impact. Contract follows pinned TC a352b1fa Spell.cpp3893/4779/8346–8353, with the explicitly approved conditional-visual gap. [Mechanism](../wiki/systems/spell-casting.md#triggered-cast-publication--2026-10-24).

## What it must do

- [ ] Send for non-triggered casts, channels, AuraPointsOnClient, positive native missile speed/launch delay, AlwaysCastLog, or a nonzero unconditional Difficulty0 SpellXSpellVisual record. Never use spell-ID exceptions.
- [ ] Conditional rows are never assumed satisfied. An unconditional nonzero record proves nonzero resolution regardless of conditional alternatives.
- [ ] Strictly-conditional-only casts with no independent send term emit neither publication event and log the unresolved gap once per spell. Actual payload execution remains unchanged.
- [ ] Ordinary, scripted, costed and bool FULL_MASK paths publish exactly once, before missile impact. Preserve explicit targets and FULL_MASK proc suppression; no duplicate creature CastSuccess.
- [ ] Preserve genuine zero-visual/no-other-term silence and existing conflicting assertions for parent decisions.

## How it works

- [Mechanism and source](../wiki/systems/spell-casting.md#triggered-cast-publication--2026-10-24)
- [Proof, census and incompatible-test inventory](../handoffs/triggered-native-send-proof-ledger.md)

## Implementation inventory

- `spell_info/{mod.rs,load.rs}` — separate visual-presence, unconditional and conditional native facts.
- `spell_cast/publication.rs` — shared publication decision and logged-once unresolved gap.
- `spell_cast/{mod.rs,creature.rs}` — ordinary/scripted/costed/bool launch wiring.
- `server-simulation/src/triggered_send_tests.rs` — native behavioral fixtures.

## Tests asserting this spec

`crates/server-simulation/src/triggered_send_tests.rs`; qualified RED1429bef59, current GREEN pending.

## Known gaps (current cycle)

- [ ] Native GREEN, one two-crate check and independent proof; separate parent full gate.
- [ ] Parent decisions for the three documented existing conflicting assertions; do not rewrite here.

## Out of scope

Full caster PlayerCondition/UnitCondition evaluation is explicitly excluded. Frozen native census has2,127 strictly-conditional-only spell definitions;487 have independent send terms, leaving1,640 unresolved (including4 missing Misc records). No conditional row is treated as matched. Script-visual override/trajectory APIs absent from the current model are not fabricated.
