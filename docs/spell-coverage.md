# Server spell behaviour coverage — 2026-10-11

Matrix runtime **449bb8e**, base `origin/spell-dummy22` **ea91c10**. [Capabilities/source/census/proof](spell-dummy23-coverage.md). Census **157 roots/157 reviewed/0 underived**, versus157/129/28; all157 unresolved upper-bound roots retained. Zero underived is zero unreviewed, not zero missing mechanics. Scripted Shadow-Blades combo doubling and real Rune-Weapon owner parry implemented; targeted12/12 GREEN, warning-free; check/fmt exit0 at1674fb2 (Rust unchanged). E3=111/119, aura4=63/71, aura226=18/19 unchanged. No broad suite/live/server/UDP5000.

Source: game-server runtime `449bb8e41f6cbe25da9b26f563a3d143b953f7d8`, read-only world.db and local Mainline12.1.0.69933 CSV. Shared-protocol44b2b0f pinned. Positive A proofs remain partial, not full ability parity.

## Scope and classification

13 classes / 40 specs, including Devourer1480. **Book:** 630 distinct visible class/spec catalog IDs, 1551 spec–spell memberships. **Core probes:** 302 distinct IDs / 355 memberships, enumerated below; supplement the book with selected talented rotation/healing/tank buttons and defensives. These are a bounded key-spell set, not every possible talent build.
Book membership exactly reproduces `godot/ui-model/tests/classbook_catalog_audit.rs::emit_classbook_catalog_coverage`: client class SkillLineAbility membership plus selected SpecializationSpells/mastery, `spellbook.lists(true)`; class/spec overlap deduplicated per spec. No acquire-method/race/level gate: membership is NOT a learned level80 character. Counts independently match all13 class/all40 spec entries in the existing classbook coverage.json.
**A:** identified enabled server test asserts at least one real spell-specific behaviour. Partial proof: remaining unimplemented types still listed and ranked. **B:** all inspected effects/auras dispatch to production handlers; no positive spell-specific test identified. Handler coverage is NOT proof of full rotation/talent semantics. **C:** structurally admissible cast with missing handler/script. **D:** deterministic cast gate (unknown store row, unsupported selecting target/power or explicit pet restriction). Priority D > A > C > B; A does not erase gaps.
C/B admission is inferred from source under granted-known-spell, valid target/state/resources/cooldowns and required assets, not demonstrated through new network casts. Passive entries are included as catalog entries; their ordinary execution is KnownSpells passive sync, not an actionable button. Server validate_start has no general passive rejection. B also includes dispatch-only/metadata cases; it does not promise a nonzero effect.
E<number> = SpellEffect; a<number> = AuraType. Gaps include DifficultyID0 descendants named by EffectTriggerSpell through3 hops plus explicit Chaos Strike/Living Flame/Brew/Frostbolt scripts; these are data dependencies, not a claim every branch executes in every build. Unimplemented paths do not necessarily prevent damage from another effect. NONE0 is excluded. Children of skipped effects remain latent requirements. No proc-chance, mastery formula, school-mask/zero-amount, empower-release, AI, group-target or talent-selection parity certification.
D uses handled/scripted selecting effects only and explicit spell-owned target plans, matching validate_start; unsupported targets on skipped effects do not reject the root. Descendant gate failures are not root D and remain unproved. Full talent-tree behaviour is NOT audited; rendered talent graphs remain presence/pixel evidence only. Prior coverage supplies numeric enum names only, not scope/counts.

## Handler citations (game-server/crates/server/src)

`spell_cast/effects.rs::handler/cast_handler/is_handled` dispatch damage2, heal10, resurrect18/172, energize30, weapon31/58/121, trigger32/64, dispel38, interrupt68, heal-percent136, force140, remove203, teleport252 and charge293; profession24/36/44, ritual50, summon-player85, pet55/56/102. B is conditional on handler-specific data, not just registration.
`hunter_pet.rs::{check_tame,has_aura_type}` consumes a146 (Exotic Beasts) outside the aura registry; it is not counted missing. `auras/handlers.rs::kind` -> `periodic.rs`, `stat_mods.rs`, `spell_mods.rs`, `procs.rs`, `unit_state.rs`; `auras/mod.rs::{absorb_damage,immune_to_school,damage_taken_multiplier,charge_mods,sync_passives}`. Only aura-effect wrappers6/27/35/65 are supported (`AURA_EFFECTS`); other DB2 aura wrappers are missing EFFECT handlers.
`spell_cast/scripts.rs::{on_effect_hit,area_target_filter}` implements115203→120954,344862→162794,361469→361500/361509 and362969 target limit; Frostbolt chilled205708 is a separate script path. `auras/scripts.rs::AuraScripts` registers Turtle186265 and Barkskin22812 (Brambles periodic) in production; other Script registrations still need per-spell behaviour: a4/a226 registration as Script alone is NOT behaviour. Energizing Flame400006 is explicitly consumed in the Living Flame script; other dummy markers may feed costs/modifiers, but their independent scripted mechanics are not established.
`spell_cast/mod.rs::{selects_targets,validate_start,spell_costs}`; `spell_cast/targets.rs::{selector,unsupported_target}`; `hunter_pet.rs::check_summon_pet` rejects non-Imp creature summons. `meeting_stone/ritual.rs::effect_trans_door` handles only catalog-backed rituals; other object50 uses are C, not falsely B.

## Per-spec totals

| Class / spec (ID) | Book A/B/C/D | Core A/B/C/D |
|---|---:|---:|
| Warrior / Arms (71) | 5/15/9/0 | 7/2/1/0 |
| Warrior / Fury (72) | 6/16/11/0 | 6/3/1/0 |
| Warrior / Protection (73) | 6/16/10/0 | 5/3/2/0 |
| Paladin / Holy (65) | 11/13/33/1 | 7/1/1/0 |
| Paladin / Protection (66) | 9/11/33/0 | 6/3/1/0 |
| Paladin / Retribution (70) | 9/12/33/0 | 5/1/3/0 |
| Hunter / Beast Mastery (253) | 4/16/24/0 | 6/1/2/0 |
| Hunter / Marksmanship (254) | 4/17/26/0 | 4/1/3/0 |
| Hunter / Survival (255) | 4/17/25/0 | 4/3/1/0 |
| Rogue / Assassination (259) | 9/15/18/0 | 8/1/0/0 |
| Rogue / Outlaw (260) | 7/15/22/0 | 5/2/3/0 |
| Rogue / Subtlety (261) | 7/15/22/0 | 7/1/2/0 |
| Priest / Discipline (256) | 6/5/12/1 | 6/4/0/0 |
| Priest / Holy (257) | 5/4/15/1 | 6/1/2/0 |
| Priest / Shadow (258) | 8/5/13/0 | 9/0/0/0 |
| Death Knight / Blood (250) | 2/4/18/0 | 5/2/2/0 |
| Death Knight / Frost (251) | 2/6/18/0 | 5/1/2/0 |
| Death Knight / Unholy (252) | 2/4/19/0 | 4/1/4/0 |
| Shaman / Elemental (262) | 2/19/17/0 | 6/2/1/0 |
| Shaman / Enhancement (263) | 3/19/15/0 | 6/1/2/0 |
| Shaman / Restoration (264) | 3/20/17/1 | 5/2/2/0 |
| Mage / Arcane (62) | 6/25/67/0 | 5/2/1/0 |
| Mage / Fire (63) | 5/25/67/0 | 5/2/1/0 |
| Mage / Frost (64) | 4/26/67/0 | 5/2/2/0 |
| Warlock / Affliction (265) | 5/7/17/3 | 5/0/4/0 |
| Warlock / Demonology (266) | 5/6/18/3 | 5/0/3/0 |
| Warlock / Destruction (267) | 6/6/19/3 | 7/0/2/0 |
| Monk / Brewmaster (268) | 4/3/16/0 | 6/1/2/0 |
| Monk / Windwalker (269) | 5/3/14/0 | 5/1/3/0 |
| Monk / Mistweaver (270) | 5/5/15/1 | 4/2/3/0 |
| Druid / Balance (102) | 8/10/21/0 | 5/3/0/0 |
| Druid / Feral (103) | 7/7/24/0 | 6/1/1/0 |
| Druid / Guardian (104) | 7/7/23/0 | 4/2/3/0 |
| Druid / Restoration (105) | 7/8/23/1 | 5/4/0/0 |
| Demon Hunter / Havoc (577) | 7/11/12/0 | 5/1/2/0 |
| Demon Hunter / Vengeance (581) | 7/9/16/0 | 4/2/3/0 |
| Demon Hunter / Devourer (1480) | 8/9/13/0 | 4/0/2/0 |
| Evoker / Devastation (1467) | 5/3/8/0 | 5/1/3/0 |
| Evoker / Preservation (1468) | 5/4/8/1 | 5/1/3/0 |
| Evoker / Augmentation (1473) | 5/3/11/0 | 4/0/5/0 |

## Exact catalog classification and named core probes

Each book ID appears once per spec below. Core probes may be talented IDs absent from the book; A test codes resolve in the final section. All B entries cite the shared handler citations above.

### Warrior / Arms (71)
Book: A: 100,1464,1715,2565,163201; B: 355,1680,6552,18499,34428,57755,117313,118779,203526,205545,205546,205547,209577,215537,1258398; C: 6673,23922,156287,203524,260708,279423,294184,294189,1229376; D: —.
Core: 12294 Mortal Strike — A[T54]; 7384 Overpower — B [handlers: E2]; 163201 Execute — A[T52]; 772 Rend — A[T56] (a308); 167105 Colossus Smash — A[T150] (a4,a343,E3); 227847 Bladestorm — C (a147,a263); 1464 Slam — A[T191]; 100 Charge — A[T16] (E3); 6552 Pummel — B [handlers: E68]; 871 Shield Wall — A[T12].
Other A: 1715 Hamstring A[T18]; 2565 Shield Block A[T177]

