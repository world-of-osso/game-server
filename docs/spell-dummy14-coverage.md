# Class spell batch14 — 2026-10-10

Eight bounded local Mainline12.1.0.69933 consumers, prioritizing core script counts of two (Frost, Survival, Havoc, Devourer), plus Deathmark's complete80-Energy Dummy contract. [Contract](specs/spell-dummy-scripts.md#fourteenth-batch-local-hit-actor-and-periodic-consumers), [runtime](wiki/systems/spell-casting.md#fourteenth-batch-owned-hits-and-native-clocks), [census](spell-coverage.md). Sources only local DB2/CASC/read-only canonical world.db; no TrinityCore C++ bodies/external source. No database writes, data/.uid commits, UDP5000, deployment or master-destination merge.

## Capabilities / limits

| Entry | Implemented / behavioral proof | Missing / uncertified |
|---|---|---|
|84714 Frozen Orb /84721 hit|Actual named child hit applies owned205708 slow, native8sec expiry.|Root179 orb actor/movement/pursuit, soft cap8 and markers; child-hit proof is not a root cast.|
|190356 Blizzard /190357 hit|Actual named child hit applies owned12486 slow, native3sec expiry.|Root field/periodic generation and markers; child-hit proof is not a root cast.|
|259489 Survival Kill Command|Named259277 cast by actual owned persistent pet at explicit enemy; foreign pet excluded. Native22AP fixture gives66 damage; routing proof, not invented owner scaling.|Live pet AI/inheritance,260285 talent and complete admission.|
|259495 Wildfire Bomb|Native35yd/sec root missile, named primary265157 hit with s3's authored percentage bonus, owned269747 six native bleed ticks and expiry.|179 cone actor/secondary targets and s2 cap8. No synthetic splash geometry.|
|360194 Deathmark|s3/80 Energy restored over native16sec/2sec bleed clock. Eight gains total80; expiry stops gains.|75% named vulnerability/poison duplication. Energy distribution uses the existing bleed clock, not a separate surviving timer.|
|198013 Eye Beam|Named198030 at forward in-range channel primary; native200ms/2000ms/EXTRA_INITIAL_PERIOD gives11 hits; active320415 guarantees crit. Cancellation stops attacks.|Secondary129 geometry, cap5, marker50, extended duration and343311 haste. Variable316 total expression references10; native11 clock retained, discrepancy explicit.|
|191427 Metamorphosis|Named200166 damage at8yd destination; nonplayers stunned, actual duel players get247121 daze rather than stun. Native endpoint movement preserved.|Leap traversal, talent cooldown resets, descendant Dummy markers. Root Dummy consumed but whole root remains unresolved through descendants.|
|473662 Consume|Named1261710 grants8 Fury once on actual hit; immune target grants none.|Optional fragment1215617/1227619 actions. No guessed binding.|

Expel Harm rejected during source selection: local115129 radius does not establish damage geometry. Guardian inheritance and wolf melee remain as recorded in [batch13](spell-dummy13-coverage.md); no guessed formulas added.

## Census / source receipts

Frozen e88f0a6 baseline exactly reproduces166 roots,90 reviewed/76 underived. After95604f2: **164 roots,93 reviewed/71 underived**. Whole remaining Dummy roots259489 and360194 close; this does not certify their talent, other aura or actor semantics. Metamorphosis consumes its root E3 but descendant Dummy contracts remain. E3=117 IDs/125 memberships→115/123; aura4=68/76→67/75; aura226=18/19 unchanged. No newly exposed Dummy root. Fixed scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged.

118 SQLite native rows exactly match local CSV columns: zero missing/different rows, numeric tolerance1e-10, strings/newlines exact. Separate native duration/radius/variable316 receipts retain Eye Beam's clock/description discrepancy. Frozen `regenerate.py` and `per_spec.py` validate the previous40 rows/ID sets exactly. Frost's child-only hit tests increase script presence without inflating root A proofs.

