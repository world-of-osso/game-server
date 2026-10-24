# Retribution native parity — active (2026-10-23)

## Goal and authority

Spec70: every SimC76f8118 APL action plus selected talent effects through native NPC casts. DB270448 and TCa352b1fa own mechanics; SimC supplies only cited missing-data policies and rotation decisions. Same-gear/talents DPS acceptance: ±10% single and three targets. Parent owns integration/final gate. No canonical database mutations.

## Observed initial blockers

- Historical native-fixed-time-health-v1/70.native.1.json stops at `NATIVE_ASSISTED_COMBAT_GAP`: rule6369/type12, values[2,10,0], action53385, fight1.95s/40 decisions. This is the Player adapter, not evidence of failed native Ret spell effects. Keep it a separate gap; do not extend it.
- Native follower catalog has11 profiles and none for70. Lead approved a **private-copy-only** Captain Garrick209057 class2/spec70/DPS profile, retaining actual NPC template/armor.
- Ordinary initializer derives KnownSpells from controller rows and attaches no TraitLoadout. Shared native-book takeover tests are pushed at ef331486f, production unchanged; genuine RED pending capacity.

## Split-B integration priority

Lead reported is1061 for98b37851f full-suite qualification (only known failures, CHECK0/FMT0), but no landing because masterd675bf8fb split B conflicts. Current Ret descendants65c0a1a8b preserve that packet plus approved proc-recipient/Empyrean work. Merge keeps the sole ItemProcInputs catalog, native ItemSparse.ItemSet membership and item-name index while retaining master's faction eligibility/public test-support boundaries. Ret simulation fixtures now live in `crates/server-simulation/src/dungeon_simulator/dps_parity/`; runtime policies remain in `crates/server/src/spell_cast/scripts/`.

Split-only adjustments expose existing native fact readers/metadata required by the separate test crate. Optional action normalization moves unchanged into the canonical knowledge helper's test-support API so core tests do not depend back on their simulation consumer. Temporary Guillotine callback observer is retired; archived provenance receipts and explicit real220637/no-self-damage assertions remain. No new mechanic or canonical write. Merge7911b9d3e and visibility-only follow-upd38586b3e precede lead-requested `cargo check -p server -p server-simulation --tests -j2`: PASS39.80s, zero warnings. `cargo fmt -p server -p server-simulation -- --check`: PASS. Receipt `/var/tmp/osso/ret-split-b-proof.json`; both logs and check-log SHA retained. Lead received exactd38586b3e for immediate gating; branch frozen there, no child push. Development continues separately on local `ret-paladin-literal-development-d385` with approved development-only target-contextdc525f641 merged as270de9504. This merged development tree is not the checked gate scope. No precombat merge; its implementation/RED remains owned and pending.

## Literal adapter qualification — local development only

| Capability | Coverage | Proof |
|---|---|---|
| HolyPower alias | Real native Judgment20271 must earn one displayed point; no power grant/reset | Genuine RED at0dd2005c6: unknown native operand `holy_power` |
| Five selected talent aliases / eight buffs / target-owned Expurgation and Execution Sentence | Shared NativeFacts reads actual TraitLoadout and aura state | Genuine RED at0dd2005c6: unknown talent alias `radiant_glory`; remaining assertions not reached |
| Unknown resource/talent/buff/cooldown/dot aliases | Explicit errors, not fabricated zero facts | Control PASS in the same run |
| Action bindings,41-row source selection and native dispatch | Sourced18identity map/17non-potion descriptors and source asset added after RED; fixture rotation not wired | d0b223e66: genuine unknown Judgment alias and missing precombat list RED65.12s →21bd6e051 unchanged5GREEN67.55s/zero warnings, including3 binding controls; broader dispatch remains open |
| Precombat execution | Shared API pending lead-approved implementation | Not merged; no local substitute |

