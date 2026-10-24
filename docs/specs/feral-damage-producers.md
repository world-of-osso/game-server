# Feral damage producers

Restore missing spec103 producer channels from `game-engine/data/diagnostics/dps-parity-2026-10-21/top10-unobserved-channel-routing.{md,json}`. Protection's scoped producer batch is separate; this is not full legal-loadout, ability, UDP or DPS-parity acceptance.

## What it must do

- [ ] Native Feral Frenzy274837 produces274838 direct attacks, actual combo-point gains and owned periodic damage at native timing, retaining the original recipient rather than following later selection changes. Catalog presence alone grants nothing.
- [ ] Thriving Growth439528 produces owned Bloodseeker Vines439531 from eligible native Rip/Rake ticks; Bursting Growth/Rampancy preserve their learned prerequisites, recipients, amounts, stack and lifetime behavior.
- [ ] Unseen Predator uses actual spent combo points and energy, native talent chance/range and source-qualified Slash/Swipe/bleed dispatch; no forced RNG or manufactured resources.
- [ ] Applicable equipped Guillotine, Morsel and Omnium channels preserve native equipment/activation predicates, scaled inputs, role and ownership. Unsupported source assumptions remain explicit gaps.
- [ ] Non-Lunar Convoke outputs preserve the shared root's source-qualified selection and dispatch without duplicating the Lunar owner's work.

## How it works

Runtime integration belongs in [spell casting](../wiki/systems/spell-casting.md). Native DB2 values override stale source comments. Generic missing consumers precede named policies; pinned SimC supplies a named policy only where TC and DB2 do not encode the decision.

## Implementation

No Feral producer changes yet. Native source packet: `/home/osso/.worktrees/logs/census-tail/feral-native-source.json`, row SHA256 `787ff21eb7d07c288b7999f1597347f263b1e5189dd6ba5ccfabd0db28c102d5`. Pinned sources: TC a352b1fa, SimC76f8118 `sc_druid.cpp` and `unique_gear_midnight.cpp`, native70448 catalogs.

Thriving Growth is not an RPPM producer: native439528 has PPM_ID0, scripted ProcChance101, Rip/Rake accumulator values100/135 and exponent52%. SimC12276–12403 supplies the accumulator threshold1000 and random multiplier[0,2], reading native effect values; its comment's62% is stale. Unseen1263657 supplies15% per spent combo point, not a guaranteed five-point proc.

## Tests

Pending native prerequisite qualification and meaningful RED. Fixtures must use unmodified native rows, normal casts and real resource transitions, then assert damage/aura/resource receipts, ownership, absence/removal and timing. No source data, chances, flags, clocks, RNG or budgets may be altered to obtain the expected result.

## Known gaps

All checklist entries remain unqualified. Convoke391528 root ownership is pending coordination with the Lunar owner; independent channels can proceed. Item source TODOs or unverified assumptions are not execution proof.

## Out of scope

13540's Lunar Inspiration155580/155625 and Demolish;13122's Fire channels; catalog-only children as standalone rotation entries; full legal-loadout, UDP, broad integration and DPS parity claims.