## Proof ledger

Initial RED1/9: eight expected missing-behavior failures; foreign-pet exclusion already passed without the missing damage bridge.14bedca implementation; first GREEN4/9 revealed real missing ownership validation plus fixture assumptions about inherited pet AP, root missile travel, damage rounding and unsupported129 geometry. f44628e uses actual pet ownership and a primary-only Eye Beam contract; second GREEN7/9. c7121a4 pins the rank-passive fixture and native EXTRA_INITIAL_PERIOD clock:8/9. Neutral/NPC player faction assumptions do not create PvP hostility;6cf5bcc uses actual duel relationship and focused landing1/1 passes. All other eight cases passed at c7121a4; no production changes after f44628e.

95604f2 adds three concrete boundaries: Energy total/expiry, channel cancellation and immune-hit resource refusal, focused3/3. The first boundary command put libtest filters in Cargo's argument position and failed before tests; corrected argv runs those3 cases. No fabricated whole-nine RED or full-crate/live/rotation proof.

Merged origin/master before the requested gate (already up to date at143ea18). Single requested `cargo test -p server --bin game-server spell_cast` ran ONCE:347 passed/2 failed. Both failures were existing fixtures loading259495 without its newly required native265157/269747 children, not a production/data failure. ff410fe adds those real rows to the fixture stores and retains the unbound cone-actor assertions. Bounded rerun of those2 failures plus all12 batch14 cases:14/14, warning-free. No repeated broad gate or fresh349/349 claim. `cargo check --locked -p server --bin game-server` passes at143ea18; later Rust changes are test-only. Final `cargo fmt --all --check` passes atff410fe. No full-crate/live/rotation or independent acceptance claim. Manual readability scan records long explicit plan functions; metric CLI unavailable.

Durable evidence: `game-engine/data/diagnostics/spell-dummy14-2026-10-10/` (frozen before/after source tars,118-row equivalence, native clock/variable receipts, census/all40 generators and proof logs). Parent owns independent integration acceptance.

## All40-spec preference table

Counts mean bounded production script presence, not whole spell parity. Frost child-hit consumers count as script presence only; prior40 rows exactly reproduced. No arbitrary talent expansion.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 2→2 | 3→3 |
| Mage Fire (63) | 1→1 | 2→2 | 4→4 |
| Mage Frost (64) | 1→1 | 2→4 | 4→4 |
| Paladin Holy (65) | 3→3 | 2→2 | 4→4 |
| Paladin Protection (66) | 4→4 | 4→4 | 5→5 |
| Paladin Retribution (70) | 4→4 | 2→2 | 3→3 |
| Warrior Arms (71) | 2→2 | 4→4 | 7→7 |
| Warrior Fury (72) | 3→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 3→3 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 4→4 | 5→5 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 2→2 | 3→3 |
| Death Knight Frost (251) | 2→2 | 2→2 | 4→4 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 4→4 | 5→5 |
| Hunter Marksmanship (254) | 2→2 | 4→4 | 4→4 |
| Hunter Survival (255) | 2→2 | 2→4 | 2→4 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 2→2 | 5→5 |
| Rogue Assassination (259) | 4→4 | 3→4 | 6→7 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 4→4 | 6→6 |
| Shaman Restoration (264) | 1→1 | 2→2 | 3→3 |
| Warlock Affliction (265) | 0→0 | 3→3 | 5→5 |
| Warlock Demonology (266) | 0→0 | 2→2 | 4→4 |
| Warlock Destruction (267) | 1→1 | 3→3 | 5→5 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 3→3 | 2→2 | 3→3 |
| Monk Mistweaver (270) | 4→4 | 3→3 | 3→3 |
| Demon Hunter Havoc (577) | 4→5 | 2→4 | 3→5 |
| Demon Hunter Vengeance (581) | 3→4 | 3→3 | 3→3 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 4→6 | 2→3 | 3→4 |