RED:2failures/1controlPASS50.59s; compilation13m12s. Log `/var/tmp/osso/ret-literal-bindings-red-0dd2005c6.log`. Binding-only b137e05fd earns unchanged **3/3GREEN88.76s**, warning-free17.64s compile; `/var/tmp/osso/ret-literal-bindings-green-v1.{log,proof.json}`. Existing aura assertions cover the observed initial state, not every possible positive producer/expiry transition. Source: engine `data/diagnostics/ret-parity-2026-10-23/ret-literal-bindings-source-v1.json` and pinned76f8118 spell lookups. No gameplay handler changes or DPS measurement. Next slice consumes the unchanged source rows (SHA256 `5eb6731701bb8653007f548472e4e21645eac987f8af1121f27c7876918d2df5`, joined by newline without terminal newline); source assignment/list normalization delegates all expressions/priority to the existing shared Program. Two native dispatch/source REDs are archived as `ret-literal-dispatch-source-red-v1.log` in the same engine diagnostics directory. Self/area routing follows native spell-effect targets; broad per-ability execution remains unqualified. Precombat snapshot selection is not a local replacement for the pending shared cursor/poll API.

## Reference and tolerance

| Case | Existing SimC DPS | Accepted native interval | Native |
|---|---:|---:|---|
| single | 178061.27 | 160255.14–195867.40 | not measured |
| three | 321346.51 | 289211.86–353481.16 | not measured |

Exact artifacts: engine `data/diagnostics/dps-parity-2026-10-21/all-spec-expanded-references/70.reference.{single,three}.normalized.json`; current census `data/diagnostics/ret-parity-2026-10-23/ret-source-census-v1.json` retains literal APL, selected allocations, native effect/cost/cooldown/proc rows and cited TC constant children. Frozen native source SHA256 `464fbb4629af22f1d32ebdd5ad35b6f5800147a4eea1aa4755d32dedb99ee3c7`. Source presence is **not** execution proof.

## Per-spell census v1

These row labels retain the initial source-only census. Scoped runtime receipts below supersede them for the explicitly tested relationships; full natural-rotation qualification remains open. APL base replacements, background children and tiered talents are distinct rows. TC edge absence does not imply absence of a script: Execution Sentence aura and Crusading Strikes require reading their source classes as well. Selected hero keystone427445 is allocated but inactive; do not grant its effects.

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

## Private Ret profile receipt

`f285b1d4b`, `scripts/dps_parity_ret_profile.sql` changes one Captain Garrick209057 profile in `world.ret-native.db`: class2 remains; spec70/DPS, native Judgment20271 and AvengingWrath31884 legacy fields. Actual template/model/armor retained. `ret-profile-copy.receipt.json` records source/prepared rows and copy SHA256 `8e2228d2c2c656a3888425e8845684e73a17f8d9f123418e1768612cd01a7db7`; SQLite quick_check passed. No Ret kit rows yet; no canonical edit or DPS proof.

## Next authorized generic mechanics

Parent approved Aura361 native swing replacement (TC Unit.cpp2288–2349), TC CrusadingStrikes406833 stack2→406834, and Aura4/nonzeroTriggerSpell default proc dispatch (TC SpellAuraEffects.cpp1463–1492). Native-book slice first; each mechanism needs pushed ignored RED then observed failure before implementation. Weapon-derived AP is owned by14754; Ret does not duplicate that change.

### Aura4 trigger census (potential source impact, not runtime qualification)

All1267 native difficulty0 Aura4+Trigger roots are retained in `aura4-trigger-source-census.json`. The following56 have native trait, specialization, or class-mask skill evidence. Existing990 starter exports establish only six learned roots across the11 current follower specs; that smaller intersection is not an all-spec census. Generic proc eligibility and script-owned amount/target behavior must still be proven. In particular LightTitans378405 already dispatches an amount-overridden child: adding an unconditional default child would double fire.

