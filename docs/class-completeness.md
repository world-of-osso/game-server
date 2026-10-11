# Level-80 class completeness

Audit scope: all 13 classes / 40 Mainline 12.1.0.69933 specs, including Devourer. Membership is a union of class/spec spells and selectable class/spec/hero talents at level 80; not a simultaneously legal talent loadout or a claim every race learns race-gated entries. Census does not promote positive partial spell tests to full ability certificates. Missing scripts, coefficients, empower/movement/guardian lifecycles and resource reservoirs remain gaps.

Durable per-spec/per-spell results and proof ledger: `/home/osso/.worktrees/handoff-class-completeness.md`. Regenerator, manifest, source receipts and command logs: engine `data/diagnostics/class-completeness-2026-10-11/`. Previous [Dummy23 census](spell-dummy23-coverage.md) reviewed all 157 roots, but explicitly excludes full talent trees and level-80 membership.

## Shared native label vulnerability (aura537)

`MOD_SPELL_DAMAGE_FROM_CASTER_BY_LABEL` now multiplies incoming spell damage only for the aura caster and native `SpellLabel` in `EffectMiscValue_0`. This is TrinityCore a352b1fa `Unit.cpp:7066–7069`, not a new per-spell coefficient. Percent amounts and stack counts come from the live aura. Existing aura271 behavior is unchanged.

Native Piercing Chill377359 is +12% to label1615 (Chill Streak204167). Tests exercise the real 20yd/s missile impact, same-owner amplification, unrelated Frostbolt116, another caster and expiry. Only fixture variance is zeroed. RED:115 damage instead of129; no coefficient/label mutation. Other mechanics on these abilities are not certified by this fixture.

No world.db writes, no master merge, no live realm or UDP5000 use.