### Warrior / Fury (72)
Book: A: 100,1464,1715,2565,163201,190411; B: 355,1680,5308,6552,18499,34428,57755,76856,117313,118779,203526,205545,205546,205547,209577,215537; C: 6673,23922,46917,81099,156287,184361,203524,231842,294184,294189,1229376; D: —.
Core: 23881 Bloodthirst — A[T175] (a443,a458); 85288 Raging Blow — A[T152] (a4); 184367 Rampage — A[T176] (a443,a458,E3); 5308 Execute — B [handlers: E2,E30,E64]; 184364 Enraged Regeneration — B [handlers: E6,a107,a87]; 1719 Recklessness — C (E3); 190411 Whirlwind — A[T155] (a4,E3); 100 Charge — A[T16] (E3); 6552 Pummel — B [handlers: E68]; 871 Shield Wall — A[T12].
Other A: 1464 Slam A[T191]; 1715 Hamstring A[T18]; 2565 Shield Block A[T177]; 163201 Execute A[T52]

### Warrior / Protection (73)
Book: A: 100,1464,1715,2565,20243,163201; B: 71,355,1161,1680,6552,18499,34428,57755,117313,118779,203526,205545,205546,205547,209577,215537; C: 6673,23922,76857,156287,161798,203524,294184,294189,397708,1229376; D: —.
Core: 23922 Shield Slam — C (a4); 20243 Devastate — A[T118] (E3); 6572 Revenge — B [handlers: E2]; 2565 Shield Block — A[T177]; 190456 Ignore Pain — A[T90] (a142); 6343 Thunder Clap — A[T124] (E3); 1160 Demoralizing Shout — C (a269,a343); 871 Shield Wall — A[T12]; 355 Taunt — B [handlers: E114,E6,a11,a31]; 6552 Pummel — B [handlers: E68].
Other A: 100 Charge A[T16] (E3); 1464 Slam A[T191]; 1715 Hamstring A[T18]; 163201 Execute A[T52]

### Paladin / Holy (65)
Book: A: 642,853,6940,19750,20271,26573,35395,53563,53600,82326,275773; B: 498,1044,4987,7328,24275,62124,66235,81297,85673,86704,114919,162056,200654; C: 465,633,1022,5502,10326,13819,23214,31884,32223,34767,34769,69820,69826,73629,73630,114852,114871,121183,183435,183997,190784,200652,205273,209202,270562,270564,290608,363613,391054,415091,440013,453785,1229376; D: 212056.
Core: 20473 Holy Shock — A[T78]; 82326 Holy Light — A[T77]; 19750 Flash of Light — A[T22]; 85222 Light of Dawn — A[T129] (E3); 85673 Word of Glory — B [handlers: E10]; 53563 Beacon of Light — A[T64] (a226); 275773 Judgment — A[T76]; 31884 Avenging Wrath — C (a4,a262,a334); 642 Divine Shield — A[T7] (a485).
Other A: 853 Hammer of Justice A[T24] (a4); 6940 Blessing of Sacrifice A[T178] (a41); 20271 Judgment A[T79]; 26573 Consecration A[T29]; 35395 Crusader Strike A[T74]; 53600 Shield of the Righteous A[T48] (a4) Other D: 212056 Absolution (target 119)

### Paladin / Protection (66)
Book: A: 642,853,6940,19750,20271,26573,35395,53600,275779; B: 1044,7328,24275,62124,66235,81297,85673,86704,114919,162056,200654; C: 465,633,1022,5502,10326,13819,23214,31884,32223,34767,34769,69820,69826,73629,73630,76671,114852,114871,121183,161800,183435,190784,200652,205273,209202,270562,270564,290608,353367,363613,391054,453785,1229376; D: —.
Core: 31935 Avenger's Shield — B [handlers: E2,E30,E6,E68,a27]; 275779 Judgment — A[T33]; 53600 Shield of the Righteous — A[T48] (a4); 53595 Hammer of the Righteous — A[T35] (a269,E3); 26573 Consecration — A[T29]; 31850 Ardent Defender — A[T96] (a316); 86659 Guardian of Ancient Kings — C (a4); 642 Divine Shield — A[T7] (a485); 62124 Hand of Reckoning — B [handlers: E114,E6,a11]; 96231 Rebuke — B [handlers: E68].
Other A: 853 Hammer of Justice A[T24] (a4); 6940 Blessing of Sacrifice A[T178] (a41); 19750 Flash of Light A[T22]; 20271 Judgment A[T79]; 35395 Crusader Strike A[T74]

### Paladin / Retribution (70)
Book: A: 642,853,6940,19750,20271,26573,35395,53600,85256; B: 1044,7328,24275,62124,66235,81297,85673,86704,114919,162056,200654,403876; C: 465,633,1022,5502,10326,13819,23214,31884,32223,34767,34769,69820,69826,73629,73630,114852,114871,121183,183435,190784,200652,205273,209202,267316,270562,270564,290608,363613,391054,404752,404834,453785,1229376; D: —.
Core: 35395 Crusader Strike — A[T74]; 20271 Judgment — A[T79]; 184575 Blade of Justice — A[T34]; 85256 Templar's Verdict — A[T148] (E3); 53385 Divine Storm — C (E3); 255937 Wake of Ashes — C (a345,a537); 31884 Avenging Wrath — C (a4,a262,a334); 642 Divine Shield — A[T7] (a485); 96231 Rebuke — B [handlers: E68].
Other A: 853 Hammer of Justice A[T24] (a4); 6940 Blessing of Sacrifice A[T178] (a41); 19750 Flash of Light A[T22]; 26573 Consecration A[T29]; 53600 Shield of the Righteous A[T48] (a4)

### Hunter / Beast Mastery (253)
Book: A: 883,2641,56641,186265; B: 136,1543,53270,83242,83243,83244,83245,109304,162487,185358,187650,195645,204147,207097,355589,355596; C: 781,982,1462,1515,5384,6991,61648,76657,125050,127933,175686,186257,203415,207068,209997,210000,257284,259398,272651,321297,325028,325037,344572,1229376; D: —.
Core: 34026 Kill Command — C (E3); 217200 Barbed Shot — A[T103]; 193455 Cobra Shot — A[T113] (E3); 2643 Multi-Shot — C (E3); 19574 Bestial Wrath — A[T51] (a226,E3); 193530 Aspect of the Wild — A[T50] (E3); 883 Call Pet 1 — A[T26]; 186265 Aspect of the Turtle — A[T4]; 147362 Counter Shot — B [handlers: E68].
Other A: 2641 Dismiss Pet A[T26] (missing child 47531); 56641 Steady Shot A[T132] (E3)

### Hunter / Marksmanship (254)
Book: A: 883,2641,56641,186265; B: 136,1543,83242,83243,83244,83245,109304,162487,185358,187650,193468,195645,204147,207097,355589,355596,466904; C: 781,982,1462,1515,5384,6197,6991,61648,125050,127933,175686,186257,203415,207068,209997,210000,257284,257620,259398,272651,325028,325037,344572,1217788,1219616,1229376; D: —.
Core: 19434 Aimed Shot — C (E3); 257044 Rapid Fire — A[T55] (a226,E3); 56641 Steady Shot — A[T132] (E3); 257620 Multi-Shot — C (E3); 53351 Kill Shot — A[T128]; 288613 Trueshot — C (a4,a148,a286); 186265 Aspect of the Turtle — A[T4]; 147362 Counter Shot — B [handlers: E68].
Other A: 883 Call Pet 1 A[T26]; 2641 Dismiss Pet A[T26] (missing child 47531)

### Hunter / Survival (255)
Book: A: 883,2641,56641,186265; B: 136,1543,83242,83243,83244,83245,109304,162487,185358,187650,190925,193265,195645,204147,207097,355589,355596; C: 781,982,1462,1515,5384,6991,61648,125050,127933,175686,186257,186289,203415,207068,209997,210000,257284,259398,263135,272651,321297,325028,325037,344572,1229376; D: —.
Core: 259489 Kill Command — A[T62]; 186270 Raptor Strike — B [handlers: E2]; 259495 Wildfire Bomb — A[T63] (E3); 212436 Butchery — A[T43] (E3); 190925 Harpoon — B [handlers: E32,E41,E6,a455]; 186289 Aspect of the Eagle — C (E3); 186265 Aspect of the Turtle — A[T4]; 147362 Counter Shot — B [handlers: E68].
Other A: 883 Call Pet 1 A[T26]; 2641 Dismiss Pet A[T26] (missing child 47531); 56641 Steady Shot A[T132] (E3)

### Rogue / Assassination (259)
Book: A: 703,1329,1766,1943,2983,5277,32645,51723,196819; B: 1752,1833,2094,2818,3408,6770,8676,22482,31209,31224,76803,113780,185311,185565,212205; C: 408,921,1725,1784,1804,1856,1860,1966,5761,5938,8679,51667,56814,57934,114018,315496,315584,1229376; D: —.
Core: 1329 Mutilate — A[T101] (E3); 703 Garrote — A[T119] (a4); 1943 Rupture — A[T183] (a226,E3); 32645 Envenom — A[T182] (a4); 51723 Fan of Knives — A[T159] (E3); 185565 Poisoned Knife — B [handlers: E2,E30]; 360194 Deathmark — A[T59]; 1766 Kick — A[T193]; 5277 Evasion — A[T9].
Other A: 2983 Sprint A[T18] (a226); 196819 Eviscerate A[T44]