| Root | Native class family | Learned starter specs | Native children |
|---|---|---|---|
| 2823 Deadly Poison | Rogue | not in11 exports | 2818 |
| 13877 Blade Flurry | Rogue | not in11 exports | 385835 |
| 30884 Nature's Guardian | Shaman | 262 | 31616 |
| 32388 Shadow Embrace | Warlock | not in11 exports | 32390 |
| 49028 Dancing Rune Weapon | Death Knight | not in11 exports | 1206 |
| 51128 Killing Machine | Death Knight | not in11 exports | 51124 |
| 53563 Beacon of Light | Paladin | not in11 exports | 53651 |
| 53576 Infusion of Light | Paladin | not in11 exports | 54149 |
| 59057 Rime | Death Knight | not in11 exports | 59052 |
| 76857 Mastery: Critical Block | Warrior | 73 | 12880 |
| 102351 Cenarion Ward | Druid | not in11 exports | 102352 |
| 135288 Tooth and Claw | Druid | not in11 exports | 135286 |
| 156910 Beacon of Faith | Paladin | not in11 exports | 53651 |
| 195300 Transfer the Power | Monk | not in11 exports | 195321 |
| 201900 Hot Hand | Shaman | not in11 exports | 215785 |
| 203415 Fury of the Eagle | Hunter | not in11 exports | 203413,203413,203413,203413 |
| 205145 Demonic Calling | Warlock | not in11 exports | 205146,205146 |
| 255652 Light's Reckoning | no class family; source row retained | not in11 exports | 256896 |
| 315496 Slice and Dice | Rogue | not in11 exports | 426605 |
| 315584 Instant Poison | Rogue | not in11 exports | 315585 |
| 321291 Death and Madness | Priest | not in11 exports | 322098 |
| 341273 Unfurling Darkness | Priest | not in11 exports | 341282 |
| 371032 Terror of the Skies | Evoker | 1467 | 372245 |
| 377073 Frigid Executioner | Death Knight | not in11 exports | 377074 |
| 378405 Light of the Titans | Paladin | 66 | 378412 |
| 378750 Cobra Sting | Hunter | not in11 exports | 336826 |
| 379993 Flash Freeze | Mage | not in11 exports | 44544 |
| 383344 Expurgation | Paladin | not in11 exports | 383346 |
| 387153 Burn to Ashes | Warlock | not in11 exports | 387154 |
| 387808 Divine Revelations | Paladin | not in11 exports | 387812 |
| 388505 Quick Sip | Monk | not in11 exports | 383764 |
| 390259 Commander of the Dead | Death Knight | not in11 exports | 390260 |
| 406380 Mastery: Timewalker | Evoker | not in11 exports | 413984 |
| 406646 Templar Strikes | Paladin | not in11 exports | 406648 |
| 424044 Underhanded Upper Hand | Rogue | not in11 exports | 1784 |
| 427445 Light's Guidance | Paladin | not in11 exports | 427441 |
| 428344 Soul Strike | Warlock | not in11 exports | 267964 |
| 434300 Bombardments | Evoker | not in11 exports | 434473 |
| 440031 Bind in Darkness | Death Knight | not in11 exports | 443532 |
| 441181 Menacing Presence | Evoker | not in11 exports | 441201 |
| 443177 Lifespark | Evoker | not in11 exports | 394552 |
| 445034 Lively Totems | Shaman | not in11 exports | 458101 |
| 467749 Bleak Arrows | Hunter | not in11 exports | 467718 |
| 471363 Headshot | Hunter | not in11 exports | 1277558 |
| 1218047 Primordial Storm | Shaman | not in11 exports | 1218125 |
| 1226140 Grove Guardians | Druid | 105 | 102693 |
| 1230339 Mindbender | Priest | not in11 exports | 200174 |
| 1233577 Call of the Forest | no class family; source row retained | not in11 exports | 1233578 |
| 1233613 Xe'ra's Embrace | no class family; source row retained | not in11 exports | 1233616 |
| 1234774 Volatile Magics | no class family; source row retained | not in11 exports | 1234775 |
| 1235159 Souls of the Caw | no class family; source row retained | not in11 exports | 1242102 |
| 1244788 Chaotic Inferno | Warlock | not in11 exports | 1244860 |
| 1245979 Valiant Crusade | Paladin | not in11 exports | 1246042 |
| 1247729 Thermal Void | Mage | not in11 exports | 1247730,1247730 |
| 1260647 Overclock | Evoker | not in11 exports | 431442 |
| 1262034 Gift of the Wild | Druid | 105,103 | 1126 |

## Explicit Hammer of Wrath knowledge policy

