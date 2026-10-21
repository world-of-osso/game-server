# Guardian follow-ups — 2026-10-10

Source-backed Darkglare attack plus a bounded local investigation of owner AP/SP/stamina inheritance, guardian/pet melee and named bindings. [Contract](specs/spell-guardians.md), [runtime](wiki/systems/spell-casting.md#guardian-source-follow-ups), [census](spell-coverage.md). No guessed inheritance percentages, attacks, IDs or rotations. Main inheritance/melee requests remain **underivable from inspected local sources**, not implemented or certified by this batch.

## Capability matrix

| Path | Done / proof | Missing / underivable |
|---|---|---|
| Darkglare103673 | Native205180 description explicitly names205231; actual effect28 actor casts Eye Beam after native2000ms. Existing demon SP contribution reads changed owner SP live; retarget, no-selection, native20000ms expiry and no persistent-pet replacement proved. | No new Retail inheritance-coefficient proof. Root DoT extension, Presence1280663, per-DoT bonus and Dummy/aura4 fields remain unconverted; direct effect28 proof is not root A. |
| Owner AP/SP/stamina and pet/guardian melee | Existing hunter22% AP and demon15% SP/57% AP paths inspected, not replaced or recertified from DB2. Archive base-stat schema and seven gametables inspected. | No local numeric non-demon inheritance coefficient, stamina share or guardian weapon-damage curve established. Wolf29264 supplies1500ms BaseAttackTime only; Ghoul26125/Army24207 supply2000ms only. Current generic creature swings are not guardian Retail proof. No generic flat-damage substitute or new extrapolation added. |
| Xuen63508 / Rune Weapon27893 | Xuen123904 explicitly names123999's1000ms periodic trigger123996 (native25.7% actor AP); display45042. Rune Weapon49028 names27893/displays1126/73371;51906 aura79 contains numeric zero. | Xuen actor AP inheritance, three-target policy and Empowered Lightning accumulation remain unproved. Rune Weapon mirroring and damage coefficients require script semantics; model/zero scaling aura cannot establish them. No dormant zero-damage driver presented as implemented. |
| Wild Imp55659 / Dreadstalker98035 | Source-qualified104317/55659→104318 uses SimC76f8118 action721/867 and constructor705 ownerSP1.2; native coefficient retained. Direct native2GREEN25.48s, retained guardians11GREEN13.66s at42f5f0cc7; authenticated template/models on copies only. Dreadstalker remains unbound, negative no-Imp-substitution control retained. | HoG routing, Imp energy-exhaustion/Core lifecycle, Tyrant orchestration and full guardian parity remain unproved. No Dreadstalker attack/scaling policy supplied. |
| Tyrant135002/250289 / DK follow-ups |265187 explicitly summons both entries; native1276788 buff reference retained. Modern42650 Army description references Lesser Ghoul Orders;46584 now triggers1242866's zero Dummy aura. | Tyrant base attack binding absent. Legacy42651 entry24207 does not establish modern Army Orders. No inferred ghoul/Army/elemental damage, Raise Dead summon bridge or talent variant. |

## Source boundaries and preparation

Read-only canonical world.db, pinned Mainline12.1.0.69933 CSVs and TDB1210.26091 archive only. **183 native rows match pinned CSV exactly**, numeric tolerance1e-10, strings/newlines exact; zero differences. Unversioned engine-root CSV was rejected after its Army description differed from the pinned version.

Archive surveyed9586 creature_template_spell,28481 creature_template_addon,55241 smart_scripts,4066 spell_script_names,2715 pet_levelstats,220 spell_linked_spell and1283 spell_proc rows. Requested named entries have no creature-spell/addon/smart-script rows. Only relevant archive linked aura is58875→58876 Spirit Walk; no relevant spell_proc binding. CreatureDisplayInfo proves model association, not combat abilities or inheritance. Darkglare's attack binding comes from the explicit root tooltip reference, not spell-name similarity.

pet_levelstats has hp/mana/armor/str/agi/sta/inte/spi—not inheritance percentages or weapon damage. Authentic416 and26125 rows reach level80; generic entry1 ends with placeholder all-one level85 data. These rows do not justify level90 extrapolation. hppersta and staminamultbyilvl describe stat-to-health/item scaling, not owner-to-pet stamina share. Tamed passive18730/30147/30148/30149/35695/35697 and Rune Weapon51906 contain zero amounts, not recoverable scripted coefficients. No TrinityCore C++ body or external/CDN data used.

Runtime Darkglare template is absent from the current content overlay. New importer preserves authentic TDB103673/display116130 on a **prepared copy**. It rejects live world.db and rolls back a missing-model import. Not executed against production:

```text
python3 scripts/add_guardian_templates.py --db data/world.db.new
```

Existing guardian_spells import is already present; no new creature-spell rows are inserted or synthesized. Rust tests run the actual importer on a disposable DB copy and load the resulting runtime overlay; no substituted Imp template. Owner stats remain the existing implementation, not a newly imported scaling policy.

## Census and proof

Initial frozen615fb90 baseline exactly reproduced **164 roots,93 reviewed/71 underived**. Aftera053c9f: **164 roots,94 reviewed/70 underived**; Darkglare moves to reviewed/partial, **no whole Dummy root closes**. E3 **115 IDs/123 memberships**, aura4 **67/75**, aura226 **18/19** unchanged. Scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged.

Final origin/master809b61e integration at25bc1fc includes batch15 and petbattle-effects3. Its baseline exactly reproduces **163 roots,98 reviewed/65 underived**; combined guardian result **163 roots,99 reviewed/64 underived**. E3 **114/122**, aura4 **66/74**, aura226 **18/19** unchanged by this follow-up. All40 batch15 baseline script/A ID sets match exactly; only Affliction core script presence3→4 for205180, root A remains5. No generic handler inflated into named-script coverage. Integrated generators/receipts live under evidence `integrated/`.

Proof ledger: importer initial0/3 reflects absent CLI, not complete semantic RED. First GREEN1/3 exposed a missing retail-map prerequisite; correctedc9d2bee **3/3** (native template/model, idempotence, missing-model rollback, live-DB refusal). Actor RED1/3: two Darkglare failures reproduce absent cast/damage; unbound-Wild-Imp negative already passes. a053c9f focused **3/3**, warning-free. Native Eye Beam variance alone is pinned0 in tests for exact damage731→1461 when owner SP changes1000→2000. Native cast time, coefficient, summon entry/model/lifetime and target ownership remain real. No live/network/full-rotation or new inheritance/melee parity claim.

Integrated origin/master at6ea07e4 (only wiki-index conflict; both guardian and toy9 entries preserved). Requested `cargo test -p server --bin game-server spell_cast` ran **once:352/352**, warning-free. Format/check passed. Later7f1bd32 is test-fixture-only: prepared DB now drops after loading instead of leaking a static TempDir; current affected Darkglare **2/2** and final fmt check pass. No repeated broad gate or fresh352/352 claim at7f1bd32. Production cargo-check proof at6ea07e4 remains valid. Scoped existing-pet-slot fixture at48cead4 verifies the same pet survives summon and expiry. Four old owned fixture copies removed. origin/master advanced afterward:25bc1fc integrates batch15/petbattle-effects3, preserving its newer coverage matrix. Bounded guardian plus batch15 tests **22/22**, warning-free; fmt/check pass at25bc1fc. Broad spell_cast was not repeated; its352/352 proof belongs to6ea07e4, not the later integration. No fresh363/363 claim. Manual changed-Rust readability audit; metric CLI absent. Existing initialize length and explicit test-fixture orchestration retained, no warning suppression or unrelated redesign.

Durable evidence: `game-engine/data/diagnostics/spell-guardians2-2026-10-10/` — source_audit.py,183-row source-receipts.json, archive-bindings.json, pet-base-stats.json, frozen before/after sources, census/all40 generators and logs. Parent owns independent integration acceptance; no live/full-crate/rotation proof.

## All40-spec preference table

Final integrated batch15 baseline→guardian follow-up. Counts mean bounded named-script presence, not full ability parity. Darkglare proof invokes native effect28 directly, so root A is unchanged.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 2→2 | 3→3 |
| Mage Fire (63) | 1→1 | 3→3 | 5→5 |
| Mage Frost (64) | 1→1 | 4→4 | 4→4 |
| Paladin Holy (65) | 4→4 | 3→3 | 5→5 |
| Paladin Protection (66) | 4→4 | 4→4 | 5→5 |
| Paladin Retribution (70) | 4→4 | 2→2 | 3→3 |
| Warrior Arms (71) | 2→2 | 4→4 | 7→7 |
| Warrior Fury (72) | 3→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 3→3 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 4→4 | 5→5 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 3→3 | 4→4 |
| Death Knight Frost (251) | 2→2 | 3→3 | 5→5 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 4→4 | 5→5 |
| Hunter Marksmanship (254) | 2→2 | 4→4 | 4→4 |
| Hunter Survival (255) | 2→2 | 4→4 | 4→4 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 2→2 | 5→5 |
| Rogue Assassination (259) | 4→4 | 4→4 | 7→7 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 4→4 | 6→6 |
| Shaman Restoration (264) | 1→1 | 2→2 | 3→3 |
| Warlock Affliction (265) | 0→0 | 3→4 | 5→5 |
| Warlock Demonology (266) | 0→0 | 2→2 | 4→4 |
| Warlock Destruction (267) | 1→1 | 3→3 | 5→5 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 4→4 | 4→4 | 5→5 |
| Monk Mistweaver (270) | 4→4 | 4→4 | 4→4 |
| Demon Hunter Havoc (577) | 6→6 | 4→4 | 5→5 |
| Demon Hunter Vengeance (581) | 5→5 | 4→4 | 4→4 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 7→7 | 3→3 | 4→4 |