### Rogue / Outlaw (260)
Book: A: 1766,2098,2983,5277,13877,193315,196819; B: 1752,1833,2094,2818,3408,6770,8676,22482,31209,31224,113780,185311,185763,212205,315341; C: 408,921,1725,1784,1804,1856,1860,1966,5761,5938,8679,51667,56814,57934,76806,79096,114018,195457,315496,315584,1214909,1229376; D: —.
Core: 193315 Sinister Strike — A[T39]; 185763 Pistol Shot — B [handlers: E2,E30,E6,a33]; 2098 Dispatch — A[T44]; 315341 Between the Eyes — B [handlers: E2,E6,a108,a218]; 13877 Blade Flurry — A[T28] (a4,missing child 385835,E3); 13750 Adrenaline Rush — C (a226); 1214909 Roll the Bones — C (a4); 195457 Grappling Hook — C (E3); 1766 Kick — A[T193]; 5277 Evasion — A[T9].
Other A: 2983 Sprint A[T18] (a226); 196819 Eviscerate A[T44]

### Rogue / Subtlety (261)
Book: A: 53,1766,2983,5277,196819,197835,319175; B: 1752,1833,2094,2818,3408,6770,8676,22482,31209,31224,76808,113780,185311,185438,212205; C: 408,921,1725,1784,1804,1856,1860,1966,5761,5938,8679,51667,56814,57934,114014,114018,185313,196912,280719,315496,315584,1229376; D: —.
Core: 53 Backstab — A[T116] (E3); 185438 Shadowstrike — B [handlers: E2,E30]; 196819 Eviscerate — A[T44]; 1943 Rupture — A[T183] (a226,E3); 319175 Black Powder — A[T180] (E3); 197835 Shuriken Storm — A[T142] (E3); 185313 Shadow Dance — C (a103,a275); 121471 Shadow Blades — C (a4); 1766 Kick — A[T193]; 5277 Evasion — A[T9].
Other A: 2983 Sprint A[T18] (a226)

### Priest / Discipline (256)
Book: A: 17,527,589,2061,21562,47540; B: 585,2006,77489,200829,207948; C: 453,1706,2096,45243,161862,205065,207946,208065,215769,271534,440006,1229376; D: 212036.
Core: 589 Shadow Word: Pain — A[T21] (a269); 585 Smite — B [handlers: E2]; 47540 Penance — A[T174] (a4); 8092 Mind Blast — A[T17] (E3); 17 Power Word: Shield — A[T8]; 194509 Power Word: Radiance — A[T168] (a4,E3); 2061 Flash Heal — A[T194]; 200829 Plea — B [handlers: E10]; 62618 Power Word: Barrier — B [handlers: E179]; 33206 Pain Suppression — B [handlers: E6,a87].
Other A: 527 Purify A[T19]; 21562 Power Word: Fortitude A[T23] (a4) Other D: 212036 Mass Resurrection (target 119)

### Priest / Holy (257)
Book: A: 17,527,589,2061,21562; B: 585,2006,77489,207948; C: 453,1706,2096,20711,33076,45243,63733,77485,161862,205065,207946,208065,215769,440006,1229376; D: 212036.
Core: 2061 Flash Heal — A[T194]; 2060 Heal — B [handlers: E10]; 139 Renew — A[T20]; 33076 Prayer of Mending — C (E3); 2050 Holy Word: Serenity — A[T123]; 34861 Holy Word: Sanctify — A[T163] (E3); 596 Prayer of Healing — A[T121] (E3); 47788 Guardian Spirit — C (a316,E3); 527 Purify — A[T19].
Other A: 17 Power Word: Shield A[T8]; 589 Shadow Word: Pain A[T21] (a269); 21562 Power Word: Fortitude A[T23] (a4) Other D: 212036 Mass Resurrection (target 119)

### Priest / Shadow (258)
Book: A: 17,589,2061,15407,15487,21562,34914,47585; B: 585,2006,77489,207948,232698; C: 453,1706,2096,15286,45243,161862,205065,207946,208065,215769,280752,343690,1229376; D: —.
Core: 589 Shadow Word: Pain — A[T21] (a269); 34914 Vampiric Touch — A[T102] (a4); 8092 Mind Blast — A[T17] (E3); 15407 Mind Flay — A[T106] (E3); 32379 Shadow Word: Death — A[T57]; 335467 Shadow Word: Madness — A[T92] (a4); 228260 Voidform — A[T42] (a4,a75,a226,a235,E3); 47585 Dispersion — A[T97] (a4); 15487 Silence — A[T24].
Other A: 17 Power Word: Shield A[T8]; 2061 Flash Heal A[T194]; 21562 Power Word: Fortitude A[T23] (a4)

### Death Knight / Blood (250)
Book: A: 47541,195292; B: 56222,190778,190780,316239; C: 3714,43265,45524,48265,48707,49039,49576,50977,51986,61999,77513,81136,127344,161797,205223,220143,374721,1229376; D: —.
Core: 49998 Death Strike — C (E3); 206930 Heart Strike — A[T151] (E3); 50842 Blood Boil — A[T81] (a4,a380,a381); 195292 Death's Caress — A[T136] (a4,a380,a381,a408,E3); 49028 Dancing Rune Weapon — C (a4,missing child 1206,E3); 55233 Vampiric Blood — A[T71] (a422,a443); 48792 Icebound Fortitude — A[T1] (a4); 56222 Dark Command — B [handlers: E114,E6,a11]; 47528 Mind Freeze — B [handlers: E68].
Other A: 47541 Death Coil A[T111] (E3)

### Death Knight / Frost (251)
Book: A: 47541,196770; B: 56222,77514,81333,190778,190780,316239; C: 3714,43265,45524,48265,48707,49039,49576,50977,51986,59057,61999,81229,127344,178819,194913,205223,220143,1229376; D: —.
Core: 49020 Obliterate — A[T93] (E3); 49143 Frost Strike — A[T66] (E3); 49184 Howling Blast — A[T127] (a380,a381,E3); 194913 Glacial Advance — C (E3); 196770 Remorseless Winter — A[T169] (a395,E3); 51271 Pillar of Frost — C (a4); 48792 Icebound Fortitude — A[T1] (a4); 47528 Mind Freeze — B [handlers: E68].
Other A: 47541 Death Coil A[T111] (E3)

### Death Knight / Unholy (252)
Book: A: 47541,207317; B: 56222,190778,190780,316239; C: 3714,43265,45524,46584,48265,48707,49039,49576,50977,51462,51986,61999,77515,85948,127344,178819,205223,220143,1229376; D: —.
Core: 85948 Festering Strike — C (E3); 55090 Scourge Strike — A[T170] (a380,a381,E3); 47541 Death Coil — A[T111] (E3); 207317 Epidemic — A[T139]; 63560 Dark Transformation — C (a4); 275699 Apocalypse — C (E3); 46584 Raise Dead — C (a4); 48792 Icebound Fortitude — A[T1] (a4); 47528 Mind Freeze — B [handlers: E68].
No additional A/D catalog entries.

### Shaman / Elemental (262)
Book: A: 2825,188196; B: 556,2008,8004,25504,32182,33750,73899,114911,210873,211004,211010,211015,269352,277778,277784,309328,320747,327163,328928; C: 546,2484,2645,6196,20608,51490,77756,115356,168534,192106,205495,207778,328930,378776,462854,470411,1229376; D: —.
Core: 188196 Lightning Bolt — A[T32]; 51505 Lava Burst — A[T140]; 188389 Flame Shock — C (E3); 8042 Earth Shock — B [handlers: E2]; 61882 Earthquake — A[T164] (a226,E3,E77); 188443 Chain Lightning — A[T125] (E3); 191634 Stormkeeper — A[T104]; 108271 Astral Shift — A[T10]; 57994 Wind Shear — B [handlers: E68].
Other A: 2825 Bloodlust A[T23]

### Shaman / Enhancement (263)
Book: A: 2825,188196,196884; B: 556,2008,8004,25504,32182,33750,73899,114911,210873,211004,211010,211015,269352,277778,277784,309328,320747,327163,328928; C: 546,2484,2645,6196,17364,20608,77223,115356,192106,205495,207778,328930,462854,470411,1229376; D: —.
Core: 17364 Stormstrike — C (a4,E3); 60103 Lava Lash — A[T36] (E3); 187874 Crash Lightning — A[T157] (a4,E3); 188196 Lightning Bolt — A[T32]; 188389 Flame Shock — C (E3); 51533 Feral Spirit — A[T45] (E3,E42,E63); 196884 Feral Lunge — A[T30]; 108271 Astral Shift — A[T10]; 57994 Wind Shear — B [handlers: E68].
Other A: 2825 Bloodlust A[T23]

### Shaman / Restoration (264)
Book: A: 2825,77472,188196; B: 556,2008,8004,25504,32182,33750,73899,77130,114911,210873,211004,211010,211015,269352,277778,277784,309328,320747,327163,328928; C: 546,2484,2645,6196,20608,52127,77226,77756,115356,192106,205495,207778,328930,440012,462854,470411,1229376; D: 212048.
Core: 77472 Healing Wave — A[T75]; 8004 Healing Surge — B [handlers: E10]; 61295 Riptide — A[T80]; 1064 Chain Heal — A[T67]; 73920 Healing Rain — A[T167] (a4,a226); 98008 Spirit Link Totem — C (E3); 108280 Healing Tide Totem — C (a4,a226); 77130 Purify Spirit — B [handlers: E38]; 108271 Astral Shift — A[T10].
Other A: 2825 Bloodlust A[T23]; 188196 Lightning Bolt A[T32] Other D: 212048 Ancestral Vision (target 119)

