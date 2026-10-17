# Pet battle effect-property coverage

## Current70391 continuation — 2026-10-16

Baseline server `4b4e017f`, active local-CASC12.1.5.70391: **105 referenced property IDs**,136 definitions,2829 effects,1650 turns,1404 abilities,123 states. Current baseline remains992 admitted abilities and1746/2829 executable rows (61.72%). The104-property table below is historical69933 coverage, not the current census. [Machine-readable current census](pet-battle-census-70391.json) records each rejected graph's effect/turn IDs, first admission gap, all123 state names with their narrower aura-admission gate (64 allowed),35 unresolved state references, and all32 defined but unreferenced properties. Sentinel property0 is referenced without a definition, so subtracting105 referenced IDs from136 definitions would undercount unused definitions. Those graph/state admission classifications are not all-row behavioral certificates.

At investigation start, the canonical server world.db was older: effects5035/5036/5039 change26→387 in70391;5039 changes aura2551→2548; definition387 supplies health-threshold min/max labels. Private server-s3 SQLite copy imported all ten current tables without changing canonical. Lead later published `game-engine/data/server-world/world.db`, preserving the historical packet; slot `data/world.db` now reads that canonical target. Independent SQL readback confirms all ten pet tables equal the private70391 import. Abilities differ only in CSV newline normalization, not text. Threshold387 semantics remain a named gap rather than pretending those rows are ordinary26 applications.

First sourced batch: Fade1084 property107/effect2429. DB2 description explicitly says user swaps with a random pet; turn1752/event6 applies dodge1083 (state73=100), turn1753/event7 removes1083 before107. Bounded implementation chooses a living reserve on the caster's team, reuses existing swap validation/callbacks, and leaves unspecified other107 routes rejected. Three real-ID RED cases reproduce unsupported107;81/81 targeted engine tests then pass at52327dd5, with lib check and fmt clean. Admission becomes993 abilities and1749/2829 rows (61.82%): only3 additional rows, not all nine107 rows. Existing inherited damage constants/rounding are not recertified.

Native privateUDP5961 proof uses owned fb_pbfx/Fadeproof and authored level25 Jadefire Spirit1348 with its unlocked Fade option plus two Mechanical Squirrel39 reserves. Both Modern and Forever show the actual pointer cast, Scratch miss, Fade expiry and ally swap; the two runs choose different living reserve slots. Four inspected1920×1080 originals `fade-{modern,forever}-{before,after}.png` live under `/syncthing/AgentShared/2026-10-16/petbattle-effects/`. All91 named PetBattle rectangles match between skins. Both active models are present and framed before/after; no new UI frame or client-side combat simulation was added. This is the inherited shared PetBattleFrame's bounded swap/log projection, not a certificate of full Retail appearance/timing.

The initial unaccepted Modern capture lacked Jadefire display51504's model571311. Current local CASC authenticated its complete11-asset chain;8 new receipts and2 aliases were staged into the private slot overlay, canonical untouched. Lead published `game-engine-s3/data/diagnostics/petbattle-effects/asset-package/` canonically. Private links now resolve all10 newly staged product/alias files and the fragment to canonical paths after exact hash checks. Actual WowAssetLoader readback of display51504 passes with two meshes, matching model SHA256, ExtractedOnly and CASC tripwire0. Fixture startup was decoupled from the unrelated auction inheritance chain after actual parse failures from removed corpse helpers; shared posed-triangle sampling is retained. Both skins use isolated layout/config stores; the shared owned endpoint token-to-account mapping must accompany the second store. Character80 is an explicit private safety fixture after the original level1 character died while waiting near the wild pet, not an authored encounter-level claim. Owned server/client PIDs and normalized agents-petbattle_effects.slice were stopped after captures.

Source hashes and exact baseline admission JSON: `/home/osso/.worktrees/logs/petbattle-effects-2026-10-16/`. Source CSVs resolve under canonical `data/db2/12.1.5.70391/`; UI reference is only current install `BlizzardInterfaceCode/Interface/AddOns/Blizzard_PetBattleUI` and its FrameXML tooltip helper. No third-party effect interpreter used.

Verified 2026-10-10; authenticated local-CASC build12.1.0.69933; production revisions `1b02589`/`f7ea28f`, final test correction `a0e04b0` (origin/master615fb90 merged into petbattle-effects3; already up to date).

## Two-way walls150 — bounded proof2026-10-16

Higher-row partial handlers retain the historical table's named target/override/stat/callback gaps. Unsupported179 needs causal retaliation ownership/order;229 needs conditional type-state polarity/refresh;0 has no property definition; remaining107 routes lack the Fade description's explicit random/self routing;156 lacks established predicate/control semantics;363's current-health damage needs source-grounded targeting/type/passive boundaries. These are retained gaps, not silently executed while proceeding to the explicit two-way wall descriptions.

Current70391 roots325/433/444/465/479/2318/2398 explicitly say both allied and enemy attacks count against two blocks. BattlePetAbility325: “Creates a wooden dam that blocks two attacks. Both enemy and allied attacks will be blocked.” Their auras326/434/443/464/480/2397 supply states69=object,126=wall,74=2,123=friendly-block mode and75=redirection. Only those authored root/aura pairs are admitted;123/126 are conditional aura-state admissions for those six wall auras, not generic new state handlers. Reuse existing shared charged-object refresh/removal and direct-hit blocking; team objects retain their owner across swaps and ordinary periodic ticks bypass the direct-attack block path. Zero-duration objects persist until charges/removal, as the inherited Decoy object path does.

Six actual RED cases fail on unsupported150 before implementation (an initial missing test-helper import and an over-specific Onyx error-message expectation were fixture errors, not behavioral RED). Canonical70391 GREEN is88/88 at4fbe6e59:1756/2829 admitted rows (62.07%),1006 abilities,150 admits7/8 rows. Lib/example check and both workspace fmt pass. Canonical journal3, wild12 and PvP9 are covered; one PvP case initially lacked its explicit ground-directory environment and passed alone with real canonical ground assets. No product workaround or broad suite rerun.

