# Arms Mortal Strike factor ledger

Verified 2026-10-23. Historical native code089face19 (documentation tipa5ea8d0b9), native70448 snapshotdd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae, compared with pinned SimC76f8118/70391. These receipts precede stat redirects landing atb67952ca4 and the generic mastery repair; they are not current-master parity acceptance.

## AP components

| Component | Historical native | Pinned oracle | Evidence |
|---|---:|---:|---|
| Base + equipped strength before plate bonus | 2537 | 2537 | Native strength2663.849854/1.05; oracle initial stat line2002 |
| Food / augmentation strength | 0 | 50 + 25 | Actual oracle precombat1232585/1264426, lines2089–2105; native explicit consumable gaps |
| Armor used by Armored to the Teeth | Not consumed | 3141.16 | Oracle cache armor and warrior strength override |
| Armored to the Teeth rank2 strength | 0 | 10% of armor | Native384124 actual aura226 amounts10/4; pinned warrior uses effect2 for non-Protection |
| Plate strength multiplier | 1.05 | 1.05 | Same initial strength reconstruction |
| Effective strength | 2663.849854 | 3072 | Oracle floors the attribute product |
| Raw weapon AP | 934 | 934 | Same364–757/3.6s weapon; both use integer native weapon AP |
| Battlefield Commander AP multiplier | 1 | 1.03 | Native424742 effect5 dummy amount3 present; pinned warrior AP override uses effect6 |
| Cast AP | 3597.849854 | 4126 | Actual formula receipts |

Native: `2537×1.05 + 934 = 3597.85`.
Oracle: `floor((2537+50+25+3141.16×0.10)×1.05)=3072`; `round((3072+934)×1.03)=4126`.
The native dummy/periodic-dummy values being present does not mean their class mechanics reach the sheet. TC a352b1fa's Warrior scripts, Unit, StatSystem and SpellAuraEffects contain neither mechanic; core226 explicitly delegates to scripts (1324–1326), and core4 has no Warrior stat handler. Exact-ID SQL search in the retained TC source tree also has no binding. Source hashes/native rows are retained in `mastery-b67952ca4/warrior-tc-source-boundary.proof.json`; the native snapshot has no `spell_script_names` table, so this does not claim complete TDB-binding coverage.