### Mage / Arcane (62)
Book: A: 116,118,122,1459,30451,44425; B: 120,130,1449,1953,2139,28271,28272,61025,61305,61721,61780,126819,161353,161354,161355,161372,210726,211088,214634,277787,277792,319836,321395,391622,460392; C: 66,3561,3562,3563,3565,3566,3567,10059,11416,11417,11418,11419,11420,32266,32267,32271,32272,33690,33691,35715,35717,49358,49359,49360,49361,53140,53142,56384,79684,80353,88342,88344,88345,88346,120145,120146,131784,132620,132621,132626,132627,176242,176244,176246,176248,190336,190740,193759,194466,210086,211076,224869,224871,224968,281400,281402,281403,281404,344587,344597,395277,395289,446534,446540,1229376,1259190,1259194; D: —.
Core: 30451 Arcane Blast — A[T72]; 44425 Arcane Barrage — A[T179] (E3); 5143 Arcane Missiles — C (a226); 153626 Arcane Orb — B [handlers: E179,E30]; 365350 Arcane Surge — A[T184] (E3,E137); 12042 Arcane Power — A[T73]; 2139 Counterspell — B [handlers: E68]; 45438 Ice Block — A[T5].
Other A: 116 Frostbolt A[T14]; 118 Polymorph A[T15]; 122 Frost Nova A[T18]; 1459 Arcane Intellect A[T13] (a4)

### Mage / Fire (63)
Book: A: 116,118,122,133,1459; B: 120,130,1449,1953,2139,28271,28272,61025,61305,61721,61780,126819,161353,161354,161355,161372,210726,211088,214634,277787,277792,319836,321395,391622,460392; C: 66,3561,3562,3563,3565,3566,3567,10059,11416,11417,11418,11419,11420,12846,32266,32267,32271,32272,33690,33691,35715,35717,49358,49359,49360,49361,53140,53142,56384,80353,88342,88344,88345,88346,120145,120146,131784,132620,132621,132626,132627,176242,176244,176246,176248,190336,193759,194466,195283,210086,211076,224869,224871,224968,281400,281402,281403,281404,344587,344597,395277,395289,446534,446540,1229376,1259190,1259194; D: —.
Core: 133 Fireball — A[T192]; 108853 Fire Blast — A[T114]; 11366 Pyroblast — B [handlers: E2]; 2948 Scorch — A[T122]; 190319 Combustion — A[T65] (a189,a226); 2120 Flamestrike — C (E3); 2139 Counterspell — B [handlers: E68]; 45438 Ice Block — A[T5].
Other A: 116 Frostbolt A[T14]; 118 Polymorph A[T15]; 122 Frost Nova A[T18]; 1459 Arcane Intellect A[T13] (a4)

### Mage / Frost (64)
Book: A: 116,118,122,1459; B: 120,130,1449,1953,2139,28271,28272,61025,61305,61721,61780,126819,161353,161354,161355,161372,210726,211088,214634,277787,277792,319836,321395,391622,460392,1246752; C: 66,3561,3562,3563,3565,3566,3567,10059,11416,11417,11418,11419,11420,32266,32267,32271,32272,33690,33691,35715,35717,49358,49359,49360,49361,53140,53142,56384,80353,88342,88344,88345,88346,120145,120146,131784,132620,132621,132626,132627,176242,176244,176246,176248,190336,193759,194466,210086,211076,224869,224871,224968,281400,281402,281403,281404,344587,344597,395277,395289,446534,446540,1229376,1246769,1247775,1259190,1259194; D: —.
Core: 116 Frostbolt — A[T14]; 30455 Ice Lance — A[T143]; 44614 Flurry — A[T98]; 84714 Frozen Orb — C (E3); 190356 Blizzard — C (E3); 12472 Icy Veins — B [handlers: E6,a108,a193]; 122 Frost Nova — A[T18]; 2139 Counterspell — B [handlers: E68]; 45438 Ice Block — A[T5].
Other A: 118 Polymorph A[T15]; 1459 Arcane Intellect A[T13] (a4)

### Warlock / Affliction (265)
Book: A: 686,688,5697,104773,234153; B: 702,5784,77215,108366,187394,211715,215279; C: 126,172,698,1098,5782,6201,20707,23161,29893,101508,119898,196586,211714,216698,246985,342601,1229376; D: 691,697,366222.
Core: 172 Corruption — C (E3); 980 Agony — C (a226); 30108 Unstable Affliction — A[T49] (E3); 324536 Malefic Rapture — A[T141]; 198590 Drain Soul — A[T117]; 27243 Seed of Corruption — C (a4,E3); 205180 Summon Darkglare — C (a4,E3); 104773 Unending Resolve — A[T11] (a285,E129); 688 Summon Imp — A[T195].
Other A: 686 Shadow Bolt A[T86]; 5697 Unending Breath A[T25] (a58); 234153 Drain Life A[T89] Other D: 691 Summon Felhunter (SUMMON_PET56: only Summon Imp supported); 697 Summon Voidwalker (SUMMON_PET56: only Summon Imp supported); 366222 Summon Sayaad (SUMMON_PET56: only Summon Imp supported)

### Warlock / Demonology (266)
Book: A: 686,688,5697,104773,234153; B: 702,5784,108366,187394,211715,215279; C: 126,172,698,1098,5782,6201,20707,23161,29893,77219,101508,119898,196586,211714,216698,246985,342601,1229376; D: 691,697,366222.
Core: 686 Shadow Bolt — A[T86]; 105174 Hand of Gul'dan — A[T31] (E3); 104316 Call Dreadstalkers — C (E3); 264178 Demonbolt — A[T158] (a4,E3); 265187 Summon Demonic Tyrant — C (a4); 196277 Implosion — C (E3); 104773 Unending Resolve — A[T11] (a285,E129); 688 Summon Imp — A[T195].
Other A: 5697 Unending Breath A[T25] (a58); 234153 Drain Life A[T89] Other D: 691 Summon Felhunter (SUMMON_PET56: only Summon Imp supported); 697 Summon Voidwalker (SUMMON_PET56: only Summon Imp supported); 366222 Summon Sayaad (SUMMON_PET56: only Summon Imp supported)

### Warlock / Destruction (267)
Book: A: 686,688,5697,29722,104773,234153; B: 702,5784,108366,187394,211715,215279; C: 126,172,348,698,1098,5782,6201,20707,23161,29893,77220,101508,119898,196586,211714,216698,246985,342601,1229376; D: 691,697,366222.
Core: 348 Immolate — C (E3); 29722 Incinerate — A[T160] (E3); 17962 Conflagrate — A[T91]; 116858 Chaos Bolt — A[T134] (E3,E137); 5740 Rain of Fire — A[T37] (a4); 80240 Havoc — A[T99]; 1122 Summon Infernal — C (a4,E148); 104773 Unending Resolve — A[T11] (a285,E129); 688 Summon Imp — A[T195].
Other A: 686 Shadow Bolt A[T86]; 5697 Unending Breath A[T25] (a58); 234153 Drain Life A[T89] Other D: 691 Summon Felhunter (SUMMON_PET56: only Summon Imp supported); 697 Summon Voidwalker (SUMMON_PET56: only Summon Imp supported); 366222 Summon Sayaad (SUMMON_PET56: only Summon Imp supported)

### Monk / Brewmaster (268)
Book: A: 100780,100784,115546,116670; B: 115178,119381,205406; C: 8647,101546,109132,115069,117906,117952,125883,126892,169340,209525,216519,245013,322101,322109,322729,1229376; D: —.
Core: 100780 Tiger Palm — A[T190] (E3); 100784 Blackout Kick — A[T115] (E3); 121253 Keg Smash — A[T144] (a4,E3); 115181 Breath of Fire — A[T156]; 322729 Spinning Crane Kick — C (a4); 119582 Purifying Brew — C (E3); 322507 Celestial Brew — B [handlers: E6,a283,a69]; 115203 Fortifying Brew — A[T6] (a4,a255); 115546 Provoke — A[T161].
Other A: 116670 Vivify A[T133]

### Monk / Windwalker (269)
Book: A: 100780,100784,115546,116670,122470; B: 115178,119381,205406; C: 8647,101545,101546,109132,115636,116092,117952,125883,126892,169340,209525,322101,322109,1229376; D: —.
Core: 100780 Tiger Palm — A[T190] (E3); 100784 Blackout Kick — A[T115] (E3); 107428 Rising Sun Kick — A[T69]; 113656 Fists of Fury — C (a4,a226,E3); 101546 Spinning Crane Kick — C (a4); 101545 Flying Serpent Kick — C (a4,a104,a305,a373); 122470 Touch of Karma — A[T68] (a4,E3); 115203 Fortifying Brew — A[T6] (a4,a255); 116705 Spear Hand Strike — B [handlers: E68].
Other A: 115546 Provoke A[T161]; 116670 Vivify A[T133]

### Monk / Mistweaver (270)
Book: A: 100780,100784,115151,115546,116670; B: 115178,115450,119381,205406,1285660; C: 8647,101546,109132,116645,117907,117952,125883,126892,169340,202577,209525,322101,322109,388023,1229376; D: 212051.
Core: 115151 Renewing Mist — A[T131] (E3); 116670 Vivify — A[T133]; 124682 Enveloping Mist — B [handlers: E10,E6,a283,a8]; 325197 Invoke Chi-Ji, the Red Crane — C (a4); 115175 Soothing Mist — C (a330); 107428 Rising Sun Kick — A[T69]; 322101 Expel Harm — C (E3); 115450 Detox — B [handlers: E38]; 115203 Fortifying Brew — A[T6] (a4,a255).
Other A: 100780 Tiger Palm A[T190] (E3); 100784 Blackout Kick A[T115] (E3); 115546 Provoke A[T161] Other D: 212051 Reawaken (target 119)

### Druid / Balance (102)
Book: A: 768,5221,5487,8921,8936,22812,33917,191034; B: 339,1850,5176,6795,50769,77758,190984,210723,231042,393014; C: 783,1126,5215,18960,20484,22568,114282,127757,164812,164815,164862,165962,193753,208253,210053,210722,236716,276012,276029,1229376,1297521; D: —.
Core: 190984 Wrath — B [handlers: E2,E30]; 194153 Starfire — A[T146] (E3,E77); 78674 Starsurge — B [handlers: E2]; 191034 Starfall — A[T145] (a307,a330,a395,E3); 8921 Moonfire — A[T120] (a269,a537); 93402 Sunfire — A[T147] (a537,E3); 194223 Celestial Alignment — B [handlers: E6,a193]; 22812 Barkskin — A[T173].
Other A: 768 Cat Form A[T27] (a4,a153,a226,a275,E3); 5221 Shred A[T38]; 5487 Bear Form A[T27] (a4,a142,a187,a240,a275); 8936 Regrowth A[T83]; 33917 Mangle A[T130] (E3)