Native d6710a1d/server and6c3c1feda/engine on privateUDP5961 use owned Mr. Bigglesworth1145's authored Ice Barrier479 option. Both explicit skins physically cast Barrier, retain own/enemy HP through the incoming Scratch block, then cast Claw: enemy HP remains147, Barrier expires, and the following Scratch deals45/65 critical respectively. Own HP1481→1436 in Modern, then1436→1371 in Forever confirms actual event amounts and retained injury, not reset health. Source cooldown5→4 and the shared pad aura appear/disappear. Six inspected1920×1080 `wall-{modern,forever}-{before,incoming-block,friendly-block}.png` originals are under `/syncthing/AgentShared/2026-10-16/petbattle-effects/`. Phase geometry94/101/104 named rectangles differs only in authoritative health fill and the timed float's2px position. Both active source-backed models are framed; no new UI widget or client combat simulation was added. One initial Forever right-click had no battle result; a second actual pointer attempt succeeded, with no product patch or claimed diagnosis. All owned live PIDs were stopped.

Exact source/artifact hashes, native snapshots, pointer ledger, geometry and image inspection: `/home/osso/.worktrees/logs/petbattle-effects-2026-10-16/`. Native build cohorts were synchronized with master after shared honor/world-quest API changes; the interrupted engine build produced no current artifact and is explicitly invalid, followed by one successful durable-log build. This proves the inherited shared HUD's health/aura/cooldown/log projection for479; it does not certify each wall's 3D visual, all seven native casts, exact Retail timing or damage-block feedback wording. Onyx439 says allied attacks are prevented while two enemy attacks are protected; whether allied attempts consume its charges is not established, so438/439 stay rejected. Multiple opposing walls' interception order and exact Retail expiry/block feedback remain uncertified.

## Bench resurrection112 — implementation2026-10-16

Current70391 Raise Dead2347 and Mass Resurrection2348 both say: “Bring all other friendly pets back to life, returning them to5% health.” Effects4328–4331/property112 supply percentage5, required caster state0, required target state1/Is_Dead; turns3889/3890 each contain two ordered rows. Admission is restricted to those roots, orders1/2 and exact parameter tuples. Each row addresses one inactive friendly slot, never the caster or enemy; living reserves are unchanged. Explicit revival clears Is_Dead and stale undead expiry while retaining once-per-battle passive usage. Owned-pet encounter construction now derives Is_Dead from persisted HP, not the fresh full-health constructor default.

Three real-row RED tests reject unsupported112. A separate redb close/reopen→owned-team load RED reproduces zero-HP reserves with missing dead state before any round. GREEN3+1 passes on a7dfb33e/213be7f8; catalog readback admits1760/2829 rows (62.21%),1008 abilities,112 all4/4 rows. Native both-skin projection passes below; final post-merge lib91, wild/PvP22 and shared UI12 gates pass; lib/bin/example check and both fmt pass. A stale shared sibling initially blocked `/hug` consumers before tests; lead-authorized fast-forward5405246→b701f32 resolved that integration drift without a gameplay patch. These are bounded proofs, not blanket parity. Source rows and current CSV hashes: `logs/petbattle-effects-2026-10-16/resurrection-source-rows.json` under the shared worktree log root. Scope excludes generic resurrection selectors, other property111 routes and unrelated HP persistence.

Native server4603d6e3 and reused engine6c3c1feda (unchanged pet-battle/network/session scope, exact artifacts saved) on privateUDP5961 physically cast Micromancer2959's unlocked Raise Dead2347 in both asserted skins. Two dead reserves become78/1563HP; the actual selection panel displays both5% restorations. Pointer swap to one revived Squirrel leaves49/50HP after29/28 Scratch damage; its real Metal Fist then deals147 and wins against Rabbit. Eight individually inspected1920×1080 `resurrection-{modern,forever}-{before,revived,selection,active}.png` originals are in `/syncthing/AgentShared/2026-10-16/petbattle-effects/`. Geometry92/98/131/98 names matches except timed floating-text position; both models are visibly framed in the accepted images. Forever's pre-capture snapshot preceded asynchronous model readiness; the subsequent original PNG itself contains both models. No widget/layout/client simulation changes. Native2348 casting, full resurrection3D VFX and exact Retail event wording/timing are not certified by this2347 projection.

Micromancer display99916's full78-asset closure is authenticated from current local CASC, zero missing/failures. Lead published8 receipts+5 aliases; all13 paths and fragment now resolve canonically after hash readback. Actual canonical WowAssetLoader readback passes one mesh with explicit ExtractedOnly and tripwire0; native world runs used the inherited developer asset mode, not claimed as strict-mode whole-world proof. An initial canonical probe omitted the explicit mode and asserted before load; its stuck child was terminated, and the corrected bounded probe passed. The first native pairing timed out before auth; fresh owned server/client restart recovered without a product patch or claimed root cause. All owned live processes are stopped. Exact hashes, snapshots, geometry, pointer ledger and scope provenance: `/home/osso/.worktrees/logs/petbattle-effects-2026-10-16/`.

## Same-species Dragon's Call370 — implementation2026-10-16

Current70391 ability2283/effect4161 explicitly adds7×AttackBonus damage for each additional pet on the caster's team of the same species. Exact parameters[25,100,7,0,0,0], ordinary turn/event and no aura; Lil' Nefarian2621 and Jeweled Onyx Whelpling3256 unlock it at level2/slot1. Property370's two test-only roots2259/2280 remain rejected. Literal `Pet.species: u32` is supplied by the existing owned/wild Identity at construction and checked against species-state metadata; family is not an identity substitute. Damage adds the authored per-reserve points then reuses direct accuracy/type/power/variance/critical/block handling. The description has no living-only qualifier: matching roster members count even if fainted; this is the literal rule, not independent Retail dead-slot certification.