Lead explicitly approved `WARRIOR_ARMORED_TO_THE_TEETH_SIMC_POLICY` and `WARRIOR_BATTLEFIELD_COMMANDER_SIMC_POLICY`. [Contract](../../specs/character-stats.md#named-warrior-stat-policies): active native ranked amounts bind once in `auras/warrior_stats.rs`; no SimC numeric constants. Strength/armor are solved together in the existing stat producer, not iterated through stale CharacterStats. Vanguard's actual native70% strength-to-armor creates feedback for Protection's4%; all existing native armor factors still apply once. Commander composes into the existing AP percentage consumed by both sheet AP and weapon-inclusive cast AP. Revisit these policies when qualified TC implementations become available; do not keep alternate paths.

## Comparator armor divergence — do not tune native armor

Oracle armor is `(2569+27)×1.10×1.10=3141.16`. Its startup line1602 already parses Reinforced Plates382939 effect2 as an armor-multiplier10%; `warrior_t::composite_armor_multiplier` then multiplies that talent effect again. Native382939 rank2 has aura101 amount10 and computes one multiplier (integer armor2855). TC `Player::UpdateArmor` applies the aura101 total-percent factor once. This is a sourced comparator double application, not missing native armor. It also inflates oracle Armored to the Teeth strength. Neither source-derived effect values nor native armor should be changed to chase4126.

## Source-product reconstruction

The common passives are Arms137049 effect1 +15%, Two-Handed Weapon Specialization382896 effect1 +6%, and Sharpened Blades383341 effect1 +10%: product1.3409.

| Historical factor | Native | Oracle | Qualification |
|---|---:|---:|---|
| Mastery rating | 693 | 605 | Old pre-redirect receipt only; recheck freshb67952ca4+ input before calling a gap |
| Raw mastery points | `8+693/46+8` | `8+605/46` | Native hardcoded8 plus actual114585 aura3188; [generic mastery producer](../systems/character-stats.md) |
| Display/damage mastery coefficient | 1.10 | 1.10 | Native effect coefficient, unchanged |
| Mastery percentage | 34.171739 | 23.267391 | Exact reconstruction of actual snapshots |
| Arms2pc1296643 | absent | ×1.10 | Actual old native before/after auras versus oracle modifier line1776; fresh membership/admission check still required |
| Source product | 1.79910885 | 1.818181695 | `1.3409×1.34171739` versus `1.3409×1.10×1.23267391` |

The nearly matching source products hide offsetting differences. Adding a guessed1% factor, or treating the whole difference as a coefficient, would obscure both the duplicated mastery base and the absent historical2pc. No coefficient changes or forged aura state follow from this ledger.

## Coverage and remaining boundaries

| Capability | Proof level |
|---|---|
| AP and source-product discrepancy decomposition | Historical actual native/oracle receipts plus exact arithmetic and pinned source inspection |
| Generic mastery root cause | Native genuine16vs8 RED→three-class/lifecycle GREEN; current retained1GREEN8.20s and stats-filter21GREEN9.07s at98093ec85; [producer proof](../systems/character-stats.md#generic-mastery-base--2026-10-23) |
| Armored to the Teeth / Battlefield Commander behavior | Named policies approved after TC boundary; four genuine REDs23.48s at e5cc1ef32→four lifecycle GREEN19.13s at98093ec85 (productiona1af93852) |
| Redirected stat inputs / Arms2pc membership | Redirect/mastery605 match; generic activation8171bf2aa now earns1296643/1296644 from original four members. Native lifecycle2GREEN30.66s and fresh MS1GREEN16.91s at778524924; source product matches oracle |
| Target item-level-K branch | Native70448 GT ItemLevelByLevel FDID2012881 still unavailable;70575 is not a substitute |

## Fresh post-redirect/policy Arms receipt

At98093ec85 (productiona1af93852), immutable native snapshotcd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6, exact original71 reference gear/active traits:

| Observation | Actual native | Oracle / interpretation |
|---|---:|---|
| Mastery rating | 605 | 605; old693 was allocation mismatch, now resolved |
| Raw/display mastery | 21.152174 / 23.267391% | Matches `(8+605/46)×1.1`; duplicate base removed |
| Strength / caster armor | 2963.624756 / 2855 | Single native armor factor retained |
| Bare / cast AP | 3052.533447 / 4014.553467 | Commander3% reaches sheet and weapon-inclusive AP |
| Source product | 1.652892113 | Oracle1.818182 is exactly this×1.10 within printed/f32 precision |
| Result | hit25119 at3.1s | Oracle hit27935.1 at2.285s; not parity certification |
| Arms2pc runtime predicate / aura | `Ok(true)` / absent | Actual equipped-set activation missing |

The actual equipped list retains2067 members271456/271454/271459/271457, including original chest271459 with allocation donor268222. Native ItemSetSpell7750 specifies spell1296643/spec71/threshold2/subtree0; set flags and skill requirements are0. Native source modifiers actually applied are137049/1258398/383341/382896, not1296643. Thus neither membership nor native metadata is missing; do not retain an input-gap label for this result. The [generic activation follow-up](../systems/character-stats.md#native-equipped-set-auras--2026-10-23) reproduces actual Arms/Protection lifecycle failures before the shared production fix; it is not an Arms spell-ID multiplier.

Current AP reconstructs as `(2537×1.05 + 2855×10%×1.05 + 934)×1.03 = 4014.55375` (actual f32 value4014.553467). Remaining oracle4126 follows the recorded food50/augment25, comparator armor3141.16 and its attribute-floor/AP-round rules. No extra correction factor is missing from this decomposition, and no native armor tuning follows.

Evidence under `game-engine/data/diagnostics/native-spell-diff-2026-10-23/mastery-b67952ca4/`: `database.proof.json`, `warrior-policy-{red,green}.{log,proof.json}`, `policy-retained-{mastery,stats}.{log,proof.json}`, `policy-arms-remeasure.{log,proof.json}`, `arms-set-metadata.proof.json`, and `arms-after-policy/{71.mortal_strike.player.json,mortal-strike.diff.json,factor-ledger.proof.json}`. Consumer exit1 correctly records remaining differences; no consumer tests were rerun because its source did not change.

## Generic activation follow-up

At778524924 (production8171bf2aa), the same frozen native data/reference inputs now earn1296643 and1296644 as `EquipmentSet(2067)` auras. Native MS actually applies1296643's10% alongside137049/1258398/383341/382896; source product becomes1.8181816339492798 (oracle model1.818181695), not a fabricated comparison factor. Native damage25119→27631; AP4014.553467, caster armor2855 and mastery605/23.267391% stay unchanged. Remaining AP/input/variance differences still prevent a total-parity claim.

Actual normal-set lifecycle cases pass2/2 (30.66s): original-member equip/unequip,2/4-piece thresholds, same-actor specialization changes, and real Protection wrong-Arms exclusion/own-bonus control. The first Prot follow-up failure was a fixture error: it retained native Legs271455 plus Head while calling that one piece. The correction reads actual ItemSparse membership; production behavior was already correct. Retained Warrior policies pass4/4 (27.23s), fresh MS1/1 (16.91s), and both-package --tests check passes90.70s without warnings.

Evidence: `set-activation/{native-red,native-green,native-green2,retained-warrior-policies,arms-after-activation,final-check}.{log,proof.json}`, `protection-control-fixture.proof.json`, and `arms-after-activation/{71.mortal_strike.player.json,mortal-strike.diff.json,factor-ledger.proof.json}` under the same `mastery-b67952ca4/` root. Current receipt SHA5c786b848137c059f4ef175c46ffd2a304b6dc907e21af7dd4a5834ae36a54fa. [Shared production seam and conditional-set inventory](../systems/character-stats.md#native-equipped-set-auras--2026-10-23) records603's initial13 skill/one-subtree gaps. Consolidation preserves rt1086's existing sourced selectors rather than removing working behavior; deployment and fresh consolidated acceptance remain separate.

## Arms ST/3T band authority and current landing dependency

After603ca45e3, a fresh canonical online backup and exact71 profile/traits were frozen under `arms-bands-603ca45e3/` (database hash unchangedcd7230ac…). The validated binary is from778524924, SHA94df235c12cde8f1e08baf3b394ad0a85ed5a3fa383e341cb58ae2ec28117fa6. Its current full300s Player entry uses native AssistedCombat, not the literal Arms APL adapter. Both diagnostic attempts stop at native rule1187/condition12/values2,10,0 (step302, spell260708). Conditions11/12 remain named gaps, never guessed.

Those stopped diagnostic attempts are **not parity-authority runs**, and no native DPS/band is inferred from them or from the matching single-spell source product. Parity measurements require the literal pinned SimC Arms APL through shared `literal_rotation`. Compiled test discovery/current dispatch on603 has shared literal execution helpers but no full Arms literal measurement route. 14775 confirms the148-row literal Arms adapter and spec71 Player routing are unimplemented. They follow the owner's persistent RED and expected_combat_length work; therefore no existing landing tip/command can unlock603. Await that adapter/routing and its qualified FIRST/target tips rather than inventing a route.

| Scenario | Pinned reference DPS | Recorded±10% band | Native literal result |
|---|---:|---:|---|
|ST|188596.2721|169736.6449–207455.8993|Blocked on qualified literal adapter route|
|3T|328901.6784|296011.5105–361791.8462|Blocked on qualified literal adapter route|

`band-status.proof.json`, `compiled-test-list.*`, native diagnostic error/gap JSON and separate setup-failure artifacts retain the exact distinction. All seven AssistedCombat CSVs match the independent frozen70448 export; no later-build substitution or alternate priority was introduced. This is a measurement-route dependency, not a reversal of the earned set/armor/AP source proofs.

## Sources

- Native evidence: `game-engine/data/diagnostics/native-spell-diff-2026-10-23/origin-089face19/arms/71.mortal_strike.player.json`; snapshot proof under `remeasure-eec5b6bcf/`; [capture implementation](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/single_spell.rs).
- Oracle evidence: `game-engine/data/diagnostics/simc-spell-oracle-2026-10-23/71/mortal_strike.{json,txt,proof.json}`; public SimC76f8118 `engine/class_modules/sc_warrior.cpp:9176–9238,9324–9345` and `engine/player/player.cpp:5093–5100,11219–11222`.
- Public TC a352b1fa `StatSystem.cpp:255–283,549–559`, retained under `game-engine/data/diagnostics/follower-power-scaling-2026-10-22/`; native70448 SpellEffect IDs1018256/1197981/1200879/1261099/127950/1319314 from the frozen database.
- [Production stats](../../../crates/server/src/stats.rs), [aura modifiers](../../../crates/server/src/auras/stat_mods.rs), [character-stat contract](../../specs/character-stats.md).

## See Also

- [[simc-spell-oracle]] — exact action/actor/factor selection.
- [[follower-armor-penetration]] — separate target armor/K and class-follower policy.
- [[character-stats]] — generic producer and redirected stat allocation.