### Druid / Feral (103)
Book: A: 768,5221,5487,8921,8936,22812,33917; B: 339,1850,5176,6795,50769,77758,210723; C: 783,1126,5215,16949,16974,18960,20484,22568,77493,114282,127757,164812,164815,164862,165962,193753,208253,210053,210722,236716,276012,276029,300349,1229376; D: —.
Core: 5221 Shred — A[T38]; 1822 Rake — A[T47]; 1079 Rip — A[T181] (a4,E3); 22568 Ferocious Bite — C (E62); 106830 Thrash — A[T40]; 106951 Berserk — A[T105]; 5217 Tiger's Fury — B [handlers: E30,E6,a108,a344]; 22812 Barkskin — A[T173].
Other A: 768 Cat Form A[T27] (a4,a153,a226,a275,E3); 5487 Bear Form A[T27] (a4,a142,a187,a240,a275); 8921 Moonfire A[T120] (a269,a537); 8936 Regrowth A[T83]; 33917 Mangle A[T130] (E3)

### Druid / Guardian (104)
Book: A: 768,5221,5487,8921,8936,22812,33917; B: 339,1850,5176,6795,50769,77758,210723; C: 783,1126,5215,18960,20484,22568,114282,127757,155783,164812,164815,164862,165962,193753,208253,210053,210722,231065,236716,276012,276029,300346,1229376; D: —.
Core: 33917 Mangle — A[T130] (E3); 77758 Thrash — B [handlers: E2,E30]; 6807 Maul — C (E3); 192081 Ironfur — C (a285); 22842 Frenzied Regeneration — C (a145); 61336 Survival Instincts — A[T153] (a4,E3); 5487 Bear Form — A[T27] (a4,a142,a187,a240,a275); 22812 Barkskin — A[T173]; 6795 Growl — B [handlers: E114,E6,a11].
Other A: 768 Cat Form A[T27] (a4,a153,a226,a275,E3); 5221 Shred A[T38]; 8921 Moonfire A[T120] (a269,a537); 8936 Regrowth A[T83]

### Druid / Restoration (105)
Book: A: 768,5221,5487,8921,8936,22812,33917; B: 339,1850,5176,6795,50769,77758,88423,210723; C: 783,1126,5215,18960,20484,22568,77495,114282,127757,164812,164815,164862,165962,193753,208253,210053,210722,236716,276012,276029,440015,468146,1229376; D: 212040.
Core: 774 Rejuvenation — B [handlers: E6,a8]; 8936 Regrowth — A[T83]; 33763 Lifebloom — A[T84]; 48438 Wild Growth — A[T149]; 18562 Swiftmend — B [handlers: E10]; 740 Tranquility — A[T171] (a4,a147,a470); 102342 Ironbark — B [handlers: E6,a283,a87]; 88423 Nature's Cure — B [handlers: E38]; 22812 Barkskin — A[T173].
Other A: 768 Cat Form A[T27] (a4,a153,a226,a275,E3); 5221 Shred A[T38]; 5487 Bear Form A[T27] (a4,a142,a187,a240,a275); 8921 Moonfire A[T120] (a269,a537); 33917 Mangle A[T130] (E3) Other D: 212040 Revitalize (target 119)

### Demon Hunter / Havoc (577)
Book: A: 162243,162794,191427,198589,204596,344859,344862; B: 183752,185123,185164,185245,201467,201628,201789,203555,206478,214743,278386; C: 131347,178940,188499,188501,195072,196055,207407,255260,258920,344865,350570,1229376; D: —.
Core: 162243 Demon's Bite — A[T137] (E3); 162794 Chaos Strike — A[T135]; 188499 Blade Dance — C (a483,E3); 198013 Eye Beam — A[T60] (a4,a85); 195072 Fel Rush — C (E3); 191427 Metamorphosis — A[T61] (a226,a263,a443,a471); 198589 Blur — A[T2]; 183752 Disrupt — B [handlers: E68].
Other A: 204596 Sigil of Flame A[T70] (a4); 344859 Demon's Bite A[T187] (E3); 344862 Chaos Strike A[T186]

### Demon Hunter / Vengeance (581)
Book: A: 187827,191427,203720,204596,263642,344859,344862; B: 183752,185123,185245,201467,201628,201789,203513,204157,214743; C: 131347,188501,189110,196055,202138,203747,204254,207197,207407,228477,255260,258920,320380,344865,350570,1229376; D: —.
Core: 263642 Fracture — A[T166] (E3); 228477 Soul Cleave — C (E3); 204596 Sigil of Flame — A[T70] (a4); 203782 Shear — B [handlers: E2,E30]; 203720 Demon Spikes — A[T110]; 212084 Fel Devastation — C (a4); 187827 Metamorphosis — A[T166] (a142,a226,a443,a471); 189110 Infernal Strike — C (E3); 183752 Disrupt — B [handlers: E68].
Other A: 191427 Metamorphosis A[T61] (a226,a263,a443,a471); 344859 Demon's Bite A[T187] (E3); 344862 Chaos Strike A[T186]

### Demon Hunter / Devourer (1480)
Book: A: 191427,198589,204596,344859,344862,473662,1226019,1234796; B: 183752,185123,185245,201467,201628,201789,214743,1238855,1277736; C: 131347,188501,196055,207407,255260,258920,344865,350570,1217605,1227619,1229376,1251417,1260008; D: —.
Core: 473662 Consume — A[T58]; 1226019 Reap — A[T162] (a4); 1217605 Void Metamorphosis — C (a4,a226,a263,a443,E3); 1234796 Shift — A[T172]; 1251417 Spectral Sight — C (a4,a17,a19,a100,a228,E167); 198589 Blur — A[T2].
Other A: 191427 Metamorphosis A[T61] (a226,a263,a443,a471); 204596 Sigil of Flame A[T70] (a4); 344859 Demon's Bite A[T187] (E3); 344862 Chaos Strike A[T186]

### Evoker / Devastation (1467)
Book: A: 355913,356995,361469,362969,400006; B: 361227,381773,390386; C: 357208,357210,358267,362980,364342,368970,396571,1229376; D: —.
Core: 361469 Living Flame — A[T189]; 362969 Azure Strike — A[T185] (a4); 356995 Disintegrate — A[T138] (a4); 357208 Fire Breath — C (a4,E3); 359073 Eternity Surge — C (a4,E3); 357211 Pyre — A[T112]; 375087 Dragonrage — C (a4,E3); 363916 Obsidian Scales — A[T3] (a4,E3); 351338 Quell — B [handlers: E68].
Other A: 355913 Emerald Blossom A[T165] (E3); 400006 Energizing Flame A[T188]

### Evoker / Preservation (1468)
Book: A: 355913,356995,361469,362969,400006; B: 360823,361227,381773,390386; C: 357208,357210,358267,363510,364342,368970,396571,1229376; D: 361178.
Core: 361469 Living Flame — A[T189]; 355913 Emerald Blossom — A[T165] (E3); 367226 Spiritbloom — C (a4,E3); 366155 Reversion — C (a504); 357170 Time Dilation — C (a4); 360995 Verdant Embrace — A[T154] (E3); 363534 Rewind — A[T107] (E3); 360823 Naturalize — B [handlers: E38]; 363916 Obsidian Scales — A[T3] (a4,E3).
Other A: 356995 Disintegrate A[T138] (a4); 362969 Azure Strike A[T185] (a4); 400006 Energizing Flame A[T188] Other D: 361178 Mass Return (target 119)

### Evoker / Augmentation (1473)
Book: A: 355913,356995,361469,362969,400006; B: 361227,381773,390386; C: 357208,357210,358267,361021,364342,368970,395153,396571,406041,406380,1229376; D: —.
Core: 361469 Living Flame — A[T189]; 362969 Azure Strike — A[T185] (a4); 395152 Ebon Might — C (a226,a540,E3); 395160 Eruption — A[T126]; 396286 Upheaval — C (a4,E3); 409311 Prescience — C (E3); 403631 Breath of Eons — C (a147,a263,a485,a488,E3); 406732 Spatial Paradox — C (a188,a226,a235,a330,E3); 363916 Obsidian Scales — A[T3] (a4,E3).
Other A: 355913 Emerald Blossom A[T165] (E3); 356995 Disintegrate A[T138] (a4); 400006 Energizing Flame A[T188]

## Positive test citations and proof limits

All paths relative to game-server/crates/server/src. Matrix proof classification uses inspected test sources; batch8 native runs are recorded separately in proof-ledger.json. Most cast tests use real world.db with fixture known spells; some aura tests directly apply real catalog auras, proving aura response but not player cast admission. Defensive historical18/18 and complete-crate1563/1563 results belong to docs/defensives-audit.md and are not fresh results on this revision. A guarantees only the named assertion, never all effects. New class hooks have targeted cast tests; the full matrix still uses source evidence and is not a realm-parity claim.

