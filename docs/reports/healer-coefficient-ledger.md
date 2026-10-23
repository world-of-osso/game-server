# Healer coefficient ledger

Verified source snapshot: 2026-10-22. DB2 `data/world.db`, Normal `DifficultyID=0`; TDB1210; TrinityCore `a352b1fa`. SimC `76f8118` has no healer HPS profile/reference. This ledger checks amounts, not healer decision policy or dungeon HPS parity.

## Formula and evidence

- TC `src/server/game/Entities/Unit/Unit.cpp:7345–7456`, `Unit::SpellHealingBonusDone`: adds healing-power × `BonusCoefficient` and attack-power × `BonusCoefficientFromAP`, applies done bonuses and spell mods. `SpellHealingBonusTaken` starts at7512.
- TC `src/server/game/Spells/SpellEffects.cpp:1246–1293`, `Spell::EffectHeal`: applies variance to done healing, then target taken bonuses. DB2 `Variance=0.05` is ±2.5%, not ±5%.
- Values below are unmodified coefficient probes at healing power100/AP100, not class passives, talents, critical healing, target effects, periodic totals, gear, or legal endgame builds. Zero coefficients on dummy/trigger parents are not zero healing.
- Companion JSON retains exact DB2 SpellEffect row IDs, effect indices, spell IDs, coefficients, aura periods, base values and variance. Amount probes remain **PENDING** until their targeted native test executes.

## Native amount probe scope

`dungeon_simulator::healer_coefficient_tests::healer_benchmark_core_direct_heal_amounts_match_db2_tc_ledger` covers15 direct child/direct heals across all seven specs. Follower profile absence does not block isolated player coefficient probes. Triggered native casts isolate amount from cast admission, except Penance: DB2 child47750 targets77 (channel target), so its probe uses actual47540→47757 and inspects the healing bolt. These are not full talent/empower qualifications. The initial child-only47750 fixture emitted CastSuccess without a heal; this was a fixture-admission failure, not evidence of a coefficient mismatch.

| Spec | Spell | Direct/periodic SP coefficient by effect | Base SP100 heal/tick | Amount proof |
|---|---|---|---|---|
| 65 | 19750 Flash of Light | e0 heal 3.1559999 | 315.6 | PENDING direct probe |
| 65 | 82326 Holy Light | e0 heal 32.5 | 3250 | PENDING direct probe |
| 65 | 20473 Holy Shock | dummy/absorb/trigger parent; inspect JSON | N/A | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 65 | 25914 Holy Shock | e0 heal 11.17 | 1117 | PENDING direct probe |
| 65 | 85673 Word of Glory | e0 heal 4.158 | 415.8 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 256 | 2061 Flash Heal | e0 heal 6.8200002 | 682 | PENDING direct probe |
| 256 | 47540 Penance | dummy/absorb/trigger parent; inspect JSON | N/A | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 256 | 47750 Penance | e0 heal 4.0110002 | 401.1 | PENDING direct probe |
| 256 | 17 Power Word: Shield | dummy/absorb/trigger parent; inspect JSON | N/A | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 256 | 194509 Power Word: Radiance | e1 heal 5.75 | 575 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 257 | 2060 Heal | e0 heal 9.72861 | 972.86 | PENDING direct probe |
| 257 | 2061 Flash Heal | e0 heal 6.8200002 | 682 | PENDING direct probe |
| 257 | 596 Prayer of Healing | e1 heal 5.7874198 | 578.74 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 257 | 139 Renew | e0 HoT 0.60927999 | 60.928 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 257 | 33076 Prayer of Mending | dummy/absorb/trigger parent; inspect JSON | N/A | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 264 | 77472 Healing Wave | e0 heal 15.01 | 1501 | PENDING direct probe |
| 264 | 8004 Healing Surge | e0 heal 11 | 1100 | PENDING direct probe |
| 264 | 61295 Riptide | e0 heal 6; e1 HoT 0.79699999 | 600; 79.7 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 264 | 1064 Chain Heal | e0 heal 2.3099999 | 231 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 270 | 116670 Vivify | e0 heal 2.5799999 | 258 | PENDING direct probe |
| 270 | 124682 Enveloping Mist | e0 HoT 1.4352; e2 heal 8.6112003 | 143.52; 861.12 | PENDING direct probe |
| 270 | 115175 Soothing Mist | e0 HoT 2.0999999 | 210 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 105 | 8936 Regrowth | e0 heal 5.3600001; e1 HoT 0.224968 | 536; 22.497 | PENDING direct probe |
| 105 | 774 Rejuvenation | e0 HoT 0.92400003 | 92.4 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 105 | 33763 Lifebloom | e0 HoT 0.26840001 | 26.84 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 105 | 18562 Swiftmend | e0 heal 10.37 | 1037 | PENDING direct probe |
| 105 | 48438 Wild Growth | e0 HoT 0.48100001 | 48.1 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 1468 | 355936 Dream Breath | dummy/absorb/trigger parent; inspect JSON | N/A | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 1468 | 361469 Living Flame | dummy/absorb/trigger parent; inspect JSON | N/A | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 1468 | 361509 Living Flame | e0 heal 7.8000002; e1 HoT 0.13 | 780; 13 | PENDING direct probe |
| 1468 | 366155 Reversion | e0 HoT 0.42750001 | 42.75 | NOT_PROBED_FULL_SPELL_BEHAVIOR |
| 1468 | 364343 Echo | e0 heal 2.4000001 | 240 | PENDING direct probe |

## Named source/coverage gaps

| Gap | Scope |
|---|---|
| NATIVE_HEALER_PROFILE_ABSENT | Dungeon replay specs65,256,264,270,1468. No new follower profiles or player healer policy invented. |
| NOT_PROBED_FULL_SPELL_BEHAVIOR | HoT tick totals, channel cadence, empower ranks, shields/absorbs, talents, legal builds, chain/area distribution and trigger-parent selection are not proven by isolated direct-heal probes. |
| TC_PARENT_CHILD_MAPPING | Holy Shock20473 → healing25914 is sourced in `spell_paladin.cpp:1260–1354`; Penance47540 → healing channel47757 →47750 in `spell_priest.cpp:3068–3306`; Living Flame361469 → healing361509 in `spell_evoker.cpp:505–538` and enum line70. Direct-child probes do not certify these parent paths. |
| SIMC_HEALER_HPS_REFERENCE_ABSENT | No SimC healer reference is fabricated; recorded dungeon deaths/overheal/mana/idle GCD/effective HPS are the benchmark. |