`RET_HAMMER_OF_WRATH_KNOWN_FROM_SELECTED_TALENT_SIMC` is lead-approved2026-10-23: NPC/parity spec70 with selected1241288 may know exactly24275. Pinned SimC `sc_paladin.cpp3407–3412` resolves Ret `hammer_of_wrath` to24275;4192 reads selected class talent. No effect/coefficient/timer changes. Native SkillLineAbility43121 is AcquireMethod3 **NeverLearned** (TC DBCEnums.h2375–2382), not a trainer grant. Native70448 SpellLearnSpell.csv and TDB spell_learn_spell have no24275/1241288 learn relation; TraitDefinition138267 has no override/visible spell. This is an explicit reference-derived knowledge exception, not native DB2 learning and not generic method3 import. Successor52788c80f reached genuine missing24275 assertion RED with native ordinary-book control PASS35.99s (`hammer-policy-red-v2.*`). v1 was setup-only: historical equipment snapshot lacks spec70; derived `hammer-policy-v1/world.ret-policy.db` uses the existing approved Garrick SQL delta, with source/copy SHA receipts. No canonical modification or DPS measurement. Native Ret grants root1241288 even with no requested purchases, so empty purchases are not an unselected control. Policy uses validated active TraitLoadout membership (including native grants); non-Ret66 and ordinary evaluator controls protect scope. Implementation3cade13b9 is confined to parity `inputs::install_traits`, using a named helper and no native evaluator change. Targeted **3/3GREEN53.63s**, warning-free3m12compile; `hammer-policy-green-v1.*`. Both selected follower/player, native granted-root reinstall, ordinary native evaluator and non-Ret66 behavior pass. Parent integration remains separate.

## Ret MID2 tier relationship source policy

Lead approved `SIMC_RET_TIER_MID2_DIVINE_PURPOSE_RELATIONSHIP`2026-10-23. Native70448 ItemSet2062 roots1296660/1296661 contain scripted Aura4 with TriggerSpell0; TC a352b1fa `spell_paladin.cpp` has no handlers for these roots or1306161/1306162/1306923. Pinned76f8118 `engine/class_modules/paladin/sc_paladin.hpp:1665–1718` supplies the missing relationship: consume an actual foreground Divine Purpose, trigger two-piece Divine Power1305230, trigger four-piece spender-kind Arbiter only if no kind is pending, then a different-kind finisher consumes the pending kind and executes1306923. Same-kind finishers do not consume it; background rules remain source-specific. Native child1306923 AP coefficients10.125/4.7249999 stay authoritative. Source receipt `ret-tier-native-source-v1.json` retains native effects/procs and historical DB SHA; this is not runtime proof. No fabricated DB2 trigger edge or TC attribution. At merged kh1057/efae45c26, native4pc missing Verdict-kind and2pc missing Divine Power both reached genuine assertion RED; no-set actual Purpose consumption control PASS.3cases51.71s, warning-free4m30compile; `ret-tier-red-v1.*` records queued36a8835ad versus actual merged build pin and binary SHA. `ret-tier-native-source-v2.json` adds actual Divine Power and pinned source hashes. Foreground implementation follows this RED: use the ordinary cast's native charged-modifier snapshot after charge consumption, retain script-owned same-kind charges, and dispatch actual native children at the supplied target. Missing primary target is an explicit named gap, never guessed. Foreground **3/3GREEN35.86s** at2c8ff6a73, warning-free3m02compile; `ret-tier-green-v3.*`.2pc/native Divine Power, no-set, same-kind retention and cross-kind native child damage pass. Prior4pc damage-only assertion observed no child events because actor had actually followed to30yards while native1306923 Speed50 projectile was still flying. Diagnosticaa8ec6f02 proves position/no-event boundary; fixture now waits for actual Damage/Miss outcome under its existing bounded native update budget, without changing gameplay timing, actor position or hit chance. Initial hook visibility compile failure and intermediate2PASS/1unqualified outcome remain in `ret-tier-green-v{1,2}.*`. Natural DP generation, Empyrean Legacy background exception and Hammer of Light runtime qualification remain open; this slice does not claim them.

