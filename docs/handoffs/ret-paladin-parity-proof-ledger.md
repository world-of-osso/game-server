# Retribution native parity — active (2026-10-23)

## Goal and authority

Spec70: every SimC76f8118 APL action plus selected talent effects through native NPC casts. DB270448 and TCa352b1fa own mechanics; SimC supplies only cited missing-data policies and rotation decisions. Same-gear/talents DPS acceptance: ±10% single and three targets. Parent owns integration/final gate. No canonical database mutations.

## Observed initial blockers

- Historical native-fixed-time-health-v1/70.native.1.json stops at `NATIVE_ASSISTED_COMBAT_GAP`: rule6369/type12, values[2,10,0], action53385, fight1.95s/40 decisions. This is the Player adapter, not evidence of failed native Ret spell effects. Keep it a separate gap; do not extend it.
- Native follower catalog has11 profiles and none for70. Lead approved a **private-copy-only** Captain Garrick209057 class2/spec70/DPS profile, retaining actual NPC template/armor.
- Ordinary initializer derives KnownSpells from controller rows and attaches no TraitLoadout. Shared native-book takeover tests are pushed at ef331486f, production unchanged; genuine RED pending capacity.

## Reference and tolerance

| Case | Existing SimC DPS | Accepted native interval | Native |
|---|---:|---:|---|
| single | 178061.27 | 160255.14–195867.40 | not measured |
| three | 321346.51 | 289211.86–353481.16 | not measured |

Exact artifacts: engine `data/diagnostics/dps-parity-2026-10-21/all-spec-expanded-references/70.reference.{single,three}.normalized.json`; current census `data/diagnostics/ret-parity-2026-10-23/ret-source-census-v1.json` retains literal APL, selected allocations, native effect/cost/cooldown/proc rows and cited TC constant children. Frozen native source SHA256 `464fbb4629af22f1d32ebdd5ad35b6f5800147a4eea1aa4755d32dedb99ee3c7`. Source presence is **not** execution proof.

## Per-spell census v1

Every row remains runtime-unqualified. APL base replacements, background children and tiered talents are distinct rows. TC edge absence does not imply absence of a script: Execution Sentence aura and Crusading Strikes require reading their source classes as well. Selected hero keystone427445 is allocated but inactive; do not grant its effects.

