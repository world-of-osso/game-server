# Class Dummy spell scripts

Spell-specific behaviours for Retail Dummy effects live in `crates/server/src/spell_cast/scripts.rs`. Generic dispatch alone is not gameplay coverage. See [spell casting](../wiki/systems/spell-casting.md).

## What it must do

- [ ] Shield Block 2565 casts 132404 on the caster, granting the catalog block aura for six seconds.
- [ ] Bloodthirst 23881 retains its damage and 80 raw Rage, heals through 117313 for 3% maximum health, and rolls effect 2's modified chance to apply Enrage 184362.
- [ ] Rampage 184367 spends its catalog cost, retains the catalog attack triggers, and applies Enrage 184362.
- [ ] Holy Shock 20473 casts 25912 at enemies or 25914 at allies (including self), using their catalog damage/heal formulas and retaining its one Holy Power gain.
- [ ] Barkskin 22812's periodic Dummy casts Brambles damage 213709 every catalog tick when Brambles 203953 is present, hits enemies only, and stops on aura removal.
- [ ] Unknown Dummy fields remain unsupported rather than receiving generic no-op handlers.

## How it works

- [Spell scripts and coverage boundaries](../wiki/systems/spell-casting.md#class-dummy-scripts)

## Implementation inventory

- `crates/server/src/spell_cast/scripts.rs` — spell-ID effect-hit hooks.
- `crates/server/src/auras/scripts.rs` — periodic class aura scripts.
- `crates/server/src/spell_cast/dummy_tests.rs` — real-ID player casts and observable outcomes.

## Tests asserting this spec

- `crates/server/src/spell_cast/dummy_tests.rs` — `dummy_warrior_*`, `dummy_paladin_*`, `dummy_druid_*`; Bloodthirst chance endpoints use explicitly labelled catalog clones, while the ordinary cast uses unchanged data.

## Known gaps (current cycle)

- [ ] Dummy markers not tied to a documented mechanic remain unresolved; coverage diagnostics enumerate them.

## Out of scope

- Full talent/hero variants: Fresh Meat's guarantee, Enrage talent gating and Rampage's optional AoE branch are not certified by these base-script tests.
- Realm UDP 5000, deployment and merging.