Actual owned-team RED fails unsupported370 at4161. The real2621/3256 mixed-species control uses all Dragonkin, all enemy species2621, and0/1/2 matching friendly reserves: expected53/68/83 damage at Rare/breed3/level2 power23. Added pure cases cover both teams/all active positions, fainted roster conservation, miss/direct-event behavior and before-RNG rejection. GREEN6/6 passes at52b6ef56; catalog readback1761/2829 rows (62.25%),1009 abilities,370 admits1/3 rows. Native positive-bonus projection passes below. Final lib96/wild-PvP23/UI12, lib/bin/example check and both fmt pass atc321202a/69bb3c334. A test-only pure roster-builder extraction is separately covered by its18-case matrix rerun1/1 ate04d66c1; production/wire/native scope is unchanged, so broad gates were not repeated. Source rows and CSV hashes: `logs/petbattle-effects-2026-10-16/species-source-rows.json` under the shared worktree log root. The pure identity field changes no wire schema or UI; existing fixtures explicitly name species39 instead of creating a missing-identity fallback.

Native serverb1aa03f1, engine artifactb0fbb953 and fixture1796fc761 (sharedb701f32) use three level2/Rare/breed3 Lil' Nefarian2621. Both actual skins physically cast2283 with two same-species reserves: normal85/77 damage reduces Rabbit147→62/70; incoming Scratch27/25 reduces own214→187/189. The non-critical log and cooldown3 match source/runtime;85/77 are inside boosted75–92, above unboosted normal maximum59. The0/1-reserve counterfactuals are proven by the actual owned-team tests, not claimed as separate native casts. Both original1920×1080 `species-{modern,forever}-{before,after-settled}.png` pairs were individually inspected. All91 pre-cast rectangles match;97 common post-cast rectangles differ only in two authoritative HP fills, and Forever retains two still-timed floats. Both active models are framed. One pre-response Modern capture was moved to private diagnostics as unaccepted; polling the authoritative cooldown avoids premature capture, without a product change. Both battles were physically forfeited and all owned processes stopped.

Lil' Nefarian91267's5-asset closure uses user-adopted current local CASC70448/build88d5e8ef, while mechanics tables remain explicitly70391. An initial missing local encoding record resolved on one bounded retry. Publication was held until the user's source decision, then lead published5 receipts+2 aliases. All7 paths+fragment hash-read back canonically; actual WowAssetLoader canonical readback passes3 meshes with ExtractedOnly/tripwire0. Both native snapshots independently report ExtractedOnly/tripwire0, not merely an inferred launch policy. Exact artifacts/source hashes, pointer/skin/damage/geometry/PNG proof: `/home/osso/.worktrees/logs/petbattle-effects-2026-10-16/species-*`. Test-only370 roots, independent Retail fainted-member interpretation, exact timing/rounding and full3D spell VFX remain unqualified.

## Armageddon1025 — bounded proof2026-10-16

Current70391 Armageddon1025 explicitly damages all enemy pets, instantly kills its user and prevents Failsafe mechanisms. Its single turn1686 has three ordinary damage24 rows2327–2329, each[20,100,0,0,0,0], then self-kill135 row2330[200,100,0,0,0,0]. Admission adds this exact root to the existing first-active/other-enemy router and Failsafe-bypassing sacrifice path; its kill tuple/order/aura remain bounded. Existing damage typing/hit/block handling is reused; no new opcode or formula. Generic kill targets, reverse/clone semantics and other sacrifice roots remain gated.

Real RED reproduces `ability 1025: undetermined team target routing`; GREEN8 passes atf1abddfe (seven library cases plus authored level4 Rascal-Bot1256 owned-team/loadout metadata). Eighteen team/active-slot combinations conserve allied reserves, typed enemy hits yield120/180/80 at power100, dead enemy reserves remain dead, all misses still sacrifice the caller, a final enemy death does not suppress self-kill, and unqualified reverse parameters reject before RNG. No Mechanical revival occurs. Current admission1776/2829 rows (62.78%)/1013 abilities: four rows/one root added. Source rows/authenticated70391 CSV hashes and exact RED/GREEN/census logs live under `logs/petbattle-effects-2026-10-16/armageddon-*` in the shared worktree log root. Native proof passes below; final integrated gates remain pending. No full Retail timing/3D VFX claim.

Native server1ddadad0 / engine8c0654ed / fixed protocol6b066803 use legal level4 Rascal-Bot1256 and two Mechanical Squirrel39 reserves. Both actual skins/private5961 physically Armageddon:61/58 damage, Rabbit147→86/89, caller321→0 with no Mechanical revival and an automatic replacement panel. Pointer reserve2 selection preserves334/334; actual Metal Fist58/66 lowers Rabbit to28/23, then Scratch25/29 leaves the reserve309/305. Both battles physically forfeited and owned processes stopped. This wild encounter has one enemy; all-three-enemy routing is separately covered by authoritative source-qualified library/owned-team tests, not claimed as a three-enemy native cast.

Eight individually inspected1920×1080 `armageddon-{modern,forever}-{before,sacrifice,reserve,reserve-attack}.png` originals: `/syncthing/AgentShared/2026-10-16/petbattle-effects/`. Every accepted snapshot waits for two framed models and reports actual ExtractedOnly/tripwire0. Common91/132/96/100 rectangles differ only HP fills/timed floats. Canonical adopted70448 closure55/55,55 receipts+2 aliases and fragment hash-readback pass; actual canonical loader returns two meshes/strict mode/tripwire0. Source/native/artifact/PNG/geometry/loader proof: `logs/petbattle-effects-2026-10-16/armageddon-*` under the shared worktree log root. GuildInvitation.tabard/BgMatch.bracket consumer drift was resolved through landed master consumers and a lead-authorized fixed protocol checkout, never guessed defaults. Its temporary machine-local build configuration is not a product change. Shared UI widgets/layout and client combat simulation remain unchanged.

## Miss-only self outcomes194 — implementation2026-10-16

Current70391 Haymaker762 and Blingtron Gift Package989 explicitly describe a self-stun/self-heal only when their attack misses. Their four-row graphs are damage24 (50% accuracy), all-zero194, all-zero159, then stun174 for one round or healing30/variance20. Admission is bounded to these roots, orders2/3 and zero gate/selector parameters. Runtime tests the current turn's actual miss event, not any failed effect: an absorbed attack does not count as a miss. The selector routes the following effect to the actual caster; Stench's distinct team routing stays unchanged. Existing damage, self-control immunity, healing and caps are reused; no generic failure/callback semantics are inferred.