Lead also approved `SIMC_RET_DIVINE_PURPOSE_TRIGGER`: selected408459 only, actual native effect0 chance, native223819 child and generic HolyPower-cost eligibility. TC a352b1fa `spell_paladin.cpp549–579` registers older223817, so selected408459→223819 is explicitly reference-derived (SimC76f8118 `sc_paladin.hpp1721–1726`), not a fabricated TC binding. TC `Spell.cpp8088–8098` tests cost-pool presence even if final cost is zero. New tests earn all spender power through native Judgments and retain unselected-aura, full-mask-triggered and non-HolyPower controls; no chance/power/charge override. v1/v2 reached setup failures, not missing-proc RED: isolated DP omission violates native earlier-row23-point gate; corrected to ordinary granted-only book plus actual learned TV. Follower funding/control also goes OutOfRange despite ordinary literal AutoAttack/Pursuing. Source trace: parity endpoints0/1.5 route via nearest frozen Stockade pack20375809=(66.8395,-22.9799,-0.820416); `walk_route.rs54–75` appends origin→graph→target. Lead notified for owned shared movement fix; no routing/range/position patch here. All follower cases retained. Complementary actual Dwarf Player70 ordinary-request test permits independent producer TDD without substituting for blocked follower proof. Actual Dwarf Player70 reached genuine missing-proc RED after32 naturally funded ordinary Final Verdict casts at699321a7d,60.70s; `ret-purpose-player-red-v1.*`. Lead-authorized origin/follower-parity-movement merged asd2b96e16e (module/wiki conflict resolution only), removing unrelated frozen Space from parity. Postmerge batch149.90s confirms non-HP/background controls PASS and repeats Player genuine missing-proc RED; follower positive/unselected now reach CasterDead rather than OutOfRange. `ret-purpose-postmovement-red-v1.*`. Lead directs checking diagnostic-party map retirement/unsummon before suspecting damage; no HP/stat changes. Selected producer fbb22d517 earns actual Player **1GREEN43.44s** (`ret-purpose-player-green-v1.*`); selected follower at diagnostic83572ae16 earns **1GREEN100.28s** (`ret-purpose-npc-lifetime-v1.*`). Generic recipient fix e49f143f5 then closes the complete unselected follower negative plus nullable/self/enemy controls: **4GREEN95.93s** (`native-proc-recipient-green-v1.*`). The fatal path was real self-damage, not actor deletion, as detailed below. Natural full-rotation parity remains unqualified.

## Empyrean Legacy relationship policy

Lead approved `SIMC_RET_EMPYREAN_LEGACY_RELATIONSHIP`2026-10-23. Native387170 Aura42→387178 has cast-success mask[0,4] and no family mask, so metadata-only dispatch grants it on every cast. Native387178's generic proc charge also lacks the intended spender predicate. TC a352b1fa has no387170/387178 handler; effective audited TDB spell_proc rows contain neither ID. SimC76f8118 `sc_paladin.cpp311–312` grants the buff on Avenging Wrath; ordinary/background action constructors explicitly use31884/454351. `sc_paladin_retribution.cpp713–716,1083–1088` consumes the buff on actual Verdict and schedules a free background53385 with1+native387170 effect1 percent. `sc_paladin.hpp1665–1718` permits that background's opposite-kind Arbiter consumption but forbids Divine Purpose consumption. Source receipt `ret-empyrean-source-v1.json` retains native fields and source hashes.

Scope: selected387170 only; suppress every-cast metadata grant only for its own driver, not generic dispatch; preserve native amounts, geometry and zero-delay scheduling. New native tests cover no-AW grant, AW→resolved TV/FV background damage with measured native percentage, actual Purpose instance/charge retention plus opposite Arbiter, and valid granted-only unselected control. Percentage is observed from real damage-stage output, not an assertion that a multiplier field was assigned. At67d83d489,3genuineRED/1unselected controlPASS56.19s, warning-free2m48compile (`ret-empyrean-red-v1.*`): Judgment funding incorrectly applies Legacy3times; both AW cases reach actual accepted AW then fail the expected retained Legacy buff. Implementation follows these REDs: source-selected Wrath hook for31884/454351, script-owned driver/buff predicates, actual Verdict consumption, zero-delay deferred callback using the existing native triggered-cast/damage-effectiveness path, and tier's explicit background-only exception. No new timer or copied damage formula. Purpose test also pins its presence before callback execution. Implementation e36e66e70 earns unchanged **4GREEN58.82s**, warning-free3m55compile (`ret-empyrean-green-v1.*`), including actual damage-stage percentage and retained Purpose instance/charge. Background454351 selected/unselected follow-up added at65c0a1a8b now earns **1GREEN57.65s** on b137e05fd's unchanged runtime policy; `/var/tmp/osso/ret-empyrean-background-wrath-green-v1.log`. Five unique cases qualified; natural full rotation remains open.

