# Momentum Boost: native eligibility versus SimC

Verified2026-10-23. Source investigation only; no WW proc filter, coefficient change or tier-event bridge is authorized by these differences. [WW matrix](ww-source-census.md).

## Native source

Current-canonical private copy from `native-after-equipment-redirect-d45fe30e8`: source SHA568200017bcb3b5ed6f17931fc0994fff502c5c801f93f96197fc718bcd493bf, copy SHA dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae.

- 451294 is passive(Attr0=64), Attr3=0, SpellClassSet53, all SpellClassMask words0. Effect0 is APPLY_AURA/PROC_TRIGGER_SPELL42→451297, no effect class mask or EffectAttributes. Proc chance100, flags262160(0x40010: melee ability and harmful periodic), no ICD, PPM or charges.
- 451297 supplies native10% damage modifier per stack, max10. Native data does not declare a FoF-only family mask.
- Both451294 and451297 have no SpellAuraRestrictions rows. Local70448 SpellCastingRequirements/SpellAuraRestrictions/SpellShapeshift CSVs contain neither ID. The missing SQLite SpellCastingRequirements table was not treated as proof of no condition; the raw CSV was checked.
- TraitDefinition129658→TraitNodeEntry124820 is selected in the exact269 export at rank1/max1/active1, TraitSubTreeID0. The passive is not an accidentally enabled inactive hero node in this fixture.

TC a352b1fa SpellMgr.cpp1888–1903 refuses certain unrestricted generated procs only when CAN_PROC_FROM_PROCS is present, among its other predicates.451294 has Attr3=0 and is not in14860's `feral-parity-2026-10-23/tc-default-proc-refusal-census.json`. The supplied pinned TDB census reports no explicit451294 spell_proc row. No451294/451297/Momentum Boost script was found in pinnedTCsrc/server C++.

With the observed native source records, ordinary Tiger Palm is an eligible melee ability; there is no identified native FoF-only or hero/aura requirement. Keep generated-TC eligibility unless additional authoritative native data proves a restriction. Do not repurpose the unrelated default-proc refusal fix to suppress this producer.

## Actual observed states and alignment

`native-spell-diff-2026-10-23/green-998083262/269.fists_of_fury.three_ticks.json` primes with actual TP→TP. Native451297 already has2stacks before FoF; first three recorded channel impacts carry20/50/60% Momentum modifiers, with source-product4.08/5.10/5.44 and script-product1. Later state has7stacks. These are observed proc/state differences, not coefficients.

Oracle ordinal0 and1 both at1.004s are different REAL actions: `fists_of_fury_damage_mid2_2pc` and regular `fists_of_fury_damage`, per SimC1805–1827. Therefore the initial three-ordinal comparison was not three aligned channel ticks.14796's `fof-aligned/269.two_tp_then_fof.json` confirms identical TP→TP→FoF still has no SimC451297 before the channel. Compare native117418 channel impacts only against the regular oracle action.

Native1296621 tier2pc aura is absent before FoF; no extra tier tick provenance follows from the ordinary113656→1174181s/tick0 bridge. Native tier application belongs to14913/is1039; no WW-specific extra hit is implemented here. Mystic Touch's1.0 versus1.05 target multiplier is a separately approved learned8647 producer.

## Comparator-only divergence

Pinned SimC monk/sc_monk.cpp1699–1704 parses451297 and separately derives a FoF damage multiplier from melee haste and451294effect1. Its1730–1734 triggers451297 after FoF impact;1839–1841 expires it after the final tick and triggers the speed buff. That is narrower than the native generated proc source above. The comparator explains different stack histories; it does not override native masks or authorize copying its haste formula into runtime. No numerical parity claim or native mechanic fix follows until actual action identities, initial state and native eligibility are aligned.