Real unsupported194 RED2 at1506/2473→GREEN8 on44dd5ba2 (seven library cases plus a real owned-team/loadout/persisted-health case). Controls cover successful/blocked attacks, another pet's earlier miss, both teams/all active slots, capped healing and before-RNG rejection. One initial fixture used Humanoid callers and observed their unrelated400 end-round passive heal after a successful attack; only those two fixture families changed to isolate the branch. Source rows and authenticated CSV hashes are in `logs/petbattle-effects-2026-10-16/miss-branch-source-rows.json`. Census recomputation admits1772/2829 rows (62.64%)/1012 abilities,1942/2: eight additional executable rows/two roots. Both-skin native proof passes below. Final integrated server9ef4bc22/enginec3f983ef9/shared9fe9863: lib111/111, wild/PvP25/25, UI12/12, lib/bin/example check2m05 and both workspace format checks pass without warnings. Exact commands/revisions/results are in `logs/petbattle-effects-2026-10-16/miss-branch-final-proof-ledger.json` under the shared worktree log root.

Native server79207a24 / engine3cc2a947f / shared8f38d111 use legal Clock'em1142/10 and injured Lil' Bling1320/2. Both actual skins physically cast Haymaker: miss→self Stunned174, enemy147 unchanged, own685→659/656 only from Scratch26/29; next pass observes expiry. Successful Haymaker controls separately kill Rabbit147 with no self-stun. Gift989 misses heal59/68; actual HP21→53 after Scratch27 in Modern and76→118 after Scratch26 in Forever, enemy80/147 unchanged. Prior real Gift hits show damage67/68/69 and no healing; casts were retried with real bench cooldowns, not forced RNG. Forever's third hit ended that battle, so its accepted heal uses a fresh private battle. All owned processes stopped; accepted battles forfeited or won.

Ten individually inspected1920×1080 `miss-branch-{modern,forever}-{before-haymaker,haymaker-miss,before-gift,gift-miss,haymaker-hit}.png` originals: `/syncthing/AgentShared/2026-10-16/petbattle-effects/`. Both figures are visibly framed; actual post-cast snapshots report two framed models/ExtractedOnly/tripwire0. Two earlier before snapshots precede model readiness; subsequent originals were inspected separately, not misrepresented as all-snapshot framing proof. Geometry91/104/95/103 differs only HP fills and timed floating text. Exact source/artifact/PNG/geometry/runtime ledgers are `logs/petbattle-effects-2026-10-16/miss-branch-*` under the shared worktree log root. Shared UI widgets/layout and client combat simulation remain unchanged.

Canonical publication supplies51 new adopted70448 receipts plus2 aliases; all53 paths/fragment hash-read back, complete108-asset closure authenticated. Clock'em retains approved70391 receipts. An initial strict probe exposed the slot's partial70448 product tree after canonical base-index migration; all60 local files matched canonical, the local tree was preserved privately and the primary product path linked to canonical. No canonical writes or fallback added. Actual canonical WowAssetLoader readback then passes both displays/two meshes each, ExtractedOnly/tripwire0. Generic failure/callback/selector semantics, exact Retail timing/rounding and full3D spell VFX remain unqualified.

## Refuge223 — implementation2026-10-16

Current70391 Refuge1344's self row2722/property26 supplies Block759 for6 rounds. Other-friendly rows2723/2724/property223 have exact[0,100,6,5,0,0]; RequiredPetType5 means Critter/family4. Description says the user and each friendly Critter block the next attack they take. Admission is limited to this root, orders2/3, that aura and exact tuple. The other slots route on the caster's team and use the common live-aura path with an explicit wearer: preserve original caster and the self row's countdown policy instead of relabeling the ally as caster or treating the family selector as a tick flag. Aura759 only supplies state74=1; each recipient owns its own charge, not a shared team object. Existing direct-hit blocking, refresh and cleanup are reused; generic type overrides/223 semantics and multi-object interception order stay excluded.

Actual RED reproduces unsupported223 at2723. Added cases cover caster-any-family/Critter-only reserves, both teams/all active positions, independent consumption and swaps, six subsequent round expiry, alternate-caster refresh without stacking, fainted conservation, periodic bypass/outgoing attacks and before-RNG rejection. The real owned-team factory case uses1430/1427/39's Elemental/Critter/Mechanical metadata and an authored equipped1344. GREEN9/9 passes at11ae48f4; catalog1764/2829 rows (62.35%),1010 abilities,223 admits2/2. Native projection passes below. Final integrated serverb6548cef/shared8f38d111: lib104/104, wild/PvP24/24 and lib/bin/example check pass without warnings; engine48d53988 UI12/12 passes. Both workspace format checks pass. Exact commands/revisions/results: `logs/petbattle-effects-2026-10-16/refuge-final-proof-ledger.json` under the shared worktree log root. Source rows and CSV hashes: `logs/petbattle-effects-2026-10-16/refuge-source-rows.json` under the shared worktree log root. The inherited live-target/countdown path is not recertification of complete Retail aura timing.

Native serverc4792d73 / engine341e13dd / shared604ea60 use legal level15 Frostfur Rat1427/Refuge1344, a second Critter Rat and non-Critter Mechanical Squirrel39. Both actual skins physically cast Refuge: two Block applications, first incoming Scratch consumes the caller's charge with892/892 unchanged, then pointer swap to Rat2 independently blocks the next Scratch with892/892 unchanged. Swap to Squirrel3 exposes real28/26 damage (978→950/952), while Rabbit147 is unchanged. Cooldown6, retained swap/block log, both models and actual ExtractedOnly/tripwire0 appear in the snapshots. Eight individually inspected1920×1080 `refuge-{modern,forever}-{before,incoming-block,critter-block,noncritter-damage}.png` originals live under `/syncthing/AgentShared/2026-10-16/petbattle-effects/`. Matched92/99/99 rectangles are identical; final99 differ only timed floating-text position. Both battles were physically forfeited and owned processes stopped. Modern initially observed Loading, then one no-result NPC pointer; an observed-input second pointer started the battle without a product patch or root-cause claim.