## Generic proc recipient self-damage

Lead authorized generic correction2026-10-23; no Guillotine-ID exception or HP/stat override. TC a352b1fa `Spell.cpp3903–3918` intentionally emits CAST_SUCCESSFUL with null actionTarget; `SpellAuraEffects.cpp6126–6155` uses nullable actionTarget for actor-side trigger and actor for recipient-side trigger. Do not substitute the spell's explicit target. Native default dispatch instead used `Some(other.unwrap_or(unit))`.

Actual fatal Guillotine path is distinct from the initial hypothesis: native1291728 proc masks are[4436,32], not CAST_SUCCESSFUL[0,4]. Test-only observer forwards the unchanged named callback and records real source events. All self-damage procs originate from triggered HolyPower Energize220637, recipient=caster, mask[16,0], can_proc=false, damage0; the actual native driver admits it. Actor exists/HP0, no map drift or unsummon. Missing-health snapshot compounds the wrong self hits. Actual enemy AA/TV recipients are retained in the same trace. Native1306604/1306624 both require explicit enemy6. No native TriggerSpell edge points at either child; named item callback owns them.

At55645d960, focused4cases67.60s: genuine default nullable-recipient RED (native273685 Aura42→273702 incorrectly applies its enemy-target aura to caster), unchanged unselected DP control fatal self-damage RED; implicit-caster270233→270232 and actual-enemy44480→44481 controls PASS. `native-proc-recipient-red-v1.*` retains complete observer/fatal events. Ret tier with the DP producer independently3GREEN54.56s at83572ae16 (`ret-tier-with-purpose-v1.*`); Player and follower selected DP positives GREEN43.44/100.28s, non-HP/background controls PASS, unselected negative remains blocked only by this self-damage.

Correction uses one native child-recipient predicate shared by default dispatch and named callback eligibility/dispatch: explicit enemy children require an actual non-self event recipient; missing/self is refused, implicit caster selectors retain their native semantics. Named eligibility checks before RPPM roll, preventing invalid self recipients from consuming attempts. Source/selection evidence, not a new item-name map or target fallback. At e49f143f5 the unchanged55645d960 RED packet is **4/4GREEN95.93s**, warning-free2m43compile; `native-proc-recipient-green-v1.*`. Unselected granted-only TV actor completes all32 naturally funded spenders, no selected DP; generic null actionTarget no longer becomes caster, while native implicit-caster buff and actual enemy damage controls remain valid. The supplied Protection filter matched normal tests but `--ignored` excluded them: no claim they ran. At16811e270, explicit220637 self-Energize/no1306624 self-damage/full32 unselected TV control plus six unchanged normal Protection item cases are **7/7GREEN107.17s**, warning-free5m54compile (`native-proc-recipient-controls-v2.*`). Ten unique cases across the strict4-case packet and this seven-case packet; no redundant generic rerun. Parent integration separate.

## Shared melee AP dependency

All AP-coefficient rows in the raw114-spell matrix now name14754's shared weapon-derived-AP fix as a dependency. Lead reported genuine TigerPalm RED91 damage versus TC358–377; no Ret per-hit ratio exists yet and Ret does not modify the shared formula. Parent integrates that fix before final same-gear measurements.

## Native permanent action override

Parent approved exposing selected DB2 TraitDefinition OverridesSpellID to NPC action resolution, not zero-override variant guessing. New ignored native fixtures request APL base85256 through the existing follower controller: selected native383328 must cast and deal its own damage; granted-only Ret must retain85256→224266 damage with no FinalVerdict. Exact76f8118 selected-rank fixture is validated through ordinary native reference traits. Both cases earn Holy Power with three ordinary native Judgments; no power, damage coefficient or timer override. RED pending; production resolver unchanged.

