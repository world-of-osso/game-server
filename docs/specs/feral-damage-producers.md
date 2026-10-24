# Feral damage producers

Restore missing spec103 producer channels from `game-engine/data/diagnostics/dps-parity-2026-10-21/top10-unobserved-channel-routing.{md,json}`. Protection's scoped producer batch is separate; this is not full legal-loadout, ability, UDP or DPS-parity acceptance.

## What it must do

- [ ] Native Feral Frenzy274837 produces274838 direct attacks, actual combo-point gains and owned periodic damage at native timing, retaining the original recipient rather than following later selection changes. Catalog presence alone grants nothing.
- [ ] Thriving Growth439528 produces owned Bloodseeker Vines439531 from eligible native Rip/Rake ticks; Bursting Growth/Rampancy preserve their learned prerequisites, recipients, amounts, stack and lifetime behavior.
- [ ] Unseen Predator uses actual spent combo points and energy, native talent chance/range and source-qualified Slash/Swipe/bleed dispatch; no forced RNG or manufactured resources.
- [ ] Applicable Guillotine/Morsel equipment and Omnium Folio expansion-trait activation preserve their actual source predicates, scaled inputs, role and ownership. Do not invent equipped-rune ownership. Unsupported source assumptions remain explicit gaps.
- [ ] Non-Lunar Convoke outputs preserve the shared root's source-qualified selection and dispatch without duplicating the Lunar owner's work.

## How it works

Runtime integration belongs in [spell casting](../wiki/systems/spell-casting.md). Native DB2 values override stale source comments. Generic missing consumers precede named policies; pinned SimC supplies a named policy only where TC and DB2 do not encode the decision.

## Implementation

No Feral producer changes yet. Native source packet: `/home/osso/.worktrees/logs/census-tail/feral-native-source.json`, row SHA256 `787ff21eb7d07c288b7999f1597347f263b1e5189dd6ba5ccfabd0db28c102d5`. Pinned sources: TC a352b1fa, SimC76f8118 `sc_druid.cpp` and `unique_gear_midnight.cpp`, native70448 catalogs.

Thriving Growth is not an RPPM producer: native439528 has PPM_ID0, scripted ProcChance101, Rip/Rake accumulator values100/135 and exponent52%. SimC12276–12403 supplies the accumulator threshold1000 and random multiplier[0,2], reading native effect values; its comment's62% is stale. Unseen1263657 supplies15% per spent combo point, not a guaranteed five-point proc.

## Tests

Six native fixtures in `spell_cast/feral_producer_tests.rs` and its `feral_talent_producer_tests.rs` child await prerequisite qualification and meaningful RED: Frenzy/retarget and catalog absence; Rake/Vines plus learned-driver removal and absent-talent control; earned five-point Bite/Unseen plus absent-talent control. Computed caster stats, ordinary Cat Form, native Energy regeneration and actual Shred gains supply the inputs. No source data, chances, flags, clocks, RNG or budgets are altered.

The first Frenzy-only and subsequent six-case admissions each ended after60 capacity polls without starting a compiler or test; neither is a RED receipt. Native root-cost packet `logs/census-tail/feral-root-prerequisites.json` (row SHA256 `0866dadfadacd9ead695c89e697765221242a39e8b5a1aa9291d4120e9faf64b`) authenticates Bite25Energy plus1CP/optional4. `feral-native-timing-controls.json` (row SHA256 `84a5e981822a1a2ca4b02b3411642511e413455474f13c0b61ce7328272deba0`) authenticates Frenzy1000ms, native Energy100/10regen and CPmax5.

## Known gaps

All checklist entries remain unqualified. Convoke391528 root ownership is pending coordination with the Lunar owner; independent channels can proceed.

Omnium is an expansion-trait input: SimC `player.cpp2221–2222` initializes `omnium_talents_str` through `initialize_expansion_trait_effects`; `unique_gear_midnight.cpp6618` names Omnium Folio12.0.7. Native TraitDefinition contains1279599/1287555; no ItemEffect or SpellItemEnchantment link for these drivers was found. Its fixed289 coefficient level (`unique_gear_midnight.cpp5980–5981`) is a SimC model constant, not an authenticated equipped item level. The same file6182 explicitly marks its proc same-unit and damage/heal routing assumptions `UNVERIFIED_IMPLEMENTATION`; native1279599 supplies scripted4 and linked285/coefficient1288183, not that mapping. Item source TODOs or unverified assumptions are not execution proof.

## Out of scope

13540's Lunar Inspiration155580/155625 and Demolish;13122's Fire channels; catalog-only children as standalone rotation entries; full legal-loadout, UDP, broad integration and DPS parity claims.