Subject4960 reuses4 existing approved canonical70391 receipts (1141018 model,1141019/1141020 skins,1141023 texture), with no extraction/new assets/aliases/publication. Closure4/4 and actual canonical WowAssetLoader readback pass one mesh/hash/ExtractedOnly/tripwire0. Source/runtime/PNG/geometry cases and exact build heads/hashes are in `/home/osso/.worktrees/logs/petbattle-effects-2026-10-16/refuge-*`. This proves bounded independent-charge/family-target projection, not full Retail timing/wording, multi-object interception order or3D shield VFX.

## Coverage by rows — do not conflate these metrics

| Metric | Before | After |
|---|---:|---:|
| Handler IDs / referenced IDs |55/104|58/104|
| Rows referencing handler IDs |2672/2829 (94.45%)|2683/2829 (94.84%)|
| Rows in recursively admitted ability graphs |1727/2829 (61.05%)|1746/2829 (61.72%)|
| Admitted ability records (including empty/passive records) |987/1404|992/1404|

**The90% executable-row goal and completion of the original partial handlers are NOT met.** This continuation adds19 admitted rows and5 ability records, with no removed admission.937 rows referencing implemented IDs still belong to rejected graphs;146 rows use46 unsupported IDs. The90% target needs801 more admitted rows. Static admission is not all-row behavioral or Retail proof. Wild inventory stays83/99 candidates on hosted maps0/1/2991; no wild gain. Current default catalog PvP ability reach617/797→618/797; all default/alternate choices663/871→664/871. Newly executable Nevermore522 is default for species3008/3054 and a candidate for six species. This is not acquisition, journal ownership, live utilization or alternate-loadout selection proof.

## petbattle-effects3 bounded lockdown

- **117:** real Mind Games2390 choosing pet's own one-based slots1/2, LockDuration9; surrounding removal rows clear wearer-owned Mind Games auras. The174 reference is not a stun application. Full Addius replacement/restoration encounter remains uncertified.
- **129/168:** Nevermore522/738 and Spell Gulp1858/1859 apply a two-round enemy-pet aura, consume the wearer's next real ability use through event9, lock that used ability for5 rounds and remove their own aura49. Pass/swap does not consume the callback; committed multi-turn ability continues. Persistent Disruptive Spores2213 remains rejected because its ownership/triggering context is not established.
- Slot clocks tick for active/bench pets beside ordinary cooldowns; snapshot now sends actual lockout and disables that slot. Duration values come directly from DB2; the existing round-start clock convention is tested, not certified as exact Retail event/expiry timing. `Action::Ability` selects an ID, not a slot: local default species1352 repeats123 and2480 repeats1737, so independent duplicate-slot selection/lock parity is not certified or redesigned.

Ranking first blockers: OverrideIndex reaches5 wild/13 default PvP/17 alternate-candidate roots; event1 reaches3/28/31; event2 reaches3/16/18. Lockdown reaches0 wild/1 default PvP root. Ranking is catalog dependency reach, not a guarantee one handler admits every graph. Highest-impact candidates remained gated rather than guessing numbers, hit/crit/variance reuse, callback recursion/causal order or level/max-health composition.

## Original12 Partial handlers — exact changes and remaining gaps

Continuation:24/222 gain active-plus-bench routes,50 gains Cyclone/Stench,23/31 gain bounded Leech Life383,54 gains Creeping Fungus742/743. Unknown OverrideIndex, per-level max-health formula, generic callbacks, clone/type and kill semantics remain excluded. The following table retains prior-cycle changes with current remaining gaps.

| ID | Previously missing | Added this cycle | Still not established/implemented |
|---:|---|---|---|
|24|OverrideIndex; multi-pet targets; hit/round state bookkeeping; weather family modifiers|Real slot routing for404/419, last-hit/round states, weather family modifiers, shared blocking|103 nonzero OverrideIndex rows; unrepresented team routing; exact Retail rounding/crit coupling|
|26|Special/max-health states; object/proc graphs; zero-accuracy next-swap trigger|Percent max-health cap via state99, source/recipient healing context, Bend Time team trigger|Per-level maxHealthBonus2 requires level metadata; most object/clone/proc graphs; unexplained Accuracy0 rows|
|50|CasterState/TargetState; shared object/team semantics; refresh state|Required-state guards and exact Decoy refresh/team lifecycle|Other object auras; reactive proc ordering (Cyclone190 now covered)|
|49|Unused Param1 values; Chance0 scripted removal; callback scope|Unused Param1 accepted; unconditional0 inferred from one-shot Bend Time; source-owned removal|Unsupported callback graphs and unknown target routing still rejected|
|23|Healing recipient versus original source power/dealt modifier|Separate source/recipient for pad healing and correct swapped recipient cap|Ambiguous backline/GM routing; live Retail rounding proof|
|222|Weather family modifiers and shared blocking/caps|Inherited shared damage pipeline with explicit variance|Referenced graph gaps; current export has only zero OverrideIndex for222|
|31|Direct Critter CC immunity and damage/death condition states|Direct CC rejects Critter; hit/round states maintained; ordinary deaths set Is_Dead|Special clone/type/stat semantics and missing metadata remain gated|
|54|Some stack/proc/object graphs and chain semantics|Shared stack/refresh infrastructure retained; last-hit/failure state fixed|Confusing Sting632/Tentacle Stab1962 reactive graphs; nontrivial chain/proc ordering|
|22|Nonzero Points rejected even when tooltip-only|All dummy Points fields accepted without HP side effects|Healthy577's state2 is separate unsupported per-level stat behavior, not a dummy heal|
|52|Weather control immunity|Arcane Winds prevents control; existing Critter/Resilient and action-aware timing retained|Exact Retail CC/expiry ordering remains unobserved|
|53|Swapped recipient max HP and weather healing modifiers|Percent heal uses recipient cap and source/recipient modifiers|Ambiguous GM team targets; no universal runtime parity certification|
|63|Source power lost on swaps; Photosynthesis wrongly treated as a pad|Preserve source power; Wish/Tranquility/Mists follow active; Photosynthesis stays wearer-bound|All4 rows admitted; runtime timing/rounding still needs independent Retail evidence|