Native resolver first execution at77c0f56ed reached two fixture setup failures, not resolver RED: ordinary Judgment returned NoCharges after its actual charge consumption. The fixture now waits on NoCharges like other native readiness failures while preserving ordinary recharge and the bounded frame budget. No runtime resolver change. `native-action-red-v3-priority.{log,proof.json}` retains the setup evidence.

`native-action-red-v4-dev.{log,proof.json}` at1e38873a7 compiled successfully; both cases recorded three Judgments but neither spender. Not a qualified resolver RED: the unselected control also failed. Added an actual earned-power assertion with native rank aura/combat/event diagnostics before controller execution; no resource grants or production changes. Next run must identify the setup boundary before implementing the resolver.

After trait-point tip50eb68971 was queued as parent gatetp1034, merged shared literal driver9912a2917 as51a79c22b. Resolver fixture funding now uses the existing native resource-generation helper until actual earned Holy Power meets the finisher's catalog cost, not an admitted-cast count. Native StatsPlugin supplies ordinary derived-stat refresh; no power/SP/AP override. Production resolver still unchanged. Next RED uses the private point-enabled DB because the new spell loader requires TraitDefinitionEffectPoints.

Shared spell-oracle receipts are available under engine `data/diagnostics/simc-spell-oracle-2026-10-23/70/manifest.json`: Final Verdict19, Dawnlight28, Crusading Strike30 and Perfected Guillotine2 matching formula records; Divine Arbiter did not trigger. Reuse these receipts; no separate SimC hand-run.

## Judgment cast-time resource correction

Pinned TC a352b1fa `spell_paladin.cpp1062–1091` registers20271/275779/275773 resource child220637 on **OnCast**, gated by **HasSpell315867**. Native `scripts/batch10.rs94–102` and `batch16.rs32–45` instead require rank aura plus actual primary damage. Thus an impact miss is not a valid native explanation for losing Judgment Holy Power. Parent approved generic correction2026-10-23; no Ret-local exception or fixture power grant.

Three source-backed controls failed genuinely atde7b6e406: learned-rank power before impact0≠1, aura-only power1≠0, known-rank versus immune primary0≠1. Combined native batch9/9RED, compile8m38s/runtime56.50s; engine diagnostics `reconcile-primary-judgment-red-v3.{proof.json,full.log}`. Ret base trace proves first Judgment Miss(Miss), AP651/SP625 nonzero, and later real damage; selected still casts85256 on this pre-resolver branch.

Correction uses one shared OnCast hook for20271/275779/275773 in ordinary, triggered and costed-trigger paths, before payment/launch; trigger depth remains bounded. Removes both damage-hit consumers and obsolete immune-exclusion oracle; existing gain controls now pin learned rank separately. Added triggered/costed-trigger and once-after-impact regression coverage. At97d473c8f native Judgment/dummy16/Protection controls16/16PASS, runtime92.47s including the still-failing Ret base case; `judgment-cast-green-v1.{proof.json,full.log}`. Removed the now-unused batch8 import in a follow-up; final warning-free compilation pending. Behavior proof remains valid because that removal changes no executable logic.

Ret base now earns1HP on its first missed Judgment but loses0.14 to out-of-combat decay before the next hit; at frame441 it has2.86/3 and fails the unchanged600-frame bound. This is a separate combat-state/fixture-ordering boundary, not authorization to grant power or extend the bound. Shared resolver integration and native Ret DPS remain unqualified.

## Generic missed-hostile launch combat

Parent approved this separate boundary2026-10-23. TC a352b1fa `Spell.cpp8538` starts caster combat for every hostile miss except EVADE. Native `missiles.rs92` instead visits only `hit_units(plan)`, after defense removes missed/immune targets. Ret's first missed Judgment reports only actor in SpellGo hit_targets; its actual target is Kobold6, not31146. Dummy-specific47667b971 does not by itself cover this target.

