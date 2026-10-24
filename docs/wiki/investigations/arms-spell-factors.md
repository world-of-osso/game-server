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
| Generic mastery root cause | Actual native114585 aura3188 plus hardcoded8; TC UpdateMastery uses aura318+rating only; native regression/repair tracked in character-stats |
| Armored to the Teeth / Battlefield Commander behavior | Named policies approved after TC source boundary; actual Arms/Protection four genuine REDs23.48s at e5cc1ef32, lifecycle GREEN pending |
| Redirected stat inputs / Arms2pc membership | Fresh master-based receipt required afterb67952ca4; no current input-gap claim |
| Target item-level-K branch | Native70448 GT ItemLevelByLevel FDID2012881 still unavailable;70575 is not a substitute |

## Sources

- Native evidence: `game-engine/data/diagnostics/native-spell-diff-2026-10-23/origin-089face19/arms/71.mortal_strike.player.json`; snapshot proof under `remeasure-eec5b6bcf/`; [capture implementation](../../../crates/server/src/dungeon_simulator/dps_parity/single_spell.rs).
- Oracle evidence: `game-engine/data/diagnostics/simc-spell-oracle-2026-10-23/71/mortal_strike.{json,txt,proof.json}`; public SimC76f8118 `engine/class_modules/sc_warrior.cpp:9176–9238,9324–9345` and `engine/player/player.cpp:5093–5100,11219–11222`.
- Public TC a352b1fa `StatSystem.cpp:255–283,549–559`, retained under `game-engine/data/diagnostics/follower-power-scaling-2026-10-22/`; native70448 SpellEffect IDs1018256/1197981/1200879/1261099/127950/1319314 from the frozen database.
- [Production stats](../../../crates/server/src/stats.rs), [aura modifiers](../../../crates/server/src/auras/stat_mods.rs), [character-stat contract](../../specs/character-stats.md).

## See Also

- [[simc-spell-oracle]] — exact action/actor/factor selection.
- [[follower-armor-penetration]] — separate target armor/K and class-follower policy.
- [[character-stats]] — generic producer and redirected stat allocation.