"Dispatch" below means the admitted parameter/state/target subset is implemented. It does not mean all rows or all referenced abilities are supported. Isolated dummy/trigger tests identify their narrower proof boundary explicitly.

## Full104-property matrix (descending row count)

| Property | Rows | Admitted rows | Implementation / boundary | Real-ability fixed-RNG proof |
|---:|---:|---:|---|---|
|24|945|571|Partial: zero OverrideIndex; grounded active/bench routes; other targets rejected|Bite110; Sunlight404; Tidal Wave419; Arcane Explosion299|
|26|244|155|Dispatch; special-stat/proc graphs and unexplained Accuracy0 gated|Burrow159; Metabolic Boost194; Bend Time1905|
|50|194|105|Dispatch: state guards, stacks/refresh, Cyclone object and Stench per-pet routing|Curse218; Moth Balls507; Cyclone190; Stench527|
|49|138|66|Dispatch: chance/default0, unused Param1; bounded wearer event9 removal; generic callbacks gated|Burrow159; Failsafe723; Bend Time1904; Nevermore738/Spell Gulp1859|
|23|115|93|Dispatch: source power/dealt modifier, recipient cap/taken modifier|Healing Wave123; Tranquility254/255; Leech Life383|
|222|90|82|Dispatch: explicit variance, weather family modifiers, central Magic cap|Burn113; Burrow159; Cyclone189/190|
|31|86|71|Dispatch: absolute state, Critter direct-CC immunity; unknown state semantics gated|Deep Breath169; GM Stun292|
|54|77|55|Dispatch: chain guard, duration/max stacks; reactive proc graphs gated|Poison Fang152; Aquatic passive; Creeping Fungus743|
|158|69|57|Dispatch: stop remaining effects after failed chain|Death Coil121 miss/hit|
|22|61|42|Dispatch: all Points fields are tooltip metadata, no HP operation|Healthy577 (isolated dummy); Curse218; Nevermore738/Spell Gulp1859|
|116|57|46|Dispatch: priority evaluated before order; equal priorities use speed/RNG|Quick Attack563; Belly Slide529|
|52|44|36|Dispatch: control immunity including Arcane Winds; action-aware expiry|Time Stop600; Grasp249|
|28|29|23|Dispatch: cancellable/self aura; shared Decoy charges; other objects gated|Decoy334/333 swap/refresh|
|85|29|13|Partial: reset auras with named state; missing state161 remains rejected|Tidal Wave419 clears objects|
|79|28|19|Dispatch: additive bounded states; priority cleanup bound to original actor|Quick Attack563; forced swap780|
|177|28|24|Dispatch: target-state equality stops later branch, not prior damage|Frog Kiss233/Resilient149|
|135|27|3|Partial: grounded self-kills282/663/2209 only; other routing rejected|Explode282 skips Failsafe|
|32|25|13|Dispatch: percentage of actual last hit, no second power scaling|Death Coil121 restores42 of84|
|80|24|11|Partial: global weather, exact delta reversal, Elemental exemptions; flat-source scaling gated|Darkness256; Moonfire595; Sunlight404; Scorched Earth172|
|178|24|15|Dispatch: target-state/chain immunity guard and immune reporting|Bash348 immune/skip|
|29|23|10|Dispatch: additional damage only with required caster/target state|Takedown221 normal/stunned|
|62|23|8|Dispatch: target max-HP percentage without power/variance; typed damage/cap|Trample377|
|384|23|23|Dispatch: faster/first-strike conditional attack|Tail Sweep122 faster|
|141|22|22|Dispatch: split pool across living enemy pets, conditional weather bonus|Arcane Storm589 three/two targets|
|27|21|20|Dispatch: capped ramp; hits156/930 versus uses for others|Frog Kiss233; Vicious Fang156 miss|
|136|19|16|Dispatch: personal aura cleanse, not global weather/team objects|Sear Magic763|
|145|18|12|Dispatch: highest-health live bench swap; displaced action cancelled|Sweep457|
|373|17|8|Dispatch: zero-based ability slot assignment; surrounding boss states gated|Necrotic Reconstruction2262 (isolated trigger)|
|139|15|12|Dispatch: lowest-health live bench swap; displaced action cancelled|Death Grip780|
|157|14|5|Dispatch: caster-state equality gates remaining branch|Belly Slide529 Blizzard state58|
|111|13|2|Partial: self resurrection; ambiguous team resurrection gated|Failsafe723 restores2000, clears dead|
|179|13|0|No; explicit rejection|—|
|229|13|0|No; explicit rejection|—|
|246|12|6|Partial: zero modification clears one-based slots; nonzero modification undetermined|Bend Time1905/1904 next incoming pet|
|68|11|3|Partial: source-HP percentage, grounded self-sacrifice/enemy routing; other graphs gated|Explode282|
|104|11|10|Dispatch: required-state heal, explicit variance|Devour538 only after death|
|53|10|7|Dispatch: recipient maximum-HP percentage, no second power scaling|Wish273/274; Recovery726|
|159|10|4|Partial: selectors only in Stench527/1010; other graphs rejected|Stench527|
|0|9|0|No; explicit rejection|—|
|107|9|0|No; explicit rejection|—|
|131|9|8|Partial: first-acting interrupt; Horn Attack chain rejected|Kick307|
|156|9|0|No; explicit rejection|—|
|363|9|0|No; explicit rejection|—|
|150|8|0|No; explicit rejection|—|
|76|7|7|Dispatch: persistent charge; second use fires and consumes charge|Wind-Up459|
|96|7|7|Dispatch: required-state guarantees accuracy; targetability still checked|Nocturnal Strike517|
|100|6|6|Dispatch: living other allies and family filters;533/922/1956 only|Rebuild533, nonzero active slot|
|168|6|2|Partial: negative next-ability aura for522/1858 only; other routing gated|Nevermore522; Spell Gulp1858 hit/miss/wait/swap|
|170|6|6|Dispatch: weather-gated additional damage|Solar Beam753 Sunlight state61|
|226|6|6|Dispatch: bonus points for target state|Deep Burn1407|
|25|5|0|No; explicit rejection|—|
|121|5|0|No; explicit rejection|—|
|122|5|0|No; explicit rejection|—|
|44|4|4|Dispatch: percentage of actual last hit taken; no second power scaling|Cauterize173|
|59|4|4|Dispatch: additional hit only with lower current HP|Comeback253|
|63|4|4|Dispatch: team pad versus wearer; retain source power through swaps|Wish273; Tranquility254; Photosynthesis268|
|112|4|0|No; explicit rejection|—|
|169|4|2|Partial: negative-state damage; front-pair weather routing; reactive events gated|Scorched Earth171/172 Elemental exemption|
|172|4|4|Dispatch: state increases stun chance; existing CC immunity|Deep Freeze481 normal/chilled/Critter|
|372|4|0|No; explicit rejection|—|
|385|4|4|Dispatch: slower/last-strike conditional attack|Tail Sweep122 slower|
|66|3|3|Dispatch: double below25%, not at25%|Predatory Strike518|
|128|3|0|No; explicit rejection|—|
|129|3|2|Partial: wearer next-used ability lockdown for738/1859; persistent2213 gated|Nevermore522; Spell Gulp1858; committed Deep Breath169|
|230|3|0|No; explicit rejection|—|
|248|3|0|No; explicit rejection|—|
|370|3|0|No; explicit rejection|—|
|77|2|0|No; explicit rejection|—|
|117|2|2|Dispatch: Mind Games2390 self one-based slots1/2 lockdown; no invented stun|Mind Games2390,9-round slot clock/bench/snapshot|
|123|2|0|No; explicit rejection|—|
|124|2|0|No; explicit rejection|—|
|138|2|0|No; explicit rejection|—|
|149|2|2|Dispatch: damage cannot reduce target below1|Weakening Blow826 Mechanical target|
|194|2|0|No; explicit rejection|—|
|197|2|0|No; explicit rejection|—|
|223|2|0|No; explicit rejection|—|
|249|2|0|No; explicit rejection|—|
|254|2|1|Partial: low-health heal; Gonk2069 callback rejected|Eat the Fly2206|
|255|2|0|No; explicit rejection|—|
|256|2|0|No; explicit rejection|—|
|360|2|0|No; explicit rejection|—|
|45|1|1|Dispatch: strict slower-speed bonus; actual20/150 values|Slow Digestion2024|
|61|1|0|No; explicit rejection|—|
|65|1|1|Dispatch: caster weather-state bonus, periodic damage|Creeping Fungus742/743|
|67|1|0|No; explicit rejection|—|
|72|1|0|No; explicit rejection|—|
|74|1|0|No; explicit rejection|—|
|75|1|0|No; explicit rejection|—|
|78|1|0|No; explicit rejection|—|
|86|1|0|No; explicit rejection|—|
|97|1|1|Dispatch: strict faster-speed bonus; actual20/150 values|Rapid Consumption2026|
|137|1|0|No; explicit rejection|—|
|140|1|0|No; explicit rejection|—|
|144|1|0|No; explicit rejection|—|
|165|1|0|No; explicit rejection|—|
|171|1|1|Dispatch: required-state additional heal|Photosynthesis267/268 doubles sunny tick|
|201|1|0|No; explicit rejection|—|
|204|1|0|No; explicit rejection|—|
|358|1|0|No; explicit rejection|—|
|359|1|0|No; explicit rejection|—|
|362|1|0|No; explicit rejection|—|
|374|1|0|No; explicit rejection|—|
|376|1|0|No; explicit rejection|—|
|380|1|0|No; explicit rejection|—|