- **T1** `auras/defensives_tests.rs:173::defensives_death_knight_icebound_fortitude_reduces_fire_30_percent` — IDs 48792.
- **T2** `auras/defensives_tests.rs:183::defensives_demon_hunter_blur_trigger_reduces_fire_25_percent` — IDs 198589.
- **T3** `auras/defensives_tests.rs:178::defensives_evoker_obsidian_scales_reduces_fire_30_percent` — IDs 363916.
- **T4** `auras/defensives_tests.rs:149::defensives_hunter_turtle_reduces_fire_30_percent` — IDs 186265.
- **T5** `auras/defensives_tests.rs:139::defensives_mage_ice_block_blocks_fire` — IDs 45438.
- **T6** `auras/defensives_tests.rs:188::defensives_monk_fortifying_brew_reduces_fire_20_percent` — IDs 115203.
- **T7** `auras/defensives_tests.rs:134::defensives_paladin_divine_shield_blocks_fire` — IDs 642.
- **T8** `auras/defensives_tests.rs:342::defensives_priest_power_word_shield_absorbs_and_is_consumed` — IDs 17.
- **T9** `auras/defensives_tests.rs:277::defensives_rogue_evasion_increases_dodge` — IDs 5277.
- **T10** `auras/defensives_tests.rs:168::defensives_shaman_astral_shift_reduces_fire_40_percent` — IDs 108271.
- **T11** `auras/defensives_tests.rs:158::defensives_warlock_unending_resolve_reduces_fire_25_percent` — IDs 104773.
- **T12** `auras/defensives_tests.rs:144::defensives_warrior_shield_wall_reduces_fire_40_percent` — IDs 871.
- **T13** `auras/mage_tests.rs:155::arcane_intellect_cast_on_a_friendly_player_stays_a_buff` — IDs 1459.
- **T14** `auras/mage_tests.rs:36::frostbolt_chills_its_target_to_half_run_speed_for_8_seconds` — IDs 116.
- **T15** `auras/mage_tests.rs:64::polymorph_replicates_its_debuff_on_the_target_with_the_caster_and_duration` — IDs 118.
- **T16** `auras/tests.rs:861::charge_auras_add_a_charge_and_shorten_recovery` — IDs 100.
- **T17** `auras/tests.rs:838::class_mask_and_label_modifiers_scale_damage` — IDs 8092.
- **T18** `auras/tests.rs:785::movement_speed_takes_strongest_increase_and_slow_and_roots_stop` — IDs 122,1715,2983.
- **T19** `auras/tests.rs:643::purify_dispels_a_magic_debuff_combat_log_metadata` — IDs 527.
- **T20** `auras/tests.rs:429::renew_heals_every_tick` — IDs 139.
- **T21** `auras/tests.rs:314::shadow_word_pain_ticks_periodic_damage_then_expires` — IDs 589.
- **T22** `auras/tests.rs:143::spellpower_strength_paladin_level_four_flash_of_light_heals_with_intellect` — IDs 19750.
- **T23** `auras/tests.rs:906::stat_auras_fold_into_character_stats_once` — IDs 2825,21562.
- **T24** `auras/tests.rs:882::stun_and_silence_block_casting` — IDs 853,15487.
- **T25** `auras/tests.rs:809::water_breathing_aura_marks_the_target_water_breathing` — IDs 5697.
- **T26** `hunter_pet_tests.rs:226::dismissed_pet_stays_stored_and_call_pet_brings_it_back` — IDs 883,2641.
- **T27** `spell_cast/class_resource_tests.rs:126::druid_forms_switch_display_power_and_model_and_exclude_each_other` — IDs 768,5487.
- **T28** `spell_cast/dummy10_tests.rs:70::dummy10_blade_flurry_initial_strike_caps_five_or_eight_with_rank` — IDs 13877.
- **T29** `spell_cast/dummy10_tests.rs:116::dummy10_consecration_stays_at_cast_position_pulses_and_expires` — IDs 26573.
- **T30** `spell_cast/dummy10_tests.rs:139::dummy10_feral_lunge_launches_named_bite_only_at_hit_enemy` — IDs 196884.
- **T31** `spell_cast/dummy10_tests.rs:193::dummy10_hand_of_guldan_uses_native_three_shard_meteor_and_target_area` — IDs 105174.
- **T32** `spell_cast/dummy10_tests.rs:224::dummy10_lightning_bolt_grants_rank_maelstrom_once_per_successful_cast` — IDs 188196.
- **T33** `spell_cast/dummy10_tests.rs:177::dummy10_protection_judgment_generates_holy_power_only_with_rank` — IDs 275779.
- **T34** `spell_cast/dummy11_tests.rs:238::dummy11_blade_of_justice_splash_requires_blade_talent` — IDs 184575.
- **T35** `spell_cast/dummy11_tests.rs:256::dummy11_hammer_wave_requires_caster_in_owned_consecration` — IDs 53595.
- **T36** `spell_cast/dummy11_tests.rs:341::dummy11_lava_lash_applies_owned_lashing_flames_only_with_rank` — IDs 60103.
- **T37** `spell_cast/dummy11_tests.rs:203::dummy11_rain_of_fire_has_eight_fixed_ground_pulses` — IDs 5740.
- **T38** `spell_cast/dummy11_tests.rs:290::dummy11_shred_stealth_rank_grants_damage_and_combo_point` — IDs 5221.
- **T39** `spell_cast/dummy11_tests.rs:155::dummy11_sinister_strike_uses_authored_level_curve` — IDs 193315.
- **T40** `spell_cast/dummy11_tests.rs:320::dummy11_thrash_applies_named_bleed_only_to_hit_enemies_and_expires` — IDs 106830.
- **T41** `spell_cast/dummy11_tests.rs:177::dummy11_void_volley_counts_primary_secondary_bolts_and_insanity` — IDs 1242173.
- **T42** `spell_cast/dummy11_tests.rs:193::dummy11_voidform_launches_initial_volley_after_cast_completion` — IDs 228260.
- **T43** `spell_cast/dummy12_tests.rs:383::dummy12_butchery_talent_bleeds_hit_enemies_only` — IDs 212436.
- **T44** `spell_cast/dummy12_tests.rs:155::dummy12_dispatch_and_eviscerate_damage_use_combo_spent_not_left` — IDs 2098,196819.
- **T45** `spell_cast/dummy12_tests.rs:59::dummy12_feral_spirit_summons_two_authentic_wolves_and_expires` — IDs 51533.
- **T46** `spell_cast/dummy12_tests.rs:84::dummy12_gargoyle_native_attack_crit_and_successful_owner_spend` — IDs 49206.
- **T47** `spell_cast/dummy12_tests.rs:177::dummy12_rake_bleed_snapshots_stealth_bonus_across_later_rank_removal` — IDs 1822.
- **T48** `spell_cast/dummy12_tests.rs:236::dummy12_shield_of_righteous_applies_native_armor_buff_and_expires` — IDs 53600.
- **T49** `spell_cast/dummy12_tests.rs:214::dummy12_unstable_affliction_vulnerability_is_owned_not_stacked_and_expires` — IDs 30108.
- **T50** `spell_cast/dummy13_tests.rs:275::dummy13_aspect_wild_fires_free_primary_cobra_without_later_focus_spend` — IDs 193530.
- **T51** `spell_cast/dummy13_tests.rs:224::dummy13_bestial_wrath_strike_uses_own_pets_current_target` — IDs 19574.
- **T52** `spell_cast/dummy13_tests.rs:79::dummy13_execute_refunds_actual_optional_rage_only_with_rank_and_survivor` — IDs 163201.
- **T53** `spell_cast/dummy13_tests.rs:333::dummy13_haunt_only_owned_target_death_resets_running_cooldown` — IDs 48181.
- **T54** `spell_cast/dummy13_tests.rs:39::dummy13_mortal_strike_wound_reduces_real_healing_and_expires` — IDs 12294.
- **T55** `spell_cast/dummy13_tests.rs:158::dummy13_rapid_fire_native_initial_and_six_clocked_shots_generate_focus` — IDs 257044.
- **T56** `spell_cast/dummy13_tests.rs:64::dummy13_rend_applies_owned_native_bleed_and_ticks` — IDs 772.
- **T57** `spell_cast/dummy13_tests.rs:132::dummy13_shadow_word_death_native_execute_threshold_and_deathspeaker` — IDs 32379.
- **T58** `spell_cast/dummy14_tests.rs:231::dummy14_consume_generates_named_fury_once_without_optional_fragments` — IDs 473662.
- **T59** `spell_cast/dummy14_tests.rs:151::dummy14_deathmark_restores_native_total_energy_on_bleed_clock` — IDs 360194.
- **T60** `spell_cast/dummy14_tests.rs:166::dummy14_eye_beam_native_initial_plus_ten_primary_hits_with_ranked_guaranteed_crit` — IDs 198013.
- **T61** `spell_cast/dummy14_tests.rs:205::dummy14_metamorphosis_landing_hits_at_destination_and_splits_player_cc` — IDs 191427.
- **T62** `spell_cast/dummy14_tests.rs:84::dummy14_survival_kill_command_routes_named_damage_from_owned_pet` — IDs 259489.
- **T63** `spell_cast/dummy14_tests.rs:120::dummy14_wildfire_primary_named_hit_bonus_and_owned_bleed` — IDs 259495.
- **T64** `spell_cast/dummy15_tests.rs:239::dummy15_beacon_transfers_only_effective_direct_group_heal_to_owned_mark` — IDs 53563.
- **T65** `spell_cast/dummy15_tests.rs:134::dummy15_combustion_pyromaniac_snapshots_native_crit_fraction_and_rank_base` — IDs 190319.
- **T66** `spell_cast/dummy15_tests.rs:55::dummy15_frost_strike_uses_native_two_hand_or_dual_wield_children_not_both` — IDs 49143.
- **T67** `spell_cast/dummy15_tests.rs:171::dummy15_high_tide_two_chain_heals_have_no_falloff_then_native_falloff_returns` — IDs 1064.
- **T68** `spell_cast/dummy15_tests.rs:113::dummy15_karma_shield_uses_native_half_health_cap_and_damage_depletion` — IDs 122470.
- **T69** `spell_cast/dummy15_tests.rs:79::dummy15_rising_sun_kick_rank_wounds_only_actual_primary_and_expires` — IDs 107428.
- **T70** `spell_cast/dummy15_tests.rs:266::dummy15_sigil_flame_waits_native_delay_hits_original_destination_and_bleeds_once` — IDs 204596.
- **T71** `spell_cast/dummy15_tests.rs:39::dummy15_vampiric_blood_grants_native_max_health_without_refresh_stacking` — IDs 55233.
- **T72** `spell_cast/dummy16_tests.rs:209::dummy16_arcane_blast_snapshots_pre_gain_charge_damage_and_cost` — IDs 30451.
- **T73** `spell_cast/dummy16_tests.rs:142::dummy16_arcane_power_mana_discount_requires_native_rank` — IDs 12042.
- **T74** `spell_cast/dummy16_tests.rs:90::dummy16_crusader_strike_crusaders_might_reduces_only_running_judgment` — IDs 35395.
- **T75** `spell_cast/dummy16_tests.rs:266::dummy16_healing_wave_heals_only_native_primary_and_never_enemy` — IDs 77472.
- **T76** `spell_cast/dummy16_tests.rs:138::dummy16_holy_judgment_generates_named_rank_power_once` — IDs 275773.
- **T77** `spell_cast/dummy16_tests.rs:94::dummy16_holy_light_generates_named_holy_power_only_with_rank` — IDs 82326.
- **T78** `spell_cast/dummy16_tests.rs:86::dummy16_holy_shock_crusaders_might_reduces_only_running_judgment` — IDs 20473.
- **T79** `spell_cast/dummy16_tests.rs:134::dummy16_retribution_judgment_generates_named_rank_power_once` — IDs 20271.
- **T80** `spell_cast/dummy16_tests.rs:310::dummy16_riptide_grants_two_capped_tidal_uses_not_double_cast_speed` — IDs 61295.
- **T81** `spell_cast/dummy17_tests.rs:184::dummy17_blood_boil_applies_plague_to_hit_enemies_unless_rank_replaces_it` — IDs 50842.
- **T82** `spell_cast/dummy17_tests.rs:410::dummy17_cenarion_ward_has_native_initial_plus_four_heals_then_expires_without_retriggering` — IDs 102351.
- **T83** `spell_cast/dummy17_tests.rs:290::dummy17_improved_regrowth_native_bonus_makes_existing_hot_initial_heal_critical` — IDs 8936.
- **T84** `spell_cast/dummy17_tests.rs:250::dummy17_lifebloom_native_expiry_blooms_once_after_periodic_heals` — IDs 33763.
- **T85** `spell_cast/dummy17_tests.rs:206::dummy17_marrowrend_generates_three_bone_shield_stacks_up_to_native_cap` — IDs 195182.
- **T86** `spell_cast/dummy17_tests.rs:88::dummy17_shadow_bolt_grants_one_native_shard_only_in_demonology` — IDs 686.
- **T87** `spell_cast/dummy17_tests.rs:158::dummy17_soul_fire_grants_named_shard_and_applies_only_selected_dot_variant` — IDs 6353.
- **T88** `spell_cast/dummy18_tests.rs:103::dummy18_blood_plague_native_tick_leech` — IDs 55078.
- **T89** `spell_cast/dummy18_tests.rs:283::dummy18_drain_life_native_channel_ticks_stop_on_cancellation_or_expiry` — IDs 234153.
- **T90** `spell_cast/dummy18_tests.rs:207::dummy18_ignore_pain_soaks_native_fraction_and_accumulates_to_health_cap` — IDs 190456.
- **T91** `spell_cast/dummy18_tests.rs:224::dummy18_improved_conflagrate_grants_two_uses_and_native_cap_four` — IDs 17962.
- **T92** `spell_cast/dummy18_tests.rs:95::dummy18_madness_native_tick_leech` — IDs 335467.
- **T93** `spell_cast/dummy18_tests.rs:248::dummy18_obliterate_selects_native_physical_and_frost_weapon_children` — IDs 49020.
- **T94** `spell_cast/dummy18_tests.rs:107::dummy18_singularity_native_primary_leech_area_dummy_retained` — IDs 205179.
- **T95** `spell_cast/dummy19_tests.rs:104::dummy19_altered_form_extends_healing_clock` — IDs 453729.
- **T96** `spell_cast/dummy19_tests.rs:180::dummy19_ardent_defender_spell_fatal_hit_restores_native_health_once` — IDs 31850.
- **T97** `spell_cast/dummy19_tests.rs:96::dummy19_dispersion_native_total_not_per_tick` — IDs 47585.
- **T98** `spell_cast/dummy19_tests.rs:43::dummy19_flurry_native_three_not_four_bolts` — IDs 44614.
- **T99** `spell_cast/dummy19_tests.rs:137::dummy19_havoc_owned_direct_single_target_copy` — IDs 80240.
- **T100** `spell_cast/dummy19_tests.rs:100::dummy19_intangibility_adds_native_total` — IDs 288733.
- **T101** `spell_cast/dummy19_tests.rs:57::dummy19_mutilate_variable394_half_offhand` — IDs 1329.
- **T102** `spell_cast/dummy19_tests.rs:108::dummy19_vampiric_touch_real_dispel_horrifies_dispeller_only` — IDs 34914.
- **T103** `spell_cast/dummy20_tests.rs:51::dummy20_barbed_reapplication_adds_unpaid_damage_to_native_five_tick_total` — IDs 217200.
- **T104** `spell_cast/dummy20_tests.rs:210::dummy20_stormkeeper_root_completion_and_cancellation` — IDs 191634.
- **T105** `spell_cast/dummy21_tests.rs:78::dummy21_berserk_completed_root_adds_one_to_shred` — IDs 106951.
- **T106** `spell_cast/dummy22_tests.rs:149::dummy22_mind_flay_native_channel_pays_six_three_insanity_ticks` — IDs 15407.
- **T107** `spell_cast/dummy22_tests.rs:73::dummy22_rewind_native_minimum_is_increased_200_percent_when_solo` — IDs 363534.
- **T108** `spell_cast/dummy23_tests.rs:31::dummy23_rune_weapon_completed_root_grants_owner_native_parry` — IDs 49028.
- **T109** `spell_cast/dummy23_tests.rs:207::dummy23_shadow_blades_native_storm_doubles_each_enemy_not_once` — IDs 121471.
- **T110** `spell_cast/dummy2_tests.rs:465::dummy2_dh_demon_spikes_grants_agility_armor_and_expires` — IDs 203720.
- **T111** `spell_cast/dummy2_tests.rs:304::dummy2_dk_death_coil_damages_an_enemy_through_47632` — IDs 47541.
- **T112** `spell_cast/dummy2_tests.rs:186::dummy2_evoker_pyre_explodes_at_target_and_hits_nearby_enemies_only` — IDs 357211.
- **T113** `spell_cast/dummy2_tests.rs:6::dummy2_hunter_cobra_shot_damages_and_reduces_kill_command_cooldown` — IDs 193455.
- **T114** `spell_cast/dummy2_tests.rs:287::dummy2_mage_fire_blast_guarantees_a_critical_hit_with_zero_crit_stats` — IDs 108853.
- **T115** `spell_cast/dummy2_tests.rs:64::dummy2_monk_blackout_kick_reduces_rising_sun_kick_recharge_with_rank_two` — IDs 100784.
- **T116** `spell_cast/dummy2_tests.rs:161::dummy2_rogue_backstab_deals_twenty_percent_more_from_behind` — IDs 53.
- **T117** `spell_cast/dummy2_tests.rs:243::dummy2_warlock_drain_soul_target_death_grants_one_whole_shard` — IDs 198590.
- **T118** `spell_cast/dummy3_tests.rs:319::dummy3_devastate_rank_chance_resets_shield_slam_and_notifies_owner` — IDs 20243.
- **T119** `spell_cast/dummy3_tests.rs:171::dummy3_garrote_from_stealth_with_rank_silences_and_reduces_damage_for_five_seconds` — IDs 703.
- **T120** `spell_cast/dummy3_tests.rs:7::dummy3_moonfire_bridges_direct_damage_and_periodic_damage_without_base_slow` — IDs 8921.
- **T121** `spell_cast/dummy3_tests.rs:101::dummy3_prayer_heals_primary_and_four_injured_allies_within_catalog_radius` — IDs 596.
- **T122** `spell_cast/dummy3_tests.rs:31::dummy3_scorch_below_thirty_percent_guarantees_crit` — IDs 2948.
- **T123** `spell_cast/dummy3_tests.rs:65::dummy3_serenity_flash_heal_reduces_running_recharge_by_six_seconds` — IDs 2050.
- **T124** `spell_cast/dummy3_tests.rs:151::dummy3_thunder_clap_slows_each_hit_enemy_not_allies_or_far_enemies` — IDs 6343.
- **T125** `spell_cast/dummy4_tests.rs:214::dummy4_chain_lightning_generates_two_maelstrom_per_hit_with_rank` — IDs 188443.
- **T126** `spell_cast/dummy4_tests.rs:292::dummy4_eruption_extends_only_owned_active_ebon_might_with_rank` — IDs 395160.
- **T127** `spell_cast/dummy4_tests.rs:223::dummy4_howling_blast_applies_frost_fever_to_each_hit_enemy` — IDs 49184.
- **T128** `spell_cast/dummy4_tests.rs:56::dummy4_kill_shot_rejects_twenty_percent_before_spending_focus` — IDs 53351.
- **T129** `spell_cast/dummy4_tests.rs:19::dummy4_light_of_dawn_heals_only_five_injured_allies_in_radius` — IDs 85222.
- **T130** `spell_cast/dummy4_tests.rs:105::dummy4_mangle_rank_increases_damage_against_bleeding_target` — IDs 33917.
- **T131** `spell_cast/dummy4_tests.rs:171::dummy4_renewing_mist_cast_applies_catalog_hot_and_expires` — IDs 115151.
- **T132** `spell_cast/dummy4_tests.rs:40::dummy4_steady_shot_generates_catalog_twenty_focus` — IDs 56641.
- **T133** `spell_cast/dummy4_tests.rs:162::dummy4_vivify_rank_heals_owned_renewing_mist_only` — IDs 116670.
- **T134** `spell_cast/dummy5_tests.rs:128::dummy5_chaos_bolt_always_crits_with_zero_crit_stats` — IDs 116858.
- **T135** `spell_cast/dummy5_tests.rs:85::dummy5_chaos_strike_refunds_once_at_catalog_chance_endpoint` — IDs 162794.
- **T136** `spell_cast/dummy5_tests.rs:194::dummy5_deaths_caress_generates_two_bone_shield_stacks` — IDs 195292.
- **T137** `spell_cast/dummy5_tests.rs:52::dummy5_demons_bite_insatiable_hunger_adds_catalog_fury_range` — IDs 162243.
- **T138** `spell_cast/dummy5_tests.rs:94::dummy5_disintegrate_applies_three_second_slow_and_ticks` — IDs 356995.
- **T139** `spell_cast/dummy5_tests.rs:217::dummy5_epidemic_eruptions_require_owned_virulent_plague` — IDs 207317.
- **T140** `spell_cast/dummy5_tests.rs:283::dummy5_lava_burst_crits_on_flame_shock_and_generates_eight_maelstrom` — IDs 51505.
- **T141** `spell_cast/dummy5_tests.rs:137::dummy5_rapture_counts_only_owned_damaging_periodic_effects` — IDs 324536.
- **T142** `spell_cast/dummy5_tests.rs:169::dummy5_shuriken_storm_awards_combo_points_per_enemy_hit` — IDs 197835.
- **T143** `spell_cast/dummy6_tests.rs:98::dummy6_ice_lance_resolves_named_damage_child` — IDs 30455.
- **T144** `spell_cast/dummy6_tests.rs:122::dummy6_keg_smash_primary_bonus_keeps_area_damage_and_slow` — IDs 121253.
- **T145** `spell_cast/dummy6_tests.rs:307::dummy6_starfall_extends_owned_dots_with_aetherial_kindling` — IDs 191034.
- **T146** `spell_cast/dummy6_tests.rs:106::dummy6_starfire_primary_full_secondary_seventy_percent` — IDs 194153.
- **T147** `spell_cast/dummy6_tests.rs:79::dummy6_sunfire_base_damage_and_dot_exclude_optional_area` — IDs 93402.
- **T148** `spell_cast/dummy6_tests.rs:88::dummy6_templars_verdict_resolves_named_damage_child` — IDs 85256.
- **T149** `spell_cast/dummy6_tests.rs:216::dummy6_wild_growth_caps_injured_allies_not_healthy_units` — IDs 48438.
- **T150** `spell_cast/dummy7_tests.rs:32::dummy7_colossus_smash_marks_each_hit_enemy_and_only_amplifies_owners_damage` — IDs 167105.
- **T151** `spell_cast/dummy7_tests.rs:145::dummy7_heartbreaker_energizes_per_native_chain_target` — IDs 206930.
- **T152** `spell_cast/dummy7_tests.rs:258::dummy7_raging_blow_rank_success_restores_one_charge_and_notifies_owner` — IDs 85288.
- **T153** `spell_cast/dummy7_tests.rs:61::dummy7_survival_instincts_halves_incoming_damage_and_cancellation_removes_child` — IDs 61336.
- **T154** `spell_cast/dummy7_tests.rs:405::dummy7_verdant_embrace_panacea_heals_caster_not_selected_ally` — IDs 360995.
- **T155** `spell_cast/dummy7_tests.rs:365::dummy7_whirlwind_rank_generates_per_enemy_rage_up_to_five` — IDs 190411.
- **T156** `spell_cast/dummy8_tests.rs:233::dummy8_breath_of_fire_burns_only_owned_keg_smash_targets_in_front` — IDs 115181.
- **T157** `spell_cast/dummy8_tests.rs:213::dummy8_crash_lightning_uses_destination_cone_and_grants_authored_enhancement` — IDs 187874.
- **T158** `spell_cast/dummy8_tests.rs:203::dummy8_demonbolt_generates_two_whole_shards_once_on_landed_damage` — IDs 264178.
- **T159** `spell_cast/dummy8_tests.rs:265::dummy8_fan_of_knives_awards_one_combo_point_not_one_per_enemy` — IDs 51723.
- **T160** `spell_cast/dummy8_tests.rs:322::dummy8_incinerate_generates_authored_base_fragments_once_not_per_area_target` — IDs 29722.
- **T161** `spell_cast/dummy8_tests.rs:69::dummy8_provoke_changes_victim_equalizes_threat_and_holds_until_expiry` — IDs 115546.
- **T162** `spell_cast/dummy8_tests.rs:29::dummy8_reap_hits_enemies_along_destination_line_not_single_target_or_radial_area` — IDs 1226019.
- **T163** `spell_cast/dummy8_tests.rs:287::dummy8_sanctify_caps_five_injured_allies_at_requested_location` — IDs 34861.
- **T164** `spell_cast/dummy9_tests.rs:94::dummy9_earthquake_pulses_six_times_at_requested_ground_and_excludes_allies` — IDs 61882.
- **T165** `spell_cast/dummy9_tests.rs:126::dummy9_emerald_blossom_delays_and_heals_three_injured_at_fixed_ally_location` — IDs 355913.
- **T166** `spell_cast/dummy9_tests.rs:218::dummy9_fracture_requires_both_meta_and_rank_for_fifteen_additional_fury` — IDs 187827,263642.
- **T167** `spell_cast/dummy9_tests.rs:54::dummy9_healing_rain_ticks_at_ground_caps_six_and_stops_at_expiry` — IDs 73920.
- **T168** `spell_cast/dummy9_tests.rs:288::dummy9_radiance_applies_nine_second_atonement_to_selected_allies_preserves_foreign_mark` — IDs 194509.
- **T169** `spell_cast/dummy9_tests.rs:151::dummy9_remorseless_winter_hit_applies_slow_and_native_expiry_removes_it` — IDs 196770.
- **T170** `spell_cast/dummy9_tests.rs:186::dummy9_scourge_strike_applies_base_virulent_plague_without_refreshing_existing_owned_mark` — IDs 55090.
- **T171** `spell_cast/dummy9_tests.rs:342::dummy9_tranquility_shield_absorbs_sixty_percent_max_health_and_ends_with_channel` — IDs 740.
- **T172** `spell_cast/dummy9_tests.rs:249::dummy9_void_step_teleports_to_requested_location_preserving_map` — IDs 1234796.
- **T173** `spell_cast/dummy_tests.rs:108::dummy_druid_barkskin_with_brambles_pulses_catalog_damage_each_second` — IDs 22812.
- **T174** `spell_cast/dummy_tests.rs:9::dummy_priest_penance_channels_three_damage_bolts` — IDs 47540.
- **T175** `spell_cast/dummy_tests.rs:263::dummy_warrior_bloodthirst_damages_generates_rage_and_heals_three_percent` — IDs 23881.
- **T176** `spell_cast/dummy_tests.rs:320::dummy_warrior_rampage_spends_rage_hits_and_enrages` — IDs 184367.
- **T177** `spell_cast/dummy_tests.rs:236::dummy_warrior_shield_block_applies_six_second_block_aura` — IDs 2565.
- **T178** `spell_cast/guardian_tests.rs:107::guardians_sacrifice_splits_spell_damage_and_stops_on_native_health_floor` — IDs 6940.
- **T179** `spell_cast/mechanics_talent_tests.rs:7::spellmech_barrage_charge_damage_and_talented_mana_refund_survive_flight` — IDs 44425.
- **T180** `spell_cast/mechanics_talent_tests.rs:119::spellmech_black_powder_scales_each_native_area_hit_by_spent_combo` — IDs 319175.
- **T181** `spell_cast/mechanics_talent_tests.rs:78::spellmech_rip_native_duration_and_optional_cost_use_same_snapshot` — IDs 1079.
- **T182** `spell_cast/mechanics_tests.rs:49::spellmech_envenom_spends_all_combo_points_and_scales_damage` — IDs 32645.
- **T183** `spell_cast/mechanics_tests.rs:7::spellmech_optional_combo_cost_snapshots_rupture_duration` — IDs 1943.
- **T184** `spell_cast/mechanics_tests.rs:38::spellmech_surge_spends_native_optional_current_mana` — IDs 365350.
- **T185** `spell_cast/new_class_tests.rs:193::azure_strike_hits_the_target_and_one_other_enemy` — IDs 362969.
- **T186** `spell_cast/new_class_tests.rs:97::chaos_strike_spends_40_fury_and_hits_twice` — IDs 344862.
- **T187** `spell_cast/new_class_tests.rs:81::demons_bite_generates_fury` — IDs 344859.
- **T188** `spell_cast/new_class_tests.rs:234::energizing_flame_refunds_most_of_living_flames_mana` — IDs 400006.
- **T189** `spell_cast/new_class_tests.rs:137::living_flame_casts_for_2_seconds_spends_mana_and_damages_enemy` — IDs 361469.
- **T190** `spell_cast/new_class_tests.rs:44::tiger_palm_spends_50_energy_and_hits` — IDs 100780.
- **T191** `spell_cast/queue_tests.rs:55::spellqueue_builder_effect_precedes_real_rage_spender` — IDs 1464.
- **T192** `spell_cast/tests.rs:150::cast_time_spell_completes_after_its_cast_time` — IDs 133.
- **T193** `spell_cast/tests.rs:205::kick_interrupts_target_cast_and_locks_its_school_combat_log_metadata` — IDs 1766.
- **T194** `spell_cast/tests.rs:289::threat_heal_clamps_to_max_and_logs_overheal` — IDs 2061.
- **T195** `warlock_pet_tests.rs:174::warlock_summon_imp_spawns_owned_pet_and_firebolt_bar` — IDs 688.