| Spell | Role/source | Native data | Test/status | Gap |
|---|---|---|---|---|
| 633 Lay on Hands | talent:102583 | 67/0,136/0 | pending native cast/effect | unqualified |
| 1022 Blessing of Protection | talent:102604 | 6/39,6/4,6/87,6/31 | pending native cast/effect | unqualified |
| 1044 Blessing of Freedom | talent:128251 | 6/77,6/77,6/31 | pending native cast/effect | unqualified |
| 6603 Auto Attack | APL:auto_attack | 78/0 | pending native cast/effect | unqualified |
| 6940 Blessing of Sacrifice | talent:102602 | 6/81,6/69,3/0,6/87,6/31,38/0,38/0,6/41,6/41 | pending native cast/effect | unqualified |
| 20271 Judgment | APL:judgment; reference child/action | 2/0,6/129,2/0 | pending native cast/effect | unqualified |
| 24275 Hammer of Wrath | APL:hammer_of_wrath; reference child/action | 2/0,30/0,2/0,6/129 | pending native cast/effect | unqualified |
| 26573 Consecration | reference child/action | 6/226,179/0,28/0 | pending native cast/effect | unqualified |
| 31884 Avenging Wrath | APL:avenging_wrath; talent:102519 | 6/108,6/108,6/290,6/344,6/4,6/334,6/262,6/107,6/4,6/318,6/193,6/108,6/129 | pending native cast/effect | unqualified |
| 35395 Crusader Strike | APL:crusader_strike | 2/0,30/0 | pending native cast/effect | unqualified |
| 53385 Divine Storm | APL:divine_storm; talent:102499; reference child/action | 2/0,3/0,64/0,64/0 | pending native cast/effect | unqualified |
| 81297 Consecration | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 85256 Templar's Verdict | APL:templars_verdict | 3/0,3/0 | pending native cast/effect | unqualified |
| 96231 Rebuke | APL:rebuke; talent:136594 | 68/0 | pending native cast/effect | unqualified |
| 114154 Unbreakable Spirit | talent:102603 | 6/108 | pending native cast/effect | unqualified |
| 146956 Light's Revocation | talent:102596 | 6/4,6/4,6/262 | pending native cast/effect | unqualified |
| 156322 Eternal Flame | talent:117692 | 6/8,10/0,3/0,3/0,3/0,3/0 | pending native cast/effect | unqualified |
| 183416 Blessing of Dawn | talent:115673 | 6/4,35/4,6/4,6/4,6/4,6/69,6/23 | pending native cast/effect | unqualified |
| 184575 Blade of Justice | APL:blade_of_justice; talent:102498; reference child/action | 2/0,30/0,64/0 | pending native cast/effect | unqualified |
| 190784 Divine Steed | talent:102625 | 3/0 | pending native cast/effect | unqualified |
| 204054 Consecrated Ground | talent:102518 | 6/4,6/107,6/108,3/0,6/218 | pending native cast/effect | unqualified |
| 224239 Divine Storm | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 230332 Cavalier | talent:102592 | 6/411 | pending native cast/effect | unqualified |
| 231663 Greater Judgment | talent:102590 | 6/4 | pending native cast/effect | unqualified |
| 234299 Fist of Justice | talent:136487 | 6/107 | pending native cast/effect | unqualified |
| 255937 Wake of Ashes | APL:wake_of_ashes; talent:102497; reference child/action | 2/0,30/0,64/0,64/0,64/0 | pending native cast/effect | unqualified |
| 305394 Unbound Freedom | talent:115454 | 6/107,0/0 | pending native cast/effect | unqualified |
| 326734 Healing Hands | talent:115481 | 6/4,6/4,6/4 | pending native cast/effect | unqualified |
| 343527 Execution Sentence | APL:execution_sentence; talent:115435 | 6/4,6/4,6/4,64/0,64/0 | pending native cast/effect | unqualified |
| 375576 Divine Toll | APL:divine_toll; talent:135564; reference child/action | 3/0,3/0,3/0,30/0,3/0,3/0 | pending native cast/effect | unqualified |
| 383328 Final Verdict | talent:102504; reference child/action | 2/0,3/0 | pending native cast/effect | unqualified |
| 383342 Holy Blade | talent:115022 | 6/107 | pending native cast/effect | unqualified |
| 383344 Expurgation | talent:114830 | 6/4 | pending native cast/effect | unqualified |
| 383346 Expurgation | reference child/action | 6/3,6/270 | pending native cast/effect | unqualified |
| 383396 Tempest of the Lightbringer | talent:115452 | 6/4,6/4 | pending native cast/effect | unqualified |
| 383921 Highlord's Judgment | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 384027 Divine Resonance | talent:115468 | 6/285 | pending native cast/effect | unqualified |
| 385427 Obduracy | talent:102622 | 6/129,6/229,6/4 | pending native cast/effect | unqualified |
| 385450 Seal of Might | talent:102612 | 6/137,6/318 | pending native cast/effect | unqualified |
| 385515 Holy Aegis | talent:102597 | 6/290,6/142,6/4 | pending native cast/effect | unqualified |
| 385633 Auras of the Resolute | talent:102587 | 6/4 | pending native cast/effect | unqualified |
| 387113 Execution Sentence | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 387170 Empyrean Legacy | talent:115453 | 6/42,3/0 | pending native cast/effect | unqualified |
| 402964 Sanctified Plates | talent:115034 | 6/137,6/229,6/142 | pending native cast/effect | unqualified |
| 402971 Jurisdiction | talent:102516 | 6/107,6/219,3/0,6/108 | pending native cast/effect | unqualified |
| 403026 Burning Crusade | talent:115024 | 6/218,6/218 | pending native cast/effect | unqualified |
| 403509 Zealot's Fervor | talent:115165 | 6/319 | pending native cast/effect | unqualified |
| 403530 Punishment | talent:136592 | 35/4,6/42 | pending native cast/effect | unqualified |
| 403664 Blades of Light | talent:115443 | 6/108 | pending native cast/effect | unqualified |
| 403695 Truth's Wake | reference child/action | 6/33,6/3,6/345,6/271,6/537,6/537 | pending native cast/effect | unqualified |
| 404357 Guided Prayer | talent:136503 | 6/42,6/4 | pending native cast/effect | unqualified |
| 404436 Light of Justice | talent:114828 | 6/453 | pending native cast/effect | unqualified |
| 404512 Highlord's Wrath | talent:102506 | 6/219,3/0,6/219,3/0 | pending native cast/effect | unqualified |
| 404542 Crusading Strikes | talent:136809 | 6/361,6/342 | pending native cast/effect | unqualified |
| 405345 Wake of Ashes | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 405350 Wake of Ashes | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 405355 Seething Flames | talent:115043 | 6/4 | pending native cast/effect | unqualified |
| 405461 Improved Judgment | talent:102505 | 6/411,6/411 | pending native cast/effect | unqualified |
| 406064 Art of War | talent:102493 | 6/4,6/4 | pending native cast/effect | unqualified |
| 406154 Heart of the Crusader | talent:115483 | 6/344,6/334,6/108,6/108 | pending native cast/effect | unqualified |
| 406545 Holy Flames | talent:115438 | 6/4,6/4 | pending native cast/effect | unqualified |
| 406647 Templar Slash | APL:templar_slash | 2/0,30/0 | pending native cast/effect | unqualified |
| 406872 Divine Wrath | talent:115439 | 6/107 | pending native cast/effect | unqualified |
| 407067 Rush of Light | talent:115475 | 6/231 | pending native cast/effect | unqualified |
| 407480 Templar Strike | APL:templar_strike | 2/0,30/0,64/0 | pending native cast/effect | unqualified |
| 408385 Crusading Strikes | reference child/action | 2/0,3/0 | pending native cast/effect | unqualified |
| 408459 Divine Purpose | talent:128243 | 6/4 | pending native cast/effect | unqualified |
| 427445 Light's Guidance | talent:117813/inactive | 6/4,3/0,3/0,3/0 | pending native cast/effect | unqualified |
| 427453 Hammer of Light | APL:hammer_of_light | 2/0,64/0 | pending native cast/effect | unqualified |
| 431377 Dawnlight | talent:117696 | 6/4,6/4,6/4,6/4 | pending native cast/effect | unqualified |
| 431380 Dawnlight | reference child/action | 6/3,3/0,6/33 | pending native cast/effect | unqualified |
| 431399 Dawnlight | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 431402 Luminosity | talent:117677 | 6/107,6/107 | pending native cast/effect | unqualified |
| 431404 Solar Grace | talent:136008 | 6/193 | pending native cast/effect | unqualified |
| 431406 Will of the Dawn | talent:117777 | 6/226,6/468,6/42,6/42 | pending native cast/effect | unqualified |
| 431413 Sun Sear | talent:117669 | 6/4 | pending native cast/effect | unqualified |
| 431414 Sun Sear | reference child/action | 6/3 | pending native cast/effect | unqualified |
| 431425 Sun's Avatar | talent:117702 | 6/108,6/4,6/4,6/4,6/4,6/4,6/4,6/4,6/4 | pending native cast/effect | unqualified |
| 431474 Second Sunrise | talent:117683 | 6/4,6/4 | pending native cast/effect | unqualified |
| 431480 Gleaming Rays | talent:117778 | 6/108,6/108 | pending native cast/effect | unqualified |
| 431911 Sun's Avatar | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 439760 Aurora | talent:117666 | 6/4,6/4 | pending native cast/effect | unqualified |
| 445200 Blessing of An'she | talent:117668 | 6/108 | pending native cast/effect | unqualified |
| 446663 Burn to Ash | talent:114824 | 6/4,6/4 | pending native cast/effect | unqualified |
| 454351 Avenging Wrath | reference child/action | 6/108,6/108,6/290,6/334,6/344,6/454,6/262,6/107,6/285,6/318,6/193,6/108,6/108 | pending native cast/effect | unqualified |
| 469416 Lightbearer | talent:128257 | 6/226,6/4 | pending native cast/effect | unqualified |
| 469439 Inspired Guard | talent:128260 | 6/107,6/107 | pending native cast/effect | unqualified |
| 469770 Blessed Calling | talent:128264 | 6/107,6/107,6/107,6/107 | pending native cast/effect | unqualified |
| 469780 Lead the Charge | talent:128263 | 6/4,6/4,6/4,6/129 | pending native cast/effect | unqualified |
| 1232917 Refreshment | reference child/action | 64/0,6/23 | pending native cast/effect | unqualified |
| 1235108 Flask of the Magisters | reference child/action | 6/189 | pending native cast/effect | unqualified |
| 1236994 Potion of Recklessness | reference child/action | 6/4,6/189,6/189,6/189,6/189,6/189,6/189,6/189,6/189,6/4 | pending native cast/effect | unqualified |
| 1241288 Hammer of Wrath | talent:133481 | 6/4 | pending native cast/effect | unqualified |
| 1241945 Blessing of Dusk | talent:128259 | 6/226,6/87 | pending native cast/effect | unqualified |
| 1241958 Vengeful Wrath | talent:128241 | 6/4 | pending native cast/effect | unqualified |
| 1253598 Crusade | talent:135565 | 6/4,6/4 | pending native cast/effect | unqualified |
| 1260251 Execution Sentence | reference child/action | 2/0,6/4 | pending native cast/effect | unqualified |
| 1261111 Light Within | talent:137024 | 6/4 | pending native cast/effect | unqualified |
| 1261113 Light Within | talent:137025 | 6/219,6/107 | pending native cast/effect | unqualified |
| 1261159 Light Within | talent:137023 | 6/4 | pending native cast/effect | unqualified |
| 1261160 Light Within | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 1261562 Shield of Vengeance | talent:136127 | 6/107 | pending native cast/effect | unqualified |
| 1263782 Walk Into Light | talent:117691 | 6/4,6/4,6/4 | pending native cast/effect | unqualified |
| 1263787 Endless Gleam | talent:136006 | 6/4,6/4,6/4 | pending native cast/effect | unqualified |
| 1263920 Born in Sunlight | talent:136007 | 6/42 | pending native cast/effect | unqualified |
| 1264426 Void-Touched | reference child/action | 6/29 | pending native cast/effect | unqualified |
| 1279408 Hammer of Wrath | reference child/action | 2/0,30/0 | pending native cast/effect | unqualified |
| 1286970 Rune of Unleashed Fire | reference child/action | 2/0,3/0 | pending native cast/effect | unqualified |
| 1287663 Rune of Lingering | reference child/action | 6/3,3/0 | pending native cast/effect | unqualified |
| 1305374 Devour Morsel | reference child/action | 2/0,3/0 | pending native cast/effect | unqualified |
| 1306624 Perfected Guillotine | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 1306635 Venomfang | reference child/action | 6/3,64/0 | pending native cast/effect | unqualified |
| 1306639 Venomfang Burst | reference child/action | 2/0 | pending native cast/effect | unqualified |
| 1306923 Divine Arbiter | reference child/action | 2/0,2/0 | pending native cast/effect | unqualified |

## Policy ledger

| Policy | Source | Status |
|---|---|---|
| Native NPC rotation | `apl_paladin.cpp:11–78` pinned76f8118 | present, not ported; not mislabelled absent |
| Player rule6369/type12 | native70448 AssistedCombat | unverified numeric/predictive semantics; outside NPC rotation scope |
| Race-only lights_judgment/fireblood/arcane_torrent | same APL cooldown/generator lists | present, unavailable to selected Dwarf race; native NPC race0 remains a comparison gap |
| Consumables/item/external buff actions | same APL | present, not yet qualified; no synthetic stat grants |

## Proof ledger

| Revision/scope | Command/artifact | Result |
|---|---|---|
| abb7328f0 baseline | read-only historical70 failure/trace, catalog SQL | blockers identified, no DPS measurement |
| ef331486f test-only | `follower_native_book_ --ignored --nocapture --test-threads=1` | queued; quiet window until02:28 |

Completion remains open: native kit/effects, full APL decisions, both dummy measurements and Stockade observation are unproved.