## Undetermined / deliberately rejected, not fabricated

- **Target/override representation:** property24 OverrideIndex has103 nonzero rows, but labels/descriptions do not establish variance/critical/hit reuse. Broad team/bench descriptions without a implemented route are rejected; Sunlight404, Tidal Wave419 and16 additional described three-row attacks have active/other-enemy routes. Arbitrary team graphs remain rejected. Heal Back Line70/GM Heal295/GM Revive714 and unknown135 kill routing are rejected rather than selecting the caster repeatedly.
- **State/special stat semantics:** state161 referenced by85 is absent from BattlePetState; no invented category. State2 maxHealthBonus is per-level, but Pet has no level field. Clone/type-override/boss state graphs remain gated. Sunny Day403 has state99=50 and65=25: max cap15000 and heal225 from Healing Wave30*6*1.25 are tested; no healing effect is invented on weather application. Current-HP transition/rounding has no captured Retail proof.
- **Reactive and conditional aura semantics:** proc events1..4/9/12 remain rejected except Leech Life383's own post-hit healing event2 and wearer-owned Nevermore738/Spell Gulp1859 event9; current data does not determine generic callback ordering/recursion, hit ownership and aura expiry interaction.179 (13 rows) needs retaliation ownership;229 (13 rows) needs conditional type-state polarity/refresh semantics. These are the two largest unsupported IDs. Complex chain values in632/1962 and special immunity/reverse/passive options in135 remain unproven.
- **Weather/terrain and cooldown boundaries:** Darkness, Moonlight, Sunlight, Arcane Winds and Scorched Earth have real-ID CPU cases. Flat-modifier weather (Sandstorm454) is rejected until the tooltip AttackBonus power owner is established; no attacker/source substitution. Cleansing Rain duration changes, Lightning proc damage and objects other than Decoy/Cyclone are not certified.246 modification1 in Forbidden Hourglass1932 is not inferable from its terse description, so only0 clearing executes.
- **Numerical/runtime boundary:** inherited weak multiplier.67, family passive/crit constants and rounding are not newly invented, but exact current Retail game-table/observed timing remains absent. No live Retail comparison or independent verifier PASS; Claude OAuth expired before verification ran.46 unsupported IDs are listed in the matrix; not every omitted rare handler is intrinsically unknowable. Life Exchange277 averaging/rounding and generic159/194 selectors remain undetermined.