New native launch controls use ordinary Judgment20271 with actual Dark Luck194356 miss penalty, Fireball133 versus actual Divine Shield642 immunity, and the native Returning/evade state. Real Kobold aggro components load from native entry6. Assert actual miss receipts, unchanged target HP and caster combat/reference state; no power or hit-roll overrides. Atdd0ed8fe5 MISS and IMMUNE each reached the exact missing active target-reference assertion (genuine RED); native miss receipts and zero damage passed. Evade's no-reference assertion passed, but its additional combat-status assertion exposed the shared caster fixture's deliberate `CombatStatus(true)` rage-test default. The launch fixture now clears that initial condition with production `end_all_combat`; no runtime miss-path implementation preceded RED. Native compile6m35s/runtime156.72s, no warnings; `reconciled-native-launch-red-v2.{proof.json,full.log}`.

Actual Ret trace: first miss leaves refsNone/combatfalse through frame220, enters combat on the later hit221, then its timed reference expires321. Thus never-entered combat, not a lost dummy reference, caused the first HP decay. Ret target remains native Kobold6; no fixture engage or resource grant.

Generic correction retains a `selected_units` vector on each PlannedEffect before defense filtering. Launch and normal delayed arrival use original selection for combat/timing, while actual hit effects still use only successful `units`; existing reachability/sanctuary checks filter both. Reflection still separates reflected effects; deflected/missed targets retain combat-only arrival. Mechanical empty-field additions cover named plans that construct their own target list. No spell-ID guard, combat timer, resource grant or hit-chance override. At02090739e launch MISS/IMMUNE/evade3PASS, retained missile/chain/reflection/moving/dead-target9PASS, Judgment5PASS and unchanged Ret base1PASS: warning-free compile2m48s/runtime65.39s; `native-miss-combat-green-v1.*`. First missed Judgment now enters combat at frame1, retains earned HP, reaches3 at441, and ordinary TV child224266 deals1302 damage. No fixture target/resource/600-frame change.

That command's only failure was WW setup missing DPS_PARITY_OUTPUT. Copied existing pinned269 profile/active traits into a new diagnostics directory; unchanged WW consumer subsequently1PASS47.55s with no recompilation (`ww-consumer-green-v1.*`). Parent requires the combined Judgment + ItemSet/Equipment + miss→combat tip merged with master8e7c7a685, replacing is1039. Selected Final Verdict and unselected TV controls **2/2GREEN24.52s** on mergedkh1057/efae45c26 (`ret-final-verdict-green-v2.*`): native383328 damage and base224266 damage1302, real earned HP, unchanged600-frame bound. v1's base miss was input ordering, not resolver failure: the surrogate DB changed Garrick to70 before at_level's initial ticks, whereas these unchanged controls initialize from original66 themselves. Restoring their exact historical equipment-current source restored native deterministic receipts; no hit chance, oracle or runtime change. Ret literal DPS remains open. Successor merged lead-authorized kh1057 resolver pin92d89f8c1 as8b80b52a8; preserved native missed-launch/OnCast behavior and complete Ret funding diagnostics. Tier test-only36a8835ad covers actual native Purpose consumption with2pc/4pc/no-set gear and same→cross-kind behavior; no tier implementation yet. Its admission remains queued under lead_ret_14752 because capacity reached SHED291GiB. Tests will cover the merged pin, not the pre-merge test-only revision.

## Combined integration qualification

Master8e7c7a685 merged cleanly asafba0d635. Post-merge `-p server --lib` focused bundle **44/44PASS147.33s**, compile5m22s/zero warnings; helper471.21s. Engine diagnostics `integrated-native-green-v3.{proof.json,full.log}` preserve exact revision, filters, command and result. Breakdown: Equipment9, WW consumer1, lifecycle/equipment6, Ret base1, shared set predicates3, commerce1, Judgment5, MISS/IMMUNE/evade3, travel/reflection9, Protection item controls6. Docs-only descendants do not invalidate this runtime proof.

Parent receives one combined replacement for is1039: native Judgment OnCast + sole ItemProcInputs catalog/count/eligibility and Equipment facts + generic missed-target combat. No canonical writes, Ret-local engage, fixture power/charge grants or timing/coefficients changed. Private equipment-current DB and isolated pinned269 input copies retain their existing source receipts. Explicitly ignored Venomfang missing-producer test remains outside this qualified slice, not fixed or hidden. Shared selected-FV resolver gate, native selected-FV damage, Ret literal adapter, tier child mechanics, ST/3T and Stockade remain open. No broad check/full suite or DPS-parity claim.