## Data citations / derivation

Source of new semantics is read-only `data/world.db` imported from authenticated local CASC. Join `battle_pet_ability_effect.BattlePetAbilityTurnID` -> `battle_pet_ability_turn.BattlePetAbilityID` -> `battle_pet_ability.ID`, and `BattlePetEffectPropertiesID` -> `battle_pet_effect_properties.ID`; resolve state IDs through `battle_pet_state` / `battle_pet_ability_state`.

- Death Coil121 effects288/289/2137 (24/158/32):14 points,50% last-hit heal ->84 damage/42 heal at power100. Param labels identify Points/Accuracy/ChainFailure; actual hit accounting is not another power-scaled heal.
- Frog Kiss233 / Arcane Blast421 labels PointsIncreasePerUse/PointsMax; Vicious Fang156 / Huge Fang930 descriptions explicitly say "each time it hits", unlike ramp-on-use descriptions. Tests distinguish a miss.
- Darkness257 states66=-50,41=-10,82=1,56=1; Moonlight596 states66=25,87=10,89=6,55=1; Sunny Day403 states65=25,99=50,61=1. Elemental241 description supplies negative-weather immunity. Arcane Storm589 labels BonusState54/BonusPoints4; its description says split among enemies. Pool24*6/3=48 or(24+4)*6/2=84.
- Decoy333 states69=1,74=2,75=333; Decoy334 description says block two attacks. Tranquility254 and Renewing Mists511 explicitly persist through pet swaps; Photosynthesis268 does not describe a team pad, and its conditional heal267 tests Sunlight61.
- Explode282 explicitly prevents Failsafe; its68 Points40 is source-health percent and135 kills the caster. GM Kill All Enemy967 contradicts that same default target, so its graph is rejected. Tidal Wave419 explicitly damages enemy pets and destroys both teams' objects. Failsafe723111 Percentage20 restores2000/10000, independent of power.

Cases are in `crates/server/src/pet_battle/{property_tests,effects2_tests}.rs`; pre-existing cases remain in `tests.rs`. New semantics come from actual labels/descriptions. Cyclone189 has6/35/50 points/accuracy/variance, object state69 and three damage rows; root190 lasts5 rounds. Stench527/1010 names enemy-team accuracy reduction;159 only separates its three50 rows. Root383 names ON_DAMAGE_DEALT healing and its event2 only heals/sets guards. No generic damage callback was enabled. No battle data staged or committed.

No local TrinityCore pet-battle effect interpreter was found in known reference locations. Diagnostic `PetBattleAbilityEffect.cpp`/`PetBattle.cpp` files are **LegionCore9222216**, not TrinityCore or current Retail authority; their explicit unknown/no-op removal and kill handlers were not adopted. Retained TrinityCorea352b1fa stat provenance and local Retail SharedPetBattleTemplates formula provenance live in the [engine wiki](wiki/systems/pet-battle-engine.md#numeric-provenance), not a claim that TrinityCore implements these new properties.

## petbattle-effects3 proof / reproduction

All native Cargo commands from leased game-server-s3 via `/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run pbfx3`; no build-lock. The requested post-master `cargo test --locked -p server pet_battle -- --nocapture` ran once at34114e0:77/78 library tests passed; a new fixture incorrectly expected Deep Breath169 damage on a third turn. Authentic DB2 and the existing Deep Breath test show two turns. Test-only correctiona0e04b0 passes the focused6/6 effect tests; the previously unexecuted main-binary pet_battle scope passes16/16 including wild/PvP/HUD. No repeated broad command or fresh94/94 single-run claim. All72 prior engine cases and six current new cases have passing evidence on unchanged production; the failure was fixture ordering, not a changed runtime rule.

New semantics reproduce missing168/117 failures before implementation and snapshot0-versus9 RED; current six deterministic engine tests and real snapshot9 GREEN prove wearer/source ownership, pass/swap/miss, bench ticking, clock expiry, committed continuation and rejected ambiguities before RNG. Census1746/992 at34114e0 remains valid after test-onlya0e04b0. Evidence: `game-engine/data/diagnostics/pbfx3-20261010/{coverage.json,before-after.json,ranking.json,wild-after.json,local-effect-evidence.json,retail-lua-source.json,tdb-search.json}` and RED/GREEN/final logs. Read-only local-CASC CSV hashes retained. TDB1210 SQL scan finds breed/species/ability tables but no effect/turn/state interpreter tables; no C++ was used and this is not exhaustive evidence certification. Format/check/readability results are recorded in the final handoff and verification receipt; no independent verifier or live Retail claim.

## Prior petbattle-effects2 proof / reproduction

From the leased slot, no dependency override/CSV environment needed:

```text
/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run pbfx2 cargo test --locked -p server pet_battle -- --nocapture
```

At Rust607671d after the feature-branch master merge:72 engine +7 wild/startup tests pass (79 total), zero failures/ignored; unrelated1872+18+4+3 filtered. Format/check exit0, no warnings observed. No full workspace suite, live/client test, deployment or master merge.

Durable proof: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/pbfx2-20261010/`. Four batches reproduce6/5/2/4 missing-behavior failures before implementation;17 new real-ID tests pass. The first post-merge suite found an obsolete Cyclone-rejection assertion; it now proves Volcano176 remains rejected before RNG. `final-pet-battle-corrected.log` is current79-test proof. `verification-receipt.json` records format/check outcomes and stdout/compiler-stderr logging boundary; large full tool captures are retained. Previous evidence remains in sibling `pbfx-20261010/`.

`coverage.json` includes every rejected root's first blocker. `wild-property-ranking.json` and `wild-after.json` preserve99 candidate abilities and16 remaining blockers. `effect-evidence.json`, `property-evidence.json`, `tooltip-source.txt` cite local records and Lua hash. Documentation-only changes do not invalidate Rust proof. Original partial handlers and90% admission remain unfinished; no independent verification or live Retail parity claim.
